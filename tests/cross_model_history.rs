//! ZE-227: history recorded by one provider/model replays on another.
//!
//! Each case runs a real session over a local SSE server, switches the
//! connection with `/model`'s owner path (`Agent::select_connection`) and
//! checks that the next request succeeds with opaque reasoning dropped,
//! visible thinking kept as text and tool-call IDs valid for the new wire.
//! Same-model replay is asserted byte-for-byte against the stored blocks.
mod support;

use serde_json::{Value, json};
use std::{
    io::{ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use support::scripted::{FakeServer, call, text};
use tempfile::tempdir;
use zenpi::{
    backend::{Backend, OpenAiCompatibleBackend, OpenAiWireApi},
    core::{Agent, ConnectionSelection, ConnectionSelectionOutcome, TurnInputRequest, TurnRole},
    providers::registry::ModelRegistry,
    session::SessionStore,
    tools::{SideEffectPolicy, ToolContext, ToolRegistry},
};

/// Serves `bodies` as SSE, one per request, and records each request body.
struct SseServer {
    url: String,
    bodies: Arc<Mutex<Vec<Value>>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl SseServer {
    fn start(replies: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let bodies = Arc::new(Mutex::new(Vec::new()));
        let seen = bodies.clone();
        let thread = thread::spawn(move || {
            for reply in replies {
                let deadline = Instant::now() + Duration::from_secs(10);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(e)
                            if e.kind() == ErrorKind::WouldBlock && Instant::now() < deadline =>
                        {
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => return,
                    }
                };
                seen.lock().unwrap().push(read_body(&mut stream));
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    reply.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(reply.as_bytes());
                let _ = stream.flush();
            }
        });
        Self {
            url,
            bodies,
            thread: Some(thread),
        }
    }

    fn bodies(&self) -> Vec<Value> {
        self.bodies.lock().unwrap().clone()
    }
}

impl Drop for SseServer {
    fn drop(&mut self) {
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

fn read_body(stream: &mut TcpStream) -> Value {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    let end = loop {
        let n = stream.read(&mut chunk).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&chunk[..n]);
        if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let head = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
    let length = head
        .lines()
        .find_map(|l| l.strip_prefix("content-length:"))
        .map_or(0, |v| v.trim().parse::<usize>().unwrap());
    while bytes.len() < end + length {
        let n = stream.read(&mut chunk).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&chunk[..n]);
    }
    serde_json::from_slice(&bytes[end..end + length]).unwrap()
}

fn sse(events: &[Value]) -> String {
    events
        .iter()
        .map(|event| match event["type"].as_str() {
            Some(name) if !name.starts_with("response.") => {
                format!("event: {name}\ndata: {event}\n\n")
            }
            _ => format!("data: {event}\n\n"),
        })
        .collect()
}

fn claude_reply(id: &str, thinking: Option<(&str, &str)>, answer: &str) -> String {
    let mut events = vec![
        json!({"type":"message_start","message":{"type":"message","id":id,"role":"assistant","model":"claude-sonnet-4-6","content":[],"stop_reason":null,"usage":{"input_tokens":3,"output_tokens":0}}}),
    ];
    let mut index = 0;
    if let Some((thought, signature)) = thinking {
        events.extend([
            json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":thought}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":signature}}),
            json!({"type":"content_block_stop","index":0}),
        ]);
        index = 1;
    }
    events.extend([
        json!({"type":"content_block_start","index":index,"content_block":{"type":"text","text":answer}}),
        json!({"type":"content_block_stop","index":index}),
        json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":5}}),
        json!({"type":"message_stop"}),
    ]);
    sse(&events)
}

fn gemini_reply(id: &str, parts: Value) -> String {
    sse(&[json!({
        "responseId": id,
        "modelVersion": "gemini-2.5-flash",
        "candidates": [{"index":0,"content":{"role":"model","parts":parts},"finishReason":"STOP"}],
        "usageMetadata": {"promptTokenCount":4,"candidatesTokenCount":2,"totalTokenCount":6}
    })])
}

fn native(url: &str, model: &str, wire: OpenAiWireApi, provider: &str) -> OpenAiCompatibleBackend {
    OpenAiCompatibleBackend::new_with_wire_api(url, Some("fixture-key".into()), model, wire)
        .unwrap()
        .with_model_registry(provider.into(), ModelRegistry::default())
        .unwrap()
}

fn switch(agent: &mut Agent, model: &str, backend: Box<dyn Backend>) {
    let owner = agent.owner_label().to_owned();
    let outcome = agent
        .select_connection(ConnectionSelection {
            owner_id: owner,
            profile: "switched".into(),
            model: Some(model.into()),
            expected_selection_revision: None,
            backend,
        })
        .unwrap();
    assert!(
        matches!(outcome, ConnectionSelectionOutcome::Applied { .. }),
        "{outcome:?}"
    );
}

#[test]
fn anthropic_thinking_session_continues_on_openai_and_replays_identically_on_claude() {
    let dir = tempdir().unwrap();
    let claude = SseServer::start(vec![
        claude_reply("msg-1", Some(("I should greet.", "sig-1")), "Hello!"),
        claude_reply("msg-2", None, "Again."),
    ]);
    let mut agent = Agent::new(
        SessionStore::open(dir.path().join("s.jsonl")).unwrap(),
        Box::new(native(
            &claude.url,
            "claude-sonnet-4-6",
            OpenAiWireApi::AnthropicMessages,
            "anthropic",
        )),
    );
    let first = agent.process(TurnInputRequest::new("hi")).unwrap();
    assert_eq!(first.assistant.unwrap().content, "Hello!");

    // Same model: the signed blocks are replayed exactly as recorded.
    agent.process(TurnInputRequest::new("again")).unwrap();
    let stored = agent
        .history()
        .iter()
        .find(|turn| turn.role == TurnRole::Assistant)
        .and_then(|turn| turn.metadata.clone())
        .unwrap()["annotations"][0]["blocks"]
        .clone();
    assert_eq!(
        stored,
        json!([{"type":"thinking","thinking":"I should greet.","signature":"sig-1"},{"type":"text","text":"Hello!"}])
    );
    let replayed = &claude.bodies()[1]["messages"][1];
    assert_eq!(replayed["role"], "assistant");
    assert_eq!(
        serde_json::to_vec(&replayed["content"]).unwrap(),
        serde_json::to_vec(&stored).unwrap()
    );

    // `/model` to an OpenAI model: accepted, and the next request succeeds.
    let openai = FakeServer::start(OpenAiWireApi::Responses, [text("from openai")]);
    switch(
        &mut agent,
        "gpt-4.1",
        Box::new(native(
            &format!("{}/v1", openai.url),
            "gpt-4.1",
            OpenAiWireApi::Responses,
            "openai",
        )),
    );
    let outcome = agent.process(TurnInputRequest::new("now openai")).unwrap();
    assert_eq!(outcome.assistant.unwrap().content, "from openai");
    let body = openai.bodies()[0].clone();
    let wire = body.to_string();
    assert!(!wire.contains("sig-1"), "{wire}");
    assert!(!wire.contains("native_history"), "{wire}");
    let assistant_texts: Vec<_> = body["input"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["role"] == "assistant")
        .map(|item| item["content"].to_string())
        .collect();
    assert!(
        assistant_texts[0].contains("I should greet.\\n\\nHello!"),
        "{assistant_texts:?}"
    );
}

#[test]
fn gemini_thought_and_tool_session_continues_on_claude() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("note.txt"), "gemini note").unwrap();
    let gemini = SseServer::start(vec![
        gemini_reply(
            "g-1",
            json!([
                {"text":"Let me read it.","thought":true},
                {"functionCall":{"name":"read_file","args":{"path":"note.txt"}},"thoughtSignature":"Z2VtaW5p"}
            ]),
        ),
        gemini_reply(
            "g-2",
            json!([{"text":"It says hi.","thought":true},{"text":"The note says gemini note."}]),
        ),
    ]);
    let mut agent = Agent::new(
        SessionStore::open(dir.path().join("s.jsonl")).unwrap(),
        Box::new(native(
            &gemini.url,
            "gemini-2.5-flash",
            OpenAiWireApi::GoogleGenerativeAi,
            "google",
        )),
    );
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(dir.path()).unwrap(),
        SideEffectPolicy::read_only(),
    );
    let first = agent
        .process(TurnInputRequest::new("read note.txt"))
        .unwrap();
    assert_eq!(
        first.assistant.unwrap().content,
        "The note says gemini note."
    );

    let claude = SseServer::start(vec![claude_reply("msg-c", None, "Claude here.")]);
    switch(
        &mut agent,
        "claude-sonnet-4-6",
        Box::new(native(
            &claude.url,
            "claude-sonnet-4-6",
            OpenAiWireApi::AnthropicMessages,
            "anthropic",
        )),
    );
    let outcome = agent.process(TurnInputRequest::new("and now?")).unwrap();
    assert_eq!(outcome.assistant.unwrap().content, "Claude here.");
    let body = claude.bodies()[0].clone();
    let wire = body.to_string();
    assert!(
        !wire.contains("thoughtSignature") && !wire.contains("Z2VtaW5p"),
        "{wire}"
    );
    let messages = body["messages"].as_array().unwrap();
    // Gemini sent no call ID; the local correlation ID is valid for Claude.
    let id = messages[1]["content"][1]["id"].as_str().unwrap().to_owned();
    assert!(id.starts_with("gemini_"), "{id}");
    assert_eq!(
        messages[1],
        json!({"role":"assistant","content":[
            {"type":"text","text":"Let me read it."},
            {"type":"tool_use","id":id,"name":"read_file","input":{"path":"note.txt"}}
        ]})
    );
    assert_eq!(messages[2]["content"][0]["tool_use_id"], json!(id));
    assert_eq!(
        messages[3],
        json!({"role":"assistant","content":[{"type":"text","text":"It says hi.\n\nThe note says gemini note."}]})
    );
}

#[test]
fn chat_tool_ids_are_normalized_for_claude() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("note.txt"), "chat note").unwrap();
    // Kimi-style Chat call IDs carry `.` and `:`, which Claude rejects.
    let chat = FakeServer::start(
        OpenAiWireApi::ChatCompletions,
        [
            call(
                "functions.read_file:0",
                "read_file",
                json!({"path":"note.txt"}),
            ),
            text("read it"),
        ],
    );
    let mut agent = Agent::new(
        SessionStore::open(dir.path().join("s.jsonl")).unwrap(),
        Box::new(native(
            &format!("{}/v1", chat.url),
            "gpt-4.1",
            OpenAiWireApi::ChatCompletions,
            "openai",
        )),
    );
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(dir.path()).unwrap(),
        SideEffectPolicy::read_only(),
    );
    agent
        .process(TurnInputRequest::new("read note.txt"))
        .unwrap();
    // Same wire: the recorded ID is replayed unchanged.
    assert_eq!(
        chat.bodies()[1]["messages"][2]["tool_calls"][0]["id"],
        "functions.read_file:0"
    );
    let claude = SseServer::start(vec![claude_reply("msg-k", None, "ok")]);
    switch(
        &mut agent,
        "claude-sonnet-4-6",
        Box::new(native(
            &claude.url,
            "claude-sonnet-4-6",
            OpenAiWireApi::AnthropicMessages,
            "anthropic",
        )),
    );
    agent.process(TurnInputRequest::new("next")).unwrap();
    let body = claude.bodies()[0].clone();
    assert_eq!(
        body["messages"][1]["content"][0]["id"],
        "functions_read_file_0"
    );
    assert_eq!(
        body["messages"][2]["content"][0]["tool_use_id"],
        "functions_read_file_0"
    );
}
