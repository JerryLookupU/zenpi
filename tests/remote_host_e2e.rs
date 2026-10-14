//! Production-binary pairing smoke: QR invite -> pinned TLS -> one-use device
//! registration.  This is intentionally a real listener test, not a pipe or
//! echo fixture; Android camera and physical-LAN gates live in the Android
//! harness.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
use zenpi::remote::{
    auth::pairing_proof_message,
    frame::{read_frame, write_frame},
    pairing::{HostIdentity, PairingInvite, unix_seconds},
    protocol::{Channel, Envelope, ErrorCode, EventCursor, ExecutionState, Message, Operation},
};

type TlsClient = rustls::StreamOwned<rustls::ClientConnection, TcpStream>;

struct ChildGuard(std::process::Child);

impl ChildGuard {
    fn child_mut(&mut self) -> &mut std::process::Child {
        &mut self.0
    }

    fn wait_success(&mut self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "production host did not exit successfully"
                );
                return;
            }
            assert!(
                Instant::now() < deadline,
                "production host shutdown deadline"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

fn read_invite_with_timeout(mut child: std::process::Child) -> (ChildGuard, String) {
    let diagnostics = child.stderr.take().map(|stderr| {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut bounded = stderr.take(4096);
            let mut bytes = Vec::new();
            let _ = bounded.read_to_end(&mut bytes);
            let _ = sender.send(String::from_utf8_lossy(&bytes).into_owned());
            let mut stderr = bounded.into_inner();
            let _ = std::io::copy(&mut stderr, &mut std::io::sink());
        });
        receiver
    });
    let stdout = child.stdout.take().expect("host stdout pipe");
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut line = Vec::new();
        let mut reader = BufReader::new(stdout.take(4097));
        let result = reader
            .read_until(b'\n', &mut line)
            .map(|_| {
                if line.len() > 4096 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "invite line exceeds 4096 bytes",
                    ));
                }
                String::from_utf8(line).map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invite is not UTF-8")
                })
            })
            .and_then(|line| line);
        let _ = sender.send(result);
        // Keep the child's stdout alive after the invite. Dropping the read
        // end here races its subsequent expiry output with SIGPIPE.
        let _ = std::io::copy(&mut reader, &mut std::io::sink());
        let mut stdout = reader.into_inner().into_inner();
        let _ = std::io::copy(&mut stdout, &mut std::io::sink());
    });
    let line = match receiver.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(line)) if line.trim_start().starts_with("zenpi://pair?") => Ok(line),
        Ok(Ok(_)) => Err("production host emitted no valid invite".to_owned()),
        Ok(Err(error)) => Err(format!(
            "production host stdout failed before invite: {error}"
        )),
        Err(_) => Err("production host did not emit an invite within five seconds".to_owned()),
    };
    let line = match line {
        Ok(line) => line,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            let diagnostics = diagnostics
                .and_then(|receiver| receiver.recv_timeout(Duration::from_secs(1)).ok())
                .unwrap_or_default();
            panic!("{error}; bounded fixture stderr: {diagnostics}");
        }
    };
    (ChildGuard(child), line)
}

fn tls_client(invite: &PairingInvite, identity: &HostIdentity) -> TlsClient {
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
    let connection =
        rustls::ClientConnection::new(Arc::new(config), "zenpi.local".try_into().unwrap()).unwrap();
    let socket = TcpStream::connect_timeout(&invite.address, Duration::from_secs(3)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    rustls::StreamOwned::new(connection, socket)
}

fn tls_pair(
    invite: &PairingInvite,
    identity: &HostIdentity,
    key: &EcdsaKeyPair,
    device_name: &str,
) -> Envelope {
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
    let connection =
        rustls::ClientConnection::new(Arc::new(config), "zenpi.local".try_into().unwrap()).unwrap();
    let socket = TcpStream::connect(invite.address).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut stream = rustls::StreamOwned::new(connection, socket);
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
        nonce: _,
        host_fingerprint,
    } = read_frame(&mut stream).unwrap().unwrap().payload
    else {
        panic!("production host did not issue a pairing challenge");
    };
    assert_eq!(host_fingerprint, invite.fingerprint);
    let public_key = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    let request_id = "production-host-e2e-pair";
    let signature = URL_SAFE_NO_PAD.encode(
        key.sign(
            &ring::rand::SystemRandom::new(),
            &pairing_proof_message(
                &invite.fingerprint,
                &invite.token,
                request_id,
                device_name,
                &public_key,
            ),
        )
        .unwrap()
        .as_ref(),
    );
    let mut pair = Envelope::new(
        Channel::Control,
        Message::Pair {
            token: invite.token.clone(),
            fingerprint: invite.fingerprint.clone(),
            device_name: device_name.to_owned(),
            public_key,
            signature,
        },
    );
    pair.id = Some(request_id.to_owned());
    write_frame(&mut stream, &pair).unwrap();
    read_frame(&mut stream).unwrap().unwrap()
}

fn authenticated_workspace_list(
    invite: &PairingInvite,
    identity: &HostIdentity,
    key: &EcdsaKeyPair,
    device_id: &str,
) -> serde_json::Value {
    let mut stream = tls_client(invite, identity);
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
    let signature = URL_SAFE_NO_PAD.encode(
        key.sign(
            &ring::rand::SystemRandom::new(),
            &serde_json::to_vec(&["zenpi-device-auth-v1", &host_fingerprint, &nonce, device_id])
                .unwrap(),
        )
        .unwrap()
        .as_ref(),
    );
    let mut auth = Envelope::new(
        Channel::Control,
        Message::Authenticate {
            device_id: device_id.to_owned(),
            signature,
        },
    );
    auth.id = Some("workspace-auth".into());
    write_frame(&mut stream, &auth).unwrap();
    assert_eq!(
        read_frame(&mut stream).unwrap().unwrap().execution_state,
        Some(ExecutionState::Succeeded)
    );
    let mut list = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: zenpi::remote::protocol::Operation::WorkspaceList,
            arguments: serde_json::json!({}),
        },
    );
    list.id = Some("workspace-list".into());
    write_frame(&mut stream, &list).unwrap();
    let Message::Response { result } = read_frame(&mut stream).unwrap().unwrap().payload else {
        panic!("workspace list")
    };
    result
}

/// Authenticate and issue one request against the production listener. This
/// keeps the provider/prompt path in the same binary E2E suite as pairing.
fn authenticated_request(
    invite: &PairingInvite,
    identity: &HostIdentity,
    key: &EcdsaKeyPair,
    device_id: &str,
    request: Envelope,
) -> Vec<Envelope> {
    let mut stream = tls_client(invite, identity);
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
    let signature = URL_SAFE_NO_PAD.encode(
        key.sign(
            &ring::rand::SystemRandom::new(),
            &serde_json::to_vec(&["zenpi-device-auth-v1", &host_fingerprint, &nonce, device_id])
                .unwrap(),
        )
        .unwrap()
        .as_ref(),
    );
    let mut auth = Envelope::new(
        Channel::Control,
        Message::Authenticate {
            device_id: device_id.to_owned(),
            signature,
        },
    );
    auth.id = Some(format!(
        "auth-{}",
        request.id.as_deref().unwrap_or("request")
    ));
    write_frame(&mut stream, &auth).unwrap();
    assert_eq!(
        read_frame(&mut stream).unwrap().unwrap().execution_state,
        Some(ExecutionState::Succeeded)
    );
    write_frame(&mut stream, &request).unwrap();
    let mut frames = Vec::new();
    while let Some(frame) = read_frame(&mut stream).unwrap() {
        let terminal = frame.id == request.id && frame.execution_state.is_some();
        frames.push(frame);
        if terminal {
            break;
        }
    }
    frames
}

const TOOL_PROMPT: &str = "STAGE2_BINARY_WRITE";
const CANCEL_PROMPT: &str = "STAGE2_BINARY_CANCEL";

fn read_provider_request(input: impl Read) -> std::io::Result<serde_json::Value> {
    let mut reader = BufReader::new(input);
    let mut header = Vec::new();
    // Consume a bounded header byte by byte; do not read past Content-Length.
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() >= 16 * 1024 {
            return Err(std::io::Error::other("fixture HTTP header limit"));
        }
        let mut byte = [0];
        reader.read_exact(&mut byte)?;
        header.push(byte[0]);
    }
    let header = std::str::from_utf8(&header).map_err(std::io::Error::other)?;
    if !header.starts_with("POST /v1/responses HTTP/1.1\r\n") {
        return Err(std::io::Error::other("unexpected provider route"));
    }
    let lengths: Vec<_> = header
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then_some(value.trim())
        })
        .collect();
    if lengths.len() != 1 {
        return Err(std::io::Error::other(
            "fixture requires exact Content-Length",
        ));
    }
    let length: usize = lengths[0].parse().map_err(std::io::Error::other)?;
    if length > 1024 * 1024 {
        return Err(std::io::Error::other("fixture HTTP body limit"));
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(std::io::Error::other)
}

fn provider_events(request: &serde_json::Value) -> Vec<serde_json::Value> {
    use serde_json::json;
    let input = request["input"].as_array().expect("Responses input");
    let user = input
        .iter()
        .rev()
        .find(|item| item["role"] == "user")
        .unwrap();
    let prompt = user["content"].as_str().unwrap();
    let completed = json!({"type":"response.completed","response":{
        "id":"stage2-e2e","status":"completed","model":request["model"]}});
    if prompt == CANCEL_PROMPT {
        return vec![json!({"type":"response.output_text.delta","delta":"CANCEL_PREVIEW\n"})];
    }
    assert_eq!(prompt, TOOL_PROMPT);
    if input
        .iter()
        .any(|item| item["type"] == "function_call_output" && item["call_id"] == "e2e-write")
    {
        vec![
            json!({"type":"response.output_text.delta","delta":"WRITE_DONE"}),
            completed,
        ]
    } else {
        vec![
            json!({"type":"response.output_text.delta","delta":"WRITE_PREVIEW\n"}),
            json!({"type":"response.output_item.done","item":{"type":"function_call",
                "call_id":"e2e-write","name":"write_file",
                "arguments":json!({"path":"approved.txt","content":"APPROVED_CONTENT"}).to_string()}}),
            completed,
        ]
    }
}

struct ProviderFixture {
    url: String,
    stop: Arc<AtomicBool>,
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
    join: Option<thread::JoinHandle<()>>,
}

impl ProviderFixture {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let join = thread::spawn(move || {
            while !stopping.load(Ordering::Acquire) {
                let (mut socket, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                };
                // macOS inherits the listener's nonblocking flag on accept.
                // Request framing uses bounded blocking reads, not readiness polling.
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let request = read_provider_request(&mut socket).unwrap();
                let events = provider_events(&request);
                let cancel = events.len() == 1;
                recorded.lock().unwrap().push(request);
                socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n").unwrap();
                for event in events {
                    write!(socket, "data: {event}\n\n").unwrap();
                    socket.flush().unwrap();
                }
                if cancel {
                    // Keep the body incomplete until the test has observed the
                    // preview and cancelled through the production owner.
                    socket
                        .set_read_timeout(Some(Duration::from_millis(100)))
                        .unwrap();
                    while !stopping.load(Ordering::Acquire) {
                        let mut byte = [0];
                        match socket.read(&mut byte) {
                            Ok(0) => break,
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                                ) => {}
                            Err(_) => break,
                            Ok(_) => panic!("unexpected bytes during held provider body"),
                        }
                    }
                }
            }
        });
        Self {
            url,
            stop,
            requests,
            join: Some(join),
        }
    }
}

impl Drop for ProviderFixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.take()
            && let Err(error) = join.join()
            && !thread::panicking()
        {
            std::panic::resume_unwind(error);
        }
    }
}

fn request_from_listing(
    listing: &serde_json::Value,
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
    request.project_id = Some(listing["projects"][0]["id"].as_str().unwrap().into());
    request.session_id = Some(
        listing["projects"][0]["session_id"]
            .as_str()
            .unwrap()
            .into(),
    );
    if operation.is_write() {
        request.required_owner = Some(listing["required_owner"].as_str().unwrap().into());
        request.expected_revision = Some(listing["workspace_revision"].as_u64().unwrap());
    }
    request
}

fn response_result(frame: &Envelope) -> &serde_json::Value {
    assert_eq!(
        frame.execution_state,
        Some(ExecutionState::Succeeded),
        "{frame:?}"
    );
    let Message::Response { result } = &frame.payload else {
        panic!("response required")
    };
    result
}

fn wait_listing(
    invite: &PairingInvite,
    identity: &HostIdentity,
    key: &EcdsaKeyPair,
    device: &str,
    ready: impl Fn(&serde_json::Value) -> bool,
) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(6);
    loop {
        let listing = authenticated_workspace_list(invite, identity, key, device);
        if ready(&listing) {
            return listing;
        }
        assert!(
            Instant::now() < deadline,
            "production lifecycle condition deadline"
        );
        thread::sleep(Duration::from_millis(30));
    }
}

/// Real production binary, HTTP provider, pinned TLS, approval and journal.
/// No socket-denied path is converted into a pass.
#[test]
#[ignore = "requires production TLS and HTTP loopback listeners"]
fn production_prompt_tool_approval_cancel_replay_and_clean_restart() {
    use serde_json::json;
    let fixture = ProviderFixture::start();
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().join("workspace");
    let home = temporary.path().join("home");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::create_dir(&home).unwrap();
    let workspace = workspace.canonicalize().unwrap();
    std::fs::write(home.join("config.toml"), format!(
        "backend=\"openai\"\nmodel=\"gpt-4.1\"\nbase_url=\"{}\"\nwire_api=\"responses\"\nrequires_openai_auth=false\nmax_retries=0\n",
        fixture.url)).unwrap();
    let session = temporary.path().join("session.jsonl");
    zenpi::session::SessionStore::open_in_workspace(&session, &workspace).unwrap();
    let identity_path = temporary.path().join("identity.json");
    let spawn = |duration: &str| {
        let child = Command::new(env!("CARGO_BIN_EXE_zenpi"))
            .args([
                "host",
                "pair",
                "--plain",
                "--bind",
                "127.0.0.1:0",
                "--duration-seconds",
                duration,
                "--identity",
            ])
            .arg(&identity_path)
            .arg("--workspace")
            .arg(&workspace)
            .arg("--session")
            .arg(&session)
            .current_dir(&workspace)
            .env_clear()
            .env("HOME", &home)
            .env("ZENPI_HOME", &home)
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("NO_PROXY", "127.0.0.1,localhost")
            .env("no_proxy", "127.0.0.1,localhost")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let (child, uri) = read_invite_with_timeout(child);
        (
            child,
            PairingInvite::from_uri(uri.trim(), unix_seconds().unwrap()).unwrap(),
        )
    };
    let (mut host, invite) = spawn("25");
    let identity = HostIdentity::load_or_create(&identity_path).unwrap();
    let random = ring::rand::SystemRandom::new();
    let document = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, document.as_ref(), &random)
        .unwrap();
    // ZE-514: scanning the desktop QR is the confirmation; no grant CLI follows.
    let paired = tls_pair(&invite, &identity, &key, "production-phone");
    assert_eq!(response_result(&paired)["control_available"], true);
    let device = response_result(&paired)["device_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let listing = authenticated_workspace_list(&invite, &identity, &key, &device);
    assert_eq!(listing["projects"][0]["prompt_control"], true);
    let prompt = request_from_listing(
        &listing,
        "e2e-prompt",
        Operation::Prompt,
        json!({"text":TOOL_PROMPT}),
    );
    let admitted = authenticated_request(&invite, &identity, &key, &device, prompt.clone());
    assert_eq!(
        admitted.last().unwrap().execution_state,
        Some(ExecutionState::Accepted)
    );
    let pending = wait_listing(&invite, &identity, &key, &device, |listing| {
        !listing["projects"][0]["pending_approvals"]
            .as_array()
            .unwrap()
            .is_empty()
    });
    assert!(
        !workspace.join("approved.txt").exists(),
        "tool must wait for phone approval"
    );
    let statuses = pending["projects"][0]["prompt_statuses"]
        .as_array()
        .unwrap();
    assert!(
        statuses
            .iter()
            .any(|status| status["request_id"] == "e2e-prompt"
                && status["stream_text"] == "WRITE_PREVIEW\n")
    );
    let approval = &pending["projects"][0]["pending_approvals"][0];
    let decision = request_from_listing(
        &pending,
        "e2e-approve",
        Operation::Approve,
        json!({"request_id":approval["request_id"],"digest":approval["digest"],"decision":"allow"}),
    );
    let approved = authenticated_request(&invite, &identity, &key, &device, decision);
    assert_eq!(
        approved.last().unwrap().execution_state,
        Some(ExecutionState::Accepted)
    );
    let completed = wait_listing(&invite, &identity, &key, &device, |listing| {
        listing["projects"][0]["prompt_statuses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|status| status["request_id"] == "e2e-prompt" && status["state"] == "succeeded")
    });
    assert_eq!(
        std::fs::read_to_string(workspace.join("approved.txt")).unwrap(),
        "APPROVED_CONTENT"
    );
    assert_eq!(fixture.requests.lock().unwrap().len(), 2);
    assert!(
        fixture.requests.lock().unwrap()[1]["input"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["type"] == "function_call_output"
                && item["call_id"] == "e2e-write"
                && item["output"]
                    .as_str()
                    .is_some_and(|output| output.contains("approved.txt"))),
        "provider continuation must contain the actual write_file result"
    );
    let retry = authenticated_request(&invite, &identity, &key, &device, prompt.clone());
    assert_eq!(
        retry, admitted,
        "exact retry must return the original admission"
    );
    let mut conflict = prompt.clone();
    conflict.payload = Message::Request {
        operation: Operation::Prompt,
        arguments: json!({"text":"changed"}),
    };
    assert_eq!(
        authenticated_request(&invite, &identity, &key, &device, conflict)
            .last()
            .unwrap()
            .code,
        Some(ErrorCode::RequestIdConflict)
    );
    let mut replay = request_from_listing(&completed, "e2e-replay", Operation::Inspect, json!({}));
    replay.payload = Message::Resume {
        cursor: EventCursor {
            session_id: prompt.session_id.clone().unwrap(),
            next_sequence: 0,
        },
        limit: 128,
    };
    let frames = authenticated_request(&invite, &identity, &key, &device, replay.clone());
    let turns: Vec<_> = frames
        .iter()
        .filter_map(|frame| match &frame.payload {
            Message::Event { name, data } if name == "transcript_turn" => Some(data),
            _ => None,
        })
        .collect();
    assert!(turns.iter().any(|turn| turn["role"] == "tool"));
    assert!(
        turns
            .iter()
            .any(|turn| turn["role"] == "assistant" && turn["content"] == "WRITE_DONE")
    );
    assert_eq!(
        response_result(frames.last().unwrap())["snapshot_required"],
        false
    );
    let cancel_prompt = request_from_listing(
        &completed,
        "e2e-cancel-target",
        Operation::Prompt,
        json!({"text":CANCEL_PROMPT}),
    );
    assert_eq!(
        authenticated_request(&invite, &identity, &key, &device, cancel_prompt)
            .last()
            .unwrap()
            .execution_state,
        Some(ExecutionState::Accepted)
    );
    let running = wait_listing(&invite, &identity, &key, &device, |listing| {
        listing["projects"][0]["prompt_statuses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|status| {
                status["request_id"] == "e2e-cancel-target"
                    && status["stream_text"] == "CANCEL_PREVIEW\n"
            })
    });
    let cancel = request_from_listing(
        &running,
        "e2e-cancel",
        Operation::Cancel,
        json!({"request_id":"e2e-cancel-target"}),
    );
    assert_eq!(
        authenticated_request(&invite, &identity, &key, &device, cancel)
            .last()
            .unwrap()
            .execution_state,
        Some(ExecutionState::Accepted)
    );
    wait_listing(&invite, &identity, &key, &device, |listing| {
        listing["projects"][0]["prompt_statuses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|status| {
                status["request_id"] == "e2e-cancel-target" && status["state"] == "cancelled"
            })
    });
    let final_frames = authenticated_request(&invite, &identity, &key, &device, replay.clone());
    host.wait_success(Duration::from_secs(30));
    let (mut restarted, restart_invite) = spawn("5");
    let recovered = authenticated_request(&restart_invite, &identity, &key, &device, prompt);
    assert_eq!(
        recovered, admitted,
        "restart must not repeat the provider or tool"
    );
    let restarted_frames = authenticated_request(&restart_invite, &identity, &key, &device, replay);
    assert_eq!(
        restarted_frames
            .iter()
            .filter(|frame| frame.channel == Channel::Event)
            .collect::<Vec<_>>(),
        final_frames
            .iter()
            .filter(|frame| frame.channel == Channel::Event)
            .collect::<Vec<_>>(),
        "durable replay must survive owner restart"
    );
    assert_eq!(
        response_result(restarted_frames.last().unwrap())["cursor"],
        response_result(final_frames.last().unwrap())["cursor"]
    );
    restarted.wait_success(Duration::from_secs(10));
    assert_eq!(fixture.requests.lock().unwrap().len(), 3);
    let journal = zenpi::session::SessionStore::open_existing(&session).unwrap();
    assert_eq!(
        journal
            .turns()
            .iter()
            .filter(|turn| turn.content == TOOL_PROMPT)
            .count(),
        1
    );
    assert_eq!(
        journal
            .turns()
            .iter()
            .filter(|turn| turn.content == "WRITE_DONE")
            .count(),
        1
    );
}

#[test]
fn provider_http_fixture_consumes_exact_bounded_body_and_refuses_truncation() {
    use serde_json::json;
    let body =
        json!({"model":"gpt-4.1","input":[{"role":"user","content":TOOL_PROMPT}]}).to_string();
    let bytes = format!(
        "POST /v1/responses HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    assert_eq!(
        read_provider_request(bytes.as_bytes()).unwrap(),
        serde_json::from_str::<serde_json::Value>(&body).unwrap()
    );
    for size in [0, 1, bytes.len() - 1] {
        assert!(read_provider_request(&bytes.as_bytes()[..size]).is_err());
    }
    for header in [
        "GET /v1/responses HTTP/1.1\r\nContent-Length: 0\r\n\r\n",
        "POST /v1/responses HTTP/1.1\r\nContent-Length: 1048577\r\n\r\n",
        "POST /v1/responses HTTP/1.1\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
    ] {
        assert!(read_provider_request(header.as_bytes()).is_err());
    }
}

#[test]
fn provider_fixture_requires_tool_result_before_final_answer_and_holds_cancel_open() {
    use serde_json::json;
    let mut request = json!({"model":"gpt-4.1","input":[{"role":"user","content":TOOL_PROMPT}]});
    let first = provider_events(&request);
    assert_eq!(first[0]["delta"], "WRITE_PREVIEW\n");
    assert_eq!(first[1]["item"]["name"], "write_file");
    assert_eq!(first[2]["type"], "response.completed");
    request["input"].as_array_mut().unwrap().push(json!({
        "type":"function_call_output","call_id":"e2e-write","output":"success"}));
    let continuation = provider_events(&request);
    assert_eq!(continuation[0]["delta"], "WRITE_DONE");
    assert_eq!(continuation.len(), 2);
    request["input"] = json!([{"role":"user","content":CANCEL_PROMPT}]);
    assert_eq!(
        provider_events(&request),
        vec![json!({
        "type":"response.output_text.delta","delta":"CANCEL_PREVIEW\n"})]
    );
}

/// Requires a real loopback listener and filesystem/network permissions.
/// Keep it out of the default offline suite so a sandbox cannot turn a skipped
/// socket bind into a false production claim; run with `cargo test --ignored`.
#[test]
#[ignore = "requires a real production TLS listener and LAN-capable sandbox"]
fn production_attached_workspace_is_controllable_right_after_pairing() {
    let fixture = ProviderFixture::start();
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().join("workspace");
    let home = temporary.path().join("home");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::create_dir(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!(
            "backend=\"openai\"\nmodel=\"gpt-4.1\"\nbase_url=\"{}\"\nwire_api=\"responses\"\nrequires_openai_auth=false\nmax_retries=0\n",
            fixture.url
        ),
    )
    .unwrap();
    let workspace = workspace.canonicalize().unwrap();
    let session = temporary.path().join("session.jsonl");
    zenpi::session::SessionStore::open_in_workspace(&session, &workspace).unwrap();
    let identity_path = temporary.path().join("identity.json");
    let child = Command::new(env!("CARGO_BIN_EXE_zenpi"))
        .args([
            "host",
            "pair",
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
        .arg(&session)
        .env_clear()
        .env("HOME", &home)
        .env("ZENPI_HOME", &home)
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let (_child, line) = read_invite_with_timeout(child);
    let invite = PairingInvite::from_uri(line.trim(), unix_seconds().unwrap()).unwrap();
    let identity = HostIdentity::load_or_create(&identity_path).unwrap();
    let random = ring::rand::SystemRandom::new();
    let document = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, document.as_ref(), &random)
        .unwrap();
    let first = tls_pair(&invite, &identity, &key, "e2e-workspace");
    let Message::Response { result } = first.payload else {
        panic!("pairing response missing");
    };
    let device_id = result["device_id"].as_str().unwrap().to_owned();
    assert_eq!(result["control_available"], true);
    use sha2::Digest;
    let mut hash = sha2::Sha256::new();
    hash.update(b"zenpi-project-workspace-v1\0");
    hash.update(workspace.to_str().unwrap().as_bytes());
    let project = format!("{:x}", hash.finalize());
    let listing = authenticated_workspace_list(&invite, &identity, &key, &device_id);
    let mounted = listing["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == project)
        .unwrap();
    assert_eq!(mounted["prompt_control"], true);
    assert_eq!(mounted["approval_control"], true);
}

/// Requires a real loopback listener and filesystem/network permissions.
#[test]
#[ignore = "requires a real production TLS listener and LAN-capable sandbox"]
fn production_binary_invite_is_pinned_and_consumed_once() {
    let temporary = tempfile::tempdir().unwrap();
    let identity_path = temporary.path().join("identity.json");
    let child = Command::new(env!("CARGO_BIN_EXE_zenpi"))
        .args([
            "host",
            "pair",
            "--bind",
            "127.0.0.1:0",
            "--duration-seconds",
            "8",
            "--identity",
        ])
        .arg(&identity_path)
        .env_clear()
        .env("HOME", temporary.path().join("home"))
        .env("ZENPI_HOME", temporary.path().join("home"))
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let (mut child, line) = read_invite_with_timeout(child);
    let invite = PairingInvite::from_uri(line.trim(), unix_seconds().unwrap()).unwrap();
    let identity = HostIdentity::load_or_create(&identity_path).unwrap();
    let random = ring::rand::SystemRandom::new();
    let document = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, document.as_ref(), &random)
        .unwrap();

    let first = tls_pair(&invite, &identity, &key, "e2e-phone");
    assert_eq!(first.execution_state, Some(ExecutionState::Succeeded));
    let Message::Response { result } = first.payload else {
        panic!("pairing response missing");
    };
    assert_eq!(result["paired"], true);
    assert_eq!(result["device_registered"], true);
    // No --workspace is mounted, so pairing truthfully grants no control.
    assert_eq!(result["control_available"], false);

    let replay = tls_pair(&invite, &identity, &key, "e2e-phone");
    assert_eq!(replay.execution_state, Some(ExecutionState::Failed));
    assert!(replay.code.is_some());
    assert!(child.child_mut().wait().unwrap().success());
}

/// ZE-513: `zenpi host serve` outlives its invite and keeps its port.
#[test]
#[ignore = "requires a real production TLS listener and LAN-capable sandbox"]
fn production_serve_outlives_the_invite_and_keeps_its_port_across_restart() {
    let fixture = ProviderFixture::start();
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().join("workspace");
    let home = temporary.path().join("home");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::create_dir(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!(
            "backend=\"openai\"\nmodel=\"gpt-4.1\"\nbase_url=\"{}\"\nwire_api=\"responses\"\nrequires_openai_auth=false\nmax_retries=0\n",
            fixture.url
        ),
    )
    .unwrap();
    let workspace = workspace.canonicalize().unwrap();
    let session = temporary.path().join("session.jsonl");
    zenpi::session::SessionStore::open_in_workspace(&session, &workspace).unwrap();
    let identity_path = temporary.path().join("identity.json");
    let serve = || {
        let child = Command::new(env!("CARGO_BIN_EXE_zenpi"))
            .args([
                "host",
                "serve",
                "--bind",
                "127.0.0.1:0",
                "--duration-seconds",
                "2",
                "--identity",
            ])
            .arg(&identity_path)
            .arg("--workspace")
            .arg(&workspace)
            .arg("--session")
            .arg(&session)
            .env_clear()
            .env("HOME", &home)
            .env("ZENPI_HOME", &home)
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let (child, line) = read_invite_with_timeout(child);
        (
            child,
            PairingInvite::from_uri(line.trim(), unix_seconds().unwrap()).unwrap(),
        )
    };
    let (child, invite) = serve();
    let identity = HostIdentity::load_or_create(&identity_path).unwrap();
    let random = ring::rand::SystemRandom::new();
    let document = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, document.as_ref(), &random)
        .unwrap();
    let paired = tls_pair(&invite, &identity, &key, "serve-phone");
    let Message::Response { result } = paired.payload else {
        panic!("pairing response missing");
    };
    let device_id = result["device_id"].as_str().unwrap().to_owned();
    assert_eq!(result["control_available"], true);
    // The invite expires; the paired device still reconnects with its key.
    thread::sleep(Duration::from_secs(3));
    let listing = authenticated_workspace_list(&invite, &identity, &key, &device_id);
    assert!(listing["projects"].as_array().is_some());
    drop(child);
    let (child, restarted) = serve();
    assert_eq!(restarted.address.port(), invite.address.port());
    let listing = authenticated_workspace_list(&restarted, &identity, &key, &device_id);
    assert!(listing["projects"].as_array().is_some());
    // ZE-515: a new certificate for the same key keeps the pinned host id, so the
    // paired device reconnects without re-pairing.
    drop(child);
    let rotated = identity.rotate_certificate().unwrap();
    let mut record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&identity_path).unwrap()).unwrap();
    record["certificate"] = URL_SAFE_NO_PAD.encode(rotated.certificate()).into();
    std::fs::write(&identity_path, serde_json::to_vec(&record).unwrap()).unwrap();
    let (_child, after_rotation) = serve();
    assert_eq!(after_rotation.fingerprint, invite.fingerprint);
    assert_eq!(
        HostIdentity::load_or_create(&identity_path)
            .unwrap()
            .certificate(),
        rotated.certificate()
    );
    let listing = authenticated_workspace_list(&after_rotation, &rotated, &key, &device_id);
    assert_eq!(listing["projects"][0]["prompt_control"], true);
}
