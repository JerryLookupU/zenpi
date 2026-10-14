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
    config::ConfigOverrides,
    core::{Agent, AgentPhase},
    project_workspace::ProjectWorkspace,
    remote::{
        auth::{
            AuthenticatedDevice, Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message,
        },
        frame::{read_frame, write_frame},
        host_adapter::HostAdapter,
        protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message, Operation},
    },
    tui::{ProjectIntent, ProjectRuntimeHost, ProjectTabMetadata, TuiState},
};

fn desktop(path: &Path, root: &Path) -> (ProjectRuntimeHost, TuiState) {
    let owner = Arc::new(Mutex::new(
        Agent::prepare_project_with_options(
            path,
            root,
            ConfigOverrides {
                backend: Some("openai".into()),
                model: Some("fixture".into()),
                base_url: Some("http://127.0.0.1:9/v1".into()),
                api_key: Some("fixture-not-a-secret".into()),
                wire_api: Some("chat".into()),
                ..Default::default()
            },
            false,
        )
        .unwrap(),
    ));
    let mut view = TuiState::default();
    view.set_active_project_metadata(ProjectTabMetadata::from_session(
        owner.lock().unwrap().session(),
    ));
    let mut host = ProjectRuntimeHost::new(&mut view, owner).unwrap();
    host.restore_shared_workspace(&mut view).unwrap();
    (host, view)
}

fn phone(
    registry: &DeviceRegistry,
    projects: Vec<String>,
    capabilities: Vec<Capability>,
) -> AuthenticatedDevice {
    let random = ring::rand::SystemRandom::new();
    let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random).unwrap();
    let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    let pin = "a".repeat(64);
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
        .set_permissions(&record.id, 1, capabilities, projects)
        .unwrap();
    let mut challenge = DeviceChallenge::new(pin).unwrap();
    let signature = URL_SAFE_NO_PAD.encode(
        key.sign(&random, &challenge.proof_message(&record.id))
            .unwrap()
            .as_ref(),
    );
    challenge
        .authenticate(registry, &record.id, &signature)
        .unwrap()
}

fn views(
    host: &mut ProjectRuntimeHost,
    view: &mut TuiState,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
) -> Value {
    let project = view.active_project().to_owned();
    let session = host
        .active(view)
        .lock()
        .unwrap()
        .session()
        .session_id()
        .to_owned();
    let mut values = serde_json::Map::new();
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
        request.session_id = Some(session.clone());
        let response = host.dispatch_remote(view, registry, device, request, &Default::default());
        assert_eq!(
            response.execution_state,
            Some(ExecutionState::Succeeded),
            "{response:?}"
        );
        let Message::Response { result } = response.payload else {
            panic!("read")
        };
        values.insert(name.into(), result);
    }
    Value::Object(values)
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_primary_workspace_commands_update_real_desktop_and_restart_receipts() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    std::fs::write(child.join("preserve.txt"), "desktop file\n").unwrap();
    let session_path = root.join("initial.jsonl");
    let (mut host, mut view) = desktop(&session_path, &root);
    let initial = view.active_project().to_owned();
    let next = ProjectWorkspace::default()
        .with_directory(Some(&child), &root)
        .unwrap()
        .0
        .active()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let projects = vec![initial.clone(), next.clone()];
    let device = phone(
        &registry,
        projects.clone(),
        vec![Capability::Read, Capability::Control],
    );
    let read_only = phone(&registry, projects, vec![Capability::Read]);
    let restricted = phone(
        &registry,
        vec![initial.clone()],
        vec![Capability::Read, Capability::Control],
    );
    let seed_path = root.join("seed.json");
    let command_path = root.join("command.bin");
    let result_path = root.join("results.json");
    let java = std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap();
    let classpath = std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap();
    let run_java = |phase: &str| {
        let mut child = Command::new(&java)
            .args([
                format!("-Dzenpi.workspace.phase={phase}"),
                format!("-Dzenpi.workspace.seed={}", seed_path.display()),
                format!("-Dzenpi.workspace.command={}", command_path.display()),
                format!("-Dzenpi.workspace.results={}", result_path.display()),
                "-cp".into(),
                classpath.clone(),
                "org.junit.runner.JUnitCore".into(),
                "zenpi.AndroidRustWorkspaceTest".into(),
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
    view.set_input("initial unsent draft");
    let mut accepted = Vec::new();
    let mut evidence = Vec::new();
    let mut child_owner = None;
    for step in [
        "open",
        "select_initial",
        "select_child",
        "local_race",
        "close",
        "reopen",
        "select_initial_again",
        "close_again",
    ] {
        let target = if matches!(step, "select_child" | "close" | "close_again") {
            &next
        } else {
            &initial
        };
        let expected_active = if matches!(step, "open" | "reopen" | "select_child") {
            &next
        } else {
            &initial
        };
        let count = if matches!(step, "close" | "close_again") {
            1
        } else {
            2
        };
        let mut seed = views(&mut host, &mut view, &registry, &device);
        seed["step"] = json!(step);
        seed["target"] = json!(target);
        seed["expected_active"] = json!(expected_active);
        seed["expected_count"] = json!(count);
        std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
        run_java("generate");
        let bytes = std::fs::read(&command_path).unwrap();
        let mut input = bytes.as_slice();
        let command = read_frame(&mut input).unwrap().unwrap();
        assert!(input.is_empty());
        let revision = host.remote_workspace_identity().1;
        let before_checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
        let before_view = view.project_checkpoint();
        let mut exchanges = Vec::new();
        let mut retain = |request: &Envelope, response: &Envelope| {
            let mut wire = Vec::new();
            write_frame(&mut wire, response).unwrap();
            exchanges.push(
                json!({"request":request,"code":response.code,"wire_base64":STANDARD.encode(wire)}),
            );
        };
        let denied = host.dispatch_remote(
            &mut view,
            &registry,
            &read_only,
            command.clone(),
            &Default::default(),
        );
        assert_eq!(denied.code, Some(ErrorCode::Unauthorized));
        retain(&command, &denied);
        if matches!(step, "open" | "reopen") {
            let denied = host.dispatch_remote(
                &mut view,
                &registry,
                &restricted,
                command.clone(),
                &Default::default(),
            );
            assert_eq!(denied.code, Some(ErrorCode::Unauthorized));
            retain(&command, &denied);
        }
        assert_eq!(
            std::fs::read(root.join("project-workspace.json")).unwrap(),
            before_checkpoint
        );
        assert_eq!(view.project_checkpoint(), before_view);
        if matches!(step, "close" | "close_again") {
            let busy = host.dispatch_remote(
                &mut view,
                &registry,
                &device,
                command.clone(),
                &[next.clone()].into(),
            );
            assert_eq!(busy.code, Some(ErrorCode::Busy));
            retain(&command, &busy);
            assert_eq!(
                std::fs::read(root.join("project-workspace.json")).unwrap(),
                before_checkpoint
            );
            assert_eq!(view.project_checkpoint(), before_view);
        }
        if step == "local_race" {
            host.apply(&mut view, ProjectIntent::Select(0)).unwrap();
            let local_checkpoint = view.project_checkpoint();
            let response = host.dispatch_remote(
                &mut view,
                &registry,
                &device,
                command.clone(),
                &Default::default(),
            );
            assert_eq!(response.code, Some(ErrorCode::RevisionConflict));
            retain(&command, &response);
            assert_eq!(view.project_checkpoint(), local_checkpoint);
        } else {
            let response = host.dispatch_remote(
                &mut view,
                &registry,
                &device,
                command.clone(),
                &Default::default(),
            );
            assert_eq!(
                response.execution_state,
                Some(ExecutionState::Succeeded),
                "{step}: {response:?}"
            );
            retain(&command, &response);
            let committed = std::fs::read(root.join("project-workspace.json")).unwrap();
            for code in [
                ErrorCode::Busy,
                ErrorCode::Unauthorized,
                ErrorCode::RevokedDevice,
            ] {
                let unknown = HostAdapter::delivery_refusal(command.id.clone(), true, code);
                assert_eq!(
                    unknown.execution_state,
                    Some(ExecutionState::UnknownOutcome)
                );
                retain(&command, &unknown);
            }
            let checkpoint = view.project_checkpoint();
            let duplicate = host.dispatch_remote(
                &mut view,
                &registry,
                &device,
                command.clone(),
                &Default::default(),
            );
            assert_eq!(duplicate, response);
            retain(&command, &duplicate);
            let mut stale = command.clone();
            stale.id = Some(format!("stale-{step}"));
            let refused = host.dispatch_remote(
                &mut view,
                &registry,
                &device,
                stale.clone(),
                &Default::default(),
            );
            assert_eq!(refused.code, Some(ErrorCode::RevisionConflict));
            retain(&stale, &refused);
            let mut conflict = command.clone();
            conflict.expected_revision = Some(revision + 1);
            let refused = host.dispatch_remote(
                &mut view,
                &registry,
                &device,
                conflict.clone(),
                &Default::default(),
            );
            assert_eq!(refused.code, Some(ErrorCode::RequestIdConflict));
            retain(&conflict, &refused);
            assert_eq!(
                std::fs::read(root.join("project-workspace.json")).unwrap(),
                committed
            );
            assert_eq!(view.project_checkpoint(), checkpoint);
            accepted.push((command.clone(), response));
        }
        assert_eq!(host.remote_workspace_identity().1, revision + 1);
        assert_eq!(view.active_project(), expected_active);
        assert_eq!(view.project_tab_count(), count);
        if matches!(step, "open" | "reopen") {
            child_owner = Some(host.active(&view));
        }
        if matches!(step, "close" | "close_again") {
            assert_eq!(
                child_owner.take().unwrap().lock().unwrap().phase(),
                AgentPhase::Closed
            );
        }
        if step == "open" {
            assert_eq!(view.input(), "");
            view.set_input("child unsent draft");
        }
        if matches!(step, "select_initial" | "local_race" | "close") {
            assert_eq!(view.input(), "initial unsent draft");
        }
        if step == "select_child" {
            assert_eq!(view.input(), "child unsent draft");
        }
        let result = json!({"request":command,"exchanges":exchanges,"after":views(&mut host, &mut view, &registry, &device)});
        std::fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();
        run_java("validate");
        evidence.push(result);
    }
    let old_owner = host.remote_workspace_identity().0.to_owned();
    let revision = host.remote_workspace_identity().1;
    drop(host);
    let (mut restarted, mut restored) = desktop(&session_path, &root);
    assert_ne!(restarted.remote_workspace_identity().0, old_owner);
    assert_eq!(restarted.remote_workspace_identity().1, revision);
    assert_eq!(restored.project_tab_count(), 1);
    let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
    for (command, response) in accepted {
        assert_eq!(
            restarted.dispatch_remote(
                &mut restored,
                &registry,
                &device,
                command.clone(),
                &Default::default()
            ),
            response
        );
        let mut stale = command;
        stale.id = Some(format!("restart-{}", stale.id.unwrap()));
        assert_eq!(
            restarted
                .dispatch_remote(
                    &mut restored,
                    &registry,
                    &device,
                    stale,
                    &Default::default()
                )
                .code,
            Some(ErrorCode::OwnerMismatch)
        );
    }
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        checkpoint
    );
    assert_eq!(
        std::fs::read_to_string(child.join("preserve.txt")).unwrap(),
        "desktop file\n"
    );
    if let Some(path) = std::env::var_os("ZENPI_ANDROID_WORKSPACE_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&json!({"fixture_only":true,"passed":true,
            "scope":"current Kotlin commands and actual desktop owner/view; not APK, camera, TCP or LAN acceptance","steps":evidence})).unwrap()).unwrap();
    }
}
