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
    ProviderEvent,
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

/// Drains one HTTP request (headers plus `Content-Length` body).
fn drain_request(stream: &mut TcpStream) {
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
        ProviderEvent::Failed { message } => format!("failed {message}"),
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
    let body = fixture(wire, name);
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
                Err(e) if e.kind() == ErrorKind::WouldBlock => return,
                Err(e) => panic!("replay server accept: {e}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        drain_request(&mut stream);
        let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n";
        // A strict parser may reject the stream and hang up before the
        // writer finishes; the replay outcome is asserted on the client side.
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&body);
        let _ = stream.flush();
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
    server.join().unwrap();
    Replay {
        trace: events.iter().map(normalize).collect(),
        result,
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
fn chat_length_finish_is_currently_an_error() {
    // Known bug, ZE-221 flips this: finish_reason=length must return the
    // partial text with StopReason::Length instead of failing the turn.
    let r = replay(Wire::Chat, "variant-length-stop.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("chat319") Some("gpt-4.1")"#,
            "text FRESH_FINAL_319",
        ],
    );
    assert!(
        r.error()
            .contains("Chat output truncated (finish_reason=length)")
    );
}

#[test]
fn chat_stop_with_tool_calls_is_currently_an_error() {
    // Known bug, ZE-226 flips this: Chat must accept finish_reason=stop
    // alongside complete tool calls.
    let r = replay(Wire::Chat, "variant-stop-with-tool.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("chat319") Some("gpt-4.1")"#,
            "text BEFORE_TOOL_319",
            r#"tool_delta Some("durable_read_319") Some("read_file") {"path": "input.txt"}"#,
        ],
    );
    assert!(
        r.error()
            .contains("Chat finish_reason does not match tool calls")
    );
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
fn responses_incomplete_max_output_is_currently_an_error() {
    // Known bug, ZE-221 flips this: response.incomplete with
    // reason=max_output_tokens must keep the partial text as StopReason::Length.
    let r = replay(Wire::Responses, "variant-incomplete-max-output.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("compact-3") Some("gpt-4.1")"#,
            "text CONTINUED_AFTER",
        ],
    );
    assert!(
        r.error()
            .contains("Responses stream ended without response.completed")
    );
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
fn anthropic_max_tokens_is_currently_an_error() {
    // Known bug, ZE-221 flips this: stop_reason=max_tokens must return the
    // partial text with StopReason::Length.
    let r = replay(Wire::Anthropic, "variant-max-tokens.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("msg319") Some("claude-sonnet-4-6")"#,
            "text FRESH_FINAL_319",
        ],
    );
    assert!(
        r.error()
            .contains("incomplete/unsupported Anthropic stop reason max_tokens")
    );
}

#[test]
fn anthropic_unknown_event_is_currently_an_error() {
    // Known bug, ZE-226 flips this: unknown Anthropic stream events must be
    // ignored rather than failing the turn.
    let r = replay(Wire::Anthropic, "variant-unknown-event.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("msg319") Some("claude-sonnet-4-6")"#,
            "text FRESH_FINAL_319",
        ],
    );
    assert!(r.error().contains("unsupported Anthropic stream event"));
}

#[test]
fn anthropic_dated_serving_model_is_currently_rejected() {
    // Known bug, ZE-223 flips this: a dated model id resolved from the
    // requested alias must be recorded, not rejected.
    let r = replay(Wire::Anthropic, "variant-dated-model.sse");
    assert!(r.trace.is_empty(), "{:?}", r.trace);
    assert!(r.error().contains("unexpected Anthropic serving model"));
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
fn gemini_max_tokens_is_currently_an_error() {
    // Known bug, ZE-221 flips this: finishReason=MAX_TOKENS must return the
    // partial text with StopReason::Length.
    let r = replay(Wire::Gemini, "variant-max-tokens.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("google319") Some("gemini-2.5-flash")"#,
            "text FRESH_FINAL_319",
        ],
    );
    assert!(
        r.error()
            .contains("Gemini incomplete/unsupported finish reason: MAX_TOKENS")
    );
}

#[test]
fn gemini_total_above_component_sum_is_currently_an_error() {
    // Known bug, ZE-226 flips this: totalTokenCount only has to be >= the sum
    // of the counted components (toolUsePromptTokenCount is not counted).
    let r = replay(Wire::Gemini, "variant-usage-total-exceeds-sum.sse");
    assert_trace(
        &r,
        &[
            r#"created Some("google319") Some("gemini-2.5-flash")"#,
            "text FRESH_FINAL_319",
        ],
    );
    assert!(r.error().contains("inconsistent/decreasing Gemini usage"));
}
