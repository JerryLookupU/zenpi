//! Synthetic loopback HTTPS fixtures exercise the production explicit backend.
#![cfg(unix)]

use super::*;
use crate::auth::AllowedDestination;
use crate::auth::store::{
    CredentialKind, LockWait, Mutation, PendingCredential, RefreshResolution, RefreshTokens,
    Replacement,
};
use crate::providers::registry::ModelRegistry;
use crate::providers::{AuthHeaderPolicy, Protocol};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread;

const CA: &[u8] = include_bytes!("../../tests/support/explicit_tls/ca.der");
const CERT: &[u8] = include_bytes!("../../tests/support/explicit_tls/leaf.der");
const KEY: &[u8] = include_bytes!("../../tests/support/explicit_tls/leaf-key.der");
const CREDENTIAL: &str = "synthetic-key";

#[derive(Clone)]
struct Reply {
    status: u16,
    body: String,
    sse: bool,
    location: Option<String>,
}

impl Reply {
    fn chat(status: u16) -> Self {
        Self {
            status,
            body: format!(
                "data: {}\n\ndata: [DONE]\n\n",
                json!({"id":"fixture", "model":"test-model", "choices":[{
                    "index":0, "delta":{"content":"tls answer"}, "finish_reason":"stop"
                }]})
            ),
            sse: true,
            location: None,
        }
    }

    fn codex() -> Self {
        Self {
            status: 200,
            body: concat!(
                "data: {\"type\":\"response.output_text.delta\",\"delta\":\"tls answer\"}\n\n",
                "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"fixture\",\"model\":\"test-model\",\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n"
            ).into(),
            sse: true,
            location: None,
        }
    }
}

#[derive(Debug)]
struct Captured {
    request_line: String,
    headers: BTreeMap<String, String>,
    body: Value,
    connect_authority: Option<String>,
    server_name: Option<String>,
}

fn read_head(reader: &mut impl Read) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        if bytes.len() == 16 * 1024 {
            return Err(std::io::Error::other("fixture header limit"));
        }
        let mut byte = [0];
        reader.read_exact(&mut byte)?;
        bytes.push(byte[0]);
    }
    String::from_utf8(bytes).map_err(std::io::Error::other)
}

fn capture(reader: &mut impl Read) -> std::io::Result<Captured> {
    let header = read_head(reader)?;
    let mut lines = header.lines();
    let request_line = lines.next().unwrap_or_default().to_owned();
    let mut headers = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| std::io::Error::other("fixture header"))?;
        headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
    }
    let length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|length| *length <= 128 * 1024)
        .ok_or_else(|| std::io::Error::other("fixture content-length"))?;
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    Ok(Captured {
        request_line,
        headers,
        body: serde_json::from_slice(&bytes).map_err(std::io::Error::other)?,
        connect_authority: None,
        server_name: None,
    })
}

fn respond(writer: &mut impl Write, reply: &Reply) -> std::io::Result<()> {
    write!(
        writer,
        "HTTP/1.1 {} Synthetic\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        if reply.sse {
            "text/event-stream"
        } else {
            "application/json"
        },
        reply.body.len()
    )?;
    if let Some(location) = &reply.location {
        write!(writer, "Location: {location}\r\n")?;
    }
    write!(writer, "\r\n{}", reply.body)?;
    writer.flush()
}

struct Peer {
    address: String,
    tls: bool,
    proxy: bool,
    accepted: Arc<AtomicUsize>,
    captured: Arc<Mutex<Vec<Captured>>>,
    errors: Arc<Mutex<Vec<String>>>,
    before_reply: Arc<Mutex<Box<dyn Fn(usize) + Send>>>,
    stop: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}

impl Peer {
    fn new(tls: bool, proxy: bool, replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let accepted = Arc::new(AtomicUsize::new(0));
        let captured = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let before_reply: Arc<Mutex<Box<dyn Fn(usize) + Send>>> =
            Arc::new(Mutex::new(Box::new(|_| {})));
        let stop = Arc::new(AtomicBool::new(false));
        let (count, requests, failures, stopped) = (
            accepted.clone(),
            captured.clone(),
            errors.clone(),
            stop.clone(),
        );
        let reply_hook = before_reply.clone();
        let server = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(CERT)],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(KEY)),
        )
        .unwrap();
        let server = Arc::new(server);
        let join = thread::spawn(move || {
            while !stopped.load(Ordering::Acquire) {
                let (mut socket, _) = match listener.accept() {
                    Ok(value) => value,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => {
                        failures.lock().unwrap().push(error.to_string());
                        break;
                    }
                };
                let index = count.fetch_add(1, Ordering::AcqRel);
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let reply = replies
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| Reply::chat(500));
                let result = (|| -> std::io::Result<()> {
                    let authority = if proxy {
                        let request = read_head(&mut socket)?;
                        let first = request.lines().next().unwrap_or_default();
                        if first != "CONNECT chatgpt.com:443 HTTP/1.1" {
                            return Err(std::io::Error::other(
                                "fixture refuses any other CONNECT destination",
                            ));
                        }
                        socket.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")?;
                        socket.flush()?;
                        Some("chatgpt.com:443".into())
                    } else {
                        None
                    };
                    if tls {
                        let connection = rustls::ServerConnection::new(server.clone()).unwrap();
                        let mut stream = rustls::StreamOwned::new(connection, socket);
                        let mut request = capture(&mut stream)?;
                        request.connect_authority = authority;
                        request.server_name = stream.conn.server_name().map(str::to_owned);
                        requests.lock().unwrap().push(request);
                        reply_hook.lock().unwrap()(index);
                        respond(&mut stream, &reply)
                    } else {
                        requests.lock().unwrap().push(capture(&mut socket)?);
                        reply_hook.lock().unwrap()(index);
                        respond(&mut socket, &reply)
                    }
                })();
                if let Err(error) = result {
                    failures.lock().unwrap().push(error.to_string());
                }
            }
        });
        Self {
            address,
            tls,
            proxy,
            accepted,
            captured,
            errors,
            before_reply,
            stop,
            join: Some(join),
        }
    }

    fn origin(&self) -> String {
        format!(
            "{}://{}",
            if self.tls { "https" } else { "http" },
            self.address
        )
    }

    fn configure(&self, backend: &mut OpenAiCompatibleBackend, trust_fixture: bool) {
        let roots = if trust_fixture {
            ureq::tls::RootCerts::new_with_certs(&[ureq::tls::Certificate::from_der(CA)])
        } else {
            ureq::tls::RootCerts::WebPki
        };
        // All proxy and root changes are fixture-local; certificate verification stays enabled.
        backend.client = ureq::Agent::config_builder()
            .proxy(
                self.proxy
                    .then(|| ureq::Proxy::new(&format!("http://{}", self.address)).unwrap()),
            )
            .tls_config(ureq::tls::TlsConfig::builder().root_certs(roots).build())
            .http_status_as_error(false)
            .max_redirects(0)
            .build()
            .into();
    }

    fn assert_no_socket(&self) {
        assert_eq!(self.accepted.load(Ordering::Acquire), 0);
        assert!(self.captured.lock().unwrap().is_empty());
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.join.take().unwrap().join().unwrap();
    }
}

fn grants(origin: &str, protocol: Protocol, header: AuthHeaderPolicy) -> Vec<AllowedDestination> {
    vec![AllowedDestination {
        origin: origin.into(),
        path_prefix: "/".into(),
        protocols: vec![protocol.as_str().into()],
        headers: header.headers().iter().map(|name| (*name).into()).collect(),
    }]
}

fn pending(
    provider: &str,
    allowed_destinations: Vec<AllowedDestination>,
    oauth: bool,
) -> PendingCredential {
    PendingCredential {
        kind: if oauth {
            CredentialKind::Oauth
        } else {
            CredentialKind::ApiKey
        },
        provider: provider.into(),
        definition_version: 1,
        issuer: if oauth {
            crate::providers::codex::ISSUER.into()
        } else {
            String::new()
        },
        client_id: if oauth {
            crate::providers::codex::CLIENT_ID.into()
        } else {
            String::new()
        },
        account_id: oauth.then(|| "synthetic-account".into()),
        user_id: None,
        allowed_destinations,
        api_key: (!oauth).then(|| CREDENTIAL.into()),
        access_token: oauth.then(|| "synthetic-access-token".into()),
        refresh_token: oauth.then(|| "synthetic-refresh-token".into()),
        expires_at_ms: oauth.then(|| crate::session::unix_time_ms() + 86_400_000),
    }
}

fn stored_backend(
    peer: &Peer,
    protocol: Protocol,
    header: AuthHeaderPolicy,
    oauth: bool,
) -> (tempfile::TempDir, CredentialStore, OpenAiCompatibleBackend) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialStore::new(dir.path().canonicalize().unwrap().join("auth.json")).unwrap();
    let provider = if oauth {
        "openai-codex"
    } else {
        "synthetic-gateway"
    };
    let origin = if oauth {
        "https://chatgpt.com".into()
    } else {
        peer.origin()
    };
    store
        .modify(
            "fixture",
            None,
            Mutation::Replace(Replacement::Login(pending(
                provider,
                grants(&origin, protocol, header),
                oauth,
            ))),
            &LockWait::default(),
        )
        .unwrap();
    let connection = ProviderConnection {
        profile: "fixture".into(),
        provider: provider.into(),
        protocol,
        base_url: (!oauth).then(|| format!("{origin}/v1")),
        auth: if oauth {
            AuthBinding::CodexOAuth {
                credential_id: "fixture".into(),
            }
        } else {
            AuthBinding::StoredApiKey {
                credential_id: "fixture".into(),
            }
        },
        header_policy: (!oauth).then_some(header),
        config_revision: 1,
        model_routes: vec![],
    };
    let mut backend = OpenAiCompatibleBackend::from_connection(
        connection,
        store.clone(),
        ModelRegistry::default(),
        "test-model".into(),
        None,
        None,
        Duration::from_secs(3),
    )
    .unwrap()
    .with_max_retries(0)
    .unwrap();
    peer.configure(&mut backend, true);
    (dir, store, backend)
}

fn scope(backend: &OpenAiCompatibleBackend) -> RequestScope {
    let binding = backend.request_binding(Some("test-model")).unwrap();
    RequestScope {
        owner_id: "fixture-owner".into(),
        session_id: "fixture-session".into(),
        operation_id: "fixture-turn".into(),
        purpose: RequestPurpose::Turn,
        route_digest: binding.route_digest,
        identity_scope: binding.identity_scope,
        policy_digest: None,
        lease_id: None,
    }
}

fn run(
    backend: &OpenAiCompatibleBackend,
    scope: RequestScope,
    admission: &mut dyn FnMut(HttpRequestKind, &RequestScope) -> Result<(), BackendError>,
) -> Result<Completion, BackendError> {
    let turns = [Turn::new("fixture-user", TurnRole::User, "hello")];
    backend.complete_with_request_control(
        CompletionRequest::new("fixture-turn", &turns, Some("test-model"), &[]),
        &mut RequestControl {
            cancelled: &|| false,
            deadline: Some(Instant::now() + Duration::from_secs(5)),
            scope,
            before_send: admission,
        },
        &mut |_| Ok(()),
    )
}

#[test]
fn anonymous_loopback_sends_without_auth_or_reading_a_credential_file() {
    let peer = Peer::new(false, false, vec![Reply::chat(200)]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().canonicalize().unwrap().join("not-created.json");
    let connection = ProviderConnection {
        profile: "local".into(),
        provider: "local".into(),
        protocol: Protocol::ChatCompletions,
        base_url: Some(format!("{}/v1", peer.origin())),
        auth: AuthBinding::Anonymous,
        header_policy: None,
        config_revision: 1,
        model_routes: vec![],
    };
    let mut backend = OpenAiCompatibleBackend::from_connection(
        connection,
        CredentialStore::new(&path).unwrap(),
        ModelRegistry::default(),
        "test-model".into(),
        None,
        None,
        Duration::from_secs(3),
    )
    .unwrap();
    peer.configure(&mut backend, true);
    let mut admissions = 0;
    let result = run(&backend, scope(&backend), &mut |kind, _| {
        assert_eq!(kind, HttpRequestKind::Inference);
        admissions += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(result.content, "tls answer");
    assert_eq!(admissions, 1);
    assert!(!path.exists());
    let requests = peer.captured.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].request_line,
        "POST /v1/chat/completions HTTP/1.1"
    );
    for name in [
        "authorization",
        "x-api-key",
        "x-goog-api-key",
        "chatgpt-account-id",
    ] {
        assert!(!requests[0].headers.contains_key(name));
    }
}

#[test]
fn https_auth_header_follows_policy_not_the_chat_wire() {
    for (policy, name, expected) in [
        (
            AuthHeaderPolicy::Bearer,
            "authorization",
            "Bearer synthetic-key",
        ),
        (AuthHeaderPolicy::XApiKey, "x-api-key", CREDENTIAL),
        (AuthHeaderPolicy::GoogleApiKey, "x-goog-api-key", CREDENTIAL),
    ] {
        let peer = Peer::new(true, false, vec![Reply::chat(200)]);
        let (_dir, _store, backend) =
            stored_backend(&peer, Protocol::ChatCompletions, policy, false);
        assert_eq!(
            run(&backend, scope(&backend), &mut |_, _| Ok(()))
                .unwrap()
                .content,
            "tls answer"
        );
        let requests = peer.captured.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].headers[name], expected);
        for other in ["authorization", "x-api-key", "x-goog-api-key"]
            .into_iter()
            .filter(|other| *other != name)
        {
            assert!(!requests[0].headers.contains_key(other));
        }
        assert_eq!(
            requests[0].request_line,
            "POST /v1/chat/completions HTTP/1.1"
        );
        assert_eq!(requests[0].body["model"], "test-model");
        assert_eq!(requests[0].body["messages"][0]["content"], "hello");
        assert!(peer.errors.lock().unwrap().is_empty());
    }
}

#[test]
fn google_final_model_operation_and_bearer_override_reach_https_peer() {
    let mut reply = Reply::chat(200);
    reply.body = format!(
        "data: {}\n\n",
        json!({"responseId":"synthetic-google","modelVersion":"test-model","candidates":[{"content":{"parts":[{"text":"tls answer"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":1,"candidatesTokenCount":1,"totalTokenCount":2}})
    );
    let peer = Peer::new(true, false, vec![reply]);
    let (_dir, _store, backend) = stored_backend(
        &peer,
        Protocol::GoogleGenerativeAi,
        AuthHeaderPolicy::Bearer,
        false,
    );
    assert_eq!(
        run(&backend, scope(&backend), &mut |_, _| Ok(()))
            .unwrap()
            .content,
        "tls answer"
    );
    let requests = peer.captured.lock().unwrap();
    assert_eq!(
        requests[0].request_line,
        "POST /v1/models/test-model:streamGenerateContent?alt=sse HTTP/1.1"
    );
    assert_eq!(requests[0].headers["authorization"], "Bearer synthetic-key");
    assert!(!requests[0].headers.contains_key("x-goog-api-key"));
    assert_eq!(requests[0].body["contents"][0]["parts"][0]["text"], "hello");
}

#[test]
fn final_preflight_rejects_grant_revocation_and_relogin_before_any_socket() {
    for relogin in [false, true] {
        let peer = Peer::new(true, false, vec![]);
        let (_dir, store, backend) = stored_backend(
            &peer,
            Protocol::ChatCompletions,
            AuthHeaderPolicy::Bearer,
            false,
        );
        let mut admissions = 0;
        let result = run(&backend, scope(&backend), &mut |kind, _| {
            assert_eq!(kind, HttpRequestKind::Inference);
            admissions += 1;
            let replacement = if relogin {
                Replacement::Login(pending(
                    "synthetic-gateway",
                    grants(
                        &peer.origin(),
                        Protocol::ChatCompletions,
                        AuthHeaderPolicy::Bearer,
                    ),
                    false,
                ))
            } else {
                Replacement::AllowedDestinations(grants(
                    "https://other.test",
                    Protocol::ChatCompletions,
                    AuthHeaderPolicy::Bearer,
                ))
            };
            store
                .modify(
                    "fixture",
                    Some(1),
                    Mutation::Replace(replacement),
                    &LockWait::default(),
                )
                .unwrap();
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(admissions, 1);
        peer.assert_no_socket();
    }
}

#[test]
fn wrong_route_or_identity_scope_is_denied_before_admission() {
    let peer = Peer::new(true, false, vec![]);
    let (_dir, _store, backend) = stored_backend(
        &peer,
        Protocol::ChatCompletions,
        AuthHeaderPolicy::Bearer,
        false,
    );
    for identity in [false, true] {
        let mut request_scope = scope(&backend);
        if identity {
            request_scope.identity_scope.push_str("other");
        } else {
            request_scope.route_digest.push_str("other");
        }
        let mut admissions = 0;
        assert!(matches!(
            run(&backend, request_scope, &mut |_, _| {
                admissions += 1;
                Ok(())
            }),
            Err(BackendError::AdmissionDenied(_))
        ));
        assert_eq!(admissions, 0);
    }
    peer.assert_no_socket();
}

#[test]
fn every_https_503_retry_is_admitted_and_keeps_request_identity() {
    let peer = Peer::new(true, false, vec![Reply::chat(503), Reply::chat(200)]);
    let (_dir, _store, backend) = stored_backend(
        &peer,
        Protocol::ChatCompletions,
        AuthHeaderPolicy::Bearer,
        false,
    );
    let backend = backend.with_max_retries(2).unwrap();
    let expected = scope(&backend);
    let mut admissions = 0;
    assert_eq!(
        run(&backend, expected.clone(), &mut |kind, actual| {
            assert_eq!(kind, HttpRequestKind::Inference);
            assert_eq!(actual, &expected);
            admissions += 1;
            Ok(())
        })
        .unwrap()
        .content,
        "tls answer"
    );
    assert_eq!(admissions, 2);
    let requests = peer.captured.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].body, requests[1].body);
    assert_eq!(
        requests[0].headers["x-idempotency-key"],
        requests[1].headers["x-idempotency-key"]
    );
}

#[test]
fn https_redirect_is_not_followed() {
    let redirected = Peer::new(true, false, vec![Reply::chat(200)]);
    let mut reply = Reply::chat(307);
    reply.location = Some(format!("{}/redirected", redirected.origin()));
    let peer = Peer::new(true, false, vec![reply]);
    let (_dir, _store, backend) = stored_backend(
        &peer,
        Protocol::ChatCompletions,
        AuthHeaderPolicy::Bearer,
        false,
    );
    let backend = backend.with_max_retries(2).unwrap();
    let mut admissions = 0;
    assert!(matches!(
        run(&backend, scope(&backend), &mut |_, _| {
            admissions += 1;
            Ok(())
        }),
        Err(BackendError::HttpStatus { status: 307, .. })
    ));
    assert_eq!(admissions, 1);
    assert_eq!(peer.captured.lock().unwrap().len(), 1);
    redirected.assert_no_socket();
}

#[test]
fn stored_api_key_401_does_not_refresh_or_retry() {
    let peer = Peer::new(true, false, vec![Reply::chat(401)]);
    let (_dir, _store, backend) = stored_backend(
        &peer,
        Protocol::ChatCompletions,
        AuthHeaderPolicy::Bearer,
        false,
    );
    let backend = backend.with_max_retries(3).unwrap();
    let mut admissions = 0;
    assert!(matches!(
        run(&backend, scope(&backend), &mut |kind, _| {
            assert_eq!(kind, HttpRequestKind::Inference);
            admissions += 1;
            Ok(())
        }),
        Err(BackendError::HttpStatus { status: 401, .. })
    ));
    assert_eq!(admissions, 1);
    assert_eq!(peer.captured.lock().unwrap().len(), 1);
}

#[test]
fn codex_fixed_https_service_sends_scoped_oauth_headers_and_stream_body() {
    let peer = Peer::new(true, true, vec![Reply::codex()]);
    let (_dir, _store, backend) = stored_backend(
        &peer,
        Protocol::OpenAiCodexResponses,
        AuthHeaderPolicy::Codex,
        true,
    );
    let mut admissions = 0;
    assert_eq!(
        run(&backend, scope(&backend), &mut |kind, _| {
            assert_eq!(kind, HttpRequestKind::Inference);
            admissions += 1;
            Ok(())
        })
        .unwrap()
        .content,
        "tls answer"
    );
    assert_eq!(admissions, 1);
    let requests = peer.captured.lock().unwrap();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(
        request.connect_authority.as_deref(),
        Some("chatgpt.com:443")
    );
    assert_eq!(request.server_name.as_deref(), Some("chatgpt.com"));
    assert_eq!(
        request.request_line,
        "POST /backend-api/codex/responses HTTP/1.1"
    );
    assert_eq!(request.headers["host"], "chatgpt.com");
    assert_eq!(
        request.headers["authorization"],
        "Bearer synthetic-access-token"
    );
    assert_eq!(request.headers["chatgpt-account-id"], "synthetic-account");
    assert_eq!(request.headers["originator"], "zenpi");
    assert_eq!(
        request.headers["user-agent"],
        format!(
            "zenpi/{} ({}; {})",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    );
    assert_eq!(request.headers["accept"], "text/event-stream");
    assert_eq!(request.headers["openai-beta"], "responses=experimental");
    assert_eq!(request.body["stream"], true);
    assert_eq!(request.body["store"], false);
    assert_eq!(request.body["model"], "test-model");
    assert!(peer.errors.lock().unwrap().is_empty());
}

#[test]
fn synthetic_https_certificate_is_rejected_without_its_fixture_root() {
    let peer = Peer::new(true, false, vec![Reply::chat(200)]);
    let (_dir, _store, mut backend) = stored_backend(
        &peer,
        Protocol::ChatCompletions,
        AuthHeaderPolicy::Bearer,
        false,
    );
    peer.configure(&mut backend, false);
    assert!(run(&backend, scope(&backend), &mut |_, _| Ok(())).is_err());
    assert!(peer.accepted.load(Ordering::Acquire) > 0);
    assert!(peer.captured.lock().unwrap().is_empty());
}

#[test]
fn codex_zero_retry_401_never_attempts_token_refresh() {
    let peer = Peer::new(true, true, vec![Reply::chat(401)]);
    let (_dir, _store, backend) = stored_backend(
        &peer,
        Protocol::OpenAiCodexResponses,
        AuthHeaderPolicy::Codex,
        true,
    );
    let mut admissions = 0;
    assert!(matches!(
        run(&backend, scope(&backend), &mut |kind, _| {
            assert_eq!(
                kind,
                HttpRequestKind::Inference,
                "no token HTTP is permitted by this fixture"
            );
            admissions += 1;
            Ok(())
        }),
        Err(BackendError::HttpStatus { status: 401, .. })
    ));
    assert_eq!(admissions, 1);
    assert_eq!(peer.captured.lock().unwrap().len(), 1);
}

#[test]
fn explicit_constructor_preserves_terminal_credential_errors() {
    for (state, expected) in [
        ("revoked", "auth_revoked"),
        ("uncertain", "auth_refresh_uncertain"),
        ("login_required", "auth_login_required"),
    ] {
        let peer = Peer::new(true, true, vec![]);
        let (_dir, store, backend) = stored_backend(
            &peer,
            Protocol::OpenAiCodexResponses,
            AuthHeaderPolicy::Codex,
            true,
        );
        if state == "revoked" {
            store
                .modify("fixture", Some(1), Mutation::Revoke, &LockWait::default())
                .unwrap();
        } else {
            let guard = store.lock_refresh("fixture", &LockWait::default()).unwrap();
            let ticket = store
                .begin_refresh(
                    &guard,
                    1,
                    crate::session::unix_time_ms(),
                    &LockWait::default(),
                )
                .unwrap();
            store
                .finish_refresh(
                    &guard,
                    &ticket,
                    if state == "uncertain" {
                        RefreshResolution::Uncertain
                    } else {
                        RefreshResolution::LoginRequired
                    },
                    &LockWait::default(),
                )
                .unwrap();
        }
        let error = OpenAiCompatibleBackend::from_connection(
            backend.explicit.as_ref().unwrap().connection.clone(),
            store,
            ModelRegistry::default(),
            "test-model".into(),
            None,
            None,
            Duration::from_secs(3),
        )
        .unwrap_err();
        assert_eq!(error.code(), expected);
        assert!(!error.is_retryable());
        peer.assert_no_socket();
    }
}

#[test]
fn startup_during_peer_refresh_waits_for_commit_without_a_second_exchange() {
    let peer = Peer::new(true, true, vec![Reply::codex()]);
    let (_dir, store, old) = stored_backend(
        &peer,
        Protocol::OpenAiCodexResponses,
        AuthHeaderPolicy::Codex,
        true,
    );
    let guard = store.lock_refresh("fixture", &LockWait::default()).unwrap();
    let ticket = store
        .begin_refresh(
            &guard,
            1,
            crate::session::unix_time_ms(),
            &LockWait::default(),
        )
        .unwrap();
    let mut backend = OpenAiCompatibleBackend::from_connection(
        old.explicit.as_ref().unwrap().connection.clone(),
        store.clone(),
        ModelRegistry::default(),
        "test-model".into(),
        None,
        None,
        Duration::from_secs(3),
    )
    .unwrap();
    peer.configure(&mut backend, true);
    let refreshed = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        store
            .finish_refresh(
                &guard,
                &ticket,
                RefreshResolution::Tokens(RefreshTokens {
                    issuer: crate::providers::codex::ISSUER.into(),
                    client_id: crate::providers::codex::CLIENT_ID.into(),
                    account_id: "synthetic-account".into(),
                    user_id: None,
                    access_token: "synthetic-peer-token".into(),
                    refresh_token: None,
                    expires_at_ms: crate::session::unix_time_ms() + 3_600_000,
                }),
                &LockWait::default(),
            )
            .unwrap();
    });
    let mut admissions = 0;
    let result = run(&backend, scope(&backend), &mut |kind, _| {
        assert_eq!(
            kind,
            HttpRequestKind::Inference,
            "peer commit must avoid token HTTP"
        );
        admissions += 1;
        Ok(())
    });
    refreshed.join().unwrap();
    assert_eq!(result.unwrap().content, "tls answer");
    assert_eq!(admissions, 1);
    assert_eq!(
        peer.captured.lock().unwrap()[0].headers["authorization"],
        "Bearer synthetic-peer-token"
    );
}

#[test]
fn codex_401_reload_uses_the_new_token_and_spends_the_only_retry_slot() {
    let peer = Peer::new(true, true, vec![Reply::chat(401), Reply::chat(503)]);
    let (_dir, store, backend) = stored_backend(
        &peer,
        Protocol::OpenAiCodexResponses,
        AuthHeaderPolicy::Codex,
        true,
    );
    let before = store.identity("fixture").unwrap();
    let other_refresher = store.clone();
    *peer.before_reply.lock().unwrap() = Box::new(move |index| {
        if index != 0 {
            return;
        }
        let wait = LockWait::default();
        let guard = other_refresher.lock_refresh("fixture", &wait).unwrap();
        let ticket = other_refresher
            .begin_refresh(&guard, 1, crate::session::unix_time_ms(), &wait)
            .unwrap();
        other_refresher
            .finish_refresh(
                &guard,
                &ticket,
                RefreshResolution::Tokens(RefreshTokens {
                    issuer: crate::providers::codex::ISSUER.into(),
                    client_id: crate::providers::codex::CLIENT_ID.into(),
                    account_id: "synthetic-account".into(),
                    user_id: None,
                    access_token: "synthetic-refreshed-access".into(),
                    refresh_token: None,
                    expires_at_ms: crate::session::unix_time_ms() + 86_400_000,
                }),
                &wait,
            )
            .unwrap();
    });
    let backend = backend.with_max_retries(1).unwrap();
    let mut admissions = 0;
    assert!(matches!(
        run(&backend, scope(&backend), &mut |kind, _| {
            assert_eq!(
                kind,
                HttpRequestKind::Inference,
                "reload must not perform token HTTP"
            );
            admissions += 1;
            Ok(())
        }),
        Err(BackendError::HttpStatus { status: 503, .. })
    ));
    assert_eq!(admissions, 2);
    let requests = peer.captured.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].headers["authorization"],
        "Bearer synthetic-access-token"
    );
    assert_eq!(
        requests[1].headers["authorization"],
        "Bearer synthetic-refreshed-access"
    );
    let after = store.identity("fixture").unwrap();
    assert_eq!(after.identity_generation, before.identity_generation);
    assert!(after.credential_revision > before.credential_revision);
}
