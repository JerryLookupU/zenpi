//! ZE-303: a cleanly closed session can be exported (the ledger bound is a
//! unit test in src/headless.rs).
#[path = "support/sequential_host.rs"]
mod sequential_host;

use sequential_host::run_headless_sequential;
use tempfile::tempdir;
use zenpi::{core::Agent, session::SessionStore};

#[test]
fn a_cleanly_closed_session_can_be_exported_and_deleted() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("closed.jsonl");
    let agent = Agent::with_echo(SessionStore::open(&path).unwrap());
    let mut output = Vec::new();
    run_headless_sequential(
        agent,
        b"{\"schema_version\":2,\"type\":\"prompt\",\"id\":\"p\",\"text\":\"hi\"}\n",
        &mut output,
    )
    .unwrap();
    assert!(directory.path().join("closed.jsonl.reconnect").exists());
    let store = SessionStore::open_existing(&path).unwrap();
    let exported = directory.path().join("exported.jsonl");
    store.export_to(&exported).unwrap();
    assert!(exported.exists());
    assert!(!directory.path().join("exported.jsonl.reconnect").exists());
}
