//! ZE-210: after successful tool calls an empty final answer ends the turn
//! normally on every wire, and the next turn replays that history.
mod support;

use serde_json::{Value, json};
use std::path::Path;
use support::scripted::{FakeServer, call, text};
use tempfile::tempdir;
use zenpi::{
    backend::{BackendError, OpenAiCompatibleBackend, OpenAiWireApi},
    core::{Agent, AgentError, TurnInputRequest, TurnRole},
    session::SessionStore,
    tools::{SideEffectPolicy, ToolContext, ToolRegistry},
};

const WIRES: [OpenAiWireApi; 3] = [
    OpenAiWireApi::Responses,
    OpenAiWireApi::ChatCompletions,
    OpenAiWireApi::AnthropicMessages,
];

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
    agent
}

/// Every assistant message in a request body, per wire convention.
fn assistant_messages(body: &Value) -> Vec<Value> {
    let items = body["messages"].as_array().or(body["input"].as_array());
    items
        .into_iter()
        .flatten()
        .filter(|item| item["role"] == "assistant")
        .cloned()
        .collect()
}

#[test]
fn empty_final_text_after_tools_completes_turn() {
    for wire in WIRES {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("note.txt"), "hello\n").unwrap();
        let server = FakeServer::start(
            wire,
            [
                call("call-1", "read_file", json!({"path": "note.txt"})),
                text(""),
                text("second answer"),
            ],
        );
        let mut agent = agent(dir.path(), &server.url, wire);

        let first = agent
            .process(TurnInputRequest::new("read it"))
            .unwrap_or_else(|error| panic!("{wire:?}: {error}"));
        let assistant = first.assistant.expect("assistant turn persisted");
        assert_eq!(assistant.content, "", "{wire:?}");
        assert_eq!(assistant.role, TurnRole::Assistant, "{wire:?}");

        let second = agent
            .process(TurnInputRequest::new("and now?"))
            .unwrap_or_else(|error| panic!("{wire:?} replay: {error}"));
        assert_eq!(
            second.assistant.unwrap().content,
            "second answer",
            "{wire:?}"
        );

        let requests = server.requests();
        assert_eq!(requests.len(), 3, "{wire:?}");
        assert_eq!(
            requests[2].messages,
            vec!["user: read it", "user: and now?"],
            "{wire:?}"
        );
        assert_eq!(requests[2].tool_results, 1, "{wire:?}");
        // The empty answer is omitted from replay, never sent as empty content.
        for message in assistant_messages(&server.bodies()[2]) {
            let empty = match &message["content"] {
                Value::String(text) => text.is_empty(),
                Value::Array(parts) => parts.is_empty(),
                _ => false,
            };
            assert!(
                !empty || message.get("tool_calls").is_some(),
                "{wire:?} replayed an empty assistant message: {message}"
            );
        }

        // The journal keeps the empty final answer, and it reopens cleanly.
        let store = SessionStore::open(dir.path().join("session.jsonl")).unwrap();
        assert!(
            store
                .turns()
                .iter()
                .any(|turn| turn.id == assistant.id && turn.content.is_empty()),
            "{wire:?}"
        );
    }
}

#[test]
fn empty_answer_without_tools_still_fails() {
    for wire in WIRES {
        let dir = tempdir().unwrap();
        let server = FakeServer::start(wire, [text("")]);
        let mut agent = agent(dir.path(), &server.url, wire);
        let error = agent.process(TurnInputRequest::new("hi")).unwrap_err();
        assert!(
            matches!(error, AgentError::Backend(BackendError::EmptyResponse)),
            "{wire:?}: {error}"
        );
    }
}
