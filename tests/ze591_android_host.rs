#![cfg(feature = "remote")]
//! ZE-591 host fixture (not a check by itself): the production `zenpi host serve` binary with the
//! shared scripted provider (ZE-104), serving the Android instrumentation
//! `android/app/src/androidTest/java/zenpi/HostExperienceTest.kt` over `adb reverse`:
//!
//! ```sh
//! ZE591_PORT=47591 cargo test --locked --features remote --test ze591_android_host -- --ignored --nocapture
//! adb reverse tcp:47591 tcp:47591   # the invite is the stdout line starting with zenpi://pair?
//! ```
//!
//! The first prompt gets one `write_file` call that waits for the phone's approval; every later
//! provider reply is the text `ZE591_REPLY`.

mod support;

use serde_json::json;
use std::{
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use support::scripted::{FakeServer, call, text};
use zenpi::backend::OpenAiWireApi;

#[test]
#[ignore = "fixture for the ZE-591 Android instrumentation; serves until ZE591_SECONDS elapse"]
fn ze591_android_host() {
    let port = std::env::var("ZE591_PORT").unwrap_or_else(|_| "47591".into());
    let seconds: u64 =
        std::env::var("ZE591_SECONDS").map_or(900, |value| value.parse().expect("ZE591_SECONDS"));
    let provider = FakeServer::start(
        OpenAiWireApi::Responses,
        [
            call(
                "ze591-write",
                "write_file",
                json!({"path":"ze591.txt","content":"ZE591"}),
            ),
            text("ZE591_REPLY"),
        ],
    );
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().join("workspace");
    let home = temporary.path().join("home");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::create_dir(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!(
            "backend=\"openai\"\nmodel=\"gpt-4.1\"\nbase_url=\"{}\"\nwire_api=\"responses\"\nrequires_openai_auth=false\nmax_retries=0\n",
            provider.url
        ),
    )
    .unwrap();
    let workspace = workspace.canonicalize().unwrap();
    let session = temporary.path().join("session.jsonl");
    zenpi::session::SessionStore::open_in_workspace(&session, &workspace).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_zenpi"))
        .args([
            "host",
            "serve",
            "--bind",
            &format!("127.0.0.1:{port}"),
            "--identity",
        ])
        .arg(temporary.path().join("identity.json"))
        .arg("--workspace")
        .arg(&workspace)
        .arg("--session")
        .arg(&session)
        .env_clear()
        .env("HOME", &home)
        .env("ZENPI_HOME", &home)
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("host exited early: {status}");
        }
        thread::sleep(Duration::from_millis(200));
    }
    let _ = child.kill();
    let _ = child.wait();
}
