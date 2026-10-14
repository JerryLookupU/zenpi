//! Bounded model projection and durable, owner-scoped model selection.

use super::{
    auth::{AuthenticatedDevice, DeviceRegistry},
    protocol::{Envelope, ErrorCode, Message, Operation},
};
use crate::project_workspace::ProjectOwnerPool;
use serde_json::{Value, json};

fn label(value: Option<&str>) -> Option<String> {
    value
        .filter(|text| {
            !text.is_empty()
                && text.len() <= 128
                && !text.starts_with('/')
                && !text.contains("://")
                && !text.chars().any(char::is_control)
                && crate::security::redact_text(text, &[]) == *text
        })
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::label;
    #[test]
    fn model_labels_never_project_credentials_urls_roots_or_unbounded_text() {
        for value in [
            "",
            "/host/path",
            "https://private.invalid/model",
            "model\0name",
            "Bearer SECRET_CREDENTIAL",
        ] {
            assert!(label(Some(value)).is_none(), "{value:?}");
        }
        assert!(label(Some(&"x".repeat(129))).is_none());
        assert_eq!(
            label(Some("provider/model-v1:latest")).as_deref(),
            Some("provider/model-v1:latest")
        );
    }
}

pub fn status(
    pool: &ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
) -> Result<Value, ErrorCode> {
    if request.validate().is_err()
        || request.id.is_none()
        || request.required_owner.is_some()
        || request.expected_revision.is_some()
        || request.turn_id.is_some()
        || request.sequence.is_some()
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let Message::Request {
        operation: Operation::ModelStatus,
        arguments,
    } = &request.payload
    else {
        return Err(ErrorCode::InvalidRequest);
    };
    let args = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
    let (offset, expected) = if args.is_empty() {
        (0usize, None)
    } else {
        if args.len() != 2 {
            return Err(ErrorCode::InvalidRequest);
        }
        let offset = args
            .get("offset")
            .and_then(Value::as_u64)
            .filter(|n| *n <= 512 && *n % 16 == 0)
            .ok_or(ErrorCode::InvalidRequest)? as usize;
        let snapshot = args
            .get("snapshot_id")
            .and_then(Value::as_str)
            .filter(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
            })
            .ok_or(ErrorCode::InvalidRequest)?;
        (offset, Some(snapshot))
    };
    let project = request
        .project_id
        .as_deref()
        .ok_or(ErrorCode::InvalidRequest)?;
    registry.with_authorized_projects(device, &[project], Operation::ModelStatus, |_| {
        let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
        if request.session_id.as_deref() != Some(context.session_id.as_str()) { return Err(ErrorCode::OwnerMismatch); }
        let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
        let agent = owner.try_lock().map_err(|_| ErrorCode::Busy)?;
        if agent.session().session_id() != context.session_id { return Err(ErrorCode::OwnerMismatch); }
        let status = agent.model_status().map_err(|_| ErrorCode::InternalError)?;
        let model = label(status["active"]["id"].as_str().or_else(|| status["model"].as_str()));
        let reasoning = status["reasoning_effort"].as_str().filter(|value| matches!(*value, "none"|"minimal"|"low"|"medium"|"high"|"xhigh"|"max"|"ultra"));
        let catalog=status["catalog"].as_array().ok_or(ErrorCode::InternalError)?;
        if catalog.len()>512 { return Err(ErrorCode::ResourceLimit); }
        let mut entries=Vec::new();
        for descriptor in catalog {
            let Some(id)=label(descriptor["id"].as_str()) else {continue;};
            let lifecycle=descriptor["status"].as_str().filter(|value| matches!(*value,"active"|"alpha"|"beta"|"deprecated")).ok_or(ErrorCode::InternalError)?;
            entries.push(json!({"id":id,"status":lifecycle,"selectable":agent.validate_model_choice(&id).is_ok()}));
        }
        entries.sort_by(|a,b| a["id"].as_str().cmp(&b["id"].as_str()));
        if entries.windows(2).any(|pair| pair[0]["id"]==pair[1]["id"]) { return Err(ErrorCode::InternalError); }
        use sha2::{Digest,Sha256};
        let snapshot_id=format!("{:x}",Sha256::digest(serde_json::to_vec(&json!({"project":project,"session":context.session_id,"selection_revision":agent.selection_revision(),"catalog":catalog,"entries":entries})).map_err(|_|ErrorCode::InternalError)?));
        if expected.is_some_and(|value| value!=snapshot_id) {return Err(ErrorCode::RevisionConflict);}
        if offset>0 && offset>=entries.len() {return Err(ErrorCode::InvalidRequest);}
        let end=(offset+16).min(entries.len());
        let catalog_page=json!({"snapshot_id":snapshot_id,"offset":offset,"next_offset":if end<entries.len(){Some(end)}else{None},"entries":&entries[offset..end]});
        let result = json!({"project_id":project,"session_id":context.session_id,"model":model,"reasoning_effort":reasoning,
            "registry_bound":status["registry_bound"],"selection_revision":agent.selection_revision(),
            "capabilities":status["capabilities"],"budget":status["budget"],
            "reasoning_choices":agent.reasoning_choices().map_err(|_| ErrorCode::InternalError)?,"catalog":catalog_page});
        if serde_json::to_vec(&result).map_err(|_| ErrorCode::InternalError)?.len() > 16384 { return Err(ErrorCode::ResourceLimit); }
        Ok(result)
    }).map_err(|error| error.remote_code())?
}

pub fn select(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    busy: bool,
) -> Envelope {
    use super::protocol::ExecutionState;
    use crate::core::{ConnectionRejectCode, ConnectionSelectionOutcome};
    use sha2::{Digest, Sha256};
    let mut response =
        super::host_adapter::HostAdapter::failure(request, ErrorCode::InvalidRequest);
    let outcome = (|| {
        if request.validate().is_err() || request.turn_id.is_some() || request.sequence.is_some() {
            return Err(ErrorCode::InvalidRequest);
        }
        let Message::Request {
            operation,
            arguments,
        } = &request.payload
        else {
            return Err(ErrorCode::InvalidRequest);
        };
        let (field, receipt_type) = match operation {
            Operation::ModelSelect => ("model", "remote_model_admission_v1"),
            Operation::ReasoningSelect => ("reasoning_effort", "remote_reasoning_admission_v1"),
            _ => return Err(ErrorCode::InvalidRequest),
        };
        let args = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
        let selected = args.get(field).ok_or(ErrorCode::InvalidRequest)?;
        let model = selected.as_str();
        let revision = args
            .get("selection_revision")
            .and_then(Value::as_u64)
            .ok_or(ErrorCode::InvalidRequest)?;
        if args.len() != 2
            || if *operation == Operation::ModelSelect {
                model.is_none() || label(model).as_deref() != model
            } else {
                !selected.is_null()
                    && !model.is_some_and(|value| {
                        matches!(
                            value,
                            "none"
                                | "minimal"
                                | "low"
                                | "medium"
                                | "high"
                                | "xhigh"
                                | "max"
                                | "ultra"
                        )
                    })
            }
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
        registry.with_authorized_projects(device, &[project], *operation, |_| {
            let bound = pool.owner_request_binding(device.id(), id, &fingerprint, *operation)?;
            let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
            let mut agent = match owner.try_lock() {
                Ok(agent) => agent,
                Err(_) if bound => return Ok((ExecutionState::UnknownOutcome, Value::Null, None)),
                Err(_) => return Err(ErrorCode::Busy),
            };
            if request.session_id.as_deref() != Some(agent.session().session_id()) {
                return Err(ErrorCode::OwnerMismatch);
            }
            let receipts: Vec<_> = agent.session().events().iter().filter(|event| event["type"] == receipt_type).collect();
            if let Some(event) = receipts.iter().rev().find(|event| event["device_id"] == device.id() && event["request_id"] == id) {
                if event["fingerprint"] != fingerprint { return Err(ErrorCode::RequestIdConflict); }
                return match event["state"].as_str() {
                    Some("succeeded") => Ok((ExecutionState::Succeeded, event["result"].clone(), None)),
                    Some("failed") => Ok((ExecutionState::Failed, Value::Null, Some(serde_json::from_value(event["code"].clone()).map_err(|_| ErrorCode::InternalError)?))),
                    _ => Ok((ExecutionState::UnknownOutcome, Value::Null, None)),
                };
            }
            if bound { return Ok((ExecutionState::UnknownOutcome, Value::Null, None)); }
            if receipts.len() >= 256 { return Err(ErrorCode::ResourceLimit); }
            if request.required_owner.as_deref() != Some(pool.workspace_owner()) { return Err(ErrorCode::OwnerMismatch); }
            if request.expected_revision != Some(pool.workspace_revision()) || revision != agent.selection_revision() { return Err(ErrorCode::RevisionConflict); }
            if busy || agent.phase() != crate::core::AgentPhase::Idle { return Err(ErrorCode::Busy); }
            let event = |state: &str, result: Value, code: Option<ErrorCode>| json!({"type":receipt_type,
                "device_id":device.id(),"request_id":id,"fingerprint":fingerprint,"state":state,"result":result,"code":code});
            // The checkpoint binds IDs across operations/projects before any selection event.
            if pool.bind_owner_request(device.id(), request, &fingerprint, *operation).is_err()
                || agent.session_mut().append_event(event("reserved", Value::Null, None)).is_err() {
                return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
            }
            let selection = if *operation == Operation::ModelSelect {
                agent.select_model(model.expect("validated model").into(), revision)
            } else { agent.select_reasoning(model.map(str::to_owned), revision) };
            let (state, result, code) = match selection {
                Ok(ConnectionSelectionOutcome::Applied { selection_revision, .. }) => {
                    let mut result = json!({"request_id":id,"project_id":project,"session_id":agent.session().session_id(),"selection_revision":selection_revision});
                    result[field] = selected.clone();
                    (ExecutionState::Succeeded, result, None)
                },
                Ok(ConnectionSelectionOutcome::Rejected { code, .. }) => (ExecutionState::Failed, Value::Null,
                    Some(match code { ConnectionRejectCode::ConnectionBusy => ErrorCode::Busy,
                        ConnectionRejectCode::ConnectionStale => ErrorCode::RevisionConflict,
                        ConnectionRejectCode::OwnerMismatch => ErrorCode::OwnerMismatch,
                        ConnectionRejectCode::ConnectionInvalid => ErrorCode::InvalidRequest })),
                Err(_) => return Ok((ExecutionState::UnknownOutcome, Value::Null, None)),
            };
            if agent.session_mut().append_event(event(if state == ExecutionState::Succeeded { "succeeded" } else { "failed" }, result.clone(), code)).is_err() {
                return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
            }
            Ok((state, result, code))
        }).map_err(|error| error.remote_code())?
    })();
    match outcome {
        Ok((state, result, code)) => {
            response.execution_state = Some(state);
            response.code = code;
            response.session_id = request.session_id.clone();
            response.payload = Message::Response { result };
        }
        Err(code) => response.code = Some(code),
    }
    response
}
