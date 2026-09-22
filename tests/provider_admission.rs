use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use zenpi::backend::{
    Backend, BackendError, Completion, CompletionRequest, HttpRequestKind, OpenAiCompatibleBackend,
    ProviderEvent, RequestControl, RequestPurpose, RequestScope,
};
use zenpi::core::{Agent, Turn, TurnInputRequest, TurnRole, WorkerExecutionBinding};
use zenpi::governance::{BudgetLedger, ResourceLimits, WorkerBudgetLedger};
use zenpi::session::SessionStore;
use zenpi::tools::{
    BlueprintGate, BlueprintLease, BlueprintPolicySpec, BlueprintRevocation, SideEffectPolicy,
    ToolContext, ToolRegistry,
};

struct Peer {
    url: String,
    seen: Arc<AtomicUsize>,
    stopped: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Peer {
    fn new(statuses: Vec<u16>, on_response: impl Fn(usize) + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let seen = Arc::new(AtomicUsize::new(0));
        let stopped = Arc::new(AtomicBool::new(false));
        let count = seen.clone();
        let stop = stopped.clone();
        let thread = thread::spawn(move || {
            while !stop.load(Ordering::Acquire) {
                let (mut stream, _) = match listener.accept() {
                    Ok(peer) => peer,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("local peer failed: {error}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                let mut bytes = [0; 4096];
                loop {
                    let n = stream.read(&mut bytes).unwrap();
                    assert!(n > 0);
                    request.extend_from_slice(&bytes[..n]);
                    assert!(request.len() < 128 * 1024);
                    if let Some(end) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                        let header = String::from_utf8_lossy(&request[..end]);
                        let length = header
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().unwrap())
                            })
                            .unwrap();
                        if request.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                let index = count.fetch_add(1, Ordering::AcqRel);
                on_response(index);
                let status = statuses.get(index).copied().unwrap_or(200);
                let body = if status == 200 {
                    r#"{"id":"synthetic","model":"mock-model","choices":[{"message":{"content":"done"},"finish_reason":"stop"}]}"#
                } else {
                    "{}"
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} Synthetic\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        Self {
            url,
            seen,
            stopped,
            thread: Some(thread),
        }
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        if let Err(error) = self.thread.take().unwrap().join()
            && !thread::panicking()
        {
            std::panic::resume_unwind(error);
        }
    }
}

fn agent(path: &Path, peer: &Peer, network: u64) -> Agent {
    let backend =
        OpenAiCompatibleBackend::new(&peer.url, Some("synthetic-key".into()), "mock-model")
            .unwrap()
            .with_max_retries(3)
            .unwrap();
    let mut agent = Agent::new(SessionStore::open(path).unwrap(), Box::new(backend));
    agent
        .set_resource_limits(ResourceLimits {
            max_network_requests: network,
            ..Default::default()
        })
        .unwrap();
    agent
}

fn requests(agent: &Agent) -> u64 {
    BudgetLedger::restore(agent.session(), ResourceLimits::default())
        .unwrap()
        .usage()
        .network_requests
}

#[test]
fn retry_cannot_bypass_the_original_request_budget() {
    let peer = Peer::new(vec![503, 200], |_| {});
    let dir = tempdir().unwrap();
    let mut agent = agent(&dir.path().join("session.jsonl"), &peer, 1);
    let error = agent.process(TurnInputRequest::new("hello")).unwrap_err();
    assert_eq!(error.code(), "resource_budget_exceeded");
    assert_eq!(peer.seen.load(Ordering::Acquire), 1);
    assert_eq!(requests(&agent), 1);
}

#[test]
fn local_completion_does_not_need_or_consume_an_http_budget() {
    let dir = tempdir().unwrap();
    let mut agent = Agent::with_echo(SessionStore::open(dir.path().join("local.jsonl")).unwrap());
    agent
        .set_resource_limits(ResourceLimits {
            max_network_requests: 0,
            ..Default::default()
        })
        .unwrap();
    agent.process(TurnInputRequest::new("local only")).unwrap();
    assert_eq!(requests(&agent), 0);
}

#[test]
fn each_retry_is_durable_and_resume_cannot_reset_the_budget() {
    let peer = Peer::new(vec![503, 200], |_| {});
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let mut first = agent(&path, &peer, 2);
    first.process(TurnInputRequest::new("hello")).unwrap();
    assert_eq!(requests(&first), 2);
    assert_eq!(peer.seen.load(Ordering::Acquire), 2);
    drop(first);
    let mut resumed = agent(&path, &peer, 2);
    assert_eq!(
        resumed
            .process(TurnInputRequest::new("again"))
            .unwrap_err()
            .code(),
        "resource_budget_exceeded"
    );
    assert_eq!(peer.seen.load(Ordering::Acquire), 2);
}

struct ProbeBackend {
    tamper: bool,
    scopes: Arc<Mutex<Vec<RequestScope>>>,
}

impl Backend for ProbeBackend {
    fn complete(&self, _: CompletionRequest<'_>) -> Result<Completion, BackendError> {
        panic!("Core must supply request control");
    }

    fn complete_with_request_control(
        &self,
        _: CompletionRequest<'_>,
        control: &mut RequestControl<'_>,
        _: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
    ) -> Result<Completion, BackendError> {
        if self.tamper {
            control.scope.session_id.push_str("-other");
        }
        for kind in [HttpRequestKind::AuthRefresh, HttpRequestKind::Inference] {
            control.before_send(kind)?;
            self.scopes.lock().unwrap().push(control.scope.clone());
        }
        Ok(Completion::text("done"))
    }
}

#[test]
fn owner_rejects_scope_replacement_before_admission() {
    let dir = tempdir().unwrap();
    let scopes = Arc::new(Mutex::new(Vec::new()));
    let mut agent = Agent::new(
        SessionStore::open(dir.path().join("scope.jsonl")).unwrap(),
        Box::new(ProbeBackend {
            tamper: true,
            scopes: scopes.clone(),
        }),
    );
    agent
        .set_resource_limits(ResourceLimits::default())
        .unwrap();
    assert!(agent.process(TurnInputRequest::new("hello")).is_err());
    assert!(scopes.lock().unwrap().is_empty());
    assert_eq!(requests(&agent), 0);
}

#[test]
fn refresh_and_inference_share_the_same_budget_and_scope() {
    for limit in [1, 2] {
        let dir = tempdir().unwrap();
        let scopes = Arc::new(Mutex::new(Vec::new()));
        let mut agent = Agent::new(
            SessionStore::open(dir.path().join("scope.jsonl")).unwrap(),
            Box::new(ProbeBackend {
                tamper: false,
                scopes: scopes.clone(),
            }),
        );
        agent
            .set_resource_limits(ResourceLimits {
                max_network_requests: limit,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            agent.process(TurnInputRequest::new("hello")).is_ok(),
            limit == 2
        );
        assert_eq!(requests(&agent), limit);
        let scopes = scopes.lock().unwrap();
        assert_eq!(scopes.len(), limit as usize);
        if scopes.len() == 2 {
            assert_eq!(scopes[0], scopes[1]);
        }
        assert_eq!(scopes[0].session_id, agent.session().session_id());
    }
}

fn admit_worker(agent: &mut Agent, root: &Path, network: u64) -> BlueprintRevocation {
    let context = ToolContext::new(root).unwrap();
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        context.clone(),
        SideEffectPolicy::read_only(),
    );
    agent
        .set_worker_budget_limits(ResourceLimits::default())
        .unwrap();
    let policy = BlueprintPolicySpec {
        blueprint_digest: "a".repeat(64),
        goal_digest: "b".repeat(64),
        item_id: "PA04".into(),
        allowed_tools: ["read_file".into()].into_iter().collect(),
        readable_paths: [".".into()].into_iter().collect(),
        writable_paths: Default::default(),
        denied_paths: Default::default(),
        protected_paths: Default::default(),
        allowed_commands: Default::default(),
        denied_commands: Default::default(),
        network_hosts: Default::default(),
        max_actions: 2,
        max_command_timeout_ms: 1000,
        max_command_output_bytes: 1024,
    };
    let now = zenpi::session::unix_time_ms();
    let lease = BlueprintLease {
        lease_id: "provider-lease".into(),
        issued_at_ms: now.saturating_sub(1),
        expires_at_ms: now + 60_000,
    };
    let (gate, _) = BlueprintGate::compile(&context, policy.clone(), lease.clone(), now).unwrap();
    let binding = WorkerExecutionBinding {
        blueprint_id: "blueprint".into(),
        blueprint_sha256: policy.blueprint_digest.clone(),
        goal_id: "goal".into(),
        item_id: policy.item_id.clone(),
        lease_id: lease.lease_id.clone(),
        policy_digest: gate.evidence().policy_digest,
        expires_at_ms: lease.expires_at_ms,
    };
    agent
        .admit_blueprint_worker(
            policy,
            lease,
            binding,
            ResourceLimits {
                max_network_requests: network,
                ..Default::default()
            },
            Vec::new(),
            now,
        )
        .unwrap()
}

#[test]
fn worker_lease_limits_each_physical_http_even_with_larger_agent_budget() {
    let peer = Peer::new(vec![503, 200], |_| {});
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let mut agent = agent(&path, &peer, 10);
    let _revocation = admit_worker(&mut agent, dir.path(), 1);
    assert!(agent.process(TurnInputRequest::new("hello")).is_err());
    assert_eq!(peer.seen.load(Ordering::Acquire), 1);
    assert_eq!(requests(&agent), 1);
    drop(agent);
    let mut session = SessionStore::open(path).unwrap();
    let worker = WorkerBudgetLedger::restore_existing(&mut session).unwrap();
    assert_eq!(
        worker
            .committed_usage(Some("provider-lease"))
            .unwrap()
            .network_requests,
        1
    );
}

#[test]
fn revoking_worker_gate_between_attempts_stops_retry() {
    let revoked: Arc<Mutex<Option<BlueprintRevocation>>> = Arc::new(Mutex::new(None));
    let handle = revoked.clone();
    let peer = Peer::new(vec![503, 200], move |_| {
        handle.lock().unwrap().as_ref().unwrap().revoke()
    });
    let dir = tempdir().unwrap();
    let mut agent = agent(&dir.path().join("session.jsonl"), &peer, 10);
    *revoked.lock().unwrap() = Some(admit_worker(&mut agent, dir.path(), 10));
    assert!(agent.process(TurnInputRequest::new("hello")).is_err());
    assert_eq!(peer.seen.load(Ordering::Acquire), 1);
    assert_eq!(requests(&agent), 1);
}

fn seed_compaction(agent: &mut Agent) {
    agent.set_context_budget(zenpi::context::ContextBudget {
        max_tokens: 4000,
        reserved_output_tokens: 1000,
    });
    for index in 0..10 {
        agent
            .session_mut()
            .append_turn(Turn::new(
                format!("old-{index}"),
                TurnRole::User,
                "old history ".repeat(110),
            ))
            .unwrap();
    }
}

#[test]
fn summary_scope_rejection_leaves_no_reservation_or_pending_operation() {
    let peer = Peer::new(vec![200], |_| {});
    let dir = tempdir().unwrap();
    let mut agent = agent(&dir.path().join("summary.jsonl"), &peer, 10);
    let revocation = admit_worker(&mut agent, dir.path(), 10);
    seed_compaction(&mut agent);
    let usage = agent
        .session()
        .events()
        .iter()
        .rev()
        .find(|event| event["type"] == "resource_usage")
        .cloned();
    revocation.revoke();
    let error = agent.compact_context().unwrap_err();
    assert!(error.to_string().contains("lease_revoked"), "{error}");
    assert_eq!(peer.seen.load(Ordering::Acquire), 0);
    assert_eq!(
        agent
            .session()
            .events()
            .iter()
            .rev()
            .find(|event| event["type"] == "resource_usage")
            .cloned(),
        usage
    );
    assert!(agent.operation_recovery().is_empty());
    assert!(agent.session().interrupted_operations().is_empty());
}

#[test]
fn semantic_compaction_keeps_the_original_worker_and_shared_request_budget() {
    let dir = tempdir().unwrap();
    let scopes = Arc::new(Mutex::new(Vec::new()));
    let mut agent = Agent::new(
        SessionStore::open(dir.path().join("summary.jsonl")).unwrap(),
        Box::new(ProbeBackend {
            tamper: false,
            scopes: scopes.clone(),
        }),
    );
    agent
        .set_resource_limits(ResourceLimits::default())
        .unwrap();
    let _revocation = admit_worker(&mut agent, dir.path(), 2);
    seed_compaction(&mut agent);
    // The probe intentionally returns invalid summary JSON after both sends.
    assert!(agent.compact_context().is_err());
    let scopes = scopes.lock().unwrap();
    assert_eq!(scopes.len(), 2);
    assert_eq!(scopes[0], scopes[1]);
    assert_eq!(scopes[0].purpose, RequestPurpose::SemanticCompaction);
    assert_eq!(scopes[0].lease_id.as_deref(), Some("provider-lease"));
    assert!(scopes[0].policy_digest.is_some());
    assert_eq!(requests(&agent), 2);
    assert!(agent.session().interrupted_operations().is_empty());
}

#[test]
fn owner_wall_deadline_interrupts_pending_http_without_another_send() {
    let peer = Peer::new(vec![200], |_| thread::sleep(Duration::from_millis(2500)));
    let dir = tempdir().unwrap();
    let mut agent = agent(&dir.path().join("deadline.jsonl"), &peer, 10);
    agent
        .set_resource_limits(ResourceLimits {
            // Leave time for durable admission under concurrent fsync load.
            max_wall_ms: 1000,
            ..Default::default()
        })
        .unwrap();
    let start = Instant::now();
    let error = agent.process(TurnInputRequest::new("hello")).unwrap_err();
    assert_eq!(error.code(), "backend_deadline_exceeded", "{error}");
    assert!(start.elapsed() < Duration::from_millis(2200));
    assert_eq!(peer.seen.load(Ordering::Acquire), 1);
    assert_eq!(requests(&agent), 1);
}

#[test]
fn worker_revocation_interrupts_pending_http_not_just_the_next_retry() {
    let revoked: Arc<Mutex<Option<BlueprintRevocation>>> = Arc::new(Mutex::new(None));
    let handle = revoked.clone();
    let peer = Peer::new(vec![200], move |_| {
        handle.lock().unwrap().as_ref().unwrap().revoke();
        thread::sleep(Duration::from_millis(800));
    });
    let dir = tempdir().unwrap();
    let mut agent = agent(&dir.path().join("revoked.jsonl"), &peer, 10);
    *revoked.lock().unwrap() = Some(admit_worker(&mut agent, dir.path(), 10));
    let start = Instant::now();
    let error = agent.process(TurnInputRequest::new("hello")).unwrap_err();
    assert_eq!(error.code(), "backend_cancelled", "{error}");
    assert!(start.elapsed() < Duration::from_millis(700));
    assert_eq!(peer.seen.load(Ordering::Acquire), 1);
    assert_eq!(requests(&agent), 1);
}

struct ToolContinuationBackend(Arc<Mutex<Vec<RequestScope>>>);

impl Backend for ToolContinuationBackend {
    fn complete(&self, _: CompletionRequest<'_>) -> Result<Completion, BackendError> {
        panic!("Core must supply request control");
    }

    fn complete_with_request_control(
        &self,
        request: CompletionRequest<'_>,
        control: &mut RequestControl<'_>,
        _: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
    ) -> Result<Completion, BackendError> {
        // A synthetic send tests Core's scope contract, not a real HTTP call.
        control.before_send(HttpRequestKind::Inference)?;
        self.0.lock().unwrap().push(control.scope.clone());
        let mut completion = Completion::text("done");
        if request.turns.last().unwrap().role != TurnRole::Tool {
            completion.tool_calls.push(zenpi::tools::ToolCall {
                id: "read-synthetic".into(),
                name: "read_file".into(),
                arguments: serde_json::json!({"path":"input.txt"}),
            });
        }
        Ok(completion)
    }
}

#[test]
fn tool_continuation_keeps_owner_and_operation_but_changes_purpose() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("input.txt"), "synthetic local input").unwrap();
    let scopes = Arc::new(Mutex::new(Vec::new()));
    let mut agent = Agent::new(
        SessionStore::open(dir.path().join("continuation.jsonl")).unwrap(),
        Box::new(ToolContinuationBackend(scopes.clone())),
    );
    agent
        .set_resource_limits(ResourceLimits::default())
        .unwrap();
    let _revocation = admit_worker(&mut agent, dir.path(), 2);
    agent
        .process(TurnInputRequest::new("read the input"))
        .unwrap();
    let scopes = scopes.lock().unwrap();
    assert_eq!(scopes.len(), 2);
    assert_eq!(scopes[0].purpose, RequestPurpose::Turn);
    assert_eq!(scopes[1].purpose, RequestPurpose::ToolContinuation);
    let mut expected = scopes[0].clone();
    expected.purpose = RequestPurpose::ToolContinuation;
    assert_eq!(scopes[1], expected);
    assert_eq!(requests(&agent), 2);
    assert!(
        agent
            .history()
            .iter()
            .any(|turn| turn.role == TurnRole::Tool)
    );
    drop(scopes);
    let admission = agent.worker_admission_operation_id().unwrap().to_owned();
    agent
        .settle_blueprint_worker(
            &admission,
            zenpi::governance::ResourceUsage {
                processes: 1,
                ..Default::default()
            },
            zenpi::governance::BudgetCompletion::Completed,
            zenpi::session::unix_time_ms(),
        )
        .unwrap();
    assert!(agent.worker_admission_operation_id().is_none());
    assert!(
        agent
            .process(TurnInputRequest::new("after settlement"))
            .is_err()
    );
    assert_eq!(requests(&agent), 2);
}
