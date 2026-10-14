//! Authorized projection of the real host-owned project/session pool.

use super::{
    auth::{AuthenticatedDevice, DeviceRegistry},
    protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message, Operation},
};
use crate::project_workspace::ProjectOwnerPool;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

pub struct HostAdapter {
    source: AdapterSource,
}
enum AdapterSource {
    Shared(Arc<Mutex<ProjectOwnerPool>>),
    OwnerThread(mpsc::SyncSender<OwnerRead>),
}
struct OwnerRead {
    registry: Arc<DeviceRegistry>,
    device: AuthenticatedDevice,
    request: Envelope,
    deadline: Instant,
    reply: mpsc::SyncSender<Envelope>,
    state: Arc<AtomicU8>,
}
/// Bounded command mailbox serviced by the desktop's existing owner thread.
pub struct HostReadMailbox(mpsc::Receiver<OwnerRead>);
impl HostReadMailbox {
    pub fn drain(&self, pool: &mut ProjectOwnerPool) {
        self.drain_with(|registry, device, request| {
            HostAdapter::dispatch_owned(pool, registry, device, request)
        });
    }
    /// Claim before dispatch so an expired queued write cannot execute later.
    pub fn drain_with(
        &self,
        mut dispatch: impl FnMut(&DeviceRegistry, &AuthenticatedDevice, Envelope) -> Envelope,
    ) {
        for _ in 0..8 {
            let Ok(read) = self.0.try_recv() else { break };
            if Instant::now() < read.deadline
                && read
                    .state
                    .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            {
                let response = dispatch(&read.registry, &read.device, read.request);
                let _ = read.reply.try_send(response);
            } else {
                read.state.store(2, Ordering::Release);
            }
        }
    }
}
impl HostAdapter {
    /// Retains the actual pool; it never creates a parallel session state machine.
    pub fn new(pool: Arc<Mutex<ProjectOwnerPool>>) -> Self {
        Self {
            source: AdapterSource::Shared(pool),
        }
    }

    pub fn owner_thread() -> (Self, HostReadMailbox) {
        let (sender, receiver) = mpsc::sync_channel(8);
        (
            Self {
                source: AdapterSource::OwnerThread(sender),
            },
            HostReadMailbox(receiver),
        )
    }

    pub fn dispatch(
        &self,
        registry: &Arc<DeviceRegistry>,
        device: &AuthenticatedDevice,
        request: Envelope,
    ) -> Envelope {
        match &self.source {
            AdapterSource::Shared(pool) => match pool.try_lock() {
                Ok(mut pool) => Self::dispatch_owned(&mut pool, registry, device, request),
                Err(_) => Self::failure(&request, ErrorCode::Busy),
            },
            AdapterSource::OwnerThread(sender) => {
                if !matches!(
                    request.payload,
                    Message::Request {
                        operation: Operation::Inspect
                            | Operation::Diff
                            | Operation::TranscriptRead
                            | Operation::FilesList
                            | Operation::ResourcesList
                            | Operation::ResourcesReload
                            | Operation::TerminalOpen
                            | Operation::TerminalInput
                            | Operation::TerminalResize
                            | Operation::TerminalRead
                            | Operation::SessionNew
                            | Operation::SessionList
                            | Operation::SessionResume
                            | Operation::SessionRename
                            | Operation::ModelStatus
                            | Operation::ModelSelect
                            | Operation::ReasoningSelect
                            | Operation::WorkspaceList
                            | Operation::WorkspaceOpen
                            | Operation::WorkspaceSelect
                            | Operation::WorkspaceClose,
                        ..
                    } | Message::Request {
                        operation: Operation::SubtabList
                            | Operation::SubtabAdd
                            | Operation::SubtabClose
                            | Operation::SubtabSelect
                            | Operation::SubtabMove
                            | Operation::SubtabRename,
                        ..
                    } | Message::Request {
                        operation: Operation::SetConcurrency,
                        ..
                    } | Message::Request {
                        operation: Operation::Prompt
                            | Operation::Steer
                            | Operation::Cancel
                            | Operation::Approve,
                        ..
                    } | Message::Resume { .. }
                        | Message::Ack { .. }
                ) {
                    return Self::failure(&request, ErrorCode::InvalidRequest);
                }
                let (reply, receiver) = mpsc::sync_channel(1);
                let state = Arc::new(AtomicU8::new(0));
                let is_write = matches!(&request.payload, Message::Request { operation, .. } if operation.is_write());
                let read = OwnerRead {
                    registry: Arc::clone(registry),
                    device: device.clone(),
                    request: request.clone(),
                    deadline: Instant::now() + Duration::from_secs(2),
                    reply,
                    state: state.clone(),
                };
                if sender.try_send(read).is_err() {
                    return Self::failure(&request, ErrorCode::Busy);
                }
                receiver
                    .recv_timeout(Duration::from_secs(2))
                    .unwrap_or_else(|_| {
                        let unstarted = state
                            .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
                            .is_ok()
                            || state.load(Ordering::Acquire) == 2;
                        let mut response = Self::failure(&request, ErrorCode::Busy);
                        if is_write && !unstarted {
                            response.execution_state = Some(ExecutionState::UnknownOutcome);
                        }
                        response
                    })
            }
        }
    }

    pub fn failure(request: &Envelope, code: ErrorCode) -> Envelope {
        let mut response = Envelope::new(
            Channel::Control,
            Message::Response {
                result: Value::Null,
            },
        );
        response.id = request.id.clone();
        response.project_id = request.project_id.clone();
        response.execution_state = Some(ExecutionState::Failed);
        response.code = Some(code);
        response
    }

    /// A delivery check after dispatch cannot prove that a write had no effect.
    pub fn delivery_refusal(id: Option<String>, is_write: bool, code: ErrorCode) -> Envelope {
        let mut response = Envelope::new(
            Channel::Control,
            Message::Response {
                result: Value::Null,
            },
        );
        response.id = id;
        response.execution_state = Some(if is_write {
            ExecutionState::UnknownOutcome
        } else {
            ExecutionState::Failed
        });
        response.code = Some(code);
        response
    }

    pub fn dispatch_owned(
        pool: &mut ProjectOwnerPool,
        registry: &DeviceRegistry,
        device: &AuthenticatedDevice,
        request: Envelope,
    ) -> Envelope {
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::SessionRename,
                ..
            }
        ) {
            return super::sessions::rename(pool, registry, device, &request, false);
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::SessionNew | Operation::SessionResume,
                ..
            }
        ) {
            return super::sessions::new_session(pool, registry, device, &request, false);
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::SessionList,
                ..
            }
        ) {
            return match super::sessions::listing(pool, registry, device, &request) {
                Ok(result) => {
                    let mut response =
                        Envelope::new(Channel::Control, Message::Response { result });
                    response.id = request.id.clone();
                    response.project_id = request.project_id.clone();
                    response.session_id = request.session_id.clone();
                    response.execution_state = Some(ExecutionState::Succeeded);
                    response
                }
                Err(code) => Self::failure(&request, code),
            };
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::TerminalRead,
                ..
            }
        ) {
            return super::terminal::read(pool, registry, device, &request, None);
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::TerminalOpen,
                ..
            }
        ) {
            return super::terminal::open(pool, registry, device, &request, true, |_| {
                Err(ErrorCode::InvalidRequest)
            });
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::TerminalInput | Operation::TerminalResize,
                ..
            }
        ) {
            return super::terminal::dispatch(pool, registry, device, &request, None, false);
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::ResourcesReload,
                ..
            }
        ) {
            return super::resources::reload(pool, registry, device, &request, false);
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::ModelSelect | Operation::ReasoningSelect,
                ..
            }
        ) {
            return super::model::select(pool, registry, device, &request, false);
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::ModelStatus,
                ..
            }
        ) {
            return match super::model::status(pool, registry, device, &request) {
                Ok(result) => {
                    let mut response =
                        Envelope::new(Channel::Control, Message::Response { result });
                    response.id = request.id.clone();
                    response.project_id = request.project_id.clone();
                    response.session_id = request.session_id.clone();
                    response.execution_state = Some(ExecutionState::Succeeded);
                    response
                }
                Err(code) => Self::failure(&request, code),
            };
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::ResourcesList,
                ..
            }
        ) {
            return match super::resources::listing(pool, registry, device, &request) {
                Ok(result) => {
                    let mut response =
                        Envelope::new(Channel::Control, Message::Response { result });
                    response.id = request.id.clone();
                    response.project_id = request.project_id.clone();
                    response.session_id = request.session_id.clone();
                    response.execution_state = Some(ExecutionState::Succeeded);
                    response
                }
                Err(code) => Self::failure(&request, code),
            };
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::FilesList,
                ..
            }
        ) {
            return match super::files::listing(pool, registry, device, &request) {
                Ok(result) => {
                    let mut response =
                        Envelope::new(Channel::Control, Message::Response { result });
                    response.id = request.id.clone();
                    response.project_id = request.project_id.clone();
                    response.session_id = request.session_id.clone();
                    response.execution_state = Some(ExecutionState::Succeeded);
                    response
                }
                Err(code) => Self::failure(&request, code),
            };
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::TranscriptRead,
                ..
            }
        ) {
            return match super::transcript::read(pool, registry, device, &request) {
                Ok(result) => {
                    let mut response =
                        Envelope::new(Channel::Control, Message::Response { result });
                    response.id = request.id.clone();
                    response.project_id = request.project_id.clone();
                    response.session_id = request.session_id.clone();
                    response.execution_state = Some(ExecutionState::Succeeded);
                    response
                }
                Err(code) => Self::failure(&request, code),
            };
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::Diff,
                ..
            }
        ) {
            return match super::diff::review(pool, registry, device, &request) {
                Ok(result) => {
                    let mut response =
                        Envelope::new(Channel::Control, Message::Response { result });
                    response.id = request.id.clone();
                    response.project_id = request.project_id.clone();
                    response.session_id = request.session_id.clone();
                    response.execution_state = Some(ExecutionState::Succeeded);
                    response
                }
                Err(code) => Self::failure(&request, code),
            };
        }
        if matches!(&request.payload, Message::Ack { .. }) {
            return match super::events::acknowledge(pool, registry, device, &request) {
                Ok(result) => {
                    let mut response =
                        Envelope::new(Channel::Control, Message::Response { result });
                    response.id = request.id.clone();
                    response.project_id = request.project_id.clone();
                    response.session_id = request.session_id.clone();
                    response.execution_state = Some(ExecutionState::Succeeded);
                    response
                }
                Err(code) => Self::failure(&request, code),
            };
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::Approve,
                ..
            }
        ) {
            return super::approval::dispatch_routed(pool, registry, device, &request);
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::SubtabSelect
                    | Operation::SubtabAdd
                    | Operation::SubtabClose
                    | Operation::SubtabMove
                    | Operation::SubtabRename,
                ..
            } | Message::Request {
                operation: Operation::SetConcurrency,
                ..
            }
        ) {
            return match super::subtab::mutate(pool, registry, device, &request) {
                Ok(result) => {
                    let unknown = result.is_null();
                    let mut response =
                        Envelope::new(Channel::Control, Message::Response { result });
                    response.id = request.id.clone();
                    response.project_id = request.project_id.clone();
                    response.execution_state = Some(if unknown {
                        ExecutionState::UnknownOutcome
                    } else {
                        ExecutionState::Succeeded
                    });
                    response
                }
                Err(code) => Self::failure(&request, code),
            };
        }
        if matches!(
            &request.payload,
            Message::Request {
                operation: Operation::WorkspaceOpen
                    | Operation::WorkspaceSelect
                    | Operation::WorkspaceClose,
                ..
            }
        ) {
            return match super::workspace_api::mutate(pool, registry, device, &request) {
                Ok(result) => {
                    let unknown = result.is_null();
                    let mut response =
                        Envelope::new(Channel::Control, Message::Response { result });
                    response.id = request.id.clone();
                    response.project_id = request.project_id.clone();
                    response.execution_state = Some(if unknown {
                        ExecutionState::UnknownOutcome
                    } else {
                        ExecutionState::Succeeded
                    });
                    response
                }
                Err(code) => Self::failure(&request, code),
            };
        }
        Self::dispatch_pool(pool, registry, device, request)
    }

    fn dispatch_pool(
        pool: &ProjectOwnerPool,
        registry: &DeviceRegistry,
        device: &AuthenticatedDevice,
        request: Envelope,
    ) -> Envelope {
        let mut response = Envelope::new(
            Channel::Control,
            Message::Response {
                result: Value::Null,
            },
        );
        response.id = request.id.clone();
        response.project_id = request.project_id.clone();
        response.execution_state = Some(ExecutionState::Failed);
        let fail = |mut response: Envelope, code| {
            response.code = Some(code);
            response
        };
        if request.validate().is_err() || request.id.is_none() {
            return fail(response, ErrorCode::InvalidRequest);
        }
        if let Message::Resume { cursor, limit } = &request.payload {
            let Some(id) = request.project_id.as_deref() else {
                return fail(response, ErrorCode::InvalidRequest);
            };
            if let Err(error) = registry.authorize(device, id, Operation::Inspect) {
                return fail(response, error.remote_code());
            }
            let Some(owner) = pool.owner(id) else {
                return fail(response, ErrorCode::InvalidRequest);
            };
            let Ok(agent) = owner.try_lock() else {
                return fail(response, ErrorCode::Busy);
            };
            let page = match super::events::replay(agent.session(), id, cursor, *limit) {
                Ok(page) => page,
                Err(code) => return fail(response, code),
            };
            let mut frames = page.frames;
            if page.snapshot_required {
                let state = match Self::snapshot(&agent, id) {
                    Ok(state) => state,
                    Err(code) => return fail(response, code),
                };
                let mut snapshot = Envelope::new(
                    Channel::Bulk,
                    Message::Snapshot {
                        cursor: page.cursor.clone(),
                        state,
                    },
                );
                snapshot.project_id = Some(id.into());
                snapshot.session_id = Some(page.cursor.session_id.clone());
                frames.push(snapshot);
            }
            match registry.is_current(device) {
                Ok(true) => {}
                Ok(false) => return fail(response, ErrorCode::Unauthorized),
                Err(error) => return fail(response, error.remote_code()),
            }
            response.session_id = Some(page.cursor.session_id.clone());
            response.execution_state = Some(ExecutionState::Succeeded);
            response.payload = Message::Response {
                result: json!({"cursor": page.cursor, "cursor_scope": "transcript_events_v1",
                "has_more": page.has_more, "snapshot_required": page.snapshot_required, "journal_next_sequence": agent.session().next_sequence(),
                "phase": agent.phase(), "wire_frames": frames}),
            };
            return response;
        }
        if !matches!(request.payload, Message::Request { .. }) {
            return fail(response, ErrorCode::InvalidRequest);
        }
        if matches!(&request.payload, Message::Request { operation: Operation::SubtabList, arguments } if arguments.as_object().is_some_and(|v| v.is_empty()))
        {
            match super::subtab::listing(pool, registry, device, &request) {
                Ok(result) => {
                    response.payload = Message::Response { result };
                    response.execution_state = Some(ExecutionState::Succeeded);
                    return response;
                }
                Err(code) => return fail(response, code),
            }
        }
        // Paged worker status is owned by the live worktree controller. A
        // plain shared-pool adapter has no worker lanes, so fail closed rather
        // than silently returning the compact workspace projection.
        if matches!(&request.payload, Message::Request { operation: Operation::WorkspaceList, arguments }
            if arguments.as_object().is_some_and(|value| value.get("view").and_then(Value::as_str) == Some("worker_status")))
        {
            return fail(response, ErrorCode::InvalidRequest);
        }
        let Message::Request {
            operation,
            arguments,
        } = request.payload
        else {
            unreachable!()
        };
        if !arguments.as_object().is_some_and(|value| value.is_empty()) {
            return fail(response, ErrorCode::InvalidRequest);
        }
        let result = match operation {
            Operation::WorkspaceList => {
                if !arguments.as_object().is_some_and(|args| args.is_empty()) {
                    return fail(response, ErrorCode::InvalidRequest);
                }
                let record = match registry.lookup(device.id()) {
                    Ok(record) => record,
                    Err(error) => return fail(response, error.remote_code()),
                };
                let mut tabs = Vec::new();
                for tab in pool.workspace().tabs() {
                    let id = tab.id().as_str();
                    if record.projects.iter().any(|value| value == id) {
                        match registry.authorize(device, id, Operation::WorkspaceList) {
                            Ok(()) => {}
                            Err(super::auth::AuthError::Busy) => {
                                return fail(response, ErrorCode::Busy);
                            }
                            Err(_) => continue,
                        }
                        let Some(context) = pool.context(id) else {
                            return fail(response, ErrorCode::InternalError);
                        };
                        let pending = match super::approval::combined_listing(pool, id) {
                            Ok(pending) => pending,
                            Err(code) => return fail(response, code),
                        };
                        let allowed = |operation| match registry.authorize(device, id, operation) {
                            Ok(()) => Ok(true),
                            Err(super::auth::AuthError::Busy) => Err(ErrorCode::Busy),
                            Err(_) => Ok(false),
                        };
                        let approval_control = match allowed(Operation::Approve) {
                            Ok(value) => value,
                            Err(code) => return fail(response, code),
                        };
                        let workspace_control = match allowed(Operation::WorkspaceSelect) {
                            Ok(value) => value,
                            Err(code) => return fail(response, code),
                        };
                        let session_name =
                            pool.owner(id)
                                .and_then(|owner| {
                                    owner.try_lock().ok().map(|agent| {
                                        super::sessions::projected_name(agent.session())
                                    })
                                })
                                .flatten();
                        tabs.push(json!({"id": id, "title": crate::security::redact_text(tab.title(), &[]), "session_id": context.session_id,"session_name":session_name,
                            "pending_approvals":pending, "diff_available":true,
                            "approval_control":pool.has_workspace_checkpoint() && approval_control,
                            "workspace_control": pool.has_workspace_checkpoint() && workspace_control,
                            "model_control":pool.has_workspace_checkpoint() && workspace_control,
                            "reasoning_control":pool.has_workspace_checkpoint() && workspace_control,
                            "resources_reload_control":pool.has_workspace_checkpoint() && workspace_control,
                            "session_control":pool.has_workspace_checkpoint() && workspace_control}));
                    }
                }
                // Recheck authorization even for an empty projection: revoked/stale handles cannot succeed.
                match registry.is_current(device) {
                    Ok(true) => {}
                    Ok(false) => return fail(response, ErrorCode::Unauthorized),
                    Err(error) => return fail(response, error.remote_code()),
                }
                let active = pool
                    .workspace()
                    .active()
                    .map(|tab| tab.id().as_str())
                    .filter(|id| tabs.iter().any(|tab| tab["id"] == *id));
                json!({"projects": tabs, "active_project_id": active, "mutations_available": false,
                    "event_ack_available":pool.has_workspace_checkpoint(),
                    "event_subscription_available":true,
                    "required_owner": pool.workspace_owner(), "workspace_revision": pool.workspace_revision()})
            }
            Operation::Inspect => {
                let Some(id) = request.project_id.as_deref() else {
                    return fail(response, ErrorCode::InvalidRequest);
                };
                if let Err(error) = registry.authorize(device, id, operation) {
                    return fail(response, error.remote_code());
                }
                let Some(owner) = pool.owner(id) else {
                    return fail(response, ErrorCode::InvalidRequest);
                };
                let Ok(agent) = owner.try_lock() else {
                    return fail(response, ErrorCode::Busy);
                };
                let session_id = agent.session().session_id();
                if request
                    .session_id
                    .as_deref()
                    .is_some_and(|value| value != session_id)
                {
                    return fail(response, ErrorCode::OwnerMismatch);
                }
                response.session_id = Some(session_id.into());
                match Self::snapshot(&agent, id) {
                    Ok(state) => state,
                    Err(code) => return fail(response, code),
                }
            }
            _ => return fail(response, ErrorCode::InvalidRequest),
        };
        match registry.is_current(device) {
            Ok(true) => {}
            Ok(false) => return fail(response, ErrorCode::Unauthorized),
            Err(error) => return fail(response, error.remote_code()),
        }
        response.execution_state = Some(ExecutionState::Succeeded);
        response.payload = Message::Response {
            result: crate::security::redact_json(&result, &[]),
        };
        response
    }

    fn snapshot(agent: &crate::core::Agent, id: &str) -> Result<Value, ErrorCode> {
        let selected = if agent.session().tree_enabled() {
            let tree = agent
                .session()
                .tree_snapshot(&|| false)
                .map_err(|_| ErrorCode::InternalError)?;
            Some(
                tree.ancestry(tree.active_leaf(), &|| false)
                    .map_err(|_| ErrorCode::InternalError)?
                    .into_iter()
                    .map(|entry| entry.turn_id)
                    .collect::<std::collections::BTreeSet<_>>(),
            )
        } else {
            None
        };
        let turns = agent
            .history()
            .iter()
            .filter(|turn| selected.as_ref().is_none_or(|ids| ids.contains(&turn.id)));
        let total = turns.clone().count();
        let mut used = 0;
        let mut transcript = Vec::new();
        for turn in turns.rev().take(64) {
            let value = super::events::projected_turn(turn);
            let size = value.to_string().len();
            if used + size > 96 * 1024 {
                break;
            }
            used += size;
            transcript.push(value);
        }
        transcript.reverse();
        Ok(crate::security::redact_json(
            &json!({"project_id": id, "session_id": agent.session().session_id(), "phase": agent.phase(),
            "active_turn_id": agent.active_turn_id(), "model": agent.model(), "transcript": transcript,
            "transcript_truncated": transcript.len() < total || transcript.iter().any(|turn| turn["content_truncated"] == true), "journal_next_sequence": agent.session().next_sequence(),
            "cursor_scope": "session_journal", "event_cursor": super::events::cursor(agent.session()),
            "event_cursor_scope": "transcript_events_v1", "mutations_available": false}),
            &[],
        ))
    }

    /// Converts the private mailbox reply to bounded event/bulk frames followed by a correlated terminal response.
    pub fn wire_frames(mut response: Envelope) -> Result<Vec<Envelope>, serde_json::Error> {
        let mut frames = Vec::new();
        if let Message::Response { result } = &mut response.payload
            && let Some(object) = result.as_object_mut()
            && let Some(value) = object.remove("wire_frames")
        {
            frames = serde_json::from_value(value)?;
        }
        frames.push(response);
        Ok(frames)
    }
}
