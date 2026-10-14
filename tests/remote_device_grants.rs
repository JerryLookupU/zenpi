mod support;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
use std::{
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use zenpi::{
    approval::{ApprovalMode, ApprovalPolicy},
    backend::{Backend, BackendError, Completion, CompletionRequest},
    core::Agent,
    project_workspace::ProjectOwnerPool,
    remote::{
        auth::{
            AuthenticatedDevice, Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message,
        },
        headless_host::HeadlessHost,
        pairing::HostIdentity,
        protocol::{Channel, Envelope, ExecutionState, Message, Operation},
    },
    session::SessionStore,
    tools::{SideEffectPolicy, ToolCall, ToolContext, ToolRegistry},
};

fn sign(key: &EcdsaKeyPair, message: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(
        key.sign(&ring::rand::SystemRandom::new(), message)
            .unwrap()
            .as_ref(),
    )
}
fn device(registry: &DeviceRegistry, fingerprint: &str) -> (EcdsaKeyPair, String) {
    paired(registry, fingerprint, &[])
}
/// Pairs through the one-use QR proof; `mounted` is what the host had attached.
fn paired(
    registry: &DeviceRegistry,
    fingerprint: &str,
    mounted: &[String],
) -> (EcdsaKeyPair, String) {
    let random = ring::rand::SystemRandom::new();
    let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random).unwrap();
    let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    let proof = sign(
        &key,
        &pairing_proof_message(fingerprint, "fixture", "pair", "phone", &public),
    );
    let record = registry
        .verify_pair_and_grant("fixture", "pair", "phone", &public, &proof, mounted, || {
            true
        })
        .unwrap();
    (key, record.id)
}
fn authenticate(
    registry: &DeviceRegistry,
    fingerprint: &str,
    key: &EcdsaKeyPair,
    id: &str,
) -> AuthenticatedDevice {
    let mut challenge = DeviceChallenge::new(fingerprint.to_owned()).unwrap();
    let signature = sign(key, &challenge.proof_message(id));
    challenge.authenticate(registry, id, &signature).unwrap()
}
fn host_pair(identity: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_zenpi"))
        .args(["host", "pair", "--identity"])
        .arg(identity)
        .args(args)
        .env(
            "ZENPI_HOME",
            identity.parent().unwrap().join("absent-config"),
        )
        .output()
        .unwrap()
}

#[test]
fn pairing_grants_mounted_project_control_and_approval_and_repair_extends_without_reviving() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("identity.json");
    let host = HostIdentity::load_or_create(&path).unwrap();
    let fingerprint = host.fingerprint();
    let registry =
        DeviceRegistry::new(path.with_extension("devices.json"), fingerprint.clone()).unwrap();
    let (key, id) = paired(&registry, &fingerprint, &["p1".into()]);
    let (_, unmounted) = device(&registry, &fingerprint);
    let record = registry.lookup(&id).unwrap();
    assert_eq!(
        record.capabilities,
        vec![Capability::Read, Capability::Control, Capability::Approval]
    );
    assert_eq!(record.projects, vec!["p1"]);
    assert_eq!(record.revision, 1);
    assert!(registry.controls_any(&id, &["p1".into()]));
    assert!(!registry.controls_any(&id, &["p2".into()]));
    // Without a mounted project nothing is granted and control is truthfully absent.
    let bare = registry.lookup(&unmounted).unwrap();
    assert_eq!(bare.capabilities, vec![Capability::Read]);
    assert!(bare.projects.is_empty());
    assert!(!registry.controls_any(&unmounted, &["p1".into()]));
    let old = authenticate(&registry, &fingerprint, &key, &id);
    for operation in [Operation::Inspect, Operation::Prompt, Operation::Approve] {
        assert!(registry.authorize(&old, "p1", operation).is_ok());
        assert!(registry.authorize(&old, "p2", operation).is_err());
    }
    // A fresh QR scan from a host mounting another project extends the grant
    // and fences the previous connection by revision.
    let public = registry.lookup(&id).unwrap().public_key;
    let proof = sign(
        &key,
        &pairing_proof_message(&fingerprint, "second", "pair-2", "phone", &public),
    );
    let extended = registry
        .verify_pair_and_grant(
            "second",
            "pair-2",
            "phone",
            &public,
            &proof,
            &["p2".into(), "p1".into()],
            || true,
        )
        .unwrap();
    assert_eq!(extended.projects, vec!["p1", "p2"]);
    assert_eq!(extended.revision, 2);
    assert!(registry.authorize(&old, "p1", Operation::Inspect).is_err());
    let current = authenticate(&registry, &fingerprint, &key, &id);
    for operation in [Operation::Inspect, Operation::Prompt, Operation::Approve] {
        assert!(registry.authorize(&current, "p2", operation).is_ok());
    }
    // Re-pairing an unchanged grant is idempotent: same bytes, same revision.
    let before = std::fs::read(path.with_extension("devices.json")).unwrap();
    let proof = sign(
        &key,
        &pairing_proof_message(&fingerprint, "third", "pair-3", "phone", &public),
    );
    registry
        .verify_pair_and_grant(
            "third",
            "pair-3",
            "phone",
            &public,
            &proof,
            &["p1".into()],
            || true,
        )
        .unwrap();
    assert_eq!(
        std::fs::read(path.with_extension("devices.json")).unwrap(),
        before
    );
    // Revocation is effective on the next frame and a later scan never revives it.
    assert!(registry.is_current(&current).unwrap());
    let revoked = host_pair(&path, &["--revoke-device", &id]);
    assert!(revoked.status.success());
    assert!(!registry.is_current(&current).unwrap());
    assert!(
        registry
            .authorize(&current, "p1", Operation::Inspect)
            .is_err()
    );
    let before = std::fs::read(path.with_extension("devices.json")).unwrap();
    let proof = sign(
        &key,
        &pairing_proof_message(&fingerprint, "fourth", "pair-4", "phone", &public),
    );
    assert!(
        registry
            .verify_pair_and_grant(
                "fourth",
                "pair-4",
                "phone",
                &public,
                &proof,
                &["p1".into()],
                || panic!("a revoked key must not consume an invite"),
            )
            .is_err()
    );
    assert_eq!(
        std::fs::read(path.with_extension("devices.json")).unwrap(),
        before
    );
    assert!(!temp.path().join("absent-config").exists());
}

#[test]
fn removed_grant_clis_are_refused_without_changing_the_registry() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("identity.json");
    let identity = HostIdentity::load_or_create(&path).unwrap();
    let registry =
        DeviceRegistry::new(path.with_extension("devices.json"), identity.fingerprint()).unwrap();
    let (_, id) = device(&registry, &identity.fingerprint());
    let before = std::fs::read(path.with_extension("devices.json")).unwrap();
    for flag in [
        "--grant-device-read",
        "--grant-device-control",
        "--grant-device-approval",
        "--grant-device-control-approval",
    ] {
        let output = host_pair(&path, &[flag, &id, "--project", "p"]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(flag));
        assert_eq!(
            std::fs::read(path.with_extension("devices.json")).unwrap(),
            before
        );
    }
}

#[test]
fn production_json_device_listing_exposes_exact_permission_view_without_key_or_write() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("identity.json");
    let identity = HostIdentity::load_or_create(&path).unwrap();
    let registry =
        DeviceRegistry::new(path.with_extension("devices.json"), identity.fingerprint()).unwrap();
    let (_, first) = paired(
        &registry,
        &identity.fingerprint(),
        &["project-one".into(), "project-two".into()],
    );
    let (_, second) = device(&registry, &identity.fingerprint());
    registry.revoke(&second).unwrap();
    let before = std::fs::read(path.with_extension("devices.json")).unwrap();
    let key = std::fs::read(&path).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_zenpi"))
        .args(["host", "pair", "--identity"])
        .arg(&path)
        .args(["--list-devices", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 1);
    let entries = value["devices"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    for entry in entries {
        let keys: std::collections::BTreeSet<_> = entry
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            std::collections::BTreeSet::from([
                "id",
                "name",
                "revoked",
                "revision",
                "capabilities",
                "projects"
            ])
        );
        let record = registry.lookup(entry["id"].as_str().unwrap()).unwrap();
        assert_eq!(
            entry["capabilities"],
            serde_json::json!(record.capabilities)
        );
        assert_eq!(entry["projects"], serde_json::json!(record.projects));
        assert_eq!(entry["revision"], record.revision);
        assert_eq!(entry["revoked"], record.revoked);
        assert_eq!(entry["name"], record.name);
        assert!(!String::from_utf8_lossy(&output.stdout).contains(&record.public_key));
    }
    assert_eq!(
        std::fs::read(path.with_extension("devices.json")).unwrap(),
        before
    );
    assert_eq!(std::fs::read(&path).unwrap(), key);
    let text = Command::new(env!("CARGO_BIN_EXE_zenpi"))
        .args(["host", "pair", "--identity"])
        .arg(&path)
        .arg("--list-devices")
        .output()
        .unwrap();
    assert!(text.status.success());
    assert!(
        String::from_utf8(text.stdout)
            .unwrap()
            .contains("revoked=true revision=")
    );
    assert!(zenpi::core::parse_args(["host", "pair", "--json"]).is_err());
    assert!(
        zenpi::core::parse_args(["host", "pair", "--json", "--revoke-device", &first]).is_err()
    );
    for args in [
        vec![
            "host",
            "pair",
            "--list-devices",
            "--json",
            "--revoke-device",
            &first,
        ],
        vec![
            "host",
            "pair",
            "--list-devices",
            "--json",
            "--list-projects",
            "--workspace",
            "/workspace",
            "--session",
            "/session.jsonl",
        ],
    ] {
        assert!(zenpi::core::parse_args(args).is_err());
    }
    std::fs::write(
        path.with_extension("devices.json"),
        b"INVALID_REGISTRY_PRIVATE_FIXTURE",
    )
    .unwrap();
    let corrupt = Command::new(env!("CARGO_BIN_EXE_zenpi"))
        .args(["host", "pair", "--identity"])
        .arg(&path)
        .args(["--list-devices", "--json"])
        .output()
        .unwrap();
    assert!(!corrupt.status.success());
    assert!(corrupt.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&corrupt.stderr).contains("INVALID_REGISTRY_PRIVATE_FIXTURE"));
    assert_eq!(
        std::fs::read(path.with_extension("devices.json")).unwrap(),
        b"INVALID_REGISTRY_PRIVATE_FIXTURE"
    );
}

struct WriteBackend(Arc<AtomicUsize>);
impl Backend for WriteBackend {
    crate::support::controlled_local_backend!();
    fn name(&self) -> &str {
        "grant-approval-fixture"
    }
    fn complete(&self, _: CompletionRequest<'_>) -> Result<Completion, BackendError> {
        if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
            let mut completion = Completion::text("");
            completion.tool_calls = vec![ToolCall {
                id: "write".into(),
                name: "write_file".into(),
                arguments: serde_json::json!({"path":"approved.txt","content":"ONE_PHONE_APPROVED"}),
            }];
            Ok(completion)
        } else {
            Ok(Completion::text("done"))
        }
    }
}
fn request(
    project: &str,
    session: &str,
    owner: &str,
    revision: u64,
    id: &str,
    operation: Operation,
    arguments: serde_json::Value,
) -> Envelope {
    let mut request = Envelope::new(
        Channel::Control,
        Message::Request {
            operation,
            arguments,
        },
    );
    request.id = Some(id.into());
    request.project_id = Some(project.into());
    request.session_id = Some(session.into());
    request.required_owner = Some(owner.into());
    request.expected_revision = Some(revision);
    request
}

#[test]
fn one_paired_phone_submits_approves_and_retries_without_duplicate_tool_effect() {
    exercise_prompt_approval("allow");
}

#[test]
fn one_paired_phone_denies_tool_without_losing_prompt_control() {
    exercise_prompt_approval("deny");
}

fn exercise_prompt_approval(decision: &str) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("identity.json");
    let identity = HostIdentity::load_or_create(&path).unwrap();
    let registry = Arc::new(
        DeviceRegistry::new(path.with_extension("devices.json"), identity.fingerprint()).unwrap(),
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let mut agent = Agent::new(
        SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        Box::new(WriteBackend(calls.clone())),
    );
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(&root).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    agent.set_approval_policy(ApprovalPolicy {
        mode: ApprovalMode::Always,
        ..Default::default()
    });
    let owner = Arc::new(Mutex::new(agent));
    let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
    pool.restore_checkpoint().unwrap();
    let project = pool.active_context().project_id.clone();
    let session = pool.active_context().session_id.clone();
    let epoch = pool.workspace_owner().to_owned();
    let revision = pool.workspace_revision();
    let (key, id) = paired(
        &registry,
        &identity.fingerprint(),
        std::slice::from_ref(&project),
    );
    let phone = authenticate(&registry, &identity.fingerprint(), &key, &id);
    let (adapter, host) = HeadlessHost::start(pool);
    let prompt = request(
        &project,
        &session,
        &epoch,
        revision,
        "prompt",
        Operation::Prompt,
        serde_json::json!({"text":"write the note"}),
    );
    let admitted = adapter.dispatch(&registry, &phone, prompt.clone());
    assert_eq!(admitted.execution_state, Some(ExecutionState::Accepted));
    let until = Instant::now() + Duration::from_secs(5);
    let pending = loop {
        let mut listing = Envelope::new(
            Channel::Control,
            Message::Request {
                operation: Operation::WorkspaceList,
                arguments: serde_json::json!({}),
            },
        );
        listing.id = Some("listing".into());
        let response = adapter.dispatch(&registry, &phone, listing);
        assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
        if let Message::Response { result } = &response.payload
            && result["projects"][0]["pending_approvals"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        {
            assert_eq!(result["projects"][0]["prompt_control"], true);
            assert_eq!(result["projects"][0]["approval_control"], true);
            break result["projects"][0]["pending_approvals"][0].clone();
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    };
    assert!(!root.join("approved.txt").exists());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let approve = request(
        &project,
        &session,
        &epoch,
        revision,
        "approval",
        Operation::Approve,
        serde_json::json!({"request_id":pending["request_id"],"digest":pending["digest"],"decision":decision}),
    );
    let receipt = adapter.dispatch(&registry, &phone, approve.clone());
    assert_eq!(receipt.execution_state, Some(ExecutionState::Accepted));
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let mut listing = Envelope::new(
            Channel::Control,
            Message::Request {
                operation: Operation::WorkspaceList,
                arguments: serde_json::json!({}),
            },
        );
        listing.id = Some("completion-listing".into());
        let result = adapter.dispatch(&registry, &phone, listing);
        let completed = matches!(&result.payload,Message::Response{result} if result["projects"][0]["prompt_statuses"].as_array().is_some_and(|statuses|
            statuses.iter().any(|status|status["request_id"]=="prompt" && status["state"]=="succeeded")));
        if completed {
            break;
        }
        assert!(Instant::now() < until, "expected succeeded: {result:?}");
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(adapter.dispatch(&registry, &phone, approve), receipt);
    assert_eq!(
        adapter.dispatch(&registry, &phone, prompt).payload,
        admitted.payload
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(root.join("approved.txt").exists(), decision == "allow");
    if decision == "allow" {
        assert_eq!(
            std::fs::read_to_string(root.join("approved.txt")).unwrap(),
            "ONE_PHONE_APPROVED"
        );
    }
    host.shutdown();
    let agent = owner.lock().unwrap();
    let events = agent.session().events();
    let approvals: Vec<_> = events
        .iter()
        .filter(|event| event["type"] == "approval_resolved")
        .collect();
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0]["decision"], decision);
    assert_eq!(approvals[0]["remember"], false);
    assert_eq!(approvals[0]["remote"]["device_id"], id);
    assert_eq!(approvals[0]["remote"]["request_id"], "approval");
    assert_eq!(approvals[0]["remote"]["project_id"], project);
    assert_eq!(approvals[0]["remote"]["session_id"], session);
    assert_eq!(approvals[0]["remote"]["request_digest"], pending["digest"]);
}
