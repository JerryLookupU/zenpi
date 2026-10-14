mod support;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use zenpi::{
    approval::{ApprovalMode, ApprovalPolicy},
    backend::{
        Backend, BackendError, Completion, CompletionRequest, OpenAiCompatibleBackend,
        OpenAiWireApi, ProviderEvent,
    },
    core::Agent,
    project_workspace::ProjectOwnerPool,
    remote::{
        auth::{
            AuthenticatedDevice, Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message,
        },
        headless_host::HeadlessHost,
        host_adapter::HostAdapter,
        protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message, Operation},
    },
    session::SessionStore,
    tools::{SideEffectPolicy, ToolContext, ToolRegistry},
};

fn phone(registry: &DeviceRegistry, project: &str, control: bool) -> AuthenticatedDevice {
    phone_projects(registry, &[project], control)
}
fn phone_projects(
    registry: &DeviceRegistry,
    projects: &[&str],
    control: bool,
) -> AuthenticatedDevice {
    let mut capabilities = vec![Capability::Read];
    if control {
        capabilities.push(Capability::Control);
    }
    phone_capabilities(registry, projects, capabilities)
}
fn phone_capabilities(
    registry: &DeviceRegistry,
    projects: &[&str],
    capabilities: Vec<Capability>,
) -> AuthenticatedDevice {
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
            capabilities,
            projects.iter().map(|project| (*project).into()).collect(),
        )
        .unwrap();
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let signed = URL_SAFE_NO_PAD.encode(
        key.sign(&random, &challenge.proof_message(&record.id))
            .unwrap()
            .as_ref(),
    );
    challenge
        .authenticate(registry, &record.id, &signed)
        .unwrap()
}

fn configured(
    root: &std::path::Path,
    backend: Box<dyn Backend>,
) -> (ProjectOwnerPool, Arc<Mutex<Agent>>) {
    let mut agent = Agent::new(
        SessionStore::open_in_workspace(root.join("session.jsonl"), root).unwrap(),
        backend,
    );
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(root).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    agent.set_approval_policy(ApprovalPolicy {
        mode: ApprovalMode::Never,
        ..Default::default()
    });
    let owner = Arc::new(Mutex::new(agent));
    let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
    pool.restore_checkpoint().unwrap();
    (pool, owner)
}

fn request(pool: &ProjectOwnerPool, id: &str, operation: Operation, arguments: Value) -> Envelope {
    let mut request = Envelope::new(
        Channel::Control,
        Message::Request {
            operation,
            arguments,
        },
    );
    request.id = Some(id.into());
    request.project_id = Some(pool.active_context().project_id.clone());
    request.session_id = Some(pool.active_context().session_id.clone());
    if operation.is_write() {
        request.required_owner = Some(pool.workspace_owner().into());
        request.expected_revision = Some(pool.workspace_revision());
    }
    request
}

fn listing(
    adapter: &HostAdapter,
    registry: &Arc<DeviceRegistry>,
    device: &AuthenticatedDevice,
) -> Value {
    let mut request = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::WorkspaceList,
            arguments: json!({}),
        },
    );
    request.id = Some("listing".into());
    let response = adapter.dispatch(registry, device, request);
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    let Message::Response { result } = response.payload else {
        panic!("response")
    };
    result
}

fn await_state(
    adapter: &HostAdapter,
    registry: &Arc<DeviceRegistry>,
    device: &AuthenticatedDevice,
    id: &str,
    state: &str,
) {
    let until = Instant::now() + Duration::from_secs(12);
    loop {
        let value = listing(adapter, registry, device);
        if value["projects"][0]["prompt_statuses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|status| status["request_id"] == id && status["state"] == state)
        {
            return;
        }
        assert!(Instant::now() < until, "expected {id} {state}: {value}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn read_http(stream: &mut TcpStream) -> Value {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut bytes = Vec::new();
    let end = loop {
        let mut buf = [0; 4096];
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&buf[..n]);
        if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
            break end + 4;
        }
        assert!(bytes.len() < 128 * 1024);
    };
    let headers = String::from_utf8(bytes[..end].to_vec()).unwrap();
    let len: usize = headers
        .lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.trim().parse().unwrap())
        })
        .unwrap();
    assert!(len < 256 * 1024);
    while bytes.len() < end + len {
        let mut buf = [0; 4096];
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&buf[..n]);
    }
    serde_json::from_slice(&bytes[end..end + len]).unwrap()
}

#[test]
fn headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    std::fs::write(root.join("marker.txt"), "MAIN_ONLY").unwrap();
    std::fs::write(child.join("marker.txt"), "CHILD_ONLY").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let backend = OpenAiCompatibleBackend::from_values_with_wire_api(
        format!("http://{}/v1", listener.local_addr().unwrap()),
        None,
        "fixture-model".into(),
        OpenAiWireApi::ChatCompletions,
    )
    .unwrap();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let received = captured.clone();
    let server = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        for index in 0..2 {
            let until = Instant::now() + Duration::from_secs(12);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < until =>
                    {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => panic!("accept: {error}"),
                }
            };
            received.lock().unwrap().push(read_http(&mut stream));
            let delta = if index == 0 {
                json!({"tool_calls":[{"index":0,"id":"read-marker","type":"function","function":{"name":"read_file","arguments":"{\"path\":\"marker.txt\"}"}}]})
            } else {
                json!({"content":"HEADLESS_HTTP_DONE"})
            };
            let first = json!({"id":"reply","model":"fixture-model","choices":[{"index":0,"delta":delta,"finish_reason":null}]});
            let last = json!({"id":"reply","model":"fixture-model","choices":[{"index":0,"delta":{},"finish_reason":if index==0 {"tool_calls"} else {"stop"}}]});
            let body = format!("data: {first}\n\ndata: {last}\n\ndata: [DONE]\n\n");
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        // A duplicate prompt must not issue a third HTTP request.
        let until = Instant::now() + Duration::from_millis(300);
        while Instant::now() < until {
            assert!(listener.accept().is_err());
            std::thread::sleep(Duration::from_millis(5));
        }
    });
    let (mut pool, owner) = configured(&root, Box::new(backend));
    let project = pool.active_context().project_id.clone();
    let main = pool.subtab_state(&project).unwrap();
    pool.commit_subtabs(
        &project,
        main.revision,
        main.add_directory(&project, &root, &child, "child")
            .unwrap(),
    )
    .unwrap();
    let prompt = request(
        &pool,
        "http-prompt",
        Operation::Prompt,
        json!({"text":"read the marker"}),
    );
    let inspect = request(&pool, "inspect", Operation::Inspect, json!({}));
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let device = phone(&registry, &project, true);
    let reader = phone(&registry, &project, false);
    let (adapter, host) = HeadlessHost::start(pool);
    assert_eq!(
        listing(&adapter, &registry, &device)["projects"][0]["prompt_control"],
        true
    );
    assert_eq!(
        listing(&adapter, &registry, &reader)["projects"][0]["prompt_control"],
        false
    );
    assert_eq!(
        adapter.dispatch(&registry, &reader, prompt.clone()).code,
        Some(ErrorCode::Unauthorized)
    );
    assert_eq!(
        adapter
            .dispatch(&registry, &device, prompt.clone())
            .execution_state,
        Some(ExecutionState::Accepted)
    );
    await_state(&adapter, &registry, &device, "http-prompt", "succeeded");
    let result = adapter.dispatch(&registry, &device, inspect);
    let Message::Response { result } = result.payload else {
        panic!("response")
    };
    assert!(
        result["transcript"]
            .to_string()
            .contains("HEADLESS_HTTP_DONE")
    );
    assert_eq!(
        adapter
            .dispatch(&registry, &device, prompt.clone())
            .execution_state,
        Some(ExecutionState::Accepted)
    );
    let mut changed = prompt.clone();
    changed.payload = Message::Request {
        operation: Operation::Prompt,
        arguments: json!({"text":"different"}),
    };
    assert_eq!(
        adapter.dispatch(&registry, &device, changed).code,
        Some(ErrorCode::RequestIdConflict)
    );
    server.join().unwrap();
    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[1]["messages"].to_string().contains("CHILD_ONLY"));
    assert!(!requests[1]["messages"].to_string().contains("MAIN_ONLY"));
    drop(requests);
    host.shutdown();
    assert_eq!(
        owner.lock().unwrap().attachment_workspace_root(),
        Some(root.as_path())
    );
    let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
    pool.restore_checkpoint().unwrap();
    let (adapter, host) = HeadlessHost::start(pool);
    assert_eq!(
        adapter.dispatch(&registry, &device, prompt).execution_state,
        Some(ExecutionState::Accepted)
    );
    await_state(&adapter, &registry, &device, "http-prompt", "succeeded");
    registry.revoke(device.id()).unwrap();
    assert_eq!(
        adapter
            .dispatch(
                &registry,
                &device,
                request(
                    &ProjectOwnerPool::new(owner).unwrap(),
                    "revoked",
                    Operation::Inspect,
                    json!({})
                )
            )
            .code,
        Some(ErrorCode::Unauthorized)
    );
    host.shutdown();
}

struct BlockingBackend {
    entered: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
}

struct AttachmentBackend(Arc<Mutex<Vec<Vec<zenpi::backend::RequestAttachment>>>>);
impl Backend for AttachmentBackend {
    crate::support::controlled_local_backend!();
    fn name(&self) -> &str {
        "attachment-fixture"
    }
    fn complete(&self, request: CompletionRequest<'_>) -> Result<Completion, BackendError> {
        self.0.lock().unwrap().push(request.attachments.to_vec());
        Ok(Completion::text("attachment received"))
    }
}

#[test]
fn headless_catalog_selection_expands_skill_and_template_on_host_and_retry_does_not_repeat() {
    struct ResourceBackend(Arc<Mutex<Vec<(Vec<zenpi::core::Turn>, String)>>>);
    impl Backend for ResourceBackend {
        crate::support::controlled_local_backend!();
        fn name(&self) -> &str {
            "resource-fixture"
        }
        fn complete(&self, request: CompletionRequest<'_>) -> Result<Completion, BackendError> {
            self.0.lock().unwrap().push((
                request.turns.to_vec(),
                request.instructions.unwrap_or_default().to_owned(),
            ));
            Ok(Completion::text("resource received"))
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    std::fs::create_dir_all(root.join("skills/research")).unwrap();
    std::fs::create_dir_all(root.join("prompts")).unwrap();
    std::fs::write(
        root.join("skills/research/SKILL.md"),
        "---\nname: research\ndescription: Find sources\n---\nHEADLESS_PRIVATE_SKILL_BODY\n",
    )
    .unwrap();
    std::fs::write(
        root.join("prompts/zz-stage2-review.md"),
        "---\ndescription: Stage2 selected template\n---\nSTAGE2_HOST_TEMPLATE_BODY\n$ARGUMENTS\n",
    )
    .unwrap();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let (pool, owner) = configured(&root, Box::new(ResourceBackend(captured.clone())));
    for index in 0..40 {
        std::fs::write(
            root.join(format!("prompts/fixture-{index:02}.md")),
            if index % 2 == 0 {
                "HEADLESS_PRIVATE_PAGE_BODY\n"
            } else {
                "---\ndescription: Page entry\n---\nHEADLESS_PRIVATE_PAGE_BODY\n"
            },
        )
        .unwrap();
    }
    owner
        .lock()
        .unwrap()
        .configure_resources(
            zenpi::resource_loader::ResourcePaths {
                user_skills: root.join("empty-skills"),
                project_skills: root.join("skills"),
                skill_paths: vec![],
                user_templates: root.join("empty-prompts"),
                project_templates: root.join("prompts"),
                template_paths: vec![],
                text_resources: vec![],
            },
            || false,
        )
        .unwrap();
    let project = pool.active_context().project_id.clone();
    let catalog_request = request(&pool, "catalog", Operation::ResourcesList, json!({}));
    let mut tail_request = catalog_request.clone();
    tail_request.id = Some("catalog-tail".into());
    let prompts = [
        "/skill:research literal $1 $(touch never-created)",
        "/zz-stage2-review STAGE2_HEADLESS_ANDROID_SECOND literal $1 $(touch never-created)",
    ]
    .into_iter()
    .enumerate()
    .map(|(index, text)| {
        request(
            &pool,
            &format!("resource-{index}"),
            Operation::Prompt,
            json!({"text":text}),
        )
    })
    .collect::<Vec<_>>();
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let device = phone(&registry, &project, true);
    let (adapter, host) = HeadlessHost::start(pool);
    let before = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    let catalog = adapter.dispatch(&registry, &device, catalog_request);
    assert_eq!(catalog.execution_state, Some(ExecutionState::Succeeded));
    assert!(
        !serde_json::to_string(&catalog)
            .unwrap()
            .contains("PRIVATE_")
    );
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        before
    );
    let Message::Response { result: catalog } = catalog.payload else {
        panic!("catalog")
    };
    assert_eq!(catalog["entries"].as_array().unwrap().len(), 32);
    tail_request.payload = Message::Request {
        operation: Operation::ResourcesList,
        arguments: json!({"offset":32,"snapshot_id":catalog["snapshot_id"]}),
    };
    let response = adapter.dispatch(&registry, &device, tail_request);
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    let Message::Response { result: tail } = response.payload else {
        panic!("tail")
    };
    assert!(!tail.to_string().contains("HEADLESS_PRIVATE_PAGE_BODY"));
    assert_eq!(tail["entries"][9]["command"], "/zz-stage2-review");
    assert!(tail["next_offset"].is_null());
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        before
    );
    for prompt in prompts {
        let accepted = adapter.dispatch(&registry, &device, prompt.clone());
        assert_eq!(accepted.execution_state, Some(ExecutionState::Accepted));
        await_state(
            &adapter,
            &registry,
            &device,
            prompt.id.as_deref().unwrap(),
            "succeeded",
        );
        assert_eq!(
            adapter.dispatch(&registry, &device, prompt).payload,
            accepted.payload
        );
    }
    host.shutdown();
    let values = captured.lock().unwrap();
    assert_eq!(values.len(), 2);
    assert!(values[0].1.contains("HEADLESS_PRIVATE_SKILL_BODY"));
    assert!(
        values[0]
            .0
            .last()
            .unwrap()
            .content
            .contains("literal $1 $(touch never-created)")
    );
    assert_eq!(
        values[1].0.last().unwrap().content,
        "STAGE2_HOST_TEMPLATE_BODY\nSTAGE2_HEADLESS_ANDROID_SECOND literal $1 $(touch never-created)"
    );
    assert!(!root.join("never-created").exists());
    let journal = std::fs::read_to_string(owner.lock().unwrap().session().path()).unwrap();
    assert!(journal.contains("/skill:research literal $1"));
    assert!(journal.contains(
        "/zz-stage2-review STAGE2_HEADLESS_ANDROID_SECOND literal $1 $(touch never-created)"
    ));
    assert!(!journal.contains("HEADLESS_PRIVATE_SKILL_BODY"));
    assert!(!journal.contains("STAGE2_HOST_TEMPLATE_BODY"));
    let records: Vec<Value> = journal
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let resource = &records.iter().find(|record| record["turn"]["content"] == "/zz-stage2-review STAGE2_HEADLESS_ANDROID_SECOND literal $1 $(touch never-created)")
        .unwrap()["turn"]["metadata"]["resource"];
    use sha2::Digest;
    assert_eq!(resource["kind"], "template");
    assert_eq!(resource["name"], "zz-stage2-review");
    assert_eq!(
        resource["source"],
        root.join("prompts/zz-stage2-review.md")
            .to_string_lossy()
            .as_ref()
    );
    assert_eq!(
        resource["source_hash"],
        format!(
            "{:x}",
            sha2::Sha256::digest(std::fs::read(root.join("prompts/zz-stage2-review.md")).unwrap())
        )
    );
}

#[test]
fn headless_new_session_uses_existing_owner_and_retries_original_receipt() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (pool, owner) = configured(&root, Box::new(zenpi::backend::EchoBackend));
    let project = pool.active_context().project_id.clone();
    let old_session = pool.active_context().session_id.clone();
    let rename = request(
        &pool,
        "rename-session",
        Operation::SessionRename,
        json!({"name":"Headless development","journal_revision":owner.lock().unwrap().session().next_sequence()}),
    );
    let new = request(&pool, "new-session", Operation::SessionNew, json!({}));
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let device = phone(&registry, &project, true);
    let (adapter, host) = HeadlessHost::start(pool);
    let renamed = adapter.dispatch(&registry, &device, rename.clone());
    assert_eq!(renamed.execution_state, Some(ExecutionState::Succeeded));
    assert_eq!(
        owner.lock().unwrap().session().session_name(),
        Some("Headless development")
    );
    let bytes = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    assert_eq!(adapter.dispatch(&registry, &device, rename), renamed);
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        bytes
    );
    assert_eq!(
        listing(&adapter, &registry, &device)["projects"][0]["session_name"],
        "Headless development"
    );
    assert_eq!(
        listing(&adapter, &registry, &device)["projects"][0]["session_control"],
        true
    );
    let response = adapter.dispatch(&registry, &device, new.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    assert_ne!(owner.lock().unwrap().session().session_id(), old_session);
    assert_eq!(adapter.dispatch(&registry, &device, new), response);
    let Message::Response { result } = response.payload else {
        panic!("new session")
    };
    assert_eq!(
        listing(&adapter, &registry, &device)["projects"][0]["session_id"],
        result["new_session_id"]
    );
    let mut read = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::SessionList,
            arguments: json!({}),
        },
    );
    read.id = Some("session-catalog".into());
    read.project_id = Some(project.clone());
    read.session_id = result["new_session_id"].as_str().map(str::to_owned);
    let catalog = adapter.dispatch(&registry, &device, read.clone());
    assert_eq!(catalog.execution_state, Some(ExecutionState::Succeeded));
    let Message::Response { result: catalog } = catalog.payload else {
        panic!("catalog")
    };
    assert!(
        catalog["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["session_id"] == old_session)
    );
    let mut resume = read;
    resume.id = Some("resume-session".into());
    resume.required_owner = result["required_owner"].as_str().map(str::to_owned);
    resume.expected_revision = result["workspace_revision"].as_u64();
    resume.payload = Message::Request {
        operation: Operation::SessionResume,
        arguments: json!({"target_session_id":old_session,"snapshot_id":catalog["snapshot_id"]}),
    };
    let response = adapter.dispatch(&registry, &device, resume.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    assert_eq!(owner.lock().unwrap().session().session_id(), old_session);
    assert_eq!(adapter.dispatch(&registry, &device, resume), response);
    host.shutdown();
}

#[test]
fn headless_resource_reload_routes_to_existing_owner_and_returns_exact_receipt() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    std::fs::create_dir(root.join("prompts")).unwrap();
    std::fs::write(
        root.join("prompts/fixture.md"),
        "---\ndescription: before\n---\nPRIVATE_BODY",
    )
    .unwrap();
    let (pool, owner) = configured(&root, Box::new(zenpi::backend::EchoBackend));
    owner
        .lock()
        .unwrap()
        .configure_resources(
            zenpi::resource_loader::ResourcePaths {
                user_skills: root.join("empty-skills"),
                project_skills: root.join("skills"),
                skill_paths: vec![],
                user_templates: root.join("empty-prompts"),
                project_templates: root.join("prompts"),
                template_paths: vec![],
                text_resources: vec![],
            },
            || false,
        )
        .unwrap();
    let project = pool.active_context().project_id.clone();
    let read = request(&pool, "resources", Operation::ResourcesList, json!({}));
    let mut reload = request(&pool, "reload", Operation::ResourcesReload, json!({}));
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let device = phone(&registry, &project, true);
    let (adapter, host) = HeadlessHost::start(pool);
    let catalog = adapter.dispatch(&registry, &device, read.clone());
    let Message::Response { result } = catalog.payload else {
        panic!("catalog")
    };
    reload.payload = Message::Request {
        operation: Operation::ResourcesReload,
        arguments: json!({"generation":result["generation"],"snapshot_id":result["snapshot_id"]}),
    };
    std::fs::write(
        root.join("prompts/fixture.md"),
        "---\ndescription: after\n---\nNEW_PRIVATE_BODY",
    )
    .unwrap();
    let receipt = adapter.dispatch(&registry, &device, reload.clone());
    assert_eq!(receipt.execution_state, Some(ExecutionState::Succeeded));
    assert_eq!(adapter.dispatch(&registry, &device, reload), receipt);
    assert!(!format!("{receipt:?}").contains("PRIVATE_BODY"));
    let catalog = adapter.dispatch(&registry, &device, read);
    let Message::Response { result: updated } = catalog.payload else {
        panic!("catalog")
    };
    assert_eq!(
        updated["generation"].as_u64().unwrap(),
        result["generation"].as_u64().unwrap() + 1
    );
    assert_eq!(updated["entries"][0]["description"], "after");
    assert_ne!(updated["snapshot_id"], result["snapshot_id"]);
    assert!(
        listing(&adapter, &registry, &device)["projects"][0]["resources_reload_control"]
            .as_bool()
            .unwrap()
    );
    host.shutdown();
    assert_eq!(
        owner
            .lock()
            .unwrap()
            .session()
            .events()
            .iter()
            .filter(|event| event["type"] == "remote_resources_admission_v1")
            .count(),
        2
    );
}

#[test]
fn headless_prompt_file_references_materialize_only_in_selected_workspace_and_retry_once() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    std::fs::write(root.join("note name.txt"), b"WRONG_PRIMARY_BYTES").unwrap();
    std::fs::write(child.join("note name.txt"), b"SELECTED_ATTACHMENT_BYTES").unwrap();
    std::fs::write(
        child.join("credentials.json"),
        b"PRIVATE_ATTACHMENT_MUST_NOT_REACH_PROVIDER",
    )
    .unwrap();
    std::fs::File::create(child.join("too-large.bin"))
        .unwrap()
        .set_len(zenpi::backend::MAX_ATTACHMENT_BYTES as u64 + 1)
        .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("note name.txt"), child.join("linked.txt")).unwrap();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let (mut pool, owner) = configured(&root, Box::new(AttachmentBackend(captured.clone())));
    let project = pool.active_context().project_id.clone();
    let main = pool.subtab_state(&project).unwrap();
    pool.commit_subtabs(
        &project,
        main.revision,
        main.add_directory(&project, &root, &child, "child")
            .unwrap(),
    )
    .unwrap();
    let prompt = request(
        &pool,
        "attachment-prompt",
        Operation::Prompt,
        json!({"text":"inspect @\"note name.txt\""}),
    );
    let local_execution = pool.subtab_execution(&project).unwrap();
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let device = phone(&registry, &project, true);
    let staged = zenpi::slash_actions::attachment_for_path(&root, "note name.txt").unwrap();
    owner
        .lock()
        .unwrap()
        .stage_attachment(staged.clone())
        .unwrap();
    let (adapter, host) = HeadlessHost::start(pool);
    assert_eq!(
        adapter.dispatch(&registry, &device, prompt.clone()).code,
        Some(ErrorCode::Busy)
    );
    assert!(captured.lock().unwrap().is_empty());
    assert_eq!(owner.lock().unwrap().pending_attachments(), &[staged]);
    owner.lock().unwrap().clear_pending_attachments();
    let response = adapter.dispatch(&registry, &device, prompt.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Accepted));
    await_state(
        &adapter,
        &registry,
        &device,
        "attachment-prompt",
        "succeeded",
    );
    let retry = adapter.dispatch(&registry, &device, prompt.clone());
    assert_eq!(retry.payload, response.payload);
    assert_eq!(captured.lock().unwrap().len(), 1);
    let before_private = std::fs::read(owner.lock().unwrap().session().path()).unwrap();
    let mut private = prompt.clone();
    private.id = Some("private-attachment".into());
    private.payload = Message::Request {
        operation: Operation::Prompt,
        arguments: json!({"text":"inspect @credentials.json"}),
    };
    let refused = adapter.dispatch(&registry, &device, private);
    assert_eq!(refused.execution_state, Some(ExecutionState::Failed));
    assert_eq!(refused.code, Some(ErrorCode::InvalidRequest));
    assert_eq!(captured.lock().unwrap().len(), 1);
    assert_eq!(
        std::fs::read(owner.lock().unwrap().session().path()).unwrap(),
        before_private
    );
    {
        let values = captured.lock().unwrap();
        let attachment = &values[0][0];
        assert_eq!(values[0].len(), 1);
        assert_eq!(attachment.input.path.as_deref(), Some("note name.txt"));
        assert_eq!(
            attachment.data.as_deref(),
            Some(b"SELECTED_ATTACHMENT_BYTES".as_slice())
        );
    }
    let mut invalid_texts = vec![
        "inspect @../note.txt".to_owned(),
        "inspect @\"unfinished".into(),
        "inspect @missing.txt".into(),
        "inspect @too-large.bin".into(),
        "@file ".repeat(9),
    ];
    #[cfg(unix)]
    invalid_texts.push("inspect @linked.txt".into());
    for (index, text) in invalid_texts.into_iter().enumerate() {
        let mut invalid = prompt.clone();
        invalid.id = Some(format!("bad-attachment-{index}"));
        invalid.payload = Message::Request {
            operation: Operation::Prompt,
            arguments: json!({"text":text}),
        };
        let admitted = adapter.dispatch(&registry, &device, invalid);
        assert_eq!(admitted.execution_state, Some(ExecutionState::Accepted));
        await_state(
            &adapter,
            &registry,
            &device,
            &format!("bad-attachment-{index}"),
            "failed",
        );
        let listed = listing(&adapter, &registry, &device);
        let status = listed["projects"][0]["prompt_statuses"]
            .as_array()
            .unwrap()
            .iter()
            .find(|status| status["request_id"] == format!("bad-attachment-{index}"))
            .unwrap();
        assert_eq!(status["failure_code"], "invalid_request");
        assert!(!status.to_string().contains("path"));
    }
    assert_eq!(captured.lock().unwrap().len(), 1);
    host.shutdown();
    let mut agent = owner.lock().unwrap();
    local_execution
        .run(&mut agent, |agent| {
            let text = "inspect @\"note name.txt\"";
            let attachments = zenpi::slash_actions::prompt_attachments(agent, text).unwrap();
            agent.process_with_cancel_and_events(
                zenpi::core::TurnInputRequest::new(text)
                    .with_mode(zenpi::protocol::TurnMode::StartIfIdle)
                    .with_attachments(attachments),
                || false,
                &mut |_| Ok(()),
            )
        })
        .unwrap()
        .unwrap();
    let values = captured.lock().unwrap();
    assert_eq!(values.len(), 2);
    assert_eq!(values[0][0].input, values[1][0].input);
    assert_eq!(values[0][0].data, values[1][0].data);
    assert_eq!(values[0][0].sha256, values[1][0].sha256);
    assert_eq!(agent.attachment_workspace_root(), Some(root.as_path()));
    let journal = std::fs::read_to_string(agent.session().path()).unwrap();
    assert!(!journal.contains("SELECTED_ATTACHMENT_BYTES"));
    assert!(!journal.contains("WRONG_PRIMARY_BYTES"));
}
struct EchoNamedBlockingBackend(BlockingBackend);
impl Backend for EchoNamedBlockingBackend {
    crate::support::controlled_local_backend!();
    fn name(&self) -> &str {
        "echo"
    }
    fn complete(&self, _: CompletionRequest<'_>) -> Result<Completion, BackendError> {
        panic!("controlled boundary required")
    }
    fn complete_with_control(
        &self,
        request: CompletionRequest<'_>,
        cancelled: &dyn Fn() -> bool,
        on_event: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
    ) -> Result<Completion, BackendError> {
        self.0.complete_with_control(request, cancelled, on_event)
    }
}

#[test]
fn cancelling_one_parallel_project_does_not_cancel_the_other_and_shutdown_signals_all() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    let first_entered = Arc::new(AtomicBool::new(false));
    let second_entered = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let (mut pool, first_owner) = configured(
        &root,
        Box::new(EchoNamedBlockingBackend(BlockingBackend {
            entered: first_entered.clone(),
            calls: calls.clone(),
        })),
    );
    let first_project = pool.active_context().project_id.clone();
    pool.open(Some(&child)).unwrap();
    let second_project = pool.active_context().project_id.clone();
    let second_owner = pool.owner(&second_project).unwrap();
    {
        let mut agent = second_owner.lock().unwrap();
        let owner_id = agent.owner_label().into();
        let outcome = agent
            .select_connection(zenpi::core::ConnectionSelection {
                owner_id,
                profile: "controlled-parallel".into(),
                model: None,
                expected_selection_revision: None,
                backend: Box::new(BlockingBackend {
                    entered: second_entered.clone(),
                    calls: calls.clone(),
                }),
            })
            .unwrap();
        assert!(matches!(
            outcome,
            zenpi::core::ConnectionSelectionOutcome::Applied { .. }
        ));
    }
    let mut first = request(
        &pool,
        "first-parallel",
        Operation::Prompt,
        json!({"text":"first"}),
    );
    first.project_id = Some(first_project.clone());
    first.session_id = Some(first_owner.lock().unwrap().session().session_id().into());
    let second = request(
        &pool,
        "second-parallel",
        Operation::Prompt,
        json!({"text":"second"}),
    );
    let mut cancel = first.clone();
    cancel.id = Some("cancel-first-parallel".into());
    cancel.payload = Message::Request {
        operation: Operation::Cancel,
        arguments: json!({"request_id":"first-parallel"}),
    };
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let device = phone_projects(&registry, &[&first_project, &second_project], true);
    let (adapter, host) = HeadlessHost::start(pool);
    assert_eq!(
        adapter.dispatch(&registry, &device, first).execution_state,
        Some(ExecutionState::Accepted)
    );
    assert_eq!(
        adapter.dispatch(&registry, &device, second).execution_state,
        Some(ExecutionState::Accepted)
    );
    let until = Instant::now() + Duration::from_secs(5);
    while !first_entered.load(Ordering::Acquire) || !second_entered.load(Ordering::Acquire) {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(first_owner.try_lock().is_err());
    assert!(second_owner.try_lock().is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        adapter.dispatch(&registry, &device, cancel).execution_state,
        Some(ExecutionState::Accepted)
    );
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let value = listing(&adapter, &registry, &device);
        let projects = value["projects"].as_array().unwrap();
        let first_status =
            &projects.iter().find(|p| p["id"] == first_project).unwrap()["prompt_statuses"][0];
        let second_status =
            &projects.iter().find(|p| p["id"] == second_project).unwrap()["prompt_statuses"][0];
        if first_status["state"] == "cancelled" {
            assert_eq!(second_status["state"], "running");
            break;
        }
        assert!(
            Instant::now() < until,
            "first cancellation unavailable: {value}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(second_owner.try_lock().is_err());
    let started = Instant::now();
    host.shutdown();
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(first_owner.try_lock().is_ok());
    assert!(second_owner.try_lock().is_ok());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn independent_project_runs_while_another_owner_is_blocked_and_controls_remain_scoped() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    let entered = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let (mut pool, first_owner) = configured(
        &root,
        Box::new(EchoNamedBlockingBackend(BlockingBackend {
            entered: entered.clone(),
            calls: calls.clone(),
        })),
    );
    let first_project = pool.active_context().project_id.clone();
    pool.open(Some(&child)).unwrap();
    let second_project = pool.active_context().project_id.clone();
    let second_owner = pool.owner(&second_project).unwrap();
    assert!(!Arc::ptr_eq(&first_owner, &second_owner));
    let second_prompt = request(
        &pool,
        "independent-prompt",
        Operation::Prompt,
        json!({"text":"SECOND_PROJECT_DONE"}),
    );
    let second_close = request(&pool, "close-second", Operation::WorkspaceClose, json!({}));
    let mut first_prompt = request(
        &pool,
        "blocked-first",
        Operation::Prompt,
        json!({"text":"hold first"}),
    );
    first_prompt.project_id = Some(first_project.clone());
    first_prompt.session_id = Some(first_owner.lock().unwrap().session().session_id().into());
    let mut first_cancel = first_prompt.clone();
    first_cancel.id = Some("cancel-first".into());
    first_cancel.payload = Message::Request {
        operation: Operation::Cancel,
        arguments: json!({"request_id":"blocked-first"}),
    };
    let mut wrong_cancel = first_cancel.clone();
    wrong_cancel.id = Some("wrong-project-cancel".into());
    wrong_cancel.project_id = Some(second_project.clone());
    wrong_cancel.session_id = second_prompt.session_id.clone();
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let device = phone_projects(&registry, &[&first_project, &second_project], true);
    let (adapter, host) = HeadlessHost::start(pool);
    let first = adapter.dispatch(&registry, &device, first_prompt.clone());
    assert_eq!(first.execution_state, Some(ExecutionState::Accepted));
    let until = Instant::now() + Duration::from_secs(5);
    while !entered.load(Ordering::Acquire) {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(first_owner.try_lock().is_err());
    let second = adapter.dispatch(&registry, &device, second_prompt.clone());
    assert_eq!(second.execution_state, Some(ExecutionState::Accepted));
    let job_id = |response: &Envelope| {
        let Message::Response { result } = &response.payload else {
            panic!("receipt")
        };
        result["job_id"].clone()
    };
    assert_ne!(job_id(&first), job_id(&second));
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let value = listing(&adapter, &registry, &device);
        if value["projects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == second_project)
            .unwrap()["prompt_statuses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["request_id"] == "independent-prompt" && s["state"] == "succeeded")
        {
            break;
        }
        assert!(
            Instant::now() < until,
            "independent project did not complete: {value}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(first_owner.try_lock().is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(
        second_owner
            .lock()
            .unwrap()
            .history()
            .iter()
            .any(|turn| turn.content.contains("SECOND_PROJECT_DONE"))
    );
    assert_ne!(
        adapter
            .dispatch(&registry, &device, wrong_cancel)
            .execution_state,
        Some(ExecutionState::Accepted)
    );
    assert!(first_owner.try_lock().is_err());
    let closed = adapter.dispatch(&registry, &device, second_close);
    assert_eq!(closed.execution_state, Some(ExecutionState::Succeeded));
    let Message::Response { result } = closed.payload else {
        panic!("close receipt")
    };
    first_cancel.expected_revision = result["workspace_revision"].as_u64();
    assert_eq!(
        adapter
            .dispatch(&registry, &device, first_cancel)
            .execution_state,
        Some(ExecutionState::Accepted)
    );
    await_state(&adapter, &registry, &device, "blocked-first", "cancelled");
    assert_eq!(
        adapter
            .dispatch(&registry, &device, first_prompt)
            .execution_state,
        Some(ExecutionState::Accepted)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    host.shutdown();
    assert!(first_owner.try_lock().is_ok());
    assert!(second_owner.try_lock().is_ok());
}
impl Backend for BlockingBackend {
    crate::support::controlled_local_backend!();
    fn complete(&self, _: CompletionRequest<'_>) -> Result<Completion, BackendError> {
        panic!("controlled boundary required")
    }
    fn complete_with_control(
        &self,
        _: CompletionRequest<'_>,
        cancelled: &dyn Fn() -> bool,
        on_event: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
    ) -> Result<Completion, BackendError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.store(true, Ordering::Release);
        on_event(ProviderEvent::TextDelta {
            delta: "STAGE2_PARTIAL_BEFORE_COMPLETION\n".into(),
        })?;
        let until = Instant::now() + Duration::from_secs(12);
        while !cancelled() {
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(5));
        }
        Err(BackendError::Cancelled)
    }
}

#[test]
fn headless_cancel_is_device_scoped_and_shutdown_joins_the_original_owner() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let entered = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let (pool, owner) = configured(
        &root,
        Box::new(BlockingBackend {
            entered: entered.clone(),
            calls: calls.clone(),
        }),
    );
    let project = pool.active_context().project_id.clone();
    let prompt = request(
        &pool,
        "blocked-prompt",
        Operation::Prompt,
        json!({"text":"wait"}),
    );
    let second = request(
        &pool,
        "second-prompt",
        Operation::Prompt,
        json!({"text":"another"}),
    );
    let cancel = request(
        &pool,
        "cancel",
        Operation::Cancel,
        json!({"request_id":"blocked-prompt"}),
    );
    let close = request(&pool, "close", Operation::WorkspaceClose, json!({}));
    let model = request(
        &pool,
        "model-while-admitted",
        Operation::ModelSelect,
        json!({"model":"gpt-5.2","selection_revision":0}),
    );
    let reasoning = request(
        &pool,
        "reasoning-while-admitted",
        Operation::ReasoningSelect,
        json!({"reasoning_effort":null,"selection_revision":0}),
    );
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let device = phone(&registry, &project, true);
    let other = phone(&registry, &project, true);
    let reader = phone(&registry, &project, false);
    let (adapter, host) = HeadlessHost::start(pool);
    assert_eq!(
        adapter
            .dispatch(&registry, &device, prompt.clone())
            .execution_state,
        Some(ExecutionState::Accepted)
    );
    await_state(&adapter, &registry, &device, "blocked-prompt", "running");
    assert_eq!(
        adapter.dispatch(&registry, &device, model).code,
        Some(ErrorCode::Busy)
    );
    assert_eq!(
        adapter.dispatch(&registry, &device, reasoning).code,
        Some(ErrorCode::Busy)
    );
    let until = Instant::now() + Duration::from_secs(5);
    while !entered.load(Ordering::Acquire) {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let listed = listing(&adapter, &registry, &device);
        if listed["projects"][0]["prompt_statuses"][0]["stream_text"]
            == "STAGE2_PARTIAL_BEFORE_COMPLETION\n"
        {
            break;
        }
        assert!(
            Instant::now() < until,
            "provider partial text unavailable while owner is locked"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        listing(&adapter, &registry, &other)["projects"][0]["prompt_statuses"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for (index, text) in [
        "/skill:research",
        " /review literal",
        "use @file.txt",
        "@\"unfinished",
    ]
    .into_iter()
    .enumerate()
    {
        let mut invalid = prompt.clone();
        invalid.id = Some(format!("invalid-steer-{index}"));
        invalid.payload = Message::Request {
            operation: Operation::Steer,
            arguments: json!({"request_id":"blocked-prompt","text":text}),
        };
        assert_eq!(
            adapter.dispatch(&registry, &device, invalid).code,
            Some(ErrorCode::InvalidRequest)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    assert_eq!(
        adapter.dispatch(&registry, &reader, close.clone()).code,
        Some(ErrorCode::Unauthorized)
    );
    assert_eq!(
        adapter.dispatch(&registry, &device, close).code,
        Some(ErrorCode::Busy)
    );
    assert_eq!(
        adapter.dispatch(&registry, &other, cancel.clone()).code,
        Some(ErrorCode::OwnerMismatch)
    );
    // Busy/unknown refusal does not admit a second backend turn.
    assert_ne!(
        adapter.dispatch(&registry, &device, second).execution_state,
        Some(ExecutionState::Accepted)
    );
    assert_eq!(
        adapter
            .dispatch(&registry, &device, cancel.clone())
            .execution_state,
        Some(ExecutionState::Accepted)
    );
    await_state(&adapter, &registry, &device, "blocked-prompt", "cancelled");
    assert!(
        listing(&adapter, &registry, &device)["projects"][0]["prompt_statuses"][0]
            .get("stream_text")
            .is_none()
    );
    assert_eq!(
        adapter.dispatch(&registry, &device, cancel).execution_state,
        Some(ExecutionState::Accepted)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    host.shutdown();
    assert!(owner.try_lock().is_ok());
    // A subsequent host owns the same Agent, and stop cancels its cooperative active turn.
    let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
    pool.restore_checkpoint().unwrap();
    let prompt = request(
        &pool,
        "stop-prompt",
        Operation::Prompt,
        json!({"text":"stop me"}),
    );
    let (adapter, host) = HeadlessHost::start(pool);
    assert_eq!(
        adapter.dispatch(&registry, &device, prompt).execution_state,
        Some(ExecutionState::Accepted)
    );
    await_state(&adapter, &registry, &device, "stop-prompt", "running");
    // Running progress precedes workspace preparation and backend entry.
    let entered = Instant::now() + Duration::from_secs(3);
    while calls.load(Ordering::SeqCst) < 2 {
        assert!(
            Instant::now() < entered,
            "second provider invocation never entered"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let start = Instant::now();
    host.shutdown();
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(owner.try_lock().is_ok());
    let mut restored = ProjectOwnerPool::new(owner.clone()).unwrap();
    restored.restore_checkpoint().unwrap();
    let (adapter, host) = HeadlessHost::start(restored);
    await_state(
        &adapter,
        &registry,
        &device,
        "stop-prompt",
        "unknown_outcome",
    );
    // Shutdown cancellation is intentionally conservative: runtime Cancelled
    // also covers detachment and is not proof that side effects stopped.
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    host.shutdown();
}

#[test]
#[ignore = "requires freshly compiled Kotlin classes; run by Android source checker"]
fn android_generated_prompt_and_cancel_drive_real_owner_once() {
    use base64::engine::general_purpose::STANDARD;
    use zenpi::remote::frame::{read_frame, write_frame};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let entered = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(AtomicBool::new(false));
    let release_second = Arc::new(AtomicBool::new(false));
    std::fs::write(root.join("android-effect.txt"), "BEFORE_ANDROID_TOOL\n").unwrap();
    let contexts = Arc::new(Mutex::new(Vec::<Vec<zenpi::core::Turn>>::new()));
    struct SteeringBackend {
        entered: Arc<AtomicBool>,
        calls: Arc<AtomicUsize>,
        release: Arc<AtomicBool>,
        release_second: Arc<AtomicBool>,
        contexts: Arc<Mutex<Vec<Vec<zenpi::core::Turn>>>>,
    }
    impl Backend for SteeringBackend {
        crate::support::controlled_local_backend!();
        fn complete(&self, _: CompletionRequest<'_>) -> Result<Completion, BackendError> {
            panic!("controlled boundary required")
        }
        fn complete_with_control(
            &self,
            request: CompletionRequest<'_>,
            cancelled: &dyn Fn() -> bool,
            on_event: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
        ) -> Result<Completion, BackendError> {
            self.contexts.lock().unwrap().push(request.turns.to_vec());
            let index = self.calls.fetch_add(1, Ordering::SeqCst);
            self.entered.store(true, Ordering::Release);
            on_event(ProviderEvent::TextDelta {
                delta: "STAGE2_PARTIAL_BEFORE_COMPLETION\n".into(),
            })?;
            let until = Instant::now() + Duration::from_secs(12);
            loop {
                if cancelled() {
                    return Err(BackendError::Cancelled);
                }
                if index == 0 && self.release.load(Ordering::Acquire) {
                    let mut completion = Completion::text("FIRST_PROVIDER_BOUNDARY");
                    completion.tool_calls.push(zenpi::tools::ToolCall {
                        id: "android-owner-write".into(), name: "write_file".into(),
                        arguments: json!({"path":"android-effect.txt","content":"ANDROID_TOOL_EFFECT_ONCE\n"}),
                    });
                    completion.tool_calls.push(zenpi::tools::ToolCall {
                        id: "android-owner-denied-write".into(), name: "write_file".into(),
                        arguments: json!({"path":"android-denied.txt","content":"DENIED_TOOL_MUST_NOT_WRITE\n"}),
                    });
                    return Ok(completion);
                }
                if index == 1 && self.release_second.load(Ordering::Acquire) {
                    let mut completion = Completion::text("SECOND_PROVIDER_BOUNDARY");
                    completion.tool_calls.push(zenpi::tools::ToolCall {
                        id: "android-owner-cancelled-write".into(), name: "write_file".into(),
                        arguments: json!({"path":"android-cancelled.txt","content":"CANCELLED_TOOL_MUST_NOT_WRITE\n"}),
                    });
                    return Ok(completion);
                }
                assert!(Instant::now() < until);
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
    let (pool, owner) = configured(
        &root,
        Box::new(SteeringBackend {
            entered: entered.clone(),
            calls: calls.clone(),
            release: release.clone(),
            release_second: release_second.clone(),
            contexts: contexts.clone(),
        }),
    );
    let project = pool.active_context().project_id.clone();
    owner.lock().unwrap().set_approval_policy(ApprovalPolicy {
        mode: ApprovalMode::Always,
        ..Default::default()
    });
    let inspect = request(&pool, "inspect", Operation::Inspect, json!({}));
    let subtabs = request(&pool, "composer-subtabs", Operation::SubtabList, json!({}));
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let device = phone(&registry, &project, true);
    let other = phone(&registry, &project, true);
    let reader = phone(&registry, &project, false);
    let approver = phone_capabilities(
        &registry,
        &[&project],
        vec![Capability::Read, Capability::Approval],
    );
    let (adapter, host) = HeadlessHost::start(pool);
    let response = adapter.dispatch(&registry, &device, inspect);
    assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
    let Message::Response { result: snapshot } = response.payload else {
        panic!("snapshot")
    };
    let tab_response = adapter.dispatch(&registry, &device, subtabs);
    assert_eq!(
        tab_response.execution_state,
        Some(ExecutionState::Succeeded)
    );
    let Message::Response { result: tabs } = tab_response.payload else {
        panic!("subtabs")
    };
    let mut seed =
        json!({"listing":listing(&adapter, &registry, &device),"snapshot":snapshot,"subtabs":tabs});
    let seed_path = root.join("seed.json");
    let command_path = root.join("command.bin");
    let results_path = root.join("results.json");
    let run_java = |phase: &str, seed: &Value| {
        std::fs::write(&seed_path, serde_json::to_vec(seed).unwrap()).unwrap();
        let mut child =
            std::process::Command::new(std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap())
                .args([
                    format!("-Dzenpi.prompt.phase={phase}"),
                    format!("-Dzenpi.prompt.seed={}", seed_path.display()),
                    format!("-Dzenpi.prompt.command={}", command_path.display()),
                    format!("-Dzenpi.prompt.results={}", results_path.display()),
                    "-cp".into(),
                    std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap(),
                    "org.junit.runner.JUnitCore".into(),
                    "zenpi.AndroidRustPromptTest".into(),
                ])
                .spawn()
                .unwrap();
        let until = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "Kotlin {phase}");
                break;
            }
            if Instant::now() >= until {
                let _ = child.kill();
                let _ = child.wait();
                panic!("Kotlin deadline");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    };
    let read_command = || {
        let bytes = std::fs::read(&command_path).unwrap();
        let mut input = bytes.as_slice();
        let command = read_frame(&mut input).unwrap().unwrap();
        assert!(input.is_empty());
        command
    };
    let mut exchanges = Vec::new();
    let mut retain = |request: &Envelope, response: &Envelope| {
        let mut wire = Vec::new();
        write_frame(&mut wire, response).unwrap();
        let encoded = STANDARD.encode(wire);
        exchanges.push(json!({"request":request,"code":response.code,"execution_state":response.execution_state,"wire_base64":encoded}));
        encoded
    };
    run_java("prompt", &seed);
    let prompt = read_command();
    assert!(
        matches!(&prompt.payload, Message::Request { operation: Operation::Prompt, arguments } if arguments == &json!({"text":"ANDROID_OWNER_PROMPT"}))
    );
    let accepted = adapter.dispatch(&registry, &device, prompt.clone());
    assert_eq!(accepted.execution_state, Some(ExecutionState::Accepted));
    let first_prompt = retain(&prompt, &accepted);
    let retry = adapter.dispatch(&registry, &device, prompt.clone());
    assert!(matches!(
        retry.execution_state,
        Some(ExecutionState::Accepted | ExecutionState::UnknownOutcome)
    ));
    retain(&prompt, &retry);
    let target = prompt.id.as_deref().unwrap();
    await_state(&adapter, &registry, &device, target, "running");
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let listed = listing(&adapter, &registry, &device);
        if listed["projects"][0]["prompt_statuses"][0]["stream_text"]
            == "STAGE2_PARTIAL_BEFORE_COMPLETION\n"
        {
            seed["listing"] = listed;
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(entered.load(Ordering::Acquire));
    seed["target"] = json!(target);
    run_java("steer", &seed);
    let steer = read_command();
    assert!(
        matches!(&steer.payload, Message::Request { operation: Operation::Steer, arguments }
        if arguments == &json!({"request_id":target,"text":"ANDROID_OWNER_STEERING"}))
    );
    for (identity, code) in [
        (&reader, ErrorCode::Unauthorized),
        (&other, ErrorCode::OwnerMismatch),
    ] {
        let response = adapter.dispatch(&registry, identity, steer.clone());
        assert_eq!(response.code, Some(code));
        retain(&steer, &response);
    }
    let mut stale = steer.clone();
    stale.expected_revision = Some(99);
    let response = adapter.dispatch(&registry, &device, stale.clone());
    assert_eq!(response.code, Some(ErrorCode::RevisionConflict));
    retain(&stale, &response);
    let mut invalid = steer.clone();
    invalid.id = Some("invalid-steer-command".into());
    if let Message::Request { arguments, .. } = &mut invalid.payload {
        arguments["text"] = json!("/skill:research");
    }
    let response = adapter.dispatch(&registry, &device, invalid.clone());
    assert_eq!(response.code, Some(ErrorCode::InvalidRequest));
    retain(&invalid, &response);
    let response = adapter.dispatch(&registry, &device, steer.clone());
    assert!(matches!(
        response.execution_state,
        Some(ExecutionState::Accepted | ExecutionState::UnknownOutcome)
    ));
    retain(&steer, &response);
    let until = Instant::now() + Duration::from_secs(3);
    let steer_accepted = loop {
        let response = adapter.dispatch(&registry, &device, steer.clone());
        if response.execution_state == Some(ExecutionState::Accepted) {
            break response;
        }
        assert_eq!(
            response.execution_state,
            Some(ExecutionState::UnknownOutcome)
        );
        assert!(Instant::now() < until, "steering did not become durable");
        std::thread::sleep(Duration::from_millis(10));
    };
    let first_steer = retain(&steer, &steer_accepted);
    let retry = adapter.dispatch(&registry, &device, steer.clone());
    assert_eq!(retry, steer_accepted);
    let second_steer = retain(&steer, &retry);
    let mut conflict = steer.clone();
    if let Message::Request { arguments, .. } = &mut conflict.payload {
        arguments["text"] = json!("different steering");
    }
    let response = adapter.dispatch(&registry, &device, conflict.clone());
    assert_eq!(response.code, Some(ErrorCode::RequestIdConflict));
    retain(&conflict, &response);
    release.store(true, Ordering::Release);
    let mut approval_requests = Vec::new();
    let mut approval_responses = Vec::new();
    let mut approval_receipts = Vec::new();
    for (decision, path) in [
        ("allow", "android-effect.txt"),
        ("deny", "android-denied.txt"),
    ] {
        let until = Instant::now() + Duration::from_secs(3);
        loop {
            let listed = listing(&adapter, &registry, &approver);
            let pending = listed["projects"][0]["pending_approvals"]
                .as_array()
                .unwrap();
            if pending.len() == 1 && pending[0]["preview"]["path"] == path {
                seed["listing"] = listed;
                break;
            }
            assert!(
                Instant::now() < until,
                "missing actual tool approval {path}: {listed}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            std::fs::read_to_string(root.join("android-effect.txt")).unwrap(),
            if decision == "allow" {
                "BEFORE_ANDROID_TOOL\n"
            } else {
                "ANDROID_TOOL_EFFECT_ONCE\n"
            }
        );
        assert!(!root.join("android-denied.txt").exists());
        seed["approval_decision"] = json!(decision);
        seed["approval_path"] = json!(path);
        run_java("approve", &seed);
        let command = read_command();
        let refused = adapter.dispatch(&registry, &device, command.clone());
        assert_eq!(refused.code, Some(ErrorCode::Unauthorized));
        retain(&command, &refused);
        let mut bad_digest = command.clone();
        bad_digest.id = Some(format!("bad-{decision}-digest"));
        if let Message::Request { arguments, .. } = &mut bad_digest.payload {
            arguments["digest"] = json!("0".repeat(64));
        }
        let refused = adapter.dispatch(&registry, &approver, bad_digest.clone());
        assert_eq!(refused.code, Some(ErrorCode::InvalidRequest));
        retain(&bad_digest, &refused);
        let accepted = adapter.dispatch(&registry, &approver, command.clone());
        assert_eq!(accepted.execution_state, Some(ExecutionState::Accepted));
        let first = retain(&command, &accepted);
        let retry = adapter.dispatch(&registry, &approver, command.clone());
        assert_eq!(retry, accepted);
        let second = retain(&command, &retry);
        approval_requests.push(command);
        approval_responses.push(accepted);
        approval_receipts.push(vec![first, second]);
    }
    let until = Instant::now() + Duration::from_secs(3);
    while calls.load(Ordering::SeqCst) < 2 {
        assert!(
            Instant::now() < until,
            "steering never reached provider boundary"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        std::fs::read_to_string(root.join("android-effect.txt")).unwrap(),
        "ANDROID_TOOL_EFFECT_ONCE\n"
    );
    let captured = contexts.lock().unwrap();
    assert_eq!(captured.len(), 2);
    assert!(
        !captured[0]
            .iter()
            .any(|turn| turn.content == "ANDROID_OWNER_STEERING")
    );
    assert_eq!(
        captured[1]
            .iter()
            .filter(|turn| turn.content == "ANDROID_OWNER_STEERING")
            .count(),
        1
    );
    drop(captured);
    release_second.store(true, Ordering::Release);
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        let listed = listing(&adapter, &registry, &approver);
        let pending = listed["projects"][0]["pending_approvals"]
            .as_array()
            .unwrap();
        if pending.len() == 1 && pending[0]["preview"]["path"] == "android-cancelled.txt" {
            seed["listing"] = listed;
            break;
        }
        assert!(Instant::now() < until, "missing cancelled-tool approval");
        std::thread::sleep(Duration::from_millis(10));
    }
    seed["approval_decision"] = json!("allow");
    seed["approval_path"] = json!("android-cancelled.txt");
    run_java("approve", &seed);
    let late_approval = read_command();
    assert!(!root.join("android-cancelled.txt").exists());
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        let listed = listing(&adapter, &registry, &device);
        if listed["projects"][0]["prompt_statuses"][0]["stream_text"]
            == "STAGE2_PARTIAL_BEFORE_COMPLETION\n".repeat(2)
        {
            seed["listing"] = listed;
            break;
        }
        assert!(
            Instant::now() < until,
            "second provider preview unavailable"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    run_java("cancel", &seed);
    let cancel = read_command();
    assert!(
        matches!(&cancel.payload, Message::Request { operation: Operation::Cancel, arguments } if arguments == &json!({"request_id":target}))
    );
    for (identity, code) in [
        (&reader, ErrorCode::Unauthorized),
        (&other, ErrorCode::OwnerMismatch),
    ] {
        let response = adapter.dispatch(&registry, identity, cancel.clone());
        assert_eq!(response.code, Some(code));
        retain(&cancel, &response);
    }
    let mut foreign = cancel.clone();
    foreign.session_id = Some("foreign".into());
    let response = adapter.dispatch(&registry, &device, foreign.clone());
    assert_eq!(response.code, Some(ErrorCode::OwnerMismatch));
    retain(&foreign, &response);
    let response = adapter.dispatch(&registry, &device, cancel.clone());
    assert_eq!(response.execution_state, Some(ExecutionState::Accepted));
    let first_cancel = retain(&cancel, &response);
    await_state(&adapter, &registry, &device, target, "cancelled");
    assert!(
        listing(&adapter, &registry, &approver)["projects"][0]["pending_approvals"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let late_response = adapter.dispatch(&registry, &approver, late_approval.clone());
    assert_eq!(late_response.code, Some(ErrorCode::InvalidRequest));
    retain(&late_approval, &late_response);
    assert!(!root.join("android-cancelled.txt").exists());
    let retry = adapter.dispatch(&registry, &device, prompt.clone());
    assert_eq!(retry, accepted);
    let second_prompt = retain(&prompt, &retry);
    let retry = adapter.dispatch(&registry, &device, cancel.clone());
    assert_eq!(retry, response);
    let second_cancel = retain(&cancel, &retry);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    host.shutdown();
    let mut agent = owner.lock().unwrap();
    let queue = zenpi::input_queue::InputQueue::recover(agent.session(), Default::default())
        .unwrap()
        .execute(
            agent.session_mut(),
            zenpi::protocol::InputQueueAction::List {
                after_sequence: None,
                limit: 32,
            },
        )
        .unwrap();
    let zenpi::input_queue::InputQueueReply::Page { inputs, .. } = queue else {
        panic!("steering queue")
    };
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].text, "ANDROID_OWNER_STEERING");
    assert_eq!(inputs[0].kind, zenpi::input_queue::InputKind::Steer);
    assert_eq!(inputs[0].status, zenpi::input_queue::InputStatus::Applied);
    assert!(inputs[0].applied_turn_id.is_some());
    let tool_turns = agent
        .session()
        .turns()
        .iter()
        .filter(|turn| {
            turn.role == zenpi::core::TurnRole::Tool
                && turn
                    .metadata
                    .as_ref()
                    .is_some_and(|metadata| metadata["tool_call_id"] == "android-owner-write")
        })
        .collect::<Vec<_>>();
    assert_eq!(tool_turns.len(), 1);
    let tool_turn_count = tool_turns.len();
    let tool_record: Value = serde_json::from_str(&tool_turns[0].content).unwrap();
    assert_eq!(tool_record["call_id"], "android-owner-write");
    assert_eq!(tool_record["tool"], "write_file");
    let tool_result = tool_record["output"].clone();
    assert_eq!(tool_result["path"], "android-effect.txt");
    assert_eq!(tool_result["bytes"], "ANDROID_TOOL_EFFECT_ONCE\n".len());
    let denied_turns = agent
        .session()
        .turns()
        .iter()
        .filter(|turn| {
            turn.role == zenpi::core::TurnRole::Tool
                && turn.metadata.as_ref().is_some_and(|metadata| {
                    metadata["tool_call_id"] == "android-owner-denied-write"
                })
        })
        .collect::<Vec<_>>();
    assert_eq!(denied_turns.len(), 1);
    let denied_result: Value = serde_json::from_str(&denied_turns[0].content).unwrap();
    assert!(denied_result.get("error").is_some());
    assert!(!root.join("android-denied.txt").exists());
    let audit = agent
        .session()
        .events()
        .iter()
        .filter(|event| {
            event["type"] == "approval_resolved"
                && matches!(
                    event["call_id"].as_str(),
                    Some("android-owner-write" | "android-owner-denied-write")
                )
        })
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(audit.len(), 2);
    for (index, event) in audit.iter().enumerate() {
        let command = &approval_requests[index];
        let Message::Request { arguments, .. } = &command.payload else {
            panic!("approval request")
        };
        assert_eq!(event["decision"], if index == 0 { "allow" } else { "deny" });
        assert_eq!(event["remember"], false);
        assert_eq!(event["source"], "host_answer");
        assert_eq!(event["remote"]["device_id"], approver.id());
        assert_eq!(
            event["remote"]["request_id"],
            command.id.as_deref().unwrap()
        );
        assert_eq!(event["remote"]["project_id"], project);
        assert_eq!(
            event["remote"]["session_id"],
            command.session_id.as_deref().unwrap()
        );
        assert_eq!(event["remote"]["request_digest"], arguments["digest"]);
    }
    let consumed = agent
        .session()
        .events()
        .iter()
        .filter(|event| {
            event["type"] == "approval_consumed"
                && matches!(
                    event["call_id"].as_str(),
                    Some("android-owner-write" | "android-owner-denied-write")
                )
        })
        .collect::<Vec<_>>();
    assert_eq!(consumed.len(), 1);
    assert_eq!(consumed[0]["call_id"], "android-owner-write");
    assert!(
        agent
            .session()
            .events()
            .iter()
            .all(|event| !(event["type"] == "approval_consumed"
                && event["call_id"] == "android-owner-cancelled-write"))
    );
    let effect_before_recovery = std::fs::read(root.join("android-effect.txt")).unwrap();
    drop(agent);
    let journal = std::fs::read(root.join("session.jsonl")).unwrap();
    struct RecoveryBackend(Arc<AtomicUsize>);
    impl Backend for RecoveryBackend {
        crate::support::controlled_local_backend!();
        fn complete(&self, _: CompletionRequest<'_>) -> Result<Completion, BackendError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            panic!("receipt recovery must not invoke a backend")
        }
    }
    let recovery_calls = Arc::new(AtomicUsize::new(0));
    let (restored, fresh_owner) =
        configured(&root, Box::new(RecoveryBackend(recovery_calls.clone())));
    assert!(!Arc::ptr_eq(&owner, &fresh_owner));
    assert_eq!(restored.active_context().project_id, project);
    assert_eq!(
        restored.active_context().session_id,
        prompt.session_id.as_deref().unwrap()
    );
    let (adapter, host) = HeadlessHost::start(restored);
    let retry = adapter.dispatch(&registry, &device, prompt.clone());
    assert_eq!(retry, accepted);
    let third_prompt = retain(&prompt, &retry);
    let retry = adapter.dispatch(&registry, &device, cancel.clone());
    assert_eq!(retry, response);
    let third_cancel = retain(&cancel, &retry);
    let retry = adapter.dispatch(&registry, &device, steer.clone());
    assert_eq!(retry, steer_accepted);
    let third_steer = retain(&steer, &retry);
    for (index, command) in approval_requests.iter().enumerate() {
        let retry = adapter.dispatch(&registry, &approver, command.clone());
        assert_eq!(retry, approval_responses[index]);
        approval_receipts[index].push(retain(command, &retry));
    }
    let final_listing = listing(&adapter, &registry, &device);
    let late_retry = adapter.dispatch(&registry, &approver, late_approval.clone());
    assert_eq!(late_retry.code, Some(ErrorCode::OwnerMismatch));
    retain(&late_approval, &late_retry);
    let mut current_late = late_approval.clone();
    current_late.id = Some("cancelled-approval-current-owner".into());
    current_late.required_owner = Some(final_listing["required_owner"].as_str().unwrap().into());
    current_late.expected_revision = final_listing["workspace_revision"].as_u64();
    let refused = adapter.dispatch(&registry, &approver, current_late.clone());
    assert_eq!(refused.code, Some(ErrorCode::InvalidRequest));
    retain(&current_late, &refused);
    assert!(!root.join("android-cancelled.txt").exists());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(recovery_calls.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read(root.join("session.jsonl")).unwrap(), journal);
    assert_eq!(
        std::fs::read(root.join("android-effect.txt")).unwrap(),
        effect_before_recovery
    );
    let mut fresh = fresh_owner.lock().unwrap();
    let recovered_queue =
        zenpi::input_queue::InputQueue::recover(fresh.session(), Default::default())
            .unwrap()
            .execute(
                fresh.session_mut(),
                zenpi::protocol::InputQueueAction::List {
                    after_sequence: None,
                    limit: 32,
                },
            )
            .unwrap();
    let zenpi::input_queue::InputQueueReply::Page {
        inputs: recovered_inputs,
        ..
    } = recovered_queue
    else {
        panic!("recovered queue")
    };
    assert_eq!(recovered_inputs, inputs);
    drop(fresh);
    registry.revoke(approver.id()).unwrap();
    for command in &approval_requests {
        let refused = adapter.dispatch(&registry, &approver, command.clone());
        assert_eq!(refused.code, Some(ErrorCode::Unauthorized));
        retain(command, &refused);
    }
    registry.revoke(device.id()).unwrap();
    for command in [&prompt, &steer, &cancel] {
        let refused = adapter.dispatch(&registry, &device, command.clone());
        assert_eq!(refused.code, Some(ErrorCode::Unauthorized));
        retain(command, &refused);
    }
    assert_eq!(std::fs::read(root.join("session.jsonl")).unwrap(), journal);
    assert_eq!(
        std::fs::read(root.join("android-effect.txt")).unwrap(),
        effect_before_recovery
    );
    assert!(!root.join("android-denied.txt").exists());
    assert_eq!(recovery_calls.load(Ordering::SeqCst), 0);
    let results = json!({"exchanges":exchanges,"listing":final_listing,"cancel":cancel,
        "prompt_receipts":[first_prompt,second_prompt,third_prompt],"cancel_receipts":[first_cancel,second_cancel,third_cancel],
        "steer_receipts":[first_steer,second_steer,third_steer],"steering_inputs":inputs.len(),"steering_text":inputs[0].text,
        "steering_status":inputs[0].status,"provider_steering_occurrences":1,"backend_calls":2,
        "fresh_agent_recovery":true,"recovery_backend_calls":0,"recovered_inputs":recovered_inputs,
        "tool_turns":tool_turn_count,"tool_result":tool_result,"tool_effect":"ANDROID_TOOL_EFFECT_ONCE\n",
        "approval_receipts":approval_receipts,"denied_result":denied_result,"denied_file_absent":true,"approval_mode":"always",
        "approval_audit":audit,"approval_consumed":1,"cancelled_file_absent":true,"late_approval":late_approval,
        "current_owner_late_approval":current_late});
    std::fs::write(&results_path, serde_json::to_vec(&results).unwrap()).unwrap();
    run_java("validate", &seed);
    host.shutdown();
    if let Some(output) = std::env::var_os("ZENPI_ANDROID_PROMPT_OUTPUT") {
        std::fs::write(output, serde_json::to_vec_pretty(&results).unwrap()).unwrap();
    }
}

#[test]
fn headless_model_selection_uses_current_owner_and_retries_exact_receipt() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let backend = OpenAiCompatibleBackend::new_with_settings(
        "http://127.0.0.1:9",
        None,
        "gpt-5.2",
        OpenAiWireApi::Responses,
        Some("high".into()),
        None,
    )
    .unwrap()
    .with_model_registry(
        "openai".into(),
        zenpi::providers::registry::ModelRegistry::default(),
    )
    .unwrap();
    let (pool, owner) = configured(&root, Box::new(backend));
    let project = pool.active_context().project_id.clone();
    let selection = request(
        &pool,
        "select-model",
        Operation::ModelSelect,
        json!({"model":"gpt-5.2-2025-12-11","selection_revision":0}),
    );
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let device = phone(&registry, &project, true);
    let (adapter, host) = HeadlessHost::start(pool);
    assert_eq!(
        listing(&adapter, &registry, &device)["projects"][0]["model_control"],
        true
    );
    let receipt = adapter.dispatch(&registry, &device, selection.clone());
    assert_eq!(receipt.execution_state, Some(ExecutionState::Succeeded));
    owner
        .lock()
        .unwrap()
        .set_model(Some("gpt-5.2".into()))
        .unwrap();
    assert_eq!(adapter.dispatch(&registry, &device, selection), receipt);
    host.shutdown();
    let agent = owner.lock().unwrap();
    assert_eq!(agent.model(), Some("gpt-5.2"));
    assert!(agent.session().turns().is_empty());
    assert_eq!(
        agent
            .session()
            .events()
            .iter()
            .filter(|event| event["type"] == "remote_model_admission_v1")
            .count(),
        2
    );
}

#[test]
fn headless_reasoning_selection_preserves_model_and_exact_retry_after_local_change() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let backend = OpenAiCompatibleBackend::new_with_settings(
        "http://127.0.0.1:9",
        None,
        "gpt-5.2",
        OpenAiWireApi::Responses,
        Some("high".into()),
        None,
    )
    .unwrap()
    .with_model_registry(
        "openai".into(),
        zenpi::providers::registry::ModelRegistry::default(),
    )
    .unwrap();
    let (pool, owner) = configured(&root, Box::new(backend));
    let project = pool.active_context().project_id.clone();
    let req = request(
        &pool,
        "reasoning-write",
        Operation::ReasoningSelect,
        json!({"reasoning_effort":"low","selection_revision":0}),
    );
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let device = phone(&registry, &project, true);
    let (adapter, host) = HeadlessHost::start(pool);
    assert_eq!(
        listing(&adapter, &registry, &device)["projects"][0]["reasoning_control"],
        true
    );
    let receipt = adapter.dispatch(&registry, &device, req.clone());
    assert_eq!(receipt.execution_state, Some(ExecutionState::Succeeded));
    assert_eq!(owner.lock().unwrap().reasoning_effort(), Some("low"));
    owner.lock().unwrap().set_reasoning_effort(None).unwrap();
    assert_eq!(adapter.dispatch(&registry, &device, req), receipt);
    host.shutdown();
    let agent = owner.lock().unwrap();
    assert_eq!(agent.reasoning_effort(), None);
    assert_eq!(agent.model(), Some("gpt-5.2"));
    assert!(agent.session().turns().is_empty());
}

#[test]
fn production_plain_host_routes_authenticated_tls_prompt_to_the_provider() {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    use zenpi::remote::{
        frame::{read_frame, write_frame},
        pairing::{HostIdentity, PairingInvite, unix_seconds},
    };
    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let workspace = root.join("workspace");
    let config = root.join("config");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::create_dir(&config).unwrap();
    let session_path = root.join("session.jsonl");
    drop(SessionStore::open_in_workspace(&session_path, &workspace).unwrap());
    let provider = TcpListener::bind("127.0.0.1:0").unwrap();
    std::fs::write(config.join("config.toml"), format!("backend=\"openai\"\nmodel=\"fixture-model\"\nbase_url=\"http://{}/v1\"\nwire_api=\"responses\"\nrequires_openai_auth=false\nmax_retries=0\n", provider.local_addr().unwrap())).unwrap();
    let server = std::thread::spawn(move || {
        provider.set_nonblocking(true).unwrap();
        let until = Instant::now() + Duration::from_secs(12);
        let mut stream = loop {
            match provider.accept() {
                Ok((stream, _)) => break stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < until =>
                {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(error) => panic!("accept: {error}"),
            }
        };
        let request = read_http(&mut stream);
        assert!(
            request["input"]
                .to_string()
                .contains("PRODUCTION_PLAIN_PROMPT")
        );
        let events = [
            json!({"type":"response.created","response":{"id":"fixture","model":"fixture-model"}}),
            json!({"type":"response.output_text.delta","delta":"PRODUCTION_PLAIN_DONE"}),
            json!({"type":"response.completed","response":{"id":"fixture","status":"completed","model":"fixture-model"}}),
        ];
        let body: String = events
            .iter()
            .map(|event| format!("data: {event}\n\n"))
            .collect();
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    });
    let identity_path = root.join("identity.json");
    let identity = HostIdentity::load_or_create(&identity_path).unwrap();
    let registry = DeviceRegistry::new(
        identity_path.with_extension("devices.json"),
        identity.fingerprint(),
    )
    .unwrap();
    let random = ring::rand::SystemRandom::new();
    let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random).unwrap();
    let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    let signature = URL_SAFE_NO_PAD.encode(
        key.sign(
            &random,
            &pairing_proof_message(&identity.fingerprint(), "token", "pair", "phone", &public),
        )
        .unwrap()
        .as_ref(),
    );
    let device = registry
        .verify_pair_and_register("token", "pair", "phone", &public, &signature, || true)
        .unwrap();
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"zenpi-project-workspace-v1\0");
    hash.update(workspace.to_str().unwrap().as_bytes());
    let project = format!("{:x}", hash.finalize());
    let grant = Command::new(env!("CARGO_BIN_EXE_zenpi"))
        .args(["host", "pair", "--identity"])
        .arg(&identity_path)
        .args(["--grant-device-control", &device.id, "--project", &project])
        .output()
        .unwrap();
    assert!(
        grant.status.success(),
        "{}",
        String::from_utf8_lossy(&grant.stderr)
    );
    let mut command = Command::new(env!("CARGO_BIN_EXE_zenpi"));
    for (name, _) in std::env::vars()
        .filter(|(name, _)| name.starts_with("ZENPI_") || name.starts_with("OPENAI_"))
    {
        command.env_remove(name);
    }
    let child = command
        .env("ZENPI_HOME", &config)
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env("no_proxy", "127.0.0.1,localhost")
        .args([
            "host",
            "pair",
            "--plain",
            "--bind",
            "127.0.0.1:0",
            "--duration-seconds",
            "8",
            "--identity",
        ])
        .arg(&identity_path)
        .arg("--workspace")
        .arg(&workspace)
        .arg("--session")
        .arg(&session_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut child = ChildGuard(child);
    let mut stdout = BufReader::new(child.0.stdout.take().unwrap());
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let invite = PairingInvite::from_uri(line.trim(), unix_seconds().unwrap()).unwrap();
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(rustls::pki_types::CertificateDer::from(
            identity.certificate().to_vec(),
        ))
        .unwrap();
    let mut tls = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    tls.alpn_protocols = vec![b"zenpi/1".to_vec()];
    let socket = TcpStream::connect(invite.address).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut stream = rustls::StreamOwned::new(
        rustls::ClientConnection::new(Arc::new(tls), "zenpi.local".try_into().unwrap()).unwrap(),
        socket,
    );
    write_frame(
        &mut stream,
        &Envelope::new(
            Channel::Control,
            Message::Hello {
                supported_versions: vec![1],
            },
        ),
    )
    .unwrap();
    assert!(matches!(
        read_frame(&mut stream).unwrap().unwrap().payload,
        Message::Welcome {
            selected_version: 1
        }
    ));
    let Message::Challenge {
        nonce,
        host_fingerprint,
    } = read_frame(&mut stream).unwrap().unwrap().payload
    else {
        panic!("challenge")
    };
    let mut auth = Envelope::new(
        Channel::Control,
        Message::Authenticate {
            device_id: device.id.clone(),
            signature: URL_SAFE_NO_PAD.encode(
                key.sign(
                    &random,
                    &serde_json::to_vec(&[
                        "zenpi-device-auth-v1",
                        &host_fingerprint,
                        &nonce,
                        &device.id,
                    ])
                    .unwrap(),
                )
                .unwrap()
                .as_ref(),
            ),
        },
    );
    auth.id = Some("authenticate".into());
    write_frame(&mut stream, &auth).unwrap();
    assert_eq!(
        read_frame(&mut stream).unwrap().unwrap().execution_state,
        Some(ExecutionState::Succeeded)
    );
    let mut list = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::WorkspaceList,
            arguments: json!({}),
        },
    );
    list.id = Some("list".into());
    write_frame(&mut stream, &list).unwrap();
    let Message::Response { result } = read_frame(&mut stream).unwrap().unwrap().payload else {
        panic!("list")
    };
    assert_eq!(result["projects"][0]["prompt_control"], true);
    let mut prompt = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::Prompt,
            arguments: json!({"text":"PRODUCTION_PLAIN_PROMPT"}),
        },
    );
    prompt.id = Some("production-prompt".into());
    prompt.project_id = Some(project);
    prompt.session_id = Some(result["projects"][0]["session_id"].as_str().unwrap().into());
    prompt.required_owner = Some(result["required_owner"].as_str().unwrap().into());
    prompt.expected_revision = result["workspace_revision"].as_u64();
    write_frame(&mut stream, &prompt).unwrap();
    assert_eq!(
        read_frame(&mut stream).unwrap().unwrap().execution_state,
        Some(ExecutionState::Accepted)
    );
    let until = Instant::now() + Duration::from_secs(6);
    loop {
        write_frame(&mut stream, &list).unwrap();
        let Message::Response { result } = read_frame(&mut stream).unwrap().unwrap().payload else {
            panic!("list")
        };
        if result["projects"][0]["prompt_statuses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| {
                value["request_id"] == "production-prompt" && value["state"] == "succeeded"
            })
        {
            break;
        }
        assert!(Instant::now() < until, "{result}");
        std::thread::sleep(Duration::from_millis(10));
    }
    write_frame(&mut stream, &prompt).unwrap();
    assert_eq!(
        read_frame(&mut stream).unwrap().unwrap().execution_state,
        Some(ExecutionState::Accepted)
    );
    drop(stream);
    server.join().unwrap();
    assert!(child.0.wait().unwrap().success());
    assert!(TcpStream::connect(invite.address).is_err());
    let journal = std::fs::read_to_string(session_path).unwrap();
    assert!(journal.contains("PRODUCTION_PLAIN_DONE"));
    assert_eq!(
        journal
            .lines()
            .filter(|line| line.contains("remote_prompt_admission_v1")
                && line.contains("\"state\":\"accepted\""))
            .count(),
        1
    );
}

#[cfg(unix)]
#[test]
fn registry_busy_refuses_read_and_write_without_prompt_reservation_or_submission() {
    use std::os::fd::AsRawFd;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut pool, owner) = configured(
        &root,
        Box::new(BlockingBackend {
            entered: Arc::new(AtomicBool::new(false)),
            calls: Arc::new(AtomicUsize::new(0)),
        }),
    );
    let project = pool.active_context().project_id.clone();
    let registry =
        Arc::new(DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap());
    let device = phone(&registry, &project, true);
    let prompt = request(
        &pool,
        "busy-prompt",
        Operation::Prompt,
        json!({"text":"must not submit"}),
    );
    let select = request(&pool, "busy-select", Operation::WorkspaceSelect, json!({}));
    let journal = std::fs::read(root.join("session.jsonl")).unwrap();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join("devices.lock"))
        .unwrap();
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    for operation in [
        Operation::Inspect,
        Operation::SubtabList,
        Operation::WorkspaceList,
    ] {
        let mut read = request(&pool, "busy-read", operation, json!({}));
        if operation == Operation::WorkspaceList {
            read.project_id = None;
            read.session_id = None;
        }
        let response = HostAdapter::dispatch_owned(&mut pool, &registry, &device, read);
        assert_eq!(response.code, Some(ErrorCode::Busy));
        assert_eq!(response.execution_state, Some(ExecutionState::Failed));
    }
    assert_eq!(
        HostAdapter::dispatch_owned(&mut pool, &registry, &device, select.clone()).code,
        Some(ErrorCode::Busy)
    );
    let denied = zenpi::remote::command::dispatch_prompt(
        &mut pool,
        &registry,
        &device,
        &prompt,
        false,
        |_, _, _| panic!("busy registry admitted a prompt"),
    );
    assert_eq!(denied.code, Some(ErrorCode::Busy));
    assert_eq!(std::fs::read(root.join("session.jsonl")).unwrap(), journal);
    assert_eq!(pool.workspace_revision(), 0);
    drop(lock);
    assert_eq!(
        HostAdapter::dispatch_owned(&mut pool, &registry, &device, select).execution_state,
        Some(ExecutionState::Succeeded)
    );
    assert!(owner.try_lock().is_ok());
}
