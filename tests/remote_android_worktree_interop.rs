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
    core::Agent,
    project_workspace::ProjectOwnerPool,
    remote::{
        auth::{Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message},
        frame::{read_frame, write_frame},
        host_adapter::HostAdapter,
        protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message, Operation},
    },
    session::SessionStore,
};

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn views(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &zenpi::remote::auth::AuthenticatedDevice,
    project: &str,
    session: &str,
) -> Value {
    let mut result = serde_json::Map::new();
    for (name, operation) in [
        ("listing", Operation::WorkspaceList),
        ("snapshot", Operation::Inspect),
        ("subtabs", Operation::SubtabList),
    ] {
        let mut request = Envelope::new(
            Channel::Control,
            Message::Request {
                operation,
                arguments: json!({}),
            },
        );
        request.id = Some(name.into());
        request.project_id = Some(project.into());
        request.session_id = Some(session.into());
        let response = HostAdapter::dispatch_owned(pool, registry, device, request);
        assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
        let Message::Response { result: value } = response.payload else {
            panic!("read result")
        };
        result.insert(name.into(), value);
    }
    Value::Object(result)
}

#[test]
#[ignore = "requires freshly compiled Kotlin JVM classes; source checker owns execution"]
fn android_worktree_crud_quota_and_dirty_refusal_roundtrip_through_rust() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.name", "fixture"]);
    git(&root, &["config", "user.email", "fixture@example.invalid"]);
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/tracked.txt"), "original\n").unwrap();
    git(&root, &["add", "src/tracked.txt"]);
    git(&root, &["commit", "-qm", "initial"]);
    let session = SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap();
    let session_id = session.session_id().to_owned();
    let backend = OpenAiCompatibleBackend::new("http://127.0.0.1:9", None, "fixture").unwrap();
    let owner = Arc::new(Mutex::new(Agent::new(session, Box::new(backend))));
    let mut pool = ProjectOwnerPool::new(owner).unwrap();
    pool.restore_checkpoint().unwrap();
    let project = pool.active_context().project_id.clone();
    let pin = "a".repeat(64);
    let registry = DeviceRegistry::new(root.join("devices.json"), pin.clone()).unwrap();
    let random = ring::rand::SystemRandom::new();
    let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random).unwrap();
    let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    let signature = URL_SAFE_NO_PAD.encode(
        key.sign(
            &random,
            &pairing_proof_message(&pin, "token", "pair", "phone", &public),
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
    let mut challenge = DeviceChallenge::new(pin).unwrap();
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
    let result_path = root.join("results.json");
    let java = std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap();
    let classpath = std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap();
    let run_java = |phase: &str| {
        let mut child = Command::new(&java)
            .args([
                format!("-Dzenpi.worktree.phase={phase}"),
                format!("-Dzenpi.worktree.seed={}", seed_path.display()),
                format!("-Dzenpi.worktree.command={}", command_path.display()),
                format!("-Dzenpi.worktree.results={}", result_path.display()),
                "-cp".into(),
                classpath.clone(),
                "org.junit.runner.JUnitCore".into(),
                "zenpi.AndroidRustWorktreeTest".into(),
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
    let worktree_root = root.join(".zenpi-worktrees/android-feature");
    let mut evidence = Vec::new();
    for step in [
        "directory_add",
        "worktree_add",
        "rename",
        "move",
        "quota64",
        "quota1",
        "select_worktree",
        "dirty_close",
        "clean_close",
        "select_directory",
        "close_directory",
        "select_main",
    ] {
        if step == "dirty_close" {
            std::fs::write(worktree_root.join("phone-local.txt"), "preserve\n").unwrap();
        }
        if step == "clean_close" {
            std::fs::remove_file(worktree_root.join("phone-local.txt")).unwrap();
        }
        let mut seed = views(&mut pool, &registry, &device, &project, &session_id);
        seed["step"] = json!(step);
        std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
        run_java("generate");
        let bytes = std::fs::read(&command_path).unwrap();
        let mut input = bytes.as_slice();
        let command = read_frame(&mut input).unwrap().unwrap();
        assert!(input.is_empty());
        let before = pool.subtab_state(&project).unwrap().clone();
        let revision = pool.workspace_revision();
        let response = HostAdapter::dispatch_owned(&mut pool, &registry, &device, command.clone());
        let mut exchanges = Vec::new();
        let mut record_response = |request: &Envelope, value: &Envelope| {
            let mut wire = Vec::new();
            write_frame(&mut wire, value).unwrap();
            exchanges.push(
                json!({"request":request,"code":value.code, "wire_base64":STANDARD.encode(wire)}),
            );
        };
        record_response(&command, &response);
        if step == "dirty_close" {
            assert_eq!(response.code, Some(ErrorCode::InvalidRequest));
            assert_eq!(pool.subtab_state(&project).unwrap(), before);
            assert_eq!(pool.workspace_revision(), revision);
            assert_eq!(
                std::fs::read_to_string(worktree_root.join("phone-local.txt")).unwrap(),
                "preserve\n"
            );
        } else {
            assert_eq!(
                response.execution_state,
                Some(ExecutionState::Succeeded),
                "{step}: {response:?}"
            );
            assert_eq!(pool.workspace_revision(), revision + 1);
            let committed = pool.subtab_state(&project).unwrap().clone();
            let duplicate =
                HostAdapter::dispatch_owned(&mut pool, &registry, &device, command.clone());
            assert_eq!(duplicate, response);
            record_response(&command, &duplicate);
            let mut stale = command.clone();
            stale.id = Some(format!("stale-{step}"));
            let refused = HostAdapter::dispatch_owned(&mut pool, &registry, &device, stale.clone());
            assert_eq!(refused.code, Some(ErrorCode::RevisionConflict));
            record_response(&stale, &refused);
            let mut conflict = command.clone();
            if let Message::Request { arguments, .. } = &mut conflict.payload {
                arguments["subtab_revision"] = json!(before.revision + 1);
            }
            let refused =
                HostAdapter::dispatch_owned(&mut pool, &registry, &device, conflict.clone());
            assert_eq!(refused.code, Some(ErrorCode::RequestIdConflict));
            record_response(&conflict, &refused);
            assert_eq!(pool.subtab_state(&project).unwrap(), committed);
            assert_eq!(pool.workspace_revision(), revision + 1);
            if step == "worktree_add" {
                assert!(worktree_root.join("src/tracked.txt").is_file());
            }
            if step == "clean_close" {
                assert!(!worktree_root.exists());
                git(
                    &root,
                    &["show-ref", "--verify", "refs/heads/android-feature"],
                );
            }
            if step == "close_directory" {
                assert!(root.join("src/tracked.txt").is_file());
            }
        }
        let after = views(&mut pool, &registry, &device, &project, &session_id);
        let result = json!({"step":step,"request":command,"exchanges":exchanges,"after":after});
        std::fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();
        run_java("validate");
        evidence.push(result);
    }
    assert_eq!(pool.subtab_state(&project).unwrap().tabs.len(), 1);
    if let Some(path) = std::env::var_os("ZENPI_ANDROID_WORKTREE_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&json!({"fixture_only":true,"passed":true,
            "scope":"current Kotlin/Rust/Git worktree commands and views; not APK, device, camera or LAN acceptance", "steps":evidence})).unwrap()).unwrap();
    }
}
