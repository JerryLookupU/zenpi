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
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use zenpi::{
    approval::{
        ApprovalCoordinator, ApprovalDecision, ApprovalPolicy, ApprovalRequest, ApprovalResponse,
    },
    backend::OpenAiCompatibleBackend,
    core::Agent,
    project_workspace::ProjectOwnerPool,
    remote::{
        auth::{Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message},
        frame::{read_frame, write_frame},
        host_adapter::HostAdapter,
        protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message, Operation},
    },
    session::SessionStore,
    tools::{ToolContext, ToolOrigin, ToolSideEffect},
};

fn pool(path: &Path, root: &Path) -> (ProjectOwnerPool, ApprovalCoordinator) {
    let backend = OpenAiCompatibleBackend::new("http://127.0.0.1:9", None, "fixture").unwrap();
    let mut agent = Agent::new(
        SessionStore::open_in_workspace(path, root).unwrap(),
        Box::new(backend),
    );
    agent.set_attachment_workspace(ToolContext::new(root).unwrap());
    let coordinator = agent.approval_coordinator().unwrap();
    let mut pool = ProjectOwnerPool::new(Arc::new(Mutex::new(agent))).unwrap();
    pool.restore_checkpoint().unwrap();
    (pool, coordinator)
}

struct Pending {
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<Result<ApprovalResponse, zenpi::approval::ApprovalError>>>,
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_approval_decisions_are_one_shot_digest_fenced_and_restart_durable() {
    for decision in [ApprovalDecision::Allow, ApprovalDecision::Deny] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let path = root.join("session.jsonl");
        let (mut host, coordinator) = pool(&path, &root);
        let project = host.active_context().project_id.clone();
        let session = host.active_context().session_id.clone();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let random = ring::rand::SystemRandom::new();
        let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
        let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random)
            .unwrap();
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
                vec![Capability::Read, Capability::Approval],
                vec![project.clone()],
            )
            .unwrap();
        let authenticate = || {
            let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
            let signature = URL_SAFE_NO_PAD.encode(
                key.sign(&random, &challenge.proof_message(&record.id))
                    .unwrap()
                    .as_ref(),
            );
            challenge
                .authenticate(&registry, &record.id, &signature)
                .unwrap()
        };
        let device = authenticate();
        let read = |host: &mut ProjectOwnerPool,
                    device: &zenpi::remote::auth::AuthenticatedDevice,
                    operation: Operation| {
            let mut request = Envelope::new(
                Channel::Control,
                Message::Request {
                    operation,
                    arguments: json!({}),
                },
            );
            request.id = Some("read".into());
            request.project_id = Some(project.clone());
            request.session_id = Some(session.clone());
            let response = HostAdapter::dispatch_owned(host, &registry, device, request);
            assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
            let Message::Response { result } = response.payload else {
                panic!("read response")
            };
            result
        };
        let snapshot = read(&mut host, &device, Operation::Inspect);
        let journal = std::fs::read(&path).unwrap();
        let worker = coordinator.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let mut pending = Pending {
            stop,
            join: Some(std::thread::spawn(move || {
                worker.request_response(
                    ApprovalRequest {
                        request_id: "approval-target".into(),
                        turn_id: "turn".into(),
                        call_id: "call".into(),
                        tool: "write_file".into(),
                        side_effect: ToolSideEffect::WorkspaceWrite,
                        arguments: json!({"content":"INTEROP_APPROVAL_PRIVATE"}),
                        preview: None,
                        origin: ToolOrigin::AgentTool,
                        policy_digest: None,
                        lease_id: None,
                    },
                    &mut ApprovalPolicy::default(),
                    &|| stopping.load(Ordering::Acquire),
                )
            })),
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        while !coordinator.is_pending("approval-target") && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(coordinator.is_pending("approval-target"));
        let seed = json!({"decision":decision,"snapshot":snapshot,"listing":read(&mut host, &device, Operation::WorkspaceList)});
        assert!(!seed.to_string().contains("INTEROP_APPROVAL_PRIVATE"));
        let seed_path = root.join("seed.json");
        let command_path = root.join("command.bin");
        let results_path = root.join("results.json");
        std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
        let run_java = |phase: &str| {
            let mut child = Command::new(std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap())
                .args([
                    format!("-Dzenpi.approval.phase={phase}"),
                    format!("-Dzenpi.approval.seed={}", seed_path.display()),
                    format!("-Dzenpi.approval.command={}", command_path.display()),
                    format!("-Dzenpi.approval.results={}", results_path.display()),
                    "-cp".into(),
                    std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap(),
                    "org.junit.runner.JUnitCore".into(),
                    "zenpi.AndroidRustApprovalTest".into(),
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
        let bytes = std::fs::read(&command_path).unwrap();
        let mut input = bytes.as_slice();
        let command = read_frame(&mut input).unwrap().unwrap();
        assert!(input.is_empty());
        assert_eq!(command.project_id.as_deref(), Some(project.as_str()));
        assert_eq!(command.session_id.as_deref(), Some(session.as_str()));
        let mut exchanges = Vec::new();
        let mut capture = |request: &Envelope, response: &Envelope| {
            let mut wire = Vec::new();
            write_frame(&mut wire, response).unwrap();
            exchanges.push(json!({"request":request,"wire_base64":STANDARD.encode(wire)}));
        };
        for (kind, code) in [
            ("digest", ErrorCode::InvalidRequest),
            ("owner", ErrorCode::OwnerMismatch),
            ("revision", ErrorCode::RevisionConflict),
            ("permission", ErrorCode::Unauthorized),
        ] {
            let mut invalid = command.clone();
            invalid.id = Some(format!("bad-{kind}"));
            match kind {
                "digest" => {
                    if let Message::Request { arguments, .. } = &mut invalid.payload {
                        arguments["digest"] = json!("b".repeat(64));
                    }
                }
                "owner" => invalid.required_owner = Some("b".repeat(64)),
                "revision" => invalid.expected_revision = Some(999),
                _ => {
                    registry
                        .set_permissions(
                            &record.id,
                            2,
                            vec![Capability::Read, Capability::Control],
                            vec![project.clone()],
                        )
                        .unwrap();
                }
            }
            let response = if kind == "permission" {
                HostAdapter::dispatch_owned(&mut host, &registry, &authenticate(), invalid.clone())
            } else {
                HostAdapter::dispatch_owned(&mut host, &registry, &device, invalid.clone())
            };
            assert_eq!(response.code, Some(code));
            capture(&invalid, &response);
            assert!(coordinator.is_pending("approval-target"));
        }
        registry
            .set_permissions(
                &record.id,
                3,
                vec![Capability::Read, Capability::Approval],
                vec![project.clone()],
            )
            .unwrap();
        let device = authenticate();
        let held_owner = host.owner(&project).unwrap();
        let held = held_owner.lock().unwrap();
        let accepted = HostAdapter::dispatch_owned(&mut host, &registry, &device, command.clone());
        assert_eq!(accepted.execution_state, Some(ExecutionState::Accepted));
        capture(&command, &accepted);
        assert_eq!(
            pending
                .join
                .take()
                .unwrap()
                .join()
                .unwrap()
                .unwrap()
                .decision,
            decision
        );
        let audit = coordinator.accepted("approval-target").unwrap();
        let actor = audit.remote.unwrap();
        assert_eq!(actor.device_id, record.id);
        assert_eq!(actor.project_id, project);
        assert_eq!(actor.session_id, session);
        assert_eq!(actor.request_id, command.id.as_deref().unwrap());
        assert_eq!(actor.request_digest, audit.request.digest().unwrap());
        coordinator.mark_persisted("approval-target").unwrap();
        let duplicate = HostAdapter::dispatch_owned(&mut host, &registry, &device, command.clone());
        assert_eq!(duplicate, accepted);
        capture(&command, &duplicate);
        let checkpoint_path = root.join("remote-state.json");
        let checkpoint = std::fs::read(&checkpoint_path).unwrap();
        let saved: Value = serde_json::from_slice(&checkpoint).unwrap();
        assert_eq!(saved["approval_receipts"].as_array().unwrap().len(), 1);
        assert!(!saved.to_string().contains("INTEROP_APPROVAL_PRIVATE"));
        let mut changed = command.clone();
        if let Message::Request { arguments, .. } = &mut changed.payload {
            arguments["decision"] = json!(if decision == ApprovalDecision::Allow {
                "deny"
            } else {
                "allow"
            });
        }
        let conflict = HostAdapter::dispatch_owned(&mut host, &registry, &device, changed.clone());
        assert_eq!(conflict.code, Some(ErrorCode::RequestIdConflict));
        capture(&changed, &conflict);
        assert_eq!(std::fs::read(&checkpoint_path).unwrap(), checkpoint);
        drop(held);
        drop(held_owner);
        drop(host);
        let (mut restored, _) = pool(&path, &root);
        let replay =
            HostAdapter::dispatch_owned(&mut restored, &registry, &device, command.clone());
        assert_eq!(replay, accepted);
        capture(&command, &replay);
        assert_eq!(std::fs::read(&checkpoint_path).unwrap(), checkpoint);
        assert_eq!(std::fs::read(&path).unwrap(), journal);
        let results = json!({"exchanges":exchanges,"listing":read(&mut restored, &device, Operation::WorkspaceList)});
        std::fs::write(&results_path, serde_json::to_vec(&results).unwrap()).unwrap();
        run_java("validate");
    }
}
