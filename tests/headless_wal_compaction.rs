//! ZE-303: online WAL compaction keeps the file bounded and terminal replay
//! working across a restart. A tiny threshold forces several rewrites.
#[path = "support/sequential_host.rs"]
mod sequential_host;

use sequential_host::run_headless_sequential;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;
use zenpi::{core::Agent, session::SessionStore};

#[test]
fn compaction_bounds_the_wal_and_keeps_terminal_replay() {
    // SAFETY: set before any host starts; this binary runs one test.
    unsafe { std::env::set_var("ZENPI_WAL_COMPACT_BYTES", "65536") };
    let directory = tempdir().unwrap();
    let path = directory.path().join("compact.jsonl");
    let mut input = Vec::new();
    for index in 0..400 {
        input.extend_from_slice(
            format!("{{\"schema_version\":2,\"type\":\"prompt\",\"id\":\"p{index}\",\"text\":\"message number {index}\"}}\n")
                .as_bytes(),
        );
    }
    let agent = Agent::with_echo(SessionStore::open(&path).unwrap());
    run_headless_sequential(agent, input, &mut Vec::new()).unwrap();
    // Every prompt appended a durable reservation; compaction keeps only live
    // ids and the bounded terminal cache, so most of them must be gone.
    let wal = fs::read_to_string(directory.path().join("compact.jsonl.reconnect")).unwrap();
    let reserved = wal.matches("\"type\":\"reserved\"").count();
    let terminals = wal.matches("\"type\":\"terminal\"").count();
    assert!(
        reserved < 400,
        "compaction dropped settled reservations: {reserved}"
    );
    assert!(
        terminals < 400,
        "compaction dropped evicted terminals: {terminals}"
    );

    // A restarted host still replays the last terminal for its request id.
    let agent = Agent::with_echo(SessionStore::open(&path).unwrap());
    let mut output = Vec::new();
    run_headless_sequential(
        agent,
        b"{\"schema_version\":2,\"type\":\"prompt\",\"id\":\"p399\",\"text\":\"message number 399\"}\n",
        &mut output,
    )
    .unwrap();
    let response: Value = String::from_utf8_lossy(&output)
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .find(|record| record["type"] == "response")
        .unwrap();
    assert_eq!(response["id"], "p399");
    assert_eq!(response["success"], true);
}
