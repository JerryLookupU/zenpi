//! Read-only review of the host's current stable-ID execution selection.

use super::{
    auth::{AuthenticatedDevice, DeviceRegistry},
    protocol::{Envelope, ErrorCode, Message, Operation},
};
use crate::project_workspace::ProjectOwnerPool;
use serde_json::{Value, json};

pub fn review(
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
        operation: Operation::Diff,
        arguments,
    } = &request.payload
    else {
        return Err(ErrorCode::InvalidRequest);
    };
    let values = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
    let paged = values.contains_key("offset") || values.contains_key("digest");
    if values.len() != if paged { 5 } else { 3 }
        || !["subtab_id", "subtab_revision", "path"]
            .iter()
            .all(|key| values.contains_key(*key))
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let offset = if paged {
        values
            .get("offset")
            .and_then(Value::as_u64)
            .filter(|n| *n <= 1024 * 1024)
            .ok_or(ErrorCode::InvalidRequest)? as usize
    } else {
        0
    };
    let digest = if !paged || values.get("digest").is_some_and(Value::is_null) {
        None
    } else {
        Some(
            values
                .get("digest")
                .and_then(Value::as_str)
                .filter(|value| {
                    value.len() == 64
                        && value
                            .bytes()
                            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
                })
                .ok_or(ErrorCode::InvalidRequest)?,
        )
    };
    if offset > 0 && digest.is_none() {
        return Err(ErrorCode::InvalidRequest);
    }
    let subtab = values["subtab_id"]
        .as_str()
        .ok_or(ErrorCode::InvalidRequest)?;
    let revision = values["subtab_revision"]
        .as_u64()
        .ok_or(ErrorCode::InvalidRequest)?;
    let path = values["path"].as_str().ok_or(ErrorCode::InvalidRequest)?;
    if path.trim().is_empty() || path.len() > 4096 || path.chars().any(char::is_control) {
        return Err(ErrorCode::InvalidRequest);
    }
    let project = request
        .project_id
        .as_deref()
        .ok_or(ErrorCode::InvalidRequest)?;
    registry
        .with_authorized_projects(device, &[project], Operation::Diff, |_| {
            let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
            if request.session_id.as_deref() != Some(context.session_id.as_str()) {
                return Err(ErrorCode::OwnerMismatch);
            }
            let state = pool
                .subtab_state(project)
                .map_err(|_| ErrorCode::InternalError)?;
            if state.revision != revision {
                return Err(ErrorCode::RevisionConflict);
            }
            if state.active_id != subtab {
                return Err(ErrorCode::OwnerMismatch);
            }
            let execution = pool
                .subtab_execution(project)
                .map_err(|_| ErrorCode::InvalidRequest)?;
            execution
                .validate()
                .map_err(|_| ErrorCode::InvalidRequest)?;
            let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
            let agent = owner.try_lock().map_err(|_| ErrorCode::Busy)?;
            if agent.session().session_id() != context.session_id {
                return Err(ErrorCode::OwnerMismatch);
            }
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1000);
            // Scope even a whole-tab review to "."; None would include the enclosing repository.
            let read = if paged { crate::slash_actions::remote_diff_document_at_with_cancel }
                else { crate::slash_actions::remote_diff_value_at_with_cancel };
            let value = read(&execution.tab.root, path, || std::time::Instant::now() >= deadline)
            .map_err(|error| match error {
                crate::slash_actions::SlashActionError::PathDenied(_) => ErrorCode::InvalidRequest,
                _ if std::time::Instant::now() >= deadline => ErrorCode::Busy,
                _ => ErrorCode::InternalError,
            })?;
            let text = |field: &str| {
                // Git --no-index headers may include absolute filenames; never project host roots.
                let text = value[field].as_str().unwrap_or_default();
                let text = text
                    .replace(&execution.tab.root.to_string_lossy().to_string(), ".")
                    .replace(&execution.project_root.to_string_lossy().to_string(), ".");
                crate::security::redact_text(&text, &[])
            };
            let mut result = json!({"project_id":project, "session_id":context.session_id, "subtab_id":subtab,
            "subtab_revision":revision, "path":value["path"], "changed":value["changed"],
            "binary":value.get("binary").and_then(Value::as_bool).unwrap_or(false),
            "truncated":value["truncated"], "diff":text("diff"), "status":text("status")});
            if paged {
                use sha2::{Digest, Sha256};
                let actual = format!("{:x}", Sha256::digest(serde_json::to_vec(&result).map_err(|_| ErrorCode::InternalError)?));
                if digest.is_some_and(|expected| expected != actual) { return Err(ErrorCode::RevisionConflict); }
                let patch = result["diff"].as_str().ok_or(ErrorCode::InternalError)?;
                if patch.len() > 1024 * 1024 { return Err(ErrorCode::ResourceLimit); }
                if offset > patch.len() || !patch.is_char_boundary(offset) || offset > 0 && offset == patch.len() { return Err(ErrorCode::InvalidRequest); }
                let mut end = (offset + 8192).min(patch.len());
                while !patch.is_char_boundary(end) { end -= 1; }
                let page = patch[offset..end].to_owned();
                let total = patch.len();
                result["diff"] = json!(page);
                result["page"] = json!({"digest":actual,"offset":offset,"next_offset":end,"total_bytes":total,"has_more":end < total});
            }
            if serde_json::to_vec(&result).map_err(|_| ErrorCode::InternalError)?.len()
                > super::protocol::MAX_CONTROL_BYTES - 4096 { return Err(ErrorCode::ResourceLimit); }
            Ok(result)
        })
        .map_err(|error| error.remote_code())?
}
