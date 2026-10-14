use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use zenpi::{
    core::{Agent, AgentPhase},
    project_workspace::ProjectOwnerPool,
    runtime::{BackgroundRunner, JobOutcome, RuntimeConfig, RuntimeEvent},
    session::SessionStore,
};

fn pool(root: &Path) -> ProjectOwnerPool {
    let owner = Arc::new(Mutex::new(Agent::with_echo(
        SessionStore::open_in_workspace(root.join("session.jsonl"), root).unwrap(),
    )));
    let mut pool = ProjectOwnerPool::new(owner).unwrap();
    pool.restore_checkpoint().unwrap();
    pool
}

fn setup(root: &Path) -> (ProjectOwnerPool, String, String) {
    let mut pool = pool(root);
    let project = pool.active_context().project_id.clone();
    let state = pool.subtab_state(&project).unwrap();
    let id = state.active_id.clone();
    pool.commit_subtabs(
        &project,
        state.revision,
        state.set_concurrency(&id, 2).unwrap(),
    )
    .unwrap();
    (pool, project, id)
}

#[test]
fn worker_slots_have_independent_journals_controls_and_restart_identity() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, tab) = setup(&root);
    let master = pool.owner(&project).unwrap();
    let before = std::fs::read(master.lock().unwrap().session().path()).unwrap();
    let first = pool.worker_owner(&project, &tab, 0).unwrap();
    let second = pool.worker_owner(&project, &tab, 1).unwrap();
    assert!(std::ptr::eq(
        first.agent(),
        pool.worker_owner(&project, &tab, 0).unwrap().agent()
    ));
    let first_id = first
        .agent()
        .lock()
        .unwrap()
        .session()
        .session_id()
        .to_owned();
    let second_id = second
        .agent()
        .lock()
        .unwrap()
        .session()
        .session_id()
        .to_owned();
    assert_ne!(first_id, second_id);
    assert_ne!(first_id, pool.active_context().session_id);
    assert_eq!(first.control().input_port.session_id().as_str(), first_id);
    assert_eq!(second.control().project, project);
    let runner = BackgroundRunner::spawn_with_concurrency(
        |(worker, text): (zenpi::remote::worker::WorkerOwner, String), token| {
            worker
                .agent()
                .lock()
                .unwrap()
                .process_with_cancel(zenpi::core::TurnInputRequest::new(text), || {
                    token.is_cancelled()
                })
                .map(|_| ())
                .map_err(|error| error.to_string())
        },
        RuntimeConfig::default(),
        2,
    )
    .unwrap();
    runner
        .try_submit((first.clone(), "slot-zero-only".into()))
        .unwrap();
    runner
        .try_submit((second.clone(), "slot-one-only".into()))
        .unwrap();
    let mut completed = 0;
    while completed < 2 {
        if let RuntimeEvent::Completed { outcome, .. } =
            runner.recv_timeout(Duration::from_secs(3)).unwrap()
        {
            assert!(matches!(outcome, JobOutcome::Succeeded(())));
            completed += 1;
        }
    }
    runner.shutdown_and_join().unwrap();
    let first_path = first.agent().lock().unwrap().session().path().to_owned();
    let second_path = second.agent().lock().unwrap().session().path().to_owned();
    let first_bytes = std::fs::read(&first_path).unwrap();
    let second_bytes = std::fs::read(&second_path).unwrap();
    assert!(
        String::from_utf8(first_bytes.clone())
            .unwrap()
            .contains("slot-zero-only")
    );
    assert!(
        !String::from_utf8(first_bytes.clone())
            .unwrap()
            .contains("slot-one-only")
    );
    assert_eq!(
        std::fs::read(master.lock().unwrap().session().path()).unwrap(),
        before
    );
    drop(first);
    drop(second);
    drop(master);
    drop(pool);
    let mut restored = self::pool(&root);
    let first = restored.worker_owner(&project, &tab, 0).unwrap();
    let second = restored.worker_owner(&project, &tab, 1).unwrap();
    assert_eq!(
        first.agent().lock().unwrap().session().session_id(),
        first_id
    );
    assert_eq!(
        second.agent().lock().unwrap().session().session_id(),
        second_id
    );
    // Normal owner preparation appends extension-selection evidence, but
    // never rewrites the prior journal or duplicates completed turns.
    assert!(std::fs::read(first_path).unwrap().starts_with(&first_bytes));
    assert!(
        std::fs::read(second_path)
            .unwrap()
            .starts_with(&second_bytes)
    );
    assert_eq!(first.agent().lock().unwrap().history().len(), 2);
    assert_eq!(second.agent().lock().unwrap().history().len(), 2);
}

#[test]
fn durable_quota_and_stable_subtab_identity_bound_owner_creation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    let (mut pool, project, main) = setup(&root);
    assert!(pool.worker_owner(&project, &main, 2).is_err());
    assert!(pool.worker_owner(&project, &main, 64).is_err());
    assert!(pool.worker_owner(&project, "../escape", 0).is_err());
    assert!(pool.worker_owner("foreign", &main, 0).is_err());
    let state = pool.subtab_state(&project).unwrap();
    let next = state
        .add_directory(&project, &root, &child, "child")
        .unwrap();
    let tab = next.active_id.clone();
    pool.commit_subtabs(&project, state.revision, next).unwrap();
    let worker = pool.worker_owner(&project, &tab, 0).unwrap();
    let master_worker = pool.worker_owner(&project, &main, 0).unwrap();
    assert_eq!(
        worker.agent().lock().unwrap().attachment_workspace_root(),
        Some(child.as_path())
    );
    assert_ne!(
        worker.agent().lock().unwrap().session().path(),
        master_worker.agent().lock().unwrap().session().path()
    );
    let state = pool.subtab_state(&project).unwrap();
    pool.commit_subtabs(
        &project,
        state.revision,
        state.rename(&tab, "renamed").unwrap(),
    )
    .unwrap();
    assert!(std::ptr::eq(
        worker.agent(),
        pool.worker_owner(&project, &tab, 0).unwrap().agent()
    ));
    let state = pool.subtab_state(&project).unwrap();
    pool.commit_subtabs(&project, state.revision, state.select(&main).unwrap())
        .unwrap();
    assert!(std::ptr::eq(
        worker.agent(),
        pool.worker_owner(&project, &tab, 0).unwrap().agent()
    ));
}

#[test]
fn held_worker_fences_subtab_changes_without_publishing_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, tab) = setup(&root);
    let worker = pool.worker_owner(&project, &tab, 1).unwrap();
    let guard = worker.agent().lock().unwrap();
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
    assert_eq!(pool.subtab_state(&project).unwrap(), state);
    drop(guard);
    pool.commit_subtabs(
        &project,
        state.revision,
        state.set_concurrency(&tab, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(worker.agent().lock().unwrap().phase(), AgentPhase::Closed);
    assert_eq!(pool.worker_handles(&project).len(), 0);
    assert!(pool.worker_owner(&project, &tab, 1).is_err());
}

#[test]
fn execution_handle_retains_exclusive_lease_after_pool_is_dropped() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut first, project, tab) = setup(&root);
    let retained = first.worker_owner(&project, &tab, 0).unwrap();
    let path = retained.agent().lock().unwrap().session().path().to_owned();
    let before = std::fs::read(&path).unwrap();
    drop(first);
    let mut second = pool(&root);
    assert!(second.worker_owner(&project, &tab, 0).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    drop(retained);
    assert!(second.worker_owner(&project, &tab, 0).is_ok());
}

#[test]
fn retiring_workers_is_atomic_when_one_is_busy_and_closes_retained_handles() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, tab) = setup(&root);
    let first = pool.worker_owner(&project, &tab, 0).unwrap();
    let second = pool.worker_owner(&project, &tab, 1).unwrap();
    let guard = second.agent().lock().unwrap();
    assert!(pool.retire_worker_owners(&project).is_err());
    assert_eq!(pool.worker_handles(&project).len(), 2);
    assert_eq!(first.agent().lock().unwrap().phase(), AgentPhase::Idle);
    drop(guard);
    pool.retire_worker_owners(&project).unwrap();
    assert!(pool.worker_handles(&project).is_empty());
    assert_eq!(first.agent().lock().unwrap().phase(), AgentPhase::Closed);
    assert_eq!(second.agent().lock().unwrap().phase(), AgentPhase::Closed);
    assert!(
        first
            .agent()
            .lock()
            .unwrap()
            .process_sync("must not execute")
            .is_err()
    );
    // Retirement cannot free an externally retained execution lease.
    assert!(pool.worker_owner(&project, &tab, 0).is_err());
    drop(first);
    assert!(pool.worker_owner(&project, &tab, 0).is_ok());
}

#[test]
fn failed_checkpoint_commit_does_not_retire_the_existing_worker() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, tab) = setup(&root);
    let worker = pool.worker_owner(&project, &tab, 1).unwrap();
    let checkpoint = root.join("project-workspace.json");
    let mut external = std::fs::read(&checkpoint).unwrap();
    external.push(b' ');
    std::fs::write(&checkpoint, &external).unwrap();
    let before = pool.subtab_state(&project).unwrap();
    assert!(
        pool.commit_subtabs(
            &project,
            before.revision,
            before.set_concurrency(&tab, 1).unwrap()
        )
        .is_err()
    );
    assert_eq!(pool.subtab_state(&project).unwrap(), before);
    assert_eq!(worker.agent().lock().unwrap().phase(), AgentPhase::Idle);
    assert_eq!(pool.worker_handles(&project).len(), 1);
    assert_eq!(std::fs::read(&checkpoint).unwrap(), external);
}

#[test]
fn project_close_fences_busy_workers_and_retires_them_after_durable_commit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("second");
    std::fs::create_dir(&child).unwrap();
    let (mut pool, project, tab) = setup(&root);
    let worker = pool.worker_owner(&project, &tab, 0).unwrap();
    pool.open(Some(&child)).unwrap();
    let guard = worker.agent().lock().unwrap();
    let checkpoint = root.join("project-workspace.json");
    let before = std::fs::read(&checkpoint).unwrap();
    assert!(pool.close(&project).is_err());
    assert_eq!(std::fs::read(&checkpoint).unwrap(), before);
    drop(guard);
    pool.close(&project).unwrap();
    assert!(pool.worker_handles(&project).is_empty());
    assert_eq!(worker.agent().lock().unwrap().phase(), AgentPhase::Closed);
    assert!(pool.worker_owner(&project, &tab, 0).is_err());
}

#[test]
fn untrusted_workers_directory_and_wrong_workspace_journal_fail_before_preparation() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut host, project, tab) = setup(&root);
    let worker = host.worker_owner(&project, &tab, 0).unwrap();
    let journal = worker.agent().lock().unwrap().session().path().to_owned();
    drop(worker);
    drop(host);
    let directory = journal.parent().unwrap();
    std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut host = pool(&root);
    let before = std::fs::read(&journal).unwrap();
    assert!(host.worker_owner(&project, &tab, 0).is_err());
    assert_eq!(std::fs::read(&journal).unwrap(), before);
    std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::remove_file(&journal).unwrap();
    let foreign = root.join("foreign");
    std::fs::create_dir(&foreign).unwrap();
    drop(SessionStore::open_in_workspace(&journal, &foreign).unwrap());
    let before = std::fs::read(&journal).unwrap();
    assert!(host.worker_owner(&project, &tab, 0).is_err());
    assert_eq!(std::fs::read(&journal).unwrap(), before);
}

#[test]
fn symlink_directory_and_journal_aliases_are_refused_without_touching_target() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut host, project, tab) = setup(&root);
    let foreign = root.join("foreign");
    std::fs::create_dir(&foreign).unwrap();
    symlink(&foreign, root.join(".zenpi")).unwrap();
    assert!(host.worker_owner(&project, &tab, 0).is_err());
    assert_eq!(std::fs::read_dir(&foreign).unwrap().count(), 0);
    std::fs::remove_file(root.join(".zenpi")).unwrap();
    let worker = host.worker_owner(&project, &tab, 0).unwrap();
    let journal = worker.agent().lock().unwrap().session().path().to_owned();
    drop(worker);
    drop(host);
    let original = root.join("original.jsonl");
    std::fs::rename(&journal, &original).unwrap();
    let before = std::fs::read(&original).unwrap();
    let mut host = pool(&root);
    symlink(&original, &journal).unwrap();
    assert!(host.worker_owner(&project, &tab, 0).is_err());
    std::fs::remove_file(&journal).unwrap();
    std::fs::hard_link(&original, &journal).unwrap();
    assert!(host.worker_owner(&project, &tab, 0).is_err());
    assert_eq!(std::fs::read(&original).unwrap(), before);
    assert!(host.worker_handles(&project).is_empty());
}

#[test]
fn worker_lease_excludes_an_independent_process_until_the_last_handle_drops() {
    const CHILD: &str = "ZENPI_WORKER_LEASE_TEST_CHILD";
    const ADMITTED: &str = "ZENPI_WORKER_LEASE_TEST_ADMITTED";
    if let Some(root) = std::env::var_os(CHILD) {
        let mut host = pool(Path::new(&root));
        let project = host.active_context().project_id.clone();
        let tab = host.subtab_state(&project).unwrap().active_id;
        let admitted = host.worker_owner(&project, &tab, 0).is_ok();
        assert_eq!(admitted, std::env::var_os(ADMITTED).is_some());
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut host, project, tab) = setup(&root);
    let worker = host.worker_owner(&project, &tab, 0).unwrap();
    let journal = worker.agent().lock().unwrap().session().path().to_owned();
    let before = std::fs::read(&journal).unwrap();
    let run = |admitted: bool| {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "worker_lease_excludes_an_independent_process_until_the_last_handle_drops",
                "--nocapture",
            ])
            .env(CHILD, &root)
            .env_remove(ADMITTED);
        if admitted {
            command.env(ADMITTED, "1");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    };
    run(false);
    assert_eq!(std::fs::read(&journal).unwrap(), before);
    drop(host);
    run(false);
    drop(worker);
    run(true);
    assert!(std::fs::read(&journal).unwrap().starts_with(&before));
}

#[test]
fn sixty_four_prepared_owners_are_bounded_across_all_subtabs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, project, tab) = setup(&root);
    let state = pool.subtab_state(&project).unwrap();
    pool.commit_subtabs(
        &project,
        state.revision,
        state.set_concurrency(&tab, 64).unwrap(),
    )
    .unwrap();
    let mut paths = std::collections::BTreeSet::new();
    for slot in 0..64 {
        let worker = pool.worker_owner(&project, &tab, slot).unwrap();
        paths.insert(worker.agent().lock().unwrap().session().path().to_owned());
    }
    assert_eq!(paths.len(), 64);
    assert_eq!(pool.worker_handles(&project).len(), 64);
    let child = root.join("another");
    std::fs::create_dir(&child).unwrap();
    let state = pool.subtab_state(&project).unwrap();
    let next = state
        .add_directory(&project, &root, &child, "another")
        .unwrap();
    let new_tab = next.active_id.clone();
    pool.commit_subtabs(&project, state.revision, next).unwrap();
    assert!(pool.worker_owner(&project, &new_tab, 0).is_err());
    assert_eq!(pool.worker_handles(&project).len(), 64);
    assert_eq!(
        std::fs::read_dir(root.join(".zenpi/workers"))
            .unwrap()
            .count(),
        128
    );
    assert!(
        pool.worker_owner(&project, &tab, 0).is_ok(),
        "existing slot does not consume another owner"
    );
    let runner = zenpi::remote::worktree_runner::WorktreeRunner::spawn(
        &mut pool,
        &project,
        &tab,
        RuntimeConfig::default(),
        |_, _: (), _| Ok::<_, String>(()),
    )
    .unwrap();
    assert_eq!(runner.quota(), 64);
    assert_eq!(pool.worker_handles(&project).len(), 64);
    assert_eq!(
        std::fs::read_dir(root.join(".zenpi/workers"))
            .unwrap()
            .count(),
        128
    );
    runner.shutdown_and_join().unwrap();
    pool.retire_worker_owners(&project).unwrap();
    assert!(pool.worker_owner(&project, &new_tab, 0).is_ok());
}

#[test]
fn only_explicit_host_auto_approval_propagates_to_worker_agents() {
    for auto in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Agent::prepare_project_with_approval(
            &root.join("session.jsonl"),
            &root,
            Default::default(),
            true,
            auto,
        )
        .unwrap();
        let mut host = ProjectOwnerPool::new(Arc::new(Mutex::new(master))).unwrap();
        host.restore_checkpoint().unwrap();
        let project = host.active_context().project_id.clone();
        let tab = host.subtab_state(&project).unwrap().active_id;
        let worker = host.worker_owner(&project, &tab, 0).unwrap();
        let agent = worker.agent().lock().unwrap();
        assert_eq!(agent.auto_approve(), auto);
        assert_eq!(agent.attachment_workspace_root(), Some(root.as_path()));
        assert!(worker.control().approval.is_some());
    }
}
