//! ZE-401: binary-level headless tasks. A real `zenpi --mode headless`
//! process talks JSONL on stdio to a test client and HTTP to the scripted
//! fake provider (ZE-104).
mod support;

use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use support::scripted::{FakeServer, Reply, call, text};
use zenpi::backend::OpenAiWireApi;

struct Host {
    child: Child,
    stdin: ChildStdin,
    lines: mpsc::Receiver<Value>,
}

impl Host {
    fn start(home: &Path, cwd: &Path, server: &FakeServer, extra: &[&str]) -> Self {
        std::fs::write(
            home.join("config.toml"),
            format!(
                "backend=\"openai\"\nmodel=\"gpt-4o-mini\"\nbase_url=\"{}\"\nwire_api=\"chat\"\nrequires_openai_auth=false\nmax_retries=0\n",
                server.url
            ),
        )
        .unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_zenpi"));
        for (key, _) in std::env::vars() {
            if key.starts_with("ZENPI_") || key.starts_with("OPENAI_") {
                command.env_remove(key);
            }
        }
        let mut child = command
            .current_dir(cwd)
            .args(["--mode", "headless"])
            .args(extra)
            .env("ZENPI_HOME", home)
            .env("HOME", home)
            .env("SHELL", "/bin/sh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, lines) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Ok(value) = serde_json::from_str(&line)
                    && sender.send(value).is_err()
                {
                    break;
                }
            }
        });
        Self {
            child,
            stdin,
            lines,
        }
    }

    fn send(&mut self, frame: Value) {
        writeln!(self.stdin, "{frame}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn prompt(&mut self, id: &str, message: &str) {
        self.send(json!({"schema_version": 2, "type": "prompt", "id": id, "message": message}));
    }

    /// The next frame matching `want`, within 20 s.
    fn wait(&self, what: &str, want: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(value) if want(&value) => return value,
                Ok(_) => {}
                Err(_) => panic!("headless host never sent {what}"),
            }
        }
    }

    fn response(&self, id: &str) -> Value {
        self.wait(&format!("the response to {id}"), |value| {
            value["type"] == "response" && value["id"] == id
        })
    }

    fn event(&self, kind: &str) -> Value {
        self.wait(&format!("a {kind} event"), |value| {
            value["event"]["type"] == kind
        })
    }

    fn finish(mut self) {
        drop(self.stdin);
        let deadline = Instant::now() + Duration::from_secs(20);
        while self.child.try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "headless host did not exit");
            thread::sleep(Duration::from_millis(20));
        }
    }
}

fn dirs() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let cwd = root.path().join("repo");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&cwd).unwrap();
    let cwd = cwd.canonicalize().unwrap();
    (root, home, cwd)
}

/// Tool results the provider received in request `index`.
fn tool_messages(server: &FakeServer, index: usize) -> Vec<Value> {
    server.bodies()[index]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|message| message["role"] == "tool")
        .cloned()
        .collect()
}

#[test]
fn new_file_and_bash_output_flow_through_the_binary() {
    let (_root, home, cwd) = dirs();
    let server = FakeServer::start(
        OpenAiWireApi::ChatCompletions,
        [
            call(
                "w1",
                "write_file",
                json!({"path": "hello.txt", "content": "hi\n"}),
            ),
            call(
                "b1",
                "run_command",
                json!({"command": "cat hello.txt; echo from-bash"}),
            ),
            text("all done"),
        ],
    );
    let mut host = Host::start(&home, &cwd, &server, &["--auto"]);
    host.prompt("p1", "write then run");
    let response = host.response("p1");
    assert_eq!(response["success"], true, "{response}");
    assert_eq!(
        std::fs::read_to_string(cwd.join("hello.txt")).unwrap(),
        "hi\n"
    );
    assert_eq!(server.requests().len(), 3);
    let results = tool_messages(&server, 2);
    assert!(
        results[1]["content"]
            .as_str()
            .unwrap()
            .contains("from-bash")
    );
    host.finish();
}

#[test]
fn a_denied_write_never_lands() {
    let (_root, home, cwd) = dirs();
    let server = FakeServer::start(
        OpenAiWireApi::ChatCompletions,
        [
            call(
                "w1",
                "write_file",
                json!({"path": "denied.txt", "content": "no"}),
            ),
            text("understood"),
        ],
    );
    let mut host = Host::start(&home, &cwd, &server, &[]);
    host.prompt("p1", "write denied.txt");
    let approval = host.event("approval_request");
    host.send(json!({"schema_version": 2, "type": "approve", "id": "deny",
        "approval_id": approval["event"]["approval"]["request_id"], "decision": "deny"}));
    assert_eq!(host.response("deny")["success"], true);
    let response = host.response("p1");
    assert_eq!(response["success"], true, "{response}");
    assert!(!cwd.join("denied.txt").exists());
    assert_eq!(server.requests().len(), 2);
    host.finish();
}

#[test]
fn cancel_stops_a_running_command() {
    let (_root, home, cwd) = dirs();
    let server = FakeServer::start(
        OpenAiWireApi::ChatCompletions,
        [
            call(
                "s1",
                "run_command",
                json!({"command": ": > started; sleep 30; : > finished"}),
            ),
            text("unreachable"),
        ],
    );
    let mut host = Host::start(&home, &cwd, &server, &["--auto"]);
    host.prompt("p1", "sleep");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !cwd.join("started").exists() {
        assert!(Instant::now() < deadline, "command never started");
        thread::sleep(Duration::from_millis(10));
    }
    let begun = Instant::now();
    host.send(json!({"schema_version": 2, "type": "cancel", "id": "c1", "target_id": "p1"}));
    let response = host.response("p1");
    assert_eq!(response["success"], false, "{response}");
    assert!(begun.elapsed() < Duration::from_secs(10));
    thread::sleep(Duration::from_millis(200));
    assert!(!cwd.join("finished").exists());
    host.finish();
}

#[test]
fn a_killed_host_resumes_with_continue_and_answers_the_dangling_call() {
    let (_root, home, cwd) = dirs();
    let server = FakeServer::start(
        OpenAiWireApi::ChatCompletions,
        [
            call(
                "k1",
                "run_command",
                json!({"command": ": > started; sleep 30"}),
            ),
            Reply::Text("resumed fine".into()),
        ],
    );
    let mut host = Host::start(&home, &cwd, &server, &["--auto"]);
    host.prompt("p1", "start a long command");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !cwd.join("started").exists() {
        assert!(Instant::now() < deadline, "command never started");
        thread::sleep(Duration::from_millis(10));
    }
    // SIGKILL: no shutdown path runs.
    host.child.kill().unwrap();
    host.child.wait().unwrap();
    drop(host);

    let mut host = Host::start(&home, &cwd, &server, &["--auto", "-c"]);
    host.prompt("p2", "continue please");
    let response = host.response("p2");
    assert_eq!(response["success"], true, "{response}");
    // The resumed request carries the first prompt and a result for the
    // killed call, so a strict provider accepts the history.
    let body = server.bodies()[1].clone();
    let wire = body.to_string();
    assert!(wire.contains("start a long command"), "{wire}");
    assert!(
        tool_messages(&server, 1)
            .iter()
            .any(|message| message["tool_call_id"] == "k1"),
        "{wire}"
    );
    host.finish();
}
