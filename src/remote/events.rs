//! Durable transcript-event projection. Its sequence is not the journal sequence.

use super::protocol::{Channel, Envelope, ErrorCode, EventCursor, Message};
use crate::{core::Turn, session::SessionStore};
use serde_json::{Value, json};

pub const REPLAY_WINDOW: usize = 256;
const PAGE_BUDGET: usize = 96 * 1024;
pub const MAX_TURN_CONTENT_BYTES: usize = 12 * 1024;

pub fn acknowledge(
    pool: &mut crate::project_workspace::ProjectOwnerPool,
    registry: &super::auth::DeviceRegistry,
    device: &super::auth::AuthenticatedDevice,
    request: &Envelope,
) -> Result<Value, ErrorCode> {
    let Message::Ack { cursor } = &request.payload else {
        return Err(ErrorCode::InvalidRequest);
    };
    if request.validate().is_err()
        || request.id.is_none()
        || request.required_owner.is_some()
        || request.expected_revision.is_some()
        || request.turn_id.is_some()
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let project = request
        .project_id
        .as_deref()
        .ok_or(ErrorCode::InvalidRequest)?;
    registry.with_authorized_projects(device, &[project], super::protocol::Operation::Inspect, |_| {
        let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
        let agent = owner.try_lock().map_err(|_| ErrorCode::Busy)?;
        let end = self::cursor(agent.session());
        if cursor.session_id != end.session_id { return Err(ErrorCode::OwnerMismatch); }
        if cursor.next_sequence > end.next_sequence { return Err(ErrorCode::InvalidRequest); }
        drop(agent);
        let persisted = pool.acknowledge_events(device.id(), project, cursor)?;
        Ok(json!({"device_id":device.id(),"project_id":project,"cursor_scope":"transcript_events_v1","cursor":persisted}))
    }).map_err(|error| error.remote_code())?
}

pub struct ReplayPage {
    pub frames: Vec<Envelope>,
    pub cursor: EventCursor,
    pub has_more: bool,
    pub snapshot_required: bool,
}

pub fn projected_turn(turn: &Turn) -> Value {
    // Redact before truncating: slicing a credential can hide its closing
    // delimiter from the redactor and expose a partial secret.
    let mut content = crate::security::redact_text(&turn.content, &[]);
    let truncated = content.len() > MAX_TURN_CONTENT_BYTES;
    if truncated {
        let mut end = MAX_TURN_CONTENT_BYTES;
        while !content.is_char_boundary(end) {
            end -= 1;
        }
        content.truncate(end);
    }
    let mut value = json!({"id": turn.id, "parent_id": turn.parent_id, "role": turn.role,
        "content": content, "created_at_ms": turn.created_at_ms});
    if truncated {
        value["content_truncated"] = Value::Bool(true);
    }
    // Malformed legacy invocation metadata is withheld, not forwarded raw or
    // allowed to break the durable transcript cursor.
    if let Ok(Some(summary)) = super::transcript::tool_call_projection(turn) {
        value
            .as_object_mut()
            .unwrap()
            .extend(summary.as_object().unwrap().clone());
    }
    value
}

/// One sequence per durable turn; filtered journal records never consume this cursor.
pub fn cursor(session: &SessionStore) -> EventCursor {
    EventCursor {
        session_id: session.session_id().into(),
        next_sequence: session.turns().len() as u64,
    }
}

pub fn replay(
    session: &SessionStore,
    project: &str,
    from: &EventCursor,
    limit: u16,
) -> Result<ReplayPage, ErrorCode> {
    if from.session_id != session.session_id() {
        return Err(ErrorCode::OwnerMismatch);
    }
    if limit == 0 || limit > super::protocol::MAX_REPLAY_EVENTS {
        return Err(ErrorCode::ResourceLimit);
    }
    let end = cursor(session);
    if from.next_sequence > end.next_sequence {
        return Err(ErrorCode::InvalidRequest);
    }
    let floor = end.next_sequence.saturating_sub(REPLAY_WINDOW as u64);
    // A selected tree branch is not an append-only transcript. Require a fresh snapshot instead.
    if from.next_sequence < floor || session.tree_enabled() {
        return Ok(ReplayPage {
            frames: vec![],
            cursor: end,
            has_more: false,
            snapshot_required: true,
        });
    }
    let mut next = from.next_sequence;
    let mut frames = Vec::new();
    let mut used = 0;
    for turn in session
        .turns()
        .iter()
        .skip(next as usize)
        .take(limit as usize)
    {
        let mut frame = Envelope::new(
            Channel::Event,
            Message::Event {
                name: "transcript_turn".into(),
                data: projected_turn(turn),
            },
        );
        frame.project_id = Some(project.into());
        frame.session_id = Some(end.session_id.clone());
        frame.turn_id = Some(turn.id.clone());
        frame.sequence = Some(next);
        let size = serde_json::to_vec(&frame)
            .map_err(|_| ErrorCode::InternalError)?
            .len();
        if size > PAGE_BUDGET {
            return Ok(ReplayPage {
                frames: vec![],
                cursor: end,
                has_more: false,
                snapshot_required: true,
            });
        }
        if used + size > PAGE_BUDGET {
            break;
        }
        used += size;
        frames.push(frame);
        next += 1;
    }
    Ok(ReplayPage {
        frames,
        cursor: EventCursor {
            session_id: end.session_id,
            next_sequence: next,
        },
        has_more: next < end.next_sequence,
        snapshot_required: false,
    })
}
