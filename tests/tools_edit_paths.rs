mod support;

use serde_json::{Value, json};
use std::fs;
use support::scripted::{ScriptedBackend, call, text};
use tempfile::tempdir;
use zenpi::{
    core::{Agent, TurnInputRequest, TurnRole},
    session::SessionStore,
    tools::{
        self, SideEffectPolicy, ToolCall, ToolContext, ToolErrorCode, ToolRegistry, ToolResult,
    },
};

fn writing_agent(root: &std::path::Path, backend: ScriptedBackend) -> Agent {
    let mut agent = Agent::new(
        SessionStore::open(root.join("session.jsonl")).unwrap(),
        Box::new(backend),
    );
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(root).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    let mut approval = zenpi::approval::ApprovalPolicy::default();
    approval.remember("write_file", zenpi::approval::ApprovalDecision::Allow);
    agent.set_approval_policy(approval);
    agent
}

fn tool_turn_metadata(agent: &Agent) -> Vec<Value> {
    agent
        .history()
        .iter()
        .filter(|turn| turn.role == TurnRole::Tool)
        .map(|turn| turn.metadata.clone().unwrap())
        .collect()
}

// ZE-204: an 80 KB write is far above the old 64 KiB per-call cap but well
// within MAX_WRITE_BYTES, so it must succeed.
#[test]
fn eighty_kilobyte_write_file_succeeds() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let content = "line of generated code;\n".repeat(80 * 1024 / 24 + 1);
    assert!(content.len() > 80 * 1024);
    let backend = ScriptedBackend::new([
        call(
            "big-write",
            "write_file",
            json!({"path": "big.txt", "content": content}),
        ),
        text("done"),
    ]);
    let mut agent = writing_agent(root, backend.clone());

    let result = agent.process(TurnInputRequest::new("write it")).unwrap();
    assert_eq!(result.assistant.unwrap().content, "done");
    assert_eq!(
        std::fs::read_to_string(root.join("big.txt")).unwrap(),
        content
    );
    let metadata = tool_turn_metadata(&agent);
    assert_eq!(metadata.len(), 1);
    assert_eq!(metadata[0]["is_error"], false);
    assert_eq!(backend.requests()[1].tool_results, 1);
}

// ZE-204: an over-limit call becomes an error tool result; the turn goes on
// and the next provider request sees that result.
#[test]
fn over_limit_tool_call_returns_an_error_result_and_the_turn_continues() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let content = "x".repeat(tools::MAX_TOOL_CALL_BYTES + 1);
    let backend = ScriptedBackend::new([
        call(
            "huge-write",
            "write_file",
            json!({"path": "huge.txt", "content": content}),
        ),
        call(
            "small-write",
            "write_file",
            json!({"path": "small.txt", "content": "ok"}),
        ),
        text("recovered"),
    ]);
    let mut agent = writing_agent(root, backend.clone());

    let result = agent.process(TurnInputRequest::new("write it")).unwrap();
    assert_eq!(result.assistant.unwrap().content, "recovered");
    assert!(!root.join("huge.txt").exists());
    assert_eq!(
        std::fs::read_to_string(root.join("small.txt")).unwrap(),
        "ok"
    );
    let metadata = tool_turn_metadata(&agent);
    assert_eq!(metadata.len(), 2);
    assert_eq!(metadata[0]["is_error"], true);
    let message = metadata[0]["model_text"].as_str().unwrap();
    assert!(message.contains("the limit is"), "{message}");
    assert_eq!(metadata[1]["is_error"], false);
    // The oversized arguments are not copied into the journal.
    let assistant = agent
        .history()
        .iter()
        .find(|turn| {
            turn.metadata
                .as_ref()
                .and_then(|metadata| metadata.get("tool_calls"))
                .is_some()
        })
        .unwrap();
    let recorded = &assistant.metadata.as_ref().unwrap()["tool_calls"][0];
    assert_eq!(recorded["arguments_omitted"], true);
    assert_eq!(recorded["arguments"], json!({}));
    let requests = backend.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1].tool_results, 1);
}

fn execute(root: &std::path::Path, name: &str, arguments: Value) -> ToolResult {
    ToolRegistry::with_all_builtins().unwrap().execute(
        &ToolContext::new(root).unwrap(),
        SideEffectPolicy::all_builtins(),
        ToolCall {
            id: "call-1".into(),
            name: name.into(),
            arguments,
        },
    )
}

fn edit(root: &std::path::Path, path: &str, old: &str, new: &str) -> ToolResult {
    execute(
        root,
        "edit_file",
        json!({"path": path, "old": old, "new": new}),
    )
}

fn error_message(result: ToolResult) -> String {
    match result {
        ToolResult::Error { error, .. } => error.message,
        ToolResult::Success { output, .. } => panic!("unexpected success: {output}"),
    }
}

// ZE-205: LF-written old text matches a CRLF file (with a BOM) and the file
// keeps CRLF and its BOM.
#[test]
fn edit_crlf_file_with_lf_old_text_keeps_crlf() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("crlf.txt");
    fs::write(&path, "\u{feff}first\r\nsecond\r\nthird\r\n").unwrap();

    let result = edit(dir.path(), "crlf.txt", "first\nsecond\n", "one\ntwo\n");
    assert!(result.is_success(), "{result:?}");
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "\u{feff}one\r\ntwo\r\nthird\r\n"
    );
}

// ZE-205: an LF file is untouched outside the edited span.
#[test]
fn edit_lf_file_keeps_lf_and_other_trailing_whitespace() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("lf.txt");
    fs::write(&path, "keep  \nalpha\nbeta\n").unwrap();

    assert!(edit(dir.path(), "lf.txt", "alpha", "ALPHA").is_success());
    assert_eq!(fs::read_to_string(&path).unwrap(), "keep  \nALPHA\nbeta\n");
}

// ZE-205: curly quotes, dashes and trailing whitespace fall back to a fuzzy
// match; only the matched span changes.
#[test]
fn edit_fuzzy_match_handles_curly_quotes_and_trailing_whitespace() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("quotes.txt");
    fs::write(
        &path,
        "intro \u{201c}keep\u{201d}\nsay(\u{201c}hello\u{201d}) \u{2014} it\u{2019}s  \nnext line\t\nend\n",
    )
    .unwrap();

    let result = edit(
        dir.path(),
        "quotes.txt",
        "say(\"hello\") - it's\nnext line",
        "say(\"bye\")\nnext line",
    );
    assert!(result.is_success(), "{result:?}");
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "intro \u{201c}keep\u{201d}\nsay(\"bye\")\nnext line\t\nend\n"
    );
}

// ZE-205: uniqueness is judged after normalization.
#[test]
fn edit_rejects_matches_that_are_ambiguous_after_normalization() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ambiguous.txt");
    let original = "print(\u{201c}x\u{201d})\nprint(\"x\")\n";
    fs::write(&path, original).unwrap();

    let message = error_message(edit(dir.path(), "ambiguous.txt", "print(\"x\")", "y"));
    assert!(message.contains("found 2"), "{message}");
    let message = error_message(edit(dir.path(), "ambiguous.txt", "missing", "y"));
    assert!(message.contains("found 0"), "{message}");
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

fn path_text(path: &std::path::Path) -> String {
    path.to_str().unwrap().to_owned()
}

fn error_code(result: ToolResult) -> ToolErrorCode {
    match result {
        ToolResult::Error { error, .. } => error.code,
        ToolResult::Success { output, .. } => panic!("unexpected success: {output}"),
    }
}

// ZE-206: absolute paths inside the workspace (spelled canonically or via a
// symlinked prefix such as macOS /var -> /private/var) and `@` paths work.
#[test]
fn absolute_and_at_prefixed_workspace_paths_read_and_write() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let canonical = root.canonicalize().unwrap();
    for (index, base) in [root.to_path_buf(), canonical.clone()].iter().enumerate() {
        let file = base.join(format!("nested/abs-{index}.txt"));
        let written = execute(
            root,
            "write_file",
            json!({"path": path_text(&file), "content": "absolute\n"}),
        );
        assert!(written.is_success(), "{written:?}");
        assert_eq!(
            fs::read_to_string(canonical.join(format!("nested/abs-{index}.txt"))).unwrap(),
            "absolute\n"
        );
        let read = execute(root, "read_file", json!({"path": path_text(&file)}));
        let ToolResult::Success { output, .. } = read else {
            panic!("absolute read failed: {read:?}");
        };
        assert_eq!(output["path"], format!("nested/abs-{index}.txt"));
        assert!(
            edit(root, &path_text(&file), "absolute", "edited").is_success(),
            "absolute edit"
        );
    }
    let read = execute(root, "read_file", json!({"path": "@nested/abs-0.txt"}));
    assert!(read.is_success(), "{read:?}");
    let listed = execute(root, "list_directory", json!({"path": path_text(root)}));
    assert!(listed.is_success(), "{listed:?}");
}

// ZE-206: paths outside the workspace stay rejected, including `..` in an
// absolute path and symlinks that escape.
#[cfg(unix)]
#[test]
fn absolute_paths_outside_the_workspace_and_symlink_escapes_are_rejected() {
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let root = dir.path();
    fs::write(outside.path().join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(outside.path(), root.join("escape")).unwrap();
    std::os::unix::fs::symlink(outside.path().join("secret.txt"), root.join("link.txt")).unwrap();

    let read = |path: String| error_code(execute(root, "read_file", json!({"path": path})));
    assert_eq!(read("/etc/passwd".into()), ToolErrorCode::PathDenied);
    assert_eq!(
        read(path_text(&outside.path().join("secret.txt"))),
        ToolErrorCode::PathDenied
    );
    assert_eq!(
        read(format!("{}/../outside.txt", path_text(root))),
        ToolErrorCode::PathDenied
    );
    for escaping in [
        root.join("escape/secret.txt"),
        root.join("link.txt"),
        root.canonicalize().unwrap().join("escape/secret.txt"),
    ] {
        assert_eq!(read(path_text(&escaping)), ToolErrorCode::PathDenied);
    }
    assert_eq!(read("escape/secret.txt".into()), ToolErrorCode::PathDenied);

    let write = execute(
        root,
        "write_file",
        json!({"path": path_text(&root.join("escape/new.txt")), "content": "x"}),
    );
    assert_eq!(error_code(write), ToolErrorCode::PathDenied);
    let write = execute(
        root,
        "write_file",
        json!({"path": "/etc/zenpi-should-not-exist.txt", "content": "x"}),
    );
    assert_eq!(error_code(write), ToolErrorCode::PathDenied);
    assert!(!outside.path().join("new.txt").exists());
    assert_eq!(
        fs::read_to_string(outside.path().join("secret.txt")).unwrap(),
        "secret"
    );
}
