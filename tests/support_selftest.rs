mod support;

use serde_json::json;
use std::path::Path;
use support::scripted::{FakeServer, Recorded, Reply, ScriptedBackend, call, text};
use tempfile::tempdir;
use zenpi::{
    backend::{Backend, BackendError, OpenAiCompatibleBackend, OpenAiWireApi},
    core::{Agent, AgentError, TurnInputRequest},
    session::SessionStore,
    tools::{SideEffectPolicy, ToolContext, ToolRegistry},
};

fn script() -> Vec<Reply> {
    vec![
        call("call-1", "read_file", json!({"path": "note.txt"})),
        text("note was read"),
        text("second answer"),
    ]
}

/// Drive two prompts (the first with a tool round-trip) and return the replies.
fn drive(root: &Path, backend: Box<dyn Backend>) -> Vec<String> {
    std::fs::write(root.join("note.txt"), "hello").unwrap();
    let store = SessionStore::open(root.join("session.jsonl")).unwrap();
    let mut agent = Agent::new(store, backend);
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(root).unwrap(),
        SideEffectPolicy::read_only(),
    );
    ["first prompt", "second prompt"]
        .into_iter()
        .map(|prompt| {
            let result = agent.process(TurnInputRequest::new(prompt)).unwrap();
            result.assistant.unwrap().content
        })
        .collect()
}

fn in_process() -> Vec<Recorded> {
    let dir = tempdir().unwrap();
    let backend = ScriptedBackend::new(script());
    let replies = drive(dir.path(), Box::new(backend.clone()));
    assert_eq!(replies, ["note was read", "second answer"]);
    backend.requests()
}

#[test]
fn same_script_yields_same_requests_in_process_and_over_every_wire() {
    let expected = in_process();
    assert_eq!(expected.len(), 3);
    assert_eq!(expected[0].messages, ["user: first prompt"]);
    assert!(expected[0].tools.iter().any(|name| name == "read_file"));
    assert_eq!(expected[1].tool_results, 1);
    assert_eq!(
        expected[2].messages,
        [
            "user: first prompt",
            "assistant: note was read",
            "user: second prompt"
        ]
    );
    for wire in [
        OpenAiWireApi::Responses,
        OpenAiWireApi::ChatCompletions,
        OpenAiWireApi::AnthropicMessages,
    ] {
        let server = FakeServer::start(wire, script());
        let backend = OpenAiCompatibleBackend::new_with_wire_api(
            server.url.clone(),
            Some("scripted-key".into()),
            "claude-sonnet-4-6",
            wire,
        )
        .unwrap();
        let dir = tempdir().unwrap();
        let replies = drive(dir.path(), Box::new(backend));
        assert_eq!(replies, ["note was read", "second answer"], "{wire:?}");
        assert_eq!(server.requests(), expected, "{wire:?}");
    }
}

#[test]
fn stall_holds_the_turn_until_cancelled() {
    let dir = tempdir().unwrap();
    let backend = ScriptedBackend::new([text("one"), Reply::Stall]);
    let store = SessionStore::open(dir.path().join("s.jsonl")).unwrap();
    let mut agent = Agent::new(store, Box::new(backend.clone()));
    agent.process(TurnInputRequest::new("go")).unwrap();
    let probe = backend.clone();
    let error = agent
        .process_with_cancel(TurnInputRequest::new("stall"), move || probe.is_stalled())
        .unwrap_err();
    assert!(matches!(
        error,
        AgentError::Backend(BackendError::Cancelled)
    ));
    assert!(backend.is_stalled());
    assert_eq!(backend.requests().len(), 2);
}
