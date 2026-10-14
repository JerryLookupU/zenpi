#![cfg(feature = "remote")]

use serde_json::json;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use zenpi::{
    core::Agent,
    project_workspace::ProjectOwnerPool,
    session::SessionStore,
    worktree::subtab::{LegacySubtab, SubtabKind, SubtabState},
};

// Owner preparation deliberately validates config, even with EchoBackend.
// Run those cases in a child rather than racing process-global environment.
fn isolated_configuration(test: &str) -> bool {
    const CHILD: &str = "ZENPI_SUBTAB_CONFIGURATION_CHILD";
    if std::env::var(CHILD).as_deref() == Ok(test) {
        return false;
    }
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join("zenpi");
    std::fs::create_dir(&config).unwrap();
    let settings = "backend='openai'\nmodel='fixture'\nbase_url='http://127.0.0.1:9/v1'\nwire_api='responses'\nrequires_openai_auth=false\n";
    std::fs::write(config.join("config.toml"), settings).unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command.args(["--exact", test, "--nocapture"]).env_clear();
    for key in ["PATH", "TMPDIR", "TEMP", "TMP", "SYSTEMROOT"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
        .env(CHILD, test)
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ZENPI_HOME", &config)
        .env("CODEX_HOME", home.path().join("codex"));
    let mut child = command.spawn().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "isolated config case {test}");
            break;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("isolated config case exceeded deadline: {test}");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        std::fs::read_to_string(config.join("config.toml")).unwrap(),
        settings
    );
    assert!(!config.join("auth.json").exists());
    assert!(!home.path().join("codex").exists());
    true
}

fn pool(root: &Path) -> ProjectOwnerPool {
    let agent = Agent::with_echo(
        SessionStore::open_in_workspace(root.join("session.jsonl"), root).unwrap(),
    );
    let mut host = ProjectOwnerPool::new(Arc::new(Mutex::new(agent))).unwrap();
    host.restore_checkpoint().unwrap();
    host
}
fn legacy(root: &Path) -> Vec<LegacySubtab> {
    [
        ("main", root.to_path_buf(), SubtabKind::Main),
        (
            "one",
            root.join(".zenpi-workspaces/one"),
            SubtabKind::InPlace,
        ),
        (
            "two",
            root.join(".zenpi-workspaces/two"),
            SubtabKind::InPlace,
        ),
    ]
    .into_iter()
    .map(|(name, root, kind)| LegacySubtab {
        name: name.into(),
        root: root.display().to_string(),
        kind,
        concurrency: 1,
    })
    .collect()
}

#[test]
fn index_migration_is_deterministic_and_selection_survives_rename_and_order() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let old = legacy(&root);
    let state = SubtabState::migrate("project", &root, &old, 2).unwrap();
    assert_eq!(
        state,
        SubtabState::migrate("project", &root, &old, 2).unwrap()
    );
    assert_eq!(state.active_id, state.tabs[2].id);
    let id = state.active_id.clone();
    let renamed = state.rename(&id, "renamed").unwrap();
    assert_eq!(renamed.tabs[2].id, id);
    let moved = renamed.move_before(&id, Some(&state.tabs[1].id)).unwrap();
    assert_eq!(moved.tabs[1].id, id);
    assert_eq!(moved.active_id, id);
    assert_eq!(moved.revision, 2);
    let configured = moved.set_concurrency(&id, 64).unwrap();
    assert_eq!(configured.tabs[1].concurrency, 64);
    assert_eq!(configured.set_concurrency(&id, 64).unwrap(), configured);
    assert!(configured.set_concurrency(&id, 0).is_err());
    assert!(configured.set_concurrency(&id, 65).is_err());
    assert!(configured.move_before(&state.tabs[0].id, None).is_err());
    assert!(
        configured
            .move_before(&id, Some(&state.tabs[0].id))
            .is_err()
    );
    assert!(configured.select("missing").is_err());
    assert!(configured.rename(&id, "main").is_err());
    assert!(configured.rename(&id, "bad\nname").is_err());
    configured.validate("project", &root).unwrap();
}

#[test]
fn migration_commits_with_owner_checkpoint_and_restart_ignores_legacy_overwrite() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let id = host.active_context().project_id.clone();
    let signal = zenpi::remote::wakeup::watch(&host.active_context().session_id).unwrap();
    let observed = signal.revision();
    let old = legacy(&root);
    host.migrate_legacy_subtabs(&id, &old, 2).unwrap();
    assert_ne!(signal.revision(), observed);
    let migrated = host.subtab_state(&id).unwrap();
    assert_eq!(migrated.revision, 1);
    let selected = migrated.select(&migrated.tabs[1].id).unwrap();
    let observed = signal.revision();
    host.commit_subtabs(&id, 1, selected.clone()).unwrap();
    assert_ne!(signal.revision(), observed);
    let observed = signal.revision();
    host.commit_subtabs(&id, 2, selected.clone()).unwrap();
    assert_eq!(signal.revision(), observed);
    assert!(host.commit_subtabs(&id, 1, migrated.clone()).is_err());
    assert_eq!(signal.revision(), observed);
    let path = root.join("project-workspace.json");
    let bytes = std::fs::read(&path).unwrap();
    let mut invalid = selected.clone();
    invalid.tabs[1].id = "a".repeat(64);
    assert!(host.commit_subtabs(&id, 2, invalid).is_err());
    assert_eq!(signal.revision(), observed);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    drop(host);
    let mut restored = pool(&root);
    assert_eq!(restored.subtab_state(&id).unwrap(), selected);
    assert!(restored.migrate_legacy_subtabs(&id, &old, 0).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(
        restored.subtab_state(&id).unwrap().active_id,
        selected.active_id
    );
}

#[test]
fn two_hosts_cannot_overwrite_each_others_subtab_commit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut first = pool(&root);
    let id = first.active_context().project_id.clone();
    first
        .migrate_legacy_subtabs(&id, &legacy(&root), 0)
        .unwrap();
    let mut second = pool(&root);
    let signal = zenpi::remote::wakeup::watch(&first.active_context().session_id).unwrap();
    let observed = signal.revision();
    let state = first.subtab_state(&id).unwrap();
    let winner = state.rename(&state.tabs[1].id, "winner").unwrap();
    first
        .commit_subtabs(&id, state.revision, winner.clone())
        .unwrap();
    assert_ne!(signal.revision(), observed);
    let observed = signal.revision();
    let bytes = std::fs::read(root.join("project-workspace.json")).unwrap();
    let loser = state.rename(&state.tabs[1].id, "loser").unwrap();
    assert!(second.commit_subtabs(&id, state.revision, loser).is_err());
    assert_eq!(signal.revision(), observed);
    assert_eq!(second.subtab_state(&id).unwrap(), state);
    assert_eq!(first.subtab_state(&id).unwrap(), winner);
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        bytes
    );
}

#[test]
fn malformed_migrations_and_checkpoint_identity_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let id = host.active_context().project_id.clone();
    let old = legacy(&root);
    assert!(host.migrate_legacy_subtabs(&id, &old, 3).is_err());
    assert!(!root.join("project-workspace.json").exists());
    for change in 0..5 {
        let mut invalid = old.clone();
        match change {
            0 => invalid[1].root = invalid[0].root.clone(),
            1 => invalid[1].concurrency = 65,
            2 => invalid[1].kind = SubtabKind::Main,
            3 => invalid[0].root = root.join("different").display().to_string(),
            _ => invalid[1].root = root.join("../escape").display().to_string(),
        }
        assert!(host.migrate_legacy_subtabs(&id, &invalid, 0).is_err());
    }
    host.migrate_legacy_subtabs(&id, &old, 1).unwrap();
    let path = root.join("project-workspace.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["subtabs"][&id]["active_id"] = json!("unknown");
    let corrupt = serde_json::to_vec(&value).unwrap();
    std::fs::write(&path, &corrupt).unwrap();
    let owner = Agent::with_echo(
        SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
    );
    let mut fresh = ProjectOwnerPool::new(Arc::new(Mutex::new(owner))).unwrap();
    assert!(fresh.restore_checkpoint().is_err());
    assert_eq!(fresh.subtab_state(&id).unwrap().tabs.len(), 1);
    assert_eq!(std::fs::read(&path).unwrap(), corrupt);
}

#[test]
fn project_selection_preserves_subtabs_and_project_close_removes_only_its_domain() {
    if isolated_configuration(
        "project_selection_preserves_subtabs_and_project_close_removes_only_its_domain",
    ) {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    let mut host = pool(&root);
    let initial = host.active_context().project_id.clone();
    host.migrate_legacy_subtabs(&initial, &legacy(&root), 2)
        .unwrap();
    let before = host.subtab_state(&initial).unwrap();
    host.open(Some(&child)).unwrap();
    let next = host.active_context().project_id.clone();
    assert_eq!(host.subtab_state(&initial).unwrap(), before);
    host.migrate_legacy_subtabs(&next, &legacy(&child), 1)
        .unwrap();
    host.select(&initial).unwrap();
    host.close(&next).unwrap();
    assert_eq!(host.subtab_state(&initial).unwrap(), before);
    assert!(host.subtab_state(&next).is_err());
    let restored = pool(&root);
    assert_eq!(restored.subtab_state(&initial).unwrap(), before);
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("project-workspace.json")).unwrap())
            .unwrap();
    assert!(value["subtabs"].get(&next).is_none());
}

#[test]
fn workspace_commits_wake_retained_and_removed_sessions_but_noops_and_failed_cas_do_not() {
    if isolated_configuration(
        "workspace_commits_wake_retained_and_removed_sessions_but_noops_and_failed_cas_do_not",
    ) {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    let mut host = pool(&root);
    let initial = host.active_context().project_id.clone();
    let initial_signal = zenpi::remote::wakeup::watch(&host.active_context().session_id).unwrap();
    let unrelated_root = tempfile::tempdir().unwrap();
    let unrelated = pool(unrelated_root.path());
    let foreign = zenpi::remote::wakeup::watch(&unrelated.active_context().session_id).unwrap();
    let observed = initial_signal.revision();
    host.open(Some(&child)).unwrap();
    assert_ne!(initial_signal.revision(), observed);
    let next = host.active_context().project_id.clone();
    let next_signal = zenpi::remote::wakeup::watch(&host.active_context().session_id).unwrap();
    let before = (initial_signal.revision(), next_signal.revision());
    host.select(&next).unwrap();
    assert_eq!((initial_signal.revision(), next_signal.revision()), before);
    host.select(&initial).unwrap();
    assert_ne!(initial_signal.revision(), before.0);
    assert_ne!(next_signal.revision(), before.1);
    // A local subtab change has no workspace revision; unrelated projects stay asleep.
    let before = (initial_signal.revision(), next_signal.revision());
    host.migrate_legacy_subtabs(&initial, &legacy(&root), 1)
        .unwrap();
    assert_ne!(initial_signal.revision(), before.0);
    assert_eq!(next_signal.revision(), before.1);
    let before = (initial_signal.revision(), next_signal.revision());
    host.close(&next).unwrap();
    assert_ne!(initial_signal.revision(), before.0);
    assert_ne!(next_signal.revision(), before.1);
    assert_eq!(foreign.revision(), 0);
    let before = initial_signal.revision();
    assert!(host.select("missing").is_err());
    assert_eq!(initial_signal.revision(), before);
    // A competing host wins the checkpoint CAS without changing this pool's state.
    let mut competitor = pool(&root);
    competitor.open(Some(&child)).unwrap();
    let before = initial_signal.revision();
    let durable = std::fs::read(root.join("project-workspace.json")).unwrap();
    assert!(host.open(Some(&child)).is_err());
    assert_eq!(initial_signal.revision(), before);
    assert_eq!(host.workspace().tabs().len(), 1);
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        durable
    );
    assert_eq!(foreign.revision(), 0);
}

#[test]
fn invalid_project_configuration_cannot_publish_workspace_or_subtab_selection() {
    if isolated_configuration(
        "invalid_project_configuration_cannot_publish_workspace_or_subtab_selection",
    ) {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir_all(child.join(".zenpi")).unwrap();
    let overlay = child.join(".zenpi/config.toml");
    let malformed = "unsupported_project_config_key=true\n";
    std::fs::write(&overlay, malformed).unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    host.migrate_legacy_subtabs(&project, &legacy(&root), 2)
        .unwrap();
    let before = host.subtab_state(&project).unwrap();
    let revision = host.workspace_revision();
    let path = root.join("project-workspace.json");
    let durable = std::fs::read(&path).unwrap();
    let session = host.active_context().session_id.clone();
    let signal = zenpi::remote::wakeup::watch(&session).unwrap();
    let observed = signal.revision();
    let error = host.open(Some(&child)).unwrap_err();
    assert!(error.contains("unsupported_project_config_key"), "{error}");
    assert_eq!(host.workspace().tabs().len(), 1);
    assert_eq!(host.active_context().project_id, project);
    assert_eq!(host.active_context().session_id, session);
    assert_eq!(host.subtab_state(&project).unwrap(), before);
    assert_eq!(host.workspace_revision(), revision);
    assert_eq!(signal.revision(), observed);
    assert_eq!(std::fs::read(path).unwrap(), durable);
    assert_eq!(std::fs::read_to_string(&overlay).unwrap(), malformed);
    // Fixing only the overlay permits the same real preparation path.
    std::fs::write(overlay, "model='project-fixture'\n").unwrap();
    host.open(Some(&child)).unwrap();
    assert_ne!(host.active_context().project_id, project);
    assert_ne!(signal.revision(), observed);
    assert_eq!(host.subtab_state(&project).unwrap(), before);
}

#[test]
fn production_desktop_migrates_view_once_and_publishes_local_rename_select_and_quota() {
    use zenpi::tui::{ProjectRuntimeHost, ProjectTabMetadata, TuiState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let session = SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap();
    let metadata = ProjectTabMetadata::from_session(&session);
    let owner = Arc::new(Mutex::new(Agent::with_echo(session)));
    let mut view = TuiState::default();
    view.set_active_project_metadata(metadata);
    let mut desktop = ProjectRuntimeHost::new(&mut view, owner.clone()).unwrap();
    assert!(view.subtab_add_in_place(Some("one".into())));
    assert!(view.subtab_add_in_place(Some("two".into())));
    desktop.restore_shared_workspace(&mut view).unwrap();
    let project = view.active_project().to_owned();
    let migrated = desktop.host_subtab_state(&project).unwrap();
    assert_eq!(migrated.tabs.len(), 3);
    assert_eq!(migrated.active_id, migrated.tabs[2].id);
    let identity = migrated.tabs[2].id.clone();
    assert!(view.subtab_rename(2, "new-name"));
    assert!(view.set_subtab_concurrency(2, 7));
    assert!(view.subtab_move(2, 1));
    desktop.sync_subtab_view(&mut view).unwrap();
    let committed = desktop.host_subtab_state(&project).unwrap();
    assert_eq!(committed.tabs[1].id, identity);
    assert_eq!(committed.tabs[1].name, "new-name");
    assert_eq!(committed.tabs[1].concurrency, 7);
    assert_eq!(committed.active_id, identity);
    assert_eq!(view.worker_concurrency(), 7);
    desktop.sync_subtab_view(&mut view).unwrap();
    assert_eq!(desktop.host_subtab_state(&project).unwrap(), committed);
    // A stale UI checkpoint cannot replace the accepted host-owned domain on restart.
    let mut stale_view = TuiState::default();
    stale_view.set_active_project_metadata(ProjectTabMetadata::from_session(
        owner.lock().unwrap().session(),
    ));
    let mut restarted = ProjectRuntimeHost::new(&mut stale_view, owner).unwrap();
    restarted.restore_shared_workspace(&mut stale_view).unwrap();
    assert_eq!(restarted.host_subtab_state(&project).unwrap(), committed);
    assert_eq!(stale_view.active_subtab(), 1);
    assert_eq!(stale_view.subtabs()[1].name, "new-name");
    assert_eq!(stale_view.worker_concurrency(), 7);
}

#[test]
fn desktop_failed_host_commit_rolls_back_view_and_keeps_external_checkpoint() {
    use zenpi::tui::{ProjectRuntimeHost, ProjectTabMetadata, TuiState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let session = SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap();
    let mut view = TuiState::default();
    view.set_active_project_metadata(ProjectTabMetadata::from_session(&session));
    let mut desktop =
        ProjectRuntimeHost::new(&mut view, Arc::new(Mutex::new(Agent::with_echo(session))))
            .unwrap();
    desktop.restore_shared_workspace(&mut view).unwrap();
    let project = view.active_project().to_owned();
    let before = desktop.host_subtab_state(&project).unwrap();
    let path = root.join("project-workspace.json");
    let bytes = std::fs::read(&path).unwrap();
    let mut external = bytes.clone();
    external.push(b' ');
    std::fs::write(&path, &external).unwrap();
    assert!(view.subtab_rename(0, "changed"));
    assert!(desktop.sync_subtab_view(&mut view).is_err());
    assert_eq!(view.subtabs()[0].name, before.tabs[0].name);
    assert_eq!(desktop.host_subtab_state(&project).unwrap(), before);
    assert_eq!(std::fs::read(&path).unwrap(), external);
}

#[test]
fn subtab_resource_limits_and_revision_exhaustion_do_not_mutate_the_candidate() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut old = legacy(&root);
    for index in old.len()..zenpi::worktree::subtab::MAX_SUBTABS {
        old.push(LegacySubtab {
            name: format!("tab-{index}"),
            root: root.join(format!("planned-{index}")).display().to_string(),
            kind: SubtabKind::InPlace,
            concurrency: 1,
        });
    }
    let mut state = SubtabState::migrate("project", &root, &old, 0).unwrap();
    old.push(LegacySubtab {
        name: "overflow".into(),
        root: root.join("overflow").display().to_string(),
        kind: SubtabKind::InPlace,
        concurrency: 1,
    });
    assert!(SubtabState::migrate("project", &root, &old, 0).is_err());
    state.revision = u64::MAX;
    assert!(state.rename(&state.tabs[1].id, "different").is_err());
    assert!(state.select(&state.tabs[1].id).is_err());
    assert_eq!(state.select(&state.active_id).unwrap(), state);
    assert_eq!(state.revision, u64::MAX);
}

#[test]
fn directory_close_preserves_selected_identity_and_never_accepts_git_worktree() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut state = SubtabState::migrate("project", &root, &legacy(&root), 2).unwrap();
    let selected = state.active_id.clone();
    let closed = state.close_directory(&state.tabs[1].id).unwrap();
    assert_eq!(closed.active_id, selected);
    assert_eq!(closed.tabs.len(), 2);
    state.tabs[1].kind = SubtabKind::Worktree;
    assert!(state.close_directory(&state.tabs[1].id).is_err());
    assert!(state.close_directory(&state.tabs[0].id).is_err());
}
