//! ZE-309: the production TUI loop is quiet when idle and never writes the
//! project checkpoint while the user is typing.
mod support;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::{Backend, TestBackend};
use std::{
    path::Path,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};
use support::scripted::{ScriptedBackend, text};
use zenpi::{core::Agent, session::SessionStore, tui::TuiHarness};

/// No production frame places the cursor here, so a moved cursor proves the
/// loop drew a frame.
const SENTINEL: (u16, u16) = (0, 0);

fn arm(screen: &Mutex<TestBackend>) {
    let mut screen = screen.lock().unwrap();
    screen.show_cursor().unwrap();
    screen.set_cursor_position(SENTINEL).unwrap();
}

/// Count frames drawn during `window` by re-arming the sentinel after each.
fn frames_during(screen: &Mutex<TestBackend>, window: Duration) -> usize {
    arm(screen);
    let end = Instant::now() + window;
    let mut frames = 0;
    while Instant::now() < end {
        thread::sleep(Duration::from_millis(5));
        let moved = screen.lock().unwrap().get_cursor_position().unwrap() != SENTINEL.into();
        if moved {
            frames += 1;
            arm(screen);
        }
    }
    frames
}

fn wait_until(what: &str, ready: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !ready() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(10));
    }
}

fn checkpoint(home: &Path) -> Option<String> {
    std::fs::read_to_string(home.join("project-tabs.json")).ok()
}

#[test]
fn idle_loop_is_quiet_and_typing_defers_checkpoint_writes() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    // SAFETY: set before the loop or any other thread starts; this is the
    // only test in this binary.
    unsafe {
        std::env::set_var("ZENPI_HOME", home.path());
        std::env::set_var("SHELL", "/bin/sh");
    }
    let agent = Agent::new(
        SessionStore::open_in_workspace(workspace.path().join("s.jsonl"), workspace.path())
            .unwrap(),
        Box::new(ScriptedBackend::new([text("unused")])),
    );
    let screen = Arc::new(Mutex::new(TestBackend::new(120, 40)));
    let (events, receiver) = mpsc::channel();
    let harness = TuiHarness {
        events: receiver,
        screen: Arc::clone(&screen),
    };
    let tui = thread::spawn(move || zenpi::tui::run_async_with_harness(agent, harness));
    wait_until("the first frame", || {
        screen.lock().unwrap().get_cursor_position().unwrap() != SENTINEL.into()
    });
    thread::sleep(Duration::from_millis(1_500));

    // Idle: before ZE-309 the checkpoint tick redrew every 250 ms.
    let idle = frames_during(&screen, Duration::from_secs(2));
    eprintln!("ZE-309 idle frames in 2 s: {idle}");
    assert!(idle <= 1, "idle loop redrew {idle} times in 2 s");

    // Typing: every key would have marked the checkpoint dirty and written it
    // on the next 250 ms tick; writes now wait for a quiet second.
    let before = checkpoint(home.path());
    for _ in 0..15 {
        events
            .send(Event::Key(KeyEvent::new(
                KeyCode::Char('z'),
                KeyModifiers::NONE,
            )))
            .unwrap();
        thread::sleep(Duration::from_millis(100));
        assert_eq!(
            checkpoint(home.path()),
            before,
            "checkpoint written while typing"
        );
    }
    wait_until("the draft checkpoint after typing stops", || {
        checkpoint(home.path()).is_some_and(|value| value.contains("zzzzzzzzzzzzzzz"))
    });

    drop(events);
    let _ = tui.join().unwrap();
}
