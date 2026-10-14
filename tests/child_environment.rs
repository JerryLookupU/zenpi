//! ZE-200: commands run through the agent's `run_command` tool inherit the
//! user's environment (PATH, HOME, ...) minus secret-bearing variables.
#![cfg(unix)]
mod support;

use serde_json::json;
use support::scripted::{ScriptedBackend, call, text};
use tempfile::tempdir;
use zenpi::{
    core::{Agent, TurnInputRequest, TurnRole},
    session::SessionStore,
    tools::{SideEffectPolicy, ToolContext, ToolRegistry},
};

const SECRET_VALUE: &str = "ze200-fixture-secret-value";

fn run_through_agent(command: &str) -> (i64, String) {
    let dir = tempdir().unwrap();
    let mut agent = Agent::new(
        SessionStore::open(dir.path().join("session.jsonl")).unwrap(),
        Box::new(ScriptedBackend::new([
            call("call-1", "run_command", json!({ "command": command })),
            text("done"),
        ])),
    );
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(dir.path()).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    let mut policy = zenpi::approval::ApprovalPolicy::default();
    policy.remember("run_command", zenpi::approval::ApprovalDecision::Allow);
    agent.set_approval_policy(policy);
    let result = agent.process(TurnInputRequest::new("run it")).unwrap();
    assert_eq!(result.assistant.unwrap().content, "done");
    let record = agent
        .history()
        .iter()
        .find(|turn| turn.role == TurnRole::Tool)
        .expect("run_command produced a tool turn")
        .content
        .clone();
    let record: serde_json::Value = serde_json::from_str(&record).unwrap();
    (
        record["output"]["exit_code"].as_i64().unwrap(),
        record["output"]["stdout"].as_str().unwrap().to_owned(),
    )
}

#[test]
fn run_command_inherits_user_environment_without_secrets() {
    // SAFETY: this test binary sets these variables once, before any child
    // spawn, and no other test in it reads the environment concurrently.
    unsafe {
        std::env::set_var("ZE200_PLAIN_MARKER", "ze200-visible");
        std::env::set_var("ZE200_FIXTURE_API_KEY", SECRET_VALUE);
        std::env::set_var("ZE200_FIXTURE_TOKEN", SECRET_VALUE);
    }

    let home = std::env::var("HOME").expect("test process has HOME");
    let (code, output) = run_through_agent("command -v cargo && echo $HOME");
    assert!(
        output.contains("cargo"),
        "cargo not on child PATH: {output}"
    );
    assert!(
        output.lines().any(|line| line == home),
        "child HOME is not {home}: {output}"
    );
    assert_eq!(code, 0, "{output}");

    let (code, output) = run_through_agent("env; echo ---; env | grep ZE200_");
    assert_eq!(code, 0, "{output}");
    assert!(
        output.contains("ZE200_PLAIN_MARKER=ze200-visible"),
        "{output}"
    );
    assert!(!output.contains("ZE200_FIXTURE_API_KEY"), "{output}");
    assert!(!output.contains("ZE200_FIXTURE_TOKEN"), "{output}");
    assert!(!output.contains(SECRET_VALUE), "{output}");
}
