//! ZE-105: recorded provider SSE replay suite.
//!
//! Every fixture under `tests/fixtures/sse/<wire>/` is served verbatim over a
//! local `std::net::TcpListener` as an HTTP `text/event-stream` response and
//! consumed through zenpi's real `OpenAiCompatibleBackend` for that wire. The
//! normalized `ProviderEvent` trace and the final `Completion`/`BackendError`
//! are asserted exactly.
//!
//! Provenance: `kill319-*` are the deduplicated captures from
//! `Docs/quality/stage1/provider-kill319/master-review-3.1.19/actual/**`,
//! `zs1-112-*` come from `Docs/quality/stage1/ZS1-112/.../worker-public319/**`
//! and `zs1-117-*` reproduce the Responses streams emitted by the ZS1-117
//! host smoke server. `variant-*` fixtures are one-field edits of a capture
//! that pin behavior a later blueprint item is scheduled to change; each such
//! assertion names that item so it flips the expectation when it lands.

use serde_json::json;
use std::{
    io::{ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};
use zenpi::backend::{
    Backend, BackendError, Completion, CompletionRequest, OpenAiCompatibleBackend, OpenAiWireApi,
    ProviderEvent, StopReason,
};
use zenpi::{
    core::{Turn, TurnRole},
    providers::registry::ModelRegistry,
    tools::{ToolDefinition, ToolSideEffect},
};

#[derive(Clone, Copy)]
enum Wire {
    Chat,
    Responses,
    Anthropic,
    Gemini,
}

impl Wire {
    fn dir(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Responses => "responses",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
        }
    }
    fn model(self) -> &'static str {
        match self {
            Self::Chat | Self::Responses => "gpt-4.1",
            Self::Anthropic => "claude-sonnet-4-6",
            Self::Gemini => "gemini-2.5-flash",
        }
    }
    fn api(self) -> OpenAiWireApi {
        match self {
            Self::Chat => OpenAiWireApi::ChatCompletions,
            Self::Responses => OpenAiWireApi::Responses,
            Self::Anthropic => OpenAiWireApi::AnthropicMessages,
            Self::Gemini => OpenAiWireApi::GoogleGenerativeAi,
        }
    }
}

/// Drains one HTTP request (headers plus `Content-Length` body) and returns
/// its lowercased header block.
fn drain_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut chunk = [0; 4096];
    let end = loop {
        let n = stream.read(&mut chunk).unwrap();
        assert!(n > 0, "client closed before sending headers");
        bytes.extend_from_slice(&chunk[..n]);
        assert!(bytes.len() < 4 * 1024 * 1024);
        if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..end]).into_owned();
    let length = headers
        .lines()
        .find_map(|l| {
            l.split_once(':')
                .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                .map(|(_, v)| v.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    while bytes.len() < end + length {
        let n = stream.read(&mut chunk).unwrap();
        assert!(n > 0, "client closed before sending body");
        bytes.extend_from_slice(&chunk[..n]);
    }
    headers.to_ascii_lowercase()
}

fn fixture(wire: Wire, name: &str) -> Vec<u8> {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests/fixtures/sse",
        wire.dir(),
        name,
    ]
    .iter()
    .collect();
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

struct Replay {
    trace: Vec<String>,
    result: Result<Completion, BackendError>,
    /// Lowercased request headers the backend sent.
    headers: String,
}

impl Replay {
    fn completion(&self) -> &Completion {
        self.result
            .as_ref()
            .unwrap_or_else(|e| panic!("expected completion, got {e}; trace {:?}", self.trace))
    }
    fn error(&self) -> String {
        match &self.result {
            Ok(c) => panic!("expected error, got {c:?}; trace {:?}", self.trace),
            Err(e) => e.to_string(),
        }
    }
}

fn normalize(event: &ProviderEvent) -> String {
    match event {
        ProviderEvent::ResponseCreated { response_id, model } => {
            format!("created {response_id:?} {model:?}")
        }
        ProviderEvent::TextDelta { delta } => format!("text {delta}"),
        ProviderEvent::ReasoningDelta { delta } => format!("reasoning {delta}"),
        ProviderEvent::TextDone { text } => format!("text_done {text}"),
        ProviderEvent::Refusal { text } => format!("refusal {text}"),
        ProviderEvent::ToolCallDelta {
            call_id,
            name,
            arguments_delta,
        } => format!("tool_delta {call_id:?} {name:?} {arguments_delta}"),
        ProviderEvent::ToolCallDone { call } => {
            format!("tool {} {} {}", call.id, call.name, call.arguments)
        }
        ProviderEvent::Usage { usage } => format!(
            "usage {}/{}/{}",
            usage.input_tokens, usage.output_tokens, usage.total_tokens
        ),
        ProviderEvent::Warning { message } => format!("warning {message}"),
        ProviderEvent::Completed { response_id, model } => {
            format!("completed {response_id:?} {model:?}")
        }
        ProviderEvent::Failed { message, retryable } => format!("failed {message} {retryable}"),
    }
}

/// Every tool name that appears in a fixture; schemas stay permissive so
/// the replay exercises the stream parser rather than argument validation.
fn tools() -> Vec<ToolDefinition> {
    ["read_file", "write_file", "find", "search_text"]
        .into_iter()
        .map(|name| ToolDefinition {
            name: name.into(),
            description: format!("replay {name}"),
            input_schema: json!({"type": "object"}),
            side_effect: ToolSideEffect::ReadOnly,
        })
        .collect()
}

/// Serves `tests/fixtures/sse/<wire>/<name>` once and drives the real backend.
fn replay(wire: Wire, name: &str) -> Replay {
    serve(
        wire,
        "200 OK",
        "text/event-stream; charset=utf-8",
        fixture(wire, name),
    )
}

/// Serves one HTTP response with `status` and drives the real backend.
fn serve(wire: Wire, status: &'static str, content_type: &'static str, body: Vec<u8>) -> Replay {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let endpoint = match wire {
        Wire::Chat | Wire::Responses => format!("{base}/v1"),
        Wire::Anthropic | Wire::Gemini => base,
    };
    let backend = OpenAiCompatibleBackend::new_with_wire_api(
        endpoint,
        Some("replay-fixture-key".into()),
        wire.model(),
        wire.api(),
    )
    .unwrap();
    let backend = match wire {
        Wire::Chat | Wire::Responses => backend,
        Wire::Anthropic => backend
            .with_model_registry("anthropic".into(), ModelRegistry::default())
            .unwrap(),
        Wire::Gemini => backend
            .with_model_registry("google".into(), ModelRegistry::default())
            .unwrap(),
    };
    listener.set_nonblocking(true).unwrap();
    let server = thread::spawn(move || {
        // Bounded accept: a backend that fails before connecting must not
        // hang the suite.
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == ErrorKind::WouldBlock && Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => return String::new(),
                Err(e) => panic!("replay server accept: {e}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        let headers = drain_request(&mut stream);
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
            body.len()
        );
        // A strict parser may reject the stream and hang up before the
        // writer finishes; the replay outcome is asserted on the client side.
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&body);
        let _ = stream.flush();
        headers
    });
    let turns = [Turn::new("replay-user", TurnRole::User, "replay")];
    let tools = tools();
    let mut events = Vec::new();
    let result = backend.complete_with_control(
        CompletionRequest::new("replay-user", &turns, None, &tools),
        &|| false,
        &mut |event| {
            events.push(event);
            Ok(())
        },
    );
    let headers = server.join().unwrap();
    Replay {
        trace: events.iter().map(normalize).collect(),
        result,
        headers,
    }
}

fn assert_trace(replay: &Replay, expected: &[&str]) {
    assert_eq!(replay.trace, expected, "result: {:?}", replay.result);
}

fn tool_call(c: &Completion) -> (&str, &str, String) {
    assert_eq!(c.tool_calls.len(), 1, "{c:?}");
    let call = &c.tool_calls[0];
    (&call.id, &call.name, call.arguments.to_string())
}

// ---------------------------------------------------------------- chat

#[test]
fn chat_text_then_tool_call() {
    let r = replay(Wire::Chat, "kill319-text-then-tool.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("chat319") Some("gpt-4.1")"#,
            "text BEFORE_TOOL_319",
            r#"tool_delta Some("durable_read_319") Some("read_file") {"path": "input.txt"}"#,
            "text_done BEFORE_TOOL_319",
            r#"tool durable_read_319 read_file {"path":"input.txt"}"#,
            r#"completed Some("chat319") Some("gpt-4.1")"#,
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "BEFORE_TOOL_319");
    assert_eq!(c.model.as_deref(), Some("gpt-4.1"));
    assert_eq!(c.usage, None);
    assert_eq!(
        tool_call(c),
        (
            "durable_read_319",
            "read_file",
            r#"{"path":"input.txt"}"#.into()
        )
    );
}

#[test]
fn chat_final_text() {
    let r = replay(Wire::Chat, "kill319-final-text.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("chat319") Some("gpt-4.1")"#,
            "text FRESH_FINAL_319",
            "text_done FRESH_FINAL_319",
            r#"completed Some("chat319") Some("gpt-4.1")"#,
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "FRESH_FINAL_319");
    assert_eq!(c.response_id.as_deref(), Some("chat319"));
    assert!(c.tool_calls.is_empty());
}

#[test]
fn chat_stream_cut_after_text_fails() {
    let r = replay(Wire::Chat, "kill319-truncated-text.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("chat319") Some("gpt-4.1")"#,
            "text INTERRUPTED_PARTIAL_319",
        ],
    );
    assert!(r.error().contains("ended without finish_reason"));
}

#[test]
fn chat_stream_cut_inside_tool_arguments_fails_without_dispatch() {
    let r = replay(Wire::Chat, "kill319-partial-tool-args.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("chat319") Some("gpt-4.1")"#,
            "text INTERRUPTED_PARTIAL_319",
            r#"tool_delta Some("incomplete_write_319") Some("write_file") {"path":"NEVER"#,
        ],
    );
    assert!(!r.trace.iter().any(|t| t.starts_with("tool ")));
    assert!(r.error().contains("ended without finish_reason"));
}

#[test]
fn chat_minimal_chunks_without_id_or_model() {
    let r = replay(Wire::Chat, "zs1-112-find-tool.sse");
    assert_trace(
        &r,
        &[
            "created None None",
            r#"tool_delta Some("search-call-319") Some("find") {"pattern": "**/*.rs", "path": "fixture", "ignore": true, "hidden": true}"#,
            r#"tool search-call-319 find {"hidden":true,"ignore":true,"path":"fixture","pattern":"**/*.rs"}"#,
            "completed None None",
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "");
    assert_eq!(c.model, None);
    assert_eq!(tool_call(c).1, "find");

    let r = replay(Wire::Chat, "zs1-112-search-tool.sse");
    let (id, name, args) = tool_call(r.completion());
    assert_eq!((id, name), ("search-call-319", "search_text"));
    assert_eq!(
        args,
        r#"{"case_sensitive":false,"context":1,"glob":"**/*.rs","hidden":true,"ignore":false,"path":"fixture","query":"^needle[0-9]+$","regex":true}"#
    );

    let r = replay(Wire::Chat, "zs1-112-final-text.sse");
    assert_trace(
        &r,
        &[
            "created None None",
            "text SEARCH_FINISHED_319",
            "text_done SEARCH_FINISHED_319",
            "completed None None",
        ],
    );
    assert_eq!(r.completion().content, "SEARCH_FINISHED_319");
}

#[test]
fn chat_length_finish_keeps_partial_text() {
    // ZE-221: finish_reason=length returns the partial text as a length stop.
    let r = replay(Wire::Chat, "variant-length-stop.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("chat319") Some("gpt-4.1")"#,
            "text FRESH_FINAL_319",
            "text_done FRESH_FINAL_319",
            r#"completed Some("chat319") Some("gpt-4.1")"#,
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "FRESH_FINAL_319");
    assert_eq!(c.stop_reason, StopReason::Length);
}

#[test]
fn chat_stop_with_tool_calls_is_accepted() {
    // ZE-226: finish_reason=stop alongside complete tool calls is accepted.
    let r = replay(Wire::Chat, "variant-stop-with-tool.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("chat319") Some("gpt-4.1")"#,
            "text BEFORE_TOOL_319",
            r#"tool_delta Some("durable_read_319") Some("read_file") {"path": "input.txt"}"#,
            "text_done BEFORE_TOOL_319",
            r#"tool durable_read_319 read_file {"path":"input.txt"}"#,
            r#"completed Some("chat319") Some("gpt-4.1")"#,
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "BEFORE_TOOL_319");
    assert_eq!(c.stop_reason, StopReason::ToolUse);
    assert_eq!(tool_call(c).0, "durable_read_319");
}

// ----------------------------------------------------------- responses

#[test]
fn responses_function_call_item() {
    let r = replay(Wire::Responses, "zs1-117-tool-call.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("1") Some("gpt-4.1")"#,
            r#"tool SLOW_A write_file {"content":"SLOW_A","path":"marker.txt"}"#,
            r#"completed Some("1") Some("gpt-4.1")"#,
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "");
    assert_eq!(
        tool_call(c),
        (
            "SLOW_A",
            "write_file",
            r#"{"content":"SLOW_A","path":"marker.txt"}"#.into()
        )
    );
}

#[test]
fn responses_text_with_usage() {
    let r = replay(Wire::Responses, "zs1-117-text-usage.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("compact-3") Some("gpt-4.1")"#,
            "text CONTINUED_AFTER_COMPACT",
            "usage 250/150/400",
            r#"completed Some("compact-3") Some("gpt-4.1")"#,
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "CONTINUED_AFTER_COMPACT");
    let u = c.usage.unwrap();
    assert_eq!(
        (u.input_tokens, u.output_tokens, u.total_tokens),
        (250, 150, 400)
    );
}

#[test]
fn responses_incomplete_max_output_keeps_partial_text() {
    // ZE-221: response.incomplete with reason=max_output_tokens keeps the
    // partial text as a length stop.
    let r = replay(Wire::Responses, "variant-incomplete-max-output.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("compact-3") Some("gpt-4.1")"#,
            "text CONTINUED_AFTER",
            "usage 250/150/400",
            r#"completed Some("compact-3") Some("gpt-4.1")"#,
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "CONTINUED_AFTER");
    assert_eq!(c.stop_reason, StopReason::Length);
}

// ----------------------------------------------------------- anthropic

#[test]
fn anthropic_thinking_text_and_tool_use() {
    let r = replay(Wire::Anthropic, "kill319-thought-text-tool.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("msg319") Some("claude-sonnet-4-6")"#,
            "reasoning SIGNED_THOUGHT_319",
            "text BEFORE_TOOL_319",
            r#"tool_delta Some("durable_read_319") Some("read_file") "#,
            "text_done BEFORE_TOOL_319",
            r#"tool durable_read_319 read_file {"path":"input.txt"}"#,
            "usage 7/4/11",
            r#"completed Some("msg319") Some("claude-sonnet-4-6")"#,
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "BEFORE_TOOL_319");
    assert_eq!(
        tool_call(c),
        (
            "durable_read_319",
            "read_file",
            r#"{"path":"input.txt"}"#.into()
        )
    );
    let history = c
        .annotations
        .iter()
        .find(|a| a["type"] == "native_history")
        .expect("native_history annotation");
    assert_eq!(history["blocks"][0]["signature"], "opaque-signature-319");
    assert!(
        c.annotations
            .iter()
            .any(|a| a["type"] == "anthropic_stop_reason" && a["stop_reason"] == "tool_use")
    );
}

#[test]
fn anthropic_final_text_with_usage() {
    let r = replay(Wire::Anthropic, "kill319-final-text.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("msg319") Some("claude-sonnet-4-6")"#,
            "text FRESH_FINAL_319",
            "text_done FRESH_FINAL_319",
            "usage 7/4/11",
            r#"completed Some("msg319") Some("claude-sonnet-4-6")"#,
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "FRESH_FINAL_319");
    assert!(c.tool_calls.is_empty());
}

#[test]
fn anthropic_streams_cut_before_message_stop_fail() {
    let r = replay(Wire::Anthropic, "kill319-truncated-text.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("msg319") Some("claude-sonnet-4-6")"#,
            "text INTERRUPTED_PARTIAL_319",
        ],
    );
    assert!(r.error().contains("Anthropic stream missing message_stop"));

    let r = replay(Wire::Anthropic, "kill319-partial-tool.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("msg319") Some("claude-sonnet-4-6")"#,
            "text INTERRUPTED_PARTIAL_319",
            r#"tool_delta Some("incomplete_write_319") Some("write_file") "#,
            r#"tool_delta Some("incomplete_write_319") Some("write_file") {"path":"NEVER"#,
        ],
    );
    assert!(r.error().contains("Anthropic stream missing message_stop"));
}

#[test]
fn anthropic_max_tokens_keeps_partial_text() {
    // ZE-221: stop_reason=max_tokens returns the partial text as a length stop.
    let r = replay(Wire::Anthropic, "variant-max-tokens.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("msg319") Some("claude-sonnet-4-6")"#,
            "text FRESH_FINAL_319",
            "text_done FRESH_FINAL_319",
            "usage 7/4/11",
            r#"completed Some("msg319") Some("claude-sonnet-4-6")"#,
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "FRESH_FINAL_319");
    assert_eq!(c.stop_reason, StopReason::Length);
}

#[test]
fn anthropic_unknown_event_is_ignored() {
    // ZE-226: unknown Anthropic stream events are ignored.
    let r = replay(Wire::Anthropic, "variant-unknown-event.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("msg319") Some("claude-sonnet-4-6")"#,
            "text FRESH_FINAL_319",
            "text_done FRESH_FINAL_319",
            "usage 7/4/11",
            r#"completed Some("msg319") Some("claude-sonnet-4-6")"#,
        ],
    );
    assert_eq!(r.completion().content, "FRESH_FINAL_319");
}

#[test]
fn anthropic_unknown_delta_type_is_ignored_but_mismatched_known_delta_fails() {
    // ZE-226: a new delta kind (citations) on a text block is ignored; a known
    // delta on the wrong block type still fails.
    let body = |delta: &str| {
        format!(
            concat!(
                "event: message_start\ndata: {{\"type\":\"message_start\",\"message\":{{\"type\":\"message\",\"id\":\"m1\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-6\",\"content\":[],\"stop_reason\":null,\"usage\":{{\"input_tokens\":1,\"output_tokens\":0}}}}}}\n\n",
                "event: content_block_start\ndata: {{\"type\":\"content_block_start\",\"index\":0,\"content_block\":{{\"type\":\"text\",\"text\":\"A\"}}}}\n\n",
                "event: content_block_delta\ndata: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{}}}\n\n",
                "event: content_block_stop\ndata: {{\"type\":\"content_block_stop\",\"index\":0}}\n\n",
                "event: message_delta\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"end_turn\"}},\"usage\":{{\"output_tokens\":1}}}}\n\n",
                "event: message_stop\ndata: {{\"type\":\"message_stop\"}}\n\n",
            ),
            delta
        )
        .into_bytes()
    };
    let r = serve(
        Wire::Anthropic,
        "200 OK",
        "text/event-stream",
        body(r#"{"type":"citations_delta","citation":{"type":"char_location","cited_text":"x"}}"#),
    );
    assert_eq!(r.completion().content, "A");
    let r = serve(
        Wire::Anthropic,
        "200 OK",
        "text/event-stream",
        body(r#"{"type":"input_json_delta","partial_json":"{}"}"#),
    );
    assert!(
        r.error()
            .contains("Anthropic delta type does not match block")
    );
}

#[test]
fn anthropic_dated_serving_model_is_recorded() {
    // ZE-223: a dated model id resolved from the requested alias is
    // recorded, not rejected.
    let r = replay(Wire::Anthropic, "variant-dated-model.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("msg319") Some("claude-sonnet-4-6-20260101")"#,
            "text FRESH_FINAL_319",
            "text_done FRESH_FINAL_319",
            "usage 7/4/11",
            r#"completed Some("msg319") Some("claude-sonnet-4-6-20260101")"#,
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "FRESH_FINAL_319");
    assert_eq!(c.model.as_deref(), Some("claude-sonnet-4-6-20260101"));
    let history = c
        .annotations
        .iter()
        .find(|a| a["type"] == "native_history")
        .expect("native_history annotation");
    assert_eq!(history["model"], "claude-sonnet-4-6-20260101");
    assert_eq!(history["requested_model"], "claude-sonnet-4-6");
}

#[test]
fn legacy_anthropic_sends_native_auth_headers() {
    // ZE-223: backend="anthropic" without an explicit connection uses
    // x-api-key + anthropic-version, never a Bearer token.
    let r = replay(Wire::Anthropic, "kill319-final-text.sse");
    r.completion();
    assert!(
        r.headers.contains("\r\nx-api-key: replay-fixture-key\r\n"),
        "{}",
        r.headers
    );
    assert!(r.headers.contains("\r\nanthropic-version: 2023-06-01\r\n"));
    assert!(!r.headers.contains("authorization:"), "{}", r.headers);
    // The OpenAI wires keep Bearer.
    let r = replay(Wire::Chat, "kill319-final-text.sse");
    r.completion();
    assert!(
        r.headers
            .contains("\r\nauthorization: bearer replay-fixture-key\r\n")
    );
    assert!(!r.headers.contains("x-api-key:"));
}

// -------------------------------------------------------------- gemini

#[test]
fn gemini_thought_text_and_function_call() {
    let r = replay(Wire::Gemini, "kill319-thought-text-tool.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("google319") Some("gemini-2.5-flash")"#,
            "reasoning SIGNED_THOUGHT_319",
            "text BEFORE_TOOL_319",
            r#"tool_delta Some("durable_read_319") Some("read_file") {"path":"input.txt"}"#,
            "text_done BEFORE_TOOL_319",
            r#"tool durable_read_319 read_file {"path":"input.txt"}"#,
            r#"completed Some("google319") Some("gemini-2.5-flash")"#,
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "BEFORE_TOOL_319");
    assert_eq!(c.usage, None);
    assert_eq!(
        tool_call(c),
        (
            "durable_read_319",
            "read_file",
            r#"{"path":"input.txt"}"#.into()
        )
    );
    let history = c
        .annotations
        .iter()
        .find(|a| a["type"] == "native_history")
        .expect("native_history annotation");
    assert_eq!(history["parts"][2]["thoughtSignature"], "dG9vbC0zMTk=");
}

#[test]
fn gemini_final_text() {
    let r = replay(Wire::Gemini, "kill319-final-text.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("google319") Some("gemini-2.5-flash")"#,
            "text FRESH_FINAL_319",
            "text_done FRESH_FINAL_319",
            r#"completed Some("google319") Some("gemini-2.5-flash")"#,
        ],
    );
    assert_eq!(r.completion().content, "FRESH_FINAL_319");
}

#[test]
fn gemini_streams_without_finish_reason_fail() {
    let r = replay(Wire::Gemini, "kill319-truncated-text.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("google319") Some("gemini-2.5-flash")"#,
            "text INTERRUPTED_PARTIAL_319",
        ],
    );
    assert!(
        r.error()
            .contains("Gemini response missing complete terminal content")
    );

    let r = replay(Wire::Gemini, "kill319-partial-tool.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("google319") Some("gemini-2.5-flash")"#,
            "text INTERRUPTED_PARTIAL_319",
            r#"tool_delta Some("incomplete_write_319") Some("write_file") {"content":"must not execute","path":"NEVER.txt"}"#,
        ],
    );
    assert!(!r.trace.iter().any(|t| t.starts_with("tool ")));
    assert!(
        r.error()
            .contains("Gemini response missing complete terminal content")
    );
}

#[test]
fn gemini_max_tokens_keeps_partial_text() {
    // ZE-221: finishReason=MAX_TOKENS returns the partial text as a length stop.
    let r = replay(Wire::Gemini, "variant-max-tokens.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("google319") Some("gemini-2.5-flash")"#,
            "text FRESH_FINAL_319",
            "text_done FRESH_FINAL_319",
            r#"completed Some("google319") Some("gemini-2.5-flash")"#,
        ],
    );
    let c = r.completion();
    assert_eq!(c.content, "FRESH_FINAL_319");
    assert_eq!(c.stop_reason, StopReason::Length);
}

#[test]
fn gemini_total_above_component_sum_is_accepted() {
    // ZE-226: totalTokenCount only has to be >= the sum of the counted
    // components (toolUsePromptTokenCount is not counted); it is reported.
    let r = replay(Wire::Gemini, "variant-usage-total-exceeds-sum.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("google319") Some("gemini-2.5-flash")"#,
            "text FRESH_FINAL_319",
            "text_done FRESH_FINAL_319",
            "usage 7/4/13",
            r#"completed Some("google319") Some("gemini-2.5-flash")"#,
        ],
    );
    assert_eq!(r.completion().content, "FRESH_FINAL_319");
}

// ------------------------------------------------- ZE-220 error bodies

/// Real overflow bodies per wire, as the providers return them.
fn overflow_body(wire: Wire) -> &'static str {
    match wire {
        Wire::Chat | Wire::Responses => {
            r#"{"error":{"message":"This model's maximum context length is 128000 tokens. However, your messages resulted in 130512 tokens.","type":"invalid_request_error","param":"messages","code":"context_length_exceeded"}}"#
        }
        Wire::Anthropic => {
            r#"{"type":"error","error":{"type":"invalid_request_error","message":"prompt is too long: 212345 tokens > 200000 maximum"}}"#
        }
        Wire::Gemini => {
            r#"{"error":{"code":400,"message":"The input token count (1210000) exceeds the maximum number of tokens allowed (1048576).","status":"INVALID_ARGUMENT"}}"#
        }
    }
}

#[test]
fn http_overflow_bodies_classify_as_context_overflow_on_every_wire() {
    for wire in [Wire::Chat, Wire::Responses, Wire::Anthropic, Wire::Gemini] {
        for status in ["400 Bad Request", "413 Payload Too Large"] {
            let r = serve(
                wire,
                status,
                "application/json",
                overflow_body(wire).as_bytes().to_vec(),
            );
            assert!(r.trace.is_empty(), "{:?}", r.trace);
            match &r.result {
                Err(error @ BackendError::ContextOverflow { message }) => {
                    assert!(!error.is_retryable());
                    assert_eq!(error.code(), "context_overflow");
                    assert_eq!(message, overflow_body(wire));
                }
                other => panic!("{} {status}: {other:?}", wire.dir()),
            }
        }
    }
}

#[test]
fn http_error_body_is_kept_bounded_and_unclassified_errors_stay_http_status() {
    let long = format!(
        r#"{{"error":{{"message":"bad tool schema {}"}}}}"#,
        "x".repeat(10_000)
    );
    let r = serve(
        Wire::Chat,
        "400 Bad Request",
        "application/json",
        long.into_bytes(),
    );
    match &r.result {
        Err(
            error @ BackendError::HttpStatus {
                status: 400,
                body: Some(body),
                ..
            },
        ) => {
            assert!(!error.is_retryable());
            assert_eq!(body.len(), zenpi::backend::MAX_ERROR_BODY_BYTES);
            assert!(body.starts_with(r#"{"error":{"message":"bad tool schema xxx"#));
            assert!(error.to_string().contains("bad tool schema"));
        }
        other => panic!("{other:?}"),
    }
    // An empty 413 is the payload limit itself.
    let r = serve(
        Wire::Anthropic,
        "413 Payload Too Large",
        "text/plain",
        Vec::new(),
    );
    assert!(
        matches!(r.result, Err(BackendError::ContextOverflow { .. })),
        "{:?}",
        r.result
    );
}

#[test]
fn stream_error_events_keep_text_and_classify_overflow() {
    let cases = [
        (
            Wire::Chat,
            "data: {\"error\":{\"message\":\"maximum context length is 8192 tokens\",\"code\":\"context_length_exceeded\"}}\n\n",
        ),
        (
            Wire::Anthropic,
            "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"invalid_request_error\",\"message\":\"prompt is too long: 9 tokens > 8 maximum\"}}\n\n",
        ),
        (
            Wire::Gemini,
            "data: {\"error\":{\"code\":400,\"message\":\"The input token count (9) exceeds the maximum number of tokens allowed (8).\",\"status\":\"INVALID_ARGUMENT\"}}\n\n",
        ),
        (
            Wire::Responses,
            "data: {\"type\":\"response.failed\",\"response\":{\"id\":\"r1\",\"status\":\"failed\",\"error\":{\"code\":\"context_length_exceeded\",\"message\":\"Your input exceeds the context window of this model.\"}}}\n\n",
        ),
    ];
    for (wire, body) in cases {
        let r = serve(
            wire,
            "200 OK",
            "text/event-stream",
            body.as_bytes().to_vec(),
        );
        match &r.result {
            Err(error @ BackendError::ContextOverflow { message }) => {
                assert!(!error.is_retryable());
                assert!(!message.is_empty());
            }
            other => panic!("{}: {other:?}", wire.dir()),
        }
    }
    // A non-overflow stream error keeps the provider's text.
    let r = serve(
        Wire::Anthropic,
        "200 OK",
        "text/event-stream",
        b"event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n".to_vec(),
    );
    assert!(
        r.error().contains("overloaded_error: Overloaded"),
        "{}",
        r.error()
    );
    let r = serve(
        Wire::Responses,
        "200 OK",
        "text/event-stream",
        b"data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"code\":\"server_error\",\"message\":\"boom\"}}}\n\n".to_vec(),
    );
    assert_eq!(
        r.trace,
        [
            "failed backend returned an invalid response: Responses provider error: server_error: boom false"
        ]
    );
}

// ------------------------------------------- ZE-221 length-cut tool calls

#[test]
fn length_cut_tool_arguments_are_reported_not_rejected() {
    let chat = concat!(
        "data: {\"id\":\"c1\",\"model\":\"gpt-4.1\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"cut-1\",\"type\":\"function\",\"function\":{\"name\":\"write_file\",\"arguments\":\"{\\\"path\\\":\\\"NEV\"}}]},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"c1\",\"model\":\"gpt-4.1\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"length\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    let anthropic = concat!(
        "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"type\":\"message\",\"id\":\"m1\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-6\",\"content\":[],\"stop_reason\":null,\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"cut-1\",\"name\":\"write_file\",\"input\":{}}}\n\n",
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"path\\\":\\\"NEV\"}}\n\n",
        "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"max_tokens\"},\"usage\":{\"output_tokens\":9}}\n\n",
        "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
    );
    for (wire, body) in [(Wire::Chat, chat), (Wire::Anthropic, anthropic)] {
        let r = serve(
            wire,
            "200 OK",
            "text/event-stream",
            body.as_bytes().to_vec(),
        );
        let c = r.completion();
        assert_eq!(c.stop_reason, StopReason::Length, "{}", wire.dir());
        assert_eq!(
            tool_call(c),
            ("cut-1", "write_file", "{}".into()),
            "{}",
            wire.dir()
        );
    }
    // Without a length stop the same cut input is still malformed.
    let r = serve(
        Wire::Anthropic,
        "200 OK",
        "text/event-stream",
        anthropic.replace("max_tokens", "tool_use").into_bytes(),
    );
    assert!(r.error().contains("malformed Anthropic tool input JSON"));
}
