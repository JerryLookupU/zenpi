//! ZE-213: the model receives plain tool-result text (pi `content`), never the
//! serialized journal record; the structured record stays in the journal.
mod support;

use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use support::scripted::{Reply, call, sse, text};
use tempfile::tempdir;
use zenpi::{
    backend::{OpenAiCompatibleBackend, OpenAiWireApi},
    core::{Agent, TurnInputRequest, TurnRole},
    session::SessionStore,
    tools::{SideEffectPolicy, ToolCall, ToolContext, ToolRegistry, ToolResult, model_facing_text},
};

/// Tabs, quotes, backslashes and a non-ASCII byte: everything JSON escapes.
const SOURCE: &str =
    "fn main() {\n\tprintln!(\"say \\\"hi\\\"\\tnow\");\n\tlet path = 'C:\\\\tmp';\n}\n// é\n";

type Responder = Box<dyn FnMut(&Value) -> Reply + Send>;

/// Local provider that records every raw request body and answers each one
/// from a closure, so a reply can depend on what the model was sent.
struct CaptureServer {
    url: String,
    bodies: Arc<Mutex<Vec<String>>>,
}

impl CaptureServer {
    fn start(wire: OpenAiWireApi, mut responder: Responder) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let bodies = Arc::new(Mutex::new(Vec::new()));
        let recorded = bodies.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let raw = read_body(&mut stream);
                recorded.lock().unwrap().push(raw.clone());
                let body: Value = serde_json::from_str(&raw).unwrap();
                let reply = responder(&body);
                let model = body["model"].as_str().unwrap_or("scripted-model");
                let payload = sse(wire, model, &reply);
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                );
            }
        });
        Self { url, bodies }
    }

    fn bodies(&self) -> Vec<String> {
        self.bodies.lock().unwrap().clone()
    }
}

fn read_body(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let (mut bytes, mut chunk) = (Vec::new(), [0; 8192]);
    let end = loop {
        let n = stream.read(&mut chunk).unwrap();
        assert!(n > 0, "client closed before headers");
        bytes.extend_from_slice(&chunk[..n]);
        if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let length = String::from_utf8_lossy(&bytes[..end])
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .expect("request has content-length");
    while bytes.len() < end + length {
        let n = stream.read(&mut chunk).unwrap();
        assert!(n > 0, "client closed before body");
        bytes.extend_from_slice(&chunk[..n]);
    }
    String::from_utf8(bytes[end..end + length].to_vec()).unwrap()
}

/// The latest tool result in an encoded request body, per wire convention,
/// with the Anthropic `is_error` flag when the wire carries one.
fn tool_result(wire: OpenAiWireApi, body: &Value) -> Option<(Value, Option<bool>)> {
    let items = body["messages"].as_array().or(body["input"].as_array())?;
    items.iter().rev().find_map(|item| match wire {
        OpenAiWireApi::Responses => {
            (item["type"] == "function_call_output").then(|| (item["output"].clone(), None))
        }
        OpenAiWireApi::ChatCompletions => {
            (item["role"] == "tool").then(|| (item["content"].clone(), None))
        }
        _ => item["content"].as_array()?.iter().rev().find_map(|part| {
            (part["type"] == "tool_result")
                .then(|| (part["content"].clone(), part["is_error"].as_bool()))
        }),
    })
}

fn agent(root: &Path, url: &str, wire: OpenAiWireApi) -> Agent {
    let backend = OpenAiCompatibleBackend::new_with_wire_api(
        url.to_owned(),
        Some("scripted-key".into()),
        "claude-sonnet-4-6",
        wire,
    )
    .unwrap();
    let mut agent = Agent::new(
        SessionStore::open(root.join("session.jsonl")).unwrap(),
        Box::new(backend),
    );
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(root).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    let mut approval = zenpi::approval::ApprovalPolicy::default();
    approval.remember("edit_file", zenpi::approval::ApprovalDecision::Allow);
    agent.set_approval_policy(approval);
    agent
}

const WIRES: [OpenAiWireApi; 3] = [
    OpenAiWireApi::Responses,
    OpenAiWireApi::ChatCompletions,
    OpenAiWireApi::AnthropicMessages,
];

#[test]
fn read_file_result_is_raw_unescaped_text_in_the_request_body() {
    for wire in WIRES {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("quote.rs"), SOURCE).unwrap();
        let mut script = vec![
            call("call-1", "read_file", json!({"path": "quote.rs"})),
            text("read it"),
        ]
        .into_iter();
        let server = CaptureServer::start(wire, Box::new(move |_| script.next().unwrap()));
        let mut agent = agent(dir.path(), &server.url, wire);
        let result = agent.process(TurnInputRequest::new("read")).unwrap();
        assert_eq!(result.assistant.unwrap().content, "read it", "{wire:?}");

        let bodies = server.bodies();
        assert_eq!(bodies.len(), 2, "{wire:?}");
        let body: Value = serde_json::from_str(&bodies[1]).unwrap();
        let (content, is_error) = tool_result(wire, &body).expect("tool result sent");
        // Snapshot: the wire value is exactly the file text, not a JSON record.
        assert_eq!(content, json!(SOURCE), "{wire:?}");
        let content = content.as_str().unwrap();
        for key in ["call_id", "output_capture", "\"status\"", "truncated"] {
            assert!(!content.contains(key), "{wire:?} leaked {key}");
        }
        // Escaped exactly once by the wire encoder: no doubly escaped quotes/tabs.
        let once = serde_json::to_string(SOURCE).unwrap();
        let twice = serde_json::to_string(&once).unwrap();
        assert!(bodies[1].contains(&once), "{wire:?}");
        assert!(!bodies[1].contains(&twice[1..twice.len() - 1]), "{wire:?}");
        assert!(!bodies[1].contains("output_capture"), "{wire:?}");
        if wire == OpenAiWireApi::AnthropicMessages {
            assert_eq!(is_error, Some(false));
        }

        // The journal keeps the structured record and the model text beside it.
        let store = SessionStore::open(dir.path().join("session.jsonl")).unwrap();
        let tool = store
            .turns()
            .iter()
            .find(|turn| turn.role == TurnRole::Tool)
            .unwrap()
            .clone();
        let record: Value = serde_json::from_str(&tool.content).unwrap();
        assert_eq!(record["call_id"], "call-1");
        assert_eq!(record["output"]["content"], SOURCE);
        let metadata = tool.metadata.unwrap();
        assert_eq!(metadata["model_text"], SOURCE);
        assert_eq!(metadata["is_error"], false);
    }
}

#[test]
fn edit_with_old_text_copied_verbatim_from_read_result_succeeds() {
    for wire in WIRES {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("quote.rs"), SOURCE).unwrap();
        let mut step = 0;
        let responder: Responder = Box::new(move |body| {
            step += 1;
            match step {
                1 => call("read-1", "read_file", json!({"path": "quote.rs"})),
                2 => {
                    // Act like a model: copy the second line of what it saw.
                    let (seen, _) = tool_result(wire, body).unwrap();
                    let old = seen.as_str().unwrap().lines().nth(1).unwrap().to_owned();
                    call(
                        "edit-1",
                        "edit_file",
                        json!({"path": "quote.rs", "old": old, "new": "\tprintln!(\"done\");"}),
                    )
                }
                3 => {
                    let (seen, is_error) = tool_result(wire, body).unwrap();
                    assert_eq!(seen, json!("Edited quote.rs (1 replacement)"));
                    assert_ne!(is_error, Some(true));
                    text("edited")
                }
                _ => text("unexpected"),
            }
        });
        let server = CaptureServer::start(wire, responder);
        let mut agent = agent(dir.path(), &server.url, wire);
        let result = agent.process(TurnInputRequest::new("edit")).unwrap();
        assert_eq!(result.assistant.unwrap().content, "edited", "{wire:?}");
        assert_eq!(server.bodies().len(), 3, "{wire:?}");
        let edited = std::fs::read_to_string(dir.path().join("quote.rs")).unwrap();
        assert_eq!(
            edited,
            SOURCE.replace(
                "\tprintln!(\"say \\\"hi\\\"\\tnow\");",
                "\tprintln!(\"done\");"
            ),
            "{wire:?}"
        );
    }
}

#[test]
fn tool_errors_are_plain_messages_flagged_is_error() {
    let dir = tempdir().unwrap();
    let mut script = vec![
        call("call-1", "read_file", json!({"path": "missing.txt"})),
        text("ok"),
    ]
    .into_iter();
    let wire = OpenAiWireApi::AnthropicMessages;
    let server = CaptureServer::start(wire, Box::new(move |_| script.next().unwrap()));
    let mut agent = agent(dir.path(), &server.url, wire);
    agent.process(TurnInputRequest::new("read")).unwrap();
    let body: Value = serde_json::from_str(&server.bodies()[1]).unwrap();
    let (content, is_error) = tool_result(wire, &body).unwrap();
    assert_eq!(is_error, Some(true));
    let content = content.as_str().unwrap();
    assert!(content.contains("missing.txt"), "{content}");
    assert!(!content.contains("call_id") && !content.starts_with('{'));
}

fn run(registry: &ToolRegistry, context: &ToolContext, name: &str, arguments: Value) -> String {
    let result = registry.execute(
        context,
        SideEffectPolicy::all_builtins(),
        ToolCall {
            id: format!("{name}-1"),
            name: name.into(),
            arguments,
        },
    );
    model_facing_text(&result, None)
}

#[test]
fn builtin_tools_render_compact_text() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.txt"), "alpha\nneedle one\nbeta\n").unwrap();
    std::fs::write(root.join("b.txt"), "needle two\n").unwrap();
    let registry = ToolRegistry::with_all_builtins().unwrap();
    let context = ToolContext::new(root).unwrap();

    assert_eq!(
        run(&registry, &context, "list_directory", json!({})),
        "b.txt\nsrc/"
    );
    assert_eq!(
        run(
            &registry,
            &context,
            "search_text",
            json!({"query": "needle"})
        ),
        "b.txt:1: needle two\nsrc/a.txt:2: needle one"
    );
    assert_eq!(
        run(
            &registry,
            &context,
            "search_text",
            json!({"query": "absent"})
        ),
        "No matches found"
    );
    let found = run(&registry, &context, "find", json!({"pattern": "*.txt"}));
    let mut found: Vec<&str> = found.lines().collect();
    found.sort_unstable();
    assert_eq!(found, ["b.txt", "src/a.txt"]);
    assert_eq!(
        run(
            &registry,
            &context,
            "read_file",
            json!({"path": "src/a.txt", "max_bytes": 5})
        ),
        "alpha\n\n[Truncated at 5 bytes. Use max_bytes (up to 524288) to read more]"
    );
    assert_eq!(
        run(
            &registry,
            &context,
            "write_file",
            json!({"path": "new.txt", "content": "x\n"})
        ),
        "Created new.txt (2 bytes)"
    );
    assert_eq!(
        run(
            &registry,
            &context,
            "edit_file",
            json!({"path": "new.txt", "old": "x", "new": "y"})
        ),
        "Edited new.txt (1 replacement)"
    );
    #[cfg(unix)]
    assert_eq!(
        run(
            &registry,
            &context,
            "run_command",
            json!({"command": "echo out; echo err >&2"})
        ),
        "out\nerr\n\nCommand exited with code 0"
    );
}

#[test]
fn errors_and_unknown_tools_keep_their_text() {
    let error = ToolResult::Error {
        call_id: "c".into(),
        tool: "read_file".into(),
        error: zenpi::tools::ToolFailure {
            code: zenpi::tools::ToolErrorCode::NotFound,
            message: "path not found: x".into(),
        },
    };
    assert_eq!(model_facing_text(&error, None), "path not found: x");
    let custom = ToolResult::Success {
        call_id: "c".into(),
        tool: "extension_tool".into(),
        output: json!({"answer": 42}),
        content: None,
    };
    assert_eq!(
        serde_json::from_str::<Value>(&model_facing_text(&custom, None)).unwrap(),
        serde_json::to_value(&custom).unwrap()
    );
}
