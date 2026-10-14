//! ZE-106: drive the production TUI loop with scripted crossterm events, a
//! `TestBackend` screen and the in-process scripted provider.
//!
//! `run_async_with_harness` and the binary's `run_async_with_remote` both
//! delegate to the same private `run_tui_loop`; only the IO handed to it
//! differs. `harness_and_binary_share_the_production_loop` pins that wiring.
mod support;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer};
use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};
use support::scripted::{ScriptedBackend, text};
use zenpi::{
    core::{Agent, TurnRole},
    session::SessionStore,
    tools::{SideEffectPolicy, ToolContext, ToolRegistry},
    tui::TuiHarness,
};

const PROMPT: &str = "hello from the harness";
const ANSWER: &str = "streamed answer from the scripted provider";

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
fn production_loop_streams_an_answer_and_exits() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    // SAFETY: set before the loop or any other thread starts; no other test
    // in this binary reads the process environment.
    unsafe {
        std::env::set_var("ZENPI_HOME", home.path());
        std::env::set_var("SHELL", "/bin/sh");
    }
    let session_path = workspace.path().join("session.jsonl");
    let provider = ScriptedBackend::new([text(ANSWER)]);
    let mut agent = Agent::new(
        SessionStore::open_in_workspace(&session_path, workspace.path()).unwrap(),
        Box::new(provider.clone()),
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

    // Keys and a bracketed paste compose one prompt.
    type_text(&events, "hello ");
    events
        .send(Event::Paste("from the harness".into()))
        .unwrap();
    let drafted = wait_for(&screen, "the drafted prompt", |buffer| {
        pane(buffer, "Prompt").is_some_and(|lines| lines.iter().any(|line| line == PROMPT))
    });
    assert!(
        drafted.is_empty(),
        "nothing is sent before Enter: {drafted:?}"
    );

    submit(&events);
    let transcript = wait_for(&screen, "the streamed answer", |buffer| {
        pane(buffer, "Conversation").is_some_and(|lines| lines.iter().any(|l| l.contains(ANSWER)))
    });
    assert_eq!(transcript, [format!("● {PROMPT}"), format!("● {ANSWER}")]);

    // A resize reflows the real frame: the next draw fills the new size.
    screen.lock().unwrap().resize(150, 44);
    events.send(Event::Resize(150, 44)).unwrap();
    let resized = wait_for(&screen, "a 150-column frame", |buffer| {
        let area = buffer.area;
        area.width == 150
            && (area.top()..area.bottom()).any(|y| {
                row(buffer, y, 0..9) == "┌ Prompt" && buffer[(area.right() - 1, y)].symbol() == "┐"
            })
            && pane(buffer, "Conversation")
                .is_some_and(|lines| lines.iter().any(|l| l.contains(ANSWER)))
    });
    assert_eq!(resized, transcript);

    type_text(&events, "/quit");
    wait_for(&screen, "the /quit draft", |buffer| {
        pane(buffer, "Prompt").is_some_and(|lines| lines.iter().any(|line| line == "/quit"))
    });
    submit(&events);
    tui.join().unwrap().unwrap();

    // One provider request carried the prompt, and the durable session holds
    // the same user/assistant pair the screen showed.
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].messages, [format!("user: {PROMPT}")]);
    let session = SessionStore::open(&session_path).unwrap();
    let turns: Vec<_> = session
        .turns()
        .iter()
        .filter(|turn| matches!(turn.role, TurnRole::User | TurnRole::Assistant))
        .map(|turn| (turn.role, turn.content.clone()))
        .collect();
    assert_eq!(
        turns,
        [
            (TurnRole::User, PROMPT.to_owned()),
            (TurnRole::Assistant, ANSWER.to_owned()),
        ]
    );
}

#[test]
fn harness_and_binary_share_the_production_loop() {
    let source = include_str!("../src/tui.rs");
    let body = |name: &str| {
        let start = source
            .find(&format!("pub fn {name}("))
            .unwrap_or_else(|| panic!("{name} missing"));
        let end = start + source[start..].find("\n}\n").unwrap();
        &source[start..end]
    };
    assert!(body("run_async_with_profile").contains("run_async_with_remote(agent, profile, None)"));
    assert!(
        body("run_async_with_remote")
            .contains("run_tui_loop(agent, profile, remote, TuiIo::Terminal)")
    );
    assert!(
        body("run_async_with_harness")
            .contains("run_tui_loop(agent, None, None, TuiIo::Harness(harness))")
    );
    assert_eq!(source.matches("fn run_tui_loop(").count(), 1);
    // The binary's TUI mode enters through run_async_with_profile.
    assert!(
        include_str!("../src/core.rs")
            .contains("RunMode::Tui => crate::tui::run_async_with_profile(agent, options.profile)")
    );
}
