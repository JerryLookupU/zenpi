//! Remote session replacement reuses the canonical owner's transaction.

use super::{
    auth::{AuthenticatedDevice, DeviceRegistry},
    protocol::{Envelope, ErrorCode, ExecutionState, Message, Operation},
};
use crate::project_workspace::ProjectOwnerPool;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

struct Catalog {
    entries: Vec<(std::path::PathBuf, Value)>,
    digest: String,
}

pub fn projected_name(session: &crate::session::SessionStore) -> Option<String> {
    session
        .session_name()
        .filter(|name| {
            crate::session::validate_session_name(name).is_ok()
                && crate::security::redact_text(name, &[]) == *name
                && !name.contains('/')
                && !name.contains('\\')
        })
        .map(str::to_owned)
}

fn catalog(
    pool: &ProjectOwnerPool,
    project: &str,
    query: Option<&str>,
) -> Result<Catalog, ErrorCode> {
    let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
    let workspace = std::path::Path::new(&context.cwd)
        .canonicalize()
        .map_err(|_| ErrorCode::InvalidRequest)?;
    let path = pool
        .sessions()
        .get(project)
        .ok_or(ErrorCode::InvalidRequest)?;
    let directory = path.parent().ok_or(ErrorCode::InvalidRequest)?;
    if std::fs::symlink_metadata(directory)
        .map_err(|_| ErrorCode::InvalidRequest)?
        .file_type()
        .is_symlink()
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let mut paths = Vec::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    for (index, entry) in std::fs::read_dir(directory)
        .map_err(|_| ErrorCode::InvalidRequest)?
        .enumerate()
    {
        if index >= 4096 || std::time::Instant::now() >= deadline {
            return Err(ErrorCode::ResourceLimit);
        }
        let entry = entry.map_err(|_| ErrorCode::InvalidRequest)?;
        if entry.path().extension().and_then(|s| s.to_str()) == Some("jsonl")
            && entry.file_name() != crate::domain_store::DOMAIN_STORE_FILE_NAME
        {
            let metadata =
                std::fs::symlink_metadata(entry.path()).map_err(|_| ErrorCode::InvalidRequest)?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(ErrorCode::InvalidRequest);
            }
            paths.push(entry.path());
        }
    }
    if paths.len() > 512 {
        return Err(ErrorCode::ResourceLimit);
    }
    paths.sort();
    let mut entries = Vec::new();
    let mut bytes = 0u64;
    let mut digest = Sha256::new();
    digest.update(project.as_bytes());
    digest.update(context.session_id.as_bytes());
    let mut ids = std::collections::BTreeSet::new();
    for path in paths {
        if std::time::Instant::now() >= deadline {
            return Err(ErrorCode::ResourceLimit);
        }
        let length = std::fs::symlink_metadata(&path)
            .map_err(|_| ErrorCode::InvalidRequest)?
            .len();
        bytes = bytes.saturating_add(length);
        if length > 8 * 1024 * 1024 || bytes > 64 * 1024 * 1024 {
            return Err(ErrorCode::ResourceLimit);
        }
        let session = crate::session::SessionStore::open_existing_bounded(&path, 8 * 1024 * 1024)
            .map_err(|_| ErrorCode::InvalidRequest)?;
        // Inspection tolerates damaged journals for local recovery. Remote resume
        // must not offer a synthetic header or a partially recovered conversation.
        if !session
            .records()
            .first()
            .is_some_and(|record| record.kind == "session" && record.sequence == 0)
            || !session.recovery_warnings().is_empty()
        {
            return Err(ErrorCode::InvalidRequest);
        }
        if std::path::Path::new(&session.header().cwd)
            .canonicalize()
            .ok()
            .as_ref()
            != Some(&workspace)
        {
            continue;
        }
        let summary = session.summary();
        if !super::auth::valid_identifier(&summary.session_id)
            || !ids.insert(summary.session_id.clone())
        {
            return Err(ErrorCode::InvalidRequest);
        }
        let mut value = json!({"session_id":summary.session_id,"created_at_ms":summary.created_at_ms,"turn_count":summary.turn_count,
            "current":summary.session_id == context.session_id,"name":projected_name(&session)});
        digest.update(path.to_string_lossy().as_bytes());
        digest.update(serde_json::to_vec(&value).map_err(|_| ErrorCode::InternalError)?);
        for record in session.records() {
            digest
                .update(serde_json::to_vec(&record.value()).map_err(|_| ErrorCode::InternalError)?);
        }
        // Hash the complete catalog, but search only public conversation text.
        // Redact before matching so a query cannot probe a hidden credential.
        if let Some(query) = query {
            let needle = query.to_lowercase();
            let mut matches = 0usize;
            let mut snippets = Vec::new();
            for turn in session.turns() {
                if std::time::Instant::now() >= deadline {
                    return Err(ErrorCode::ResourceLimit);
                }
                if needle.is_empty()
                    || !matches!(
                        turn.role,
                        crate::core::TurnRole::User | crate::core::TurnRole::Assistant
                    )
                {
                    continue;
                }
                let text = crate::security::redact_text(&turn.content, &[]);
                if text.to_lowercase().contains(&needle) {
                    matches += 1;
                    if snippets.len() < 4 {
                        snippets.push(
                            text.chars()
                                .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
                                .take(240)
                                .collect::<String>(),
                        );
                    }
                }
            }
            value["matches"] = json!(matches);
            value["snippets"] = json!(snippets);
        }
        entries.push((path, value));
    }
    entries.sort_by(|a, b| {
        b.1["created_at_ms"]
            .as_u64()
            .cmp(&a.1["created_at_ms"].as_u64())
            .then(a.1["session_id"].as_str().cmp(&b.1["session_id"].as_str()))
    });
    Ok(Catalog {
        entries,
        digest: format!("{:x}", digest.finalize()),
    })
}

pub fn listing(
    pool: &ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
) -> Result<Value, ErrorCode> {
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
        operation: Operation::SessionList,
        arguments,
    } = &request.payload
    else {
        return Err(ErrorCode::InvalidRequest);
    };
    let args = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
    let query = args
        .get("query")
        .map(|value| {
            value
                .as_str()
                .filter(|text| {
                    text.len() <= 128
                        && text.trim() == *text
                        && !text.chars().any(char::is_control)
                        && crate::security::redact_text(text, &[]) == *text
                })
                .ok_or(ErrorCode::InvalidRequest)
        })
        .transpose()?;
    let page_args = args.len() - usize::from(query.is_some());
    let (offset, expected) = if page_args == 0 {
        (0, None)
    } else {
        if page_args != 2 {
            return Err(ErrorCode::InvalidRequest);
        }
        (
            args.get("offset")
                .and_then(Value::as_u64)
                .filter(|n| *n <= 512 && *n % 16 == 0)
                .ok_or(ErrorCode::InvalidRequest)? as usize,
            Some(
                args.get("snapshot_id")
                    .and_then(Value::as_str)
                    .ok_or(ErrorCode::InvalidRequest)?,
            ),
        )
    };
    let project = request
        .project_id
        .as_deref()
        .ok_or(ErrorCode::InvalidRequest)?;
    registry.with_authorized_projects(device, &[project], Operation::SessionList, |_| {
        let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
        if request.session_id.as_deref()!=Some(context.session_id.as_str()) { return Err(ErrorCode::OwnerMismatch); }
        let mut catalog = catalog(pool, project, query)?;
        if expected.is_some_and(|expected| expected!=catalog.digest) { return Err(ErrorCode::RevisionConflict); }
        if let Some(query) = query {
            let query = query.to_lowercase();
            catalog.entries.retain(|(_, entry)| entry["session_id"].as_str().is_some_and(|id| id.to_lowercase().contains(&query))
                || entry["name"].as_str().is_some_and(|name| name.to_lowercase().contains(&query))
                || entry["matches"].as_u64().is_some_and(|count| count > 0));
        }
        if offset>0 && offset>=catalog.entries.len() { return Err(ErrorCode::InvalidRequest); }
        let end = (offset+16).min(catalog.entries.len());
        let mut result = json!({"project_id":project,"session_id":context.session_id,"snapshot_id":catalog.digest,"offset":offset,
            "next_offset":if end<catalog.entries.len() {Some(end)}else{None},"entries":catalog.entries[offset..end].iter().map(|entry|entry.1.clone()).collect::<Vec<_>>()});
        if let Some(query) = query { result["query"] = json!(query); }
        Ok(result)
    }).map_err(|error|error.remote_code())?
}

pub fn new_session(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
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
        let resume = match operation {
            Operation::SessionNew if args.is_empty() => None,
            Operation::SessionResume if args.len() == 2 => Some((
                args.get("target_session_id")
                    .and_then(Value::as_str)
                    .filter(|id| super::auth::valid_identifier(id))
                    .ok_or(ErrorCode::InvalidRequest)?,
                args.get("snapshot_id")
                    .and_then(Value::as_str)
                    .filter(|id| {
                        id.len() == 64
                            && id
                                .bytes()
                                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    })
                    .ok_or(ErrorCode::InvalidRequest)?,
            )),
            _ => return Err(ErrorCode::InvalidRequest),
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
        registry
            .with_authorized_projects(device, &[project], *operation, |_| {
                if let Some(result) = pool.workspace_receipt(device.id(), id, &fingerprint)? {
                    return Ok((
                        if result.is_null() {
                            ExecutionState::UnknownOutcome
                        } else {
                            ExecutionState::Succeeded
                        },
                        result,
                    ));
                }
                if request.required_owner.as_deref() != Some(pool.workspace_owner()) {
                    return Err(ErrorCode::OwnerMismatch);
                }
                if request.expected_revision != Some(pool.workspace_revision()) {
                    return Err(ErrorCode::RevisionConflict);
                }
                let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
                if request.session_id.as_deref() != Some(context.session_id.as_str()) {
                    return Err(ErrorCode::OwnerMismatch);
                }
                let tabs = pool
                    .subtab_state(project)
                    .map_err(|_| ErrorCode::InvalidRequest)?;
                if tabs.tabs.first().is_none_or(|tab| tab.id != tabs.active_id) {
                    return Err(ErrorCode::InvalidRequest);
                }
                let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
                let mut agent = owner.try_lock().map_err(|_| ErrorCode::Busy)?;
                let queue =
                    crate::input_queue::InputQueue::recover(agent.session(), Default::default())
                        .map_err(|_| ErrorCode::InternalError)?;
                if busy
                    || queue.has_pending()
                    || agent.phase() != crate::core::AgentPhase::Idle
                    || agent.input_port().has_pending()
                    || agent.input_port().is_preparing()
                    || !agent.pending_attachments().is_empty()
                    || !pool.worker_handles(project).is_empty()
                {
                    return Err(ErrorCode::Busy);
                }
                let target = if let Some((target, snapshot)) = resume {
                    if target == context.session_id {
                        return Err(ErrorCode::InvalidRequest);
                    }
                    let catalog = catalog(pool, project, None)?;
                    if catalog.digest != snapshot {
                        return Err(ErrorCode::RevisionConflict);
                    }
                    Some(
                        catalog
                            .entries
                            .into_iter()
                            .find(|entry| entry.1["session_id"] == target)
                            .ok_or(ErrorCode::InvalidRequest)?
                            .0,
                    )
                } else {
                    None
                };
                let plan =
                    match pool.remote_new_session_plan(project, device.id(), id, &fingerprint) {
                        Ok(plan) => plan,
                        Err(_) => return Ok((ExecutionState::UnknownOutcome, Value::Null)),
                    };
                if let Some(path) = target {
                    let owner_epoch = request.required_owner.as_deref().unwrap();
                    let expected_id = resume.unwrap().0;
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
                    if agent
                        .resume_session_with_commit_cancelled(
                            path,
                            &|| std::time::Instant::now() >= deadline,
                            |replacement| {
                                if replacement.session_id() != expected_id {
                                    return Err(crate::core::AgentError::InvalidTurn(
                                        "resume target identity changed".into(),
                                    ));
                                }
                                if catalog(pool, project, None)
                                    .map(|catalog| catalog.digest != resume.unwrap().1)
                                    .unwrap_or(true)
                                {
                                    return Err(crate::core::AgentError::InvalidTurn(
                                        "resume catalog changed during preparation".into(),
                                    ));
                                }
                                pool.commit_remote_resumed_session(
                                    project,
                                    replacement,
                                    device.id(),
                                    id,
                                    &fingerprint,
                                    owner_epoch,
                                )
                                .map_err(crate::core::AgentError::InvalidTurn)
                            },
                        )
                        .is_err()
                    {
                        return Ok((ExecutionState::UnknownOutcome, Value::Null));
                    }
                    pool.refresh_owner_control(project, &agent);
                    return Ok((
                        ExecutionState::Succeeded,
                        pool.workspace_receipt(device.id(), id, &fingerprint)?
                            .ok_or(ErrorCode::InternalError)?,
                    ));
                }
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
                let committed =
                    match plan.execute(&mut agent, &|| std::time::Instant::now() >= deadline) {
                        Ok(committed) => committed,
                        Err(_) => return Ok((ExecutionState::UnknownOutcome, Value::Null)),
                    };
                pool.accept_new_session(&committed);
                let result = pool
                    .workspace_receipt(device.id(), id, &fingerprint)?
                    .ok_or(ErrorCode::InternalError)?;
                Ok((ExecutionState::Succeeded, result))
            })
            .map_err(|error| error.remote_code())?
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

/// Rename only the currently owned journal. Bind before append so uncertain
/// disk outcomes can never be replayed as another rename.
pub fn rename(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    busy: bool,
) -> Envelope {
    const RECEIPT: &str = "remote_session_name_admission_v1";
    let mut response =
        super::host_adapter::HostAdapter::failure(request, ErrorCode::InvalidRequest);
    let outcome = (|| {
        request.validate().map_err(|_| ErrorCode::InvalidRequest)?;
        if request.turn_id.is_some() || request.sequence.is_some() {
            return Err(ErrorCode::InvalidRequest);
        }
        let Message::Request {
            operation: Operation::SessionRename,
            arguments,
        } = &request.payload
        else {
            return Err(ErrorCode::InvalidRequest);
        };
        let args = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
        if args.len() != 2 {
            return Err(ErrorCode::InvalidRequest);
        }
        let name = args
            .get("name")
            .and_then(Value::as_str)
            .ok_or(ErrorCode::InvalidRequest)?;
        if !crate::session::validate_session_name(name).is_ok_and(|checked| checked == name)
            || crate::security::redact_text(name, &[]) != name
            || name.contains('/')
            || name.contains('\\')
        {
            return Err(ErrorCode::InvalidRequest);
        }
        let revision = args
            .get("journal_revision")
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
        registry.with_authorized_projects(device, &[project], Operation::SessionRename, |_| {
            let bound = pool.owner_request_binding(device.id(), id, &fingerprint, Operation::SessionRename)?;
            let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
            let mut agent = match owner.try_lock() {
                Ok(agent) => agent,
                Err(_) if bound => return Ok((ExecutionState::UnknownOutcome, Value::Null)),
                Err(_) => return Err(ErrorCode::Busy),
            };
            if request.session_id.as_deref() != Some(agent.session().session_id()) { return Err(ErrorCode::OwnerMismatch); }
            let receipts: Vec<_> = agent.session().events().iter().filter(|event| event["type"] == RECEIPT).collect();
            if let Some(event) = receipts.iter().rev().find(|event| event["device_id"] == device.id() && event["request_id"] == id) {
                if event["fingerprint"] != fingerprint { return Err(ErrorCode::RequestIdConflict); }
                return Ok((if event["state"] == "succeeded" {ExecutionState::Succeeded} else {ExecutionState::UnknownOutcome}, event["result"].clone()));
            }
            if bound { return Ok((ExecutionState::UnknownOutcome, Value::Null)); }
            if receipts.len() >= 256 { return Err(ErrorCode::ResourceLimit); }
            if request.required_owner.as_deref() != Some(pool.workspace_owner()) { return Err(ErrorCode::OwnerMismatch); }
            if request.expected_revision != Some(pool.workspace_revision()) || revision != agent.session().next_sequence() { return Err(ErrorCode::RevisionConflict); }
            let tabs = pool.subtab_state(project).map_err(|_| ErrorCode::InvalidRequest)?;
            if tabs.tabs.first().is_none_or(|tab| tab.id != tabs.active_id) { return Err(ErrorCode::InvalidRequest); }
            if busy || agent.phase() != crate::core::AgentPhase::Idle || agent.input_port().has_pending()
                || agent.input_port().is_preparing() || !agent.pending_attachments().is_empty()
                || !pool.worker_handles(project).is_empty()
                || crate::input_queue::InputQueue::recover(agent.session(), Default::default()).map_err(|_| ErrorCode::InternalError)?.has_pending() {
                return Err(ErrorCode::Busy);
            }
            let all = catalog(pool, project, None)?;
            if all.entries.iter().any(|entry| entry.1["session_id"] != agent.session().session_id() && entry.1["name"] == name) { return Err(ErrorCode::RevisionConflict); }
            let receipt = |state: &str, result: Value| json!({"type":RECEIPT,"device_id":device.id(),"request_id":id,"fingerprint":fingerprint,"state":state,"result":result});
            if pool.bind_owner_request(device.id(), request, &fingerprint, Operation::SessionRename).is_err()
                || agent.session_mut().append_event(receipt("reserved", Value::Null)).is_err() {
                return Ok((ExecutionState::UnknownOutcome, Value::Null));
            }
            if agent.session_mut().rename(name).is_err() { return Ok((ExecutionState::UnknownOutcome, Value::Null)); }
            let result = json!({"request_id":id,"project_id":project,"session_id":request.session_id,"name":name,"journal_revision":agent.session().next_sequence()});
            if agent.session_mut().append_event(receipt("succeeded", result.clone())).is_err() { return Ok((ExecutionState::UnknownOutcome, Value::Null)); }
            Ok((ExecutionState::Succeeded, result))
        }).map_err(|error| error.remote_code())?
    })();
    match outcome {
        Ok((state, result)) => {
            response.code = None;
            response.execution_state = Some(state);
            response.session_id = request.session_id.clone();
            response.payload = Message::Response { result };
        }
        Err(code) => response.code = Some(code),
    }
    response
}
