#![cfg(unix)]

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
    pty_shell::PtyShell,
    remote::{
        auth::{Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message},
        frame::{read_frame, write_frame},
        host_adapter::HostAdapter,
        protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message, Operation},
        terminal,
    },
    session::SessionStore,
};

fn pool(path: &Path, root: &Path) -> ProjectOwnerPool {
    let backend = OpenAiCompatibleBackend::new("http://127.0.0.1:9", None, "fixture").unwrap();
    let owner = Agent::new(
        SessionStore::open_in_workspace(path, root).unwrap(),
        Box::new(backend),
    );
    let mut pool = ProjectOwnerPool::new(Arc::new(Mutex::new(owner))).unwrap();
    pool.restore_checkpoint().unwrap();
    pool
}

fn exchange(request: &Envelope, response: &Envelope) -> Value {
    let mut wire = Vec::new();
    write_frame(&mut wire, response).unwrap();
    json!({"request":request,"wire_base64":STANDARD.encode(wire)})
}

fn pump_until(shell: &mut PtyShell, ready: impl Fn(&PtyShell) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        shell.pump();
        if ready(shell) {
            return;
        }
        assert!(Instant::now() < deadline, "PTY fixture deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_terminal_input_resize_and_history_use_real_owned_pty_once() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("session.jsonl");
    let mut host = pool(&path, &root);
    let project = host.active_context().project_id.clone();
    let session = host.active_context().session_id.clone();
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
    let mut shell = PtyShell::spawn(root.clone()).unwrap();
    shell.write_input(b"printf '%s%s\\n' 'PTY_' 'READY'\r");
    pump_until(&mut shell, |shell| {
        shell.remote_content(64).contains("PTY_READY")
    });
    let tabs = host.subtab_state(&project).unwrap();
    let mut seed = json!({});
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
        request.project_id = Some(project.clone());
        request.session_id = Some(session.clone());
        let response = HostAdapter::dispatch_owned(&mut host, &registry, &device, request);
        assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
        let Message::Response { result } = response.payload else {
            panic!("read")
        };
        seed[name] = result;
    }
    // Match the desktop mailbox's terminal enrichment; the base adapter owns no PTY.
    let mut screen = terminal::screen(
        &project,
        &session,
        &tabs.active_id,
        tabs.revision,
        &shell.remote_content(64),
        &[&root],
    );
    screen["terminal_id"] = json!(shell.instance_id());
    screen["running"] = json!(shell.is_running());
    seed["listing"]["projects"][0]["desktop_terminal"] = screen;
    seed["listing"]["projects"][0]["terminal_control"] = json!(true);
    let seed_path = root.join("seed.json");
    let commands_path = root.join("commands.bin");
    let results_path = root.join("results.json");
    let next_path = root.join("next.bin");
    std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
    let run_java = |phase: &str| {
        let mut child = Command::new(std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap())
            .args([
                format!("-Dzenpi.terminal.phase={phase}"),
                format!("-Dzenpi.terminal.seed={}", seed_path.display()),
                format!("-Dzenpi.terminal.commands={}", commands_path.display()),
                format!("-Dzenpi.terminal.results={}", results_path.display()),
                format!("-Dzenpi.terminal.next={}", next_path.display()),
                "-cp".into(),
                std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap(),
                "org.junit.runner.JUnitCore".into(),
                "zenpi.AndroidRustTerminalTest".into(),
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
    let bytes = std::fs::read(&commands_path).unwrap();
    let mut input = bytes.as_slice();
    let resize = read_frame(&mut input).unwrap().unwrap();
    let command = read_frame(&mut input).unwrap().unwrap();
    let mut read = read_frame(&mut input).unwrap().unwrap();
    assert!(input.is_empty());
    let mut commands = Vec::new();
    for (kind, code) in [
        ("revision", ErrorCode::RevisionConflict),
        ("terminal", ErrorCode::OwnerMismatch),
        ("permission", ErrorCode::Unauthorized),
    ] {
        let mut invalid = command.clone();
        invalid.id = Some(format!("bad-{kind}"));
        if let Message::Request { arguments, .. } = &mut invalid.payload {
            if kind == "revision" {
                arguments["subtab_revision"] = json!(tabs.revision + 1);
            }
            if kind == "terminal" {
                arguments["terminal_id"] = json!("b".repeat(64));
            }
        }
        let response = if kind == "permission" {
            registry
                .set_permissions(&record.id, 2, vec![Capability::Read], vec![project.clone()])
                .unwrap();
            terminal::dispatch(
                &mut host,
                &registry,
                &authenticate(),
                &invalid,
                Some(&mut shell),
                false,
            )
        } else {
            terminal::dispatch(
                &mut host,
                &registry,
                &device,
                &invalid,
                Some(&mut shell),
                false,
            )
        };
        assert_eq!(response.code, Some(code));
        commands.push(exchange(&invalid, &response));
        assert!(!root.join("effect.txt").exists());
    }
    registry
        .set_permissions(
            &record.id,
            3,
            vec![Capability::Read, Capability::Control],
            vec![project.clone()],
        )
        .unwrap();
    let device = authenticate();
    let sized = terminal::dispatch(
        &mut host,
        &registry,
        &device,
        &resize,
        Some(&mut shell),
        false,
    );
    assert_eq!(sized.execution_state, Some(ExecutionState::Succeeded));
    commands.push(exchange(&resize, &sized));
    assert_eq!(shell.size().unwrap(), (33, 91));
    let executed = terminal::dispatch(
        &mut host,
        &registry,
        &device,
        &command,
        Some(&mut shell),
        false,
    );
    assert_eq!(executed.execution_state, Some(ExecutionState::Succeeded));
    commands.push(exchange(&command, &executed));
    pump_until(&mut shell, |shell| {
        shell.remote_content(4000).contains("ANDROID_PTY_FINISHED")
    });
    assert_eq!(
        std::fs::read_to_string(root.join("effect.txt")).unwrap(),
        "ANDROID_PTY_ONCE\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("size.txt")).unwrap(),
        "33 91\n"
    );
    let journal = std::fs::read(&path).unwrap();
    let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
    assert!(!String::from_utf8_lossy(&journal).contains("INTEROP_PTY_PRIVATE"));
    let duplicate = terminal::dispatch(
        &mut host,
        &registry,
        &device,
        &command,
        Some(&mut shell),
        false,
    );
    assert_eq!(duplicate, executed);
    commands.push(exchange(&command, &duplicate));
    let mut changed = command.clone();
    if let Message::Request { arguments, .. } = &mut changed.payload {
        arguments["text"] = json!("touch unexpected\r");
    }
    let conflict = terminal::dispatch(
        &mut host,
        &registry,
        &device,
        &changed,
        Some(&mut shell),
        false,
    );
    assert_eq!(conflict.code, Some(ErrorCode::RequestIdConflict));
    commands.push(exchange(&changed, &conflict));
    drop(host);
    let mut host = pool(&path, &root);
    for (request, original) in [(&resize, &sized), (&command, &executed)] {
        let replay = terminal::dispatch(&mut host, &registry, &device, request, None, false);
        assert_eq!(&replay, original);
        commands.push(exchange(request, &replay));
    }
    let mut new = command.clone();
    new.id = Some("new-without-terminal".into());
    new.required_owner = Some(host.workspace_owner().into());
    new.expected_revision = Some(host.workspace_revision());
    let refused = terminal::dispatch(&mut host, &registry, &device, &new, None, false);
    assert_eq!(refused.code, Some(ErrorCode::InvalidRequest));
    commands.push(exchange(&new, &refused));
    registry
        .set_permissions(&record.id, 4, vec![Capability::Read], vec![project.clone()])
        .unwrap();
    let reader = authenticate();
    let mut listing = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::WorkspaceList,
            arguments: json!({}),
        },
    );
    listing.id = Some("reader-listing".into());
    listing.project_id = Some(project.clone());
    listing.session_id = Some(session.clone());
    let response = HostAdapter::dispatch_owned(&mut host, &registry, &reader, listing);
    let Message::Response { result } = response.payload else {
        panic!("reader listing")
    };
    seed["listing"] = result;
    assert_ne!(seed["listing"]["projects"][0]["prompt_control"], true);
    let mut subtabs = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::SubtabList,
            arguments: json!({}),
        },
    );
    subtabs.id = Some("reader-subtabs".into());
    subtabs.project_id = Some(project.clone());
    subtabs.session_id = Some(session.clone());
    let response = HostAdapter::dispatch_owned(&mut host, &registry, &reader, subtabs);
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    let Message::Response { result } = response.payload else {
        panic!("reader subtabs")
    };
    seed["subtabs"] = result;
    // Enrich only the screen as the desktop mailbox does, never add Control.
    let mut screen = terminal::screen(
        &project,
        &session,
        &tabs.active_id,
        tabs.revision,
        &shell.remote_content(64),
        &[&root],
    );
    screen["terminal_id"] = json!(shell.instance_id());
    screen["running"] = json!(shell.is_running());
    seed["listing"]["projects"][0]["desktop_terminal"] = screen;
    seed["listing"]["projects"][0]["terminal_control"] = json!(false);
    std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
    let mut read_only_controls = Vec::new();
    for (index, request) in [&command, &resize].into_iter().enumerate() {
        let mut denied = request.clone();
        denied.id = Some(format!("reader-denied-{index}"));
        denied.required_owner = Some(host.workspace_owner().into());
        denied.expected_revision = Some(host.workspace_revision());
        let response = terminal::dispatch(
            &mut host,
            &registry,
            &reader,
            &denied,
            Some(&mut shell),
            false,
        );
        assert_eq!(response.code, Some(ErrorCode::Unauthorized));
        read_only_controls.push(exchange(&denied, &response));
    }
    let mut read_refusals = Vec::new();
    let mut foreign = read.clone();
    foreign.session_id = Some("foreign".into());
    let refused = terminal::read(&host, &registry, &reader, &foreign, Some(&shell));
    assert_eq!(refused.code, Some(ErrorCode::OwnerMismatch));
    read_refusals.push(exchange(&foreign, &refused));
    let mut pages = Vec::new();
    for _ in 0..8 {
        let response = terminal::read(&host, &registry, &reader, &read, Some(&shell));
        assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
        pages.push(exchange(&read, &response));
        std::fs::write(
            &results_path,
            serde_json::to_vec(&json!({"commands":commands,"pages":pages,
                "read_only_controls":read_only_controls,"read_refusals":read_refusals}))
            .unwrap(),
        )
        .unwrap();
        run_java("read");
        let bytes = std::fs::read(&next_path).unwrap();
        if bytes.is_empty() {
            break;
        }
        let mut input = bytes.as_slice();
        read = read_frame(&mut input).unwrap().unwrap();
        assert!(input.is_empty());
    }
    assert!(
        std::fs::read(&next_path).unwrap().is_empty(),
        "bounded history complete"
    );
    registry.revoke(reader.id()).unwrap();
    let refused = terminal::read(&host, &registry, &reader, &read, Some(&shell));
    assert_eq!(refused.code, Some(ErrorCode::Unauthorized));
    read_refusals.push(exchange(&read, &refused));
    let evidence = json!({"commands":commands,"pages":pages,
        "read_only_controls":read_only_controls,"read_refusals":read_refusals});
    std::fs::write(&results_path, serde_json::to_vec(&evidence).unwrap()).unwrap();
    run_java("read");
    if let Some(output) = std::env::var_os("ZENPI_ANDROID_TERMINAL_OUTPUT") {
        std::fs::write(output, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
    assert_eq!(std::fs::read(&path).unwrap(), journal);
    assert_eq!(
        std::fs::read(root.join("project-workspace.json")).unwrap(),
        checkpoint
    );
    shell.write_input(b"exit\r");
    pump_until(&mut shell, |shell| !shell.is_running());
    assert_eq!(
        std::fs::read_to_string(root.join("effect.txt")).unwrap(),
        "ANDROID_PTY_ONCE\n"
    );
    assert!(!root.join("unexpected").exists());
}
