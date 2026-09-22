//! Responses request encoding and bounded SSE/JSON response decoding.
use crate::backend::{
    AttachmentKind, BackendError, Completion, CompletionRequest, MAX_RESPONSE_BYTES, ProviderEvent,
    RequestAttachment, Usage,
};
use crate::core::{Turn, TurnRole};
use crate::tools::ToolCall;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

use super::content::{
    attachment_source, attachment_turn_matches, attachments_for_context, data_url,
    strip_internal_turn_ids,
};
use super::{
    emit_completion_events, extract_annotations, is_recv_body_poll_timeout, parse_tool_call,
    parse_usage, read_bounded_json_body,
};

pub(super) fn encode_request(
    request: &CompletionRequest<'_>,
    model: &str,
    reasoning_effort: Option<&str>,
    verbosity: Option<&str>,
) -> Result<Value, BackendError> {
    encode_request_inner(request, model, reasoning_effort, verbosity, None)
}

pub(super) fn encode_request_for_route(
    request: &CompletionRequest<'_>,
    model: &str,
    reasoning_effort: Option<&str>,
    verbosity: Option<&str>,
    codex: bool,
) -> Result<Value, BackendError> {
    encode_request_inner(request, model, reasoning_effort, verbosity, Some(codex))
}

fn encode_request_inner(
    request: &CompletionRequest<'_>,
    model: &str,
    reasoning_effort: Option<&str>,
    verbosity: Option<&str>,
    strict_history: Option<bool>,
) -> Result<Value, BackendError> {
    let mut input = if let Some(codex) = strict_history {
        checked_input(request.turns, codex)?
    } else {
        request
            .turns
            .iter()
            .flat_map(responses_input_items)
            .collect()
    };
    let attachments = attachments_for_context(request.turns, request.attachments);
    apply_responses_attachments(&mut input, &attachments)?;
    strip_internal_turn_ids(&mut input);
    let mut body = json!({
        "model": model,
        "input": input,
        "stream": true,
        "store": false,
    });
    if let Some(effort) = reasoning_effort {
        body["reasoning"] = json!({"effort": effort});
    }
    if let Some(verbosity) = verbosity {
        body["text"] = json!({"verbosity": verbosity});
    }
    if !request.tools.is_empty() {
        body["tools"] = Value::Array(
            request
                .tools
                .iter()
                .map(|tool| {
                    json!({
                        "type": "function",
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.input_schema,
                    })
                })
                .collect(),
        );
        body["tool_choice"] = json!("auto");
    }
    if let Some(instructions) = request.instructions {
        body["instructions"] = json!(instructions);
    }
    if let Some(metadata) = request.metadata {
        body["metadata"] = metadata.clone();
    }
    Ok(body)
}

fn checked_input(turns: &[Turn], codex: bool) -> Result<Vec<Value>, BackendError> {
    let invalid =
        || BackendError::Configuration("invalid or unpaired Responses tool history".into());
    let mut input = Vec::new();
    let mut pending = BTreeSet::new();
    let mut ids = BTreeMap::new();
    let mut wire_ids = BTreeMap::new();
    for turn in turns {
        if turn.role == TurnRole::Tool {
            let id = turn
                .metadata
                .as_ref()
                .and_then(|m| m["tool_call_id"].as_str())
                .ok_or_else(invalid)?;
            if !pending.remove(id) {
                return Err(invalid());
            }
            input.push(json!({"type":"function_call_output", "call_id":ids.get(id).ok_or_else(invalid)?, "output":turn.content}));
            continue;
        }
        if !pending.is_empty() {
            return Err(invalid());
        }
        let calls = turn.metadata.as_ref().and_then(|m| m.get("tool_calls"));
        if calls.is_some() && turn.role != TurnRole::Assistant {
            return Err(invalid());
        }
        if let Some(calls) = calls {
            let calls = calls.as_array().ok_or_else(invalid)?;
            if calls.is_empty() || calls.len() > 128 {
                return Err(invalid());
            }
            if !turn.content.is_empty() {
                input.push(json!({"role":"assistant", "content":turn.content}));
            }
            for call in calls {
                let id = call["id"]
                    .as_str()
                    .filter(|id| !id.is_empty() && id.len() <= 4096)
                    .ok_or_else(invalid)?;
                let name = call["name"]
                    .as_str()
                    .filter(|name| {
                        !name.is_empty()
                            && name.len() <= 64
                            && name
                                .bytes()
                                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
                    })
                    .ok_or_else(invalid)?;
                let arguments = call
                    .get("arguments")
                    .filter(|value| value.is_object())
                    .ok_or_else(invalid)?;
                if ids.contains_key(id) {
                    return Err(invalid());
                }
                let wire_id = if codex {
                    codex_call_id(id)
                } else {
                    id.to_owned()
                };
                if wire_ids.insert(wire_id.clone(), id.to_owned()).is_some() {
                    return Err(BackendError::Configuration(
                        "Responses tool ID mapping collision".into(),
                    ));
                }
                ids.insert(id.to_owned(), wire_id.clone());
                pending.insert(id.to_owned());
                input.push(json!({"type":"function_call", "call_id":wire_id, "name":name, "arguments":arguments.to_string()}));
            }
        } else {
            input.extend(responses_input_items(turn));
        }
    }
    if !pending.is_empty() {
        return Err(invalid());
    }
    Ok(input)
}

fn codex_call_id(id: &str) -> String {
    if id.len() <= 64
        && !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        id.to_owned()
    } else {
        let digest = format!("{:x}", Sha256::digest(id.as_bytes()));
        format!("zpi1_{}", &digest[..59])
    }
}

pub(super) fn read_response(
    body: &mut ureq::Body,
    streaming: bool,
    strict_terminal: bool,
    cancelled: &dyn Fn() -> bool,
    sink: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
) -> Result<Completion, BackendError> {
    if !streaming {
        let payload = read_bounded_json_body(body, cancelled)?;
        validate_responses_terminal(&payload, true)?;
        let completion = completion_from_responses_json(&payload)?;
        emit_completion_events(&completion, sink)?;
        Ok(completion)
    } else {
        read_responses_stream(body, strict_terminal, false, cancelled, sink)
    }
}

pub(super) fn read_response_for_route(
    body: &mut ureq::Body,
    streaming: bool,
    cancelled: &dyn Fn() -> bool,
    sink: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
) -> Result<Completion, BackendError> {
    let sink = &mut |event| {
        if cancelled() {
            return Err(BackendError::Cancelled);
        }
        sink(event)
    };
    if streaming {
        read_responses_stream(body, true, true, cancelled, sink)
    } else {
        let mut payload = read_bounded_json_body(body, cancelled)?;
        validate_responses_terminal(&payload, true)?;
        validate_strict_output(&mut payload)?;
        let completion = completion_from_responses_json(&payload)?;
        emit_completion_events(&completion, &mut |event| {
            if cancelled() {
                return Err(BackendError::Cancelled);
            }
            sink(event)
        })?;
        Ok(completion)
    }
}

fn validate_strict_output(payload: &mut Value) -> Result<(), BackendError> {
    if payload.get("error").is_some_and(|value| !value.is_null()) {
        return Err(BackendError::InvalidResponse(
            "Responses returned an error".into(),
        ));
    }
    let output = payload
        .get_mut("output")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            BackendError::InvalidResponse("Responses terminal output is missing".into())
        })?;
    let mut ids = BTreeSet::new();
    for item in output.iter() {
        match item["type"].as_str() {
            Some("message") => (),
            Some("function_call") => {
                if item["call_id"].as_str().is_none() || item["arguments"].as_str().is_none() {
                    return Err(BackendError::InvalidResponse(
                        "incomplete Responses function call".into(),
                    ));
                }
                let call = parse_response_function_call(item).ok_or_else(|| {
                    BackendError::InvalidResponse("incomplete Responses function call".into())
                })?;
                validate_checked_call(&call)?;
                if !ids.insert(call.id) {
                    return Err(BackendError::InvalidResponse(
                        "duplicate Responses function call".into(),
                    ));
                }
            }
            Some("reasoning") if item.get("encrypted_content").is_none_or(Value::is_null) => (),
            Some("reasoning") => {
                return Err(BackendError::InvalidResponse(
                    "opaque Responses reasoning requires a verified history scope".into(),
                ));
            }
            _ => {
                return Err(BackendError::InvalidResponse(
                    "unsupported Responses output item".into(),
                ));
            }
        }
    }
    // Reasoning content is not an assistant answer, even when it has text.
    output.retain(|item| item["type"] != "reasoning");
    Ok(())
}

fn apply_responses_attachments(
    input: &mut [Value],
    attachments: &[&RequestAttachment],
) -> Result<(), BackendError> {
    for attachment in attachments {
        attachment.input.validate()?;
        let message = input
            .iter_mut()
            .rev()
            .find(|item| {
                item.get("role").and_then(Value::as_str) == Some("user")
                    && attachment_turn_matches(item, &attachment.turn_id)
                    && item.get("content").is_some()
            })
            .ok_or_else(|| {
                BackendError::InvalidResponse("attachment has no corresponding user message".into())
            })?;
        let text = message
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let content = message
            .get_mut("content")
            .ok_or_else(|| BackendError::InvalidResponse("message content missing".into()))?;
        let parts = if let Value::Array(parts) = content {
            parts
        } else {
            *content = Value::Array(vec![json!({"type":"input_text", "text": text})]);
            content.as_array_mut().ok_or_else(|| {
                BackendError::InvalidResponse("message content is not an array".into())
            })?
        };
        match attachment.input.kind {
            AttachmentKind::Image => parts.push(json!({
                "type": "input_image",
                "image_url": attachment_source(attachment)?,
            })),
            AttachmentKind::File => {
                let mut part = json!({ "type": "input_file" });
                if let Some(file_id) = &attachment.input.file_id {
                    part["file_id"] = json!(file_id);
                } else if let Some(data) = &attachment.data {
                    part["file_data"] = json!(data_url(&attachment.input.mime_type, data));
                    if let Some(filename) = &attachment.filename {
                        part["filename"] = json!(filename);
                    }
                } else {
                    return Err(BackendError::Configuration(
                        "file attachment requires workspace bytes or provider file_id".into(),
                    ));
                }
                parts.push(part);
            }
        }
    }
    Ok(())
}

fn responses_input_items(turn: &Turn) -> Vec<Value> {
    if turn.role == TurnRole::Tool {
        let call_id = turn
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("tool_call_id"))
            .and_then(Value::as_str)
            .unwrap_or(&turn.id);
        return vec![json!({
            "type": "function_call_output",
            "call_id": call_id,
            "output": turn.content,
        })];
    }
    if turn.role == TurnRole::Assistant
        && let Some(calls) = turn
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("tool_calls"))
            .and_then(Value::as_array)
    {
        return calls
            .iter()
            .filter_map(|item| {
                Some(json!({
                    "type": "function_call",
                    "call_id": item.get("id")?.as_str()?,
                    "name": item.get("name")?.as_str()?,
                    "arguments": item.get("arguments")?.to_string(),
                }))
            })
            .collect();
    }
    let role = match turn.role {
        TurnRole::System => "system",
        TurnRole::User => "user",
        TurnRole::Assistant => "assistant",
        TurnRole::Tool => unreachable!(),
    };
    vec![json!({
        "role": role,
        "content": turn.content,
        "_zenpi_turn_id": turn.id,
    })]
}

fn extract_responses_refusal(payload: &Value) -> Option<String> {
    if let Some(refusal) = payload.get("refusal").and_then(Value::as_str) {
        return Some(refusal.to_owned());
    }
    payload
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        .flat_map(|item| item.get("content").and_then(Value::as_array))
        .flatten()
        .find_map(|part| {
            if part.get("type").and_then(Value::as_str) == Some("refusal") {
                part.get("refusal")
                    .or_else(|| part.get("text"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            } else {
                None
            }
        })
}

fn extract_responses_content(payload: &Value) -> Result<String, BackendError> {
    if let Some(text) = payload.get("output_text").and_then(Value::as_str) {
        return Ok(text.to_owned());
    }
    let output = payload
        .get("output")
        .and_then(Value::as_array)
        .ok_or_else(|| BackendError::InvalidResponse("missing output".into()))?;
    let mut text = String::new();
    for item in output {
        let Some(content) = item.get("content") else {
            continue;
        };
        match content {
            Value::String(value) => text.push_str(value),
            Value::Array(parts) => {
                for part in parts {
                    if let Some(value) = part.get("text").and_then(Value::as_str) {
                        text.push_str(value);
                    }
                }
            }
            other => {
                return Err(BackendError::InvalidResponse(format!(
                    "response content must be a string or text parts, got {other}"
                )));
            }
        }
    }
    if text.is_empty() {
        return Err(BackendError::InvalidResponse(
            "missing response output text".into(),
        ));
    }
    Ok(text)
}

fn read_responses_stream(
    body: &mut ureq::Body,
    strict_terminal: bool,
    strict_stream: bool,
    cancelled: &dyn Fn() -> bool,
    sink: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
) -> Result<Completion, BackendError> {
    use std::io::Read;

    let configured = body.with_config().limit(MAX_RESPONSE_BYTES as u64).reader();
    let mut reader = configured;
    // Some OpenAI-compatible proxies ignore `stream:true` and return a
    // regular Responses JSON object. Accept that shape as a compatibility
    // fallback while keeping the normal path event-aware.
    let mut content = String::new();
    let mut usage = None;
    let mut model = None;
    let mut saw_completed = false;
    let mut tool_calls = Vec::new();
    let mut response_id: Option<String> = None;
    let mut refusal: Option<String> = None;
    let mut annotations = Vec::new();
    let mut pending = Vec::new();
    let mut frame = Vec::new();
    let mut chunk = [0_u8; 16 * 1024];
    loop {
        if cancelled() {
            return Err(BackendError::Cancelled);
        }
        let read = match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if is_recv_body_poll_timeout(&error) => {
                if cancelled() {
                    return Err(BackendError::Cancelled);
                }
                // This timeout is intentionally a cancellation poll, not a
                // provider failure. ureq retains the body handler and its
                // global deadline, so a later chunk can continue the same SSE
                // frame without opening a second request.
                continue;
            }
            Err(error) => return Err(BackendError::Transport(error.to_string())),
        };
        pending.extend_from_slice(&chunk[..read]);
        while let Some(newline) = pending.iter().position(|byte| *byte == b'\n') {
            let line = pending.drain(..=newline).collect::<Vec<_>>();
            let line = if strict_stream {
                let Some(line) = response_frame_line(&line, &mut frame)? else {
                    continue;
                };
                line
            } else {
                line
            };
            process_responses_line(
                strict_terminal,
                strict_stream,
                &line,
                &mut content,
                &mut usage,
                &mut model,
                &mut saw_completed,
                &mut tool_calls,
                &mut response_id,
                &mut refusal,
                &mut annotations,
                sink,
            )?;
        }
    }
    if !pending.is_empty() {
        if strict_stream {
            return Err(BackendError::InvalidResponse(
                "unterminated Responses SSE frame".into(),
            ));
        }
        process_responses_line(
            strict_terminal,
            strict_stream,
            &pending,
            &mut content,
            &mut usage,
            &mut model,
            &mut saw_completed,
            &mut tool_calls,
            &mut response_id,
            &mut refusal,
            &mut annotations,
            sink,
        )?;
    }
    if strict_stream && !frame.is_empty() {
        return Err(BackendError::InvalidResponse(
            "unterminated Responses SSE frame".into(),
        ));
    }
    if cancelled() {
        return Err(BackendError::Cancelled);
    }
    if !saw_completed {
        return Err(BackendError::InvalidResponse(
            "Responses stream ended without response.completed".into(),
        ));
    }
    if content.trim().is_empty() {
        if !tool_calls.is_empty() {
            return Ok(Completion {
                content,
                usage,
                model,
                tool_calls,
                response_id,
                refusal,
                annotations,
            });
        }
        return Err(BackendError::EmptyResponse);
    }
    Ok(Completion {
        content,
        usage,
        model,
        tool_calls,
        response_id,
        refusal,
        annotations,
    })
}

fn response_frame_line(line: &[u8], frame: &mut Vec<u8>) -> Result<Option<Vec<u8>>, BackendError> {
    let line = std::str::from_utf8(line)
        .map_err(|_| BackendError::InvalidResponse("invalid Responses SSE UTF-8".into()))?;
    let line =
        line.trim_matches(|character: char| character == '\0' || character.is_ascii_whitespace());
    if line.is_empty() {
        if frame.is_empty() {
            return Ok(None);
        }
        let mut data = b"data:".to_vec();
        data.append(frame);
        return Ok(Some(data));
    }
    if let Some(data) = line.strip_prefix("data:") {
        let data = data.strip_prefix(' ').unwrap_or(data);
        if frame.len().saturating_add(data.len()).saturating_add(1) > MAX_RESPONSE_BYTES {
            return Err(BackendError::InvalidResponse(
                "Responses SSE frame exceeds bound".into(),
            ));
        }
        if !frame.is_empty() {
            frame.push(b'\n');
        }
        frame.extend_from_slice(data.as_bytes());
    } else if !line.starts_with(':')
        && !line.starts_with("event:")
        && !line.starts_with("id:")
        && !line.starts_with("retry:")
    {
        return Err(BackendError::InvalidResponse(
            "invalid Responses SSE field".into(),
        ));
    }
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
fn process_responses_line(
    strict_terminal: bool,
    strict_stream: bool,
    line: &[u8],
    content: &mut String,
    usage: &mut Option<Usage>,
    model: &mut Option<String>,
    saw_completed: &mut bool,
    tool_calls: &mut Vec<ToolCall>,
    response_id: &mut Option<String>,
    refusal: &mut Option<String>,
    annotations: &mut Vec<Value>,
    sink: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
) -> Result<(), BackendError> {
    let line = std::str::from_utf8(line).map_err(|error| {
        BackendError::InvalidResponse(format!("Responses stream is not valid UTF-8: {error}"))
    })?;
    let clean_line =
        line.trim_matches(|character: char| character == '\0' || character.is_ascii_whitespace());
    if clean_line.starts_with('{') && !clean_line.contains("data:") {
        let payload: Value = serde_json::from_str(clean_line).map_err(|error| {
            BackendError::InvalidResponse(format!("invalid Responses JSON: {error}"))
        })?;
        validate_responses_terminal(&payload, strict_terminal)?;
        let completion = completion_from_responses_json(&payload)?;
        emit_completion_events(&completion, sink)?;
        *content = completion.content;
        *usage = completion.usage;
        *model = completion.model;
        *tool_calls = completion.tool_calls;
        *response_id = completion.response_id;
        *refusal = completion.refusal;
        *annotations = completion.annotations;
        *saw_completed = true;
        return Ok(());
    }
    let Some(data) = clean_line.strip_prefix("data:") else {
        return Ok(());
    };
    // Some compatible gateways pad SSE frames with NUL bytes between
    // events. They are transport padding, not part of the JSON payload.
    let data =
        data.trim_matches(|character: char| character == '\0' || character.is_ascii_whitespace());
    if data.is_empty() || data == "[DONE]" {
        if strict_stream && data == "[DONE]" {
            return Err(BackendError::InvalidResponse(
                "Responses does not use a DONE terminator".into(),
            ));
        }
        return Ok(());
    }
    if strict_stream && *saw_completed {
        return Err(BackendError::InvalidResponse(
            "Responses data after completed terminal".into(),
        ));
    }
    let event: Value = serde_json::from_str(data)
        .map_err(|error| BackendError::InvalidResponse(format!("invalid SSE event: {error}")))?;
    match event.get("type").and_then(Value::as_str) {
        Some("response.output_text.delta") => {
            if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                content.push_str(delta);
                sink(ProviderEvent::TextDelta {
                    delta: delta.to_owned(),
                })?;
            }
        }
        Some("response.output_text.done") => {
            if let Some(value) = event.get("text").and_then(Value::as_str) {
                if content.is_empty() {
                    content.push_str(value);
                }
                sink(ProviderEvent::TextDone {
                    text: value.to_owned(),
                })?;
            }
        }
        Some("response.reasoning_text.delta" | "response.reasoning_summary_text.delta")
            if strict_stream =>
        {
            let delta = event["delta"].as_str().ok_or_else(|| {
                BackendError::InvalidResponse("invalid Responses reasoning delta".into())
            })?;
            sink(ProviderEvent::ReasoningDelta {
                delta: delta.to_owned(),
            })?;
        }
        Some("response.created") => {
            let response = event.get("response").unwrap_or(&event);
            *response_id = response
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_owned);
            *model = response
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_owned);
            sink(ProviderEvent::ResponseCreated {
                response_id: response_id.clone(),
                model: model.clone(),
            })?;
        }
        Some("response.output_item.done") => {
            if strict_stream {
                let mut output = json!({"output":[event["item"].clone()]});
                validate_strict_output(&mut output)?;
            }
            if let Some(item) = event.get("item")
                && item.get("type").and_then(Value::as_str) == Some("function_call")
                && let Some(call) = parse_response_function_call(item)
            {
                if strict_stream {
                    if push_checked_tool_call(tool_calls, call.clone())? {
                        sink(ProviderEvent::ToolCallDone { call })?;
                    }
                } else {
                    push_unique_tool_call(tool_calls, call);
                    if let Some(call) = tool_calls.last().cloned() {
                        sink(ProviderEvent::ToolCallDone { call })?;
                    }
                }
            }
        }
        Some("response.function_call_arguments.delta") => {
            sink(ProviderEvent::ToolCallDelta {
                call_id: event
                    .get("call_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                name: event.get("name").and_then(Value::as_str).map(str::to_owned),
                arguments_delta: event
                    .get("delta")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            })?;
        }
        Some("response.function_call_arguments.done") => {
            if strict_stream {
                let arguments = event["arguments"].as_str().ok_or_else(|| {
                    BackendError::InvalidResponse("incomplete Responses function arguments".into())
                })?;
                if !serde_json::from_str::<Value>(arguments).is_ok_and(|value| value.is_object()) {
                    return Err(BackendError::InvalidResponse(
                        "malformed Responses function arguments".into(),
                    ));
                }
            }
            if let Some(call_id) = event.get("call_id").and_then(Value::as_str)
                && let Some(name) = event.get("name").and_then(Value::as_str)
            {
                let arguments = event
                    .get("arguments")
                    .and_then(Value::as_str)
                    .unwrap_or("{}");
                let call = parse_tool_call(call_id, name, arguments)?;
                if strict_stream {
                    if push_checked_tool_call(tool_calls, call.clone())? {
                        sink(ProviderEvent::ToolCallDone { call })?;
                    }
                } else {
                    push_unique_tool_call(tool_calls, call.clone());
                    sink(ProviderEvent::ToolCallDone { call })?;
                }
            }
        }
        Some("response.completed") => {
            let mut checked = event.get("response").unwrap_or(&event).clone();
            if strict_stream {
                validate_responses_terminal(&checked, true)?;
                validate_strict_output(&mut checked)?;
                let calls = extract_responses_tool_calls(&checked)?;
                for call in tool_calls.iter() {
                    if !calls.contains(call) {
                        return Err(BackendError::InvalidResponse(
                            "Responses terminal tool calls conflict with stream".into(),
                        ));
                    }
                }
                for call in calls {
                    if push_checked_tool_call(tool_calls, call.clone())? {
                        sink(ProviderEvent::ToolCallDone { call })?;
                    }
                }
                if let Ok(text) = extract_responses_content(&checked) {
                    if !content.is_empty() && *content != text {
                        return Err(BackendError::InvalidResponse(
                            "Responses terminal text conflicts with stream".into(),
                        ));
                    }
                    if content.is_empty() {
                        if !text.is_empty() {
                            sink(ProviderEvent::TextDone { text: text.clone() })?;
                        }
                        *content = text;
                    }
                }
            }
            let response = &checked;
            validate_responses_terminal(response, strict_terminal)?;
            *saw_completed = true;
            *usage = response.get("usage").and_then(parse_usage);
            *model = response
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_owned);
            *response_id = response
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| response_id.clone());
            *refusal = extract_responses_refusal(response);
            *annotations = extract_annotations(response);
            if let Some(usage_value) = response.get("usage")
                && let Some(parsed) = parse_usage(usage_value)
            {
                sink(ProviderEvent::Usage { usage: parsed })?;
            }
            sink(ProviderEvent::Completed {
                response_id: response_id.clone(),
                model: model.clone(),
            })?;
        }
        Some("response.refusal.delta") | Some("response.refusal.done") => {
            if let Some(value) = event
                .get("delta")
                .or_else(|| event.get("text"))
                .and_then(Value::as_str)
            {
                refusal.get_or_insert_with(String::new).push_str(value);
                sink(ProviderEvent::Refusal {
                    text: value.to_owned(),
                })?;
            }
        }
        Some("response.incomplete") if strict_stream => {
            return Err(BackendError::InvalidResponse(
                "Responses incomplete terminal".into(),
            ));
        }
        Some("response.failed") | Some("error") => {
            let message = extract_responses_error(&event);
            let _ = sink(ProviderEvent::Failed {
                message: message.clone(),
            });
            return Err(BackendError::InvalidResponse(message));
        }
        _ => {}
    }
    Ok(())
}

fn push_checked_tool_call(calls: &mut Vec<ToolCall>, call: ToolCall) -> Result<bool, BackendError> {
    validate_checked_call(&call)?;
    if let Some(existing) = calls.iter().find(|existing| existing.id == call.id) {
        if existing != &call {
            return Err(BackendError::InvalidResponse(
                "conflicting Responses function call".into(),
            ));
        }
        Ok(false)
    } else {
        if calls.len() >= 128 {
            return Err(BackendError::InvalidResponse(
                "too many Responses function calls".into(),
            ));
        }
        calls.push(call);
        Ok(true)
    }
}

fn validate_checked_call(call: &ToolCall) -> Result<(), BackendError> {
    if !call.arguments.is_object()
        || call.id.is_empty()
        || call.id.len() > 4096
        || call.id.chars().any(char::is_control)
        || call.name.is_empty()
        || call.name.len() > 64
        || !call
            .name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(BackendError::InvalidResponse(
            "invalid Responses function call".into(),
        ));
    }
    Ok(())
}

fn completion_from_responses_json(payload: &Value) -> Result<Completion, BackendError> {
    let tool_calls = extract_responses_tool_calls(payload)?;
    let content = extract_responses_content(payload).or_else(|error| {
        if tool_calls.is_empty() {
            Err(error)
        } else {
            Ok(String::new())
        }
    })?;
    if content.trim().is_empty() && tool_calls.is_empty() {
        return Err(BackendError::EmptyResponse);
    }
    Ok(Completion {
        content,
        usage: payload.get("usage").and_then(parse_usage),
        model: payload
            .get("model")
            .and_then(Value::as_str)
            .map(str::to_owned),
        tool_calls,
        response_id: payload.get("id").and_then(Value::as_str).map(str::to_owned),
        refusal: extract_responses_refusal(payload),
        annotations: extract_annotations(payload),
    })
}

fn push_unique_tool_call(calls: &mut Vec<ToolCall>, call: ToolCall) {
    if !calls.iter().any(|existing| existing.id == call.id) {
        calls.push(call);
    }
}

fn extract_responses_tool_calls(payload: &Value) -> Result<Vec<ToolCall>, BackendError> {
    let Some(output) = payload.get("output").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    output
        .iter()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("function_call"))
        .map(parse_response_function_call)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| BackendError::InvalidResponse("response function call is incomplete".into()))
}

fn parse_response_function_call(item: &Value) -> Option<ToolCall> {
    if item
        .get("status")
        .is_some_and(|status| status != "completed")
    {
        return None;
    }
    let id = item
        .get("call_id")
        .or_else(|| item.get("id"))
        .and_then(Value::as_str)?;
    let name = item.get("name").and_then(Value::as_str)?;
    let arguments = item
        .get("arguments")
        .and_then(Value::as_str)
        .unwrap_or("{}");
    parse_tool_call(id, name, arguments).ok()
}

fn extract_responses_error(event: &Value) -> String {
    let error = event.get("error").or_else(|| {
        event
            .get("response")
            .and_then(|response| response.get("error"))
    });
    match error {
        Some(Value::String(message)) => message.to_owned(),
        Some(Value::Object(error)) => error
            .get("message")
            .and_then(Value::as_str)
            .or_else(|| error.get("code").and_then(Value::as_str))
            .unwrap_or("Responses stream failed")
            .to_owned(),
        _ => "Responses stream failed".to_owned(),
    }
}

fn validate_responses_terminal(payload: &Value, strict: bool) -> Result<(), BackendError> {
    if payload
        .get("status")
        .is_some_and(|status| status != "completed")
        || (strict && payload["status"] != "completed")
        || payload
            .get("incomplete_details")
            .is_some_and(|details| !details.is_null())
    {
        return Err(BackendError::InvalidResponse(
            "Responses missing or incomplete terminal status".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::extract_responses_content;
    use serde_json::json;

    #[test]
    fn responses_output_text_variants_are_extracted() {
        assert_eq!(
            extract_responses_content(&json!({"output_text":"top-level"})).unwrap(),
            "top-level"
        );
        assert_eq!(
            extract_responses_content(&json!({
                "output": [
                    {"type":"reasoning","summary":[]},
                    {"type":"message","content":[
                        {"type":"output_text","text":"first"},
                        {"type":"output_text","text":" second"}
                    ]}
                ]
            }))
            .unwrap(),
            "first second"
        );
    }
}
