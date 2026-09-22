//! Views over materialized request attachments; this module never reads paths.
use crate::backend::{BackendError, RequestAttachment};
use crate::core::Turn;
use serde_json::Value;

pub(super) fn attachments_for_context<'a>(
    turns: &[Turn],
    attachments: &'a [RequestAttachment],
) -> Vec<&'a RequestAttachment> {
    attachments
        .iter()
        .filter(|attachment| turns.iter().any(|turn| turn.id == attachment.turn_id))
        .collect()
}

pub(super) fn attachment_source(attachment: &RequestAttachment) -> Result<String, BackendError> {
    if let Some(url) = &attachment.input.url {
        return Ok(url.clone());
    }
    if let Some(data) = &attachment.data {
        return Ok(data_url(&attachment.input.mime_type, data));
    }
    if let Some(file_id) = &attachment.input.file_id {
        return Ok(file_id.clone());
    }
    Err(BackendError::Configuration(
        "attachment source was not materialized".into(),
    ))
}

pub(super) fn data_url(mime_type: &str, bytes: &[u8]) -> String {
    use base64::Engine;

    format!(
        "data:{mime_type};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

pub(super) fn attachment_turn_matches(message: &Value, turn_id: &str) -> bool {
    message
        .get("_zenpi_turn_id")
        .and_then(Value::as_str)
        .is_none_or(|candidate| candidate == turn_id)
}

pub(super) fn strip_internal_turn_ids(items: &mut [Value]) {
    for item in items {
        if let Some(object) = item.as_object_mut() {
            object.remove("_zenpi_turn_id");
        }
    }
}
