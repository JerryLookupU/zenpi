mod support;

use serde_json::{Value, json};
use support::scripted::{ScriptedBackend, call, text};
use tempfile::tempdir;
use zenpi::{
    core::{Agent, TurnInputRequest, TurnRole},
    session::SessionStore,
    tools::{self, SideEffectPolicy, ToolContext, ToolRegistry},
};

fn writing_agent(root: &std::path::Path, backend: ScriptedBackend) -> Agent {
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
    approval.remember("write_file", zenpi::approval::ApprovalDecision::Allow);
    agent.set_approval_policy(approval);
    agent
}

fn tool_turn_metadata(agent: &Agent) -> Vec<Value> {
    agent
        .history()
        .iter()
        .filter(|turn| turn.role == TurnRole::Tool)
        .map(|turn| turn.metadata.clone().unwrap())
        .collect()
}

// ZE-204: an 80 KB write is far above the old 64 KiB per-call cap but well
// within MAX_WRITE_BYTES, so it must succeed.
#[test]
fn eighty_kilobyte_write_file_succeeds() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let content = "line of generated code;\n".repeat(80 * 1024 / 24 + 1);
    assert!(content.len() > 80 * 1024);
    let backend = ScriptedBackend::new([
        call(
            "big-write",
            "write_file",
            json!({"path": "big.txt", "content": content}),
        ),
        text("done"),
    ]);
    let mut agent = writing_agent(root, backend.clone());

    let result = agent.process(TurnInputRequest::new("write it")).unwrap();
    assert_eq!(result.assistant.unwrap().content, "done");
    assert_eq!(
        std::fs::read_to_string(root.join("big.txt")).unwrap(),
        content
    );
    let metadata = tool_turn_metadata(&agent);
    assert_eq!(metadata.len(), 1);
    assert_eq!(metadata[0]["is_error"], false);
    assert_eq!(backend.requests()[1].tool_results, 1);
}

// ZE-204: an over-limit call becomes an error tool result; the turn goes on
// and the next provider request sees that result.
#[test]
fn over_limit_tool_call_returns_an_error_result_and_the_turn_continues() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let content = "x".repeat(tools::MAX_TOOL_CALL_BYTES + 1);
    let backend = ScriptedBackend::new([
        call(
            "huge-write",
            "write_file",
            json!({"path": "huge.txt", "content": content}),
        ),
        call(
            "small-write",
            "write_file",
            json!({"path": "small.txt", "content": "ok"}),
        ),
        text("recovered"),
    ]);
    let mut agent = writing_agent(root, backend.clone());

    let result = agent.process(TurnInputRequest::new("write it")).unwrap();
    assert_eq!(result.assistant.unwrap().content, "recovered");
    assert!(!root.join("huge.txt").exists());
    assert_eq!(
        std::fs::read_to_string(root.join("small.txt")).unwrap(),
        "ok"
    );
    let metadata = tool_turn_metadata(&agent);
    assert_eq!(metadata.len(), 2);
    assert_eq!(metadata[0]["is_error"], true);
    let message = metadata[0]["model_text"].as_str().unwrap();
    assert!(message.contains("the limit is"), "{message}");
    assert_eq!(metadata[1]["is_error"], false);
    // The oversized arguments are not copied into the journal.
    let assistant = agent
        .history()
        .iter()
        .find(|turn| {
            turn.metadata
                .as_ref()
                .and_then(|metadata| metadata.get("tool_calls"))
                .is_some()
        })
        .unwrap();
    let recorded = &assistant.metadata.as_ref().unwrap()["tool_calls"][0];
    assert_eq!(recorded["arguments_omitted"], true);
    assert_eq!(recorded["arguments"], json!({}));
    let requests = backend.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1].tool_results, 1);
}
