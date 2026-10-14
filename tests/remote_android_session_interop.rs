use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
use serde_json::{Value, json};
use std::{
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use zenpi::{
    backend::OpenAiCompatibleBackend,
    core::{Agent, Turn, TurnRole},
    project_workspace::ProjectOwnerPool,
    remote::{
        auth::{
            AuthenticatedDevice, Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message,
        },
        frame::{read_frame, write_frame},
        host_adapter::HostAdapter,
        protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message, Operation},
    },
    session::SessionStore,
};

fn pool(path: &Path, root: &Path) -> ProjectOwnerPool {
    let session = SessionStore::open_in_workspace(path, root).unwrap();
    let backend = OpenAiCompatibleBackend::new("http://127.0.0.1:9", None, "fixture").unwrap();
    let mut pool =
        ProjectOwnerPool::new(Arc::new(Mutex::new(Agent::new(session, Box::new(backend)))))
            .unwrap();
    pool.restore_checkpoint().unwrap();
    pool
}

fn views(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    project: &str,
) -> Value {
    let session = pool.context(project).unwrap().session_id.clone();
    let mut result = serde_json::Map::new();
    for (name, operation) in [
        ("listing", Operation::WorkspaceList),
        ("snapshot", Operation::Inspect),
        ("subtabs", Operation::SubtabList),
        ("catalog", Operation::SessionList),
        ("searched_catalog", Operation::SessionList),
    ] {
        let mut request = Envelope::new(
            Channel::Control,
            Message::Request {
                operation,
                arguments: if name == "searched_catalog" {
                    json!({"query": "desktop"})
                } else {
                    json!({})
                },
            },
        );
        request.id = Some(name.into());
        request.project_id = Some(project.into());
        request.session_id = Some(session.clone());
        let response = HostAdapter::dispatch_owned(pool, registry, device, request);
        assert_eq!(
            response.execution_state,
            Some(ExecutionState::Succeeded),
            "{name}: {response:?}"
        );
        let Message::Response { result: value } = response.payload else {
            panic!("read")
        };
        result.insert(name.into(), value);
    }
    Value::Object(result)
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_session_new_resume_and_restart_receipts_replace_real_owner() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let original_path = root.join("original.jsonl");
    let mut original = SessionStore::open_in_workspace(&original_path, &root).unwrap();
    original
        .append_turn(Turn::new(
            "original",
            TurnRole::User,
            "Original desktop conversation",
        ))
        .unwrap();
    let original_id = original.session_id().to_owned();
    drop(original);
    let history_path = root.join("history.jsonl");
    // Restore a valid opaque legacy header identity before any conversation records exist.
    drop(SessionStore::open_in_workspace(&history_path, &root).unwrap());
    let mut header: Value = serde_json::from_slice(&std::fs::read(&history_path).unwrap()).unwrap();
    assert_eq!(header["kind"], "session");
    header["session_id"] = json!("history:session/\u{4e2d}");
    let mut encoded = serde_json::to_vec(&header).unwrap();
    encoded.push(b'\n');
    std::fs::write(&history_path, encoded).unwrap();
    let mut history = SessionStore::open_in_workspace(&history_path, &root).unwrap();
    assert!(history.recovery_warnings().is_empty());
    history
        .append_turn(Turn::new(
            "history",
            TurnRole::Assistant,
            "Historical desktop answer",
        ))
        .unwrap();
    let history_id = history.session_id().to_owned();
    assert_eq!(history_id, "history:session/\u{4e2d}");
    drop(history);
    let history_bytes = std::fs::read(&history_path).unwrap();
    let original_bytes = std::fs::read(&original_path).unwrap();
    let mut host = pool(&original_path, &root);
    let project = host.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let random = ring::rand::SystemRandom::new();
    let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random).unwrap();
    let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    let signature = URL_SAFE_NO_PAD.encode(
        key.sign(
            &random,
            &pairing_proof_message(&"a".repeat(64), "token", "pair", "phone", &public),
        )
        .unwrap()
        .as_ref(),
    );
    let record = registry
        .verify_pair_and_register("token", "pair", "phone", &public, &signature, || true)
        .unwrap();
    registry
        .set_permissions(
            &record.id,
            1,
            vec![Capability::Read, Capability::Control],
            vec![project.clone()],
        )
        .unwrap();
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let signature = URL_SAFE_NO_PAD.encode(
        key.sign(&random, &challenge.proof_message(&record.id))
            .unwrap()
            .as_ref(),
    );
    let device = challenge
        .authenticate(&registry, &record.id, &signature)
        .unwrap();
    let seed_path = root.join("seed.json");
    let command_path = root.join("command.bin");
    let results_path = root.join("results.json");
    let java = std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap();
    let classpath = std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap();
    let run_java = |phase: &str| {
        let mut child = Command::new(&java)
            .args([
                format!("-Dzenpi.session.phase={phase}"),
                format!("-Dzenpi.session.seed={}", seed_path.display()),
                format!("-Dzenpi.session.command={}", command_path.display()),
                format!("-Dzenpi.session.results={}", results_path.display()),
                "-cp".into(),
                classpath.clone(),
                "org.junit.runner.JUnitCore".into(),
                "zenpi.AndroidRustSessionTest".into(),
            ])
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "Kotlin {phase}");
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("Kotlin deadline");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    };
    let mut evidence = Vec::new();
    for step in [
        "new_first",
        "stale_resume",
        "resume_history",
        "resume_original",
        "new_second",
    ] {
        let target = if step == "resume_original" {
            &original_id
        } else {
            &history_id
        };
        let mut seed = views(&mut host, &registry, &device, &project);
        seed["step"] = json!(step);
        seed["target"] = json!(target);
        seed["expected_content"] = json!(if step == "resume_original" {
            "Original desktop conversation"
        } else {
            "Historical desktop answer"
        });
        std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
        run_java("generate");
        let bytes = std::fs::read(&command_path).unwrap();
        let mut input = bytes.as_slice();
        let command = read_frame(&mut input).unwrap().unwrap();
        assert!(input.is_empty());
        let before = host.context(&project).unwrap().session_id.clone();
        let revision = host.workspace_revision();
        let pre_restart_owner = host.workspace_owner().to_owned();
        let mut exchanges = Vec::new();
        let mut retain = |request: &Envelope, response: &Envelope| {
            let mut wire = Vec::new();
            write_frame(&mut wire, response).unwrap();
            exchanges.push(
                json!({"request":request,"code":response.code,"wire_base64":STANDARD.encode(wire)}),
            );
        };
        if step == "stale_resume" {
            SessionStore::open_in_workspace(root.join("catalog-change.jsonl"), &root).unwrap();
            let response =
                HostAdapter::dispatch_owned(&mut host, &registry, &device, command.clone());
            assert_eq!(response.code, Some(ErrorCode::RevisionConflict));
            retain(&command, &response);
            assert_eq!(host.context(&project).unwrap().session_id, before);
            assert_eq!(host.workspace_revision(), revision);
        } else {
            let owner = host.owner(&project).unwrap();
            let old_port = host.control(&project).unwrap().input_port;
            let held = owner.lock().unwrap();
            let busy = HostAdapter::dispatch_owned(&mut host, &registry, &device, command.clone());
            assert_eq!(busy.code, Some(ErrorCode::Busy));
            retain(&command, &busy);
            drop(held);
            let response =
                HostAdapter::dispatch_owned(&mut host, &registry, &device, command.clone());
            assert_eq!(
                response.execution_state,
                Some(ExecutionState::Succeeded),
                "{step}: {response:?}"
            );
            retain(&command, &response);
            let after = host.context(&project).unwrap().session_id.clone();
            assert_ne!(after, before);
            assert_eq!(host.workspace_owner(), pre_restart_owner);
            assert_eq!(host.workspace_revision(), revision + 1);
            assert_eq!(old_port.session_id(), before);
            assert_eq!(
                host.control(&project).unwrap().input_port.session_id(),
                after
            );
            let new_path = owner.lock().unwrap().session().path().to_path_buf();
            let committed = std::fs::read(&new_path).unwrap();
            let duplicate =
                HostAdapter::dispatch_owned(&mut host, &registry, &device, command.clone());
            assert_eq!(duplicate, response);
            retain(&command, &duplicate);
            assert_eq!(std::fs::read(&new_path).unwrap(), committed);
            let mut stale = command.clone();
            stale.id = Some(format!("stale-{step}"));
            let refused = HostAdapter::dispatch_owned(&mut host, &registry, &device, stale.clone());
            assert_eq!(refused.code, Some(ErrorCode::RevisionConflict));
            retain(&stale, &refused);
            let mut conflict = command.clone();
            conflict.expected_revision = Some(revision + 1);
            let refused =
                HostAdapter::dispatch_owned(&mut host, &registry, &device, conflict.clone());
            assert_eq!(refused.code, Some(ErrorCode::RequestIdConflict));
            retain(&conflict, &refused);
            drop(owner);
            drop(host);
            host = pool(&new_path, &root);
            assert_ne!(host.workspace_owner(), pre_restart_owner);
            assert_eq!(host.context(&project).unwrap().session_id, after);
            let restarted = std::fs::read(&new_path).unwrap();
            let repeated =
                HostAdapter::dispatch_owned(&mut host, &registry, &device, command.clone());
            assert_eq!(repeated, response);
            retain(&command, &repeated);
            assert_eq!(host.workspace_revision(), revision + 1);
            assert_eq!(std::fs::read(&new_path).unwrap(), restarted);
        }
        assert_eq!(std::fs::read(&original_path).unwrap(), original_bytes);
        assert_eq!(std::fs::read(&history_path).unwrap(), history_bytes);
        let after = views(&mut host, &registry, &device, &project);
        let result = json!({"step":step,"request":command,"exchanges":exchanges,"after":after,"pre_restart_owner":pre_restart_owner});
        std::fs::write(&results_path, serde_json::to_vec(&result).unwrap()).unwrap();
        run_java("validate");
        evidence.push(result);
    }
    if let Some(path) = std::env::var_os("ZENPI_ANDROID_SESSION_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&json!({"fixture_only":true,"passed":true,
            "scope":"current Kotlin/Rust owner session replacement/resume and restart deduplication; not device, APK, TLS or LAN acceptance", "steps":evidence})).unwrap()).unwrap();
    }
}
