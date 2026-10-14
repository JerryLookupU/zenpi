//! Workspace CAS and durable retry receipts around the existing owner pool.

use super::{
    auth::{AuthenticatedDevice, DeviceRegistry},
    protocol::{Envelope, ErrorCode, Message, Operation},
};
use crate::project_workspace::{ProjectOwnerPool, ProjectWorkspace};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn mutate(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
) -> Result<Value, ErrorCode> {
    let Message::Request {
        operation,
        arguments,
    } = &request.payload
    else {
        return Err(ErrorCode::InvalidRequest);
    };
    if !matches!(
        operation,
        Operation::WorkspaceOpen | Operation::WorkspaceSelect | Operation::WorkspaceClose
    ) || request.validate().is_err()
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let id = request
        .project_id
        .as_deref()
        .ok_or(ErrorCode::InvalidRequest)?;
    // Request identity covers the exact owner/revision/session and arguments, not only its command name.
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(request).map_err(|_| ErrorCode::InternalError)?)
    );
    let request_id = request.id.as_deref().ok_or(ErrorCode::InvalidRequest)?;
    registry
        .with_authorized_projects(device, &[id], *operation, |authorization| {
            if let Some(result) = pool.workspace_receipt(device.id(), request_id, &fingerprint)? {
                return Ok(result);
            }
            if request.required_owner.as_deref() != Some(pool.workspace_owner()) {
                return Err(ErrorCode::OwnerMismatch);
            }
            if request.expected_revision != Some(pool.workspace_revision()) {
                return Err(ErrorCode::RevisionConflict);
            }
            if !pool.has_workspace_checkpoint() {
                return Err(ErrorCode::InvalidRequest);
            }
            let context = pool.context(id).ok_or(ErrorCode::InvalidRequest)?;
            if request.session_id.as_deref() != Some(context.session_id.as_str()) {
                return Err(ErrorCode::OwnerMismatch);
            }
            let values = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
            let candidate: ProjectWorkspace = match operation {
                Operation::WorkspaceOpen => {
                    if values.len() != 1 {
                        return Err(ErrorCode::InvalidRequest);
                    }
                    let cwd = values
                        .get("cwd")
                        .and_then(Value::as_str)
                        .ok_or(ErrorCode::InvalidRequest)?;
                    if cwd.trim().is_empty() {
                        return Err(ErrorCode::InvalidRequest);
                    }
                    let base = pool
                        .workspace()
                        .tabs()
                        .iter()
                        .find(|tab| tab.id().as_str() == id)
                        .ok_or(ErrorCode::InvalidRequest)?
                        .cwd();
                    pool.workspace()
                        .with_directory(Some(Path::new(cwd)), base)
                        .map_err(|_| ErrorCode::InvalidRequest)?
                        .0
                }
                Operation::WorkspaceSelect | Operation::WorkspaceClose => {
                    if !values.is_empty() {
                        return Err(ErrorCode::InvalidRequest);
                    }
                    let tab = pool
                        .workspace()
                        .tabs()
                        .iter()
                        .find(|tab| tab.id().as_str() == id)
                        .ok_or(ErrorCode::InvalidRequest)?;
                    if *operation == Operation::WorkspaceSelect {
                        pool.workspace().with_active(tab.id())
                    } else {
                        pool.workspace().without_project(tab.id())
                    }
                    .map_err(|_| ErrorCode::InvalidRequest)?
                }
                _ => return Err(ErrorCode::InvalidRequest),
            };
            let target = candidate
                .active()
                .ok_or(ErrorCode::InvalidRequest)?
                .id()
                .as_str();
            // Opening a canonical path never grants that project implicitly; a local grant is required first.
            authorization
                .authorize(target, Operation::WorkspaceSelect)
                .map_err(|_| ErrorCode::Unauthorized)?;
            authorization
                .authorize(target, Operation::Inspect)
                .map_err(|_| ErrorCode::Unauthorized)?;
            if *operation == Operation::WorkspaceClose {
                let owner = pool.owner(id).ok_or(ErrorCode::InvalidRequest)?;
                let guard = owner.try_lock().map_err(|_| ErrorCode::Busy)?;
                if guard.phase() == crate::core::AgentPhase::Running
                    || guard.input_port().has_pending()
                    || guard.input_port().is_preparing()
                {
                    return Err(ErrorCode::Busy);
                }
            }
            // Pool publication fences replaced owners again across the durable commit.
            let result = pool
                .publish_remote_workspace(candidate, device.id(), request_id, &fingerprint)
                .map_err(|_| ErrorCode::InternalError)?;
            Ok(result)
        })
        .map_err(|error| error.remote_code())?
}
