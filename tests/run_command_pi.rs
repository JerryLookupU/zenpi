//! ZE-201 / ZE-202: `run_command` follows pi semantics and command failures
//! (including timeouts) come back to the model as ordinary error results.

mod support;

use serde_json::{Map, Value, json};
use support::scripted::{ScriptedBackend, call, text};
use tempfile::tempdir;
use zenpi::{
    core::{Agent, TurnInputRequest, TurnRole},
    session::SessionStore,
    tools::{RunCommandTool, SideEffectPolicy, ToolContext, ToolError, ToolRegistry},
};

fn arguments(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

#[test]
fn nonzero_exit_is_an_error_that_keeps_stdout_and_the_exit_code() {
    let workspace = tempdir().unwrap();
    let context = ToolContext::new(workspace.path()).unwrap();
    let error = RunCommandTool::invoke_with_cancel(
        &context,
        &arguments(json!({"command": "echo out; echo err >&2; exit 3"})),
        &|| false,
    )
    .unwrap_err();
    let ToolError::CommandFailed(message) = error else {
        panic!("expected CommandFailed, got {error:?}");
    };
    assert!(message.contains("out"), "{message}");
    assert!(message.contains("err"), "{message}");
    assert!(message.contains("Command exited with code 3"), "{message}");
}

#[test]
fn long_output_keeps_the_tail() {
    let workspace = tempdir().unwrap();
    let context = ToolContext::new(workspace.path()).unwrap();
    let output = RunCommandTool::invoke_with_cancel(
        &context,
        &arguments(json!({"command": "seq 100000"})),
        &|| false,
    )
    .unwrap();
    let stdout = output["stdout"].as_str().unwrap();
    assert_eq!(output["stdout_truncated"], true);
    assert!(stdout.trim_end().ends_with("100000"), "tail must be kept");
    assert!(!stdout.starts_with("1\n2\n"), "head must be dropped");
}

#[test]
fn timeout_is_optional_and_explicit_timeouts_still_apply() {
    let workspace = tempdir().unwrap();
    let context = ToolContext::new(workspace.path()).unwrap();
    // No timeout_ms: runs to completion even past the old 30 s default would
    // be impractical to test, so assert the absence of a default deadline.
    let output = RunCommandTool::invoke_with_cancel(
        &context,
        &arguments(json!({"command": "sleep 0.2; echo done"})),
        &|| false,
    )
    .unwrap();
    assert_eq!(output["timeout_ms"], 0, "no default deadline");
    let error = RunCommandTool::invoke_with_cancel(
        &context,
        &arguments(json!({"command": "sleep 5", "timeout_ms": 100})),
        &|| false,
    )
    .unwrap_err();
    assert!(matches!(error, ToolError::CommandTimeout(100)), "{error:?}");
}

#[test]
fn command_timeout_returns_to_the_model_and_the_turn_continues() {
    let workspace = tempdir().unwrap();
    let session = SessionStore::open(workspace.path().join("timeout.jsonl")).unwrap();
    let backend = ScriptedBackend::new([
        call(
            "call-1",
            "run_command",
            json!({"command": "sleep 5", "timeout_ms": 100}),
        ),
        call("call-2", "run_command", json!({"command": "echo fixed"})),
        text("recovered after the timeout"),
    ]);
    let mut agent = Agent::new(session, Box::new(backend.clone()));
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(workspace.path()).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    agent.enable_auto_approve();

    let result = agent
        .process(TurnInputRequest::new("run the slow command"))
        .unwrap();
    assert_eq!(
        result.assistant.unwrap().content,
        "recovered after the timeout"
    );
    assert_eq!(agent.automation_vetoed(), None);
    let tool_turns: Vec<_> = agent
        .history()
        .iter()
        .filter(|turn| turn.role == TurnRole::Tool)
        .collect();
    assert_eq!(tool_turns.len(), 2);
    assert!(
        tool_turns[0].content.contains("timed out"),
        "the model must see the timeout: {}",
        tool_turns[0].content
    );
    let requests = backend.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1].tool_results, 1);
}
