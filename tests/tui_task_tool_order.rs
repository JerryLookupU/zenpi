//! ZE-250: text that streams after a tool call lands after the tool row and
//! never rewrites the text streamed before it (production TUI loop).
mod support;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer};
use serde_json::json;
use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};
use support::scripted::{Reply, ScriptedBackend, call, text};
use zenpi::{
    core::Agent,
    session::SessionStore,
    tools::{SideEffectPolicy, ToolContext, ToolRegistry},
    tui::TuiHarness,
};

const BEFORE: &str = "BEFORE_TOOL_TEXT";
const AFTER: &str = "AFTER_TOOL_TEXT";

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn type_text(events: &mpsc::Sender<Event>, value: &str) {
    for ch in value.chars() {
        events.send(key(KeyCode::Char(ch))).unwrap();
    }
}

/// Enter that follows keystrokes too closely is folded into the ordinary
/// paste burst as a newline, exactly as on a real terminal. Wait out that
/// window so the scripted Enter is a deliberate submit.
fn submit(events: &mpsc::Sender<Event>) {
    thread::sleep(Duration::from_millis(250));
    events.send(key(KeyCode::Enter)).unwrap();
}

fn row(buffer: &Buffer, y: u16, xs: std::ops::Range<u16>) -> String {
    xs.map(|x| buffer[(x, y)].symbol())
        .collect::<String>()
        .trim_end()
        .to_owned()
}

fn screen_text(buffer: &Buffer) -> String {
    let area = buffer.area;
    (area.top()..area.bottom())
        .map(|y| row(buffer, y, area.left()..area.right()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Inner text of the bordered pane whose title starts with `title`.
fn pane(buffer: &Buffer, title: &str) -> Option<Vec<String>> {
    let area = buffer.area;
    let width = title.chars().count() as u16 + 2;
    let (left, top) = (area.top()..area.bottom())
        .flat_map(|y| (area.left()..area.right()).map(move |x| (x, y)))
        .find(|&(x, y)| {
            x + width < area.right()
                && buffer[(x, y)].symbol() == "┌"
                && row(buffer, y, x + 1..x + width) == format!(" {title}")
        })?;
    let right = (left + 1..area.right()).find(|&x| buffer[(x, top)].symbol() == "┐")?;
    let bottom = (top + 1..area.bottom()).find(|&y| buffer[(left, y)].symbol() == "└")?;
    Some(
        (top + 1..bottom)
            .map(|y| row(buffer, y, left + 1..right))
            .filter(|line| !line.is_empty())
            .collect(),
    )
}

fn wait_for(
    screen: &Mutex<TestBackend>,
    what: &str,
    ready: impl Fn(&Buffer) -> bool,
) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        {
            let screen = screen.lock().unwrap();
            let buffer = screen.buffer();
            if ready(buffer) {
                return pane(buffer, "Conversation").unwrap_or_default();
            }
            assert!(
                Instant::now() < deadline,
                "screen never showed {what}:\n{}",
                screen_text(buffer)
            );
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn text_tool_text_keeps_both_segments_in_order() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    // SAFETY: set before the loop or any other thread starts; this binary has
    // no other test reading the process environment.
    unsafe {
        std::env::set_var("ZENPI_HOME", home.path());
        std::env::set_var("SHELL", "/bin/sh");
    }
    let session_path = workspace.path().join("session.jsonl");
    let Reply::Call(listing) = call("call-1", "list_directory", json!({"path": "."})) else {
        unreachable!()
    };
    let provider = ScriptedBackend::new([Reply::TextThenCall(BEFORE.into(), listing), text(AFTER)]);
    let mut agent = Agent::new(
        SessionStore::open_in_workspace(&session_path, workspace.path()).unwrap(),
        Box::new(provider),
    );
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(workspace.path()).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    let screen = Arc::new(Mutex::new(TestBackend::new(120, 40)));
    let (events, receiver) = mpsc::channel();
    let harness = TuiHarness {
        events: receiver,
        screen: Arc::clone(&screen),
    };
    let tui = thread::spawn(move || zenpi::tui::run_async_with_harness(agent, harness));

    type_text(&events, "list the files");
    submit(&events);
    let transcript = wait_for(&screen, "the post-tool text", |buffer| {
        pane(buffer, "Conversation").is_some_and(|lines| lines.iter().any(|l| l.contains(AFTER)))
    });
    let position = |needle: &str| {
        transcript
            .iter()
            .position(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("{needle} missing from {transcript:?}"))
    };
    let (before, tool, after) = (
        position(BEFORE),
        position("list_directory"),
        position(AFTER),
    );
    assert!(before < tool && tool < after, "order wrong: {transcript:?}");

    type_text(&events, "/quit");
    submit(&events);
    tui.join().unwrap().unwrap();
}
