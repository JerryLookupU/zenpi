//! ZE-221: a turn the output token limit cut short keeps its text, and the
//! tool calls it carries are never dispatched. Each call is answered with an
//! "output token limit reached" error result and the loop continues.
mod support;

use serde_json::json;
use std::path::Path;
use support::scripted::{FakeServer, ScriptedBackend, text, truncated_call};
use tempfile::tempdir;
use zenpi::{
    backend::{Backend, OpenAiCompatibleBackend, OpenAiWireApi},
    core::{Agent, TurnInputRequest, TurnRole},
    session::SessionStore,
    tools::{SideEffectPolicy, ToolContext, ToolRegistry},
};

fn agent(root: &Path, backend: Box<dyn Backend>) -> Agent {
    let mut agent = Agent::new(
        SessionStore::open(root.join("session.jsonl")).unwrap(),
        backend,
    );
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(root).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    agent
}

fn assert_not_dispatched(root: &Path, label: &str) {
    assert!(!root.join("NEVER.txt").exists(), "{label}: call ran");
    let store = SessionStore::open(root.join("session.jsonl")).unwrap();
    let results: Vec<_> = store
        .turns()
        .iter()
        .filter(|turn| turn.role == TurnRole::Tool)
        .collect();
    assert_eq!(results.len(), 1, "{label}");
    assert!(
        results[0].content.contains("output token limit reached"),
        "{label}: {}",
        results[0].content
    );
}

fn script() -> [support::scripted::Reply; 2] {
    [
        truncated_call(
            "cut-1",
            "write_file",
            json!({"path": "NEVER.txt", "content": "must not run"}),
        ),
        text("retried with a shorter answer"),
    ]
}

#[test]
fn length_truncated_tool_calls_not_dispatched() {
    // In-process backend.
    let dir = tempdir().unwrap();
    let backend = ScriptedBackend::new(script());
    let mut scripted = agent(dir.path(), Box::new(backend.clone()));
    let outcome = scripted.process(TurnInputRequest::new("write it")).unwrap();
    assert_eq!(
        outcome.assistant.unwrap().content,
        "retried with a shorter answer"
    );
    let requests = backend.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].tool_results, 1);
    assert_not_dispatched(dir.path(), "scripted");

    // The same script over each real wire decoder.
    for wire in [
        OpenAiWireApi::ChatCompletions,
        OpenAiWireApi::Responses,
        OpenAiWireApi::AnthropicMessages,
    ] {
        let dir = tempdir().unwrap();
        let server = FakeServer::start(wire, script());
        let backend = OpenAiCompatibleBackend::new_with_wire_api(
            server.url.clone(),
            Some("scripted-key".into()),
            "claude-sonnet-4-6",
            wire,
        )
        .unwrap();
        let mut agent = agent(dir.path(), Box::new(backend));
        let outcome = agent
            .process(TurnInputRequest::new("write it"))
            .unwrap_or_else(|error| panic!("{wire:?}: {error}"));
        assert_eq!(
            outcome.assistant.unwrap().content,
            "retried with a shorter answer",
            "{wire:?}"
        );
        let requests = server.requests();
        assert_eq!(requests.len(), 2, "{wire:?}");
        assert_eq!(requests[1].tool_results, 1, "{wire:?}");
        assert_not_dispatched(dir.path(), &format!("{wire:?}"));
    }
}
