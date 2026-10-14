#![cfg(feature = "remote")]

use ratatui::style::Color;
use std::{
    io::{BufRead, BufReader},
    net::TcpStream,
    process::{Child, Command, Stdio},
    sync::Arc,
    time::Duration,
};
use zenpi::{
    host_pair::qr_lines,
    remote::{
        frame::{read_frame, write_frame},
        pairing::{HostIdentity, InviteIssuer, PairingInvite, unix_seconds},
        protocol::{Channel, Envelope, ExecutionState, Message},
    },
};

#[test]
fn host_pair_help_is_side_effect_free_and_does_not_require_home_or_provider() {
    for flag in ["--help", "-h"] {
        let parsed = zenpi::core::parse_args(["host", "pair", flag]).unwrap();
        assert!(parsed.host_pair.unwrap().help);
        let root = tempfile::tempdir().unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_zenpi"))
            .args(["host", "pair", flag])
            .env_remove("HOME")
            .env("ZENPI_HOME", root.path().join("missing-home"))
            .current_dir(root.path())
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output);
        assert!(output.stderr.is_empty());
        let help = String::from_utf8(output.stdout).unwrap();
        for expected in [
            "Usage: zenpi host pair",
            "--desktop",
            "--workspace PATH --session PATH",
            "--list-devices",
            "--revoke-device",
            "--reconcile-worktree",
            "Internet/NAT",
        ] {
            assert!(help.contains(expected));
        }
        assert!(!help.contains("zenpi://pair?"));
        assert!(!help.contains("--grant-device") && !help.to_lowercase().contains("safety code"));
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
    for arguments in [
        vec!["--help", "--plain"],
        vec!["--desktop", "--help"],
        vec!["--bad", "--help"],
    ] {
        assert!(zenpi::core::parse_args([vec!["host", "pair"], arguments].concat()).is_err());
    }
    let identity = zenpi::core::parse_args(["host", "pair", "--identity", "--help"])
        .unwrap()
        .host_pair
        .unwrap();
    assert!(!identity.help);
    assert_eq!(identity.identity, std::path::PathBuf::from("--help"));
}

#[test]
fn terminal_pixels_decode_to_the_exact_invite_with_quiet_zone() {
    let identity = HostIdentity::generate().unwrap();
    let issuer = InviteIssuer::new(&identity);
    for address in ["192.168.50.55:7777", "[fd00::1234]:65535"] {
        let invite = issuer
            .issue(address.parse().unwrap(), Duration::from_secs(60))
            .unwrap();
        let lines = qr_lines(&invite).unwrap();
        let width = lines[0].spans.len();
        let scale = 4;
        let height = lines.len() * 2;
        let mut pixels = vec![255u8; width * height * scale * scale];
        for (row, line) in lines.iter().enumerate() {
            assert_eq!(line.spans.len(), width);
            for (column, span) in line.spans.iter().enumerate() {
                assert_eq!(span.content, "▀");
                for (half, color) in [span.style.fg, span.style.bg].into_iter().enumerate() {
                    let value = if color == Some(Color::Black) { 0 } else { 255 };
                    for y in 0..scale {
                        for x in 0..scale {
                            pixels[((row * 2 + half) * scale + y) * width * scale
                                + column * scale
                                + x] = value;
                        }
                    }
                }
            }
        }
        let mut decoder = quircs::Quirc::default();
        let codes: Vec<_> = decoder
            .identify(width * scale, height * scale, &pixels)
            .collect();
        assert_eq!(codes.len(), 1);
        let decoded = codes[0].as_ref().unwrap().decode().unwrap();
        assert_eq!(
            String::from_utf8(decoded.payload).unwrap(),
            invite.to_uri().unwrap()
        );
    }
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn authenticate_host(
    invite: &PairingInvite,
    identity: &HostIdentity,
    key: &ring::signature::EcdsaKeyPair,
) -> Envelope {
    authenticated_stream(invite, identity, key).0
}

fn authenticated_stream(
    invite: &PairingInvite,
    identity: &HostIdentity,
    key: &ring::signature::EcdsaKeyPair,
) -> (
    Envelope,
    rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
) {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use ring::signature::KeyPair;
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
    let socket = TcpStream::connect(invite.address).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut stream = rustls::StreamOwned::new(
        rustls::ClientConnection::new(Arc::new(config), "zenpi.local".try_into().unwrap()).unwrap(),
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
    read_frame(&mut stream).unwrap().unwrap();
    let Message::Challenge {
        nonce,
        host_fingerprint,
    } = read_frame(&mut stream).unwrap().unwrap().payload
    else {
        panic!("challenge");
    };
    let id =
        zenpi::remote::auth::device_id(&URL_SAFE_NO_PAD.encode(key.public_key().as_ref())).unwrap();
    let message =
        serde_json::to_vec(&["zenpi-device-auth-v1", &host_fingerprint, &nonce, &id]).unwrap();
    let mut request = Envelope::new(
        Channel::Control,
        Message::Authenticate {
            device_id: id,
            signature: URL_SAFE_NO_PAD.encode(
                key.sign(&ring::rand::SystemRandom::new(), &message)
                    .unwrap()
                    .as_ref(),
            ),
        },
    );
    request.id = Some("auth-1".into());
    write_frame(&mut stream, &request).unwrap();
    (read_frame(&mut stream).unwrap().unwrap(), stream)
}

#[cfg(unix)]
#[test]
fn committed_tls_write_with_busy_delivery_stays_unknown_and_exact_retry_is_idempotent() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
    use std::{
        os::fd::AsRawFd,
        sync::{
            Mutex,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
    };
    use zenpi::remote::{
        auth::{Capability, DeviceRegistry, pairing_proof_message},
        host_adapter::HostAdapter,
        protocol::{ErrorCode, Operation},
    };
    use zenpi::{
        backend::EchoBackend, core::Agent, project_workspace::ProjectOwnerPool,
        session::SessionStore,
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let identity_path = root.join("identity.json");
    let identity = HostIdentity::load_or_create(&identity_path).unwrap();
    let registry = Arc::new(
        DeviceRegistry::new(
            identity_path.with_extension("devices.json"),
            identity.fingerprint(),
        )
        .unwrap(),
    );
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
    let record = registry
        .verify_pair_and_register("token", "pair", "phone", &public, &signature, || true)
        .unwrap();
    let owner = Arc::new(Mutex::new(Agent::new(
        SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        Box::new(EchoBackend),
    )));
    let mut pool = ProjectOwnerPool::new(owner).unwrap();
    pool.restore_checkpoint().unwrap();
    let project = pool.active_context().project_id.clone();
    registry
        .set_permissions(
            &record.id,
            record.revision,
            vec![Capability::Read, Capability::Control],
            vec![project.clone()],
        )
        .unwrap();
    let mut request = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::WorkspaceSelect,
            arguments: serde_json::json!({}),
        },
    );
    request.id = Some("committed-before-busy".into());
    request.project_id = Some(project);
    request.session_id = Some(pool.active_context().session_id.clone());
    request.required_owner = Some(pool.workspace_owner().into());
    request.expected_revision = Some(pool.workspace_revision());
    let (adapter, mailbox) = HostAdapter::owner_thread();
    let options = zenpi::core::parse_args([
        "host",
        "pair",
        "--bind",
        "127.0.0.1:0",
        "--plain",
        "--identity",
        identity_path.to_str().unwrap(),
    ])
    .unwrap()
    .host_pair
    .unwrap();
    let mut host = zenpi::host_pair::PairingHost::start(&options, Arc::new(adapter)).unwrap();
    let invite = host.invite().clone();
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let mut injected = false;
        while !stopping.load(Ordering::Acquire) {
            mailbox.drain_with(|registry, device, request| {
                let response = HostAdapter::dispatch_owned(&mut pool, registry, device, request);
                if !injected && response.execution_state == Some(ExecutionState::Succeeded) {
                    assert_eq!(pool.workspace_revision(), 1);
                    let lock = std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(identity_path.with_extension("devices.lock"))
                        .unwrap();
                    assert_eq!(
                        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
                        0
                    );
                    sender.send(lock).unwrap();
                    injected = true;
                }
                response
            });
            std::thread::sleep(Duration::from_millis(1));
        }
        pool.workspace_revision()
    });
    let (authenticated, mut stream) = authenticated_stream(&invite, &identity, &key);
    assert_eq!(
        authenticated.execution_state,
        Some(ExecutionState::Succeeded)
    );
    write_frame(&mut stream, &request).unwrap();
    let unknown = read_frame(&mut stream).unwrap().unwrap();
    assert_eq!(unknown.id, request.id);
    assert_eq!(
        unknown.execution_state,
        Some(ExecutionState::UnknownOutcome)
    );
    assert_eq!(unknown.code, Some(ErrorCode::Busy));
    unknown.validate().unwrap();
    drop(stream);
    drop(receiver.recv_timeout(Duration::from_secs(1)).unwrap());
    let (authenticated, mut stream) = authenticated_stream(&invite, &identity, &key);
    assert_eq!(
        authenticated.execution_state,
        Some(ExecutionState::Succeeded)
    );
    write_frame(&mut stream, &request).unwrap();
    let duplicate = read_frame(&mut stream).unwrap().unwrap();
    assert_eq!(duplicate.execution_state, Some(ExecutionState::Succeeded));
    let Message::Response { result } = duplicate.payload else {
        panic!("receipt")
    };
    assert_eq!(result["workspace_revision"], 1);
    drop(stream);
    host.shutdown().unwrap();
    stop.store(true, Ordering::Release);
    assert_eq!(worker.join().unwrap(), 1);
}

#[test]
fn production_host_pair_negotiates_and_consumes_invite_over_tls() {
    let temporary = tempfile::tempdir().unwrap();
    let identity_path = temporary.path().join("identity.json");
    let child = Command::new(env!("CARGO_BIN_EXE_zenpi"))
        .args([
            "host",
            "pair",
            "--bind",
            "127.0.0.1:0",
            "--plain",
            "--duration-seconds",
            "6",
            "--identity",
        ])
        .arg(&identity_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut child = ChildGuard(child);
    let mut output = BufReader::new(child.0.stdout.take().unwrap());
    let mut line = String::new();
    output.read_line(&mut line).unwrap();
    let invite = PairingInvite::from_uri(line.trim(), unix_seconds().unwrap()).unwrap();
    let identity = HostIdentity::load_or_create(&identity_path).unwrap();
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use ring::signature::KeyPair;
    let random = ring::rand::SystemRandom::new();
    let pkcs8 = ring::signature::EcdsaKeyPair::generate_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
        &random,
    )
    .unwrap();
    let key = ring::signature::EcdsaKeyPair::from_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
        pkcs8.as_ref(),
        &random,
    )
    .unwrap();
    let public_key = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    assert_eq!(invite.fingerprint, identity.fingerprint());
    for expected in [ExecutionState::Succeeded, ExecutionState::Failed] {
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
            rustls::ClientConnection::new(Arc::new(config), "zenpi.local".try_into().unwrap())
                .unwrap();
        let socket = TcpStream::connect(invite.address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(2)))
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
        assert!(matches!(
            read_frame(&mut stream).unwrap().unwrap().payload,
            Message::Challenge { .. }
        ));
        let mut pair = Envelope::new(
            Channel::Control,
            Message::Pair {
                token: invite.token.clone(),
                fingerprint: invite.fingerprint.clone(),
                device_name: "test-phone".into(),
                public_key: public_key.clone(),
                signature: URL_SAFE_NO_PAD.encode(
                    key.sign(
                        &random,
                        &zenpi::remote::auth::pairing_proof_message(
                            &invite.fingerprint,
                            &invite.token,
                            "pair-1",
                            "test-phone",
                            &public_key,
                        ),
                    )
                    .unwrap()
                    .as_ref(),
                ),
            },
        );
        pair.id = Some("pair-1".into());
        write_frame(&mut stream, &pair).unwrap();
        let response = read_frame(&mut stream).unwrap().unwrap();
        assert_eq!(response.id.as_deref(), Some("pair-1"));
        assert_eq!(response.execution_state, Some(expected));
        let Message::Response { result } = response.payload else {
            panic!("response expected");
        };
        assert_eq!(result["paired"], expected == ExecutionState::Succeeded);
        assert_eq!(result["control_available"], false);
        assert_eq!(
            result["device_registered"],
            expected == ExecutionState::Succeeded
        );
    }
    let registry = zenpi::remote::auth::DeviceRegistry::new(
        identity_path.with_extension("devices.json"),
        identity.fingerprint(),
    )
    .unwrap();
    let device = registry.list().unwrap().pop().unwrap();
    assert_eq!(
        device.id,
        zenpi::remote::auth::device_id(&public_key).unwrap()
    );
    assert!(device.projects.is_empty());
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(identity_path.with_extension("devices.lock"))
            .unwrap();
        assert_eq!(
            unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        let response = authenticate_host(&invite, &identity, &key);
        assert_eq!(response.execution_state, Some(ExecutionState::Failed));
        assert_eq!(
            response.code,
            Some(zenpi::remote::protocol::ErrorCode::Busy)
        );
        drop(lock);
    }
    assert_eq!(
        authenticate_host(&invite, &identity, &key).execution_state,
        Some(ExecutionState::Succeeded)
    );
    let revoked = Command::new(env!("CARGO_BIN_EXE_zenpi"))
        .args(["host", "pair", "--identity"])
        .arg(&identity_path)
        .args(["--revoke-device", &device.id])
        .output()
        .unwrap();
    assert!(revoked.status.success());
    assert!(registry.lookup(&device.id).unwrap().revoked);
    assert_eq!(
        authenticate_host(&invite, &identity, &key).execution_state,
        Some(ExecutionState::Failed)
    );
    assert!(child.0.wait().unwrap().success());
    assert!(TcpStream::connect(invite.address).is_err());
}

#[test]
fn host_cli_is_explicit_and_rejects_invalid_policy() {
    let quota = zenpi::core::parse_args([
        "host",
        "pair",
        "--workspace",
        "/workspace",
        "--session",
        "/session.jsonl",
        "--reconcile-quota",
        "request",
        "--device",
        &"a".repeat(64),
    ])
    .unwrap()
    .host_pair
    .unwrap();
    assert_eq!(quota.reconcile_quota.as_deref(), Some("request"));
    assert!(
        zenpi::core::parse_args([
            "host",
            "pair",
            "--workspace",
            "/workspace",
            "--session",
            "/session.jsonl",
            "--list-quota-intents"
        ])
        .unwrap()
        .host_pair
        .unwrap()
        .list_quota_intents
    );
    for flags in [
        vec!["--list-quota-intents"],
        vec!["--reconcile-quota", "request"],
        vec![
            "--workspace",
            "/workspace",
            "--session",
            "/session.jsonl",
            "--list-quota-intents",
            "--desktop",
        ],
        vec![
            "--workspace",
            "/workspace",
            "--session",
            "/session.jsonl",
            "--list-quota-intents",
            "--list-worktree-intents",
        ],
        vec![
            "--workspace",
            "/workspace",
            "--session",
            "/session.jsonl",
            "--reconcile-quota",
            "request",
            "--reconcile-worktree",
            "request",
            "--device",
            "invalid",
        ],
    ] {
        assert!(zenpi::core::parse_args(["host", "pair"].into_iter().chain(flags)).is_err());
    }
    let repair = zenpi::core::parse_args([
        "host",
        "pair",
        "--workspace",
        "/workspace",
        "--session",
        "/session.jsonl",
        "--reconcile-worktree",
        "request",
        "--device",
        &"a".repeat(64),
    ])
    .unwrap()
    .host_pair
    .unwrap();
    assert_eq!(repair.reconcile_worktree.as_deref(), Some("request"));
    let list = zenpi::core::parse_args([
        "host",
        "pair",
        "--workspace",
        "/workspace",
        "--session",
        "/session.jsonl",
        "--list-worktree-intents",
    ])
    .unwrap()
    .host_pair
    .unwrap();
    assert!(list.list_worktree_intents);
    for flags in [
        vec!["--list-worktree-intents"],
        vec!["--reconcile-worktree", "request"],
        vec![
            "--workspace",
            "/workspace",
            "--session",
            "/session.jsonl",
            "--list-worktree-intents",
            "--desktop",
        ],
        vec![
            "--workspace",
            "/workspace",
            "--session",
            "/session.jsonl",
            "--list-worktree-intents",
            "--list-devices",
        ],
        vec![
            "--workspace",
            "/workspace",
            "--session",
            "/session.jsonl",
            "--reconcile-worktree",
            "request",
            "--device",
            "invalid",
        ],
    ] {
        assert!(zenpi::core::parse_args(["host", "pair"].into_iter().chain(flags)).is_err());
    }
    assert!(
        zenpi::core::parse_args(["host", "pair", "--bind", "127.0.0.1:0", "--plain"])
            .unwrap()
            .host_pair
            .is_some()
    );
    assert!(
        zenpi::core::parse_args(["--headless"])
            .unwrap()
            .host_pair
            .is_none()
    );
    for arguments in [
        vec!["host"],
        vec!["host", "unknown"],
        vec!["host", "pair", "--duration-seconds", "0"],
        vec!["host", "pair", "--duration-seconds", "301"],
        vec!["host", "pair", "--advertise", "example.com"],
        vec!["host", "pair", "--auto"],
        vec!["host", "pair", "--workspace", "/tmp"],
        vec!["host", "pair", "--session", "/tmp/session.jsonl"],
        vec!["host", "pair", "--desktop"],
        vec![
            "host",
            "pair",
            "--desktop",
            "--workspace",
            "/tmp",
            "--session",
            "/tmp/session",
            "--plain",
        ],
        vec![
            "host",
            "pair",
            "--grant-device-read",
            "invalid",
            "--project",
            "p",
        ],
        vec!["host", "pair", "--project", "p"],
        vec!["host", "pair", "--grant-device-read", "invalid"],
    ] {
        assert!(zenpi::core::parse_args(arguments).is_err());
    }
    // ZE-514: pairing itself grants the mounted project; the grant CLIs are gone.
    let id = "a".repeat(64);
    for flag in [
        "--grant-device-read",
        "--grant-device-control",
        "--grant-device-approval",
        "--grant-device-control-approval",
    ] {
        assert!(zenpi::core::parse_args(["host", "pair", flag, &id, "--project", "one"]).is_err());
        assert!(zenpi::core::parse_args(["host", "pair", flag, &id]).is_err());
    }
    for flags in [vec!["--list-devices"], vec!["--revoke-device", &id]] {
        assert!(zenpi::core::parse_args([vec!["host", "pair"], flags].concat()).is_ok());
    }
}

#[test]
fn unsafe_explicit_addresses_refuse_before_identity_or_listener_creation() {
    let root = tempfile::tempdir().unwrap();
    let identity = root.path().join("identity.json");
    for (bind, advertise) in [
        ("169.254.1.2:0", None),
        ("255.255.255.255:0", None),
        ("224.0.0.1:0", None),
        ("[fe80::1]:0", None),
        ("[::ffff:169.254.1.2]:0", None),
        ("[::ffff:0.0.0.0]:0", None),
        ("0.0.0.0:0", Some("169.254.1.2")),
        ("0.0.0.0:0", Some("255.255.255.255")),
        ("[::]:0", Some("fe80::1")),
        ("[::]:0", Some("::ffff:224.0.0.1")),
        ("127.0.0.1:0", Some("192.168.1.2")),
        ("0.0.0.0:0", Some("fd00::1")),
    ] {
        let mut options = zenpi::host_pair::HostPairOptions::parse(std::iter::empty()).unwrap();
        options.identity = identity.clone();
        options.bind = bind.parse().unwrap();
        options.advertise = advertise.map(|ip| ip.parse().unwrap());
        let error = zenpi::host_pair::run(options).unwrap_err();
        assert!(error.to_string().contains("pairing"), "{bind}: {error}");
        assert!(!identity.exists());
        assert!(!identity.with_extension("devices.json").exists());
        let mut args = vec!["host", "pair", "--bind", bind];
        if let Some(ip) = advertise {
            args.extend(["--advertise", ip]);
        }
        assert!(zenpi::core::parse_args(args).is_err());
    }
    for bind in [
        "0.0.0.0:0",
        "[::]:0",
        "127.0.0.1:0",
        "[::1]:0",
        "192.168.1.2:0",
        "[fd00::1]:0",
    ] {
        assert!(zenpi::core::parse_args(["host", "pair", "--bind", bind]).is_ok());
    }
}
