#![cfg(feature = "remote")]

use serde_json::json;
use zenpi::{
    core::{Turn, TurnRole},
    remote::{
        events::{REPLAY_WINDOW, cursor, replay},
        protocol::{ErrorCode, EventCursor, Message},
    },
    session::SessionStore,
};

#[test]
fn transcript_cursor_is_bounded_by_journal_before_and_after_recovery_and_branch_changes() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("cursor-bound.jsonl");
    let mut session = SessionStore::open(&path).unwrap();
    for index in 0..32 {
        session
            .append_event(json!({"type":"private_fixture","index":index}))
            .unwrap();
        assert!(cursor(&session).next_sequence <= session.next_sequence());
        session
            .append_turn(Turn::new(
                format!("turn-{index}"),
                TurnRole::Assistant,
                "visible",
            ))
            .unwrap();
        assert_eq!(cursor(&session).next_sequence, index + 1);
        assert!(cursor(&session).next_sequence < session.next_sequence());
    }
    drop(session);
    let mut session = SessionStore::open_existing_writable(&path).unwrap();
    assert_eq!(cursor(&session).next_sequence, 32);
    assert!(cursor(&session).next_sequence < session.next_sequence());
    session.enable_tree(Default::default(), &|| false).unwrap();
    let leaf = session.active_tree_leaf().unwrap().to_owned();
    session.select_tree_leaf(None, &|| false).unwrap();
    session
        .append_turn(Turn::new(
            "alternate",
            TurnRole::Assistant,
            "alternate branch",
        ))
        .unwrap();
    session.select_tree_leaf(Some(&leaf), &|| false).unwrap();
    assert_eq!(cursor(&session).next_sequence, 33);
    assert!(cursor(&session).next_sequence < session.next_sequence());
    drop(session);
    let recovered = SessionStore::open_existing(&path).unwrap();
    assert_eq!(cursor(&recovered).next_sequence, 33);
    assert!(cursor(&recovered).next_sequence < recovered.next_sequence());
}

#[test]
fn transcript_replay_exposes_bounded_public_tool_calls_without_execution_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let mut session = SessionStore::open(temp.path().join("session.jsonl")).unwrap();
    let mut turn = Turn::new("tool-turn", TurnRole::Assistant, "Inspecting");
    turn.metadata = Some(json!({"tool_calls":(0..9).map(|index| json!({
        "id":format!("call-{index}"),"name":"read_file",
        "arguments":{"path":"note.txt","api_key":"SECRET_ARGUMENT","text":"界".repeat(600)},
        "execution":{"policy_digest":"PRIVATE_POLICY","worker_binding":"PRIVATE_BINDING"}
    })).collect::<Vec<_>>(),"credential":"PRIVATE_METADATA"}));
    session.append_turn(turn.clone()).unwrap();
    let before = std::fs::read(session.path()).unwrap();
    let page = replay(
        &session,
        "project",
        &EventCursor {
            session_id: session.session_id().into(),
            next_sequence: 0,
        },
        1,
    )
    .unwrap();
    let Message::Event { data, .. } = &page.frames[0].payload else {
        panic!("event")
    };
    assert_eq!(data["tool_calls"].as_array().unwrap().len(), 8);
    assert_eq!(data["tool_calls_truncated"], true);
    for call in data["tool_calls"].as_array().unwrap() {
        assert_eq!(call.as_object().unwrap().len(), 4);
        assert_eq!(call["arguments_truncated"], true);
        assert!(call["arguments_preview"].as_str().unwrap().len() <= 512);
    }
    let encoded = data.to_string();
    for secret in [
        "SECRET_ARGUMENT",
        "PRIVATE_POLICY",
        "PRIVATE_BINDING",
        "PRIVATE_METADATA",
    ] {
        assert!(!encoded.contains(secret));
    }
    assert_eq!(page.cursor.next_sequence, 1);
    assert_eq!(std::fs::read(session.path()).unwrap(), before);
    assert_eq!(session.turns()[0].metadata, turn.metadata);
}

#[test]
fn malformed_tool_metadata_is_withheld_without_losing_transcript_progress() {
    use zenpi::remote::events::projected_turn;
    for metadata in [
        json!({"tool_calls":"raw-private-value"}),
        json!({"tool_calls":[{"id":"call","name":"read_file","arguments":{}},
            {"id":"call","name":"read_file","arguments":{}}]}),
        json!({"tool_calls":[{"id":"call","name":"read_file","arguments":{"text":"x".repeat(65537)}}]}),
    ] {
        let mut turn = Turn::new("legacy", TurnRole::Assistant, "visible");
        turn.metadata = Some(metadata);
        let projected = projected_turn(&turn);
        assert_eq!(projected["content"], "visible");
        assert!(projected.get("tool_calls").is_none());
    }
}

#[test]
fn private_pem_in_durable_turn_is_redacted_on_replay_without_rewriting_journal() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("session.jsonl");
    let content = "safe before\n-----BEGIN RSA PRIVATE KEY-----\nPRIVATE_BODY_DURABLE_8Qf7\n-----END RSA PRIVATE KEY-----\nsafe after\n";
    let mut session = SessionStore::open(&path).unwrap();
    session
        .append_turn(Turn::new("private-fixture", TurnRole::Assistant, content))
        .unwrap();
    let durable = std::fs::read(&path).unwrap();
    assert!(String::from_utf8_lossy(&durable).contains("PRIVATE_BODY_DURABLE"));
    drop(session);
    let session = SessionStore::open_existing(&path).unwrap();
    let page = replay(
        &session,
        "project",
        &EventCursor {
            session_id: session.session_id().into(),
            next_sequence: 0,
        },
        1,
    )
    .unwrap();
    assert_eq!(page.cursor.next_sequence, 1);
    assert!(!page.has_more);
    assert!(!page.snapshot_required);
    assert_eq!(page.frames.len(), 1);
    page.frames[0].validate().unwrap();
    let Message::Event { data, .. } = &page.frames[0].payload else {
        panic!("event")
    };
    let text = data["content"].as_str().unwrap();
    assert!(text.starts_with("safe before\n"));
    assert!(text.ends_with("safe after\n"));
    assert!(
        !serde_json::to_string(&page.frames)
            .unwrap()
            .contains("PRIVATE_BODY_DURABLE")
    );
    assert_eq!(session.turns()[0].content, content);
    assert_eq!(std::fs::read(&path).unwrap(), durable);
}

#[test]
fn durable_projection_reconnects_without_confusing_journal_and_event_sequences() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("session.jsonl");
    let mut session = SessionStore::open(&path).unwrap();
    session
        .append_event(
            json!({"type":"private_extension", "api_key":"DO_NOT_PROJECT", "cwd": "/private/path"}),
        )
        .unwrap();
    session
        .append_turn(Turn::new("one", TurnRole::User, "hello"))
        .unwrap();
    session
        .append_event(json!({"type":"operation", "private":"DO_NOT_PROJECT"}))
        .unwrap();
    session
        .append_turn(Turn::new("two", TurnRole::Assistant, "answer"))
        .unwrap();
    let from = EventCursor {
        session_id: session.session_id().into(),
        next_sequence: 0,
    };
    let first = replay(&session, "project", &from, 1).unwrap();
    assert_eq!(first.cursor.next_sequence, 1);
    assert!(first.has_more);
    assert_eq!(first.frames[0].sequence, Some(0));
    assert!(
        !serde_json::to_string(&first.frames)
            .unwrap()
            .contains("DO_NOT_PROJECT")
    );
    drop(session);
    let session = SessionStore::open_existing(&path).unwrap();
    assert_eq!(cursor(&session).next_sequence, 2);
    assert!(session.next_sequence() > 2);
    let second = replay(&session, "project", &first.cursor, 128).unwrap();
    assert!(!second.has_more);
    assert_eq!(second.frames.len(), 1);
    assert_eq!(second.frames[0].sequence, Some(1));
    assert_eq!(second.frames[0].turn_id.as_deref(), Some("two"));
    second.frames[0].validate().unwrap();
    let Message::Event { name, data } = &second.frames[0].payload else {
        panic!("event")
    };
    assert_eq!(name, "transcript_turn");
    assert_eq!(data["content"], "answer");
    assert!(
        replay(&session, "project", &second.cursor, 128)
            .unwrap()
            .frames
            .is_empty()
    );
}

#[test]
fn bounded_window_and_future_or_foreign_cursors_are_explicit() {
    let temp = tempfile::tempdir().unwrap();
    let mut session = SessionStore::open(temp.path().join("session.jsonl")).unwrap();
    for n in 0..REPLAY_WINDOW + 1 {
        session
            .append_turn(Turn::new(format!("turn-{n}"), TurnRole::Assistant, "small"))
            .unwrap();
    }
    let mut from = cursor(&session);
    from.next_sequence = 0;
    let gap = replay(&session, "project", &from, 128).unwrap();
    assert!(gap.snapshot_required);
    assert!(gap.frames.is_empty());
    from.next_sequence = 1;
    let page = replay(&session, "project", &from, 128).unwrap();
    assert_eq!(page.frames.len(), 128);
    assert!(page.has_more);
    assert!(!page.snapshot_required);
    from.next_sequence = cursor(&session).next_sequence + 1;
    assert!(matches!(
        replay(&session, "project", &from, 1),
        Err(ErrorCode::InvalidRequest)
    ));
    from.session_id = "foreign".into();
    assert!(matches!(
        replay(&session, "project", &from, 1),
        Err(ErrorCode::OwnerMismatch)
    ));
    assert!(matches!(
        replay(&session, "project", &cursor(&session), 0),
        Err(ErrorCode::ResourceLimit)
    ));
}

#[test]
fn oversized_turn_replays_explicit_redacted_prefix_without_rewriting_journal() {
    let temp = tempfile::tempdir().unwrap();
    let mut session = SessionStore::open(temp.path().join("session.jsonl")).unwrap();
    let mut turn = Turn::new("large", TurnRole::Assistant, "x".repeat(100 * 1024));
    turn.metadata = Some(json!({"credential":"never-on-wire"}));
    session.append_turn(turn).unwrap();
    let mut from = cursor(&session);
    from.next_sequence = 0;
    let page = replay(&session, "project", &from, 1).unwrap();
    assert!(!page.snapshot_required);
    assert_eq!(page.cursor.next_sequence, 1);
    let Message::Event { data, .. } = &page.frames[0].payload else {
        panic!("event")
    };
    assert_eq!(
        data["content"].as_str().unwrap().len(),
        zenpi::remote::events::MAX_TURN_CONTENT_BYTES
    );
    assert_eq!(data["content_truncated"], true);
    assert!(!data.to_string().contains("never-on-wire"));
    assert_eq!(session.turns()[0].content.len(), 100 * 1024);
}

#[test]
fn large_projection_redacts_before_utf8_boundary_truncation() {
    use zenpi::remote::events::{MAX_TURN_CONTENT_BYTES, projected_turn};
    let raw = format!(
        "api_key=PRIVATE_LARGE_SECRET\n{}",
        "\u{4e2d}".repeat(MAX_TURN_CONTENT_BYTES)
    );
    let turn = Turn::new("unicode", TurnRole::Assistant, raw.clone());
    let value = projected_turn(&turn);
    let text = value["content"].as_str().unwrap();
    assert!(text.len() <= MAX_TURN_CONTENT_BYTES);
    assert!(!text.contains("PRIVATE_LARGE_SECRET"));
    assert_eq!(value["content_truncated"], true);
    assert_eq!(turn.content, raw);
}

#[test]
fn tree_sessions_explicitly_replace_instead_of_replaying_abandoned_branches() {
    let temp = tempfile::tempdir().unwrap();
    let mut session = SessionStore::open(temp.path().join("tree.jsonl")).unwrap();
    session
        .append_turn(Turn::new("one", TurnRole::User, "hello"))
        .unwrap();
    session.enable_tree(Default::default(), &|| false).unwrap();
    let page = replay(&session, "project", &cursor(&session), 1).unwrap();
    assert!(page.snapshot_required);
    assert!(page.frames.is_empty());
}
