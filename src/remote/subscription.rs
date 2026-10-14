//! A bounded foreground transcript subscription over an authenticated connection.

use super::{
    host_adapter::HostAdapter,
    protocol::{Envelope, ErrorCode, ExecutionState, Message},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io,
    time::{Duration, Instant},
};

/// Replace every live text in a view (`desktop_stream_text` on a project,
/// `stream_text` on its prompt and worker statuses) with an ordered delta:
/// `<prefix>_offset` is the number of Unicode scalar values the client keeps
/// from the text this subscription last sent for the same stream, and
/// `<prefix>_delta` is appended after it. A new subscription starts every
/// stream at offset 0, so replay after a reconnect neither repeats nor skips
/// text; an offset below the sent length rewrites a suffix that the redactor
/// changed once it saw later tokens. Streams absent from the view are
/// forgotten, matching the client dropping their text.
fn live_text_deltas(view: &mut Value, sent: &mut BTreeMap<String, String>) {
    let mut current = BTreeMap::new();
    if let Some(projects) = view
        .pointer_mut("/listing/projects")
        .and_then(Value::as_array_mut)
    {
        for project in projects {
            let Some(id) = project["id"].as_str().map(str::to_owned) else {
                continue;
            };
            delta(
                project,
                "desktop_stream",
                format!("{id}\0desktop"),
                sent,
                &mut current,
            );
            for list in ["prompt_statuses", "worker_statuses"] {
                let Some(items) = project.get_mut(list).and_then(Value::as_array_mut) else {
                    continue;
                };
                for item in items {
                    if let Some(request) = item["request_id"].as_str().map(str::to_owned) {
                        let key = format!("{id}\0{list}\0{request}");
                        delta(item, "stream", key, sent, &mut current);
                    }
                }
            }
        }
    }
    *sent = current;
}

fn delta(
    object: &mut Value,
    prefix: &str,
    key: String,
    sent: &BTreeMap<String, String>,
    current: &mut BTreeMap<String, String>,
) {
    let Some(map) = object.as_object_mut() else {
        return;
    };
    let field = format!("{prefix}_text");
    if !map.get(&field).is_some_and(Value::is_string) {
        return;
    }
    let Some(Value::String(text)) = map.remove(&field) else {
        return;
    };
    let previous = sent.get(&key).map_or("", String::as_str);
    let mut kept = 0;
    let mut chars = 0usize;
    for ((at, left), right) in text.char_indices().zip(previous.chars()) {
        if left != right {
            break;
        }
        kept = at + left.len_utf8();
        chars += 1;
    }
    map.insert(format!("{prefix}_offset"), json!(chars));
    map.insert(format!("{prefix}_delta"), json!(text[kept..]));
    current.insert(key, text);
}

pub fn serve(
    request: &Envelope,
    dispatch: impl FnMut(Envelope) -> Envelope,
    emit: impl FnMut(Envelope) -> io::Result<()>,
) -> io::Result<()> {
    serve_with_view(request, dispatch, || Ok(None), emit)
}

pub fn serve_authorized(
    request: &Envelope,
    adapter: &HostAdapter,
    registry: &std::sync::Arc<super::auth::DeviceRegistry>,
    device: &super::auth::AuthenticatedDevice,
    output: &mut impl io::Write,
) -> io::Result<()> {
    serve_with_view(
        request,
        |resume| adapter.dispatch(registry, device, resume),
        || view(request, |read| adapter.dispatch(registry, device, read)),
        |frame| {
            let refusal = match registry.is_current(device) {
                Ok(true) => None,
                Ok(false) => Some(ErrorCode::Unauthorized),
                Err(error) => Some(error.remote_code()),
            };
            if let Some(code) = refusal {
                // Send only a correlated refusal, never the sampled/event data.
                // Stop even if permissions are restored before the next frame.
                super::frame::write_frame(output, &HostAdapter::failure(request, code))
                    .map_err(io::Error::other)?;
                return Err(io::Error::other("subscription authorization unavailable"));
            }
            super::frame::write_frame(output, &frame).map_err(io::Error::other)
        },
    )
}

pub fn view(
    request: &Envelope,
    mut dispatch: impl FnMut(Envelope) -> Envelope,
) -> Result<Option<serde_json::Value>, ErrorCode> {
    if request.validate().is_err() || !matches!(request.payload, Message::Subscribe { .. }) {
        return Err(ErrorCode::InvalidRequest);
    }
    let mut listing = Envelope::new(
        super::protocol::Channel::Control,
        Message::Request {
            operation: super::protocol::Operation::WorkspaceList,
            arguments: serde_json::json!({}),
        },
    );
    listing.id = request.id.clone();
    let listed = dispatch(listing);
    if listed.execution_state != Some(ExecutionState::Succeeded) {
        return Err(listed.code.unwrap_or(ErrorCode::InternalError));
    }
    let Message::Response { result } = listed.payload else {
        return Err(ErrorCode::InternalError);
    };
    let project = result["projects"]
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["id"].as_str() == request.project_id.as_deref())
        })
        .ok_or(ErrorCode::Unauthorized)?;
    if project["session_id"].as_str() != request.session_id.as_deref() {
        return Err(ErrorCode::SessionChanged);
    }
    let mut tabs = request.clone();
    tabs.payload = Message::Request {
        operation: super::protocol::Operation::SubtabList,
        arguments: serde_json::json!({}),
    };
    let tab_response = dispatch(tabs);
    if tab_response.execution_state != Some(ExecutionState::Succeeded) {
        return Err(tab_response.code.unwrap_or(ErrorCode::InternalError));
    }
    let Message::Response { result: subtabs } = tab_response.payload else {
        return Err(ErrorCode::InternalError);
    };
    if result["workspace_revision"] != subtabs["workspace_revision"]
        || result["required_owner"] != subtabs["required_owner"]
    {
        return Err(ErrorCode::Busy);
    }
    let mut model = request.clone();
    model.payload = Message::Request {
        operation: super::protocol::Operation::ModelStatus,
        arguments: serde_json::json!({}),
    };
    let model_response = dispatch(model);
    let model_status = if model_response.execution_state == Some(ExecutionState::Succeeded) {
        let Message::Response { result } = model_response.payload else {
            return Err(ErrorCode::InternalError);
        };
        if result["project_id"].as_str() != request.project_id.as_deref()
            || result["session_id"].as_str() != request.session_id.as_deref()
        {
            return Err(ErrorCode::OwnerMismatch);
        }
        result
    } else if model_response.code == Some(ErrorCode::Busy) {
        serde_json::Value::Null
    } else {
        return Err(model_response.code.unwrap_or(ErrorCode::InternalError));
    };
    Ok(Some(
        serde_json::json!({"listing":result,"subtabs":subtabs,"model_status":model_status}),
    ))
}

pub fn serve_with_view(
    request: &Envelope,
    mut dispatch: impl FnMut(Envelope) -> Envelope,
    mut sample: impl FnMut() -> Result<Option<serde_json::Value>, ErrorCode>,
    mut emit: impl FnMut(Envelope) -> io::Result<()>,
) -> io::Result<()> {
    let Message::Subscribe {
        cursor,
        limit,
        duration_ms,
    } = &request.payload
    else {
        return emit(HostAdapter::failure(request, ErrorCode::InvalidRequest));
    };
    if request.validate().is_err() {
        return emit(HostAdapter::failure(request, ErrorCode::InvalidRequest));
    }
    let deadline = Instant::now() + Duration::from_millis(u64::from(*duration_ms));
    let mut cursor = cursor.clone();
    let mut events = 0usize;
    let mut bytes = 0usize;
    let mut previous_view = None;
    let mut sent_text = BTreeMap::new();
    let wakeup = super::wakeup::watch(&cursor.session_id);
    let mut batches = 0;
    loop {
        let observed = wakeup.as_ref().map_or(0, |signal| signal.revision());
        let sampled = match sample() {
            Ok(value) => value,
            Err(ErrorCode::OwnerMismatch) => {
                return emit(HostAdapter::failure(request, owner_mismatch_code(sample())));
            }
            Err(code) => return emit(HostAdapter::failure(request, code)),
        };
        let mut resume = request.clone();
        resume.payload = Message::Resume {
            cursor: cursor.clone(),
            limit: *limit,
        };
        let mut response = dispatch(resume);
        // The owner can switch between the metadata sample and replay. Only
        // a fresh listing can classify this as a session change or authority loss.
        if response.code == Some(ErrorCode::OwnerMismatch) {
            return emit(HostAdapter::failure(request, owner_mismatch_code(sample())));
        }
        let transcript_available = response.execution_state == Some(ExecutionState::Succeeded);
        if !transcript_available && !(response.code == Some(ErrorCode::Busy) && sampled.is_some()) {
            return emit(response);
        }
        if !transcript_available {
            response.execution_state = Some(ExecutionState::Succeeded);
            response.code = None;
            response.payload = Message::Response {
                result: serde_json::json!({"cursor":cursor,"cursor_scope":"transcript_events_v1","wire_frames":[],"snapshot_required":false,"has_more":false}),
            };
        }
        let Message::Response { result } = &mut response.payload else {
            return emit(HostAdapter::failure(request, ErrorCode::InternalError));
        };
        let next = serde_json::from_value(result["cursor"].clone()).map_err(io::Error::other)?;
        result["subscription_transcript"] = serde_json::Value::Bool(transcript_available);
        if sampled != previous_view {
            let mut view = sampled.clone().unwrap_or(serde_json::Value::Null);
            live_text_deltas(&mut view, &mut sent_text);
            result["subscription_view"] = view;
            previous_view = sampled;
        }
        let frame_count = result["wire_frames"]
            .as_array()
            .ok_or_else(|| io::Error::other("subscription frames unavailable"))?
            .len();
        batches += 1;
        let done = Instant::now() >= deadline
            || batches >= 32
            || events + frame_count >= 128
            || result["snapshot_required"] == true
            || result["has_more"] == true;
        result["subscription_done"] = serde_json::Value::Bool(done);
        let frames = HostAdapter::wire_frames(response).map_err(io::Error::other)?;
        let size = frames
            .iter()
            .try_fold(0usize, |size, frame| {
                serde_json::to_vec(frame).map(|body| size + body.len())
            })
            .map_err(io::Error::other)?;
        // Android budgets actual JSON bodies, including each envelope. Leave
        // room for a later correlated refusal while this subscription is live.
        let refusal = HostAdapter::failure(request, ErrorCode::ResourceLimit);
        let reserved = if done {
            0
        } else {
            serde_json::to_vec(&HostAdapter::failure(
                request,
                ErrorCode::UnsupportedVersion,
            ))
            .map_err(io::Error::other)?
            .len()
        };
        if events + frame_count > 128 || bytes + size + reserved > 196_608 {
            return emit(refusal);
        }
        events += frame_count;
        bytes += size;
        for frame in frames {
            emit(frame)?;
        }
        if done {
            return Ok(());
        }
        cursor = next;
        super::wakeup::wait(wakeup.as_deref(), observed, deadline);
    }
}

fn owner_mismatch_code(sample: Result<Option<serde_json::Value>, ErrorCode>) -> ErrorCode {
    match sample {
        Err(
            code @ (ErrorCode::SessionChanged | ErrorCode::Unauthorized | ErrorCode::RevokedDevice),
        ) => code,
        _ => ErrorCode::OwnerMismatch,
    }
}
