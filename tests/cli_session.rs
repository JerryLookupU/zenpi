//! ZE-236: pi session semantics — a fresh journal per launch kept per cwd,
//! `-c` reopens the newest one for the cwd, `--no-session` leaves nothing.
#![cfg(feature = "dev-fixtures")]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::tempdir;

/// Run one echo turn headlessly in `cwd` and return its stdout.
fn run_once(home: &Path, cwd: &Path, extra: &[&str], message: &str) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_zenpi"));
    for (key, _) in std::env::vars() {
        if key.starts_with("ZENPI_") || key.starts_with("OPENAI_") {
            command.env_remove(key);
        }
    }
    let mut child = command
        .current_dir(cwd)
        .args(["--mode", "headless", "--auto", "--backend", "echo"])
        .args(extra)
        .env("ZENPI_HOME", home)
        .env("HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Request ids are idempotency keys in a resumed session, so use a fresh one.
    let frame = format!(
        "{{\"version\":2,\"id\":\"{message}\",\"type\":\"prompt\",\"message\":\"{message}\"}}\n"
    );
    child
        .stdin
        .take()
        .unwrap()
        .write_all(frame.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "exit {:?}\nstdout: {stdout}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    stdout
}

fn journals(home: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(home.join("sessions"))
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

#[test]
fn each_launch_gets_its_own_journal_and_continue_stays_in_its_cwd() {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    let (a, b) = (root.path().join("repo-a"), root.path().join("repo-b"));
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();

    run_once(&home, &a, &[], "first-in-a");
    run_once(&home, &b, &[], "first-in-b");
    assert_eq!(journals(&home).len(), 2, "two launches, two journals");
    assert!(
        !home.join("session.jsonl").exists(),
        "no shared global journal"
    );

    // `-c` in repo-a appends to repo-a's journal and never to repo-b's.
    run_once(&home, &a, &["-c"], "second-in-a");
    let all = journals(&home);
    assert_eq!(all.len(), 2, "-c must not create a new journal");
    let text: Vec<String> = all.iter().map(|p| fs::read_to_string(p).unwrap()).collect();
    let a_journal = text.iter().find(|t| t.contains("first-in-a")).unwrap();
    let b_journal = text.iter().find(|t| t.contains("first-in-b")).unwrap();
    assert!(a_journal.contains("second-in-a"));
    assert!(!b_journal.contains("second-in-a"));
}

#[test]
fn no_session_writes_no_journal() {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    let cwd = root.path().join("repo");
    fs::create_dir_all(&cwd).unwrap();
    let stdout = run_once(&home, &cwd, &["--no-session"], "ephemeral");
    assert!(stdout.contains("ephemeral"));
    assert!(journals(&home).is_empty());
    assert!(!home.join("session.jsonl").exists());
}

#[test]
fn parallel_default_launches_never_collide_on_a_session_lock() {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    let cwd = root.path().join("repo");
    fs::create_dir_all(&cwd).unwrap();
    let handles: Vec<_> = (0..24)
        .map(|index| {
            let (home, cwd) = (home.clone(), cwd.clone());
            std::thread::spawn(move || run_once(&home, &cwd, &[], &format!("p{index}")))
        })
        .collect();
    for handle in handles {
        let stdout = handle.join().unwrap();
        assert!(!stdout.contains("session_locked"), "{stdout}");
    }
    assert_eq!(journals(&home).len(), 24);
}
