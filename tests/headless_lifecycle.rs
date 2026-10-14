#![cfg(unix)]
//! ZE-242: process lifecycle of the production headless host — SIGTERM is a
//! `shutdown`, EOF drains for a bounded time, and a session owned by another
//! live process is refused with `session_locked`.

mod support;

use std::{
    io::{BufRead, BufReader, Cursor, Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;
use tempfile::tempdir;
use zenpi::{
    backend::{Backend, BackendError, Completion, CompletionRequest, ProviderEvent},
    core::Agent,
    headless::HeadlessOptions,
    session::SessionStore,
};

/// An OpenAI-compatible endpoint that accepts each request and never answers,
/// so the child's turn stays in flight until the host cancels it.
struct StallingProvider {
    url: String,
    requests: mpsc::Receiver<()>,
    stop: Arc<AtomicBool>,
}

impl StallingProvider {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let (request_tx, requests) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        thread::spawn(move || {
            let mut held = Vec::new();
            while !stopped.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(5)))
                            .unwrap();
                        let mut seen = Vec::new();
                        let mut buffer = [0_u8; 4096];
                        while !seen.windows(4).any(|window| window == b"\r\n\r\n") {
                            match stream.read(&mut buffer) {
                                Ok(0) | Err(_) => break,
                                Ok(read) => seen.extend_from_slice(&buffer[..read]),
                            }
                        }
                        let _ = request_tx.send(());
                        held.push(stream);
                    }
                    Err(_) => thread::sleep(Duration::from_millis(5)),
                }
            }
        });
        Self {
            url,
            requests,
            stop,
        }
    }
}

impl Drop for StallingProvider {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

fn zenpi(root: &Path, session: &Path, endpoint: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_zenpi"));
    for (key, _) in std::env::vars() {
        if key.starts_with("ZENPI_") || key.starts_with("OPENAI_") {
            command.env_remove(key);
        }
    }
    command
        .current_dir(root)
        .args(["--mode", "headless", "--session"])
        .arg(session)
        .env("ZENPI_HOME", root.join("user"))
        .env("ZENPI_BACKEND", "openai")
        .env("ZENPI_PROVIDER", "openai")
        .env("ZENPI_MODEL", "gpt-4.1")
        .env("ZENPI_WIRE_API", "chat")
        .env("ZENPI_BASE_URL", endpoint)
        .env("ZENPI_API_KEY", "lifecycle-fixture-key")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// Forward every stdout line of `child` as JSON.
fn stdout_lines(child: &mut Child) -> mpsc::Receiver<Value> {
    let stdout = child.stdout.take().unwrap();
    let (send, lines) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if send.send(serde_json::from_str(&line).unwrap()).is_err() {
                break;
            }
        }
    });
    lines
}

fn wait_for_response(lines: &mpsc::Receiver<Value>, id: &str, seen: &mut Vec<Value>) -> Value {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let value = lines
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap_or_else(|_| panic!("no response for {id}; saw {seen:?}"));
        seen.push(value.clone());
        if value["type"] == "response" && value["id"] == id {
            return value;
        }
    }
}

fn wait_with_deadline(child: &mut Child, limit: Duration) -> std::process::ExitStatus {
    let deadline = Instant::now() + limit;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("zenpi did not exit within {limit:?}");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn reconnect_path(session: &Path) -> PathBuf {
    let mut path = session.as_os_str().to_owned();
    path.push(".reconnect");
    PathBuf::from(path)
}

/// Every request reserved in the transport WAL has a terminal outcome (or an
/// explicit release): nothing is left `unknown` for the next owner.
fn assert_clean_wal(session: &Path) {
    let wal = std::fs::read_to_string(reconnect_path(session)).unwrap();
    let entries: Vec<Value> = wal
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap()["event"].clone())
        .collect();
    let reserved: Vec<&str> = entries
        .iter()
        .filter(|entry| entry["type"] == "reserved")
        .filter_map(|entry| entry["id"].as_str())
        .collect();
    assert!(!reserved.is_empty(), "WAL recorded no reservation: {wal}");
    for id in reserved {
        assert!(
            entries.iter().any(|entry| {
                (entry["type"] == "terminal" || entry["type"] == "released") && entry["id"] == id
            }),
            "request {id} has no terminal WAL outcome: {wal}"
        );
    }
    assert!(
        SessionStore::open_existing(session)
            .unwrap()
            .recovery_warnings()
            .is_empty(),
        "session journal needs recovery after SIGTERM"
    );
}

#[test]
fn sigterm_mid_turn_writes_terminal_response_and_leaves_a_clean_wal() {
    let root = tempdir().unwrap();
    let session = root.path().join("sigterm.jsonl");
    let provider = StallingProvider::start();
    let mut child = zenpi(root.path(), &session, &provider.url).spawn().unwrap();
    let lines = stdout_lines(&mut child);
    let mut stdin = child.stdin.take().unwrap();
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"schema_version":2,"type":"prompt","id":"turn","text":"stall"})
    )
    .unwrap();
    stdin.flush().unwrap();
    provider
        .requests
        .recv_timeout(Duration::from_secs(60))
        .expect("provider request never arrived");

    // SAFETY: `kill` only sends a signal to our own live child process.
    assert_eq!(
        unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) },
        0
    );

    let mut seen = Vec::new();
    let turn = wait_for_response(&lines, "turn", &mut seen);
    assert_eq!(turn["success"], false, "{turn}");
    assert_eq!(turn["code"], "backend_cancelled", "{turn}");
    let shutdown = seen
        .into_iter()
        .chain(std::iter::from_fn(|| {
            lines.recv_timeout(Duration::from_secs(10)).ok()
        }))
        .find(|value| value["type"] == "response" && value["command"] == "shutdown")
        .expect("SIGTERM must be acknowledged like a shutdown request");
    assert_eq!(shutdown["success"], true, "{shutdown}");
    assert_eq!(shutdown["data"]["drained"], true, "{shutdown}");
    let status = wait_with_deadline(&mut child, Duration::from_secs(60));
    assert!(status.success(), "SIGTERM exit status: {status:?}");
    drop(stdin);
    assert_clean_wal(&session);
}

#[test]
fn second_process_on_an_owned_session_exits_with_session_locked() {
    let root = tempdir().unwrap();
    let session = root.path().join("locked.jsonl");
    let provider = StallingProvider::start();
    let mut holder = zenpi(root.path(), &session, &provider.url).spawn().unwrap();
    let lines = stdout_lines(&mut holder);
    let mut stdin = holder.stdin.take().unwrap();
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"schema_version":2,"type":"status","id":"ready"})
    )
    .unwrap();
    stdin.flush().unwrap();
    let ready = wait_for_response(&lines, "ready", &mut Vec::new());
    assert_eq!(ready["success"], true, "{ready}");

    let mut second = zenpi(root.path(), &session, &provider.url)
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    let status = wait_with_deadline(&mut second, Duration::from_secs(60));
    let mut stdout = String::new();
    second
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    let mut stderr = String::new();
    second
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(
        !status.success(),
        "second owner must fail: {stdout} {stderr}"
    );
    assert!(stderr.contains("session_locked"), "stderr: {stderr}");
    let frames: Vec<Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(frames.len(), 1, "stdout: {stdout}");
    assert_eq!(frames[0]["type"], "response");
    assert_eq!(frames[0]["success"], false);
    assert_eq!(frames[0]["code"], "session_locked");

    // The holder keeps its session and exits cleanly on EOF.
    drop(stdin);
    let status = wait_with_deadline(&mut holder, Duration::from_secs(60));
    assert!(status.success(), "holder exit status: {status:?}");
}

#[cfg(feature = "dev-fixtures")]
#[test]
fn one_shot_pipe_exits_zero_after_the_terminal_response() {
    let root = tempdir().unwrap();
    let session = root.path().join("one-shot.jsonl");
    let mut child = Command::new(env!("CARGO_BIN_EXE_zenpi"));
    for (key, _) in std::env::vars() {
        if key.starts_with("ZENPI_") || key.starts_with("OPENAI_") {
            child.env_remove(key);
        }
    }
    let mut child = child
        .current_dir(root.path())
        .args([
            "--mode",
            "headless",
            "--auto",
            "--backend",
            "echo",
            "--session",
        ])
        .arg(&session)
        .env("ZENPI_HOME", root.path().join("user"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"{\"version\":2,\"id\":\"p1\",\"type\":\"prompt\",\"message\":\"hi\"}\n")
        .unwrap();
    let status = wait_with_deadline(&mut child, Duration::from_secs(60));
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    assert!(status.success(), "one-shot exit status: {status:?}");
    let frames: Vec<Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let last = frames.last().expect("one-shot produced no output");
    assert_eq!(last["type"], "response", "{stdout}");
    assert_eq!(last["id"], "p1");
    assert_eq!(last["success"], true);
    assert_eq!(last["data"]["assistant"]["content"], "hi");
}

/// Blocks in the provider until the host cancels the request.
struct NeverFinishes;

impl Backend for NeverFinishes {
    crate::support::controlled_local_backend!();
    fn complete(&self, _: CompletionRequest<'_>) -> Result<Completion, BackendError> {
        Ok(Completion::text("unused"))
    }

    fn complete_with_control(
        &self,
        _: CompletionRequest<'_>,
        cancelled: &dyn Fn() -> bool,
        _: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
    ) -> Result<Completion, BackendError> {
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            if cancelled() {
                return Err(BackendError::Cancelled);
            }
            thread::sleep(Duration::from_millis(5));
        }
        Err(BackendError::Transport("EOF drain was not bounded".into()))
    }

    fn name(&self) -> &str {
        "never-finishes-fixture"
    }
}

#[derive(Clone, Default)]
struct SharedWriter(Arc<Mutex<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn eof_drain_deadline_cancels_a_wedged_turn_with_a_terminal_response() {
    let root = tempdir().unwrap();
    let agent = Agent::new(
        SessionStore::open(root.path().join("eof-drain.jsonl")).unwrap(),
        Box::new(NeverFinishes),
    );
    let output = SharedWriter::default();
    let captured = output.clone();
    let started = Instant::now();
    zenpi::headless::run_async_streams_with(
        agent,
        Cursor::new(
            b"{\"schema_version\":2,\"type\":\"prompt\",\"id\":\"wedged\",\"text\":\"hang\"}\n"
                .to_vec(),
        ),
        output,
        HeadlessOptions {
            eof_drain: Duration::from_millis(300),
        },
    )
    .unwrap();
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(15),
        "EOF drain took {elapsed:?}"
    );
    let output = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    let response = output
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .find(|value| value["type"] == "response" && value["id"] == "wedged")
        .unwrap_or_else(|| panic!("no terminal response: {output}"));
    assert_eq!(response["success"], false);
    assert_eq!(response["code"], "backend_cancelled");
}
