use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

struct Process(Child);

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn command(root: &std::path::Path) -> Command {
    let root = root.canonicalize().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_zenpi"));
    command
        .env_clear()
        .env("HOME", &root)
        .env("ZENPI_HOME", root.join("state"))
        .env("CODEX_HOME", root.join("codex"))
        .current_dir(&root)
        .args(["--mode", "headless", "--session"])
        .arg(root.join("session.jsonl"));
    command
}

#[test]
fn explicit_anonymous_config_reaches_headless_without_legacy_credentials() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    fs::create_dir(&state).unwrap();
    // Anonymous explicit connections must not even parse the legacy credential file.
    fs::write(state.join("auth.json"), "deliberately not credential JSON").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    fs::write(state.join("config.toml"), format!(
        "provider='local-test'\nmodel='test-model'\nwire_api='chat'\nbase_url='http://{address}/v1'\nauth_method='none'\nmax_retries=0\ntimeout_seconds=3\n"
    )).unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(8);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(error.to_string()),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut reader = BufReader::new(&mut stream);
        let mut headers = Vec::new();
        let mut length = 0;
        loop {
            let mut line = String::new();
            if reader
                .read_line(&mut line)
                .map_err(|error| error.to_string())?
                == 0
            {
                return Err("incomplete request headers".into());
            }
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                length = value
                    .trim()
                    .parse::<usize>()
                    .map_err(|error| error.to_string())?;
            }
            headers.push(line);
            if headers.len() > 100 {
                return Err("too many request headers".into());
            }
        }
        if length > 1024 * 1024 {
            return Err("request body too large".into());
        }
        let mut body = vec![0; length];
        reader
            .read_exact(&mut body)
            .map_err(|error| error.to_string())?;
        let body: Value = serde_json::from_slice(&body).map_err(|error| error.to_string())?;
        let reply = format!(
            "data: {}\n\ndata: [DONE]\n\n",
            json!({"choices":[{"index":0,"delta":{"role":"assistant","content":"explicit route accepted"},"finish_reason":"stop"}]})
        );
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len())
            .map_err(|error| error.to_string())?;
        Ok((headers, body))
    });
    let mut child = Process(
        command(root.path())
            .env("ZENPI_API_KEY", "synthetic-must-not-be-sent")
            .env("ZENPI_BASE_URL", "https://must-not-be-contacted.invalid")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = child.0.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let output = thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let line = line.unwrap();
            if sender
                .send(serde_json::from_str::<Value>(&line).unwrap())
                .is_err()
            {
                break;
            }
        }
    });
    writeln!(
        child.0.stdin.as_mut().unwrap(),
        "{}",
        json!({"type":"prompt","id":"explicit","text":"hello"})
    )
    .unwrap();
    let response = loop {
        let record = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(!record.to_string().contains("synthetic-must-not-be-sent"));
        if record["type"] == "response" && record["id"] == "explicit" {
            break record;
        }
    };
    writeln!(
        child.0.stdin.as_mut().unwrap(),
        "{}",
        json!({"type":"shutdown","id":"stop"})
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "headless process failed to stop");
        thread::sleep(Duration::from_millis(5));
    };
    output.join().unwrap();
    let captured = server.join().unwrap().unwrap();
    assert!(status.success());
    assert_eq!(response["success"], true, "{response}");
    assert!(response.to_string().contains("explicit route accepted"));
    assert_eq!(captured.0[0], "POST /v1/chat/completions HTTP/1.1\r\n");
    assert!(!captured.0.iter().any(|line| {
        let name = line.split(':').next().unwrap().to_ascii_lowercase();
        matches!(
            name.as_str(),
            "authorization" | "x-api-key" | "x-goog-api-key"
        )
    }));
    assert_eq!(captured.1["model"], "test-model");
    assert_eq!(
        fs::read_to_string(state.join("auth.json")).unwrap(),
        "deliberately not credential JSON"
    );
    let session = fs::read_to_string(root.path().join("session.jsonl")).unwrap();
    assert!(session.contains("explicit route accepted"));
    assert!(!session.contains("synthetic-must-not-be-sent"));
}

#[cfg(unix)]
#[test]
fn missing_explicit_credential_does_not_fall_back_or_create_session() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("state")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root.path().join("state"), fs::Permissions::from_mode(0o700)).unwrap();
    }
    fs::write(root.path().join("state/config.toml"),
        "provider='openai'\nmodel='gpt-4.1'\nwire_api='responses'\nauth_method='api_key'\nauth_ref='missing'\n").unwrap();
    let output = command(root.path())
        .env("OPENAI_API_KEY", "synthetic-no-fallback")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("auth_not_configured"), "{error}");
    assert!(!error.contains("synthetic-no-fallback"));
    assert!(!error.contains("not connected to this runtime"));
    assert!(!root.path().join("session.jsonl").exists());
}
