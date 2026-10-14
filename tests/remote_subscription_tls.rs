use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    sync::{Arc, Mutex, mpsc},
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
        pairing::HostIdentity,
        protocol::{Channel, Envelope, ErrorCode, EventCursor, ExecutionState, Message, Operation},
        subscription,
    },
    session::SessionStore,
};

// Bounded fragmented encrypted transport, deliberately no socket/listener emulation claim.
struct Pipe {
    input: mpsc::Receiver<Vec<u8>>,
    output: mpsc::SyncSender<Vec<u8>>,
    pending: VecDeque<u8>,
    corrupt_next_write: bool,
}
struct ServerTask(Option<std::thread::JoinHandle<io::Result<()>>>);
impl ServerTask {
    fn join(mut self) -> io::Result<()> {
        self.0.take().unwrap().join().unwrap()
    }
}
impl Drop for ServerTask {
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            let _ = task.join();
        }
    }
}
struct GatedOutput<'a, W> {
    output: &'a mut W,
    gate: Option<mpsc::Receiver<()>>,
    flushes_before_gate: usize,
}
impl<W: Write> Write for GatedOutput<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.output.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()?;
        self.flushes_before_gate = self.flushes_before_gate.saturating_sub(1);
        if self.flushes_before_gate == 0
            && let Some(gate) = self.gate.take()
        {
            gate.recv_timeout(Duration::from_secs(3))
                .map_err(io::Error::other)?;
        }
        Ok(())
    }
}
fn pipes() -> (Pipe, Pipe) {
    let (atx, arx) = mpsc::sync_channel(64);
    let (btx, brx) = mpsc::sync_channel(64);
    (
        Pipe {
            input: arx,
            output: btx,
            pending: VecDeque::new(),
            corrupt_next_write: false,
        },
        Pipe {
            input: brx,
            output: atx,
            pending: VecDeque::new(),
            corrupt_next_write: false,
        },
    )
}
impl Read for Pipe {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if self.pending.is_empty() {
            match self.input.recv_timeout(Duration::from_secs(3)) {
                Ok(chunk) => self.pending.extend(chunk),
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(0),
                Err(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "fixture read deadline",
                    ));
                }
            }
        }
        let count = bytes.len().min(self.pending.len()).min(31);
        for byte in &mut bytes[..count] {
            *byte = self.pending.pop_front().unwrap();
        }
        Ok(count)
    }
}
impl Write for Pipe {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        let count = bytes.len().min(257);
        let mut chunk = bytes[..count].to_vec();
        if self.corrupt_next_write {
            self.corrupt_next_write = false;
            *chunk.last_mut().unwrap() ^= 1;
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match self.output.try_send(chunk) {
                Ok(()) => return Ok(count),
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "fixture peer closed",
                    ));
                }
                Err(mpsc::TrySendError::Full(value)) => chunk = value,
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "fixture write deadline",
                ));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn client(
    identity: &HostIdentity,
    pipe: Pipe,
) -> rustls::StreamOwned<rustls::ClientConnection, Pipe> {
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(rustls::pki_types::CertificateDer::from(
            identity.certificate().to_vec(),
        ))
        .unwrap();
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.alpn_protocols = vec![b"zenpi/1".to_vec()];
    rustls::StreamOwned::new(
        rustls::ClientConnection::new(Arc::new(config), "zenpi.local".try_into().unwrap()).unwrap(),
        pipe,
    )
}

fn exercise(mode: &str) {
    let tool_pages = mode.starts_with("transcript-tool-");
    let tool_arguments =
        serde_json::json!({"api_key":"PRIVATE_TLS_PAGE_SECRET","text":"\u{4e2d}".repeat(7000)});
    let paged_content = format!(
        "safe\n-----BEGIN PRIVATE KEY-----\nPRIVATE_TLS_PAGE_SECRET\n-----END PRIVATE KEY-----\n{}",
        "\u{4e2d} encrypted page\n".repeat(2000)
    );
    let root = tempfile::tempdir().unwrap();
    let identity = HostIdentity::generate().unwrap();
    let fingerprint = identity.fingerprint();
    let registry = Arc::new(
        DeviceRegistry::new(root.path().join("devices.json"), fingerprint.clone()).unwrap(),
    );
    let random = ring::rand::SystemRandom::new();
    let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random).unwrap();
    let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    let proof = URL_SAFE_NO_PAD.encode(
        key.sign(
            &random,
            &pairing_proof_message(&fingerprint, "fixture-token", "pair", "phone", &public),
        )
        .unwrap()
        .as_ref(),
    );
    let record = registry
        .verify_pair_and_register("fixture-token", "pair", "phone", &public, &proof, || true)
        .unwrap();
    let session =
        SessionStore::open_in_workspace(root.path().join("session.jsonl"), root.path()).unwrap();
    let session_id = session.session_id().to_owned();
    let backend = if matches!(
        mode,
        "model-registry" | "model-select" | "model-reasoning" | "model-pages" | "selection-sync"
    ) {
        OpenAiCompatibleBackend::new_with_settings(
            "http://127.0.0.1:9",
            Some("PRIVATE_MODEL_CREDENTIAL".into()),
            "gpt-5.2",
            zenpi::backend::OpenAiWireApi::Responses,
            Some("high".into()),
            None,
        )
        .unwrap()
        .with_model_registry(
            "openai".into(),
            if mode == "model-pages" {
                let overrides:Vec<zenpi::providers::registry::ModelOverride>=(0..40).map(|index|serde_json::from_value(serde_json::json!({
                    "provider":"openai","id":format!("a-model-{index:02}"),"version":"PRIVATE_MODEL_SOURCE"})).unwrap()).collect();
                zenpi::providers::registry::ModelRegistry::with_overrides(&overrides).unwrap()
            } else { zenpi::providers::registry::ModelRegistry::default() },
        )
        .unwrap()
    } else {
        OpenAiCompatibleBackend::new("http://127.0.0.1:9", None, "idle-fixture").unwrap()
    };
    let owner = Arc::new(Mutex::new(Agent::new(session, Box::new(backend))));
    let mut alternate_leaf = None;
    if mode.starts_with("transcript-") {
        let mut agent = owner.lock().unwrap();
        let mut turn = Turn::new("paged", TurnRole::Assistant, paged_content.clone());
        if tool_pages {
            turn.metadata = Some(
                serde_json::json!({"tool_calls":[{"id":"call","name":"read_file",
                "arguments":tool_arguments,"execution":"PRIVATE_TLS_EXECUTION"}]}),
            );
        }
        agent.session_mut().append_turn(turn).unwrap();
        let path = agent.session().path().to_path_buf();
        agent.resume_session(path).unwrap();
        if mode == "transcript-branch" {
            let session = agent.session_mut();
            session.enable_tree(Default::default(), &|| false).unwrap();
            let selected = session.active_tree_leaf().unwrap().to_owned();
            session.select_tree_leaf(None, &|| false).unwrap();
            session
                .append_turn(Turn::new(
                    "alternate",
                    TurnRole::Assistant,
                    "alternate branch",
                ))
                .unwrap();
            alternate_leaf = session.active_tree_leaf().map(str::to_owned);
            session
                .select_tree_leaf(Some(&selected), &|| false)
                .unwrap();
        }
    }
    let pool = Arc::new(Mutex::new(ProjectOwnerPool::new(owner.clone()).unwrap()));
    pool.lock().unwrap().restore_checkpoint().unwrap();
    if mode.starts_with("resources-") {
        let base = root.path();
        std::fs::create_dir_all(base.join("skills/research")).unwrap();
        std::fs::create_dir_all(base.join("prompts")).unwrap();
        std::fs::write(
            base.join("skills/research/SKILL.md"),
            "---\nname: research\ndescription: Find sources\n---\nPRIVATE_SKILL_BODY\n",
        )
        .unwrap();
        std::fs::write(
            base.join("prompts/fixture-review.md"),
            if mode == "resources-no-description" {
                "PRIVATE_TEMPLATE_BODY\n"
            } else {
                "---\ndescription: Review change\n---\nPRIVATE_TEMPLATE_BODY\n"
            },
        )
        .unwrap();
        if mode.starts_with("resources-page") {
            for index in 0..40 {
                std::fs::write(
                    base.join(format!("prompts/fixture-{index:02}.md")),
                    "PAGE_PRIVATE_TEMPLATE_BODY\n",
                )
                .unwrap();
            }
        }
        owner
            .lock()
            .unwrap()
            .configure_resources(
                zenpi::resource_loader::ResourcePaths {
                    user_skills: base.join("empty-skills"),
                    project_skills: base.join("skills"),
                    skill_paths: vec![],
                    user_templates: base.join("empty-prompts"),
                    project_templates: base.join("prompts"),
                    template_paths: vec![],
                    text_resources: vec![],
                },
                || false,
            )
            .unwrap();
    }
    let project = pool
        .lock()
        .unwrap()
        .workspace()
        .active()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    if mode.starts_with("diff-") {
        let base = root.path().canonicalize().unwrap();
        let child = base.join("selected");
        std::fs::create_dir_all(child.join("credentials")).unwrap();
        for (path, text) in [
            ("file.txt", "before\n"),
            ("credentials/opaque.txt", "OPAQUE_TLS_PRIVATE_BEFORE\n"),
            ("AUTH.JSON", "OPAQUE_TLS_PRIVATE_BEFORE\n"),
        ] {
            std::fs::write(child.join(path), text).unwrap();
        }
        let git = |args: &[&str]| {
            assert!(
                std::process::Command::new("git")
                    .current_dir(&base)
                    .args(args)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        };
        git(&["init", "-q"]);
        git(&["add", "selected"]);
        git(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "seed",
        ]);
        std::fs::write(child.join("file.txt"), "EXPECTED_TLS_DIFF\n").unwrap();
        for path in ["credentials/opaque.txt", "AUTH.JSON", "new.pem"] {
            std::fs::write(child.join(path), "OPAQUE_TLS_PRIVATE_AFTER\n").unwrap();
        }
        std::fs::write(base.join("outside.txt"), "OUTSIDE_TLS_DIFF\n").unwrap();
        let mut actual = pool.lock().unwrap();
        let main = actual.subtab_state(&project).unwrap();
        actual
            .commit_subtabs(
                &project,
                main.revision,
                main.add_directory(&project, &base, &child, "selected")
                    .unwrap(),
            )
            .unwrap();
    }
    if mode.starts_with("files-") {
        let base = root.path().canonicalize().unwrap();
        let child = base.join("selected");
        std::fs::create_dir(&child).unwrap();
        std::fs::create_dir(child.join("docs")).unwrap();
        std::fs::write(child.join("docs/file name.txt"), "PRIVATE_FILE_BYTES").unwrap();
        if mode == "files-pages" {
            for index in 0..80 {
                std::fs::write(
                    child.join(format!("file-{index:03}.txt")),
                    "PRIVATE_FILE_BYTES",
                )
                .unwrap();
            }
        }
        std::fs::write(child.join(".hidden"), "hidden").unwrap();
        for name in [
            "credentials.json",
            "auth.json",
            "private.pem",
            "PRIVATE.KEY",
            "id_rsa",
        ] {
            std::fs::write(child.join(name), "PRIVATE_FILE_BYTES").unwrap();
        }
        std::fs::write(base.join("PRIMARY_FILE_MUST_NOT_LEAK"), "outside").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(base.join("PRIMARY_FILE_MUST_NOT_LEAK"), child.join("link"))
            .unwrap();
        let mut actual = pool.lock().unwrap();
        let main = actual.subtab_state(&project).unwrap();
        actual
            .commit_subtabs(
                &project,
                main.revision,
                main.add_directory(&project, &base, &child, "selected")
                    .unwrap(),
            )
            .unwrap();
    }
    let historical = if mode == "session-resume" {
        let mut session =
            SessionStore::open_in_workspace(root.path().join("history.jsonl"), root.path())
                .unwrap();
        session
            .append_turn(Turn::new(
                "historical",
                TurnRole::User,
                "PRIVATE_CATALOG_BODY",
            ))
            .unwrap();
        Some(session.session_id().to_owned())
    } else {
        None
    };
    let capabilities = if matches!(
        mode,
        "remote-subtab"
            | "model-select"
            | "model-reasoning"
            | "session-resume"
            | "session-switch-sync"
    ) {
        vec![Capability::Read, Capability::Control]
    } else {
        vec![Capability::Read]
    };
    registry
        .set_permissions(&record.id, 1, capabilities, vec![project.clone()])
        .unwrap();
    let adapter = HostAdapter::new(pool.clone());
    let (client_pipe, server_pipe) = pipes();
    let wrong_identity = HostIdentity::generate().unwrap();
    let mut client = client(
        if mode == "wrong-certificate" {
            &wrong_identity
        } else {
            &identity
        },
        client_pipe,
    );
    let mut server = rustls::StreamOwned::new(
        rustls::ServerConnection::new(identity.server_config().unwrap()).unwrap(),
        server_pipe,
    );
    let server_registry = registry.clone();
    let fingerprint_for_server = fingerprint.clone();
    let transcript_pages = mode.starts_with("transcript-");
    let branch_pages = mode == "transcript-branch";
    let file_reads = mode.starts_with("files-");
    let diff_reads = mode.starts_with("diff-");
    let resource_reads = mode.starts_with("resources-");
    let model_reads = mode.starts_with("model-");
    let model_select = mode == "model-select";
    let reasoning_select = mode == "model-reasoning";
    let session_resume = mode == "session-resume";
    let failure_sync = mode == "failure-sync";
    let status_pool = pool.clone();
    let (gate_sender, gate_receiver) = mpsc::sync_channel(1);
    let gate = if matches!(
        mode,
        "revoke"
            | "grant-change"
            | "revoke-event"
            | "local-subtab"
            | "remote-subtab"
            | "selection-sync"
            | "session-name-sync"
            | "session-switch-sync"
            | "failure-sync"
    ) {
        Some(gate_receiver)
    } else {
        None
    };
    let flushes_before_gate = if mode == "revoke-event" { 2 } else { 1 };
    let task = ServerTask(Some(std::thread::spawn(move || -> io::Result<()> {
        let hello = read_frame(&mut server).map_err(io::Error::other)?.unwrap();
        assert!(matches!(hello.payload, Message::Hello { .. }));
        assert_eq!(
            server.conn.protocol_version(),
            Some(rustls::ProtocolVersion::TLSv1_3)
        );
        assert_eq!(server.conn.alpn_protocol(), Some(b"zenpi/1".as_slice()));
        let mut challenge = DeviceChallenge::new(fingerprint_for_server.clone()).unwrap();
        write_frame(
            &mut server,
            &Envelope::new(
                Channel::Control,
                Message::Challenge {
                    nonce: challenge.nonce().into(),
                    host_fingerprint: fingerprint_for_server,
                },
            ),
        )
        .map_err(io::Error::other)?;
        let auth = read_frame(&mut server).map_err(io::Error::other)?.unwrap();
        let Message::Authenticate {
            device_id,
            signature,
        } = auth.payload
        else {
            panic!("authentication");
        };
        let device = challenge
            .authenticate(&server_registry, &device_id, &signature)
            .map_err(io::Error::other)?;
        let mut accepted = Envelope::new(
            Channel::Control,
            Message::Response {
                result: serde_json::json!({"authenticated":true}),
            },
        );
        accepted.id = auth.id;
        accepted.execution_state = Some(ExecutionState::Succeeded);
        write_frame(&mut server, &accepted).map_err(io::Error::other)?;
        if session_resume {
            for _ in 0..5 {
                let request = read_frame(&mut server).map_err(io::Error::other)?.unwrap();
                assert!(matches!(
                    request.payload,
                    Message::Request {
                        operation: Operation::SessionList
                            | Operation::SessionResume
                            | Operation::SessionRename,
                        ..
                    }
                ));
                let response = adapter.dispatch(&server_registry, &device, request);
                write_frame(&mut server, &response).map_err(io::Error::other)?;
            }
            return Ok(());
        }
        if file_reads || resource_reads || model_reads || diff_reads {
            for _ in 0..2 {
                let request = read_frame(&mut server).map_err(io::Error::other)?.unwrap();
                assert!(
                    matches!(&request.payload, Message::Request { operation, .. }
                    if *operation == if diff_reads { Operation::Diff } else if reasoning_select { Operation::ReasoningSelect } else if model_select { Operation::ModelSelect } else if model_reads { Operation::ModelStatus } else if resource_reads { Operation::ResourcesList } else { Operation::FilesList })
                );
                let response = adapter.dispatch(&server_registry, &device, request);
                write_frame(&mut server, &response).map_err(io::Error::other)?;
            }
            return Ok(());
        }
        if transcript_pages {
            for _ in 0..16 {
                let request = read_frame(&mut server).map_err(io::Error::other)?.unwrap();
                assert!(matches!(
                    request.payload,
                    Message::Request {
                        operation: Operation::TranscriptRead | Operation::Inspect,
                        ..
                    } | Message::Resume { .. }
                ));
                let resume = matches!(request.payload, Message::Resume { .. });
                let response = adapter.dispatch(&server_registry, &device, request);
                let finished = response.execution_state != Some(ExecutionState::Succeeded)
                    || matches!(&response.payload, Message::Response { result } if result["has_more"] == false);
                for frame in HostAdapter::wire_frames(response).map_err(io::Error::other)? {
                    write_frame(&mut server, &frame).map_err(io::Error::other)?;
                }
                if resume || finished && !branch_pages {
                    return Ok(());
                }
            }
            return Err(io::Error::other("transcript fixture page limit"));
        }
        let request = read_frame(&mut server).map_err(io::Error::other)?.unwrap();
        if failure_sync {
            // The shared read adapter has no live runner. Add the production
            // journal projection used by both TUI/headless workspace listings.
            let progress = zenpi::remote::command::PromptProgress::default();
            let mut output = GatedOutput {
                output: &mut server,
                gate,
                flushes_before_gate,
            };
            return subscription::serve_with_view(
                &request,
                |read| adapter.dispatch(&server_registry, &device, read),
                || {
                    subscription::view(&request, |read| {
                        let listing = matches!(
                            read.payload,
                            Message::Request {
                                operation: Operation::WorkspaceList,
                                ..
                            }
                        );
                        let mut response = adapter.dispatch(&server_registry, &device, read);
                        if listing
                            && let Message::Response { result } = &mut response.payload
                            && let Some(projects) = result["projects"].as_array_mut()
                        {
                            for project in projects {
                                let id = project["id"].as_str().unwrap().to_owned();
                                project["prompt_statuses"] = progress.listing(
                                    &status_pool.lock().unwrap(),
                                    device.id(),
                                    &id,
                                );
                            }
                        }
                        response
                    })
                },
                |frame| {
                    if !server_registry
                        .is_current(&device)
                        .map_err(io::Error::other)?
                    {
                        return Err(io::Error::other("subscription authorization changed"));
                    }
                    write_frame(&mut output, &frame).map_err(io::Error::other)
                },
            );
        }
        subscription::serve_authorized(
            &request,
            &adapter,
            &server_registry,
            &device,
            &mut GatedOutput {
                output: &mut server,
                gate,
                flushes_before_gate,
            },
        )
    })));
    let hello = write_frame(
        &mut client,
        &Envelope::new(
            Channel::Control,
            Message::Hello {
                supported_versions: vec![1],
            },
        ),
    );
    if mode == "wrong-certificate" {
        assert!(hello.is_err());
        drop(client);
        assert!(task.join().is_err());
        return;
    }
    hello.unwrap();
    let Message::Challenge {
        nonce,
        host_fingerprint,
    } = read_frame(&mut client).unwrap().unwrap().payload
    else {
        panic!("challenge");
    };
    assert_eq!(host_fingerprint, fingerprint);
    let mut message =
        serde_json::to_vec(&["zenpi-device-auth-v1", &fingerprint, &nonce, &record.id]).unwrap();
    if mode == "wrong-proof" {
        message.push(b'!');
    }
    let mut auth = Envelope::new(
        Channel::Control,
        Message::Authenticate {
            device_id: record.id.clone(),
            signature: URL_SAFE_NO_PAD.encode(key.sign(&random, &message).unwrap().as_ref()),
        },
    );
    auth.id = Some("auth".into());
    write_frame(&mut client, &auth).unwrap();
    if mode == "wrong-proof" {
        assert!(matches!(read_frame(&mut client), Ok(None) | Err(_)));
        drop(client);
        assert!(task.join().is_err());
        return;
    }
    assert_eq!(
        read_frame(&mut client).unwrap().unwrap().execution_state,
        Some(ExecutionState::Succeeded)
    );
    if session_resume {
        let mut rename = Envelope::new(
            Channel::Control,
            Message::Request {
                operation: Operation::SessionRename,
                arguments: serde_json::json!({"name":"TLS development","journal_revision":owner.lock().unwrap().session().next_sequence()}),
            },
        );
        rename.id = Some("tls-session-name".into());
        rename.project_id = Some(project.clone());
        rename.session_id = Some(session_id.clone());
        rename.required_owner = Some(pool.lock().unwrap().workspace_owner().into());
        rename.expected_revision = Some(pool.lock().unwrap().workspace_revision());
        write_frame(&mut client, &rename).unwrap();
        let renamed = read_frame(&mut client).unwrap().unwrap();
        assert_eq!(renamed.execution_state, Some(ExecutionState::Succeeded));
        assert_eq!(
            owner.lock().unwrap().session().session_name(),
            Some("TLS development")
        );
        let bytes = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
        write_frame(&mut client, &rename).unwrap();
        assert_eq!(read_frame(&mut client).unwrap().unwrap(), renamed);
        assert_eq!(
            std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
            bytes
        );
        let previous = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
        let mut request = Envelope::new(
            Channel::Control,
            Message::Request {
                operation: Operation::SessionList,
                arguments: serde_json::json!({}),
            },
        );
        request.id = Some("tls-session-list".into());
        request.project_id = Some(project.clone());
        request.session_id = Some(session_id.clone());
        write_frame(&mut client, &request).unwrap();
        let response = read_frame(&mut client).unwrap().unwrap();
        assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
        let Message::Response { result } = response.payload else {
            panic!("catalog")
        };
        let target = historical.unwrap();
        assert_eq!(result["entries"].as_array().unwrap().len(), 2);
        assert!(
            result["entries"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["session_id"] == target)
        );
        assert!(!result.to_string().contains("PRIVATE_CATALOG_BODY"));
        assert!(!result.to_string().contains("path"));
        assert!(
            result["entries"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["name"] == "TLS development" && entry["current"] == true)
        );
        request.id = Some("tls-session-resume".into());
        request.required_owner = Some(pool.lock().unwrap().workspace_owner().into());
        request.expected_revision = Some(pool.lock().unwrap().workspace_revision());
        request.payload = Message::Request {
            operation: Operation::SessionResume,
            arguments: serde_json::json!({"target_session_id":target,"snapshot_id":result["snapshot_id"]}),
        };
        write_frame(&mut client, &request).unwrap();
        let response = read_frame(&mut client).unwrap().unwrap();
        assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
        assert_eq!(
            pool.lock().unwrap().context(&project).unwrap().session_id,
            target
        );
        assert_eq!(
            owner.lock().unwrap().history()[0].content,
            "PRIVATE_CATALOG_BODY"
        );
        let resumed = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
        write_frame(&mut client, &request).unwrap();
        assert_eq!(read_frame(&mut client).unwrap().unwrap(), response);
        assert_eq!(
            std::fs::read(root.path().join("session.jsonl")).unwrap(),
            previous
        );
        assert_eq!(
            std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
            resumed
        );
        drop(client);
        task.join().unwrap();
        return;
    }
    if model_reads {
        if mode == "model-pages" {
            let before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
            let mut req = Envelope::new(
                Channel::Control,
                Message::Request {
                    operation: Operation::ModelStatus,
                    arguments: serde_json::json!({}),
                },
            );
            req.id = Some("models-page-0".into());
            req.project_id = Some(project.clone());
            req.session_id = Some(session_id.clone());
            write_frame(&mut client, &req).unwrap();
            let response = read_frame(&mut client).unwrap().unwrap();
            assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
            let Message::Response { result: first } = response.payload else {
                panic!()
            };
            assert_eq!(first["catalog"]["entries"].as_array().unwrap().len(), 16);
            assert_eq!(first["catalog"]["next_offset"], 16);
            req.id = Some("models-page-16".into());
            req.payload = Message::Request {
                operation: Operation::ModelStatus,
                arguments: serde_json::json!({"offset":16,"snapshot_id":first["catalog"]["snapshot_id"]}),
            };
            write_frame(&mut client, &req).unwrap();
            let response = read_frame(&mut client).unwrap().unwrap();
            assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
            let Message::Response { result: second } = response.payload else {
                panic!()
            };
            assert_eq!(second["catalog"]["offset"], 16);
            assert_eq!(
                second["catalog"]["snapshot_id"],
                first["catalog"]["snapshot_id"]
            );
            assert_eq!(second["catalog"]["entries"][0]["id"], "a-model-16");
            for result in [first, second] {
                for forbidden in [
                    "PRIVATE_MODEL_CREDENTIAL",
                    "PRIVATE_MODEL_SOURCE",
                    "sources",
                    "http://",
                    "price",
                ] {
                    assert!(!result.to_string().contains(forbidden));
                }
            }
            task.join().unwrap();
            assert_eq!(
                std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
                before
            );
            return;
        }
        if matches!(mode, "model-select" | "model-reasoning") {
            let mut listing = Envelope::new(
                Channel::Control,
                Message::Request {
                    operation: if mode == "model-reasoning" {
                        Operation::ReasoningSelect
                    } else {
                        Operation::ModelSelect
                    },
                    arguments: if mode == "model-reasoning" {
                        serde_json::json!({"reasoning_effort":null,"selection_revision":0})
                    } else {
                        serde_json::json!({"model":"gpt-5.2-2025-12-11","selection_revision":0})
                    },
                },
            );
            listing.id = Some("tls-model-write".into());
            listing.project_id = Some(project.clone());
            listing.session_id = Some(session_id.clone());
            {
                let pool = pool.lock().unwrap();
                listing.required_owner = Some(pool.workspace_owner().into());
                listing.expected_revision = Some(pool.workspace_revision());
            }
            let mut receipt = None;
            for _ in 0..2 {
                write_frame(&mut client, &listing).unwrap();
                let response = read_frame(&mut client).unwrap().unwrap();
                assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
                if let Some(old) = &receipt {
                    assert_eq!(old, &response);
                }
                assert!(
                    !serde_json::to_string(&response)
                        .unwrap()
                        .contains("PRIVATE_MODEL_CREDENTIAL")
                );
                receipt = Some(response);
            }
            task.join().unwrap();
            let agent = owner.lock().unwrap();
            if mode == "model-reasoning" {
                assert_eq!(agent.reasoning_effort(), None);
                assert_eq!(agent.model(), Some("gpt-5.2"));
            } else {
                assert_eq!(agent.model(), Some("gpt-5.2-2025-12-11"));
            }
            assert_eq!(
                agent
                    .session()
                    .events()
                    .iter()
                    .filter(|event| event["type"] == "model_selected")
                    .count(),
                1
            );
            return;
        }
        let before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
        let checkpoint_path = owner
            .lock()
            .unwrap()
            .session()
            .path()
            .parent()
            .unwrap()
            .join("project-workspace.json");
        let checkpoint = std::fs::read(&checkpoint_path).ok();
        for index in 0..2 {
            if index == 1 && mode == "model-revoke" {
                registry.revoke(&record.id).unwrap();
            }
            let mut request = Envelope::new(
                Channel::Control,
                Message::Request {
                    operation: Operation::ModelStatus,
                    arguments: serde_json::json!({}),
                },
            );
            request.id = Some(format!("model-{index}"));
            request.project_id = Some(project.clone());
            request.session_id = Some(if index == 1 && mode == "model-stale" {
                "foreign".into()
            } else {
                session_id.clone()
            });
            write_frame(&mut client, &request).unwrap();
            let response = read_frame(&mut client).unwrap().unwrap();
            assert_eq!(response.id, request.id);
            if index == 1 && !matches!(mode, "model-deliver" | "model-registry") {
                assert_eq!(response.execution_state, Some(ExecutionState::Failed));
                assert_eq!(
                    response.code,
                    Some(if mode == "model-revoke" {
                        zenpi::remote::protocol::ErrorCode::Unauthorized
                    } else {
                        zenpi::remote::protocol::ErrorCode::OwnerMismatch
                    })
                );
                assert!(
                    matches!(response.payload, Message::Response { result } if result.is_null())
                );
            } else {
                assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
                let Message::Response { result } = response.payload else {
                    panic!("model status")
                };
                assert_eq!(result["project_id"], project);
                assert_eq!(result["session_id"], session_id);
                assert_eq!(
                    result["model"],
                    if mode == "model-registry" {
                        "gpt-5.2"
                    } else {
                        "idle-fixture"
                    }
                );
                assert_eq!(result.as_object().unwrap().len(), 10);
                if mode == "model-registry" {
                    assert_eq!(result["capabilities"]["text"], true);
                    assert_eq!(result["reasoning_effort"], "high");
                } else {
                    assert!(result["capabilities"].is_null());
                }
                for forbidden in [
                    "PRIVATE_MODEL_CREDENTIAL",
                    "http://",
                    "base_url",
                    "credential",
                    "sources",
                    &root.path().to_string_lossy(),
                ] {
                    assert!(!result.to_string().contains(forbidden));
                }
            }
        }
        task.join().unwrap();
        assert_eq!(
            std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
            before
        );
        assert_eq!(std::fs::read(checkpoint_path).ok(), checkpoint);
        return;
    }
    if resource_reads {
        let mut before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
        let checkpoint_path = owner
            .lock()
            .unwrap()
            .session()
            .path()
            .parent()
            .unwrap()
            .join("project-workspace.json");
        let checkpoint = std::fs::read(&checkpoint_path).ok();
        let mut page_identity = serde_json::Value::Null;
        for index in 0..2 {
            if index == 1 && matches!(mode, "resources-revoke" | "resources-page-revoke") {
                registry.revoke(&record.id).unwrap();
            }
            if index == 1 && mode == "resources-page-reload" {
                owner.lock().unwrap().reload_resources(|| false).unwrap();
                before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
            }
            let mut request = Envelope::new(
                Channel::Control,
                Message::Request {
                    operation: Operation::ResourcesList,
                    arguments: if index == 1 && mode.starts_with("resources-page") {
                        serde_json::json!({"offset":32,"snapshot_id":page_identity})
                    } else {
                        serde_json::json!({})
                    },
                },
            );
            request.id = Some(format!("resources-{index}"));
            request.project_id = Some(project.clone());
            request.session_id = Some(if index == 1 && mode == "resources-stale" {
                "foreign".into()
            } else {
                session_id.clone()
            });
            write_frame(&mut client, &request).unwrap();
            let response = read_frame(&mut client).unwrap().unwrap();
            assert_eq!(response.id, request.id);
            if index == 1
                && !matches!(
                    mode,
                    "resources-deliver" | "resources-page" | "resources-no-description"
                )
            {
                assert_eq!(response.execution_state, Some(ExecutionState::Failed));
                assert_eq!(
                    response.code,
                    Some(
                        if matches!(mode, "resources-revoke" | "resources-page-revoke") {
                            zenpi::remote::protocol::ErrorCode::Unauthorized
                        } else if mode == "resources-page-reload" {
                            zenpi::remote::protocol::ErrorCode::RevisionConflict
                        } else {
                            zenpi::remote::protocol::ErrorCode::OwnerMismatch
                        }
                    )
                );
                assert!(
                    matches!(response.payload, Message::Response { result } if result.is_null())
                );
            } else {
                assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
                let Message::Response { result } = response.payload else {
                    panic!("resource catalog")
                };
                assert_eq!(result["project_id"], project);
                assert_eq!(result["session_id"], session_id);
                if mode.starts_with("resources-page") {
                    assert_eq!(
                        result["entries"].as_array().unwrap().len(),
                        if index == 0 { 32 } else { 10 }
                    );
                    if index == 0 {
                        page_identity = result["snapshot_id"].clone();
                        assert_eq!(result["next_offset"], 32);
                    } else {
                        assert_eq!(result["snapshot_id"], page_identity);
                        assert!(result["next_offset"].is_null());
                        assert_eq!(result["entries"][9]["command"], "/fixture-review");
                    }
                    assert_eq!(result["offset"], index * 32);
                } else {
                    assert_eq!(result["entries"].as_array().unwrap().len(), 2);
                    assert_eq!(result["entries"][0]["command"], "/skill:research");
                    assert_eq!(result["entries"][1]["command"], "/fixture-review");
                    if mode == "resources-no-description" {
                        assert_eq!(result["entries"][1]["description"], "");
                    }
                }
                for secret in [
                    "PRIVATE_SKILL_BODY",
                    "PRIVATE_TEMPLATE_BODY",
                    "PAGE_PRIVATE_TEMPLATE_BODY",
                    &root.path().to_string_lossy(),
                ] {
                    assert!(!result.to_string().contains(secret));
                }
            }
        }
        task.join().unwrap();
        assert_eq!(
            std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
            before
        );
        assert_eq!(std::fs::read(checkpoint_path).ok(), checkpoint);
        return;
    }
    if diff_reads {
        let state = pool.lock().unwrap().subtab_state(&project).unwrap();
        let before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
        let checkpoint_path = root.path().join("project-workspace.json");
        let checkpoint = std::fs::read(&checkpoint_path).unwrap();
        for (index, path) in [".", "AUTH.JSON"].into_iter().enumerate() {
            if index == 1 && mode == "diff-revoke" {
                registry.revoke(&record.id).unwrap();
            }
            let mut request = Envelope::new(
                Channel::Control,
                Message::Request {
                    operation: Operation::Diff,
                    arguments: serde_json::json!({"subtab_id":state.active_id,"subtab_revision":state.revision,"path":path}),
                },
            );
            request.id = Some(format!("diff-{index}"));
            request.project_id = Some(project.clone());
            request.session_id = Some(session_id.clone());
            write_frame(&mut client, &request).unwrap();
            let response = read_frame(&mut client).unwrap().unwrap();
            assert_eq!(response.id, request.id);
            let encoded = serde_json::to_string(&response).unwrap();
            for forbidden in [
                "OPAQUE_TLS_PRIVATE",
                "AUTH.JSON",
                "credentials/opaque.txt",
                "new.pem",
                "OUTSIDE_TLS_DIFF",
                root.path().to_str().unwrap(),
            ] {
                assert!(!encoded.contains(forbidden), "{forbidden}: {encoded}");
            }
            if index == 0 {
                assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
                let Message::Response { result } = response.payload else {
                    panic!("diff")
                };
                assert_eq!(result["project_id"], project);
                assert_eq!(result["session_id"], session_id);
                assert_eq!(result["subtab_id"], state.active_id);
                assert_eq!(result["subtab_revision"], state.revision);
                assert!(
                    result["diff"]
                        .as_str()
                        .unwrap()
                        .contains("EXPECTED_TLS_DIFF")
                );
            } else {
                assert_eq!(response.execution_state, Some(ExecutionState::Failed));
                assert_eq!(
                    response.code,
                    Some(if mode == "diff-revoke" {
                        zenpi::remote::protocol::ErrorCode::Unauthorized
                    } else {
                        zenpi::remote::protocol::ErrorCode::InvalidRequest
                    })
                );
                assert!(
                    matches!(response.payload, Message::Response { result } if result.is_null())
                );
            }
        }
        drop(client);
        task.join().unwrap();
        assert_eq!(
            std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
            before
        );
        assert_eq!(std::fs::read(checkpoint_path).unwrap(), checkpoint);
        return;
    }
    if file_reads {
        let state = pool.lock().unwrap().subtab_state(&project).unwrap();
        if mode == "files-pages" {
            let journal = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
            let mut snapshot = serde_json::Value::Null;
            let mut names = Vec::new();
            for offset in [0, 64] {
                let mut request = Envelope::new(
                    Channel::Control,
                    Message::Request {
                        operation: Operation::FilesList,
                        arguments: serde_json::json!({"subtab_id":state.active_id,"subtab_revision":state.revision,"path":".","offset":offset,"snapshot_id":snapshot}),
                    },
                );
                request.id = Some(format!("files-page-{offset}"));
                request.project_id = Some(project.clone());
                request.session_id = Some(session_id.clone());
                write_frame(&mut client, &request).unwrap();
                let response = read_frame(&mut client).unwrap().unwrap();
                assert_eq!(response.id, request.id);
                assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
                let Message::Response { result } = response.payload else {
                    panic!("files");
                };
                assert_eq!(result["page"]["offset"], offset);
                if offset == 0 {
                    snapshot = result["page"]["snapshot_id"].clone();
                    assert_eq!(result["page"]["next_offset"], 64);
                } else {
                    assert_eq!(result["page"]["snapshot_id"], snapshot);
                    assert!(result["page"]["next_offset"].is_null());
                }
                for secret in [
                    "PRIVATE_FILE_BYTES",
                    "PRIMARY_FILE_MUST_NOT_LEAK",
                    ".hidden",
                    "link",
                    "credentials.json",
                    "auth.json",
                    "private.pem",
                    "PRIVATE.KEY",
                    "id_rsa",
                ] {
                    assert!(!result.to_string().contains(secret));
                }
                names.extend(
                    result["entries"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|entry| entry["name"].as_str().unwrap().to_owned()),
                );
            }
            assert_eq!(names.len(), 81);
            assert_eq!(names.first().unwrap(), "docs");
            assert_eq!(names.last().unwrap(), "file-079.txt");
            assert_eq!(
                std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
                journal
            );
            drop(client);
            task.join().unwrap();
            return;
        }
        let before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
        let checkpoint_path = owner
            .lock()
            .unwrap()
            .session()
            .path()
            .parent()
            .unwrap()
            .join("project-workspace.json");
        let mut checkpoint = std::fs::read(&checkpoint_path).unwrap();
        for (index, path) in [".", "docs"].into_iter().enumerate() {
            if index == 1 && mode == "files-revoke" {
                registry.revoke(&record.id).unwrap();
            }
            if index == 1 && mode == "files-stale" {
                let mut actual = pool.lock().unwrap();
                let candidate = state.select(&state.tabs[0].id).unwrap();
                actual
                    .commit_subtabs(&project, state.revision, candidate)
                    .unwrap();
                checkpoint = std::fs::read(&checkpoint_path).unwrap();
            }
            let mut request = Envelope::new(
                Channel::Control,
                Message::Request {
                    operation: Operation::FilesList,
                    arguments: serde_json::json!({"subtab_id":state.active_id,"subtab_revision":state.revision,"path":path}),
                },
            );
            request.id = Some(format!("files-{index}"));
            request.project_id = Some(project.clone());
            request.session_id = Some(session_id.clone());
            write_frame(&mut client, &request).unwrap();
            let response = read_frame(&mut client).unwrap().unwrap();
            assert_eq!(response.id, request.id);
            if index == 1 && mode != "files-deliver" {
                assert_eq!(response.execution_state, Some(ExecutionState::Failed));
                assert_eq!(
                    response.code,
                    Some(if mode == "files-revoke" {
                        zenpi::remote::protocol::ErrorCode::Unauthorized
                    } else {
                        zenpi::remote::protocol::ErrorCode::RevisionConflict
                    })
                );
                assert!(
                    matches!(response.payload, Message::Response { result } if result.is_null())
                );
            } else {
                assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
                assert_eq!(response.session_id.as_deref(), Some(session_id.as_str()));
                let Message::Response { result } = response.payload else {
                    panic!("file listing")
                };
                assert_eq!(result["project_id"], project);
                assert_eq!(result["session_id"], session_id);
                assert_eq!(result["subtab_id"], state.active_id);
                assert_eq!(result["subtab_revision"], state.revision);
                assert_eq!(result["path"], path);
                assert_eq!(result["entries"].as_array().unwrap().len(), 1);
                assert_eq!(
                    result["entries"][0]["path"],
                    if index == 0 {
                        "docs"
                    } else {
                        "docs/file name.txt"
                    }
                );
                assert_eq!(result["entries"][0]["attachable"], index == 1);
                for forbidden in [
                    "PRIVATE_FILE_BYTES",
                    "PRIMARY_FILE_MUST_NOT_LEAK",
                    ".hidden",
                    "link",
                    "credentials.json",
                    "auth.json",
                    "private.pem",
                    "PRIVATE.KEY",
                    "id_rsa",
                    &root.path().display().to_string(),
                ] {
                    assert!(!result.to_string().contains(forbidden));
                }
            }
        }
        drop(client);
        task.join().unwrap();
        assert_eq!(
            std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
            before
        );
        assert_eq!(std::fs::read(checkpoint_path).unwrap(), checkpoint);
        return;
    }
    if mode.starts_with("transcript-") {
        use sha2::{Digest, Sha256};
        let mut before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
        let checkpoint = std::fs::read(root.path().join("project-workspace.json")).ok();
        let mut offset = 0u64;
        let mut digest = None;
        let mut joined = String::new();
        let mut pages = 0;
        loop {
            if pages == 1 && matches!(mode, "transcript-revoke" | "transcript-tool-revoke") {
                registry.revoke(&record.id).unwrap();
            }
            if pages == 1 && matches!(mode, "transcript-stale" | "transcript-tool-stale") {
                digest = Some("0".repeat(64));
            }
            if pages == 1 && branch_pages {
                let mut agent = owner.lock().unwrap();
                agent
                    .session_mut()
                    .select_tree_leaf(alternate_leaf.as_deref(), &|| false)
                    .unwrap();
                before = std::fs::read(agent.session().path()).unwrap();
            }
            let mut arguments =
                serde_json::json!({"turn_id":"paged","offset":offset,"digest":digest});
            if tool_pages {
                arguments["tool_call_id"] = serde_json::json!("call");
            }
            let mut request = Envelope::new(
                Channel::Control,
                Message::Request {
                    operation: Operation::TranscriptRead,
                    arguments,
                },
            );
            request.id = Some(format!("page-{pages}"));
            request.project_id = Some(project.clone());
            request.session_id = Some(session_id.clone());
            write_frame(&mut client, &request).unwrap();
            let response = read_frame(&mut client).unwrap().unwrap();
            assert_eq!(response.id, request.id);
            if pages == 1 && !matches!(mode, "transcript-deliver" | "transcript-tool-deliver") {
                assert_eq!(response.execution_state, Some(ExecutionState::Failed));
                assert_eq!(
                    response.code,
                    Some(
                        if matches!(mode, "transcript-revoke" | "transcript-tool-revoke") {
                            zenpi::remote::protocol::ErrorCode::Unauthorized
                        } else if branch_pages {
                            zenpi::remote::protocol::ErrorCode::InvalidRequest
                        } else {
                            zenpi::remote::protocol::ErrorCode::RevisionConflict
                        }
                    )
                );
                assert!(
                    matches!(response.payload, Message::Response { result } if result.is_null())
                );
                if branch_pages {
                    let mut inspect = request.clone();
                    inspect.id = Some("branch-inspect".into());
                    inspect.payload = Message::Request {
                        operation: Operation::Inspect,
                        arguments: serde_json::json!({}),
                    };
                    write_frame(&mut client, &inspect).unwrap();
                    let inspected = read_frame(&mut client).unwrap().unwrap();
                    assert_eq!(inspected.execution_state, Some(ExecutionState::Succeeded));
                    let Message::Response { result } = inspected.payload else {
                        panic!("inspect")
                    };
                    assert_eq!(result["transcript"].as_array().unwrap().len(), 1);
                    assert_eq!(result["transcript"][0]["id"], "alternate");
                    assert_eq!(result["event_cursor"]["next_sequence"], 2);
                    inspect.id = Some("branch-resume".into());
                    inspect.payload = Message::Resume {
                        cursor: EventCursor {
                            session_id: session_id.clone(),
                            next_sequence: 1,
                        },
                        limit: 64,
                    };
                    write_frame(&mut client, &inspect).unwrap();
                    let snapshot = read_frame(&mut client).unwrap().unwrap();
                    let Message::Snapshot { state, cursor } = snapshot.payload else {
                        panic!("replacement")
                    };
                    assert_eq!(state["transcript"], result["transcript"]);
                    assert_eq!(cursor.next_sequence, 2);
                    let terminal = read_frame(&mut client).unwrap().unwrap();
                    assert_eq!(terminal.id, inspect.id);
                    assert_eq!(terminal.execution_state, Some(ExecutionState::Succeeded));
                }
                break;
            }
            assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
            let Message::Response { result } = response.payload else {
                panic!("page")
            };
            assert_eq!(result["project_id"], project);
            assert_eq!(result["session_id"], session_id);
            assert_eq!(result["turn_id"], "paged");
            if tool_pages {
                assert_eq!(result["tool_call_id"], "call");
                assert_eq!(result["tool_name"], "read_file");
                assert!(!result.to_string().contains("PRIVATE_TLS_EXECUTION"));
            }
            assert_eq!(result["offset"], offset);
            let text = result["text"].as_str().unwrap();
            assert!(text.len() <= zenpi::remote::transcript::PAGE_BYTES);
            assert!(!text.contains("PRIVATE_TLS_PAGE_SECRET"));
            joined.push_str(text);
            offset += text.len() as u64;
            assert_eq!(result["next_offset"], offset);
            let next_digest = result["digest"].as_str().unwrap().to_owned();
            if let Some(previous) = &digest {
                assert_eq!(previous, &next_digest);
            }
            digest = Some(next_digest);
            pages += 1;
            if result["has_more"] == false {
                assert!(matches!(
                    mode,
                    "transcript-deliver" | "transcript-tool-deliver"
                ));
                assert!(pages > 1);
                assert_eq!(result["total_bytes"], offset);
                let expected = if tool_pages {
                    serde_json::to_string(&zenpi::security::redact_json(&tool_arguments, &[]))
                        .unwrap()
                } else {
                    zenpi::security::redact_text(&paged_content, &[])
                };
                assert_eq!(joined, expected);
                assert_eq!(
                    digest.unwrap(),
                    format!("{:x}", Sha256::digest(joined.as_bytes()))
                );
                break;
            }
            assert!(pages < 16);
        }
        drop(client);
        task.join().unwrap();
        assert_eq!(
            std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
            before
        );
        assert_eq!(
            std::fs::read(root.path().join("project-workspace.json")).ok(),
            checkpoint
        );
        return;
    }
    let mut request = Envelope::new(
        Channel::Control,
        Message::Subscribe {
            cursor: EventCursor {
                session_id: session_id.clone(),
                next_sequence: 0,
            },
            limit: 128,
            duration_ms: 1000,
        },
    );
    request.id = Some("subscription".into());
    request.project_id = Some(project.clone());
    request.session_id = Some(session_id.clone());
    let held = if mode == "busy" {
        Some(owner.lock().unwrap())
    } else {
        None
    };
    if mode == "corrupt-record" {
        client.sock.corrupt_next_write = true;
    }
    write_frame(&mut client, &request).unwrap();
    if mode == "corrupt-record" {
        assert!(matches!(read_frame(&mut client), Ok(None) | Err(_)));
        drop(client);
        assert!(task.join().is_err());
        return;
    }
    let first = read_frame(&mut client).unwrap().unwrap();
    let Message::Response { result } = first.payload else {
        panic!("initial batch");
    };
    assert_eq!(result["cursor"]["next_sequence"], 0);
    assert_eq!(result["subscription_done"], false);
    assert_eq!(
        result["subscription_view"]["subtabs"]["project_id"],
        project
    );
    assert!(
        !result
            .to_string()
            .contains(&root.path().display().to_string())
    );
    if mode == "busy" {
        assert_eq!(result["subscription_transcript"], false);
        assert!(result["subscription_view"]["model_status"].is_null());
        assert!(result.get("journal_next_sequence").is_none());
        drop(held);
    }
    if mode == "disconnect" {
        drop(client);
        assert!(task.join().is_err());
        return;
    }
    if mode == "failure-sync" {
        assert_eq!(
            result["subscription_view"]["listing"]["projects"][0]["prompt_statuses"],
            serde_json::json!([])
        );
        let journal = {
            let mut agent = owner.lock().unwrap();
            let mut failed = request.clone();
            failed.id = Some("failed-message".into());
            let code = zenpi::remote::command::failure_code(&zenpi::core::AgentError::InvalidTurn(
                "Bearer PRIVATE_FAILURE /private/host".into(),
            ));
            zenpi::remote::command::persist_progress_with_code(
                &mut agent,
                &failed,
                &record.id,
                ExecutionState::Failed,
                Some(code),
            )
            .unwrap();
            failed.id = Some("foreign-message".into());
            zenpi::remote::command::persist_progress_with_code(
                &mut agent,
                &failed,
                "foreign-device",
                ExecutionState::Failed,
                Some(code),
            )
            .unwrap();
            agent.session().next_sequence()
        };
        gate_sender.send(()).unwrap();
        let mut updates = 0;
        loop {
            let frame = read_frame(&mut client).unwrap().unwrap();
            assert_eq!(frame.id, request.id);
            assert_eq!(frame.execution_state, Some(ExecutionState::Succeeded));
            let Message::Response { result } = frame.payload else {
                panic!("failure diagnosis must not become a transcript event");
            };
            assert_eq!(result["cursor"]["next_sequence"], 0);
            if let Some(view) = result.get("subscription_view") {
                let statuses = &view["listing"]["projects"][0]["prompt_statuses"];
                assert_eq!(statuses.as_array().unwrap().len(), 1);
                assert_eq!(statuses[0]["request_id"], "failed-message");
                assert_eq!(statuses[0]["state"], "failed");
                assert_eq!(statuses[0]["failure_code"], "invalid_request");
                assert_eq!(statuses[0].as_object().unwrap().len(), 5);
                assert!(!view.to_string().contains("PRIVATE_FAILURE"));
                assert!(!view.to_string().contains("/private/host"));
                updates += 1;
            }
            if result["subscription_done"] == true {
                break;
            }
        }
        assert_eq!(updates, 1);
        assert_eq!(owner.lock().unwrap().session().next_sequence(), journal);
        assert!(owner.lock().unwrap().session().turns().is_empty());
        drop(client);
        task.join().unwrap();
        return;
    }
    if mode == "selection-sync" {
        let before = result["subscription_view"]["model_status"]["selection_revision"]
            .as_u64()
            .unwrap();
        assert_eq!(
            result["subscription_view"]["model_status"]["model"],
            "gpt-5.2"
        );
        assert_eq!(
            result["subscription_view"]["model_status"]["reasoning_effort"],
            "high"
        );
        let (revision, journal) = {
            let mut agent = owner.lock().unwrap();
            agent.set_model(Some("gpt-5.2-2025-12-11".into())).unwrap();
            agent.set_reasoning_effort(Some("low".into())).unwrap();
            assert!(agent.session().turns().is_empty());
            (agent.selection_revision(), agent.session().next_sequence())
        };
        assert!(revision > before);
        gate_sender.send(()).unwrap();
        let mut updates = 0;
        loop {
            let frame = read_frame(&mut client).unwrap().unwrap();
            assert_eq!(frame.id, request.id);
            assert_eq!(frame.execution_state, Some(ExecutionState::Succeeded));
            let Message::Response { result } = frame.payload else {
                panic!("selection changes must not become transcript events");
            };
            assert_eq!(result["cursor"]["next_sequence"], 0);
            if let Some(view) = result.get("subscription_view") {
                let model = &view["model_status"];
                assert_eq!(model["model"], "gpt-5.2-2025-12-11");
                assert_eq!(model["reasoning_effort"], "low");
                assert_eq!(model["selection_revision"], revision);
                assert_ne!(model["catalog"]["snapshot_id"], serde_json::Value::Null);
                assert!(!view.to_string().contains("PRIVATE_MODEL_CREDENTIAL"));
                assert!(!view.to_string().contains("127.0.0.1:9"));
                updates += 1;
            }
            if result["subscription_done"] == true {
                break;
            }
        }
        assert_eq!(updates, 1);
        assert_eq!(owner.lock().unwrap().session().next_sequence(), journal);
        drop(client);
        task.join().unwrap();
        return;
    }
    if mode == "session-name-sync" {
        assert!(result["subscription_view"]["listing"]["projects"][0]["session_name"].is_null());
        owner
            .lock()
            .unwrap()
            .session_mut()
            .rename("Local desktop name")
            .unwrap();
        let journal = owner.lock().unwrap().session().next_sequence();
        gate_sender.send(()).unwrap();
        let mut updates = 0;
        loop {
            let frame = read_frame(&mut client).unwrap().unwrap();
            assert_eq!(frame.execution_state, Some(ExecutionState::Succeeded));
            let Message::Response { result } = frame.payload else {
                panic!("name change is metadata, not transcript")
            };
            assert_eq!(result["cursor"]["next_sequence"], 0);
            if let Some(view) = result.get("subscription_view") {
                assert_eq!(
                    view["listing"]["projects"][0]["session_name"],
                    "Local desktop name"
                );
                updates += 1;
            }
            if result["subscription_done"] == true {
                break;
            }
        }
        assert_eq!(updates, 1);
        assert_eq!(owner.lock().unwrap().session().next_sequence(), journal);
        assert!(owner.lock().unwrap().history().is_empty());
        drop(client);
        task.join().unwrap();
        return;
    }
    if mode == "session-switch-sync" {
        let previous = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
        {
            let mut host = pool.lock().unwrap();
            let mut replacement = Envelope::new(
                Channel::Control,
                Message::Request {
                    operation: Operation::SessionNew,
                    arguments: serde_json::json!({}),
                },
            );
            replacement.id = Some("other-device-switch".into());
            replacement.project_id = Some(project.clone());
            replacement.session_id = Some(session_id.clone());
            replacement.required_owner = Some(host.workspace_owner().into());
            replacement.expected_revision = Some(host.workspace_revision());
            let mut challenge = DeviceChallenge::new(fingerprint.clone()).unwrap();
            let proof = URL_SAFE_NO_PAD.encode(
                key.sign(&random, &challenge.proof_message(&record.id))
                    .unwrap()
                    .as_ref(),
            );
            let device = challenge
                .authenticate(&registry, &record.id, &proof)
                .unwrap();
            let response = zenpi::remote::sessions::new_session(
                &mut host,
                &registry,
                &device,
                &replacement,
                false,
            );
            assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
        }
        gate_sender.send(()).unwrap();
        let frame = read_frame(&mut client).unwrap().unwrap();
        assert_eq!(
            frame.code,
            Some(zenpi::remote::protocol::ErrorCode::SessionChanged)
        );
        assert!(matches!(frame.payload, Message::Response {result} if result.is_null()));
        assert_eq!(
            std::fs::read(root.path().join("session.jsonl")).unwrap(),
            previous
        );
        let current = pool
            .lock()
            .unwrap()
            .context(&project)
            .unwrap()
            .session_id
            .clone();
        assert_ne!(current, session_id);
        let fresh_bytes = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
        let mut inspect = Envelope::new(
            Channel::Control,
            Message::Request {
                operation: Operation::Inspect,
                arguments: serde_json::json!({}),
            },
        );
        inspect.id = Some("fresh-snapshot".into());
        inspect.project_id = Some(project.clone());
        inspect.session_id = Some(current.clone());
        let mut challenge = DeviceChallenge::new(fingerprint.clone()).unwrap();
        let proof = URL_SAFE_NO_PAD.encode(
            key.sign(&random, &challenge.proof_message(&record.id))
                .unwrap()
                .as_ref(),
        );
        let device = challenge
            .authenticate(&registry, &record.id, &proof)
            .unwrap();
        for (index, payload) in [
            Message::Ack {
                cursor: EventCursor {
                    session_id: session_id.clone(),
                    next_sequence: 0,
                },
            },
            Message::Resume {
                cursor: EventCursor {
                    session_id: session_id.clone(),
                    next_sequence: 0,
                },
                limit: 128,
            },
            Message::Request {
                operation: Operation::SubtabList,
                arguments: serde_json::json!({}),
            },
            Message::Request {
                operation: Operation::Inspect,
                arguments: serde_json::json!({}),
            },
        ]
        .into_iter()
        .enumerate()
        {
            let mut stale = Envelope::new(Channel::Control, payload);
            stale.id = Some(format!("stale-read-{index}"));
            stale.project_id = Some(project.clone());
            stale.session_id = Some(session_id.clone());
            let refused =
                HostAdapter::dispatch_owned(&mut pool.lock().unwrap(), &registry, &device, stale);
            assert_eq!(
                refused.code,
                Some(zenpi::remote::protocol::ErrorCode::OwnerMismatch)
            );
        }
        let mut listing = Envelope::new(
            Channel::Control,
            Message::Request {
                operation: Operation::WorkspaceList,
                arguments: serde_json::json!({}),
            },
        );
        listing.id = Some("recheck-authorized-session".into());
        let listed =
            HostAdapter::dispatch_owned(&mut pool.lock().unwrap(), &registry, &device, listing);
        assert_eq!(listed.execution_state, Some(ExecutionState::Succeeded));
        assert!(matches!(listed.payload, Message::Response {result}
            if result["projects"].as_array().unwrap().iter().any(|entry|
                entry["id"] == project && entry["session_id"] == current)));
        let snapshot =
            HostAdapter::dispatch_owned(&mut pool.lock().unwrap(), &registry, &device, inspect);
        assert_eq!(snapshot.execution_state, Some(ExecutionState::Succeeded));
        assert!(
            matches!(snapshot.payload,Message::Response {result} if result["session_id"]==current && result["transcript"].as_array().unwrap().is_empty() && result["event_cursor"]["next_sequence"]==0)
        );
        assert_eq!(
            std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
            fresh_bytes
        );
        drop(client);
        task.join().unwrap();
        return;
    }
    if matches!(mode, "revoke" | "grant-change" | "revoke-event") {
        if mode == "revoke-event" {
            owner
                .lock()
                .unwrap()
                .session_mut()
                .append_turn(Turn::new(
                    "unconfirmed",
                    TurnRole::Assistant,
                    "unconfirmed before revoke",
                ))
                .unwrap();
            let event = read_frame(&mut client).unwrap().unwrap();
            assert!(matches!(event.payload, Message::Event { .. }));
            assert_eq!(event.sequence, Some(0));
        }
        let journal = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
        if mode != "grant-change" {
            registry.revoke(&record.id).unwrap();
        } else {
            registry
                .set_permissions(
                    &record.id,
                    registry.lookup(&record.id).unwrap().revision,
                    vec![Capability::Read],
                    vec![],
                )
                .unwrap();
        }
        gate_sender.send(()).unwrap();
        let refusal = read_frame(&mut client).unwrap().unwrap();
        assert_eq!(refusal.id, request.id);
        assert_eq!(refusal.project_id, request.project_id);
        assert_eq!(refusal.execution_state, Some(ExecutionState::Failed));
        assert_eq!(refusal.code, Some(ErrorCode::Unauthorized));
        assert_eq!(
            refusal.payload,
            Message::Response {
                result: serde_json::Value::Null
            }
        );
        assert!(matches!(read_frame(&mut client), Ok(None) | Err(_)));
        drop(client);
        assert!(task.join().is_err());
        assert_eq!(
            std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
            journal
        );
        return;
    }
    if matches!(mode, "local-subtab" | "remote-subtab") {
        let before = result["subscription_view"]["subtabs"]["subtab_revision"]
            .as_u64()
            .unwrap();
        if mode == "local-subtab" {
            let mut host = pool.lock().unwrap();
            let state = host.subtab_state(&project).unwrap();
            let candidate = state.rename(&state.active_id, "committed-local").unwrap();
            host.commit_subtabs(&project, state.revision, candidate)
                .unwrap();
        } else {
            let mut challenge = DeviceChallenge::new(fingerprint.clone()).unwrap();
            let message = serde_json::to_vec(&[
                "zenpi-device-auth-v1",
                &fingerprint,
                challenge.nonce(),
                &record.id,
            ])
            .unwrap();
            let authenticated = challenge
                .authenticate(
                    &registry,
                    &record.id,
                    &URL_SAFE_NO_PAD.encode(key.sign(&random, &message).unwrap().as_ref()),
                )
                .unwrap();
            let mut host = pool.lock().unwrap();
            let state = host.subtab_state(&project).unwrap();
            let mut rename = Envelope::new(
                Channel::Control,
                Message::Request {
                    operation: Operation::SubtabRename,
                    arguments: serde_json::json!({"subtab_revision":state.revision,
                    "subtab_id":state.active_id,"name":"committed-remote"}),
                },
            );
            rename.id = Some("rename-on-other-connection".into());
            rename.project_id = request.project_id.clone();
            rename.session_id = request.session_id.clone();
            rename.required_owner = Some(host.workspace_owner().into());
            rename.expected_revision = Some(host.workspace_revision());
            let changed =
                zenpi::worktree::subtab::mutate(&mut host, &registry, &authenticated, &rename)
                    .unwrap();
            let signal =
                zenpi::remote::wakeup::watch(request.session_id.as_deref().unwrap()).unwrap();
            let observed = signal.revision();
            assert_eq!(
                zenpi::worktree::subtab::mutate(&mut host, &registry, &authenticated, &rename)
                    .unwrap(),
                changed
            );
            assert_eq!(signal.revision(), observed);
        }
        gate_sender.send(()).unwrap();
        let mut updates = 0;
        loop {
            let frame = read_frame(&mut client).unwrap().unwrap();
            assert_eq!(frame.id, request.id);
            assert_eq!(frame.execution_state, Some(ExecutionState::Succeeded));
            let Message::Response { result } = frame.payload else {
                panic!("workspace-only mutation must not emit transcript events");
            };
            assert_eq!(result["cursor"]["next_sequence"], 0);
            assert_eq!(result["cursor_scope"], "transcript_events_v1");
            if let Some(view) = result.get("subscription_view") {
                assert_eq!(view["subtabs"]["subtab_revision"], before + 1);
                assert_eq!(
                    view["subtabs"]["tabs"][0]["name"],
                    if mode == "local-subtab" {
                        "committed-local"
                    } else {
                        "committed-remote"
                    }
                );
                assert_eq!(
                    view["listing"]["workspace_revision"],
                    view["subtabs"]["workspace_revision"]
                );
                updates += 1;
            }
            if result["subscription_done"] == true {
                break;
            }
        }
        assert_eq!(updates, 1);
        assert_eq!(owner.lock().unwrap().session().turns().len(), 0);
        drop(client);
        task.join().unwrap();
        return;
    }
    let answer = if mode == "large-turn" {
        format!(
            "api_key=PRIVATE_LARGE_TLS\n{}",
            "large TLS answer ".repeat(10_000)
        )
    } else if mode == "private-pem" {
        "TLS safe before\n-----BEGIN PRIVATE KEY-----\nPRIVATE_BODY_TLS_6Rd8\n-----END PRIVATE KEY-----\nTLS safe after\n"
            .to_owned()
    } else {
        "TLS durable answer".to_owned()
    };
    owner
        .lock()
        .unwrap()
        .session_mut()
        .append_turn(Turn::new("delivered", TurnRole::Assistant, answer.clone()))
        .unwrap();
    let mut delivered = 0;
    loop {
        let frame = read_frame(&mut client).unwrap().unwrap();
        match frame.payload {
            Message::Event { name, data } => {
                assert_eq!(name, "transcript_turn");
                assert_eq!(frame.sequence, Some(0));
                if mode == "large-turn" {
                    assert_eq!(data["content_truncated"], true);
                    assert!(
                        data["content"].as_str().unwrap().len()
                            <= zenpi::remote::events::MAX_TURN_CONTENT_BYTES
                    );
                    assert!(
                        data["content"]
                            .as_str()
                            .unwrap()
                            .contains("large TLS answer")
                    );
                    assert!(!data.to_string().contains("PRIVATE_LARGE_TLS"));
                } else if mode == "private-pem" {
                    let content = data["content"].as_str().unwrap();
                    assert!(content.starts_with("TLS safe before\n"));
                    assert!(content.ends_with("TLS safe after\n"));
                    assert!(content.contains("<redacted private key>"));
                    assert!(!data.to_string().contains("PRIVATE_BODY_TLS"));
                } else {
                    assert_eq!(data["content"], answer);
                }
                delivered += 1;
            }
            Message::Response { result } => {
                assert_eq!(frame.id, request.id);
                if result["subscription_done"] == true {
                    assert_eq!(result["cursor"]["next_sequence"], 1);
                    break;
                }
            }
            _ => panic!("unexpected encrypted frame"),
        }
    }
    assert_eq!(delivered, 1);
    assert_eq!(owner.lock().unwrap().session().turns()[0].content, answer);
    drop(client);
    task.join().unwrap();
}

#[test]
fn authenticated_tls_delivers_new_durable_turn_without_reconnect() {
    exercise("deliver");
}

#[test]
fn authenticated_tls_session_catalog_and_resume_retry_use_canonical_owner() {
    exercise("session-resume");
}

#[test]
fn authenticated_tls_pushes_local_session_name_without_transcript_event() {
    exercise("session-name-sync");
}

#[test]
fn authenticated_tls_session_switch_finishes_old_subscription_with_typed_refresh() {
    exercise("session-switch-sync");
}
#[test]
fn authenticated_tls_syncs_typed_prompt_failure_without_raw_error_or_transcript_event() {
    exercise("failure-sync");
}
#[test]
fn authenticated_tls_redacts_private_pem_before_transcript_delivery() {
    exercise("private-pem");
}
#[test]
fn authenticated_tls_delivers_large_redacted_turn_prefix_and_matching_cursor() {
    exercise("large-turn");
}

#[test]
fn authenticated_tls_pages_reconstruct_redacted_turn_without_journal_or_ack_changes() {
    exercise("transcript-deliver");
}
#[test]
fn authenticated_tls_pages_reconstruct_ordinary_tool_arguments_without_metadata_or_writes() {
    exercise("transcript-tool-deliver");
}
#[test]
fn authenticated_tls_revocation_refuses_next_ordinary_tool_argument_page() {
    exercise("transcript-tool-revoke");
}
#[test]
fn authenticated_tls_stale_digest_refuses_next_ordinary_tool_argument_page() {
    exercise("transcript-tool-stale");
}
#[test]
fn authenticated_tls_revocation_refuses_next_transcript_page_without_data() {
    exercise("transcript-revoke");
}
#[test]
fn authenticated_tls_stale_digest_refuses_next_transcript_page_without_data() {
    exercise("transcript-stale");
}
#[test]
fn authenticated_tls_branch_switch_refuses_old_page_and_replaces_transcript_without_cursor_rewind()
{
    exercise("transcript-branch");
}
#[test]
fn authenticated_tls_model_status_exposes_registry_capabilities_without_credentials() {
    exercise("model-registry");
}
#[test]
fn authenticated_tls_model_selection_retry_commits_once() {
    exercise("model-select");
}
#[test]
fn authenticated_tls_reasoning_default_retry_commits_once_without_model_change() {
    exercise("model-reasoning");
}
#[test]
fn authenticated_tls_model_catalog_pages_are_snapshot_bound_and_private() {
    exercise("model-pages");
}
#[test]
fn authenticated_tls_pushes_local_model_and_reasoning_changes_without_transcript_ack() {
    exercise("selection-sync");
}
#[test]
fn authenticated_tls_model_status_is_allowlisted_and_has_no_read_side_effects() {
    exercise("model-deliver");
}
#[test]
fn authenticated_tls_model_status_rechecks_revoke_without_data() {
    exercise("model-revoke");
}
#[test]
fn authenticated_tls_model_status_refuses_foreign_session_without_data() {
    exercise("model-stale");
}
#[test]
fn authenticated_tls_template_without_description_never_projects_body_first_line() {
    exercise("resources-no-description");
}
#[test]
fn authenticated_tls_resource_next_page_rechecks_device_revocation_without_metadata() {
    exercise("resources-page-revoke");
}
#[test]
fn authenticated_tls_resource_pages_reach_tail_without_catalog_body_or_journal_changes() {
    exercise("resources-page");
}
#[test]
fn authenticated_tls_resource_pages_refuse_reload_without_mixing_snapshots() {
    exercise("resources-page-reload");
}
#[test]
fn authenticated_tls_resource_catalog_delivers_only_scoped_metadata_without_journal_changes() {
    exercise("resources-deliver");
}
#[test]
fn authenticated_tls_resource_catalog_rechecks_revoke_before_next_read() {
    exercise("resources-revoke");
}
#[test]
fn authenticated_tls_resource_catalog_refuses_foreign_session_without_metadata() {
    exercise("resources-stale");
}
#[test]
fn authenticated_tls_lists_only_selected_directory_metadata_without_journal_or_ack_changes() {
    exercise("files-deliver");
}
#[test]
fn authenticated_tls_diff_excludes_known_private_paths_and_refuses_explicit_private_reads() {
    exercise("diff-deliver");
}
#[test]
fn authenticated_tls_diff_rechecks_revocation_before_a_second_read() {
    exercise("diff-revoke");
}
#[test]
fn authenticated_tls_file_pages_reach_tail_without_contents_or_cursor_changes() {
    exercise("files-pages");
}
#[test]
fn authenticated_tls_file_navigation_refuses_revoke_without_directory_data() {
    exercise("files-revoke");
}
#[test]
fn authenticated_tls_file_navigation_refuses_changed_selection_without_directory_data() {
    exercise("files-stale");
}
#[test]
fn revoked_device_cannot_receive_a_later_encrypted_batch() {
    exercise("revoke");
}
#[test]
fn changed_device_grant_receives_correlated_refusal_not_later_subscription_data() {
    exercise("grant-change");
}
#[test]
fn revocation_after_event_before_page_receipt_emits_refusal_not_cursor_commit() {
    exercise("revoke-event");
}
#[test]
fn disconnected_peer_terminates_authorized_subscription() {
    exercise("disconnect");
}
#[test]
fn busy_owner_view_recovers_to_durable_transcript_on_same_tls_stream() {
    exercise("busy");
}
#[test]
fn wrong_certificate_fails_before_device_authentication_or_subscription() {
    exercise("wrong-certificate");
}
#[test]
fn invalid_device_proof_fails_before_subscription_dispatch() {
    exercise("wrong-proof");
}
#[test]
fn corrupt_encrypted_record_is_terminal_not_a_successful_subscription() {
    exercise("corrupt-record");
}
#[test]
fn committed_local_subtab_change_pushes_once_without_advancing_transcript_cursor() {
    exercise("local-subtab");
}
#[test]
fn committed_remote_subtab_change_and_exact_retry_push_once_over_existing_tls_subscription() {
    exercise("remote-subtab");
}
