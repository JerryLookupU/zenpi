#![cfg(feature = "remote")]

use std::{
    process::Command,
    sync::{Arc, Mutex},
};
use zenpi::{core::Agent, project_workspace::ProjectOwnerPool, session::SessionStore};

fn inspect(
    root: &std::path::Path,
    workspace: &std::path::Path,
    session: &std::path::Path,
    extra: &[&str],
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_zenpi"))
        .args(["host", "pair", "--list-projects", "--workspace"])
        .arg(workspace)
        .arg("--session")
        .arg(session)
        .arg("--identity")
        .arg(root.join("must-not-create/identity.json"))
        .args(extra)
        .env("ZENPI_HOME", root.join("missing-provider-config"))
        .env_remove("OPENAI_API_KEY")
        .output()
        .unwrap()
}

#[test]
fn production_cli_lists_bootstrap_project_without_identity_provider_or_disk_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let workspace = root.join("workspace with spaces");
    std::fs::create_dir(&workspace).unwrap();
    let session = root.join("session.jsonl");
    SessionStore::open_in_workspace(&session, &workspace).unwrap();
    let before = std::fs::read(&session).unwrap();
    let output = inspect(&root, &workspace, &session, &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let expected = zenpi::project_workspace::ProjectWorkspace::default()
        .with_directory(Some(&workspace), &workspace)
        .unwrap()
        .0;
    assert_eq!(
        actual,
        serde_json::json!({"projects":[{"id":expected.active().unwrap().id().as_str(),"cwd":workspace,"active":true}]})
    );
    assert_eq!(std::fs::read(&session).unwrap(), before);
    assert!(!root.join("project-workspace.json").exists());
    assert!(!root.join("must-not-create").exists());
    assert!(!root.join("missing-provider-config").exists());
}

#[test]
fn production_cli_lists_saved_projects_with_stable_ids_and_preserves_live_owner_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let second = root.join("second");
    std::fs::create_dir(&second).unwrap();
    let session = root.join("session.jsonl");
    let owner = Arc::new(Mutex::new(Agent::with_echo(
        SessionStore::open_in_workspace(&session, &root).unwrap(),
    )));
    let mut pool = ProjectOwnerPool::new(owner).unwrap();
    pool.restore_checkpoint().unwrap();
    let (candidate, _) = pool
        .workspace()
        .with_directory(Some(&second), &root)
        .unwrap();
    pool.publish(candidate.clone(), &Default::default())
        .unwrap();
    let checkpoint = root.join("project-workspace.json");
    let before = std::fs::read(&checkpoint).unwrap();
    let journal = std::fs::read(&session).unwrap();
    let output = inspect(&root, &root, &session, &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let entries = actual["projects"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    for (entry, tab) in entries.iter().zip(candidate.tabs()) {
        assert_eq!(entry["id"], tab.id().as_str());
        assert_eq!(entry["cwd"], tab.cwd().to_str().unwrap());
        assert_eq!(
            entry["active"],
            candidate.active().unwrap().id() == tab.id()
        );
        assert_eq!(entry.as_object().unwrap().len(), 3);
    }
    assert_eq!(std::fs::read(&checkpoint).unwrap(), before);
    assert_eq!(std::fs::read(session).unwrap(), journal);
    assert!(!root.join("must-not-create").exists());
    let saved: serde_json::Value = serde_json::from_slice(&before).unwrap();
    for change in 0..4 {
        let mut malformed = saved.clone();
        let first = candidate.tabs()[0].id().as_str();
        match change {
            0 => {
                malformed["sessions"][first] = serde_json::json!("/different/session.jsonl");
            }
            1 => {
                malformed["sessions"].as_object_mut().unwrap().remove(first);
            }
            2 => {
                malformed["sessions"][candidate.tabs()[1].id().as_str()] =
                    serde_json::json!("relative/session.jsonl");
            }
            _ => {
                malformed["workspace"]["tabs"][0]["id"] = serde_json::json!("a".repeat(64));
            }
        }
        let bytes = serde_json::to_vec(&malformed).unwrap();
        std::fs::write(&checkpoint, &bytes).unwrap();
        let refused = inspect(&root, &root, &root.join("session.jsonl"), &[]);
        assert!(!refused.status.success());
        assert!(refused.stdout.is_empty());
        assert_eq!(std::fs::read(&checkpoint).unwrap(), bytes);
    }
}

#[test]
fn inspection_refuses_missing_wrong_session_and_corrupt_or_linked_checkpoint_without_repair() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let session = root.join("session.jsonl");
    let missing = inspect(&root, &root, &session, &[]);
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    assert!(!session.exists());
    SessionStore::open_in_workspace(&session, &root).unwrap();
    let other = root.join("other");
    std::fs::create_dir(&other).unwrap();
    assert!(!inspect(&root, &other, &session, &[]).status.success());
    let checkpoint = root.join("project-workspace.json");
    for bytes in [
        b"invalid".to_vec(),
        serde_json::to_vec(&serde_json::json!({"schema_version":99})).unwrap(),
        vec![b' '; 512 * 1024 + 1],
    ] {
        std::fs::write(&checkpoint, &bytes).unwrap();
        let output = inspect(&root, &root, &session, &[]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(std::fs::read(&checkpoint).unwrap(), bytes);
    }
    #[cfg(unix)]
    {
        std::fs::remove_file(&checkpoint).unwrap();
        std::os::unix::fs::symlink(&session, &checkpoint).unwrap();
        let before = std::fs::read(&session).unwrap();
        assert!(!inspect(&root, &root, &session, &[]).status.success());
        assert_eq!(std::fs::read(&session).unwrap(), before);
        assert!(
            std::fs::symlink_metadata(&checkpoint)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let alias = root.join("session-link.jsonl");
        std::os::unix::fs::symlink(&session, &alias).unwrap();
        assert!(!inspect(&root, &root, &alias, &[]).status.success());
        assert_eq!(std::fs::read(&session).unwrap(), before);
    }
    assert!(!root.join("must-not-create").exists());
}

#[test]
fn guide_uses_current_cli_actions_and_resolves_local_document_links() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide =
        std::fs::read_to_string(root.join("Docs/stage2_android2unix_user_guide.md")).unwrap();
    for action in ["--list-projects", "--list-devices", "--revoke-device"] {
        assert!(guide.contains(action));
    }
    assert!(!guide.contains("--grant-device"));
    assert!(!guide.to_lowercase().contains("safety code"));
    for args in [
        vec![
            "host",
            "pair",
            "--desktop",
            "--workspace",
            "/workspace",
            "--session",
            "/session.jsonl",
        ],
        vec![
            "host",
            "pair",
            "--desktop",
            "--workspace",
            "/workspace",
            "--session",
            "/session.jsonl",
            "--bind",
            "192.168.50.14:7777",
        ],
        vec!["host", "pair", "--list-devices"],
        vec![
            "host",
            "pair",
            "--list-projects",
            "--workspace",
            "/workspace",
            "--session",
            "/session.jsonl",
        ],
        vec!["host", "pair", "--revoke-device", &"a".repeat(64)],
    ] {
        assert!(zenpi::core::parse_args(args).is_ok());
    }
    for document in [
        "Zenpi_Execution_Blueprint_v2.md",
        "stage2_android2unix_android.md",
        "stage2_android2unix_protocol.md",
    ] {
        assert!(guide.contains(&format!("]({document})")));
        assert!(root.join("Docs").join(document).is_file());
    }
    let help = Command::new(env!("CARGO_BIN_EXE_zenpi"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    for flag in ["--list-projects", "--list-devices", "--revoke-device"] {
        assert!(help.contains(flag));
    }
    assert!(!help.contains("--grant-device"));
}

#[test]
fn project_inspection_parser_refuses_combined_side_effect_actions() {
    let options = zenpi::core::parse_args([
        "host",
        "pair",
        "--list-projects",
        "--workspace",
        "/workspace",
        "--session",
        "/session.jsonl",
    ])
    .unwrap();
    assert!(options.host_pair.unwrap().list_projects);
    let device = "a".repeat(64);
    for flags in [
        vec!["--desktop"],
        vec!["--plain"],
        vec!["--list-devices"],
        vec!["--list-worktree-intents"],
        vec!["--revoke-device", &device],
        vec!["--device", &device],
    ] {
        let mut args = vec![
            "host",
            "pair",
            "--list-projects",
            "--workspace",
            "/workspace",
            "--session",
            "/session.jsonl",
        ];
        args.extend(flags);
        assert!(zenpi::core::parse_args(args).is_err());
    }
    assert!(zenpi::core::parse_args(["host", "pair", "--list-projects"]).is_err());
}
