use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
use serde_json::{Value, json};
use sha2::Digest;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use zenpi::{
    core::Agent,
    project_workspace::{ProjectOwnerPool, ProjectWorkspace},
    remote::{
        auth::{
            AuthenticatedDevice, Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message,
        },
        protocol::{Channel, Envelope, ErrorCode, Message, Operation},
        workspace_api::mutate,
    },
    session::SessionStore,
};

fn pool(root: &Path) -> ProjectOwnerPool {
    let owner = Agent::with_echo(
        SessionStore::open_in_workspace(root.join("initial.jsonl"), root).unwrap(),
    );
    let mut pool = ProjectOwnerPool::new(Arc::new(Mutex::new(owner))).unwrap();
    pool.restore_checkpoint().unwrap();
    pool
}

#[test]
fn remote_session_rename_is_cas_bound_once_only_and_recovers_receipt() {
    use zenpi::remote::{host_adapter::HostAdapter, protocol::ExecutionState, sessions::rename};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let session = host.active_context().session_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let revision = host
        .owner(&project)
        .unwrap()
        .lock()
        .unwrap()
        .session()
        .next_sequence();
    let req = request(
        &host,
        Operation::SessionRename,
        &project,
        "rename-current",
        json!({"name":"Development","journal_revision":revision}),
    );
    assert_eq!(
        rename(&mut host, &registry, &reader, &req, false).code,
        Some(ErrorCode::Unauthorized)
    );
    assert_eq!(
        rename(&mut host, &registry, &phone, &req, true).code,
        Some(ErrorCode::Busy)
    );
    let mut stale = req.clone();
    stale.payload = Message::Request {
        operation: Operation::SessionRename,
        arguments: json!({"name":"Development","journal_revision":revision+1}),
    };
    assert_eq!(
        rename(&mut host, &registry, &phone, &stale, false).code,
        Some(ErrorCode::RevisionConflict)
    );
    for name in [
        "",
        " padded ",
        "bad/name",
        "bad\nname",
        "Bearer SECRET_CREDENTIAL",
    ] {
        let mut invalid = req.clone();
        invalid.payload = Message::Request {
            operation: Operation::SessionRename,
            arguments: json!({"name":name,"journal_revision":revision}),
        };
        assert_eq!(
            rename(&mut host, &registry, &phone, &invalid, false).code,
            Some(ErrorCode::InvalidRequest)
        );
    }
    let response = HostAdapter::dispatch_owned(&mut host, &registry, &phone, req.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    let owner = host.owner(&project).unwrap();
    let path = owner.lock().unwrap().session().path().to_path_buf();
    assert_eq!(
        owner.lock().unwrap().session().session_name(),
        Some("Development")
    );
    assert_eq!(host.context(&project).unwrap().session_id, session);
    assert_eq!(host.workspace_revision(), req.expected_revision.unwrap());
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(rename(&mut host, &registry, &phone, &req, true), response);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let mut changed = req.clone();
    changed.payload = Message::Request {
        operation: Operation::SessionRename,
        arguments: json!({"name":"Changed","journal_revision":revision}),
    };
    assert_eq!(
        rename(&mut host, &registry, &phone, &changed, false).code,
        Some(ErrorCode::RequestIdConflict)
    );
    owner
        .lock()
        .unwrap()
        .session_mut()
        .rename("Local rename")
        .unwrap();
    assert_eq!(rename(&mut host, &registry, &phone, &req, false), response);
    assert_eq!(
        owner.lock().unwrap().session().session_name(),
        Some("Local rename")
    );
    drop(owner);
    drop(host);
    let mut restored = pool(&root);
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        rename(&mut restored, &registry, &phone, &req, false),
        response
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let mut listing = request(
        &restored,
        Operation::WorkspaceList,
        &project,
        "list-name",
        json!({}),
    );
    listing.required_owner = None;
    listing.expected_revision = None;
    let response = HostAdapter::dispatch_owned(&mut restored, &registry, &phone, listing);
    assert!(
        matches!(response.payload, Message::Response {result} if result["projects"][0]["session_name"] == "Local rename")
    );
    let mut historical =
        SessionStore::open_in_workspace(root.join("named-history.jsonl"), &root).unwrap();
    historical.rename("Already used").unwrap();
    drop(historical);
    let revision = restored
        .owner(&project)
        .unwrap()
        .lock()
        .unwrap()
        .session()
        .next_sequence();
    let duplicate = request(
        &restored,
        Operation::SessionRename,
        &project,
        "duplicate-name",
        json!({"name":"Already used","journal_revision":revision}),
    );
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        rename(&mut restored, &registry, &phone, &duplicate, false).code,
        Some(ErrorCode::RevisionConflict)
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    restored
        .owner(&project)
        .unwrap()
        .lock()
        .unwrap()
        .session_mut()
        .rename("Bearer PRIVATE_NAME")
        .unwrap();
    assert!(
        zenpi::remote::sessions::projected_name(
            restored.owner(&project).unwrap().lock().unwrap().session()
        )
        .is_none()
    );
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        rename(&mut restored, &registry, &phone, &req, false).code,
        Some(ErrorCode::Unauthorized)
    );
}

#[test]
fn reserved_session_rename_recovers_unknown_without_repeating_effect() {
    use zenpi::remote::{protocol::ExecutionState, sessions::rename};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let owner = host.owner(&project).unwrap();
    let revision = owner.lock().unwrap().session().next_sequence();
    let req = request(
        &host,
        Operation::SessionRename,
        &project,
        "reserved-name",
        json!({"name":"Remote target","journal_revision":revision}),
    );
    let fingerprint = format!(
        "{:x}",
        sha2::Sha256::digest(serde_json::to_vec(&req).unwrap())
    );
    owner
        .lock()
        .unwrap()
        .session_mut()
        .append_event(
            json!({"type":"remote_session_name_admission_v1","device_id":phone.id(),
        "request_id":"reserved-name","fingerprint":fingerprint,"state":"reserved","result":null}),
        )
        .unwrap();
    owner
        .lock()
        .unwrap()
        .session_mut()
        .rename("Later local name")
        .unwrap();
    let path = owner.lock().unwrap().session().path().to_path_buf();
    drop(owner);
    drop(host);
    let mut restored = pool(&root);
    let bytes = std::fs::read(&path).unwrap();
    let response = rename(&mut restored, &registry, &phone, &req, false);
    assert_eq!(
        response.execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(
        restored
            .owner(&project)
            .unwrap()
            .lock()
            .unwrap()
            .session()
            .session_name(),
        Some("Later local name")
    );
}

#[test]
fn remote_session_catalog_pages_only_same_project_and_resume_replays_after_restart() {
    use zenpi::remote::{
        host_adapter::HostAdapter,
        protocol::ExecutionState,
        sessions::{listing, new_session},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let mut target = None;
    for n in 0..20 {
        let path = root.join(format!("history-{n}.jsonl"));
        let mut session = SessionStore::open_in_workspace(&path, &root).unwrap();
        session
            .append_turn(zenpi::core::Turn::new(
                format!("turn-{n}"),
                zenpi::core::TurnRole::User,
                format!("PRIVATE_HISTORY_{n}"),
            ))
            .unwrap();
        if n == 0 {
            target = Some((session.session_id().to_owned(), path));
        }
    }
    let foreign_root = root.join("foreign");
    std::fs::create_dir(&foreign_root).unwrap();
    let foreign = SessionStore::open_in_workspace(root.join("foreign.jsonl"), &foreign_root)
        .unwrap()
        .session_id()
        .to_owned();
    let mut read = request(
        &host,
        Operation::SessionList,
        &project,
        "catalog",
        json!({}),
    );
    read.required_owner = None;
    read.expected_revision = None;
    let before = std::fs::read(
        host.owner(&project)
            .unwrap()
            .lock()
            .unwrap()
            .session()
            .path(),
    )
    .unwrap();
    let first = listing(&host, &registry, &reader, &read).unwrap();
    assert_eq!(first["entries"].as_array().unwrap().len(), 16);
    assert_eq!(first["next_offset"], 16);
    assert!(!first.to_string().contains("PRIVATE_HISTORY"));
    assert!(!first.to_string().contains("path"));
    read.payload = Message::Request {
        operation: Operation::SessionList,
        arguments: json!({"offset":16,"snapshot_id":first["snapshot_id"]}),
    };
    let second = listing(&host, &registry, &reader, &read).unwrap();
    assert_eq!(second["entries"].as_array().unwrap().len(), 5);
    assert!(second["next_offset"].is_null());
    assert!(
        !first["entries"]
            .as_array()
            .unwrap()
            .iter()
            .chain(second["entries"].as_array().unwrap())
            .any(|entry| entry["session_id"] == foreign)
    );
    assert_eq!(
        std::fs::read(
            host.owner(&project)
                .unwrap()
                .lock()
                .unwrap()
                .session()
                .path()
        )
        .unwrap(),
        before
    );
    let (target, path) = target.unwrap();
    let mut stale = request(
        &host,
        Operation::SessionResume,
        &project,
        "resume-stale",
        json!({"target_session_id":target,"snapshot_id":"0".repeat(64)}),
    );
    assert_eq!(
        new_session(&mut host, &registry, &phone, &stale, false).code,
        Some(ErrorCode::RevisionConflict)
    );
    stale.payload = Message::Request {
        operation: Operation::SessionResume,
        arguments: json!({"target_session_id":foreign,"snapshot_id":first["snapshot_id"]}),
    };
    assert_eq!(
        new_session(&mut host, &registry, &phone, &stale, false).code,
        Some(ErrorCode::InvalidRequest)
    );
    let req = request(
        &host,
        Operation::SessionResume,
        &project,
        "resume",
        json!({"target_session_id":target,"snapshot_id":first["snapshot_id"]}),
    );
    assert_eq!(
        new_session(&mut host, &registry, &reader, &req, false).code,
        Some(ErrorCode::Unauthorized)
    );
    let response = HostAdapter::dispatch_owned(&mut host, &registry, &phone, req.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    assert_eq!(host.context(&project).unwrap().session_id, target);
    assert_eq!(
        host.control(&project).unwrap().input_port.session_id(),
        target
    );
    assert_eq!(
        host.owner(&project).unwrap().lock().unwrap().history()[0].content,
        "PRIVATE_HISTORY_0"
    );
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        new_session(&mut host, &registry, &phone, &req, true),
        response
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    drop(host);
    let mut restored = pool(&root);
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        new_session(&mut restored, &registry, &phone, &req, false),
        response
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    read.session_id = Some(target.clone());
    read.payload = Message::Request {
        operation: Operation::SessionList,
        arguments: json!({}),
    };
    let catalog = listing(&restored, &registry, &phone, &read).unwrap();
    std::fs::write(root.join("broken.jsonl"), "not a journal").unwrap();
    assert!(listing(&restored, &registry, &phone, &read).is_err());
    assert_eq!(
        std::fs::read_to_string(root.join("broken.jsonl")).unwrap(),
        "not a journal"
    );
    std::fs::remove_file(root.join("broken.jsonl")).unwrap();
    std::fs::write(root.join("empty.jsonl"), "").unwrap();
    assert_eq!(
        listing(&restored, &registry, &phone, &read),
        Err(ErrorCode::InvalidRequest)
    );
    assert_eq!(
        std::fs::metadata(root.join("empty.jsonl")).unwrap().len(),
        0
    );
    std::fs::remove_file(root.join("empty.jsonl")).unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&path, root.join("escape.jsonl")).unwrap();
        assert_eq!(
            listing(&restored, &registry, &phone, &read),
            Err(ErrorCode::InvalidRequest)
        );
        std::fs::remove_file(root.join("escape.jsonl")).unwrap();
    }
    let mut changed = SessionStore::open_existing_writable(&path).unwrap();
    changed
        .append_event(json!({"type":"test_catalog_change"}))
        .unwrap();
    drop(changed);
    read.payload = Message::Request {
        operation: Operation::SessionList,
        arguments: json!({"offset":16,"snapshot_id":catalog["snapshot_id"]}),
    };
    assert_eq!(
        listing(&restored, &registry, &phone, &read),
        Err(ErrorCode::RevisionConflict)
    );
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        new_session(&mut restored, &registry, &phone, &req, false).code,
        Some(ErrorCode::Unauthorized)
    );
}

#[test]
fn remote_session_name_search_is_bounded_scoped_read_only_and_page_bound() {
    use zenpi::remote::sessions::listing;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    for index in 0..20 {
        let mut session =
            SessionStore::open_in_workspace(root.join(format!("search-{index}.jsonl")), &root)
                .unwrap();
        session.rename(&format!("Search Target {index}")).unwrap();
        session
            .append_turn(zenpi::core::Turn::new(
                format!("paged-{index}"),
                zenpi::core::TurnRole::User,
                "Paged conversation needle",
            ))
            .unwrap();
    }
    let mut read = request(
        &host,
        Operation::SessionList,
        &project,
        "search",
        json!({"query":"sEaRcH"}),
    );
    read.required_owner = None;
    read.expected_revision = None;
    let revision = host.workspace_revision();
    let before = std::fs::read(root.join("initial.jsonl")).unwrap();
    let first = listing(&host, &registry, &reader, &read).unwrap();
    assert_eq!(first["query"], "sEaRcH");
    assert_eq!(first["entries"].as_array().unwrap().len(), 16);
    assert!(
        first["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["name"].as_str().unwrap().starts_with("Search Target"))
    );
    read.payload = Message::Request {
        operation: Operation::SessionList,
        arguments: json!({"query":"sEaRcH","offset":16,"snapshot_id":first["snapshot_id"]}),
    };
    let last = listing(&host, &registry, &reader, &read).unwrap();
    assert_eq!(last["entries"].as_array().unwrap().len(), 4);
    assert!(last["next_offset"].is_null());
    assert_eq!(last["snapshot_id"], first["snapshot_id"]);
    read.payload = Message::Request {
        operation: Operation::SessionList,
        arguments: json!({"query":"CONVERSATION"}),
    };
    let body_first = listing(&host, &registry, &reader, &read).unwrap();
    assert_eq!(body_first["snapshot_id"], first["snapshot_id"]);
    assert_eq!(body_first["entries"].as_array().unwrap().len(), 16);
    assert!(
        body_first["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["matches"] == 1)
    );
    read.payload = Message::Request {
        operation: Operation::SessionList,
        arguments: json!({"query":"CONVERSATION","offset":16,"snapshot_id":body_first["snapshot_id"]}),
    };
    assert_eq!(
        listing(&host, &registry, &reader, &read).unwrap()["entries"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let id = first["entries"][0]["session_id"].as_str().unwrap();
    read.payload = Message::Request {
        operation: Operation::SessionList,
        arguments: json!({"query":id}),
    };
    assert_eq!(
        listing(&host, &registry, &reader, &read).unwrap()["entries"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    read.payload = Message::Request {
        operation: Operation::SessionList,
        arguments: json!({"query":"no such name"}),
    };
    assert!(
        listing(&host, &registry, &reader, &read).unwrap()["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for query in [
        json!(true),
        json!(" padded "),
        json!("bad\nquery"),
        json!("x".repeat(129)),
    ] {
        read.payload = Message::Request {
            operation: Operation::SessionList,
            arguments: json!({"query":query}),
        };
        assert_eq!(
            listing(&host, &registry, &reader, &read),
            Err(ErrorCode::InvalidRequest)
        );
    }
    assert_eq!(host.workspace_revision(), revision);
    assert_eq!(std::fs::read(root.join("initial.jsonl")).unwrap(), before);
}

#[test]
fn remote_session_body_search_projects_only_redacted_scoped_conversation() {
    use zenpi::core::{Turn, TurnRole};
    use zenpi::remote::sessions::listing;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let path = root.join("body.jsonl");
    let mut session = SessionStore::open_in_workspace(&path, &root).unwrap();
    let target = session.session_id().to_owned();
    for index in 0..6 {
        session
            .append_turn(Turn::new(
                format!("body-{index}"),
                TurnRole::Assistant,
                format!(
                    "Mailbox api_key=HIDDEN_BODY_TOKEN\n{}",
                    "\u{4e2d}".repeat(400)
                ),
            ))
            .unwrap();
    }
    session
        .append_turn(Turn::new("system", TurnRole::System, "SYSTEM_ONLY_NEEDLE"))
        .unwrap();
    session
        .append_turn(Turn::new("tool", TurnRole::Tool, "TOOL_ONLY_NEEDLE"))
        .unwrap();
    let foreign_root = root.join("foreign");
    std::fs::create_dir(&foreign_root).unwrap();
    let mut foreign =
        SessionStore::open_in_workspace(root.join("foreign.jsonl"), &foreign_root).unwrap();
    foreign
        .append_turn(Turn::new(
            "foreign",
            TurnRole::User,
            "Mailbox FOREIGN_ONLY_NEEDLE",
        ))
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    let revision = host.workspace_revision();
    let mut read = request(
        &host,
        Operation::SessionList,
        &project,
        "body-search",
        json!({"query":"MAILBOX"}),
    );
    read.required_owner = None;
    read.expected_revision = None;
    let result = listing(&host, &registry, &reader, &read).unwrap();
    assert_eq!(result["entries"].as_array().unwrap().len(), 1);
    let hit = &result["entries"][0];
    assert_eq!(hit["session_id"], target);
    assert_eq!(hit["matches"], 6);
    let snippets = hit["snippets"].as_array().unwrap();
    assert_eq!(snippets.len(), 4);
    for snippet in snippets {
        let text = snippet.as_str().unwrap();
        assert_eq!(text.chars().count(), 240);
        assert!(text.contains("<redacted>"));
        assert!(!text.contains("HIDDEN_BODY_TOKEN"));
    }
    let digest = result["snapshot_id"].clone();
    for query in [
        "HIDDEN_BODY_TOKEN",
        "SYSTEM_ONLY_NEEDLE",
        "TOOL_ONLY_NEEDLE",
        "FOREIGN_ONLY_NEEDLE",
    ] {
        read.payload = Message::Request {
            operation: Operation::SessionList,
            arguments: json!({"query":query}),
        };
        assert!(
            listing(&host, &registry, &reader, &read).unwrap()["entries"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    read.payload = Message::Request {
        operation: Operation::SessionList,
        arguments: json!({}),
    };
    let all = listing(&host, &registry, &reader, &read).unwrap();
    assert_eq!(all["snapshot_id"], digest);
    assert!(
        all["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry.get("snippets").is_none())
    );
    assert_eq!(host.workspace_revision(), revision);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    registry.revoke(reader.id()).unwrap();
    assert_eq!(
        listing(&host, &registry, &reader, &read),
        Err(ErrorCode::Unauthorized)
    );
}

#[test]
fn remote_current_conversation_history_pages_beyond_snapshot_and_fences_changes() {
    use zenpi::core::{Turn, TurnRole};
    use zenpi::remote::transcript::read;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let owner = host.owner(&project).unwrap();
    for index in 0..70 {
        owner
            .lock()
            .unwrap()
            .session_mut()
            .append_turn(Turn::new(
                format!("history-{index}"),
                TurnRole::User,
                format!("Old message {index} api_key=HISTORY_CREDENTIAL"),
            ))
            .unwrap();
    }
    let journal = owner.lock().unwrap().session().next_sequence();
    let before = std::fs::read(root.join("initial.jsonl")).unwrap();
    let revision = host.workspace_revision();
    let mut request = request(
        &host,
        Operation::TranscriptRead,
        &project,
        "history-read",
        json!({"view":"session_history","offset":0,"digest":null,"journal_revision":journal}),
    );
    request.required_owner = None;
    request.expected_revision = None;
    let first = read(&host, &registry, &reader, &request).unwrap();
    assert_eq!(first["total_turns"], 70);
    assert_eq!(first["turns"][0]["id"], "history-0");
    let mut detail_request = request.clone();
    detail_request.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"history-0","offset":0,"digest":null,"journal_revision":journal}),
    };
    let detail = read(&host, &registry, &reader, &detail_request).unwrap();
    assert_eq!(detail["journal_revision"], journal);
    assert_eq!(detail["turn_id"], "history-0");
    assert!(!detail.to_string().contains("HISTORY_CREDENTIAL"));
    for bad_revision in [json!("1"), json!(-1), Value::Null] {
        detail_request.payload = Message::Request {
            operation: Operation::TranscriptRead,
            arguments: json!({"turn_id":"history-0","offset":0,"digest":null,"journal_revision":bad_revision}),
        };
        assert_eq!(
            read(&host, &registry, &reader, &detail_request),
            Err(ErrorCode::InvalidRequest)
        );
    }
    let digest = first["digest"].clone();
    let mut ids = Vec::new();
    let mut page = first.clone();
    loop {
        assert_eq!(page["journal_revision"], journal);
        assert!(!page.to_string().contains("HISTORY_CREDENTIAL"));
        ids.extend(
            page["turns"]
                .as_array()
                .unwrap()
                .iter()
                .map(|entry| entry["id"].as_str().unwrap().to_owned()),
        );
        if page["has_more"] == false {
            break;
        }
        request.payload = Message::Request {
            operation: Operation::TranscriptRead,
            arguments: json!({"view":"session_history","offset":page["next_offset"],"digest":digest,"journal_revision":journal}),
        };
        page = read(&host, &registry, &reader, &request).unwrap();
    }
    assert_eq!(
        ids,
        (0..70)
            .map(|index| format!("history-{index}"))
            .collect::<Vec<_>>()
    );
    assert_eq!(host.workspace_revision(), revision);
    assert_eq!(std::fs::read(root.join("initial.jsonl")).unwrap(), before);
    for args in [
        json!({"view":"session_history","offset":16,"digest":null,"journal_revision":journal}),
        json!({"view":"session_history","offset":0,"digest":"bad","journal_revision":journal}),
        json!({"view":"session_history","offset":0,"digest":null,"journal_revision":journal,"path":"private"}),
        json!({"view":"session_history","offset":0,"digest":null,"journal_revision":"1"}),
    ] {
        request.payload = Message::Request {
            operation: Operation::TranscriptRead,
            arguments: args,
        };
        assert_eq!(
            read(&host, &registry, &reader, &request),
            Err(ErrorCode::InvalidRequest)
        );
    }
    request.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"view":"session_history","offset":16,"digest":"0".repeat(64),"journal_revision":journal}),
    };
    assert_eq!(
        read(&host, &registry, &reader, &request),
        Err(ErrorCode::RevisionConflict)
    );
    owner
        .lock()
        .unwrap()
        .session_mut()
        .append_turn(Turn::new("changed", TurnRole::Assistant, "changed"))
        .unwrap();
    detail_request.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"history-0","offset":0,"digest":null,"journal_revision":journal}),
    };
    assert_eq!(
        read(&host, &registry, &reader, &detail_request),
        Err(ErrorCode::RevisionConflict)
    );
    request.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"view":"session_history","offset":16,"digest":digest,"journal_revision":journal}),
    };
    assert_eq!(
        read(&host, &registry, &reader, &request),
        Err(ErrorCode::RevisionConflict)
    );
    request.session_id = Some("foreign".into());
    assert_eq!(
        read(&host, &registry, &reader, &request),
        Err(ErrorCode::OwnerMismatch)
    );
    registry.revoke(reader.id()).unwrap();
    assert_eq!(
        read(&host, &registry, &reader, &request),
        Err(ErrorCode::Unauthorized)
    );
}

#[test]
fn historical_detail_revision_fences_message_and_tool_continuations_without_changing_legacy_reads()
{
    use zenpi::core::{Turn, TurnRole};
    use zenpi::remote::transcript::read;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let owner = host.owner(&project).unwrap();
    let mut turn = Turn::new(
        "old-detail",
        TurnRole::Assistant,
        "conversation text\n".repeat(1000),
    );
    turn.metadata = Some(json!({"tool_calls":[{"id":"call-old","name":"read_file",
        "arguments":{"api_key":"HIDDEN_DETAIL_KEY","body":"tool argument ".repeat(1000)}}]}));
    owner
        .lock()
        .unwrap()
        .session_mut()
        .append_turn(turn)
        .unwrap();
    let revision = owner.lock().unwrap().session().next_sequence();
    let before = std::fs::read(root.join("initial.jsonl")).unwrap();
    let mut req = request(
        &host,
        Operation::TranscriptRead,
        &project,
        "bound-detail",
        json!({"turn_id":"old-detail","offset":0,"digest":null,"journal_revision":revision}),
    );
    req.required_owner = None;
    req.expected_revision = None;
    let body = read(&host, &registry, &reader, &req).unwrap();
    assert_eq!(body["journal_revision"], revision);
    assert_eq!(body["has_more"], true);
    req.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"old-detail","offset":body["next_offset"],"digest":body["digest"],"journal_revision":revision}),
    };
    let tail = read(&host, &registry, &reader, &req).unwrap();
    assert_eq!(tail["journal_revision"], revision);
    req.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"old-detail","tool_call_id":"call-old","offset":0,"digest":null,"journal_revision":revision}),
    };
    let tool = read(&host, &registry, &reader, &req).unwrap();
    assert_eq!(tool["journal_revision"], revision);
    assert_eq!(tool["has_more"], true);
    assert!(!tool.to_string().contains("HIDDEN_DETAIL_KEY"));
    req.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"old-detail","tool_call_id":"call-old","offset":tool["next_offset"],"digest":tool["digest"],"journal_revision":revision}),
    };
    assert_eq!(
        read(&host, &registry, &reader, &req).unwrap()["journal_revision"],
        revision
    );
    assert_eq!(std::fs::read(root.join("initial.jsonl")).unwrap(), before);
    owner
        .lock()
        .unwrap()
        .session_mut()
        .append_event(json!({"type":"unrelated-change"}))
        .unwrap();
    assert_eq!(
        read(&host, &registry, &reader, &req),
        Err(ErrorCode::RevisionConflict)
    );
    req.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"old-detail","offset":body["next_offset"],"digest":body["digest"],"journal_revision":revision}),
    };
    assert_eq!(
        read(&host, &registry, &reader, &req),
        Err(ErrorCode::RevisionConflict)
    );
    req.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"old-detail","offset":body["next_offset"],"digest":body["digest"]}),
    };
    let legacy = read(&host, &registry, &reader, &req).unwrap();
    assert!(legacy.get("journal_revision").is_none());
    req.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"request_id":"worker","offset":0,"digest":null,"journal_revision":revision}),
    };
    assert_eq!(
        read(&host, &registry, &reader, &req),
        Err(ErrorCode::InvalidRequest)
    );
}

#[test]
fn remote_new_session_preserves_old_journal_and_retries_after_restart_without_replacement() {
    use zenpi::remote::{
        host_adapter::HostAdapter, protocol::ExecutionState, sessions::new_session,
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let owner = host.owner(&project).unwrap();
    {
        let mut agent = owner.lock().unwrap();
        agent
            .session_mut()
            .append_turn(zenpi::core::Turn::new(
                "old",
                zenpi::core::TurnRole::User,
                "retained conversation",
            ))
            .unwrap();
        let path = agent.session().path().to_path_buf();
        agent.resume_session(path).unwrap();
    }
    let old_path = owner.lock().unwrap().session().path().to_path_buf();
    let old_bytes = std::fs::read(&old_path).unwrap();
    let old_port = owner.lock().unwrap().input_port();
    let req = request(
        &host,
        Operation::SessionNew,
        &project,
        "new-session",
        json!({}),
    );
    assert_eq!(
        new_session(&mut host, &registry, &reader, &req, false).code,
        Some(ErrorCode::Unauthorized)
    );
    assert_eq!(
        new_session(&mut host, &registry, &phone, &req, true).code,
        Some(ErrorCode::Busy)
    );
    let held = owner.lock().unwrap();
    assert_eq!(
        new_session(&mut host, &registry, &phone, &req, false).code,
        Some(ErrorCode::Busy)
    );
    drop(held);
    let revision = host.workspace_revision();
    let response = HostAdapter::dispatch_owned(&mut host, &registry, &phone, req.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    let Message::Response { result } = &response.payload else {
        panic!("new session")
    };
    assert_eq!(result["old_session_id"], req.session_id.as_deref().unwrap());
    assert_eq!(
        result["new_session_id"],
        host.context(&project).unwrap().session_id
    );
    assert_eq!(result["workspace_revision"], revision + 1);
    assert_eq!(
        result["required_owner"],
        req.required_owner.as_deref().unwrap()
    );
    assert_eq!(result.as_object().unwrap().len(), 7);
    assert_ne!(
        host.context(&project).unwrap().session_id,
        req.session_id.as_deref().unwrap()
    );
    assert!(owner.lock().unwrap().history().is_empty());
    assert_eq!(std::fs::read(&old_path).unwrap(), old_bytes);
    assert_ne!(
        old_port.session_id(),
        owner.lock().unwrap().input_port().session_id()
    );
    assert_eq!(
        host.control(&project).unwrap().input_port.session_id(),
        host.context(&project).unwrap().session_id
    );
    let new_path = owner.lock().unwrap().session().path().to_path_buf();
    let new_bytes = std::fs::read(&new_path).unwrap();
    assert_eq!(
        new_session(&mut host, &registry, &phone, &req, true),
        response
    );
    assert_eq!(std::fs::read(&new_path).unwrap(), new_bytes);
    let mut changed = req.clone();
    changed.expected_revision = Some(revision + 1);
    assert_eq!(
        new_session(&mut host, &registry, &phone, &changed, false).code,
        Some(ErrorCode::RequestIdConflict)
    );
    drop(host);
    drop(owner);
    let mut restored = pool(&root);
    assert_eq!(
        restored.context(&project).unwrap().session_id,
        result["new_session_id"].as_str().unwrap()
    );
    // Owner startup may append its normal selection audit; replay itself must
    // not append or replace a second session.
    let restored_bytes = std::fs::read(&new_path).unwrap();
    assert_eq!(
        new_session(&mut restored, &registry, &phone, &req, false),
        response
    );
    assert_eq!(std::fs::read(&new_path).unwrap(), restored_bytes);
    let mut stale = req.clone();
    stale.id = Some("stale-new".into());
    assert_eq!(
        new_session(&mut restored, &registry, &phone, &stale, false).code,
        Some(ErrorCode::OwnerMismatch)
    );
    let pending = request(
        &restored,
        Operation::SessionNew,
        &project,
        "reserved-new",
        json!({}),
    );
    let checkpoint = root.join("project-workspace.json");
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&checkpoint).unwrap()).unwrap();
    saved["remote_workspace"]["receipts"].as_array_mut().unwrap().push(json!({"device_id":phone.id(),
        "request_id":"reserved-new","fingerprint":format!("{:x}",sha2::Sha256::digest(serde_json::to_vec(&pending).unwrap())),"result":null}));
    std::fs::write(&checkpoint, serde_json::to_vec(&saved).unwrap()).unwrap();
    drop(restored);
    let mut restored = pool(&root);
    let current = restored.context(&project).unwrap().session_id.clone();
    assert_eq!(
        new_session(&mut restored, &registry, &phone, &pending, false).execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert_eq!(restored.context(&project).unwrap().session_id, current);
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        new_session(&mut restored, &registry, &phone, &req, false).code,
        Some(ErrorCode::Unauthorized)
    );
}

#[test]
#[cfg(unix)]
fn terminal_history_pages_reconstruct_retained_redacted_screen_and_refuse_stale_or_revoked_reads() {
    use zenpi::remote::terminal::read;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let tabs = host.subtab_state(&project).unwrap();
    let mut shell = zenpi::pty_shell::PtyShell::spawn(root.clone()).unwrap();
    std::fs::write(root.join("history-fixture.sh"), "i=0; while [ $i -lt 600 ]; do printf 'history-line-%s abcdefghijklmnopqrstuvwxyz\n' \"$i\"; i=$((i+1)); done; printf 'api_key=%s\n' 'PRIVATE_HISTORY_SECRET'; printf '%s%s\n' 'HISTORY_' 'FINISHED'\n").unwrap();
    shell.write_input(b"sh history-fixture.sh\r");
    let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        shell.pump();
        if shell.remote_content(4000).contains("HISTORY_FINISHED") {
            break;
        }
        assert!(
            std::time::Instant::now() < until,
            "history fixture did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let mut req = request(
        &host,
        Operation::TerminalRead,
        &project,
        "history",
        json!({"terminal_id":shell.instance_id(),"subtab_id":tabs.active_id,"subtab_revision":tabs.revision,"offset":0,"digest":null}),
    );
    req.required_owner = None;
    req.expected_revision = None;
    let journal = host
        .owner(&project)
        .unwrap()
        .lock()
        .unwrap()
        .session()
        .path()
        .to_path_buf();
    let before = std::fs::read(&journal).unwrap();
    let checkpoint = std::fs::read(root.join("project-workspace.json")).ok();
    let first = read(&host, &registry, &phone, &req, Some(&shell));
    assert_eq!(
        first.execution_state,
        Some(zenpi::remote::protocol::ExecutionState::Succeeded)
    );
    let Message::Response { result: first } = first.payload else {
        panic!("history")
    };
    let mut page = first.clone();
    let mut joined = String::new();
    let mut pages = 0;
    loop {
        let text = page["text"].as_str().unwrap();
        assert!(text.len() <= 8192);
        joined.push_str(text);
        pages += 1;
        if page["has_more"] == false {
            break;
        }
        let Message::Request { arguments, .. } = &mut req.payload else {
            unreachable!()
        };
        arguments["offset"] = page["next_offset"].clone();
        arguments["digest"] = first["digest"].clone();
        let response = read(&host, &registry, &phone, &req, Some(&shell));
        assert_eq!(
            response.execution_state,
            Some(zenpi::remote::protocol::ExecutionState::Succeeded)
        );
        let Message::Response { result } = response.payload else {
            panic!("page")
        };
        page = result;
    }
    assert!(pages > 2);
    assert!(joined.contains("history-line-0"));
    assert!(joined.contains("history-line-599"));
    assert!(joined.contains("HISTORY_FINISHED"));
    assert!(!joined.contains("PRIVATE_HISTORY_SECRET"));
    assert!(!joined.contains(&root.to_string_lossy().to_string()));
    assert_eq!(
        first["digest"],
        format!("{:x}", sha2::Sha256::digest(joined.as_bytes()))
    );
    assert_eq!(std::fs::read(&journal).unwrap(), before);
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).ok(),
        checkpoint
    );
    let Message::Request { arguments, .. } = &mut req.payload else {
        unreachable!()
    };
    arguments["offset"] = first["next_offset"].clone();
    arguments["digest"] = first["digest"].clone();
    shell.write_input(b"printf '%s%s\\n' 'CHANGED_' 'HISTORY'\r");
    loop {
        shell.pump();
        if shell.remote_content(64).contains("CHANGED_HISTORY") {
            break;
        }
        assert!(std::time::Instant::now() < until);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(
        read(&host, &registry, &phone, &req, Some(&shell)).code,
        Some(ErrorCode::RevisionConflict)
    );
    let Message::Request { arguments, .. } = &mut req.payload else {
        unreachable!()
    };
    arguments["offset"] = json!(0);
    arguments["digest"] = Value::Null;
    arguments["terminal_id"] = json!("b".repeat(64));
    assert_eq!(
        read(&host, &registry, &phone, &req, Some(&shell)).code,
        Some(ErrorCode::OwnerMismatch)
    );
    let Message::Request { arguments, .. } = &mut req.payload else {
        unreachable!()
    };
    arguments["terminal_id"] = json!(shell.instance_id());
    assert_eq!(
        read(&host, &registry, &phone, &req, None).code,
        Some(ErrorCode::InvalidRequest)
    );
    for (field, value, expected) in [
        ("offset", json!(1), ErrorCode::InvalidRequest),
        (
            "subtab_revision",
            json!(tabs.revision + 1),
            ErrorCode::RevisionConflict,
        ),
        ("extra", json!(true), ErrorCode::InvalidRequest),
        ("digest", json!("bad"), ErrorCode::InvalidRequest),
    ] {
        let mut invalid = req.clone();
        let Message::Request { arguments, .. } = &mut invalid.payload else {
            unreachable!()
        };
        arguments[field] = value;
        assert_eq!(
            read(&host, &registry, &phone, &invalid, Some(&shell)).code,
            Some(expected)
        );
    }
    let mut foreign = req.clone();
    foreign.session_id = Some("foreign".into());
    assert_eq!(
        read(&host, &registry, &phone, &foreign, Some(&shell)).code,
        Some(ErrorCode::OwnerMismatch)
    );
    foreign = req.clone();
    foreign.project_id = Some("foreign".into());
    assert_eq!(
        read(&host, &registry, &phone, &foreign, Some(&shell)).code,
        Some(ErrorCode::Unauthorized)
    );
    foreign = req.clone();
    foreign.required_owner = Some(host.workspace_owner().into());
    assert_eq!(
        read(&host, &registry, &phone, &foreign, Some(&shell)).code,
        Some(ErrorCode::InvalidRequest)
    );
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        read(&host, &registry, &phone, &req, Some(&shell)).code,
        Some(ErrorCode::Unauthorized)
    );
}

#[test]
#[cfg(unix)]
fn terminal_control_writes_owned_pty_once_and_replays_after_restart_without_shell() {
    use zenpi::remote::protocol::ExecutionState;
    use zenpi::remote::terminal::dispatch;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let tabs = host.subtab_state(&project).unwrap();
    let mut shell = zenpi::pty_shell::PtyShell::spawn(root.clone()).unwrap();
    let terminal = shell.instance_id();
    let args = json!({"terminal_id":terminal,"subtab_id":tabs.active_id,"subtab_revision":tabs.revision,
        "text":"printf '%s%s\\n' 'REMOTE_CONTROL_' 'DONE' >> remote-pty.txt\r"});
    let req = request(
        &host,
        Operation::TerminalInput,
        &project,
        "pty-write",
        args.clone(),
    );
    assert_eq!(
        dispatch(&mut host, &registry, &reader, &req, Some(&mut shell), false).code,
        Some(ErrorCode::Unauthorized)
    );
    assert_eq!(
        dispatch(&mut host, &registry, &phone, &req, Some(&mut shell), true).code,
        Some(ErrorCode::Busy)
    );
    let response = dispatch(&mut host, &registry, &phone, &req, Some(&mut shell), false);
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    let Message::Response { result } = &response.payload else {
        panic!("input receipt")
    };
    assert_eq!(result["terminal_id"], terminal);
    assert_eq!(
        result["accepted_bytes"],
        args["text"].as_str().unwrap().len()
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        shell.pump();
        if std::fs::read_to_string(root.join("remote-pty.txt"))
            .ok()
            .as_deref()
            == Some("REMOTE_CONTROL_DONE\n")
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "remote PTY did not execute input"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let journal = host
        .owner(&project)
        .unwrap()
        .lock()
        .unwrap()
        .session()
        .path()
        .to_path_buf();
    let before = std::fs::read(&journal).unwrap();
    assert!(!String::from_utf8_lossy(&before).contains(args["text"].as_str().unwrap()));
    for _ in 0..3 {
        assert_eq!(
            dispatch(&mut host, &registry, &phone, &req, Some(&mut shell), false),
            response
        );
    }
    assert_eq!(std::fs::read(&journal).unwrap(), before);
    let mut changed = req.clone();
    changed.payload = Message::Request {
        operation: Operation::TerminalInput,
        arguments: args.clone(),
    };
    if let Message::Request { arguments, .. } = &mut changed.payload {
        arguments["text"] = json!("echo duplicate\r");
    }
    assert_eq!(
        dispatch(
            &mut host,
            &registry,
            &phone,
            &changed,
            Some(&mut shell),
            false
        )
        .code,
        Some(ErrorCode::RequestIdConflict)
    );
    let resize = request(
        &host,
        Operation::TerminalResize,
        &project,
        "pty-resize",
        json!({"terminal_id":terminal,"subtab_id":tabs.active_id,"subtab_revision":tabs.revision,"rows":33,"cols":91}),
    );
    shell.resize(24, 80);
    assert_eq!(
        dispatch(
            &mut host,
            &registry,
            &phone,
            &resize,
            Some(&mut shell),
            false
        )
        .execution_state,
        Some(ExecutionState::Succeeded)
    );
    shell.resize(24, 80);
    shell.write_input(b"stty size > remote-size.txt\r");
    loop {
        shell.pump();
        if std::fs::read_to_string(root.join("remote-size.txt"))
            .ok()
            .as_deref()
            == Some("33 91\n")
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "remote resize was overwritten by redraw"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    for (field, value, expected) in [
        (
            "terminal_id",
            json!("b".repeat(64)),
            ErrorCode::OwnerMismatch,
        ),
        (
            "subtab_revision",
            json!(tabs.revision + 1),
            ErrorCode::RevisionConflict,
        ),
        ("text", json!("x".repeat(4097)), ErrorCode::InvalidRequest),
        ("text", json!("\0"), ErrorCode::InvalidRequest),
    ] {
        let mut invalid = args.clone();
        invalid[field] = value;
        let invalid = request(
            &host,
            Operation::TerminalInput,
            &project,
            "pty-invalid",
            invalid,
        );
        assert_eq!(
            dispatch(
                &mut host,
                &registry,
                &phone,
                &invalid,
                Some(&mut shell),
                false
            )
            .code,
            Some(expected)
        );
    }
    let uncertain = request(
        &host,
        Operation::TerminalInput,
        &project,
        "pty-uncertain",
        args.clone(),
    );
    let fingerprint = format!(
        "{:x}",
        sha2::Sha256::digest(serde_json::to_vec(&uncertain).unwrap())
    );
    host.owner(&project).unwrap().lock().unwrap().session_mut().append_event(json!({
        "type":"remote_terminal_admission_v1","device_id":phone.id(),"request_id":"pty-uncertain",
        "fingerprint":fingerprint,"operation":"terminal_input","state":"reserved","result":null
    })).unwrap();
    assert_eq!(
        dispatch(
            &mut host,
            &registry,
            &phone,
            &uncertain,
            Some(&mut shell),
            false
        )
        .execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    drop(host);
    let mut restored = pool(&root);
    assert_eq!(
        dispatch(&mut restored, &registry, &phone, &uncertain, None, false).execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert_eq!(
        dispatch(&mut restored, &registry, &phone, &req, None, false),
        response
    );
    let fresh = request(
        &restored,
        Operation::TerminalInput,
        &project,
        "pty-fresh",
        args,
    );
    assert_eq!(
        dispatch(&mut restored, &registry, &phone, &fresh, None, false).code,
        Some(ErrorCode::InvalidRequest)
    );
    assert_eq!(
        std::fs::read_to_string(root.join("remote-pty.txt")).unwrap(),
        "REMOTE_CONTROL_DONE\n"
    );
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        dispatch(&mut restored, &registry, &phone, &req, None, false).code,
        Some(ErrorCode::Unauthorized)
    );
}

#[test]
fn resource_reload_requires_control_generation_and_replays_without_reloading_after_restart() {
    use zenpi::remote::{host_adapter::HostAdapter, protocol::ExecutionState, resources::reload};
    use zenpi::resource_loader::ResourcePaths;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    std::fs::create_dir(root.join("prompts")).unwrap();
    let template = root.join("prompts/fixture.md");
    std::fs::write(
        &template,
        "---\ndescription: old description\n---\nPRIVATE_TEMPLATE_BODY",
    )
    .unwrap();
    let paths = ResourcePaths {
        user_skills: root.join("empty-skills"),
        project_skills: root.join("skills"),
        skill_paths: vec![],
        user_templates: root.join("empty-prompts"),
        project_templates: root.join("prompts"),
        template_paths: vec![],
        text_resources: vec![],
    };
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let owner = host.owner(&project).unwrap();
    owner
        .lock()
        .unwrap()
        .configure_resources(paths.clone(), || false)
        .unwrap();
    let generation = owner.lock().unwrap().resource_snapshot().generation;
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let mut catalog_request = request(
        &host,
        Operation::ResourcesList,
        &project,
        "reload-catalog",
        json!({}),
    );
    catalog_request.required_owner = None;
    catalog_request.expected_revision = None;
    let snapshot = zenpi::remote::resources::listing(&host, &registry, &phone, &catalog_request)
        .unwrap()["snapshot_id"]
        .clone();
    let req = request(
        &host,
        Operation::ResourcesReload,
        &project,
        "reload",
        json!({"generation":generation,"snapshot_id":snapshot}),
    );
    assert!(Operation::ResourcesReload.is_write());
    let before = std::fs::read(root.join("project-workspace.json")).ok();
    assert_eq!(
        reload(&mut host, &registry, &reader, &req, false).code,
        Some(ErrorCode::Unauthorized)
    );
    assert_eq!(
        reload(&mut host, &registry, &phone, &req, true).code,
        Some(ErrorCode::Busy)
    );
    let mut stale = req.clone();
    stale.expected_revision = Some(host.workspace_revision() + 1);
    assert_eq!(
        reload(&mut host, &registry, &phone, &stale, false).code,
        Some(ErrorCode::RevisionConflict)
    );
    stale = req.clone();
    stale.required_owner = Some("e".repeat(64));
    assert_eq!(
        reload(&mut host, &registry, &phone, &stale, false).code,
        Some(ErrorCode::OwnerMismatch)
    );
    stale = req.clone();
    stale.payload = Message::Request {
        operation: Operation::ResourcesReload,
        arguments: json!({"generation":generation + 1,"snapshot_id":snapshot}),
    };
    assert_eq!(
        reload(&mut host, &registry, &phone, &stale, false).code,
        Some(ErrorCode::RevisionConflict)
    );
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).ok(),
        before
    );
    std::fs::write(
        &template,
        "---\ndescription: new description\n---\nNEW_PRIVATE_TEMPLATE_BODY",
    )
    .unwrap();
    stale = req.clone();
    stale.payload = Message::Request {
        operation: Operation::ResourcesReload,
        arguments: json!({"generation":generation,"snapshot_id":"0".repeat(64)}),
    };
    assert_eq!(
        reload(&mut host, &registry, &phone, &stale, false).code,
        Some(ErrorCode::RevisionConflict)
    );
    let response = HostAdapter::dispatch_owned(&mut host, &registry, &phone, req.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    assert!(!format!("{response:?}").contains("PRIVATE_TEMPLATE"));
    assert!(!format!("{response:?}").contains(root.to_str().unwrap()));
    let mut view = zenpi::tui::TuiState::default();
    let mut desktop = zenpi::tui::ProjectRuntimeHost::new(&mut view, owner.clone()).unwrap();
    assert_eq!(
        desktop.dispatch_remote(
            &mut view,
            &registry,
            &phone,
            req.clone(),
            &Default::default()
        ),
        response
    );
    let updated = owner.lock().unwrap().resource_snapshot().generation;
    assert_eq!(updated, generation + 1);
    assert_eq!(reload(&mut host, &registry, &phone, &req, true), response);
    assert_eq!(
        owner.lock().unwrap().resource_snapshot().generation,
        updated
    );
    let mut changed = req.clone();
    changed.payload = Message::Request {
        operation: Operation::ResourcesReload,
        arguments: json!({"generation":updated,"snapshot_id":snapshot}),
    };
    assert_eq!(
        reload(&mut host, &registry, &phone, &changed, false).code,
        Some(ErrorCode::RequestIdConflict)
    );
    let mut conflict = req.clone();
    conflict.payload = Message::Request {
        operation: Operation::ModelSelect,
        arguments: json!({"model":"fixture","selection_revision":0}),
    };
    assert_eq!(
        zenpi::remote::model::select(&mut host, &registry, &phone, &conflict, false).code,
        Some(ErrorCode::RequestIdConflict)
    );
    let session = owner.lock().unwrap().session().path().to_owned();
    drop(host);
    drop(owner);
    let mut restored_agent = Agent::with_echo(SessionStore::open_existing(&session).unwrap());
    restored_agent.restore_resources(paths, || false).unwrap();
    let restored_owner = Arc::new(Mutex::new(restored_agent));
    let mut restored = ProjectOwnerPool::new(restored_owner.clone()).unwrap();
    restored.restore_checkpoint().unwrap();
    let mut old_catalog = req.clone();
    old_catalog.id = Some("new-after-restart".into());
    old_catalog.required_owner = Some(restored.workspace_owner().into());
    old_catalog.expected_revision = Some(restored.workspace_revision());
    assert_eq!(
        reload(&mut restored, &registry, &phone, &old_catalog, false).code,
        Some(ErrorCode::RevisionConflict)
    );
    let restored_generation = restored_owner
        .lock()
        .unwrap()
        .resource_snapshot()
        .generation;
    assert_eq!(
        reload(&mut restored, &registry, &phone, &req, false),
        response
    );
    assert_eq!(
        restored_owner
            .lock()
            .unwrap()
            .resource_snapshot()
            .generation,
        restored_generation
    );
    assert_eq!(
        restored_owner
            .lock()
            .unwrap()
            .session()
            .events()
            .iter()
            .filter(|event| event["type"] == "remote_resources_admission_v1")
            .count(),
        2
    );
}

#[test]
fn ambiguous_resource_reload_remains_unknown_and_never_retries_changed_files() {
    use zenpi::remote::{protocol::ExecutionState, resources::reload};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let req = request(
        &host,
        Operation::ResourcesReload,
        &project,
        "unconfigured-reload",
        json!({"generation":0,"snapshot_id": ({
            let mut read = request(&host,Operation::ResourcesList,&project,"catalog",json!({}));
            read.required_owner=None;read.expected_revision=None;
            zenpi::remote::resources::listing(&host,&registry,&phone,&read).unwrap()["snapshot_id"].clone()
        })}),
    );
    let response = reload(&mut host, &registry, &phone, &req, false);
    assert_eq!(
        response.execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    let owner = host.owner(&project).unwrap();
    let before = owner.lock().unwrap().session().next_sequence();
    assert_eq!(reload(&mut host, &registry, &phone, &req, false), response);
    assert_eq!(owner.lock().unwrap().session().next_sequence(), before);
    let held = owner.lock().unwrap();
    assert_eq!(
        reload(&mut host, &registry, &phone, &req, false).execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    let mut fresh = req.clone();
    fresh.id = Some("fresh".into());
    assert_eq!(
        reload(&mut host, &registry, &phone, &fresh, false).code,
        Some(ErrorCode::Busy)
    );
    drop(held);
    for arguments in [
        json!({}),
        json!({"generation":"0"}),
        json!({"generation":-1}),
        json!({"generation":0,"cwd":"/private"}),
    ] {
        fresh.payload = Message::Request {
            operation: Operation::ResourcesReload,
            arguments,
        };
        assert_eq!(
            reload(&mut host, &registry, &phone, &fresh, false).code,
            Some(ErrorCode::InvalidRequest)
        );
    }
}

#[test]
fn reasoning_selection_default_none_cas_and_restart_receipts_share_model_namespace() {
    use zenpi::remote::{model::select, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("reasoning.jsonl");
    let backend = || {
        Box::new(
            zenpi::backend::OpenAiCompatibleBackend::new_with_settings(
                "http://127.0.0.1:9",
                Some("PRIVATE_REASONING_KEY".into()),
                "gpt-5.2",
                zenpi::backend::OpenAiWireApi::Responses,
                Some("high".into()),
                None,
            )
            .unwrap()
            .with_model_registry(
                "openai".into(),
                zenpi::providers::registry::ModelRegistry::default(),
            )
            .unwrap(),
        )
    };
    let owner = Arc::new(Mutex::new(Agent::new(
        SessionStore::open_in_workspace(&path, &root).unwrap(),
        backend(),
    )));
    let mut host = ProjectOwnerPool::new(owner.clone()).unwrap();
    host.restore_checkpoint().unwrap();
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let req = request(
        &host,
        Operation::ReasoningSelect,
        &project,
        "reasoning-default",
        json!({"reasoning_effort":null,"selection_revision":0}),
    );
    assert_eq!(
        select(&mut host, &registry, &reader, &req, false).code,
        Some(ErrorCode::Unauthorized)
    );
    assert_eq!(
        select(&mut host, &registry, &phone, &req, true).code,
        Some(ErrorCode::Busy)
    );
    let receipt = select(&mut host, &registry, &phone, &req, false);
    assert_eq!(receipt.execution_state, Some(ExecutionState::Succeeded));
    assert!(
        matches!(&receipt.payload, Message::Response { result } if result["reasoning_effort"].is_null() && result.as_object().unwrap().len() == 5)
    );
    assert_eq!(owner.lock().unwrap().reasoning_effort(), None);
    let revision = owner.lock().unwrap().selection_revision();
    let explicit = request(
        &host,
        Operation::ReasoningSelect,
        &project,
        "reasoning-none",
        json!({"reasoning_effort":"none","selection_revision":revision}),
    );
    let none_receipt = select(&mut host, &registry, &phone, &explicit, false);
    assert_eq!(
        none_receipt.execution_state,
        Some(ExecutionState::Succeeded)
    );
    assert_eq!(owner.lock().unwrap().reasoning_effort(), Some("none"));
    let before = std::fs::read(&path).unwrap();
    assert_eq!(select(&mut host, &registry, &phone, &req, true), receipt);
    assert_eq!(owner.lock().unwrap().reasoning_effort(), Some("none"));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let stale = request(
        &host,
        Operation::ReasoningSelect,
        &project,
        "reasoning-stale",
        json!({"reasoning_effort":"low","selection_revision":revision}),
    );
    assert_eq!(
        select(&mut host, &registry, &phone, &stale, false).code,
        Some(ErrorCode::RevisionConflict)
    );
    let conflict = request(
        &host,
        Operation::ModelSelect,
        &project,
        "reasoning-default",
        json!({"model":"gpt-5.2","selection_revision":0}),
    );
    assert_eq!(
        select(&mut host, &registry, &phone, &conflict, false).code,
        Some(ErrorCode::RequestIdConflict)
    );
    let current = owner.lock().unwrap().selection_revision();
    let invalid = request(
        &host,
        Operation::ReasoningSelect,
        &project,
        "reasoning-invalid",
        json!({"reasoning_effort":"ultra","selection_revision":current}),
    );
    let rejected = select(&mut host, &registry, &phone, &invalid, false);
    assert_eq!(rejected.code, Some(ErrorCode::InvalidRequest));
    owner
        .lock()
        .unwrap()
        .set_reasoning_effort(Some("low".into()))
        .unwrap();
    assert_eq!(
        select(&mut host, &registry, &phone, &invalid, false),
        rejected
    );
    let checkpoint = root.join("project-workspace.json");
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&checkpoint).unwrap()).unwrap();
    saved["remote_workspace"]
        .as_object_mut()
        .unwrap()
        .remove("prompt_bindings");
    std::fs::write(&checkpoint, serde_json::to_vec(&saved).unwrap()).unwrap();
    drop(host);
    *owner.lock().unwrap() = Agent::new(
        SessionStore::open_in_workspace(&path, &root).unwrap(),
        backend(),
    );
    owner.lock().unwrap().restore_model_selection().unwrap();
    let mut restored = ProjectOwnerPool::new(owner.clone()).unwrap();
    restored.restore_checkpoint().unwrap();
    assert_eq!(owner.lock().unwrap().reasoning_effort(), Some("low"));
    assert_eq!(
        select(&mut restored, &registry, &phone, &req, false),
        receipt
    );
    assert_eq!(
        select(&mut restored, &registry, &phone, &explicit, false),
        none_receipt
    );
    assert_eq!(owner.lock().unwrap().reasoning_effort(), Some("low"));
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        select(&mut restored, &registry, &phone, &req, false).code,
        Some(ErrorCode::Unauthorized)
    );
}

#[test]
fn reasoning_selection_malformed_and_ambiguous_receipts_do_not_reexecute() {
    use zenpi::remote::{model::select, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(&registry, vec![project.clone()], vec![Capability::Control]);
    let owner = host.owner(&project).unwrap();
    let req = request(
        &host,
        Operation::ReasoningSelect,
        &project,
        "unknown-reasoning",
        json!({"reasoning_effort":null,"selection_revision":0}),
    );
    for args in [
        json!({"reasoning_effort":false,"selection_revision":0}),
        json!({"reasoning_effort":"default","selection_revision":0}),
        json!({"reasoning_effort":null,"selection_revision":"0"}),
        json!({"reasoning_effort":null,"selection_revision":0,"model":"other"}),
    ] {
        let mut malformed = req.clone();
        malformed.payload = Message::Request {
            operation: Operation::ReasoningSelect,
            arguments: args,
        };
        assert_eq!(
            select(&mut host, &registry, &phone, &malformed, false).code,
            Some(ErrorCode::InvalidRequest)
        );
    }
    let fingerprint = format!(
        "{:x}",
        sha2::Sha256::digest(serde_json::to_vec(&req).unwrap())
    );
    owner.lock().unwrap().session_mut().append_event(json!({"type":"remote_reasoning_admission_v1","device_id":phone.id(),"request_id":"unknown-reasoning","fingerprint":fingerprint,"state":"reserved","result":null,"code":null})).unwrap();
    drop(host);
    let mut restored = ProjectOwnerPool::new(owner.clone()).unwrap();
    restored.restore_checkpoint().unwrap();
    let before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    assert_eq!(
        select(&mut restored, &registry, &phone, &req, false).execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        before
    );
    let held = owner.lock().unwrap();
    assert_eq!(
        select(&mut restored, &registry, &phone, &req, false).execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    drop(held);
}

#[test]
fn reasoning_selection_staged_failure_is_durable_and_fresh_request_can_apply_after_clear() {
    use zenpi::remote::{model::select, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let backend = zenpi::backend::OpenAiCompatibleBackend::new_with_settings(
        "http://127.0.0.1:9",
        None,
        "gpt-5.2",
        zenpi::backend::OpenAiWireApi::Responses,
        Some("high".into()),
        None,
    )
    .unwrap()
    .with_model_registry(
        "openai".into(),
        zenpi::providers::registry::ModelRegistry::default(),
    )
    .unwrap();
    let owner = Arc::new(Mutex::new(Agent::new(
        SessionStore::open_in_workspace(root.join("reasoning-staged.jsonl"), &root).unwrap(),
        Box::new(backend),
    )));
    let mut host = ProjectOwnerPool::new(owner.clone()).unwrap();
    host.restore_checkpoint().unwrap();
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(&registry, vec![project.clone()], vec![Capability::Control]);
    let req = request(
        &host,
        Operation::ReasoningSelect,
        &project,
        "reasoning-staged",
        json!({"reasoning_effort":"low","selection_revision":0}),
    );
    owner
        .lock()
        .unwrap()
        .stage_attachment(zenpi::backend::InputAttachment {
            kind: zenpi::backend::AttachmentKind::Image,
            mime_type: "image/png".into(),
            path: None,
            url: Some("https://example.invalid/staged.png".into()),
            file_id: None,
        })
        .unwrap();
    let failed = select(&mut host, &registry, &phone, &req, false);
    assert_eq!(failed.code, Some(ErrorCode::Busy));
    owner.lock().unwrap().clear_pending_attachments();
    let before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    assert_eq!(select(&mut host, &registry, &phone, &req, false), failed);
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        before
    );
    assert_eq!(owner.lock().unwrap().reasoning_effort(), Some("high"));
    let mut fresh = req;
    fresh.id = Some("reasoning-after-clear".into());
    assert_eq!(
        select(&mut host, &registry, &phone, &fresh, false).execution_state,
        Some(ExecutionState::Succeeded)
    );
    assert_eq!(owner.lock().unwrap().reasoning_effort(), Some("low"));
    assert_eq!(owner.lock().unwrap().model(), Some("gpt-5.2"));
}

#[test]
fn model_selection_is_durable_cas_acl_fenced_and_retry_never_overwrites_later_choice() {
    use zenpi::remote::{host_adapter::HostAdapter, model::select, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let backend = || {
        Box::new(
            zenpi::backend::OpenAiCompatibleBackend::new_with_settings(
                "http://127.0.0.1:9/v1",
                Some("PRIVATE_MODEL_KEY".into()),
                "gpt-5.2",
                zenpi::backend::OpenAiWireApi::Responses,
                Some("high".into()),
                None,
            )
            .unwrap()
            .with_model_registry(
                "openai".into(),
                zenpi::providers::registry::ModelRegistry::default(),
            )
            .unwrap(),
        )
    };
    let path = root.join("selection.jsonl");
    let owner = Arc::new(Mutex::new(Agent::new(
        SessionStore::open_in_workspace(&path, &root).unwrap(),
        backend(),
    )));
    let mut host = ProjectOwnerPool::new(owner.clone()).unwrap();
    host.restore_checkpoint().unwrap();
    let project = host.active_context().project_id.clone();
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let read = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let req = request(
        &host,
        Operation::ModelSelect,
        &project,
        "model-write",
        json!({"model":"gpt-5.2-2025-12-11","selection_revision":0}),
    );
    let before = std::fs::read(&path).unwrap();
    for arguments in [
        json!({"model":"next"}),
        json!({"model":"next","selection_revision":"0"}),
        json!({"model":"next","selection_revision":-1}),
        json!({"model":"next","selection_revision":0,"effort":"low"}),
        json!({"model":"https://private.invalid/model","selection_revision":0}),
    ] {
        let mut malformed = req.clone();
        malformed.payload = Message::Request {
            operation: Operation::ModelSelect,
            arguments,
        };
        assert_eq!(
            select(&mut host, &registry, &phone, &malformed, false).code,
            Some(ErrorCode::InvalidRequest)
        );
    }
    assert_eq!(
        select(&mut host, &registry, &read, &req, false).code,
        Some(ErrorCode::Unauthorized)
    );
    assert_eq!(
        select(&mut host, &registry, &phone, &req, true).code,
        Some(ErrorCode::Busy)
    );
    let mut stale = req.clone();
    stale.expected_revision = Some(host.workspace_revision() + 1);
    assert_eq!(
        select(&mut host, &registry, &phone, &stale, false).code,
        Some(ErrorCode::RevisionConflict)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let revision = host.workspace_revision();
    let receipt = HostAdapter::dispatch_owned(&mut host, &registry, &phone, req.clone());
    assert_eq!(receipt.execution_state, Some(ExecutionState::Succeeded));
    assert_eq!(host.workspace_revision(), revision);
    assert_eq!(owner.lock().unwrap().model(), Some("gpt-5.2-2025-12-11"));
    assert_eq!(owner.lock().unwrap().reasoning_effort(), Some("high"));
    assert!(
        !serde_json::to_string(&receipt)
            .unwrap()
            .contains("PRIVATE_MODEL_KEY")
    );
    let selected_revision = owner.lock().unwrap().selection_revision();
    owner
        .lock()
        .unwrap()
        .set_model(Some("gpt-5.2".into()))
        .unwrap();
    let journal = std::fs::read(&path).unwrap();
    assert_eq!(select(&mut host, &registry, &phone, &req, true), receipt);
    assert_eq!(std::fs::read(&path).unwrap(), journal);
    let stale_selection = request(
        &host,
        Operation::ModelSelect,
        &project,
        "stale-selection",
        json!({"model":"gpt-5.2-2025-12-11","selection_revision":selected_revision}),
    );
    assert_eq!(
        select(&mut host, &registry, &phone, &stale_selection, false).code,
        Some(ErrorCode::RevisionConflict)
    );
    let collision = request(
        &host,
        Operation::WorkspaceSelect,
        &project,
        "model-write",
        json!({}),
    );
    assert_eq!(
        mutate(&mut host, &registry, &phone, &collision),
        Err(ErrorCode::RequestIdConflict)
    );
    let prompt = request(
        &host,
        Operation::Prompt,
        &project,
        "model-write",
        json!({"text":"do not execute"}),
    );
    assert_eq!(
        zenpi::remote::command::dispatch_prompt(
            &mut host,
            &registry,
            &phone,
            &prompt,
            false,
            |_, _, _| panic!("collision executed")
        )
        .code,
        Some(ErrorCode::RequestIdConflict)
    );
    let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
    let (adapter, mailbox) = HostAdapter::owner_thread();
    std::thread::scope(|scope| {
        let pending = scope.spawn(|| adapter.dispatch(&registry, &phone, req.clone()));
        while !pending.is_finished() {
            mailbox.drain(&mut host);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(pending.join().unwrap(), receipt);
    });
    drop(host);
    *owner.lock().unwrap() = Agent::new(
        SessionStore::open_in_workspace(&path, &root).unwrap(),
        backend(),
    );
    owner.lock().unwrap().restore_model_selection().unwrap();
    let mut restored = ProjectOwnerPool::new(owner.clone()).unwrap();
    restored.restore_checkpoint().unwrap();
    assert_eq!(
        select(&mut restored, &registry, &phone, &req, false),
        receipt
    );
    let held = owner.lock().unwrap();
    assert_eq!(
        select(&mut restored, &registry, &phone, &req, false).execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    drop(held);
    assert_eq!(owner.lock().unwrap().model(), Some("gpt-5.2"));
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        checkpoint
    );
    let records: Vec<Value> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let mut interrupted = String::new();
    for record in records.iter().filter(|record| {
        !(record["event"]["type"] == "remote_model_admission_v1"
            && record["event"]["state"] == "succeeded")
    }) {
        interrupted.push_str(&serde_json::to_string(record).unwrap());
        interrupted.push('\n');
    }
    std::fs::write(&path, interrupted).unwrap();
    *owner.lock().unwrap() = Agent::new(
        SessionStore::open_in_workspace(&path, &root).unwrap(),
        backend(),
    );
    owner.lock().unwrap().restore_model_selection().unwrap();
    assert_eq!(
        select(&mut restored, &registry, &phone, &req, false).execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert_eq!(owner.lock().unwrap().model(), Some("gpt-5.2"));
    std::fs::write(&path, journal).unwrap();
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        select(&mut restored, &registry, &phone, &req, false).code,
        Some(ErrorCode::Unauthorized)
    );
}

#[test]
fn model_selection_invalid_busy_and_unresolved_reservations_do_not_repeat() {
    use zenpi::remote::{model::select, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(&registry, vec![project.clone()], vec![Capability::Control]);
    let req = request(
        &host,
        Operation::ModelSelect,
        &project,
        "unsupported-model",
        json!({"model":"fixture","selection_revision":0}),
    );
    let owner = host.owner(&project).unwrap();
    let rejected = select(&mut host, &registry, &phone, &req, false);
    assert_eq!(rejected.code, Some(ErrorCode::InvalidRequest));
    let before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    assert_eq!(select(&mut host, &registry, &phone, &req, false), rejected);
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        before
    );
    let unknown = request(
        &host,
        Operation::ModelSelect,
        &project,
        "unknown-model",
        json!({"model":"fixture","selection_revision":0}),
    );
    let fingerprint = format!(
        "{:x}",
        sha2::Sha256::digest(serde_json::to_vec(&unknown).unwrap())
    );
    owner.lock().unwrap().session_mut().append_event(json!({"type":"remote_model_admission_v1","device_id":phone.id(),"request_id":"unknown-model","fingerprint":fingerprint,"state":"reserved","result":null,"code":null})).unwrap();
    drop(host);
    let mut host = ProjectOwnerPool::new(owner.clone()).unwrap();
    host.restore_checkpoint().unwrap();
    assert_eq!(
        select(&mut host, &registry, &phone, &unknown, false).execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    let mut changed = unknown.clone();
    changed.payload = Message::Request {
        operation: Operation::ModelSelect,
        arguments: json!({"model":"other","selection_revision":0}),
    };
    assert_eq!(
        select(&mut host, &registry, &phone, &changed, false).code,
        Some(ErrorCode::RequestIdConflict)
    );
    let checkpoint_path = root.join("project-workspace.json");
    let mut checkpoint: Value =
        serde_json::from_slice(&std::fs::read(&checkpoint_path).unwrap()).unwrap();
    checkpoint["remote_workspace"]
        .as_object_mut()
        .unwrap()
        .remove("prompt_bindings");
    std::fs::write(&checkpoint_path, serde_json::to_vec(&checkpoint).unwrap()).unwrap();
    drop(host);
    let mut imported = ProjectOwnerPool::new(owner.clone()).unwrap();
    imported.restore_checkpoint().unwrap();
    assert_eq!(
        select(&mut imported, &registry, &phone, &unknown, false).execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    let prompt = request(
        &imported,
        Operation::Prompt,
        &project,
        "unknown-model",
        json!({"text":"collision"}),
    );
    assert_eq!(
        zenpi::remote::command::dispatch_prompt(
            &mut imported,
            &registry,
            &phone,
            &prompt,
            false,
            |_, _, _| panic!()
        )
        .code,
        Some(ErrorCode::RequestIdConflict)
    );
}

#[test]
fn model_selection_checkpoint_failure_never_changes_model_or_journal() {
    use zenpi::remote::{model::select, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(&registry, vec![project.clone()], vec![Capability::Control]);
    let req = request(
        &host,
        Operation::ModelSelect,
        &project,
        "checkpoint-failure",
        json!({"model":"fixture","selection_revision":0}),
    );
    let owner = host.owner(&project).unwrap();
    let before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    std::fs::create_dir(root.join("project-workspace.json")).unwrap();
    assert_eq!(
        select(&mut host, &registry, &phone, &req, false).execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        before
    );
    assert_eq!(owner.lock().unwrap().model(), None);
}

#[test]
fn model_request_binding_operation_corruption_fails_checkpoint_restore() {
    use zenpi::remote::model::select;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(&registry, vec![project.clone()], vec![Capability::Control]);
    let req = request(
        &host,
        Operation::ModelSelect,
        &project,
        "model-kind",
        json!({"model":"fixture","selection_revision":0}),
    );
    select(&mut host, &registry, &phone, &req, false);
    let path = root.join("project-workspace.json");
    let saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let owner = host.owner(&project).unwrap();
    drop(host);
    for operation in ["cancel", "model_status", "unknown", "prompt"] {
        let mut corrupt = saved.clone();
        corrupt["remote_workspace"]["prompt_bindings"][0]["operation"] = json!(operation);
        std::fs::write(&path, serde_json::to_vec(&corrupt).unwrap()).unwrap();
        let mut restored = ProjectOwnerPool::new(owner.clone()).unwrap();
        assert!(restored.restore_checkpoint().is_err(), "{operation}");
    }
}

#[test]
fn model_selection_staged_attachment_failure_remains_failed_after_clearing_and_retry() {
    use zenpi::remote::{model::select, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let backend = zenpi::backend::OpenAiCompatibleBackend::new_with_settings(
        "http://127.0.0.1:9",
        None,
        "gpt-5.2",
        zenpi::backend::OpenAiWireApi::Responses,
        Some("high".into()),
        None,
    )
    .unwrap()
    .with_model_registry(
        "openai".into(),
        zenpi::providers::registry::ModelRegistry::default(),
    )
    .unwrap();
    let owner = Arc::new(Mutex::new(Agent::new(
        SessionStore::open_in_workspace(root.join("fenced.jsonl"), &root).unwrap(),
        Box::new(backend),
    )));
    let mut host = ProjectOwnerPool::new(owner.clone()).unwrap();
    host.restore_checkpoint().unwrap();
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(&registry, vec![project.clone()], vec![Capability::Control]);
    let req = request(
        &host,
        Operation::ModelSelect,
        &project,
        "staged-model",
        json!({"model":"gpt-5.2-2025-12-11","selection_revision":0}),
    );
    owner
        .lock()
        .unwrap()
        .stage_attachment(zenpi::backend::InputAttachment {
            kind: zenpi::backend::AttachmentKind::Image,
            mime_type: "image/png".into(),
            path: None,
            url: Some("https://example.invalid/staged.png".into()),
            file_id: None,
        })
        .unwrap();
    let rejected = select(&mut host, &registry, &phone, &req, false);
    assert_eq!(rejected.code, Some(ErrorCode::Busy));
    assert_eq!(rejected.execution_state, Some(ExecutionState::Failed));
    owner.lock().unwrap().clear_pending_attachments();
    let journal = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    assert_eq!(select(&mut host, &registry, &phone, &req, false), rejected);
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        journal
    );
    assert_eq!(owner.lock().unwrap().model(), Some("gpt-5.2"));
    let mut fresh = req;
    fresh.id = Some("fresh-model".into());
    assert_eq!(
        select(&mut host, &registry, &phone, &fresh, false).execution_state,
        Some(ExecutionState::Succeeded)
    );
}

#[test]
fn model_status_is_allowlisted_read_only_scoped_busy_and_revoke_fenced() {
    use zenpi::remote::{host_adapter::HostAdapter, model::status, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut pool = pool(&root);
    let project = pool.active_context().project_id.clone();
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let phone = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let mut req = request(
        &pool,
        Operation::ModelStatus,
        &project,
        "model-status",
        json!({}),
    );
    req.required_owner = None;
    req.expected_revision = None;
    let owner = pool.owner(&project).unwrap();
    let journal = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    let checkpoint = std::fs::read(root.join("project-workspace.json")).ok();
    let value = status(&pool, &registry, &phone, &req).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 10);
    assert_eq!(value["reasoning_choices"], json!([]));
    assert_eq!(value["registry_bound"], false);
    assert!(value["model"].is_null());
    assert!(value["capabilities"].is_null());
    assert_eq!(value["selection_revision"], 0);
    let backend = zenpi::backend::OpenAiCompatibleBackend::new_with_settings(
        "http://127.0.0.1:9/v1",
        Some("PRIVATE_MODEL_CREDENTIAL".into()),
        "gpt-5.2",
        zenpi::backend::OpenAiWireApi::Responses,
        Some("high".into()),
        None,
    )
    .unwrap()
    .with_model_registry(
        "openai".into(),
        zenpi::providers::registry::ModelRegistry::default(),
    )
    .unwrap();
    let journal_path = owner.lock().unwrap().session().path().to_owned();
    *owner.lock().unwrap() = Agent::new(
        SessionStore::open_in_workspace(&journal_path, &root).unwrap(),
        Box::new(backend),
    );
    let actual = status(&pool, &registry, &phone, &req).unwrap();
    assert_eq!(actual["model"], "gpt-5.2");
    assert_eq!(actual["reasoning_effort"], "high");
    assert_eq!(
        actual["reasoning_choices"],
        json!([null, "none", "low", "medium", "high", "xhigh"])
    );
    assert_eq!(actual["registry_bound"], true);
    assert_eq!(actual["capabilities"]["text"], true);
    assert!(!actual.to_string().contains("PRIVATE_MODEL_CREDENTIAL"));
    assert!(!actual.to_string().contains("127.0.0.1"));
    let response = HostAdapter::dispatch_owned(&mut pool, &registry, &phone, req.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    let (adapter, mailbox) = HostAdapter::owner_thread();
    std::thread::scope(|scope| {
        let pending = scope.spawn(|| adapter.dispatch(&registry, &phone, req.clone()));
        let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !pending.is_finished() && std::time::Instant::now() < until {
            mailbox.drain(&mut pool);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(pending.join().unwrap().payload, response.payload);
    });
    let mut bad = req.clone();
    bad.session_id = Some("other".into());
    assert_eq!(
        status(&pool, &registry, &phone, &bad),
        Err(ErrorCode::OwnerMismatch)
    );
    bad = req.clone();
    bad.payload = Message::Request {
        operation: Operation::ModelStatus,
        arguments: json!({"catalog":true}),
    };
    assert_eq!(
        status(&pool, &registry, &phone, &bad),
        Err(ErrorCode::InvalidRequest)
    );
    let held = owner.lock().unwrap();
    assert_eq!(status(&pool, &registry, &phone, &req), Err(ErrorCode::Busy));
    drop(held);
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        status(&pool, &registry, &phone, &req),
        Err(ErrorCode::Unauthorized)
    );
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        journal
    );
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).ok(),
        checkpoint
    );
}

#[test]
fn model_catalog_pages_bind_selection_and_metadata_without_side_effects_or_private_fields() {
    use zenpi::remote::model::status;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let host = pool(&root);
    let project = host.active_context().project_id.clone();
    let owner = host.owner(&project).unwrap();
    let overrides: Vec<zenpi::providers::registry::ModelOverride> = (0..40)
        .map(|index| {
            serde_json::from_value(json!({
        "provider":"openai","id":format!("a-fixture-{index:02}"),"version":"PRIVATE_SOURCE_VERSION",
        "status":"beta","context_window":48000}))
            .unwrap()
        })
        .collect();
    let registry_models =
        zenpi::providers::registry::ModelRegistry::with_overrides(&overrides).unwrap();
    let backend = zenpi::backend::OpenAiCompatibleBackend::new_with_settings(
        "http://127.0.0.1:9",
        Some("PRIVATE_CATALOG_KEY".into()),
        "gpt-5.2",
        zenpi::backend::OpenAiWireApi::Responses,
        None,
        None,
    )
    .unwrap()
    .with_model_registry("openai".into(), registry_models)
    .unwrap();
    let path = owner.lock().unwrap().session().path().to_owned();
    *owner.lock().unwrap() = Agent::new(
        SessionStore::open_in_workspace(&path, &root).unwrap(),
        Box::new(backend),
    );
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let mut req = request(
        &host,
        Operation::ModelStatus,
        &project,
        "model-pages",
        json!({}),
    );
    req.required_owner = None;
    req.expected_revision = None;
    let before = std::fs::read(&path).unwrap();
    let checkpoint = std::fs::read(root.join("project-workspace.json")).ok();
    let first = status(&host, &registry, &phone, &req).unwrap();
    assert_eq!(first["catalog"]["entries"].as_array().unwrap().len(), 16);
    assert_eq!(first["catalog"]["next_offset"], 16);
    let mut ids = Vec::new();
    let mut page = first.clone();
    loop {
        for entry in page["catalog"]["entries"].as_array().unwrap() {
            assert_eq!(entry.as_object().unwrap().len(), 3);
            ids.push(entry["id"].as_str().unwrap().to_owned());
            if entry["id"].as_str().unwrap().starts_with("a-fixture") {
                assert_eq!(entry["status"], "beta");
                assert_eq!(entry["selectable"], true);
            }
        }
        let Some(offset) = page["catalog"]["next_offset"].as_u64() else {
            break;
        };
        req.payload = Message::Request {
            operation: Operation::ModelStatus,
            arguments: json!({"offset":offset,"snapshot_id":first["catalog"]["snapshot_id"]}),
        };
        page = status(&host, &registry, &phone, &req).unwrap();
        assert_eq!(
            page["catalog"]["snapshot_id"],
            first["catalog"]["snapshot_id"]
        );
    }
    assert!(ids.len() > 40);
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    for forbidden in [
        "PRIVATE_CATALOG_KEY",
        "PRIVATE_SOURCE_VERSION",
        "sources",
        "price",
        "provider",
        "http://",
        root.to_str().unwrap(),
    ] {
        assert!(!first.to_string().contains(forbidden));
    }
    req.payload = Message::Request {
        operation: Operation::ModelStatus,
        arguments: json!({"offset":0,"snapshot_id":first["catalog"]["snapshot_id"]}),
    };
    assert_eq!(status(&host, &registry, &phone, &req).unwrap(), first);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).ok(),
        checkpoint
    );
    owner
        .lock()
        .unwrap()
        .set_reasoning_effort(Some("high".into()))
        .unwrap();
    assert_eq!(
        status(&host, &registry, &phone, &req),
        Err(ErrorCode::RevisionConflict)
    );
    req.payload = Message::Request {
        operation: Operation::ModelStatus,
        arguments: json!({}),
    };
    let changed = status(&host, &registry, &phone, &req).unwrap();
    assert_ne!(
        changed["catalog"]["snapshot_id"],
        first["catalog"]["snapshot_id"]
    );
    let mut offset = 0u64;
    let mut found = false;
    loop {
        req.payload = Message::Request {
            operation: Operation::ModelStatus,
            arguments: json!({"offset":offset,"snapshot_id":changed["catalog"]["snapshot_id"]}),
        };
        let page = status(&host, &registry, &phone, &req).unwrap();
        if let Some(entry) = page["catalog"]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == "gpt-4.1")
        {
            assert_eq!(entry["selectable"], false);
            found = true;
        }
        let Some(next) = page["catalog"]["next_offset"].as_u64() else {
            break;
        };
        offset = next;
    }
    assert!(found);
    for args in [
        json!({"offset":1,"snapshot_id":changed["catalog"]["snapshot_id"]}),
        json!({"offset":512,"snapshot_id":changed["catalog"]["snapshot_id"]}),
        json!({"offset":"16","snapshot_id":changed["catalog"]["snapshot_id"]}),
        json!({"offset":16,"snapshot_id":"bad"}),
    ] {
        req.payload = Message::Request {
            operation: Operation::ModelStatus,
            arguments: args,
        };
        assert_eq!(
            status(&host, &registry, &phone, &req),
            Err(ErrorCode::InvalidRequest)
        );
    }
    registry.revoke(phone.id()).unwrap();
    req.payload = Message::Request {
        operation: Operation::ModelStatus,
        arguments: json!({}),
    };
    assert_eq!(
        status(&host, &registry, &phone, &req),
        Err(ErrorCode::Unauthorized)
    );
}

#[test]
fn resources_catalog_is_scoped_metadata_only_read_and_invokes_existing_agent_resources() {
    use zenpi::{
        remote::{host_adapter::HostAdapter, protocol::ExecutionState, resources::listing},
        resource_loader::ResourcePaths,
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    std::fs::create_dir_all(root.join(".zenpi/skills/research")).unwrap();
    std::fs::create_dir_all(root.join(".zenpi/prompts")).unwrap();
    std::fs::write(
        root.join(".zenpi/skills/research/SKILL.md"),
        format!(
            "---\nname: research\ndescription: Inspect {}\n---\nSKILL_BODY_NOT_CATALOGUED\n",
            root.display()
        ),
    )
    .unwrap();
    std::fs::write(
        root.join(".zenpi/prompts/fixture-review.md"),
        "---\ndescription: Review change\n---\nTEMPLATE_BODY_NOT_CATALOGUED $ARGUMENTS\n",
    )
    .unwrap();
    std::fs::write(
        root.join(".zenpi/prompts/help.md"),
        "RESERVED_TEMPLATE_NOT_ADVERTISED\n",
    )
    .unwrap();
    std::fs::write(
        root.join(".zenpi/prompts/review.code.md"),
        "DOTTED_TEMPLATE_BODY\n",
    )
    .unwrap();
    let mut pool = pool(&root);
    let project = pool.active_context().project_id.clone();
    let paths = ResourcePaths {
        user_skills: root.join("empty-skills"),
        project_skills: root.join(".zenpi/skills"),
        skill_paths: vec![],
        user_templates: root.join("empty-prompts"),
        project_templates: root.join(".zenpi/prompts"),
        template_paths: vec![],
        text_resources: vec![],
    };
    let owner = pool.owner(&project).unwrap();
    owner
        .lock()
        .unwrap()
        .configure_resources(paths.clone(), || false)
        .unwrap();
    let journal_path = owner.lock().unwrap().session().path().to_owned();
    let journal = std::fs::read(&journal_path).unwrap();
    let checkpoint = std::fs::read(root.join("project-workspace.json")).ok();
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let phone = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let mut req = request(
        &pool,
        Operation::ResourcesList,
        &project,
        "resources",
        json!({}),
    );
    req.required_owner = None;
    req.expected_revision = None;
    assert!(!Operation::ResourcesList.is_write());
    let value = listing(&pool, &registry, &phone, &req).unwrap();
    assert_eq!(value["entries"].as_array().unwrap().len(), 3);
    assert_eq!(value["entries"][0]["command"], "/skill:research");
    assert_eq!(value["entries"][1]["command"], "/fixture-review");
    assert_eq!(value["entries"][2]["command"], "/review.code");
    for secret in [
        "SKILL_BODY_NOT_CATALOGUED",
        "TEMPLATE_BODY_NOT_CATALOGUED",
        "DOTTED_TEMPLATE_BODY",
        "RESERVED_TEMPLATE_NOT_ADVERTISED",
        &root.to_string_lossy(),
    ] {
        assert!(!value.to_string().contains(secret));
    }
    assert_eq!(value["entries"][1]["description"], "Review change");
    assert_eq!(value["entries"][2]["description"], "");
    let response = HostAdapter::dispatch_owned(&mut pool, &registry, &phone, req.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    let (adapter, mailbox) = HostAdapter::owner_thread();
    std::thread::scope(|scope| {
        let pending = scope.spawn(|| adapter.dispatch(&registry, &phone, req.clone()));
        let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !pending.is_finished() && std::time::Instant::now() < until {
            mailbox.drain(&mut pool);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(pending.join().unwrap().payload, response.payload);
    });
    let mut bad = req.clone();
    bad.session_id = Some("foreign".into());
    assert_eq!(
        listing(&pool, &registry, &phone, &bad),
        Err(ErrorCode::OwnerMismatch)
    );
    bad = req.clone();
    bad.payload = Message::Request {
        operation: Operation::ResourcesList,
        arguments: json!({"body":true}),
    };
    assert_eq!(
        listing(&pool, &registry, &phone, &bad),
        Err(ErrorCode::InvalidRequest)
    );
    bad = req.clone();
    bad.expected_revision = Some(1);
    assert_eq!(
        listing(&pool, &registry, &phone, &bad),
        Err(ErrorCode::InvalidRequest)
    );
    let denied = device(&registry, vec![project.clone()], vec![Capability::Control]);
    assert_eq!(
        listing(&pool, &registry, &denied, &req),
        Err(ErrorCode::Unauthorized)
    );
    let held = owner.lock().unwrap();
    assert_eq!(
        listing(&pool, &registry, &phone, &req),
        Err(ErrorCode::Busy)
    );
    drop(held);
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        listing(&pool, &registry, &phone, &req),
        Err(ErrorCode::Unauthorized)
    );
    assert_eq!(std::fs::read(&journal_path).unwrap(), journal);
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).ok(),
        checkpoint
    );

    let phone = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let mut agent = owner.lock().unwrap();
    for command in ["/skill:research literal $1", "/fixture-review literal $1"] {
        let result = agent
            .process(zenpi::core::TurnInputRequest::new(command))
            .unwrap();
        assert!(!result.assistant.unwrap().content.is_empty());
    }
    std::fs::write(
        root.join(".zenpi/skills/research/SKILL.md"),
        "---\nname: research\ndescription: Updated\n---\nNEW_BODY\n",
    )
    .unwrap();
    drop(agent);
    let mut restored =
        Agent::with_echo(SessionStore::open_in_workspace(&journal_path, &root).unwrap());
    restored.restore_resources(paths, || false).unwrap();
    *owner.lock().unwrap() = restored;
    let stale = listing(&pool, &registry, &phone, &req).unwrap();
    assert_eq!(stale["entries"][0]["invocable"], false);
    assert!(
        owner
            .lock()
            .unwrap()
            .process(zenpi::core::TurnInputRequest::new("/skill:research"))
            .is_err()
    );
    owner.lock().unwrap().reload_resources(|| false).unwrap();
    assert_eq!(
        listing(&pool, &registry, &phone, &req).unwrap()["entries"][0]["invocable"],
        true
    );
    for index in 0..40 {
        std::fs::write(
            root.join(format!(".zenpi/prompts/fixture-{index:02}.md")),
            "bounded\n",
        )
        .unwrap();
    }
    owner.lock().unwrap().reload_resources(|| false).unwrap();
    let capped = listing(&pool, &registry, &phone, &req).unwrap();
    assert_eq!(capped["entries"].as_array().unwrap().len(), 32);
    assert_eq!(capped["truncated"], true);
    assert_eq!(capped["offset"], 0);
    assert_eq!(capped["next_offset"], 32);
    let mut next = req.clone();
    next.payload = Message::Request {
        operation: Operation::ResourcesList,
        arguments: json!({"offset":32,"snapshot_id":capped["snapshot_id"]}),
    };
    let tail = listing(&pool, &registry, &phone, &next).unwrap();
    assert_eq!(tail["entries"].as_array().unwrap().len(), 11);
    assert_eq!(tail["generation"], capped["generation"]);
    assert_eq!(tail["snapshot_id"], capped["snapshot_id"]);
    assert_eq!(tail["truncated"], false);
    assert!(tail["next_offset"].is_null());
    let mut all = capped["entries"].as_array().unwrap().clone();
    all.extend(tail["entries"].as_array().unwrap().iter().cloned());
    assert!(
        all.iter()
            .filter(
                |entry| entry["name"].as_str().unwrap().starts_with("fixture-")
                    && entry["name"] != "fixture-review"
            )
            .all(|entry| entry["description"] == "")
    );
    let names = all
        .iter()
        .map(|entry| entry["command"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(names.len(), 43);
    assert!(names.contains("/review.code"));
    let mut back = next.clone();
    back.payload = Message::Request {
        operation: Operation::ResourcesList,
        arguments: json!({"offset":0,"snapshot_id":capped["snapshot_id"]}),
    };
    assert_eq!(listing(&pool, &registry, &phone, &back).unwrap(), capped);
    for args in [
        json!({"offset":1,"snapshot_id":capped["snapshot_id"]}),
        json!({"offset":64,"snapshot_id":capped["snapshot_id"]}),
        json!({"offset":"32","snapshot_id":capped["snapshot_id"]}),
        json!({"offset":32}),
        json!({"offset":32,"snapshot_id":"bad"}),
        json!({"offset":32,"snapshot_id":capped["snapshot_id"],"extra":true}),
    ] {
        let mut bad = req.clone();
        bad.payload = Message::Request {
            operation: Operation::ResourcesList,
            arguments: args,
        };
        assert_eq!(
            listing(&pool, &registry, &phone, &bad),
            Err(ErrorCode::InvalidRequest)
        );
    }
    owner.lock().unwrap().reload_resources(|| false).unwrap();
    assert_eq!(
        listing(&pool, &registry, &phone, &next),
        Err(ErrorCode::RevisionConflict)
    );
    assert_eq!(
        listing(&pool, &registry, &phone, &back),
        Err(ErrorCode::RevisionConflict)
    );
    let same_generation = listing(&pool, &registry, &phone, &req).unwrap();
    std::fs::write(
        root.join(".zenpi/prompts/fixture-00.md"),
        "---\ndescription: Stable metadata\n---\nchanged body\n",
    )
    .unwrap();
    let paths = zenpi::resource_loader::ResourcePaths {
        user_skills: root.join("empty-skills"),
        project_skills: root.join(".zenpi/skills"),
        skill_paths: vec![],
        user_templates: root.join("empty-prompts"),
        project_templates: root.join(".zenpi/prompts"),
        template_paths: vec![],
        text_resources: vec![],
    };
    let mut restored =
        Agent::with_echo(SessionStore::open_in_workspace(&journal_path, &root).unwrap());
    restored.restore_resources(paths.clone(), || false).unwrap();
    *owner.lock().unwrap() = restored;
    next.payload = Message::Request {
        operation: Operation::ResourcesList,
        arguments: json!({"offset":32,"snapshot_id":same_generation["snapshot_id"]}),
    };
    assert_eq!(
        listing(&pool, &registry, &phone, &next),
        Err(ErrorCode::RevisionConflict)
    );
    assert_eq!(
        listing(&pool, &registry, &phone, &req).unwrap()["next_offset"],
        32
    );
    let restored_snapshot = listing(&pool, &registry, &phone, &req).unwrap();
    std::fs::write(
        root.join(".zenpi/prompts/fixture-00.md"),
        "---\ndescription: Stable metadata\n---\nnew body, same metadata\n",
    )
    .unwrap();
    let mut restored =
        Agent::with_echo(SessionStore::open_in_workspace(&journal_path, &root).unwrap());
    restored.restore_resources(paths, || false).unwrap();
    *owner.lock().unwrap() = restored;
    let updated_snapshot = listing(&pool, &registry, &phone, &req).unwrap();
    assert_eq!(
        updated_snapshot["generation"],
        restored_snapshot["generation"]
    );
    assert_eq!(updated_snapshot["entries"], restored_snapshot["entries"]);
    assert_ne!(
        updated_snapshot["snapshot_id"],
        restored_snapshot["snapshot_id"]
    );
    next.payload = Message::Request {
        operation: Operation::ResourcesList,
        arguments: json!({"offset":32,"snapshot_id":restored_snapshot["snapshot_id"]}),
    };
    assert_eq!(
        listing(&pool, &registry, &phone, &next),
        Err(ErrorCode::RevisionConflict)
    );
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        listing(&pool, &registry, &phone, &next),
        Err(ErrorCode::Unauthorized)
    );
}

#[test]
fn files_list_is_selected_read_only_bounded_and_revoke_fenced() {
    use zenpi::remote::{files::listing, host_adapter::HostAdapter, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    std::fs::create_dir(child.join("docs")).unwrap();
    std::fs::write(root.join("PRIMARY_ONLY"), "outside").unwrap();
    std::fs::write(child.join("file name.txt"), "FILE_BYTES_NOT_PROJECTED").unwrap();
    std::fs::write(child.join(".secret"), "hidden").unwrap();
    for name in [
        "credentials.json",
        "auth.json",
        "private.pem",
        "PRIVATE.KEY",
        "id_ed25519",
    ] {
        std::fs::write(child.join(name), "PRIVATE_FILE_NEVER_LISTED").unwrap();
    }
    std::fs::create_dir(child.join("credentials")).unwrap();
    std::fs::File::create(child.join("large.bin"))
        .unwrap()
        .set_len(10 * 1024 * 1024 + 1)
        .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("PRIMARY_ONLY"), child.join("link")).unwrap();
    let mut pool = pool(&root);
    let project = pool.active_context().project_id.clone();
    let initial = pool.subtab_state(&project).unwrap();
    pool.commit_subtabs(
        &project,
        initial.revision,
        initial
            .add_directory(&project, &root, &child, "child")
            .unwrap(),
    )
    .unwrap();
    let state = pool.subtab_state(&project).unwrap();
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let phone = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let mut req = request(
        &pool,
        Operation::FilesList,
        &project,
        "files",
        json!({"subtab_id":state.active_id,"subtab_revision":state.revision,"path":"."}),
    );
    req.required_owner = None;
    req.expected_revision = None;
    let journal = std::fs::read(
        pool.owner(&project)
            .unwrap()
            .lock()
            .unwrap()
            .session()
            .path(),
    )
    .unwrap();
    let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
    let value = listing(&pool, &registry, &phone, &req).unwrap();
    assert_eq!(value["entries"][0]["kind"], "directory");
    assert_eq!(value["entries"].as_array().unwrap().len(), 3);
    assert_eq!(value["entries"][2]["attachable"], false);
    assert!(!value.to_string().contains("PRIMARY_ONLY"));
    assert!(!value.to_string().contains("FILE_BYTES_NOT_PROJECTED"));
    assert!(!value.to_string().contains(&root.display().to_string()));
    let response = HostAdapter::dispatch_owned(&mut pool, &registry, &phone, req.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    let (adapter, mailbox) = HostAdapter::owner_thread();
    std::thread::scope(|scope| {
        let pending = scope.spawn(|| adapter.dispatch(&registry, &phone, req.clone()));
        let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !pending.is_finished() && std::time::Instant::now() < until {
            mailbox.drain(&mut pool);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(pending.join().unwrap().payload, response.payload);
    });
    for path in [
        "../",
        "/etc",
        "link",
        "file name.txt",
        ".secret",
        "missing",
        "credentials",
    ] {
        let mut bad = req.clone();
        bad.payload = Message::Request {
            operation: Operation::FilesList,
            arguments: json!({"subtab_id":state.active_id,"subtab_revision":state.revision,"path":path}),
        };
        assert_eq!(
            listing(&pool, &registry, &phone, &bad),
            Err(ErrorCode::InvalidRequest)
        );
    }
    let mut stale = req.clone();
    stale.session_id = Some("foreign".into());
    assert_eq!(
        listing(&pool, &registry, &phone, &stale),
        Err(ErrorCode::OwnerMismatch)
    );
    stale = req.clone();
    stale.payload = Message::Request {
        operation: Operation::FilesList,
        arguments: json!({"subtab_id":state.active_id,"subtab_revision":state.revision + 1,"path":"."}),
    };
    assert_eq!(
        listing(&pool, &registry, &phone, &stale),
        Err(ErrorCode::RevisionConflict)
    );
    for index in 0..80 {
        std::fs::write(child.join(format!("z-{index:03}")), "").unwrap();
    }
    let capped = listing(&pool, &registry, &phone, &req).unwrap();
    assert_eq!(capped["entries"].as_array().unwrap().len(), 64);
    assert_eq!(capped["truncated"], true);
    let owner = pool.owner(&project).unwrap();
    let held = owner.lock().unwrap();
    assert_eq!(
        listing(&pool, &registry, &phone, &req),
        Err(ErrorCode::Busy)
    );
    drop(held);
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        listing(&pool, &registry, &phone, &req),
        Err(ErrorCode::Unauthorized)
    );
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        journal
    );
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        checkpoint
    );
}

#[test]
fn files_pages_reach_every_entry_and_reject_changed_snapshot_without_read_side_effects() {
    use zenpi::remote::files::listing;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("catalog");
    std::fs::create_dir(&child).unwrap();
    for index in 0..140 {
        std::fs::write(
            child.join(format!("file-{index:03}.txt")),
            "PRIVATE_CONTENT",
        )
        .unwrap();
    }
    for name in [
        "credentials.json",
        "auth.json",
        "private.pem",
        "PRIVATE.KEY",
        "id_rsa",
    ] {
        std::fs::write(child.join(name), "PRIVATE_KEY").unwrap();
    }
    std::fs::write(child.join(".private"), "PRIVATE_KEY").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("initial.jsonl"), child.join("link")).unwrap();
    let host = pool(&root);
    let project = host.active_context().project_id.clone();
    let tabs = host.subtab_state(&project).unwrap();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let make = |offset: u64, snapshot: Value| {
        let mut req = request(
            &host,
            Operation::FilesList,
            &project,
            "page",
            json!({"subtab_id":tabs.active_id,
            "subtab_revision":tabs.revision,"path":"catalog","offset":offset,"snapshot_id":snapshot}),
        );
        req.required_owner = None;
        req.expected_revision = None;
        req
    };
    let owner = host.owner(&project).unwrap();
    let journal = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    let first = listing(&host, &registry, &phone, &make(0, Value::Null)).unwrap();
    let digest = first["page"]["snapshot_id"].clone();
    assert_eq!(first["entries"].as_array().unwrap().len(), 64);
    assert_eq!(first["page"]["next_offset"], 64);
    let middle = listing(&host, &registry, &phone, &make(64, digest.clone())).unwrap();
    assert_eq!(middle["page"]["snapshot_id"], digest);
    assert_eq!(middle["page"]["next_offset"], 128);
    let tail = listing(&host, &registry, &phone, &make(128, digest.clone())).unwrap();
    assert_eq!(tail["entries"].as_array().unwrap().len(), 12);
    assert!(tail["page"]["next_offset"].is_null());
    let names: Vec<_> = [&first, &middle, &tail]
        .into_iter()
        .flat_map(|v| {
            v["entries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item["name"].as_str().unwrap().to_owned())
        })
        .collect();
    assert_eq!(
        names,
        (0..140)
            .map(|index| format!("file-{index:03}.txt"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        listing(&host, &registry, &phone, &make(0, digest.clone())).unwrap(),
        first
    );
    for bad in [
        make(1, digest.clone()),
        make(64, Value::Null),
        make(192, digest.clone()),
        make(u64::MAX, digest.clone()),
    ] {
        assert_eq!(
            listing(&host, &registry, &phone, &bad),
            Err(ErrorCode::InvalidRequest)
        );
    }
    for page in [&first, &middle, &tail] {
        for secret in [
            "PRIVATE_CONTENT",
            "PRIVATE_KEY",
            ".private",
            "link",
            &root.display().to_string(),
        ] {
            assert!(!page.to_string().contains(secret));
        }
    }
    std::fs::write(child.join("file-139.txt"), "changed-length").unwrap();
    assert_eq!(
        listing(&host, &registry, &phone, &make(64, digest.clone())),
        Err(ErrorCode::RevisionConflict)
    );
    let fresh = listing(&host, &registry, &phone, &make(0, Value::Null)).unwrap();
    assert_ne!(fresh["page"]["snapshot_id"], digest);
    let mut foreign = make(64, fresh["page"]["snapshot_id"].clone());
    foreign.session_id = Some("foreign".into());
    assert_eq!(
        listing(&host, &registry, &phone, &foreign),
        Err(ErrorCode::OwnerMismatch)
    );
    let mut stale = make(64, fresh["page"]["snapshot_id"].clone());
    if let Message::Request { arguments, .. } = &mut stale.payload {
        arguments["subtab_revision"] = json!(tabs.revision + 1);
    }
    assert_eq!(
        listing(&host, &registry, &phone, &stale),
        Err(ErrorCode::RevisionConflict)
    );
    std::fs::write(child.join("new-file.txt"), "").unwrap();
    assert_eq!(
        listing(
            &host,
            &registry,
            &phone,
            &make(64, fresh["page"]["snapshot_id"].clone())
        ),
        Err(ErrorCode::RevisionConflict)
    );
    for index in 140..2049 {
        std::fs::write(child.join(format!("file-{index:04}.txt")), "").unwrap();
    }
    assert_eq!(
        listing(&host, &registry, &phone, &make(0, Value::Null)),
        Err(ErrorCode::ResourceLimit)
    );
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        listing(
            &host,
            &registry,
            &phone,
            &make(64, fresh["page"]["snapshot_id"].clone())
        ),
        Err(ErrorCode::Unauthorized)
    );
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        journal
    );
}

#[test]
fn transcript_read_excludes_abandoned_tree_branch_and_resumes_when_selected() {
    use zenpi::core::{Turn, TurnRole};
    use zenpi::remote::transcript::read;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut pool = pool(&root);
    let project = pool.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let owner = pool.owner(&project).unwrap();
    let branch;
    let other;
    let content = "branch a\n".repeat(2000);
    {
        let mut agent = owner.lock().unwrap();
        let session = agent.session_mut();
        session
            .append_turn(Turn::new("root", TurnRole::User, "root"))
            .unwrap();
        session
            .append_turn(Turn::with_parent(
                "branch-a",
                "root",
                TurnRole::Assistant,
                content.clone(),
            ))
            .unwrap();
        session.enable_tree(Default::default(), &|| false).unwrap();
        let tree = session.tree_snapshot(&|| false).unwrap();
        branch = tree.entry_for_turn("branch-a").unwrap().id.clone();
        let root_entry = tree.entry_for_turn("root").unwrap().id.clone();
        session
            .select_tree_leaf(Some(&root_entry), &|| false)
            .unwrap();
        session
            .append_turn(Turn::with_parent(
                "branch-b",
                "root",
                TurnRole::Assistant,
                "branch b",
            ))
            .unwrap();
        let tree = session.tree_snapshot(&|| false).unwrap();
        other = tree.entry_for_turn("branch-b").unwrap().id.clone();
        session.select_tree_leaf(Some(&other), &|| false).unwrap();
    }
    let journal_revision = owner.lock().unwrap().session().next_sequence();
    let mut history_request = request(
        &pool,
        Operation::TranscriptRead,
        &project,
        "tree-history",
        json!({"view":"session_history","offset":0,"digest":null,"journal_revision":journal_revision}),
    );
    history_request.required_owner = None;
    history_request.expected_revision = None;
    let history = read(&pool, &registry, &phone, &history_request).unwrap();
    assert_eq!(
        history["turns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["root", "branch-b"]
    );
    let mut req = request(
        &pool,
        Operation::TranscriptRead,
        &project,
        "tree-page",
        json!({"turn_id":"branch-a","offset":0,"digest":null}),
    );
    req.required_owner = None;
    req.expected_revision = None;
    assert_eq!(
        read(&pool, &registry, &phone, &req),
        Err(ErrorCode::InvalidRequest)
    );
    owner
        .lock()
        .unwrap()
        .session_mut()
        .select_tree_leaf(Some(&branch), &|| false)
        .unwrap();
    assert_eq!(
        read(&pool, &registry, &phone, &history_request),
        Err(ErrorCode::RevisionConflict)
    );
    let page = read(&pool, &registry, &phone, &req).unwrap();
    assert_eq!(page["text"], &content[..8192]);
    assert_eq!(page["has_more"], true);
    req.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"branch-a","offset":page["next_offset"],"digest":page["digest"]}),
    };
    for leaf in [Some(other.as_str()), None, Some(branch.as_str())] {
        let before = {
            let mut agent = owner.lock().unwrap();
            agent
                .session_mut()
                .select_tree_leaf(leaf, &|| false)
                .unwrap();
            std::fs::read(agent.session().path()).unwrap()
        };
        let result = read(&pool, &registry, &phone, &req);
        if leaf == Some(branch.as_str()) {
            let next = result.unwrap();
            assert_eq!(next["text"], &content[8192..16384]);
            assert_eq!(next["digest"], page["digest"]);
        } else {
            assert_eq!(result, Err(ErrorCode::InvalidRequest));
        }
        let expected = match leaf {
            Some(value) if value == branch => vec!["root", "branch-a"],
            Some(_) => vec!["root", "branch-b"],
            None => vec![],
        };
        let mut inspect = req.clone();
        inspect.payload = Message::Request {
            operation: Operation::Inspect,
            arguments: json!({}),
        };
        let response = zenpi::remote::host_adapter::HostAdapter::dispatch_owned(
            &mut pool,
            &registry,
            &phone,
            inspect.clone(),
        );
        assert_eq!(
            response.execution_state,
            Some(zenpi::remote::protocol::ExecutionState::Succeeded)
        );
        let Message::Response { result } = response.payload else {
            panic!("inspect")
        };
        let ids = result["transcript"]
            .as_array()
            .unwrap()
            .iter()
            .map(|turn| turn["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(ids, expected);
        assert_eq!(
            result["transcript_truncated"],
            leaf == Some(branch.as_str())
        );
        assert_eq!(result["event_cursor"]["next_sequence"], 3);
        inspect.payload = Message::Resume {
            cursor: zenpi::remote::protocol::EventCursor {
                session_id: pool.active_context().session_id.clone(),
                next_sequence: 0,
            },
            limit: 64,
        };
        let response = zenpi::remote::host_adapter::HostAdapter::dispatch_owned(
            &mut pool, &registry, &phone, inspect,
        );
        assert_eq!(
            response.execution_state,
            Some(zenpi::remote::protocol::ExecutionState::Succeeded)
        );
        let frames = zenpi::remote::host_adapter::HostAdapter::wire_frames(response).unwrap();
        let Message::Snapshot { state, cursor } = &frames[0].payload else {
            panic!("snapshot")
        };
        assert_eq!(cursor.next_sequence, 3);
        assert_eq!(state["transcript"], result["transcript"]);
        assert_eq!(
            std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
            before
        );
    }
    assert_eq!(owner.lock().unwrap().session().turns().len(), 3);
}

#[test]
fn ordinary_tool_argument_pages_bind_turn_call_branch_and_current_read_grant() {
    use zenpi::core::{Turn, TurnRole};
    use zenpi::remote::transcript::{PAGE_BYTES, read};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let outsider = device(&registry, vec!["foreign".into()], vec![Capability::Read]);
    let owner = host.owner(&project).unwrap();
    let arguments = json!({"api_key":"PRIVATE_ARGUMENT_SECRET","text":"\u{4e2d}".repeat(7000)});
    let safe = serde_json::to_string(&zenpi::security::redact_json(&arguments, &[])).unwrap();
    {
        let mut agent = owner.lock().unwrap();
        agent
            .session_mut()
            .append_turn(Turn::new("input", TurnRole::User, "request"))
            .unwrap();
        let mut turn = Turn::with_parent("tools", "input", TurnRole::Assistant, "ordinary message");
        turn.metadata = Some(
            json!({"tool_calls":[{"id":"call","name":"read_file","arguments":arguments,
            "execution":"PRIVATE_EXECUTION_EVIDENCE"},{"id":"other","name":"read_file","arguments":{"text":"other"}}]}),
        );
        agent.session_mut().append_turn(turn).unwrap();
        let path = agent.session().path().to_path_buf();
        agent.resume_session(path).unwrap();
    }
    let before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    let mut req = request(
        &host,
        Operation::TranscriptRead,
        &project,
        "tool-page",
        json!({"turn_id":"tools","tool_call_id":"call","offset":0,"digest":null}),
    );
    req.required_owner = None;
    req.expected_revision = None;
    let first = read(&host, &registry, &phone, &req).unwrap();
    assert_eq!(first["tool_call_id"], "call");
    assert_eq!(first["tool_name"], "read_file");
    assert_eq!(first["role"], "assistant");
    assert_eq!(first["project_id"], project);
    assert_eq!(first["session_id"], host.active_context().session_id);
    let mut value = first.clone();
    let mut text = String::new();
    loop {
        let page = value["text"].as_str().unwrap();
        assert!(page.len() <= PAGE_BYTES);
        text.push_str(page);
        if value["has_more"] == false {
            break;
        }
        req.payload = Message::Request {
            operation: Operation::TranscriptRead,
            arguments: json!({"turn_id":"tools","tool_call_id":"call","offset":value["next_offset"],"digest":first["digest"]}),
        };
        value = read(&host, &registry, &phone, &req).unwrap();
    }
    assert_eq!(text, safe);
    assert!(!text.contains("PRIVATE_ARGUMENT_SECRET"));
    assert!(!text.contains("PRIVATE_EXECUTION_EVIDENCE"));
    assert_eq!(
        first["digest"],
        format!("{:x}", sha2::Sha256::digest(safe.as_bytes()))
    );
    for (args, expected) in [
        (
            json!({"turn_id":"tools","tool_call_id":"missing","offset":0,"digest":null}),
            ErrorCode::InvalidRequest,
        ),
        (
            json!({"turn_id":"input","tool_call_id":"call","offset":0,"digest":null}),
            ErrorCode::InvalidRequest,
        ),
        (
            json!({"turn_id":"tools","tool_call_id":"call","offset":1,"digest":null}),
            ErrorCode::InvalidRequest,
        ),
        (
            json!({"turn_id":"tools","tool_call_id":"call","offset":1,"digest":"0".repeat(64)}),
            ErrorCode::RevisionConflict,
        ),
        (
            json!({"turn_id":"tools","tool_call_id":"other","offset":0,"digest":first["digest"]}),
            ErrorCode::RevisionConflict,
        ),
        (
            json!({"turn_id":"tools","tool_call_id":"call","view":"history","offset":0,"digest":null}),
            ErrorCode::InvalidRequest,
        ),
    ] {
        req.payload = Message::Request {
            operation: Operation::TranscriptRead,
            arguments: args,
        };
        assert_eq!(read(&host, &registry, &phone, &req), Err(expected));
    }
    req.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"tools","tool_call_id":"call","offset":0,"digest":null}),
    };
    assert_eq!(
        read(&host, &registry, &outsider, &req),
        Err(ErrorCode::Unauthorized)
    );
    req.session_id = Some("foreign".into());
    assert_eq!(
        read(&host, &registry, &phone, &req),
        Err(ErrorCode::OwnerMismatch)
    );
    req.session_id = Some(host.active_context().session_id.clone());
    let held = owner.lock().unwrap();
    assert_eq!(read(&host, &registry, &phone, &req), Err(ErrorCode::Busy));
    drop(held);
    let response = zenpi::remote::host_adapter::HostAdapter::dispatch_owned(
        &mut host,
        &registry,
        &phone,
        req.clone(),
    );
    assert!(matches!(response.payload, Message::Response { result } if result == first));
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        before
    );
    let tool_leaf;
    {
        let mut agent = owner.lock().unwrap();
        let session = agent.session_mut();
        session.enable_tree(Default::default(), &|| false).unwrap();
        let tree = session.tree_snapshot(&|| false).unwrap();
        tool_leaf = tree.entry_for_turn("tools").unwrap().id.clone();
        let input_leaf = tree.entry_for_turn("input").unwrap().id.clone();
        session
            .select_tree_leaf(Some(&input_leaf), &|| false)
            .unwrap();
    }
    assert_eq!(
        read(&host, &registry, &phone, &req),
        Err(ErrorCode::InvalidRequest)
    );
    owner
        .lock()
        .unwrap()
        .session_mut()
        .select_tree_leaf(Some(&tool_leaf), &|| false)
        .unwrap();
    assert_eq!(read(&host, &registry, &phone, &req).unwrap(), first);
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        read(&host, &registry, &phone, &req),
        Err(ErrorCode::Unauthorized)
    );
}

#[test]
fn transcript_pages_reconstruct_redacted_durable_message_and_refuse_foreign_or_stale_reads() {
    use zenpi::core::{Turn, TurnRole};
    use zenpi::remote::transcript::{PAGE_BYTES, read};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut pool = pool(&root);
    let project = pool.active_context().project_id.clone();
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let phone = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let owner = pool.owner(&project).unwrap();
    let raw = format!(
        "safe\n-----BEGIN PRIVATE KEY-----\nPRIVATE_PAGE_SECRET\n-----END PRIVATE KEY-----\n{}",
        "\u{4e2d} answer\n".repeat(5000)
    );
    {
        let mut agent = owner.lock().unwrap();
        agent
            .session_mut()
            .append_turn(Turn::new("paged", TurnRole::Assistant, raw.clone()))
            .unwrap();
        let path = agent.session().path().to_path_buf();
        agent.resume_session(path).unwrap();
    }
    let before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    let mut req = request(
        &pool,
        Operation::TranscriptRead,
        &project,
        "read-page",
        json!({"turn_id":"paged","offset":0,"digest":null}),
    );
    req.required_owner = None;
    req.expected_revision = None;
    let mut offset = 0u64;
    let mut digest = None;
    let mut reconstructed = String::new();
    let mut pages = Vec::new();
    loop {
        req.payload = Message::Request {
            operation: Operation::TranscriptRead,
            arguments: json!({"turn_id":"paged","offset":offset,"digest":digest}),
        };
        let value = read(&pool, &registry, &phone, &req).unwrap();
        let text = value["text"].as_str().unwrap();
        assert!(text.len() <= PAGE_BYTES);
        assert_eq!(value["offset"], offset);
        assert!(!text.contains("PRIVATE_PAGE_SECRET"));
        pages.push(value.clone());
        reconstructed.push_str(text);
        offset = value["next_offset"].as_u64().unwrap();
        digest = Some(value["digest"].as_str().unwrap().to_owned());
        if value["has_more"] == false {
            assert_eq!(value["total_bytes"], offset);
            break;
        }
    }
    assert_eq!(reconstructed, zenpi::security::redact_text(&raw, &[]));
    assert_eq!(
        digest.as_deref().unwrap(),
        format!("{:x}", sha2::Sha256::digest(reconstructed.as_bytes()))
    );
    // Going back locally and then forward must re-read the same digest-bound
    // page, without mutating journal or durable transport cursors.
    for page in pages.iter().rev().take(7) {
        req.payload = Message::Request {
            operation: Operation::TranscriptRead,
            arguments: json!({"turn_id":"paged","offset":page["offset"],"digest":digest}),
        };
        assert_eq!(read(&pool, &registry, &phone, &req).unwrap(), *page);
    }
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        before
    );
    req.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"paged","offset":1,"digest":"0".repeat(64)}),
    };
    assert_eq!(
        read(&pool, &registry, &phone, &req),
        Err(ErrorCode::RevisionConflict)
    );
    req.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"paged","offset":1,"digest":null}),
    };
    assert_eq!(
        read(&pool, &registry, &phone, &req),
        Err(ErrorCode::InvalidRequest)
    );
    req.session_id = Some("foreign".into());
    assert_eq!(
        read(&pool, &registry, &phone, &req),
        Err(ErrorCode::InvalidRequest)
    );
    req.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"paged","offset":0,"digest":null}),
    };
    assert_eq!(
        read(&pool, &registry, &phone, &req),
        Err(ErrorCode::OwnerMismatch)
    );
    req.session_id = Some(pool.active_context().session_id.clone());
    let response = zenpi::remote::host_adapter::HostAdapter::dispatch_owned(
        &mut pool,
        &registry,
        &phone,
        req.clone(),
    );
    assert_eq!(
        response.execution_state,
        Some(zenpi::remote::protocol::ExecutionState::Succeeded)
    );
    let Message::Response { result } = response.payload else {
        panic!("page response")
    };
    assert_eq!(result["offset"], 0);
    let (mailbox_adapter, mailbox) = zenpi::remote::host_adapter::HostAdapter::owner_thread();
    std::thread::scope(|scope| {
        let pending = scope.spawn(|| mailbox_adapter.dispatch(&registry, &phone, req.clone()));
        let until = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while !pending.is_finished() && std::time::Instant::now() < until {
            mailbox.drain(&mut pool);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(
            pending.join().unwrap().execution_state,
            Some(zenpi::remote::protocol::ExecutionState::Succeeded)
        );
    });
    let unicode = result["text"].as_str().unwrap().find('\u{4e2d}').unwrap();
    req.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"paged","offset":unicode + 1,"digest":result["digest"]}),
    };
    assert_eq!(
        read(&pool, &registry, &phone, &req),
        Err(ErrorCode::InvalidRequest)
    );
    req.payload = Message::Request {
        operation: Operation::TranscriptRead,
        arguments: json!({"turn_id":"paged","offset":0,"digest":null}),
    };
    let held = owner.lock().unwrap();
    assert_eq!(read(&pool, &registry, &phone, &req), Err(ErrorCode::Busy));
    drop(held);
    req.project_id = Some("foreign".into());
    assert_eq!(
        read(&pool, &registry, &phone, &req),
        Err(ErrorCode::Unauthorized)
    );
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        before
    );
    registry.revoke(phone.id()).unwrap();
    req.project_id = Some(project);
    assert_eq!(
        read(&pool, &registry, &phone, &req),
        Err(ErrorCode::Unauthorized)
    );
}

#[test]
fn live_prompt_overflow_projects_explicit_truncation_without_cursor_or_durable_state_change() {
    use zenpi::backend::ProviderEvent;
    use zenpi::remote::{command::PromptProgress, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let pool = pool(&root);
    let project = pool.active_context().project_id.clone();
    let request = request(
        &pool,
        Operation::Prompt,
        &project,
        "preview-overflow",
        json!({"text":"hello"}),
    );
    let owner = pool.owner(&project).unwrap();
    let before = owner.lock().unwrap().session().events().len();
    let progress = PromptProgress::default();
    progress.record(&request, "phone", ExecutionState::Running);
    progress.provider_event(
        &request,
        "phone",
        &ProviderEvent::TextDelta {
            delta: "visible\napi_".into(),
        },
    );
    progress.provider_event(
        &request,
        "phone",
        &ProviderEvent::TextDelta {
            delta: format!("key=PRIVATE_OVERFLOW{}", "x".repeat(4096)),
        },
    );
    let statuses = progress.listing(&pool, "phone", &project);
    assert_eq!(statuses[0]["stream_text"], "visible\n");
    assert_eq!(statuses[0]["stream_truncated"], true);
    assert_eq!(statuses[0]["state"], "running");
    assert!(
        progress
            .listing(&pool, "other-phone", &project)
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(owner.lock().unwrap().session().events().len(), before);
    progress.record(&request, "phone", ExecutionState::Succeeded);
    let statuses = progress.listing(&pool, "phone", &project);
    assert!(statuses[0].get("stream_text").is_none());
    assert!(statuses[0].get("stream_truncated").is_none());
}
fn sign(key: &EcdsaKeyPair, message: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(
        key.sign(&ring::rand::SystemRandom::new(), message)
            .unwrap()
            .as_ref(),
    )
}
fn device(
    registry: &DeviceRegistry,
    projects: Vec<String>,
    capabilities: Vec<Capability>,
) -> AuthenticatedDevice {
    let random = ring::rand::SystemRandom::new();
    let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random).unwrap();
    let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    let record = registry
        .verify_pair_and_register(
            "token",
            "pair",
            "phone",
            &public,
            &sign(
                &key,
                &pairing_proof_message(&"a".repeat(64), "token", "pair", "phone", &public),
            ),
            || true,
        )
        .unwrap();
    registry
        .set_permissions(&record.id, record.revision, capabilities, projects)
        .unwrap();
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    challenge
        .authenticate(
            registry,
            &record.id,
            &sign(&key, &challenge.proof_message(&record.id)),
        )
        .unwrap()
}
fn request(
    pool: &ProjectOwnerPool,
    operation: Operation,
    id: &str,
    request_id: &str,
    arguments: Value,
) -> Envelope {
    let mut request = Envelope::new(
        Channel::Control,
        Message::Request {
            operation,
            arguments,
        },
    );
    request.id = Some(request_id.into());
    request.project_id = Some(id.into());
    request.session_id = Some(pool.context(id).unwrap().session_id.clone());
    request.required_owner = Some(pool.workspace_owner().into());
    request.expected_revision = Some(pool.workspace_revision());
    request
}

#[test]
fn remote_prompt_never_consumes_local_staged_attachments_or_reserves_a_blocked_request() {
    use zenpi::remote::command::{dispatch_prompt, prompt_input};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let owner = host.owner(&project).unwrap();
    let attachment = zenpi::backend::InputAttachment {
        kind: zenpi::backend::AttachmentKind::Image,
        mime_type: "image/png".into(),
        path: None,
        url: Some("https://example.invalid/local.png".into()),
        file_id: None,
    };
    owner
        .lock()
        .unwrap()
        .stage_attachment(attachment.clone())
        .unwrap();
    let input = request(
        &host,
        Operation::Prompt,
        &project,
        "staged-fence",
        json!({"text":"remote text"}),
    );
    let before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    let checkpoint = std::fs::read(root.join("project-workspace.json")).ok();
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &phone,
            &input,
            false,
            |_, _, _| panic!("staged attachment admitted")
        )
        .code,
        Some(ErrorCode::Busy)
    );
    {
        let agent = owner.lock().unwrap();
        assert!(prompt_input(&agent, "queued remote".into()).is_err());
        assert_eq!(
            agent.pending_attachments(),
            std::slice::from_ref(&attachment)
        );
    }
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        before
    );
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).ok(),
        checkpoint
    );
    owner.lock().unwrap().clear_pending_attachments();
    let accepted = dispatch_prompt(&mut host, &registry, &phone, &input, false, |_, _, _| {
        Ok("job".into())
    });
    assert_eq!(
        accepted.execution_state,
        Some(zenpi::remote::protocol::ExecutionState::Accepted)
    );
    owner
        .lock()
        .unwrap()
        .stage_attachment(attachment.clone())
        .unwrap();
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &phone,
            &input,
            true,
            |_, _, _| panic!("retry resubmitted")
        ),
        accepted
    );
    let mut agent = owner.lock().unwrap();
    assert!(prompt_input(&agent, "queued remote".into()).is_err());
    assert_eq!(agent.pending_attachments(), &[attachment]);
    agent.clear_pending_attachments();
    let value = prompt_input(&agent, "mail@example.org".into()).unwrap();
    assert!(value.attachments.is_empty());
    assert_eq!(value.mode, zenpi::protocol::TurnMode::StartIfIdle);
}

#[test]
fn remote_prompt_admission_is_durable_acl_fenced_and_never_resubmitted() {
    use zenpi::remote::{command::dispatch_prompt, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let input = request(
        &host,
        Operation::Prompt,
        &project,
        "remote-prompt",
        json!({"text":"hello"}),
    );
    let owner = host.owner(&project).unwrap();
    let read_only = device(&registry, vec![project.clone()], vec![Capability::Read]);
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &read_only,
            &input,
            false,
            |_, _, _| panic!()
        )
        .code,
        Some(ErrorCode::Unauthorized)
    );
    let accepted = dispatch_prompt(
        &mut host,
        &registry,
        &phone,
        &input,
        false,
        |actual, text, _| {
            assert!(Arc::ptr_eq(&owner, &actual));
            assert_eq!(text, "hello");
            Ok("job-1".into())
        },
    );
    assert_eq!(accepted.execution_state, Some(ExecutionState::Accepted));
    use zenpi::remote::command::{PromptProgress, persist_progress};
    let live = PromptProgress::default();
    live.record(&input, phone.id(), ExecutionState::Accepted);
    assert_eq!(
        live.listing(&host, phone.id(), &project)[0]["state"],
        "accepted"
    );
    let held = owner.lock().unwrap();
    live.record(&input, phone.id(), ExecutionState::Running);
    assert_eq!(
        live.listing(&host, phone.id(), &project)[0]["state"],
        "running"
    );
    assert!(
        live.listing(&host, read_only.id(), &project)
            .as_array()
            .unwrap()
            .is_empty()
    );
    drop(held);
    persist_progress(
        &mut owner.lock().unwrap(),
        &input,
        phone.id(),
        ExecutionState::Succeeded,
    )
    .unwrap();
    live.record(&input, phone.id(), ExecutionState::Succeeded);
    assert_eq!(
        live.listing(&host, phone.id(), &project)[0]["state"],
        "succeeded"
    );
    assert_eq!(
        PromptProgress::default().listing(&host, phone.id(), &project)[0]["state"],
        "succeeded"
    );
    let duplicate = dispatch_prompt(&mut host, &registry, &phone, &input, true, |_, _, _| {
        panic!("duplicate started work")
    });
    assert_eq!(accepted, duplicate);
    let mut changed = input.clone();
    if let Message::Request { arguments, .. } = &mut changed.payload {
        arguments["text"] = json!("changed");
    }
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &phone,
            &changed,
            false,
            |_, _, _| panic!()
        )
        .code,
        Some(ErrorCode::RequestIdConflict)
    );
    for (name, change, expected) in [
        ("busy", 0, ErrorCode::Busy),
        ("owner", 1, ErrorCode::OwnerMismatch),
        ("revision", 2, ErrorCode::RevisionConflict),
        ("shell", 3, ErrorCode::InvalidRequest),
    ] {
        let mut other = input.clone();
        other.id = Some(name.into());
        if change == 1 {
            other.required_owner = Some("foreign".into());
        }
        if change == 2 {
            other.expected_revision = Some(host.workspace_revision() + 1);
        }
        if change == 3
            && let Message::Request { arguments, .. } = &mut other.payload
        {
            arguments["text"] = json!("!rm");
        }
        assert_eq!(
            dispatch_prompt(
                &mut host,
                &registry,
                &phone,
                &other,
                change == 0,
                |_, _, _| panic!()
            )
            .code,
            Some(expected)
        );
    }
    let mut refused = input.clone();
    refused.id = Some("runner-full".into());
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &phone,
            &refused,
            false,
            |_, _, _| Err(ErrorCode::Busy)
        )
        .execution_state,
        Some(ExecutionState::Failed)
    );
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &phone,
            &refused,
            false,
            |_, _, _| panic!()
        )
        .execution_state,
        Some(ExecutionState::Failed)
    );
    let mut reserved = input.clone();
    reserved.id = Some("interrupted-admission".into());
    use sha2::{Digest, Sha256};
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&reserved).unwrap())
    );
    owner.lock().unwrap().session_mut().append_event(json!({"type":"remote_prompt_admission_v1", "device_id":phone.id(),
        "request_id":"interrupted-admission", "fingerprint":fingerprint, "state":"reserved", "result":null})).unwrap();
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &phone,
            &reserved,
            false,
            |_, _, _| panic!("interrupted admission restarted work")
        )
        .execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    let held = owner.lock().unwrap();
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &phone,
            &input,
            false,
            |_, _, _| panic!()
        )
        .execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &read_only,
            &input,
            false,
            |_, _, _| panic!()
        )
        .code,
        Some(ErrorCode::Unauthorized)
    );
    drop(held);
    let path = owner.lock().unwrap().session().path().to_owned();
    owner.lock().unwrap().resume_session(path).unwrap();
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &phone,
            &input,
            false,
            |_, _, _| panic!()
        ),
        accepted
    );
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &phone,
            &input,
            false,
            |_, _, _| panic!()
        )
        .code,
        Some(ErrorCode::Unauthorized)
    );
    // Keep the host mutable in the fixture to mirror the production owner pool.
    host.restore_checkpoint().unwrap();
}

#[test]
fn remote_approval_one_shot_decision_is_digest_fenced_durable_and_separately_authorized() {
    use zenpi::approval::{ApprovalDecision, ApprovalPolicy, ApprovalRequest, ApprovalResponse};
    use zenpi::remote::approval::dispatch;
    use zenpi::tools::{ToolContext, ToolOrigin, ToolSideEffect};
    for decision in [ApprovalDecision::Allow, ApprovalDecision::Deny] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let mut agent = Agent::with_echo(
            SessionStore::open_in_workspace(root.join("initial.jsonl"), &root).unwrap(),
        );
        agent.set_attachment_workspace(ToolContext::new(&root).unwrap());
        let coordinator = agent.approval_coordinator().unwrap();
        let owner = Arc::new(Mutex::new(agent));
        let mut host = ProjectOwnerPool::new(owner.clone()).unwrap();
        host.restore_checkpoint().unwrap();
        let project = host.active_context().project_id.clone();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let phone = device(
            &registry,
            vec![project.clone()],
            vec![Capability::Read, Capability::Approval, Capability::Control],
        );
        let controller = device(
            &registry,
            vec![project.clone()],
            vec![Capability::Read, Capability::Control],
        );
        let target = ApprovalRequest {
            request_id: "approval".into(),
            turn_id: "turn".into(),
            call_id: "call".into(),
            tool: "write_file".into(),
            side_effect: ToolSideEffect::WorkspaceWrite,
            arguments: json!({"content":"secret"}),
            preview: None,
            origin: ToolOrigin::AgentTool,
            policy_digest: None,
            lease_id: None,
        };
        let digest = target.digest().unwrap();
        let worker = coordinator.clone();
        let join = std::thread::spawn(move || {
            worker.request(target, &mut ApprovalPolicy::default(), &|| false)
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !coordinator.is_pending("approval") && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let command = request(
            &host,
            Operation::Approve,
            &project,
            "answer",
            json!({"request_id":"approval","digest":digest,"decision":decision}),
        );
        let held = owner.lock().unwrap();
        assert_eq!(
            dispatch(&mut host, &registry, &controller, &command).code,
            Some(ErrorCode::Unauthorized)
        );
        let mut wrong = command.clone();
        wrong.required_owner = Some("foreign".into());
        assert_eq!(
            dispatch(&mut host, &registry, &phone, &wrong).code,
            Some(ErrorCode::OwnerMismatch)
        );
        let mut wrong = command.clone();
        wrong.expected_revision = Some(999);
        assert_eq!(
            dispatch(&mut host, &registry, &phone, &wrong).code,
            Some(ErrorCode::RevisionConflict)
        );
        let mut wrong = command.clone();
        if let Message::Request { arguments, .. } = &mut wrong.payload {
            arguments["digest"] = json!("b".repeat(64));
        }
        assert_eq!(
            dispatch(&mut host, &registry, &phone, &wrong).code,
            Some(ErrorCode::InvalidRequest)
        );
        assert!(coordinator.is_pending("approval"));
        let accepted = dispatch(&mut host, &registry, &phone, &command);
        assert_eq!(
            accepted.execution_state,
            Some(zenpi::remote::protocol::ExecutionState::Accepted)
        );
        assert_eq!(dispatch(&mut host, &registry, &phone, &command), accepted);
        assert_eq!(join.join().unwrap().unwrap(), decision);
        assert!(
            coordinator
                .respond(ApprovalResponse {
                    request_id: "approval".into(),
                    decision: ApprovalDecision::Deny,
                    remember: false,
                    message: None
                })
                .is_err()
        );
        let saved: Value =
            serde_json::from_slice(&std::fs::read(root.join("project-workspace.json")).unwrap())
                .unwrap();
        assert_eq!(
            saved["remote_workspace"]["approval_receipts"][0]["binding"]["state"],
            "accepted"
        );
        assert!(!saved.to_string().contains("secret"));
        let mut restored = pool(&root);
        assert_ne!(restored.workspace_owner(), host.workspace_owner());
        assert_eq!(
            dispatch(&mut restored, &registry, &phone, &command),
            accepted
        );
        let path = root.join("project-workspace.json");
        let mut unresolved = saved.clone();
        unresolved["remote_workspace"]["approval_receipts"][0]["binding"]["state"] =
            json!("unknown_outcome");
        std::fs::write(&path, serde_json::to_vec(&unresolved).unwrap()).unwrap();
        let mut uncertain = pool(&root);
        assert_eq!(
            dispatch(&mut uncertain, &registry, &phone, &command).execution_state,
            Some(zenpi::remote::protocol::ExecutionState::UnknownOutcome)
        );
        let mut corrupt = saved.clone();
        corrupt["remote_workspace"]["approval_receipts"][0]["digest"] = json!("bad");
        std::fs::write(&path, serde_json::to_vec(&corrupt).unwrap()).unwrap();
        let agent = Agent::with_echo(
            SessionStore::open_in_workspace(root.join("initial.jsonl"), &root).unwrap(),
        );
        let mut failed = ProjectOwnerPool::new(Arc::new(Mutex::new(agent))).unwrap();
        assert!(
            failed
                .restore_checkpoint()
                .unwrap_err()
                .contains("approval receipts")
        );
        std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
        let mut changed = command.clone();
        if let Message::Request { arguments, .. } = &mut changed.payload {
            arguments["decision"] = json!(if decision == ApprovalDecision::Allow {
                "deny"
            } else {
                "allow"
            });
        }
        assert_eq!(
            dispatch(&mut restored, &registry, &phone, &changed).code,
            Some(ErrorCode::RequestIdConflict)
        );
        let cross = request(
            &restored,
            Operation::WorkspaceSelect,
            &project,
            "answer",
            json!({}),
        );
        assert_eq!(
            mutate(&mut restored, &registry, &controller, &cross),
            Ok(
                json!({"workspace_revision":1,"active_project_id":project,"required_owner":restored.workspace_owner()})
            )
        );
        // Request identities are device-scoped: the same device cannot change operation.
        let cross = request(
            &restored,
            Operation::Prompt,
            &project,
            "answer",
            json!({"text":"hello"}),
        );
        assert_eq!(
            zenpi::remote::command::dispatch_prompt(
                &mut restored,
                &registry,
                &phone,
                &cross,
                false,
                |_, _, _| panic!()
            )
            .code,
            Some(ErrorCode::RequestIdConflict)
        );
        registry.revoke(phone.id()).unwrap();
        assert_eq!(
            dispatch(&mut restored, &registry, &phone, &command).code,
            Some(ErrorCode::Unauthorized)
        );
        drop(held);
    }
}

#[test]
fn remote_prompt_identity_fences_other_operations_restart_and_missing_journal() {
    use zenpi::remote::{
        command::{CancelAdmissions, dispatch_prompt},
        protocol::ExecutionState,
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control, Capability::Approval],
    );
    let original = request(
        &host,
        Operation::Prompt,
        &project,
        "shared-id",
        json!({"text":"hello"}),
    );
    let accepted = dispatch_prompt(&mut host, &registry, &phone, &original, false, |_, _, _| {
        Ok("job".into())
    });
    assert_eq!(accepted.execution_state, Some(ExecutionState::Accepted));
    let checkpoint = root.join("project-workspace.json");
    let saved: Value = serde_json::from_slice(&std::fs::read(&checkpoint).unwrap()).unwrap();
    assert_eq!(
        saved["remote_workspace"]["prompt_bindings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(!saved.to_string().contains("hello"));
    let owner = host.owner(&project).unwrap();
    let held = owner.lock().unwrap();
    for operation in [
        Operation::WorkspaceSelect,
        Operation::SubtabSelect,
        Operation::Cancel,
        Operation::Approve,
    ] {
        let arguments = match operation {
            Operation::Cancel => json!({"request_id":"original"}),
            Operation::Approve => {
                json!({"request_id":"approval","digest":"a".repeat(64),"decision":"allow"})
            }
            Operation::SubtabSelect => {
                json!({"subtab_id":host.subtab_state(&project).unwrap().active_id})
            }
            _ => json!({}),
        };
        let cross = request(&host, operation, &project, "shared-id", arguments);
        let code = match operation {
            Operation::WorkspaceSelect => mutate(&mut host, &registry, &phone, &cross).unwrap_err(),
            Operation::SubtabSelect => {
                zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &cross).unwrap_err()
            }
            Operation::Cancel => CancelAdmissions
                .dispatch(&mut host, &registry, &phone, &cross, None, || panic!())
                .code
                .unwrap(),
            _ => zenpi::remote::approval::dispatch(&mut host, &registry, &phone, &cross)
                .code
                .unwrap(),
        };
        assert_eq!(code, ErrorCode::RequestIdConflict);
    }
    drop(held);
    let mut restarted = pool(&root);
    assert_eq!(
        dispatch_prompt(
            &mut restarted,
            &registry,
            &phone,
            &original,
            false,
            |_, _, _| panic!()
        ),
        accepted
    );
    let mut changed = original.clone();
    changed.expected_revision = Some(999);
    assert_eq!(
        dispatch_prompt(
            &mut restarted,
            &registry,
            &phone,
            &changed,
            false,
            |_, _, _| panic!()
        )
        .code,
        Some(ErrorCode::RequestIdConflict)
    );
    // The journal is deliberately removed while the durable cross-operation reservation remains.
    let path = owner.lock().unwrap().session().path().to_owned();
    let journal = std::fs::read_to_string(&path).unwrap();
    let without_admission = journal
        .lines()
        .filter(|line| {
            let event: Value = serde_json::from_str(line).unwrap();
            event["event"]["type"] != "remote_prompt_admission_v1"
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(&path, without_admission).unwrap();
    let mut unresolved = pool(&root);
    assert_eq!(
        dispatch_prompt(
            &mut unresolved,
            &registry,
            &phone,
            &original,
            false,
            |_, _, _| panic!("reservation was resubmitted")
        )
        .execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    let cross = request(
        &unresolved,
        Operation::WorkspaceSelect,
        &project,
        "shared-id",
        json!({}),
    );
    assert_eq!(
        mutate(&mut unresolved, &registry, &phone, &cross),
        Err(ErrorCode::RequestIdConflict)
    );
    // A legacy checkpoint without bindings imports the journal before exposing mutation controls.
    std::fs::write(&path, journal).unwrap();
    let mut legacy = saved.clone();
    legacy["remote_workspace"]
        .as_object_mut()
        .unwrap()
        .remove("prompt_bindings");
    std::fs::write(&checkpoint, serde_json::to_vec(&legacy).unwrap()).unwrap();
    let mut migrated = pool(&root);
    assert_eq!(
        mutate(&mut migrated, &registry, &phone, &cross),
        Err(ErrorCode::RequestIdConflict)
    );
    assert_eq!(
        dispatch_prompt(
            &mut migrated,
            &registry,
            &phone,
            &original,
            false,
            |_, _, _| panic!()
        ),
        accepted
    );
    std::fs::remove_file(&checkpoint).unwrap();
    let mut missing_checkpoint = pool(&root);
    assert_eq!(
        mutate(&mut missing_checkpoint, &registry, &phone, &cross),
        Err(ErrorCode::RequestIdConflict)
    );
    assert_eq!(
        dispatch_prompt(
            &mut missing_checkpoint,
            &registry,
            &phone,
            &original,
            false,
            |_, _, _| panic!()
        ),
        accepted
    );
    for field in ["fingerprint", "session_id", "device_id"] {
        let mut corrupt: Value =
            serde_json::from_slice(&std::fs::read(&checkpoint).unwrap()).unwrap();
        corrupt["remote_workspace"]["prompt_bindings"][0][field] = json!("");
        std::fs::write(&checkpoint, serde_json::to_vec(&corrupt).unwrap()).unwrap();
        let agent = Agent::with_echo(
            SessionStore::open_in_workspace(root.join("initial.jsonl"), &root).unwrap(),
        );
        assert!(
            ProjectOwnerPool::new(Arc::new(Mutex::new(agent)))
                .unwrap()
                .restore_checkpoint()
                .is_err()
        );
        std::fs::write(&checkpoint, serde_json::to_vec(&saved).unwrap()).unwrap();
    }
}

#[test]
fn remote_prompt_identity_is_project_independent_and_checkpoint_failure_never_submits() {
    use zenpi::remote::{command::dispatch_prompt, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let initial = host.active_context().project_id.clone();
    std::fs::create_dir(root.join("child")).unwrap();
    host.open(Some(&root.join("child"))).unwrap();
    let child = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![initial.clone(), child.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let first = request(
        &host,
        Operation::Prompt,
        &initial,
        "same",
        json!({"text":"first"}),
    );
    assert_eq!(
        dispatch_prompt(&mut host, &registry, &phone, &first, false, |_, _, _| Ok(
            "job".into()
        ))
        .execution_state,
        Some(ExecutionState::Accepted)
    );
    let second = request(
        &host,
        Operation::Prompt,
        &child,
        "same",
        json!({"text":"second"}),
    );
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &phone,
            &second,
            false,
            |_, _, _| panic!()
        )
        .code,
        Some(ErrorCode::RequestIdConflict)
    );
    let owner = host.owner(&initial).unwrap();
    let guard = owner.lock().unwrap();
    let mut changed = first.clone();
    changed.expected_revision = Some(999);
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &phone,
            &changed,
            true,
            |_, _, _| panic!()
        )
        .code,
        Some(ErrorCode::RequestIdConflict)
    );
    drop(guard);
    // A competing writer defeats reservation CAS before any runner admission.
    let checkpoint = root.join("project-workspace.json");
    let saved = std::fs::read(&checkpoint).unwrap();
    let mut changed: Value = serde_json::from_slice(&saved).unwrap();
    changed["remote_workspace"]["revision"] = json!(999);
    std::fs::write(&checkpoint, serde_json::to_vec(&changed).unwrap()).unwrap();
    let fresh = request(
        &host,
        Operation::Prompt,
        &initial,
        "not-submitted",
        json!({"text":"private"}),
    );
    assert_eq!(
        dispatch_prompt(
            &mut host,
            &registry,
            &phone,
            &fresh,
            false,
            |_, _, _| panic!("reservation CAS failed but runner started")
        )
        .execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert!(
        owner
            .lock()
            .unwrap()
            .session()
            .events()
            .iter()
            .all(|event| event["request_id"] != "not-submitted")
    );
    std::fs::write(&checkpoint, saved).unwrap();
    // Bindings are device-scoped, not a global collision between independent phones.
    let other = device(
        &registry,
        vec![child.clone()],
        vec![Capability::Read, Capability::Control],
    );
    assert_eq!(
        dispatch_prompt(&mut host, &registry, &other, &second, false, |_, _, _| Ok(
            "other-job".into()
        ))
        .execution_state,
        Some(ExecutionState::Accepted)
    );
    let mut restored = pool(&root);
    assert_eq!(
        dispatch_prompt(
            &mut restored,
            &registry,
            &phone,
            &second,
            false,
            |_, _, _| panic!()
        )
        .code,
        Some(ErrorCode::RequestIdConflict)
    );
}

#[test]
fn remote_pending_approval_projection_is_bounded_non_consuming_and_secret_free() {
    use zenpi::approval::{ApprovalDecision, ApprovalPolicy, ApprovalRequest, ApprovalResponse};
    use zenpi::remote::host_adapter::HostAdapter;
    use zenpi::tools::{ToolContext, ToolOrigin, ToolSideEffect};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut agent = Agent::with_echo(
        SessionStore::open_in_workspace(root.join("initial.jsonl"), &root).unwrap(),
    );
    agent.set_attachment_workspace(ToolContext::new(&root).unwrap());
    let coordinator = agent.approval_coordinator().unwrap();
    let signal = zenpi::remote::wakeup::watch(agent.session().session_id()).unwrap();
    let observed = signal.revision();
    let owner = Arc::new(Mutex::new(agent));
    let mut host = ProjectOwnerPool::new(owner.clone()).unwrap();
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let foreign = device(&registry, vec![], vec![Capability::Read]);
    let worker = coordinator.clone();
    let join = std::thread::spawn(move || {
        worker.request(
            ApprovalRequest {
                request_id: "approval".into(),
                turn_id: "turn".into(),
                call_id: "call".into(),
                tool: "write_file".into(),
                side_effect: ToolSideEffect::WorkspaceWrite,
                arguments: json!({"path":"/private/secret-path","content":"secret-token"}),
                preview: None,
                origin: ToolOrigin::AgentTool,
                policy_digest: None,
                lease_id: None,
            },
            &mut ApprovalPolicy::default(),
            &|| false,
        )
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !coordinator.is_pending("approval") && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(coordinator.is_pending("approval"));
    assert!(signal.wait(observed, std::time::Duration::ZERO));
    let held = owner.lock().unwrap();
    let summaries = zenpi::remote::approval::listing(&host, &project).unwrap();
    assert_eq!(summaries[0]["tool"], "write_file");
    assert_eq!(summaries[0]["project_id"], project);
    assert_eq!(
        summaries[0]["session_id"],
        host.context(&project).unwrap().session_id
    );
    assert_eq!(summaries[0].as_object().unwrap().len(), 9);
    assert_eq!(summaries[0]["digest"].as_str().unwrap().len(), 64);
    assert!(!summaries.to_string().contains("secret"));
    assert_eq!(
        zenpi::remote::approval::listing(&host, &project).unwrap(),
        summaries
    );
    let read = request(
        &host,
        Operation::WorkspaceList,
        &project,
        "listing",
        json!({}),
    );
    let response = HostAdapter::dispatch_owned(&mut host, &registry, &phone, read.clone());
    let Message::Response { result } = response.payload else {
        panic!()
    };
    assert_eq!(result["projects"][0]["pending_approvals"], summaries);
    let foreign_result = HostAdapter::dispatch_owned(&mut host, &registry, &foreign, read.clone());
    let Message::Response { result } = foreign_result.payload else {
        panic!()
    };
    assert_eq!(result["projects"], json!([]));
    // Reading the summary must not consume desktop visibility or answer its prompt.
    assert_eq!(coordinator.drain_pending().len(), 1);
    assert!(coordinator.drain_pending().is_empty());
    assert_eq!(coordinator.pending_summaries(0).unwrap().len(), 0);
    let observed = signal.revision();
    coordinator
        .respond(ApprovalResponse {
            request_id: "approval".into(),
            decision: ApprovalDecision::Deny,
            remember: false,
            message: None,
        })
        .unwrap();
    assert!(signal.wait(observed, std::time::Duration::ZERO));
    assert_eq!(join.join().unwrap().unwrap(), ApprovalDecision::Deny);
    assert_eq!(
        zenpi::remote::approval::listing(&host, &project).unwrap(),
        json!([])
    );
    let mut workers = Vec::new();
    for index in 0..17 {
        let worker = coordinator.clone();
        workers.push(std::thread::spawn(move || {
            worker.request(
                ApprovalRequest {
                    request_id: format!("cap-{index:02}"),
                    turn_id: "turn".into(),
                    call_id: format!("call-{index}"),
                    tool: "write_file".into(),
                    side_effect: ToolSideEffect::WorkspaceWrite,
                    arguments: json!({"content":format!("private-{index}")}),
                    preview: None,
                    origin: ToolOrigin::AgentTool,
                    policy_digest: None,
                    lease_id: None,
                },
                &mut ApprovalPolicy::default(),
                &|| false,
            )
        }));
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !(0..17).all(|index| coordinator.is_pending(&format!("cap-{index:02}")))
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let capped = coordinator.pending_summaries(usize::MAX).unwrap();
    assert_eq!(capped.len(), 16);
    assert_eq!(
        capped
            .iter()
            .map(|summary| &summary.digest)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        16
    );
    assert_eq!(coordinator.drain_pending().len(), 17);
    for index in 0..17 {
        coordinator
            .respond(ApprovalResponse {
                request_id: format!("cap-{index:02}"),
                decision: ApprovalDecision::Deny,
                remember: false,
                message: None,
            })
            .unwrap();
    }
    for worker in workers {
        assert_eq!(worker.join().unwrap().unwrap(), ApprovalDecision::Deny);
    }
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &phone, read).code,
        Some(ErrorCode::Unauthorized)
    );
    drop(held);
}

#[test]
fn remote_prompt_progress_retains_recent_updates_and_active_work_past_display_capacity() {
    use zenpi::remote::{
        command::{PromptProgress, persist_progress},
        protocol::ExecutionState,
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let host = pool(&root);
    let project = host.active_context().project_id.clone();
    let live = PromptProgress::default();
    let mut input = request(
        &host,
        Operation::Prompt,
        &project,
        "active-oldest",
        json!({"text":"hello"}),
    );
    live.record(&input, "device", ExecutionState::Running);
    for index in 0..160 {
        input.id = Some(format!("request-{:03}", 160 - index));
        live.record(&input, "device", ExecutionState::Succeeded);
    }
    let owner = host.owner(&project).unwrap();
    let held = owner.lock().unwrap();
    let listing = live.listing(&host, "device", &project);
    assert_eq!(listing.as_array().unwrap().len(), 16);
    assert_eq!(listing[0]["request_id"], "active-oldest");
    assert_eq!(listing[0]["state"], "running");
    assert_eq!(listing[1]["request_id"], "request-001");
    assert_eq!(listing[15]["request_id"], "request-015");
    input.id = Some("active-oldest".into());
    live.record(&input, "device", ExecutionState::Cancelled);
    assert_eq!(
        live.listing(&host, "device", &project)[0]["state"],
        "cancelled"
    );
    input.id = Some("0-newest-running".into());
    live.record(&input, "device", ExecutionState::Running);
    let listing = live.listing(&host, "device", &project);
    assert_eq!(listing[0]["request_id"], "0-newest-running");
    assert_eq!(listing[0]["state"], "running");
    let mut old_session = input.clone();
    old_session.session_id = Some("old-session".into());
    live.record(&old_session, "device", ExecutionState::Failed);
    assert_eq!(
        live.listing(&host, "device", &project)[0]["state"],
        "running"
    );
    assert_eq!(live.listing(&host, "other-device", &project), json!([]));
    drop(held);
    let mut agent = owner.lock().unwrap();
    for index in 0..25 {
        input.id = Some(format!("persisted-{:03}", 25 - index));
        persist_progress(&mut agent, &input, "device", ExecutionState::Succeeded).unwrap();
    }
    input.id = Some("persisted-025".into());
    persist_progress(&mut agent, &input, "device", ExecutionState::Cancelled).unwrap();
    input.id = Some("unfinished".into());
    persist_progress(&mut agent, &input, "device", ExecutionState::Running).unwrap();
    drop(agent);
    let restored = PromptProgress::default().listing(&host, "device", &project);
    assert_eq!(restored.as_array().unwrap().len(), 16);
    assert_eq!(restored[0]["request_id"], "unfinished");
    assert_eq!(restored[0]["state"], "unknown_outcome");
    assert_eq!(restored[1]["request_id"], "persisted-025");
    assert_eq!(restored[1]["state"], "cancelled");
    assert_eq!(restored[2]["request_id"], "persisted-001");
    assert_eq!(restored[15]["request_id"], "persisted-014");
    let merged = live.listing(&host, "device", &project);
    assert_eq!(merged[0]["request_id"], "0-newest-running");
    assert_eq!(merged[0]["state"], "running");
}

#[test]
fn remote_cancel_targets_only_its_own_active_prompt_and_exact_retry_never_recancels() {
    use zenpi::remote::{command::CancelAdmissions, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let other = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let original = request(
        &host,
        Operation::Prompt,
        &project,
        "prompt-target",
        json!({"text":"hello"}),
    );
    let cancel = request(
        &host,
        Operation::Cancel,
        &project,
        "cancel-request",
        json!({"request_id":"prompt-target"}),
    );
    let mut receipts = CancelAdmissions;
    let owner = host.owner(&project).unwrap();
    let held = owner.lock().unwrap();
    assert_eq!(
        receipts
            .dispatch(
                &mut host,
                &registry,
                &other,
                &cancel,
                Some((&original, phone.id())),
                || panic!()
            )
            .code,
        Some(ErrorCode::OwnerMismatch)
    );
    let accepted = receipts.dispatch(
        &mut host,
        &registry,
        &phone,
        &cancel,
        Some((&original, phone.id())),
        || Ok(()),
    );
    assert_eq!(accepted.execution_state, Some(ExecutionState::Accepted));
    let mut restored = pool(&root);
    assert_ne!(restored.workspace_owner(), host.workspace_owner());
    assert_eq!(
        CancelAdmissions.dispatch(&mut restored, &registry, &phone, &cancel, None, || panic!(
            "restart repeated cancellation"
        )),
        accepted
    );
    assert_eq!(
        receipts.dispatch(&mut host, &registry, &phone, &cancel, None, || panic!(
            "retry cancelled new job"
        )),
        accepted
    );
    let mut conflict = cancel.clone();
    if let Message::Request { arguments, .. } = &mut conflict.payload {
        arguments["request_id"] = json!("different");
    }
    assert_eq!(
        receipts
            .dispatch(
                &mut host,
                &registry,
                &phone,
                &conflict,
                Some((&original, phone.id())),
                || panic!()
            )
            .code,
        Some(ErrorCode::RequestIdConflict)
    );
    let mut fresh = cancel.clone();
    fresh.id = Some("new-cancel".into());
    assert_eq!(
        receipts
            .dispatch(&mut host, &registry, &phone, &fresh, None, || panic!())
            .code,
        Some(ErrorCode::InvalidRequest)
    );
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        receipts
            .dispatch(&mut host, &registry, &phone, &cancel, None, || panic!())
            .code,
        Some(ErrorCode::Unauthorized)
    );
    drop(held);
}

#[test]
fn remote_cancel_reservations_and_failed_admission_survive_restart_without_reexecution() {
    use zenpi::remote::{command::CancelAdmissions, protocol::ExecutionState};
    for outcome in [Ok(()), Err(ErrorCode::Busy), Err(ErrorCode::InternalError)] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let mut host = pool(&root);
        let project = host.active_context().project_id.clone();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let phone = device(
            &registry,
            vec![project.clone()],
            vec![Capability::Read, Capability::Control],
        );
        let original = request(
            &host,
            Operation::Prompt,
            &project,
            "original",
            json!({"text":"hello"}),
        );
        let command = request(
            &host,
            Operation::Cancel,
            &project,
            "cancel",
            json!({"request_id":"original"}),
        );
        let expected = match outcome {
            Ok(()) => ExecutionState::UnknownOutcome,
            Err(ErrorCode::Busy) => ExecutionState::Failed,
            _ => ExecutionState::UnknownOutcome,
        };
        let checkpoint = root.join("project-workspace.json");
        let result = CancelAdmissions.dispatch(
            &mut host,
            &registry,
            &phone,
            &command,
            Some((&original, phone.id())),
            || {
                let mut saved: Value =
                    serde_json::from_slice(&std::fs::read(&checkpoint).unwrap()).unwrap();
                assert_eq!(
                    saved["remote_workspace"]["cancel_receipts"][0]["state"],
                    "unknown_outcome"
                );
                if outcome.is_ok() {
                    // A competing checkpoint commit after the side effect prevents a falsely durable acceptance.
                    saved["remote_workspace"]["revision"] = json!(1);
                    std::fs::write(&checkpoint, serde_json::to_vec(&saved).unwrap()).unwrap();
                }
                outcome
            },
        );
        assert_eq!(result.execution_state, Some(expected));
        let mut restored = pool(&root);
        assert_eq!(
            CancelAdmissions.dispatch(&mut restored, &registry, &phone, &command, None, || panic!(
                "replayed unresolved cancel"
            )),
            result
        );
        let mut changed = command.clone();
        changed.expected_revision = Some(999);
        assert_eq!(
            CancelAdmissions
                .dispatch(
                    &mut restored,
                    &registry,
                    &phone,
                    &changed,
                    None,
                    || panic!()
                )
                .code,
            Some(ErrorCode::RequestIdConflict)
        );
        let mut cross_operation = command.clone();
        cross_operation.payload = Message::Request {
            operation: Operation::WorkspaceSelect,
            arguments: json!({}),
        };
        assert_eq!(
            mutate(&mut restored, &registry, &phone, &cross_operation),
            Err(ErrorCode::RequestIdConflict)
        );
        cross_operation.payload = Message::Request {
            operation: Operation::Prompt,
            arguments: json!({"text":"hello"}),
        };
        assert_eq!(
            zenpi::remote::command::dispatch_prompt(
                &mut restored,
                &registry,
                &phone,
                &cross_operation,
                false,
                |_, _, _| panic!()
            )
            .code,
            Some(ErrorCode::RequestIdConflict)
        );
    }
}

#[test]
fn remote_cancel_checkpoint_conflict_refuses_side_effect_and_legacy_receipts_are_strict() {
    use zenpi::remote::{command::CancelAdmissions, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let original = request(
        &host,
        Operation::Prompt,
        &project,
        "original",
        json!({"text":"hello"}),
    );
    let command = request(
        &host,
        Operation::Cancel,
        &project,
        "cancel",
        json!({"request_id":"original"}),
    );
    let mut competitor = pool(&root);
    competitor.select(&project).unwrap();
    let result = CancelAdmissions.dispatch(
        &mut host,
        &registry,
        &phone,
        &command,
        Some((&original, phone.id())),
        || panic!("failed reservation cancelled work"),
    );
    assert_eq!(result.execution_state, Some(ExecutionState::UnknownOutcome));
    let checkpoint = root.join("project-workspace.json");
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&checkpoint).unwrap()).unwrap();
    saved["remote_workspace"]
        .as_object_mut()
        .unwrap()
        .remove("cancel_receipts");
    std::fs::write(&checkpoint, serde_json::to_vec(&saved).unwrap()).unwrap();
    let mut legacy = pool(&root);
    legacy.select(&project).unwrap();
    let command = request(
        &legacy,
        Operation::Cancel,
        &project,
        "cancel",
        json!({"request_id":"original"}),
    );
    assert_eq!(
        CancelAdmissions
            .dispatch(
                &mut legacy,
                &registry,
                &phone,
                &command,
                Some((&original, phone.id())),
                || Ok(())
            )
            .execution_state,
        Some(ExecutionState::Accepted)
    );
    let saved: Value = serde_json::from_slice(&std::fs::read(&checkpoint).unwrap()).unwrap();
    for change in 0..5 {
        let mut invalid = saved.clone();
        match change {
            0 => invalid["remote_workspace"]["cancel_receipts"][0]["state"] = json!("succeeded"),
            1 => invalid["remote_workspace"]["cancel_receipts"][0]["target_request_id"] = json!(""),
            2 => invalid["remote_workspace"]["cancel_receipts"][0]["code"] = json!("busy"),
            3 => {
                invalid["remote_workspace"]["cancel_receipts"] = json!([
                    saved["remote_workspace"]["cancel_receipts"][0],
                    saved["remote_workspace"]["cancel_receipts"][0]
                ])
            }
            _ => {
                invalid["remote_workspace"]["cancel_receipts"] = json!(vec![
                        saved["remote_workspace"]["cancel_receipts"][0].clone();
                        129
                    ])
            }
        }
        std::fs::write(&checkpoint, serde_json::to_vec(&invalid).unwrap()).unwrap();
        let owner = Agent::with_echo(
            SessionStore::open_in_workspace(root.join("initial.jsonl"), &root).unwrap(),
        );
        let mut failed = ProjectOwnerPool::new(Arc::new(Mutex::new(owner))).unwrap();
        assert!(
            failed
                .restore_checkpoint()
                .unwrap_err()
                .contains("cancellation receipts")
        );
    }
}

#[test]
fn remote_git_worktree_create_close_dirty_refusal_and_restart_deduplication() {
    use zenpi::remote::{host_adapter::HostAdapter, protocol::ExecutionState, subtab};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "fixture"]);
    git(&["config", "user.email", "fixture@example.invalid"]);
    std::fs::write(root.join("tracked.txt"), "original").unwrap();
    git(&["add", "tracked.txt"]);
    git(&["commit", "-qm", "initial"]);
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let readonly = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let state = host.subtab_state(&project).unwrap();
    let add = request(
        &host,
        Operation::SubtabAdd,
        &project,
        "create-worktree",
        json!({"kind":"worktree","name":"feature","subtab_revision":state.revision}),
    );
    assert_eq!(
        subtab::mutate(&mut host, &registry, &readonly, &add),
        Err(ErrorCode::Unauthorized)
    );
    assert!(!root.join(".zenpi-worktrees").exists());
    let owner = host.owner(&project).unwrap();
    let busy = owner.lock().unwrap();
    assert_eq!(
        subtab::mutate(&mut host, &registry, &phone, &add),
        Err(ErrorCode::Busy)
    );
    assert!(!root.join(".zenpi-worktrees").exists());
    drop(busy);
    for name in ["../outside", "-option", "bad..name", "bad name"] {
        let mut invalid = add.clone();
        invalid.id = Some("invalid-create".into());
        if let Message::Request { arguments, .. } = &mut invalid.payload {
            arguments["name"] = json!(name);
        }
        assert_eq!(
            subtab::mutate(&mut host, &registry, &phone, &invalid),
            Err(ErrorCode::InvalidRequest)
        );
    }
    let result = subtab::mutate(&mut host, &registry, &phone, &add).unwrap();
    assert!(!result.is_null());
    let path = root.join(".zenpi-worktrees/feature");
    assert_eq!(
        std::fs::read_to_string(path.join("tracked.txt")).unwrap(),
        "original"
    );
    let created = host.subtab_state(&project).unwrap();
    assert_eq!(created.tabs.len(), 2);
    assert_eq!(created.tabs[1].root, path);
    assert_eq!(
        subtab::mutate(&mut host, &registry, &phone, &add).unwrap(),
        result
    );
    drop(host);
    let mut host = pool(&root);
    assert_eq!(
        subtab::mutate(&mut host, &registry, &phone, &add).unwrap(),
        result
    );
    let close = request(
        &host,
        Operation::SubtabClose,
        &project,
        "close-worktree",
        json!({"subtab_id":created.active_id,"subtab_revision":created.revision}),
    );
    std::fs::write(path.join("untracked.txt"), "must survive").unwrap();
    let before = std::fs::read(root.join("project-workspace.json")).unwrap();
    assert_eq!(
        subtab::mutate(&mut host, &registry, &phone, &close),
        Err(ErrorCode::InvalidRequest)
    );
    assert_eq!(host.subtab_state(&project).unwrap(), created);
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        before
    );
    std::fs::remove_file(path.join("untracked.txt")).unwrap();
    let removed = subtab::mutate(&mut host, &registry, &phone, &close).unwrap();
    assert!(!path.exists());
    assert_eq!(host.subtab_state(&project).unwrap().tabs.len(), 1);
    assert_eq!(
        subtab::mutate(&mut host, &registry, &phone, &close).unwrap(),
        removed
    );
    git(&["show-ref", "--verify", "refs/heads/feature"]);
    // Existing branch makes git add fail after reservation: never retry that identity.
    let state = host.subtab_state(&project).unwrap();
    let uncertain = request(
        &host,
        Operation::SubtabAdd,
        &project,
        "ambiguous-git",
        json!({"kind":"worktree","name":"feature","subtab_revision":state.revision}),
    );
    let response = HostAdapter::dispatch_owned(&mut host, &registry, &phone, uncertain.clone());
    assert_eq!(
        response.execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert_eq!(host.subtab_state(&project).unwrap(), state);
    drop(host);
    let mut host = pool(&root);
    git(&["branch", "-D", "feature"]);
    let response = HostAdapter::dispatch_owned(&mut host, &registry, &phone, uncertain.clone());
    assert_eq!(
        response.execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert!(!path.exists());
    let mut changed = uncertain;
    if let Message::Request { arguments, .. } = &mut changed.payload {
        arguments["name"] = json!("other");
    }
    assert_eq!(
        subtab::mutate(&mut host, &registry, &phone, &changed),
        Err(ErrorCode::RequestIdConflict)
    );
    for operation in [
        Operation::WorkspaceSelect,
        Operation::Prompt,
        Operation::Cancel,
        Operation::Steer,
    ] {
        let cross = request(
            &host,
            operation,
            &project,
            "ambiguous-git",
            match operation {
                Operation::Prompt => json!({"text":"must not start"}),
                Operation::Cancel => json!({"request_id":"target"}),
                Operation::Steer => json!({"request_id":"target","text":"must not enqueue"}),
                _ => json!({}),
            },
        );
        let response = if operation == Operation::Prompt {
            let mut starts = 0;
            let response = zenpi::remote::command::dispatch_prompt(
                &mut host,
                &registry,
                &phone,
                &cross,
                true,
                |_, _, _| {
                    starts += 1;
                    Ok("must not start".into())
                },
            );
            assert_eq!(starts, 0);
            response
        } else if operation == Operation::Cancel {
            zenpi::remote::command::CancelAdmissions.dispatch(
                &mut host,
                &registry,
                &phone,
                &cross,
                None,
                || panic!("must not cancel"),
            )
        } else if operation == Operation::Steer {
            zenpi::remote::command::SteerAdmissions::default()
                .dispatch(&mut host, &registry, &phone, &cross, None)
        } else {
            HostAdapter::dispatch_owned(&mut host, &registry, &phone, cross)
        };
        assert_eq!(
            response.code,
            Some(ErrorCode::RequestIdConflict),
            "{operation:?}"
        );
    }
}

#[test]
#[cfg(unix)]
fn post_git_checkpoint_conflict_preserves_unknown_intent_without_reexecution() {
    use std::os::unix::fs::PermissionsExt;
    use zenpi::remote::{host_adapter::HostAdapter, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "fixture"]);
    git(&["config", "user.email", "fixture@example.invalid"]);
    std::fs::write(root.join("keep.txt"), "original").unwrap();
    git(&["add", "keep.txt"]);
    git(&["commit", "-qm", "initial"]);
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let state = host.subtab_state(&project).unwrap();
    // The real Git hook changes only formatting of the already reserved
    // checkpoint, simulating a competing writer after the filesystem action.
    let hook = root.join(".git/hooks/post-checkout");
    let checkpoint = root.join("project-workspace.json");
    let quoted = checkpoint.to_str().unwrap().replace('\'', "'\"'\"'");
    std::fs::write(&hook, format!("#!/bin/sh\nprintf '\\n' >> '{quoted}'\n")).unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o700)).unwrap();
    let add = request(
        &host,
        Operation::SubtabAdd,
        &project,
        "post-git-conflict",
        json!({"kind":"worktree","name":"partial","subtab_revision":state.revision}),
    );
    let response = HostAdapter::dispatch_owned(&mut host, &registry, &phone, add.clone());
    assert_eq!(
        response.execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    let path = root.join(".zenpi-worktrees/partial");
    assert!(path.join("keep.txt").exists());
    assert_eq!(host.subtab_state(&project).unwrap(), state);
    std::fs::remove_file(hook).unwrap();
    drop(host);
    let mut host = pool(&root);
    // Delete the worktree/branch externally to prove retry is not running add.
    zenpi::project_workspace::remove_worktree(&root, &path).unwrap();
    git(&["branch", "-D", "partial"]);
    let response = HostAdapter::dispatch_owned(&mut host, &registry, &phone, add);
    assert_eq!(
        response.execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert!(!path.exists());
    assert_eq!(host.subtab_state(&project).unwrap(), state);
}

#[test]
#[cfg(unix)]
fn operator_reconciles_observed_create_and_remove_without_git_replay() {
    use std::os::unix::fs::PermissionsExt;
    use zenpi::remote::{host_adapter::HostAdapter, protocol::ExecutionState, subtab};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "fixture"]);
    git(&["config", "user.email", "fixture@example.invalid"]);
    std::fs::write(root.join("keep.txt"), "original").unwrap();
    git(&["add", "keep.txt"]);
    git(&["commit", "-qm", "initial"]);
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let state = host.subtab_state(&project).unwrap();
    let hook = root.join(".git/hooks/post-checkout");
    let checkpoint = root.join("project-workspace.json");
    let quoted = checkpoint.to_str().unwrap().replace('\'', "'\"'\"'");
    std::fs::write(&hook, format!("#!/bin/sh\nprintf '\\n' >> '{quoted}'\n")).unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o700)).unwrap();
    let add = request(
        &host,
        Operation::SubtabAdd,
        &project,
        "repair-create",
        json!({"kind":"worktree","name":"repair","subtab_revision":state.revision}),
    );
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &phone, add.clone()).execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    std::fs::remove_file(hook).unwrap();
    drop(host);
    let config = root.join("cli-config");
    std::fs::create_dir(&config).unwrap();
    std::fs::write(config.join("config.toml"), "backend=\"openai\"\nmodel=\"gpt-4.1\"\nbase_url=\"http://127.0.0.1:9/v1\"\nwire_api=\"responses\"\nrequires_openai_auth=false\n").unwrap();
    let cli = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_zenpi"))
            .args([
                "host",
                "pair",
                "--workspace",
                root.to_str().unwrap(),
                "--session",
                root.join("initial.jsonl").to_str().unwrap(),
            ])
            .args(args)
            .env("ZENPI_HOME", &config)
            .output()
            .unwrap()
    };
    let listing = cli(&["--list-worktree-intents"]);
    assert!(
        listing.status.success(),
        "{}",
        String::from_utf8_lossy(&listing.stderr)
    );
    let listed: Value = serde_json::from_slice(&listing.stdout).unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["request_id"], "repair-create");
    assert!(!config.join("remote/identity.json").exists());
    let missing_session = root.join("missing.jsonl");
    let refused = std::process::Command::new(env!("CARGO_BIN_EXE_zenpi"))
        .args([
            "host",
            "pair",
            "--workspace",
            root.to_str().unwrap(),
            "--session",
            missing_session.to_str().unwrap(),
            "--list-worktree-intents",
        ])
        .env("ZENPI_HOME", &config)
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(!missing_session.exists());
    let mut host = pool(&root);
    let path = root.join(".zenpi-worktrees/repair");
    git(&["branch", "-m", "repair", "different"]);
    assert!(
        host.reconcile_worktree_intent(phone.id(), "repair-create")
            .is_err()
    );
    git(&["branch", "-m", "different", "repair"]);
    std::fs::write(path.join("keep.txt"), "keep user changes").unwrap();
    assert_eq!(host.unresolved_worktree_intents().len(), 1);
    assert_eq!(
        host.unresolved_worktree_intents()[0]["intent"]["path"],
        json!(path)
    );
    let owner = host.owner(&project).unwrap();
    let busy = owner.lock().unwrap();
    assert!(
        host.reconcile_worktree_intent(phone.id(), "repair-create")
            .is_err()
    );
    drop(busy);
    drop(host);
    let response = cli(&[
        "--reconcile-worktree",
        "repair-create",
        "--device",
        phone.id(),
    ]);
    assert!(
        response.status.success(),
        "{}",
        String::from_utf8_lossy(&response.stderr)
    );
    let repaired: Value = serde_json::from_slice(&response.stdout).unwrap();
    let mut host = pool(&root);
    assert_eq!(repaired["required_owner"], json!(add.required_owner));
    assert_eq!(
        repaired["workspace_revision"],
        add.expected_revision.unwrap() + 1
    );
    assert_eq!(
        std::fs::read_to_string(path.join("keep.txt")).unwrap(),
        "keep user changes"
    );
    assert!(host.unresolved_worktree_intents().is_empty());
    assert_eq!(
        subtab::mutate(&mut host, &registry, &phone, &add).unwrap(),
        repaired
    );
    assert_eq!(
        host.reconcile_worktree_intent(phone.id(), "repair-create")
            .unwrap(),
        repaired
    );
    let before_remove = host.subtab_state(&project).unwrap();
    let close = request(
        &host,
        Operation::SubtabClose,
        &project,
        "repair-remove",
        json!({"subtab_id":before_remove.active_id,"subtab_revision":before_remove.revision}),
    );
    let before_checkpoint: Value =
        serde_json::from_slice(&std::fs::read(&checkpoint).unwrap()).unwrap();
    std::fs::write(path.join("keep.txt"), "original").unwrap();
    let removed = subtab::mutate(&mut host, &registry, &phone, &close).unwrap();
    let after_remove = host.subtab_state(&project).unwrap();
    // Reconstruct the exact pre-publication crash window: Git removed the
    // directory, but only its pre-Git intent and prior tab state are durable.
    let mut crash = before_checkpoint;
    crash["remote_workspace"]["receipts"].as_array_mut().unwrap().push(json!({"device_id":phone.id(),"request_id":"repair-remove",
        "fingerprint":format!("{:x}",sha2::Sha256::digest(serde_json::to_vec(&close).unwrap())),"result":null,
        "worktree_intent":{"request":close,"before":before_remove,"candidate":after_remove,"path":path,"branch":null}}));
    drop(host);
    std::fs::write(&checkpoint, serde_json::to_vec(&crash).unwrap()).unwrap();
    let mut host = pool(&root);
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("foreign.txt"), "do not delete").unwrap();
    assert!(
        host.reconcile_worktree_intent(phone.id(), "repair-remove")
            .is_err()
    );
    assert!(path.join("foreign.txt").exists());
    std::fs::remove_file(path.join("foreign.txt")).unwrap();
    std::fs::remove_dir(&path).unwrap();
    let parent = root.join(".zenpi-worktrees");
    let preserved_parent = root.join("managed-backup");
    std::fs::rename(&parent, &preserved_parent).unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), &parent).unwrap();
    assert!(
        host.reconcile_worktree_intent(phone.id(), "repair-remove")
            .is_err()
    );
    std::fs::remove_file(&parent).unwrap();
    std::fs::rename(preserved_parent, &parent).unwrap();
    let result = host
        .reconcile_worktree_intent(phone.id(), "repair-remove")
        .unwrap();
    assert_eq!(result, removed);
    assert!(!path.exists());
    assert_eq!(host.subtab_state(&project).unwrap().tabs.len(), 1);
    git(&["show-ref", "--verify", "refs/heads/repair"]);
    assert_eq!(
        subtab::mutate(&mut host, &registry, &phone, &close).unwrap(),
        result
    );
    drop(host);
    std::fs::write(&checkpoint, serde_json::to_vec(&crash).unwrap()).unwrap();
    let mut host = pool(&root);
    let stale = host.subtab_state(&project).unwrap();
    host.commit_subtabs(
        &project,
        stale.revision,
        stale.rename(&stale.tabs[0].id, "newer-main").unwrap(),
    )
    .unwrap();
    let checkpoint_before = std::fs::read(&checkpoint).unwrap();
    assert!(
        host.reconcile_worktree_intent(phone.id(), "repair-remove")
            .is_err()
    );
    assert_eq!(std::fs::read(&checkpoint).unwrap(), checkpoint_before);
    // Corrupted intent targets cannot be adopted on restart.
    let mut corrupt = crash;
    corrupt["remote_workspace"]["receipts"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["worktree_intent"]["path"] = json!(root.join("outside"));
    drop(host);
    std::fs::write(&checkpoint, serde_json::to_vec(&corrupt).unwrap()).unwrap();
    let a = Agent::with_echo(
        SessionStore::open_in_workspace(root.join("initial.jsonl"), &root).unwrap(),
    );
    let mut host = ProjectOwnerPool::new(Arc::new(Mutex::new(a))).unwrap();
    assert!(host.restore_checkpoint().is_err());
}

#[test]
fn remote_directory_tabs_are_contained_non_destructive_and_exact_retry_safe() {
    use zenpi::remote::subtab::mutate;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let inside = root.join("inside");
    std::fs::create_dir(&inside).unwrap();
    std::fs::write(inside.join("keep.txt"), b"keep").unwrap();
    let outside = tempfile::tempdir().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let signal = zenpi::remote::wakeup::watch(&host.active_context().session_id).unwrap();
    let observed = signal.revision();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let state = host.subtab_state(&project).unwrap();
    let add = request(
        &host,
        Operation::SubtabAdd,
        &project,
        "add-directory",
        json!({"subtab_revision":state.revision,"kind":"in_place","name":"directory","cwd":"inside"}),
    );
    for cwd in ["..", ".", "missing", outside.path().to_str().unwrap()] {
        let mut invalid = add.clone();
        invalid.id = Some("invalid-directory".into());
        if let Message::Request { arguments, .. } = &mut invalid.payload {
            arguments["cwd"] = json!(cwd);
        }
        assert_eq!(
            mutate(&mut host, &registry, &phone, &invalid),
            Err(ErrorCode::InvalidRequest)
        );
        assert_eq!(host.subtab_state(&project).unwrap(), state);
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.path(), root.join("escape")).unwrap();
        let mut invalid = add.clone();
        invalid.id = Some("symlink-escape".into());
        if let Message::Request { arguments, .. } = &mut invalid.payload {
            arguments["cwd"] = json!("escape");
        }
        assert_eq!(
            mutate(&mut host, &registry, &phone, &invalid),
            Err(ErrorCode::InvalidRequest)
        );
    }
    assert_eq!(signal.revision(), observed);
    let added = mutate(&mut host, &registry, &phone, &add).unwrap();
    assert_ne!(signal.revision(), observed);
    let observed = signal.revision();
    assert_eq!(mutate(&mut host, &registry, &phone, &add).unwrap(), added);
    assert_eq!(signal.revision(), observed);
    let state = host.subtab_state(&project).unwrap();
    assert_eq!(state.tabs.len(), 2);
    assert_eq!(state.tabs[1].root, inside);
    let directory_id = state.active_id.clone();
    let duplicate = request(
        &host,
        Operation::SubtabAdd,
        &project,
        "duplicate",
        json!({"subtab_revision":state.revision,"kind":"in_place","name":"other","cwd":"inside"}),
    );
    assert_eq!(
        mutate(&mut host, &registry, &phone, &duplicate),
        Err(ErrorCode::InvalidRequest)
    );
    let main_close = request(
        &host,
        Operation::SubtabClose,
        &project,
        "main-close",
        json!({"subtab_revision":state.revision,"subtab_id":state.tabs[0].id}),
    );
    assert_eq!(
        mutate(&mut host, &registry, &phone, &main_close),
        Err(ErrorCode::InvalidRequest)
    );
    assert_eq!(signal.revision(), observed);
    let close = request(
        &host,
        Operation::SubtabClose,
        &project,
        "close-directory",
        json!({"subtab_revision":state.revision,"subtab_id":directory_id}),
    );
    let closed = mutate(&mut host, &registry, &phone, &close).unwrap();
    assert_ne!(signal.revision(), observed);
    assert_ne!(closed["active_subtab_id"], directory_id);
    assert_eq!(
        closed["active_subtab_id"],
        host.subtab_state(&project).unwrap().active_id
    );
    assert_eq!(host.subtab_state(&project).unwrap().tabs.len(), 1);
    assert_eq!(std::fs::read(inside.join("keep.txt")).unwrap(), b"keep");
    drop(host);
    let mut host = pool(&root);
    let observed = signal.revision();
    assert_eq!(
        mutate(&mut host, &registry, &phone, &close).unwrap(),
        closed
    );
    assert_eq!(mutate(&mut host, &registry, &phone, &add).unwrap(), added);
    assert_eq!(signal.revision(), observed);
    assert_eq!(host.subtab_state(&project).unwrap().tabs.len(), 1);
    assert_eq!(std::fs::read(inside.join("keep.txt")).unwrap(), b"keep");
}

#[test]
fn subtab_remote_control_is_acl_revision_fenced_durable_and_reconciles_desktop() {
    use zenpi::remote::{
        host_adapter::HostAdapter,
        subtab::{LegacySubtab, SubtabKind},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    host.migrate_legacy_subtabs(
        &project,
        &[
            LegacySubtab {
                name: "main".into(),
                root: root.display().to_string(),
                kind: SubtabKind::Main,
                concurrency: 1,
            },
            LegacySubtab {
                name: "one".into(),
                root: root.join("one").display().to_string(),
                kind: SubtabKind::InPlace,
                concurrency: 1,
            },
            LegacySubtab {
                name: "two".into(),
                root: root.join("two").display().to_string(),
                kind: SubtabKind::InPlace,
                concurrency: 1,
            },
        ],
        0,
    )
    .unwrap();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let read_only = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let foreign = device(
        &registry,
        vec![],
        vec![Capability::Read, Capability::Control],
    );
    let state = host.subtab_state(&project).unwrap();
    let target = state.tabs[2].id.clone();
    let rename = request(
        &host,
        Operation::SubtabRename,
        &project,
        "subtab-rename",
        json!({"subtab_id":target,"subtab_revision":state.revision,"name":"renamed"}),
    );
    assert_eq!(
        zenpi::remote::subtab::mutate(&mut host, &registry, &read_only, &rename),
        Err(ErrorCode::Unauthorized)
    );
    assert_eq!(
        zenpi::remote::subtab::mutate(&mut host, &registry, &foreign, &rename),
        Err(ErrorCode::Unauthorized)
    );
    let result = zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &rename).unwrap();
    assert_eq!(host.subtab_state(&project).unwrap().tabs[2].name, "renamed");
    let revision = host.workspace_revision();
    assert_eq!(
        zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &rename).unwrap(),
        result
    );
    assert_eq!(host.workspace_revision(), revision);
    let mut conflict = rename.clone();
    if let Message::Request { arguments, .. } = &mut conflict.payload {
        arguments["name"] = json!("different");
    }
    assert_eq!(
        zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &conflict),
        Err(ErrorCode::RequestIdConflict)
    );
    let state = host.subtab_state(&project).unwrap();
    let stale = request(
        &host,
        Operation::SubtabSelect,
        &project,
        "stale",
        json!({"subtab_id":target,"subtab_revision":state.revision-1}),
    );
    assert_eq!(
        zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &stale),
        Err(ErrorCode::RevisionConflict)
    );
    let select = request(
        &host,
        Operation::SubtabSelect,
        &project,
        "select",
        json!({"subtab_id":target,"subtab_revision":state.revision}),
    );
    for fault in 0..4 {
        let mut invalid = select.clone();
        invalid.id = Some(format!("invalid-{fault}"));
        match fault {
            0 => invalid.required_owner = Some("f".repeat(64)),
            1 => invalid.session_id = Some("foreign-session".into()),
            2 => {
                if let Message::Request { arguments, .. } = &mut invalid.payload {
                    arguments["index"] = json!(1);
                }
            }
            _ => {
                if let Message::Request { arguments, .. } = &mut invalid.payload {
                    arguments["subtab_revision"] = json!("2");
                }
            }
        }
        let expected = if fault < 2 {
            ErrorCode::OwnerMismatch
        } else {
            ErrorCode::InvalidRequest
        };
        assert_eq!(
            zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &invalid),
            Err(expected)
        );
        assert_eq!(host.subtab_state(&project).unwrap(), state);
    }
    let arch = host.arch_agent(&project).unwrap();
    let arch_guard = arch.lock().unwrap();
    assert_eq!(
        zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &select),
        Err(ErrorCode::Busy)
    );
    drop(arch_guard);
    let owner = host.active();
    let held = owner.lock().unwrap();
    assert_eq!(
        zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &select),
        Err(ErrorCode::Busy)
    );
    drop(held);
    let selected_receipt = HostAdapter::dispatch_owned(&mut host, &registry, &phone, select);
    assert_eq!(
        selected_receipt.execution_state,
        Some(zenpi::remote::protocol::ExecutionState::Succeeded)
    );
    assert!(
        matches!(&selected_receipt.payload, Message::Response { result } if result["active_subtab_id"] == target)
    );
    assert_eq!(host.subtab_state(&project).unwrap().active_id, target);
    let state = host.subtab_state(&project).unwrap();
    let move_request = request(
        &host,
        Operation::SubtabMove,
        &project,
        "move",
        json!({"subtab_id":target,"subtab_revision":state.revision,"before_id":state.tabs[1].id}),
    );
    zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &move_request).unwrap();
    let committed = host.subtab_state(&project).unwrap();
    assert_eq!(committed.tabs[1].id, target);
    drop(host);
    let mut host = pool(&root);
    assert_eq!(host.subtab_state(&project).unwrap(), committed);
    assert_eq!(
        zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &rename).unwrap(),
        result
    );
    let list = Envelope {
        project_id: Some(project.clone()),
        id: Some("list".into()),
        ..Envelope::new(
            Channel::Control,
            Message::Request {
                operation: Operation::SubtabList,
                arguments: json!({}),
            },
        )
    };
    let visible = zenpi::remote::subtab::listing(&host, &registry, &read_only, &list).unwrap();
    assert_eq!(visible["active_subtab_id"], target);
    assert_eq!(visible["subtab_control"], false);
    assert!(!visible.to_string().contains(&root.display().to_string()));
    assert_eq!(
        zenpi::remote::subtab::listing(&host, &registry, &foreign, &list),
        Err(ErrorCode::Unauthorized)
    );
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &rename),
        Err(ErrorCode::Unauthorized)
    );
}

#[test]
fn remote_subtab_write_refuses_busy_worker_before_checkpoint_and_retries_exactly() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let initial = host.subtab_state(&project).unwrap();
    host.commit_subtabs(
        &project,
        initial.revision,
        initial.set_concurrency(&initial.active_id, 2).unwrap(),
    )
    .unwrap();
    let state = host.subtab_state(&project).unwrap();
    let worker = host.worker_owner(&project, &state.active_id, 0).unwrap();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let rename = request(
        &host,
        Operation::SubtabRename,
        &project,
        "worker-fenced-rename",
        json!({"subtab_id":state.active_id,"subtab_revision":state.revision,"name":"renamed-main"}),
    );
    let path = root.join("project-workspace.json");
    let before = std::fs::read(&path).unwrap();
    let guard = worker.agent().lock().unwrap();
    assert_eq!(
        zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &rename),
        Err(ErrorCode::Busy)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(host.subtab_state(&project).unwrap(), state);
    drop(guard);
    let result = zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &rename).unwrap();
    let after = std::fs::read(&path).unwrap();
    let guard = worker.agent().lock().unwrap();
    // A durable exact receipt is still queryable while execution is busy.
    assert_eq!(
        zenpi::remote::subtab::mutate(&mut host, &registry, &phone, &rename).unwrap(),
        result
    );
    assert_eq!(std::fs::read(&path).unwrap(), after);
    assert_eq!(guard.phase(), zenpi::core::AgentPhase::Idle);
}

#[test]
fn worker_admission_is_durable_device_scoped_and_exact_retries_never_run_again() {
    use zenpi::{
        remote::{
            protocol::ExecutionState, worker_admission::dispatch, worktree_runner::WorktreeRunner,
        },
        runtime::{JobOutcome, RuntimeConfig, RuntimeEvent},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let state = host.subtab_state(&project).unwrap();
    let tab = state.active_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let runner =
        WorktreeRunner::spawn_prompts(&mut host, &project, &tab, RuntimeConfig::default()).unwrap();
    let req = request(
        &host,
        Operation::Prompt,
        &project,
        "durable-worker",
        json!({"text":"run-worker-once",
        "subtab_id":tab,"subtab_revision":state.revision}),
    );
    assert_eq!(
        dispatch(&mut host, &registry, &reader, &req, &runner).code,
        Some(ErrorCode::Unauthorized)
    );
    let accepted = dispatch(&mut host, &registry, &phone, &req, &runner);
    assert_eq!(accepted.execution_state, Some(ExecutionState::Accepted));
    assert_eq!(
        dispatch(&mut host, &registry, &phone, &req, &runner),
        accepted
    );
    loop {
        if let RuntimeEvent::Completed { outcome, .. } = runner
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap()
        {
            assert!(matches!(outcome, JobOutcome::Succeeded(_)), "{outcome:?}");
            break;
        }
    }
    let worker = &runner.workers()[0];
    let worker_path = worker.agent().lock().unwrap().session().path().to_owned();
    let before = std::fs::read(&worker_path).unwrap();
    assert_eq!(
        dispatch(&mut host, &registry, &phone, &req, &runner),
        accepted
    );
    assert_eq!(std::fs::read(&worker_path).unwrap(), before);
    let mut changed = req.clone();
    if let Message::Request { arguments, .. } = &mut changed.payload {
        arguments["text"] = json!("changed-body");
    }
    assert_eq!(
        dispatch(&mut host, &registry, &phone, &changed, &runner).code,
        Some(ErrorCode::RequestIdConflict)
    );
    let normal = request(
        &host,
        Operation::Prompt,
        &project,
        "durable-worker",
        json!({"text":"master-collision"}),
    );
    let collision = zenpi::remote::command::dispatch_prompt(
        &mut host,
        &registry,
        &phone,
        &normal,
        false,
        |_, _, _| panic!("cross-owner request ID collision must never submit"),
    );
    assert_eq!(collision.code, Some(ErrorCode::RequestIdConflict));
    runner.shutdown_and_join().unwrap();
    drop(host);
    let mut host = pool(&root);
    let runner =
        WorktreeRunner::spawn_prompts(&mut host, &project, &tab, RuntimeConfig::default()).unwrap();
    let restored_before = std::fs::read(&worker_path).unwrap();
    assert!(restored_before.starts_with(&before));
    // The original owner nonce is stale after restart, but an exact receipt remains queryable.
    assert_eq!(
        dispatch(&mut host, &registry, &phone, &req, &runner),
        accepted
    );
    assert!(
        runner
            .recv_timeout(std::time::Duration::from_millis(30))
            .is_err()
    );
    assert_eq!(std::fs::read(worker_path).unwrap(), restored_before);
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        dispatch(&mut host, &registry, &phone, &req, &runner).code,
        Some(ErrorCode::Unauthorized)
    );
    runner.shutdown_and_join().unwrap();
}

#[test]
fn worker_admission_refusals_leave_checkpoint_and_worker_history_unchanged() {
    use zenpi::{
        remote::{worker_admission::dispatch, worktree_runner::WorktreeRunner},
        runtime::RuntimeConfig,
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let state = host.subtab_state(&project).unwrap();
    let tab = state.active_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let runner =
        WorktreeRunner::spawn_prompts(&mut host, &project, &tab, RuntimeConfig::default()).unwrap();
    let valid = request(
        &host,
        Operation::Prompt,
        &project,
        "invalid-worker",
        json!({"text":"safe",
        "subtab_id":tab,"subtab_revision":state.revision}),
    );
    let path = runner.workers()[0]
        .agent()
        .lock()
        .unwrap()
        .session()
        .path()
        .to_owned();
    let before = std::fs::read(&path).unwrap();
    let master = host.owner(&project).unwrap();
    let master_path = master.lock().unwrap().session().path().to_owned();
    let master_before = std::fs::read(&master_path).unwrap();
    for text in ["", "!touch unexpected", "has\0nul", "@.zenpi/auth.json"] {
        let mut req = valid.clone();
        if let Message::Request { arguments, .. } = &mut req.payload {
            arguments["text"] = json!(text);
        }
        assert_eq!(
            dispatch(&mut host, &registry, &phone, &req, &runner).code,
            Some(ErrorCode::InvalidRequest)
        );
    }
    let mut wrong = valid.clone();
    wrong.required_owner = Some("wrong-owner".into());
    assert_eq!(
        dispatch(&mut host, &registry, &phone, &wrong, &runner).code,
        Some(ErrorCode::OwnerMismatch)
    );
    let mut wrong = valid.clone();
    wrong.expected_revision = Some(host.workspace_revision() + 1);
    assert_eq!(
        dispatch(&mut host, &registry, &phone, &wrong, &runner).code,
        Some(ErrorCode::RevisionConflict)
    );
    let mut wrong = valid.clone();
    if let Message::Request { arguments, .. } = &mut wrong.payload {
        arguments["subtab_revision"] = json!(state.revision + 1);
    }
    assert_eq!(
        dispatch(&mut host, &registry, &phone, &wrong, &runner).code,
        Some(ErrorCode::RevisionConflict)
    );
    let mut wrong = valid.clone();
    if let Message::Request { arguments, .. } = &mut wrong.payload {
        arguments["subtab_id"] = json!("foreign-tab");
    }
    assert_eq!(
        dispatch(&mut host, &registry, &phone, &wrong, &runner).code,
        Some(ErrorCode::OwnerMismatch)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read(&master_path).unwrap(), master_before);
    let mut wrong = valid.clone();
    if let Message::Request { arguments, .. } = &mut wrong.payload {
        arguments["unexpected"] = json!(true);
    }
    assert_eq!(
        dispatch(&mut host, &registry, &phone, &wrong, &runner).code,
        Some(ErrorCode::InvalidRequest)
    );
    let master_guard = master.lock().unwrap();
    let held_response = dispatch(&mut host, &registry, &phone, &valid, &runner);
    assert_eq!(
        held_response.execution_state,
        Some(zenpi::remote::protocol::ExecutionState::UnknownOutcome)
    );
    assert_eq!(std::fs::read(&master_path).unwrap(), master_before);
    drop(master_guard);
    assert!(
        runner
            .recv_timeout(std::time::Duration::from_millis(30))
            .is_err()
    );
    runner.shutdown_and_join().unwrap();
}

#[test]
fn worker_checkpoint_only_reservation_after_crash_is_unknown_and_never_resubmitted() {
    use zenpi::{
        remote::{
            protocol::ExecutionState, worker_admission::dispatch, worktree_runner::WorktreeRunner,
        },
        runtime::{RuntimeConfig, RuntimeEvent},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let state = host.subtab_state(&project).unwrap();
    let tab = state.active_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let runner =
        WorktreeRunner::spawn_prompts(&mut host, &project, &tab, RuntimeConfig::default()).unwrap();
    let req = request(
        &host,
        Operation::Prompt,
        &project,
        "worker-crash-reservation",
        json!({"text":"original-once","subtab_id":tab,"subtab_revision":state.revision}),
    );
    assert_eq!(
        dispatch(&mut host, &registry, &phone, &req, &runner).execution_state,
        Some(ExecutionState::Accepted)
    );
    loop {
        if matches!(
            runner
                .recv_timeout(std::time::Duration::from_secs(3))
                .unwrap(),
            RuntimeEvent::Completed { .. }
        ) {
            break;
        }
    }
    let master_path = host
        .owner(&project)
        .unwrap()
        .lock()
        .unwrap()
        .session()
        .path()
        .to_owned();
    let worker_path = runner.workers()[0]
        .agent()
        .lock()
        .unwrap()
        .session()
        .path()
        .to_owned();
    let worker_before = std::fs::read(&worker_path).unwrap();
    runner.shutdown_and_join().unwrap();
    drop(host);
    // Model a crash after durable checkpoint binding but before either admission
    // journal event. Preserve all pre-reservation records and valid sequence.
    let records: Vec<Value> = std::fs::read_to_string(&master_path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .take_while(|record: &Value| record["event"]["type"] != "remote_worker_admission_v1")
        .collect();
    let mut bytes = Vec::new();
    for record in records {
        serde_json::to_writer(&mut bytes, &record).unwrap();
        bytes.push(b'\n');
    }
    std::fs::write(&master_path, bytes).unwrap();
    let mut host = pool(&root);
    let runner =
        WorktreeRunner::spawn_prompts(&mut host, &project, &tab, RuntimeConfig::default()).unwrap();
    let restored_worker = std::fs::read(&worker_path).unwrap();
    assert!(restored_worker.starts_with(&worker_before));
    let response = dispatch(&mut host, &registry, &phone, &req, &runner);
    assert_eq!(
        response.execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert!(matches!(response.payload, Message::Response { result } if result.is_null()));
    assert!(
        runner
            .recv_timeout(std::time::Duration::from_millis(30))
            .is_err()
    );
    assert_eq!(std::fs::read(&worker_path).unwrap(), restored_worker);
    runner.shutdown_and_join().unwrap();
}

#[test]
fn worker_journal_import_restores_cross_owner_request_binding_before_new_submission() {
    use zenpi::{
        remote::{
            protocol::ExecutionState, worker_admission::dispatch, worktree_runner::WorktreeRunner,
        },
        runtime::{RuntimeConfig, RuntimeEvent},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let state = host.subtab_state(&project).unwrap();
    let tab = state.active_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let runner =
        WorktreeRunner::spawn_prompts(&mut host, &project, &tab, RuntimeConfig::default()).unwrap();
    let req = request(
        &host,
        Operation::Prompt,
        &project,
        "worker-import-binding",
        json!({"text":"already-executed","subtab_id":tab,"subtab_revision":state.revision}),
    );
    let accepted = dispatch(&mut host, &registry, &phone, &req, &runner);
    assert_eq!(accepted.execution_state, Some(ExecutionState::Accepted));
    loop {
        if matches!(
            runner
                .recv_timeout(std::time::Duration::from_secs(3))
                .unwrap(),
            RuntimeEvent::Completed { .. }
        ) {
            break;
        }
    }
    runner.shutdown_and_join().unwrap();
    drop(host);
    let checkpoint = root.join("project-workspace.json");
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&checkpoint).unwrap()).unwrap();
    saved["remote_workspace"]["prompt_bindings"] = json!([]);
    std::fs::write(&checkpoint, serde_json::to_vec(&saved).unwrap()).unwrap();
    let mut host = pool(&root);
    let normal = request(
        &host,
        Operation::Prompt,
        &project,
        "worker-import-binding",
        json!({"text":"master-must-not-run"}),
    );
    let collision = zenpi::remote::command::dispatch_prompt(
        &mut host,
        &registry,
        &phone,
        &normal,
        false,
        |_, _, _| panic!("imported worker ID must fence master submission"),
    );
    assert_eq!(collision.code, Some(ErrorCode::RequestIdConflict));
    let runner =
        WorktreeRunner::spawn_prompts(&mut host, &project, &tab, RuntimeConfig::default()).unwrap();
    assert_eq!(
        dispatch(&mut host, &registry, &phone, &req, &runner),
        accepted
    );
    assert!(
        runner
            .recv_timeout(std::time::Duration::from_millis(30))
            .is_err()
    );
    runner.shutdown_and_join().unwrap();
}

#[test]
fn tracked_worker_completion_survives_master_contention_and_restart_without_duplicate_append() {
    use zenpi::{
        remote::{
            protocol::ExecutionState,
            worker_admission::dispatch_tracked,
            worker_progress::{WorkerProgress, listing},
            worktree_runner::WorktreeRunner,
        },
        runtime::{RuntimeConfig, RuntimeEvent},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let state = host.subtab_state(&project).unwrap();
    let tab = state.active_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let runner =
        WorktreeRunner::spawn_prompts(&mut host, &project, &tab, RuntimeConfig::default()).unwrap();
    let mut progress = WorkerProgress::default();
    let req = request(
        &host,
        Operation::Prompt,
        &project,
        "tracked-worker",
        json!({"text":"worker-only-content",
        "subtab_id":tab,"subtab_revision":state.revision}),
    );
    let accepted = dispatch_tracked(&mut host, &registry, &phone, &req, &runner, &mut progress);
    assert_eq!(accepted.execution_state, Some(ExecutionState::Accepted));
    assert_eq!(progress.pending_count(), 1);
    assert_eq!(
        dispatch_tracked(&mut host, &registry, &phone, &req, &runner, &mut progress),
        accepted
    );
    assert_eq!(progress.pending_count(), 1);
    let master = host.owner(&project).unwrap();
    let guard = master.lock().unwrap();
    let path = guard.session().path().to_owned();
    let before = std::fs::read(&path).unwrap();
    loop {
        let event = runner
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        let completed = matches!(event, RuntimeEvent::Completed { .. });
        progress.observe(event, false);
        progress.flush(&host);
        if completed {
            break;
        }
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(progress.pending_count(), 1);
    assert_eq!(
        progress.listing(&host, phone.id(), &project)[0]["state"],
        "succeeded"
    );
    assert!(
        progress
            .listing(&host, "foreign", &project)
            .as_array()
            .unwrap()
            .is_empty()
    );
    drop(guard);
    progress.flush(&host);
    assert_eq!(progress.pending_count(), 0);
    let after = std::fs::read(&path).unwrap();
    progress.flush(&host);
    assert_eq!(std::fs::read(&path).unwrap(), after);
    let value = listing(&host, phone.id(), &project);
    assert_eq!(value[0]["state"], "succeeded");
    assert_eq!(value[0]["subtab_id"], tab);
    assert!(
        value[0]["output_text"]
            .as_str()
            .unwrap()
            .contains("worker-only-content")
    );
    assert_eq!(value[0]["output_truncated"], false);
    runner.shutdown_and_join().unwrap();
    drop(host);
    let host = pool(&root);
    assert_eq!(listing(&host, phone.id(), &project), value);
}

#[test]
fn worker_progress_append_failure_is_unknown_and_never_blindly_retried() {
    use zenpi::{
        remote::{
            worker_admission::dispatch_tracked, worker_progress::WorkerProgress,
            worktree_runner::WorktreeRunner,
        },
        runtime::{RuntimeConfig, RuntimeEvent},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let state = host.subtab_state(&project).unwrap();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let runner = WorktreeRunner::spawn_prompts(
        &mut host,
        &project,
        &state.active_id,
        RuntimeConfig::default(),
    )
    .unwrap();
    let mut progress = WorkerProgress::default();
    let req = request(
        &host,
        Operation::Prompt,
        &project,
        "failed-progress-append",
        json!({"text":"safe",
        "subtab_id":state.active_id,"subtab_revision":state.revision}),
    );
    dispatch_tracked(&mut host, &registry, &phone, &req, &runner, &mut progress);
    loop {
        let event = runner
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        let completed = matches!(event, RuntimeEvent::Completed { .. });
        progress.observe(event, false);
        if completed {
            break;
        }
    }
    let master = host.owner(&project).unwrap();
    let path = master.lock().unwrap().session().path().to_owned();
    let original = path.with_extension("held-original");
    std::fs::rename(&path, &original).unwrap();
    std::fs::write(&path, "foreign replacement\n").unwrap();
    progress.flush(&host);
    assert_eq!(
        progress.listing(&host, phone.id(), &project)[0]["state"],
        "unknown_outcome"
    );
    assert!(
        progress.listing(&host, phone.id(), &project)[0]
            .get("output_text")
            .is_none()
    );
    std::fs::remove_file(&path).unwrap();
    std::fs::rename(&original, &path).unwrap();
    let before = std::fs::read(&path).unwrap();
    progress.flush(&host);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    runner.shutdown_and_join().unwrap();
}

#[test]
fn worker_completed_output_survives_restart_redacted_without_rewriting_private_worker_journal() {
    use zenpi::{
        remote::{
            worker_admission::dispatch_tracked,
            worker_progress::{WorkerProgress, listing},
            worktree_runner::WorktreeRunner,
        },
        runtime::{RuntimeConfig, RuntimeEvent},
    };
    for body in ["public answer\nAPI_KEY=PRIVATE_COMPLETED_SECRET\n-----BEGIN PRIVATE KEY-----\nPRIVATE_BODY\n-----END PRIVATE KEY-----\nend".to_owned(), "\u{754c}".repeat(2000)] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let mut host = pool(&root);
        let project = host.active_context().project_id.clone();
        let state = host.subtab_state(&project).unwrap();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let phone = device(&registry, vec![project.clone()], vec![Capability::Read, Capability::Control]);
        let runner = WorktreeRunner::spawn_prompts(&mut host, &project, &state.active_id, RuntimeConfig::default()).unwrap();
        let request = request(&host, Operation::Prompt, &project, "completed-output", json!({"text":body, "subtab_id":state.active_id,"subtab_revision":state.revision}));
        let mut progress = WorkerProgress::default();
        dispatch_tracked(&mut host, &registry, &phone, &request, &runner, &mut progress);
        loop {
            let event = runner.recv_timeout(std::time::Duration::from_secs(3)).unwrap();
            let completed = matches!(event, RuntimeEvent::Completed { .. });
            progress.observe(event, false);
            if completed { break; }
        }
        progress.flush(&host);
        let value = listing(&host, phone.id(), &project);
        assert_eq!(value[0]["state"], "succeeded");
        let output = value[0]["output_text"].as_str().unwrap();
        assert!(output.len() <= 4096);
        assert!(!output.contains("PRIVATE_COMPLETED_SECRET"));
        assert!(!output.contains("PRIVATE_BODY"));
        assert_eq!(value[0]["output_truncated"], body.len() > 4096);
        assert!(listing(&host, "foreign", &project).as_array().unwrap().is_empty());
        let worker = &runner.workers()[0];
        let path = worker.agent().lock().unwrap().session().path().to_owned();
        let raw = std::fs::read_to_string(&path).unwrap();
        if body.len() < 4096 { assert!(raw.contains("PRIVATE_COMPLETED_SECRET")); }
        let before = std::fs::read(&path).unwrap();
        runner.shutdown_and_join().unwrap();
        drop(host);
        let host = pool(&root);
        assert_eq!(listing(&host, phone.id(), &project), value);
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}

#[test]
fn tracked_worker_queue_rejection_is_durable_failure_not_permanent_accepted() {
    use zenpi::{
        remote::{
            protocol::ExecutionState,
            worker_admission::dispatch_tracked,
            worker_progress::{WorkerProgress, listing},
            worktree_runner::WorktreeRunner,
        },
        runtime::{RuntimeConfig, RuntimeEvent},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let state = host.subtab_state(&project).unwrap();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let runner = WorktreeRunner::spawn_prompts(
        &mut host,
        &project,
        &state.active_id,
        RuntimeConfig {
            max_pending: 1,
            ..RuntimeConfig::default()
        },
    )
    .unwrap();
    let guard = runner.workers()[0].agent().lock().unwrap();
    let mut progress = WorkerProgress::default();
    let mut requests = Vec::new();
    for index in 0..3 {
        let req = request(
            &host,
            Operation::Prompt,
            &project,
            &format!("queue-{index}"),
            json!({"text":"safe","subtab_id":state.active_id,"subtab_revision":state.revision}),
        );
        assert_eq!(
            dispatch_tracked(&mut host, &registry, &phone, &req, &runner, &mut progress)
                .execution_state,
            Some(ExecutionState::Accepted)
        );
        requests.push(req);
    }
    loop {
        let event = runner
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        let rejected = matches!(event, RuntimeEvent::Rejected { .. });
        progress.observe(event, false);
        progress.flush(&host);
        if rejected {
            break;
        }
    }
    let failed = listing(&host, phone.id(), &project)
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["request_id"] == "queue-2")
        .unwrap()
        .clone();
    drop(guard);
    let mut completed = 0;
    while completed < 2 {
        let event = runner
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        if matches!(event, RuntimeEvent::Completed { .. }) {
            completed += 1;
        }
        progress.observe(event, false);
        progress.flush(&host);
    }
    assert_eq!(failed["state"], "failed");
    assert_eq!(failed["failure_code"], "busy");
    assert_eq!(progress.pending_count(), 0);
    // Immutable admission remains accepted, while execution status truthfully failed.
    assert_eq!(
        dispatch_tracked(
            &mut host,
            &registry,
            &phone,
            &requests[2],
            &runner,
            &mut progress
        )
        .execution_state,
        Some(ExecutionState::Accepted)
    );
    assert!(
        runner
            .recv_timeout(std::time::Duration::from_millis(30))
            .is_err()
    );
    runner.shutdown_and_join().unwrap();
}

#[test]
fn worker_cancellation_is_device_scoped_exact_retry_safe_and_does_not_stop_other_slots() {
    use zenpi::{
        remote::{
            protocol::ExecutionState, worker_admission::dispatch_tracked,
            worker_progress::WorkerProgress, worktree_runner::WorktreeRunner,
        },
        runtime::{RuntimeConfig, RuntimeEvent},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let initial = host.subtab_state(&project).unwrap();
    host.commit_subtabs(
        &project,
        initial.revision,
        initial.set_concurrency(&initial.active_id, 2).unwrap(),
    )
    .unwrap();
    let state = host.subtab_state(&project).unwrap();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let other = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let runner = WorktreeRunner::spawn_prompts(
        &mut host,
        &project,
        &state.active_id,
        RuntimeConfig::default(),
    )
    .unwrap();
    // Hold each Agent while its runtime receives work, without holding master
    // or approval controls. No worker can finish before targeted cancellation.
    let guards: Vec<_> = runner
        .workers()
        .iter()
        .map(|worker| worker.agent().lock().unwrap())
        .collect();
    let mut progress = WorkerProgress::default();
    for index in 0..3 {
        let req = request(
            &host,
            Operation::Prompt,
            &project,
            &format!("cancel-worker-{index}"),
            json!({"text":format!("private-worker-{index}"),"subtab_id":state.active_id,"subtab_revision":state.revision}),
        );
        assert_eq!(
            dispatch_tracked(&mut host, &registry, &phone, &req, &runner, &mut progress)
                .execution_state,
            Some(ExecutionState::Accepted)
        );
    }
    let mut started = 0;
    let mut queued = false;
    while started < 2 || !queued {
        let event = runner
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        if matches!(event, RuntimeEvent::Started { .. }) {
            started += 1;
        }
        if matches!(event, RuntimeEvent::Queued { .. }) {
            queued = true;
        }
        progress.observe(event, false);
        progress.flush(&host);
    }
    let cancel_active = request(
        &host,
        Operation::Cancel,
        &project,
        "cancel-active-worker",
        json!({"request_id":"cancel-worker-0"}),
    );
    let before = std::fs::read(root.join("project-workspace.json")).unwrap();
    let mut wrong = cancel_active.clone();
    wrong.required_owner = Some("foreign-owner".into());
    assert_eq!(
        progress
            .cancel(&mut host, &registry, &phone, &wrong, &runner)
            .code,
        Some(ErrorCode::OwnerMismatch)
    );
    let mut wrong = cancel_active.clone();
    wrong.expected_revision = Some(host.workspace_revision() + 1);
    assert_eq!(
        progress
            .cancel(&mut host, &registry, &phone, &wrong, &runner)
            .code,
        Some(ErrorCode::RevisionConflict)
    );
    let mut wrong = cancel_active.clone();
    wrong.session_id = Some("foreign-session".into());
    assert_eq!(
        progress
            .cancel(&mut host, &registry, &phone, &wrong, &runner)
            .code,
        Some(ErrorCode::OwnerMismatch)
    );
    assert_eq!(
        progress
            .cancel(&mut host, &registry, &other, &cancel_active, &runner)
            .code,
        Some(ErrorCode::OwnerMismatch)
    );
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        before
    );
    let receipt = progress.cancel(&mut host, &registry, &phone, &cancel_active, &runner);
    assert_eq!(receipt.execution_state, Some(ExecutionState::Accepted));
    assert_eq!(
        progress.cancel(&mut host, &registry, &phone, &cancel_active, &runner),
        receipt
    );
    let cancel_queued = request(
        &host,
        Operation::Cancel,
        &project,
        "cancel-queued-worker",
        json!({"request_id":"cancel-worker-2"}),
    );
    assert_eq!(
        progress
            .cancel(&mut host, &registry, &phone, &cancel_queued, &runner)
            .execution_state,
        Some(ExecutionState::Accepted)
    );
    // Consume both cancellation commands before releasing execution locks.
    let mut cancellations = 0;
    while cancellations < 2 {
        let event = runner
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        if matches!(event, RuntimeEvent::CancelRequested { .. }) {
            cancellations += 1;
        }
        progress.observe(event, false);
        progress.flush(&host);
    }
    drop(guards);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while progress.pending_count() != 0 {
        if let Ok(event) = runner.recv_timeout(std::time::Duration::from_millis(20)) {
            progress.observe(event, false);
        }
        progress.flush(&host);
        assert!(std::time::Instant::now() < until);
    }
    let values = progress.listing(&host, phone.id(), &project);
    let state_of = |id: &str| {
        values
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["request_id"] == id)
            .unwrap()["state"]
            .clone()
    };
    assert_eq!(state_of("cancel-worker-0"), "cancelled");
    assert_eq!(state_of("cancel-worker-2"), "cancelled");
    assert_eq!(state_of("cancel-worker-1"), "succeeded");
    assert_eq!(
        progress.cancel(&mut host, &registry, &phone, &cancel_active, &runner),
        receipt
    );
    let fresh = request(
        &host,
        Operation::Cancel,
        &project,
        "late-new-cancel",
        json!({"request_id":"cancel-worker-0"}),
    );
    assert_eq!(
        progress
            .cancel(&mut host, &registry, &phone, &fresh, &runner)
            .code,
        Some(ErrorCode::InvalidRequest)
    );
    runner.shutdown_and_join().unwrap();
    drop(host);
    let mut host = pool(&root);
    let runner = WorktreeRunner::spawn_prompts(
        &mut host,
        &project,
        &state.active_id,
        RuntimeConfig::default(),
    )
    .unwrap();
    let empty = WorkerProgress::default();
    assert_eq!(
        empty.cancel(&mut host, &registry, &phone, &cancel_active, &runner),
        receipt
    );
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        empty
            .cancel(&mut host, &registry, &phone, &cancel_active, &runner)
            .code,
        Some(ErrorCode::Unauthorized)
    );
    assert!(
        runner
            .recv_timeout(std::time::Duration::from_millis(30))
            .is_err()
    );
    runner.shutdown_and_join().unwrap();
}

#[test]
fn old_worker_tracker_cannot_cancel_a_new_runner_even_when_its_job_ids_restart() {
    use zenpi::{
        remote::{
            protocol::ExecutionState, worker_admission::dispatch_tracked,
            worker_progress::WorkerProgress, worktree_runner::WorktreeRunner,
        },
        runtime::RuntimeConfig,
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let state = host.subtab_state(&project).unwrap();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let runner = WorktreeRunner::spawn_prompts(
        &mut host,
        &project,
        &state.active_id,
        RuntimeConfig::default(),
    )
    .unwrap();
    let mut old = WorkerProgress::default();
    let req = request(
        &host,
        Operation::Prompt,
        &project,
        "old-generation-worker",
        json!({"text":"old",
        "subtab_id":state.active_id,"subtab_revision":state.revision}),
    );
    assert_eq!(
        dispatch_tracked(&mut host, &registry, &phone, &req, &runner, &mut old).execution_state,
        Some(ExecutionState::Accepted)
    );
    // Deliberately do not observe terminal events in the old tracker.
    runner.shutdown_and_join().unwrap();
    let runner = WorktreeRunner::spawn_prompts(
        &mut host,
        &project,
        &state.active_id,
        RuntimeConfig::default(),
    )
    .unwrap();
    let cancel = request(
        &host,
        Operation::Cancel,
        &project,
        "new-cancel-old-generation",
        json!({"request_id":"old-generation-worker"}),
    );
    let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
    assert_eq!(
        old.cancel(&mut host, &registry, &phone, &cancel, &runner)
            .code,
        Some(ErrorCode::InvalidRequest)
    );
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        checkpoint
    );
    assert!(
        runner
            .recv_timeout(std::time::Duration::from_millis(30))
            .is_err()
    );
    runner.shutdown_and_join().unwrap();
}

#[test]
fn two_devices_using_the_same_worker_request_string_cancel_only_their_own_job() {
    use zenpi::{
        remote::{
            protocol::ExecutionState, worker_admission::dispatch_tracked,
            worker_progress::WorkerProgress, worktree_runner::WorktreeRunner,
        },
        runtime::{RuntimeConfig, RuntimeEvent},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let initial = host.subtab_state(&project).unwrap();
    host.commit_subtabs(
        &project,
        initial.revision,
        initial.set_concurrency(&initial.active_id, 2).unwrap(),
    )
    .unwrap();
    let state = host.subtab_state(&project).unwrap();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let first = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let second = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let runner = WorktreeRunner::spawn_prompts(
        &mut host,
        &project,
        &state.active_id,
        RuntimeConfig::default(),
    )
    .unwrap();
    let guards: Vec<_> = runner
        .workers()
        .iter()
        .map(|worker| worker.agent().lock().unwrap())
        .collect();
    let mut progress = WorkerProgress::default();
    let req = request(
        &host,
        Operation::Prompt,
        &project,
        "same-device-scoped-id",
        json!({"text":"safe","subtab_id":state.active_id,"subtab_revision":state.revision}),
    );
    for phone in [&first, &second] {
        assert_eq!(
            dispatch_tracked(&mut host, &registry, phone, &req, &runner, &mut progress)
                .execution_state,
            Some(ExecutionState::Accepted)
        );
    }
    let cancel = request(
        &host,
        Operation::Cancel,
        &project,
        "second-device-cancel",
        json!({"request_id":"same-device-scoped-id"}),
    );
    assert_eq!(
        progress
            .cancel(&mut host, &registry, &second, &cancel, &runner)
            .execution_state,
        Some(ExecutionState::Accepted)
    );
    loop {
        let event = runner
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        let cancelled = matches!(event, RuntimeEvent::CancelRequested { .. });
        progress.observe(event, false);
        progress.flush(&host);
        if cancelled {
            break;
        }
    }
    drop(guards);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while progress.pending_count() != 0 {
        if let Ok(event) = runner.recv_timeout(std::time::Duration::from_millis(20)) {
            progress.observe(event, false);
        }
        progress.flush(&host);
        assert!(std::time::Instant::now() < until);
    }
    assert_eq!(
        progress.listing(&host, first.id(), &project)[0]["state"],
        "succeeded"
    );
    assert_eq!(
        progress.listing(&host, second.id(), &project)[0]["state"],
        "cancelled"
    );
    runner.shutdown_and_join().unwrap();
}

#[test]
fn worker_approvals_are_session_scoped_one_shot_and_receipts_survive_restart() {
    use zenpi::{
        approval::{ApprovalDecision, ApprovalPolicy, ApprovalRequest},
        remote::{
            approval::{dispatch_worker, worker_listing},
            protocol::ExecutionState,
        },
        tools::{ToolOrigin, ToolSideEffect},
    };
    for decision in [ApprovalDecision::Allow, ApprovalDecision::Deny] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let mut host = pool(&root);
        let project = host.active_context().project_id.clone();
        let initial = host.subtab_state(&project).unwrap();
        host.commit_subtabs(
            &project,
            initial.revision,
            initial.set_concurrency(&initial.active_id, 2).unwrap(),
        )
        .unwrap();
        let first = host.worker_owner(&project, &initial.active_id, 0).unwrap();
        let second = host.worker_owner(&project, &initial.active_id, 1).unwrap();
        let coordinators: Vec<_> = [&first, &second]
            .iter()
            .map(|worker| worker.control().approval.unwrap())
            .collect();
        let sessions: Vec<_> = [&first, &second]
            .iter()
            .map(|worker| worker.control().input_port.session_id().as_str().to_owned())
            .collect();
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut joins = Vec::new();
        let mut digests = Vec::new();
        for (index, coordinator) in coordinators.iter().enumerate() {
            let pending = ApprovalRequest {
                request_id: "same-approval-id".into(),
                turn_id: format!("turn-{index}"),
                call_id: "write-call".into(),
                tool: "write_file".into(),
                side_effect: ToolSideEffect::WorkspaceWrite,
                arguments: json!({"path":"note.txt","content":format!("PRIVATE_WORKER_BODY_{index}")}),
                preview: None,
                origin: ToolOrigin::AgentTool,
                policy_digest: None,
                lease_id: None,
            };
            digests.push(pending.digest().unwrap());
            let coordinator = coordinator.clone();
            let stopping = stop.clone();
            joins.push(std::thread::spawn(move || {
                coordinator.request_response(pending, &mut ApprovalPolicy::default(), &|| {
                    stopping.load(std::sync::atomic::Ordering::Acquire)
                })
            }));
        }
        let until = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while coordinators
            .iter()
            .any(|coordinator| !coordinator.is_pending("same-approval-id"))
        {
            assert!(std::time::Instant::now() < until);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let phone = device(
            &registry,
            vec![project.clone()],
            vec![Capability::Read, Capability::Approval],
        );
        let controller = device(
            &registry,
            vec![project.clone()],
            vec![Capability::Read, Capability::Control],
        );
        let listed = worker_listing(&host, &project).unwrap();
        assert_eq!(listed.as_array().unwrap().len(), 2);
        assert!(!listed.to_string().contains("PRIVATE_WORKER_BODY"));
        let mut inspect = request(
            &host,
            Operation::Inspect,
            &project,
            "worker-approval-inspect",
            json!({}),
        );
        inspect.required_owner = None;
        inspect.expected_revision = None;
        assert_eq!(
            zenpi::remote::approval::authorized_worker_listing(
                &host,
                &registry,
                &controller,
                &inspect
            )
            .unwrap(),
            listed
        );
        let mut foreign_inspect = inspect.clone();
        foreign_inspect.session_id = Some("foreign-session".into());
        assert_eq!(
            zenpi::remote::approval::authorized_worker_listing(
                &host,
                &registry,
                &phone,
                &foreign_inspect
            ),
            Err(ErrorCode::OwnerMismatch)
        );
        let ungranted = device(
            &registry,
            vec![],
            vec![Capability::Read, Capability::Approval],
        );
        assert_eq!(
            zenpi::remote::approval::authorized_worker_listing(
                &host, &registry, &ungranted, &inspect
            ),
            Err(ErrorCode::Unauthorized)
        );
        assert!(
            zenpi::remote::approval::listing(&host, &project)
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
        );
        let answer = request(
            &host,
            Operation::Approve,
            &project,
            "answer-worker",
            json!({"request_id":"same-approval-id","digest":digests[0],"decision":decision,"worker_session_id":sessions[0]}),
        );
        let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
        assert_eq!(
            dispatch_worker(&mut host, &registry, &controller, &answer).code,
            Some(ErrorCode::Unauthorized)
        );
        let mut wrong = answer.clone();
        wrong.required_owner = Some("foreign-owner".into());
        assert_eq!(
            dispatch_worker(&mut host, &registry, &phone, &wrong).code,
            Some(ErrorCode::OwnerMismatch)
        );
        let mut wrong = answer.clone();
        wrong.expected_revision = Some(host.workspace_revision() + 1);
        assert_eq!(
            dispatch_worker(&mut host, &registry, &phone, &wrong).code,
            Some(ErrorCode::RevisionConflict)
        );
        let mut wrong = answer.clone();
        if let Message::Request { arguments, .. } = &mut wrong.payload {
            arguments["remember"] = json!(true);
        }
        assert_eq!(
            dispatch_worker(&mut host, &registry, &phone, &wrong).code,
            Some(ErrorCode::InvalidRequest)
        );
        let mut wrong = answer.clone();
        if let Message::Request { arguments, .. } = &mut wrong.payload {
            arguments["worker_session_id"] = json!(sessions[1]);
        }
        assert_eq!(
            dispatch_worker(&mut host, &registry, &phone, &wrong).code,
            Some(ErrorCode::InvalidRequest)
        );
        assert_eq!(
            std::fs::read(root.join("project-workspace.json")).unwrap(),
            checkpoint
        );
        let master = host.owner(&project).unwrap();
        let saved_context = host.active_context().session_id.clone();
        let next_path = root.join("replacement-master.jsonl");
        drop(SessionStore::open_in_workspace(&next_path, &root).unwrap());
        {
            let mut agent = master.lock().unwrap();
            agent.resume_session(&next_path).unwrap();
            host.refresh_owner_context(&project, &agent);
        }
        let mut stale_worker = answer.clone();
        stale_worker.session_id = Some(host.active_context().session_id.clone());
        assert!(
            worker_listing(&host, &project)
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            dispatch_worker(&mut host, &registry, &phone, &stale_worker).code,
            Some(ErrorCode::InvalidRequest)
        );
        {
            let mut agent = master.lock().unwrap();
            agent.resume_session(root.join("initial.jsonl")).unwrap();
            host.refresh_owner_context(&project, &agent);
        }
        assert_eq!(host.active_context().session_id, saved_context);
        let checkpoint_path = root.join("project-workspace.json");
        let checkpoint_bytes = std::fs::read(&checkpoint_path).unwrap();
        let mut external = checkpoint_bytes.clone();
        external.push(b' ');
        std::fs::write(&checkpoint_path, &external).unwrap();
        assert_eq!(
            dispatch_worker(&mut host, &registry, &phone, &answer).execution_state,
            Some(ExecutionState::UnknownOutcome)
        );
        assert!(coordinators[0].is_pending("same-approval-id"));
        assert!(coordinators[1].is_pending("same-approval-id"));
        assert_eq!(std::fs::read(&checkpoint_path).unwrap(), external);
        std::fs::write(&checkpoint_path, &checkpoint_bytes).unwrap();
        let guard = first.agent().lock().unwrap();
        let receipt = dispatch_worker(&mut host, &registry, &phone, &answer);
        assert_eq!(receipt.execution_state, Some(ExecutionState::Accepted));
        assert_eq!(
            dispatch_worker(&mut host, &registry, &phone, &answer),
            receipt
        );
        let mut changed_target = answer.clone();
        if let Message::Request { arguments, .. } = &mut changed_target.payload {
            arguments["worker_session_id"] = json!(sessions[1]);
        }
        assert_eq!(
            dispatch_worker(&mut host, &registry, &phone, &changed_target).code,
            Some(ErrorCode::RequestIdConflict)
        );
        assert!(coordinators[1].is_pending("same-approval-id"));
        let actor = coordinators[0]
            .accepted("same-approval-id")
            .unwrap()
            .remote
            .unwrap();
        assert_eq!(actor.session_id, sessions[0]);
        assert_eq!(actor.device_id, phone.id());
        drop(guard);
        let response = joins.remove(0).join().unwrap().unwrap();
        assert_eq!(response.decision, decision);
        coordinators[0].mark_persisted("same-approval-id").unwrap();
        stop.store(true, std::sync::atomic::Ordering::Release);
        let _ = joins.remove(0).join().unwrap();
        assert_eq!(
            dispatch_worker(&mut host, &registry, &phone, &answer),
            receipt
        );
        drop(first);
        drop(second);
        drop(host);
        let mut host = pool(&root);
        // Exact old receipt does not require preparing or touching replacement workers.
        assert_eq!(
            dispatch_worker(&mut host, &registry, &phone, &answer),
            receipt
        );
        assert!(host.worker_handles(&project).is_empty());
        registry.revoke(phone.id()).unwrap();
        assert_eq!(
            dispatch_worker(&mut host, &registry, &phone, &answer).code,
            Some(ErrorCode::Unauthorized)
        );
    }
}

#[test]
fn headless_mailbox_projects_and_routes_worker_approvals_without_worker_agent_lock() {
    use zenpi::{
        approval::{ApprovalDecision, ApprovalPolicy, ApprovalRequest},
        remote::{headless_host::HeadlessHost, protocol::ExecutionState},
        tools::{ToolOrigin, ToolSideEffect},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let initial = host.subtab_state(&project).unwrap();
    host.commit_subtabs(
        &project,
        initial.revision,
        initial.set_concurrency(&initial.active_id, 2).unwrap(),
    )
    .unwrap();
    let workers: Vec<_> = (0..2)
        .map(|slot| {
            host.worker_owner(&project, &initial.active_id, slot)
                .unwrap()
        })
        .collect();
    let sessions: Vec<_> = workers
        .iter()
        .map(|worker| worker.control().input_port.session_id().as_str().to_owned())
        .collect();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut joins = Vec::new();
    let mut digests = Vec::new();
    let coordinators: Vec<_> = workers
        .iter()
        .map(|worker| worker.control().approval.unwrap())
        .collect();
    for (index, coordinator) in coordinators.iter().enumerate() {
        let pending = ApprovalRequest {
            request_id: "same-mailbox-approval".into(),
            turn_id: format!("turn-{index}"),
            call_id: "call".into(),
            tool: "write_file".into(),
            side_effect: ToolSideEffect::WorkspaceWrite,
            arguments: json!({"path":"note.txt","content":"PRIVATE_MAILBOX_WORKER_BODY"}),
            preview: None,
            origin: ToolOrigin::AgentTool,
            policy_digest: None,
            lease_id: None,
        };
        digests.push(pending.digest().unwrap());
        let coordinator = coordinator.clone();
        let stopping = stop.clone();
        joins.push(std::thread::spawn(move || {
            coordinator.request_response(pending, &mut ApprovalPolicy::default(), &|| {
                stopping.load(std::sync::atomic::Ordering::Acquire)
            })
        }));
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while coordinators
        .iter()
        .any(|coordinator| !coordinator.is_pending("same-mailbox-approval"))
    {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Approval],
    );
    let controller = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let mut list = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::WorkspaceList,
            arguments: json!({}),
        },
    );
    list.id = Some("worker-mailbox-list".into());
    let answer = request(
        &host,
        Operation::Approve,
        &project,
        "mailbox-answer",
        json!({
            "request_id":"same-mailbox-approval","digest":digests[0],"decision":"deny","worker_session_id":sessions[0]
        }),
    );
    let (adapter, runtime) = HeadlessHost::start(host);
    let projected = adapter.dispatch(&registry, &phone, list.clone());
    assert_eq!(
        projected.execution_state,
        Some(ExecutionState::Succeeded),
        "{projected:?}"
    );
    let Message::Response { result } = projected.payload else {
        panic!("listing")
    };
    let pending = result["projects"][0]["pending_approvals"]
        .as_array()
        .unwrap();
    assert_eq!(pending.len(), 2);
    assert!(
        pending
            .iter()
            .all(|value| value["subtab_id"] == initial.active_id)
    );
    assert_eq!(
        pending[0]["session_id"],
        answer.session_id.as_deref().unwrap()
    );
    assert!(!result.to_string().contains("PRIVATE_MAILBOX_WORKER_BODY"));
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    assert_eq!(
        adapter
            .dispatch(&registry, &reader, list.clone())
            .execution_state,
        Some(ExecutionState::Succeeded)
    );
    let ungranted = device(
        &registry,
        vec![],
        vec![Capability::Read, Capability::Approval],
    );
    let hidden = adapter.dispatch(&registry, &ungranted, list.clone());
    assert!(
        matches!(hidden.payload, Message::Response { result } if result["projects"].as_array().unwrap().is_empty())
    );
    let before = std::fs::read(root.join("project-workspace.json")).unwrap();
    let mut invalid = answer.clone();
    if let Message::Request { arguments, .. } = &mut invalid.payload {
        arguments["worker_session_id"] = Value::Null;
    }
    assert_eq!(
        adapter.dispatch(&registry, &phone, invalid).code,
        Some(ErrorCode::InvalidRequest)
    );
    let mut master_target = answer.clone();
    if let Message::Request { arguments, .. } = &mut master_target.payload {
        arguments
            .as_object_mut()
            .unwrap()
            .remove("worker_session_id");
    }
    assert_eq!(
        adapter.dispatch(&registry, &phone, master_target).code,
        Some(ErrorCode::InvalidRequest)
    );
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        before
    );
    assert!(
        coordinators
            .iter()
            .all(|coordinator| coordinator.is_pending("same-mailbox-approval"))
    );
    assert_eq!(
        adapter
            .dispatch(&registry, &controller, answer.clone())
            .code,
        Some(ErrorCode::Unauthorized)
    );
    let guard = workers[0].agent().lock().unwrap();
    let receipt = adapter.dispatch(&registry, &phone, answer.clone());
    assert_eq!(receipt.execution_state, Some(ExecutionState::Accepted));
    assert_eq!(adapter.dispatch(&registry, &phone, answer.clone()), receipt);
    assert!(coordinators[1].is_pending("same-mailbox-approval"));
    let mut changed = answer.clone();
    if let Message::Request { arguments, .. } = &mut changed.payload {
        arguments["worker_session_id"] = json!(sessions[1]);
    }
    assert_eq!(
        adapter.dispatch(&registry, &phone, changed).code,
        Some(ErrorCode::RequestIdConflict)
    );
    drop(guard);
    assert_eq!(
        joins.remove(0).join().unwrap().unwrap().decision,
        ApprovalDecision::Deny
    );
    coordinators[0]
        .mark_persisted("same-mailbox-approval")
        .unwrap();
    let remaining = adapter.dispatch(&registry, &phone, list);
    assert!(matches!(remaining.payload, Message::Response { result }
        if result["projects"][0]["pending_approvals"].as_array().unwrap().len() == 1
        && result["projects"][0]["pending_approvals"][0]["worker_session_id"] == sessions[1]));
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        adapter.dispatch(&registry, &phone, answer).code,
        Some(ErrorCode::Unauthorized)
    );
    stop.store(true, std::sync::atomic::Ordering::Release);
    let _ = joins.remove(0).join().unwrap();
    runtime.shutdown();
}

#[test]
fn headless_worker_prompt_is_lazy_durable_and_restart_queries_never_create_workers() {
    use zenpi::remote::{headless_host::HeadlessHost, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let master = host.active();
    let project = host.active_context().project_id.clone();
    let state = host.subtab_state(&project).unwrap();
    host.commit_subtabs(
        &project,
        state.revision,
        state.rename(&state.active_id, "persisted-main").unwrap(),
    )
    .unwrap();
    let state = host.subtab_state(&project).unwrap();
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let req = request(
        &host,
        Operation::Prompt,
        &project,
        "production-worker-once",
        json!({
            "text":"worker-main-once","subtab_id":state.active_id,"subtab_revision":state.revision
        }),
    );
    let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
    let mut listing = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::WorkspaceList,
            arguments: json!({}),
        },
    );
    listing.id = Some("worker-listing".into());
    let (adapter, runtime) = HeadlessHost::start(host);
    assert_eq!(
        adapter.dispatch(&registry, &reader, req.clone()).code,
        Some(ErrorCode::Unauthorized)
    );
    assert!(!root.join(".zenpi/workers").exists());
    let mut stale = req.clone();
    stale.expected_revision = Some(999);
    assert_eq!(
        adapter.dispatch(&registry, &phone, stale).code,
        Some(ErrorCode::RevisionConflict)
    );
    let mut unsafe_request = req.clone();
    if let Message::Request { arguments, .. } = &mut unsafe_request.payload {
        arguments["text"] = json!("!touch forbidden");
    }
    assert_eq!(
        adapter.dispatch(&registry, &phone, unsafe_request).code,
        Some(ErrorCode::InvalidRequest)
    );
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        checkpoint
    );
    assert!(!root.join(".zenpi/workers").exists());
    let accepted = adapter.dispatch(&registry, &phone, req.clone());
    assert_eq!(
        accepted.execution_state,
        Some(ExecutionState::Accepted),
        "{accepted:?}"
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let response = adapter.dispatch(&registry, &phone, listing.clone());
        assert!(matches!(&response.payload, Message::Response { result }
            if result["projects"][0]["worker_control"] == true));
        if matches!(&response.payload, Message::Response { result }
            if result["projects"][0]["worker_statuses"].as_array().is_some_and(|values|
                values.iter().any(|value| value["request_id"] == "production-worker-once" && value["state"] == "succeeded"
                    && value["output_text"].as_str().is_some_and(|text| text.contains("worker-main-once")))))
        {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "{response:?}");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(master.lock().unwrap().session().turns().is_empty());
    let readonly_listing = adapter.dispatch(&registry, &reader, listing.clone());
    assert!(
        matches!(readonly_listing.payload, Message::Response { result }
        if result["projects"][0]["worker_control"] == false
        && result["projects"][0]["worker_statuses"].as_array().unwrap().is_empty())
    );
    assert_eq!(adapter.dispatch(&registry, &phone, req.clone()), accepted);
    runtime.shutdown();
    let worker_dir = root.join(".zenpi/workers");
    let before: std::collections::BTreeMap<_, _> = std::fs::read_dir(&worker_dir)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (path.clone(), std::fs::read(path).unwrap())
        })
        .collect();
    assert_eq!(
        before
            .values()
            .filter(|bytes| String::from_utf8_lossy(bytes).contains("worker-main-once"))
            .count(),
        1
    );
    drop(master);
    let (adapter, runtime) = HeadlessHost::start(pool(&root));
    assert_eq!(adapter.dispatch(&registry, &phone, req.clone()), accepted);
    let after: std::collections::BTreeMap<_, _> = std::fs::read_dir(&worker_dir)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (path.clone(), std::fs::read(path).unwrap())
        })
        .collect();
    assert_eq!(
        before, after,
        "receipt query must not prepare worker/session startup"
    );
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        adapter.dispatch(&registry, &phone, req).code,
        Some(ErrorCode::Unauthorized)
    );
    runtime.shutdown();
}

#[test]
fn headless_worker_shutdown_retains_terminal_status_without_replaying_turns() {
    use zenpi::remote::{headless_host::HeadlessHost, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let initial = host.subtab_state(&project).unwrap();
    host.commit_subtabs(
        &project,
        initial.revision,
        initial.set_concurrency(&initial.active_id, 2).unwrap(),
    )
    .unwrap();
    let state = host.subtab_state(&project).unwrap();
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let prompts: Vec<_> = (0..3).map(|index| request(&host, Operation::Prompt, &project, &format!("headless-slot-{index}"), json!({
        "text":format!("mailbox-slot-{index}"),"subtab_id":state.active_id,"subtab_revision":state.revision
    }))).collect();
    let mut listing = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::WorkspaceList,
            arguments: json!({}),
        },
    );
    listing.id = Some("lane-status-list".into());
    let (adapter, runtime) = HeadlessHost::start(host);
    let accepted = adapter.dispatch(&registry, &phone, prompts[0].clone());
    assert_eq!(accepted.execution_state, Some(ExecutionState::Accepted));
    assert_eq!(
        adapter.dispatch(&registry, &phone, prompts[0].clone()),
        accepted
    );
    let second = adapter.dispatch(&registry, &phone, prompts[1].clone());
    assert_eq!(second.execution_state, Some(ExecutionState::Accepted));
    let third = adapter.dispatch(&registry, &phone, prompts[2].clone());
    assert_eq!(third.execution_state, Some(ExecutionState::Accepted));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let response = adapter.dispatch(&registry, &phone, listing.clone());
        if matches!(&response.payload, Message::Response { result }
            if result["projects"][0]["worker_statuses"].as_array().is_some_and(|values|
                (0..3).all(|index| values.iter().any(|value| value["request_id"] == format!("headless-slot-{index}")
                    && value["state"] == "succeeded"))))
        {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "{response:?}");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    runtime.shutdown();
    let (adapter, runtime) = HeadlessHost::start(pool(&root));
    assert_eq!(
        adapter.dispatch(&registry, &phone, prompts[0].clone()),
        accepted
    );
    assert_eq!(
        adapter.dispatch(&registry, &phone, prompts[1].clone()),
        second
    );
    assert_eq!(
        adapter.dispatch(&registry, &phone, prompts[2].clone()),
        third
    );
    runtime.shutdown();
}

#[test]
fn desktop_remote_subtab_selection_updates_the_same_view_and_does_not_use_indices() {
    use zenpi::tui::{ProjectRuntimeHost, ProjectTabMetadata, TuiState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let session = SessionStore::open_in_workspace(root.join("initial.jsonl"), &root).unwrap();
    let mut view = TuiState::default();
    view.set_active_project_metadata(ProjectTabMetadata::from_session(&session));
    let owner = Arc::new(Mutex::new(Agent::with_echo(session)));
    let mut desktop = ProjectRuntimeHost::new(&mut view, owner).unwrap();
    view.subtab_add_in_place(Some("one".into()));
    view.subtab_add_in_place(Some("two".into()));
    desktop.restore_shared_workspace(&mut view).unwrap();
    let project = view.active_project().to_owned();
    let state = desktop.host_subtab_state(&project).unwrap();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let (owner, revision) = desktop.remote_workspace_identity();
    let mut req = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::SubtabSelect,
            arguments: json!({"subtab_id":state.tabs[1].id,"subtab_revision":state.revision}),
        },
    );
    req.id = Some("desktop-select".into());
    req.project_id = Some(project.clone());
    req.session_id = Some(
        desktop
            .active(&view)
            .lock()
            .unwrap()
            .session()
            .session_id()
            .to_owned(),
    );
    req.required_owner = Some(owner.into());
    req.expected_revision = Some(revision);
    let admitted = [project.clone()].into();
    assert_eq!(
        desktop
            .dispatch_remote(&mut view, &registry, &phone, req.clone(), &admitted)
            .code,
        Some(ErrorCode::Busy)
    );
    let response = desktop.dispatch_remote(
        &mut view,
        &registry,
        &phone,
        req.clone(),
        &Default::default(),
    );
    assert_eq!(
        response.execution_state,
        Some(zenpi::remote::protocol::ExecutionState::Succeeded)
    );
    assert_eq!(view.active_subtab(), 1);
    let after = desktop.host_subtab_state(&project).unwrap();
    desktop.sync_subtab_view(&mut view).unwrap();
    assert_eq!(desktop.host_subtab_state(&project).unwrap(), after);
    assert_eq!(
        desktop
            .dispatch_remote(&mut view, &registry, &phone, req, &Default::default())
            .payload,
        response.payload
    );
}

#[test]
fn desktop_local_cancel_routes_without_phone_identity_or_rewriting_admission() {
    use zenpi::{
        remote::protocol::ExecutionState,
        tui::{ProjectRuntimeHost, ProjectTabMetadata, TuiState},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let owner = Arc::new(Mutex::new(Agent::with_echo(
        SessionStore::open_in_workspace(root.join("local-cancel.jsonl"), &root).unwrap(),
    )));
    let mut view = TuiState::default();
    view.set_active_project_metadata(ProjectTabMetadata::from_session(
        owner.lock().unwrap().session(),
    ));
    let mut desktop = ProjectRuntimeHost::new(&mut view, owner.clone()).unwrap();
    desktop.restore_shared_workspace(&mut view).unwrap();
    let project = view.active_project().to_owned();
    assert!(!desktop.dispatch_local_worktree_cancel(&mut view, "/cancel"));
    assert!(!desktop.dispatch_local_worktree_status(&mut view, "/status"));
    assert!(!desktop.cancel_active_worktree_tasks(&mut view));
    let tabs = desktop.host_subtab_state(&project).unwrap();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let (identity, revision) = desktop.remote_workspace_identity();
    let mut prompt = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::Prompt,
            arguments: json!({"text":"local interrupt race", "subtab_id":tabs.active_id,"subtab_revision":tabs.revision}),
        },
    );
    prompt.id = Some("phone-task".into());
    prompt.project_id = Some(project.clone());
    prompt.session_id = Some(owner.lock().unwrap().session().session_id().into());
    prompt.required_owner = Some(identity.into());
    prompt.expected_revision = Some(revision);
    let response = desktop.dispatch_remote(
        &mut view,
        &registry,
        &phone,
        prompt.clone(),
        &Default::default(),
    );
    assert_eq!(response.execution_state, Some(ExecutionState::Accepted));
    let mut immediate = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::WorkspaceList,
            arguments: json!({}),
        },
    );
    immediate.id = Some("immediate-task-list".into());
    let listing =
        desktop.dispatch_remote(&mut view, &registry, &phone, immediate, &Default::default());
    assert!(
        matches!(&listing.payload, Message::Response { result } if result["projects"][0]["worker_statuses"].as_array().unwrap().iter()
        .any(|task| task["request_id"] == "phone-task" && task["state"] == "accepted" && task["runtime_active"] == true)),
        "tracked admission must be visible before the first controller poll: {listing:?}"
    );
    view.set_input("DESKTOP_DRAFT_RETAINED");
    assert!(desktop.dispatch_local_worktree_status(&mut view, " /status "));
    let status = view.messages().last().unwrap().text.clone();
    assert!(status.contains("phone-task: accepted (runtime active)"));
    assert!(!status.contains("local interrupt race"));
    assert_eq!(view.input(), "DESKTOP_DRAFT_RETAINED");
    assert!(!desktop.dispatch_local_worktree_status(&mut view, "/status extra"));
    let held = owner.lock().unwrap();
    assert!(desktop.dispatch_local_worktree_status(&mut view, "/status"));
    assert!(view.status().contains("unavailable"));
    drop(held);
    for input in [
        "ordinary input",
        "/cancel unexpected",
        "/cancellation",
        "/cancel; anything",
        "/status",
    ] {
        assert!(!desktop.dispatch_local_worktree_cancel(&mut view, input));
    }
    assert!(view.subtab_add_in_place(Some("uncommitted-selection".into())));
    assert!(desktop.dispatch_local_worktree_status(&mut view, "/status"));
    assert!(view.status().contains("unavailable"));
    assert!(desktop.cancel_active_worktree_tasks(&mut view));
    assert!(view.status().contains("unavailable"));
    assert!(view.subtab_select(0));
    assert!(desktop.dispatch_local_worktree_cancel(&mut view, " /cancel "));
    assert!(view.status().contains("waiting for task exit"));
    assert!(desktop.remote_workers_busy(&project));
    assert_eq!(view.input(), "DESKTOP_DRAFT_RETAINED");
    assert_eq!(
        desktop.dispatch_remote(
            &mut view,
            &registry,
            &phone,
            prompt.clone(),
            &Default::default()
        ),
        response
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while desktop.remote_workers_busy(&project) {
        desktop.poll_remote_workers();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(!desktop.cancel_active_worktree_tasks(&mut view));
    assert!(owner.lock().unwrap().session().turns().is_empty());
    // A tiny Echo turn may win the completion race; neither outcome is rewritten locally.
    let events = owner
        .lock()
        .unwrap()
        .session()
        .events()
        .iter()
        .filter(|event| {
            event["type"] == "remote_worker_progress_v1" && event["request_id"] == "phone-task"
        })
        .cloned()
        .collect::<Vec<_>>();
    let last = events.last().unwrap();
    assert!(matches!(
        last["state"].as_str(),
        Some("cancelled" | "succeeded")
    ));
    assert!(desktop.dispatch_local_worktree_status(&mut view, "/status"));
    let terminal = view.messages().last().unwrap().text.clone();
    assert!(terminal.contains(&format!("phone-task: {}", last["state"].as_str().unwrap())));
    assert!(!terminal.contains("runtime active"));
    drop(desktop);
    let mut restored = TuiState::default();
    restored.set_active_project_metadata(ProjectTabMetadata::from_session(
        owner.lock().unwrap().session(),
    ));
    let mut desktop = ProjectRuntimeHost::new(&mut restored, owner.clone()).unwrap();
    desktop.restore_shared_workspace(&mut restored).unwrap();
    let journal_path = owner.lock().unwrap().session().path().to_owned();
    let before = std::fs::read(&journal_path).unwrap();
    assert!(desktop.dispatch_local_worktree_status(&mut restored, "/status"));
    assert_eq!(restored.messages().last().unwrap().text, terminal);
    assert_eq!(std::fs::read(&journal_path).unwrap(), before);
    assert!(!desktop.remote_workers_busy(&project));
    assert_eq!(
        desktop.dispatch_remote(
            &mut restored,
            &registry,
            &phone,
            prompt,
            &Default::default()
        ),
        response
    );
}

#[test]
fn desktop_worker_route_executes_independent_turn_and_retries_across_headless_restart() {
    use zenpi::{
        remote::{headless_host::HeadlessHost, protocol::ExecutionState},
        tui::{ProjectRuntimeHost, ProjectTabMetadata, TuiState},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("initial.jsonl");
    let owner = Arc::new(Mutex::new(Agent::with_echo(
        SessionStore::open_in_workspace(&path, &root).unwrap(),
    )));
    let mut view = TuiState::default();
    view.set_active_project_metadata(ProjectTabMetadata::from_session(
        owner.lock().unwrap().session(),
    ));
    let mut desktop = ProjectRuntimeHost::new(&mut view, owner.clone()).unwrap();
    desktop.restore_shared_workspace(&mut view).unwrap();
    view.set_input("LOCAL_DRAFT_NOT_A_WORKER_PROMPT");
    let project = view.active_project().to_owned();
    let state = desktop.host_subtab_state(&project).unwrap();
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let (required_owner, revision) = desktop.remote_workspace_identity();
    let mut quota = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::SetConcurrency,
            arguments: json!({"subtab_id":state.active_id,"subtab_revision":state.revision,"concurrency":2}),
        },
    );
    quota.id = Some("desktop-quota-two".into());
    quota.project_id = Some(project.clone());
    quota.session_id = Some(owner.lock().unwrap().session().session_id().into());
    quota.required_owner = Some(required_owner.into());
    quota.expected_revision = Some(revision);
    let no_admitted = std::collections::BTreeSet::new();
    assert_eq!(
        desktop
            .dispatch_remote(&mut view, &registry, &reader, quota.clone(), &no_admitted)
            .code,
        Some(ErrorCode::Unauthorized)
    );
    let admitted = std::collections::BTreeSet::from([project.clone()]);
    assert_eq!(
        desktop
            .dispatch_remote(&mut view, &registry, &phone, quota.clone(), &admitted)
            .code,
        Some(ErrorCode::Busy)
    );
    let quota_receipt =
        desktop.dispatch_remote(&mut view, &registry, &phone, quota.clone(), &no_admitted);
    assert_eq!(
        quota_receipt.execution_state,
        Some(ExecutionState::Succeeded),
        "{quota_receipt:?}"
    );
    let state = desktop.host_subtab_state(&project).unwrap();
    assert_eq!(state.tabs[0].concurrency, 2);
    assert_eq!(view.subtabs()[0].concurrency, 2);
    assert_eq!(
        desktop.dispatch_remote(&mut view, &registry, &phone, quota.clone(), &no_admitted),
        quota_receipt
    );
    let (required_owner, revision) = desktop.remote_workspace_identity();
    let mut request = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::Prompt,
            arguments: json!({"text":"desktop-worker-once", "subtab_id":state.active_id, "subtab_revision":state.revision}),
        },
    );
    request.id = Some("desktop-worker".into());
    request.project_id = Some(project.clone());
    request.session_id = Some(owner.lock().unwrap().session().session_id().into());
    request.required_owner = Some(required_owner.into());
    request.expected_revision = Some(revision);
    let no_admitted = std::collections::BTreeSet::new();
    assert_eq!(
        desktop
            .dispatch_remote(&mut view, &registry, &reader, request.clone(), &no_admitted)
            .code,
        Some(ErrorCode::Unauthorized)
    );
    assert!(!root.join(".zenpi/workers").exists());
    let admitted = std::collections::BTreeSet::from([project.clone()]);
    assert_eq!(
        desktop
            .dispatch_remote(&mut view, &registry, &phone, request.clone(), &admitted)
            .code,
        Some(ErrorCode::ResourceLimit)
    );
    assert!(!root.join(".zenpi/workers").exists());
    let accepted =
        desktop.dispatch_remote(&mut view, &registry, &phone, request.clone(), &no_admitted);
    assert_eq!(
        accepted.execution_state,
        Some(ExecutionState::Accepted),
        "{accepted:?}"
    );
    assert!(desktop.remote_workers_busy(&project));
    let current = desktop.host_subtab_state(&project).unwrap();
    let (quota_owner, quota_revision) = desktop.remote_workspace_identity();
    let mut live_quota = quota.clone();
    live_quota.id = Some("desktop-live-quota-three".into());
    live_quota.required_owner = Some(quota_owner.into());
    live_quota.expected_revision = Some(quota_revision);
    live_quota.payload = Message::Request {
        operation: Operation::SetConcurrency,
        arguments: json!({"subtab_id":current.active_id,"subtab_revision":current.revision,"concurrency":3}),
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let live_quota_receipt = loop {
        let value = desktop.dispatch_remote(
            &mut view,
            &registry,
            &phone,
            live_quota.clone(),
            &no_admitted,
        );
        if value.execution_state == Some(ExecutionState::Succeeded) {
            break value;
        }
        assert_eq!(
            value.execution_state,
            Some(ExecutionState::UnknownOutcome),
            "{value:?}"
        );
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    assert_eq!(
        desktop.host_subtab_state(&project).unwrap().tabs[0].concurrency,
        3
    );
    assert_eq!(view.subtabs()[0].concurrency, 3);
    assert_eq!(
        desktop.dispatch_remote(&mut view, &registry, &phone, request.clone(), &admitted),
        accepted
    );
    let mut list = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::WorkspaceList,
            arguments: json!({}),
        },
    );
    list.id = Some("desktop-worker-list".into());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        desktop.poll_remote_workers();
        let response =
            desktop.dispatch_remote(&mut view, &registry, &phone, list.clone(), &no_admitted);
        if matches!(&response.payload, Message::Response { result }
            if result["projects"][0]["worker_control"] == true && result["projects"][0]["worker_statuses"].as_array().unwrap().iter()
                .any(|value| value["request_id"] == "desktop-worker" && value["state"] == "succeeded"
                    && value["output_text"].as_str().is_some_and(|text| text.contains("desktop-worker-once"))))
        {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "{response:?}");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(!desktop.remote_workers_busy(&project));
    assert_eq!(view.input(), "LOCAL_DRAFT_NOT_A_WORKER_PROMPT");
    assert!(owner.lock().unwrap().session().turns().is_empty());
    let read = desktop.dispatch_remote(&mut view, &registry, &reader, list, &no_admitted);
    assert!(matches!(read.payload, Message::Response { result }
        if result["projects"][0]["worker_control"] == false && result["projects"][0]["worker_statuses"].as_array().unwrap().is_empty()));
    let before: std::collections::BTreeMap<_, _> = std::fs::read_dir(root.join(".zenpi/workers"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (path.clone(), std::fs::read(path).unwrap())
        })
        .collect();
    drop(desktop);
    drop(owner);
    let (adapter, runtime) = HeadlessHost::start(pool(&root));
    assert_eq!(adapter.dispatch(&registry, &phone, quota), quota_receipt);
    assert_eq!(
        adapter.dispatch(&registry, &phone, live_quota),
        live_quota_receipt
    );
    assert_eq!(
        adapter.dispatch(&registry, &phone, request.clone()),
        accepted
    );
    let after: std::collections::BTreeMap<_, _> = std::fs::read_dir(root.join(".zenpi/workers"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (path.clone(), std::fs::read(path).unwrap())
        })
        .collect();
    assert_eq!(before, after);
    runtime.shutdown();
    let owner = Arc::new(Mutex::new(Agent::with_echo(
        SessionStore::open_in_workspace(&path, &root).unwrap(),
    )));
    let mut restored_view = TuiState::default();
    restored_view.set_active_project_metadata(ProjectTabMetadata::from_session(
        owner.lock().unwrap().session(),
    ));
    let mut desktop = ProjectRuntimeHost::new(&mut restored_view, owner).unwrap();
    desktop
        .restore_shared_workspace(&mut restored_view)
        .unwrap();
    assert_eq!(
        desktop.dispatch_remote(
            &mut restored_view,
            &registry,
            &phone,
            request.clone(),
            &no_admitted
        ),
        accepted
    );
    assert!(!desktop.remote_workers_busy(&project));
    let restored_files: std::collections::BTreeMap<_, _> =
        std::fs::read_dir(root.join(".zenpi/workers"))
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                (path.clone(), std::fs::read(path).unwrap())
            })
            .collect();
    assert_eq!(before, restored_files);
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        desktop
            .dispatch_remote(&mut restored_view, &registry, &phone, request, &no_admitted)
            .code,
        Some(ErrorCode::Unauthorized)
    );
}

#[test]
fn remote_concurrency_is_durable_cas_fenced_and_drives_real_worker_quota() {
    use zenpi::{
        remote::{
            host_adapter::HostAdapter, protocol::ExecutionState, worktree_runner::WorktreeRunner,
        },
        runtime::{RuntimeConfig, RuntimeEvent},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let initial = host.subtab_state(&project).unwrap();
    host.commit_subtabs(
        &project,
        initial.revision,
        initial
            .rename(&initial.active_id, "persisted-main")
            .unwrap(),
    )
    .unwrap();
    let initial = host.subtab_state(&project).unwrap();
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let req = request(
        &host,
        Operation::SetConcurrency,
        &project,
        "quota-three",
        json!({"subtab_id":initial.active_id,"subtab_revision":initial.revision,"concurrency":3}),
    );
    let listing = request(
        &host,
        Operation::SubtabList,
        &project,
        "quota-capability",
        json!({}),
    );
    for (device, enabled) in [(&phone, true), (&reader, false)] {
        let response = HostAdapter::dispatch_owned(&mut host, &registry, device, listing.clone());
        assert!(
            matches!(response.payload, Message::Response { result } if result["concurrency_control"] == enabled)
        );
    }
    let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &reader, req.clone()).code,
        Some(ErrorCode::Unauthorized)
    );
    for invalid in [json!(0), json!(65), json!(-1), json!(1.0), json!("2")] {
        let mut invalid_req = req.clone();
        if let Message::Request { arguments, .. } = &mut invalid_req.payload {
            arguments["concurrency"] = invalid;
        }
        assert_eq!(
            HostAdapter::dispatch_owned(&mut host, &registry, &phone, invalid_req).code,
            Some(ErrorCode::InvalidRequest)
        );
    }
    let mut stale = req.clone();
    stale.expected_revision = Some(999);
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &phone, stale).code,
        Some(ErrorCode::RevisionConflict)
    );
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        checkpoint
    );
    let receipt = HostAdapter::dispatch_owned(&mut host, &registry, &phone, req.clone());
    assert_eq!(
        receipt.execution_state,
        Some(ExecutionState::Succeeded),
        "{receipt:?}"
    );
    assert_eq!(host.subtab_state(&project).unwrap().tabs[0].concurrency, 3);
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &phone, req.clone()),
        receipt
    );
    let mut changed = req.clone();
    if let Message::Request { arguments, .. } = &mut changed.payload {
        arguments["concurrency"] = json!(2);
    }
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &phone, changed).code,
        Some(ErrorCode::RequestIdConflict)
    );
    let entered = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let started = entered.clone();
    let release = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let released = release.clone();
    let runner = WorktreeRunner::spawn(
        &mut host,
        &project,
        &initial.active_id,
        RuntimeConfig::default(),
        move |_, _: usize, token| {
            started.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while !released.load(std::sync::atomic::Ordering::Acquire) && !token.is_cancelled() {
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Ok::<_, String>(())
        },
    )
    .unwrap();
    assert_eq!(runner.quota(), 3);
    for index in 0..4 {
        runner.try_submit(index).unwrap();
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while entered.load(std::sync::atomic::Ordering::Acquire) < 3 {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    std::thread::sleep(std::time::Duration::from_millis(30));
    assert_eq!(
        entered.load(std::sync::atomic::Ordering::Acquire),
        3,
        "N+1 must remain queued"
    );
    let state = host.subtab_state(&project).unwrap();
    let busy = request(
        &host,
        Operation::SetConcurrency,
        &project,
        "busy-quota",
        json!({"subtab_id":state.active_id,"subtab_revision":state.revision,"concurrency":2}),
    );
    let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &phone, busy.clone()).code,
        Some(ErrorCode::Busy)
    );
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        checkpoint
    );
    release.store(true, std::sync::atomic::Ordering::Release);
    let mut finished = 0;
    while finished < 4 {
        if matches!(
            runner
                .recv_timeout(std::time::Duration::from_secs(3))
                .unwrap(),
            RuntimeEvent::Completed { .. }
        ) {
            finished += 1;
        }
    }
    runner.shutdown_and_join().unwrap();
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &phone, busy).execution_state,
        Some(ExecutionState::Succeeded)
    );
    drop(host);
    let mut host = pool(&root);
    assert_eq!(host.subtab_state(&project).unwrap().tabs[0].concurrency, 2);
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &phone, req.clone()),
        receipt
    );
    assert_eq!(
        host.subtab_state(&project).unwrap().tabs[0].concurrency,
        2,
        "old receipt cannot roll back newer quota"
    );
    let (adapter, mailbox) = HostAdapter::owner_thread();
    std::thread::scope(|scope| {
        let pending = scope.spawn(|| adapter.dispatch(&registry, &phone, req.clone()));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !pending.is_finished() && std::time::Instant::now() < deadline {
            mailbox.drain(&mut host);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(pending.join().unwrap(), receipt);
    });
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &phone, req).code,
        Some(ErrorCode::Unauthorized)
    );
}

#[test]
fn quota_crash_intent_is_unknown_on_remote_retry_and_local_cli_commits_without_execution() {
    use zenpi::remote::{host_adapter::HostAdapter, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut host = pool(&root);
    let project = host.active_context().project_id.clone();
    let before = host.subtab_state(&project).unwrap();
    let candidate = before.set_concurrency(&before.active_id, 3).unwrap();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![project.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let reader = device(&registry, vec![project.clone()], vec![Capability::Read]);
    let req = request(
        &host,
        Operation::SetConcurrency,
        &project,
        "quota-crash",
        json!({"subtab_id":before.active_id,"subtab_revision":before.revision,"concurrency":3}),
    );
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &phone, req.clone()).execution_state,
        Some(ExecutionState::Succeeded)
    );
    let path = root.join("project-workspace.json");
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    // Reconstruct the exact persisted pre-application crash boundary.
    saved["subtabs"][&project] = json!(before);
    saved["remote_workspace"]["revision"] = json!(req.expected_revision.unwrap());
    saved["remote_workspace"]["receipts"][0]["result"] = Value::Null;
    saved["remote_workspace"]["receipts"][0]["quota_intent"] =
        json!({"request":req,"before":before,"candidate":candidate});
    drop(host);
    std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let mut host = pool(&root);
    assert_eq!(host.subtab_state(&project).unwrap(), before);
    let unknown = HostAdapter::dispatch_owned(&mut host, &registry, &phone, req.clone());
    assert_eq!(
        unknown.execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert!(matches!(unknown.payload, Message::Response { result } if result.is_null()));
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &reader, req.clone()).code,
        Some(ErrorCode::Unauthorized)
    );
    let mut changed = req.clone();
    if let Message::Request { arguments, .. } = &mut changed.payload {
        arguments["concurrency"] = json!(2);
    }
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &phone, changed).code,
        Some(ErrorCode::RequestIdConflict)
    );
    assert!(!root.join(".zenpi/workers").exists());
    drop(host);
    let config = root.join("cli-config");
    std::fs::create_dir(&config).unwrap();
    std::fs::write(config.join("config.toml"), "backend=\"openai\"\nmodel=\"gpt-4.1\"\nbase_url=\"http://127.0.0.1:9/v1\"\nwire_api=\"responses\"\nrequires_openai_auth=false\n").unwrap();
    let cli = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_zenpi"))
            .args([
                "host",
                "pair",
                "--workspace",
                root.to_str().unwrap(),
                "--session",
                root.join("initial.jsonl").to_str().unwrap(),
            ])
            .args(args)
            .env("ZENPI_HOME", &config)
            .output()
            .unwrap()
    };
    let list = cli(&["--list-quota-intents"]);
    assert!(
        list.status.success(),
        "{}",
        String::from_utf8_lossy(&list.stderr)
    );
    let values: Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(values.as_array().unwrap().len(), 1);
    assert_eq!(values[0]["request_id"], "quota-crash");
    assert_eq!(values[0]["concurrency"], 3);
    assert_eq!(values[0]["state"], "unknown_outcome");
    let repair = cli(&["--reconcile-quota", "quota-crash", "--device", phone.id()]);
    assert!(
        repair.status.success(),
        "{}",
        String::from_utf8_lossy(&repair.stderr)
    );
    let result: Value = serde_json::from_slice(&repair.stdout).unwrap();
    assert_eq!(
        result["required_owner"],
        req.required_owner.as_deref().unwrap()
    );
    let mut host = pool(&root);
    assert_eq!(host.subtab_state(&project).unwrap(), candidate);
    assert!(host.unresolved_quota_intents().is_empty());
    let retried = HostAdapter::dispatch_owned(&mut host, &registry, &phone, req.clone());
    assert_eq!(retried.execution_state, Some(ExecutionState::Succeeded));
    assert!(matches!(retried.payload, Message::Response { result: value } if value == result));
    assert!(!root.join(".zenpi/workers").exists());
    assert!(!config.join("remote/identity.json").exists());
    assert!(
        host.owner(&project)
            .unwrap()
            .lock()
            .unwrap()
            .session()
            .turns()
            .is_empty()
    );
    registry.revoke(phone.id()).unwrap();
    assert_eq!(
        HostAdapter::dispatch_owned(&mut host, &registry, &phone, req).code,
        Some(ErrorCode::Unauthorized)
    );
}
#[test]
fn canonical_open_select_close_commit_retry_receipts_with_workspace_and_survive_restart() {
    let temp = tempfile::tempdir().unwrap();
    let child = temp.path().join("child");
    std::fs::create_dir(&child).unwrap();
    let mut host = pool(temp.path());
    let initial = host.active_context().project_id.clone();
    let initial_signal = zenpi::remote::wakeup::watch(&host.active_context().session_id).unwrap();
    let observed = initial_signal.revision();
    let next = ProjectWorkspace::default()
        .with_directory(Some(&child), temp.path())
        .unwrap()
        .0
        .active()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let registry = DeviceRegistry::new(temp.path().join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![initial.clone(), next.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let open = request(
        &host,
        Operation::WorkspaceOpen,
        &initial,
        "open",
        json!({"cwd":"child"}),
    );
    let result = mutate(&mut host, &registry, &phone, &open).unwrap();
    assert_ne!(initial_signal.revision(), observed);
    let next_signal = zenpi::remote::wakeup::watch(&host.active_context().session_id).unwrap();
    let observed = (initial_signal.revision(), next_signal.revision());
    assert_eq!(host.workspace().tabs().len(), 2);
    assert_eq!(result["active_project_id"], next);
    assert_eq!(result["workspace_revision"], host.workspace_revision());
    let durable = std::fs::read(temp.path().join("project-workspace.json")).unwrap();
    assert_eq!(mutate(&mut host, &registry, &phone, &open).unwrap(), result);
    assert_eq!(
        (initial_signal.revision(), next_signal.revision()),
        observed
    );
    assert_eq!(
        std::fs::read(temp.path().join("project-workspace.json")).unwrap(),
        durable
    );
    let mut conflict = open.clone();
    conflict.expected_revision = Some(99);
    assert_eq!(
        mutate(&mut host, &registry, &phone, &conflict),
        Err(ErrorCode::RequestIdConflict)
    );
    assert_eq!(
        (initial_signal.revision(), next_signal.revision()),
        observed
    );
    let select = request(
        &host,
        Operation::WorkspaceSelect,
        &initial,
        "select",
        json!({}),
    );
    let local_owner = host.owner(&initial).unwrap();
    let held = local_owner.lock().unwrap();
    mutate(&mut host, &registry, &phone, &select).unwrap();
    assert_ne!(initial_signal.revision(), observed.0);
    assert_ne!(next_signal.revision(), observed.1);
    drop(held);
    assert_eq!(host.active_context().project_id, initial);
    let local_stale = request(
        &host,
        Operation::WorkspaceSelect,
        &initial,
        "local-stale",
        json!({}),
    );
    host.select(&next).unwrap();
    assert_eq!(
        mutate(&mut host, &registry, &phone, &local_stale),
        Err(ErrorCode::RevisionConflict)
    );
    host.select(&initial).unwrap();
    let close = request(&host, Operation::WorkspaceClose, &next, "close", json!({}));
    let observed = (initial_signal.revision(), next_signal.revision());
    let closed_owner = host.owner(&next).unwrap();
    let held = closed_owner.lock().unwrap();
    assert_eq!(
        mutate(&mut host, &registry, &phone, &close),
        Err(ErrorCode::Busy)
    );
    assert_eq!(
        (initial_signal.revision(), next_signal.revision()),
        observed
    );
    drop(held);
    let arch = host.arch_agent(&next).unwrap();
    let held_arch = arch.lock().unwrap();
    let before = std::fs::read(temp.path().join("project-workspace.json")).unwrap();
    assert_eq!(
        mutate(&mut host, &registry, &phone, &close),
        Err(ErrorCode::InternalError)
    );
    assert_eq!(
        std::fs::read(temp.path().join("project-workspace.json")).unwrap(),
        before
    );
    assert!(host.owner(&next).is_some());
    assert_eq!(
        (initial_signal.revision(), next_signal.revision()),
        observed
    );
    drop(held_arch);
    let closed = mutate(&mut host, &registry, &phone, &close).unwrap();
    assert_ne!(initial_signal.revision(), observed.0);
    assert_ne!(next_signal.revision(), observed.1);
    assert_eq!(
        closed_owner.lock().unwrap().phase(),
        zenpi::core::AgentPhase::Closed
    );
    assert!(host.owner(&next).is_none());
    assert_eq!(
        arch.lock().unwrap().phase(),
        zenpi::core::AgentPhase::Closed
    );
    let old_owner = host.workspace_owner().to_owned();
    let revision = host.workspace_revision();
    drop(host);
    let mut restarted = pool(temp.path());
    let observed = (initial_signal.revision(), next_signal.revision());
    assert_eq!(restarted.workspace_revision(), revision);
    assert_ne!(restarted.workspace_owner(), old_owner);
    assert_eq!(
        mutate(&mut restarted, &registry, &phone, &close).unwrap(),
        closed
    );
    assert_eq!(
        mutate(&mut restarted, &registry, &phone, &open).unwrap(),
        result
    );
    let mut stale = select.clone();
    stale.id = Some("fresh-id-old-owner".into());
    assert_eq!(
        mutate(&mut restarted, &registry, &phone, &stale),
        Err(ErrorCode::OwnerMismatch)
    );
    assert_eq!(
        (initial_signal.revision(), next_signal.revision()),
        observed
    );
    let restored = Arc::new(Mutex::new(restarted));
    let adapter = zenpi::remote::host_adapter::HostAdapter::new(restored.clone());
    let latest = request(
        &restored.lock().unwrap(),
        Operation::WorkspaceSelect,
        &initial,
        "adapter-select",
        json!({}),
    );
    assert_eq!(
        adapter
            .dispatch(&Arc::new(registry), &phone, latest)
            .execution_state,
        Some(zenpi::remote::protocol::ExecutionState::Succeeded)
    );
}
#[test]
fn deny_read_only_foreign_path_stale_revision_and_external_checkpoint_without_changes() {
    let temp = tempfile::tempdir().unwrap();
    let child = temp.path().join("child");
    std::fs::create_dir(&child).unwrap();
    let mut host = pool(temp.path());
    let initial = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(temp.path().join("devices.json"), "a".repeat(64)).unwrap();
    let read = device(&registry, vec![initial.clone()], vec![Capability::Read]);
    let control = device(
        &registry,
        vec![initial.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let req = request(
        &host,
        Operation::WorkspaceSelect,
        &initial,
        "select",
        json!({}),
    );
    assert_eq!(
        mutate(&mut host, &registry, &read, &req),
        Err(ErrorCode::Unauthorized)
    );
    let open = request(
        &host,
        Operation::WorkspaceOpen,
        &initial,
        "open",
        json!({"cwd":"child"}),
    );
    assert_eq!(
        mutate(&mut host, &registry, &control, &open),
        Err(ErrorCode::Unauthorized)
    );
    let mut stale = req.clone();
    stale.expected_revision = Some(100);
    assert_eq!(
        mutate(&mut host, &registry, &control, &stale),
        Err(ErrorCode::RevisionConflict)
    );
    let mut wrong = req.clone();
    wrong.session_id = Some("foreign".into());
    assert_eq!(
        mutate(&mut host, &registry, &control, &wrong),
        Err(ErrorCode::OwnerMismatch)
    );
    let checkpoint = temp.path().join("project-workspace.json");
    std::fs::write(&checkpoint, b"external host").unwrap();
    assert_eq!(
        mutate(&mut host, &registry, &control, &req),
        Err(ErrorCode::InternalError)
    );
    assert_eq!(std::fs::read(checkpoint).unwrap(), b"external host");
    assert_eq!(host.workspace_revision(), 0);
    assert_eq!(host.workspace().tabs().len(), 1);
    registry.revoke(control.id()).unwrap();
    assert_eq!(
        mutate(&mut host, &registry, &control, &req),
        Err(ErrorCode::Unauthorized)
    );
}

#[test]
fn retry_ledger_never_evicts_an_ambiguous_request_and_refuses_new_work_at_capacity() {
    let temp = tempfile::tempdir().unwrap();
    let mut host = pool(temp.path());
    let initial = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(temp.path().join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![initial.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let first = request(&host, Operation::WorkspaceSelect, &initial, "0", json!({}));
    let result = mutate(&mut host, &registry, &phone, &first).unwrap();
    for i in 1..128 {
        let next = request(
            &host,
            Operation::WorkspaceSelect,
            &initial,
            &i.to_string(),
            json!({}),
        );
        mutate(&mut host, &registry, &phone, &next).unwrap();
    }
    let overflow = request(
        &host,
        Operation::WorkspaceSelect,
        &initial,
        "overflow",
        json!({}),
    );
    let before = std::fs::read(temp.path().join("project-workspace.json")).unwrap();
    assert_eq!(
        mutate(&mut host, &registry, &phone, &overflow),
        Err(ErrorCode::ResourceLimit)
    );
    assert_eq!(
        mutate(&mut host, &registry, &phone, &first).unwrap(),
        result
    );
    assert_eq!(
        std::fs::read(temp.path().join("project-workspace.json")).unwrap(),
        before
    );
}

#[test]
fn concurrent_clients_have_one_cas_winner_and_no_second_commit() {
    let temp = tempfile::tempdir().unwrap();
    let host = Arc::new(Mutex::new(pool(temp.path())));
    let initial = host.lock().unwrap().active_context().project_id.clone();
    let registry =
        Arc::new(DeviceRegistry::new(temp.path().join("devices.json"), "a".repeat(64)).unwrap());
    let first = device(
        &registry,
        vec![initial.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let second = device(
        &registry,
        vec![initial.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let a = request(
        &host.lock().unwrap(),
        Operation::WorkspaceSelect,
        &initial,
        "a",
        json!({}),
    );
    let b = request(
        &host.lock().unwrap(),
        Operation::WorkspaceSelect,
        &initial,
        "b",
        json!({}),
    );
    let results = std::thread::scope(|scope| {
        let a = scope.spawn(|| mutate(&mut host.lock().unwrap(), &registry, &first, &a));
        let b = scope.spawn(|| mutate(&mut host.lock().unwrap(), &registry, &second, &b));
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| **result == Err(ErrorCode::RevisionConflict))
            .count(),
        1
    );
    assert_eq!(host.lock().unwrap().workspace_revision(), 1);
}

#[test]
fn legacy_checkpoint_migrates_on_commit_and_malformed_retry_ledger_is_refused() {
    let temp = tempfile::tempdir().unwrap();
    let mut host = pool(temp.path());
    let initial = host.active_context().project_id.clone();
    host.select(&initial).unwrap();
    let path = temp.path().join("project-workspace.json");
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    saved.as_object_mut().unwrap().remove("remote_workspace");
    drop(host);
    std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let mut migrated = pool(temp.path());
    assert_eq!(migrated.workspace_revision(), 0);
    migrated.select(&initial).unwrap();
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(saved["remote_workspace"]["receipts"].is_array());
    saved["remote_workspace"]["receipts"] =
        json!([{"device_id":"bad", "request_id":"id", "fingerprint":"bad", "result":{}}]);
    drop(migrated);
    std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let owner = Agent::with_echo(
        SessionStore::open_in_workspace(temp.path().join("initial.jsonl"), temp.path()).unwrap(),
    );
    let mut failed = ProjectOwnerPool::new(Arc::new(Mutex::new(owner))).unwrap();
    assert!(
        failed
            .restore_checkpoint()
            .unwrap_err()
            .contains("receipts")
    );
    assert_eq!(failed.workspace_revision(), 0);
}

#[test]
fn desktop_remote_control_reconciles_the_same_owner_and_preserves_other_project_drafts() {
    use zenpi::remote::protocol::ExecutionState;
    use zenpi::tui::{ProjectRuntimeHost, ProjectTabMetadata, TuiState};
    let temp = tempfile::tempdir().unwrap();
    let child = temp.path().join("child");
    std::fs::create_dir(&child).unwrap();
    let owner = Arc::new(Mutex::new(Agent::with_echo(
        SessionStore::open_in_workspace(temp.path().join("initial.jsonl"), temp.path()).unwrap(),
    )));
    let mut view = TuiState::default();
    view.set_active_project_metadata(ProjectTabMetadata::from_session(
        owner.lock().unwrap().session(),
    ));
    let mut desktop = ProjectRuntimeHost::new(&mut view, owner.clone()).unwrap();
    desktop.restore_shared_workspace(&mut view).unwrap();
    let initial = view.active_project().to_owned();
    view.set_input("initial unsent draft");
    let next = ProjectWorkspace::default()
        .with_directory(Some(&child), temp.path())
        .unwrap()
        .0
        .active()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let registry = DeviceRegistry::new(temp.path().join("devices.json"), "a".repeat(64)).unwrap();
    let phone = device(
        &registry,
        vec![initial.clone(), next.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let make = |desktop: &ProjectRuntimeHost,
                id: &str,
                session: &str,
                operation: Operation,
                name: &str,
                arguments| {
        let mut request = Envelope::new(
            Channel::Control,
            Message::Request {
                operation,
                arguments,
            },
        );
        let (identity, revision) = desktop.remote_workspace_identity();
        request.id = Some(name.into());
        request.project_id = Some(id.into());
        request.session_id = Some(session.into());
        request.required_owner = Some(identity.into());
        request.expected_revision = Some(revision);
        request
    };
    let initial_session = owner.lock().unwrap().session().session_id().to_owned();
    let open = make(
        &desktop,
        &initial,
        &initial_session,
        Operation::WorkspaceOpen,
        "open",
        json!({"cwd":"child"}),
    );
    assert_eq!(
        desktop
            .dispatch_remote(&mut view, &registry, &phone, open, &Default::default())
            .execution_state,
        Some(ExecutionState::Succeeded)
    );
    assert_eq!(view.active_project(), next);
    assert_eq!(view.project_tab_count(), 2);
    assert_eq!(view.input(), "");
    view.set_input("child unsent draft");
    let child_owner = desktop.active(&view);
    let child_session = child_owner
        .lock()
        .unwrap()
        .session()
        .session_id()
        .to_owned();
    let select = make(
        &desktop,
        &initial,
        &initial_session,
        Operation::WorkspaceSelect,
        "select",
        json!({}),
    );
    let held = owner.lock().unwrap();
    assert_eq!(
        desktop
            .dispatch_remote(&mut view, &registry, &phone, select, &Default::default())
            .execution_state,
        Some(ExecutionState::Succeeded)
    );
    drop(held);
    assert_eq!(view.input(), "initial unsent draft");
    assert!(Arc::ptr_eq(&desktop.active(&view), &owner));
    let close = make(
        &desktop,
        &next,
        &child_session,
        Operation::WorkspaceClose,
        "close",
        json!({}),
    );
    let checkpoint = view.project_checkpoint();
    assert_eq!(
        desktop
            .dispatch_remote(
                &mut view,
                &registry,
                &phone,
                close.clone(),
                &[next.clone()].into()
            )
            .code,
        Some(ErrorCode::Busy)
    );
    assert_eq!(view.project_checkpoint(), checkpoint);
    assert_eq!(
        desktop
            .dispatch_remote(
                &mut view,
                &registry,
                &phone,
                close.clone(),
                &Default::default()
            )
            .execution_state,
        Some(ExecutionState::Succeeded)
    );
    assert_eq!(view.project_tab_count(), 1);
    assert_eq!(view.input(), "initial unsent draft");
    assert_eq!(
        child_owner.lock().unwrap().phase(),
        zenpi::core::AgentPhase::Closed
    );
    let committed = view.project_checkpoint();
    assert_eq!(
        desktop
            .dispatch_remote(&mut view, &registry, &phone, close, &Default::default())
            .execution_state,
        Some(ExecutionState::Succeeded)
    );
    assert_eq!(view.project_checkpoint(), committed);
}

#[test]
fn mailbox_expires_unstarted_writes_and_marks_started_timeout_as_unknown_not_failed() {
    use std::time::{Duration, Instant};
    use zenpi::remote::{host_adapter::HostAdapter, protocol::ExecutionState};
    let temp = tempfile::tempdir().unwrap();
    let mut host = pool(temp.path());
    let initial = host.active_context().project_id.clone();
    let registry =
        Arc::new(DeviceRegistry::new(temp.path().join("devices.json"), "a".repeat(64)).unwrap());
    let phone = device(
        &registry,
        vec![initial.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let (adapter, mailbox) = HostAdapter::owner_thread();
    let expired = request(
        &host,
        Operation::WorkspaceSelect,
        &initial,
        "expired",
        json!({}),
    );
    assert_eq!(
        adapter.dispatch(&registry, &phone, expired).execution_state,
        Some(ExecutionState::Failed)
    );
    let mut ran = false;
    mailbox.drain_with(|registry, phone, request| {
        ran = true;
        HostAdapter::dispatch_owned(&mut host, registry, phone, request)
    });
    assert!(!ran);
    assert_eq!(host.workspace_revision(), 0);
    let started = request(
        &host,
        Operation::WorkspaceSelect,
        &initial,
        "started",
        json!({}),
    );
    std::thread::scope(|scope| {
        let pending = scope.spawn(|| adapter.dispatch(&registry, &phone, started.clone()));
        let until = Instant::now() + Duration::from_secs(1);
        while !ran && Instant::now() < until {
            mailbox.drain_with(|registry, phone, request| {
                ran = true;
                std::thread::sleep(Duration::from_millis(2200));
                HostAdapter::dispatch_owned(&mut host, registry, phone, request)
            });
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            pending.join().unwrap().execution_state,
            Some(ExecutionState::UnknownOutcome)
        );
    });
    assert_eq!(host.workspace_revision(), 1);
    let replay = mutate(&mut host, &registry, &phone, &started).unwrap();
    assert_eq!(replay["workspace_revision"], 1);
}
