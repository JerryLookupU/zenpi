use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
    time::Duration,
};
use tempfile::tempdir;
use zenpi::{
    backend::{OpenAiCompatibleBackend, OpenAiWireApi},
    core::{Agent, TurnInputRequest},
    providers::registry::ModelRegistry,
    session::SessionStore,
};

fn backend(url: &str, effort: Option<&str>, wire: OpenAiWireApi) -> OpenAiCompatibleBackend {
    OpenAiCompatibleBackend::new_with_settings(
        url,
        None,
        "gpt-5.2",
        wire,
        effort.map(str::to_owned),
        None,
    )
    .unwrap()
    .with_model_registry("openai".into(), ModelRegistry::default())
    .unwrap()
}

fn read_request(stream: &mut TcpStream) -> Value {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut bytes = Vec::new();
    let end = loop {
        let mut chunk = [0; 4096];
        let count = stream.read(&mut chunk).unwrap();
        assert!(count > 0);
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            break end + 4;
        }
        assert!(bytes.len() < 65536);
    };
    let len: usize = String::from_utf8_lossy(&bytes[..end])
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(str::to_owned)
        })
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(len < 2 * 1024 * 1024);
    while bytes.len() < end + len {
        let mut chunk = [0; 4096];
        let count = stream.read(&mut chunk).unwrap();
        assert!(count > 0);
        bytes.extend_from_slice(&chunk[..count]);
    }
    serde_json::from_slice(&bytes[end..end + len]).unwrap()
}
fn server(count: usize) -> (String, mpsc::Receiver<Value>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let (send, recv) = mpsc::channel();
    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        for _ in 0..count {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            let request = read_request(&mut stream);
            let body = format!(
                "data: {}\n\ndata: {}\n\n",
                json!({"type":"response.output_text.delta","delta":"ok"}),
                json!({"type":"response.completed","response":{"id":"r","model":request["model"],"status":"completed"}})
            );
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            stream.flush().unwrap();
            send.send(request).unwrap();
        }
    });
    (url, recv, handle)
}

#[test]
fn real_requests_use_new_effort_and_restart_preserves_model_effort_and_default_reset() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("s.jsonl");
    let (url, requests, server) = server(3);
    let mut agent = Agent::new(
        SessionStore::open(&path).unwrap(),
        Box::new(backend(&url, None, OpenAiWireApi::Responses)),
    );
    agent.set_reasoning_effort(Some("high".into())).unwrap();
    assert_eq!(agent.reasoning_effort(), Some("high"));
    assert_eq!(agent.model_status().unwrap()["reasoning_effort"], "high");
    agent.process(TurnInputRequest::new("first")).unwrap();
    assert_eq!(
        requests.recv_timeout(Duration::from_secs(5)).unwrap()["reasoning"]["effort"],
        "high"
    );
    agent.set_reasoning_effort(Some("low".into())).unwrap();
    drop(agent);
    let mut agent = Agent::new(
        SessionStore::open(&path).unwrap(),
        Box::new(backend(&url, Some("high"), OpenAiWireApi::Responses)),
    );
    agent.restore_model_selection().unwrap();
    assert_eq!(agent.reasoning_effort(), Some("low"));
    agent.process(TurnInputRequest::new("second")).unwrap();
    assert_eq!(
        requests.recv_timeout(Duration::from_secs(5)).unwrap()["reasoning"]["effort"],
        "low"
    );
    agent.set_reasoning_effort(None).unwrap();
    agent.set_model(Some("gpt-4.1".into())).unwrap();
    drop(agent);
    // Restoring saved None must not validate against stale factory 'high'.
    let mut agent = Agent::new(
        SessionStore::open(&path).unwrap(),
        Box::new(backend(&url, Some("high"), OpenAiWireApi::Responses)),
    );
    agent.restore_model_selection().unwrap();
    assert_eq!(agent.reasoning_effort(), None);
    assert_eq!(agent.model_status().unwrap()["active"]["id"], "gpt-4.1");
    agent.process(TurnInputRequest::new("third")).unwrap();
    let request = requests.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(request["model"], "gpt-4.1");
    assert!(request.get("reasoning").is_none());
    server.join().unwrap();
}

#[test]
fn invalid_level_or_wire_does_not_change_effective_state_or_journal() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("s.jsonl");
    let mut agent = Agent::new(
        SessionStore::open(&path).unwrap(),
        Box::new(backend(
            "http://127.0.0.1:9",
            Some("high"),
            OpenAiWireApi::Responses,
        )),
    );
    let state = agent.model_status().unwrap();
    let bytes = fs::read(&path).unwrap();
    for effort in ["max", "ultra", "HIGH", "", "low\n"] {
        assert!(agent.set_reasoning_effort(Some(effort.into())).is_err());
        assert_eq!(state, agent.model_status().unwrap());
        assert_eq!(bytes, fs::read(&path).unwrap());
    }
    let other = dir.path().join("chat.jsonl");
    let mut chat = Agent::new(
        SessionStore::open(&other).unwrap(),
        Box::new(backend(
            "http://127.0.0.1:9",
            None,
            OpenAiWireApi::ChatCompletions,
        )),
    );
    let before = fs::read(&other).unwrap();
    // ZE-224: Chat carries the catalogued levels, but not one gpt-5.2 lacks.
    assert!(chat.set_reasoning_effort(Some("max".into())).is_err());
    assert_eq!(before, fs::read(&other).unwrap());
    assert_eq!(chat.reasoning_effort(), None);
    agent.set_reasoning_effort(Some("none".into())).unwrap();
    assert_eq!(agent.reasoning_effort(), Some("none"));
}

#[test]
fn failed_durable_append_keeps_provider_effort_unchanged() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("readonly.jsonl");
    drop(SessionStore::open(&path).unwrap());
    let readonly = SessionStore::open_existing(&path).unwrap();
    let mut agent = Agent::new(
        readonly,
        Box::new(backend(
            "http://127.0.0.1:9",
            Some("high"),
            OpenAiWireApi::Responses,
        )),
    );
    let old = agent.model_status().unwrap();
    let bytes = fs::read(&path).unwrap();
    let saved = dir.path().join("saved.jsonl");
    fs::rename(&path, &saved).unwrap();
    fs::create_dir(&path).unwrap();
    let records = agent.session().records().len();
    assert!(agent.set_reasoning_effort(Some("low".into())).is_err());
    assert_eq!(records, agent.session().records().len());
    assert_eq!(old, agent.model_status().unwrap());
    assert_eq!(bytes, fs::read(&saved).unwrap());
}

#[test]
fn active_or_closed_owner_rejects_changes_before_validation_or_journaling() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("busy.jsonl");
    let mut agent = Agent::new(
        SessionStore::open(&path).unwrap(),
        Box::new(backend(
            "http://127.0.0.1:9",
            Some("high"),
            OpenAiWireApi::Responses,
        )),
    );
    agent.submit(TurnInputRequest::new("pending")).unwrap();
    let before = fs::read(&path).unwrap();
    assert!(agent.set_reasoning_effort(Some("low".into())).is_err());
    assert_eq!(agent.reasoning_effort(), Some("high"));
    assert_eq!(before, fs::read(&path).unwrap());
    let other = dir.path().join("closed.jsonl");
    let mut closed = Agent::new(
        SessionStore::open(&other).unwrap(),
        Box::new(backend(
            "http://127.0.0.1:9",
            Some("high"),
            OpenAiWireApi::Responses,
        )),
    );
    closed.try_close().unwrap();
    let before = fs::read(&other).unwrap();
    assert!(closed.set_reasoning_effort(None).is_err());
    assert_eq!(before, fs::read(&other).unwrap());
}

#[test]
fn invalid_saved_effort_fails_resume_before_replacing_any_current_owner_state() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("current.jsonl");
    let target = dir.path().join("invalid.jsonl");
    let mut agent = Agent::new(
        SessionStore::open(&path).unwrap(),
        Box::new(backend(
            "http://127.0.0.1:9",
            Some("high"),
            OpenAiWireApi::Responses,
        )),
    );
    let descriptor = ModelRegistry::default()
        .resolve("openai", "gpt-5.2")
        .unwrap();
    let mut invalid = SessionStore::open(&target).unwrap();
    invalid.append_event(json!({"type":"model_selected","model":"gpt-5.2","descriptor":descriptor,"digest":descriptor.digest(),"reasoning_effort":"ultra"})).unwrap();
    drop(invalid);
    let old = agent.model_status().unwrap();
    let session = agent.session().session_id().to_owned();
    let bytes = fs::read(&path).unwrap();
    assert!(agent.resume_session(&target).is_err());
    assert_eq!(session, agent.session().session_id());
    assert_eq!(old, agent.model_status().unwrap());
    assert_eq!(bytes, fs::read(&path).unwrap());
}

fn assert_selection_fenced(agent: &mut Agent) {
    let before = fs::read(agent.session().path()).unwrap();
    let state = agent.model_status().unwrap();
    let revision = agent.selection_revision();
    assert!(matches!(
        agent.set_model(Some("gpt-5.2-2025-12-11".into())),
        Err(zenpi::core::AgentError::NotIdle)
    ));
    assert!(matches!(
        agent.set_reasoning_effort(Some("low".into())),
        Err(zenpi::core::AgentError::NotIdle)
    ));
    assert!(matches!(
        agent
            .select_reasoning(Some("low".into()), revision)
            .unwrap(),
        zenpi::core::ConnectionSelectionOutcome::Rejected {
            code: zenpi::core::ConnectionRejectCode::ConnectionBusy,
            ..
        }
    ));
    assert_eq!(state, agent.model_status().unwrap());
    assert_eq!(revision, agent.selection_revision());
    assert_eq!(before, fs::read(agent.session().path()).unwrap());
}

fn queue_request(
    agent: &Agent,
    id: &str,
    action: zenpi::protocol::InputQueueAction,
) -> zenpi::protocol::InputQueueRequest {
    zenpi::protocol::InputQueueRequest {
        schema_version: 2,
        id: id.into(),
        kind: "input_queue".into(),
        session_id: agent.session().session_id().into(),
        input_queue: action,
    }
}

#[test]
fn staged_attachments_fence_model_and_reasoning_until_cleared() {
    let dir = tempdir().unwrap();
    let mut agent = Agent::new(
        SessionStore::open(dir.path().join("staged.jsonl")).unwrap(),
        Box::new(backend(
            "http://127.0.0.1:9",
            Some("high"),
            OpenAiWireApi::Responses,
        )),
    );
    let attachment = zenpi::backend::InputAttachment {
        kind: zenpi::backend::AttachmentKind::Image,
        mime_type: "image/png".into(),
        path: None,
        url: Some("https://example.invalid/staged.png".into()),
        file_id: None,
    };
    agent.stage_attachment(attachment.clone()).unwrap();
    assert_eq!(agent.phase(), zenpi::core::AgentPhase::Idle);
    assert_selection_fenced(&mut agent);
    assert_eq!(agent.pending_attachments(), &[attachment]);
    agent.clear_pending_attachments();
    agent.set_model(Some("gpt-5.2-2025-12-11".into())).unwrap();
    agent.set_reasoning_effort(Some("low".into())).unwrap();
    assert_eq!(agent.reasoning_effort(), Some("low"));
}

#[test]
fn durable_queued_input_fences_selection_after_restart_until_cancelled() {
    use zenpi::{input_queue::InputKind, protocol::InputQueueAction};
    let dir = tempdir().unwrap();
    let path = dir.path().join("queued.jsonl");
    let make_backend = || {
        Box::new(backend(
            "http://127.0.0.1:9",
            Some("high"),
            OpenAiWireApi::Responses,
        ))
    };
    let mut agent = Agent::new(SessionStore::open(&path).unwrap(), make_backend());
    let request = queue_request(
        &agent,
        "enqueue",
        InputQueueAction::Enqueue {
            input_id: "queued-phone-input".into(),
            kind: InputKind::FollowUp,
            text: "already addressed to this model".into(),
        },
    );
    agent.input_queue_request(request).unwrap();
    assert_selection_fenced(&mut agent);
    drop(agent);
    let mut restored = Agent::new(SessionStore::open(&path).unwrap(), make_backend());
    restored.restore_model_selection().unwrap();
    assert_eq!(restored.phase(), zenpi::core::AgentPhase::Idle);
    assert_selection_fenced(&mut restored);
    let cancel = queue_request(
        &restored,
        "cancel",
        InputQueueAction::Cancel {
            input_id: "queued-phone-input".into(),
        },
    );
    restored.input_queue_request(cancel).unwrap();
    restored.set_reasoning_effort(Some("low".into())).unwrap();
    restored
        .set_model(Some("gpt-5.2-2025-12-11".into()))
        .unwrap();
    assert_eq!(restored.model(), Some("gpt-5.2-2025-12-11"));
}

#[test]
fn unresolved_operations_fence_selection_after_restart_until_explicit_decision() {
    use zenpi::session::{InterruptedOperation, OperationKind, OperationOutcome};
    let dir = tempdir().unwrap();
    let path = dir.path().join("unknown.jsonl");
    let mut session = SessionStore::open(&path).unwrap();
    session
        .begin_operation(&InterruptedOperation {
            operation_id: "unknown-tool".into(),
            kind: OperationKind::Tool,
            turn_id: "previous-turn".into(),
            retry_requires_confirmation: true,
        })
        .unwrap();
    session
        .finish_operation("unknown-tool", OperationOutcome::UnknownOutcome)
        .unwrap();
    drop(session);
    let mut agent = Agent::new(
        SessionStore::open(&path).unwrap(),
        Box::new(backend(
            "http://127.0.0.1:9",
            Some("high"),
            OpenAiWireApi::Responses,
        )),
    );
    assert_eq!(agent.phase(), zenpi::core::AgentPhase::Idle);
    assert_selection_fenced(&mut agent);
    agent
        .session_mut()
        .decide_operation_recovery("unknown-tool", zenpi::core::ToolRecoveryDecision::Retry)
        .unwrap();
    agent.set_model(Some("gpt-5.2-2025-12-11".into())).unwrap();
    agent.set_reasoning_effort(Some("low".into())).unwrap();
}

#[test]
fn submitted_turn_fences_model_changes_even_before_provider_execution() {
    let dir = tempdir().unwrap();
    let mut agent = Agent::new(
        SessionStore::open(dir.path().join("submitted.jsonl")).unwrap(),
        Box::new(backend(
            "http://127.0.0.1:9",
            Some("high"),
            OpenAiWireApi::Responses,
        )),
    );
    agent
        .submit(TurnInputRequest::new("admitted mobile turn"))
        .unwrap();
    assert_selection_fenced(&mut agent);
}

#[test]
fn pending_approval_fences_idle_selection_until_the_request_is_settled() {
    use zenpi::{
        approval::{ApprovalMode, ApprovalPolicy, ApprovalRequest},
        tools::{SideEffectPolicy, ToolContext, ToolOrigin, ToolRegistry, ToolSideEffect},
    };
    let dir = tempdir().unwrap();
    let mut agent = Agent::new(
        SessionStore::open_in_workspace(dir.path().join("approval.jsonl"), dir.path()).unwrap(),
        Box::new(backend(
            "http://127.0.0.1:9",
            Some("high"),
            OpenAiWireApi::Responses,
        )),
    );
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(dir.path()).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    agent.set_approval_policy(ApprovalPolicy {
        mode: ApprovalMode::Always,
        ..Default::default()
    });
    let coordinator = agent.approval_coordinator().unwrap();
    let (registered, ready) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let resume = std::sync::Mutex::new(resume);
    let worker = thread::spawn(move || {
        coordinator.request(
            ApprovalRequest {
                request_id: "pending-approval".into(),
                turn_id: "approval-turn".into(),
                call_id: "approval-call".into(),
                tool: "write_file".into(),
                side_effect: ToolSideEffect::WorkspaceWrite,
                arguments: json!({"path":"note.txt","content":"pending"}),
                preview: None,
                origin: ToolOrigin::AgentTool,
                policy_digest: None,
                lease_id: None,
            },
            &mut ApprovalPolicy::default(),
            &|| {
                registered.send(()).unwrap();
                resume
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
                true
            },
        )
    });
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(agent.phase(), zenpi::core::AgentPhase::Idle);
    assert_selection_fenced(&mut agent);
    release.send(()).unwrap();
    assert!(worker.join().unwrap().is_err());
    assert!(!agent.approval_coordinator().unwrap().has_pending());
    agent.set_reasoning_effort(Some("low".into())).unwrap();
    agent.set_model(Some("gpt-5.2-2025-12-11".into())).unwrap();
    assert!(!dir.path().join("note.txt").exists());
}

#[test]
fn invalid_queue_recovery_fails_closed_before_selection_append() {
    let dir = tempdir().unwrap();
    let mut agent = Agent::new(
        SessionStore::open(dir.path().join("invalid-queue.jsonl")).unwrap(),
        Box::new(backend(
            "http://127.0.0.1:9",
            Some("high"),
            OpenAiWireApi::Responses,
        )),
    );
    agent
        .session_mut()
        .append_event(json!({"type":"input_queue","change":{"action":"invalid"}}))
        .unwrap();
    assert_selection_fenced(&mut agent);
}

#[test]
fn typed_model_selection_cas_and_failed_durable_append_preserve_effective_state() {
    use zenpi::core::{ConnectionRejectCode, ConnectionSelectionOutcome};
    let dir = tempdir().unwrap();
    let path = dir.path().join("typed-model.jsonl");
    let mut agent = Agent::new(
        SessionStore::open(&path).unwrap(),
        Box::new(backend(
            "http://127.0.0.1:9",
            Some("high"),
            OpenAiWireApi::Responses,
        )),
    );
    let before = fs::read(&path).unwrap();
    let state = agent.model_status().unwrap();
    assert!(matches!(
        agent.select_model("gpt-5.2-2025-12-11".into(), 1).unwrap(),
        ConnectionSelectionOutcome::Rejected {
            code: ConnectionRejectCode::ConnectionStale,
            ..
        }
    ));
    assert!(matches!(
        agent.select_model("gpt-4.1".into(), 0).unwrap(),
        ConnectionSelectionOutcome::Rejected {
            code: ConnectionRejectCode::ConnectionInvalid,
            ..
        }
    ));
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(agent.model_status().unwrap(), state);
    let saved = dir.path().join("saved.jsonl");
    fs::rename(&path, &saved).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(agent.select_model("gpt-5.2-2025-12-11".into(), 0).is_err());
    assert_eq!(agent.model_status().unwrap(), state);
    assert_eq!(agent.selection_revision(), 0);
    assert_eq!(fs::read(&saved).unwrap(), before);
}

#[test]
fn typed_reasoning_selection_choices_validate_actual_wire_and_append_before_commit() {
    use zenpi::core::{ConnectionRejectCode, ConnectionSelectionOutcome};
    let dir = tempdir().unwrap();
    let path = dir.path().join("typed-reasoning.jsonl");
    let mut agent = Agent::new(
        SessionStore::open(&path).unwrap(),
        Box::new(backend(
            "http://127.0.0.1:9",
            Some("high"),
            OpenAiWireApi::Responses,
        )),
    );
    assert_eq!(
        agent.reasoning_choices().unwrap(),
        vec![
            None,
            Some("none".into()),
            Some("low".into()),
            Some("medium".into()),
            Some("high".into()),
            Some("xhigh".into())
        ]
    );
    assert!(matches!(
        agent.select_reasoning(Some("low".into()), 1).unwrap(),
        ConnectionSelectionOutcome::Rejected {
            code: ConnectionRejectCode::ConnectionStale,
            ..
        }
    ));
    assert!(matches!(
        agent.select_reasoning(Some("ultra".into()), 0).unwrap(),
        ConnectionSelectionOutcome::Rejected {
            code: ConnectionRejectCode::ConnectionInvalid,
            ..
        }
    ));
    let before = fs::read(&path).unwrap();
    let state = agent.model_status().unwrap();
    let saved = dir.path().join("saved-reasoning.jsonl");
    fs::rename(&path, &saved).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(agent.select_reasoning(None, 0).is_err());
    assert_eq!(agent.model_status().unwrap(), state);
    assert_eq!(fs::read(&saved).unwrap(), before);
    let chat = Agent::new(
        SessionStore::open(dir.path().join("chat-choice.jsonl")).unwrap(),
        Box::new(backend(
            "http://127.0.0.1:9",
            None,
            OpenAiWireApi::ChatCompletions,
        )),
    );
    // ZE-224: Chat offers the model's catalogued levels.
    let choices = chat.reasoning_choices().unwrap();
    assert_eq!(choices.first(), Some(&None));
    assert!(choices.contains(&Some("high".into())) && !choices.contains(&Some("max".into())));
    let echo = Agent::with_echo(SessionStore::open(dir.path().join("echo-choice.jsonl")).unwrap());
    assert!(echo.reasoning_choices().unwrap().is_empty());
}
