//! ZE-225: a session's requests carry the provider's prompt-cache hints.
mod support;

use serde_json::json;
use support::scripted::{FakeServer, text};
use tempfile::tempdir;
use zenpi::{
    backend::{OpenAiCompatibleBackend, OpenAiWireApi},
    core::{Agent, TurnInputRequest},
    session::SessionStore,
};

fn run(wire: OpenAiWireApi, model: &str) -> (serde_json::Value, String) {
    let dir = tempdir().unwrap();
    let server = FakeServer::start(wire, [text("first"), text("second")]);
    let backend = OpenAiCompatibleBackend::new_with_wire_api(
        server.url.clone(),
        Some("scripted-key".into()),
        model,
        wire,
    )
    .unwrap();
    let mut agent = Agent::new(
        SessionStore::open(dir.path().join("session.jsonl")).unwrap(),
        Box::new(backend),
    );
    agent.process(TurnInputRequest::new("one")).unwrap();
    agent.process(TurnInputRequest::new("two")).unwrap();
    let bodies = server.bodies();
    assert_eq!(bodies.len(), 2, "{wire:?}");
    (bodies[1].clone(), agent.session().session_id().to_owned())
}

#[test]
fn responses_prompt_cache_key_is_the_session_id() {
    let (body, session) = run(OpenAiWireApi::Responses, "gpt-4.1");
    assert!(!session.is_empty());
    assert_eq!(body["prompt_cache_key"], json!(session));
    // Chat Completions has no session cache key.
    let (body, _) = run(OpenAiWireApi::ChatCompletions, "gpt-4.1");
    assert!(body.get("prompt_cache_key").is_none());
}

#[test]
fn anthropic_marks_the_newest_user_block_as_a_cache_breakpoint() {
    let (body, _) = run(OpenAiWireApi::AnthropicMessages, "claude-sonnet-4-6");
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages.last().unwrap()["content"],
        json!([{"type":"text","text":"two","cache_control":{"type":"ephemeral"}}])
    );
    // Earlier blocks carry no marker, so the breakpoint count stays bounded.
    let marked = messages
        .iter()
        .flat_map(|message| message["content"].as_array().unwrap())
        .filter(|block| block.get("cache_control").is_some())
        .count();
    assert_eq!(marked, 1);
    assert!(body.get("prompt_cache_key").is_none());
}
