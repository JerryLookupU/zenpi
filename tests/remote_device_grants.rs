#![cfg(feature = "remote")]

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
        .verify_pair_and_register("fixture", "pair", "phone", &public, &proof, || true)
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
fn grant(
    identity: &std::path::Path,
    id: &str,
    operation: &str,
    projects: &[&str],
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_zenpi"));
    command
        .args(["host", "pair", "--identity"])
        .arg(identity)
        .args([operation, id]);
    for project in projects {
        command.args(["--project", project]);
    }
    command
        .env(
            "ZENPI_HOME",
            identity.parent().unwrap().join("absent-config"),
        )
        .output()
        .unwrap()
}

#[test]
fn production_combined_grant_persists_scopes_fences_downgrades_and_cannot_restore_revoked_device() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("identity.json");
    let host = HostIdentity::load_or_create(&path).unwrap();
    let fingerprint = host.fingerprint();
    let registry =
        DeviceRegistry::new(path.with_extension("devices.json"), fingerprint.clone()).unwrap();
    let (key, id) = device(&registry, &fingerprint);
    let (_, foreign) = device(&registry, &fingerprint);
    let foreign_before = registry.lookup(&foreign).unwrap();
    let old = authenticate(&registry, &fingerprint, &key, &id);
    let identity_before = std::fs::read(&path).unwrap();
    let output = grant(&path, &id, "--grant-device-control-approval", &["p1", "p2"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Read/Control/Approval permission granted")
    );
    assert!(output.stderr.is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), identity_before);
    assert!(!temp.path().join("absent-config").exists());
    assert_eq!(registry.lookup(&foreign).unwrap(), foreign_before);
    assert!(registry.authorize(&old, "p1", Operation::Inspect).is_err());
    let restored =
        DeviceRegistry::new(path.with_extension("devices.json"), fingerprint.clone()).unwrap();
    let record = restored.lookup(&id).unwrap();
    assert_eq!(
        record.capabilities,
        vec![Capability::Read, Capability::Control, Capability::Approval]
    );
    assert_eq!(record.projects, vec!["p1", "p2"]);
    assert_eq!(record.revision, 2);
    let current = authenticate(&restored, &fingerprint, &key, &id);
    for operation in [Operation::Inspect, Operation::Prompt, Operation::Approve] {
        assert!(restored.authorize(&current, "p1", operation).is_ok());
        assert!(restored.authorize(&current, "p2", operation).is_ok());
        assert!(
            restored
                .authorize(&current, "foreign-project", operation)
                .is_err()
        );
    }
    for (action, capabilities, prompt, approval) in [
        (
            "--grant-device-control",
            vec![Capability::Read, Capability::Control],
            true,
            false,
        ),
        (
            "--grant-device-approval",
            vec![Capability::Read, Capability::Approval],
            false,
            true,
        ),
        ("--grant-device-read", vec![Capability::Read], false, false),
    ] {
        let previous = authenticate(&registry, &fingerprint, &key, &id);
        assert!(grant(&path, &id, action, &["p1"]).status.success());
        assert!(
            registry
                .authorize(&previous, "p1", Operation::Inspect)
                .is_err()
        );
        assert_eq!(registry.lookup(&id).unwrap().capabilities, capabilities);
        let current = authenticate(&registry, &fingerprint, &key, &id);
        assert_eq!(
            registry
                .authorize(&current, "p1", Operation::Prompt)
                .is_ok(),
            prompt
        );
        assert_eq!(
            registry
                .authorize(&current, "p1", Operation::Approve)
                .is_ok(),
            approval
        );
        assert!(
            registry
                .authorize(&current, "p2", Operation::Inspect)
                .is_err()
        );
    }
    registry.revoke(&id).unwrap();
    let before = std::fs::read(path.with_extension("devices.json")).unwrap();
    assert!(
        !grant(&path, &id, "--grant-device-control-approval", &["p1"])
            .status
            .success()
    );
    assert_eq!(
        std::fs::read(path.with_extension("devices.json")).unwrap(),
        before
    );
    assert_eq!(registry.lookup(&foreign).unwrap(), foreign_before);
}

#[test]
fn combined_grant_invalid_policy_never_changes_registry() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("identity.json");
    let identity = HostIdentity::load_or_create(&path).unwrap();
    let registry =
        DeviceRegistry::new(path.with_extension("devices.json"), identity.fingerprint()).unwrap();
    let (_, id) = device(&registry, &identity.fingerprint());
    let before = std::fs::read(path.with_extension("devices.json")).unwrap();
    for projects in [&[][..], &["p", "p"][..], &["\n"][..]] {
        assert!(
            !grant(&path, &id, "--grant-device-control-approval", projects)
                .status
                .success()
        );
        assert_eq!(
            std::fs::read(path.with_extension("devices.json")).unwrap(),
            before
        );
    }
    for extra in [
        "--grant-device-control",
        "--grant-device-approval",
        "--grant-device-read",
        "--revoke-device",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_zenpi"))
            .args(["host", "pair", "--identity"])
            .arg(&path)
            .args([
                "--grant-device-control-approval",
                &id,
                "--project",
                "p",
                extra,
                &id,
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
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
    let (_, first) = device(&registry, &identity.fingerprint());
    let (_, second) = device(&registry, &identity.fingerprint());
    assert!(
        grant(
            &path,
            &first,
            "--grant-device-control-approval",
            &["project-one", "project-two"]
        )
        .status
        .success()
    );
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
        zenpi::core::parse_args([
            "host",
            "pair",
            "--json",
            "--grant-device-read",
            &first,
            "--project",
            "project-one"
        ])
        .is_err()
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
fn one_cli_granted_phone_submits_approves_and_retries_without_duplicate_tool_effect() {
    exercise_prompt_approval("allow");
}

#[test]
fn one_cli_granted_phone_denies_tool_without_losing_prompt_control() {
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
    let (key, id) = device(&registry, &identity.fingerprint());
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
    assert!(
        grant(&path, &id, "--grant-device-control-approval", &[&project])
            .status
            .success()
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
