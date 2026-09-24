use std::time::Duration;
use tempfile::tempdir;
use zenpi::dag::{DagStore, send_to_worker, spawn_worker};

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
    let dir = tempdir().unwrap();
    let stub = dir.path().join("stub.sh");
    std::fs::write(&stub, "#!/bin/sh\nexec cat\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
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
    for _ in 0..60 {
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
