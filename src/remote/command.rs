//! Durable admission receipts for requests sent to the existing desktop runner.

use super::{
    auth::{AuthenticatedDevice, DeviceRegistry},
    protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message, Operation},
};
use crate::{core::Agent, project_workspace::ProjectOwnerPool};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

const MAX_PROMPT_BYTES: usize = 64 * 1024;
const MAX_RECEIPTS: usize = 128;
const RECEIPT_TYPE: &str = "remote_prompt_admission_v1";
const PROGRESS_TYPE: &str = "remote_prompt_progress_v1";

pub fn failure_code(error: &crate::core::AgentError) -> ErrorCode {
    use crate::core::AgentError;
    match error {
        AgentError::InvalidTurn(_) | AgentError::Closed | AgentError::NoActiveTurn => {
            ErrorCode::InvalidRequest
        }
        AgentError::NotIdle => ErrorCode::Busy,
        AgentError::ToolLoopLimit(_) | AgentError::Governance(_) => ErrorCode::ResourceLimit,
        _ => ErrorCode::InternalError,
    }
}

/// Remote input must not consume attachments staged by the local composer.
pub fn prompt_input(
    agent: &Agent,
    text: String,
) -> Result<crate::core::TurnInputRequest, crate::core::AgentError> {
    validate_remote_file_references(&text).map_err(crate::core::AgentError::InvalidTurn)?;
    if !agent.pending_attachments().is_empty() {
        return Err(crate::core::AgentError::InvalidTurn(
            "local staged attachments require local submission or explicit clearing".into(),
        ));
    }
    let attachments = crate::slash_actions::prompt_attachments(agent, &text)?;
    Ok(crate::core::TurnInputRequest::new(text)
        .with_mode(crate::protocol::TurnMode::StartIfIdle)
        .with_attachments(attachments))
}

pub(crate) fn validate_remote_file_references(text: &str) -> Result<(), String> {
    // Slash resources are expanded by the host, not parsed as raw attachment text here.
    if text.starts_with('/') {
        return Ok(());
    }
    // Malformed ordinary references retain the existing runner-side validation.
    let Ok(references) = crate::slash_actions::prompt_file_references(text) else {
        return Ok(());
    };
    for (_, _, path, _) in references {
        if crate::tools::credential_path(std::path::Path::new(&path)) {
            return Err("Private workspace files cannot be attached remotely".into());
        }
    }
    Ok(())
}

struct PendingSteer {
    device: String,
    request: Envelope,
    fingerprint: String,
    target: String,
    input_id: String,
    ticket: Option<crate::input_queue::InputTicket>,
    durable: bool,
}

#[derive(Default)]
pub struct SteerAdmissions(Vec<PendingSteer>);
impl SteerAdmissions {
    /// Observe the existing session writer without blocking the desktop owner thread.
    pub fn poll(&mut self, pool: &mut ProjectOwnerPool) {
        self.0.retain_mut(|pending| {
            if !pending.durable {
                match pending
                    .ticket
                    .as_ref()
                    .expect("unobserved steering ticket")
                    .try_recv()
                {
                    Err(std::sync::mpsc::TryRecvError::Empty) => return true,
                    Ok(Ok(crate::input_queue::InputQueueReply::Input { input, .. }))
                        if input.id == pending.input_id
                            && input.kind == crate::input_queue::InputKind::Steer =>
                    {
                        pending.durable = true;
                        pending.ticket = None;
                    }
                    // Journal failure may be ambiguous. Never turn it into permission to resubmit.
                    _ => return false,
                }
            }
            pool.commit_steer_receipt(
                &pending.device,
                &pending.request,
                &pending.fingerprint,
                &pending.target,
                ExecutionState::Accepted,
                None,
            )
            .is_err()
        });
    }

    pub fn dispatch(
        &mut self,
        pool: &mut ProjectOwnerPool,
        registry: &DeviceRegistry,
        device: &AuthenticatedDevice,
        request: &Envelope,
        active: Option<(&Envelope, &str)>,
    ) -> Envelope {
        self.poll(pool);
        let mut response =
            super::host_adapter::HostAdapter::failure(request, ErrorCode::InvalidRequest);
        let result = (|| {
            if request.validate().is_err()
                || request.turn_id.is_some()
                || request.sequence.is_some()
            {
                return Err(ErrorCode::InvalidRequest);
            }
            let Message::Request {
                operation: Operation::Steer,
                arguments,
            } = &request.payload
            else {
                return Err(ErrorCode::InvalidRequest);
            };
            let values = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
            let target = values
                .get("request_id")
                .and_then(Value::as_str)
                .ok_or(ErrorCode::InvalidRequest)?;
            let text = values
                .get("text")
                .and_then(Value::as_str)
                .ok_or(ErrorCode::InvalidRequest)?;
            if values.len() != 2
                || !super::auth::valid_identifier(target)
                || text.trim().is_empty()
                || text.len() > MAX_PROMPT_BYTES
                || text.contains('\0')
                || text.trim_start().starts_with('!')
            {
                return Err(ErrorCode::InvalidRequest);
            }
            let project = request
                .project_id
                .as_deref()
                .ok_or(ErrorCode::InvalidRequest)?;
            let id = request.id.as_deref().ok_or(ErrorCode::InvalidRequest)?;
            let fingerprint = format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(request).map_err(|_| ErrorCode::InternalError)?)
            );
            registry
                .with_authorized_projects(device, &[project], Operation::Steer, |_| {
                    if let Some(receipt) = pool.steer_receipt(device.id(), id, &fingerprint)? {
                        return Ok(receipt);
                    }
                    crate::slash_actions::plain_queue_input(text)
                        .map_err(|_| ErrorCode::InvalidRequest)?;
                    if request.required_owner.as_deref() != Some(pool.workspace_owner()) {
                        return Err(ErrorCode::OwnerMismatch);
                    }
                    if request.expected_revision != Some(pool.workspace_revision()) {
                        return Err(ErrorCode::RevisionConflict);
                    }
                    let Some((original, owner)) = active else {
                        return Err(ErrorCode::InvalidRequest);
                    };
                    if owner != device.id()
                        || original.id.as_deref() != Some(target)
                        || original.project_id != request.project_id
                        || original.session_id != request.session_id
                        || !pool.context(project).is_some_and(|context| {
                            request.session_id.as_deref() == Some(context.session_id.as_str())
                        })
                    {
                        return Err(ErrorCode::OwnerMismatch);
                    }
                    if self.0.len() >= 32 {
                        return Err(ErrorCode::ResourceLimit);
                    }
                    let port = pool
                        .control(project)
                        .ok_or(ErrorCode::OwnerMismatch)?
                        .input_port;
                    if request.session_id.as_deref() != Some(port.session_id().as_str()) {
                        return Err(ErrorCode::OwnerMismatch);
                    }
                    if pool
                        .commit_steer_receipt(
                            device.id(),
                            request,
                            &fingerprint,
                            target,
                            ExecutionState::UnknownOutcome,
                            None,
                        )
                        .is_err()
                    {
                        return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
                    }
                    // A deterministic, device-scoped queue ID is bound to this request, never to a replacement turn.
                    let input_id = format!(
                        "{:x}",
                        Sha256::digest(format!("remote-steer-v1\n{}\n{id}", device.id()))
                    );
                    let queued = crate::protocol::InputQueueRequest {
                        schema_version: crate::protocol::PROTOCOL_VERSION,
                        id: input_id.clone(),
                        kind: "input_queue".into(),
                        session_id: request
                            .session_id
                            .clone()
                            .ok_or(ErrorCode::InvalidRequest)?,
                        input_queue: crate::protocol::InputQueueAction::Enqueue {
                            input_id: input_id.clone(),
                            kind: crate::input_queue::InputKind::Steer,
                            text: text.into(),
                        },
                    };
                    match port.submit(queued) {
                        Ok(ticket) => self.0.push(PendingSteer {
                            device: device.id().into(),
                            request: request.clone(),
                            fingerprint,
                            target: target.into(),
                            input_id,
                            ticket: Some(ticket),
                            durable: false,
                        }),
                        Err(_) => {
                            if pool
                                .commit_steer_receipt(
                                    device.id(),
                                    request,
                                    &fingerprint,
                                    target,
                                    ExecutionState::Failed,
                                    Some(ErrorCode::Busy),
                                )
                                .is_ok()
                            {
                                return Ok((
                                    ExecutionState::Failed,
                                    Value::Null,
                                    Some(ErrorCode::Busy),
                                ));
                            }
                        }
                    }
                    Ok((ExecutionState::UnknownOutcome, Value::Null, None))
                })
                .map_err(|error| error.remote_code())?
        })();
        match result {
            Ok((state, result, code)) => {
                response.execution_state = Some(state);
                response.code = code;
                response.payload = Message::Response { result };
            }
            Err(code) => response.code = Some(code),
        }
        response
    }
}

#[derive(Default)]
pub struct CancelAdmissions;
impl CancelAdmissions {
    pub fn dispatch(
        &mut self,
        pool: &mut ProjectOwnerPool,
        registry: &DeviceRegistry,
        device: &AuthenticatedDevice,
        request: &Envelope,
        active: Option<(&Envelope, &str)>,
        cancel: impl FnOnce() -> Result<(), ErrorCode>,
    ) -> Envelope {
        let mut response =
            super::host_adapter::HostAdapter::failure(request, ErrorCode::InvalidRequest);
        let result = (|| {
            if request.validate().is_err()
                || request.turn_id.is_some()
                || request.sequence.is_some()
            {
                return Err(ErrorCode::InvalidRequest);
            }
            let Message::Request {
                operation: Operation::Cancel,
                arguments,
            } = &request.payload
            else {
                return Err(ErrorCode::InvalidRequest);
            };
            let values = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
            let target = values
                .get("request_id")
                .and_then(Value::as_str)
                .ok_or(ErrorCode::InvalidRequest)?;
            if values.len() != 1 || !super::auth::valid_identifier(target) {
                return Err(ErrorCode::InvalidRequest);
            }
            let project = request
                .project_id
                .as_deref()
                .ok_or(ErrorCode::InvalidRequest)?;
            let id = request.id.as_deref().ok_or(ErrorCode::InvalidRequest)?;
            let fingerprint = format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(request).map_err(|_| ErrorCode::InternalError)?)
            );
            registry.with_authorized_projects(device, &[project], Operation::Cancel, |_| {
                if let Some(receipt) = pool.cancel_receipt(device.id(), id, &fingerprint)? { return Ok(receipt); }
                if request.required_owner.as_deref() != Some(pool.workspace_owner()) { return Err(ErrorCode::OwnerMismatch); }
                if request.expected_revision != Some(pool.workspace_revision()) { return Err(ErrorCode::RevisionConflict); }
                if !pool.context(project).is_some_and(|context| request.session_id.as_deref() == Some(context.session_id.as_str())) { return Err(ErrorCode::OwnerMismatch); }
                let Some((original, owner)) = active else { return Err(ErrorCode::InvalidRequest); };
                if owner != device.id() || original.id.as_deref() != Some(target) || original.project_id != request.project_id
                    || original.session_id != request.session_id { return Err(ErrorCode::OwnerMismatch); }
                // Persist ambiguity before the side effect: neither restart nor retry may cancel a replacement job.
                if pool.commit_cancel_receipt(device.id(), request, &fingerprint, target, ExecutionState::UnknownOutcome, None).is_err() {
                    return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
                }
                let (state, code) = match cancel() {
                    Ok(()) => (ExecutionState::Accepted, None),
                    Err(ErrorCode::Busy) => (ExecutionState::Failed, Some(ErrorCode::Busy)),
                    Err(_) => return Ok((ExecutionState::UnknownOutcome, Value::Null, None)),
                };
                if pool.commit_cancel_receipt(device.id(), request, &fingerprint, target, state, code).is_err() {
                    return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
                }
                Ok((state, if state == ExecutionState::Accepted { json!({"request_id":id,"target_request_id":target,"project_id":project,"session_id":request.session_id}) } else { Value::Null }, code))
            }).map_err(|error| error.remote_code())?
        })();
        match result {
            Ok((state, result, code)) => {
                response.execution_state = Some(state);
                response.code = code;
                response.payload = Message::Response { result };
            }
            Err(code) => response.code = Some(code),
        }
        response
    }
}

#[derive(Default)]
pub struct PromptProgress(
    Mutex<std::collections::VecDeque<(String, Value)>>,
    super::live_text::LivePromptText,
    super::live_text::LiveDesktopText,
);

impl PromptProgress {
    pub fn desktop_text(&self) -> &super::live_text::LiveDesktopText {
        &self.2
    }
    pub fn provider_event(
        &self,
        request: &Envelope,
        device: &str,
        event: &crate::backend::ProviderEvent,
    ) {
        self.1.update(request, device, event);
    }
    pub fn record(&self, request: &Envelope, device: &str, state: ExecutionState) {
        self.record_with_code(request, device, state, None)
    }
    pub fn record_with_code(
        &self,
        request: &Envelope,
        device: &str,
        state: ExecutionState,
        code: Option<ErrorCode>,
    ) {
        if !matches!(state, ExecutionState::Accepted | ExecutionState::Running) {
            self.1.remove(request, device);
        }
        if request
            .id
            .as_deref()
            .is_none_or(|id| !super::auth::valid_identifier(id))
            || request.project_id.is_none()
            || request.session_id.is_none()
        {
            return;
        }
        let mut values = self.0.lock().unwrap_or_else(|error| error.into_inner());
        values.retain(|(owner, value)| {
            !(owner == device
                && value["request_id"].as_str() == request.id.as_deref()
                && value["project_id"].as_str() == request.project_id.as_deref()
                && value["session_id"].as_str() == request.session_id.as_deref())
        });
        if values.len() >= MAX_RECEIPTS {
            // Display retention may evict terminal records, never deduplication receipts or active work.
            let Some(index) = values.iter().position(|(_, value)| {
                !matches!(value["state"].as_str(), Some("accepted" | "running"))
            }) else {
                return;
            };
            values.remove(index);
        }
        let mut value = progress_value(request, state);
        if state == ExecutionState::Failed
            && let Some(code) = code
        {
            value["failure_code"] = json!(code);
        }
        values.push_back((device.to_owned(), value));
        if let Some(session) = request.session_id.as_deref() {
            super::wakeup::notify(session);
        }
    }
    pub fn listing(&self, pool: &ProjectOwnerPool, device: &str, project: &str) -> Value {
        let live = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let Some(context) = pool.context(project) else {
            return json!([]);
        };
        let session = &context.session_id;
        let mut values = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        let active =
            |value: &Value| matches!(value["state"].as_str(), Some("accepted" | "running"));
        for (owner, value) in live
            .iter()
            .rev()
            .filter(|(_, value)| active(value))
            .chain(live.iter().rev().filter(|(_, value)| !active(value)))
        {
            if owner == device
                && value["project_id"] == project
                && value["session_id"] == *session
                && let Some(id) = value["request_id"].as_str()
                && seen.insert(id.to_owned())
            {
                let mut projected = value.clone();
                if active(value)
                    && let Some(preview) = self.1.snapshot(device, project, session, id)
                {
                    projected["stream_text"] = Value::String(preview.text);
                    if preview.truncated {
                        projected["stream_truncated"] = Value::Bool(true);
                    }
                }
                values.push(projected);
                if values.len() == 16 {
                    return Value::Array(values);
                }
            }
        }
        if let Some(owner) = pool.owner(project)
            && let Ok(agent) = owner.try_lock()
            && agent.session().session_id() == session
        {
            for event in agent
                .session()
                .events()
                .iter()
                .rev()
                .filter(|event| event["device_id"] == device)
            {
                if event["type"] == RECEIPT_TYPE || event["type"] == PROGRESS_TYPE {
                    if event["type"] == PROGRESS_TYPE
                        && (event["project_id"] != project || event["session_id"] != *session)
                    {
                        continue;
                    }
                    if let Some(id) = event["request_id"].as_str()
                        && super::auth::valid_identifier(id)
                        && seen.insert(id.to_owned())
                    {
                        let state = match event["state"].as_str() {
                            Some("succeeded") => "succeeded",
                            Some("failed") => "failed",
                            Some("cancelled") => "cancelled",
                            _ => "unknown_outcome",
                        };
                        let mut value = json!({"request_id":id,"project_id":project,"session_id":session,"state":state});
                        if state == "failed"
                            && let Ok(code) =
                                serde_json::from_value::<ErrorCode>(event["failure_code"].clone())
                        {
                            value["failure_code"] = json!(code);
                        }
                        values.push(value);
                        if values.len() == 16 {
                            break;
                        }
                    }
                }
            }
        }
        Value::Array(values)
    }
}

fn progress_value(request: &Envelope, state: ExecutionState) -> Value {
    json!({"request_id":request.id,"project_id":request.project_id,"session_id":request.session_id,"state":state})
}

pub fn persist_progress(
    agent: &mut Agent,
    request: &Envelope,
    device: &str,
    state: ExecutionState,
) -> Result<(), crate::session::SessionError> {
    persist_progress_with_code(agent, request, device, state, None)
}
pub fn persist_progress_with_code(
    agent: &mut Agent,
    request: &Envelope,
    device: &str,
    state: ExecutionState,
    code: Option<ErrorCode>,
) -> Result<(), crate::session::SessionError> {
    let mut value = progress_value(request, state);
    if state == ExecutionState::Failed
        && let Some(code) = code
    {
        value["failure_code"] = json!(code);
    }
    value["type"] = json!(PROGRESS_TYPE);
    value["device_id"] = json!(device);
    agent.session_mut().append_event(value)
}

pub fn dispatch_prompt(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    busy: bool,
    submit: impl FnOnce(
        Arc<Mutex<Agent>>,
        String,
        super::subtab::SubtabExecution,
    ) -> Result<String, ErrorCode>,
) -> Envelope {
    let mut response =
        super::host_adapter::HostAdapter::failure(request, ErrorCode::InvalidRequest);
    let result = (|| {
        if request.validate().is_err() || request.turn_id.is_some() || request.sequence.is_some() {
            return Err(ErrorCode::InvalidRequest);
        }
        let Message::Request {
            operation: Operation::Prompt,
            arguments,
        } = &request.payload
        else {
            return Err(ErrorCode::InvalidRequest);
        };
        let values = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
        let text = values
            .get("text")
            .and_then(Value::as_str)
            .ok_or(ErrorCode::InvalidRequest)?;
        if values.len() != 1
            || text.trim().is_empty()
            || text.len() > MAX_PROMPT_BYTES
            || text.trim_start().starts_with('!')
        {
            return Err(ErrorCode::InvalidRequest);
        }
        let project = request
            .project_id
            .as_deref()
            .ok_or(ErrorCode::InvalidRequest)?;
        let request_id = request.id.as_deref().ok_or(ErrorCode::InvalidRequest)?;
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(request).map_err(|_| ErrorCode::InternalError)?)
        );
        registry
            .with_authorized_projects(device, &[project], Operation::Prompt, |_| {
                let bound = pool.prompt_binding(device.id(), request_id, &fingerprint)?;
                let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
                let mut agent = match owner.try_lock() {
                    Ok(agent) => agent,
                    Err(_) => return Ok((ExecutionState::UnknownOutcome, Value::Null, None)),
                };
                if request.session_id.as_deref() != Some(agent.session().session_id()) {
                    return Err(ErrorCode::OwnerMismatch);
                }
                let receipts: Vec<_> = agent
                    .session()
                    .events()
                    .iter()
                    .filter(|event| event["type"] == RECEIPT_TYPE)
                    .collect();
                if let Some(event) = receipts.iter().rev().find(|event| {
                    event["device_id"] == device.id() && event["request_id"] == request_id
                }) {
                    if event["fingerprint"] != fingerprint {
                        return Err(ErrorCode::RequestIdConflict);
                    }
                    let state = match event["state"].as_str() {
                        Some("accepted") => ExecutionState::Accepted,
                        Some("failed") => ExecutionState::Failed,
                        _ => ExecutionState::UnknownOutcome,
                    };
                    return Ok((
                        state,
                        event["result"].clone(),
                        (state == ExecutionState::Failed).then_some(ErrorCode::Busy),
                    ));
                }
                if receipts.len() >= MAX_RECEIPTS * 2 {
                    return Err(ErrorCode::ResourceLimit);
                }
                // A checkpoint reservation without a journal admission is unresolved, never resubmitted.
                if bound {
                    return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
                }
                validate_remote_file_references(text).map_err(|_|ErrorCode::InvalidRequest)?;
                if request.required_owner.as_deref() != Some(pool.workspace_owner()) {
                    return Err(ErrorCode::OwnerMismatch);
                }
                if request.expected_revision != Some(pool.workspace_revision()) {
                    return Err(ErrorCode::RevisionConflict);
                }
                if busy
                    || agent.phase() != crate::core::AgentPhase::Idle
                    || !agent.pending_attachments().is_empty()
                    || agent.input_port().has_pending()
                    || agent.input_port().is_preparing()
                {
                    return Err(ErrorCode::Busy);
                }
                let execution = pool
                    .subtab_execution(project)
                    .map_err(|_| ErrorCode::InvalidRequest)?;
                execution
                    .validate()
                    .map_err(|_| ErrorCode::InvalidRequest)?;
                let event = |state: &str, result: Value| {
                    json!({"type":RECEIPT_TYPE,"device_id":device.id(),
                "request_id":request_id,"fingerprint":fingerprint,"state":state,"result":result,
                "execution_subtab":{"id":execution.tab.id,"kind":execution.tab.kind,"root":execution.tab.root}})
                };
                // Bind across operations/projects before journal reservation or runner side effects.
                if pool
                    .bind_remote_prompt(device.id(), request, &fingerprint)
                    .is_err()
                {
                    return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
                }
                // Reserve before handing work to the runner. A crash here is unresolved, never auto-resubmitted.
                if agent
                    .session_mut()
                    .append_event(event("reserved", Value::Null))
                    .is_err()
                {
                    return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
                }
                let submission = submit(Arc::clone(&owner), text.to_owned(), execution.clone());
                let (state, result, code) = match submission {
                    Ok(job_id) => (
                        ExecutionState::Accepted,
                        json!({"request_id":request_id,"job_id":job_id,
                    "session_id":agent.session().session_id(),"project_id":project}),
                        None,
                    ),
                    Err(code) => (ExecutionState::Failed, Value::Null, Some(code)),
                };
                let saved = agent.session_mut().append_event(event(
                    if state == ExecutionState::Accepted {
                        "accepted"
                    } else {
                        "failed"
                    },
                    result.clone(),
                ));
                if saved.is_err() {
                    return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
                }
                Ok((state, result, code))
            })
            .map_err(|error| error.remote_code())?
    })();
    match result {
        Ok((state, result, code)) => {
            response.execution_state = Some(state);
            response.code = code;
            response.session_id = request.session_id.clone();
            response.channel = Channel::Control;
            response.payload = Message::Response { result };
        }
        Err(code) => response.code = Some(code),
    }
    response
}

#[cfg(test)]
mod steer_tests {
    use super::*;
    #[test]
    fn typed_prompt_failure_survives_restart_without_raw_error_or_cross_device_scope() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let owner = Arc::new(Mutex::new(Agent::with_echo(
            crate::session::SessionStore::open_in_workspace(root.join("session.jsonl"), &root)
                .unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let req = request(
            &pool,
            "failure",
            Operation::Prompt,
            json!({"text":"ordinary"}),
        );
        let error = crate::core::AgentError::InvalidTurn(
            "Bearer PRIVATE_DIAGNOSTIC /private/host/secret".into(),
        );
        let code = failure_code(&error);
        assert_eq!(code, ErrorCode::InvalidRequest);
        assert_eq!(
            failure_code(&crate::core::AgentError::Governance("SECRET".into())),
            ErrorCode::ResourceLimit
        );
        assert_eq!(
            failure_code(&crate::core::AgentError::Recovery("SECRET".into())),
            ErrorCode::InternalError
        );
        persist_progress_with_code(
            &mut owner.lock().unwrap(),
            &req,
            "device",
            ExecutionState::Failed,
            Some(code),
        )
        .unwrap();
        let progress = PromptProgress::default();
        progress.record_with_code(&req, "device", ExecutionState::Failed, Some(code));
        let live = progress.listing(&pool, "device", &project);
        assert_eq!(live[0]["failure_code"], "invalid_request");
        assert_eq!(live[0].as_object().unwrap().len(), 5);
        assert!(!live.to_string().contains("PRIVATE_DIAGNOSTIC"));
        assert!(!live.to_string().contains("/private/host"));
        assert_eq!(
            progress.listing(&pool, "foreign-device", &project),
            json!([])
        );
        assert_eq!(
            PromptProgress::default().listing(&pool, "device", &project),
            live
        );
        progress.record_with_code(
            &req,
            "device",
            ExecutionState::Succeeded,
            Some(ErrorCode::InternalError),
        );
        assert!(
            progress.listing(&pool, "device", &project)[0]
                .get("failure_code")
                .is_none()
        );
        assert!(
            !std::fs::read_to_string(owner.lock().unwrap().session().path())
                .unwrap()
                .contains("PRIVATE_DIAGNOSTIC")
        );
        drop(pool);
        drop(owner);
        let restarted = Arc::new(Mutex::new(Agent::with_echo(
            crate::session::SessionStore::open_in_workspace(root.join("session.jsonl"), &root)
                .unwrap(),
        )));
        let mut restored = ProjectOwnerPool::new(restarted).unwrap();
        restored.restore_checkpoint().unwrap();
        assert_eq!(
            PromptProgress::default().listing(&restored, "device", &project),
            live
        );
    }
    use crate::{
        input_queue::{InputQueue, InputQueueLimits, InputQueueReply},
        remote::auth::{Capability, DeviceChallenge, pairing_proof_message},
        session::SessionStore,
    };
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};

    fn phone(registry: &DeviceRegistry, project: &str, control: bool) -> AuthenticatedDevice {
        let random = ring::rand::SystemRandom::new();
        let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
        let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random)
            .unwrap();
        let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
        let sign =
            |bytes: &[u8]| URL_SAFE_NO_PAD.encode(key.sign(&random, bytes).unwrap().as_ref());
        let record = registry
            .verify_pair_and_register(
                "token",
                "pair",
                "phone",
                &public,
                &sign(&pairing_proof_message(
                    &"a".repeat(64),
                    "token",
                    "pair",
                    "phone",
                    &public,
                )),
                || true,
            )
            .unwrap();
        registry
            .set_permissions(
                &record.id,
                record.revision,
                if control {
                    vec![Capability::Read, Capability::Control]
                } else {
                    vec![Capability::Read]
                },
                vec![project.into()],
            )
            .unwrap();
        let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
        challenge
            .authenticate(
                registry,
                &record.id,
                &sign(&challenge.proof_message(&record.id)),
            )
            .unwrap()
    }
    fn request(pool: &ProjectOwnerPool, id: &str, operation: Operation, args: Value) -> Envelope {
        let mut request = Envelope::new(
            Channel::Control,
            Message::Request {
                operation,
                arguments: args,
            },
        );
        request.id = Some(id.into());
        request.project_id = Some(pool.active_context().project_id.clone());
        request.session_id = Some(pool.active_context().session_id.clone());
        request.required_owner = Some(pool.workspace_owner().into());
        request.expected_revision = Some(pool.workspace_revision());
        request
    }

    #[test]
    fn steering_refuses_commands_and_file_tokens_before_receipt_or_queue_side_effects() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let owner = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project, true);
        let original = request(&pool, "prompt", Operation::Prompt, json!({"text":"first"}));
        let port = pool.control(&project).unwrap().input_port;
        port.set_scope(Some("turn-1"));
        let mut admissions = SteerAdmissions::default();
        let mut agent = owner.lock().unwrap();
        let journal = std::fs::read(agent.session().path()).unwrap();
        let checkpoint = std::fs::read(root.join("project-workspace.json")).ok();
        for (index, text) in [
            "/skill:research",
            " /review literal",
            "/help",
            "\u{85}/unknown",
            "use @file.txt",
            "@",
            "@\"unfinished",
            "use\u{a0}@file",
        ]
        .into_iter()
        .enumerate()
        {
            let input = request(
                &pool,
                &format!("bad-{index}"),
                Operation::Steer,
                json!({"request_id":"prompt","text":text}),
            );
            let response = admissions.dispatch(
                &mut pool,
                &registry,
                &device,
                &input,
                Some((&original, device.id())),
            );
            assert_eq!(response.code, Some(ErrorCode::InvalidRequest));
            assert_eq!(response.execution_state, Some(ExecutionState::Failed));
            assert!(matches!(response.payload, Message::Response { result } if result.is_null()));
            assert!(!port.has_pending());
            assert!(admissions.0.is_empty());
            assert_eq!(std::fs::read(agent.session().path()).unwrap(), journal);
            assert_eq!(
                std::fs::read(root.join("project-workspace.json")).ok(),
                checkpoint
            );
        }
        // Historical durable retries remain resolvable without resubmitting now-refused text.
        let legacy = request(
            &pool,
            "legacy",
            Operation::Steer,
            json!({"request_id":"prompt","text":"/review literal"}),
        );
        let fingerprint = format!("{:x}", Sha256::digest(serde_json::to_vec(&legacy).unwrap()));
        pool.commit_steer_receipt(
            device.id(),
            &legacy,
            &fingerprint,
            "prompt",
            ExecutionState::Accepted,
            None,
        )
        .unwrap();
        assert_eq!(
            admissions
                .dispatch(&mut pool, &registry, &device, &legacy, None)
                .execution_state,
            Some(ExecutionState::Accepted)
        );
        assert!(!port.has_pending());
        assert_eq!(std::fs::read(agent.session().path()).unwrap(), journal);
        let input = request(
            &pool,
            "plain",
            Operation::Steer,
            json!({"request_id":"prompt","text":"email a@example.invalid; inline /path and !command"}),
        );
        assert_eq!(
            admissions
                .dispatch(
                    &mut pool,
                    &registry,
                    &device,
                    &input,
                    Some((&original, device.id()))
                )
                .execution_state,
            Some(ExecutionState::UnknownOutcome)
        );
        port.service(agent.session_mut(), "turn-1", false).unwrap();
        admissions.poll(&mut pool);
        assert_eq!(
            admissions
                .dispatch(&mut pool, &registry, &device, &input, None)
                .execution_state,
            Some(ExecutionState::Accepted)
        );
        registry.revoke(device.id()).unwrap();
        assert_eq!(
            admissions
                .dispatch(&mut pool, &registry, &device, &legacy, None)
                .code,
            Some(ErrorCode::Unauthorized)
        );
    }

    #[test]
    fn remote_private_attachments_refuse_before_binding_journal_or_runner_and_keep_local_behavior()
    {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::write(root.join("credentials.json"), "PRIVATE_FIXTURE").unwrap();
        std::fs::write(root.join("note.txt"), "ordinary").unwrap();
        let owner = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        pool.select(&project).unwrap();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project, true);
        let journal = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
        let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
        for (index, path) in [
            "credentials.json",
            "docs/auth.json",
            "private.pem",
            "PRIVATE.KEY",
            "certificate.p12",
            "id_rsa",
            ".aws/config",
            "credentials/notes",
        ]
        .iter()
        .enumerate()
        {
            let req = request(
                &pool,
                &format!("private-{index}"),
                Operation::Prompt,
                json!({"text":format!("inspect @\"{path}\"")}),
            );
            let response =
                dispatch_prompt(&mut pool, &registry, &device, &req, false, |_, _, _| {
                    panic!("private file entered runner")
                });
            assert_eq!(response.code, Some(ErrorCode::InvalidRequest));
            assert_eq!(response.execution_state, Some(ExecutionState::Failed));
            assert_eq!(
                std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
                journal
            );
            assert_eq!(
                std::fs::read(root.join("project-workspace.json")).unwrap(),
                checkpoint
            );
        }
        let mut agent = owner.lock().unwrap();
        agent.set_attachment_workspace(crate::tools::ToolContext::new(&root).unwrap());
        assert!(
            crate::slash_actions::prompt_attachments(&agent, "inspect @credentials.json").is_ok()
        );
        assert!(prompt_input(&agent, "inspect @credentials.json".into()).is_err());
        assert!(prompt_input(&agent, "inspect @note.txt".into()).is_ok());
        assert!(prompt_input(&agent, "mail@example.invalid".into()).is_ok());
        drop(agent);
        // Refusal did not reserve this ID; an ordinary request may still use it.
        let ordinary = request(
            &pool,
            "private-0",
            Operation::Prompt,
            json!({"text":"ordinary text"}),
        );
        assert_eq!(
            dispatch_prompt(
                &mut pool,
                &registry,
                &device,
                &ordinary,
                false,
                |_, _, _| Ok("job".into())
            )
            .execution_state,
            Some(ExecutionState::Accepted)
        );
        // A pre-policy durable receipt remains queryable, never resubmitted.
        let legacy = request(
            &pool,
            "historical-private",
            Operation::Prompt,
            json!({"text":"inspect @credentials.json"}),
        );
        let fingerprint = format!("{:x}", Sha256::digest(serde_json::to_vec(&legacy).unwrap()));
        pool.bind_remote_prompt(device.id(), &legacy, &fingerprint)
            .unwrap();
        let result = json!({"request_id":"historical-private","project_id":project,"session_id":pool.active_context().session_id,"job_id":"historical"});
        owner.lock().unwrap().session_mut().append_event(json!({"type":RECEIPT_TYPE,"device_id":device.id(),
            "request_id":"historical-private","fingerprint":fingerprint,"state":"accepted","result":result})).unwrap();
        let saved = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
        let replay = dispatch_prompt(&mut pool, &registry, &device, &legacy, true, |_, _, _| {
            panic!("historical private attachment replayed")
        });
        assert_eq!(replay.execution_state, Some(ExecutionState::Accepted));
        assert!(matches!(replay.payload,Message::Response{result:actual} if actual==result));
        assert_eq!(
            std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
            saved
        );
    }

    #[test]
    fn steering_uses_sole_writer_and_durable_receipt_survives_restart_without_resubmission() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let owner = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project, true);
        let readonly = phone(&registry, &project, false);
        let other = phone(&registry, &project, true);
        let original = request(&pool, "prompt", Operation::Prompt, json!({"text":"first"}));
        let input = request(
            &pool,
            "steering",
            Operation::Steer,
            json!({"request_id":"prompt","text":"change direction"}),
        );
        let port = pool.control(&project).unwrap().input_port;
        port.set_scope(Some("turn-1"));
        let active = Some((&original, device.id()));
        let mut admissions = SteerAdmissions::default();
        assert_eq!(
            admissions
                .dispatch(&mut pool, &registry, &readonly, &input, active)
                .code,
            Some(ErrorCode::Unauthorized)
        );
        assert_eq!(
            admissions
                .dispatch(&mut pool, &registry, &other, &input, active)
                .code,
            Some(ErrorCode::OwnerMismatch)
        );
        let mut stale = input.clone();
        stale.required_owner = Some("b".repeat(64));
        assert_eq!(
            admissions
                .dispatch(&mut pool, &registry, &device, &stale, active)
                .code,
            Some(ErrorCode::OwnerMismatch)
        );
        stale = input.clone();
        stale.expected_revision = Some(99);
        assert_eq!(
            admissions
                .dispatch(&mut pool, &registry, &device, &stale, active)
                .code,
            Some(ErrorCode::RevisionConflict)
        );
        // Hold the Agent lock during admission, as the production provider runner does.
        let mut agent = owner.lock().unwrap();
        assert_eq!(
            admissions
                .dispatch(&mut pool, &registry, &device, &input, active)
                .execution_state,
            Some(ExecutionState::UnknownOutcome)
        );
        assert_eq!(
            admissions
                .dispatch(&mut pool, &registry, &device, &input, active)
                .execution_state,
            Some(ExecutionState::UnknownOutcome)
        );
        assert_eq!(admissions.0.len(), 1);
        assert_eq!(
            serde_json::to_value(
                InputQueue::recover(agent.session(), InputQueueLimits::default())
                    .unwrap()
                    .execute(
                        agent.session_mut(),
                        crate::protocol::InputQueueAction::List {
                            after_sequence: None,
                            limit: 32
                        }
                    )
                    .unwrap()
            )
            .unwrap(),
            json!({"result":"page","inputs":[],"next_sequence":null})
        );
        port.service(agent.session_mut(), "turn-1", false).unwrap();
        admissions.poll(&mut pool);
        let response = admissions.dispatch(&mut pool, &registry, &device, &input, None);
        assert_eq!(response.execution_state, Some(ExecutionState::Accepted));
        let Message::Response { result } = response.payload else {
            panic!()
        };
        assert_eq!(result["target_request_id"], "prompt");
        assert_eq!(result["request_id"], "steering");
        let listed = InputQueue::recover(agent.session(), InputQueueLimits::default())
            .unwrap()
            .execute(
                agent.session_mut(),
                crate::protocol::InputQueueAction::List {
                    after_sequence: None,
                    limit: 32,
                },
            )
            .unwrap();
        let InputQueueReply::Page { inputs, .. } = listed else {
            panic!()
        };
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].text, "change direction");
        drop(agent);
        port.set_scope(Some("replacement"));
        let mut restored = ProjectOwnerPool::new(owner).unwrap();
        restored.restore_checkpoint().unwrap();
        assert_eq!(
            SteerAdmissions::default()
                .dispatch(&mut restored, &registry, &device, &input, None)
                .execution_state,
            Some(ExecutionState::Accepted)
        );
        assert!(!port.has_pending());
        let collision = request(
            &restored,
            "steering",
            Operation::Prompt,
            json!({"text":"new"}),
        );
        assert_eq!(
            dispatch_prompt(
                &mut restored,
                &registry,
                &device,
                &collision,
                false,
                |_, _, _| panic!()
            )
            .code,
            Some(ErrorCode::RequestIdConflict)
        );
        let mut changed = input.clone();
        changed.payload = Message::Request {
            operation: Operation::Steer,
            arguments: json!({"request_id":"prompt","text":"different"}),
        };
        assert_eq!(
            admissions
                .dispatch(&mut restored, &registry, &device, &changed, None)
                .code,
            Some(ErrorCode::RequestIdConflict)
        );
    }

    #[test]
    fn lost_ticket_stays_ambiguous_and_never_steers_replacement_turn() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let owner = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project, true);
        let original = request(&pool, "prompt", Operation::Prompt, json!({"text":"first"}));
        let input = request(
            &pool,
            "steering",
            Operation::Steer,
            json!({"request_id":"prompt","text":"hello"}),
        );
        let port = pool.control(&project).unwrap().input_port;
        port.set_scope(Some("turn-1"));
        let mut admissions = SteerAdmissions::default();
        assert_eq!(
            admissions
                .dispatch(
                    &mut pool,
                    &registry,
                    &device,
                    &input,
                    Some((&original, device.id()))
                )
                .execution_state,
            Some(ExecutionState::UnknownOutcome)
        );
        drop(admissions);
        port.set_scope(Some("replacement"));
        let mut restored = ProjectOwnerPool::new(owner.clone()).unwrap();
        restored.restore_checkpoint().unwrap();
        let reply = SteerAdmissions::default().dispatch(
            &mut restored,
            &registry,
            &device,
            &input,
            Some((&original, device.id())),
        );
        assert_eq!(reply.execution_state, Some(ExecutionState::UnknownOutcome));
        assert!(!port.has_pending());
        let mut agent = owner.lock().unwrap();
        let listed = InputQueue::recover(agent.session(), InputQueueLimits::default())
            .unwrap()
            .execute(
                agent.session_mut(),
                crate::protocol::InputQueueAction::List {
                    after_sequence: None,
                    limit: 32,
                },
            )
            .unwrap();
        assert_eq!(
            serde_json::to_value(listed).unwrap(),
            json!({"result":"page","inputs":[],"next_sequence":null})
        );
    }
}
