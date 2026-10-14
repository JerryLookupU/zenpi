use base64::{Engine, engine::general_purpose::STANDARD};
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
use serde_json::json;
use std::sync::{Arc, Mutex};
use zenpi::{
    backend::OpenAiCompatibleBackend,
    core::{Agent, Turn, TurnRole},
    project_workspace::ProjectOwnerPool,
    remote::{
        auth::{Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message},
        frame::{read_frame, write_frame},
        host_adapter::HostAdapter,
        protocol::{Channel, Envelope, ErrorCode, EventCursor, ExecutionState, Message, Operation},
        subscription,
    },
    session::SessionStore,
};

fn request(id: &str, project: &str, session: &str, payload: Message) -> Envelope {
    let mut value = Envelope::new(Channel::Control, payload);
    value.id = Some(id.into());
    value.project_id = Some(project.into());
    value.session_id = Some(session.into());
    value
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_p256_pair_and_challenge_proofs_are_verified_by_rust() {
    use zenpi::remote::{
        auth::verify_pairing_proof,
        pairing::{HostIdentity, InviteIssuer},
    };
    let root = tempfile::tempdir().unwrap();
    let identity = HostIdentity::generate().unwrap();
    let issuer = InviteIssuer::new(&identity);
    let invite = issuer
        .issue(
            "192.168.50.55:7777".parse().unwrap(),
            std::time::Duration::from_secs(120),
        )
        .unwrap();
    let fingerprint = identity.fingerprint();
    let mut challenges: Vec<_> = [
        "valid",
        "foreign_host",
        "foreign_nonce",
        "foreign_device",
        "revoked",
        "revoke",
        "revoke_wrong_query",
    ]
    .into_iter()
    .map(|name| (name, DeviceChallenge::new(fingerprint.clone()).unwrap()))
    .collect();
    let seed_path = root.path().join("proof-seed.json");
    let output_path = root.path().join("proof-output.json");
    std::fs::write(&seed_path, serde_json::to_vec(&json!({"uri":invite.to_uri().unwrap(),"nonce":challenges[0].1.nonce(),
        "challenges":challenges.iter().map(|(kind, challenge)|json!({"kind":kind,"nonce":challenge.nonce()})).collect::<Vec<_>>()})).unwrap()).unwrap();
    let mut child =
        std::process::Command::new(std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap())
            .args([
                format!("-Dzenpi.proof.seed={}", seed_path.display()),
                format!("-Dzenpi.proof.output={}", output_path.display()),
                "-cp".into(),
                std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap(),
                "org.junit.runner.JUnitCore".into(),
                "zenpi.AndroidRustProofTest".into(),
            ])
            .spawn()
            .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("Android proof fixture deadline");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    assert!(status.success());
    let output: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&output_path).unwrap()).unwrap();
    let wire = STANDARD
        .decode(output["pair_wire"].as_str().unwrap())
        .unwrap();
    let mut input = wire.as_slice();
    assert!(matches!(
        read_frame(&mut input).unwrap().unwrap().payload,
        Message::Hello { .. }
    ));
    let pair = read_frame(&mut input).unwrap().unwrap();
    assert!(input.is_empty());
    let Message::Pair {
        token,
        fingerprint: pin,
        device_name: name,
        public_key: public,
        signature,
    } = &pair.payload
    else {
        panic!("signed pair")
    };
    assert_eq!(pin, &fingerprint);
    assert_eq!(name, "Phone\u{2028}desk\\u2028");
    let id = pair.id.as_deref().unwrap();
    for (bad_pin, bad_token, bad_id, bad_name) in [
        ("b".repeat(64), token.clone(), id.to_owned(), name.clone()),
        (pin.clone(), "A".repeat(43), id.to_owned(), name.clone()),
        (pin.clone(), token.clone(), "foreign".into(), name.clone()),
        (
            pin.clone(),
            token.clone(),
            id.to_owned(),
            "Different phone".into(),
        ),
    ] {
        assert!(
            verify_pairing_proof(&bad_pin, &bad_token, &bad_id, &bad_name, public, signature)
                .is_err()
        );
    }
    let registry =
        DeviceRegistry::new(root.path().join("devices.json"), fingerprint.clone()).unwrap();
    // The actual Kotlin-signed pair must be refused at the frame boundary
    // before its one-use token or registry can be touched by host admission.
    for field in [
        "project_id",
        "session_id",
        "turn_id",
        "required_owner",
        "expected_revision",
        "sequence",
    ] {
        for value in [
            serde_json::Value::Null,
            match field {
                "expected_revision" | "sequence" => json!(0),
                _ => json!("foreign"),
            },
        ] {
            let mut hostile = serde_json::to_value(&pair).unwrap();
            hostile[field] = value;
            let body = serde_json::to_vec(&hostile).unwrap();
            let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
            bytes.extend(body);
            assert!(read_frame(&mut bytes.as_slice()).is_err());
            assert!(registry.list().unwrap().is_empty());
        }
    }
    let mounted = vec!["mounted-project".to_owned()];
    let record = registry
        .verify_pair_and_grant(token, id, name, public, signature, &mounted, || {
            issuer.consume(token, pin).is_ok()
        })
        .unwrap();
    assert_eq!(record.id, output["device_id"]);
    assert_eq!(
        record.capabilities,
        [Capability::Read, Capability::Control, Capability::Approval]
    );
    assert_eq!(record.projects, mounted);
    assert!(issuer.consume(token, pin).is_err());
    let mut framed_signatures = std::collections::BTreeMap::new();
    for proof in output["proofs"].as_array().unwrap() {
        let kind = proof["kind"].as_str().unwrap();
        let wire = STANDARD
            .decode(proof["wire_base64"].as_str().unwrap())
            .unwrap();
        let mut input = wire.as_slice();
        let request = read_frame(&mut input).unwrap().unwrap();
        assert!(input.is_empty(), "one authentication frame only");
        assert_eq!(request.channel, Channel::Control);
        assert_eq!(request.project_id, None);
        assert_eq!(request.session_id, None);
        let (device_id, signature) = match &request.payload {
            Message::Authenticate {
                device_id,
                signature,
            } => {
                assert!(!kind.starts_with("revoke") || kind == "revoked");
                assert_eq!(request.id.as_deref(), Some(format!("auth-{kind}").as_str()));
                (device_id, signature)
            }
            Message::RevokeStatus {
                device_id,
                request_id,
                signature,
            } => {
                assert!(matches!(kind, "revoke" | "revoke_wrong_query"));
                assert_eq!(request.id.as_deref(), Some("query-interop"));
                assert_eq!(request_id, "revoke-interop");
                (device_id, signature)
            }
            _ => panic!("unexpected authentication proof frame"),
        };
        assert_eq!(device_id, &record.id);
        assert_eq!(signature, proof["signature"].as_str().unwrap());
        assert!(
            framed_signatures
                .insert(kind.to_owned(), signature.clone())
                .is_none()
        );
    }
    assert_eq!(framed_signatures.len(), challenges.len());
    let signature_for = |kind: &str| framed_signatures.get(kind).unwrap().as_str();
    let device = challenges[0]
        .1
        .authenticate(&registry, &record.id, signature_for("valid"))
        .unwrap();
    assert!(
        challenges[0]
            .1
            .authenticate(&registry, &record.id, signature_for("valid"))
            .is_err()
    );
    for (kind, challenge) in &mut challenges[1..4] {
        assert!(
            challenge
                .authenticate(&registry, &record.id, signature_for(kind))
                .is_err()
        );
    }
    let mut revoke = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::DeviceRevokeSelf,
            arguments: json!({}),
        },
    );
    revoke.id = Some("revoke-interop".into());
    revoke.required_owner = Some(device.id().into());
    revoke.expected_revision = Some(device.revision());
    assert_eq!(
        registry
            .dispatch_self_revoke(&device, &revoke)
            .execution_state,
        Some(ExecutionState::Succeeded)
    );
    assert!(
        challenges[4]
            .1
            .authenticate(&registry, &record.id, signature_for("revoked"))
            .is_err()
    );
    assert!(
        challenges[5]
            .1
            .revoke_status(
                &registry,
                &record.id,
                "revoke-interop",
                "query-interop",
                signature_for("revoke")
            )
            .unwrap()
    );
    assert!(
        challenges[5]
            .1
            .revoke_status(
                &registry,
                &record.id,
                "revoke-interop",
                "query-interop",
                signature_for("revoke")
            )
            .is_err()
    );
    assert!(
        challenges[6]
            .1
            .revoke_status(
                &registry,
                &record.id,
                "revoke-interop",
                "different-query",
                signature_for("revoke_wrong_query")
            )
            .is_err()
    );
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_generated_rename_roundtrips_through_the_current_host() {
    let classpath = std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap();
    let java = std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("session.jsonl");
    let session = SessionStore::open_in_workspace(&path, root.path()).unwrap();
    let session_id = session.session_id().to_owned();
    let backend = OpenAiCompatibleBackend::new("http://127.0.0.1:9", None, "fixture").unwrap();
    let owner = Arc::new(Mutex::new(Agent::new(session, Box::new(backend))));
    let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
    pool.restore_checkpoint().unwrap();
    let project = pool.active_context().project_id.clone();
    let registry = DeviceRegistry::new(root.path().join("devices.json"), "a".repeat(64)).unwrap();
    let random = ring::rand::SystemRandom::new();
    let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random).unwrap();
    let encode = |bytes: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let public = encode(key.public_key().as_ref());
    let signature = encode(
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
    let authenticate = || {
        let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
        let proof = encode(
            key.sign(&random, &challenge.proof_message(&record.id))
                .unwrap()
                .as_ref(),
        );
        challenge
            .authenticate(&registry, &record.id, &proof)
            .unwrap()
    };
    let device = authenticate();
    let mut seed = serde_json::Map::new();
    for (name, operation) in [
        ("listing", Operation::WorkspaceList),
        ("snapshot", Operation::Inspect),
        ("subtabs", Operation::SubtabList),
    ] {
        let read = request(
            name,
            &project,
            &session_id,
            Message::Request {
                operation,
                arguments: json!({}),
            },
        );
        let response = HostAdapter::dispatch_owned(&mut pool, &registry, &device, read);
        assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
        let Message::Response { result } = response.payload else {
            panic!("response")
        };
        seed.insert(name.into(), result);
    }
    let fixture_path = root.path().join("write-seed.json");
    let command_path = root.path().join("android-command.bin");
    let result_path = root.path().join("write-results.json");
    std::fs::write(&fixture_path, serde_json::to_vec(&seed).unwrap()).unwrap();
    let run_java = |phase: &str| {
        let mut child = std::process::Command::new(&java)
            .args([
                format!("-Dzenpi.write.phase={phase}"),
                format!("-Dzenpi.write.seed={}", fixture_path.display()),
                format!("-Dzenpi.write.command={}", command_path.display()),
                format!("-Dzenpi.write.results={}", result_path.display()),
                "-cp".into(),
                classpath.clone(),
                "org.junit.runner.JUnitCore".into(),
                "zenpi.AndroidRustWriteTest".into(),
            ])
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("Android JVM {phase} exceeded fixture deadline");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert!(status.success(), "Android JVM {phase} failed");
    };
    run_java("generate");
    let bytes = std::fs::read(&command_path).unwrap();
    let mut input = bytes.as_slice();
    let command = read_frame(&mut input).unwrap().unwrap();
    assert!(input.is_empty());
    let before = std::fs::read(&path).unwrap();
    let response = HostAdapter::dispatch_owned(&mut pool, &registry, &device, command.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    assert_eq!(
        owner.lock().unwrap().session().session_name(),
        Some("Android development")
    );
    let committed = std::fs::read(&path).unwrap();
    assert_ne!(before, committed);
    let duplicate = HostAdapter::dispatch_owned(&mut pool, &registry, &device, command.clone());
    assert_eq!(duplicate, response);
    assert_eq!(std::fs::read(&path).unwrap(), committed);
    owner
        .lock()
        .unwrap()
        .session_mut()
        .rename("Desktop newer name")
        .unwrap();
    let local = std::fs::read(&path).unwrap();
    assert_eq!(
        HostAdapter::dispatch_owned(&mut pool, &registry, &device, command.clone()),
        response
    );
    let mut stale = command.clone();
    stale.id = Some("stale-android-request".into());
    let stale_response = HostAdapter::dispatch_owned(&mut pool, &registry, &device, stale.clone());
    assert_eq!(
        stale_response.code,
        Some(zenpi::remote::protocol::ErrorCode::RevisionConflict)
    );
    let mut conflicting = command.clone();
    if let Message::Request { arguments, .. } = &mut conflicting.payload {
        arguments["name"] = json!("Changed payload");
    }
    let conflict_response =
        HostAdapter::dispatch_owned(&mut pool, &registry, &device, conflicting.clone());
    assert_eq!(
        conflict_response.code,
        Some(zenpi::remote::protocol::ErrorCode::RequestIdConflict)
    );
    registry
        .set_permissions(&record.id, 2, vec![Capability::Read], vec![project.clone()])
        .unwrap();
    let reader = authenticate();
    let refused = HostAdapter::dispatch_owned(&mut pool, &registry, &reader, command.clone());
    assert_eq!(
        refused.code,
        Some(zenpi::remote::protocol::ErrorCode::Unauthorized)
    );
    assert_eq!(
        owner.lock().unwrap().session().session_name(),
        Some("Desktop newer name")
    );
    assert_eq!(std::fs::read(&path).unwrap(), local);
    let mut results: Vec<_> = [
        (command.clone(), response),
        (command.clone(), duplicate),
        (stale, stale_response),
        (conflicting, conflict_response),
        (command, refused),
    ]
    .into_iter()
    .map(|(request, response)| {
        let mut wire = Vec::new();
        write_frame(&mut wire, &response).unwrap();
        json!({"request":request,"wire_base64":STANDARD.encode(wire)})
    })
    .collect();
    let listing = request(
        "newer-desktop-listing",
        &project,
        &session_id,
        Message::Request {
            operation: Operation::WorkspaceList,
            arguments: json!({}),
        },
    );
    let response = HostAdapter::dispatch_owned(&mut pool, &registry, &reader, listing.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    let mut wire = Vec::new();
    write_frame(&mut wire, &response).unwrap();
    results.push(json!({"request":listing,"wire_base64":STANDARD.encode(wire)}));
    std::fs::write(&result_path, serde_json::to_vec(&results).unwrap()).unwrap();
    run_java("validate");
    if let Some(path) = std::env::var_os("ZENPI_ANDROID_WRITE_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&json!({"fixture_only":true,
            "scope":"current-source Android/Rust session-rename frames; not device or LAN acceptance",
            "passed":true,"exchanges":results})).unwrap()).unwrap();
    }
}

#[test]
fn current_host_frames_form_an_android_replay_and_gap_fixture() {
    let root = tempfile::tempdir().unwrap();
    let mut session =
        SessionStore::open_in_workspace(root.path().join("session.jsonl"), root.path()).unwrap();
    session
        .append_turn(Turn::new("seed", TurnRole::User, "desktop seed"))
        .unwrap();
    let session_id = session.session_id().to_owned();
    let backend = OpenAiCompatibleBackend::new("http://127.0.0.1:9", None, "fixture").unwrap();
    let owner = Arc::new(Mutex::new(Agent::new(session, Box::new(backend))));
    let pool = Arc::new(Mutex::new(ProjectOwnerPool::new(owner.clone()).unwrap()));
    let project = pool
        .lock()
        .unwrap()
        .workspace()
        .active()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let adapter = HostAdapter::new(pool);
    let registry =
        Arc::new(DeviceRegistry::new(root.path().join("devices.json"), "a".repeat(64)).unwrap());
    let random = ring::rand::SystemRandom::new();
    let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random).unwrap();
    let public = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        key.sign(
            &random,
            &pairing_proof_message(&"a".repeat(64), "fixture-token", "pair", "phone", &public),
        )
        .unwrap()
        .as_ref(),
    );
    let record = registry
        .verify_pair_and_register(
            "fixture-token",
            "pair",
            "phone",
            &public,
            &signature,
            || true,
        )
        .unwrap();
    registry
        .set_permissions(&record.id, 1, vec![Capability::Read], vec![project.clone()])
        .unwrap();
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        key.sign(&random, &challenge.proof_message(&record.id))
            .unwrap()
            .as_ref(),
    );
    let device = challenge
        .authenticate(&registry, &record.id, &signature)
        .unwrap();
    let mut exchanges = Vec::new();
    let mut capture = |request: Envelope, frames: Vec<Envelope>| {
        assert_eq!(
            frames.last().unwrap().execution_state,
            Some(ExecutionState::Succeeded)
        );
        let mut wire = Vec::new();
        for frame in &frames {
            write_frame(&mut wire, frame).unwrap();
        }
        assert!(!String::from_utf8_lossy(&wire).contains("INTEROP_PRIVATE_BODY"));
        exchanges.push(json!({"request":request,"wire_base64":STANDARD.encode(wire)}));
    };
    for (id, operation) in [
        ("listing", Operation::WorkspaceList),
        ("initial", Operation::Inspect),
    ] {
        let read = request(
            id,
            &project,
            &session_id,
            Message::Request {
                operation,
                arguments: json!({}),
            },
        );
        capture(
            read.clone(),
            HostAdapter::wire_frames(adapter.dispatch(&registry, &device, read)).unwrap(),
        );
    }
    let busy_subscribe = request(
        "busy-subscribe",
        &project,
        &session_id,
        Message::Subscribe {
            cursor: EventCursor {
                session_id: session_id.clone(),
                next_sequence: 1,
            },
            limit: 128,
            duration_ms: 1000,
        },
    );
    let held = owner.lock().unwrap();
    let mut busy_frames = Vec::new();
    subscription::serve_with_view(
        &busy_subscribe,
        |read| adapter.dispatch(&registry, &device, read),
        || {
            subscription::view(&busy_subscribe, |read| {
                adapter.dispatch(&registry, &device, read)
            })
        },
        |frame| {
            busy_frames.push(frame);
            Ok(())
        },
    )
    .unwrap();
    drop(held);
    assert!(busy_frames.iter().all(
        |frame| matches!(&frame.payload, Message::Response { result }
        if result["subscription_transcript"] == false && result.get("phase").is_none()
            && result.get("journal_next_sequence").is_none())
    ));
    capture(busy_subscribe, busy_frames);
    let append = |count: usize| {
        let mut agent = owner.lock().unwrap();
        for index in 0..count {
            agent.session_mut().append_turn(Turn::new(format!("update-{count}-{index}"), TurnRole::Assistant,
                if count == 1 { "live desktop answer\n-----BEGIN PRIVATE KEY-----\nINTEROP_PRIVATE_BODY\n-----END PRIVATE KEY-----".to_owned() }
                else { format!("desktop history {index}") })).unwrap();
        }
        let path = agent.session().path().to_owned();
        agent.resume_session(path).unwrap();
    };
    append(1);
    let resume = request(
        "resume",
        &project,
        &session_id,
        Message::Resume {
            cursor: EventCursor {
                session_id: session_id.clone(),
                next_sequence: 1,
            },
            limit: 128,
        },
    );
    capture(
        resume.clone(),
        HostAdapter::wire_frames(adapter.dispatch(&registry, &device, resume)).unwrap(),
    );
    let subscribe = request(
        "subscribe",
        &project,
        &session_id,
        Message::Subscribe {
            cursor: EventCursor {
                session_id: session_id.clone(),
                next_sequence: 1,
            },
            limit: 128,
            duration_ms: 1000,
        },
    );
    let mut frames = Vec::new();
    subscription::serve_with_view(
        &subscribe,
        |read| adapter.dispatch(&registry, &device, read),
        || {
            subscription::view(&subscribe, |read| {
                adapter.dispatch(&registry, &device, read)
            })
        },
        |frame| {
            frames.push(frame);
            Ok(())
        },
    )
    .unwrap();
    capture(subscribe, frames);
    let race_registry = Arc::new(
        DeviceRegistry::new(root.path().join("race-devices.json"), "a".repeat(64)).unwrap(),
    );
    let proof = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        key.sign(
            &random,
            &pairing_proof_message(&"a".repeat(64), "race-token", "race-pair", "phone", &public),
        )
        .unwrap()
        .as_ref(),
    );
    let race_record = race_registry
        .verify_pair_and_register("race-token", "race-pair", "phone", &public, &proof, || true)
        .unwrap();
    race_registry
        .set_permissions(
            &race_record.id,
            1,
            vec![Capability::Read],
            vec![project.clone()],
        )
        .unwrap();
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let proof = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        key.sign(&random, &challenge.proof_message(&race_record.id))
            .unwrap()
            .as_ref(),
    );
    let race_device = challenge
        .authenticate(&race_registry, &race_record.id, &proof)
        .unwrap();
    struct RevokeAfterEvent<'a> {
        wire: Vec<u8>,
        offset: usize,
        registry: &'a DeviceRegistry,
        device: &'a str,
    }
    impl std::io::Write for RevokeAfterEvent<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.wire.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            let mut input = &self.wire[self.offset..];
            let frame = read_frame(&mut input)
                .map_err(std::io::Error::other)?
                .unwrap();
            assert!(input.is_empty());
            self.offset = self.wire.len();
            if matches!(frame.payload, Message::Event { .. }) {
                self.registry
                    .revoke(self.device)
                    .map_err(std::io::Error::other)?;
            }
            Ok(())
        }
    }
    let event_request = request(
        "event-then-revoke",
        &project,
        &session_id,
        Message::Subscribe {
            cursor: EventCursor {
                session_id: session_id.clone(),
                next_sequence: 1,
            },
            limit: 128,
            duration_ms: 1000,
        },
    );
    let journal = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    let mut output = RevokeAfterEvent {
        wire: Vec::new(),
        offset: 0,
        registry: &race_registry,
        device: &race_record.id,
    };
    assert!(
        subscription::serve_authorized(
            &event_request,
            &adapter,
            &race_registry,
            &race_device,
            &mut output
        )
        .is_err()
    );
    let mut input = output.wire.as_slice();
    let event = read_frame(&mut input).unwrap().unwrap();
    assert!(matches!(event.payload, Message::Event { .. }));
    assert_eq!(event.sequence, Some(1));
    let refusal = read_frame(&mut input).unwrap().unwrap();
    assert_eq!(refusal.id, event_request.id);
    assert_eq!(refusal.code, Some(ErrorCode::Unauthorized));
    assert_eq!(
        refusal.payload,
        Message::Response {
            result: serde_json::Value::Null
        }
    );
    assert!(input.is_empty());
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        journal
    );
    let event_then_refusal =
        json!({"request":event_request,"wire_base64":STANDARD.encode(output.wire)});
    append(300);
    let gap = request(
        "gap",
        &project,
        &session_id,
        Message::Resume {
            cursor: EventCursor {
                session_id: session_id.clone(),
                next_sequence: 2,
            },
            limit: 128,
        },
    );
    capture(
        gap.clone(),
        HostAdapter::wire_frames(adapter.dispatch(&registry, &device, gap)).unwrap(),
    );
    let mut owner_race_failures = Vec::new();
    for code in [ErrorCode::Unauthorized, ErrorCode::RevokedDevice] {
        let request = request(
            "owner-race",
            &project,
            &session_id,
            Message::Subscribe {
                cursor: EventCursor {
                    session_id: session_id.clone(),
                    next_sequence: 1,
                },
                limit: 128,
                duration_ms: 1000,
            },
        );
        let mut samples = 0;
        let mut wire = Vec::new();
        subscription::serve_with_view(
            &request,
            |read| HostAdapter::failure(&read, ErrorCode::OwnerMismatch),
            || {
                samples += 1;
                if samples == 1 { Ok(None) } else { Err(code) }
            },
            |frame| {
                assert_eq!(frame.code, Some(code));
                write_frame(&mut wire, &frame).map_err(std::io::Error::other)
            },
        )
        .unwrap();
        assert_eq!(samples, 2);
        owner_race_failures
            .push(json!({"request":request,"code":code,"wire_base64":STANDARD.encode(wire)}));
    }
    let mut authorization_failures = Vec::new();
    for revoked in [false, true] {
        if revoked {
            registry.revoke(&record.id).unwrap();
        } else {
            registry
                .set_permissions(
                    &record.id,
                    device.revision(),
                    vec![Capability::Read],
                    vec![],
                )
                .unwrap();
        }
        let request = request(
            "authorization-loss",
            &project,
            &session_id,
            Message::Subscribe {
                cursor: EventCursor {
                    session_id: session_id.clone(),
                    next_sequence: 1,
                },
                limit: 128,
                duration_ms: 1000,
            },
        );
        let mut wire = Vec::new();
        assert!(
            subscription::serve_authorized(&request, &adapter, &registry, &device, &mut wire)
                .is_err()
        );
        let mut input = wire.as_slice();
        let failure = read_frame(&mut input).unwrap().unwrap();
        assert_eq!(failure.code, Some(ErrorCode::Unauthorized));
        assert!(input.is_empty());
        authorization_failures.push(json!({"request":request,"code":"unauthorized","revoked":revoked,"wire_base64":STANDARD.encode(wire)}));
    }
    let fixture = json!({"schema_version":"stage2-rust-android-wire/v1","fixture_only":true,
        "project_id":project,"session_id":session_id,"exchanges":exchanges,
        "owner_race_failures":owner_race_failures,"authorization_failures":authorization_failures,
        "event_then_refusal":event_then_refusal});
    let bytes = serde_json::to_vec_pretty(&fixture).unwrap();
    if let Some(path) = std::env::var_os("ZENPI_ANDROID_INTEROP_OUTPUT") {
        std::fs::write(path, bytes).unwrap();
    }
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_reducer_rebuilds_streamed_live_text_in_order_across_replay() {
    use zenpi::{backend::ProviderEvent, remote::live_text::LivePromptText};
    let classpath = std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap();
    let java = std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap();
    let root = tempfile::tempdir().unwrap();
    let mut session =
        SessionStore::open_in_workspace(root.path().join("session.jsonl"), root.path()).unwrap();
    session
        .append_turn(Turn::new("seed", TurnRole::User, "stream seed"))
        .unwrap();
    let session_id = session.session_id().to_owned();
    let backend = OpenAiCompatibleBackend::new("http://127.0.0.1:9", None, "fixture").unwrap();
    let owner = Arc::new(Mutex::new(Agent::new(session, Box::new(backend))));
    let pool = Arc::new(Mutex::new(ProjectOwnerPool::new(owner).unwrap()));
    let project = pool.lock().unwrap().active_context().project_id.clone();
    let adapter = HostAdapter::new(pool);
    let registry =
        Arc::new(DeviceRegistry::new(root.path().join("devices.json"), "a".repeat(64)).unwrap());
    let random = ring::rand::SystemRandom::new();
    let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random).unwrap();
    let encode = |bytes: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let public = encode(key.public_key().as_ref());
    let proof = pairing_proof_message(&"a".repeat(64), "token", "pair", "phone", &public);
    let signature = encode(key.sign(&random, &proof).unwrap().as_ref());
    let record = registry
        .verify_pair_and_register("token", "pair", "phone", &public, &signature, || true)
        .unwrap();
    registry
        .set_permissions(&record.id, 1, vec![Capability::Read], vec![project.clone()])
        .unwrap();
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let signature = encode(
        key.sign(&random, &challenge.proof_message(&record.id))
            .unwrap()
            .as_ref(),
    );
    let device = challenge
        .authenticate(&registry, &record.id, &signature)
        .unwrap();
    let exchange = |request: &Envelope, frames: &[Envelope]| {
        let mut wire = Vec::new();
        for frame in frames {
            write_frame(&mut wire, frame).unwrap();
        }
        json!({"request":request,"wire_base64":STANDARD.encode(wire)})
    };
    let mut fixture = serde_json::Map::new();
    for (id, operation) in [
        ("listing", Operation::WorkspaceList),
        ("initial", Operation::Inspect),
    ] {
        let read = request(
            id,
            &project,
            &session_id,
            Message::Request {
                operation,
                arguments: json!({}),
            },
        );
        let frames =
            HostAdapter::wire_frames(adapter.dispatch(&registry, &device, read.clone())).unwrap();
        fixture.insert(id.into(), exchange(&read, &frames));
    }
    // The prompt progress a desktop-attached host lists for this device.
    let prompt = request(
        "android-stream-prompt",
        &project,
        &session_id,
        Message::Request {
            operation: Operation::Prompt,
            arguments: json!({"text":"stream"}),
        },
    );
    let texts = LivePromptText::default();
    let tokens = [
        "Stream",
        "ing to",
        " the",
        " phone",
        " api_key=SEC",
        "RET then",
        " 中文",
        " done\n",
    ];
    let final_text = "Streaming to the phone, final";
    let mut published = Vec::new();
    let subscribe = |id: &str, feed: &mut dyn FnMut()| {
        let request = request(
            id,
            &project,
            &session_id,
            Message::Subscribe {
                cursor: EventCursor {
                    session_id: session_id.clone(),
                    next_sequence: 1,
                },
                limit: 128,
                duration_ms: 1000,
            },
        );
        let mut frames = Vec::new();
        subscription::serve_with_view(
            &request,
            |read| adapter.dispatch(&registry, &device, read),
            || {
                feed();
                let mut view =
                    subscription::view(&request, |read| adapter.dispatch(&registry, &device, read))?;
                if let Some(view) = view.as_mut() {
                    let mut status = json!({"request_id":"android-stream-prompt","project_id":project,
                        "session_id":session_id,"state":"running"});
                    if let Some(text) =
                        texts.projected(device.id(), &project, &session_id, "android-stream-prompt")
                    {
                        status["stream_text"] = json!(text);
                    }
                    view["listing"]["projects"][0]["prompt_statuses"] = json!([status]);
                }
                Ok(view)
            },
            |frame| {
                frames.push(frame);
                Ok(())
            },
        )
        .unwrap();
        exchange(&request, &frames)
    };
    let mut fed = 0;
    let stream = subscribe("stream", &mut || {
        let event = match tokens.get(fed) {
            Some(token) => ProviderEvent::TextDelta {
                delta: (*token).into(),
            },
            None if fed == tokens.len() => ProviderEvent::TextDone {
                text: final_text.into(),
            },
            None => return,
        };
        texts.update(&prompt, device.id(), &event);
        fed += 1;
        if let Some(text) =
            texts.projected(device.id(), &project, &session_id, "android-stream-prompt")
            && published.last() != Some(&text)
        {
            published.push(text);
        }
    });
    let replay = subscribe("replay", &mut || {});
    assert_eq!(
        published.last().map(String::as_str),
        Some(final_text),
        "{published:?}"
    );
    assert!(published.iter().all(|text| !text.contains("SECRET")));
    fixture.insert("stream".into(), stream);
    fixture.insert("replay".into(), replay);
    fixture.insert("request_id".into(), json!("android-stream-prompt"));
    fixture.insert("published".into(), json!(published));
    fixture.insert("final".into(), json!(final_text));
    let path = root.path().join("stream.json");
    std::fs::write(&path, serde_json::to_vec(&fixture).unwrap()).unwrap();
    let status = std::process::Command::new(&java)
        .args([
            format!("-Dzenpi.stream.fixture={}", path.display()),
            "-cp".into(),
            classpath,
            "org.junit.runner.JUnitCore".into(),
            "zenpi.AndroidRustStreamTest".into(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "Android JVM stream reducer failed");
}
