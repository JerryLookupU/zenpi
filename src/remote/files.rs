//! Read-only selected-workspace file discovery; contents are not projected.

use super::{
    auth::{AuthenticatedDevice, DeviceRegistry},
    protocol::{Envelope, ErrorCode, Message, Operation},
};
use crate::project_workspace::ProjectOwnerPool;
use serde_json::{Value, json};

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
        operation: Operation::FilesList,
        arguments,
    } = &request.payload
    else {
        return Err(ErrorCode::InvalidRequest);
    };
    let args = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
    let paged = args.contains_key("offset") || args.contains_key("snapshot_id");
    if args.len() != if paged { 5 } else { 3 }
        || !["subtab_id", "subtab_revision", "path"]
            .iter()
            .all(|key| args.contains_key(*key))
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let (offset, snapshot) = if paged {
        let offset = args
            .get("offset")
            .and_then(Value::as_u64)
            .filter(|offset| *offset <= 2048 && *offset % 64 == 0)
            .ok_or(ErrorCode::InvalidRequest)? as usize;
        let snapshot = args.get("snapshot_id").ok_or(ErrorCode::InvalidRequest)?;
        let snapshot = if snapshot.is_null() && offset == 0 {
            None
        } else {
            Some(
                snapshot
                    .as_str()
                    .filter(|value| {
                        value.len() == 64
                            && value
                                .bytes()
                                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
                    })
                    .ok_or(ErrorCode::InvalidRequest)?,
            )
        };
        (offset, snapshot)
    } else {
        (0, None)
    };
    let tab = args["subtab_id"]
        .as_str()
        .ok_or(ErrorCode::InvalidRequest)?;
    let revision = args["subtab_revision"]
        .as_u64()
        .ok_or(ErrorCode::InvalidRequest)?;
    let path = args["path"]
        .as_str()
        .filter(|path| {
            !path.is_empty() && path.len() <= 4096 && !path.chars().any(char::is_control)
        })
        .ok_or(ErrorCode::InvalidRequest)?;
    let project = request
        .project_id
        .as_deref()
        .ok_or(ErrorCode::InvalidRequest)?;
    registry
        .with_authorized_projects(device, &[project], Operation::FilesList, |_| {
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
            if state.active_id != tab {
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
            let mut result = if paged {
                crate::slash_actions::prompt_files_catalog_at(&execution.tab.root, path)
            } else {
                crate::slash_actions::remote_prompt_files_at(&execution.tab.root, path)
            }
            .map_err(|_| ErrorCode::InvalidRequest)?;
            if paged {
                if result["truncated"] == true {
                    return Err(ErrorCode::ResourceLimit);
                }
                use sha2::{Digest, Sha256};
                let digest = format!(
                    "{:x}",
                    Sha256::digest(
                        serde_json::to_vec(&json!({"project":project,"session":context.session_id,
                    "subtab":tab,"revision":revision,"directory":result}))
                        .map_err(|_| ErrorCode::InternalError)?
                    )
                );
                if snapshot.is_some_and(|expected| expected != digest) {
                    return Err(ErrorCode::RevisionConflict);
                }
                let entries = result["entries"]
                    .as_array()
                    .ok_or(ErrorCode::InternalError)?;
                if offset > 0 && offset >= entries.len() {
                    return Err(ErrorCode::InvalidRequest);
                }
                let end = (offset + 64).min(entries.len());
                let page = entries[offset..end].to_vec();
                let next = if end < entries.len() { Some(end) } else { None };
                result["entries"] = json!(page);
                result["page"] = json!({"snapshot_id":digest,"offset":offset,"next_offset":next});
            }
            result["project_id"] = json!(project);
            result["session_id"] = json!(context.session_id);
            result["subtab_id"] = json!(tab);
            result["subtab_revision"] = json!(revision);
            if serde_json::to_vec(&result)
                .map_err(|_| ErrorCode::InternalError)?
                .len()
                > 64 * 1024
            {
                return Err(ErrorCode::ResourceLimit);
            }
            Ok(result)
        })
        .map_err(|error| error.remote_code())?
}
