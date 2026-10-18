//! Metadata-only catalogue of resources loaded by the existing Agent.

use super::{
    auth::{AuthenticatedDevice, DeviceRegistry},
    protocol::{Envelope, ErrorCode, Message, Operation},
};
use crate::project_workspace::ProjectOwnerPool;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const RELOAD_RECEIPT: &str = "remote_resources_admission_v1";

/// Reload only paths already selected by the host. Bind intent before reading
/// files or restarting trusted extensions; uncertain outcomes are never rerun.
pub fn reload(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    busy: bool,
) -> Envelope {
    use super::protocol::ExecutionState;
    let mut response =
        super::host_adapter::HostAdapter::failure(request, ErrorCode::InvalidRequest);
    let outcome = (|| {
        request.validate().map_err(|_| ErrorCode::InvalidRequest)?;
        if request.turn_id.is_some() || request.sequence.is_some() {
            return Err(ErrorCode::InvalidRequest);
        }
        let Message::Request {
            operation: Operation::ResourcesReload,
            arguments,
        } = &request.payload
        else {
            return Err(ErrorCode::InvalidRequest);
        };
        let args = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
        if args.len() != 2 {
            return Err(ErrorCode::InvalidRequest);
        }
        let generation = args
            .get("generation")
            .and_then(Value::as_u64)
            .ok_or(ErrorCode::InvalidRequest)?;
        let snapshot = args
            .get("snapshot_id")
            .and_then(Value::as_str)
            .filter(|value| {
                value.len() == 64
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            .ok_or(ErrorCode::InvalidRequest)?;
        let project = request
            .project_id
            .as_deref()
            .ok_or(ErrorCode::InvalidRequest)?;
        let id = request.id.as_deref().ok_or(ErrorCode::InvalidRequest)?;
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(request).map_err(|_| ErrorCode::InternalError)?)
        );
        let mut catalog_request = request.clone();
        catalog_request.required_owner = None;
        catalog_request.expected_revision = None;
        catalog_request.payload = Message::Request {
            operation: Operation::ResourcesList,
            arguments: json!({}),
        };
        // Avoid nesting registry leases. Retries below ignore this freshness
        // result, while new requests also recheck generation under the owner.
        let catalog = listing(pool, registry, device, &catalog_request);
        registry.with_authorized_projects(device, &[project], Operation::ResourcesReload, |_| {
            let bound = pool.owner_request_binding(device.id(), id, &fingerprint, Operation::ResourcesReload)?;
            let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
            let mut agent = match owner.try_lock() {
                Ok(agent) => agent,
                Err(_) if bound => return Ok((ExecutionState::UnknownOutcome, Value::Null, None)),
                Err(_) => return Err(ErrorCode::Busy),
            };
            if request.session_id.as_deref() != Some(agent.session().session_id()) { return Err(ErrorCode::OwnerMismatch); }
            let receipts: Vec<_> = agent.session().events().iter().filter(|event| event["type"] == RELOAD_RECEIPT).collect();
            if let Some(event) = receipts.iter().rev().find(|event| event["device_id"] == device.id() && event["request_id"] == id) {
                if event["fingerprint"] != fingerprint { return Err(ErrorCode::RequestIdConflict); }
                return match event["state"].as_str() {
                    Some("succeeded") => Ok((ExecutionState::Succeeded, event["result"].clone(), None)),
                    Some("failed") => Ok((ExecutionState::Failed, Value::Null, Some(
                        serde_json::from_value(event["code"].clone()).map_err(|_| ErrorCode::InternalError)?))),
                    _ => Ok((ExecutionState::UnknownOutcome, Value::Null, None)),
                };
            }
            if bound { return Ok((ExecutionState::UnknownOutcome, Value::Null, None)); }
            if receipts.len() >= 256 { return Err(ErrorCode::ResourceLimit); }
            if request.required_owner.as_deref() != Some(pool.workspace_owner()) { return Err(ErrorCode::OwnerMismatch); }
            if request.expected_revision != Some(pool.workspace_revision())
                || generation != agent.resource_snapshot().generation { return Err(ErrorCode::RevisionConflict); }
            if catalog.as_ref().map_err(|code| *code)?["snapshot_id"] != snapshot { return Err(ErrorCode::RevisionConflict); }
            if busy || agent.phase() != crate::core::AgentPhase::Idle || agent.input_port().has_pending()
                || agent.input_port().is_preparing() { return Err(ErrorCode::Busy); }
            let receipt = |state: &str, result: Value, code: Option<ErrorCode>| json!({
                "type":RELOAD_RECEIPT,"device_id":device.id(),"request_id":id,"fingerprint":fingerprint,
                "state":state,"result":result,"code":code});
            if pool.bind_owner_request(device.id(), request, &fingerprint, Operation::ResourcesReload).is_err()
                || agent.session_mut().append_event(receipt("reserved", Value::Null, None)).is_err() {
                return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
            }
            // Cooperative reads/extension startup share a bounded deadline;
            // do not hold the desktop owner mailbox indefinitely.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
            let (state, result, code) = match agent.reload_resources(|| std::time::Instant::now() >= deadline) {
                Ok(_) => (ExecutionState::Succeeded, json!({"request_id":id,"project_id":project,
                    "session_id":request.session_id,"generation":agent.resource_snapshot().generation}), None),
                Err(_) => return Ok((ExecutionState::UnknownOutcome, Value::Null, None)),
            };
            if agent.session_mut().append_event(receipt("succeeded", result.clone(), code)).is_err() {
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

const MAX_CATALOG_ENTRIES: u64 =
    (crate::skills::MAX_SKILL_FILES + crate::prompt_templates::MAX_TEMPLATE_COUNT) as u64;

fn name_valid(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}
fn summary(text: &str, private_paths: &[&str]) -> String {
    let mut text = text.to_owned();
    for path in private_paths.iter().filter(|path| !path.is_empty()) {
        text = text.replace(path, "[host path]");
    }
    let mut text = crate::security::redact_text(&text, &[]);
    text.retain(|c| !c.is_control() || matches!(c, '\n' | '\t'));
    let mut end = text.len().min(1024);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text
}

pub fn listing(
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
        operation: Operation::ResourcesList,
        arguments,
    } = &request.payload
    else {
        return Err(ErrorCode::InvalidRequest);
    };
    let args = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
    let (offset, expected_snapshot) = if args.is_empty() {
        (0, None)
    } else {
        if args.len() != 2 || !args.contains_key("offset") || !args.contains_key("snapshot_id") {
            return Err(ErrorCode::InvalidRequest);
        }
        let offset = args["offset"]
            .as_u64()
            .filter(|n| *n <= MAX_CATALOG_ENTRIES && *n % 32 == 0)
            .ok_or(ErrorCode::InvalidRequest)? as usize;
        let digest = args["snapshot_id"]
            .as_str()
            .filter(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
            })
            .ok_or(ErrorCode::InvalidRequest)?;
        (offset, Some(digest))
    };
    let project = request
        .project_id
        .as_deref()
        .ok_or(ErrorCode::InvalidRequest)?;
    registry.with_authorized_projects(device, &[project], Operation::ResourcesList, |_| {
        let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
        if request.session_id.as_deref() != Some(context.session_id.as_str()) { return Err(ErrorCode::OwnerMismatch); }
        let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
        let agent = owner.try_lock().map_err(|_| ErrorCode::Busy)?;
        if agent.session().session_id() != context.session_id { return Err(ErrorCode::OwnerMismatch); }
        let snapshot = agent.resource_snapshot();
        let status = agent.resource_status();
        let stale = &status["reload_required_skills"];
        let mut entries = Vec::new();
        let mut total = 0usize;
        let mut digest = Sha256::new();
        digest.update(snapshot.generation.to_be_bytes());
        for identity in [project, context.session_id.as_str()] {
            digest.update((identity.len() as u64).to_be_bytes());
            digest.update(identity.as_bytes());
        }
        let mut retain = |entry: Value, provenance: &[&str]| -> Result<(), ErrorCode> {
            for field in provenance {
                digest.update((field.len() as u64).to_be_bytes());
                digest.update(field.as_bytes());
            }
            let bytes = serde_json::to_vec(&entry).map_err(|_| ErrorCode::InternalError)?;
            digest.update((bytes.len() as u64).to_be_bytes());
            digest.update(bytes);
            if total >= offset && entries.len() < 32 { entries.push(entry); }
            total += 1;
            Ok(())
        };
        for skill in snapshot.skills.metadata().filter(|skill| name_valid(&skill.name)) {
                let needs_reload = stale.as_array().is_some_and(|items| items.iter().any(|value| value == &skill.name));
                retain(json!({"name":skill.name,"kind":"skill","command":format!("/skill:{}", skill.name),
                    "description":summary(&skill.description, &[&skill.source.to_string_lossy(), &skill.base_dir.to_string_lossy(), &context.cwd]),"invocable":!needs_reload}),
                    &[&skill.source_hash, &skill.source.to_string_lossy(), &skill.base_dir.to_string_lossy()])?;
        }
        for template in snapshot.templates.templates().filter(|template| name_valid(&template.name)
            && crate::slash::resource_input_candidate(&format!("/{}", template.name))) {
                retain(json!({"name":template.name,"kind":"template","command":format!("/{}", template.name),
                    "description":summary(template.remote_description(), &[&template.source.to_string_lossy(), &context.cwd]),"invocable":true}),
                    &[&template.source_hash, &template.source.to_string_lossy()])?;
        }
        let snapshot_id = format!("{:x}", digest.finalize());
        if expected_snapshot.is_some_and(|expected| expected != snapshot_id) { return Err(ErrorCode::RevisionConflict); }
        if offset > 0 && offset >= total { return Err(ErrorCode::InvalidRequest); }
        let end = offset + entries.len();
        let result = json!({"project_id":project,"session_id":context.session_id,"generation":snapshot.generation,
            "snapshot_id":snapshot_id,"offset":offset,"next_offset":if end < total { Some(end) } else { None },
            "entries":entries,"truncated":end < total});
        if serde_json::to_vec(&result).map_err(|_| ErrorCode::InternalError)?.len() > 64 * 1024 {
            return Err(ErrorCode::ResourceLimit);
        }
        Ok(result)
    }).map_err(|error| error.remote_code())?
}

#[cfg(test)]
mod tests {
    use super::summary;

    #[test]
    fn descriptions_strip_known_paths_controls_and_truncate_at_utf8_boundary() {
        let text = format!(
            "/private/host/skills/research/SKILL.md\0\n{}",
            "\u{6c49}".repeat(1000)
        );
        let result = summary(
            &text,
            &["/private/host/skills/research/SKILL.md", "/private/host"],
        );
        assert!(result.starts_with("[host path]\n"));
        assert!(!result.contains("/private/host"));
        assert!(!result.contains('\0'));
        assert!(result.len() <= 1024);
        assert_eq!(result.len(), 1023);
    }
}
