use std::path::Path;
use zenpi::remote::terminal::{MAX_TERMINAL_TEXT_BYTES, screen};

#[cfg(unix)]
#[test]
fn exited_shell_retains_output_but_refuses_new_input_and_resize() {
    let root = tempfile::tempdir().unwrap();
    let mut shell = zenpi::pty_shell::PtyShell::spawn(root.path().to_path_buf()).unwrap();
    let id = shell.instance_id();
    assert!(shell.is_running());
    shell
        .try_write_input(b"printf '%s%s\\n' 'EXIT_' 'FINAL'; exit\r")
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while shell.is_running() || !shell.remote_content(64).contains("EXIT_FINAL") {
        shell.pump();
        assert!(
            std::time::Instant::now() < deadline,
            "shell exit/output not observed"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(shell.instance_id(), id);
    assert_eq!(
        shell
            .try_write_input(b"touch should-not-exist\r")
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::BrokenPipe
    );
    assert_eq!(
        shell.try_resize(24, 80).unwrap_err().kind(),
        std::io::ErrorKind::BrokenPipe
    );
    shell.write_input(b"touch should-not-exist\r");
    assert!(!root.path().join("should-not-exist").exists());
    for _ in 0..20 {
        shell.pump();
    }
    assert!(!shell.is_running());
    assert!(shell.remote_content(64).contains("EXIT_FINAL"));
}

#[test]
fn terminal_screen_has_exact_scope_and_removes_roots_secrets_and_controls() {
    let value = screen(
        "project",
        "session",
        "a",
        7,
        "/private/work/child/file\napi_key=TERMINAL_SECRET\n\u{1b}[31mvisible\0\n",
        &[Path::new("/private/work/child"), Path::new("/private/work")],
    );
    assert_eq!(value["project_id"], "project");
    assert_eq!(value["session_id"], "session");
    assert_eq!(value["subtab_revision"], 7);
    assert_eq!(value["read_only"], true);
    let text = value["text"].as_str().unwrap();
    assert!(!text.contains("/private"));
    assert!(!text.contains("TERMINAL_SECRET"));
    assert!(!text.contains('\u{1b}'));
    assert!(!text.contains('\0'));
    assert!(text.contains("visible"));
    assert_eq!(value["truncated"], false);
    assert_eq!(value.as_object().unwrap().len(), 7);
}

#[test]
fn terminal_text_cap_is_exact_and_multibyte_truncation_is_valid_utf8() {
    let value = screen("p", "s", "t", 0, &"x".repeat(MAX_TERMINAL_TEXT_BYTES), &[]);
    assert_eq!(value["truncated"], false);
    let value = screen("p", "s", "t", 0, &"界".repeat(MAX_TERMINAL_TEXT_BYTES), &[]);
    assert_eq!(value["truncated"], true);
    let text = value["text"].as_str().unwrap();
    assert!(text.len() <= MAX_TERMINAL_TEXT_BYTES);
    assert!(std::str::from_utf8(text.as_bytes()).is_ok());
}

#[cfg(unix)]
#[test]
fn real_owned_pty_output_projects_without_creating_remote_input_authority() {
    let root = tempfile::tempdir().unwrap();
    let mut shell = zenpi::pty_shell::PtyShell::spawn(root.path().to_path_buf()).unwrap();
    shell.write_input(
        b"printf '%s%s\\n' 'REMOTE_' 'PTY_SCREEN'; printf 'api_key=%s\\n' 'PTY_SECRET'\r",
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        shell.pump();
        let text = shell.content(64);
        if text.contains("REMOTE_PTY_SCREEN") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "PTY never emitted fixture result"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let value = screen(
        "project",
        "session",
        "tab",
        1,
        &shell.remote_content(64),
        &[root.path()],
    );
    assert!(
        value["text"]
            .as_str()
            .unwrap()
            .contains("REMOTE_PTY_SCREEN")
    );
    assert!(!value["text"].as_str().unwrap().contains("PTY_SECRET"));
    assert_eq!(value["read_only"], true);
}

#[cfg(unix)]
#[test]
fn actual_pty_size_follows_remote_resize_until_desktop_geometry_changes() {
    let root = tempfile::tempdir().unwrap();
    let mut shell = zenpi::pty_shell::PtyShell::spawn(root.path().to_path_buf()).unwrap();
    shell.write_input(b"printf '%s%s\\n' 'SIZE_' 'READY'\r");
    let ready = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !shell.content(64).contains("SIZE_READY") {
        shell.pump();
        assert!(std::time::Instant::now() < ready, "Slave PTY did not open");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    shell.resize(24, 80);
    assert_eq!(shell.size().unwrap(), (24, 80));
    shell.try_resize(33, 91).unwrap();
    assert_eq!(shell.size().unwrap(), (33, 91));
    shell.resize(24, 80);
    assert_eq!(shell.size().unwrap(), (33, 91));
    shell.resize(25, 82);
    assert_eq!(shell.size().unwrap(), (25, 82));
    assert!(shell.try_resize(0, 80).is_err());
    assert_eq!(shell.size().unwrap(), (25, 82));
    shell.write_input(b"stty size > actual-size.txt\r");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        shell.pump();
        if std::fs::read_to_string(root.path().join("actual-size.txt"))
            .ok()
            .as_deref()
            == Some("25 82\n")
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Slave PTY did not observe actual size"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn private_and_secret_key_assignment_lines_are_not_remote_terminal_content() {
    for line in [
        "private_key=PRIVATE_VALUE",
        "secret_key=SECRET_VALUE",
        "BEGIN PRIVATE KEY",
    ] {
        let value = screen("p", "s", "t", 0, &format!("{line}\nvisible"), &[]);
        assert_eq!(value["text"], "<sensitive terminal line withheld>\nvisible");
    }
    let value = screen(
        "p",
        "s",
        "t",
        0,
        "-----BEGIN EC PRIVATE KEY-----\nPRIVATE_BASE64\n-----END EC PRIVATE KEY-----\nvisible",
        &[],
    );
    let text = value["text"].as_str().unwrap();
    assert!(!text.contains("PRIVATE_BASE64"));
    assert!(text.ends_with("visible"));
}

#[cfg(unix)]
#[test]
fn real_pty_soft_wrapped_sensitive_line_is_reassembled_before_projection() {
    let root = tempfile::tempdir().unwrap();
    let mut shell = zenpi::pty_shell::PtyShell::spawn(root.path().to_path_buf()).unwrap();
    shell.write_input(b"printf '%509s%s%s\\n' '' 'api_key=' 'WRAPPED_PTY_SECRET'; printf '%s%s\\n' 'WRAP_' 'COMPLETE'\r");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        shell.pump();
        if shell.content(64).contains("WRAP_COMPLETE") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "PTY fixture did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let local = shell.content(64);
    assert!(local.contains("api\n_key=WRAPPED_PTY_SECRET"));
    let value = screen("p", "s", "t", 0, &shell.remote_content(64), &[root.path()]);
    let text = value["text"].as_str().unwrap();
    assert!(!text.contains("WRAPPED_PTY_SECRET"));
    assert!(text.contains("WRAP_COMPLETE"));
    assert_eq!(shell.content(64), local);
}

#[cfg(unix)]
#[test]
fn real_pty_backpressure_delivers_ordered_utf8_output_beyond_queue_capacity() {
    let root = tempfile::tempdir().unwrap();
    let mut shell = zenpi::pty_shell::PtyShell::spawn(root.path().to_path_buf()).unwrap();
    shell.write_input("awk 'BEGIN { for (i=0;i<3000;i++) printf \"ROW_%04d_%0180d_中\\n\",i,0; print \"QUEUE_\" \"FINISHED\" }'\r".as_bytes());
    std::thread::sleep(std::time::Duration::from_millis(100));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let started = std::time::Instant::now();
        shell.pump();
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "Unbounded PTY pump"
        );
        if shell.content(8).contains("QUEUE_FINISHED") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Backpressured PTY did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let text = shell.remote_content(4000);
    let rows: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("ROW_"))
        .collect();
    assert_eq!(rows.len(), 3000, "Missing or duplicate PTY output rows");
    for (index, actual) in rows.iter().enumerate() {
        let line = format!("ROW_{index:04}_{}_中", "0".repeat(180));
        assert_eq!(*actual, line, "Corrupt or reordered PTY output row");
    }
    let value = screen("p", "s", "tab", 1, &text, &[]);
    assert_eq!(value["truncated"], true);
    assert!(value["text"].as_str().unwrap().len() <= MAX_TERMINAL_TEXT_BYTES);
    let tail = screen("p", "s", "tab", 1, &shell.remote_content(64), &[]);
    assert!(tail["text"].as_str().unwrap().contains("QUEUE_FINISHED"));
}

#[cfg(unix)]
#[test]
fn real_pty_shutdown_does_not_wait_for_a_full_output_queue_to_drain() {
    let root = tempfile::tempdir().unwrap();
    let mut shell = zenpi::pty_shell::PtyShell::spawn(root.path().to_path_buf()).unwrap();
    shell.write_input(b"awk 'BEGIN { for (i=0;i<10000;i++) printf \"%0500d\\n\",i }'\r");
    std::thread::sleep(std::time::Duration::from_millis(200));
    let (done, result) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        drop(shell);
        done.send(()).unwrap();
    });
    result
        .recv_timeout(std::time::Duration::from_secs(3))
        .expect("PTY shutdown waited for consumer drain");
    worker.join().unwrap();
}
