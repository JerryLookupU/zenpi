use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
use serde_json::json;
use std::{
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
};
use zenpi::{
    core::Agent,
    project_workspace::ProjectOwnerPool,
    remote::{
        auth::{Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message},
        host_adapter::HostAdapter,
        protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message, Operation},
    },
    session::SessionStore,
};

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(args)
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn read_only_diff_uses_exact_selected_root_and_refuses_stale_or_unauthorized_routes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    git(&root, &["init", "-q"]);
    std::fs::write(root.join("outside.txt"), "outside before\n").unwrap();
    std::fs::write(child.join("file.txt"), "before\n").unwrap();
    let private_paths = [
        "credentials.json",
        "AUTH.JSON",
        "private.pem",
        "PRIVATE.KEY",
        "id_rsa",
        ".env.local",
        "nested/credentials/opaque.txt",
    ];
    std::fs::create_dir_all(child.join("nested/credentials")).unwrap();
    for path in private_paths {
        std::fs::write(child.join(path), "OPAQUE_PRIVATE_BEFORE\n").unwrap();
    }
    std::fs::write(child.join("deleted.p12"), "OPAQUE_PRIVATE_DELETED\n").unwrap();
    std::fs::write(child.join("rename.key"), "OPAQUE_PRIVATE_RENAMED\n").unwrap();
    git(&root, &["add", "."]);
    git(
        &root,
        &[
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "user.name=Fixture",
            "commit",
            "-qm",
            "seed",
        ],
    );
    std::fs::write(root.join("outside.txt"), "OUTSIDE_DIFF_MUST_NOT_LEAK\n").unwrap();
    std::fs::write(child.join("file.txt"), "SELECTED_DIFF\n").unwrap();
    std::fs::write(child.join("new.txt"), "SELECTED_UNTRACKED\n").unwrap();
    for path in private_paths {
        std::fs::write(child.join(path), "OPAQUE_PRIVATE_DIFF_CONTENT\n").unwrap();
    }
    std::fs::write(child.join("untracked.p12"), "OPAQUE_PRIVATE_UNTRACKED\n").unwrap();
    std::fs::remove_file(child.join("deleted.p12")).unwrap();
    git(&root, &["mv", "child/rename.key", "child/renamed.key"]);
    git(&root, &["config", "diff.renames", "copies"]);
    git(&root, &["config", "status.renames", "copies"]);
    std::fs::write(
        child.join("nested/credentials/new.txt"),
        "OPAQUE_PRIVATE_NESTED_UNTRACKED\n",
    )
    .unwrap();
    let owner = Arc::new(Mutex::new(Agent::with_echo(
        SessionStore::open_in_workspace(temp.path().join("session.jsonl"), &root).unwrap(),
    )));
    let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
    pool.restore_checkpoint().unwrap();
    let context = pool.active_context().clone();
    let initial = pool.subtab_state(&context.project_id).unwrap();
    let selected = initial
        .add_directory(&context.project_id, &root, &child, "child")
        .unwrap();
    pool.commit_subtabs(&context.project_id, initial.revision, selected.clone())
        .unwrap();
    let checkpoint = temp.path().join("project-workspace.json");
    let bytes = std::fs::read(&checkpoint).unwrap();
    let journal = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    let registry =
        Arc::new(DeviceRegistry::new(temp.path().join("devices.json"), "a".repeat(64)).unwrap());
    let random = ring::rand::SystemRandom::new();
    let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random).unwrap();
    let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    let sign =
        |message: &[u8]| URL_SAFE_NO_PAD.encode(key.sign(&random, message).unwrap().as_ref());
    let record = registry
        .verify_pair_and_register(
            "token",
            "pair",
            "phone",
            &public,
            &sign(&pairing_proof_message(
                &"a".repeat(64),
                "token",
                "pair",
                "phone",
                &public,
            )),
            || true,
        )
        .unwrap();
    registry
        .set_permissions(
            &record.id,
            record.revision,
            vec![Capability::Read],
            vec![context.project_id.clone()],
        )
        .unwrap();
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let device = challenge
        .authenticate(
            &registry,
            &record.id,
            &sign(&challenge.proof_message(&record.id)),
        )
        .unwrap();
    let request = |path: &str| {
        let mut request = Envelope::new(
            Channel::Control,
            Message::Request {
                operation: Operation::Diff,
                arguments: json!({"subtab_id":selected.active_id,"subtab_revision":selected.revision,"path":path}),
            },
        );
        request.id = Some("review".into());
        request.project_id = Some(context.project_id.clone());
        request.session_id = Some(context.session_id.clone());
        request
    };
    let pool = Arc::new(Mutex::new(pool));
    let adapter = HostAdapter::new(pool.clone());
    let response = adapter.dispatch(&registry, &device, request("."));
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    response.validate().unwrap();
    let Message::Response { result } = response.payload else {
        panic!("diff")
    };
    assert_eq!(result["subtab_id"], selected.active_id);
    assert!(result["diff"].as_str().unwrap().contains("SELECTED_DIFF"));
    assert!(
        result["diff"]
            .as_str()
            .unwrap()
            .contains("SELECTED_UNTRACKED")
    );
    assert!(!result.to_string().contains("OUTSIDE_DIFF_MUST_NOT_LEAK"));
    assert!(!result.to_string().contains(root.to_str().unwrap()));
    assert!(!result.to_string().contains("OPAQUE_PRIVATE"));
    for path in private_paths {
        assert!(
            !result.to_string().contains(path),
            "private path leaked: {path}"
        );
    }
    for path in private_paths.into_iter().chain([
        "untracked.p12",
        "nested/credentials",
        "deleted.p12",
        "rename.key",
        "renamed.key",
    ]) {
        let refused = adapter.dispatch(&registry, &device, request(path));
        assert_eq!(refused.code, Some(ErrorCode::InvalidRequest), "{path}");
        assert!(matches!(refused.payload,Message::Response{result} if result.is_null()));
    }
    let nested = adapter.dispatch(&registry, &device, request("nested"));
    assert_eq!(nested.execution_state, Some(ExecutionState::Succeeded));
    assert!(
        !serde_json::to_string(&nested)
            .unwrap()
            .contains("OPAQUE_PRIVATE")
    );
    let local = zenpi::slash_actions::diff_value_at(&child, Some("credentials.json")).unwrap();
    assert!(
        local["diff"]
            .as_str()
            .unwrap()
            .contains("OPAQUE_PRIVATE_DIFF_CONTENT")
    );
    let response = adapter.dispatch(&registry, &device, request("file.txt"));
    let Message::Response { result } = response.payload else {
        panic!("diff")
    };
    assert!(!result.to_string().contains("SELECTED_UNTRACKED"));
    for path in ["../outside.txt", "/etc/passwd", "", "missing.txt"] {
        assert_eq!(
            adapter.dispatch(&registry, &device, request(path)).code,
            Some(ErrorCode::InvalidRequest)
        );
    }
    let mut stale = request(".");
    if let Message::Request { arguments, .. } = &mut stale.payload {
        arguments["subtab_revision"] = json!(initial.revision);
    }
    assert_eq!(
        adapter.dispatch(&registry, &device, stale).code,
        Some(ErrorCode::RevisionConflict)
    );
    let mut foreign = request(".");
    foreign.session_id = Some("foreign".into());
    assert_eq!(
        adapter.dispatch(&registry, &device, foreign).code,
        Some(ErrorCode::OwnerMismatch)
    );
    let mut foreign = request(".");
    foreign.project_id = Some("foreign".into());
    assert_eq!(
        adapter.dispatch(&registry, &device, foreign).code,
        Some(ErrorCode::Unauthorized)
    );
    let held = owner.lock().unwrap();
    assert_eq!(
        adapter.dispatch(&registry, &device, request(".")).code,
        Some(ErrorCode::Busy)
    );
    drop(held);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("outside.txt"), child.join("link.txt")).unwrap();
        assert_eq!(
            adapter
                .dispatch(&registry, &device, request("link.txt"))
                .code,
            Some(ErrorCode::InvalidRequest)
        );
        let hook = root.join("fsmonitor.sh");
        std::fs::write(
            &hook,
            format!("#!/bin/sh\ntouch '{}'\n", root.join("hook-ran").display()),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o700)).unwrap();
        git(&root, &["config", "core.fsmonitor", hook.to_str().unwrap()]);
        assert_eq!(
            adapter
                .dispatch(&registry, &device, request("file.txt"))
                .execution_state,
            Some(ExecutionState::Succeeded)
        );
        assert!(!root.join("hook-ran").exists());
    }
    assert_eq!(std::fs::read(&checkpoint).unwrap(), bytes);
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        journal
    );
    std::fs::write(child.join("binary.bin"), [0, 1, 2, 3]).unwrap();
    let response = adapter.dispatch(&registry, &device, request("binary.bin"));
    let Message::Response { result } = response.payload else {
        panic!("binary")
    };
    assert_eq!(result["binary"], true);
    assert_eq!(result["diff"], "");
    std::fs::write(child.join("large.txt"), "line\n".repeat(40_000)).unwrap();
    let response = adapter.dispatch(&registry, &device, request("large.txt"));
    let Message::Response { result } = response.payload else {
        panic!("large")
    };
    assert_eq!(result["truncated"], true);
    assert!(result["diff"].as_str().unwrap().len() <= 65536);
    let page_request = |offset: usize, digest: serde_json::Value| {
        let mut req = request("large.txt");
        if let Message::Request { arguments, .. } = &mut req.payload {
            arguments["offset"] = json!(offset);
            arguments["digest"] = digest;
        }
        req
    };
    let first = adapter.dispatch(&registry, &device, page_request(0, serde_json::Value::Null));
    assert_eq!(first.execution_state, Some(ExecutionState::Succeeded));
    let Message::Response { result: first } = first.payload else {
        panic!("page")
    };
    assert_eq!(first["truncated"], false);
    assert!(first["page"]["total_bytes"].as_u64().unwrap() > 65536);
    let mut current = first.clone();
    let mut full = String::new();
    let mut count = 0;
    loop {
        let patch = current["diff"].as_str().unwrap();
        assert!(patch.len() <= 8192);
        full.push_str(patch);
        count += 1;
        if current["page"]["has_more"] == false {
            break;
        }
        let response = adapter.dispatch(
            &registry,
            &device,
            page_request(
                current["page"]["next_offset"].as_u64().unwrap() as usize,
                first["page"]["digest"].clone(),
            ),
        );
        assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
        let Message::Response { result } = response.payload else {
            panic!("tail")
        };
        current = result;
    }
    assert!(count > 8);
    assert_eq!(
        full.len() as u64,
        first["page"]["total_bytes"].as_u64().unwrap()
    );
    assert!(full.ends_with("+line\n"));
    assert!(!full.contains("[diff truncated]"));
    assert_eq!(std::fs::read(&checkpoint).unwrap(), bytes);
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        journal
    );
    assert_eq!(
        adapter
            .dispatch(&registry, &device, page_request(1, serde_json::Value::Null))
            .code,
        Some(ErrorCode::InvalidRequest)
    );
    assert_eq!(
        adapter
            .dispatch(
                &registry,
                &device,
                page_request(8192, json!("e".repeat(64)))
            )
            .code,
        Some(ErrorCode::RevisionConflict)
    );
    std::fs::write(child.join("large.txt"), "changed\n").unwrap();
    assert_eq!(
        adapter
            .dispatch(
                &registry,
                &device,
                page_request(8192, first["page"]["digest"].clone())
            )
            .code,
        Some(ErrorCode::RevisionConflict)
    );
    std::fs::write(child.join("large.txt"), "line\n".repeat(250_000)).unwrap();
    let capped = adapter.dispatch(&registry, &device, page_request(0, serde_json::Value::Null));
    let Message::Response { result } = capped.payload else {
        panic!("cap")
    };
    assert_eq!(result["truncated"], true);
    assert!(result["page"]["total_bytes"].as_u64().unwrap() <= 1024 * 1024);
    std::fs::write(child.join("large.txt"), "\u{4e2d}\n".repeat(3000)).unwrap();
    let utf8 = adapter.dispatch(&registry, &device, page_request(0, serde_json::Value::Null));
    let Message::Response { result: utf8 } = utf8.payload else {
        panic!("utf8")
    };
    assert_eq!(utf8["page"]["has_more"], true);
    assert!(utf8["diff"].as_str().unwrap().len() <= 8192);
    let patch = utf8["diff"].as_str().unwrap();
    let invalid = patch.find('\u{4e2d}').unwrap() + 1;
    assert_eq!(
        adapter
            .dispatch(
                &registry,
                &device,
                page_request(invalid, utf8["page"]["digest"].clone())
            )
            .code,
        Some(ErrorCode::InvalidRequest)
    );
    let mut incomplete = page_request(0, serde_json::Value::Null);
    if let Message::Request { arguments, .. } = &mut incomplete.payload {
        arguments.as_object_mut().unwrap().remove("digest");
    }
    assert_eq!(
        adapter.dispatch(&registry, &device, incomplete).code,
        Some(ErrorCode::InvalidRequest)
    );
    std::fs::write(child.join("file.txt"), "before\n").unwrap();
    let response = adapter.dispatch(&registry, &device, request("file.txt"));
    let Message::Response { result } = response.payload else {
        panic!("clean")
    };
    assert_eq!(result["changed"], false);
    assert_eq!(result["status"], "");
    let mut wrong = request(".");
    if let Message::Request { arguments, .. } = &mut wrong.payload {
        arguments["subtab_id"] = json!(initial.active_id);
    }
    assert_eq!(
        adapter.dispatch(&registry, &device, wrong).code,
        Some(ErrorCode::OwnerMismatch)
    );
    let mut wrong = request(".");
    wrong.required_owner = Some("fake".into());
    wrong.expected_revision = Some(0);
    assert_eq!(
        adapter.dispatch(&registry, &device, wrong).code,
        Some(ErrorCode::InvalidRequest)
    );
    registry.revoke(device.id()).unwrap();
    assert_eq!(
        adapter.dispatch(&registry, &device, request(".")).code,
        Some(ErrorCode::Unauthorized)
    );
}
