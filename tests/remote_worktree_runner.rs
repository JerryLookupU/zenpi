use std::{
    path::Path,
    sync::{Arc, Condvar, Mutex, mpsc},
    time::{Duration, Instant},
};
use zenpi::{
    core::Agent,
    project_workspace::ProjectOwnerPool,
    remote::{worker::WorkerOwner, worktree_runner::WorktreeRunner},
    runtime::{JobOutcome, RuntimeConfig, RuntimeEvent},
    session::SessionStore,
};

fn pool(root: &Path, quota: u16) -> (ProjectOwnerPool, String, String) {
    let owner = Arc::new(Mutex::new(Agent::with_echo(
        SessionStore::open_in_workspace(root.join("session.jsonl"), root).unwrap(),
    )));
    let mut pool = ProjectOwnerPool::new(owner).unwrap();
    pool.restore_checkpoint().unwrap();
    let project = pool.active_context().project_id.clone();
    let state = pool.subtab_state(&project).unwrap();
    let tab = state.active_id.clone();
    if quota != 1 {
        pool.commit_subtabs(
            &project,
            state.revision,
            state.set_concurrency(&tab, quota).unwrap(),
        )
        .unwrap();
    }
    (pool, project, tab)
}

#[test]
fn spawn_refuses_partial_capacity_without_preparing_or_leaking_a_slot() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, main) = pool(&root, 63);
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    let state = pool.subtab_state(&project).unwrap();
    let next = state
        .add_directory(&project, &root, &child, "child")
        .unwrap();
    let tab = next.active_id.clone();
    pool.commit_subtabs(&project, state.revision, next).unwrap();
    let state = pool.subtab_state(&project).unwrap();
    pool.commit_subtabs(
        &project,
        state.revision,
        state.set_concurrency(&tab, 2).unwrap(),
    )
    .unwrap();
    for slot in 0..63 {
        pool.worker_owner(&project, &main, slot).unwrap();
    }
    let journals = std::fs::read_dir(root.join(".zenpi/workers"))
        .unwrap()
        .count();
    assert!(
        WorktreeRunner::spawn(
            &mut pool,
            &project,
            &tab,
            RuntimeConfig::default(),
            |_, _: (), _| Ok::<_, String>(())
        )
        .is_err()
    );
    assert_eq!(pool.worker_handles(&project).len(), 63);
    assert_eq!(
        std::fs::read_dir(root.join(".zenpi/workers"))
            .unwrap()
            .count(),
        journals
    );
    let worker = pool.worker_owner(&project, &tab, 0).unwrap();
    assert_eq!(pool.worker_handles(&project).len(), 64);
    drop(worker);
    pool.retire_worker_owners(&project).unwrap();
    let runner = WorktreeRunner::spawn(
        &mut pool,
        &project,
        &tab,
        RuntimeConfig::default(),
        |_, _: (), _| Ok::<_, String>(()),
    )
    .unwrap();
    assert_eq!(runner.quota(), 2);
    runner.shutdown_and_join().unwrap();
    pool.retire_worker_owners(&project).unwrap();
}

#[test]
fn durable_quota_admits_n_independent_slots_and_queues_n_plus_one() {
    check_independent_quota(3);
}

#[test]
fn maximum_quota_runs_sixty_four_independent_writers_and_queues_the_sixty_fifth() {
    check_independent_quota(64);
}

fn check_independent_quota(quota: u16) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, tab) = pool(&root, quota);
    let master = pool.owner(&project).unwrap();
    let master_before = std::fs::read(master.lock().unwrap().session().path()).unwrap();
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let worker_release = release.clone();
    let (entered, entries) = mpsc::channel();
    let runner = WorktreeRunner::spawn(
        &mut pool,
        &project,
        &tab,
        RuntimeConfig {
            event_capacity: 512,
            max_pending: 65,
            command_capacity: 128,
            ..RuntimeConfig::default()
        },
        move |worker, input: usize, token| {
            let mut agent = worker.agent().lock().unwrap();
            entered
                .send((input, agent.session().session_id().to_owned()))
                .unwrap();
            let (lock, changed) = &*worker_release;
            let until = Instant::now() + Duration::from_secs(5);
            let mut ready = lock.lock().unwrap();
            while !*ready && !token.is_cancelled() {
                assert!(Instant::now() < until);
                ready = changed
                    .wait_timeout(ready, Duration::from_millis(5))
                    .unwrap()
                    .0;
            }
            drop(ready);
            agent
                .process_with_cancel(
                    zenpi::core::TurnInputRequest::new(format!("job-{input}")),
                    || token.is_cancelled(),
                )
                .map(|_| input)
                .map_err(|error| error.to_string())
        },
    )
    .unwrap();
    assert_eq!(runner.quota(), quota);
    let count = usize::from(quota);
    let ids: Vec<_> = (0..=count)
        .map(|index| runner.try_submit(index).unwrap())
        .collect();
    let first: Vec<_> = (0..count)
        .map(|_| entries.recv_timeout(Duration::from_secs(3)).unwrap())
        .collect();
    assert_eq!(
        first
            .iter()
            .map(|(_, id)| id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        count
    );
    loop {
        if matches!(runner.recv_timeout(Duration::from_secs(3)).unwrap(), RuntimeEvent::Queued { id, .. } if id == ids[count])
        {
            break;
        }
    }
    assert!(entries.recv_timeout(Duration::from_millis(30)).is_err());
    *release.0.lock().unwrap() = true;
    release.1.notify_all();
    let mut completed = 0;
    while completed <= count {
        if let RuntimeEvent::Completed { outcome, .. } =
            runner.recv_timeout(Duration::from_secs(3)).unwrap()
        {
            assert!(matches!(outcome, JobOutcome::Succeeded(_)));
            completed += 1;
        }
    }
    runner.shutdown_and_join().unwrap();
    assert_eq!(
        std::fs::read(master.lock().unwrap().session().path()).unwrap(),
        master_before
    );
    let all_text = pool
        .worker_handles(&project)
        .iter()
        .map(|worker| {
            std::fs::read_to_string(worker.agent().lock().unwrap().session().path()).unwrap()
        })
        .collect::<Vec<_>>();
    for index in 0..=count {
        assert_eq!(
            all_text
                .iter()
                .filter(|text| text.contains(&format!("job-{index}\"")))
                .count(),
            1
        );
    }
    pool.retire_worker_owners(&project).unwrap();
}

#[test]
fn live_limit_drains_then_raises_using_only_exclusively_leased_worker_writers() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, tab) = pool(&root, 3);
    let initial = pool.subtab_state(&project).unwrap();
    let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
    let releases = Arc::new((Mutex::new(vec![false; 7]), Condvar::new()));
    let gates = releases.clone();
    let (entered, starts) = mpsc::channel();
    let runner = WorktreeRunner::spawn(
        &mut pool,
        &project,
        &tab,
        RuntimeConfig::default(),
        move |worker, input: usize, token| {
            let mut agent = worker.agent().lock().unwrap();
            entered
                .send((input, agent.session().session_id().to_owned()))
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut ready = gates.0.lock().unwrap();
            while !ready[input] && !token.is_cancelled() {
                assert!(Instant::now() < deadline);
                ready = gates
                    .1
                    .wait_timeout(ready, Duration::from_millis(5))
                    .unwrap()
                    .0;
            }
            drop(ready);
            agent
                .process_with_cancel(
                    zenpi::core::TurnInputRequest::new(format!("resized-job-{input}")),
                    || token.is_cancelled(),
                )
                .map(|_| input)
                .map_err(|error| error.to_string())
        },
    )
    .unwrap();
    let ids: Vec<_> = (0..7)
        .map(|index| runner.try_submit(index).unwrap())
        .collect();
    let first: Vec<_> = (0..3)
        .map(|_| starts.recv_timeout(Duration::from_secs(3)).unwrap())
        .collect();
    let sessions: std::collections::BTreeSet<_> = first.iter().map(|(_, id)| id.clone()).collect();
    assert_eq!(sessions.len(), 3);
    while !matches!(runner.recv_timeout(Duration::from_secs(3)).unwrap(), RuntimeEvent::Queued { id, depth: 4 } if id == ids[6])
    {
    }
    for limit in [0, 4, 65] {
        assert_eq!(
            runner.try_set_concurrency(limit).unwrap_err(),
            zenpi::runtime::ConcurrencyError::InvalidLimit
        );
    }
    runner
        .try_set_concurrency(1)
        .unwrap()
        .recv_timeout(Duration::from_secs(3))
        .unwrap();
    assert_eq!(runner.applied_concurrency(), 1);
    assert_eq!(runner.quota(), 3, "leased capacity is not an applied limit");
    assert_eq!(runner.workers().len(), 3);
    let release = |index: usize| {
        releases.0.lock().unwrap()[index] = true;
        releases.1.notify_all();
    };
    let mut completed = std::collections::BTreeSet::new();
    for index in 0..2 {
        release(index);
        loop {
            if let RuntimeEvent::Completed { id, outcome } =
                runner.recv_timeout(Duration::from_secs(3)).unwrap()
            {
                assert!(matches!(outcome, JobOutcome::Succeeded(_)));
                completed.insert(id.get());
                if id == ids[index] {
                    break;
                }
            }
        }
        assert!(
            starts.recv_timeout(Duration::from_millis(30)).is_err(),
            "lowering must not fill a slot while existing live count >= limit"
        );
    }
    release(2);
    assert_eq!(starts.recv_timeout(Duration::from_secs(3)).unwrap().0, 3);
    assert!(starts.recv_timeout(Duration::from_millis(30)).is_err());
    assert!(pool.retire_worker_owners(&project).is_err());
    assert!(
        pool.commit_subtabs(
            &project,
            initial.revision,
            initial.set_concurrency(&tab, 1).unwrap()
        )
        .is_err()
    );
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        checkpoint
    );
    runner
        .try_set_concurrency(3)
        .unwrap()
        .recv_timeout(Duration::from_secs(3))
        .unwrap();
    assert_eq!(runner.applied_concurrency(), 3);
    let raised: Vec<_> = (0..2)
        .map(|_| starts.recv_timeout(Duration::from_secs(3)).unwrap())
        .collect();
    let raised_inputs: std::collections::BTreeSet<_> =
        raised.iter().map(|(input, _)| *input).collect();
    assert_eq!(raised_inputs, std::collections::BTreeSet::from([4, 5]));
    assert!(raised.iter().all(|(_, session)| sessions.contains(session)));
    assert!(
        starts.recv_timeout(Duration::from_millis(30)).is_err(),
        "N+1 is queued after raising"
    );
    release(3);
    assert_eq!(starts.recv_timeout(Duration::from_secs(3)).unwrap().0, 6);
    for index in 4..7 {
        release(index);
    }
    while completed.len() < 7 {
        if let RuntimeEvent::Completed { id, outcome } =
            runner.recv_timeout(Duration::from_secs(3)).unwrap()
        {
            assert!(matches!(outcome, JobOutcome::Succeeded(_)));
            assert!(completed.insert(id.get()), "duplicate terminal event");
        }
    }
    runner.shutdown_and_join().unwrap();
    let journals: Vec<_> = pool
        .worker_handles(&project)
        .iter()
        .map(|worker| {
            std::fs::read_to_string(worker.agent().lock().unwrap().session().path()).unwrap()
        })
        .collect();
    for index in 0..7 {
        assert_eq!(
            journals
                .iter()
                .filter(|text| text.contains(&format!("resized-job-{index}")))
                .count(),
            1
        );
    }
    assert!(
        pool.owner(&project)
            .unwrap()
            .lock()
            .unwrap()
            .session()
            .turns()
            .is_empty()
    );
    assert_eq!(
        pool.subtab_state(&project).unwrap(),
        initial,
        "runtime adjustment is not a durable workspace commit"
    );
    pool.retire_worker_owners(&project).unwrap();
}

#[test]
fn worker_limit_reports_only_scheduler_application_and_retains_claims_when_lowered() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, tab) = pool(&root, 3);
    let runner = WorktreeRunner::spawn(
        &mut pool,
        &project,
        &tab,
        RuntimeConfig {
            event_capacity: 1,
            ..RuntimeConfig::default()
        },
        |_: WorkerOwner, _: (), token| {
            while !token.is_cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok::<_, String>(())
        },
    )
    .unwrap();
    let id = runner.try_submit(()).unwrap();
    let receipt = runner.try_set_concurrency(1).unwrap();
    assert!(receipt.recv_timeout(Duration::from_millis(30)).is_err());
    assert_eq!(runner.applied_concurrency(), 3);
    assert!(matches!(
        runner.recv_timeout(Duration::from_secs(3)).unwrap(),
        RuntimeEvent::Accepted { .. }
    ));
    assert!(
        matches!(runner.recv_timeout(Duration::from_secs(3)).unwrap(), RuntimeEvent::Started { id: value } if value == id)
    );
    receipt.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(runner.applied_concurrency(), 1);
    assert!(pool.retire_worker_owners(&project).is_err());
    assert!(
        WorktreeRunner::spawn(
            &mut pool,
            &project,
            &tab,
            RuntimeConfig::default(),
            |_: WorkerOwner, _: (), _| Ok::<_, String>(())
        )
        .is_err()
    );
    runner
        .try_shutdown_with_grace(Duration::from_secs(1))
        .unwrap();
    let unapplied = runner.try_set_concurrency(2);
    runner.shutdown_and_join().unwrap();
    if let Ok(unapplied) = unapplied {
        assert_eq!(unapplied.try_recv(), Err(mpsc::TryRecvError::Disconnected));
    }
    pool.retire_worker_owners(&project).unwrap();
}

#[test]
fn runtime_claim_fences_even_idle_slots_and_releases_after_orderly_shutdown() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, tab) = pool(&root, 2);
    let spawn = |pool: &mut ProjectOwnerPool| {
        WorktreeRunner::spawn(
            pool,
            &project,
            &tab,
            RuntimeConfig::default(),
            |_: WorkerOwner, _: (), _| Ok::<_, String>(()),
        )
    };
    let runner = spawn(&mut pool).unwrap();
    assert!(spawn(&mut pool).is_err());
    assert!(pool.retire_worker_owners(&project).is_err());
    let state = pool.subtab_state(&project).unwrap();
    let checkpoint = root.join("project-workspace.json");
    let before = std::fs::read(&checkpoint).unwrap();
    assert!(
        pool.commit_subtabs(
            &project,
            state.revision,
            state.set_concurrency(&tab, 1).unwrap()
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&checkpoint).unwrap(), before);
    runner.shutdown_and_join().unwrap();
    let runner = spawn(&mut pool).unwrap();
    runner.shutdown_and_join().unwrap();
    pool.commit_subtabs(
        &project,
        state.revision,
        state.set_concurrency(&tab, 1).unwrap(),
    )
    .unwrap();
    let runner = spawn(&mut pool).unwrap();
    assert_eq!(runner.quota(), 1);
    runner.shutdown_and_join().unwrap();
    pool.retire_worker_owners(&project).unwrap();
}

#[test]
fn failed_multi_slot_start_rolls_back_partial_runtime_claims() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, tab) = pool(&root, 2);
    let second = pool.worker_owner(&project, &tab, 1).unwrap();
    let guard = second.agent().lock().unwrap();
    let result = WorktreeRunner::spawn(
        &mut pool,
        &project,
        &tab,
        RuntimeConfig::default(),
        |_: WorkerOwner, _: (), _| Ok::<_, String>(()),
    );
    assert!(result.is_err());
    drop(guard);
    // Slot zero's claim from the failed startup cannot poison the next startup.
    let runner = WorktreeRunner::spawn(
        &mut pool,
        &project,
        &tab,
        RuntimeConfig::default(),
        |_: WorkerOwner, _: (), _| Ok::<_, String>(()),
    )
    .unwrap();
    runner.shutdown_and_join().unwrap();
    pool.retire_worker_owners(&project).unwrap();
}

#[test]
fn active_and_queued_cancellation_do_not_cancel_another_worker_slot() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, tab) = pool(&root, 2);
    let (entered, entries) = mpsc::channel();
    let runner = WorktreeRunner::spawn(
        &mut pool,
        &project,
        &tab,
        RuntimeConfig::default(),
        move |_: WorkerOwner, index: usize, token| {
            entered.send(index).unwrap();
            let until = Instant::now() + Duration::from_secs(5);
            while !token.is_cancelled() {
                assert!(Instant::now() < until);
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok::<_, String>(index)
        },
    )
    .unwrap();
    let ids: Vec<_> = (0..3)
        .map(|index| runner.try_submit(index).unwrap())
        .collect();
    entries.recv_timeout(Duration::from_secs(3)).unwrap();
    entries.recv_timeout(Duration::from_secs(3)).unwrap();
    loop {
        if matches!(runner.recv_timeout(Duration::from_secs(3)).unwrap(), RuntimeEvent::Queued { id, .. } if id == ids[2])
        {
            break;
        }
    }
    for id in [ids[2], ids[0]] {
        runner.try_cancel(id).unwrap();
        loop {
            if let RuntimeEvent::Completed {
                id: actual,
                outcome,
            } = runner.recv_timeout(Duration::from_secs(3)).unwrap()
            {
                assert_eq!(actual, id);
                assert!(matches!(outcome, JobOutcome::Cancelled));
                break;
            }
        }
    }
    assert!(entries.recv_timeout(Duration::from_millis(30)).is_err());
    assert!(
        runner.recv_timeout(Duration::from_millis(30)).is_err(),
        "other slot remains running"
    );
    let mut terminals = Vec::new();
    runner
        .shutdown_and_join_with_events(Duration::from_secs(1), |event| {
            if let RuntimeEvent::Completed {
                id,
                outcome: JobOutcome::Cancelled,
            } = event
            {
                terminals.push(id);
            }
        })
        .unwrap();
    assert_eq!(terminals, vec![ids[1]]);
    pool.retire_worker_owners(&project).unwrap();
}

#[test]
fn panic_returns_slot_to_pool_and_other_jobs_still_complete() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, tab) = pool(&root, 1);
    let runner = WorktreeRunner::spawn(
        &mut pool,
        &project,
        &tab,
        RuntimeConfig::default(),
        |_: WorkerOwner, index: usize, _| {
            if index == 0 {
                panic!("controlled slot panic before taking Agent lock");
            }
            Ok::<_, String>(index)
        },
    )
    .unwrap();
    let first = runner.try_submit(0).unwrap();
    let second = runner.try_submit(1).unwrap();
    let mut completed = 0;
    while completed < 2 {
        if let RuntimeEvent::Completed { id, outcome } =
            runner.recv_timeout(Duration::from_secs(3)).unwrap()
        {
            if id == first {
                assert!(matches!(outcome, JobOutcome::Panicked));
            } else {
                assert_eq!(id, second);
                assert!(matches!(outcome, JobOutcome::Succeeded(1)));
            }
            completed += 1;
        }
    }
    runner.shutdown_and_join().unwrap();
    pool.retire_worker_owners(&project).unwrap();
}

#[test]
fn detached_job_retains_runtime_claim_and_journal_lease_until_it_really_finishes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, tab) = pool(&root, 3);
    let (entered, entries) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let gate = Mutex::new(gate);
    let (finished, finishes) = mpsc::channel();
    let runner = WorktreeRunner::spawn(
        &mut pool,
        &project,
        &tab,
        RuntimeConfig::default(),
        move |_: WorkerOwner, _: (), _| {
            entered.send(()).unwrap();
            gate.lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            finished.send(()).unwrap();
            Ok::<_, String>(())
        },
    )
    .unwrap();
    runner.try_submit(()).unwrap();
    entries.recv_timeout(Duration::from_secs(3)).unwrap();
    runner
        .try_set_concurrency(1)
        .unwrap()
        .recv_timeout(Duration::from_secs(3))
        .unwrap();
    assert_eq!(runner.applied_concurrency(), 1);
    assert_eq!(runner.workers().len(), 3);
    runner
        .shutdown_and_join_with_events(Duration::from_millis(20), |_| {})
        .unwrap();
    let held = pool.retire_worker_owners(&project).is_err();
    release.send(()).unwrap();
    finishes.recv_timeout(Duration::from_secs(3)).unwrap();
    let until = Instant::now() + Duration::from_secs(3);
    while pool.retire_worker_owners(&project).is_err() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        held,
        "detached execution must retain claim until real completion"
    );
}

#[test]
fn restart_reconstructs_quota_and_reuses_worker_history_without_resubmission() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut first, project, tab) = pool(&root, 2);
    let runner = WorktreeRunner::spawn(
        &mut first,
        &project,
        &tab,
        RuntimeConfig::default(),
        |worker, _: (), _| {
            worker
                .agent()
                .lock()
                .unwrap()
                .process_sync("persisted-before-restart")
                .map(|_| ())
                .map_err(|error| error.to_string())
        },
    )
    .unwrap();
    let identities: Vec<_> = runner
        .workers()
        .iter()
        .map(|worker| {
            worker
                .agent()
                .lock()
                .unwrap()
                .session()
                .session_id()
                .to_owned()
        })
        .collect();
    runner.try_submit(()).unwrap();
    loop {
        if let RuntimeEvent::Completed { outcome, .. } =
            runner.recv_timeout(Duration::from_secs(3)).unwrap()
        {
            assert!(matches!(outcome, JobOutcome::Succeeded(())));
            break;
        }
    }
    runner
        .try_set_concurrency(1)
        .unwrap()
        .recv_timeout(Duration::from_secs(3))
        .unwrap();
    assert_eq!(runner.applied_concurrency(), 1);
    runner.shutdown_and_join().unwrap();
    drop(first);
    let (mut second, _, _) = pool(&root, 2);
    let runner = WorktreeRunner::spawn(
        &mut second,
        &project,
        &tab,
        RuntimeConfig::default(),
        |_: WorkerOwner, _: (), _| -> Result<(), String> {
            panic!("restart must never resubmit");
        },
    )
    .unwrap();
    assert_eq!(
        runner.applied_concurrency(),
        2,
        "runtime-only changes must not masquerade as durable quota after restart"
    );
    assert_eq!(runner.quota(), 2);
    for (worker, expected) in runner.workers().iter().zip(identities) {
        assert_eq!(
            worker.agent().lock().unwrap().session().session_id(),
            expected
        );
    }
    assert!(runner.recv_timeout(Duration::from_millis(30)).is_err());
    runner.shutdown_and_join().unwrap();
    second.retire_worker_owners(&project).unwrap();
}

#[test]
fn production_prompt_adapter_uses_selected_root_and_rejects_private_or_shell_inputs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, _) = pool(&root, 2);
    let child = root.join("selected");
    std::fs::create_dir(&child).unwrap();
    std::fs::write(child.join("note.txt"), "selected attachment").unwrap();
    let before = pool.subtab_state(&project).unwrap();
    let next = before
        .add_directory(&project, &root, &child, "selected")
        .unwrap();
    let tab = next.active_id.clone();
    pool.commit_subtabs(&project, before.revision, next)
        .unwrap();
    let runner =
        WorktreeRunner::spawn_prompts(&mut pool, &project, &tab, RuntimeConfig::default()).unwrap();
    assert_eq!(runner.quota(), 1);
    assert_eq!(
        runner.workers()[0]
            .agent()
            .lock()
            .unwrap()
            .attachment_workspace_root(),
        Some(child.as_path())
    );
    for (text, success) in [
        ("normal worker prompt", true),
        ("!touch never-created", false),
        ("@.zenpi/auth.json", false),
        ("", false),
        ("has\0nul", false),
    ] {
        let id = runner.try_submit(text.into()).unwrap();
        loop {
            if let RuntimeEvent::Completed {
                id: actual,
                outcome,
            } = runner.recv_timeout(Duration::from_secs(3)).unwrap()
            {
                assert_eq!(actual, id);
                assert_eq!(
                    matches!(outcome, JobOutcome::Succeeded(_)),
                    success,
                    "{text}: {outcome:?}"
                );
                break;
            }
        }
    }
    assert!(!child.join("never-created").exists());
    assert_eq!(
        runner.workers()[0].agent().lock().unwrap().history().len(),
        2
    );
    runner.shutdown_and_join().unwrap();
    pool.retire_worker_owners(&project).unwrap();
}

#[test]
fn registered_git_worktree_runner_keeps_main_journal_and_tool_root_separate() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    for args in [
        vec!["init", "--quiet"],
        vec![
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "initial",
        ],
    ] {
        assert!(
            std::process::Command::new("git")
                .args(args)
                .current_dir(&root)
                .status()
                .unwrap()
                .success()
        );
    }
    let (mut pool, project, _) = pool(&root, 1);
    let managed = root.join(".zenpi-worktrees");
    std::fs::create_dir(&managed).unwrap();
    let worktree = managed.join("worker-branch");
    let before = pool.subtab_state(&project).unwrap();
    let candidate = before
        .plan_worktree(&project, &root, &worktree, "worker-branch")
        .unwrap();
    zenpi::project_workspace::add_worktree(&root, &worktree, "worker-branch").unwrap();
    let tab = candidate.active_id.clone();
    pool.commit_subtabs(&project, before.revision, candidate)
        .unwrap();
    let master = pool.owner(&project).unwrap();
    let master_path = master.lock().unwrap().session().path().to_owned();
    let before = std::fs::read(&master_path).unwrap();
    let runner =
        WorktreeRunner::spawn_prompts(&mut pool, &project, &tab, RuntimeConfig::default()).unwrap();
    assert_eq!(runner.workers()[0].execution().tab.root, worktree);
    runner.try_submit("worktree-scoped-prompt".into()).unwrap();
    loop {
        if let RuntimeEvent::Completed { outcome, .. } =
            runner.recv_timeout(Duration::from_secs(3)).unwrap()
        {
            assert!(matches!(outcome, JobOutcome::Succeeded(_)), "{outcome:?}");
            break;
        }
    }
    let worker = &runner.workers()[0];
    assert_eq!(
        worker.agent().lock().unwrap().attachment_workspace_root(),
        Some(worktree.as_path())
    );
    assert_ne!(worker.agent().lock().unwrap().session().path(), master_path);
    assert_eq!(std::fs::read(master_path).unwrap(), before);
    runner.shutdown_and_join().unwrap();
    pool.retire_worker_owners(&project).unwrap();
}
