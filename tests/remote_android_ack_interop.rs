#![cfg(feature = "remote")]

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
        auth::{Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message},
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

fn exchange(request: &Envelope, response: &Envelope) -> Value {
    let mut bytes = Vec::new();
    for frame in HostAdapter::wire_frames(response.clone()).unwrap() {
        write_frame(&mut bytes, &frame).unwrap();
    }
    json!({"request":request,"code":response.code,"wire_base64":STANDARD.encode(bytes)})
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_cached_ack_and_resume_survive_host_restart_without_cursor_rollback() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("session.jsonl");
    let mut session = SessionStore::open_in_workspace(&path, &root).unwrap();
    session
        .append_turn(Turn::new("seed", TurnRole::User, "desktop seed"))
        .unwrap();
    let session_id = session.session_id().to_owned();
    drop(session);
    let mut host = pool(&path, &root);
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
        .set_permissions(&record.id, 1, vec![Capability::Read], vec![project.clone()])
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
    let mut seed = json!({"device_id":record.id});
    for (name, operation) in [
        ("listing", Operation::WorkspaceList),
        ("snapshot", Operation::Inspect),
    ] {
        let mut request = Envelope::new(
            Channel::Control,
            Message::Request {
                operation,
                arguments: json!({}),
            },
        );
        request.id = Some(name.into());
        request.project_id = Some(project.clone());
        request.session_id = Some(session_id.clone());
        let response = HostAdapter::dispatch_owned(&mut host, &registry, &device, request);
        assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
        let Message::Response { result } = response.payload else {
            panic!("read")
        };
        seed[name] = result;
    }
    let seed_path = root.join("seed.json");
    let commands = root.join("commands.bin");
    let results = root.join("results.json");
    let next = root.join("next.bin");
    std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
    let run_java = |phase: &str| {
        let mut child = Command::new(std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap())
            .args([
                format!("-Dzenpi.ack.phase={phase}"),
                format!("-Dzenpi.ack.seed={}", seed_path.display()),
                format!("-Dzenpi.ack.commands={}", commands.display()),
                format!("-Dzenpi.ack.results={}", results.display()),
                format!("-Dzenpi.ack.next={}", next.display()),
                "-cp".into(),
                std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap(),
                "org.junit.runner.JUnitCore".into(),
                "zenpi.AndroidRustAckTest".into(),
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
    run_java("generate");
    let bytes = std::fs::read(&commands).unwrap();
    let mut input = bytes.as_slice();
    let requests: Vec<_> = (0..4)
        .map(|_| read_frame(&mut input).unwrap().unwrap())
        .collect();
    assert!(input.is_empty());
    for request in &requests {
        assert_eq!(request.project_id.as_deref(), Some(project.as_str()));
        assert_eq!(request.session_id.as_deref(), Some(session_id.as_str()));
    }
    let journal = std::fs::read(&path).unwrap();
    let checkpoint = root.join("remote-state.json");
    let persisted = |sequence: u64| {
        let value: Value = serde_json::from_slice(&std::fs::read(&checkpoint).unwrap()).unwrap();
        assert_eq!(
            value["event_acks"],
            json!([{"device_id":record.id,"project_id":project,
            "cursor":{"session_id":session_id,"next_sequence":sequence}}])
        );
    };
    let mut evidence = Vec::new();
    let owner = host.owner(&project).unwrap();
    let held = owner.lock().unwrap();
    let busy = HostAdapter::dispatch_owned(&mut host, &registry, &device, requests[0].clone());
    assert_eq!(busy.code, Some(ErrorCode::Busy));
    evidence.push(exchange(&requests[0], &busy));
    drop(held);
    drop(owner);
    let ack = HostAdapter::dispatch_owned(&mut host, &registry, &device, requests[0].clone());
    assert_eq!(ack.execution_state, Some(ExecutionState::Succeeded));
    persisted(1);
    evidence.push(exchange(&requests[0], &ack));
    let durable = std::fs::read(&checkpoint).unwrap();
    drop(host);
    host = pool(&path, &root);
    persisted(1);
    for request in &requests[..3] {
        let response = HostAdapter::dispatch_owned(&mut host, &registry, &device, request.clone());
        if request.id == requests[2].id {
            assert_eq!(response.code, Some(ErrorCode::InvalidRequest));
        } else {
            assert_eq!(response.payload, ack.payload);
            assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
        }
        evidence.push(exchange(request, &response));
        persisted(1);
        assert_eq!(std::fs::read(&checkpoint).unwrap(), durable);
    }
    assert_eq!(std::fs::read(&path).unwrap(), journal);
    {
        let owner = host.owner(&project).unwrap();
        let mut agent = owner.lock().unwrap();
        agent
            .session_mut()
            .append_turn(Turn::new(
                "update",
                TurnRole::Assistant,
                "desktop after restart",
            ))
            .unwrap();
        agent.resume_session(&path).unwrap();
    }
    let updated_journal = std::fs::read(&path).unwrap();
    let replay = HostAdapter::dispatch_owned(&mut host, &registry, &device, requests[3].clone());
    assert_eq!(replay.execution_state, Some(ExecutionState::Succeeded));
    evidence.push(exchange(&requests[3], &replay));
    persisted(1);
    assert_eq!(std::fs::read(&path).unwrap(), updated_journal);
    std::fs::write(
        &results,
        serde_json::to_vec(&json!({"exchanges":evidence})).unwrap(),
    )
    .unwrap();
    run_java("validate");
    let bytes = std::fs::read(&next).unwrap();
    let mut input = bytes.as_slice();
    let next_ack = read_frame(&mut input).unwrap().unwrap();
    assert!(input.is_empty());
    let response = HostAdapter::dispatch_owned(&mut host, &registry, &device, next_ack.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    persisted(2);
    let mut final_exchanges = vec![exchange(&next_ack, &response)];
    drop(host);
    host = pool(&path, &root);
    persisted(2);
    let durable = std::fs::read(&checkpoint).unwrap();
    let old = HostAdapter::dispatch_owned(&mut host, &registry, &device, requests[0].clone());
    assert_eq!(old.payload, response.payload);
    final_exchanges.push(exchange(&requests[0], &old));
    registry
        .set_permissions(&record.id, 2, vec![Capability::Read], vec![])
        .unwrap();
    let denied = HostAdapter::dispatch_owned(&mut host, &registry, &device, next_ack.clone());
    assert_eq!(denied.code, Some(ErrorCode::Unauthorized));
    final_exchanges.push(exchange(&next_ack, &denied));
    persisted(2);
    assert_eq!(std::fs::read(&checkpoint).unwrap(), durable);
    assert_eq!(std::fs::read(&path).unwrap(), updated_journal);
    std::fs::write(
        &results,
        serde_json::to_vec(&json!({"exchanges":final_exchanges})).unwrap(),
    )
    .unwrap();
    run_java("final");
    if let Some(path) = std::env::var_os("ZENPI_ANDROID_ACK_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&json!({"fixture_only":true,"passed":true,
            "scope":"current Kotlin cached cursor ACK/resume with Rust checkpoint restart; not APK, TLS, LAN or device acceptance",
            "initial_exchanges":evidence,"final_exchanges":final_exchanges})).unwrap()).unwrap();
    }
}
