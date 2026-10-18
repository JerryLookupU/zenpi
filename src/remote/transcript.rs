//! Read-only pages of a selected durable turn, redacted before byte slicing.

use super::{
    auth::{AuthenticatedDevice, DeviceRegistry},
    protocol::{Envelope, ErrorCode, Message, Operation},
};
use crate::project_workspace::ProjectOwnerPool;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const PAGE_BYTES: usize = 8192;
pub const MAX_CONTENT_BYTES: usize = 4 * 1024 * 1024;

/// Public invocation summaries only; policy/execution metadata never leaves
/// the host. Shared by ordinary transcript replay and worker history.
pub(crate) fn tool_call_projection(turn: &crate::core::Turn) -> Result<Option<Value>, ErrorCode> {
    let Some(calls) = turn
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("tool_calls"))
    else {
        return Ok(None);
    };
    if turn.role != crate::core::TurnRole::Assistant {
        return Err(ErrorCode::InvalidRequest);
    }
    let calls = calls.as_array().ok_or(ErrorCode::InvalidRequest)?;
    let mut projected = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    for call in calls.iter().take(8) {
        let id = call["id"]
            .as_str()
            .filter(|id| super::auth::valid_identifier(id))
            .ok_or(ErrorCode::InvalidRequest)?;
        if !ids.insert(id) {
            return Err(ErrorCode::InvalidRequest);
        }
        let name = call["name"]
            .as_str()
            .filter(|name| super::auth::valid_identifier(name) && name.len() <= 64)
            .ok_or(ErrorCode::InvalidRequest)?;
        let arguments = call.get("arguments").ok_or(ErrorCode::InvalidRequest)?;
        if serde_json::to_vec(arguments)
            .map_err(|_| ErrorCode::InternalError)?
            .len()
            > 64 * 1024
        {
            return Err(ErrorCode::ResourceLimit);
        }
        let safe = serde_json::to_string(&crate::security::redact_json(arguments, &[]))
            .map_err(|_| ErrorCode::InternalError)?;
        let mut end = safe.len().min(512);
        while !safe.is_char_boundary(end) {
            end -= 1;
        }
        projected.push(json!({"id":id,"name":name,"arguments_preview":&safe[..end],
            "arguments_truncated":end < safe.len()}));
    }
    Ok(Some(
        json!({"tool_calls":projected,"tool_calls_truncated":calls.len() > projected.len()}),
    ))
}

pub(crate) fn history_entry(turn: &crate::core::Turn) -> Result<Value, ErrorCode> {
    Ok(json!({"id":turn.id,"role":turn.role,
        "digest":format!("{:x}", Sha256::digest(turn.content.as_bytes())),
        "metadata_digest":format!("{:x}", Sha256::digest(serde_json::to_vec(&turn.metadata).map_err(|_| ErrorCode::InternalError)?))}))
}

fn content_page(
    turn: &crate::core::Turn,
    offset: usize,
    digest: Option<&str>,
) -> Result<Value, ErrorCode> {
    if turn.content.len() > MAX_CONTENT_BYTES || turn.content.contains('\0') {
        return Err(ErrorCode::ResourceLimit);
    }
    let content = crate::security::redact_text(&turn.content, &[]);
    text_page(turn, &content, offset, digest)
}

fn text_page(
    turn: &crate::core::Turn,
    content: &str,
    offset: usize,
    digest: Option<&str>,
) -> Result<Value, ErrorCode> {
    if content.len() > MAX_CONTENT_BYTES {
        return Err(ErrorCode::ResourceLimit);
    }
    let actual = format!("{:x}", Sha256::digest(content.as_bytes()));
    if digest.is_some_and(|digest| digest != actual) {
        return Err(ErrorCode::RevisionConflict);
    }
    if offset > content.len() || !content.is_char_boundary(offset) {
        return Err(ErrorCode::InvalidRequest);
    }
    let mut end = offset.saturating_add(PAGE_BYTES).min(content.len());
    while !content.is_char_boundary(end) {
        end -= 1;
    }
    Ok(
        json!({"turn_id":turn.id,"role":turn.role,"digest":actual,"offset":offset,"next_offset":end,
        "total_bytes":content.len(),"has_more":end < content.len(),"text":&content[offset..end]}),
    )
}

fn tool_arguments_page(
    turn: &crate::core::Turn,
    call_id: &str,
    offset: usize,
    digest: Option<&str>,
) -> Result<Value, ErrorCode> {
    if turn.role != crate::core::TurnRole::Assistant {
        return Err(ErrorCode::InvalidRequest);
    }
    let calls = turn
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("tool_calls"))
        .and_then(Value::as_array)
        .ok_or(ErrorCode::InvalidRequest)?;
    let mut matching = calls.iter().filter(|call| call["id"] == call_id);
    let call = matching.next().ok_or(ErrorCode::InvalidRequest)?;
    if matching.next().is_some() {
        return Err(ErrorCode::InvalidRequest);
    }
    let name = call["name"]
        .as_str()
        .filter(|name| super::auth::valid_identifier(name) && name.len() <= 64)
        .ok_or(ErrorCode::InvalidRequest)?;
    let arguments = call.get("arguments").ok_or(ErrorCode::InvalidRequest)?;
    if serde_json::to_vec(arguments)
        .map_err(|_| ErrorCode::InternalError)?
        .len()
        > 64 * 1024
    {
        return Err(ErrorCode::ResourceLimit);
    }
    let safe = serde_json::to_string(&crate::security::redact_json(arguments, &[]))
        .map_err(|_| ErrorCode::InternalError)?;
    if safe.len() > 64 * 1024 {
        return Err(ErrorCode::ResourceLimit);
    }
    let mut page = text_page(turn, &safe, offset, digest)?;
    page["tool_call_id"] = json!(call_id);
    page["tool_name"] = json!(name);
    Ok(page)
}

pub fn read(
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
        operation: Operation::TranscriptRead,
        arguments,
    } = &request.payload
    else {
        return Err(ErrorCode::InvalidRequest);
    };
    let args = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
    if args.get("view").and_then(Value::as_str) == Some("session_history") {
        return session_history(pool, registry, device, request, args);
    }
    let worker = args.contains_key("request_id");
    let journal_revision = args
        .get("journal_revision")
        .map(|value| value.as_u64().ok_or(ErrorCode::InvalidRequest))
        .transpose()?;
    if worker && journal_revision.is_some() {
        return Err(ErrorCode::InvalidRequest);
    }
    let history = worker && args.get("view").and_then(Value::as_str) == Some("history");
    let selected_turn = if worker && args.contains_key("turn_id") {
        Some(
            args["turn_id"]
                .as_str()
                .filter(|id| super::auth::valid_identifier(id))
                .ok_or(ErrorCode::InvalidRequest)?,
        )
    } else {
        None
    };
    let tool_call = if args.contains_key("tool_call_id") {
        Some(
            args["tool_call_id"]
                .as_str()
                .filter(|id| super::auth::valid_identifier(id))
                .ok_or(ErrorCode::InvalidRequest)?,
        )
    } else {
        None
    };
    let identity_field = if worker { "request_id" } else { "turn_id" };
    if args.len()
        != 3 + usize::from(history || selected_turn.is_some())
            + usize::from(tool_call.is_some())
            + usize::from(journal_revision.is_some())
        || tool_call.is_some() && worker && selected_turn.is_none()
        || args.contains_key("view") && !history
        || history && selected_turn.is_some()
        || ![identity_field, "offset", "digest"]
            .iter()
            .all(|key| args.contains_key(*key))
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let turn_id = args[identity_field]
        .as_str()
        .filter(|id| super::auth::valid_identifier(id))
        .ok_or(ErrorCode::InvalidRequest)?;
    let offset = args["offset"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or(ErrorCode::InvalidRequest)?;
    let digest = if args["digest"].is_null() {
        None
    } else {
        Some(
            args["digest"]
                .as_str()
                .filter(|value| {
                    value.len() == 64
                        && value
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                })
                .ok_or(ErrorCode::InvalidRequest)?,
        )
    };
    if offset != 0 && digest.is_none() {
        return Err(ErrorCode::InvalidRequest);
    }
    let project = request
        .project_id
        .as_deref()
        .ok_or(ErrorCode::InvalidRequest)?;
    registry
        .with_authorized_projects(device, &[project], Operation::TranscriptRead, |_| {
            let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
            if request.session_id.as_deref() != Some(context.session_id.as_str()) {
                return Err(ErrorCode::OwnerMismatch);
            }
            let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
            let agent = owner.try_lock().map_err(|_| ErrorCode::Busy)?;
            if agent.session().session_id() != context.session_id {
                return Err(ErrorCode::OwnerMismatch);
            }
            if journal_revision.is_some_and(|revision| agent.session().next_sequence() != revision)
            {
                return Err(ErrorCode::RevisionConflict);
            }
            if worker {
                let event = agent
                    .session()
                    .events()
                    .iter()
                    .rev()
                    .find(|event| {
                        event["type"] == "remote_worker_progress_v1"
                            && event["device_id"] == device.id()
                            && event["request_id"] == turn_id
                            && event["project_id"] == project
                            && event["session_id"] == context.session_id
                    })
                    .ok_or(ErrorCode::InvalidRequest)?;
                if event["state"] != "succeeded" {
                    return Err(ErrorCode::Busy);
                }
                let output_id = event["output_turn_id"]
                    .as_str()
                    .filter(|id| super::auth::valid_identifier(id))
                    .ok_or(ErrorCode::InvalidRequest)?;
                let subtab = event["subtab_id"]
                    .as_str()
                    .filter(|id| super::auth::valid_identifier(id))
                    .ok_or(ErrorCode::InvalidRequest)?;
                let directory = agent
                    .session()
                    .path()
                    .parent()
                    .ok_or(ErrorCode::InternalError)?
                    .join(".zenpi/workers");
                let slot = event["worker_slot"]
                    .as_u64()
                    .filter(|slot| *slot < u64::from(crate::runtime::MAX_CONCURRENCY))
                    .ok_or(ErrorCode::InvalidRequest)? as u16;
                let worker_session = event["worker_session_id"]
                    .as_str()
                    .filter(|id| super::auth::valid_identifier(id))
                    .ok_or(ErrorCode::InvalidRequest)?;
                // Resolve only the runtime-confirmed slot; unrelated journals are not read.
                {
                    let key =
                        crate::worktree::worker::key(project, &context.session_id, subtab, slot)
                            .map_err(|_| ErrorCode::InvalidRequest)?;
                    let path = directory.join(format!("{key}.jsonl"));
                    let journal = crate::worktree::worker::read_existing_journal(&path)
                        .map_err(|_| ErrorCode::ResourceLimit)?;
                    if journal.session_id() != worker_session {
                        return Err(ErrorCode::InvalidRequest);
                    }
                    if let Some(turn) = journal.turns().iter().find(|turn| turn.id == output_id) {
                        if turn.role != crate::core::TurnRole::Assistant
                            || journal
                                .turns()
                                .iter()
                                .filter(|turn| turn.id == output_id)
                                .count()
                                != 1
                        {
                            return Err(ErrorCode::InvalidRequest);
                        }
                        let binding = journal
                            .events()
                            .iter()
                            .filter(|binding| {
                                binding["type"] == "remote_worker_output_v1"
                                    && binding["output_turn_id"] == output_id
                            })
                            .collect::<Vec<_>>();
                        if binding.len() != 1 {
                            return Err(ErrorCode::InvalidRequest);
                        }
                        let binding = binding[0];
                        let fingerprint = event["request_fingerprint"]
                            .as_str()
                            .filter(|value| {
                                value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
                            })
                            .ok_or(ErrorCode::InvalidRequest)?;
                        if binding["device_id"] != device.id()
                            || binding["request_id"] != turn_id
                            || binding["project_id"] != project
                            || binding["session_id"] != context.session_id
                            || binding["subtab_id"] != subtab
                            || binding["worker_session_id"] != journal.session_id()
                            || binding["worker_key"] != key
                            || binding["worker_slot"] != json!(slot)
                            || binding["request_fingerprint"] != fingerprint
                            || binding["output_digest"]
                                != format!("{:x}", Sha256::digest(turn.content.as_bytes()))
                            || binding["output_digest"] != event["output_digest"]
                        {
                            return Err(ErrorCode::InvalidRequest);
                        }
                        let mut result = if history || selected_turn.is_some() {
                            let entries = binding["history"]
                                .as_array()
                                .filter(|entries| !entries.is_empty() && entries.len() <= 1024)
                                .ok_or(ErrorCode::ResourceLimit)?;
                            let manifest_digest = format!(
                                "{:x}",
                                Sha256::digest(
                                    serde_json::to_vec(entries)
                                        .map_err(|_| ErrorCode::InternalError)?
                                )
                            );
                            if binding["history_digest"] != manifest_digest
                                || event["history_digest"] != manifest_digest
                            {
                                return Err(ErrorCode::InvalidRequest);
                            }
                            let input_id = binding["input_turn_id"]
                                .as_str()
                                .ok_or(ErrorCode::InvalidRequest)?;
                            if binding["input_turn_id"] != event["input_turn_id"] {
                                return Err(ErrorCode::InvalidRequest);
                            }
                            let start = journal
                                .turns()
                                .iter()
                                .position(|turn| turn.id == input_id)
                                .ok_or(ErrorCode::InvalidRequest)?;
                            let turns = journal
                                .turns()
                                .get(start..start + entries.len())
                                .ok_or(ErrorCode::InvalidRequest)?;
                            if turns.first().unwrap().role != crate::core::TurnRole::User
                                || turns.last().unwrap().id != output_id
                            {
                                return Err(ErrorCode::InvalidRequest);
                            }
                            let mut ids = std::collections::BTreeSet::new();
                            for (entry, turn) in entries.iter().zip(turns) {
                                if *entry != history_entry(turn)?
                                    || !ids.insert(&turn.id)
                                    || turn.id != input_id
                                        && turn.parent_id.as_deref() != Some(input_id)
                                {
                                    return Err(ErrorCode::InvalidRequest);
                                }
                            }
                            if let Some(id) = selected_turn {
                                let selected = turns
                                    .iter()
                                    .find(|turn| turn.id == id)
                                    .ok_or(ErrorCode::InvalidRequest)?;
                                if let Some(call_id) = tool_call {
                                    tool_arguments_page(selected, call_id, offset, digest)?
                                } else {
                                    content_page(selected, offset, digest)?
                                }
                            } else {
                                history_page(turns, offset, digest)?
                            }
                        } else {
                            content_page(turn, offset, digest)?
                        };
                        result["project_id"] = json!(project);
                        result["session_id"] = json!(context.session_id);
                        result["request_id"] = json!(turn_id);
                        result["subtab_id"] = json!(subtab);
                        return Ok(result);
                    }
                }
                return Err(ErrorCode::InvalidRequest);
            }
            // Check branch membership without cloning every durable message for a page.
            if agent.session().tree_enabled() {
                let tree = agent
                    .session()
                    .tree_snapshot(&|| false)
                    .map_err(|_| ErrorCode::InternalError)?;
                let path = tree
                    .ancestry(tree.active_leaf(), &|| false)
                    .map_err(|_| ErrorCode::InternalError)?;
                if !path.iter().any(|entry| entry.turn_id == turn_id) {
                    return Err(ErrorCode::InvalidRequest);
                }
            }
            let turn = agent
                .history()
                .iter()
                .find(|turn| turn.id == turn_id)
                .ok_or(ErrorCode::InvalidRequest)?;
            let mut result = if let Some(call_id) = tool_call {
                tool_arguments_page(turn, call_id, offset, digest)?
            } else {
                content_page(turn, offset, digest)?
            };
            result["project_id"] = json!(project);
            result["session_id"] = json!(context.session_id);
            if let Some(revision) = journal_revision {
                result["journal_revision"] = json!(revision);
            }
            Ok(result)
        })
        .map_err(|error| error.remote_code())?
}

fn session_history(
    pool: &ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    args: &serde_json::Map<String, Value>,
) -> Result<Value, ErrorCode> {
    if args.len() != 4 {
        return Err(ErrorCode::InvalidRequest);
    }
    let offset = args
        .get("offset")
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or(ErrorCode::InvalidRequest)?;
    let revision = args
        .get("journal_revision")
        .and_then(Value::as_u64)
        .ok_or(ErrorCode::InvalidRequest)?;
    let digest = match args.get("digest") {
        Some(Value::Null) if offset == 0 => None,
        Some(Value::String(value))
            if value.len() == 64
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
        {
            Some(value.as_str())
        }
        _ => return Err(ErrorCode::InvalidRequest),
    };
    let project = request
        .project_id
        .as_deref()
        .ok_or(ErrorCode::InvalidRequest)?;
    registry
        .with_authorized_projects(device, &[project], Operation::TranscriptRead, |_| {
            let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
            if request.session_id.as_deref() != Some(context.session_id.as_str()) {
                return Err(ErrorCode::OwnerMismatch);
            }
            let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
            let agent = owner.try_lock().map_err(|_| ErrorCode::Busy)?;
            if agent.session().session_id() != context.session_id {
                return Err(ErrorCode::OwnerMismatch);
            }
            if agent.session().next_sequence() != revision {
                return Err(ErrorCode::RevisionConflict);
            }
            // The active branch, not the physical audit journal, is the conversation.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
            if agent.session().turns().len() > 16384 {
                return Err(ErrorCode::ResourceLimit);
            }
            // Borrow durable turns: a read must not clone/serialize the entire
            // conversation into another large in-memory buffer on every page.
            let all = agent.session().turns();
            let turns = if agent.session().tree_enabled() {
                let tree = agent
                    .session()
                    .tree_snapshot(&|| std::time::Instant::now() >= deadline)
                    .map_err(|_| ErrorCode::ResourceLimit)?;
                let path = tree
                    .ancestry(tree.active_leaf(), &|| {
                        std::time::Instant::now() >= deadline
                    })
                    .map_err(|_| ErrorCode::ResourceLimit)?;
                let by_id = all
                    .iter()
                    .map(|turn| (turn.id.as_str(), turn))
                    .collect::<std::collections::BTreeMap<_, _>>();
                path.iter()
                    .map(|entry| {
                        by_id
                            .get(entry.turn_id.as_str())
                            .copied()
                            .ok_or(ErrorCode::InvalidRequest)
                    })
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                all.iter().collect::<Vec<_>>()
            };
            let mut ids = std::collections::BTreeSet::new();
            if turns
                .iter()
                .any(|turn| !super::auth::valid_identifier(&turn.id) || !ids.insert(&turn.id))
            {
                return Err(ErrorCode::InvalidRequest);
            }
            let mut result = history_page_refs(&turns, offset, digest, Some(deadline))?;
            if std::time::Instant::now() >= deadline {
                return Err(ErrorCode::ResourceLimit);
            }
            result["project_id"] = json!(project);
            result["session_id"] = json!(context.session_id);
            result["journal_revision"] = json!(revision);
            Ok(result)
        })
        .map_err(|error| error.remote_code())?
}

fn history_page(
    turns: &[crate::core::Turn],
    offset: usize,
    digest: Option<&str>,
) -> Result<Value, ErrorCode> {
    history_page_refs(&turns.iter().collect::<Vec<_>>(), offset, digest, None)
}

struct HistoryDigest {
    digest: Sha256,
    remaining: usize,
    deadline: Option<std::time::Instant>,
}

impl std::io::Write for HistoryDigest {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.remaining
            || self
                .deadline
                .is_some_and(|deadline| std::time::Instant::now() >= deadline)
        {
            return Err(std::io::Error::other(
                "history serialization budget exceeded",
            ));
        }
        self.remaining -= bytes.len();
        self.digest.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn history_page_refs(
    turns: &[&crate::core::Turn],
    offset: usize,
    digest: Option<&str>,
    deadline: Option<std::time::Instant>,
) -> Result<Value, ErrorCode> {
    if offset > turns.len() || offset > 0 && offset == turns.len() {
        return Err(ErrorCode::InvalidRequest);
    }
    let mut writer = HistoryDigest {
        digest: Sha256::new(),
        remaining: 64 * 1024 * 1024,
        deadline,
    };
    serde_json::to_writer(&mut writer, turns).map_err(|_| ErrorCode::ResourceLimit)?;
    let actual = format!("{:x}", writer.digest.finalize());
    if digest.is_some_and(|digest| digest != actual) {
        return Err(ErrorCode::RevisionConflict);
    }
    let mut end = offset.saturating_add(16).min(turns.len());
    let mut values = Vec::new();
    for turn in &turns[offset..end] {
        if turn.content.len() > MAX_CONTENT_BYTES || turn.content.contains('\0') {
            return Err(ErrorCode::ResourceLimit);
        }
        let safe = crate::security::redact_text(&turn.content, &[]);
        let mut boundary = safe.len().min(2048);
        while !safe.is_char_boundary(boundary) {
            boundary -= 1;
        }
        let mut value = json!({"id":turn.id,"role":turn.role,"content":&safe[..boundary],"content_truncated":boundary < safe.len()});
        if let Some(summary) = tool_call_projection(turn)? {
            value
                .as_object_mut()
                .unwrap()
                .extend(summary.as_object().unwrap().clone());
        }
        values.push(value);
        // Escaping can expand previews; reserve envelope/scope space in control frames.
        if serde_json::to_vec(&values)
            .map_err(|_| ErrorCode::InternalError)?
            .len()
            > super::protocol::MAX_CONTROL_BYTES - 4096
        {
            values.pop();
            end = offset + values.len();
            break;
        }
    }
    Ok(
        json!({"digest":actual,"offset":offset,"next_offset":end,"total_turns":turns.len(),"has_more":end < turns.len(),"turns":values}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Turn, TurnRole};

    #[test]
    fn history_digest_streams_exact_json_and_refuses_budget_or_expired_deadline() {
        let turns = [Turn::new(
            "unicode",
            TurnRole::User,
            "\u{4e2d}\nquoted \"message\"",
        )];
        let refs = turns.iter().collect::<Vec<_>>();
        let bytes = serde_json::to_vec(&turns).unwrap();
        let page = history_page_refs(&refs, 0, None, None).unwrap();
        assert_eq!(page["digest"], format!("{:x}", Sha256::digest(&bytes)));
        let mut exact = HistoryDigest {
            digest: Sha256::new(),
            remaining: bytes.len(),
            deadline: None,
        };
        serde_json::to_writer(&mut exact, &refs).unwrap();
        assert_eq!(exact.remaining, 0);
        assert_eq!(
            format!("{:x}", exact.digest.finalize()),
            page["digest"].as_str().unwrap()
        );
        let mut small = HistoryDigest {
            digest: Sha256::new(),
            remaining: bytes.len() - 1,
            deadline: None,
        };
        assert!(serde_json::to_writer(&mut small, &refs).is_err());
        assert_eq!(
            history_page_refs(&refs, 0, None, Some(std::time::Instant::now())),
            Err(ErrorCode::ResourceLimit)
        );
        let escaped = Turn::new("escaped", TurnRole::User, "\u{1}".repeat(12 * 1024 * 1024));
        assert_eq!(
            history_page_refs(&[&escaped], 0, None, None),
            Err(ErrorCode::ResourceLimit)
        );
    }

    #[test]
    fn tool_arguments_pages_redact_before_slicing_and_bind_call_identity() {
        let mut turn = Turn::new("invocation", TurnRole::Assistant, "");
        turn.metadata = Some(
            json!({"tool_calls":[{"id":"call-a","name":"read_file","arguments":{"api_key":"ARGUMENT_SECRET","text":"\u{4e2d}".repeat(7000)},"execution":"PRIVATE_EVIDENCE"}]}),
        );
        let expected = serde_json::to_string(&crate::security::redact_json(
            &turn.metadata.as_ref().unwrap()["tool_calls"][0]["arguments"],
            &[],
        ))
        .unwrap();
        let mut offset = 0;
        let mut digest = None;
        let mut joined = String::new();
        loop {
            let page = tool_arguments_page(&turn, "call-a", offset, digest.as_deref()).unwrap();
            assert_eq!(page["tool_call_id"], "call-a");
            assert_eq!(page["tool_name"], "read_file");
            assert!(!page.to_string().contains("ARGUMENT_SECRET"));
            assert!(!page.to_string().contains("PRIVATE_EVIDENCE"));
            assert!(page["text"].as_str().unwrap().len() <= PAGE_BYTES);
            joined.push_str(page["text"].as_str().unwrap());
            offset = page["next_offset"].as_u64().unwrap() as usize;
            digest = Some(page["digest"].as_str().unwrap().to_owned());
            if page["has_more"] == false {
                break;
            }
        }
        assert_eq!(joined, expected);
        assert_eq!(
            tool_arguments_page(&turn, "foreign", 0, None),
            Err(ErrorCode::InvalidRequest)
        );
        assert_eq!(
            tool_arguments_page(&turn, "call-a", 0, Some(&"0".repeat(64))),
            Err(ErrorCode::RevisionConflict)
        );
        let first = tool_arguments_page(&turn, "call-a", 0, None).unwrap();
        let inside = first["text"].as_str().unwrap().find('\u{4e2d}').unwrap() + 1;
        assert_eq!(
            tool_arguments_page(&turn, "call-a", inside, digest.as_deref()),
            Err(ErrorCode::InvalidRequest)
        );
        turn.metadata.as_mut().unwrap()["tool_calls"][0]["arguments"] =
            json!({"text":"x".repeat(64 * 1024 - 11)});
        assert_eq!(
            tool_arguments_page(&turn, "call-a", 0, None).unwrap()["total_bytes"],
            64 * 1024
        );
        turn.metadata.as_mut().unwrap()["tool_calls"][0]["arguments"] =
            json!({"text":"x".repeat(64 * 1024 - 10)});
        assert_eq!(
            tool_arguments_page(&turn, "call-a", 0, None),
            Err(ErrorCode::ResourceLimit)
        );
        turn.metadata.as_mut().unwrap()["tool_calls"][0]["arguments"] = json!({});
        turn.metadata.as_mut().unwrap()["tool_calls"][0]["arguments"] =
            json!({"text":"api_key=x\n".repeat(5000)});
        assert_eq!(
            tool_arguments_page(&turn, "call-a", 0, None),
            Err(ErrorCode::ResourceLimit)
        );
        turn.metadata.as_mut().unwrap()["tool_calls"][0]["arguments"] = json!({});
        let call = turn.metadata.as_ref().unwrap()["tool_calls"][0].clone();
        turn.metadata.as_mut().unwrap()["tool_calls"]
            .as_array_mut()
            .unwrap()
            .push(call);
        assert_eq!(
            tool_arguments_page(&turn, "call-a", 0, None),
            Err(ErrorCode::InvalidRequest)
        );
        turn.role = TurnRole::Tool;
        assert_eq!(
            tool_arguments_page(&turn, "call-a", 0, None),
            Err(ErrorCode::InvalidRequest)
        );
    }

    #[test]
    fn tool_call_history_projects_only_bounded_redacted_public_fields() {
        let mut turn = Turn::new("calls", TurnRole::Assistant, "");
        turn.metadata = Some(
            json!({"tool_calls":(0..9).map(|n| json!({"id":format!("call-{n}"),"name":"read_file",
            "arguments":{"path":"evidence.txt","api_key":"PRIVATE_ARGUMENT","text":format!("access_token=PRIVATE_TEXT\n{}", "\u{4e2d}".repeat(400))},
            "execution":"PRIVATE_EXECUTION","policy_digest":"PRIVATE_POLICY","worker_binding":"PRIVATE_BINDING"})).collect::<Vec<_>>(),
            "annotations":"PRIVATE_ANNOTATIONS","model":"PRIVATE_MODEL"}),
        );
        let page = history_page(&[turn.clone()], 0, None).unwrap();
        let projected = &page["turns"][0];
        assert_eq!(projected["tool_calls"].as_array().unwrap().len(), 8);
        assert_eq!(projected["tool_calls_truncated"], true);
        for call in projected["tool_calls"].as_array().unwrap() {
            assert!(call["arguments_preview"].as_str().unwrap().len() <= 512);
            assert_eq!(call["arguments_truncated"], true);
        }
        assert!(!page.to_string().contains("PRIVATE_"));
        let entry = history_entry(&turn).unwrap();
        turn.metadata.as_mut().unwrap()["tool_calls"][0]["arguments"]["path"] =
            json!("another.txt");
        assert_ne!(entry, history_entry(&turn).unwrap());
        turn.metadata.as_mut().unwrap()["tool_calls"][0]["arguments"] =
            json!({"text":"x".repeat(64 * 1024 - 11)});
        assert!(history_page(&[turn.clone()], 0, None).is_ok());
        turn.metadata.as_mut().unwrap()["tool_calls"][0]["arguments"] =
            json!({"text":"x".repeat(64 * 1024)});
        assert_eq!(
            history_page(&[turn.clone()], 0, None),
            Err(ErrorCode::ResourceLimit)
        );
        turn.metadata.as_mut().unwrap()["tool_calls"][0]["arguments"] = json!({});
        turn.metadata.as_mut().unwrap()["tool_calls"][0]["name"] = json!("x".repeat(65));
        assert_eq!(
            history_page(&[turn.clone()], 0, None),
            Err(ErrorCode::InvalidRequest)
        );
        turn.metadata.as_mut().unwrap()["tool_calls"][0]["name"] = json!("read_file");
        turn.metadata.as_mut().unwrap()["tool_calls"][1]["id"] = json!("call-0");
        assert_eq!(
            history_page(&[turn], 0, None),
            Err(ErrorCode::InvalidRequest)
        );
    }

    #[test]
    fn escaped_tool_history_pages_fit_control_frames_without_losing_turns() {
        let turns = (0..16).map(|n| {
            let mut turn = Turn::new(format!("escaped-{n}"), TurnRole::Assistant, "\u{0001}".repeat(2048));
            turn.metadata = Some(json!({"tool_calls":(0..8).map(|index| json!({"id":format!("call-{index}"), "name":"read_file",
                "arguments":{"text":"\u{0001}".repeat(3000)}})).collect::<Vec<_>>()}));
            turn
        }).collect::<Vec<_>>();
        let mut offset = 0;
        let mut digest = None;
        let mut ids = Vec::new();
        loop {
            let page = history_page(&turns, offset, digest.as_deref()).unwrap();
            if offset == 0 {
                assert!(page["has_more"].as_bool().unwrap());
            }
            assert!(
                serde_json::to_vec(&page).unwrap().len() + 2048
                    < super::super::protocol::MAX_CONTROL_BYTES
            );
            let values = page["turns"].as_array().unwrap();
            assert!(!values.is_empty());
            ids.extend(
                values
                    .iter()
                    .map(|turn| turn["id"].as_str().unwrap().to_owned()),
            );
            offset = page["next_offset"].as_u64().unwrap() as usize;
            digest = Some(page["digest"].as_str().unwrap().to_owned());
            if page["has_more"] == false {
                break;
            }
            assert!(offset < 16);
        }
        assert_eq!(
            ids,
            turns.iter().map(|turn| turn.id.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn exact_content_cap_is_admitted_and_one_more_is_refused() {
        let turn = Turn::new("cap", TurnRole::Assistant, "x".repeat(MAX_CONTENT_BYTES));
        let page = content_page(&turn, 0, None).unwrap();
        assert_eq!(page["text"].as_str().unwrap().len(), PAGE_BYTES);
        assert_eq!(page["total_bytes"], MAX_CONTENT_BYTES);
        assert_eq!(page["has_more"], true);
        let digest = page["digest"].as_str().unwrap();
        let last = content_page(&turn, MAX_CONTENT_BYTES, Some(digest)).unwrap();
        assert_eq!(last["text"], "");
        assert_eq!(last["has_more"], false);
        assert_eq!(
            content_page(&turn, MAX_CONTENT_BYTES + 1, Some(digest)),
            Err(ErrorCode::InvalidRequest)
        );
        let mut over = turn;
        over.content.push('x');
        assert_eq!(content_page(&over, 0, None), Err(ErrorCode::ResourceLimit));
        let unsafe_body = Turn::new("unsafe", TurnRole::Assistant, "hidden\0body");
        assert_eq!(
            content_page(&unsafe_body, 0, None),
            Err(ErrorCode::ResourceLimit)
        );
    }

    #[test]
    fn task_history_pages_are_bounded_redacted_contiguous_and_digest_fenced() {
        let turns = (0..35)
            .map(|index| {
                crate::core::Turn::new(
                    format!("turn-{index}"),
                    crate::core::TurnRole::Tool,
                    format!("API_KEY=HISTORY_SECRET\n{}", "\u{4e2d}".repeat(3000)),
                )
            })
            .collect::<Vec<_>>();
        let mut offset = 0;
        let mut digest = None;
        let mut ids = Vec::new();
        loop {
            let page = history_page(&turns, offset, digest.as_deref()).unwrap();
            let values = page["turns"].as_array().unwrap();
            assert!(values.len() <= 16);
            for value in values {
                assert!(value["content"].as_str().unwrap().len() <= 2048);
                assert!(
                    !value["content"]
                        .as_str()
                        .unwrap()
                        .contains("HISTORY_SECRET")
                );
                ids.push(value["id"].as_str().unwrap().to_owned());
            }
            offset = page["next_offset"].as_u64().unwrap() as usize;
            digest = Some(page["digest"].as_str().unwrap().to_owned());
            if page["has_more"] == false {
                break;
            }
        }
        assert_eq!(ids.len(), 35);
        assert_eq!(
            history_page(&turns, 36, digest.as_deref()),
            Err(ErrorCode::InvalidRequest)
        );
        assert_eq!(
            history_page(&turns, 0, Some(&"0".repeat(64))),
            Err(ErrorCode::RevisionConflict)
        );
    }

    #[test]
    fn every_utf8_page_boundary_is_valid_and_redaction_precedes_offsets() {
        for width in 1..=4 {
            let character = ['x', '\u{00e9}', '\u{4e2d}', '\u{1f600}'][width - 1];
            let raw = format!(
                "api_key=PAGE_SECRET\n{}",
                character.to_string().repeat(PAGE_BYTES * 2)
            );
            let turn = Turn::new("boundary", TurnRole::Assistant, raw.clone());
            let expected = crate::security::redact_text(&raw, &[]);
            let mut joined = String::new();
            let mut offset = 0;
            let mut digest = None;
            loop {
                let page = content_page(&turn, offset, digest.as_deref()).unwrap();
                let text = page["text"].as_str().unwrap();
                assert!(text.len() <= PAGE_BYTES);
                assert!(!text.contains("PAGE_SECRET"));
                joined.push_str(text);
                offset = page["next_offset"].as_u64().unwrap() as usize;
                digest = Some(page["digest"].as_str().unwrap().to_owned());
                if page["has_more"] == false {
                    break;
                }
            }
            assert_eq!(joined, expected);
            assert_eq!(turn.content, raw);
        }
    }
}
