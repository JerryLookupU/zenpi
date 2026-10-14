//! Bounded execution observation; completed results survive owner contention.

use crate::remote::protocol::{Envelope, ErrorCode, ExecutionState};
use crate::{
    core::{AgentError, ProcessResult},
    project_workspace::ProjectOwnerPool,
    runtime::{JobId, JobOutcome, RuntimeEvent},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const PROGRESS: &str = "remote_worker_progress_v1";
const MAX_TRACKED: usize = 128;
const MAX_OUTPUT_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalTaskStatus {
    pub device: String,
    pub request: String,
    pub state: ExecutionState,
    pub runtime_active: bool,
}

pub const LOCAL_STATUS_LIMIT: usize = 32;

fn output_projection(text: &str) -> Value {
    if text.len() > 64 * 1024 || text.contains('\0') {
        return json!({"output_withheld":true});
    }
    // Redact the whole bounded result before slicing, including private PEM
    // bodies and assignments whose secret extends past the display boundary.
    let mut safe = crate::security::redact_text(text, &[]);
    let truncated = safe.len() > MAX_OUTPUT_BYTES;
    let mut end = safe.len().min(MAX_OUTPUT_BYTES);
    while !safe.is_char_boundary(end) {
        end -= 1;
    }
    safe.truncate(end);
    json!({"output_text":safe,"output_truncated":truncated})
}

struct Pending {
    job: JobId,
    request: Envelope,
    device: String,
    value: Option<Value>,
    terminal: bool,
    dirty: bool,
    runtime_done: bool,
}

#[derive(Default)]
pub struct WorkerProgress {
    pending: BTreeMap<u64, Pending>,
    scope: Option<(String, String, String, u64, u64)>,
}

impl WorkerProgress {
    pub(crate) fn local_status(
        &self,
        project: &str,
        session: &str,
        subtab: &str,
    ) -> Vec<LocalTaskStatus> {
        let mut values = self.pending.values().rev().filter(|pending| pending.request.project_id.as_deref() == Some(project)
            && pending.request.session_id.as_deref() == Some(session)
            && matches!(&pending.request.payload, crate::remote::protocol::Message::Request { arguments, .. } if arguments["subtab_id"] == subtab))
            .map(|pending| LocalTaskStatus { device: pending.device.clone(), request: pending.request.id.clone().unwrap_or_default(),
                state: pending.value.as_ref().and_then(|value| serde_json::from_value(value["state"].clone()).ok()).unwrap_or(ExecutionState::Accepted),
                runtime_active: !pending.runtime_done }).collect::<Vec<_>>();
        values.sort_by_key(|task| !task.runtime_active);
        values.truncate(LOCAL_STATUS_LIMIT);
        values
    }
    pub(crate) fn active_jobs(&self) -> impl Iterator<Item = JobId> + '_ {
        self.pending
            .values()
            .filter(|pending| !pending.runtime_done)
            .map(|pending| pending.job)
    }

    pub(crate) fn identity(&self, job: JobId) -> Option<(&Envelope, &str)> {
        self.pending
            .get(&job.get())
            .map(|pending| (&pending.request, pending.device.as_str()))
    }
    pub fn is_full(&self) -> bool {
        self.pending.len() >= MAX_TRACKED
    }

    pub(crate) fn matches_scope(&self, scope: (&str, &str, &str, u64), instance: u64) -> bool {
        self.scope.as_ref().is_none_or(|previous| {
            previous.0 == scope.0
                && previous.1 == scope.1
                && previous.2 == scope.2
                && previous.3 <= scope.3
                && previous.4 == instance
        })
    }

    pub(crate) fn bind_scope(&mut self, scope: (&str, &str, &str, u64), instance: u64) -> bool {
        let scope = (
            scope.0.into(),
            scope.1.into(),
            scope.2.into(),
            scope.3,
            instance,
        );
        match &self.scope {
            Some(previous)
                if previous.0 == scope.0
                    && previous.1 == scope.1
                    && previous.2 == scope.2
                    && previous.3 <= scope.3
                    && previous.4 == scope.4 =>
            {
                self.scope = Some(scope);
                true
            }
            Some(_) => false,
            None => {
                self.scope = Some(scope);
                true
            }
        }
    }

    pub(crate) fn track(&mut self, job: JobId, request: Envelope, device: String) {
        self.pending.insert(
            job.get(),
            Pending {
                job,
                request,
                device,
                value: None,
                terminal: false,
                dirty: false,
                runtime_done: false,
            },
        );
    }

    pub fn observe(&mut self, event: RuntimeEvent<ProcessResult, AgentError>, shutting_down: bool) {
        use sha2::{Digest, Sha256};
        let history_digest = match &event {
            RuntimeEvent::Completed {
                outcome: JobOutcome::Succeeded(result),
                ..
            } => result.worker_history_evidence.clone(),
            _ => None,
        };
        let input_turn = match &event {
            RuntimeEvent::Completed {
                outcome: JobOutcome::Succeeded(result),
                ..
            } => result.submission.turn_id().map(str::to_owned),
            _ => None,
        };
        let output_digest = match &event {
            RuntimeEvent::Completed {
                outcome: JobOutcome::Succeeded(result),
                ..
            } => result
                .assistant
                .as_ref()
                .map(|turn| format!("{:x}", Sha256::digest(turn.content.as_bytes()))),
            _ => None,
        };
        let output_turn = match &event {
            RuntimeEvent::Completed {
                outcome: JobOutcome::Succeeded(result),
                ..
            } => result.assistant.as_ref().map(|turn| turn.id.clone()),
            _ => None,
        };
        let output = match &event {
            RuntimeEvent::Completed {
                outcome: JobOutcome::Succeeded(result),
                ..
            } => result
                .assistant
                .as_ref()
                .map(|turn| output_projection(&turn.content)),
            _ => None,
        };
        let (id, state, code, terminal) = match event {
            RuntimeEvent::Accepted { id, .. } | RuntimeEvent::Queued { id, .. } => {
                (id, ExecutionState::Accepted, None, false)
            }
            RuntimeEvent::Started { id } => (id, ExecutionState::Running, None, false),
            RuntimeEvent::Completed { id, outcome } => {
                let (state, code) = match outcome {
                    JobOutcome::Succeeded(_) => (ExecutionState::Succeeded, None),
                    JobOutcome::Failed(error) => (
                        ExecutionState::Failed,
                        Some(crate::remote::command::failure_code(&error)),
                    ),
                    JobOutcome::Panicked => {
                        (ExecutionState::Failed, Some(ErrorCode::InternalError))
                    }
                    JobOutcome::Cancelled if shutting_down => {
                        (ExecutionState::UnknownOutcome, None)
                    }
                    JobOutcome::Cancelled => (ExecutionState::Cancelled, None),
                };
                (id, state, code, true)
            }
            RuntimeEvent::Rejected { id, .. } => {
                (id, ExecutionState::Failed, Some(ErrorCode::Busy), true)
            }
            _ => return,
        };
        let Some(pending) = self.pending.get_mut(&id.get()) else {
            return;
        };
        pending.runtime_done |= terminal;
        if pending.terminal {
            return;
        }
        let mut value = json!({"type":PROGRESS,"device_id":pending.device,
            "request_id":pending.request.id,"project_id":pending.request.project_id,
            "session_id":pending.request.session_id,"state":state});
        if let crate::remote::protocol::Message::Request { arguments, .. } =
            &pending.request.payload
        {
            value["subtab_id"] = arguments["subtab_id"].clone();
        }
        if let Some(code) = code {
            value["failure_code"] = json!(code);
        }
        if let Some(output) = output {
            value
                .as_object_mut()
                .unwrap()
                .extend(output.as_object().unwrap().clone());
        }
        if let Some(turn) = output_turn {
            value["output_turn_id"] = json!(turn);
            value["request_fingerprint"] = json!(format!(
                "{:x}",
                Sha256::digest(
                    serde_json::to_vec(&pending.request).expect("typed worker request serializes")
                )
            ));
            value["output_digest"] = json!(output_digest);
            value["input_turn_id"] = json!(input_turn);
            if let Some(evidence) = history_digest {
                value["history_digest"] = json!(evidence.digest);
                value["worker_slot"] = json!(evidence.slot);
                value["worker_session_id"] = json!(evidence.session);
            }
        }
        pending.value = Some(value);
        pending.terminal = terminal;
        pending.dirty = true;
    }

    /// Retry only lock contention. A failed append is ambiguous and not repeated.
    pub fn flush(&mut self, pool: &ProjectOwnerPool) {
        self.pending.retain(|_, pending| {
            if !pending.dirty {
                return true;
            }
            let Some(value) = pending.value.as_ref() else {
                return true;
            };
            let Some(project) = pending.request.project_id.as_deref() else {
                return false;
            };
            let Some(owner) = pool.owner(project) else {
                return false;
            };
            let mut agent = match owner.try_lock() {
                Ok(agent) => agent,
                Err(std::sync::TryLockError::WouldBlock) => return true,
                Err(_) => return false,
            };
            if Some(agent.session().session_id()) != pending.request.session_id.as_deref() {
                return false;
            }
            let saved = agent.session_mut().append_event(value.clone()).is_ok();
            pending.dirty = false;
            if saved {
                crate::remote::wakeup::notify(agent.session().session_id());
            } else {
                if let Some(value) = &mut pending.value {
                    value["state"] = json!(ExecutionState::UnknownOutcome);
                    value.as_object_mut().unwrap().remove("failure_code");
                    for field in [
                        "output_text",
                        "output_truncated",
                        "output_withheld",
                        "output_turn_id",
                        "request_fingerprint",
                        "output_digest",
                        "input_turn_id",
                        "history_digest",
                        "worker_slot",
                        "worker_session_id",
                    ] {
                        value.as_object_mut().unwrap().remove(field);
                    }
                }
                pending.terminal = true;
                return true;
            }
            saved && !pending.terminal
        });
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub(crate) fn finished(&self) -> bool {
        self.pending
            .values()
            .all(|pending| pending.runtime_done && pending.terminal && !pending.dirty)
    }

    pub(crate) fn has_target(&self, request: &Envelope, device: Option<&str>) -> bool {
        let crate::remote::protocol::Message::Request { arguments, .. } = &request.payload else {
            return false;
        };
        self.pending.values().any(|pending| {
            !pending.runtime_done
                && pending.request.id.as_deref()
                    == arguments.get("request_id").and_then(Value::as_str)
                && pending.request.project_id == request.project_id
                && pending.request.session_id == request.session_id
                && device.is_none_or(|device| pending.device == device)
        })
    }

    pub(crate) fn finish_shutdown(&mut self) {
        let events: Vec<_> = self
            .pending
            .values()
            .filter(|pending| !pending.terminal)
            .map(|pending| RuntimeEvent::Completed {
                id: pending.job,
                outcome: JobOutcome::Cancelled,
            })
            .collect();
        for event in events {
            self.observe(event, true);
        }
        for pending in self.pending.values_mut() {
            pending.runtime_done = true;
        }
    }

    /// Exact receipt queries work without a live target; a fresh cancellation
    /// requires the original device/request and this runner generation.
    pub fn cancel<I: Send + 'static>(
        &self,
        pool: &mut ProjectOwnerPool,
        registry: &crate::remote::auth::DeviceRegistry,
        device: &crate::remote::auth::AuthenticatedDevice,
        request: &Envelope,
        runner: &super::worktree_runner::WorktreeRunner<I, ProcessResult, AgentError>,
    ) -> Envelope {
        let target = match &request.payload {
            crate::remote::protocol::Message::Request {
                operation: crate::remote::protocol::Operation::Cancel,
                arguments,
            } => arguments.get("request_id").and_then(Value::as_str),
            _ => None,
        };
        let candidates = || {
            self.pending.values().filter(|pending| {
                !pending.runtime_done
                    && pending.request.id.as_deref() == target
                    && pending.request.project_id == request.project_id
                    && self.scope.is_some()
                    && self.matches_scope(runner.scope(), runner.instance())
            })
        };
        // Request IDs are device-scoped; a different device can use the same
        // string without shadowing the caller's own live request.
        let active = candidates()
            .find(|pending| pending.device == device.id())
            .or_else(|| candidates().next());
        crate::remote::command::CancelAdmissions.dispatch(
            pool,
            registry,
            device,
            request,
            active.map(|pending| (&pending.request, pending.device.as_str())),
            || {
                let pending = active.ok_or(ErrorCode::InvalidRequest)?;
                runner.try_cancel(pending.job).map_err(|_| ErrorCode::Busy)
            },
        )
    }

    pub fn listing(&self, pool: &ProjectOwnerPool, device: &str, project: &str) -> Value {
        self.listing_inner(pool, device, project, true)
    }

    pub(crate) fn live_listing(
        &self,
        pool: &ProjectOwnerPool,
        device: &str,
        project: &str,
    ) -> Value {
        self.listing_inner(pool, device, project, false)
    }

    pub(crate) fn paged_listing(
        &self,
        pool: &ProjectOwnerPool,
        device: &str,
        project: &str,
    ) -> Value {
        self.listing_bounded(pool, device, project, false, MAX_TRACKED)
    }

    fn listing_inner(
        &self,
        pool: &ProjectOwnerPool,
        device: &str,
        project: &str,
        include_durable: bool,
    ) -> Value {
        self.listing_bounded(pool, device, project, include_durable, 16)
    }

    fn listing_bounded(
        &self,
        pool: &ProjectOwnerPool,
        device: &str,
        project: &str,
        include_durable: bool,
        limit: usize,
    ) -> Value {
        let mut values = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        let mut pending = self
            .pending
            .values()
            .filter(|pending| {
                pending.device == device
                    && pending.request.project_id.as_deref() == Some(project)
                    && pool.context(project).is_some_and(|context| {
                        pending.request.session_id.as_deref() == Some(&context.session_id)
                    })
            })
            .collect::<Vec<_>>();
        pending.sort_by_key(|pending| {
            (
                pending.runtime_done,
                if pending.runtime_done {
                    u64::MAX - pending.job.get()
                } else {
                    pending.job.get()
                },
            )
        });
        for pending in pending {
            // Admission is already tracked even before the first runtime event is drained.
            let mut projected = pending.value.clone().unwrap_or_else(|| {
                let subtab = match &pending.request.payload {
                    crate::remote::protocol::Message::Request { arguments, .. } => arguments["subtab_id"].clone(),
                    _ => Value::Null,
                };
                json!({"request_id":pending.request.id,"project_id":pending.request.project_id,
                    "session_id":pending.request.session_id,"subtab_id":subtab,"state":ExecutionState::Accepted})
            });
            if !pending.runtime_done {
                projected["runtime_active"] = json!(true);
            }
            projected.as_object_mut().unwrap().remove("type");
            projected.as_object_mut().unwrap().remove("device_id");
            projected.as_object_mut().unwrap().remove("output_turn_id");
            projected.as_object_mut().unwrap().remove("output_digest");
            projected.as_object_mut().unwrap().remove("input_turn_id");
            projected.as_object_mut().unwrap().remove("history_digest");
            projected.as_object_mut().unwrap().remove("worker_slot");
            projected
                .as_object_mut()
                .unwrap()
                .remove("worker_session_id");
            projected
                .as_object_mut()
                .unwrap()
                .remove("request_fingerprint");
            seen.insert(projected["request_id"].as_str().unwrap().to_owned());
            values.push(projected);
            if values.len() == limit {
                return Value::Array(values);
            }
        }
        if include_durable && let Value::Array(durable) = listing(pool, device, project) {
            for value in durable {
                if seen.insert(value["request_id"].as_str().unwrap().to_owned()) {
                    values.push(value);
                }
                if values.len() == 16 {
                    break;
                }
            }
        }
        Value::Array(values)
    }
}

/// Current durable execution display, separate from immutable admission receipts.
pub(crate) fn local_durable_status(
    pool: &ProjectOwnerPool,
    project: &str,
    subtab: &str,
) -> Result<Vec<LocalTaskStatus>, ErrorCode> {
    let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
    let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
    let agent = owner.try_lock().map_err(|_| ErrorCode::Busy)?;
    if agent.session().session_id() != context.session_id {
        return Err(ErrorCode::OwnerMismatch);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut values = Vec::new();
    for event in agent.session().events().iter().rev().filter(|event| {
        event["project_id"] == project
            && event["session_id"] == context.session_id
            && event["subtab_id"] == subtab
            && (event["type"] == PROGRESS || event["type"] == "remote_worker_admission_v1")
    }) {
        let (Some(device), Some(request)) =
            (event["device_id"].as_str(), event["request_id"].as_str())
        else {
            continue;
        };
        if !crate::remote::auth::valid_identifier(device)
            || !crate::remote::auth::valid_identifier(request)
            || !seen.insert((device.to_owned(), request.to_owned()))
        {
            continue;
        }
        let state = match event["state"].as_str() {
            Some("succeeded") => ExecutionState::Succeeded,
            Some("failed") => ExecutionState::Failed,
            Some("cancelled") => ExecutionState::Cancelled,
            _ => ExecutionState::UnknownOutcome,
        };
        values.push(LocalTaskStatus {
            device: device.into(),
            request: request.into(),
            state,
            runtime_active: false,
        });
        if values.len() == LOCAL_STATUS_LIMIT {
            break;
        }
    }
    Ok(values)
}

pub fn listing(pool: &ProjectOwnerPool, device: &str, project: &str) -> Value {
    durable_listing(pool, device, project, 16).unwrap_or_else(|_| json!([]))
}

/// Read the durable journal without turning lock contention into an empty,
/// apparently authoritative task list. Callers serving a wire response must
/// propagate `Busy`; the legacy local display wrapper above intentionally keeps
/// its historical best-effort shape.
pub(crate) fn durable_listing(
    pool: &ProjectOwnerPool,
    device: &str,
    project: &str,
    limit: usize,
) -> Result<Value, ErrorCode> {
    let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
    let agent = owner.try_lock().map_err(|_| ErrorCode::Busy)?;
    if !pool
        .context(project)
        .is_some_and(|context| context.session_id == agent.session().session_id())
    {
        return Err(ErrorCode::OwnerMismatch);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut values = Vec::new();
    for event in agent.session().events().iter().rev().filter(|event| {
        event["device_id"] == device
            && (event["type"] == PROGRESS || event["type"] == "remote_worker_admission_v1")
    }) {
        let Some(id) = event["request_id"].as_str() else {
            continue;
        };
        if !crate::remote::auth::valid_identifier(id)
            || event["project_id"] != project
            || event["session_id"] != agent.session().session_id()
        {
            continue;
        }
        if !seen.insert(id.to_owned()) {
            continue;
        }
        let state = match event["state"].as_str() {
            Some("succeeded") => ExecutionState::Succeeded,
            Some("failed") => ExecutionState::Failed,
            Some("cancelled") => ExecutionState::Cancelled,
            // A durable running/admission record after restart is not proof of live work.
            _ => ExecutionState::UnknownOutcome,
        };
        let mut value = json!({"request_id":id,"project_id":project,"session_id":agent.session().session_id(),
            "subtab_id":event["subtab_id"],"state":state});
        if state == ExecutionState::Failed
            && let Ok(code) = serde_json::from_value::<ErrorCode>(event["failure_code"].clone())
        {
            value["failure_code"] = json!(code);
        }
        if state == ExecutionState::Succeeded {
            if event["output_withheld"] == true {
                value["output_withheld"] = json!(true);
            } else if let Some(text) = event["output_text"].as_str()
                && let Some(truncated) = event["output_truncated"].as_bool()
            {
                let output = output_projection(text);
                value
                    .as_object_mut()
                    .unwrap()
                    .extend(output.as_object().unwrap().clone());
                if value.get("output_text").is_some() {
                    value["output_truncated"] =
                        json!(truncated || output["output_truncated"] == true);
                }
            }
        }
        values.push(value);
        if values.len() == limit {
            break;
        }
    }
    Ok(Value::Array(values))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        core::Agent,
        runtime::{BackgroundRunner, RuntimeConfig},
        session::SessionStore,
    };
    use std::sync::{Arc, Mutex};

    #[test]
    fn completed_output_is_redacted_before_utf8_truncation_and_unsafe_bodies_are_withheld() {
        let raw = "summary\nAPI_KEY=PRIVATE_WORKER_SECRET\n-----BEGIN PRIVATE KEY-----\nPRIVATE_KEY_BODY\n-----END PRIVATE KEY-----\nend";
        let output = output_projection(raw);
        let safe = output["output_text"].as_str().unwrap();
        assert!(safe.contains("summary"));
        assert!(safe.contains("end"));
        assert!(!safe.contains("PRIVATE_WORKER_SECRET"));
        assert!(!safe.contains("PRIVATE_KEY_BODY"));
        assert_eq!(output["output_truncated"], false);
        let boundary = format!(
            "{}API_KEY={}\n",
            "x".repeat(4080),
            "BOUNDARY_PRIVATE_SECRET".repeat(20)
        );
        let output = output_projection(&boundary);
        assert!(!output.to_string().contains("BOUNDARY_PRIVATE_SECRET"));
        assert!(output["output_text"].as_str().unwrap().len() <= MAX_OUTPUT_BYTES);
        let output = output_projection(&"\u{754c}".repeat(2000));
        assert_eq!(output["output_truncated"], true);
        assert_eq!(output["output_text"].as_str().unwrap().len(), 4095);
        for body in ["bad\0body".to_owned(), "x".repeat(65537)] {
            assert_eq!(output_projection(&body), json!({"output_withheld":true}));
        }
    }

    #[test]
    fn ambiguous_progress_does_not_prove_runtime_completion() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let pool = ProjectOwnerPool::new(master.clone()).unwrap();
        let project = pool.active_context().project_id.clone();
        let runner =
            BackgroundRunner::spawn(|_: (), _| Ok::<_, String>(()), RuntimeConfig::default());
        let id = runner.try_submit(()).unwrap();
        let mut req = Envelope::new(
            crate::remote::protocol::Channel::Control,
            crate::remote::protocol::Message::Request {
                operation: crate::remote::protocol::Operation::Prompt,
                arguments: json!({"subtab_id":"tab"}),
            },
        );
        req.id = Some("held-runtime".into());
        req.project_id = Some(project);
        req.session_id = Some(pool.active_context().session_id.clone());
        let mut progress = WorkerProgress::default();
        progress.track(id, req, "device".into());
        progress.observe(RuntimeEvent::Started { id }, false);
        let path = master.lock().unwrap().session().path().to_owned();
        std::fs::rename(&path, path.with_extension("original")).unwrap();
        std::fs::write(&path, "foreign\n").unwrap();
        progress.flush(&pool);
        let local = progress.local_status(
            pool.active_context().project_id.as_str(),
            pool.active_context().session_id.as_str(),
            "tab",
        );
        assert_eq!(local.len(), 1);
        assert_eq!(local[0].state, ExecutionState::UnknownOutcome);
        assert!(local[0].runtime_active);
        let wire =
            progress.live_listing(&pool, "device", pool.active_context().project_id.as_str());
        assert_eq!(wire[0]["runtime_active"], true);
        assert_eq!(wire[0]["state"], "unknown_outcome");
        assert_eq!(
            progress.active_jobs().collect::<Vec<_>>(),
            vec![id],
            "ambiguous persistence still has a cancellable runtime job"
        );
        assert!(
            !progress.finished(),
            "failed journal append is not a runtime terminal event"
        );
        progress.observe(
            RuntimeEvent::Completed {
                id,
                outcome: JobOutcome::Cancelled,
            },
            false,
        );
        assert!(progress.finished());
        assert!(progress.active_jobs().next().is_none());
        assert!(
            progress.live_listing(&pool, "device", pool.active_context().project_id.as_str())[0]
                .get("runtime_active")
                .is_none()
        );
        assert_eq!(
            progress
                .pending
                .values()
                .next()
                .unwrap()
                .value
                .as_ref()
                .unwrap()["state"],
            "unknown_outcome"
        );
        runner.shutdown_and_join().unwrap();
    }

    #[test]
    fn local_durable_status_is_bounded_scoped_and_never_claims_restored_running_work() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("status.jsonl"), &root).unwrap(),
        )));
        let pool = ProjectOwnerPool::new(master.clone()).unwrap();
        let context = pool.active_context().clone();
        for index in 0..40 {
            master.lock().unwrap().session_mut().append_event(json!({"type":PROGRESS,"device_id":"phone-a","request_id":format!("task-{index}"),
                "project_id":context.project_id,"session_id":context.session_id,"subtab_id":"tab","state":"running","output_text":"PRIVATE_OUTPUT"})).unwrap();
        }
        for (device, scope, session, state) in [
            ("phone-a", "tab", context.session_id.as_str(), "succeeded"),
            ("phone-b", "tab", context.session_id.as_str(), "cancelled"),
            ("phone-c", "sibling", context.session_id.as_str(), "failed"),
            ("phone-d", "tab", "foreign-session", "succeeded"),
        ] {
            master.lock().unwrap().session_mut().append_event(json!({"type":PROGRESS,"device_id":device,"request_id":"same-id",
                "project_id":context.project_id,"session_id":session,"subtab_id":scope,"state":state})).unwrap();
        }
        let before = std::fs::read(master.lock().unwrap().session().path()).unwrap();
        let values = local_durable_status(&pool, &context.project_id, "tab").unwrap();
        assert_eq!(values.len(), LOCAL_STATUS_LIMIT);
        assert_eq!(values[0].device, "phone-b");
        assert_eq!(values[0].state, ExecutionState::Cancelled);
        assert_eq!(values[1].device, "phone-a");
        assert_eq!(values[1].state, ExecutionState::Succeeded);
        assert!(values[2..].iter().all(|value| value.state == ExecutionState::UnknownOutcome && !value.runtime_active));
        assert!(!format!("{values:?}").contains("PRIVATE_OUTPUT"));
        assert!(
            values
                .iter()
                .all(|value| value.device != "phone-c" && value.device != "phone-d")
        );
        let guard = master.lock().unwrap();
        assert_eq!(
            local_durable_status(&pool, &context.project_id, "tab"),
            Err(ErrorCode::Busy)
        );
        drop(guard);
        assert_eq!(
            std::fs::read(master.lock().unwrap().session().path()).unwrap(),
            before
        );
    }

    #[test]
    fn rejected_cancelled_failed_and_shutdown_unknown_are_distinct_and_terminal_once() {
        for (mode, expected, code) in [
            (0, "failed", Some("busy")),
            (1, "cancelled", None),
            (2, "unknown_outcome", None),
            (3, "failed", Some("invalid_request")),
            (4, "failed", Some("internal_error")),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().canonicalize().unwrap();
            let master = Arc::new(Mutex::new(Agent::with_echo(
                SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
            )));
            let pool = ProjectOwnerPool::new(master.clone()).unwrap();
            let project = pool.active_context().project_id.clone();
            let runner =
                BackgroundRunner::spawn(|_: (), _| Ok::<_, String>(()), RuntimeConfig::default());
            let id = runner.try_submit(()).unwrap();
            let mut req = Envelope::new(
                crate::remote::protocol::Channel::Control,
                crate::remote::protocol::Message::Request {
                    operation: crate::remote::protocol::Operation::Prompt,
                    arguments: json!({"subtab_id":"tab"}),
                },
            );
            req.id = Some("observed".into());
            req.project_id = Some(project.clone());
            req.session_id = Some(pool.active_context().session_id.clone());
            let mut progress = WorkerProgress::default();
            progress.track(id, req, "device".into());
            let event = match mode {
                0 => RuntimeEvent::Rejected {
                    id,
                    reason: crate::runtime::SubmitError::QueueFull,
                },
                1 | 2 => RuntimeEvent::Completed {
                    id,
                    outcome: JobOutcome::Cancelled,
                },
                3 => RuntimeEvent::Completed {
                    id,
                    outcome: JobOutcome::Failed(AgentError::InvalidTurn(
                        "RAW_PRIVATE_ERROR".into(),
                    )),
                },
                _ => RuntimeEvent::Completed {
                    id,
                    outcome: JobOutcome::Panicked,
                },
            };
            progress.observe(event, mode == 2);
            progress.observe(RuntimeEvent::Started { id }, false);
            assert_eq!(
                progress.listing(&pool, "device", &project)[0]["state"],
                expected
            );
            progress.flush(&pool);
            assert_eq!(progress.pending_count(), 0);
            let listed = listing(&pool, "device", &project);
            assert_eq!(listed[0]["state"], expected);
            assert_eq!(listed[0]["failure_code"].as_str(), code);
            assert!(!listed.to_string().contains("RAW_PRIVATE_ERROR"));
            let before = std::fs::read(master.lock().unwrap().session().path()).unwrap();
            progress.observe(RuntimeEvent::Started { id }, false);
            progress.flush(&pool);
            assert_eq!(
                std::fs::read(master.lock().unwrap().session().path()).unwrap(),
                before
            );
            runner.shutdown_and_join().unwrap();
        }
    }

    #[test]
    fn tracker_scope_and_retention_cap_do_not_hide_a_second_runner() {
        let mut progress = WorkerProgress::default();
        assert!(progress.bind_scope(("project", "session", "tab", 1), 1));
        assert!(progress.bind_scope(("project", "session", "tab", 1), 1));
        assert!(!progress.bind_scope(("project", "session", "tab", 1), 2));
        assert!(!progress.bind_scope(("project", "session", "other", 1), 1));
        assert!(progress.bind_scope(("project", "session", "tab", 2), 1));
        assert!(!progress.bind_scope(("project", "session", "tab", 1), 1));
        assert!(!progress.bind_scope(("project", "foreign", "tab", 3), 1));
        let runner = BackgroundRunner::spawn(
            |_: (), _| Ok::<_, String>(()),
            RuntimeConfig {
                command_capacity: 256,
                event_capacity: 512,
                max_pending: 256,
                ..RuntimeConfig::default()
            },
        );
        for index in 0..MAX_TRACKED {
            let id = runner.try_submit(()).unwrap();
            let mut req = Envelope::new(
                crate::remote::protocol::Channel::Control,
                crate::remote::protocol::Message::Response {
                    result: Value::Null,
                },
            );
            req.id = Some(index.to_string());
            progress.track(id, req, "device".into());
        }
        assert!(progress.is_full());
        assert_eq!(progress.pending_count(), MAX_TRACKED);
        runner.shutdown_and_join().unwrap();
    }

    #[test]
    fn local_status_retains_live_jobs_before_recent_terminal_entries() {
        let runner = BackgroundRunner::spawn(
            |_: (), _| Ok::<_, String>(()),
            RuntimeConfig {
                command_capacity: 256,
                event_capacity: 512,
                max_pending: 256,
                ..RuntimeConfig::default()
            },
        );
        let mut progress = WorkerProgress::default();
        for index in 0..40 {
            let id = runner.try_submit(()).unwrap();
            let mut request = Envelope::new(
                crate::remote::protocol::Channel::Control,
                crate::remote::protocol::Message::Request {
                    operation: crate::remote::protocol::Operation::Prompt,
                    arguments: json!({"subtab_id":"tab"}),
                },
            );
            request.id = Some(format!("task-{index}"));
            request.project_id = Some("project".into());
            request.session_id = Some("session".into());
            progress.track(id, request, "device".into());
            if index > 0 {
                progress.observe(
                    RuntimeEvent::Completed {
                        id,
                        outcome: JobOutcome::Cancelled,
                    },
                    false,
                );
            }
        }
        let values = progress.local_status("project", "session", "tab");
        assert_eq!(values.len(), LOCAL_STATUS_LIMIT);
        assert_eq!(values[0].request, "task-0");
        assert!(values[0].runtime_active);
        assert_eq!(values[1].request, "task-39");
        assert!(!values[1].runtime_active);
        assert!(
            progress
                .local_status("foreign", "session", "tab")
                .is_empty()
        );
        runner.shutdown_and_join().unwrap();
    }

    #[test]
    fn durable_listing_reports_contention_instead_of_fabricating_empty_state() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let pool = ProjectOwnerPool::new(master).unwrap();
        let project = pool.active_context().project_id.clone();
        let owner = pool.owner(&project).unwrap();
        let held = owner.lock().unwrap();
        assert_eq!(
            durable_listing(&pool, "device", &project, 16),
            Err(ErrorCode::Busy)
        );
        drop(held);
        assert!(durable_listing(&pool, "device", &project, 16).is_ok());
    }

    #[test]
    fn phone_status_prioritizes_live_tasks_and_shows_admission_before_event_drain() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("status.jsonl"), &root).unwrap(),
        )));
        let pool = ProjectOwnerPool::new(master.clone()).unwrap();
        let context = pool.active_context().clone();
        let runner = BackgroundRunner::spawn(
            |_: (), _| Ok::<_, String>(()),
            RuntimeConfig {
                command_capacity: 256,
                event_capacity: 512,
                max_pending: 256,
                ..RuntimeConfig::default()
            },
        );
        let mut progress = WorkerProgress::default();
        for index in 0..40 {
            let id = runner.try_submit(()).unwrap();
            let mut request = Envelope::new(
                crate::remote::protocol::Channel::Control,
                crate::remote::protocol::Message::Request {
                    operation: crate::remote::protocol::Operation::Prompt,
                    arguments: json!({"subtab_id":"tab","text":"PRIVATE_PROMPT"}),
                },
            );
            request.id = Some(format!("task-{index}"));
            request.project_id = Some(context.project_id.clone());
            request.session_id = Some(context.session_id.clone());
            progress.track(id, request, "device".into());
            if index < 38 {
                progress.observe(
                    RuntimeEvent::Completed {
                        id,
                        outcome: JobOutcome::Cancelled,
                    },
                    false,
                );
            }
        }
        let before = std::fs::read(master.lock().unwrap().session().path()).unwrap();
        let wire = progress.live_listing(&pool, "device", &context.project_id);
        let statuses = wire.as_array().unwrap();
        assert_eq!(statuses.len(), 16);
        assert_eq!(statuses[0]["request_id"], "task-38");
        assert_eq!(statuses[1]["request_id"], "task-39");
        for status in &statuses[..2] {
            assert_eq!(status["state"], "accepted");
            assert_eq!(status["runtime_active"], true);
        }
        assert_eq!(statuses[2]["request_id"], "task-37");
        assert_eq!(statuses[2]["state"], "cancelled");
        assert!(!wire.to_string().contains("PRIVATE_PROMPT"));
        assert!(
            progress
                .live_listing(&pool, "foreign", &context.project_id)
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            std::fs::read(master.lock().unwrap().session().path()).unwrap(),
            before
        );
        runner.shutdown_and_join().unwrap();
    }
}
