//! Host-quota execution over independently leased subtab worker owners.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use super::worker::{WorkerClaim, WorkerOwner};
use crate::{
    project_workspace::ProjectOwnerPool,
    runtime::{
        BackgroundRunner, CancellationToken, ConcurrencyError, JobId, RuntimeConfig, RuntimeEvent,
        SubmitError,
    },
};

type Job<I, O, E> = Box<dyn Fn(I, CancellationToken) -> Result<O, E> + Send + Sync>;

pub(crate) struct WorkerPrompt {
    pub text: String,
    pub request: super::protocol::Envelope,
    pub device: String,
}

impl WorktreeRunner<WorkerPrompt, crate::core::ProcessResult, crate::core::AgentError> {
    pub(crate) fn spawn_observed_prompts(
        pool: &mut ProjectOwnerPool,
        project: &str,
        subtab: &str,
        config: RuntimeConfig,
        live: Arc<super::live_text::LivePromptText>,
    ) -> Result<Self, String> {
        Self::spawn(
            pool,
            project,
            subtab,
            config,
            move |worker, job: WorkerPrompt, token| {
                let mut agent = worker.agent().lock().map_err(|_| {
                    crate::core::AgentError::InvalidTurn("worker owner lock poisoned".into())
                })?;
                let history_start = agent.session().turns().len();
                let mut result = worker.execution().run_cancelable(
                    &mut agent,
                    &|| token.is_cancelled(),
                    |agent| {
                        let input = super::command::prompt_input(agent, job.text)?;
                        agent.process_with_cancel_and_events(
                            input,
                            || token.is_cancelled(),
                            &mut |event| {
                                live.update(&job.request, &job.device, &event);
                                Ok(())
                            },
                        )
                    },
                )??;
                if let Some(turn) = &result.assistant {
                    use sha2::{Digest, Sha256};
                    let history = agent.session().turns()[history_start..]
                        .iter()
                        .map(super::transcript::history_entry)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| {
                            crate::core::AgentError::InvalidTurn(
                                "worker history encoding failed".into(),
                            )
                        })?;
                    let history_digest = format!(
                        "{:x}",
                        Sha256::digest(serde_json::to_vec(&history).map_err(|_| {
                            crate::core::AgentError::InvalidTurn(
                                "worker history encoding failed".into(),
                            )
                        })?)
                    );
                    let binding = serde_json::json!({"type":"remote_worker_output_v1",
                        "device_id":job.device,"request_id":job.request.id,
                        "project_id":job.request.project_id,"session_id":job.request.session_id,
                        "subtab_id":worker.execution().tab.id,
                        "worker_session_id":agent.session().session_id(),"output_turn_id":turn.id,
                        "worker_key":agent.session().path().file_stem().and_then(|name| name.to_str()),
                        "input_turn_id":result.submission.turn_id(),"history":history,
                        "history_digest":history_digest,
                        "worker_slot":worker.slot(),
                        "request_fingerprint":format!("{:x}",Sha256::digest(serde_json::to_vec(&job.request).map_err(|_| crate::core::AgentError::InvalidTurn("worker identity encoding failed".into()))?)),
                        "output_digest":format!("{:x}",Sha256::digest(turn.content.as_bytes()))});
                    agent.session_mut().append_event(binding)?;
                    result.worker_history_evidence = Some(super::worker::HistoryEvidence {
                        digest: history_digest,
                        slot: worker.slot(),
                        session: agent.session().session_id().into(),
                    });
                }
                token.mark_completed();
                Ok(result)
            },
        )
    }
}

struct Slots {
    state: Mutex<SlotState>,
}

struct SlotState {
    workers: Vec<WorkerOwner>,
    free: Vec<usize>,
    // Retained by job closures, including noncooperative detached jobs.
    _claims: Vec<WorkerClaim>,
}

struct Slot {
    slots: Arc<Slots>,
    index: usize,
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.slots
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .free
            .push(self.index);
    }
}

pub struct WorktreeRunner<I: Send + 'static, O: Send + 'static, E: Send + 'static> {
    runner: BackgroundRunner<I, O, E, Job<I, O, E>>,
    workers: Vec<WorkerOwner>,
    slots: Arc<Slots>,
    project: String,
    session: String,
    subtab: String,
    subtab_revision: u64,
    instance: u64,
    configured_quota: u16,
    pending_quota: Option<PendingQuota>,
}

struct PendingQuota {
    device: String,
    request: String,
    fingerprint: String,
    limit: u16,
    revision: u64,
    applied: std::sync::mpsc::Receiver<()>,
    acknowledged: bool,
}

impl WorktreeRunner<String, crate::core::ProcessResult, crate::core::AgentError> {
    /// Run host-owned text turns through the existing Agent/tool pipeline.
    pub fn spawn_prompts(
        pool: &mut ProjectOwnerPool,
        project: &str,
        subtab: &str,
        config: RuntimeConfig,
    ) -> Result<Self, String> {
        Self::spawn(
            pool,
            project,
            subtab,
            config,
            |worker, text: String, token| {
                if text.trim().is_empty()
                    || text.len() > 64 * 1024
                    || text.contains('\0')
                    || text.trim_start().starts_with('!')
                {
                    return Err(crate::core::AgentError::InvalidTurn(
                        "invalid worker prompt".into(),
                    ));
                }
                let mut agent = worker.agent().lock().map_err(|_| {
                    crate::core::AgentError::InvalidTurn("worker owner lock poisoned".into())
                })?;
                let result = worker.execution().run_cancelable(
                    &mut agent,
                    &|| token.is_cancelled(),
                    |agent| {
                        let input = super::command::prompt_input(agent, text)?;
                        agent.process_with_cancel(input, || token.is_cancelled())
                    },
                )??;
                token.mark_completed();
                Ok(result)
            },
        )
    }
}

impl<I: Send + 'static, O: Send + 'static, E: Send + 'static> WorktreeRunner<I, O, E> {
    /// Capture the durable host quota. No client-provided worker count/path.
    /// Live execution limits can vary within this captured, leased capacity;
    /// changing durable configuration or growing ownership needs a host commit.
    pub fn spawn(
        pool: &mut ProjectOwnerPool,
        project: &str,
        subtab: &str,
        config: RuntimeConfig,
        job: impl Fn(WorkerOwner, I, CancellationToken) -> Result<O, E> + Send + Sync + 'static,
    ) -> Result<Self, String> {
        let state = pool.subtab_state(project)?;
        let session = pool
            .context(project)
            .ok_or("unknown runner project")?
            .session_id
            .clone();
        let quota = state
            .tabs
            .iter()
            .find(|tab| tab.id == subtab)
            .ok_or("unknown runner subtab")?
            .concurrency;
        if !pool.worker_capacity_available(project, subtab, 0, quota)? {
            return Err("worker owner capacity exhausted".into());
        }
        let mut workers = Vec::new();
        let mut claims = Vec::new();
        for slot in 0..quota {
            let worker = pool.worker_owner(project, subtab, slot)?;
            claims.push(worker.claim_runtime()?);
            workers.push(worker);
        }
        let slots = Arc::new(Slots {
            state: Mutex::new(SlotState {
                free: (0..workers.len()).rev().collect(),
                workers: workers.clone(),
                _claims: claims,
            }),
        });
        let job_slots = slots.clone();
        let job: Job<I, O, E> = Box::new(move |input, token| {
            let (index, worker) = {
                let mut state = job_slots
                    .state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                let index = state
                    .free
                    .pop()
                    .expect("runtime quota exceeds independent worker slots");
                (index, state.workers[index].clone())
            };
            let slot = Slot {
                slots: job_slots.clone(),
                index,
            };
            let result = job(worker, input, token);
            drop(slot);
            result
        });
        let runner = BackgroundRunner::spawn_with_concurrency(job, config, quota)
            .map_err(|error| error.to_string())?;
        static NEXT_INSTANCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let instance = NEXT_INSTANCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(Self {
            runner,
            workers,
            slots,
            project: project.into(),
            session,
            subtab: subtab.into(),
            subtab_revision: state.revision,
            instance,
            configured_quota: quota,
            pending_quota: None,
        })
    }

    pub(crate) fn instance(&self) -> u64 {
        self.instance
    }

    pub fn scope(&self) -> (&str, &str, &str, u64) {
        (
            &self.project,
            &self.session,
            &self.subtab,
            self.subtab_revision,
        )
    }

    /// A sibling quota commit advances project-wide subtab revision without
    /// changing this lane. Never refresh through a pending intent or a changed
    /// execution root/kind/session/configured quota.
    pub(crate) fn refresh_unchanged_scope(
        &mut self,
        pool: &ProjectOwnerPool,
    ) -> Result<(), String> {
        if self.pending_quota.is_some() {
            return Err("quota intent owns the captured revision".into());
        }
        if pool
            .context(&self.project)
            .map(|context| context.session_id.as_str())
            != Some(self.session.as_str())
        {
            return Err("runner master session changed".into());
        }
        let state = pool.subtab_state(&self.project)?;
        let tab = state
            .tabs
            .iter()
            .find(|tab| tab.id == self.subtab)
            .ok_or("runner tab removed")?;
        let captured = self
            .workers
            .first()
            .ok_or("runner has no workers")?
            .execution();
        if state.revision < self.subtab_revision
            || tab.root != captured.tab.root
            || tab.kind != captured.tab.kind
            || tab.concurrency != self.configured_quota
        {
            return Err("runner execution or quota changed".into());
        }
        captured.validate()?;
        self.subtab_revision = state.revision;
        Ok(())
    }

    pub fn quota(&self) -> u16 {
        self.workers.len() as u16
    }

    pub fn configured_quota(&self) -> u16 {
        self.configured_quota
    }
    pub fn quota_change_pending(&self) -> bool {
        self.pending_quota.is_some()
    }
    pub(crate) fn pending_quota_matches(
        &self,
        device: &str,
        request: &str,
        fingerprint: &str,
    ) -> bool {
        self.pending_quota.as_ref().is_some_and(|pending| {
            pending.device == device
                && pending.request == request
                && pending.fingerprint == fingerprint
        })
    }

    /// Last scheduler-applied limit, which can be below the durable leased
    /// quota while existing jobs drain. This is not an active-job count.
    pub fn applied_concurrency(&self) -> u16 {
        self.runner.applied_concurrency()
    }

    /// Host-only runtime adjustment within already leased independent owners.
    /// Lowering preserves running jobs and does not release their claims;
    /// raising reuses those owners. This does not commit a workspace receipt.
    pub fn try_set_concurrency(
        &self,
        limit: u16,
    ) -> Result<std::sync::mpsc::Receiver<()>, ConcurrencyError> {
        if self.pending_quota.is_some() {
            return Err(ConcurrencyError::Submit(SubmitError::QueueFull));
        }
        if limit > self.quota() {
            return Err(ConcurrencyError::InvalidLimit);
        }
        self.runner.try_set_concurrency(limit)
    }

    pub fn workers(&self) -> &[WorkerOwner] {
        &self.workers
    }

    /// Prepare growth only from an unresolved durable host intent. This
    /// expands ownership capacity, not applied quota, scope or configuration.
    pub fn grow_reserved(
        &mut self,
        pool: &mut ProjectOwnerPool,
        device: &str,
        request: &str,
        fingerprint: &str,
    ) -> Result<(), String> {
        if self.pending_quota.is_some() {
            return Err("quota change already pending".into());
        }
        let added = pool.claim_quota_growth(
            device,
            request,
            fingerprint,
            self.scope(),
            self.configured_quota,
            self.quota(),
        )?;
        let mut state = self
            .slots
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for (worker, claim) in added {
            let index = state.workers.len();
            state.workers.push(worker.clone());
            state._claims.push(claim);
            state.free.push(index);
            self.workers.push(worker);
        }
        Ok(())
    }

    /// Start one reserved live change. New task submission is fenced until
    /// scheduler acknowledgement and the matching checkpoint commit settle.
    pub fn begin_reserved_quota(
        &mut self,
        pool: &mut ProjectOwnerPool,
        device: &str,
        request: &str,
        fingerprint: &str,
    ) -> Result<(), String> {
        if self.pending_quota.is_some() {
            return Err("quota change already pending".into());
        }
        let candidate = pool.quota_candidate(
            device,
            request,
            fingerprint,
            self.scope(),
            self.configured_quota,
        )?;
        let limit = candidate
            .tabs
            .iter()
            .find(|tab| tab.id == self.subtab)
            .ok_or("missing quota target")?
            .concurrency;
        if limit > self.quota() {
            self.grow_reserved(pool, device, request, fingerprint)?;
        }
        let applied = self
            .runner
            .try_set_concurrency(limit)
            .map_err(|error| error.to_string())?;
        self.pending_quota = Some(PendingQuota {
            device: device.into(),
            request: request.into(),
            fingerprint: fingerprint.into(),
            limit,
            revision: candidate.revision,
            applied,
            acknowledged: false,
        });
        Ok(())
    }

    /// Poll without consuming worker events. A checkpoint error keeps the
    /// intent and submission fence; callers must not treat it as no effect.
    pub fn poll_reserved_quota(
        &mut self,
        pool: &mut ProjectOwnerPool,
    ) -> Result<Option<serde_json::Value>, String> {
        let Some(pending) = self.pending_quota.as_mut() else {
            return Ok(None);
        };
        if !pending.acknowledged {
            match pending.applied.try_recv() {
                Ok(()) => pending.acknowledged = true,
                Err(std::sync::mpsc::TryRecvError::Empty) => return Ok(None),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return Err("quota application unconfirmed; intent remains unknown".into());
                }
            }
        }
        if self.runner.applied_concurrency() != pending.limit {
            return Err("scheduler quota no longer matches intent".into());
        }
        let (device, request, fingerprint, revision, limit) = (
            pending.device.clone(),
            pending.request.clone(),
            pending.fingerprint.clone(),
            pending.revision,
            pending.limit,
        );
        let value = pool.commit_runtime_quota(
            &device,
            &request,
            &fingerprint,
            self.scope(),
            self.configured_quota,
            &self.workers,
        )?;
        self.subtab_revision = revision;
        self.configured_quota = limit;
        self.pending_quota = None;
        Ok(Some(value))
    }

    pub fn try_submit(&self, input: I) -> Result<JobId, SubmitError> {
        if self.pending_quota.is_some() {
            return Err(SubmitError::QueueFull);
        }
        self.runner.try_submit(input)
    }

    pub fn try_cancel(&self, id: JobId) -> Result<(), SubmitError> {
        self.runner.try_cancel(id)
    }

    pub fn try_shutdown_with_grace(&self, grace: Duration) -> Result<(), SubmitError> {
        self.runner.try_shutdown_with_grace(grace)
    }

    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<RuntimeEvent<O, E>, std::sync::mpsc::RecvTimeoutError> {
        self.runner.recv_timeout(timeout)
    }

    pub fn try_next_event(&self) -> Result<RuntimeEvent<O, E>, std::sync::mpsc::TryRecvError> {
        self.runner.try_next_event()
    }

    pub fn shutdown_and_join(self) -> std::thread::Result<()> {
        self.runner.shutdown_and_join()
    }

    pub fn shutdown_and_join_with_events(
        self,
        grace: Duration,
        observe: impl FnMut(RuntimeEvent<O, E>),
    ) -> std::thread::Result<()> {
        self.runner.shutdown_and_join_with_events(grace, observe)
    }
}
