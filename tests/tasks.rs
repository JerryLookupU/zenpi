//! ZE-400: in-process golden tasks. A scripted model drives the real agent
//! loop, real tools and a temporary workspace; every task asserts the final
//! files, the journal and the provider request count.
mod support;

use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use support::scripted::{FakeServer, Reply, ScriptedBackend, call, text, truncated_call};
use tempfile::tempdir;
use zenpi::{
    approval::{ApprovalDecision, ApprovalMode, ApprovalPolicy, ApprovalResponse},
    backend::{OpenAiCompatibleBackend, OpenAiWireApi},
    core::{Agent, ConnectionSelection, ConnectionSelectionOutcome, TurnInputRequest, TurnRole},
    session::SessionStore,
    tools::{SideEffectPolicy, ToolCall, ToolContext, ToolRegistry},
};

fn agent_with(workspace: &Path, backend: Box<dyn zenpi::backend::Backend>) -> Agent {
    let session = SessionStore::open(workspace.join("task.jsonl")).unwrap();
    let mut agent = Agent::new(session, backend);
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(workspace).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    agent
}

fn scripted(workspace: &Path, replies: Vec<Reply>) -> (Agent, ScriptedBackend) {
    let backend = ScriptedBackend::new(replies);
    let mut agent = agent_with(workspace, Box::new(backend.clone()));
    agent.enable_auto_approve();
    (agent, backend)
}

fn tool_turns(agent: &Agent) -> Vec<String> {
    agent
        .history()
        .iter()
        .filter(|turn| turn.role == TurnRole::Tool)
        .map(|turn| turn.content.clone())
        .collect()
}

/// The journal reopens cleanly and holds exactly the agent's turns.
fn assert_journal_consistent(agent: &Agent) {
    let reopened = SessionStore::open_existing(agent.session().path()).unwrap();
    assert_eq!(reopened.turns(), agent.history());
    assert!(reopened.recovery_warnings().is_empty());
}

fn call_of(id: &str, name: &str, arguments: Value) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: name.into(),
        arguments,
    }
}

#[test]
fn read_a_file_then_answer() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("notes.txt"), "the answer is 42\n").unwrap();
    let (mut agent, backend) = scripted(
        dir.path(),
        vec![
            call("c1", "read_file", json!({"path": "notes.txt"})),
            text("It says 42."),
        ],
    );
    let result = agent
        .process(TurnInputRequest::new("what is in notes?"))
        .unwrap();
    assert_eq!(result.assistant.unwrap().content, "It says 42.");
    let requests = backend.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].tool_results, 1);
    assert!(tool_turns(&agent)[0].contains("the answer is 42"));
    assert_journal_consistent(&agent);
}

#[test]
fn create_a_file() {
    let dir = tempdir().unwrap();
    let (mut agent, backend) = scripted(
        dir.path(),
        vec![
            call(
                "c1",
                "write_file",
                json!({"path": "src/new.rs", "content": "fn main() {}\n"}),
            ),
            text("Created."),
        ],
    );
    agent
        .process(TurnInputRequest::new("create src/new.rs"))
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("src/new.rs")).unwrap(),
        "fn main() {}\n"
    );
    assert_eq!(backend.requests().len(), 2);
    assert_journal_consistent(&agent);
}

#[test]
fn edit_a_file_and_report_the_diff() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("lib.rs"), "let x = 1;\nlet y = 2;\n").unwrap();
    let (mut agent, backend) = scripted(
        dir.path(),
        vec![
            call(
                "c1",
                "edit_file",
                json!({"path": "lib.rs", "old": "let x = 1;", "new": "let x = 10;"}),
            ),
            text("Edited."),
        ],
    );
    agent.process(TurnInputRequest::new("set x to 10")).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("lib.rs")).unwrap(),
        "let x = 10;\nlet y = 2;\n"
    );
    let result = &tool_turns(&agent)[0];
    assert!(result.contains("-let x = 1;"), "{result}");
    assert!(result.contains("+let x = 10;"), "{result}");
    assert_eq!(backend.requests().len(), 2);
    assert_journal_consistent(&agent);
}

#[test]
fn bash_output_reaches_the_next_request() {
    let dir = tempdir().unwrap();
    let (mut agent, backend) = scripted(
        dir.path(),
        vec![
            call(
                "c1",
                "run_command",
                json!({"command": "echo hello-from-bash"}),
            ),
            text("Saw it."),
        ],
    );
    agent.process(TurnInputRequest::new("run echo")).unwrap();
    assert!(tool_turns(&agent)[0].contains("hello-from-bash"));
    let requests = backend.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].tool_results, 1);
    assert_journal_consistent(&agent);
}

#[test]
fn bash_failure_is_fixed_by_the_model() {
    let dir = tempdir().unwrap();
    let (mut agent, backend) = scripted(
        dir.path(),
        vec![
            call("c1", "run_command", json!({"command": "cat missing.txt"})),
            call(
                "c2",
                "run_command",
                json!({"command": "echo fixed > missing.txt && cat missing.txt"}),
            ),
            text("Fixed."),
        ],
    );
    let result = agent
        .process(TurnInputRequest::new("show missing.txt"))
        .unwrap();
    assert_eq!(result.assistant.unwrap().content, "Fixed.");
    let results = tool_turns(&agent);
    assert!(results[0].contains("exited with code"), "{}", results[0]);
    assert!(results[1].contains("fixed"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("missing.txt")).unwrap(),
        "fixed\n"
    );
    assert_eq!(backend.requests().len(), 3);
    assert_journal_consistent(&agent);
}

#[test]
fn three_parallel_tools_in_one_turn() {
    let dir = tempdir().unwrap();
    for name in ["a", "b", "c"] {
        std::fs::write(
            dir.path().join(format!("{name}.txt")),
            format!("content {name}\n"),
        )
        .unwrap();
    }
    let (mut agent, backend) = scripted(
        dir.path(),
        vec![
            Reply::Calls(
                ["a", "b", "c"]
                    .iter()
                    .map(|name| {
                        call_of(
                            &format!("c-{name}"),
                            "read_file",
                            json!({"path": format!("{name}.txt")}),
                        )
                    })
                    .collect(),
            ),
            text("Read all three."),
        ],
    );
    agent
        .process(TurnInputRequest::new("read a, b and c"))
        .unwrap();
    let results = tool_turns(&agent);
    assert_eq!(results.len(), 3);
    for name in ["a", "b", "c"] {
        assert!(
            results
                .iter()
                .any(|result| result.contains(&format!("content {name}")))
        );
    }
    let requests = backend.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].tool_results, 3);
    assert_journal_consistent(&agent);
}

#[test]
fn denied_write_stays_off_disk_and_approved_write_lands() {
    for decision in [ApprovalDecision::Deny, ApprovalDecision::Allow] {
        let dir = tempdir().unwrap();
        let backend = ScriptedBackend::new([
            call(
                "c1",
                "write_file",
                json!({"path": "gated.txt", "content": "written"}),
            ),
            text("Done."),
        ]);
        let mut agent = agent_with(dir.path(), Box::new(backend.clone()));
        let coordinator = agent
            .set_approval_policy(ApprovalPolicy {
                mode: ApprovalMode::Always,
                ..ApprovalPolicy::default()
            })
            .unwrap();
        let worker = thread::spawn(move || {
            let result = agent.process(TurnInputRequest::new("write gated.txt"));
            (agent, result)
        });
        let deadline = Instant::now() + Duration::from_secs(10);
        let request = loop {
            if let Some(request) = coordinator.drain_pending().pop() {
                break request;
            }
            assert!(Instant::now() < deadline, "approval never became pending");
            thread::sleep(Duration::from_millis(2));
        };
        assert!(!dir.path().join("gated.txt").exists());
        coordinator
            .respond(ApprovalResponse {
                request_id: request.request_id,
                decision,
                remember: false,
                message: None,
            })
            .unwrap();
        let (agent, result) = worker.join().unwrap();
        result.unwrap();
        assert_eq!(
            dir.path().join("gated.txt").exists(),
            decision == ApprovalDecision::Allow
        );
        assert_eq!(backend.requests().len(), 2);
        assert_journal_consistent(&agent);
    }
}

#[test]
fn cancel_during_the_provider_call_and_during_a_tool() {
    // Provider: the model never answers until cancelled.
    let dir = tempdir().unwrap();
    let (mut agent, backend) = scripted(dir.path(), vec![Reply::Stall]);
    let flag = Arc::new(AtomicBool::new(false));
    let trigger = {
        let (flag, backend) = (flag.clone(), backend.clone());
        thread::spawn(move || {
            while !backend.is_stalled() {
                thread::sleep(Duration::from_millis(1));
            }
            flag.store(true, Ordering::SeqCst);
        })
    };
    let cancelled = flag.clone();
    assert!(
        agent
            .process_with_cancel(TurnInputRequest::new("hang"), move || cancelled
                .load(Ordering::SeqCst))
            .is_err()
    );
    trigger.join().unwrap();
    assert_journal_consistent(&agent);

    // Tool: a long command is stopped and its outcome journaled.
    let dir = tempdir().unwrap();
    let (mut agent, _backend) = scripted(
        dir.path(),
        vec![
            call(
                "c1",
                "run_command",
                json!({"command": ": > started; sleep 30"}),
            ),
            text("unreachable"),
        ],
    );
    let started = dir.path().join("started");
    let begun = Instant::now();
    let _ = agent.process_with_cancel(TurnInputRequest::new("sleep"), move || started.exists());
    assert!(
        begun.elapsed() < Duration::from_secs(10),
        "cancel did not stop the tool"
    );
    // A killed side-effecting command has unknown side effects: one recorded
    // outcome awaits an explicit decision, and its result turn is journaled.
    assert_eq!(agent.unknown_tool_outcomes().len(), 1);
    assert_eq!(tool_turns(&agent).len(), 1);
    assert_journal_consistent(&agent);
}

#[test]
fn a_turn_killed_mid_tool_is_repaired_on_restart() {
    let dir = tempdir().unwrap();
    let (mut agent, _) = scripted(
        dir.path(),
        vec![
            call("c1", "read_file", json!({"path": "missing-on-purpose.txt"})),
            text("first done"),
        ],
    );
    agent.process(TurnInputRequest::new("first")).unwrap();
    let path = agent.session().path().to_owned();
    drop(agent);
    // Simulate a kill right after the model asked for the tool: keep the
    // journal up to the assistant call turn, losing the tool result onward.
    let journal = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = journal.lines().collect();
    let cut = lines
        .iter()
        .position(|line| line.contains("\"kind\":\"turn\"") && line.contains("\"c1\""))
        .unwrap();
    std::fs::write(&path, format!("{}\n", lines[..=cut].join("\n"))).unwrap();

    let server = FakeServer::start(OpenAiWireApi::ChatCompletions, [text("after restart")]);
    let http = OpenAiCompatibleBackend::from_values_with_settings_and_timeout(
        server.url.clone(),
        None,
        "gpt-4o-mini".into(),
        OpenAiWireApi::ChatCompletions,
        None,
        None,
        Duration::from_secs(10),
    )
    .unwrap();
    let mut agent = Agent::new(SessionStore::open(&path).unwrap(), Box::new(http));
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(dir.path()).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    agent.enable_auto_approve();
    // Every host does this at start-up.
    agent.acknowledge_recovery().unwrap();
    let result = agent.process(TurnInputRequest::new("continue")).unwrap();
    assert_eq!(result.assistant.unwrap().content, "after restart");
    // The dangling call is answered on the wire, so a strict provider accepts it.
    let messages = server.bodies()[0]["messages"].clone();
    assert!(
        messages
            .as_array()
            .unwrap()
            .iter()
            .any(|message| message["role"] == "tool" && message["tool_call_id"] == "c1"),
        "{messages}"
    );
    assert_journal_consistent(&agent);
}

#[test]
fn iteration_and_doom_loop_budgets_stop_a_runaway_model() {
    // Iteration cap: a model that always asks for one more distinct call.
    let dir = tempdir().unwrap();
    let replies: Vec<Reply> = (0..50)
        .map(|index| {
            call(
                &format!("c{index}"),
                "run_command",
                json!({"command": format!("echo {index}")}),
            )
        })
        .collect();
    let (mut agent, backend) = scripted(dir.path(), replies);
    agent.set_max_tool_iterations(3);
    let _ = agent.process(TurnInputRequest::new("loop forever"));
    assert!(
        backend.requests().len() <= 4,
        "{}",
        backend.requests().len()
    );
    assert!(tool_turns(&agent).len() <= 3);
    assert_journal_consistent(&agent);

    // Doom loop: the identical call repeated is refused, not executed again.
    let dir = tempdir().unwrap();
    let (mut agent, _) = scripted(
        dir.path(),
        vec![
            call(
                "d1",
                "run_command",
                json!({"command": "echo same >> count"}),
            ),
            call(
                "d2",
                "run_command",
                json!({"command": "echo same >> count"}),
            ),
            call(
                "d3",
                "run_command",
                json!({"command": "echo same >> count"}),
            ),
            call(
                "d4",
                "run_command",
                json!({"command": "echo same >> count"}),
            ),
            call(
                "d5",
                "run_command",
                json!({"command": "echo same >> count"}),
            ),
            call(
                "d6",
                "run_command",
                json!({"command": "echo same >> count"}),
            ),
            call(
                "d7",
                "run_command",
                json!({"command": "echo same >> count"}),
            ),
            text("gave up"),
        ],
    );
    let _ = agent.process(TurnInputRequest::new("repeat"));
    let executed = std::fs::read_to_string(dir.path().join("count")).unwrap_or_default();
    // The threshold is 5 identical calls; later repeats are refused unexecuted.
    assert_eq!(executed.lines().count(), 5);
    assert!(
        tool_turns(&agent)
            .iter()
            .any(|result| result.contains("doom_loop"))
    );
    assert_journal_consistent(&agent);
}

#[test]
fn a_retryable_provider_error_is_retried() {
    let dir = tempdir().unwrap();
    let server = FakeServer::start(
        OpenAiWireApi::ChatCompletions,
        [Reply::Overloaded, text("recovered")],
    );
    let backend = OpenAiCompatibleBackend::from_values_with_settings_and_timeout(
        server.url.clone(),
        None,
        "gpt-4o-mini".into(),
        OpenAiWireApi::ChatCompletions,
        None,
        None,
        Duration::from_secs(10),
    )
    .unwrap()
    .with_max_retries(2)
    .unwrap();
    let mut agent = agent_with(dir.path(), Box::new(backend));
    let result = agent.process(TurnInputRequest::new("hello")).unwrap();
    assert_eq!(result.assistant.unwrap().content, "recovered");
    assert_eq!(server.requests().len(), 2);
    assert_journal_consistent(&agent);
}

#[test]
fn a_length_truncated_turn_runs_none_of_its_tools() {
    let dir = tempdir().unwrap();
    let (mut agent, _) = scripted(
        dir.path(),
        vec![
            truncated_call(
                "c1",
                "write_file",
                json!({"path": "half.txt", "content": "partial"}),
            ),
            text("Retried within budget."),
        ],
    );
    let _ = agent.process(TurnInputRequest::new("write half.txt"));
    assert!(!dir.path().join("half.txt").exists());
    assert_journal_consistent(&agent);
}

#[test]
fn switching_model_mid_session_keeps_working() {
    let dir = tempdir().unwrap();
    let first = FakeServer::start(OpenAiWireApi::ChatCompletions, [text("from first")]);
    let second = FakeServer::start(OpenAiWireApi::Responses, [text("from second")]);
    let native = |server: &FakeServer, model: &str, wire| {
        OpenAiCompatibleBackend::from_values_with_settings_and_timeout(
            server.url.clone(),
            None,
            model.into(),
            wire,
            None,
            None,
            Duration::from_secs(10),
        )
        .unwrap()
    };
    let mut agent = agent_with(
        dir.path(),
        Box::new(native(
            &first,
            "gpt-4o-mini",
            OpenAiWireApi::ChatCompletions,
        )),
    );
    agent.process(TurnInputRequest::new("one")).unwrap();
    let owner = agent.owner_label().to_owned();
    let outcome = agent
        .select_connection(ConnectionSelection {
            owner_id: owner,
            profile: "switched".into(),
            model: Some("gpt-4.1".into()),
            expected_selection_revision: None,
            backend: Box::new(native(&second, "gpt-4.1", OpenAiWireApi::Responses)),
        })
        .unwrap();
    assert!(matches!(
        outcome,
        ConnectionSelectionOutcome::Applied { .. }
    ));
    let result = agent.process(TurnInputRequest::new("two")).unwrap();
    assert_eq!(result.assistant.unwrap().content, "from second");
    // The second provider received the first exchange as history.
    let history = &second.requests()[0].messages;
    assert!(
        history.iter().any(|message| message.contains("from first")),
        "{history:?}"
    );
    assert_journal_consistent(&agent);
}

#[test]
fn a_project_owner_repairs_a_dangling_call_on_open() {
    // The remote host and project tabs open sessions through this path; it
    // must answer a dangling call just like the CLI start-up does.
    let dir = tempdir().unwrap();
    let workspace = dir.path().canonicalize().unwrap();
    let path = workspace.join("owner.jsonl");
    {
        let backend = ScriptedBackend::new([
            call("c1", "read_file", json!({"path": "absent.txt"})),
            text("done"),
        ]);
        let mut agent = Agent::new(
            SessionStore::open_in_workspace(&path, &workspace).unwrap(),
            Box::new(backend),
        );
        agent.set_tools(
            ToolRegistry::with_all_builtins().unwrap(),
            ToolContext::new(&workspace).unwrap(),
            SideEffectPolicy::all_builtins(),
        );
        agent.enable_auto_approve();
        agent.process(TurnInputRequest::new("first")).unwrap();
    }
    let journal = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = journal.lines().collect();
    let cut = lines
        .iter()
        .position(|line| line.contains("\"kind\":\"turn\"") && line.contains("\"c1\""))
        .unwrap();
    std::fs::write(&path, format!("{}\n", lines[..=cut].join("\n"))).unwrap();
    let owner =
        Agent::prepare_project_with_options(&path, &workspace, Default::default(), true).unwrap();
    assert!(
        owner
            .history()
            .iter()
            .any(|turn| turn.role == TurnRole::Tool && turn.content.contains("never dispatched"))
    );
}
