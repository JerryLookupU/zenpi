#![cfg(feature = "remote")]

mod support;

use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use zenpi::runtime::{
    BackgroundRunner, CancellationToken, ConcurrencyError, JobOutcome, RuntimeConfig, RuntimeEvent,
    SubmitError,
};

type Runner = BackgroundRunner<
    usize,
    usize,
    String,
    Box<dyn Fn(usize, CancellationToken) -> Result<usize, String> + Send + Sync>,
>;

struct Gates {
    released: Mutex<Vec<bool>>,
    changed: Condvar,
    live: AtomicUsize,
    peak: AtomicUsize,
}

impl Gates {
    fn release(&self, index: usize) {
        self.released.lock().unwrap()[index] = true;
        self.changed.notify_all();
    }
}

fn fixture(limit: u16, pending: usize) -> (Runner, Arc<Gates>, mpsc::Receiver<usize>) {
    let gates = Arc::new(Gates {
        released: Mutex::new(vec![false; 128]),
        changed: Condvar::new(),
        live: AtomicUsize::new(0),
        peak: AtomicUsize::new(0),
    });
    let worker_gates = gates.clone();
    let (started, starts) = mpsc::channel();
    let job: Box<dyn Fn(usize, CancellationToken) -> Result<usize, String> + Send + Sync> =
        Box::new(move |index, token| {
            let live = worker_gates.live.fetch_add(1, Ordering::SeqCst) + 1;
            worker_gates.peak.fetch_max(live, Ordering::SeqCst);
            started.send(index).unwrap();
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut released = worker_gates.released.lock().unwrap();
            while !released[index] && !token.is_cancelled() {
                assert!(Instant::now() < deadline, "fixture was not released");
                released = worker_gates
                    .changed
                    .wait_timeout(released, Duration::from_millis(5))
                    .unwrap()
                    .0;
            }
            worker_gates.live.fetch_sub(1, Ordering::SeqCst);
            Ok(index)
        });
    (
        BackgroundRunner::spawn_with_concurrency(
            job,
            RuntimeConfig {
                max_pending: pending,
                command_capacity: 128,
                event_capacity: 512,
                ..RuntimeConfig::default()
            },
            limit,
        )
        .unwrap(),
        gates,
        starts,
    )
}

fn wait(runner: &Runner, predicate: impl Fn(&RuntimeEvent<usize, String>) -> bool) {
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        let event = runner
            .recv_timeout(until.saturating_duration_since(Instant::now()))
            .expect("runtime did not emit expected event");
        if predicate(&event) {
            return;
        }
    }
}

fn start(starts: &mpsc::Receiver<usize>) -> usize {
    starts.recv_timeout(Duration::from_secs(3)).unwrap()
}

fn apply(runner: &Runner, limit: u16) {
    runner
        .try_set_concurrency(limit)
        .unwrap()
        .recv_timeout(Duration::from_secs(3))
        .expect("quota command was not applied");
    assert_eq!(runner.applied_concurrency(), limit);
}

#[test]
fn every_quota_endpoint_runs_exactly_n_and_never_n_plus_one() {
    for limit in [1, 2, 7, 64] {
        let (runner, gates, starts) = fixture(limit, 2);
        let mut ids = Vec::new();
        for index in 0..usize::from(limit) + 1 {
            ids.push(runner.try_submit(index).unwrap());
        }
        let mut running: Vec<_> = (0..limit).map(|_| start(&starts)).collect();
        running.sort();
        assert_eq!(running, (0..usize::from(limit)).collect::<Vec<_>>());
        let queued = *ids.last().unwrap();
        wait(
            &runner,
            |event| matches!(event, RuntimeEvent::Queued { id, .. } if *id == queued),
        );
        assert!(starts.recv_timeout(Duration::from_millis(30)).is_err());
        assert_eq!(gates.live.load(Ordering::SeqCst), usize::from(limit));
        assert_eq!(gates.peak.load(Ordering::SeqCst), usize::from(limit));
        gates.release(0);
        assert_eq!(start(&starts), usize::from(limit));
        assert_eq!(gates.peak.load(Ordering::SeqCst), usize::from(limit));
        runner.shutdown_and_join().unwrap();
        assert_eq!(gates.live.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn lowering_quota_drains_existing_jobs_before_starting_fifo_followups() {
    let (runner, gates, starts) = fixture(3, 8);
    let ids: Vec<_> = (0..6)
        .map(|index| runner.try_submit(index).unwrap())
        .collect();
    for _ in 0..3 {
        start(&starts);
    }
    wait(
        &runner,
        |event| matches!(event, RuntimeEvent::Queued { id, depth: 3 } if *id == ids[5]),
    );
    apply(&runner, 1);
    gates.release(0);
    wait(
        &runner,
        |event| matches!(event, RuntimeEvent::Completed { id, outcome: JobOutcome::Succeeded(0) } if *id == ids[0]),
    );
    assert!(starts.recv_timeout(Duration::from_millis(30)).is_err());
    gates.release(1);
    wait(
        &runner,
        |event| matches!(event, RuntimeEvent::Completed { id, outcome: JobOutcome::Succeeded(1) } if *id == ids[1]),
    );
    assert!(starts.recv_timeout(Duration::from_millis(30)).is_err());
    gates.release(2);
    assert_eq!(start(&starts), 3);
    assert!(starts.recv_timeout(Duration::from_millis(30)).is_err());
    gates.release(3);
    assert_eq!(start(&starts), 4);
    gates.release(4);
    assert_eq!(start(&starts), 5);
    gates.release(5);
    wait(
        &runner,
        |event| matches!(event, RuntimeEvent::Completed { id, outcome: JobOutcome::Succeeded(5) } if *id == ids[5]),
    );
    runner.shutdown_and_join().unwrap();
}

#[test]
fn raising_quota_admits_oldest_queued_jobs_without_waiting_for_active_completion() {
    let (runner, gates, starts) = fixture(1, 8);
    let ids: Vec<_> = (0..5)
        .map(|index| runner.try_submit(index).unwrap())
        .collect();
    assert_eq!(start(&starts), 0);
    wait(
        &runner,
        |event| matches!(event, RuntimeEvent::Queued { id, .. } if *id == ids[4]),
    );
    apply(&runner, 3);
    let mut additional = vec![start(&starts), start(&starts)];
    additional.sort();
    assert_eq!(additional, vec![1, 2]);
    assert!(starts.recv_timeout(Duration::from_millis(30)).is_err());
    assert_eq!(gates.peak.load(Ordering::SeqCst), 3);
    runner.shutdown_and_join().unwrap();
}

#[test]
fn queued_and_active_cancellation_preserve_other_jobs_and_the_queue_bound() {
    let (runner, _gates, starts) = fixture(2, 1);
    let ids: Vec<_> = (0..4)
        .map(|index| runner.try_submit(index).unwrap())
        .collect();
    start(&starts);
    start(&starts);
    wait(
        &runner,
        |event| matches!(event, RuntimeEvent::Rejected { id, reason: SubmitError::QueueFull } if *id == ids[3]),
    );
    runner.try_cancel(ids[2]).unwrap();
    wait(
        &runner,
        |event| matches!(event, RuntimeEvent::Completed { id, outcome: JobOutcome::Cancelled } if *id == ids[2]),
    );
    let replacement = runner.try_submit(4).unwrap();
    wait(
        &runner,
        |event| matches!(event, RuntimeEvent::Queued { id, depth: 1 } if *id == replacement),
    );
    runner.try_cancel(ids[0]).unwrap();
    wait(
        &runner,
        |event| matches!(event, RuntimeEvent::Completed { id, outcome: JobOutcome::Cancelled } if *id == ids[0]),
    );
    assert_eq!(start(&starts), 4);
    assert!(starts.recv_timeout(Duration::from_millis(30)).is_err());
    let mut terminals = Vec::new();
    runner
        .shutdown_and_join_with_events(Duration::from_secs(1), |event| {
            if let RuntimeEvent::Completed { id, .. } = event {
                terminals.push(id);
            }
        })
        .unwrap();
    assert_eq!(terminals.len(), 2);
    assert!(terminals.contains(&ids[1]));
    assert!(terminals.contains(&replacement));
}

#[test]
fn invalid_limits_do_not_start_workers_or_change_the_applied_quota() {
    for limit in [0, 65, u16::MAX] {
        assert_eq!(
            BackgroundRunner::spawn_with_concurrency(
                |_: (), _| Ok::<_, String>(()),
                RuntimeConfig::default(),
                limit,
            )
            .unwrap_err(),
            ConcurrencyError::InvalidLimit
        );
    }
    let (runner, gates, starts) = fixture(1, 2);
    for limit in [0, 65, u16::MAX] {
        assert_eq!(
            runner.try_set_concurrency(limit).unwrap_err(),
            ConcurrencyError::InvalidLimit
        );
    }
    runner.try_submit(0).unwrap();
    runner.try_submit(1).unwrap();
    assert_eq!(start(&starts), 0);
    assert!(starts.recv_timeout(Duration::from_millis(30)).is_err());
    assert_eq!(gates.peak.load(Ordering::SeqCst), 1);
    runner.shutdown_and_join().unwrap();
}

#[test]
fn quota_receipt_is_not_resolved_while_the_scheduler_is_blocked_on_events() {
    let runner = BackgroundRunner::spawn(
        |_: (), token| {
            while !token.is_cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok::<_, String>(())
        },
        RuntimeConfig {
            event_capacity: 1,
            ..RuntimeConfig::default()
        },
    );
    let id = runner.try_submit(()).unwrap();
    // Accepted fills the channel; Started must precede quota application.
    let receipt = runner.try_set_concurrency(2).unwrap();
    assert!(receipt.recv_timeout(Duration::from_millis(30)).is_err());
    assert_eq!(runner.applied_concurrency(), 1);
    assert!(matches!(
        runner.next_event().unwrap(),
        RuntimeEvent::Accepted { .. }
    ));
    assert!(
        matches!(runner.next_event().unwrap(), RuntimeEvent::Started { id: actual } if actual == id)
    );
    receipt.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(runner.applied_concurrency(), 2);
    runner.shutdown_and_join().unwrap();
}

#[test]
fn shutdown_rejects_quota_changes_instead_of_acknowledging_unapplied_values() {
    let (runner, _gates, starts) = fixture(2, 4);
    runner.try_submit(0).unwrap();
    runner.try_submit(1).unwrap();
    start(&starts);
    start(&starts);
    runner.try_shutdown().unwrap();
    let receipt = runner.try_set_concurrency(64);
    let mut closed = 0;
    runner
        .shutdown_and_join_with_events(Duration::from_secs(1), |event| {
            if matches!(event, RuntimeEvent::Closed) {
                closed += 1;
            }
        })
        .unwrap();
    assert_eq!(closed, 1);
    match receipt {
        Ok(receipt) => assert_eq!(receipt.try_recv(), Err(mpsc::TryRecvError::Disconnected)),
        Err(error) => assert_eq!(error, ConcurrencyError::Submit(SubmitError::Closed)),
    }
}

#[test]
fn discarding_a_quota_receipt_does_not_lose_authoritative_application_state() {
    let (runner, _gates, _starts) = fixture(3, 4);
    assert_eq!(runner.applied_concurrency(), 3);
    drop(runner.try_set_concurrency(1).unwrap());
    let deadline = Instant::now() + Duration::from_secs(3);
    while runner.applied_concurrency() != 1 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    for limit in [0, 65] {
        assert_eq!(
            runner.try_set_concurrency(limit).unwrap_err(),
            ConcurrencyError::InvalidLimit
        );
        assert_eq!(runner.applied_concurrency(), 1);
    }
    runner.shutdown_and_join().unwrap();
}

#[test]
fn parallel_shutdown_has_one_deadline_and_one_terminal_per_active_and_queued_job() {
    let (entered, entries) = mpsc::channel();
    let (finished, finishes) = mpsc::channel();
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let worker_release = release.clone();
    let runner = BackgroundRunner::spawn_with_concurrency(
        move |index: usize, _| {
            entered.send(index).unwrap();
            let (lock, changed) = &*worker_release;
            let until = Instant::now() + Duration::from_secs(10);
            let mut released = lock.lock().unwrap();
            while !*released {
                assert!(Instant::now() < until);
                released = changed
                    .wait_timeout(released, Duration::from_millis(5))
                    .unwrap()
                    .0;
            }
            finished.send(index).unwrap();
            Ok::<_, String>(index)
        },
        RuntimeConfig::default(),
        8,
    )
    .unwrap();
    let ids: Vec<_> = (0..10)
        .map(|index| runner.try_submit(index).unwrap())
        .collect();
    for _ in 0..8 {
        start(&entries);
    }
    loop {
        if matches!(runner.recv_timeout(Duration::from_secs(3)).unwrap(), RuntimeEvent::Queued { id, depth: 2 } if id == ids[9])
        {
            break;
        }
    }
    let began = Instant::now();
    let mut terminals = Vec::new();
    let mut closed = 0;
    runner
        .shutdown_and_join_with_events(Duration::from_millis(30), |event| match event {
            RuntimeEvent::Completed {
                id,
                outcome: JobOutcome::Cancelled,
            } => terminals.push(id),
            RuntimeEvent::Closed => closed += 1,
            _ => {}
        })
        .unwrap();
    let elapsed = began.elapsed();
    {
        let (lock, changed) = &*release;
        *lock.lock().unwrap() = true;
        changed.notify_all();
    }
    for _ in 0..8 {
        start(&finishes);
    }
    assert!(
        entries.try_recv().is_err(),
        "queued work must never run during shutdown"
    );
    assert!(
        elapsed < Duration::from_millis(200),
        "grace was applied per lane: {elapsed:?}"
    );
    assert_eq!(closed, 1);
    assert_eq!(terminals, ids);
}

#[test]
fn panicking_parallel_job_releases_its_slot_without_losing_other_results() {
    let runner = BackgroundRunner::spawn_with_concurrency(
        |index: usize, _| {
            if index == 0 {
                panic!("controlled worker panic");
            }
            Ok::<_, String>(index)
        },
        RuntimeConfig::default(),
        2,
    )
    .unwrap();
    let ids: Vec<_> = (0..5)
        .map(|index| runner.try_submit(index).unwrap())
        .collect();
    let mut terminals = Vec::new();
    while terminals.len() < 5 {
        if let RuntimeEvent::Completed { id, outcome } =
            runner.recv_timeout(Duration::from_secs(3)).unwrap()
        {
            let index = ids.iter().position(|candidate| *candidate == id).unwrap();
            match outcome {
                JobOutcome::Panicked => assert_eq!(index, 0),
                JobOutcome::Succeeded(value) => assert_eq!(value, index),
                _ => panic!("unexpected outcome"),
            }
            assert!(!terminals.contains(&id));
            terminals.push(id);
        }
    }
    runner.shutdown_and_join().unwrap();
}

#[test]
fn shutdown_signals_all_parallel_jobs_before_a_saturated_event_channel_blocks() {
    let (entered, entries) = mpsc::channel();
    let (cancelled, cancellations) = mpsc::channel();
    let runner = BackgroundRunner::spawn_with_concurrency(
        move |index: usize, token| {
            entered.send(index).unwrap();
            let until = Instant::now() + Duration::from_secs(3);
            while !token.is_cancelled() {
                assert!(Instant::now() < until);
                std::thread::sleep(Duration::from_millis(1));
            }
            cancelled.send(index).unwrap();
            Ok::<_, String>(index)
        },
        RuntimeConfig {
            event_capacity: 1,
            ..RuntimeConfig::default()
        },
        3,
    )
    .unwrap();
    for index in 0..3 {
        let id = runner.try_submit(index).unwrap();
        loop {
            if matches!(runner.recv_timeout(Duration::from_secs(3)).unwrap(), RuntimeEvent::Started { id: actual } if actual == id)
            {
                break;
            }
        }
        start(&entries);
    }
    runner.try_shutdown().unwrap();
    let mut signalled: Vec<_> = (0..3).map(|_| start(&cancellations)).collect();
    signalled.sort();
    assert_eq!(signalled, vec![0, 1, 2]);
    let mut terminals = 0;
    runner
        .shutdown_and_join_with_events(Duration::from_secs(1), |event| {
            if matches!(
                event,
                RuntimeEvent::Completed {
                    outcome: JobOutcome::Cancelled,
                    ..
                }
            ) {
                terminals += 1;
            }
        })
        .unwrap();
    assert_eq!(terminals, 3);
}

#[test]
fn scheduler_executes_independent_agents_and_real_workspace_tools_without_shared_journals() {
    use zenpi::{
        approval::{ApprovalMode, ApprovalPolicy},
        backend::{Backend, BackendError, Completion, CompletionRequest},
        core::Agent,
        session::SessionStore,
        tools::{SideEffectPolicy, ToolCall, ToolContext, ToolRegistry},
    };
    struct ToolBackend {
        calls: AtomicUsize,
        entered: mpsc::Sender<()>,
        release: Arc<(Mutex<bool>, Condvar)>,
        content: String,
    }
    impl Backend for ToolBackend {
        crate::support::controlled_local_backend!();
        fn name(&self) -> &str {
            "concurrent-tool-fixture"
        }
        fn complete(&self, _: CompletionRequest<'_>) -> Result<Completion, BackendError> {
            if self.calls.fetch_add(1, Ordering::SeqCst) > 0 {
                return Ok(Completion::text("workspace written"));
            }
            self.entered.send(()).unwrap();
            let (lock, changed) = &*self.release;
            let until = Instant::now() + Duration::from_secs(10);
            let mut released = lock.lock().unwrap();
            while !*released {
                assert!(Instant::now() < until);
                released = changed
                    .wait_timeout(released, Duration::from_millis(5))
                    .unwrap()
                    .0;
            }
            Ok(Completion {
                tool_calls: vec![ToolCall {
                    id: "write-owned-workspace".into(),
                    name: "write_file".into(),
                    arguments: serde_json::json!({"path":"result.txt", "content":self.content}),
                }],
                ..Completion::default()
            })
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let (entered, entries) = mpsc::channel();
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let owners: Vec<_> = (0..3)
        .map(|index| {
            let root = temp.path().join(format!("workspace-{index}"));
            std::fs::create_dir(&root).unwrap();
            let root = root.canonicalize().unwrap();
            let mut agent = Agent::new(
                SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
                Box::new(ToolBackend {
                    calls: AtomicUsize::new(0),
                    entered: entered.clone(),
                    release: release.clone(),
                    content: format!("owned-result-{index}"),
                }),
            );
            agent.set_tools(
                ToolRegistry::with_all_builtins().unwrap(),
                ToolContext::new(&root).unwrap(),
                SideEffectPolicy::all_builtins(),
            );
            agent.set_approval_policy(ApprovalPolicy {
                mode: ApprovalMode::Never,
                ..Default::default()
            });
            Arc::new(Mutex::new(agent))
        })
        .collect();
    let runner = BackgroundRunner::spawn_with_concurrency(
        |owner: Arc<Mutex<Agent>>, token| {
            let result = owner.lock().unwrap().process_with_cancel(
                zenpi::core::TurnInputRequest::new("write owned workspace"),
                || token.is_cancelled(),
            );
            if result.is_ok() {
                token.mark_completed();
            }
            result.map(|_| ()).map_err(|error| error.to_string())
        },
        RuntimeConfig::default(),
        2,
    )
    .unwrap();
    let ids: Vec<_> = owners
        .iter()
        .map(|owner| runner.try_submit(owner.clone()).unwrap())
        .collect();
    entries.recv_timeout(Duration::from_secs(3)).unwrap();
    entries.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(entries.recv_timeout(Duration::from_millis(30)).is_err());
    {
        let (lock, changed) = &*release;
        *lock.lock().unwrap() = true;
        changed.notify_all();
    }
    let mut results = Vec::new();
    while results.len() < 3 {
        if let RuntimeEvent::Completed { id, outcome } =
            runner.recv_timeout(Duration::from_secs(3)).unwrap()
        {
            assert!(matches!(outcome, JobOutcome::Succeeded(())), "{outcome:?}");
            assert!(!results.contains(&id));
            results.push(id);
        }
    }
    assert!(ids.iter().all(|id| results.contains(id)));
    runner.shutdown_and_join().unwrap();
    for (index, owner) in owners.iter().enumerate() {
        let agent = owner.lock().unwrap();
        let root = temp.path().join(format!("workspace-{index}"));
        assert_eq!(
            std::fs::read_to_string(root.join("result.txt")).unwrap(),
            format!("owned-result-{index}")
        );
        let journal = std::fs::read_to_string(agent.session().path()).unwrap();
        assert!(journal.contains("write-owned-workspace"));
        assert!(journal.contains(&format!("owned-result-{index}")));
        for other in 0..3 {
            if other != index {
                assert!(!journal.contains(&format!("owned-result-{other}")));
            }
        }
        let session_id = agent.session().session_id().to_owned();
        drop(agent);
        let restored = SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap();
        assert_eq!(restored.session_id(), session_id);
    }
}
