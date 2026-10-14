//! Bounded projection and control of an already-owned desktop terminal.

use super::{
    auth::{AuthenticatedDevice, DeviceRegistry},
    protocol::{Envelope, ErrorCode, ExecutionState, Message, Operation},
};
use crate::{project_workspace::ProjectOwnerPool, pty_shell::PtyShell};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const RECEIPT: &str = "remote_terminal_admission_v1";

/// The owner thread supplies creation; retries resolve the durable receipt
/// without invoking the callback or replacing an existing terminal.
pub fn open(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    busy: bool,
    create: impl FnOnce(&std::path::Path) -> Result<String, ErrorCode>,
) -> Envelope {
    let outcome = (|| {
        request.validate().map_err(|_| ErrorCode::InvalidRequest)?;
        if request.turn_id.is_some() || request.sequence.is_some() {
            return Err(ErrorCode::InvalidRequest);
        }
        let Message::Request {
            operation: Operation::TerminalOpen,
            arguments,
        } = &request.payload
        else {
            return Err(ErrorCode::InvalidRequest);
        };
        let args = arguments
            .as_object()
            .filter(|args| args.len() == 2)
            .ok_or(ErrorCode::InvalidRequest)?;
        let subtab = args
            .get("subtab_id")
            .and_then(Value::as_str)
            .filter(|id| {
                id.len() == 64
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
            .ok_or(ErrorCode::InvalidRequest)?;
        let revision = args
            .get("subtab_revision")
            .and_then(Value::as_u64)
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
        registry.with_authorized_projects(device, &[project], Operation::TerminalOpen, |_| {
            let bound = pool.owner_request_binding(device.id(), id, &fingerprint, Operation::TerminalOpen)?;
            let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
            let mut agent = match owner.try_lock() {
                Ok(agent) => agent,
                Err(_) if bound => return Ok((ExecutionState::UnknownOutcome, Value::Null)),
                Err(_) => return Err(ErrorCode::Busy),
            };
            if request.session_id.as_deref() != Some(agent.session().session_id()) { return Err(ErrorCode::OwnerMismatch); }
            let receipts: Vec<_> = agent.session().events().iter().filter(|event| event["type"] == RECEIPT).collect();
            if let Some(event) = receipts.iter().rev().find(|event| event["device_id"] == device.id() && event["request_id"] == id) {
                if event["fingerprint"] != fingerprint || event["operation"] != json!(Operation::TerminalOpen) { return Err(ErrorCode::RequestIdConflict); }
                return Ok(if event["state"] == "succeeded" { (ExecutionState::Succeeded, event["result"].clone()) }
                    else { (ExecutionState::UnknownOutcome, Value::Null) });
            }
            if bound { return Ok((ExecutionState::UnknownOutcome, Value::Null)); }
            if receipts.len() >= 2048 { return Err(ErrorCode::ResourceLimit); }
            if request.required_owner.as_deref() != Some(pool.workspace_owner()) { return Err(ErrorCode::OwnerMismatch); }
            if request.expected_revision != Some(pool.workspace_revision()) { return Err(ErrorCode::RevisionConflict); }
            let tabs = pool.subtab_state(project).map_err(|_| ErrorCode::InvalidRequest)?;
            if tabs.revision != revision || tabs.active_id != subtab { return Err(ErrorCode::RevisionConflict); }
            let tab = tabs.tabs.iter().find(|tab| tab.id == subtab).ok_or(ErrorCode::InvalidRequest)?;
            if busy { return Err(ErrorCode::Busy); }
            let receipt = |state: &str, result: Value| json!({"type":RECEIPT,"device_id":device.id(),
                "request_id":id,"fingerprint":fingerprint,"operation":Operation::TerminalOpen,"state":state,"result":result});
            if pool.bind_owner_request(device.id(), request, &fingerprint, Operation::TerminalOpen).is_err()
                || agent.session_mut().append_event(receipt("reserved", Value::Null)).is_err() {
                return Ok((ExecutionState::UnknownOutcome, Value::Null));
            }
            let terminal = match create(std::path::Path::new(&tab.root)) {
                Ok(id) => id,
                Err(_) => return Ok((ExecutionState::UnknownOutcome, Value::Null)),
            };
            let result = json!({"request_id":id,"project_id":project,"session_id":request.session_id,
                "terminal_id":terminal,"subtab_id":subtab,"subtab_revision":revision});
            if agent.session_mut().append_event(receipt("succeeded", result.clone())).is_err() {
                return Ok((ExecutionState::UnknownOutcome, Value::Null));
            }
            super::wakeup::notify(agent.session().session_id());
            Ok((ExecutionState::Succeeded, result))
        }).map_err(|error| error.remote_code())?
    })();
    match outcome {
        Ok((state, result)) => {
            let mut response = Envelope::new(
                super::protocol::Channel::Control,
                Message::Response { result },
            );
            response.id = request.id.clone();
            response.project_id = request.project_id.clone();
            response.session_id = request.session_id.clone();
            response.execution_state = Some(state);
            response
        }
        Err(code) => super::host_adapter::HostAdapter::failure(request, code),
    }
}

pub fn dispatch(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    shell: Option<&mut PtyShell>,
    busy: bool,
) -> Envelope {
    let mut response =
        super::host_adapter::HostAdapter::failure(request, ErrorCode::InvalidRequest);
    let outcome = (|| {
        request.validate().map_err(|_| ErrorCode::InvalidRequest)?;
        if request.turn_id.is_some() || request.sequence.is_some() {
            return Err(ErrorCode::InvalidRequest);
        }
        let Message::Request {
            operation,
            arguments,
        } = &request.payload
        else {
            return Err(ErrorCode::InvalidRequest);
        };
        let args = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
        let hex = |name: &str| {
            args.get(name)
                .and_then(Value::as_str)
                .filter(|value| {
                    value.len() == 64
                        && value
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                })
                .ok_or(ErrorCode::InvalidRequest)
        };
        let terminal = hex("terminal_id")?;
        let subtab = hex("subtab_id")?;
        let revision = args
            .get("subtab_revision")
            .and_then(Value::as_u64)
            .ok_or(ErrorCode::InvalidRequest)?;
        let bytes = match operation {
            Operation::TerminalInput if args.len() == 4 => {
                let text = args
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or(ErrorCode::InvalidRequest)?;
                if text.is_empty() || text.len() > 4096 || text.contains('\0') {
                    return Err(ErrorCode::InvalidRequest);
                }
                Some(text.as_bytes())
            }
            Operation::TerminalResize if args.len() == 5 => None,
            _ => return Err(ErrorCode::InvalidRequest),
        };
        let size = if bytes.is_none() {
            let number = |name| {
                args.get(name)
                    .and_then(Value::as_u64)
                    .filter(|n| (1..=512).contains(n))
                    .map(|n| n as u16)
                    .ok_or(ErrorCode::InvalidRequest)
            };
            Some((number("rows")?, number("cols")?))
        } else {
            None
        };
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
                Err(_) if bound => return Ok((ExecutionState::UnknownOutcome, Value::Null)),
                Err(_) => return Err(ErrorCode::Busy),
            };
            if request.session_id.as_deref() != Some(agent.session().session_id()) { return Err(ErrorCode::OwnerMismatch); }
            let receipts: Vec<_> = agent.session().events().iter().filter(|event| event["type"] == RECEIPT).collect();
            if let Some(event) = receipts.iter().rev().find(|event| event["device_id"] == device.id() && event["request_id"] == id) {
                if event["fingerprint"] != fingerprint || event["operation"] != json!(operation) { return Err(ErrorCode::RequestIdConflict); }
                return Ok(if event["state"] == "succeeded" { (ExecutionState::Succeeded, event["result"].clone()) }
                    else { (ExecutionState::UnknownOutcome, Value::Null) });
            }
            if bound { return Ok((ExecutionState::UnknownOutcome, Value::Null)); }
            if receipts.len() >= 2048 { return Err(ErrorCode::ResourceLimit); }
            if request.required_owner.as_deref() != Some(pool.workspace_owner()) { return Err(ErrorCode::OwnerMismatch); }
            if request.expected_revision != Some(pool.workspace_revision()) { return Err(ErrorCode::RevisionConflict); }
            let tabs = pool.subtab_state(project).map_err(|_| ErrorCode::InvalidRequest)?;
            if tabs.revision != revision || tabs.active_id != subtab { return Err(ErrorCode::RevisionConflict); }
            let tab = tabs.tabs.iter().find(|tab| tab.id == subtab).ok_or(ErrorCode::InvalidRequest)?;
            let shell = shell.ok_or(ErrorCode::InvalidRequest)?;
            if shell.instance_id() != terminal || shell.cwd() != std::path::Path::new(&tab.root) { return Err(ErrorCode::OwnerMismatch); }
            if !shell.is_running() { return Err(ErrorCode::InvalidRequest); }
            if busy { return Err(ErrorCode::Busy); }
            let receipt = |state: &str, result: Value| json!({"type":RECEIPT,"device_id":device.id(),
                "request_id":id,"fingerprint":fingerprint,"operation":operation,"state":state,"result":result});
            if pool.bind_owner_request(device.id(), request, &fingerprint, *operation).is_err()
                || agent.session_mut().append_event(receipt("reserved", Value::Null)).is_err() {
                return Ok((ExecutionState::UnknownOutcome, Value::Null));
            }
            let effect = if let Some(bytes) = bytes { shell.try_write_input(bytes) }
                else { let (rows, cols) = size.unwrap(); shell.try_resize(rows, cols) };
            if effect.is_err() { return Ok((ExecutionState::UnknownOutcome, Value::Null)); }
            let mut result = json!({"request_id":id,"project_id":project,"session_id":request.session_id,
                "terminal_id":terminal,"subtab_id":subtab,"subtab_revision":revision});
            if let Some(bytes) = bytes { result["accepted_bytes"] = json!(bytes.len()); }
            else { let (rows, cols) = size.unwrap(); result["rows"] = json!(rows); result["cols"] = json!(cols); }
            if agent.session_mut().append_event(receipt("succeeded", result.clone())).is_err() {
                return Ok((ExecutionState::UnknownOutcome, Value::Null));
            }
            super::wakeup::notify(agent.session().session_id());
            Ok((ExecutionState::Succeeded, result))
        }).map_err(|error| error.remote_code())?
    })();
    match outcome {
        Ok((state, result)) => {
            response.execution_state = Some(state);
            response.code = None;
            response.session_id = request.session_id.clone();
            response.payload = Message::Response { result };
        }
        Err(code) => response.code = Some(code),
    }
    response
}

pub const MAX_TERMINAL_TEXT_BYTES: usize = 16 * 1024;
pub const MAX_HISTORY_BYTES: usize = 8 * 1024 * 1024;

/// Read the retained parser screen, never raw PTY bytes. A changed screen
/// refuses continuation rather than combining different terminal states.
pub fn read(
    pool: &ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    shell: Option<&PtyShell>,
) -> Envelope {
    let outcome = (|| {
        request.validate().map_err(|_| ErrorCode::InvalidRequest)?;
        if request.id.is_none()
            || request.required_owner.is_some()
            || request.expected_revision.is_some()
            || request.turn_id.is_some()
            || request.sequence.is_some()
        {
            return Err(ErrorCode::InvalidRequest);
        }
        let Message::Request {
            operation: Operation::TerminalRead,
            arguments,
        } = &request.payload
        else {
            return Err(ErrorCode::InvalidRequest);
        };
        let args = arguments
            .as_object()
            .filter(|args| args.len() == 5)
            .ok_or(ErrorCode::InvalidRequest)?;
        let hex = |name| {
            args.get(name)
                .and_then(Value::as_str)
                .filter(|s| {
                    s.len() == 64
                        && s.bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                })
                .ok_or(ErrorCode::InvalidRequest)
        };
        let terminal = hex("terminal_id")?;
        let subtab = hex("subtab_id")?;
        let revision = args
            .get("subtab_revision")
            .and_then(Value::as_u64)
            .ok_or(ErrorCode::InvalidRequest)?;
        let offset = args
            .get("offset")
            .and_then(Value::as_u64)
            .filter(|n| *n <= MAX_HISTORY_BYTES as u64)
            .ok_or(ErrorCode::InvalidRequest)? as usize;
        let digest = if args.get("digest").is_some_and(Value::is_null) {
            None
        } else {
            Some(hex("digest")?)
        };
        if offset != 0 && digest.is_none() {
            return Err(ErrorCode::InvalidRequest);
        }
        let project = request
            .project_id
            .as_deref()
            .ok_or(ErrorCode::InvalidRequest)?;
        registry.with_authorized_projects(device, &[project], Operation::TerminalRead, |_| {
            let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
            if request.session_id.as_deref() != Some(context.session_id.as_str()) { return Err(ErrorCode::OwnerMismatch); }
            let tabs = pool.subtab_state(project).map_err(|_| ErrorCode::InvalidRequest)?;
            if tabs.active_id != subtab || tabs.revision != revision { return Err(ErrorCode::RevisionConflict); }
            let tab = tabs.tabs.iter().find(|tab| tab.id == subtab).ok_or(ErrorCode::InvalidRequest)?;
            let shell = shell.ok_or(ErrorCode::InvalidRequest)?;
            if shell.instance_id() != terminal || shell.cwd() != std::path::Path::new(&tab.root) { return Err(ErrorCode::OwnerMismatch); }
            let raw = shell.remote_content(4000);
            if raw.len() > MAX_HISTORY_BYTES { return Err(ErrorCode::ResourceLimit); }
            let text = safe_text(&raw, &[std::path::Path::new(&tab.root), std::path::Path::new(&context.cwd)]);
            if text.len() > MAX_HISTORY_BYTES { return Err(ErrorCode::ResourceLimit); }
            let actual = format!("{:x}", Sha256::digest(text.as_bytes()));
            if digest.is_some_and(|digest| digest != actual) { return Err(ErrorCode::RevisionConflict); }
            if offset > text.len() || !text.is_char_boundary(offset) { return Err(ErrorCode::InvalidRequest); }
            let mut end = (offset + 8192).min(text.len());
            while !text.is_char_boundary(end) { end -= 1; }
            Ok(json!({"project_id":project,"session_id":context.session_id,"terminal_id":terminal,
                "subtab_id":subtab,"subtab_revision":revision,"digest":actual,"offset":offset,
                "next_offset":end,"total_bytes":text.len(),"has_more":end < text.len(),"text":&text[offset..end]}))
        }).map_err(|error| error.remote_code())?
    })();
    match outcome {
        Ok(result) => {
            let mut response = Envelope::new(
                super::protocol::Channel::Control,
                Message::Response { result },
            );
            response.id = request.id.clone();
            response.project_id = request.project_id.clone();
            response.session_id = request.session_id.clone();
            response.execution_state = Some(ExecutionState::Succeeded);
            response
        }
        Err(code) => super::host_adapter::HostAdapter::failure(request, code),
    }
}

pub fn screen(
    project: &str,
    session: &str,
    subtab: &str,
    revision: u64,
    text: &str,
    roots: &[&std::path::Path],
) -> serde_json::Value {
    let mut text = safe_text(text, roots);
    let truncated = text.len() > MAX_TERMINAL_TEXT_BYTES;
    if truncated {
        let mut end = MAX_TERMINAL_TEXT_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    serde_json::json!({"project_id":project, "session_id":session, "subtab_id":subtab,
        "subtab_revision":revision, "text":text, "truncated":truncated, "read_only":true})
}

fn safe_text(text: &str, roots: &[&std::path::Path]) -> String {
    let mut text = text.to_owned();
    for root in roots {
        let root = root.to_string_lossy();
        if !root.is_empty() {
            text = text.replace(root.as_ref(), ".");
        }
    }
    text.retain(|character| !character.is_control() || matches!(character, '\n' | '\t'));
    // A shell may echo a quoted secret separately from its assignment value.
    let mut private_block = false;
    text = text
        .lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            if lower.contains("-----begin ") && lower.contains("private key-----") {
                private_block = true;
            }
            if private_block {
                if lower.contains("-----end ") && lower.contains("private key-----") {
                    private_block = false;
                }
                return "<sensitive terminal line withheld>".to_owned();
            }
            if [
                "api_key",
                "api-key",
                "authorization",
                "cookie",
                "access_token",
                "refresh_token",
                "client_secret",
                "password",
                "private_key",
                "private key",
                "secret_key",
                "token=",
            ]
            .iter()
            .any(|marker| lower.contains(marker))
            {
                "<sensitive terminal line withheld>".to_owned()
            } else {
                crate::security::redact_text(line, &[])
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    text
}
