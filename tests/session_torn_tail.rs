#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{fs, io::Write};
use tempfile::tempdir;
use zenpi::{
    core::{Turn, TurnRole},
    session::{
        SessionLifecycleReceipt as Receipt, SessionLifecycleRequest as Request, SessionStore,
        execute_session_lifecycle, resume_last_session, session_catalog,
        session_catalog_with_skipped,
    },
};

const FRAGMENT: &[u8] = b"{\"kind\":\"turn\",\"turn\":{\"id\":\"half";

fn append_raw(path: &std::path::Path, bytes: &[u8]) {
    fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
}

#[test]
fn torn_tail_is_quarantined_and_startup_appends_again() {
    let workspace = tempdir().unwrap();
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let mut store = SessionStore::open_in_workspace(&path, workspace.path()).unwrap();
    store
        .append_turn(Turn::new("u1", TurnRole::User, "complete"))
        .unwrap();
    drop(store);
    let clean = fs::read(&path).unwrap();
    append_raw(&path, FRAGMENT);
    assert_eq!(
        SessionStore::open_existing(&path)
            .unwrap()
            .recovery_warnings()
            .len(),
        1
    );

    let mut reopened = SessionStore::open_in_workspace(&path, workspace.path()).unwrap();
    assert!(reopened.recovery_warnings().is_empty());
    assert_eq!(reopened.turns().len(), 1);
    reopened
        .append_turn(Turn::new("u2", TurnRole::User, "after crash"))
        .unwrap();

    let torn = dir.path().join("session.jsonl.torn");
    let quarantined = fs::read(&torn).unwrap();
    assert!(quarantined.starts_with(FRAGMENT));
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(&torn).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let bytes = fs::read(&path).unwrap();
    assert!(bytes.starts_with(&clean));
    let recovered = SessionStore::open_existing(&path).unwrap();
    assert!(recovered.recovery_warnings().is_empty());
    assert_eq!(recovered.turns().len(), 2);
}

#[test]
fn corruption_before_the_final_line_is_still_rejected() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let mut store = SessionStore::open(&path).unwrap();
    store
        .append_turn(Turn::new("u1", TurnRole::User, "complete"))
        .unwrap();
    drop(store);
    append_raw(&path, b"not json\n");
    append_raw(&path, FRAGMENT);
    let before = fs::read(&path).unwrap();
    assert!(SessionStore::open_existing_writable(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(!dir.path().join("session.jsonl.torn").exists());
}

#[test]
fn one_corrupt_sibling_does_not_break_catalog_or_resume_last() {
    let dir = tempdir().unwrap();
    let good = dir.path().join("a-good.jsonl");
    let mut store = SessionStore::open(&good).unwrap();
    store
        .append_turn(Turn::new("u1", TurnRole::User, "keep me"))
        .unwrap();
    let good_id = store.session_id().to_owned();
    drop(store);
    let corrupt = dir.path().join("b-corrupt.jsonl");
    let mut other = SessionStore::open(&corrupt).unwrap();
    other
        .append_turn(Turn::new("u1", TurnRole::User, "damaged"))
        .unwrap();
    drop(other);
    append_raw(&corrupt, b"garbage in the middle\n");
    SessionStore::open(&corrupt)
        .unwrap()
        .append_turn(Turn::new("u2", TurnRole::User, "later"))
        .unwrap();

    let (entries, skipped) = session_catalog_with_skipped(dir.path(), false).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].path, corrupt);
    assert_eq!(session_catalog(dir.path(), true).unwrap().len(), 1);

    let resumed = resume_last_session(dir.path()).unwrap();
    assert_eq!(resumed.session_id(), good_id);
    let receipt = execute_session_lifecycle(
        Request::ResumeLast {
            directory: dir.path().into(),
        },
        None,
        1,
    )
    .unwrap();
    assert!(matches!(receipt, Receipt::Session { .. }));
}

#[test]
fn resume_last_repairs_a_torn_latest_session() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("only.jsonl");
    let mut store = SessionStore::open(&path).unwrap();
    store
        .append_turn(Turn::new("u1", TurnRole::User, "complete"))
        .unwrap();
    drop(store);
    append_raw(&path, FRAGMENT);
    let (entries, skipped) = session_catalog_with_skipped(dir.path(), false).unwrap();
    assert_eq!((entries.len(), skipped.len()), (1, 0));
    let mut resumed = resume_last_session(dir.path()).unwrap();
    resumed
        .append_turn(Turn::new("u2", TurnRole::User, "resumed"))
        .unwrap();
    assert!(dir.path().join("only.jsonl.torn").is_file());
}
