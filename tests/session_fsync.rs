//! ZE-304: a tool call costs at most 3 journal fsyncs (operation start, tool
//! result turn, operation finish); audit events ride the next boundary.
mod support;

use serde_json::json;
use support::scripted::{ScriptedBackend, call, text};
use zenpi::{
    core::{Agent, TurnInputRequest},
    session::{SessionStore, journal_fsync_count},
    tools::{SideEffectPolicy, ToolContext, ToolRegistry},
};

#[test]
fn tool_calls_fsync_only_at_boundaries_and_keep_every_turn() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("a.txt"), "hello").unwrap();
    let path = workspace.path().join("s.jsonl");
    let calls: u64 = 8;
    let mut replies: Vec<_> = (0..calls)
        .map(|index| call(&format!("c{index}"), "read_file", json!({"path": "a.txt"})))
        .collect();
    replies.push(text("done"));
    let backend = ScriptedBackend::new(replies);
    let mut agent = Agent::new(SessionStore::open(&path).unwrap(), Box::new(backend));
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(workspace.path()).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    agent.enable_auto_approve();
    let before = journal_fsync_count();
    let started = std::time::Instant::now();
    agent.process(TurnInputRequest::new("read it")).unwrap();
    let fsyncs = journal_fsync_count() - before;
    eprintln!(
        "{fsyncs} fsyncs for {calls} tool calls in {:?}",
        started.elapsed()
    );
    // Per tool call: start, result turn, finish, plus the next model reply
    // turn; the prompt adds its user turn, provider operation pair and one
    // session event.
    assert!(fsyncs <= 4 * calls + 4, "{fsyncs}");
    // Every accepted turn reached the journal.
    drop(agent);
    let reopened = SessionStore::open_existing(&path).unwrap();
    assert_eq!(reopened.turns().len() as u64, 2 * calls + 2);
}
