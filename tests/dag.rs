use std::sync::Mutex;
use std::time::Duration;
use tempfile::tempdir;
use zenpi::dag::{DagStore, send_to_worker, spawn_worker};

/// Serializes tests that set `ZENPI_DAG_WORKER` (process-global env).
static WORKER_ENV: Mutex<()> = Mutex::new(());

fn stub_worker(dir: &std::path::Path, name: &str, script: &str) -> std::path::PathBuf {
    let stub = dir.join(name);
    std::fs::write(&stub, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    stub
}

#[test]
fn dag_roles_addressing_and_mailbox_read() {
    let dir = tempdir().unwrap();
    let store = DagStore::at(dir.path().join("dag.json"));
    for (id, parent) in [
        ("root", None),
        ("a", Some("root")),
        ("b", Some("root")),
        ("a1", Some("a")),
        ("a2", Some("a")),
        ("a1x", Some("a1")),
    ] {
        store.upsert(id, parent).unwrap();
    }

    assert_eq!(store.recipients("a", "parent").unwrap(), vec!["root"]);
    assert_eq!(store.recipients("a1", "grandparent").unwrap(), vec!["root"]);
    assert_eq!(store.recipients("a1", "sibling").unwrap(), vec!["a2"]);
    assert_eq!(store.recipients("a", "child").unwrap(), vec!["a1", "a2"]);
    let all = store.recipients("a1", "all").unwrap();
    for expected in ["a", "root", "a2", "a1x"] {
        assert!(all.contains(&expected.to_string()), "{expected} in {all:?}");
    }
    assert!(!all.contains(&"a1".to_string()), "sender is never a target");

    let delivered = store.send("a1", "all", "hello").unwrap();
    assert!(delivered.contains(&"a".to_string()));
    let inbox = store.inbox("root", true).unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].from, "a1");
    assert!(
        store.inbox("root", true).unwrap().is_empty(),
        "read consumes"
    );
}

#[test]
fn dag_close_gate_requires_a_green_subtree() {
    let dir = tempdir().unwrap();
    let store = DagStore::at(dir.path().join("dag.json"));
    store.upsert("root", None).unwrap();
    store.upsert("a", Some("root")).unwrap();
    store.upsert("a1", Some("a")).unwrap();
    store.upsert("a1x", Some("a1")).unwrap();

    // Everything open: cannot close.
    let (can_close, unfinished) = store.can_close("a").unwrap();
    assert!(!can_close);
    assert!(unfinished.contains(&"a".to_string()));
    assert!(unfinished.contains(&"a1x".to_string()));

    // Node green but a grandchild red: still blocked.
    store.set_status("a", "green", None).unwrap();
    store.set_status("a1", "green", None).unwrap();
    store.set_status("a1x", "red", None).unwrap();
    let (can_close, unfinished) = store.can_close("a").unwrap();
    assert!(!can_close);
    assert_eq!(unfinished, vec!["a1x".to_string()]);

    // Whole subtree green: closable.
    store.set_status("a1x", "green", None).unwrap();
    let (can_close, unfinished) = store.can_close("a").unwrap();
    assert!(can_close, "unfinished: {unfinished:?}");
    assert!(unfinished.is_empty());
}

#[test]
fn dag_spawn_keeps_a_live_worker_stdin() {
    // A stub worker echoes its stdin to the node log, proving the parent keeps
    // the pipe and can deliver follow-up work without restarting the child.
    let _env = WORKER_ENV
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let dir = tempdir().unwrap();
    let stub = stub_worker(dir.path(), "stub.sh", "#!/bin/sh\nexec cat\n");
    unsafe {
        std::env::set_var("ZENPI_DAG_WORKER", &stub);
    }
    let store = DagStore::at(dir.path().join("dag.json"));
    store.upsert("root", None).unwrap();
    let workers = dir.path().join("workers");
    let pid = spawn_worker(&store, "child", "first task", &workers).unwrap();
    assert!(pid > 0);
    assert!(send_to_worker("child", "follow up").unwrap());

    let log = workers.join("child.log");
    let mut text = String::new();
    // Exec of a freshly written script can be held for seconds by host
    // security scanning, so allow a generous bound.
    for _ in 0..600 {
        text = std::fs::read_to_string(&log).unwrap_or_default();
        if text.contains("follow up") {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(text.contains("first task"), "log: {text}");
    assert!(text.contains("follow up"), "log: {text}");
    unsafe {
        std::env::remove_var("ZENPI_DAG_WORKER");
    }
}

#[test]
fn dag_tools_and_websearch_are_registered_builtins() {
    let registry = zenpi::tools::ToolRegistry::with_all_builtins().unwrap();
    let names: Vec<String> = registry
        .definitions()
        .into_iter()
        .map(|definition| definition.name)
        .collect();
    for name in [
        "dag_status",
        "dag_send",
        "dag_recv",
        "dag_finish",
        "dag_spawn",
        "websearch",
    ] {
        assert!(names.iter().any(|candidate| candidate == name), "{name}");
    }
}

#[test]
fn dag_relations_are_typed_and_heartbeat_reports_idle() {
    use zenpi::dag::DagRelation;
    assert_eq!(DagRelation::parse("parent"), Some(DagRelation::Parent));
    assert_eq!(DagRelation::parse("children"), Some(DagRelation::Child));
    assert_eq!(DagRelation::parse("all"), Some(DagRelation::All));
    assert_eq!(DagRelation::All.as_str(), "all");

    let dir = tempdir().unwrap();
    let store = DagStore::at(dir.path().join("dag.json"));
    store.upsert("root", None).unwrap();
    store.assign_worker("root", "zenpi-dev").unwrap();

    let before = zenpi::dag::status_view(&store, "root").unwrap();
    assert!(before["worker_idle_seconds"].is_null());

    store.touch_worker("root").unwrap();
    let after = zenpi::dag::status_view(&store, "root").unwrap();
    assert!(after["worker_idle_seconds"].as_u64().is_some());
    assert_eq!(after["worker"], "zenpi-dev");
    assert_eq!(after["relations"][0], "parent");
    assert!(store.touch_worker("missing").is_err());
}

#[test]
fn dag_claim_redelivery_and_ack_prevent_message_loss() {
    let dir = tempdir().unwrap();
    let store = DagStore::at(dir.path().join("dag.json"));
    store.upsert("root", None).unwrap();
    store.upsert("a", Some("root")).unwrap();

    store.send("root", "a", "do the thing").unwrap();
    let first = store.claim_inbox("a").unwrap();
    assert_eq!(first.len(), 1);
    let id = first[0].id;
    assert!(id > 0);
    assert_eq!(store.unread_count("a").unwrap(), 0);
    assert_eq!(store.claimed_count("a").unwrap(), 1);

    // Unacked: a restarted reader still gets the message (no silent loss).
    let second = store.claim_inbox("a").unwrap();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].id, id);

    // Another node cannot ack someone else's claim.
    assert_eq!(store.ack("root", &[id]).unwrap(), 0);
    assert_eq!(store.ack("a", &[id]).unwrap(), 1);
    assert!(store.claim_inbox("a").unwrap().is_empty());
    assert_eq!(store.claimed_count("a").unwrap(), 0);
}

#[test]
fn dag_send_full_mailbox_errors() {
    use zenpi::dag::MAX_DAG_INBOX_MESSAGES;
    let dir = tempdir().unwrap();
    let store = DagStore::at(dir.path().join("dag.json"));
    store.upsert("root", None).unwrap();
    store.upsert("a", Some("root")).unwrap();
    for index in 0..MAX_DAG_INBOX_MESSAGES {
        store.send("root", "a", &format!("m{index}")).unwrap();
    }
    let error = store.send("root", "a", "one too many").unwrap_err();
    assert!(error.contains("full"), "{error}");
    assert_eq!(store.unread_count("a").unwrap(), MAX_DAG_INBOX_MESSAGES);
    // Nothing was silently dropped: the newest accepted message is stored.
    let inbox = store.claim_inbox("a").unwrap();
    assert_eq!(inbox.len(), MAX_DAG_INBOX_MESSAGES);
    assert_eq!(
        inbox.last().unwrap().body,
        format!("m{}", MAX_DAG_INBOX_MESSAGES - 1)
    );
    // Acking frees the mailbox again.
    let ids: Vec<u64> = inbox.iter().map(|message| message.id).collect();
    store.ack("a", &ids).unwrap();
    store.send("root", "a", "after ack").unwrap();
}

#[test]
fn dag_body_limit_is_enforced_in_bytes() {
    use zenpi::dag::MAX_DAG_BODY_BYTES;
    let dir = tempdir().unwrap();
    let store = DagStore::at(dir.path().join("dag.json"));
    store.upsert("root", None).unwrap();
    store.upsert("a", Some("root")).unwrap();
    // 2-byte chars: within the char count but over the byte limit.
    let wide = "\u{e9}".repeat(MAX_DAG_BODY_BYTES);
    assert!(store.send("root", "a", &wide).is_err());
    let fits = "\u{e9}".repeat(MAX_DAG_BODY_BYTES / 2);
    store.send("root", "a", &fits).unwrap();
    assert_eq!(
        store.claim_inbox("a").unwrap()[0].body.len(),
        MAX_DAG_BODY_BYTES
    );
}

#[test]
fn dag_spawn_defaults_to_current_exe() {
    let current = std::env::current_exe().unwrap();
    assert_eq!(zenpi::dag::resolve_worker_binary(None).unwrap(), current);
    assert_eq!(
        zenpi::dag::resolve_worker_binary(Some("  ")).unwrap(),
        current
    );
    assert_eq!(
        zenpi::dag::resolve_worker_binary(Some("/opt/worker")).unwrap(),
        std::path::PathBuf::from("/opt/worker")
    );
}

#[test]
fn dag_store_survives_a_truncated_temp_write() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("dag.json");
    let store = DagStore::at(&path);
    store.upsert("root", None).unwrap();
    store.upsert("a", Some("root")).unwrap();
    store.send("root", "a", "kept").unwrap();
    assert!(!dir.path().join("dag.json.tmp").exists(), "temp is renamed");

    // A crash halfway through the next write leaves a truncated temp file.
    let committed = std::fs::read(&path).unwrap();
    std::fs::write(
        dir.path().join("dag.json.tmp"),
        &committed[..committed.len() / 2],
    )
    .unwrap();

    assert_eq!(store.nodes().unwrap().len(), 2, "store still parses");
    assert_eq!(store.unread_count("a").unwrap(), 1);
    // The next writer overwrites the stale temp file and commits normally.
    store.upsert("b", Some("root")).unwrap();
    assert_eq!(store.nodes().unwrap().len(), 3);
    serde_json::from_slice::<serde_json::Value>(&std::fs::read(&path).unwrap()).unwrap();
}

#[test]
fn dag_spawn_depth_and_fanout_are_bounded() {
    use zenpi::dag::{MAX_DAG_FANOUT, MAX_DAG_SPAWN_DEPTH};
    assert_eq!(MAX_DAG_SPAWN_DEPTH, 3);
    let dir = tempdir().unwrap();
    let store = DagStore::at(dir.path().join("dag.json"));
    store.upsert("root", None).unwrap();
    store.spawn_child("l1", Some("root")).unwrap();
    store.spawn_child("l2", Some("l1")).unwrap();
    store.spawn_child("l3", Some("l2")).unwrap();
    let error = store.spawn_child("l4", Some("l3")).unwrap_err();
    assert!(error.contains("depth"), "{error}");
    assert!(
        store.node("l4").unwrap().is_none(),
        "rejected spawn is not stored"
    );

    for index in 1..MAX_DAG_FANOUT {
        store
            .spawn_child(&format!("c{index}"), Some("root"))
            .unwrap();
    }
    let error = store.spawn_child("overflow", Some("root")).unwrap_err();
    assert!(error.contains("fan-out"), "{error}");
    // Re-spawning an existing child does not count against the fan-out.
    store.spawn_child("c1", Some("root")).unwrap();
}

#[test]
fn dag_spawned_workers_are_reaped_after_exit() {
    let _env = WORKER_ENV
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let dir = tempdir().unwrap();
    let stub = stub_worker(dir.path(), "once.sh", "#!/bin/sh\nread line\nexit 0\n");
    unsafe {
        std::env::set_var("ZENPI_DAG_WORKER", &stub);
    }
    let store = DagStore::at(dir.path().join("dag.json"));
    store.upsert("root", None).unwrap();
    let spawned = spawn_worker(&store, "reaped", "only task", &dir.path().join("workers"));
    unsafe {
        std::env::remove_var("ZENPI_DAG_WORKER");
    }
    spawned.unwrap();
    let mut gone = false;
    for _ in 0..600 {
        zenpi::dag::reap_workers();
        if !zenpi::dag::spawned_nodes().contains(&"reaped".to_string()) {
            gone = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        gone,
        "exited worker is reaped and dropped from the registry"
    );
    assert!(!send_to_worker("reaped", "late").unwrap());
}
