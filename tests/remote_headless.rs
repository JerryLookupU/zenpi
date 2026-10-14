use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
use serde_json::json;
use std::sync::{Arc, Mutex};
use zenpi::{
    backend::OpenAiCompatibleBackend,
    core::{Agent, Turn, TurnRole},
    project_workspace::ProjectOwnerPool,
    remote::{
        auth::{Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message},
        host_adapter::HostAdapter,
        protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message, Operation},
    },
    session::SessionStore,
};

fn request(operation: Operation, project: Option<&str>) -> Envelope {
    let mut request = Envelope::new(
        Channel::Control,
        Message::Request {
            operation,
            arguments: json!({}),
        },
    );
    request.id = Some("r1".into());
    request.project_id = project.map(str::to_owned);
    request
}

#[test]
fn adapter_reads_the_actual_owner_and_new_local_turns_without_a_parallel_agent() {
    let temp = tempfile::tempdir().unwrap();
    let session =
        SessionStore::open_in_workspace(temp.path().join("session.jsonl"), temp.path()).unwrap();
    let backend =
        OpenAiCompatibleBackend::new("http://127.0.0.1:9", None, "local-fixture").unwrap();
    let owner = Arc::new(Mutex::new(Agent::new(session, Box::new(backend))));
    let pool = Arc::new(Mutex::new(ProjectOwnerPool::new(owner.clone()).unwrap()));
    let id = pool
        .lock()
        .unwrap()
        .workspace()
        .active()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    assert!(Arc::ptr_eq(
        &pool.lock().unwrap().owner(&id).unwrap(),
        &owner
    ));
    let adapter = HostAdapter::new(pool.clone());
    let registry =
        Arc::new(DeviceRegistry::new(temp.path().join("devices.json"), "a".repeat(64)).unwrap());
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
        .set_permissions(&record.id, 1, vec![Capability::Read], vec![id.clone()])
        .unwrap();
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let signed = URL_SAFE_NO_PAD.encode(
        key.sign(&random, &challenge.proof_message(&record.id))
            .unwrap()
            .as_ref(),
    );
    let device = challenge
        .authenticate(&registry, &record.id, &signed)
        .unwrap();
    let (mailbox_adapter, mailbox) = HostAdapter::owner_thread();
    std::thread::scope(|scope| {
        let pending = scope.spawn(|| {
            mailbox_adapter.dispatch(&registry, &device, request(Operation::WorkspaceList, None))
        });
        let until = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while !pending.is_finished() && std::time::Instant::now() < until {
            mailbox.drain(&mut pool.lock().unwrap());
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(
            pending.join().unwrap().execution_state,
            Some(ExecutionState::Succeeded)
        );
    });
    let mut mutation = request(Operation::Prompt, Some(&id));
    mutation.required_owner = Some("owner".into());
    mutation.expected_revision = Some(0);
    assert_eq!(
        adapter.dispatch(&registry, &device, mutation).code,
        Some(ErrorCode::InvalidRequest)
    );
    // No desktop drain: the caller times out, then expired work is dropped rather than dispatched later.
    assert_eq!(
        mailbox_adapter
            .dispatch(&registry, &device, request(Operation::Inspect, Some(&id)))
            .code,
        Some(ErrorCode::Busy)
    );
    mailbox.drain(&mut pool.lock().unwrap());
    let response = adapter.dispatch(&registry, &device, request(Operation::WorkspaceList, None));
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    let Message::Response { result } = response.payload else {
        panic!("response");
    };
    assert_eq!(result["projects"][0]["id"], id);
    assert!(
        !result
            .to_string()
            .contains(&temp.path().display().to_string())
    );
    for index in 0..2 {
        owner
            .lock()
            .unwrap()
            .session_mut()
            .append_turn(Turn::new(
                format!("local-{index}"),
                TurnRole::Assistant,
                format!("desktop update {index}"),
            ))
            .unwrap();
        // Agent reload is the same local owner operation used by session reopening.
        let path = owner.lock().unwrap().session().path().to_path_buf();
        owner.lock().unwrap().resume_session(path).unwrap();
        let response = adapter.dispatch(&registry, &device, request(Operation::Inspect, Some(&id)));
        let Message::Response { result } = response.payload else {
            panic!("response");
        };
        assert_eq!(result["transcript"].as_array().unwrap().len(), index + 1);
        assert_eq!(
            result["transcript"][index]["content"],
            format!("desktop update {index}")
        );
    }
    {
        let mut agent = owner.lock().unwrap();
        agent
            .session_mut()
            .append_turn(Turn::new(
                "large-local",
                TurnRole::Assistant,
                "large ".repeat(30_000),
            ))
            .unwrap();
        let path = agent.session().path().to_path_buf();
        agent.resume_session(path).unwrap();
    }
    let response = adapter.dispatch(&registry, &device, request(Operation::Inspect, Some(&id)));
    let Message::Response { result } = response.payload else {
        panic!("snapshot");
    };
    assert_eq!(result["transcript"].as_array().unwrap().len(), 3);
    assert_eq!(result["transcript"][2]["id"], "large-local");
    assert_eq!(result["transcript"][2]["content_truncated"], true);
    assert_eq!(result["transcript_truncated"], true);
    assert_eq!(result["event_cursor"]["next_sequence"], 3);
    let session_id = owner.lock().unwrap().session().session_id().to_owned();
    let mut subscription = Envelope::new(
        Channel::Control,
        Message::Subscribe {
            cursor: zenpi::remote::protocol::EventCursor {
                session_id: session_id.clone(),
                next_sequence: 0,
            },
            limit: 128,
            duration_ms: 1000,
        },
    );
    subscription.id = Some("live-subscription".into());
    subscription.project_id = Some(id.clone());
    subscription.session_id = Some(session_id.clone());
    let view = zenpi::remote::subscription::view(&subscription, |read| {
        adapter.dispatch(&registry, &device, read)
    })
    .unwrap()
    .unwrap();
    assert_eq!(view["listing"]["projects"][0]["id"], id);
    assert_eq!(view["subtabs"]["session_id"], session_id);
    assert!(
        !view
            .to_string()
            .contains(&temp.path().display().to_string())
    );
    let held_owner = owner.lock().unwrap();
    let mut busy_frames = Vec::new();
    zenpi::remote::subscription::serve_with_view(
        &subscription,
        |read| adapter.dispatch(&registry, &device, read),
        || {
            zenpi::remote::subscription::view(&subscription, |read| {
                adapter.dispatch(&registry, &device, read)
            })
        },
        |frame| {
            busy_frames.push(frame);
            Ok(())
        },
    )
    .unwrap();
    drop(held_owner);
    assert!(!busy_frames.is_empty());
    for frame in busy_frames {
        assert_eq!(frame.execution_state, Some(ExecutionState::Succeeded));
        let Message::Response { result } = frame.payload else {
            panic!("transient batch");
        };
        assert_eq!(result["subscription_transcript"], false);
        assert_eq!(result["cursor"]["next_sequence"], 0);
    }
    let mut pushed = Vec::new();
    zenpi::remote::subscription::serve(
        &subscription,
        |resume| adapter.dispatch(&registry, &device, resume),
        |frame| {
            pushed.push(frame);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        pushed
            .iter()
            .filter(|frame| matches!(frame.payload, Message::Event { .. }))
            .count(),
        3
    );
    let Message::Response { result } = &pushed.last().unwrap().payload else {
        panic!("subscription completion");
    };
    assert_eq!(result["subscription_done"], true);
    assert_eq!(result["cursor"]["next_sequence"], 3);
    let ack = |next| {
        let mut value = Envelope::new(
            Channel::Control,
            Message::Ack {
                cursor: zenpi::remote::protocol::EventCursor {
                    session_id: session_id.clone(),
                    next_sequence: next,
                },
            },
        );
        value.id = Some("ack".into());
        value.project_id = Some(id.clone());
        value.session_id = Some(session_id.clone());
        value
    };
    // Enabling the same durable checkpoint used by workspace mutations must not replace the owner.
    pool.lock().unwrap().restore_checkpoint().unwrap();
    let revision = pool.lock().unwrap().workspace_revision();
    let response = adapter.dispatch(&registry, &device, ack(2));
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    let checkpoint = temp.path().join("project-workspace.json");
    let bytes = std::fs::read(&checkpoint).unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        saved["remote_workspace"]["event_acks"][0]["cursor"]["next_sequence"],
        2
    );
    assert_eq!(
        saved["remote_workspace"]["event_acks"][0]["device_id"],
        device.id()
    );
    for next in [2, 1, 0] {
        let response = adapter.dispatch(&registry, &device, ack(next));
        assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
        let Message::Response { result } = response.payload else {
            panic!("ack")
        };
        assert_eq!(result["cursor"]["next_sequence"], 2);
        assert_eq!(std::fs::read(&checkpoint).unwrap(), bytes);
    }
    std::thread::scope(|scope| {
        let pending = scope.spawn(|| mailbox_adapter.dispatch(&registry, &device, ack(2)));
        let until = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while !pending.is_finished() && std::time::Instant::now() < until {
            mailbox.drain(&mut pool.lock().unwrap());
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(
            pending.join().unwrap().execution_state,
            Some(ExecutionState::Succeeded)
        );
    });
    assert_eq!(std::fs::read(&checkpoint).unwrap(), bytes);
    assert_eq!(
        adapter.dispatch(&registry, &device, ack(u64::MAX)).code,
        Some(ErrorCode::InvalidRequest)
    );
    let mut foreign = ack(2);
    foreign.project_id = Some("foreign".into());
    assert_eq!(
        adapter.dispatch(&registry, &device, foreign).code,
        Some(ErrorCode::Unauthorized)
    );
    let mut wrong_session = ack(2);
    wrong_session.session_id = Some("wrong".into());
    wrong_session.payload = Message::Ack {
        cursor: zenpi::remote::protocol::EventCursor {
            session_id: "wrong".into(),
            next_sequence: 2,
        },
    };
    assert_eq!(
        adapter.dispatch(&registry, &device, wrong_session).code,
        Some(ErrorCode::OwnerMismatch)
    );
    let held = owner.lock().unwrap();
    assert_eq!(
        adapter.dispatch(&registry, &device, ack(2)).code,
        Some(ErrorCode::Busy)
    );
    assert_eq!(std::fs::read(&checkpoint).unwrap(), bytes);
    let mut subscribed = Envelope::new(
        Channel::Control,
        Message::Subscribe {
            cursor: zenpi::remote::protocol::EventCursor {
                session_id: session_id.clone(),
                next_sequence: 2,
            },
            limit: 128,
            duration_ms: 1000,
        },
    );
    subscribed.id = Some("busy-ack-view".into());
    subscribed.project_id = Some(id.clone());
    subscribed.session_id = Some(session_id.clone());
    let view = zenpi::remote::subscription::view(&subscribed, |read| {
        adapter.dispatch(&registry, &device, read)
    })
    .unwrap()
    .unwrap();
    assert_eq!(view["subtabs"]["session_id"], session_id);
    assert_eq!(
        view["listing"]["workspace_revision"],
        view["subtabs"]["workspace_revision"]
    );
    assert_eq!(std::fs::read(&checkpoint).unwrap(), bytes);
    drop(held);
    assert_eq!(pool.lock().unwrap().workspace_revision(), revision);
    let mut restored = ProjectOwnerPool::new(owner.clone()).unwrap();
    restored.restore_checkpoint().unwrap();
    let restored_adapter = HostAdapter::new(Arc::new(Mutex::new(restored)));
    assert_eq!(
        restored_adapter
            .dispatch(&registry, &device, ack(1))
            .execution_state,
        Some(ExecutionState::Succeeded)
    );
    assert_eq!(std::fs::read(&checkpoint).unwrap(), bytes);
    let mut corrupt: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let duplicate = corrupt["remote_workspace"]["event_acks"][0].clone();
    corrupt["remote_workspace"]["event_acks"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    std::fs::write(&checkpoint, serde_json::to_vec(&corrupt).unwrap()).unwrap();
    assert!(
        ProjectOwnerPool::new(owner.clone())
            .unwrap()
            .restore_checkpoint()
            .is_err()
    );
    std::fs::write(&checkpoint, &bytes).unwrap();
    for invalid in [
        json!([{"device_id":"bad", "project_id":id, "cursor":{"session_id":session_id, "next_sequence":2}}]),
        json!([{"device_id":device.id(), "project_id":id, "cursor":{"session_id":session_id, "next_sequence":-1}}]),
        json!([{"device_id":device.id(), "project_id":id, "cursor":{"session_id":session_id, "next_sequence":2, "extra":true}}]),
        json!(null),
    ] {
        let mut corrupt = saved.clone();
        corrupt["remote_workspace"]["event_acks"] = invalid;
        std::fs::write(&checkpoint, serde_json::to_vec(&corrupt).unwrap()).unwrap();
        assert!(
            ProjectOwnerPool::new(owner.clone())
                .unwrap()
                .restore_checkpoint()
                .is_err()
        );
    }
    let mut legacy = saved.clone();
    legacy["remote_workspace"]
        .as_object_mut()
        .unwrap()
        .remove("event_acks");
    std::fs::write(&checkpoint, serde_json::to_vec(&legacy).unwrap()).unwrap();
    assert!(
        ProjectOwnerPool::new(owner.clone())
            .unwrap()
            .restore_checkpoint()
            .is_ok()
    );
    let mut full = saved.clone();
    let slots: Vec<_> = (0..128)
        .map(|index| {
            json!({"device_id":format!("{index:064x}"),
        "project_id":id, "cursor":{"session_id":session_id, "next_sequence":2}})
        })
        .collect();
    full["remote_workspace"]["event_acks"] = json!(slots);
    let full_bytes = serde_json::to_vec(&full).unwrap();
    std::fs::write(&checkpoint, &full_bytes).unwrap();
    let mut full_pool = ProjectOwnerPool::new(owner.clone()).unwrap();
    full_pool.restore_checkpoint().unwrap();
    let full_adapter = HostAdapter::new(Arc::new(Mutex::new(full_pool)));
    assert_eq!(
        full_adapter.dispatch(&registry, &device, ack(2)).code,
        Some(ErrorCode::ResourceLimit)
    );
    assert_eq!(std::fs::read(&checkpoint).unwrap(), full_bytes);
    full["remote_workspace"]["event_acks"]
        .as_array_mut()
        .unwrap()
        .push(json!({"device_id":format!("{:064x}", 128), "project_id":id,
            "cursor":{"session_id":session_id, "next_sequence":2}}));
    std::fs::write(&checkpoint, serde_json::to_vec(&full).unwrap()).unwrap();
    assert!(
        ProjectOwnerPool::new(owner.clone())
            .unwrap()
            .restore_checkpoint()
            .is_err()
    );
    std::fs::write(&checkpoint, &bytes).unwrap();
    let mut resume = Envelope::new(
        Channel::Control,
        Message::Resume {
            cursor: zenpi::remote::protocol::EventCursor {
                session_id: session_id.clone(),
                next_sequence: 0,
            },
            limit: 128,
        },
    );
    resume.id = Some("resume".into());
    resume.project_id = Some(id.clone());
    resume.session_id = Some(session_id.clone());
    let wire =
        HostAdapter::wire_frames(adapter.dispatch(&registry, &device, resume.clone())).unwrap();
    assert_eq!(wire.len(), 4);
    assert!(matches!(wire[0].payload, Message::Event { .. }));
    assert_eq!(wire[0].sequence, Some(0));
    assert_eq!(wire[1].sequence, Some(1));
    for frame in &wire {
        frame.validate().unwrap();
    }
    let mut wrong = resume.clone();
    wrong.project_id = Some("foreign".into());
    assert_eq!(
        adapter.dispatch(&registry, &device, wrong).code,
        Some(ErrorCode::Unauthorized)
    );
    let held = owner.lock().unwrap();
    assert_eq!(
        adapter
            .dispatch(&registry, &device, request(Operation::Inspect, Some(&id)))
            .code,
        Some(ErrorCode::Busy)
    );
    drop(held);
    let mut wrong_session = request(Operation::Inspect, Some(&id));
    wrong_session.session_id = Some("wrong".into());
    assert_eq!(
        adapter.dispatch(&registry, &device, wrong_session).code,
        Some(ErrorCode::OwnerMismatch)
    );
    for index in 2..258 {
        owner
            .lock()
            .unwrap()
            .session_mut()
            .append_turn(Turn::new(
                format!("tail-{index}"),
                TurnRole::Assistant,
                "tail",
            ))
            .unwrap();
    }
    let path = owner.lock().unwrap().session().path().to_path_buf();
    owner.lock().unwrap().resume_session(path).unwrap();
    // ACK storage uses the existing checkpoint CAS, not a last-writer-wins overwrite.
    let mut changed = bytes.clone();
    changed.push(b'\n');
    std::fs::write(&checkpoint, &changed).unwrap();
    assert_eq!(
        adapter.dispatch(&registry, &device, ack(3)).code,
        Some(ErrorCode::InternalError)
    );
    assert_eq!(std::fs::read(&checkpoint).unwrap(), changed);
    let response = adapter.dispatch(&registry, &device, ack(2));
    let Message::Response { result } = response.payload else {
        panic!("ack")
    };
    assert_eq!(result["cursor"]["next_sequence"], 2);
    assert_eq!(std::fs::read(&checkpoint).unwrap(), changed);
    std::fs::write(&checkpoint, &bytes).unwrap();
    let gap = HostAdapter::wire_frames(adapter.dispatch(&registry, &device, resume)).unwrap();
    assert_eq!(gap.len(), 2);
    let Message::Snapshot { cursor, state } = &gap[0].payload else {
        panic!("gap must have a snapshot")
    };
    assert_eq!(cursor.next_sequence, 259);
    assert_eq!(state["transcript"].as_array().unwrap().len(), 64);
    assert_eq!(state["transcript_truncated"], true);
    for frame in gap {
        frame.validate().unwrap();
    }
    assert_eq!(
        adapter
            .dispatch(
                &registry,
                &device,
                request(Operation::Inspect, Some("outside"))
            )
            .code,
        Some(ErrorCode::Unauthorized)
    );
    registry.revoke(&record.id).unwrap();
    assert_eq!(
        zenpi::remote::subscription::view(&subscription, |read| adapter
            .dispatch(&registry, &device, read)),
        Err(ErrorCode::Unauthorized)
    );
    assert_eq!(
        adapter.dispatch(&registry, &device, ack(2)).code,
        Some(ErrorCode::Unauthorized)
    );
    let mut revoked_frames = Vec::new();
    zenpi::remote::subscription::serve(
        &subscription,
        |resume| adapter.dispatch(&registry, &device, resume),
        |frame| {
            revoked_frames.push(frame);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(revoked_frames.len(), 1);
    assert_eq!(revoked_frames[0].code, Some(ErrorCode::Unauthorized));
    assert_eq!(std::fs::read(&checkpoint).unwrap(), bytes);
    assert_eq!(
        adapter
            .dispatch(&registry, &device, request(Operation::WorkspaceList, None))
            .code,
        Some(ErrorCode::Unauthorized)
    );
}
