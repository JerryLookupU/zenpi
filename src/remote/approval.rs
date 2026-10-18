//! Bounded read-only projection of the existing host approval rendezvous.

use super::auth::{AuthenticatedDevice, DeviceRegistry};
use super::protocol::{Envelope, ErrorCode, ExecutionState, Message, Operation};
use crate::project_workspace::ProjectOwnerPool;
use serde_json::{Value, json};

pub fn listing(pool: &ProjectOwnerPool, project: &str) -> Result<Value, ErrorCode> {
    let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
    let Some(coordinator) = pool.control(project).and_then(|control| control.approval) else {
        return Ok(json!([]));
    };
    let summaries = coordinator
        .pending_summaries(16)
        .map_err(|_| ErrorCode::InternalError)?;
    let mut values = Vec::with_capacity(summaries.len());
    for summary in summaries {
        let mut value = serde_json::to_value(summary).map_err(|_| ErrorCode::InternalError)?;
        value["project_id"] = json!(project);
        value["session_id"] = json!(context.session_id);
        values.push(value);
    }
    Ok(Value::Array(values))
}

/// One bounded live projection for the shared desktop/headless/phone route.
pub fn combined_listing(pool: &ProjectOwnerPool, project: &str) -> Result<Value, ErrorCode> {
    let Value::Array(mut values) = listing(pool, project)? else {
        unreachable!()
    };
    if values.len() < 16 {
        let Value::Array(workers) = worker_listing(pool, project)? else {
            unreachable!()
        };
        values.extend(workers.into_iter().take(16 - values.len()));
    }
    Ok(Value::Array(values))
}

pub fn dispatch_routed(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
) -> Envelope {
    let worker = matches!(&request.payload, Message::Request { arguments, .. }
        if arguments.get("worker_session_id").is_some());
    dispatch_inner(pool, registry, device, request, worker)
}

pub fn dispatch(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
) -> Envelope {
    dispatch_inner(pool, registry, device, request, false)
}

/// Worker controls are independent from the master approval rendezvous.
pub fn dispatch_worker(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
) -> Envelope {
    dispatch_inner(pool, registry, device, request, true)
}

pub fn worker_listing(pool: &ProjectOwnerPool, project: &str) -> Result<Value, ErrorCode> {
    let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
    let mut values = Vec::new();
    for worker in pool.worker_handles(project) {
        if worker.master_session_id() != context.session_id {
            continue;
        }
        let control = worker.control();
        let Some(coordinator) = control.approval else {
            continue;
        };
        for summary in coordinator
            .pending_summaries(16 - values.len())
            .map_err(|_| ErrorCode::InternalError)?
        {
            let mut value = serde_json::to_value(summary).map_err(|_| ErrorCode::InternalError)?;
            value["project_id"] = json!(project);
            value["session_id"] = json!(context.session_id);
            value["worker_session_id"] = json!(control.input_port.session_id().as_str());
            value["subtab_id"] = json!(worker.execution().tab.id);
            values.push(value);
            if values.len() == 16 {
                return Ok(Value::Array(values));
            }
        }
    }
    Ok(Value::Array(values))
}

pub fn authorized_worker_listing(
    pool: &ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
) -> Result<Value, ErrorCode> {
    if request.validate().is_err()
        || request.required_owner.is_some()
        || request.expected_revision.is_some()
        || request.turn_id.is_some()
        || request.sequence.is_some()
        || !matches!(&request.payload, Message::Request { operation: Operation::Inspect, arguments }
            if arguments.as_object().is_some_and(|values| values.is_empty()))
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let project = request
        .project_id
        .as_deref()
        .ok_or(ErrorCode::InvalidRequest)?;
    registry
        .with_authorized_projects(device, &[project], Operation::Inspect, |_| {
            if !pool
                .context(project)
                .is_some_and(|context| request.session_id.as_deref() == Some(&context.session_id))
            {
                return Err(ErrorCode::OwnerMismatch);
            }
            worker_listing(pool, project)
        })
        .map_err(|error| error.remote_code())?
}

fn dispatch_inner(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    worker: bool,
) -> Envelope {
    let mut response =
        super::host_adapter::HostAdapter::failure(request, ErrorCode::InvalidRequest);
    let result = (|| {
        use sha2::{Digest, Sha256};
        if request.validate().is_err() || request.turn_id.is_some() || request.sequence.is_some() {
            return Err(ErrorCode::InvalidRequest);
        }
        let Message::Request {
            operation: Operation::Approve,
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
        let digest = values
            .get("digest")
            .and_then(Value::as_str)
            .ok_or(ErrorCode::InvalidRequest)?;
        let decision = match values.get("decision").and_then(Value::as_str) {
            Some("allow") => crate::approval::ApprovalDecision::Allow,
            Some("deny") => crate::approval::ApprovalDecision::Deny,
            _ => return Err(ErrorCode::InvalidRequest),
        };
        let worker_session = if worker {
            Some(
                values
                    .get("worker_session_id")
                    .and_then(Value::as_str)
                    .filter(|id| super::auth::valid_identifier(id))
                    .ok_or(ErrorCode::InvalidRequest)?,
            )
        } else {
            None
        };
        if values.len() != if worker { 4 } else { 3 }
            || !super::auth::valid_identifier(target)
            || digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
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
            .with_authorized_projects(device, &[project], Operation::Approve, |_| {
                if let Some(receipt) = pool.approval_receipt(device.id(), id, &fingerprint)? {
                    return Ok(receipt);
                }
                if request.required_owner.as_deref() != Some(pool.workspace_owner()) {
                    return Err(ErrorCode::OwnerMismatch);
                }
                if request.expected_revision != Some(pool.workspace_revision()) {
                    return Err(ErrorCode::RevisionConflict);
                }
                if !pool
                    .context(project)
                    .is_some_and(|c| request.session_id.as_deref() == Some(c.session_id.as_str()))
                {
                    return Err(ErrorCode::OwnerMismatch);
                }
                let coordinator = if let Some(session) = worker_session {
                    pool.worker_handles(project)
                        .into_iter()
                        .filter(|owner| {
                            Some(owner.master_session_id()) == request.session_id.as_deref()
                        })
                        .map(|owner| owner.control())
                        .find(|control| control.input_port.session_id().as_str() == session)
                        .and_then(|control| control.approval)
                } else {
                    pool.control(project).and_then(|control| control.approval)
                }
                .ok_or(ErrorCode::InvalidRequest)?;
                // Checking the summary is only preflight; the final coordinator claim checks the digest atomically.
                if !coordinator
                    .pending_summaries(16)
                    .map_err(|_| ErrorCode::InternalError)?
                    .iter()
                    .any(|r| r.request_id == target && r.digest == digest)
                {
                    return Err(ErrorCode::InvalidRequest);
                }
                if pool
                    .commit_approval_receipt(
                        device.id(),
                        request,
                        &fingerprint,
                        target,
                        digest,
                        decision,
                        ExecutionState::UnknownOutcome,
                        None,
                    )
                    .is_err()
                {
                    return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
                }
                let result = coordinator.respond_remote(
                    crate::approval::ApprovalResponse {
                        request_id: target.into(),
                        decision,
                        remember: false,
                        message: None,
                    },
                    digest,
                    crate::approval::RemoteApprovalActor {
                        device_id: device.id().into(),
                        request_id: id.into(),
                        fingerprint: fingerprint.clone(),
                        project_id: project.into(),
                        session_id: worker_session
                            .map(str::to_owned)
                            .or_else(|| request.session_id.clone())
                            .ok_or(ErrorCode::InvalidRequest)?,
                        request_digest: digest.into(),
                    },
                );
                let (state, code) = if result.is_ok() {
                    (ExecutionState::Accepted, None)
                } else {
                    (ExecutionState::Failed, Some(ErrorCode::Busy))
                };
                if pool
                    .commit_approval_receipt(
                        device.id(),
                        request,
                        &fingerprint,
                        target,
                        digest,
                        decision,
                        state,
                        code,
                    )
                    .is_err()
                {
                    return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
                }
                Ok((
                    state,
                    if state == ExecutionState::Accepted {
                        json!({"request_id":id,"target_request_id":target,"project_id":project,
                "session_id":request.session_id,"digest":digest,"decision":decision})
                    } else {
                        Value::Null
                    },
                    code,
                ))
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
