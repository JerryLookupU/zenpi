mod support;

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use support::scripted::{ScriptedBackend, text};
use zenpi::{
    backend::{Backend, BackendError, Completion, CompletionRequest},
    context::ContextBudget,
    core::{Agent, AgentError, AgentEvent, TurnInputRequest},
    session::SessionStore,
    system_prompt::{MAX_CONTEXT_BYTES, TRUNCATION_MARKER},
    tools::{SideEffectPolicy, ToolContext, ToolRegistry},
};

/// Records the instructions of every request; the shared scripted backend
/// records only turns and tool names, so this wraps it to keep its replies.
#[derive(Clone)]
struct InstructionRecorder {
    inner: ScriptedBackend,
    seen: Arc<Mutex<Vec<String>>>,
}

impl InstructionRecorder {
    fn new() -> Self {
        Self {
            inner: ScriptedBackend::new([text("ok")]),
            seen: Arc::default(),
        }
    }
    fn instructions(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }
}

impl Backend for InstructionRecorder {
    support::controlled_local_backend!();
    fn name(&self) -> &str {
        "instruction-recorder"
    }
    fn complete(&self, request: CompletionRequest<'_>) -> Result<Completion, BackendError> {
        self.seen
            .lock()
            .unwrap()
            .push(request.instructions.unwrap_or_default().to_owned());
        self.inner.complete(request)
    }
}

struct Fixture {
    _temp: tempfile::TempDir,
    home: PathBuf,
    root: PathBuf,
    sub: PathBuf,
}

/// `home/` (global), `ws/` (git workspace root) and `ws/pkg/sub` (cwd).
fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    let root = base.join("ws");
    let sub = root.join("pkg/sub");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::create_dir_all(&sub).unwrap();
    Fixture {
        _temp: temp,
        home,
        root,
        sub,
    }
}

fn agent_in(cwd: &Path, home: Option<&Path>, backend: InstructionRecorder) -> Agent {
    let session = SessionStore::open(cwd.join("session.jsonl")).unwrap();
    let mut agent = Agent::new(session, Box::new(backend));
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(cwd).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    agent.set_system_prompt_home(home.map(Path::to_path_buf));
    agent
}

fn run(cwd: &Path, home: Option<&Path>) -> (String, Vec<AgentEvent>) {
    let backend = InstructionRecorder::new();
    let mut agent = agent_in(cwd, home, backend.clone());
    agent.process(TurnInputRequest::new("hello")).unwrap();
    let instructions = backend.instructions();
    assert_eq!(instructions.len(), 1);
    (instructions[0].clone(), agent.take_events())
}

fn header(path: &Path) -> String {
    format!("## {}\n", path.display())
}

fn warnings(events: &[AgentEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::Warning { message } => Some(message.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn context_files_merge_global_then_root_then_cwd_with_headers() {
    let f = fixture();
    std::fs::write(f.home.join("AGENTS.md"), "GLOBAL_RULE\n").unwrap();
    std::fs::write(f.root.join("AGENTS.md"), "ROOT_RULE\n").unwrap();
    std::fs::write(f.sub.join("AGENTS.md"), "SUB_RULE\n").unwrap();
    // Above the workspace root: never part of the merge.
    std::fs::write(f.root.parent().unwrap().join("AGENTS.md"), "OUTSIDE\n").unwrap();

    let (instructions, events) = run(&f.sub, Some(&f.home));

    let positions: Vec<usize> = [
        header(&f.home.join("AGENTS.md")),
        "GLOBAL_RULE".into(),
        header(&f.root.join("AGENTS.md")),
        "ROOT_RULE".into(),
        header(&f.sub.join("AGENTS.md")),
        "SUB_RULE".into(),
    ]
    .iter()
    .map(|needle| {
        instructions
            .find(needle.as_str())
            .unwrap_or_else(|| panic!("missing {needle:?} in {instructions}"))
    })
    .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(!instructions.contains("OUTSIDE"));
    assert!(instructions.starts_with("You are an expert coding assistant operating inside zenpi"));
    assert!(instructions.contains("# Project Context"));
    let env = format!("Environment: cwd={}; date=", f.sub.display());
    let line = instructions
        .lines()
        .find(|line| line.starts_with(&env))
        .expect("environment line");
    assert!(line.contains(" (UTC); os="));
    let date = &line[env.len()..env.len() + 10];
    assert!(date.chars().all(|c| c.is_ascii_digit() || c == '-') && date.len() == 10);
    assert!(warnings(&events).is_empty(), "{events:?}");
}

#[test]
fn claude_md_is_a_per_directory_fallback() {
    let f = fixture();
    std::fs::write(f.root.join("CLAUDE.md"), "ROOT_CLAUDE\n").unwrap();
    std::fs::write(f.sub.join("AGENTS.md"), "SUB_AGENTS\n").unwrap();
    std::fs::write(f.sub.join("CLAUDE.md"), "SUB_CLAUDE\n").unwrap();

    let (instructions, _) = run(&f.sub, None);

    assert!(instructions.contains(&header(&f.root.join("CLAUDE.md"))));
    assert!(instructions.contains("ROOT_CLAUDE"));
    assert!(instructions.contains("SUB_AGENTS"));
    assert!(!instructions.contains("SUB_CLAUDE"));
}

#[test]
fn over_cap_context_is_truncated_with_marker_and_later_files_omitted() {
    let f = fixture();
    let big = "x".repeat(MAX_CONTEXT_BYTES + 4096);
    std::fs::write(f.root.join("AGENTS.md"), &big).unwrap();
    std::fs::write(f.sub.join("AGENTS.md"), "SUB_AFTER_CAP\n").unwrap();

    let (instructions, events) = run(&f.sub, None);

    assert!(instructions.contains(TRUNCATION_MARKER));
    assert!(!instructions.contains(&big));
    assert!(instructions.contains(&"x".repeat(MAX_CONTEXT_BYTES)));
    assert!(!instructions.contains("SUB_AFTER_CAP"));
    let warnings = warnings(&events);
    assert!(
        warnings
            .iter()
            .any(|w| w.starts_with("truncated context file"))
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.starts_with("omitted context file"))
    );
}

#[test]
fn unreadable_context_file_is_skipped_with_a_warning() {
    let f = fixture();
    // A directory named AGENTS.md exists but cannot be read as a file.
    std::fs::create_dir_all(f.root.join("AGENTS.md")).unwrap();
    std::fs::write(f.root.join("CLAUDE.md"), "ROOT_FALLBACK\n").unwrap();
    std::fs::create_dir_all(f.sub.join("AGENTS.md")).unwrap();

    let (instructions, events) = run(&f.sub, None);

    assert!(!instructions.contains(&header(&f.root.join("AGENTS.md"))));
    assert!(!instructions.contains(&header(&f.sub.join("AGENTS.md"))));
    assert!(instructions.contains("ROOT_FALLBACK"));
    let warnings = warnings(&events);
    for path in [f.root.join("AGENTS.md"), f.sub.join("AGENTS.md")] {
        let expected = format!("skipped context file {}", path.display());
        assert!(
            warnings.iter().any(|w| w.starts_with(&expected)),
            "{warnings:?}"
        );
    }
}

#[cfg(unix)]
#[test]
fn permission_denied_context_file_is_skipped() {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture();
    let path = f.sub.join("AGENTS.md");
    std::fs::write(&path, "SECRET_UNREADABLE\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read(&path).is_ok() {
        return; // Privileged runners ignore mode bits.
    }
    let (instructions, events) = run(&f.sub, None);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(!instructions.contains("SECRET_UNREADABLE"));
    assert!(!instructions.contains(&header(&path)));
    assert!(warnings(&events).iter().any(|w| w.contains("AGENTS.md")));
}

#[test]
fn system_md_replaces_base_and_append_system_md_is_appended() {
    let f = fixture();
    std::fs::write(f.home.join("SYSTEM.md"), "GLOBAL_SYSTEM").unwrap();
    std::fs::write(f.home.join("APPEND_SYSTEM.md"), "GLOBAL_APPEND").unwrap();
    let (instructions, _) = run(&f.sub, Some(&f.home));
    assert!(instructions.starts_with("GLOBAL_SYSTEM\n\nGLOBAL_APPEND"));
    assert!(!instructions.contains("You are an expert coding assistant"));
    assert!(instructions.contains("Environment: cwd="));

    std::fs::create_dir_all(f.root.join(".zenpi")).unwrap();
    std::fs::write(f.root.join(".zenpi/SYSTEM.md"), "PROJECT_SYSTEM").unwrap();
    let (instructions, _) = run(&f.sub, Some(&f.home));
    assert!(instructions.starts_with("PROJECT_SYSTEM\n\nGLOBAL_APPEND"));
    assert!(!instructions.contains("GLOBAL_SYSTEM"));

    std::fs::remove_file(f.root.join(".zenpi/SYSTEM.md")).unwrap();
    std::fs::write(f.root.join(".zenpi/APPEND_SYSTEM.md"), "PROJECT_APPEND").unwrap();
    let (instructions, _) = run(&f.sub, None);
    assert!(instructions.starts_with("You are an expert coding assistant"));
    assert!(instructions.contains("\n\nPROJECT_APPEND"));
}

#[test]
fn tool_guide_states_limits_and_continuation_hints() {
    let f = fixture();
    let (instructions, _) = run(&f.sub, None);
    let line = |name: &str| {
        instructions
            .lines()
            .find(|line| line.starts_with(&format!("- {name}: ")))
            .unwrap_or_else(|| panic!("no {name} line in {instructions}"))
            .to_owned()
    };
    let read = line("read_file");
    assert!(
        read.contains("max_bytes") && read.contains("512 KiB") && read.contains("Showing lines")
    );
    assert!(read.contains("offset"));
    let output = line("read_tool_output");
    assert!(output.contains("offset += length"));
    let command = line("run_command");
    assert!(command.contains("no default") && command.contains("read_tool_output"));
    assert!(command.contains("last 2000 lines"));
    assert!(command.contains("offset"));
    assert!(line("search_text").contains("max_matches"));
    assert!(line("find").contains("cap 256"));
}

#[test]
fn instructions_count_toward_the_context_budget() {
    let f = fixture();
    let budget = ContextBudget {
        max_tokens: 12_000,
        reserved_output_tokens: 1_000,
    };

    // Base prompt alone fits the budget.
    let backend = InstructionRecorder::new();
    let mut agent = agent_in(&f.sub, None, backend.clone());
    agent.set_context_budget(budget);
    agent.process(TurnInputRequest::new("fits")).unwrap();
    assert_eq!(backend.instructions().len(), 1);

    // ~15k tokens of AGENTS.md (under the byte cap) no longer fits.
    std::fs::write(f.root.join("AGENTS.md"), "y".repeat(60 * 1024)).unwrap();
    let backend = InstructionRecorder::new();
    let mut agent = agent_in(&f.sub, None, backend.clone());
    agent.set_context_budget(budget);
    let error = agent.process(TurnInputRequest::new("too big")).unwrap_err();
    assert!(
        matches!(&error, AgentError::InvalidTurn(message) if message.contains("instructions exceed context budget")),
        "{error:?}"
    );
    assert!(backend.instructions().is_empty());
}

#[test]
fn agents_without_a_workspace_send_no_base_prompt() {
    let temp = tempfile::tempdir().unwrap();
    let backend = InstructionRecorder::new();
    let session = SessionStore::open(temp.path().join("session.jsonl")).unwrap();
    let mut agent = Agent::new(session, Box::new(backend.clone()));
    agent.process(TurnInputRequest::new("hello")).unwrap();
    assert!(!backend.instructions()[0].contains("You are an expert coding assistant"));
}
