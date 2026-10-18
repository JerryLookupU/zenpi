//! ZE-214: the builtin tool set advertised to the model is scoped.
mod support;

use support::scripted::{ScriptedBackend, text};
use tempfile::tempdir;
use zenpi::{
    core::{Agent, TurnInputRequest, parse_args},
    session::SessionStore,
    tools::{BuiltinSelection, SideEffectPolicy, ToolContext, ToolRegistry},
};

fn names(registry: &ToolRegistry) -> Vec<String> {
    registry
        .definitions()
        .into_iter()
        .map(|definition| definition.name)
        .collect()
}

/// Tool names the agent actually advertised in its first provider request.
fn advertised(registry: ToolRegistry) -> Vec<String> {
    let dir = tempdir().unwrap();
    let backend = ScriptedBackend::new([text("done")]);
    let mut agent = Agent::new(
        SessionStore::open(dir.path().join("session.jsonl")).unwrap(),
        Box::new(backend.clone()),
    );
    agent.set_tools(
        registry,
        ToolContext::new(dir.path()).unwrap(),
        SideEffectPolicy::all_builtins(),
    );
    agent.process(TurnInputRequest::new("hello")).unwrap();
    let requests = backend.requests();
    assert!(!requests.is_empty());
    requests[0].tools.clone()
}

#[test]
fn default_request_advertises_no_dag_tools_or_unkeyed_websearch() {
    let registry = ToolRegistry::with_builtin_selection(&BuiltinSelection::default()).unwrap();
    let tools = advertised(registry);
    assert!(!tools.iter().any(|name| name == "dag_spawn"), "{tools:?}");
    assert!(
        !tools.iter().any(|name| name.starts_with("dag_")),
        "{tools:?}"
    );
    assert!(!tools.iter().any(|name| name == "websearch"), "{tools:?}");
    assert!(tools.iter().any(|name| name == "read_file"), "{tools:?}");
}

#[test]
fn environment_without_dag_node_does_not_enable_dag_tools() {
    // This is the only test in this binary that touches the environment.
    unsafe {
        std::env::remove_var(zenpi::dag::DAG_NODE_ENV);
        std::env::remove_var(zenpi::dag::DAG_TOOLS_ENV);
    }
    let selection = BuiltinSelection::from_environment();
    assert!(!selection.dag);
    assert_eq!(selection.websearch, zenpi::tools::exa_api_key().is_some());
    let registry = ToolRegistry::with_default_builtins().unwrap();
    assert!(registry.definition("dag_spawn").is_none());

    unsafe {
        std::env::set_var(zenpi::dag::DAG_NODE_ENV, "worker-node");
    }
    let worker = BuiltinSelection::from_environment();
    unsafe {
        std::env::remove_var(zenpi::dag::DAG_NODE_ENV);
    }
    assert!(worker.dag, "DAG-spawned workers see the dag tools");
}

#[test]
fn dag_and_websearch_flags_add_their_tools() {
    let registry = ToolRegistry::with_builtin_selection(&BuiltinSelection {
        dag: true,
        websearch: true,
        only: None,
    })
    .unwrap();
    let names = names(&registry);
    for name in ["dag_spawn", "dag_send", "websearch"] {
        assert!(names.iter().any(|candidate| candidate == name), "{name}");
    }
}

#[test]
fn tools_flag_advertises_exactly_the_named_builtins() {
    let options = parse_args(["--tools", "read_file,run_command"]).unwrap();
    let only = options.tools.clone().unwrap();
    assert_eq!(
        only,
        vec!["read_file".to_string(), "run_command".to_string()]
    );
    let registry = ToolRegistry::with_builtin_selection(&BuiltinSelection {
        dag: false,
        websearch: false,
        only: Some(only),
    })
    .unwrap();
    let mut tools = advertised(registry);
    tools.sort();
    assert_eq!(tools, vec!["read_file", "run_command"]);

    assert!(parse_args(["--tools", "read_file,nope"]).is_err());
    assert!(parse_args(["--tools", ""]).is_err());
    assert!(parse_args(["--tools=read_file"]).unwrap().tools.is_some());
}

#[test]
fn builtin_name_list_matches_the_registry() {
    let mut all = names(&ToolRegistry::with_all_builtins().unwrap());
    let mut known: Vec<String> = zenpi::tools::BUILTIN_TOOL_NAMES
        .iter()
        .map(|name| name.to_string())
        .collect();
    all.sort();
    known.sort();
    assert_eq!(all, known);
}
