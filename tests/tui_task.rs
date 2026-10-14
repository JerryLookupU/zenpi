//! ZE-106: drive the production TUI loop with scripted crossterm events, a
//! `TestBackend` screen and the in-process scripted provider.
//!
//! `run_async_with_harness` and the binary's `run_async_with_remote` both
//! delegate to the same private `run_tui_loop`; only the IO handed to it
//! differs. `harness_and_binary_share_the_production_loop` pins that wiring.
mod support;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer};
use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};
use support::scripted::{Reply, ScriptedBackend, text};
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
    init_home();
    let _one = ONE_TUI.lock().unwrap_or_else(|poison| poison.into_inner());
    reset_home();
    let workspace = tempfile::tempdir().unwrap();
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

// ZE-402: task snapshots over the production loop. Every TUI below reads the
// same `ZENPI_HOME`, set once before the first loop starts.
static HOME: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();

struct Running {
    events: mpsc::Sender<Event>,
    screen: Arc<Mutex<TestBackend>>,
    provider: ScriptedBackend,
    tui: thread::JoinHandle<Result<(), zenpi::error::ZenpiError>>,
    workspace: tempfile::TempDir,
    _one: std::sync::MutexGuard<'static, ()>,
}

/// Empty the shared home so no checkpoint from an earlier TUI (whose
/// workspace is gone) reaches the next one. Call while holding `ONE_TUI`.
fn reset_home() {
    let home = HOME.get().expect("init_home first").path();
    for entry in std::fs::read_dir(home).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path).unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
        }
    }
}

fn init_home() {
    HOME.get_or_init(|| {
        let home = tempfile::tempdir().unwrap();
        // SAFETY: runs once, before this binary starts any TUI thread.
        unsafe {
            std::env::set_var("ZENPI_HOME", home.path());
            std::env::set_var("SHELL", "/bin/sh");
        }
        home
    });
}

/// One production TUI at a time per process: the loop owns process-wide
/// state (layout and project checkpoints under `ZENPI_HOME`).
static ONE_TUI: Mutex<()> = Mutex::new(());

fn start(replies: Vec<Reply>) -> Running {
    init_home();
    let one = ONE_TUI.lock().unwrap_or_else(|poison| poison.into_inner());
    reset_home();
    let workspace = tempfile::tempdir().unwrap();
    let provider = ScriptedBackend::new(replies);
    let mut agent = Agent::new(
        SessionStore::open_in_workspace(workspace.path().join("s.jsonl"), workspace.path())
            .unwrap(),
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
    Running {
        _one: one,
        events,
        screen,
        provider,
        tui,
        workspace,
    }
}

impl Running {
    fn wait_screen(&self, what: &str, ready: impl Fn(&str) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            // Release the screen before `ready`, which may read it again.
            let text = screen_text(self.screen.lock().unwrap().buffer());
            if ready(&text) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "screen never showed {what}:\n{text}"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn prompt(&self) -> Vec<String> {
        pane(self.screen.lock().unwrap().buffer(), "Prompt").unwrap_or_default()
    }

    fn status(&self) -> String {
        let screen = self.screen.lock().unwrap();
        let buffer = screen.buffer();
        row(buffer, buffer.area.bottom() - 1, 0..buffer.area.width)
    }

    fn quit(self) {
        // Empty the draft (a multi-line paste survives Ctrl-U), then quit
        // through the real command path; fail instead of hanging.
        let deadline = Instant::now() + Duration::from_secs(10);
        while !self.prompt().iter().all(|line| line.is_empty()) {
            for _ in 0..64 {
                self.events.send(key(KeyCode::Backspace)).unwrap();
            }
            settle();
            assert!(
                Instant::now() < deadline,
                "draft never emptied: {:?}",
                self.prompt()
            );
        }
        type_text(&self.events, "/quit");
        submit(&self.events);
        let deadline = Instant::now() + Duration::from_secs(10);
        while !self.tui.is_finished() {
            assert!(
                Instant::now() < deadline,
                "TUI did not quit:\n{}",
                self.status()
            );
            thread::sleep(Duration::from_millis(10));
        }
        self.tui.join().unwrap().unwrap();
    }
}

fn ctrl(c: char) -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
}

/// Let the loop drain queued events and draw.
fn settle() {
    thread::sleep(Duration::from_millis(300));
}

#[test]
fn esc_and_ctrl_c_respect_idle_draft_and_busy_states() {
    let run = start(vec![Reply::Stall, text("after interrupt")]);
    // Idle with a draft: neither key sends or discards it.
    type_text(&run.events, "keep me");
    run.wait_screen("the draft", |_| run.prompt().iter().any(|l| l == "keep me"));
    run.events.send(key(KeyCode::Esc)).unwrap();
    run.events.send(ctrl('c')).unwrap();
    settle();
    assert_eq!(run.prompt(), ["keep me"]);
    assert!(run.provider.requests().is_empty());

    // Busy: Esc interrupts the running turn.
    submit(&run.events);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !run.provider.is_stalled() {
        assert!(Instant::now() < deadline, "turn never reached the provider");
        thread::sleep(Duration::from_millis(5));
    }
    run.events.send(key(KeyCode::Esc)).unwrap();
    run.wait_screen("the interrupt", |text| text.contains("Interrupted"));
    run.quit();

    // Busy again: Ctrl-C interrupts too, and the loop stays usable.
    let run2 = start(vec![Reply::Stall]);
    type_text(&run2.events, "second");
    submit(&run2.events);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !run2.provider.is_stalled() {
        assert!(Instant::now() < deadline, "turn never reached the provider");
        thread::sleep(Duration::from_millis(5));
    }
    run2.events.send(ctrl('c')).unwrap();
    run2.wait_screen("the interrupt", |text| text.contains("Interrupted"));
    run2.quit();
}

#[test]
fn esc_with_an_overlay_open_does_not_interrupt_the_turn() {
    let run = start(vec![Reply::Stall]);
    type_text(&run.events, "long job");
    submit(&run.events);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !run.provider.is_stalled() {
        assert!(Instant::now() < deadline, "turn never reached the provider");
        thread::sleep(Duration::from_millis(5));
    }
    // Ctrl-R opens history search; Esc closes it and nothing else.
    run.events.send(ctrl('r')).unwrap();
    settle();
    run.events.send(key(KeyCode::Esc)).unwrap();
    settle();
    assert!(!run.screen_contains("Interrupted"), "{}", run.status());
    assert!(run.provider.is_stalled());
    run.events.send(ctrl('c')).unwrap();
    run.wait_screen("the interrupt", |text| text.contains("Interrupted"));
    run.quit();
}

impl Running {
    fn screen_contains(&self, needle: &str) -> bool {
        screen_text(self.screen.lock().unwrap().buffer()).contains(needle)
    }
}

#[test]
fn pasted_control_lines_and_key_releases_submit_nothing() {
    let run = start(vec![text("unused")]);
    let marker = run.workspace.path().join("keep.txt");
    std::fs::write(&marker, "x").unwrap();
    run.events
        .send(Event::Paste("look\n/exit\n!rm keep.txt".into()))
        .unwrap();
    for code in [KeyCode::Char('z'), KeyCode::Enter] {
        run.events
            .send(Event::Key(KeyEvent::new_with_kind(
                code,
                KeyModifiers::NONE,
                KeyEventKind::Release,
            )))
            .unwrap();
    }
    settle();
    assert_eq!(run.prompt(), ["look", "/exit", "!rm keep.txt"]);
    assert!(run.provider.requests().is_empty());
    assert!(marker.exists());
    assert!(!run.tui.is_finished());
    run.quit();
}

#[test]
fn a_folded_paste_submits_its_full_text() {
    let run = start(vec![text("got it")]);
    let big: String = (0..400).map(|index| format!("row{index:03} ")).collect();
    run.events.send(Event::Paste(big.clone())).unwrap();
    run.wait_screen("the folded paste", |_| {
        run.prompt()
            .iter()
            .any(|line| line.contains("[Pasted Content"))
    });
    submit(&run.events);
    run.wait_screen("the answer", |text| text.contains("got it"));
    let requests = run.provider.requests();
    assert_eq!(requests.len(), 1);
    // The model receives every pasted row, not the fold placeholder.
    let sent = &requests[0].messages[0];
    assert!(sent.starts_with("user: row000 "), "{sent}");
    assert!(sent.contains("row399"), "{sent}");
    assert!(!sent.contains("[Pasted Content"), "{sent}");
    assert_eq!(sent.trim_end(), format!("user: {}", big.trim_end()));
    run.quit();
}

#[test]
fn a_slash_command_round_trips_through_the_real_loop() {
    let run = start(vec![text("unused")]);
    type_text(&run.events, "/help");
    submit(&run.events);
    run.wait_screen("the help output", |text| {
        text.contains("/reload [selection.json]")
    });
    assert!(run.provider.requests().is_empty());
    run.quit();
}
