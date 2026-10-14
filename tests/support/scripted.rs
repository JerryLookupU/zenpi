//! One provider script, two transports: an in-process [`ScriptedBackend`] and a
//! local HTTP/SSE [`FakeServer`] speaking Responses, Chat and Anthropic wires.
//! Both record every request in the same normalized [`Recorded`] form so a test
//! can prove the core sends identical conversations through either path.
#![allow(dead_code)]

use serde_json::{Value, json};
use std::{
    io::{ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use zenpi::{
    backend::{Backend, BackendError, Completion, CompletionRequest, OpenAiWireApi, ProviderEvent},
    core::TurnRole,
    tools::ToolCall,
};

/// One provider turn. The last reply repeats once the script is exhausted.
#[derive(Clone, Debug)]
pub enum Reply {
    Text(String),
    Call(ToolCall),
    /// Streamed text followed by a tool call in the same response.
    TextThenCall(String, ToolCall),
    /// Transport failure in-process; HTTP 400 on the wire.
    Fail(String),
    /// Block until the caller cancels (in-process only).
    Stall,
}

pub fn text(value: &str) -> Reply {
    Reply::Text(value.into())
}

pub fn call(id: &str, name: &str, arguments: Value) -> Reply {
    Reply::Call(ToolCall {
        id: id.into(),
        name: name.into(),
        arguments,
    })
}

/// Transport-neutral view of one request: visible user/assistant text,
/// advertised tool names, and how many tool results were replayed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recorded {
    pub messages: Vec<String>,
    pub tools: Vec<String>,
    pub tool_results: usize,
}

#[derive(Default)]
struct Script {
    replies: Vec<Reply>,
    cursor: AtomicUsize,
    requests: Mutex<Vec<Recorded>>,
    bodies: Mutex<Vec<Value>>,
    stalled: AtomicBool,
}

impl Script {
    fn next(&self, recorded: Recorded) -> Reply {
        self.requests.lock().unwrap().push(recorded);
        let index = self.cursor.fetch_add(1, Ordering::SeqCst);
        let reply = self.replies.get(index).or(self.replies.last());
        reply.cloned().expect("script has no replies")
    }
}

/// Cloning shares the script, so a clone kept by the test observes requests
/// made through the boxed copy owned by the agent.
#[derive(Clone)]
pub struct ScriptedBackend(Arc<Script>);

impl ScriptedBackend {
    pub fn new(replies: impl IntoIterator<Item = Reply>) -> Self {
        Self(Arc::new(Script {
            replies: replies.into_iter().collect(),
            ..Script::default()
        }))
    }
    pub fn requests(&self) -> Vec<Recorded> {
        self.0.requests.lock().unwrap().clone()
    }
    pub fn is_stalled(&self) -> bool {
        self.0.stalled.load(Ordering::SeqCst)
    }
}

impl Backend for ScriptedBackend {
    super::controlled_local_backend!();

    fn complete(&self, request: CompletionRequest<'_>) -> Result<Completion, BackendError> {
        self.complete_with_control(request, &|| false, &mut |_| Ok(()))
    }

    fn complete_with_control(
        &self,
        request: CompletionRequest<'_>,
        cancelled: &dyn Fn() -> bool,
        sink: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
    ) -> Result<Completion, BackendError> {
        if cancelled() {
            return Err(BackendError::Cancelled);
        }
        let mut recorded = Recorded::default();
        for turn in request.turns {
            let role = match turn.role {
                TurnRole::User => "user",
                TurnRole::Assistant => "assistant",
                TurnRole::Tool => {
                    recorded.tool_results += 1;
                    continue;
                }
                TurnRole::System => continue,
            };
            if !turn.content.is_empty() {
                recorded.messages.push(format!("{role}: {}", turn.content));
            }
        }
        recorded.tools = request.tools.iter().map(|t| t.name.clone()).collect();
        let completion = match self.0.next(recorded) {
            Reply::Text(text) => {
                sink(ProviderEvent::TextDelta {
                    delta: text.clone(),
                })?;
                Completion::text(text)
            }
            Reply::Call(call) => {
                sink(ProviderEvent::ToolCallDone { call: call.clone() })?;
                Completion {
                    tool_calls: vec![call],
                    ..Completion::default()
                }
            }
            Reply::TextThenCall(text, call) => {
                sink(ProviderEvent::TextDelta {
                    delta: text.clone(),
                })?;
                sink(ProviderEvent::ToolCallDone { call: call.clone() })?;
                Completion {
                    content: text,
                    tool_calls: vec![call],
                    ..Completion::default()
                }
            }
            Reply::Fail(message) => return Err(BackendError::Transport(message)),
            Reply::Stall => {
                self.0.stalled.store(true, Ordering::SeqCst);
                let deadline = Instant::now() + Duration::from_secs(10);
                while !cancelled() {
                    assert!(Instant::now() < deadline, "stalled script never cancelled");
                    thread::sleep(Duration::from_millis(1));
                }
                return Err(BackendError::Cancelled);
            }
        };
        if cancelled() {
            return Err(BackendError::Cancelled);
        }
        sink(ProviderEvent::Completed {
            response_id: None,
            model: None,
        })?;
        Ok(completion)
    }
}

/// Local provider endpoint serving the same script as SSE on one wire.
pub struct FakeServer {
    pub url: String,
    script: Arc<Script>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl FakeServer {
    pub fn start(wire: OpenAiWireApi, replies: impl IntoIterator<Item = Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let script = ScriptedBackend::new(replies).0;
        let stop = Arc::new(AtomicBool::new(false));
        let (shared, halt) = (script.clone(), stop.clone());
        let thread = thread::spawn(move || {
            while !halt.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => serve(stream, wire, &shared),
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(e) => panic!("fake server accept: {e}"),
                }
            }
        });
        Self {
            url,
            script,
            stop,
            thread: Some(thread),
        }
    }
    pub fn requests(&self) -> Vec<Recorded> {
        self.script.requests.lock().unwrap().clone()
    }
    /// Raw request bodies, for wire-level assertions [`Recorded`] hides.
    pub fn bodies(&self) -> Vec<Value> {
        self.script.bodies.lock().unwrap().clone()
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.take()
            && handle.join().is_err()
            && !thread::panicking()
        {
            panic!("fake server thread panicked");
        }
    }
}

fn serve(mut stream: TcpStream, wire: OpenAiWireApi, script: &Script) {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
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
    let body: Value = serde_json::from_slice(&bytes[end..end + length]).unwrap();
    script.bodies.lock().unwrap().push(body.clone());
    let (status, mime, payload) = match script.next(normalize(&body)) {
        Reply::Fail(message) => (
            "400 Bad Request",
            "application/json",
            json!({"error":{"message":message}}).to_string(),
        ),
        Reply::Stall => panic!("Reply::Stall is in-process only"),
        reply => {
            let model = body["model"].as_str().unwrap_or("scripted-model");
            ("200 OK", "text/event-stream", sse(wire, model, &reply))
        }
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    );
}

/// Inverse of the three request encoders, reduced to [`Recorded`].
pub fn normalize(body: &Value) -> Recorded {
    let mut recorded = Recorded::default();
    let none = Vec::new();
    for tool in body["tools"].as_array().unwrap_or(&none) {
        let name = tool["function"]["name"].as_str().or(tool["name"].as_str());
        recorded.tools.extend(name.map(str::to_owned));
    }
    let items = body["messages"].as_array().or(body["input"].as_array());
    for item in items.unwrap_or(&none) {
        if item["type"] == "function_call_output" || item["role"] == "tool" {
            recorded.tool_results += 1;
            continue;
        }
        let Some(role @ ("user" | "assistant")) = item["role"].as_str() else {
            continue;
        };
        let text = match &item["content"] {
            Value::String(text) => text.clone(),
            Value::Array(parts) => parts
                .iter()
                .filter_map(|part| {
                    recorded.tool_results += usize::from(part["type"] == "tool_result");
                    part["text"].as_str()
                })
                .collect(),
            _ => String::new(),
        };
        if !text.is_empty() {
            recorded.messages.push(format!("{role}: {text}"));
        }
    }
    recorded
}

fn frames(events: &[Value], named: bool) -> String {
    events
        .iter()
        .map(|event| match event["type"].as_str().filter(|_| named) {
            Some(name) => format!("event: {name}\ndata: {event}\n\n"),
            None => format!("data: {event}\n\n"),
        })
        .collect()
}

/// Encode one scripted reply as a complete SSE body for `wire`, served as `model`.
pub fn sse(wire: OpenAiWireApi, model: &str, reply: &Reply) -> String {
    let (text, call) = match reply {
        Reply::Text(text) => (Some(text.as_str()), None),
        Reply::Call(call) => (None, Some(call)),
        _ => unreachable!("only text and calls are streamed"),
    };
    let args = call.map(|c| c.arguments.to_string());
    match wire {
        OpenAiWireApi::ChatCompletions => {
            let chunk = |delta: Value, finish: Value| json!({"id":"chat-1","model":model,"choices":[{"index":0,"delta":delta,"finish_reason":finish}]});
            let delta = match call {
                Some(c) => {
                    json!({"tool_calls":[{"index":0,"id":c.id,"type":"function","function":{"name":c.name,"arguments":args}}]})
                }
                None => json!({"content":text}),
            };
            let finish = if call.is_some() { "tool_calls" } else { "stop" };
            let usage = json!({"choices":[],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}});
            frames(
                &[
                    chunk(delta, Value::Null),
                    chunk(json!({}), json!(finish)),
                    usage,
                ],
                false,
            ) + "data: [DONE]\n\n"
        }
        OpenAiWireApi::Responses => {
            let mut events =
                vec![json!({"type":"response.created","response":{"id":"resp-1","model":model}})];
            match call {
                Some(c) => events.push(json!({"type":"response.output_item.done","item":{"type":"function_call","status":"completed","call_id":c.id,"name":c.name,"arguments":args}})),
                None => events.push(json!({"type":"response.output_text.delta","delta":text})),
            }
            events.push(json!({"type":"response.completed","response":{"id":"resp-1","model":model,"status":"completed","usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}}));
            frames(&events, true) + "data: [DONE]\n\n"
        }
        OpenAiWireApi::AnthropicMessages => {
            let block = match call {
                Some(c) => json!({"type":"tool_use","id":c.id,"name":c.name,"input":c.arguments}),
                None => json!({"type":"text","text":text}),
            };
            let stop = if call.is_some() {
                "tool_use"
            } else {
                "end_turn"
            };
            frames(
                &[
                    json!({"type":"message_start","message":{"type":"message","id":"msg-1","role":"assistant","model":model,"content":[],"stop_reason":null,"usage":{"input_tokens":1,"output_tokens":0}}}),
                    json!({"type":"content_block_start","index":0,"content_block":block}),
                    json!({"type":"content_block_stop","index":0}),
                    json!({"type":"message_delta","delta":{"stop_reason":stop},"usage":{"output_tokens":1}}),
                    json!({"type":"message_stop"}),
                ],
                true,
            )
        }
        OpenAiWireApi::GoogleGenerativeAi => unimplemented!("google wire is not scripted"),
    }
}
