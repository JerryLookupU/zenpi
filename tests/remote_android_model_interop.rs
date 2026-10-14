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
    core::Agent,
    remote::{
        auth::{
            AuthenticatedDevice, Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message,
        },
        frame::{read_frame, write_frame},
        protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message, Operation},
    },
    tui::{ProjectRuntimeHost, ProjectTabMetadata, TuiState},
};

fn desktop(path: &Path, root: &Path) -> (ProjectRuntimeHost, TuiState) {
    let resume = path.exists();
    let owner = Arc::new(Mutex::new(
        Agent::prepare_project_with_options(
            path,
            root,
            ConfigOverrides {
                backend: Some("openai".into()),
                model: Some("gpt-5.2".into()),
                base_url: Some("http://127.0.0.1:9/v1".into()),
                api_key: Some("PRIVATE_INTEROP_MODEL_KEY".into()),
                wire_api: Some("responses".into()),
                model_reasoning_effort: Some("high".into()),
                ..Default::default()
            },
            false,
        )
        .unwrap(),
    ));
    // Keep this offline registry fixture independent of the operator's default
    // model, while explicitly exercising durable selection restoration.
    if resume {
        owner.lock().unwrap().restore_model_selection().unwrap();
    }
    let mut view = TuiState::default();
    view.set_active_project_metadata(ProjectTabMetadata::from_session(
        owner.lock().unwrap().session(),
    ));
    let mut host = ProjectRuntimeHost::new(&mut view, owner).unwrap();
    host.restore_shared_workspace(&mut view).unwrap();
    (host, view)
}

fn phone(registry: &DeviceRegistry, project: &str, control: bool) -> AuthenticatedDevice {
    phone_with_key(registry, project, control).0
}

fn phone_with_key(
    registry: &DeviceRegistry,
    project: &str,
    control: bool,
) -> (AuthenticatedDevice, EcdsaKeyPair) {
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
        .set_permissions(
            &record.id,
            1,
            if control {
                vec![Capability::Read, Capability::Control]
            } else {
                vec![Capability::Read]
            },
            vec![project.into()],
        )
        .unwrap();
    let mut challenge = DeviceChallenge::new(pin).unwrap();
    let signature = URL_SAFE_NO_PAD.encode(
        key.sign(&random, &challenge.proof_message(&record.id))
            .unwrap()
            .as_ref(),
    );
    let device = challenge
        .authenticate(registry, &record.id, &signature)
        .unwrap();
    (device, key)
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
        ("model", Operation::ModelStatus),
        ("catalog", Operation::ResourcesList),
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
        let response = host.dispatch_remote(view, registry, device, request, &Default::default());
        assert_eq!(
            response.execution_state,
            Some(ExecutionState::Succeeded),
            "{response:?}"
        );
        let Message::Response { result } = response.payload else {
            panic!("read")
        };
        assert!(!result.to_string().contains("PRIVATE_INTEROP_MODEL_KEY"));
        values.insert(name.into(), result);
    }
    Value::Object(values)
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_worker_reads_use_real_desktop_tasks_and_restart_journals() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("workers.jsonl");
    let echo = || {
        let owner = Arc::new(Mutex::new(Agent::with_echo(
            zenpi::session::SessionStore::open_in_workspace(&path, &root).unwrap(),
        )));
        let mut view = TuiState::default();
        view.set_active_project_metadata(ProjectTabMetadata::from_session(
            owner.lock().unwrap().session(),
        ));
        let mut host = ProjectRuntimeHost::new(&mut view, owner).unwrap();
        host.restore_shared_workspace(&mut view).unwrap();
        (host, view)
    };
    let (mut host, mut view) = echo();
    view.set_input("LOCAL_DRAFT_UNCHANGED");
    let project = view.active_project().to_owned();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let (mut device, key) = phone_with_key(&registry, &project, true);
    let foreign = phone(&registry, &project, true);
    let master = host.active(&view);
    let session = master.lock().unwrap().session().session_id().to_owned();
    let scope = |operation, arguments, id: &str| {
        let mut request = Envelope::new(
            Channel::Control,
            Message::Request {
                operation,
                arguments,
            },
        );
        request.id = Some(id.into());
        request.project_id = Some(project.clone());
        request.session_id = Some(session.clone());
        request
    };
    let mut prompts = Vec::new();
    for index in 0..18 {
        let tab = host.host_subtab_state(&project).unwrap();
        let text = if index == 0 {
            format!(
                "worker payload {} tail",
                "abcdefghijklmnopqrstuvwxyz".repeat(800)
            )
        } else {
            format!("worker payload {index}")
        };
        let mut request = scope(
            Operation::Prompt,
            json!({"text":text,"subtab_id":tab.active_id,"subtab_revision":tab.revision}),
            &format!("worker-{index:02}"),
        );
        let (owner, revision) = host.remote_workspace_identity();
        request.required_owner = Some(owner.into());
        request.expected_revision = Some(revision);
        let accepted = host.dispatch_remote(
            &mut view,
            &registry,
            &device,
            request.clone(),
            &Default::default(),
        );
        assert_eq!(
            accepted.execution_state,
            Some(ExecutionState::Accepted),
            "{accepted:?}"
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while host.remote_workers_busy(&project) {
            host.poll_remote_workers();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            host.dispatch_remote(
                &mut view,
                &registry,
                &device,
                request.clone(),
                &Default::default()
            ),
            accepted
        );
        prompts.push((request, accepted));
    }
    assert!(master.lock().unwrap().session().turns().is_empty());
    assert_eq!(view.input(), "LOCAL_DRAFT_UNCHANGED");
    let revision = registry.lookup(device.id()).unwrap().revision;
    registry
        .set_permissions(
            device.id(),
            revision,
            vec![Capability::Read],
            vec![project.clone()],
        )
        .unwrap();
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let signature = URL_SAFE_NO_PAD.encode(
        key.sign(
            &ring::rand::SystemRandom::new(),
            &challenge.proof_message(device.id()),
        )
        .unwrap()
        .as_ref(),
    );
    device = challenge
        .authenticate(&registry, device.id(), &signature)
        .unwrap();
    let journals = || {
        std::fs::read_dir(root.join(".zenpi/workers"))
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                (path.clone(), std::fs::read(path).unwrap())
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let before = journals();
    let java = std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap();
    let classpath = std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap();
    let seed_path = root.join("seed.json");
    let command_path = root.join("command.frame");
    let result_path = root.join("results.json");
    let run_java = |phase: &str| {
        let mut child = Command::new(&java)
            .args([
                format!("-Dzenpi.workerread.phase={phase}"),
                format!("-Dzenpi.workerread.seed={}", seed_path.display()),
                format!("-Dzenpi.workerread.command={}", command_path.display()),
                format!("-Dzenpi.workerread.results={}", result_path.display()),
                "-cp".into(),
                classpath.clone(),
                "org.junit.runner.JUnitCore".into(),
                "zenpi.AndroidRustWorkerReadTest".into(),
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
    let mut lists = Vec::<Value>::new();
    let mut histories = Vec::<Value>::new();
    let mut texts = Vec::<Value>::new();
    let mut last_command: Option<Envelope> = None;
    for mode in [
        "list", "list", "history", "output", "output", "output", "detail", "detail", "detail",
    ] {
        if mode == "detail"
            && last_command.as_ref().is_some_and(|request| {
                matches!(&request.payload,
                Message::Request { arguments, .. } if arguments.get("turn_id").is_none())
            })
        {
            texts.clear();
        }
        let mut seed = views(&mut host, &mut view, &registry, &device);
        // The target is older than the bounded 16-task summary, not forged into it.
        assert!(
            !seed["listing"]["projects"][0]["worker_statuses"]
                .as_array()
                .unwrap()
                .iter()
                .any(|task| task["request_id"] == "worker-00")
        );
        assert_eq!(seed["listing"]["projects"][0]["worker_control"], false);
        seed["mode"] = json!(mode);
        seed["lists"] = json!(lists);
        seed["histories"] = json!(histories);
        seed["texts"] = json!(texts);
        if mode != "list" {
            seed["target"] = json!("worker-00");
        }
        std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
        run_java("generate");
        let bytes = std::fs::read(&command_path).unwrap();
        let mut input = bytes.as_slice();
        let request = read_frame(&mut input).unwrap().unwrap();
        assert!(input.is_empty());
        assert!(request.required_owner.is_none() && request.expected_revision.is_none());
        let response = host.dispatch_remote(
            &mut view,
            &registry,
            &device,
            request.clone(),
            &Default::default(),
        );
        assert_eq!(
            response.execution_state,
            Some(ExecutionState::Succeeded),
            "{mode}: {response:?}"
        );
        let Message::Response { result } = &response.payload else {
            panic!("read result")
        };
        let mut exchanges = Vec::new();
        let mut retain = |sent: &Envelope, receipt: &Envelope| {
            let mut wire = Vec::new();
            write_frame(&mut wire, receipt).unwrap();
            exchanges.push(
                json!({"request":sent,"code":receipt.code,"wire_base64":STANDARD.encode(wire)}),
            );
        };
        retain(&request, &response);
        let mut altered = request.clone();
        altered.session_id = Some("foreign-session".into());
        let refusal = host.dispatch_remote(
            &mut view,
            &registry,
            &device,
            altered.clone(),
            &Default::default(),
        );
        assert_eq!(refusal.code, Some(ErrorCode::OwnerMismatch));
        retain(&altered, &refusal);
        if mode != "list" {
            let refusal = host.dispatch_remote(
                &mut view,
                &registry,
                &foreign,
                request.clone(),
                &Default::default(),
            );
            assert_eq!(refusal.code, Some(ErrorCode::InvalidRequest));
            retain(&request, &refusal);
        }
        if !lists.is_empty() && mode == "list" || !texts.is_empty() {
            let mut stale = request.clone();
            if let Message::Request { arguments, .. } = &mut stale.payload {
                arguments["digest"] = json!("0".repeat(64));
            }
            let refusal = host.dispatch_remote(
                &mut view,
                &registry,
                &device,
                stale.clone(),
                &Default::default(),
            );
            assert_eq!(refusal.code, Some(ErrorCode::RevisionConflict));
            retain(&stale, &refusal);
        }
        std::fs::write(
            &result_path,
            serde_json::to_vec(&json!({"exchanges":exchanges})).unwrap(),
        )
        .unwrap();
        run_java("validate");
        evidence.push(json!({"mode":mode,"exchanges":exchanges}));
        match mode {
            "list" => lists.push(result.clone()),
            "history" => histories.push(result.clone()),
            _ => texts.push(result.clone()),
        }
        last_command = Some(request);
        assert_eq!(journals(), before);
        if mode == "history" {
            host.shutdown_remote_workers(Instant::now() + Duration::from_secs(2));
            drop(host);
            drop(view);
            (host, view) = echo();
            let receipt = host.dispatch_remote(
                &mut view,
                &registry,
                &device,
                prompts[0].0.clone(),
                &Default::default(),
            );
            assert_eq!(receipt.code, Some(ErrorCode::Unauthorized));
            assert!(!host.remote_workers_busy(&project));
            assert_eq!(journals(), before);
        }
    }
    registry.revoke(device.id()).unwrap();
    let request = last_command.unwrap();
    let revoked = host.dispatch_remote(
        &mut view,
        &registry,
        &device,
        request.clone(),
        &Default::default(),
    );
    assert_eq!(revoked.code, Some(ErrorCode::Unauthorized));
    assert_eq!(journals(), before);
    let mut wire = Vec::new();
    write_frame(&mut wire, &revoked).unwrap();
    let mut revoked_exchanges =
        vec![json!({"request":request,"code":revoked.code,"wire_base64":STANDARD.encode(wire)})];
    for (operation, arguments) in [
        (
            Operation::WorkspaceList,
            json!({"view":"worker_status","offset":0,"digest":null}),
        ),
        (
            Operation::TranscriptRead,
            json!({"request_id":"worker-00","view":"history","offset":0,"digest":null}),
        ),
    ] {
        let request = scope(operation, arguments, "revoked-read");
        let response = host.dispatch_remote(
            &mut view,
            &registry,
            &device,
            request.clone(),
            &Default::default(),
        );
        assert_eq!(response.code, Some(ErrorCode::Unauthorized));
        let mut wire = Vec::new();
        write_frame(&mut wire, &response).unwrap();
        revoked_exchanges.push(
            json!({"request":request,"code":response.code,"wire_base64":STANDARD.encode(wire)}),
        );
    }
    std::fs::write(
        &result_path,
        serde_json::to_vec(&json!({"exchanges":revoked_exchanges})).unwrap(),
    )
    .unwrap();
    run_java("validate");
    if let Ok(path) = std::env::var("ZENPI_ANDROID_WORKER_READ_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&json!({"fixture_only":true,"steps":evidence,"revoked":revoked,
            "revoked_exchanges":revoked_exchanges,"read_only_after_downgrade":true,"restart_without_resubmission":true,
            "worker_journals_unchanged":true})).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_read_only_history_pages_use_real_desktop_journal() {
    let history_id = |index| {
        if index == 0 {
            "history:0/\u{4e2d}".to_owned()
        } else {
            format!("history-{index}")
        }
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("history.jsonl");
    let (mut host, mut view) = desktop(&path, &root);
    let owner = host.active(&view);
    for index in 0..70 {
        owner
            .lock()
            .unwrap()
            .session_mut()
            .append_turn(zenpi::core::Turn::new(
                history_id(index),
                zenpi::core::TurnRole::User,
                if index == 0 {
                    format!(
                        "{}api_key=PRIVATE_HISTORY_CREDENTIAL\nend of historical detail",
                        "Historical UTF-8 \u{4e2d} message\n".repeat(750)
                    )
                } else {
                    format!("Old message {index} api_key=PRIVATE_HISTORY_CREDENTIAL")
                },
            ))
            .unwrap();
    }
    let project = view.active_project().to_owned();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let reader = phone(&registry, &project, false);
    let seed_path = root.join("seed.json");
    let command_path = root.join("command.bin");
    let result_path = root.join("results.json");
    let java = std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap();
    let classpath = std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap();
    let run_java = |phase: &str| {
        let mut child = Command::new(&java)
            .args([
                format!("-Dzenpi.history.phase={phase}"),
                format!("-Dzenpi.history.seed={}", seed_path.display()),
                format!("-Dzenpi.history.command={}", command_path.display()),
                format!("-Dzenpi.history.results={}", result_path.display()),
                "-cp".into(),
                classpath.clone(),
                "org.junit.runner.JUnitCore".into(),
                "zenpi.AndroidRustHistoryTest".into(),
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
    let journal = std::fs::read(&path).unwrap();
    let workspace_revision = host.remote_workspace_identity().1;
    let mut pages = Vec::<Value>::new();
    let mut ids = Vec::new();
    let mut evidence = Vec::new();
    let mut detail_request: Option<Envelope> = None;
    for offset in [0, 16, 32, 48, 64] {
        let mut seed = views(&mut host, &mut view, &registry, &reader);
        seed["pages"] = json!(pages);
        seed["offset"] = json!(offset);
        seed["count"] = json!(if offset == 64 { 6 } else { 16 });
        std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
        run_java("generate");
        let bytes = std::fs::read(&command_path).unwrap();
        let mut input = bytes.as_slice();
        let command = read_frame(&mut input).unwrap().unwrap();
        assert!(input.is_empty());
        assert!(command.required_owner.is_none() && command.expected_revision.is_none());
        let mut exchanges = Vec::new();
        let mut retain = |request: &Envelope, response: &Envelope| {
            let mut wire = Vec::new();
            write_frame(&mut wire, response).unwrap();
            assert!(!String::from_utf8_lossy(&wire).contains("PRIVATE_HISTORY_CREDENTIAL"));
            assert!(!String::from_utf8_lossy(&wire).contains(root.to_str().unwrap()));
            exchanges.push(
                json!({"request":request,"code":response.code,"wire_base64":STANDARD.encode(wire)}),
            );
        };
        let mut foreign = command.clone();
        foreign.session_id = Some("foreign".into());
        let refused = host.dispatch_remote(
            &mut view,
            &registry,
            &reader,
            foreign.clone(),
            &Default::default(),
        );
        assert_eq!(refused.code, Some(ErrorCode::OwnerMismatch));
        retain(&foreign, &refused);
        let response = host.dispatch_remote(
            &mut view,
            &registry,
            &reader,
            command.clone(),
            &Default::default(),
        );
        assert_eq!(
            response.execution_state,
            Some(ExecutionState::Succeeded),
            "{response:?}"
        );
        retain(&command, &response);
        let Message::Response { result } = response.payload else {
            panic!("history response")
        };
        assert_eq!(result["offset"], offset);
        ids.extend(
            result["turns"]
                .as_array()
                .unwrap()
                .iter()
                .map(|turn| turn["id"].as_str().unwrap().to_owned()),
        );
        assert_eq!(std::fs::read(&path).unwrap(), journal);
        if offset == 64 {
            owner
                .lock()
                .unwrap()
                .session_mut()
                .append_turn(zenpi::core::Turn::new(
                    "changed",
                    zenpi::core::TurnRole::Assistant,
                    "changed",
                ))
                .unwrap();
            let changed = std::fs::read(&path).unwrap();
            let refused = host.dispatch_remote(
                &mut view,
                &registry,
                &reader,
                command.clone(),
                &Default::default(),
            );
            assert_eq!(refused.code, Some(ErrorCode::RevisionConflict));
            retain(&command, &refused);
            let detail = detail_request.as_ref().unwrap();
            let refused = host.dispatch_remote(
                &mut view,
                &registry,
                &reader,
                detail.clone(),
                &Default::default(),
            );
            assert_eq!(refused.code, Some(ErrorCode::RevisionConflict));
            retain(detail, &refused);
            registry.revoke(reader.id()).unwrap();
            let refused = host.dispatch_remote(
                &mut view,
                &registry,
                &reader,
                command.clone(),
                &Default::default(),
            );
            assert_eq!(refused.code, Some(ErrorCode::Unauthorized));
            retain(&command, &refused);
            let refused = host.dispatch_remote(
                &mut view,
                &registry,
                &reader,
                detail.clone(),
                &Default::default(),
            );
            assert_eq!(refused.code, Some(ErrorCode::Unauthorized));
            retain(detail, &refused);
            assert_eq!(std::fs::read(&path).unwrap(), changed);
        }
        let item = json!({"offset":offset,"exchanges":exchanges});
        std::fs::write(&result_path, serde_json::to_vec(&item).unwrap()).unwrap();
        run_java("validate");
        evidence.push(item);
        if offset == 0 {
            let mut details = Vec::<Value>::new();
            for index in 0..8 {
                seed["first"] = result.clone();
                seed["details"] = json!(details);
                std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
                run_java("generate_detail");
                let bytes = std::fs::read(&command_path).unwrap();
                let mut input = bytes.as_slice();
                let command = read_frame(&mut input).unwrap().unwrap();
                assert!(input.is_empty());
                let response = host.dispatch_remote(
                    &mut view,
                    &registry,
                    &reader,
                    command.clone(),
                    &Default::default(),
                );
                assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
                let mut wire = Vec::new();
                write_frame(&mut wire, &response).unwrap();
                assert!(!String::from_utf8_lossy(&wire).contains("PRIVATE_HISTORY_CREDENTIAL"));
                let Message::Response { result: detail } = response.payload else {
                    panic!("detail")
                };
                let item = json!({"detail":index,"exchanges":[{"request":command,"code":null,"wire_base64":STANDARD.encode(wire)}]});
                std::fs::write(&result_path, serde_json::to_vec(&item).unwrap()).unwrap();
                run_java("validate_detail");
                evidence.push(item);
                let more = detail["has_more"] == true;
                details.push(detail);
                detail_request = Some(command);
                if !more {
                    break;
                }
            }
            assert!(details.len() > 2);
            assert_eq!(details.last().unwrap()["has_more"], false);
            assert_eq!(std::fs::read(&path).unwrap(), journal);
        }
        pages.push(result);
    }
    assert_eq!(ids, (0..70).map(history_id).collect::<Vec<_>>());
    assert_eq!(host.remote_workspace_identity().1, workspace_revision);
    if let Some(output) = std::env::var_os("ZENPI_ANDROID_HISTORY_OUTPUT") {
        std::fs::write(output, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_read_only_diff_pages_use_real_desktop_git() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.name", "Interop"],
        vec!["config", "user.email", "interop@example.invalid"],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(&root)
                .status()
                .unwrap()
                .success()
        );
    }
    std::fs::write(root.join("file.txt"), "before\n").unwrap();
    for args in [vec!["add", "file.txt"], vec!["commit", "-qm", "baseline"]] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(&root)
                .status()
                .unwrap()
                .success()
        );
    }
    let content = "\u{4e2d} synchronized desktop change\n".repeat(900);
    std::fs::write(root.join("file.txt"), &content).unwrap();
    let path = root.join("diff.jsonl");
    let (mut host, mut view) = desktop(&path, &root);
    let project = view.active_project().to_owned();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let reader = phone(&registry, &project, false);
    let seed_path = root.join("seed.json");
    let command_path = root.join("command.bin");
    let result_path = root.join("results.json");
    let java = std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap();
    let classpath = std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap();
    let run_java = |phase: &str| {
        let mut child = Command::new(&java)
            .args([
                format!("-Dzenpi.diff.phase={phase}"),
                format!("-Dzenpi.diff.seed={}", seed_path.display()),
                format!("-Dzenpi.diff.command={}", command_path.display()),
                format!("-Dzenpi.diff.results={}", result_path.display()),
                "-cp".into(),
                classpath.clone(),
                "org.junit.runner.JUnitCore".into(),
                "zenpi.AndroidRustDiffTest".into(),
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
    let journal = std::fs::read(&path).unwrap();
    let mut pages = Vec::<Value>::new();
    let mut evidence = Vec::new();
    let mut complete = String::new();
    for index in 0..8 {
        let mut seed = views(&mut host, &mut view, &registry, &reader);
        seed["pages"] = json!(pages);
        std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
        run_java("generate");
        let bytes = std::fs::read(&command_path).unwrap();
        let mut input = bytes.as_slice();
        let command = read_frame(&mut input).unwrap().unwrap();
        assert!(input.is_empty());
        assert!(command.required_owner.is_none() && command.expected_revision.is_none());
        let mut exchanges = Vec::new();
        let mut retain = |request: &Envelope, response: &Envelope| {
            let mut wire = Vec::new();
            write_frame(&mut wire, response).unwrap();
            assert!(!String::from_utf8_lossy(&wire).contains(root.to_str().unwrap()));
            assert!(!String::from_utf8_lossy(&wire).contains("PRIVATE_INTEROP_MODEL_KEY"));
            exchanges.push(
                json!({"request":request,"code":response.code,"wire_base64":STANDARD.encode(wire)}),
            );
        };
        let mut foreign = command.clone();
        foreign.session_id = Some("foreign".into());
        let refused = host.dispatch_remote(
            &mut view,
            &registry,
            &reader,
            foreign.clone(),
            &Default::default(),
        );
        assert_eq!(refused.code, Some(ErrorCode::OwnerMismatch));
        retain(&foreign, &refused);
        let response = host.dispatch_remote(
            &mut view,
            &registry,
            &reader,
            command.clone(),
            &Default::default(),
        );
        assert_eq!(
            response.execution_state,
            Some(ExecutionState::Succeeded),
            "{response:?}"
        );
        retain(&command, &response);
        let Message::Response { result } = response.payload else {
            panic!("diff response")
        };
        complete.push_str(result["diff"].as_str().unwrap());
        let more = result["page"]["has_more"] == true;
        if index == 0 {
            std::fs::write(root.join("file.txt"), "changed since first page\n").unwrap();
            let mut stale = command.clone();
            if let Message::Request { arguments, .. } = &mut stale.payload {
                arguments["offset"] = result["page"]["next_offset"].clone();
                arguments["digest"] = result["page"]["digest"].clone();
            }
            let refused = host.dispatch_remote(
                &mut view,
                &registry,
                &reader,
                stale.clone(),
                &Default::default(),
            );
            assert_eq!(refused.code, Some(ErrorCode::RevisionConflict));
            retain(&stale, &refused);
            std::fs::write(root.join("file.txt"), &content).unwrap();
        }
        if !more {
            registry.revoke(reader.id()).unwrap();
            let refused = host.dispatch_remote(
                &mut view,
                &registry,
                &reader,
                command.clone(),
                &Default::default(),
            );
            assert_eq!(refused.code, Some(ErrorCode::Unauthorized));
            retain(&command, &refused);
        }
        assert_eq!(std::fs::read(&path).unwrap(), journal);
        let result_evidence = json!({"page":index,"exchanges":exchanges});
        std::fs::write(&result_path, serde_json::to_vec(&result_evidence).unwrap()).unwrap();
        run_java("validate");
        evidence.push(result_evidence);
        pages.push(result);
        if !more {
            break;
        }
    }
    assert!(pages.len() > 2);
    let last = pages.last().unwrap();
    assert_eq!(last["page"]["has_more"], false);
    assert_eq!(
        complete.len() as u64,
        last["page"]["total_bytes"].as_u64().unwrap()
    );
    assert!(complete.ends_with("+\u{4e2d} synchronized desktop change\n"));
    if let Some(output) = std::env::var_os("ZENPI_ANDROID_DIFF_OUTPUT") {
        std::fs::write(output, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_read_only_files_use_real_desktop_owner() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let docs = root.join("docs");
    std::fs::create_dir_all(docs.join("nested")).unwrap();
    std::fs::write(docs.join("nested/child.txt"), "PRIVATE_FILE_BODY").unwrap();
    for index in 0..140 {
        std::fs::write(
            docs.join(format!("file-{index:03}.txt")),
            "PRIVATE_FILE_BODY",
        )
        .unwrap();
    }
    for name in [
        ".hidden",
        "credentials.json",
        "auth.json",
        "private.pem",
        "PRIVATE.KEY",
        "id_rsa",
    ] {
        std::fs::write(docs.join(name), "PRIVATE_FILE_BODY").unwrap();
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(docs.join("file-000.txt"), docs.join("link")).unwrap();
    let path = root.join("files.jsonl");
    let (mut host, mut view) = desktop(&path, &root);
    let project = view.active_project().to_owned();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let reader = phone(&registry, &project, false);
    let seed_path = root.join("seed.json");
    let command_path = root.join("command.bin");
    let result_path = root.join("results.json");
    let java = std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap();
    let classpath = std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap();
    let run_java = |phase: &str| {
        let mut child = Command::new(&java)
            .args([
                format!("-Dzenpi.files.phase={phase}"),
                format!("-Dzenpi.files.seed={}", seed_path.display()),
                format!("-Dzenpi.files.command={}", command_path.display()),
                format!("-Dzenpi.files.results={}", result_path.display()),
                "-cp".into(),
                classpath.clone(),
                "org.junit.runner.JUnitCore".into(),
                "zenpi.AndroidRustFileTest".into(),
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
    let journal = std::fs::read(&path).unwrap();
    let mut first = Value::Null;
    let mut middle = Value::Null;
    let mut names = Vec::new();
    let mut evidence = Vec::new();
    for (step, directory, offset, count) in [
        ("first", "docs", 0, 64),
        ("middle", "docs", 64, 64),
        ("tail", "docs", 128, 13),
        ("back", "docs", 0, 64),
        ("nested", "docs/nested", 0, 1),
        ("stale", "docs", 64, 0),
        ("revoked", "docs", 0, 0),
    ] {
        let previous = match step {
            "middle" | "stale" => first.clone(),
            "tail" | "back" => middle.clone(),
            _ => Value::Null,
        };
        let mut seed = views(&mut host, &mut view, &registry, &reader);
        seed["path"] = json!(directory);
        seed["offset"] = json!(offset);
        seed["expected_count"] = json!(count);
        seed["first"] = first.clone();
        seed["previous"] = previous;
        seed["previous_offset"] = json!(if matches!(step, "tail" | "back") {
            64
        } else {
            0
        });
        std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
        run_java("generate");
        let bytes = std::fs::read(&command_path).unwrap();
        let mut input = bytes.as_slice();
        let command = read_frame(&mut input).unwrap().unwrap();
        assert!(input.is_empty());
        assert!(command.required_owner.is_none() && command.expected_revision.is_none());
        let mut exchanges = Vec::new();
        let mut retain = |request: &Envelope, response: &Envelope| {
            let mut wire = Vec::new();
            write_frame(&mut wire, response).unwrap();
            assert!(!String::from_utf8_lossy(&wire).contains("PRIVATE_FILE_BODY"));
            assert!(!String::from_utf8_lossy(&wire).contains(root.to_str().unwrap()));
            exchanges.push(
                json!({"request":request,"code":response.code,"wire_base64":STANDARD.encode(wire)}),
            );
        };
        if step == "stale" {
            std::fs::write(docs.join("file-000.txt"), "PRIVATE_FILE_BODY_CHANGED_SIZE").unwrap();
        } else if step == "revoked" {
            registry.revoke(reader.id()).unwrap();
        } else {
            let mut stale = command.clone();
            stale.session_id = Some("foreign".into());
            let response = host.dispatch_remote(
                &mut view,
                &registry,
                &reader,
                stale.clone(),
                &Default::default(),
            );
            assert_eq!(response.code, Some(ErrorCode::OwnerMismatch));
            retain(&stale, &response);
        }
        let response = host.dispatch_remote(
            &mut view,
            &registry,
            &reader,
            command.clone(),
            &Default::default(),
        );
        if step == "stale" {
            assert_eq!(response.code, Some(ErrorCode::RevisionConflict));
        } else if step == "revoked" {
            assert_eq!(response.code, Some(ErrorCode::Unauthorized));
        } else {
            assert_eq!(
                response.execution_state,
                Some(ExecutionState::Succeeded),
                "{response:?}"
            );
            let Message::Response { result } = &response.payload else {
                panic!("files response")
            };
            assert_eq!(result["entries"].as_array().unwrap().len(), count);
            if matches!(step, "first" | "middle" | "tail") {
                names.extend(
                    result["entries"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|entry| entry["name"].as_str().unwrap().to_owned()),
                );
            }
            if step == "first" {
                first = result.clone();
            }
            if step == "middle" {
                middle = result.clone();
            }
        }
        retain(&command, &response);
        assert_eq!(std::fs::read(&path).unwrap(), journal);
        let result = json!({"step":step,"exchanges":exchanges});
        std::fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();
        run_java("validate");
        evidence.push(result);
    }
    assert_eq!(
        names,
        std::iter::once("nested".to_owned())
            .chain((0..140).map(|index| format!("file-{index:03}.txt")))
            .collect::<Vec<_>>()
    );
    if let Some(output) = std::env::var_os("ZENPI_ANDROID_FILE_OUTPUT") {
        std::fs::write(output, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_resource_paging_reload_and_restart_use_real_desktop_catalog() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let prompts = root.join(".zenpi/prompts");
    std::fs::create_dir_all(&prompts).unwrap();
    for index in 0..40 {
        std::fs::write(prompts.join(format!("resource-{index:02}.md")),
            format!("---\ndescription: old description {index:02}\n---\nPRIVATE_TEMPLATE_BODY_{index:02}")).unwrap();
    }
    let path = root.join("resources.jsonl");
    let (mut host, mut view) = desktop(&path, &root);
    let project = view.active_project().to_owned();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let device = phone(&registry, &project, true);
    let reader = phone(&registry, &project, false);
    let seed_path = root.join("seed.json");
    let command_path = root.join("command.bin");
    let result_path = root.join("results.json");
    let java = std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap();
    let classpath = std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap();
    let run_java = |phase: &str| {
        let mut child = Command::new(&java)
            .args([
                format!("-Dzenpi.resource.phase={phase}"),
                format!("-Dzenpi.resource.seed={}", seed_path.display()),
                format!("-Dzenpi.resource.command={}", command_path.display()),
                format!("-Dzenpi.resource.results={}", result_path.display()),
                "-cp".into(),
                classpath.clone(),
                "org.junit.runner.JUnitCore".into(),
                "zenpi.AndroidRustResourceTest".into(),
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
    let mut accepted = None;
    let mut evidence = Vec::new();
    for step in ["page", "reload", "local_race"] {
        let mut seed = views(&mut host, &mut view, &registry, &device);
        seed["step"] = json!(step);
        assert_eq!(seed["catalog"]["entries"].as_array().unwrap().len(), 32);
        assert_eq!(seed["catalog"]["next_offset"], 32);
        std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
        run_java("generate");
        let bytes = std::fs::read(&command_path).unwrap();
        let mut input = bytes.as_slice();
        let command = read_frame(&mut input).unwrap().unwrap();
        assert!(input.is_empty());
        let owner = host.active(&view);
        let generation = owner.lock().unwrap().resource_snapshot().generation;
        let before = std::fs::read(&path).unwrap();
        let workspace_revision = host.remote_workspace_identity().1;
        let mut exchanges = Vec::new();
        let mut retain = |request: &Envelope, response: &Envelope| {
            let mut wire = Vec::new();
            write_frame(&mut wire, response).unwrap();
            assert!(!String::from_utf8_lossy(&wire).contains("PRIVATE_TEMPLATE_BODY"));
            assert!(!String::from_utf8_lossy(&wire).contains(root.to_str().unwrap()));
            exchanges.push(
                json!({"request":request,"code":response.code,"wire_base64":STANDARD.encode(wire)}),
            );
        };
        if step == "page" {
            let response = host.dispatch_remote(
                &mut view,
                &registry,
                &reader,
                command.clone(),
                &Default::default(),
            );
            assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
            retain(&command, &response);
            assert_eq!(std::fs::read(&path).unwrap(), before);
        } else {
            let denied = host.dispatch_remote(
                &mut view,
                &registry,
                &reader,
                command.clone(),
                &Default::default(),
            );
            assert_eq!(denied.code, Some(ErrorCode::Unauthorized));
            retain(&command, &denied);
            let busy = host.dispatch_remote(
                &mut view,
                &registry,
                &device,
                command.clone(),
                &[project.clone()].into(),
            );
            assert_eq!(busy.code, Some(ErrorCode::Busy));
            retain(&command, &busy);
            assert_eq!(std::fs::read(&path).unwrap(), before);
            std::fs::write(
                prompts.join("resource-00.md"),
                "---\ndescription: new description\n---\nPRIVATE_TEMPLATE_BODY_NEW",
            )
            .unwrap();
            if step == "local_race" {
                owner.lock().unwrap().reload_resources(|| false).unwrap();
                let committed = std::fs::read(&path).unwrap();
                let response = host.dispatch_remote(
                    &mut view,
                    &registry,
                    &device,
                    command.clone(),
                    &Default::default(),
                );
                assert_eq!(response.code, Some(ErrorCode::RevisionConflict));
                retain(&command, &response);
                assert_eq!(std::fs::read(&path).unwrap(), committed);
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
                    "{response:?}"
                );
                retain(&command, &response);
                let committed = std::fs::read(&path).unwrap();
                let duplicate = host.dispatch_remote(
                    &mut view,
                    &registry,
                    &device,
                    command.clone(),
                    &[project.clone()].into(),
                );
                assert_eq!(duplicate, response);
                retain(&command, &duplicate);
                let mut stale = command.clone();
                stale.id = Some("new-stale-reload".into());
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
                conflict.expected_revision = Some(workspace_revision + 1);
                let refused = host.dispatch_remote(
                    &mut view,
                    &registry,
                    &device,
                    conflict.clone(),
                    &Default::default(),
                );
                assert_eq!(refused.code, Some(ErrorCode::RequestIdConflict));
                retain(&conflict, &refused);
                assert_eq!(std::fs::read(&path).unwrap(), committed);
                accepted = Some((command.clone(), response));
            }
        }
        let expected_generation = generation + u64::from(step != "page");
        assert_eq!(
            owner.lock().unwrap().resource_snapshot().generation,
            expected_generation
        );
        assert_eq!(host.remote_workspace_identity().1, workspace_revision);
        let after = views(&mut host, &mut view, &registry, &device);
        let result = json!({"request":command,"exchanges":exchanges,"expected_generation":expected_generation,
            "expected_description":if step == "page" {"old description 00"} else {"new description"},"after":after});
        std::fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();
        run_java("validate");
        evidence.push(result);
    }
    let before = std::fs::read(&path).unwrap();
    drop(host);
    drop(view);
    let (mut host, mut view) = desktop(&path, &root);
    let restarted = std::fs::read(&path).unwrap();
    assert!(restarted.starts_with(&before));
    let (command, expected) = accepted.unwrap();
    let generation = host
        .active(&view)
        .lock()
        .unwrap()
        .resource_snapshot()
        .generation;
    let response = host.dispatch_remote(
        &mut view,
        &registry,
        &device,
        command.clone(),
        &Default::default(),
    );
    assert_eq!(response, expected);
    assert!(std::fs::read(&path).unwrap() == restarted);
    assert_eq!(
        host.active(&view)
            .lock()
            .unwrap()
            .resource_snapshot()
            .generation,
        generation
    );
    let mut wire = Vec::new();
    write_frame(&mut wire, &response).unwrap();
    let result = json!({"request":command,"exchanges":[{"request":command,"code":response.code,"wire_base64":STANDARD.encode(wire)}],
        "expected_generation":generation,"expected_description":"new description","after":views(&mut host,&mut view,&registry,&device)});
    std::fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();
    run_java("validate");
    evidence.push(result);
    if let Some(output) = std::env::var_os("ZENPI_ANDROID_RESOURCE_OUTPUT") {
        std::fs::write(output, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "needs Android JVM classes; run tools/build_android_interop_classes.sh --run"]
fn android_model_reasoning_cas_and_restart_receipts_use_real_desktop_owner() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("model.jsonl");
    let (mut host, mut view) = desktop(&path, &root);
    let project = view.active_project().to_owned();
    let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
    let device = phone(&registry, &project, true);
    let reader = phone(&registry, &project, false);
    let seed_path = root.join("seed.json");
    let command_path = root.join("command.bin");
    let result_path = root.join("results.json");
    let java = std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap();
    let classpath = std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap();
    let run_java = |phase: &str| {
        let mut child = Command::new(&java)
            .args([
                format!("-Dzenpi.model.phase={phase}"),
                format!("-Dzenpi.model.seed={}", seed_path.display()),
                format!("-Dzenpi.model.command={}", command_path.display()),
                format!("-Dzenpi.model.results={}", result_path.display()),
                "-cp".into(),
                classpath.clone(),
                "org.junit.runner.JUnitCore".into(),
                "zenpi.AndroidRustModelTest".into(),
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
    let mut accepted = Vec::new();
    let mut evidence = Vec::new();
    let budget_before = views(&mut host, &mut view, &registry, &device)["model"].clone();
    let journal_before_budget = std::fs::read(&path).unwrap();
    host.active(&view)
        .lock()
        .unwrap()
        .set_context_budget(zenpi::context::ContextBudget {
            max_tokens: 32000,
            reserved_output_tokens: 2048,
        });
    let budget_after = views(&mut host, &mut view, &registry, &device)["model"].clone();
    assert_eq!(
        budget_before["selection_revision"],
        budget_after["selection_revision"]
    );
    assert_eq!(budget_before["catalog"], budget_after["catalog"]);
    assert_ne!(budget_before["budget"], budget_after["budget"]);
    assert_eq!(std::fs::read(&path).unwrap(), journal_before_budget);
    for (step, operation, selected) in [
        ("default", "reasoning_select", Value::Null),
        ("none", "reasoning_select", json!("none")),
        ("model", "model_select", json!("gpt-5.2-2025-12-11")),
        ("local_race", "reasoning_select", json!("low")),
        ("low", "reasoning_select", json!("low")),
    ] {
        let mut seed = views(&mut host, &mut view, &registry, &device);
        seed["operation"] = json!(operation);
        seed["selected"] = selected;
        seed["budget_before"] = budget_before.clone();
        seed["budget_after"] = budget_after.clone();
        std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
        run_java("generate");
        let bytes = std::fs::read(&command_path).unwrap();
        let mut input = bytes.as_slice();
        let command = read_frame(&mut input).unwrap().unwrap();
        assert!(input.is_empty());
        let owner = host.active(&view);
        let selection_revision = owner.lock().unwrap().selection_revision();
        let workspace_revision = host.remote_workspace_identity().1;
        let journal_revision = owner.lock().unwrap().session().next_sequence();
        let before = std::fs::read(&path).unwrap();
        let mut exchanges = Vec::new();
        let mut retain = |request: &Envelope, response: &Envelope| {
            let mut wire = Vec::new();
            write_frame(&mut wire, response).unwrap();
            assert!(!String::from_utf8_lossy(&wire).contains("PRIVATE_INTEROP_MODEL_KEY"));
            exchanges.push(
                json!({"request":request,"code":response.code,"wire_base64":STANDARD.encode(wire)}),
            );
        };
        let denied = host.dispatch_remote(
            &mut view,
            &registry,
            &reader,
            command.clone(),
            &Default::default(),
        );
        assert_eq!(denied.code, Some(ErrorCode::Unauthorized));
        retain(&command, &denied);
        let busy = host.dispatch_remote(
            &mut view,
            &registry,
            &device,
            command.clone(),
            &[project.clone()].into(),
        );
        assert_eq!(busy.code, Some(ErrorCode::Busy));
        retain(&command, &busy);
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let response = if step == "local_race" {
            owner
                .lock()
                .unwrap()
                .set_reasoning_effort(Some("high".into()))
                .unwrap();
            let local = std::fs::read(&path).unwrap();
            let response = host.dispatch_remote(
                &mut view,
                &registry,
                &device,
                command.clone(),
                &Default::default(),
            );
            assert_eq!(response.code, Some(ErrorCode::RevisionConflict));
            assert_eq!(std::fs::read(&path).unwrap(), local);
            response
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
            let committed = std::fs::read(&path).unwrap();
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
            conflict.expected_revision = Some(workspace_revision + 1);
            let refused = host.dispatch_remote(
                &mut view,
                &registry,
                &device,
                conflict.clone(),
                &Default::default(),
            );
            assert_eq!(refused.code, Some(ErrorCode::RequestIdConflict));
            retain(&conflict, &refused);
            assert_eq!(std::fs::read(&path).unwrap(), committed);
            accepted.push((command.clone(), response.clone()));
            response
        };
        retain(&command, &response);
        assert_eq!(host.remote_workspace_identity().1, workspace_revision);
        let after = views(&mut host, &mut view, &registry, &device);
        let expected_model = if matches!(step, "default" | "none") {
            "gpt-5.2"
        } else {
            "gpt-5.2-2025-12-11"
        };
        let expected_reasoning = match step {
            "default" => Value::Null,
            "none" | "model" => json!("none"),
            "local_race" => json!("high"),
            _ => json!("low"),
        };
        assert_eq!(owner.lock().unwrap().model(), Some(expected_model));
        if step == "model" {
            assert_eq!(
                view.zone_model(zenpi::view_model::Zone::Discussion),
                Some(expected_model)
            );
        }
        assert_eq!(after["model"]["reasoning_effort"], expected_reasoning);
        assert_eq!(
            owner.lock().unwrap().selection_revision(),
            if step == "local_race" {
                journal_revision
            } else {
                journal_revision + 1
            }
        );
        assert!(owner.lock().unwrap().selection_revision() > selection_revision);
        let result = json!({"exchanges":exchanges,"expected_model":expected_model,
            "expected_reasoning":expected_reasoning,"expected_revision":if step == "local_race" { journal_revision } else { journal_revision + 1 },"after":after});
        std::fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();
        run_java("validate");
        evidence.push(result);
    }
    let journal = std::fs::read(&path).unwrap();
    let revision = host.active(&view).lock().unwrap().selection_revision();
    drop(host);
    drop(view);
    let (mut restored, mut view) = desktop(&path, &root);
    let restarted_journal = std::fs::read(&path).unwrap();
    assert!(
        restarted_journal.starts_with(&journal),
        "Owner bootstrap must preserve the original journal prefix"
    );
    assert_eq!(
        restored.active(&view).lock().unwrap().selection_revision(),
        revision
    );
    assert_eq!(
        restored.active(&view).lock().unwrap().reasoning_effort(),
        Some("low")
    );
    let mut exchanges = Vec::new();
    for (request, expected) in &accepted {
        let response = restored.dispatch_remote(
            &mut view,
            &registry,
            &device,
            request.clone(),
            &Default::default(),
        );
        assert_eq!(&response, expected);
        let mut wire = Vec::new();
        write_frame(&mut wire, &response).unwrap();
        exchanges.push(
            json!({"request":request,"code":response.code,"wire_base64":STANDARD.encode(wire)}),
        );
    }
    assert!(
        std::fs::read(&path).unwrap() == restarted_journal,
        "Exact retries appended to the restored journal"
    );
    let after = views(&mut restored, &mut view, &registry, &device);
    let result = json!({"exchanges":exchanges,"expected_model":"gpt-5.2-2025-12-11", "expected_reasoning":"low","expected_revision":revision,"after":after});
    std::fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();
    run_java("validate");
    evidence.push(result);
    if let Some(output) = std::env::var_os("ZENPI_ANDROID_MODEL_OUTPUT") {
        std::fs::write(output, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}
