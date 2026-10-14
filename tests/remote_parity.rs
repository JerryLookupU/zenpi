#[path = "support/sequential_host.rs"]
mod sequential_host;
mod support;
use sequential_host::run_headless_sequential;

use serde_json::json;
use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use zenpi::{
    approval::{ApprovalMode, ApprovalPolicy},
    backend::{Backend, BackendError, Completion, CompletionRequest},
    core::Agent,
    project_workspace::ProjectOwnerPool,
    session::SessionStore,
    tools::{SideEffectPolicy, ToolCall, ToolContext, ToolRegistry},
    worktree::subtab::SubtabState,
};

struct RootBackend(AtomicUsize);
impl Backend for RootBackend {
    crate::support::controlled_local_backend!();
    fn name(&self) -> &str {
        "selected-root-fixture"
    }
    fn complete(&self, _: CompletionRequest<'_>) -> Result<Completion, BackendError> {
        let index = self.0.fetch_add(1, Ordering::SeqCst);
        if index % 2 == 1 {
            return Ok(Completion::text("done"));
        }
        let mut value = Completion::text("");
        value.tool_calls = vec![ToolCall {
            id: format!("write-{index}"),
            name: "write_file".into(),
            arguments: json!({"path":"result.txt","content":format!("turn-{index}")}),
        }];
        Ok(value)
    }
}
fn configured(root: &Path) -> Arc<Mutex<Agent>> {
    let mut agent = Agent::new(
        SessionStore::open_in_workspace(root.join("session.jsonl"), root).unwrap(),
        Box::new(RootBackend(AtomicUsize::new(0))),
    );
    agent.set_tools(
        ToolRegistry::with_all_builtins().unwrap(),
        ToolContext::new(root).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    agent.set_approval_policy(ApprovalPolicy {
        mode: ApprovalMode::Never,
        ..Default::default()
    });
    Arc::new(Mutex::new(agent))
}

#[test]
fn host_selection_is_request_scoped_and_restores_root_on_error_and_unwind() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    let owner = configured(&root);
    let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
    pool.restore_checkpoint().unwrap();
    let project = pool.active_context().project_id.clone();
    let original_session = pool.active_context().session_id.clone();
    let main = pool.subtab_state(&project).unwrap();
    let selected = main
        .add_directory(&project, &root, &child, "child")
        .unwrap();
    pool.commit_subtabs(&project, main.revision, selected.clone())
        .unwrap();
    let execution = pool.subtab_execution(&project).unwrap();
    let mut agent = owner.lock().unwrap();
    execution
        .run(&mut agent, |agent| {
            agent.process_sync("write in selected directory")
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(child.join("result.txt")).unwrap(),
        "turn-0"
    );
    assert!(!root.join("result.txt").exists());
    assert_eq!(agent.attachment_workspace_root(), Some(root.as_path()));
    assert_eq!(agent.session().session_id(), original_session);
    let error: Result<Result<(), &str>, _> = execution.run(&mut agent, |agent| {
        assert_eq!(agent.attachment_workspace_root(), Some(child.as_path()));
        Err("controlled failure")
    });
    assert_eq!(error.unwrap(), Err("controlled failure"));
    assert_eq!(agent.attachment_workspace_root(), Some(root.as_path()));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = execution.run(&mut agent, |_| panic!("controlled unwind"));
        }))
        .is_err()
    );
    assert_eq!(agent.attachment_workspace_root(), Some(root.as_path()));
    drop(agent);
    // Local view can change after admission; the captured request never follows that later selection.
    pool.commit_subtabs(
        &project,
        selected.revision,
        selected.select(&main.active_id).unwrap(),
    )
    .unwrap();
    let mut agent = owner.lock().unwrap();
    execution
        .run(&mut agent, |agent| {
            agent.process_sync("same captured directory")
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(child.join("result.txt")).unwrap(),
        "turn-2"
    );
    assert!(!root.join("result.txt").exists());
    pool.subtab_execution(&project)
        .unwrap()
        .run(&mut agent, |agent| agent.process_sync("main again"))
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(root.join("result.txt")).unwrap(),
        "turn-4"
    );
}

#[test]
fn changed_symlink_or_missing_directory_never_falls_back_to_primary_root() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    let outside = tempfile::tempdir().unwrap();
    let owner = configured(&root);
    let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
    pool.restore_checkpoint().unwrap();
    let project = pool.active_context().project_id.clone();
    let main = pool.subtab_state(&project).unwrap();
    pool.commit_subtabs(
        &project,
        main.revision,
        main.add_directory(&project, &root, &child, "child")
            .unwrap(),
    )
    .unwrap();
    let execution = pool.subtab_execution(&project).unwrap();
    std::fs::remove_dir(&child).unwrap();
    let mut agent = owner.lock().unwrap();
    assert!(
        execution
            .run(&mut agent, |_| panic!("missing root executed"))
            .is_err()
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.path(), &child).unwrap();
        assert!(
            execution
                .run(&mut agent, |_| panic!("replacement symlink executed"))
                .is_err()
        );
    }
    assert_eq!(agent.attachment_workspace_root(), Some(root.as_path()));
    assert!(!root.join("result.txt").exists());
    assert!(!outside.path().join("result.txt").exists());
}

#[test]
fn selected_directory_shell_uses_same_owner_and_approval_coordinator() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let child = root.join("child");
    std::fs::create_dir(&child).unwrap();
    let owner = configured(&root);
    let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
    pool.restore_checkpoint().unwrap();
    let project = pool.active_context().project_id.clone();
    let main = pool.subtab_state(&project).unwrap();
    pool.commit_subtabs(
        &project,
        main.revision,
        main.add_directory(&project, &root, &child, "child")
            .unwrap(),
    )
    .unwrap();
    let selection = pool.subtab_execution(&project).unwrap();
    let mut agent = owner.lock().unwrap();
    let session = agent.session().session_id().to_owned();
    selection
        .run(&mut agent, |agent| {
            agent.run_user_shell_with_cancel("!printf selected > shell.txt", || false)
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(child.join("shell.txt")).unwrap(),
        "selected"
    );
    assert!(!root.join("shell.txt").exists());
    assert_eq!(agent.session().session_id(), session);
    assert_eq!(agent.attachment_workspace_root(), Some(root.as_path()));
    assert!(
        agent
            .session()
            .events()
            .iter()
            .any(|event| event["type"] == "user_shell_input")
    );
}

#[test]
fn registered_git_worktree_is_selected_without_replacing_session_owner() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    for args in [
        vec!["init"],
        vec![
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "initial",
        ],
    ] {
        assert!(
            std::process::Command::new("git")
                .args(args)
                .current_dir(&root)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    let worktree = root.join(".zenpi-worktrees").join("branch");
    std::fs::create_dir(root.join(".zenpi-worktrees")).unwrap();
    zenpi::project_workspace::add_worktree(&root, &worktree, "fixture-branch").unwrap();
    let owner = configured(&root);
    let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
    pool.restore_checkpoint().unwrap();
    let project = pool.active_context().project_id.clone();
    let main = pool.subtab_state(&project).unwrap();
    let legacy = vec![
        zenpi::worktree::subtab::LegacySubtab {
            name: "main".into(),
            root: root.display().to_string(),
            kind: zenpi::worktree::subtab::SubtabKind::Main,
            concurrency: 1,
        },
        zenpi::worktree::subtab::LegacySubtab {
            name: "branch".into(),
            root: worktree.display().to_string(),
            kind: zenpi::worktree::subtab::SubtabKind::Worktree,
            concurrency: 1,
        },
    ];
    let mut state = SubtabState::migrate(&project, &root, &legacy, 1).unwrap();
    state.revision = main.revision + 1;
    pool.commit_subtabs(&project, main.revision, state).unwrap();
    let execution = pool.subtab_execution(&project).unwrap();
    let mut agent = owner.lock().unwrap();
    execution
        .run(&mut agent, |agent| agent.process_sync("write in worktree"))
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(worktree.join("result.txt")).unwrap(),
        "turn-0"
    );
    assert!(!root.join("result.txt").exists());
    assert_eq!(agent.attachment_workspace_root(), Some(root.as_path()));
}

#[test]
fn local_model_commands_cannot_change_configuration_under_durable_phone_follow_up() {
    use zenpi::{
        backend::{OpenAiCompatibleBackend, OpenAiWireApi},
        input_queue::InputKind,
        protocol::{InputQueueAction, InputQueueRequest},
        providers::registry::ModelRegistry,
        slash::SlashCommand,
        tui::{TuiState, dispatch_reasoning_input, dispatch_slash_command},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("selection.jsonl");
    let backend = OpenAiCompatibleBackend::new_with_settings(
        "http://127.0.0.1:9/v1",
        None,
        "gpt-5.2",
        OpenAiWireApi::Responses,
        Some("high".into()),
        None,
    )
    .unwrap()
    .with_model_registry("openai".into(), ModelRegistry::default())
    .unwrap();
    let mut agent = Agent::new(
        SessionStore::open_in_workspace(&path, &root).unwrap(),
        Box::new(backend),
    );
    let session = agent.session().session_id().to_owned();
    agent
        .input_queue_request(InputQueueRequest {
            schema_version: 2,
            id: "phone-follow-up".into(),
            kind: "input_queue".into(),
            session_id: session,
            input_queue: InputQueueAction::Enqueue {
                input_id: "phone-input".into(),
                kind: InputKind::FollowUp,
                text: "work admitted against the current selection".into(),
            },
        })
        .unwrap();
    let selection = agent.model_status().unwrap();
    let before = std::fs::read(&path).unwrap();
    let mut ui = TuiState::default();
    let old_ui_model = ui
        .zone_model(zenpi::view_model::Zone::Discussion)
        .map(str::to_owned);
    dispatch_slash_command(
        SlashCommand::Model {
            name: Some("gpt-5.2-2025-12-11".into()),
        },
        &mut ui,
        Some(&mut agent),
    );
    assert_eq!(
        ui.zone_model(zenpi::view_model::Zone::Discussion),
        old_ui_model.as_deref()
    );
    assert!(
        ui.messages()
            .any(|message| message.role == zenpi::tui::MessageRole::Error)
    );
    assert!(dispatch_reasoning_input(
        &mut ui,
        Some(&mut agent),
        "/reasoning low"
    ));
    assert_eq!(ui.input(), "/reasoning low");
    assert_eq!(agent.model_status().unwrap(), selection);
    assert_eq!(std::fs::read(&path).unwrap(), before);

    // Exercise the public headless parser and dispatcher, not a second setter.
    let mut wire = Vec::new();
    run_headless_sequential(agent, concat!(
        "{\"type\":\"command\",\"id\":\"local-model\",\"text\":\"/model gpt-5.2-2025-12-11\"}\n",
        "{\"type\":\"command\",\"id\":\"model-after\",\"text\":\"/model\"}\n",
        "{\"type\":\"command\",\"id\":\"reasoning-after\",\"text\":\"/reasoning\"}\n",
        "{\"type\":\"shutdown\",\"id\":\"stop\"}\n",
    ), &mut wire).unwrap();
    let records: Vec<serde_json::Value> = String::from_utf8(wire)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let rejected = records
        .iter()
        .find(|record| record["id"] == "local-model" && record["type"] == "response")
        .unwrap();
    assert_eq!(rejected["success"], false);
    assert_eq!(rejected["code"], "model_error");
    let after = |id: &str| {
        records
            .iter()
            .find(|record| record["id"] == id && record["type"] == "response")
            .unwrap()["data"]
            .clone()
    };
    assert_eq!(after("model-after")["model"], selection["model"]);
    assert_eq!(
        after("reasoning-after")["reasoning_effort"],
        selection["reasoning_effort"]
    );
    let journal = SessionStore::open_existing(&path).unwrap();
    assert!(
        !journal
            .events()
            .iter()
            .any(|event| event["type"] == "model_selected")
    );
    assert!(
        zenpi::input_queue::InputQueue::recover(&journal, Default::default())
            .unwrap()
            .has_pending()
    );
}
