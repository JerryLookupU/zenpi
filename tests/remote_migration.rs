use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use zenpi::{core::Agent, project_workspace::ProjectOwnerPool, session::SessionStore};

fn fresh(root: &Path) -> (ProjectOwnerPool, Arc<Mutex<Agent>>) {
    let owner = Arc::new(Mutex::new(Agent::with_echo(
        SessionStore::open_in_workspace(root.join("session.jsonl"), root).unwrap(),
    )));
    (ProjectOwnerPool::new(owner.clone()).unwrap(), owner)
}

fn legacy(root: &Path) -> (String, String, Vec<u8>) {
    let (mut host, owner) = fresh(root);
    host.restore_checkpoint().unwrap();
    let project = host.active_context().project_id.clone();
    let session = host.active_context().session_id.clone();
    host.select(&project).unwrap();
    let path = root.join("project-workspace.json");
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    saved.as_object_mut().unwrap().remove("remote_workspace");
    saved.as_object_mut().unwrap().remove("subtabs");
    let bytes = serde_json::to_vec(&saved).unwrap();
    std::fs::write(path, &bytes).unwrap();
    drop(host);
    drop(owner);
    (project, session, bytes)
}

fn admission(kind: &str, request: &str, fingerprint: &str) -> Value {
    json!({"type":kind,"device_id":"a".repeat(64),"request_id":request,
        "fingerprint":fingerprint,"state":"reserved"})
}

#[test]
fn legacy_owner_imports_all_admission_kinds_once_without_replaying_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (project, session, _) = legacy(&root);
    let path = root.join("session.jsonl");
    let mut journal = SessionStore::open_existing(&path).unwrap();
    for (index, kind) in [
        "remote_prompt_admission_v1",
        "remote_model_admission_v1",
        "remote_reasoning_admission_v1",
    ]
    .iter()
    .enumerate()
    {
        let record = admission(kind, &format!("request-{index}"), &"b".repeat(64));
        journal.append_event(record.clone()).unwrap();
        journal.append_event(record).unwrap();
    }
    let journal_bytes = std::fs::read(&path).unwrap();
    drop(journal);
    let (mut host, owner) = fresh(&root);
    assert!(host.restore_checkpoint().unwrap());
    assert_eq!(host.active_context().project_id, project);
    assert_eq!(host.active_context().session_id, session);
    assert_eq!(host.workspace_revision(), 0);
    assert!(owner.lock().unwrap().session().turns().is_empty());
    let checkpoint = root.join("project-workspace.json");
    let migrated = std::fs::read(&checkpoint).unwrap();
    let saved: Value = serde_json::from_slice(&migrated).unwrap();
    let bindings = saved["remote_workspace"]["prompt_bindings"]
        .as_array()
        .unwrap();
    assert_eq!(bindings.len(), 3);
    assert!(bindings[0].get("operation").is_none());
    assert_eq!(bindings[1]["operation"], "model_select");
    assert_eq!(bindings[2]["operation"], "reasoning_select");
    for binding in bindings {
        assert_eq!(binding["project_id"], project);
        assert_eq!(binding["session_id"], session);
    }
    assert_eq!(std::fs::read(&path).unwrap(), journal_bytes);
    drop(host);
    drop(owner);
    let (mut restarted, _) = fresh(&root);
    assert!(restarted.restore_checkpoint().unwrap());
    assert_eq!(std::fs::read(checkpoint).unwrap(), migrated);
    assert_eq!(std::fs::read(path).unwrap(), journal_bytes);
}

#[test]
fn conflicting_or_malformed_legacy_admissions_never_partially_publish_migration() {
    for mode in ["fingerprint", "operation", "invalid-device", "capacity"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (project, session, original) = legacy(&root);
        let mut journal = SessionStore::open_existing(root.join("session.jsonl")).unwrap();
        journal
            .append_event(admission(
                "remote_prompt_admission_v1",
                "same-id",
                &"b".repeat(64),
            ))
            .unwrap();
        let mut invalid = admission("remote_prompt_admission_v1", "same-id", &"b".repeat(64));
        match mode {
            "fingerprint" => invalid["fingerprint"] = json!("c".repeat(64)),
            "operation" => invalid["type"] = json!("remote_model_admission_v1"),
            "invalid-device" => invalid["device_id"] = json!("invalid"),
            _ => {
                for index in 0..128 {
                    journal
                        .append_event(admission(
                            "remote_prompt_admission_v1",
                            &format!("extra-{index}"),
                            &"b".repeat(64),
                        ))
                        .unwrap();
                }
            }
        }
        journal.append_event(invalid).unwrap();
        let journal_bytes = std::fs::read(root.join("session.jsonl")).unwrap();
        drop(journal);
        let (mut host, _) = fresh(&root);
        assert!(host.restore_checkpoint().is_err(), "{mode}");
        assert!(!host.has_workspace_checkpoint());
        assert_eq!(host.active_context().project_id, project);
        assert_eq!(host.active_context().session_id, session);
        assert_eq!(
            std::fs::read(root.join("project-workspace.json")).unwrap(),
            original
        );
        assert_eq!(
            std::fs::read(root.join("session.jsonl")).unwrap(),
            journal_bytes
        );
    }
}

#[test]
fn interrupted_migration_preserves_old_checkpoint_and_can_retry_without_loss() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (_, _, original) = legacy(&root);
    let mut journal = SessionStore::open_existing(root.join("session.jsonl")).unwrap();
    journal
        .append_event(admission(
            "remote_model_admission_v1",
            "reserved",
            &"b".repeat(64),
        ))
        .unwrap();
    drop(journal);
    let temporary = root.join(format!("project-workspace.{}.tmp", std::process::id()));
    std::fs::write(&temporary, b"interrupted partial checkpoint").unwrap();
    let (mut host, _) = fresh(&root);
    assert!(host.restore_checkpoint().is_err());
    assert!(!host.has_workspace_checkpoint());
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        original
    );
    assert_eq!(
        std::fs::read(&temporary).unwrap(),
        b"interrupted partial checkpoint"
    );
    std::fs::remove_file(&temporary).unwrap();
    assert!(host.restore_checkpoint().unwrap());
    let saved: Value =
        serde_json::from_slice(&std::fs::read(root.join("project-workspace.json")).unwrap())
            .unwrap();
    assert_eq!(
        saved["remote_workspace"]["prompt_bindings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(!temporary.exists());
}

#[test]
fn missing_checkpoint_recovers_journal_bindings_without_recreating_execution() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (project, session, _) = legacy(&root);
    std::fs::remove_file(root.join("project-workspace.json")).unwrap();
    let mut journal = SessionStore::open_existing(root.join("session.jsonl")).unwrap();
    journal
        .append_event(admission(
            "remote_reasoning_admission_v1",
            "previous-reservation",
            &"b".repeat(64),
        ))
        .unwrap();
    let bytes = std::fs::read(root.join("session.jsonl")).unwrap();
    drop(journal);
    let (mut host, owner) = fresh(&root);
    assert!(!host.restore_checkpoint().unwrap());
    assert!(host.has_workspace_checkpoint());
    assert_eq!(host.active_context().project_id, project);
    assert_eq!(host.active_context().session_id, session);
    assert!(owner.lock().unwrap().session().turns().is_empty());
    let saved: Value =
        serde_json::from_slice(&std::fs::read(root.join("project-workspace.json")).unwrap())
            .unwrap();
    assert_eq!(
        saved["remote_workspace"]["prompt_bindings"][0]["operation"],
        "reasoning_select"
    );
    assert_eq!(
        saved["remote_workspace"]["prompt_bindings"][0]["request_id"],
        "previous-reservation"
    );
    assert_eq!(std::fs::read(root.join("session.jsonl")).unwrap(), bytes);
}

#[cfg(unix)]
#[test]
fn another_host_migration_lock_refuses_publication_until_explicit_retry() {
    use std::os::fd::AsRawFd;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (_, _, original) = legacy(&root);
    let mut journal = SessionStore::open_existing(root.join("session.jsonl")).unwrap();
    journal
        .append_event(admission(
            "remote_model_admission_v1",
            "migration-lock",
            &"b".repeat(64),
        ))
        .unwrap();
    drop(journal);
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join("project-workspace.lock"))
        .unwrap();
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    let (mut host, _) = fresh(&root);
    assert!(
        host.restore_checkpoint()
            .unwrap_err()
            .contains("being updated")
    );
    assert!(!host.has_workspace_checkpoint());
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        original
    );
    assert_eq!(unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_UN) }, 0);
    assert!(host.restore_checkpoint().unwrap());
    assert!(host.has_workspace_checkpoint());
    let mode = std::os::unix::fs::PermissionsExt::mode(
        &std::fs::metadata(root.join("project-workspace.json"))
            .unwrap()
            .permissions(),
    );
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn unsupported_or_corrupt_checkpoint_is_preserved_for_explicit_recovery() {
    for mode in ["version", "unknown-field", "session-map"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (project, _, original) = legacy(&root);
        let mut saved: Value = serde_json::from_slice(&original).unwrap();
        match mode {
            "version" => saved["schema_version"] = json!(99),
            "unknown-field" => saved["future_state"] = json!({"secret":"preserve"}),
            _ => saved["sessions"][&project] = json!("relative-session.jsonl"),
        }
        let bytes = serde_json::to_vec(&saved).unwrap();
        let path = root.join("project-workspace.json");
        std::fs::write(&path, &bytes).unwrap();
        let (mut host, _) = fresh(&root);
        assert!(host.restore_checkpoint().is_err());
        assert!(!host.has_workspace_checkpoint());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}
