//! Network commands serviced by a non-terminal host's existing project owners.

use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const SHUTDOWN_PERSIST_GRACE: Duration = Duration::from_millis(250);

use super::{
    command::{CancelAdmissions, PromptProgress, SteerAdmissions},
    host_adapter::HostAdapter,
    protocol::{Envelope, ErrorCode, ExecutionState, Message, Operation},
    subtab::SubtabExecution,
};
use crate::runtime::{
    BackgroundRunner, CancellationToken, JobOutcome, RuntimeConfig, RuntimeEvent,
};
use crate::{
    core::Agent,
    project_workspace::{MAX_PROJECT_TABS, ProjectOwnerPool},
};

#[derive(Clone)]
struct PromptJob {
    owner: Arc<Mutex<Agent>>,
    request: Envelope,
    device: String,
    text: String,
    execution: SubtabExecution,
}

type PromptRunner = BackgroundRunner<
    PromptJob,
    (),
    crate::core::AgentError,
    Box<dyn Fn(PromptJob, CancellationToken) -> Result<(), crate::core::AgentError> + Send + Sync>,
>;
struct ActivePrompt {
    id: crate::runtime::JobId,
    job: PromptJob,
    runner: PromptRunner,
    completion: Option<(ExecutionState, Option<ErrorCode>)>,
}

fn completion_status(
    outcome: &JobOutcome<(), crate::core::AgentError>,
) -> (ExecutionState, Option<ErrorCode>) {
    match outcome {
        JobOutcome::Succeeded(()) => (ExecutionState::Succeeded, None),
        JobOutcome::Failed(error) => (
            ExecutionState::Failed,
            Some(super::command::failure_code(error)),
        ),
        JobOutcome::Panicked => (ExecutionState::Failed, Some(ErrorCode::InternalError)),
        JobOutcome::Cancelled => (ExecutionState::Cancelled, None),
    }
}

/// Keep the confirmed result while a reader holds the owner; never retry an ambiguous append.
fn persist_completion(lane: &mut ActivePrompt, progress: &PromptProgress) -> bool {
    let Some((state, code)) = lane.completion else {
        return false;
    };
    let job = &lane.job;
    let saved = match job.owner.try_lock() {
        Ok(mut agent) => super::command::persist_progress_with_code(
            &mut agent,
            &job.request,
            &job.device,
            state,
            code,
        )
        .is_ok(),
        Err(std::sync::TryLockError::WouldBlock) => {
            return false;
        }
        Err(std::sync::TryLockError::Poisoned(_)) => false,
    };
    lane.completion = None;
    progress.record_with_code(
        &job.request,
        &job.device,
        if saved {
            state
        } else {
            ExecutionState::UnknownOutcome
        },
        if saved { code } else { None },
    );
    true
}

fn prompt_runner(progress: Arc<PromptProgress>) -> PromptRunner {
    let worker: Box<
        dyn Fn(PromptJob, CancellationToken) -> Result<(), crate::core::AgentError> + Send + Sync,
    > = Box::new(move |job, token| {
        let mut agent = job
            .owner
            .lock()
            .map_err(|_| crate::core::AgentError::InvalidTurn("agent lock poisoned".into()))?;
        super::command::persist_progress(
            &mut agent,
            &job.request,
            &job.device,
            ExecutionState::Running,
        )
        .map_err(|error| crate::core::AgentError::Recovery(error.to_string()))?;
        progress.record(&job.request, &job.device, ExecutionState::Running);
        job.execution
            .run_cancelable(&mut agent, &|| token.is_cancelled(), |agent| {
                let input = super::command::prompt_input(agent, job.text)?;
                agent.process_with_cancel_and_events(input, || token.is_cancelled(), &mut |event| {
                    progress.provider_event(&job.request, &job.device, &event);
                    Ok(())
                })
            })??;
        token.mark_completed();
        Ok(())
    });
    BackgroundRunner::spawn(worker, RuntimeConfig::default())
}

struct ShutdownProgress {
    job: PromptJob,
    state: ExecutionState,
    code: Option<ErrorCode>,
}

fn shutdown_prompt(lane: ActivePrompt, deadline: Instant) -> ShutdownProgress {
    let (mut state, mut code) = lane
        .completion
        .unwrap_or((ExecutionState::UnknownOutcome, None));
    if state == ExecutionState::Cancelled {
        state = ExecutionState::UnknownOutcome;
    }
    let joined = lane.runner.shutdown_and_join_with_events(
        deadline.saturating_duration_since(Instant::now()),
        |event| {
            if let RuntimeEvent::Completed { id, outcome } = event
                && id == lane.id
            {
                (state, code) = completion_status(&outcome);
                if state == ExecutionState::Cancelled {
                    state = ExecutionState::UnknownOutcome;
                }
            }
        },
    );
    if joined.is_err() {
        state = ExecutionState::UnknownOutcome;
        code = None;
    }
    ShutdownProgress {
        job: lane.job,
        state,
        code,
    }
}

/// One shared lock-wait budget; a held owner must not hide other projects' results.
fn persist_shutdown_progress(mut pending: Vec<ShutdownProgress>, deadline: Instant) {
    loop {
        pending.retain(|item| match item.job.owner.try_lock() {
            Ok(mut agent) => {
                let _ = super::command::persist_progress_with_code(
                    &mut agent,
                    &item.job.request,
                    &item.job.device,
                    item.state,
                    item.code,
                );
                false
            }
            Err(std::sync::TryLockError::WouldBlock) => true,
            Err(std::sync::TryLockError::Poisoned(_)) => false,
        });
        if pending.is_empty() || Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(
            Duration::from_millis(5).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
}

/// Mailbox wait when nothing runs; `shutdown` is observed within it.
const IDLE_WAIT: Duration = Duration::from_millis(500);
/// Progress poll while a prompt, worker or steer is in flight.
const BUSY_POLL: Duration = Duration::from_millis(10);

pub struct HeadlessHost {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl HeadlessHost {
    /// At most one captured job per existing project owner; no duplicate Agent/session writer.
    pub fn start(mut pool: ProjectOwnerPool) -> (Arc<HostAdapter>, Self) {
        let (adapter, mailbox) = HostAdapter::owner_thread();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let join = std::thread::spawn(move || {
            let progress = Arc::new(PromptProgress::default());
            let mut active: BTreeMap<String, ActivePrompt> = BTreeMap::new();
            let mut next_job = 0u64;
            let mut steer = SteerAdmissions::default();
            let mut cancel = CancelAdmissions;
            let mut worktrees = super::worktree_host::WorktreeHost::default();
            while !stopping.load(Ordering::Acquire) {
                worktrees.poll(&mut pool);
                let mut completed = Vec::new();
                for (project, lane) in &mut active {
                    while let Ok(event) = lane.runner.try_next_event() {
                        if let RuntimeEvent::Completed { id, outcome } = event
                            && lane.id == id
                        {
                            lane.completion = Some(completion_status(&outcome));
                        }
                    }
                    if persist_completion(lane, &progress) {
                        completed.push(project.clone());
                    }
                }
                for project in completed {
                    if let Some(lane) = active.remove(&project) {
                        let _ = lane.runner.shutdown_and_join();
                    }
                }
                steer.poll(&mut pool);
                // Sleep on the request mailbox; running prompts, workers and
                // pending steers still need their short progress poll.
                let wait = if active.is_empty() && worktrees.idle() && steer.is_empty() {
                    IDLE_WAIT
                } else {
                    BUSY_POLL
                };
                mailbox.drain_with_wait(wait, |registry, device, request| {
                    let operation = match &request.payload {
                        Message::Request { operation, .. } => Some(*operation),
                        _ => None,
                    };
                    let project = request.project_id.as_deref().unwrap_or_default();
                    let original = active
                        .get(project)
                        .map(|lane| (&lane.job.request, lane.job.device.as_str()));
                    match operation {
                        Some(Operation::SessionRename) => super::sessions::rename(
                            &mut pool,
                            registry,
                            device,
                            &request,
                            active.contains_key(project) || worktrees.busy(project),
                        ),
                        Some(Operation::SessionNew | Operation::SessionResume) => {
                            super::sessions::new_session(
                                &mut pool,
                                registry,
                                device,
                                &request,
                                active.contains_key(project) || worktrees.busy(project),
                            )
                        }
                        Some(Operation::SetConcurrency) => worktrees
                            .dispatch_quota(
                                &mut pool,
                                registry,
                                device,
                                &request,
                                active.contains_key(project),
                            )
                            .unwrap_or_else(|| {
                                HostAdapter::dispatch_owned(&mut pool, registry, device, request)
                            }),
                        Some(Operation::ModelSelect | Operation::ReasoningSelect) => {
                            super::model::select(
                                &mut pool,
                                registry,
                                device,
                                &request,
                                active.contains_key(project) || worktrees.busy(project),
                            )
                        }
                        Some(Operation::ResourcesReload) => super::resources::reload(
                            &mut pool,
                            registry,
                            device,
                            &request,
                            active.contains_key(project) || worktrees.busy(project),
                        ),
                        Some(Operation::Prompt)
                            if super::worktree_host::WorktreeHost::is_worker_prompt(&request) =>
                        {
                            worktrees.dispatch_prompt(
                                &mut pool,
                                registry,
                                device,
                                &request,
                                active.contains_key(project),
                            )
                        }
                        Some(Operation::Prompt) => super::command::dispatch_prompt(
                            &mut pool,
                            registry,
                            device,
                            &request,
                            active.contains_key(project)
                                || worktrees.busy(project)
                                || active.len() >= MAX_PROJECT_TABS,
                            |owner, text, execution| {
                                next_job =
                                    next_job.checked_add(1).ok_or(ErrorCode::ResourceLimit)?;
                                let job = PromptJob {
                                    owner,
                                    text,
                                    execution,
                                    request: request.clone(),
                                    device: device.id().into(),
                                };
                                let runner = prompt_runner(progress.clone());
                                let id = runner
                                    .try_submit(job.clone())
                                    .map_err(|_| ErrorCode::Busy)?;
                                progress.record(&request, device.id(), ExecutionState::Accepted);
                                let job_id = next_job.to_string();
                                active.insert(
                                    request
                                        .project_id
                                        .clone()
                                        .ok_or(ErrorCode::InvalidRequest)?,
                                    ActivePrompt {
                                        id,
                                        job,
                                        runner,
                                        completion: None,
                                    },
                                );
                                Ok(job_id)
                            },
                        ),
                        Some(Operation::Steer) => {
                            steer.dispatch(&mut pool, registry, device, &request, original)
                        }
                        Some(Operation::Cancel) => {
                            if let Some(response) =
                                worktrees.cancel(&mut pool, registry, device, &request)
                            {
                                return response;
                            }
                            let approval = request
                                .project_id
                                .as_deref()
                                .and_then(|id| pool.control(id))
                                .and_then(|control| control.approval);
                            cancel.dispatch(&mut pool, registry, device, &request, original, || {
                                let lane = active.get(project).ok_or(ErrorCode::InvalidRequest)?;
                                lane.runner
                                    .try_cancel(lane.id)
                                    .map_err(|_| ErrorCode::Busy)?;
                                if let Some(approval) = approval {
                                    approval.emergency_cancel();
                                }
                                Ok(())
                            })
                        }
                        _ => {
                            if let Some(response) =
                                worktrees.status_page(&pool, registry, device, &request)
                            {
                                return response;
                            }
                            // Selection can change, but a captured job must not lose its project owner.
                            if matches!(
                                operation,
                                Some(Operation::WorkspaceClose | Operation::SubtabClose)
                            ) && active.contains_key(project)
                            {
                                if request.validate().is_err() || request.id.is_none() {
                                    return HostAdapter::failure(
                                        &request,
                                        ErrorCode::InvalidRequest,
                                    );
                                }
                                if let Err(error) = registry.authorize(
                                    device,
                                    request.project_id.as_deref().unwrap_or_default(),
                                    operation.unwrap(),
                                ) {
                                    return HostAdapter::failure(&request, error.remote_code());
                                }
                                return HostAdapter::failure(&request, ErrorCode::Busy);
                            }
                            let mut response =
                                HostAdapter::dispatch_owned(&mut pool, registry, device, request);
                            if operation == Some(Operation::WorkspaceList)
                                && let Message::Response { result } = &mut response.payload
                                && let Some(projects) = result["projects"].as_array_mut()
                            {
                                for project in projects {
                                    if let Some(id) = project["id"].as_str().map(str::to_owned) {
                                        project["prompt_control"] = serde_json::json!(
                                            pool.has_workspace_checkpoint()
                                                && registry
                                                    .authorize(device, &id, Operation::Prompt)
                                                    .is_ok()
                                        );
                                        project["prompt_statuses"] =
                                            progress.listing(&pool, device.id(), &id);
                                        project["worker_control"] =
                                            project["prompt_control"].clone();
                                        project["worker_statuses"] =
                                            worktrees.listing(&pool, device.id(), &id);
                                    }
                                }
                            }
                            response
                        }
                    }
                });
            }
            for lane in active.values() {
                let job = &lane.job;
                if let Some(approval) = job
                    .request
                    .project_id
                    .as_deref()
                    .and_then(|id| pool.control(id))
                    .and_then(|control| control.approval)
                {
                    approval.emergency_cancel();
                }
                let _ = lane.runner.try_shutdown();
            }
            // Cancelled can mean detached code; only confirmed runtime results
            // may replace the conservative unknown outcome during shutdown.
            let join_deadline = Instant::now() + crate::runtime::DEFAULT_SHUTDOWN_GRACE;
            worktrees.request_shutdown(join_deadline);
            let pending = active
                .into_values()
                .map(|lane| shutdown_prompt(lane, join_deadline))
                .collect();
            worktrees.shutdown(&pool, join_deadline);
            persist_shutdown_progress(pending, Instant::now() + SHUTDOWN_PERSIST_GRACE);
            steer.poll(&mut pool);
        });
        (
            Arc::new(adapter),
            Self {
                stop,
                join: Some(join),
            },
        )
    }

    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for HeadlessHost {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn shutdown(lane: ActivePrompt) {
        let progress = shutdown_prompt(
            lane,
            Instant::now() + crate::runtime::DEFAULT_SHUTDOWN_GRACE,
        );
        persist_shutdown_progress(vec![progress], Instant::now() + SHUTDOWN_PERSIST_GRACE);
    }

    fn completed_lane(root: &std::path::Path) -> (ProjectOwnerPool, ActivePrompt) {
        let session_path = root.join("session.jsonl");
        let session = crate::session::SessionStore::open_in_workspace(&session_path, root).unwrap();
        drop(session);
        let session = crate::session::SessionStore::open_existing_writable(&session_path).unwrap();
        let owner = Arc::new(Mutex::new(Agent::with_echo(session)));
        let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let mut request = Envelope::new(
            super::super::protocol::Channel::Control,
            Message::Request {
                operation: Operation::Prompt,
                arguments: json!({"text":"ordinary"}),
            },
        );
        request.id = Some("deferred-completion".into());
        request.project_id = Some(project.clone());
        request.session_id = Some(pool.active_context().session_id.clone());
        let job = PromptJob {
            owner,
            request,
            device: "device".into(),
            text: "ordinary".into(),
            execution: pool.subtab_execution(&project).unwrap(),
        };
        let worker: Box<
            dyn Fn(PromptJob, CancellationToken) -> Result<(), crate::core::AgentError>
                + Send
                + Sync,
        > = Box::new(|_, token| {
            token.mark_completed();
            Ok(())
        });
        let runner = BackgroundRunner::spawn(worker, RuntimeConfig::default());
        let id = runner.try_submit(job.clone()).unwrap();
        loop {
            if let RuntimeEvent::Completed { id: done, outcome } =
                runner.recv_timeout(Duration::from_secs(5)).unwrap()
            {
                assert_eq!(done, id);
                return (
                    pool,
                    ActivePrompt {
                        id,
                        job,
                        runner,
                        completion: Some(completion_status(&outcome)),
                    },
                );
            }
        }
    }

    #[test]
    fn confirmed_completion_waits_for_reader_then_persists_once_and_restores() {
        for state in [
            ExecutionState::Succeeded,
            ExecutionState::Failed,
            ExecutionState::Cancelled,
        ] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().canonicalize().unwrap();
            let (pool, mut lane) = completed_lane(&root);
            let code = (state == ExecutionState::Failed).then_some(ErrorCode::InvalidRequest);
            lane.completion = Some((state, code));
            let project = lane.job.request.project_id.clone().unwrap();
            let owner = lane.job.owner.clone();
            let progress = PromptProgress::default();
            progress.record(&lane.job.request, "device", ExecutionState::Running);
            let before = std::fs::read(root.join("session.jsonl")).unwrap();
            let reader = owner.lock().unwrap();
            for _ in 0..10 {
                assert!(!persist_completion(&mut lane, &progress));
                assert_eq!(lane.completion, Some((state, code)));
                assert_eq!(
                    progress.listing(&pool, "device", &project)[0]["state"],
                    "running"
                );
            }
            assert_eq!(std::fs::read(root.join("session.jsonl")).unwrap(), before);
            drop(reader);
            assert!(persist_completion(&mut lane, &progress));
            assert!(!persist_completion(&mut lane, &progress));
            assert_eq!(lane.completion, None);
            let statuses = progress.listing(&pool, "device", &project);
            assert_eq!(statuses[0]["state"], serde_json::to_value(state).unwrap());
            assert_eq!(
                statuses[0].get("failure_code"),
                code.map(|value| serde_json::to_value(value).unwrap())
                    .as_ref()
            );
            assert_eq!(
                PromptProgress::default().listing(&pool, "device", &project),
                statuses
            );
            let journal =
                crate::session::SessionStore::open_existing(root.join("session.jsonl")).unwrap();
            assert_eq!(
                journal
                    .events()
                    .iter()
                    .filter(|event| event["type"] == "remote_prompt_progress_v1")
                    .count(),
                1
            );
            lane.runner.shutdown_and_join().unwrap();
        }
    }

    #[test]
    fn failed_completion_append_is_unknown_and_not_retried() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (pool, mut lane) = completed_lane(&root);
        let progress = PromptProgress::default();
        let path = root.join("session.jsonl");
        let mut external = std::fs::read(&path).unwrap();
        external.extend_from_slice(b"EXTERNAL_JOURNAL_CHANGE\n");
        std::fs::write(&path, external).unwrap();
        let before = std::fs::read(root.join("session.jsonl")).unwrap();
        assert!(persist_completion(&mut lane, &progress));
        assert_eq!(lane.completion, None);
        assert!(!persist_completion(&mut lane, &progress));
        let status = progress.listing(
            &pool,
            "device",
            lane.job.request.project_id.as_deref().unwrap(),
        );
        assert_eq!(status[0]["state"], "unknown_outcome");
        assert!(status[0].get("failure_code").is_none());
        assert_eq!(std::fs::read(root.join("session.jsonl")).unwrap(), before);
        lane.runner.shutdown_and_join().unwrap();
    }

    #[test]
    fn shutdown_retains_completion_already_consumed_during_reader_contention() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (pool, mut lane) = completed_lane(&root);
        let owner = lane.job.owner.clone();
        let reader = owner.lock().unwrap();
        assert!(!persist_completion(&mut lane, &PromptProgress::default()));
        drop(reader);
        shutdown(lane);
        let status =
            PromptProgress::default().listing(&pool, "device", &pool.active_context().project_id);
        assert_eq!(status[0]["state"], "succeeded");
    }

    #[test]
    fn shutdown_persistence_waits_for_short_reader_without_repeating_append() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (pool, lane) = completed_lane(&root);
        let progress = shutdown_prompt(lane, Instant::now() + Duration::from_secs(1));
        let owner = progress.job.owner.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let reader = std::thread::spawn(move || {
            let _guard = owner.lock().unwrap();
            ready_tx.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(40));
        });
        ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        persist_shutdown_progress(vec![progress], Instant::now() + SHUTDOWN_PERSIST_GRACE);
        reader.join().unwrap();
        let status =
            PromptProgress::default().listing(&pool, "device", &pool.active_context().project_id);
        assert_eq!(status[0]["state"], "succeeded");
        let journal =
            crate::session::SessionStore::open_existing(root.join("session.jsonl")).unwrap();
        assert_eq!(
            journal
                .events()
                .iter()
                .filter(|event| event["type"] == "remote_prompt_progress_v1")
                .count(),
            1
        );
    }

    #[test]
    fn shutdown_persistence_has_one_shared_budget_and_does_not_skip_free_owner() {
        let mut roots = Vec::new();
        let mut pools = Vec::new();
        let mut owners = Vec::new();
        let mut pending = Vec::new();
        for _ in 0..8 {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().canonicalize().unwrap();
            let (pool, lane) = completed_lane(&root);
            owners.push(lane.job.owner.clone());
            pending.push(shutdown_prompt(
                lane,
                Instant::now() + Duration::from_secs(1),
            ));
            roots.push(temp);
            pools.push(pool);
        }
        let guards: Vec<_> = owners[..7]
            .iter()
            .map(|owner| owner.lock().unwrap())
            .collect();
        let started = Instant::now();
        persist_shutdown_progress(pending, Instant::now() + Duration::from_millis(50));
        assert!(
            started.elapsed() < Duration::from_millis(250),
            "lock wait multiplied per project"
        );
        for root in &roots[..7] {
            let journal =
                crate::session::SessionStore::open_existing(root.path().join("session.jsonl"))
                    .unwrap();
            assert!(journal.events().is_empty());
        }
        drop(guards);
        let free = &pools[7];
        assert_eq!(
            PromptProgress::default().listing(free, "device", &free.active_context().project_id)[0]
                ["state"],
            "succeeded"
        );
    }

    #[test]
    fn shutdown_runtime_grace_is_shared_across_noncooperative_lanes() {
        let mut roots = Vec::new();
        let mut lanes = Vec::new();
        let mut releases = Vec::new();
        let mut finished = Vec::new();
        for _ in 0..8 {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().canonicalize().unwrap();
            let (_, lane) = completed_lane(&root);
            lane.runner.shutdown_and_join().unwrap();
            let job = lane.job;
            let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
            let release = Mutex::new(release_rx);
            let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
            let (done_tx, done_rx) = std::sync::mpsc::sync_channel(1);
            let worker: Box<
                dyn Fn(PromptJob, CancellationToken) -> Result<(), crate::core::AgentError>
                    + Send
                    + Sync,
            > = Box::new(move |_, _| {
                ready_tx.send(()).unwrap();
                let _ = release.lock().unwrap().recv_timeout(Duration::from_secs(2));
                let _ = done_tx.send(());
                Ok(())
            });
            let runner = BackgroundRunner::spawn(worker, RuntimeConfig::default());
            let id = runner.try_submit(job.clone()).unwrap();
            ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
            lanes.push(ActivePrompt {
                id,
                job,
                runner,
                completion: None,
            });
            releases.push(release_tx);
            finished.push(done_rx);
            roots.push(temp);
        }
        for lane in &lanes {
            lane.runner
                .try_shutdown_with_grace(Duration::from_secs(1))
                .unwrap();
        }
        let started = Instant::now();
        let deadline = started + Duration::from_millis(50);
        let progress: Vec<_> = lanes
            .into_iter()
            .map(|lane| shutdown_prompt(lane, deadline))
            .collect();
        assert!(started.elapsed() < Duration::from_millis(250));
        assert!(
            progress
                .iter()
                .all(|item| item.state == ExecutionState::UnknownOutcome && item.code.is_none())
        );
        for release in releases {
            let _ = release.send(());
        }
        for done in finished {
            done.recv_timeout(Duration::from_secs(1)).unwrap();
        }
        persist_shutdown_progress(progress, Instant::now() + SHUTDOWN_PERSIST_GRACE);
        for root in roots {
            let journal =
                crate::session::SessionStore::open_existing(root.path().join("session.jsonl"))
                    .unwrap();
            assert_eq!(journal.events().last().unwrap()["state"], "unknown_outcome");
        }
    }

    #[test]
    fn shutdown_retains_confirmed_failure_code_without_raw_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let owner = Arc::new(Mutex::new(Agent::with_echo(
            crate::session::SessionStore::open_in_workspace(root.join("session.jsonl"), &root)
                .unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let mut request = Envelope::new(
            super::super::protocol::Channel::Control,
            Message::Request {
                operation: Operation::Prompt,
                arguments: json!({"text":"ordinary"}),
            },
        );
        request.id = Some("shutdown-failure".into());
        request.project_id = Some(project.clone());
        request.session_id = Some(pool.active_context().session_id.clone());
        let job = PromptJob {
            owner: owner.clone(),
            request,
            device: "device".into(),
            text: "ordinary".into(),
            execution: pool.subtab_execution(&project).unwrap(),
        };
        let completed = Arc::new(AtomicBool::new(false));
        let finished = completed.clone();
        let worker: Box<
            dyn Fn(PromptJob, CancellationToken) -> Result<(), crate::core::AgentError>
                + Send
                + Sync,
        > = Box::new(move |_, token| {
            // Freeze a confirmed terminal result before shutdown can cancel it.
            token.mark_completed();
            finished.store(true, Ordering::Release);
            Err(crate::core::AgentError::InvalidTurn(
                "Bearer PRIVATE_ERROR /private/host".into(),
            ))
        });
        let runner = BackgroundRunner::spawn(worker, RuntimeConfig::default());
        let id = runner.try_submit(job.clone()).unwrap();
        let until = std::time::Instant::now() + Duration::from_secs(5);
        while !completed.load(Ordering::Acquire) {
            assert!(std::time::Instant::now() < until);
            std::thread::sleep(Duration::from_millis(1));
        }
        shutdown(ActivePrompt {
            id,
            job,
            runner,
            completion: None,
        });
        drop(pool);
        drop(owner);
        let restarted = Arc::new(Mutex::new(Agent::with_echo(
            crate::session::SessionStore::open_in_workspace(root.join("session.jsonl"), &root)
                .unwrap(),
        )));
        let mut restored = ProjectOwnerPool::new(restarted).unwrap();
        restored.restore_checkpoint().unwrap();
        let statuses = PromptProgress::default().listing(&restored, "device", &project);
        assert_eq!(statuses[0]["state"], "failed");
        assert_eq!(statuses[0]["failure_code"], "invalid_request");
        assert_eq!(statuses[0]["request_id"], "shutdown-failure");
        let journal = std::fs::read_to_string(root.join("session.jsonl")).unwrap();
        assert!(!journal.contains("PRIVATE_ERROR"));
        assert!(!journal.contains("/private/host"));
        assert_eq!(
            PromptProgress::default().listing(&restored, "foreign", &project),
            json!([])
        );
    }

    #[test]
    fn shutdown_retains_success_from_a_completed_real_prompt_before_owner_poll() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let mut agent = Agent::new(
            crate::session::SessionStore::open_in_workspace(root.join("session.jsonl"), &root)
                .unwrap(),
            Box::new(crate::backend::EchoBackend),
        );
        agent.set_tools(
            crate::tools::ToolRegistry::with_all_builtins().unwrap(),
            crate::tools::ToolContext::new(&root).unwrap(),
            crate::tools::SideEffectPolicy::all_builtins(),
        );
        let owner = Arc::new(Mutex::new(agent));
        let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let mut request = Envelope::new(
            super::super::protocol::Channel::Control,
            Message::Request {
                operation: Operation::Prompt,
                arguments: json!({"text":"complete before shutdown"}),
            },
        );
        request.id = Some("shutdown-success".into());
        request.project_id = Some(project.clone());
        request.session_id = Some(pool.active_context().session_id.clone());
        let job = PromptJob {
            owner: owner.clone(),
            request,
            device: "device".into(),
            text: "complete before shutdown".into(),
            execution: pool.subtab_execution(&project).unwrap(),
        };
        let runner = prompt_runner(Arc::new(PromptProgress::default()));
        let id = runner.try_submit(job.clone()).unwrap();
        let until = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if owner
                .try_lock()
                .ok()
                .is_some_and(|agent| !agent.session().turns().is_empty())
            {
                break;
            }
            assert!(std::time::Instant::now() < until);
            std::thread::sleep(Duration::from_millis(1));
        }
        // No owner-loop event polling has occurred. Shutdown must observe
        // the terminal result instead of appending UnknownOutcome over it.
        shutdown(ActivePrompt {
            id,
            job,
            runner,
            completion: None,
        });
        let agent = owner.lock().unwrap();
        let status = agent
            .session()
            .events()
            .iter()
            .rev()
            .find(|event| event["type"] == "remote_prompt_progress_v1")
            .unwrap();
        assert_eq!(status["state"], "succeeded");
        assert_eq!(status["request_id"], "shutdown-success");
        drop(agent);
        let reopened =
            crate::session::SessionStore::open_in_workspace(root.join("session.jsonl"), &root)
                .unwrap();
        let durable = reopened
            .events()
            .iter()
            .rev()
            .find(|event| event["type"] == "remote_prompt_progress_v1")
            .unwrap();
        assert_eq!(durable["state"], "succeeded");
    }
}
