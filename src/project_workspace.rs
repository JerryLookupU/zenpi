//! Project identities and directory selection, independent of BentoBox panes.
//!
//! A host prepares a candidate with `with_directory`, prepares its session,
//! tools and configuration for `candidate.active().cwd()`, then publishes both
//! together. The owner pool prepares sessions without changing the process cwd.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const PROJECT_WORKSPACE_SCHEMA_VERSION: u16 = 1;
/// Resource caps for retained project identities and their JSON checkpoint.
pub const MAX_PROJECT_TABS: usize = 64;
pub const MAX_PROJECT_PATH_BYTES: usize = 4096;
pub const MAX_PROJECT_WORKSPACE_BYTES: usize = 512 * 1024;

/// Stable across restarts and independent of the displayed directory name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(String);

impl ProjectId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectTab {
    id: ProjectId,
    cwd: PathBuf,
}

impl ProjectTab {
    pub fn id(&self) -> &ProjectId {
        &self.id
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// Display text only; callers must use terminal cell widths for hit tests.
    /// Identically named directories deliberately have distinct IDs.
    pub fn title(&self) -> &str {
        self.cwd
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| self.cwd.to_str().expect("validated UTF-8 cwd"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenOutcome {
    Cancelled,
    Opened(ProjectId),
    Selected(ProjectId),
}

#[derive(Debug, Error)]
pub enum ProjectWorkspaceError {
    #[error("select an existing working folder")]
    EmptyPath,
    #[error("working folder must be an existing directory")]
    NotDirectory,
    #[error(
        "working folder must be UTF-8, at most {MAX_PROJECT_PATH_BYTES} bytes, and contain no control characters"
    )]
    InvalidPath,
    #[error("relative selection requires an absolute base directory")]
    RelativeBase,
    #[error("project tab limit reached ({MAX_PROJECT_TABS})")]
    Capacity,
    #[error("project tab was not found")]
    UnknownProject,
    #[error("the last project tab cannot be closed")]
    LastProject,
    #[error("unsupported project workspace schema")]
    Schema,
    #[error("project checkpoint identity or selection is invalid")]
    InvalidCheckpoint,
    #[error("project checkpoint exceeds {MAX_PROJECT_WORKSPACE_BYTES} bytes")]
    CheckpointTooLarge,
    #[error("working folder I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("project checkpoint JSON: {0}")]
    Json(#[from] serde_json::Error),
}

/// The checkpoint contains no credentials, transcript, or layout data.
/// Deserialize only via `from_json_bytes`, which validates all identities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectWorkspace {
    schema_version: u16,
    tabs: Vec<ProjectTab>,
    active: Option<ProjectId>,
}

impl Default for ProjectWorkspace {
    fn default() -> Self {
        Self {
            schema_version: PROJECT_WORKSPACE_SCHEMA_VERSION,
            tabs: Vec::new(),
            active: None,
        }
    }
}

impl ProjectWorkspace {
    pub fn tabs(&self) -> &[ProjectTab] {
        &self.tabs
    }

    pub fn active(&self) -> Option<&ProjectTab> {
        self.active
            .as_ref()
            .and_then(|id| self.tabs.iter().find(|tab| &tab.id == id))
    }

    /// Prepare a new projection without changing the current selection. A
    /// picker passes `None` on cancellation; there is never a placeholder tab.
    /// Host context preparation may fail after this returns: discard the
    /// candidate in that case and retain the current workspace and owner.
    pub fn with_directory(
        &self,
        selected: Option<&Path>,
        base: &Path,
    ) -> Result<(Self, OpenOutcome), ProjectWorkspaceError> {
        let Some(selected) = selected else {
            return Ok((self.clone(), OpenOutcome::Cancelled));
        };
        let cwd = normalize_directory(selected, base)?;
        let id = directory_id(&cwd);
        let mut candidate = self.clone();
        let outcome = if candidate.tabs.iter().any(|tab| tab.id == id) {
            OpenOutcome::Selected(id.clone())
        } else {
            if candidate.tabs.len() >= MAX_PROJECT_TABS {
                return Err(ProjectWorkspaceError::Capacity);
            }
            candidate.tabs.push(ProjectTab {
                id: id.clone(),
                cwd,
            });
            OpenOutcome::Opened(id.clone())
        };
        candidate.active = Some(id);
        Ok((candidate, outcome))
    }

    /// Selection candidates use IDs, never names or mutable display indices.
    pub fn with_active(&self, id: &ProjectId) -> Result<Self, ProjectWorkspaceError> {
        let tab = self
            .tabs
            .iter()
            .find(|tab| &tab.id == id)
            .ok_or(ProjectWorkspaceError::UnknownProject)?;
        // Recheck availability before the host starts preparing this owner.
        if normalize_directory(&tab.cwd, &tab.cwd)? != tab.cwd {
            return Err(ProjectWorkspaceError::InvalidCheckpoint);
        }
        let mut candidate = self.clone();
        candidate.active = Some(id.clone());
        Ok(candidate)
    }

    pub fn without_project(&self, id: &ProjectId) -> Result<Self, ProjectWorkspaceError> {
        let index = self
            .tabs
            .iter()
            .position(|tab| &tab.id == id)
            .ok_or(ProjectWorkspaceError::UnknownProject)?;
        if self.tabs.len() == 1 {
            return Err(ProjectWorkspaceError::LastProject);
        }
        let mut candidate = self.clone();
        candidate.tabs.remove(index);
        if candidate.active.as_ref() == Some(id) {
            let next = index.min(candidate.tabs.len() - 1);
            candidate.active = Some(candidate.tabs[next].id.clone());
        }
        Ok(candidate)
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, ProjectWorkspaceError> {
        let bytes = serde_json::to_vec(self)?;
        if bytes.len() > MAX_PROJECT_WORKSPACE_BYTES {
            return Err(ProjectWorkspaceError::CheckpointTooLarge);
        }
        Ok(bytes)
    }

    /// Restore all-or-nothing. Missing directories, aliases, duplicate IDs,
    /// invalid selection and unsupported schemas leave the caller unchanged.
    /// Restoring does not create directories, sessions, or configuration.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, ProjectWorkspaceError> {
        if bytes.len() > MAX_PROJECT_WORKSPACE_BYTES {
            return Err(ProjectWorkspaceError::CheckpointTooLarge);
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Checkpoint {
            schema_version: u16,
            tabs: Vec<ProjectTab>,
            active: Option<ProjectId>,
        }
        let saved: Checkpoint = serde_json::from_slice(bytes)?;
        if saved.schema_version != PROJECT_WORKSPACE_SCHEMA_VERSION {
            return Err(ProjectWorkspaceError::Schema);
        }
        if saved.tabs.len() > MAX_PROJECT_TABS {
            return Err(ProjectWorkspaceError::Capacity);
        }
        let mut ids = BTreeSet::new();
        for tab in &saved.tabs {
            if !tab.cwd.is_absolute() {
                return Err(ProjectWorkspaceError::InvalidCheckpoint);
            }
            let canonical = normalize_directory(&tab.cwd, &tab.cwd)?;
            if canonical != tab.cwd || directory_id(&canonical) != tab.id || !ids.insert(&tab.id) {
                return Err(ProjectWorkspaceError::InvalidCheckpoint);
            }
        }
        if saved.tabs.is_empty() != saved.active.is_none()
            || saved.active.as_ref().is_some_and(|id| !ids.contains(id))
        {
            return Err(ProjectWorkspaceError::InvalidCheckpoint);
        }
        Ok(Self {
            schema_version: saved.schema_version,
            tabs: saved.tabs,
            active: saved.active,
        })
    }
}

/// Resolve a picker path against its explicit browsing base. Symlinks and
/// `..` refer to their actual filesystem target; process-wide cwd is untouched.
pub fn normalize_directory(selected: &Path, base: &Path) -> Result<PathBuf, ProjectWorkspaceError> {
    if selected.as_os_str().is_empty() {
        return Err(ProjectWorkspaceError::EmptyPath);
    }
    validate_path_text(selected)?;
    let path = if selected.is_absolute() {
        selected.to_path_buf()
    } else {
        if !base.is_absolute() {
            return Err(ProjectWorkspaceError::RelativeBase);
        }
        base.join(selected)
    };
    let canonical = path.canonicalize()?;
    if !canonical.is_dir() {
        return Err(ProjectWorkspaceError::NotDirectory);
    }
    validate_path_text(&canonical)?;
    Ok(canonical)
}

fn validate_path_text(path: &Path) -> Result<(), ProjectWorkspaceError> {
    let text = path.to_str().ok_or(ProjectWorkspaceError::InvalidPath)?;
    if text.len() > MAX_PROJECT_PATH_BYTES || text.chars().any(char::is_control) {
        return Err(ProjectWorkspaceError::InvalidPath);
    }
    Ok(())
}

fn directory_id(cwd: &Path) -> ProjectId {
    let mut hash = Sha256::new();
    hash.update(b"zenpi-project-workspace-v1\0");
    hash.update(cwd.to_str().expect("validated UTF-8 cwd").as_bytes());
    ProjectId(format!("{:x}", hash.finalize()))
}

/// Stable correlation stamped onto admitted work, independent of selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectContext {
    pub project_id: String,
    pub cwd: String,
    pub session_id: String,
}
impl ProjectContext {
    pub fn from_agent(project_id: &str, agent: &crate::core::Agent) -> Self {
        let cwd = agent
            .attachment_workspace_root()
            .unwrap_or_else(|| Path::new(&agent.session().header().cwd));
        // Contexts are shared between TUI/headless owners and survive a
        // restart. Keep the identity canonical so a relative session/header
        // can never make a later request resolve against process cwd.
        let cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
        Self {
            project_id: project_id.to_owned(),
            cwd: cwd.display().to_string(),
            session_id: agent.session().session_id().to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NewSessionPlan {
    remote_identity: Option<(String, String)>,
    context: ProjectContext,
    old_path: PathBuf,
    workspace: ProjectWorkspace,
    sessions: std::collections::BTreeMap<String, PathBuf>,
    checkpoint: OwnerCheckpoint,
    revision: u64,
    request_id: Option<String>,
    fingerprint: String,
    version: u16,
    #[cfg(feature = "remote")]
    remote_workspace: RemoteWorkspaceState,
    subtabs: std::collections::BTreeMap<String, crate::worktree::subtab::SubtabState>,
}

#[derive(Debug)]
pub(crate) struct NewSessionCommit {
    control: Option<OwnerControl>,
    pub(crate) context: ProjectContext,
    pub(crate) path: PathBuf,
    pub(crate) data: serde_json::Value,
    checkpoint: OwnerCheckpoint,
    revision: u64,
    #[cfg(feature = "remote")]
    remote_workspace: RemoteWorkspaceState,
}

impl NewSessionPlan {
    pub(crate) fn execute(
        self,
        agent: &mut crate::core::Agent,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<NewSessionCommit, crate::core::AgentError> {
        use crate::core::AgentError;
        if agent.session().session_id() != self.context.session_id
            || agent
                .session()
                .path()
                .canonicalize()
                .map_err(crate::session::SessionError::from)?
                != self
                    .old_path
                    .canonicalize()
                    .map_err(crate::session::SessionError::from)?
        {
            return Err(AgentError::InvalidTurn(
                "new session request belongs to an old owner".into(),
            ));
        }
        let digest = format!(
            "{:x}",
            Sha256::digest(
                format!(
                    "{}:{}:{}",
                    self.context.session_id,
                    self.request_id.as_deref().unwrap_or("tui"),
                    self.fingerprint
                )
                .as_bytes()
            )
        );
        let mut committed = None;
        agent.new_session_with_commit(&digest, cancelled, |replacement, file| {
            let path = replacement.path().to_path_buf();
            let mut context = self.context.clone();
            context.session_id = replacement.session_id().to_owned();
            let data = serde_json::json!({
                "command":"new", "route":if self.remote_identity.is_some() { "remote" } else { "local" }, "created":true,
                "project_id":context.project_id, "cwd":context.cwd,
                "old_session_id":self.context.session_id, "old_session_path":self.old_path,
                "new_session_id":context.session_id, "new_session_path":path,
            });
            file.append_event(
                replacement,
                serde_json::json!({
                    "type":"session_new_transition", "version":self.version,
                    "request_id":self.request_id, "fingerprint":self.fingerprint,
                    "project":context, "data":data,
                }),
            )?;
            let mut sessions = self.sessions;
            sessions.insert(context.project_id.clone(), path.clone());
            let revision = self
                .revision
                .checked_add(1)
                .ok_or_else(|| AgentError::InvalidTurn("workspace revision exhausted".into()))?;
            #[cfg(feature = "remote")]
            let mut remote_workspace = self.remote_workspace;
            #[cfg(feature = "remote")]
            if let Some((device, owner)) = &self.remote_identity {
                let id = self.request_id.as_deref().ok_or_else(|| {
                    AgentError::InvalidTurn("missing remote new session identity".into())
                })?;
                let receipt = remote_workspace
                    .receipts
                    .iter_mut()
                    .find(|receipt| receipt.device_id == *device && receipt.request_id == id)
                    .ok_or_else(|| {
                        AgentError::InvalidTurn("missing new session reservation".into())
                    })?;
                if receipt.fingerprint != self.fingerprint || !receipt.result.is_null() {
                    return Err(AgentError::InvalidTurn(
                        "new session reservation conflict".into(),
                    ));
                }
                receipt.result = serde_json::json!({"request_id":id,"project_id":context.project_id,
                    "old_session_id":self.context.session_id,"new_session_id":context.session_id,
                    "required_owner":owner,"workspace_revision":revision,
                    "active_project_id":self.workspace.active().map(|tab| tab.id().as_str())});
            }
            let bytes = core_checkpoint(&self.workspace, &sessions, revision, &self.subtabs)
                .map_err(AgentError::InvalidTurn)?;
            file.verify()?;
            if cancelled() {
                return Err(crate::backend::BackendError::Cancelled.into());
            }
            #[cfg(feature = "remote")]
            let remote = self
                .remote_identity
                .is_some()
                .then(|| remote_workspace.to_bytes())
                .transpose()
                .map_err(AgentError::InvalidTurn)?;
            #[cfg(not(feature = "remote"))]
            let remote: Option<Vec<u8>> = None;
            let mut checkpoint = self.checkpoint;
            checkpoint
                .commit(&bytes, remote.as_deref())
                .map_err(AgentError::InvalidTurn)?;
            committed = Some(NewSessionCommit {
                control: None,
                context,
                path,
                data,
                checkpoint,
                revision,
                #[cfg(feature = "remote")]
                remote_workspace,
            });
            Ok(())
        })?;
        let mut committed = committed.expect("successful new session committed its checkpoint");
        committed.control = Some(OwnerControl {
            project: committed.context.project_id.clone(),
            input_port: agent.input_port(),
            approval: agent.approval_coordinator(),
        });
        Ok(committed)
    }
}

/// Actual project runtimes shared by TUI and the owned JSONL host. View labels
/// never enter this map; every key comes from canonical ProjectWorkspace IDs.
/// The handles a host needs to drive one owner: its input mailbox and its
/// approval coordinator.  Both are cheap clones, and both are published when
/// the owner is created rather than discovered later through the owner's
/// execution lock.  That is what keeps a busy owner answerable -- an owner
/// blocked on an approval still holds its own `Mutex<Agent>`, so a host that
/// had to take that lock to reach the coordinator could never answer it.
#[derive(Debug, Clone)]
pub struct OwnerControl {
    /// The project this owner answers for.  Arch owners get their own key in
    /// the control table but still belong to their project, so a host
    /// attributes their requests to the right tab.
    pub project: String,
    pub input_port: crate::input_queue::InputPort,
    pub approval: Option<crate::approval::ApprovalCoordinator>,
}

/// Key for an arch owner's control entry.  Project owners are keyed by project
/// id, so arch owners need a key that cannot collide with one.
fn arch_owner_key(id: &str) -> String {
    format!("arch:{id}")
}

pub struct ProjectOwnerPool {
    workspace: ProjectWorkspace,
    owners:
        std::collections::BTreeMap<String, std::sync::Arc<std::sync::Mutex<crate::core::Agent>>>,
    /// Independent arch master-session owners (ZS1-156). They are created
    /// lazily, use a distinct journal, and are never part of the durable
    /// project checkpoint, so the discussion lane cannot adopt their history.
    arch_owners:
        std::collections::BTreeMap<String, std::sync::Arc<std::sync::Mutex<crate::core::Agent>>>,
    sessions: std::collections::BTreeMap<String, PathBuf>,
    overrides: crate::config::ConfigOverrides,
    echo_fixture: bool,
    /// ZS1-180: when the root owner runs with `--auto` (`Never`), every project
    /// and arch owner is prepared with the same auto-approval policy.
    auto_approve: bool,
    session_root: PathBuf,
    initial_session: (String, PathBuf),
    contexts: std::collections::BTreeMap<String, ProjectContext>,
    /// Control handles per project owner, kept in step with `owners`.
    control: std::collections::BTreeMap<String, OwnerControl>,
    checkpoint: Option<OwnerCheckpoint>,
    /// Bumped by every committed workspace change; kept in the project
    /// checkpoint so a build without remote state still advances it.
    revision: u64,
    #[cfg(feature = "remote")]
    remote_workspace: RemoteWorkspaceState,
    #[cfg(feature = "remote")]
    remote_owner: String,
    #[cfg(feature = "remote")]
    pending_remote_receipt: Option<WorkspaceReceipt>,
    subtabs: std::collections::BTreeMap<String, crate::worktree::subtab::SubtabState>,
    #[cfg(feature = "remote")]
    worker_owners: std::collections::BTreeMap<String, crate::worktree::worker::WorkerOwner>,
}

#[cfg(feature = "remote")]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RemoteWorkspaceState {
    receipts: Vec<WorkspaceReceipt>,
    #[serde(default)]
    cancel_receipts: Vec<CancelReceipt>,
    #[serde(default)]
    steer_receipts: Vec<CancelReceipt>,
    #[serde(default)]
    approval_receipts: Vec<ApprovalReceipt>,
    #[serde(default)]
    prompt_bindings: Vec<PromptBinding>,
    #[serde(default)]
    event_acks: Vec<EventAck>,
}

#[cfg(feature = "remote")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EventAck {
    device_id: String,
    project_id: String,
    cursor: crate::remote::protocol::EventCursor,
}
#[cfg(feature = "remote")]
impl EventAck {
    fn valid(&self) -> bool {
        [&self.device_id, &self.project_id].iter().all(|id| {
            id.len() == 64
                && id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }) && crate::remote::auth::valid_identifier(&self.cursor.session_id)
    }
}
#[cfg(feature = "remote")]
impl RemoteWorkspaceState {
    fn to_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|e| e.to_string())
    }
    /// Strict bounds and identity checks for state loaded from disk.
    fn validate(&self, revision: u64) -> Result<(), String> {
        let mut receipt_ids = BTreeSet::new();
        if self.receipts.len() > 128
            || self.receipts.iter().any(|receipt| {
                !receipt.valid(revision)
                    || !receipt_ids.insert((&receipt.device_id, &receipt.request_id))
            })
        {
            return Err("invalid remote workspace receipts".into());
        }
        for receipt in &self.receipts {
            if let Some(intent) = &receipt.quota_intent {
                let base = &intent
                    .before
                    .tabs
                    .first()
                    .ok_or("missing quota intent main tab")?
                    .root;
                intent.validate(receipt, base)?;
            }
            if let Some(intent) = &receipt.worktree_intent {
                // A later project close must not discard its deduplication
                // intent or make the entire owner checkpoint unreadable.
                let base = &intent
                    .before
                    .tabs
                    .first()
                    .ok_or("missing intent main tab")?
                    .root;
                intent.validate(receipt, base)?;
            }
        }
        if self.cancel_receipts.len() > 128
            || self.cancel_receipts.iter().any(|receipt| {
                !receipt.valid() || !receipt_ids.insert((&receipt.device_id, &receipt.request_id))
            })
        {
            return Err("invalid remote cancellation receipts".into());
        }
        if self.approval_receipts.len() > 128
            || self.approval_receipts.iter().any(|receipt| {
                !receipt.valid()
                    || !receipt_ids
                        .insert((&receipt.binding.device_id, &receipt.binding.request_id))
            })
        {
            return Err("invalid remote approval receipts".into());
        }
        if self.steer_receipts.len() > 128
            || self.steer_receipts.iter().any(|receipt| {
                !receipt.valid() || !receipt_ids.insert((&receipt.device_id, &receipt.request_id))
            })
        {
            return Err("invalid remote steering receipts".into());
        }
        if self.prompt_bindings.len() > 128
            || self.prompt_bindings.iter().any(|binding| {
                !binding.valid() || !receipt_ids.insert((&binding.device_id, &binding.request_id))
            })
        {
            return Err("invalid remote prompt bindings".into());
        }
        let mut ack_ids = std::collections::BTreeSet::new();
        if self.event_acks.len() > 128
            || self.event_acks.iter().any(|ack| {
                !ack.valid()
                    || !ack_ids.insert((&ack.device_id, &ack.project_id, &ack.cursor.session_id))
            })
        {
            return Err("invalid remote event acknowledgements".into());
        }
        Ok(())
    }
    fn import_prompt_journals(
        &mut self,
        sessions: &std::collections::BTreeMap<String, PathBuf>,
    ) -> Result<bool, String> {
        let mut bindings = self.prompt_bindings.clone();
        for (project, path) in sessions {
            let session =
                crate::session::SessionStore::open_existing(path).map_err(|e| e.to_string())?;
            for event in session.events().iter().filter(|event| {
                matches!(
                    event["type"].as_str(),
                    Some(
                        "remote_prompt_admission_v1"
                            | "remote_worker_admission_v1"
                            | "remote_model_admission_v1"
                            | "remote_reasoning_admission_v1"
                            | "remote_resources_admission_v1"
                            | "remote_terminal_admission_v1"
                            | "remote_session_name_admission_v1"
                    )
                )
            }) {
                let binding = PromptBinding {
                    operation: if event["type"] == "remote_model_admission_v1" {
                        crate::remote::protocol::Operation::ModelSelect
                    } else if event["type"] == "remote_reasoning_admission_v1" {
                        crate::remote::protocol::Operation::ReasoningSelect
                    } else if event["type"] == "remote_resources_admission_v1" {
                        crate::remote::protocol::Operation::ResourcesReload
                    } else if event["type"] == "remote_session_name_admission_v1" {
                        crate::remote::protocol::Operation::SessionRename
                    } else if event["type"] == "remote_terminal_admission_v1" {
                        serde_json::from_value(event["operation"].clone())
                            .map_err(|_| "invalid terminal operation")?
                    } else {
                        crate::remote::protocol::Operation::Prompt
                    },
                    device_id: event["device_id"]
                        .as_str()
                        .ok_or("invalid legacy prompt device")?
                        .into(),
                    request_id: event["request_id"]
                        .as_str()
                        .ok_or("invalid legacy prompt ID")?
                        .into(),
                    fingerprint: event["fingerprint"]
                        .as_str()
                        .ok_or("invalid legacy prompt fingerprint")?
                        .into(),
                    project_id: project.clone(),
                    session_id: session.session_id().into(),
                };
                if !binding.valid() {
                    return Err("invalid legacy prompt binding".into());
                }
                if let Some(old) = bindings.iter().find(|old| {
                    old.device_id == binding.device_id && old.request_id == binding.request_id
                }) {
                    if old != &binding {
                        return Err("legacy prompt binding conflict".into());
                    }
                } else {
                    let collision = self.receipts.iter().any(|r| {
                        r.device_id == binding.device_id && r.request_id == binding.request_id
                    }) || self.cancel_receipts.iter().any(|r| {
                        r.device_id == binding.device_id && r.request_id == binding.request_id
                    }) || self.approval_receipts.iter().any(|r| {
                        r.binding.device_id == binding.device_id
                            && r.binding.request_id == binding.request_id
                    }) || self.steer_receipts.iter().any(|r| {
                        r.device_id == binding.device_id && r.request_id == binding.request_id
                    });
                    if bindings.len() >= 128 || collision {
                        return Err("legacy prompt identity conflict or capacity".into());
                    }
                    bindings.push(binding);
                }
            }
        }
        let changed = bindings != self.prompt_bindings;
        self.prompt_bindings = bindings;
        Ok(changed)
    }
}
#[cfg(feature = "remote")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PromptBinding {
    #[serde(
        default = "prompt_operation",
        skip_serializing_if = "is_prompt_operation"
    )]
    operation: crate::remote::protocol::Operation,
    device_id: String,
    request_id: String,
    fingerprint: String,
    project_id: String,
    session_id: String,
}
#[cfg(feature = "remote")]
fn prompt_operation() -> crate::remote::protocol::Operation {
    crate::remote::protocol::Operation::Prompt
}
#[cfg(feature = "remote")]
fn is_prompt_operation(operation: &crate::remote::protocol::Operation) -> bool {
    *operation == prompt_operation()
}
#[cfg(feature = "remote")]
impl PromptBinding {
    fn valid(&self) -> bool {
        use crate::remote::auth::valid_identifier;
        let hex = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        matches!(
            self.operation,
            crate::remote::protocol::Operation::Prompt
                | crate::remote::protocol::Operation::ModelSelect
                | crate::remote::protocol::Operation::ReasoningSelect
                | crate::remote::protocol::Operation::ResourcesReload
                | crate::remote::protocol::Operation::SessionRename
                | crate::remote::protocol::Operation::TerminalOpen
                | crate::remote::protocol::Operation::TerminalInput
                | crate::remote::protocol::Operation::TerminalResize
        ) && hex(&self.device_id)
            && hex(&self.fingerprint)
            && hex(&self.project_id)
            && valid_identifier(&self.request_id)
            && valid_identifier(&self.session_id)
    }
}
#[cfg(feature = "remote")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalReceipt {
    binding: CancelReceipt,
    digest: String,
    decision: crate::approval::ApprovalDecision,
}
#[cfg(feature = "remote")]
impl ApprovalReceipt {
    fn valid(&self) -> bool {
        self.binding.valid()
            && self.digest.len() == 64
            && self
                .digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
    fn result(&self) -> serde_json::Value {
        let mut result = self.binding.result();
        if result.is_object() {
            result["digest"] = serde_json::json!(self.digest);
            result["decision"] = serde_json::json!(self.decision);
        }
        result
    }
}
#[cfg(feature = "remote")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CancelReceipt {
    device_id: String,
    request_id: String,
    fingerprint: String,
    project_id: String,
    session_id: String,
    target_request_id: String,
    state: crate::remote::protocol::ExecutionState,
    code: Option<crate::remote::protocol::ErrorCode>,
}
#[cfg(feature = "remote")]
impl CancelReceipt {
    fn valid(&self) -> bool {
        use crate::remote::{auth::valid_identifier, protocol::ExecutionState};
        let hex = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
        };
        hex(&self.device_id)
            && hex(&self.fingerprint)
            && hex(&self.project_id)
            && valid_identifier(&self.request_id)
            && valid_identifier(&self.session_id)
            && valid_identifier(&self.target_request_id)
            && match self.state {
                ExecutionState::UnknownOutcome | ExecutionState::Accepted => self.code.is_none(),
                ExecutionState::Failed => {
                    self.code == Some(crate::remote::protocol::ErrorCode::Busy)
                }
                _ => false,
            }
    }
    fn result(&self) -> serde_json::Value {
        if self.state == crate::remote::protocol::ExecutionState::Accepted {
            serde_json::json!({"request_id":self.request_id,"target_request_id":self.target_request_id,
                "project_id":self.project_id,"session_id":self.session_id})
        } else {
            serde_json::Value::Null
        }
    }
}
#[cfg(feature = "remote")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceReceipt {
    device_id: String,
    request_id: String,
    fingerprint: String,
    result: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    worktree_intent: Option<WorktreeIntent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    quota_intent: Option<QuotaIntent>,
}

#[cfg(feature = "remote")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct QuotaIntent {
    request: crate::remote::protocol::Envelope,
    before: crate::worktree::subtab::SubtabState,
    candidate: crate::worktree::subtab::SubtabState,
}
#[cfg(feature = "remote")]
impl QuotaIntent {
    fn validate(&self, receipt: &WorkspaceReceipt, base: &Path) -> Result<(), String> {
        use crate::remote::protocol::{Message, Operation};
        let request = &self.request;
        request.validate().map_err(|error| error.to_string())?;
        let project = request
            .project_id
            .as_deref()
            .ok_or("missing quota intent project")?;
        let hex = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if request.id.as_deref() != Some(receipt.request_id.as_str())
            || format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(request).map_err(|error| error.to_string())?)
            ) != receipt.fingerprint
            || directory_id(base).as_str() != project
            || request.expected_revision.is_none()
            || !request.required_owner.as_deref().is_some_and(hex)
            || request.session_id.is_none()
            || request.turn_id.is_some()
            || request.sequence.is_some()
        {
            return Err("invalid quota intent identity or owner binding".into());
        }
        self.before.validate(project, base)?;
        let Message::Request {
            operation: Operation::SetConcurrency,
            arguments,
        } = &request.payload
        else {
            return Err("invalid quota intent command".into());
        };
        let args = arguments
            .as_object()
            .ok_or("invalid quota intent arguments")?;
        if args.len() != 3
            || args
                .get("subtab_revision")
                .and_then(serde_json::Value::as_u64)
                != Some(self.before.revision)
        {
            return Err("invalid quota intent revision or fields".into());
        }
        let id = args
            .get("subtab_id")
            .and_then(serde_json::Value::as_str)
            .ok_or("missing quota subtab")?;
        let count = args
            .get("concurrency")
            .and_then(serde_json::Value::as_u64)
            .filter(|value| (1..=64).contains(value))
            .ok_or("invalid quota intent limit")?;
        let mut expected = self.before.set_concurrency(id, count as u16)?;
        expected.revision = self
            .before
            .revision
            .checked_add(1)
            .ok_or("quota intent revision exhausted")?;
        if expected != self.candidate {
            return Err("quota intent candidate differs from command".into());
        }
        Ok(())
    }
}

#[cfg(feature = "remote")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorktreeIntent {
    request: crate::remote::protocol::Envelope,
    before: crate::worktree::subtab::SubtabState,
    candidate: crate::worktree::subtab::SubtabState,
    path: PathBuf,
    branch: Option<String>,
}
#[cfg(feature = "remote")]
impl WorktreeIntent {
    fn validate(&self, receipt: &WorkspaceReceipt, base: &Path) -> Result<(), String> {
        use crate::remote::protocol::{Message, Operation};
        let request = &self.request;
        request.validate().map_err(|e| e.to_string())?;
        let project = request
            .project_id
            .as_deref()
            .ok_or("missing Git intent project")?;
        if request.id.as_deref() != Some(receipt.request_id.as_str())
            || format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(request).map_err(|e| e.to_string())?)
            ) != receipt.fingerprint
        {
            return Err("Git intent identity disagrees with reservation".into());
        }
        self.before.validate(project, base)?;
        if directory_id(base).as_str() != project
            || request.expected_revision.is_none()
            || request.required_owner.as_deref().is_none_or(|owner| {
                owner.len() != 64
                    || !owner
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            || request.session_id.is_none()
        {
            return Err("invalid Git intent project or owner binding".into());
        }
        let Message::Request {
            operation,
            arguments,
        } = &request.payload
        else {
            return Err("invalid Git intent command".into());
        };
        let args = arguments
            .as_object()
            .ok_or("invalid Git intent arguments")?;
        if args
            .get("subtab_revision")
            .and_then(serde_json::Value::as_u64)
            != Some(self.before.revision)
        {
            return Err("Git intent revision disagrees with command".into());
        }
        let mut expected = match (&self.branch, operation) {
            (Some(branch), Operation::SubtabAdd)
                if args.len() == 3
                    && args.get("kind").and_then(serde_json::Value::as_str) == Some("worktree")
                    && args.get("name").and_then(serde_json::Value::as_str)
                        == Some(branch.as_str()) =>
            {
                self.before
                    .plan_worktree(project, base, &self.path, branch)?
            }
            (None, Operation::SubtabClose) if args.len() == 2 => {
                let id = args
                    .get("subtab_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("missing removed tab")?;
                let tab = self
                    .before
                    .tabs
                    .iter()
                    .find(|tab| tab.id == id)
                    .ok_or("unknown removed tab")?;
                if tab.kind != crate::worktree::subtab::SubtabKind::Worktree
                    || tab.root != self.path
                {
                    return Err("Git removal intent target disagrees with tab".into());
                }
                self.before.close_secondary(id)?
            }
            _ => return Err("unsupported Git intent".into()),
        };
        expected.revision = self
            .before
            .revision
            .checked_add(1)
            .ok_or("Git intent revision exhausted")?;
        if expected != self.candidate {
            return Err("Git intent candidate differs from command".into());
        }
        Ok(())
    }
}
#[cfg(feature = "remote")]
impl WorkspaceReceipt {
    fn valid(&self, revision: u64) -> bool {
        let hex = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
        };
        if self.result.is_null() {
            // A durable side-effect reservation is deliberately not a success.
            return crate::remote::auth::valid_identifier(&self.request_id)
                && hex(&self.device_id)
                && hex(&self.fingerprint)
                && !(self.worktree_intent.is_some() && self.quota_intent.is_some());
        }
        if self.worktree_intent.is_some() || self.quota_intent.is_some() {
            return false;
        }
        let Some(result) = self.result.as_object() else {
            return false;
        };
        crate::remote::auth::valid_identifier(&self.request_id)
            && hex(&self.device_id)
            && hex(&self.fingerprint)
            && (result.len() == 3
                || (result.len() == 7
                    && result.get("request_id").and_then(serde_json::Value::as_str)
                        == Some(self.request_id.as_str())
                    && result
                        .get("project_id")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(hex)
                    && ["old_session_id", "new_session_id"].iter().all(|key| {
                        result
                            .get(*key)
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(crate::remote::auth::valid_identifier)
                    })
                    && result["old_session_id"] != result["new_session_id"])
                || (result.len() == 6
                    && result
                        .get("project_id")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(hex)
                    && result
                        .get("active_subtab_id")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(hex)
                    && result
                        .get("subtab_revision")
                        .and_then(serde_json::Value::as_u64)
                        .is_some_and(|v| v > 0)))
            && result
                .get("workspace_revision")
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|v| v > 0 && v <= revision)
            && result
                .get("required_owner")
                .and_then(serde_json::Value::as_str)
                .is_some_and(hex)
            && result
                .get("active_project_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(hex)
    }
}
impl ProjectOwnerPool {
    pub fn new(
        agent: std::sync::Arc<std::sync::Mutex<crate::core::Agent>>,
    ) -> Result<Self, String> {
        let owner = agent.lock().map_err(|_| "project owner lock poisoned")?;
        let cwd = owner
            .attachment_workspace_root()
            .unwrap_or_else(|| Path::new(&owner.session().header().cwd));
        let workspace = ProjectWorkspace::default()
            .with_directory(Some(cwd), cwd)
            .map_err(|e| e.to_string())?
            .0;
        let id = workspace
            .active()
            .ok_or("missing initial project")?
            .id()
            .as_str()
            .to_owned();
        let session = owner.session().path().to_path_buf();
        let session = if session.is_absolute() {
            session
        } else {
            std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(session)
        };
        // An explicit value, never inferred from the approval mode: an owner
        // that happens not to prompt is not the same fact as a user asking for
        // a blanket allow, and only the second may spread to other owners.
        let auto_approve = owner.auto_approve();
        let control = OwnerControl {
            project: id.clone(),
            input_port: owner.input_port(),
            approval: owner.approval_coordinator(),
        };
        let mut owner_nonce = [0; 32];
        getrandom::getrandom(&mut owner_nonce).map_err(|e| e.to_string())?;
        Ok(Self {
            workspace,
            owners: [(id.clone(), std::sync::Arc::clone(&agent))].into(),
            arch_owners: std::collections::BTreeMap::new(),
            initial_session: (id.clone(), session.clone()),
            contexts: [(id.clone(), ProjectContext::from_agent(&id, &owner))].into(),
            control: [(id.clone(), control)].into(),
            checkpoint: None,
            revision: 0,
            #[cfg(feature = "remote")]
            remote_workspace: RemoteWorkspaceState::default(),
            #[cfg(feature = "remote")]
            remote_owner: format!("{:x}", Sha256::digest(owner_nonce)),
            #[cfg(feature = "remote")]
            pending_remote_receipt: None,
            subtabs: Default::default(),
            #[cfg(feature = "remote")]
            worker_owners: Default::default(),
            sessions: [(id, session.clone())].into(),
            overrides: owner.project_overrides(),
            echo_fixture: owner.backend_name() == "echo",
            auto_approve,
            session_root: session
                .parent()
                .ok_or("session has no parent")?
                .join("projects"),
        })
    }
    /// True when every owner prepared by this pool auto-approves (ZS1-180).
    pub fn auto_approve(&self) -> bool {
        self.auto_approve
    }

    pub fn context(&self, id: &str) -> Option<&ProjectContext> {
        self.contexts.get(id)
    }
    pub fn active_context(&self) -> &ProjectContext {
        self.context(
            self.workspace
                .active()
                .expect("committed project")
                .id()
                .as_str(),
        )
        .expect("prepared context")
    }
    pub fn refresh_owner_context(&mut self, id: &str, agent: &crate::core::Agent) {
        self.contexts
            .insert(id.to_owned(), ProjectContext::from_agent(id, agent));
        let session_path = agent.session().path();
        let session_path = session_path.canonicalize().unwrap_or_else(|_| {
            if session_path.is_absolute() {
                session_path.to_path_buf()
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(session_path))
                    .unwrap_or_else(|_| session_path.to_path_buf())
            }
        });
        self.sessions.insert(id.to_owned(), session_path);
    }
    /// A worker owns only a checkpoint proposal, never another Agent. The
    /// existing compare-and-replace writer rejects concurrent host changes.
    pub(crate) fn new_session_plan(
        &self,
        id: &str,
        request_id: Option<String>,
        fingerprint: String,
        version: u16,
    ) -> Result<NewSessionPlan, String> {
        #[cfg(feature = "remote")]
        if !self.worker_handles(id).is_empty() {
            return Err("retire project worker owners before replacing its master session".into());
        }
        let checkpoint = self
            .checkpoint
            .clone()
            .ok_or("new session requires a durable project owner checkpoint")?;
        Ok(NewSessionPlan {
            remote_identity: None,
            context: self.contexts.get(id).ok_or("project not found")?.clone(),
            old_path: self
                .sessions
                .get(id)
                .ok_or("project session not found")?
                .clone(),
            workspace: self.workspace.clone(),
            sessions: self.sessions.clone(),
            checkpoint,
            revision: self.revision,
            request_id,
            fingerprint,
            version,
            #[cfg(feature = "remote")]
            remote_workspace: self.remote_workspace.clone(),
            subtabs: self.subtabs.clone(),
        })
    }

    pub(crate) fn accept_new_session(&mut self, committed: &NewSessionCommit) {
        if let Some(control) = &committed.control {
            self.control
                .insert(committed.context.project_id.clone(), control.clone());
        }
        #[cfg(feature = "remote")]
        let previous_session = self
            .contexts
            .get(&committed.context.project_id)
            .map(|context| context.session_id.clone());
        #[cfg(feature = "remote")]
        {
            self.remote_workspace = committed.remote_workspace.clone();
        }
        self.revision = committed.revision;
        self.contexts.insert(
            committed.context.project_id.clone(),
            committed.context.clone(),
        );
        self.sessions
            .insert(committed.context.project_id.clone(), committed.path.clone());
        if let Some(checkpoint) = &mut self.checkpoint {
            *checkpoint = committed.checkpoint.clone();
        }
        #[cfg(feature = "remote")]
        if let Some(session) = previous_session {
            crate::remote::wakeup::notify(&session);
        }
        self.notify_workspace_sessions();
    }

    #[cfg(feature = "remote")]
    /// Reserve a remote identity before the existing replacement transaction.
    pub(crate) fn remote_new_session_plan(
        &mut self,
        id: &str,
        device: &str,
        request: &str,
        fingerprint: &str,
    ) -> Result<NewSessionPlan, String> {
        let _ = self.new_session_plan(id, Some(request.into()), fingerprint.into(), 1)?;
        if self
            .workspace_receipt(device, request, fingerprint)
            .map_err(|_| "new session identity unavailable")?
            .is_some()
        {
            return Err("new session identity already reserved".into());
        }
        let mut remote = self.remote_workspace.clone();
        remote.receipts.push(WorkspaceReceipt {
            device_id: device.into(),
            request_id: request.into(),
            fingerprint: fingerprint.into(),
            result: serde_json::Value::Null,
            worktree_intent: None,
            quota_intent: None,
        });
        let bytes = self.core_bytes()?;
        self.checkpoint
            .as_mut()
            .ok_or("missing new session checkpoint")?
            .commit(&bytes, Some(&remote.to_bytes()?))?;
        self.remote_workspace = remote;
        let mut plan = self.new_session_plan(id, Some(request.into()), fingerprint.into(), 1)?;
        plan.remote_identity = Some((device.into(), self.workspace_owner().into()));
        Ok(plan)
    }

    /// Commit a fully prepared resume while the caller still holds the old
    /// Agent. Only the infallible Agent replacement may follow this method.
    pub(crate) fn commit_resumed_session(
        &mut self,
        id: &str,
        session: &crate::session::SessionStore,
    ) -> Result<(), String> {
        self.commit_resumed_session_receipt(id, session, None)
    }

    #[cfg(feature = "remote")]
    pub(crate) fn commit_remote_resumed_session(
        &mut self,
        id: &str,
        session: &crate::session::SessionStore,
        device: &str,
        request: &str,
        fingerprint: &str,
        owner: &str,
    ) -> Result<(), String> {
        self.commit_resumed_session_receipt(
            id,
            session,
            Some((device, request, fingerprint, owner)),
        )
    }

    pub(crate) fn refresh_owner_control(&mut self, id: &str, agent: &crate::core::Agent) {
        self.control.insert(
            id.into(),
            OwnerControl {
                project: id.into(),
                input_port: agent.input_port(),
                approval: agent.approval_coordinator(),
            },
        );
    }

    fn commit_resumed_session_receipt(
        &mut self,
        id: &str,
        session: &crate::session::SessionStore,
        receipt: Option<(&str, &str, &str, &str)>,
    ) -> Result<(), String> {
        #[cfg(feature = "remote")]
        if !self.worker_handles(id).is_empty() {
            return Err("retire project worker owners before resuming a master session".into());
        }
        let mut context = self.contexts.get(id).ok_or("project not found")?.clone();
        #[cfg(feature = "remote")]
        let previous_session = context.session_id.clone();
        let path = session.path().canonicalize().map_err(|e| e.to_string())?;
        validate_path_text(&path).map_err(|e| e.to_string())?;
        let mut sessions = self.sessions.clone();
        sessions.insert(id.to_owned(), path);
        context.session_id = session.session_id().to_owned();
        let changed = self.contexts.get(id) != Some(&context) || sessions != self.sessions;
        let mut revision = self.revision;
        if changed {
            revision = revision
                .checked_add(1)
                .ok_or("workspace revision exhausted")?;
        }
        if receipt.is_some() && !changed {
            return Err("remote resume must replace current session".into());
        }
        #[cfg(feature = "remote")]
        let mut remote_workspace = self.remote_workspace.clone();
        #[cfg(feature = "remote")]
        if let Some((device, request, fingerprint, owner)) = receipt {
            let receipt = remote_workspace
                .receipts
                .iter_mut()
                .find(|receipt| receipt.device_id == device && receipt.request_id == request)
                .ok_or("missing resume reservation")?;
            if receipt.fingerprint != fingerprint || !receipt.result.is_null() {
                return Err("resume reservation conflict".into());
            }
            receipt.result = serde_json::json!({"request_id":request,"project_id":id,"old_session_id":previous_session,
                "new_session_id":context.session_id,"required_owner":owner,"workspace_revision":revision,
                "active_project_id":self.workspace.active().map(|tab|tab.id().as_str())});
        }
        if let Some(checkpoint) = &mut self.checkpoint {
            let bytes = core_checkpoint(&self.workspace, &sessions, revision, &self.subtabs)?;
            #[cfg(feature = "remote")]
            let remote = receipt
                .is_some()
                .then(|| remote_workspace.to_bytes())
                .transpose()?;
            #[cfg(not(feature = "remote"))]
            let remote: Option<Vec<u8>> = None;
            checkpoint.commit(&bytes, remote.as_deref())?;
        }
        self.sessions = sessions;
        self.contexts.insert(id.to_owned(), context);
        self.revision = revision;
        #[cfg(feature = "remote")]
        {
            self.remote_workspace = remote_workspace;
        }
        if changed {
            #[cfg(feature = "remote")]
            crate::remote::wakeup::notify(&previous_session);
            self.notify_workspace_sessions();
        }
        Ok(())
    }
    /// Enable a session-scoped checkpoint shared by both production hosts.
    /// Strict validation precedes owner preparation; a failed restore leaves
    /// the initial owner usable and returns an explicit error to the host.
    pub fn restore_checkpoint(&mut self) -> Result<bool, String> {
        let path = self
            .session_root
            .parent()
            .ok_or("missing session parent")?
            .join("project-workspace.json");
        let bytes = read_owner_checkpoint(&path)?;
        let remote_path = path.with_file_name(REMOTE_STATE_FILE);
        let Some(bytes) = bytes else {
            let checkpoint = OwnerCheckpoint {
                path,
                previous: None,
                remote_previous: None,
            };
            #[cfg(feature = "remote")]
            let checkpoint = {
                // Remote state never outlives the project checkpoint it describes.
                let mut checkpoint = checkpoint;
                checkpoint.remote_previous = read_owner_checkpoint(&remote_path)?;
                let mut remote = RemoteWorkspaceState::default();
                if remote.import_prompt_journals(&self.sessions)?
                    || checkpoint.remote_previous.is_some()
                {
                    checkpoint.commit(&self.core_bytes()?, Some(&remote.to_bytes()?))?;
                }
                self.remote_workspace = remote;
                checkpoint
            };
            self.checkpoint = Some(checkpoint);
            return Ok(false);
        };
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Saved {
            schema_version: u16,
            workspace: serde_json::Value,
            sessions: std::collections::BTreeMap<String, PathBuf>,
            #[serde(default)]
            workspace_revision: u64,
            /// Remote state written inside the project checkpoint before ZE-510.
            #[serde(default)]
            remote_workspace: Option<serde_json::Map<String, serde_json::Value>>,
            #[serde(default)]
            subtabs: std::collections::BTreeMap<String, crate::worktree::subtab::SubtabState>,
        }
        let mut saved: Saved = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if saved.schema_version != 1 {
            return Err("unsupported project owner checkpoint".into());
        }
        let mut revision = saved.workspace_revision;
        let legacy = saved
            .remote_workspace
            .take()
            .map(|mut legacy| {
                revision = legacy
                    .remove("revision")
                    .and_then(|revision| revision.as_u64())
                    .ok_or("invalid legacy workspace revision")?;
                serde_json::to_vec(&legacy).map_err(|e| e.to_string())
            })
            .transpose()?;
        let workspace = ProjectWorkspace::from_json_bytes(
            &serde_json::to_vec(&saved.workspace).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if saved.sessions.len() != workspace.tabs().len()
            || saved.sessions.iter().any(|(id, path)| {
                !workspace.tabs().iter().any(|tab| tab.id().as_str() == id)
                    || !path.is_absolute()
                    || validate_path_text(path).is_err()
            })
        {
            return Err("invalid project session mapping".into());
        }
        for (id, state) in &saved.subtabs {
            let tab = workspace
                .tabs()
                .iter()
                .find(|tab| tab.id().as_str() == id)
                .ok_or("subtabs belong to unknown project")?;
            state.validate(id, tab.cwd())?;
        }
        let mut checkpoint = OwnerCheckpoint {
            path,
            previous: Some(bytes.clone()),
            remote_previous: None,
        };
        // Only a remote build reads remote state; any build moves a legacy
        // inline copy out unchanged.
        if cfg!(feature = "remote") || legacy.is_some() {
            checkpoint.remote_previous = read_owner_checkpoint(&remote_path)?;
        }
        // Never adopt a checkpoint naming an unreadable journal; a remote
        // build proves this while importing journal admissions below.
        #[cfg(not(feature = "remote"))]
        for path in saved.sessions.values() {
            crate::session::SessionStore::open_existing(path).map_err(|e| e.to_string())?;
        }
        #[cfg(feature = "remote")]
        let remote = {
            let mut remote: RemoteWorkspaceState = legacy
                .as_ref()
                .or(checkpoint.remote_previous.as_ref())
                .map(|bytes| serde_json::from_slice(bytes).map_err(|e| e.to_string()))
                .transpose()?
                .unwrap_or_default();
            remote.validate(revision)?;
            // Import legacy journal admissions once, before any remote mutation can reuse their IDs.
            if remote.import_prompt_journals(&saved.sessions)? || legacy.is_some() {
                checkpoint.commit(&bytes, Some(&remote.to_bytes()?))?;
            }
            remote
        };
        if legacy.is_some() {
            #[cfg(not(feature = "remote"))]
            checkpoint.commit(&bytes, legacy.as_deref())?;
            // Only now that remote-state.json holds it may the legacy copy go.
            let bytes =
                core_checkpoint(&saved.workspace, &saved.sessions, revision, &saved.subtabs)?;
            checkpoint.commit(&bytes, None)?;
        }
        // Adopt the checkpoint even when a saved project fails to prepare
        // (for example a stale saved model digest): a failed publish leaves
        // the pool and the files untouched, so they stay the CAS baseline,
        // and a later new session still has a durable owner to commit against.
        // Restore is adoption, not a new checkpoint publication using stale in-memory receipts.
        self.checkpoint = None;
        let published = self.publish(workspace, &saved.sessions);
        self.checkpoint = Some(checkpoint);
        self.revision = revision;
        #[cfg(feature = "remote")]
        {
            self.remote_workspace = remote;
        }
        published?;
        self.subtabs = saved.subtabs;
        Ok(true)
    }
    pub fn subtab_state(
        &self,
        project: &str,
    ) -> Result<crate::worktree::subtab::SubtabState, String> {
        let tab = self
            .workspace
            .tabs()
            .iter()
            .find(|tab| tab.id().as_str() == project)
            .ok_or("unknown project")?;
        self.subtabs
            .get(project)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| crate::worktree::subtab::SubtabState::main(project, tab.cwd()))
    }
    pub fn has_subtab_state(&self, project: &str) -> bool {
        self.subtabs.contains_key(project)
    }
    pub fn subtab_execution(
        &self,
        project: &str,
    ) -> Result<crate::worktree::subtab::SubtabExecution, String> {
        let state = self.subtab_state(project)?;
        let tab = state
            .tabs
            .into_iter()
            .find(|tab| tab.id == state.active_id)
            .ok_or("missing selected subtab")?;
        let project_root = self
            .workspace
            .tabs()
            .iter()
            .find(|tab| tab.id().as_str() == project)
            .ok_or("unknown project")?
            .cwd()
            .to_owned();
        Ok(crate::worktree::subtab::SubtabExecution { tab, project_root })
    }

    #[cfg(feature = "remote")]
    /// Check the whole ownership range before preparing any new journals.
    pub(crate) fn worker_capacity_available(
        &self,
        project: &str,
        subtab: &str,
        start: u16,
        end: u16,
    ) -> Result<bool, String> {
        if start > end || end > crate::runtime::MAX_CONCURRENCY {
            return Err("invalid worker capacity range".into());
        }
        let session = &self
            .context(project)
            .ok_or("unknown worker project")?
            .session_id;
        let keys = (start..end)
            .map(|slot| crate::worktree::worker::key(project, session, subtab, slot))
            .collect::<Result<Vec<_>, _>>()?;
        let missing = keys
            .iter()
            .filter(|key| !self.worker_owners.contains_key(*key))
            .count();
        Ok(self.worker_owners.len() + missing <= crate::worktree::worker::MAX_WORKER_OWNERS)
    }

    #[cfg(feature = "remote")]
    /// Prepare an independent slot from host-owned subtab state, not a client path.
    pub fn worker_owner(
        &mut self,
        project: &str,
        subtab: &str,
        slot: u16,
    ) -> Result<crate::worktree::worker::WorkerOwner, String> {
        if !self.has_workspace_checkpoint() {
            return Err("worker requires a durable owner checkpoint".into());
        }
        let mut execution = self.subtab_execution(project)?;
        execution.tab = self
            .subtab_state(project)?
            .tabs
            .into_iter()
            .find(|tab| tab.id == subtab)
            .ok_or("unknown worker subtab")?;
        if slot >= execution.tab.concurrency || slot >= crate::runtime::MAX_CONCURRENCY {
            return Err("worker slot exceeds the host-owned quota".into());
        }
        execution.validate()?;
        let session = &self
            .context(project)
            .ok_or("unknown worker project")?
            .session_id;
        let key = crate::worktree::worker::key(project, session, subtab, slot)?;
        if let Some(owner) = self.worker_owners.get(&key) {
            return Ok(owner.clone());
        }
        if self.worker_owners.len() >= crate::worktree::worker::MAX_WORKER_OWNERS {
            return Err("worker owner capacity exhausted".into());
        }
        let master = self.owner(project).ok_or("missing worker master")?;
        let master = master.try_lock().map_err(|_| "worker master is busy")?;
        if master.session().session_id() != session {
            return Err("worker master session changed; refresh host context".into());
        }
        if master.phase() != crate::core::AgentPhase::Idle
            || master.input_port().has_pending()
            || master.input_port().is_preparing()
        {
            return Err("worker master is busy".into());
        }
        let owner = crate::worktree::worker::WorkerOwner::prepare(
            project,
            session,
            self.sessions
                .get(project)
                .ok_or("missing worker master journal")?,
            execution,
            slot,
            master.project_overrides(),
            master.backend_name() == "echo",
            self.auto_approve,
        )?;
        self.worker_owners.insert(key, owner.clone());
        Ok(owner)
    }

    #[cfg(feature = "remote")]
    /// Stage exclusive new slots against a durable, unresolved growth intent.
    pub(crate) fn claim_quota_growth(
        &mut self,
        device: &str,
        request: &str,
        fingerprint: &str,
        scope: (&str, &str, &str, u64),
        configured: u16,
        capacity: u16,
    ) -> Result<
        Vec<(
            crate::worktree::worker::WorkerOwner,
            crate::worktree::worker::WorkerClaim,
        )>,
        String,
    > {
        let receipt = self
            .remote_workspace
            .receipts
            .iter()
            .find(|receipt| {
                receipt.device_id == device
                    && receipt.request_id == request
                    && receipt.fingerprint == fingerprint
            })
            .ok_or("missing quota growth reservation")?;
        let intent = receipt
            .quota_intent
            .as_ref()
            .filter(|_| receipt.result.is_null())
            .ok_or("quota growth intent is settled or missing")?;
        let (project, session, subtab, revision) = scope;
        let base = self
            .workspace
            .tabs()
            .iter()
            .find(|tab| tab.id().as_str() == project)
            .ok_or("unknown quota growth project")?
            .cwd();
        intent.validate(receipt, base)?;
        let before = intent
            .before
            .tabs
            .iter()
            .find(|tab| tab.id == subtab)
            .ok_or("unknown quota growth tab")?;
        let target = intent
            .candidate
            .tabs
            .iter()
            .find(|tab| tab.id == subtab)
            .ok_or("missing quota growth candidate")?;
        if intent.request.project_id.as_deref() != Some(project)
            || intent.request.session_id.as_deref() != Some(session)
            || self
                .context(project)
                .map(|context| context.session_id.as_str())
                != Some(session)
            || intent.request.expected_revision != Some(self.workspace_revision())
            || intent.before != self.subtab_state(project)?
            || intent.before.revision != revision
            || before.concurrency != configured
            || target.concurrency <= capacity
        {
            return Err("quota growth scope, revision or capacity mismatch".into());
        }
        let mut execution = self.subtab_execution(project)?;
        execution.tab = target.clone();
        execution.validate()?;
        let master = self.owner(project).ok_or("missing quota growth master")?;
        let guard = master
            .try_lock()
            .map_err(|_| "quota growth master is busy")?;
        if guard.session().session_id() != session
            || !crate::worktree::worker::WorkerOwner::idle(&guard)
        {
            return Err("quota growth master is busy or changed".into());
        }
        let arch = self.existing_arch_owner(project);
        let arch_guard = arch
            .as_ref()
            .map(|owner| owner.try_lock().map_err(|_| "quota growth arch is busy"))
            .transpose()?;
        if arch_guard
            .as_ref()
            .is_some_and(|agent| !crate::worktree::worker::WorkerOwner::idle(agent))
        {
            return Err("quota growth arch is busy".into());
        }
        let keys: Vec<_> = (capacity..target.concurrency)
            .map(|slot| crate::worktree::worker::key(project, session, subtab, slot))
            .collect::<Result<_, _>>()?;
        let missing = keys
            .iter()
            .filter(|key| !self.worker_owners.contains_key(*key))
            .count();
        if self.worker_owners.len() + missing > crate::worktree::worker::MAX_WORKER_OWNERS {
            return Err("quota growth owner capacity exhausted".into());
        }
        // Publish handles only after every new writer and runtime claim succeeds.
        // Staged claims/leases release automatically on any preparation failure.
        let mut added = Vec::new();
        for (slot, key) in (capacity..target.concurrency).zip(&keys) {
            let worker = match self.worker_owners.get(key) {
                Some(worker) => worker.clone(),
                None => crate::worktree::worker::WorkerOwner::prepare(
                    project,
                    session,
                    self.sessions
                        .get(project)
                        .ok_or("missing quota growth journal")?,
                    execution.clone(),
                    slot,
                    guard.project_overrides(),
                    guard.backend_name() == "echo",
                    self.auto_approve,
                )?,
            };
            let claim = worker.claim_runtime()?;
            added.push((worker, claim));
        }
        for (key, (worker, _)) in keys.into_iter().zip(&added) {
            self.worker_owners.insert(key, worker.clone());
        }
        Ok(added)
    }

    #[cfg(feature = "remote")]
    /// Worker controls remain reachable without taking an execution lock.
    pub fn worker_handles(&self, project: &str) -> Vec<crate::worktree::worker::WorkerOwner> {
        self.worker_owners
            .values()
            .filter(|owner| owner.control().project == project)
            .cloned()
            .collect()
    }

    #[cfg(feature = "remote")]
    /// Retire idle owners before replacing their master. Caller-held handles
    /// keep leases alive, but closed Agents cannot admit another turn.
    pub fn retire_worker_owners(&mut self, project: &str) -> Result<(), String> {
        let workers = self.worker_handles(project);
        let mut guards = Vec::new();
        for worker in &workers {
            let guard = worker
                .agent()
                .try_lock()
                .map_err(|_| "project worker is busy")?;
            if worker.runtime_owned() || !crate::worktree::worker::WorkerOwner::idle(&guard) {
                return Err("project worker is busy".into());
            }
            guards.push(guard);
        }
        for mut guard in guards {
            guard.close();
        }
        self.worker_owners
            .retain(|_, worker| worker.control().project != project);
        Ok(())
    }

    #[cfg(feature = "remote")]
    fn retired_workers(
        &self,
        project: &str,
        candidate: &crate::worktree::subtab::SubtabState,
    ) -> Vec<String> {
        let Some(context) = self.context(project) else {
            return Vec::new();
        };
        self.worker_owners
            .iter()
            .filter(|(key, owner)| {
                owner.control().project == project
                    && !candidate.tabs.iter().any(|tab| {
                        tab.id == owner.execution().tab.id
                            && tab.root == owner.execution().tab.root
                            && tab.kind == owner.execution().tab.kind
                            && (0..tab.concurrency).any(|slot| {
                                crate::worktree::worker::key(
                                    project,
                                    &context.session_id,
                                    &tab.id,
                                    slot,
                                )
                                .as_ref()
                                    == Ok(*key)
                            })
                    })
            })
            .map(|(key, _)| key.clone())
            .collect()
    }
    /// Local and remote adapters commit the same stable-ID domain, never a view index.
    pub fn commit_subtabs(
        &mut self,
        project: &str,
        expected_revision: u64,
        candidate: crate::worktree::subtab::SubtabState,
    ) -> Result<(), String> {
        let before = self.subtab_state(project)?;
        if before.revision != expected_revision {
            return Err("subtab revision conflict".into());
        }
        let tab = self
            .workspace
            .tabs()
            .iter()
            .find(|tab| tab.id().as_str() == project)
            .ok_or("unknown project")?;
        candidate.validate(project, tab.cwd())?;
        if candidate == before {
            return Ok(());
        }
        if candidate.revision
            != expected_revision
                .checked_add(1)
                .ok_or("subtab revision exhausted")?
        {
            return Err("invalid subtab revision transition".into());
        }
        #[cfg(feature = "remote")]
        let workers = self.worker_handles(project);
        #[cfg(feature = "remote")]
        let mut guards = Vec::new();
        #[cfg(feature = "remote")]
        for worker in &workers {
            let guard = worker
                .agent()
                .try_lock()
                .map_err(|_| "subtab worker is busy")?;
            if worker.runtime_owned() || !crate::worktree::worker::WorkerOwner::idle(&guard) {
                return Err("subtab worker is busy".into());
            }
            guards.push(guard);
        }
        #[cfg(feature = "remote")]
        let retired = self.retired_workers(project, &candidate);
        let mut subtabs = self.subtabs.clone();
        subtabs.insert(project.into(), candidate);
        let bytes = core_checkpoint(&self.workspace, &self.sessions, self.revision, &subtabs)?;
        self.checkpoint
            .as_mut()
            .ok_or("subtabs require a durable owner checkpoint")?
            .commit(&bytes, None)?;
        self.subtabs = subtabs;
        #[cfg(feature = "remote")]
        for (worker, mut guard) in workers.iter().zip(guards) {
            if retired.iter().any(|key| {
                self.worker_owners
                    .get(key)
                    .is_some_and(|old| std::ptr::eq(old.agent(), worker.agent()))
            }) {
                guard.close();
            }
        }
        #[cfg(feature = "remote")]
        for key in retired {
            self.worker_owners.remove(&key);
        }
        #[cfg(feature = "remote")]
        if let Some(context) = self.contexts.get(project) {
            crate::remote::wakeup::notify(&context.session_id);
        }
        Ok(())
    }
    pub fn migrate_legacy_subtabs(
        &mut self,
        project: &str,
        legacy: &[crate::worktree::subtab::LegacySubtab],
        active_index: usize,
    ) -> Result<(), String> {
        if self.subtabs.contains_key(project) {
            return Err("subtabs already migrated; host state is authoritative".into());
        }
        let tab = self
            .workspace
            .tabs()
            .iter()
            .find(|tab| tab.id().as_str() == project)
            .ok_or("unknown project")?;
        let mut candidate = crate::worktree::subtab::SubtabState::migrate(
            project,
            tab.cwd(),
            legacy,
            active_index,
        )?;
        candidate.revision = 1;
        self.commit_subtabs(project, 0, candidate)
    }
    #[cfg(feature = "remote")]
    pub(crate) fn commit_remote_subtabs(
        &mut self,
        project: &str,
        candidate: crate::worktree::subtab::SubtabState,
        device: &str,
        request: &str,
        fingerprint: &str,
    ) -> Result<serde_json::Value, String> {
        self.commit_remote_subtabs_inner(project, candidate, device, request, fingerprint, None)
    }

    #[cfg(feature = "remote")]
    pub(crate) fn quota_candidate(
        &self,
        device: &str,
        request: &str,
        fingerprint: &str,
        scope: (&str, &str, &str, u64),
        configured: u16,
    ) -> Result<crate::worktree::subtab::SubtabState, String> {
        let receipt = self
            .remote_workspace
            .receipts
            .iter()
            .find(|receipt| {
                receipt.device_id == device
                    && receipt.request_id == request
                    && receipt.fingerprint == fingerprint
            })
            .ok_or("missing quota reservation")?;
        let intent = receipt
            .quota_intent
            .as_ref()
            .filter(|_| receipt.result.is_null())
            .ok_or("quota intent settled or missing")?;
        let (project, session, subtab, revision) = scope;
        let base = self
            .workspace
            .tabs()
            .iter()
            .find(|tab| tab.id().as_str() == project)
            .ok_or("unknown quota project")?
            .cwd();
        intent.validate(receipt, base)?;
        if intent.request.project_id.as_deref() != Some(project)
            || intent.request.session_id.as_deref() != Some(session)
            || self
                .context(project)
                .map(|context| context.session_id.as_str())
                != Some(session)
            || intent.request.expected_revision != Some(self.workspace_revision())
            || intent.before != self.subtab_state(project)?
            || intent.before.revision != revision
            || !intent
                .before
                .tabs
                .iter()
                .any(|tab| tab.id == subtab && tab.concurrency == configured)
            || !matches!(&intent.request.payload, crate::remote::protocol::Message::Request { arguments, .. } if arguments["subtab_id"] == subtab)
        {
            return Err("quota reservation scope or state changed".into());
        }
        Ok(intent.candidate.clone())
    }

    #[cfg(feature = "remote")]
    pub(crate) fn commit_runtime_quota(
        &mut self,
        device: &str,
        request: &str,
        fingerprint: &str,
        scope: (&str, &str, &str, u64),
        configured: u16,
        workers: &[crate::worktree::worker::WorkerOwner],
    ) -> Result<serde_json::Value, String> {
        let candidate = self.quota_candidate(device, request, fingerprint, scope, configured)?;
        let owner = self.owner(scope.0).ok_or("missing quota master")?;
        let guard = owner.try_lock().map_err(|_| "quota master is busy")?;
        if !crate::worktree::worker::WorkerOwner::idle(&guard) {
            return Err("quota master is busy".into());
        }
        let arch = self.existing_arch_owner(scope.0);
        let arch_guard = arch
            .as_ref()
            .map(|owner| owner.try_lock().map_err(|_| "quota arch is busy"))
            .transpose()?;
        if arch_guard
            .as_ref()
            .is_some_and(|agent| !crate::worktree::worker::WorkerOwner::idle(agent))
        {
            return Err("quota arch is busy".into());
        }
        self.commit_remote_subtabs_inner(
            scope.0,
            candidate,
            device,
            request,
            fingerprint,
            Some((scope.1, scope.2, workers)),
        )
    }

    #[cfg(feature = "remote")]
    fn commit_remote_subtabs_inner(
        &mut self,
        project: &str,
        candidate: crate::worktree::subtab::SubtabState,
        device: &str,
        request: &str,
        fingerprint: &str,
        runtime: Option<(&str, &str, &[crate::worktree::worker::WorkerOwner])>,
    ) -> Result<serde_json::Value, String> {
        let before = self.subtab_state(project)?;
        let tab = self
            .workspace
            .tabs()
            .iter()
            .find(|tab| tab.id().as_str() == project)
            .ok_or("unknown project")?;
        candidate.validate(project, tab.cwd())?;
        let workers = self.worker_handles(project);
        let mut guards = Vec::new();
        for worker in &workers {
            if let Some((session, subtab, claimed)) = runtime {
                let execution = worker.execution();
                if execution.tab.id != subtab && worker.master_session_id() == session {
                    let unchanged = before
                        .tabs
                        .iter()
                        .find(|tab| tab.id == execution.tab.id)
                        .zip(candidate.tabs.iter().find(|tab| tab.id == execution.tab.id))
                        .is_some_and(|(old, next)| {
                            old == next
                                && old.root == execution.tab.root
                                && old.kind == execution.tab.kind
                        });
                    if unchanged {
                        continue;
                    }
                }
                if claimed
                    .iter()
                    .any(|owned| std::ptr::eq(owned.agent(), worker.agent()))
                {
                    if !worker.runtime_owned()
                        || worker.master_session_id() != session
                        || worker.execution().tab.id != subtab
                    {
                        return Err("quota runtime ownership mismatch".into());
                    }
                    continue;
                }
            }
            let guard = worker
                .agent()
                .try_lock()
                .map_err(|_| "subtab worker is busy")?;
            if worker.runtime_owned() || !crate::worktree::worker::WorkerOwner::idle(&guard) {
                return Err("subtab worker is busy".into());
            }
            guards.push((worker, guard));
        }
        let retired: Vec<_> = self
            .retired_workers(project, &candidate)
            .into_iter()
            .filter(|key| {
                !self
                    .worker_owners
                    .get(key)
                    .is_some_and(|worker| worker.runtime_owned())
            })
            .collect();
        if candidate.revision
            != before
                .revision
                .checked_add(1)
                .ok_or("subtab revision exhausted")?
        {
            return Err("invalid subtab revision transition".into());
        }
        let mut remote = self.remote_workspace.clone();
        let reserved = remote
            .receipts
            .iter()
            .position(|receipt| receipt.device_id == device && receipt.request_id == request);
        if reserved.is_some_and(|index| {
            remote.receipts[index].fingerprint != fingerprint
                || !remote.receipts[index].result.is_null()
        }) {
            return Err("workspace receipt already settled or conflicted".into());
        }
        if let Some(intent) =
            reserved.and_then(|index| remote.receipts[index].quota_intent.as_ref())
            && (intent.before != before
                || intent.candidate != candidate
                || intent.request.project_id.as_deref() != Some(project)
                || intent.request.expected_revision != Some(self.workspace_revision())
                || intent.request.session_id.as_deref()
                    != self
                        .context(project)
                        .map(|context| context.session_id.as_str()))
        {
            return Err("quota reservation no longer matches committed state".into());
        }
        if reserved.is_none() && remote.receipts.len() >= 128 {
            return Err("workspace receipt capacity exhausted".into());
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or("workspace revision exhausted")?;
        let receipt_owner = reserved
            .and_then(|index| {
                remote.receipts[index]
                    .worktree_intent
                    .as_ref()
                    .map(|intent| &intent.request)
                    .or_else(|| {
                        remote.receipts[index]
                            .quota_intent
                            .as_ref()
                            .map(|intent| &intent.request)
                    })
            })
            .and_then(|request| request.required_owner.as_deref())
            .unwrap_or(&self.remote_owner);
        let result = serde_json::json!({"workspace_revision":revision, "active_project_id":self.workspace.active().map(|tab| tab.id().as_str()),
            "required_owner":receipt_owner, "project_id":project, "active_subtab_id":candidate.active_id, "subtab_revision":candidate.revision});
        let receipt = WorkspaceReceipt {
            device_id: device.into(),
            request_id: request.into(),
            fingerprint: fingerprint.into(),
            result: result.clone(),
            worktree_intent: None,
            quota_intent: None,
        };
        if let Some(index) = reserved {
            remote.receipts[index] = receipt;
        } else {
            remote.receipts.push(receipt);
        }
        let mut subtabs = self.subtabs.clone();
        subtabs.insert(project.into(), candidate);
        let bytes = core_checkpoint(&self.workspace, &self.sessions, revision, &subtabs)?;
        self.checkpoint
            .as_mut()
            .ok_or("subtabs require a durable owner checkpoint")?
            .commit(&bytes, Some(&remote.to_bytes()?))?;
        self.subtabs = subtabs;
        self.revision = revision;
        for (worker, mut guard) in guards {
            if retired.iter().any(|key| {
                self.worker_owners
                    .get(key)
                    .is_some_and(|old| std::ptr::eq(old.agent(), worker.agent()))
            }) {
                guard.close();
            }
        }
        for key in retired {
            self.worker_owners.remove(&key);
        }
        self.remote_workspace = remote;
        self.notify_workspace_sessions();
        Ok(result)
    }

    #[cfg(feature = "remote")]
    /// Reserve exact identity before any scheduler application. This does not
    /// change configured quota or prove a runtime applied it.
    pub(crate) fn reserve_remote_quota(
        &mut self,
        device: &str,
        request: &crate::remote::protocol::Envelope,
        fingerprint: &str,
        candidate: &crate::worktree::subtab::SubtabState,
    ) -> Result<(), String> {
        let id = request.id.as_deref().ok_or("missing quota request ID")?;
        let project = request
            .project_id
            .as_deref()
            .ok_or("missing quota project")?;
        if self
            .workspace_receipt(device, id, fingerprint)
            .map_err(|_| "quota identity conflict or capacity")?
            .is_some()
        {
            return Err("quota request already reserved".into());
        }
        if self.quota_intent_pending() {
            return Err("another quota intent owns the workspace revision".into());
        }
        let before = self.subtab_state(project)?;
        let receipt = WorkspaceReceipt {
            device_id: device.into(),
            request_id: id.into(),
            fingerprint: fingerprint.into(),
            result: serde_json::Value::Null,
            worktree_intent: None,
            quota_intent: Some(QuotaIntent {
                request: request.clone(),
                before,
                candidate: candidate.clone(),
            }),
        };
        let base = self
            .workspace
            .tabs()
            .iter()
            .find(|tab| tab.id().as_str() == project)
            .ok_or("unknown quota project")?
            .cwd();
        if request.required_owner.as_deref() != Some(self.workspace_owner())
            || request.expected_revision != Some(self.workspace_revision())
            || request.session_id.as_deref()
                != self
                    .context(project)
                    .map(|context| context.session_id.as_str())
            || !receipt.valid(self.workspace_revision())
        {
            return Err("quota reservation owner or revision mismatch".into());
        }
        receipt
            .quota_intent
            .as_ref()
            .unwrap()
            .validate(&receipt, base)?;
        let mut remote = self.remote_workspace.clone();
        remote.receipts.push(receipt);
        let bytes = self.core_bytes()?;
        self.checkpoint
            .as_mut()
            .ok_or("quota mutation requires durable checkpoint")?
            .commit(&bytes, Some(&remote.to_bytes()?))?;
        self.remote_workspace = remote;
        Ok(())
    }

    // Quota receipts commit against the global workspace revision, even across projects.
    #[cfg(feature = "remote")]
    pub(crate) fn quota_intent_pending(&self) -> bool {
        self.remote_workspace
            .receipts
            .iter()
            .any(|receipt| receipt.result.is_null() && receipt.quota_intent.is_some())
    }

    #[cfg(feature = "remote")]
    /// Local operator evidence only; no automatic replay or runtime claim.
    pub fn unresolved_quota_intents(&self) -> Vec<serde_json::Value> {
        self.remote_workspace.receipts.iter().filter_map(|receipt| {
            let intent = receipt.quota_intent.as_ref()?;
            if !receipt.result.is_null() { return None; }
            let crate::remote::protocol::Message::Request { arguments, .. } = &intent.request.payload else { return None; };
            Some(serde_json::json!({"device_id":receipt.device_id,"request_id":receipt.request_id,"fingerprint":receipt.fingerprint,
                "project_id":intent.request.project_id,"session_id":intent.request.session_id,"subtab_id":arguments["subtab_id"],
                "concurrency":arguments["concurrency"],"workspace_revision":intent.request.expected_revision,
                "subtab_revision":intent.before.revision,"state":"unknown_outcome"}))
        }).collect()
    }

    #[cfg(feature = "remote")]
    /// Explicit local acceptance after the old runtime has stopped. Commit
    /// configuration, never replay a scheduler command or start workers.
    pub fn reconcile_quota_intent(
        &mut self,
        device: &str,
        id: &str,
    ) -> Result<serde_json::Value, String> {
        let receipt = self
            .remote_workspace
            .receipts
            .iter()
            .find(|receipt| receipt.device_id == device && receipt.request_id == id)
            .cloned()
            .ok_or("unknown quota request")?;
        if !receipt.result.is_null() {
            return Ok(receipt.result);
        }
        let intent = receipt
            .quota_intent
            .as_ref()
            .ok_or("missing recoverable quota intent")?;
        let project = intent
            .request
            .project_id
            .as_deref()
            .ok_or("missing quota project")?;
        if self.workspace_revision()
            != intent
                .request
                .expected_revision
                .ok_or("missing quota workspace revision")?
            || self.subtab_state(project)? != intent.before
            || self
                .context(project)
                .map(|context| context.session_id.as_str())
                != intent.request.session_id.as_deref()
        {
            return Err("quota intent is stale; cannot overwrite newer host state".into());
        }
        let base = self
            .workspace
            .tabs()
            .iter()
            .find(|tab| tab.id().as_str() == project)
            .ok_or("quota project is closed")?
            .cwd();
        intent.validate(&receipt, base)?;
        let master = self.owner(project).ok_or("quota master is missing")?;
        let guard = master.try_lock().map_err(|_| "quota master is busy")?;
        if !crate::worktree::worker::WorkerOwner::idle(&guard) {
            return Err("quota master is busy".into());
        }
        let arch = self.existing_arch_owner(project);
        let arch_guard = arch
            .as_ref()
            .map(|owner| owner.try_lock().map_err(|_| "quota arch owner is busy"))
            .transpose()?;
        if arch_guard
            .as_ref()
            .is_some_and(|agent| !crate::worktree::worker::WorkerOwner::idle(agent))
        {
            return Err("quota arch owner is busy".into());
        }
        // A separate host can retain a detached writer absent from this pool.
        // Hold its existing slot leases through commit; never create workers.
        let crate::remote::protocol::Message::Request { arguments, .. } = &intent.request.payload
        else {
            return Err("invalid quota intent command".into());
        };
        let subtab = arguments["subtab_id"]
            .as_str()
            .ok_or("missing quota subtab")?;
        let session = intent
            .request
            .session_id
            .as_deref()
            .ok_or("missing quota session")?;
        let directory = self
            .sessions
            .get(project)
            .ok_or("missing quota master journal")?
            .parent()
            .ok_or("missing quota journal parent")?
            .join(".zenpi/workers");
        let mut leases = Vec::new();
        for slot in 0..crate::runtime::MAX_CONCURRENCY {
            let key = crate::worktree::worker::key(project, session, subtab, slot)?;
            if !self.worker_owners.contains_key(&key)
                && let Some(lease) = crate::worktree::worker::existing_lease(
                    &directory.join(format!("{key}.lease")),
                )?
            {
                leases.push(lease);
            }
        }
        self.commit_remote_subtabs(
            project,
            intent.candidate.clone(),
            device,
            id,
            &receipt.fingerprint,
        )
    }

    #[cfg(feature = "remote")]
    /// Persist the exact identity before a Git mutation. A crash or ambiguous
    /// result remains null/unknown and cannot automatically execute again.
    pub(crate) fn reserve_remote_subtab_git(
        &mut self,
        device: &str,
        request: &crate::remote::protocol::Envelope,
        fingerprint: &str,
        candidate: &crate::worktree::subtab::SubtabState,
        path: &Path,
        branch: Option<&str>,
    ) -> Result<(), String> {
        let request_id = request.id.as_deref().ok_or("missing worktree request ID")?;
        let project = request
            .project_id
            .as_deref()
            .ok_or("missing worktree project")?;
        if self
            .workspace_receipt(device, request_id, fingerprint)
            .map_err(|_| "workspace request identity conflict or capacity")?
            .is_some()
        {
            return Err("workspace request already reserved".into());
        }
        let mut remote = self.remote_workspace.clone();
        remote.receipts.push(WorkspaceReceipt {
            device_id: device.into(),
            request_id: request_id.into(),
            fingerprint: fingerprint.into(),
            result: serde_json::Value::Null,
            worktree_intent: Some(WorktreeIntent {
                request: request.clone(),
                before: self.subtab_state(project)?,
                candidate: candidate.clone(),
                path: path.into(),
                branch: branch.map(str::to_owned),
            }),
            quota_intent: None,
        });
        let bytes = self.core_bytes()?;
        self.checkpoint
            .as_mut()
            .ok_or("Git mutation requires durable checkpoint")?
            .commit(&bytes, Some(&remote.to_bytes()?))?;
        self.remote_workspace = remote;
        Ok(())
    }

    #[cfg(feature = "remote")]
    /// Local operator surface only. No capability secrets or executable replay.
    pub fn unresolved_worktree_intents(&self) -> Vec<serde_json::Value> {
        self.remote_workspace.receipts.iter().filter(|receipt| receipt.result.is_null() && receipt.quota_intent.is_none()).map(|receipt| {
            serde_json::json!({"device_id":receipt.device_id,"request_id":receipt.request_id,
                "fingerprint":receipt.fingerprint,"intent":receipt.worktree_intent.as_ref().map(|intent|
                    serde_json::json!({"project_id":intent.request.project_id,"session_id":intent.request.session_id,
                        "operation":if intent.branch.is_some() { "create" } else { "remove" },
                        "path":intent.path,"branch":intent.branch,"subtab_revision":intent.before.revision,
                        "workspace_revision":intent.request.expected_revision})),"state":"unknown_outcome"})
        }).collect()
    }

    #[cfg(feature = "remote")]
    /// Accept the observed filesystem result, never rerun Git or delete data.
    /// A caller must be the local operator, not an authenticated remote peer.
    pub fn reconcile_worktree_intent(
        &mut self,
        device: &str,
        request_id: &str,
    ) -> Result<serde_json::Value, String> {
        let receipt = self
            .remote_workspace
            .receipts
            .iter()
            .find(|receipt| receipt.device_id == device && receipt.request_id == request_id)
            .cloned()
            .ok_or("unknown worktree request")?;
        if !receipt.result.is_null() {
            return Ok(receipt.result);
        }
        let intent = receipt
            .worktree_intent
            .as_ref()
            .ok_or("legacy unknown receipt has no recoverable Git intent")?;
        let project = intent
            .request
            .project_id
            .as_deref()
            .ok_or("missing intent project")?;
        let base = self
            .workspace
            .tabs()
            .iter()
            .find(|tab| tab.id().as_str() == project)
            .ok_or("intent project is closed")?
            .cwd()
            .to_path_buf();
        intent.validate(&receipt, &base)?;
        if self
            .workspace
            .active()
            .is_none_or(|tab| tab.id().as_str() != project)
        {
            return Err("worktree intent project is not active".into());
        }
        if self.context(project).is_none_or(|context| {
            Some(context.session_id.as_str()) != intent.request.session_id.as_deref()
        }) || self.workspace_revision()
            != intent
                .request
                .expected_revision
                .ok_or("missing intent revision")?
            || self.subtab_state(project)? != intent.before
        {
            return Err(
                "worktree intent conflicts with current session or workspace/subtab revision"
                    .into(),
            );
        }
        let owner = self.owner(project).ok_or("missing intent owner")?;
        let guard = owner.try_lock().map_err(|_| "worktree owner is busy")?;
        if guard.phase() != crate::core::AgentPhase::Idle
            || guard.input_port().has_pending()
            || guard.input_port().is_preparing()
        {
            return Err("worktree owner is busy".into());
        }
        let arch = self.existing_arch_owner(project);
        let arch_guard = arch
            .as_ref()
            .map(|owner| owner.try_lock().map_err(|_| "worktree arch owner is busy"))
            .transpose()?;
        if arch_guard.as_ref().is_some_and(|owner| {
            owner.phase() != crate::core::AgentPhase::Idle
                || owner.input_port().has_pending()
                || owner.input_port().is_preparing()
        }) {
            return Err("worktree arch owner is busy".into());
        }
        let entries = list_worktrees(&base)?;
        if let Some(branch) = &intent.branch {
            let entry = entries
                .iter()
                .find(|entry| entry.path == intent.path)
                .ok_or("created worktree is not registered; no action replayed")?;
            if entry.branch.as_deref() != Some(branch) || entry.detached {
                return Err("created worktree branch differs from intent".into());
            }
            validate_registered_worktree(&base, &intent.path)?;
        } else {
            // Even an absent leaf is unsafe evidence beneath a replaced
            // managed-directory symlink; validate the complete boundary.
            managed_worktree_path(&base, &intent.path, false)?;
            if std::fs::symlink_metadata(&intent.path).is_ok()
                || entries.iter().any(|entry| entry.path == intent.path)
            {
                return Err(
                    "removed worktree still exists or is registered; no deletion attempted".into(),
                );
            }
            if !matches!(std::fs::symlink_metadata(&intent.path), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
            {
                return Err("worktree absence cannot be proved".into());
            }
        }
        self.commit_remote_subtabs(
            project,
            intent.candidate.clone(),
            device,
            request_id,
            &receipt.fingerprint,
        )
    }
    pub fn workspace(&self) -> &ProjectWorkspace {
        &self.workspace
    }
    #[cfg(feature = "remote")]
    pub fn workspace_owner(&self) -> &str {
        &self.remote_owner
    }
    pub fn workspace_revision(&self) -> u64 {
        self.revision
    }
    #[cfg(feature = "remote")]
    fn core_bytes(&self) -> Result<Vec<u8>, String> {
        core_checkpoint(
            &self.workspace,
            &self.sessions,
            self.revision,
            &self.subtabs,
        )
    }
    pub fn has_workspace_checkpoint(&self) -> bool {
        self.checkpoint.is_some()
    }
    #[cfg(feature = "remote")]
    pub(crate) fn prompt_binding(
        &self,
        device: &str,
        request: &str,
        fingerprint: &str,
    ) -> Result<bool, crate::remote::protocol::ErrorCode> {
        self.owner_request_binding(
            device,
            request,
            fingerprint,
            crate::remote::protocol::Operation::Prompt,
        )
    }
    #[cfg(feature = "remote")]
    pub(crate) fn owner_request_binding(
        &self,
        device: &str,
        request: &str,
        fingerprint: &str,
        operation: crate::remote::protocol::Operation,
    ) -> Result<bool, crate::remote::protocol::ErrorCode> {
        use crate::remote::protocol::ErrorCode;
        self.reject_steer_identity(device, request)?;
        if self
            .remote_workspace
            .receipts
            .iter()
            .any(|r| r.device_id == device && r.request_id == request)
            || self
                .remote_workspace
                .cancel_receipts
                .iter()
                .any(|r| r.device_id == device && r.request_id == request)
            || self
                .remote_workspace
                .approval_receipts
                .iter()
                .any(|r| r.binding.device_id == device && r.binding.request_id == request)
        {
            return Err(ErrorCode::RequestIdConflict);
        }
        match self
            .remote_workspace
            .prompt_bindings
            .iter()
            .find(|r| r.device_id == device && r.request_id == request)
        {
            Some(r) if r.fingerprint == fingerprint && r.operation == operation => Ok(true),
            Some(_) => Err(ErrorCode::RequestIdConflict),
            None if self.remote_workspace.prompt_bindings.len() >= 128 => {
                Err(ErrorCode::ResourceLimit)
            }
            None => Ok(false),
        }
    }
    #[cfg(feature = "remote")]
    pub(crate) fn acknowledge_events(
        &mut self,
        device: &str,
        project: &str,
        cursor: &crate::remote::protocol::EventCursor,
    ) -> Result<crate::remote::protocol::EventCursor, crate::remote::protocol::ErrorCode> {
        use crate::remote::protocol::ErrorCode;
        let ack = EventAck {
            device_id: device.into(),
            project_id: project.into(),
            cursor: cursor.clone(),
        };
        if !ack.valid() {
            return Err(ErrorCode::InvalidRequest);
        }
        let mut remote = self.remote_workspace.clone();
        if let Some(existing) = remote.event_acks.iter_mut().find(|ack| {
            ack.device_id == device
                && ack.project_id == project
                && ack.cursor.session_id == cursor.session_id
        }) {
            if existing.cursor.next_sequence >= cursor.next_sequence {
                return Ok(existing.cursor.clone());
            }
            existing.cursor = cursor.clone();
        } else {
            if remote.event_acks.len() >= 128 {
                return Err(ErrorCode::ResourceLimit);
            }
            remote.event_acks.push(ack);
        }
        let bytes = self.core_bytes().map_err(|_| ErrorCode::InternalError)?;
        self.checkpoint
            .as_mut()
            .ok_or(ErrorCode::InvalidRequest)?
            .commit(
                &bytes,
                Some(&remote.to_bytes().map_err(|_| ErrorCode::InternalError)?),
            )
            .map_err(|_| ErrorCode::InternalError)?;
        self.remote_workspace = remote;
        Ok(cursor.clone())
    }

    #[cfg(feature = "remote")]
    pub(crate) fn bind_remote_prompt(
        &mut self,
        device: &str,
        request: &crate::remote::protocol::Envelope,
        fingerprint: &str,
    ) -> Result<(), String> {
        self.bind_owner_request(
            device,
            request,
            fingerprint,
            crate::remote::protocol::Operation::Prompt,
        )
    }
    #[cfg(feature = "remote")]
    pub(crate) fn bind_owner_request(
        &mut self,
        device: &str,
        request: &crate::remote::protocol::Envelope,
        fingerprint: &str,
        operation: crate::remote::protocol::Operation,
    ) -> Result<(), String> {
        let binding = PromptBinding {
            operation,
            device_id: device.into(),
            request_id: request.id.clone().ok_or("missing prompt ID")?,
            fingerprint: fingerprint.into(),
            project_id: request.project_id.clone().ok_or("missing prompt project")?,
            session_id: request.session_id.clone().ok_or("missing prompt session")?,
        };
        if !binding.valid() {
            return Err("invalid prompt binding".into());
        }
        if self
            .owner_request_binding(device, &binding.request_id, fingerprint, operation)
            .map_err(|_| "prompt identity conflict or capacity")?
        {
            return Ok(());
        }
        let mut remote = self.remote_workspace.clone();
        remote.prompt_bindings.push(binding);
        let bytes = self.core_bytes()?;
        self.checkpoint
            .as_mut()
            .ok_or("prompt requires durable checkpoint")?
            .commit(&bytes, Some(&remote.to_bytes()?))?;
        self.remote_workspace = remote;
        Ok(())
    }
    #[cfg(feature = "remote")]
    fn reject_prompt_identity(
        &self,
        device: &str,
        request: &str,
    ) -> Result<(), crate::remote::protocol::ErrorCode> {
        if self
            .remote_workspace
            .prompt_bindings
            .iter()
            .any(|r| r.device_id == device && r.request_id == request)
        {
            return Err(crate::remote::protocol::ErrorCode::RequestIdConflict);
        }
        Ok(())
    }
    #[cfg(feature = "remote")]
    pub(crate) fn workspace_receipt(
        &self,
        device: &str,
        request: &str,
        fingerprint: &str,
    ) -> Result<Option<serde_json::Value>, crate::remote::protocol::ErrorCode> {
        use crate::remote::protocol::ErrorCode;
        self.reject_steer_identity(device, request)?;
        self.reject_prompt_identity(device, request)?;
        if self
            .remote_workspace
            .approval_receipts
            .iter()
            .any(|receipt| {
                receipt.binding.device_id == device && receipt.binding.request_id == request
            })
        {
            return Err(ErrorCode::RequestIdConflict);
        }
        if self
            .remote_workspace
            .cancel_receipts
            .iter()
            .any(|receipt| receipt.device_id == device && receipt.request_id == request)
        {
            return Err(ErrorCode::RequestIdConflict);
        }
        match self
            .remote_workspace
            .receipts
            .iter()
            .find(|receipt| receipt.device_id == device && receipt.request_id == request)
        {
            Some(receipt) if receipt.fingerprint == fingerprint => Ok(Some(receipt.result.clone())),
            Some(_) => Err(ErrorCode::RequestIdConflict),
            None if self.remote_workspace.receipts.len() >= 128 => Err(ErrorCode::ResourceLimit),
            None => Ok(None),
        }
    }
    #[cfg(feature = "remote")]
    pub(crate) fn cancel_receipt(
        &self,
        device: &str,
        request: &str,
        fingerprint: &str,
    ) -> Result<
        Option<(
            crate::remote::protocol::ExecutionState,
            serde_json::Value,
            Option<crate::remote::protocol::ErrorCode>,
        )>,
        crate::remote::protocol::ErrorCode,
    > {
        use crate::remote::protocol::ErrorCode;
        self.reject_steer_identity(device, request)?;
        self.reject_prompt_identity(device, request)?;
        if self
            .remote_workspace
            .approval_receipts
            .iter()
            .any(|receipt| {
                receipt.binding.device_id == device && receipt.binding.request_id == request
            })
        {
            return Err(ErrorCode::RequestIdConflict);
        }
        if self
            .remote_workspace
            .receipts
            .iter()
            .any(|receipt| receipt.device_id == device && receipt.request_id == request)
        {
            return Err(ErrorCode::RequestIdConflict);
        }
        match self
            .remote_workspace
            .cancel_receipts
            .iter()
            .find(|receipt| receipt.device_id == device && receipt.request_id == request)
        {
            Some(receipt) if receipt.fingerprint == fingerprint => {
                Ok(Some((receipt.state, receipt.result(), receipt.code)))
            }
            Some(_) => Err(ErrorCode::RequestIdConflict),
            None if self.remote_workspace.cancel_receipts.len() >= 128 => {
                Err(ErrorCode::ResourceLimit)
            }
            None => Ok(None),
        }
    }
    #[cfg(feature = "remote")]
    pub(crate) fn approval_receipt(
        &self,
        device: &str,
        id: &str,
        fingerprint: &str,
    ) -> Result<
        Option<(
            crate::remote::protocol::ExecutionState,
            serde_json::Value,
            Option<crate::remote::protocol::ErrorCode>,
        )>,
        crate::remote::protocol::ErrorCode,
    > {
        use crate::remote::protocol::ErrorCode;
        self.reject_steer_identity(device, id)?;
        self.reject_prompt_identity(device, id)?;
        if self
            .remote_workspace
            .receipts
            .iter()
            .any(|r| r.device_id == device && r.request_id == id)
            || self
                .remote_workspace
                .cancel_receipts
                .iter()
                .any(|r| r.device_id == device && r.request_id == id)
        {
            return Err(ErrorCode::RequestIdConflict);
        }
        match self
            .remote_workspace
            .approval_receipts
            .iter()
            .find(|r| r.binding.device_id == device && r.binding.request_id == id)
        {
            Some(r) if r.binding.fingerprint == fingerprint => {
                Ok(Some((r.binding.state, r.result(), r.binding.code)))
            }
            Some(_) => Err(ErrorCode::RequestIdConflict),
            None if self.remote_workspace.approval_receipts.len() >= 128 => {
                Err(ErrorCode::ResourceLimit)
            }
            None => Ok(None),
        }
    }
    #[cfg(feature = "remote")]
    fn reject_steer_identity(
        &self,
        device: &str,
        request: &str,
    ) -> Result<(), crate::remote::protocol::ErrorCode> {
        if self
            .remote_workspace
            .steer_receipts
            .iter()
            .any(|r| r.device_id == device && r.request_id == request)
        {
            return Err(crate::remote::protocol::ErrorCode::RequestIdConflict);
        }
        Ok(())
    }
    #[cfg(feature = "remote")]
    pub(crate) fn steer_receipt(
        &self,
        device: &str,
        id: &str,
        fingerprint: &str,
    ) -> Result<
        Option<(
            crate::remote::protocol::ExecutionState,
            serde_json::Value,
            Option<crate::remote::protocol::ErrorCode>,
        )>,
        crate::remote::protocol::ErrorCode,
    > {
        use crate::remote::protocol::ErrorCode;
        self.reject_prompt_identity(device, id)?;
        if self
            .remote_workspace
            .receipts
            .iter()
            .any(|r| r.device_id == device && r.request_id == id)
            || self
                .remote_workspace
                .cancel_receipts
                .iter()
                .any(|r| r.device_id == device && r.request_id == id)
            || self
                .remote_workspace
                .approval_receipts
                .iter()
                .any(|r| r.binding.device_id == device && r.binding.request_id == id)
        {
            return Err(ErrorCode::RequestIdConflict);
        }
        match self
            .remote_workspace
            .steer_receipts
            .iter()
            .find(|r| r.device_id == device && r.request_id == id)
        {
            Some(r) if r.fingerprint == fingerprint => Ok(Some((r.state, r.result(), r.code))),
            Some(_) => Err(ErrorCode::RequestIdConflict),
            None if self.remote_workspace.steer_receipts.len() >= 128 => {
                Err(ErrorCode::ResourceLimit)
            }
            None => Ok(None),
        }
    }
    #[cfg(feature = "remote")]
    pub(crate) fn commit_steer_receipt(
        &mut self,
        device: &str,
        request: &crate::remote::protocol::Envelope,
        fingerprint: &str,
        target: &str,
        state: crate::remote::protocol::ExecutionState,
        code: Option<crate::remote::protocol::ErrorCode>,
    ) -> Result<(), String> {
        let receipt = CancelReceipt {
            device_id: device.into(),
            request_id: request.id.clone().ok_or("missing steering ID")?,
            fingerprint: fingerprint.into(),
            project_id: request.project_id.clone().ok_or("missing project")?,
            session_id: request.session_id.clone().ok_or("missing session")?,
            target_request_id: target.into(),
            state,
            code,
        };
        if !receipt.valid() {
            return Err("invalid steering receipt".into());
        }
        let mut remote = self.remote_workspace.clone();
        if let Some(old) = remote
            .steer_receipts
            .iter_mut()
            .find(|r| r.device_id == device && r.request_id == receipt.request_id)
        {
            if old.fingerprint != fingerprint
                || old.state != crate::remote::protocol::ExecutionState::UnknownOutcome
            {
                return Err("steering receipt conflict".into());
            }
            *old = receipt;
        } else {
            self.steer_receipt(device, &receipt.request_id, fingerprint)
                .map_err(|_| "steering identity conflict or capacity")?;
            remote.steer_receipts.push(receipt);
        }
        let bytes = self.core_bytes()?;
        self.checkpoint
            .as_mut()
            .ok_or("steering requires durable checkpoint")?
            .commit(&bytes, Some(&remote.to_bytes()?))?;
        self.remote_workspace = remote;
        Ok(())
    }
    #[cfg(feature = "remote")]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn commit_approval_receipt(
        &mut self,
        device: &str,
        request: &crate::remote::protocol::Envelope,
        fingerprint: &str,
        target: &str,
        digest: &str,
        decision: crate::approval::ApprovalDecision,
        state: crate::remote::protocol::ExecutionState,
        code: Option<crate::remote::protocol::ErrorCode>,
    ) -> Result<(), String> {
        let receipt = ApprovalReceipt {
            binding: CancelReceipt {
                device_id: device.into(),
                request_id: request.id.clone().ok_or("missing request ID")?,
                fingerprint: fingerprint.into(),
                project_id: request.project_id.clone().ok_or("missing project")?,
                session_id: request.session_id.clone().ok_or("missing session")?,
                target_request_id: target.into(),
                state,
                code,
            },
            digest: digest.into(),
            decision,
        };
        if !receipt.valid() {
            return Err("invalid approval receipt".into());
        }
        let mut remote = self.remote_workspace.clone();
        if let Some(old) = remote.approval_receipts.iter_mut().find(|r| {
            r.binding.device_id == device && r.binding.request_id == receipt.binding.request_id
        }) {
            if old.binding.fingerprint != fingerprint
                || old.binding.state != crate::remote::protocol::ExecutionState::UnknownOutcome
            {
                return Err("approval receipt conflict".into());
            }
            *old = receipt;
        } else {
            self.approval_receipt(device, &receipt.binding.request_id, fingerprint)
                .map_err(|_| "approval receipt conflict or capacity")?;
            remote.approval_receipts.push(receipt);
        }
        let bytes = self.core_bytes()?;
        self.checkpoint
            .as_mut()
            .ok_or("approval requires durable checkpoint")?
            .commit(&bytes, Some(&remote.to_bytes()?))?;
        self.remote_workspace = remote;
        Ok(())
    }
    #[cfg(feature = "remote")]
    pub(crate) fn commit_cancel_receipt(
        &mut self,
        device: &str,
        request: &crate::remote::protocol::Envelope,
        fingerprint: &str,
        target: &str,
        state: crate::remote::protocol::ExecutionState,
        code: Option<crate::remote::protocol::ErrorCode>,
    ) -> Result<(), String> {
        let receipt = CancelReceipt {
            device_id: device.into(),
            request_id: request.id.clone().ok_or("missing request ID")?,
            fingerprint: fingerprint.into(),
            project_id: request.project_id.clone().ok_or("missing project ID")?,
            session_id: request.session_id.clone().ok_or("missing session ID")?,
            target_request_id: target.into(),
            state,
            code,
        };
        if !receipt.valid() {
            return Err("invalid cancellation receipt".into());
        }
        let mut remote = self.remote_workspace.clone();
        if let Some(previous) = remote
            .cancel_receipts
            .iter_mut()
            .find(|old| old.device_id == device && old.request_id == receipt.request_id)
        {
            if previous.fingerprint != fingerprint
                || previous.state != crate::remote::protocol::ExecutionState::UnknownOutcome
            {
                return Err("cancellation receipt conflict".into());
            }
            *previous = receipt;
        } else {
            if remote.cancel_receipts.len() >= 128
                || remote
                    .receipts
                    .iter()
                    .any(|old| old.device_id == device && old.request_id == receipt.request_id)
            {
                return Err("cancellation receipt capacity or identity conflict".into());
            }
            remote.cancel_receipts.push(receipt);
        }
        let bytes = self.core_bytes()?;
        self.checkpoint
            .as_mut()
            .ok_or("cancellation requires durable checkpoint")?
            .commit(&bytes, Some(&remote.to_bytes()?))?;
        self.remote_workspace = remote;
        Ok(())
    }
    #[cfg(feature = "remote")]
    /// Commit the workspace and retry receipt in the same compare-and-replace checkpoint.
    pub(crate) fn publish_remote_workspace(
        &mut self,
        candidate: ProjectWorkspace,
        device: &str,
        request: &str,
        fingerprint: &str,
    ) -> Result<serde_json::Value, String> {
        if !self.has_workspace_checkpoint() || self.pending_remote_receipt.is_some() {
            return Err("remote workspace requires an available durable checkpoint".into());
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or("workspace revision exhausted")?;
        let result = serde_json::json!({"workspace_revision": revision,
            "active_project_id": candidate.active().map(|tab| tab.id().as_str()),
            "required_owner": self.remote_owner});
        self.pending_remote_receipt = Some(WorkspaceReceipt {
            device_id: device.into(),
            request_id: request.into(),
            fingerprint: fingerprint.into(),
            result: result.clone(),
            worktree_intent: None,
            quota_intent: None,
        });
        let published = self.publish(candidate, &Default::default());
        self.pending_remote_receipt = None;
        published?;
        Ok(result)
    }
    pub fn sessions(&self) -> &std::collections::BTreeMap<String, PathBuf> {
        &self.sessions
    }
    pub fn owner(&self, id: &str) -> Option<std::sync::Arc<std::sync::Mutex<crate::core::Agent>>> {
        self.owners.get(id).cloned()
    }

    /// Lazily prepare the independent arch master-session owner for a project
    /// (ZS1-156). It opens `<session dir>/arch.jsonl`, so it has its own
    /// journal, model, and approval coordinator distinct from the discussion
    /// owner.
    pub fn arch_agent(
        &mut self,
        id: &str,
    ) -> Result<std::sync::Arc<std::sync::Mutex<crate::core::Agent>>, String> {
        if let Some(existing) = self.arch_owners.get(id) {
            return Ok(existing.clone());
        }
        let agent = self.prepare_arch_owner(id)?;
        Ok(self.register_arch_owner(id, agent))
    }
    #[cfg(feature = "remote")]
    pub(crate) fn existing_arch_owner(
        &self,
        id: &str,
    ) -> Option<std::sync::Arc<std::sync::Mutex<crate::core::Agent>>> {
        self.arch_owners.get(id).cloned()
    }

    /// Build the arch owner for a project: an independent agent on
    /// `<session dir>/arch.jsonl` with the discussion owner's overrides and
    /// approval policy, labelled so connection commands addressed to `arch`
    /// reach this one and not the discussion owner.
    fn prepare_arch_owner(&self, id: &str) -> Result<crate::core::Agent, String> {
        let active = self
            .owners
            .get(id)
            .cloned()
            .ok_or("project owner not found")?;
        let (arch_session, cwd, overrides, echo_fixture) = {
            let agent = active.lock().map_err(|_| "project owner lock poisoned")?;
            let session = agent.session().path().to_path_buf();
            let arch_session = session.with_file_name("arch.jsonl");
            let cwd = agent
                .attachment_workspace_root()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from(agent.session().header().cwd.clone()));
            (
                arch_session,
                cwd,
                agent.project_overrides(),
                agent.backend_name() == "echo",
            )
        };
        let mut agent = crate::core::Agent::prepare_project_with_approval(
            &arch_session,
            &cwd,
            overrides,
            echo_fixture,
            self.auto_approve,
        )
        .map_err(|error| error.to_string())?;
        agent
            .set_owner_label("arch")
            .map_err(|error| error.to_string())?;
        Ok(agent)
    }

    /// Publish an arch owner and its control handles under the project id.
    /// Registered like any other owner: a host has to be able to drain and
    /// answer this lane's approvals without taking its execution lock.
    fn register_arch_owner(
        &mut self,
        id: &str,
        agent: crate::core::Agent,
    ) -> std::sync::Arc<std::sync::Mutex<crate::core::Agent>> {
        let control = OwnerControl {
            project: id.to_owned(),
            input_port: agent.input_port(),
            approval: agent.approval_coordinator(),
        };
        let handle = std::sync::Arc::new(std::sync::Mutex::new(agent));
        self.arch_owners.insert(id.to_owned(), handle.clone());
        self.control.insert(arch_owner_key(id), control);
        handle
    }

    /// Reset the arch master session for `/new` in the arch console: archive
    /// the old journal and prepare a fresh owner at the canonical
    /// `arch.jsonl` path, so the reset survives host restarts. Refuses while
    /// a master turn is running.
    pub fn reset_arch(&mut self, id: &str) -> Result<(), String> {
        let existing = self
            .arch_owners
            .get(id)
            .cloned()
            .ok_or("arch owner not found")?;
        let old_path = {
            let agent = existing.lock().map_err(|_| "arch owner lock poisoned")?;
            if agent.phase() == crate::core::AgentPhase::Running {
                return Err("arch session is running; cancel or wait before /new".into());
            }
            agent.session().path().to_path_buf()
        };
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let archive = old_path.with_file_name(format!("arch-{stamp}.jsonl"));
        let renamed = old_path.exists();
        if renamed {
            std::fs::rename(&old_path, &archive).map_err(|e| e.to_string())?;
        }
        match self.prepare_arch_owner(id) {
            Ok(fresh) => {
                if let Ok(mut old) = existing.lock() {
                    old.close();
                }
                self.register_arch_owner(id, fresh);
                Ok(())
            }
            Err(error) => {
                // Keep the old owner reachable and put its journal back.
                if renamed {
                    let _ = std::fs::rename(&archive, &old_path);
                }
                Err(error)
            }
        }
    }

    pub fn close_all(&self) {
        for owner in self.owners.values() {
            if let Ok(mut agent) = owner.lock() {
                agent.close();
            }
        }
        for owner in self.arch_owners.values() {
            if let Ok(mut agent) = owner.lock() {
                agent.close();
            }
        }
        #[cfg(feature = "remote")]
        for worker in self.worker_owners.values() {
            if let Ok(mut agent) = worker.agent().lock() {
                agent.close();
            }
        }
    }
    pub fn owner_handles(
        &self,
    ) -> Vec<(String, std::sync::Arc<std::sync::Mutex<crate::core::Agent>>)> {
        self.owners
            .iter()
            .map(|(id, owner)| (id.clone(), owner.clone()))
            .collect()
    }
    /// Control handles for every project owner, keyed by project id.  Unlike
    /// [`Self::owner_handles`] these need no lock, so they stay reachable
    /// while an owner is running or waiting for an approval.
    pub fn control_handles(&self) -> Vec<(String, OwnerControl)> {
        self.control
            .iter()
            .map(|(id, control)| (id.clone(), control.clone()))
            .collect()
    }
    /// Control handles for one project owner.  `None` means no owner is
    /// published under that id -- it never means the owner is busy.
    pub fn control(&self, id: &str) -> Option<OwnerControl> {
        self.control.get(id).cloned()
    }
    pub fn active(&self) -> std::sync::Arc<std::sync::Mutex<crate::core::Agent>> {
        self.owner(
            self.workspace
                .active()
                .expect("committed project")
                .id()
                .as_str(),
        )
        .expect("prepared owner")
    }
    /// Prepare configuration, durable session and tool cwd before publishing a
    /// new active ID. Existing running owners need no lock for selection.
    pub fn publish(
        &mut self,
        candidate: ProjectWorkspace,
        saved_sessions: &std::collections::BTreeMap<String, PathBuf>,
    ) -> Result<(), String> {
        let tab = candidate.active().ok_or("no active project")?;
        #[cfg(feature = "remote")]
        let mut remote_workspace = self.remote_workspace.clone();
        let id = tab.id().as_str().to_owned();
        let mut subtabs = self.subtabs.clone();
        subtabs.retain(|id, _| candidate.tabs().iter().any(|tab| tab.id().as_str() == id));
        let mut next_sessions = std::collections::BTreeMap::new();
        for tab in candidate.tabs() {
            let key = tab.id().as_str();
            let path = saved_sessions
                .get(key)
                .or_else(|| self.sessions.get(key))
                .or_else(|| (key == self.initial_session.0).then_some(&self.initial_session.1))
                .cloned()
                .unwrap_or_else(|| self.session_root.join(key).join("session.jsonl"));
            let path = if path.is_absolute() {
                path
            } else {
                std::env::current_dir()
                    .map_err(|e| e.to_string())?
                    .join(path)
            };
            validate_path_text(&path).map_err(|e| e.to_string())?;
            next_sessions.insert(key.to_owned(), path);
        }
        // An explicit session resume may change a retained project's journal.
        // Restore its actual Agent instead of keeping the startup handle and
        // publishing only the saved label/session metadata.
        let replaced_owners: Vec<_> = self
            .owners
            .iter()
            .filter(|(key, _)| next_sessions.get(*key) != self.sessions.get(*key))
            .map(|(_, owner)| owner.clone())
            .chain(
                self.arch_owners
                    .iter()
                    .filter(|(key, _)| next_sessions.get(*key) != self.sessions.get(*key))
                    .map(|(_, owner)| owner.clone()),
            )
            .collect();
        let mut replaced_guards = Vec::new();
        for owner in &replaced_owners {
            let agent = owner
                .try_lock()
                .map_err(|_| "Project is running; cancel or wait before replacing it")?;
            if agent.phase() == crate::core::AgentPhase::Running
                || agent.input_port().has_pending()
                || agent.input_port().is_preparing()
            {
                return Err("Project is running".into());
            }
            replaced_guards.push(agent);
        }
        #[cfg(feature = "remote")]
        let replaced_workers: Vec<_> = self
            .worker_owners
            .iter()
            .filter(|(_, worker)| {
                next_sessions.get(&worker.control().project)
                    != self.sessions.get(&worker.control().project)
            })
            .map(|(key, worker)| (key.clone(), worker.clone()))
            .collect();
        #[cfg(feature = "remote")]
        let mut worker_guards = Vec::new();
        #[cfg(feature = "remote")]
        for (_, worker) in &replaced_workers {
            let agent = worker
                .agent()
                .try_lock()
                .map_err(|_| "project worker is busy")?;
            if worker.runtime_owned() || !crate::worktree::worker::WorkerOwner::idle(&agent) {
                return Err("project worker is busy".into());
            }
            worker_guards.push(agent);
        }
        let prepared =
            if self.owners.contains_key(&id) && self.sessions.get(&id) == next_sessions.get(&id) {
                None
            } else {
                Some(
                    crate::core::Agent::prepare_project_with_approval(
                        next_sessions.get(&id).ok_or("missing project session")?,
                        tab.cwd(),
                        self.overrides.clone(),
                        self.echo_fixture,
                        self.auto_approve,
                    )
                    .map_err(|e| e.to_string())?,
                )
            };
        #[cfg(feature = "remote")]
        let pending_receipt = self.pending_remote_receipt.is_some();
        #[cfg(not(feature = "remote"))]
        let pending_receipt = false;
        let changed =
            candidate != self.workspace || next_sessions != self.sessions || pending_receipt;
        #[cfg(feature = "remote")]
        let previous_sessions: Vec<_> = if changed {
            self.contexts
                .values()
                .map(|context| context.session_id.clone())
                .collect()
        } else {
            Vec::new()
        };
        let mut revision = self.revision;
        if changed {
            revision = revision
                .checked_add(1)
                .ok_or("workspace revision exhausted")?;
        }
        #[cfg(feature = "remote")]
        if let Some(receipt) = &self.pending_remote_receipt {
            if remote_workspace.receipts.len() >= 128 {
                return Err("workspace receipt capacity exhausted".into());
            }
            remote_workspace.receipts.push(receipt.clone());
        }
        if let Some(checkpoint) = &mut self.checkpoint {
            let bytes = core_checkpoint(&candidate, &next_sessions, revision, &subtabs)?;
            #[cfg(feature = "remote")]
            let remote = self
                .pending_remote_receipt
                .is_some()
                .then(|| remote_workspace.to_bytes())
                .transpose()?;
            #[cfg(not(feature = "remote"))]
            let remote: Option<Vec<u8>> = None;
            checkpoint.commit(&bytes, remote.as_deref())?;
        }
        for agent in &mut replaced_guards {
            agent.close();
        }
        #[cfg(feature = "remote")]
        for mut agent in worker_guards {
            agent.close();
        }
        #[cfg(feature = "remote")]
        for (key, _) in &replaced_workers {
            self.worker_owners.remove(key);
        }
        self.owners.retain(|key, owner| {
            let keep = next_sessions.get(key) == self.sessions.get(key);
            if !keep && let Ok(mut agent) = owner.try_lock() {
                agent.close();
            }
            keep
        });
        self.contexts.retain(|id, _| self.owners.contains_key(id));
        // An arch owner carries its own key but answers for its project, so
        // retention follows the project, not the key.
        self.control
            .retain(|_, control| self.owners.contains_key(&control.project));
        self.arch_owners.retain(|key, owner| {
            let keep =
                self.owners.contains_key(key) && next_sessions.get(key) == self.sessions.get(key);
            if !keep && let Ok(mut agent) = owner.try_lock() {
                agent.close();
            }
            keep
        });
        if let Some(agent) = prepared {
            self.contexts
                .insert(id.clone(), ProjectContext::from_agent(&id, &agent));
            // Published here, while the owner is still exclusively ours, so a
            // host never has to take its execution lock to reach it later.
            self.control.insert(
                id.clone(),
                OwnerControl {
                    project: id.clone(),
                    input_port: agent.input_port(),
                    approval: agent.approval_coordinator(),
                },
            );
            self.owners
                .insert(id, std::sync::Arc::new(std::sync::Mutex::new(agent)));
        }
        self.sessions = next_sessions;
        self.workspace = candidate;
        self.revision = revision;
        #[cfg(feature = "remote")]
        {
            self.remote_workspace = remote_workspace;
        }
        self.subtabs = subtabs;
        if changed {
            #[cfg(feature = "remote")]
            for session in previous_sessions {
                crate::remote::wakeup::notify(&session);
            }
            self.notify_workspace_sessions();
        }
        Ok(())
    }
    fn notify_workspace_sessions(&self) {
        #[cfg(feature = "remote")]
        for context in self.contexts.values() {
            crate::remote::wakeup::notify(&context.session_id);
        }
    }
    pub fn open(&mut self, cwd: Option<&Path>) -> Result<OpenOutcome, String> {
        let base = self.workspace.active().ok_or("no active project")?.cwd();
        let (candidate, outcome) = self
            .workspace
            .with_directory(cwd, base)
            .map_err(|e| e.to_string())?;
        self.publish(candidate, &Default::default())?;
        Ok(outcome)
    }
    pub fn select(&mut self, id: &str) -> Result<(), String> {
        let tab = self
            .workspace
            .tabs()
            .iter()
            .find(|tab| tab.id().as_str() == id)
            .ok_or("project not found")?;
        let candidate = self
            .workspace
            .with_active(tab.id())
            .map_err(|e| e.to_string())?;
        self.publish(candidate, &Default::default())
    }
    pub fn close(&mut self, id: &str) -> Result<(), String> {
        let tab = self
            .workspace
            .tabs()
            .iter()
            .find(|tab| tab.id().as_str() == id)
            .ok_or("project not found")?;
        let candidate = self
            .workspace
            .without_project(tab.id())
            .map_err(|e| e.to_string())?;
        self.publish(candidate, &Default::default())
    }
}

/// Refuse links and oversized input before parsing or allocating its contents.
pub(crate) fn read_owner_checkpoint(path: &Path) -> Result<Option<Vec<u8>>, String> {
    use std::io::Read;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    // Validate the opened object, not a pathname that can change before open.
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_PROJECT_WORKSPACE_BYTES as u64 {
        return Err("invalid or oversized project owner checkpoint".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_PROJECT_WORKSPACE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_PROJECT_WORKSPACE_BYTES {
        return Err("project checkpoint exceeds byte limit".into());
    }
    Ok(Some(bytes))
}
/// Remote receipts, acknowledgements and intents live beside the project
/// checkpoint, never inside it (ZE-510): a build without the remote feature
/// reads `project-workspace.json` alone and never sees remote state.
pub(crate) const REMOTE_STATE_FILE: &str = "remote-state.json";

fn core_checkpoint(
    workspace: &impl Serialize,
    sessions: &std::collections::BTreeMap<String, PathBuf>,
    revision: u64,
    subtabs: &std::collections::BTreeMap<String, crate::worktree::subtab::SubtabState>,
) -> Result<Vec<u8>, String> {
    serde_json::to_vec(
        &serde_json::json!({"schema_version":1, "workspace":workspace,
        "sessions":sessions, "workspace_revision":revision, "subtabs":subtabs}),
    )
    .map_err(|e| e.to_string())
}

/// Compare-and-replace baselines of the two owner checkpoint files.
#[derive(Debug, Clone)]
struct OwnerCheckpoint {
    path: PathBuf,
    previous: Option<Vec<u8>>,
    remote_previous: Option<Vec<u8>>,
}

impl OwnerCheckpoint {
    /// Replace the core checkpoint and, when given, the remote state, under
    /// one lock; an unchanged file is only checked. Core lands first: a crash between the two renames leaves a remote
    /// reservation unresolved, an unknown outcome, never a remote result the
    /// workspace does not show.
    fn commit(&mut self, core: &[u8], remote: Option<&[u8]>) -> Result<(), String> {
        let remote_path = self.path.with_file_name(REMOTE_STATE_FILE);
        let mut writes = vec![(self.path.as_path(), self.previous.as_deref(), core)];
        if let Some(bytes) = remote {
            writes.push((
                remote_path.as_path(),
                self.remote_previous.as_deref(),
                bytes,
            ));
        }
        write_owner_files(&writes)?;
        self.previous = Some(core.to_vec());
        if let Some(bytes) = remote {
            self.remote_previous = Some(bytes.to_vec());
        }
        Ok(())
    }
}

#[cfg(test)]
fn write_owner_checkpoint(
    path: &Path,
    previous: Option<&[u8]>,
    bytes: &[u8],
) -> Result<(), String> {
    write_owner_files(&[(path, previous, bytes)])
}

/// Each write is `(path, expected current bytes, replacement)`; a replacement
/// equal to the current bytes is only checked. The first path's lock guards them all,
/// every file is checked before any is staged, and every replacement is
/// durable before the first rename.
type OwnerFileWrite<'a> = (&'a Path, Option<&'a [u8]>, &'a [u8]);
fn write_owner_files(writes: &[OwnerFileWrite<'_>]) -> Result<(), String> {
    use std::io::Write;
    let (first, _, _) = writes.first().ok_or("no project checkpoint to write")?;
    if writes
        .iter()
        .any(|(_, _, bytes)| bytes.len() > MAX_PROJECT_WORKSPACE_BYTES)
    {
        return Err("project checkpoint exceeds byte limit".into());
    }
    let lock_path = first.with_extension("lock");
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let lock = options.open(&lock_path).map_err(|e| e.to_string())?;
    if !lock
        .metadata()
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("invalid project checkpoint lock".into());
    }
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("project checkpoint is being updated".into());
        }
    }
    struct Unlock(std::fs::File);
    impl Drop for Unlock {
        fn drop(&mut self) {
            #[cfg(unix)]
            {
                use std::os::fd::AsRawFd;
                unsafe {
                    libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
                }
            }
        }
    }
    let _guard = Unlock(lock);
    for (path, previous, _) in writes {
        if read_owner_checkpoint(path)?.as_deref() != *previous {
            return Err(
                "project checkpoint changed in another host; reopen the host before modifying projects"
                    .into(),
            );
        }
    }
    let mut staged = Vec::new();
    let result = (|| -> std::io::Result<()> {
        for (path, previous, bytes) in writes {
            if Some(*bytes) == *previous {
                continue;
            }
            let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary)?;
            staged.push((temporary, *path));
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        for (temporary, path) in &staged {
            std::fs::rename(temporary, path)?;
        }
        staged.clear();
        std::fs::File::open(
            first
                .parent()
                .ok_or_else(|| std::io::Error::other("missing checkpoint parent"))?,
        )?
        .sync_all()?;
        Ok(())
    })();
    for (temporary, _) in staged {
        let _ = std::fs::remove_file(temporary);
    }
    result.map_err(|e| e.to_string())
}

#[cfg(all(test, unix))]
mod checkpoint_file_tests {
    use super::*;
    use std::os::unix::fs::{FileTypeExt, symlink};

    fn fifo(path: &Path) {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    }

    #[test]
    fn checkpoint_descriptor_refuses_nonfiles_and_oversized_inputs() {
        let temp = tempfile::tempdir().unwrap();
        let checkpoint = temp.path().join("checkpoint.json");
        assert_eq!(read_owner_checkpoint(&checkpoint).unwrap(), None);
        std::fs::write(&checkpoint, b"original").unwrap();
        assert_eq!(
            read_owner_checkpoint(&checkpoint).unwrap(),
            Some(b"original".to_vec())
        );
        let link = temp.path().join("link.json");
        symlink(&checkpoint, &link).unwrap();
        let directory = temp.path().join("directory");
        std::fs::create_dir(&directory).unwrap();
        for candidate in [&link, &directory] {
            assert!(read_owner_checkpoint(candidate).is_err());
        }
        let huge = temp.path().join("oversized.json");
        std::fs::File::create(&huge)
            .unwrap()
            .set_len(MAX_PROJECT_WORKSPACE_BYTES as u64 + 1)
            .unwrap();
        assert!(read_owner_checkpoint(&huge).is_err());
        assert_eq!(std::fs::read(&checkpoint).unwrap(), b"original");
    }

    #[test]
    fn checkpoint_lock_refuses_nonfiles_without_publication() {
        let temp = tempfile::tempdir().unwrap();
        let checkpoint = temp.path().join("checkpoint.json");
        std::fs::write(&checkpoint, b"original").unwrap();
        let lock = checkpoint.with_extension("lock");
        for kind in ["directory", "symlink"] {
            match kind {
                "directory" => std::fs::create_dir(&lock).unwrap(),
                _ => symlink(&checkpoint, &lock).unwrap(),
            }
            assert!(write_owner_checkpoint(&checkpoint, Some(b"original"), b"next").is_err());
            assert_eq!(std::fs::read(&checkpoint).unwrap(), b"original");
            assert!(
                !checkpoint
                    .with_extension(format!("{}.tmp", std::process::id()))
                    .exists()
            );
            if kind == "directory" {
                std::fs::remove_dir(&lock).unwrap();
            } else {
                std::fs::remove_file(&lock).unwrap();
            }
        }
        write_owner_checkpoint(&checkpoint, Some(b"original"), b"next").unwrap();
        assert_eq!(
            read_owner_checkpoint(&checkpoint).unwrap(),
            Some(b"next".to_vec())
        );
        assert!(write_owner_checkpoint(&checkpoint, Some(b"original"), b"stale").is_err());
        assert_eq!(std::fs::read(&checkpoint).unwrap(), b"next");
    }

    #[test]
    fn checkpoint_fifo_refusal_is_bounded_in_a_child_process() {
        const CHILD: &str = "ZENPI_CHECKPOINT_FIFO_CHILD";
        if let Some(root) = std::env::var_os(CHILD) {
            let root = PathBuf::from(root);
            assert!(read_owner_checkpoint(&root.join("read-fifo")).is_err());
            assert!(
                write_owner_checkpoint(&root.join("checkpoint.json"), Some(b"original"), b"next")
                    .is_err()
            );
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        fifo(&temp.path().join("read-fifo"));
        fifo(&temp.path().join("checkpoint.lock"));
        std::fs::write(temp.path().join("checkpoint.json"), b"original").unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "project_workspace::checkpoint_file_tests::checkpoint_fifo_refusal_is_bounded_in_a_child_process"])
            .env(CHILD, temp.path()).spawn().unwrap();
        let started = std::time::Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            if started.elapsed() >= std::time::Duration::from_secs(3) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("checkpoint FIFO refusal blocked");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(
            std::fs::read(temp.path().join("checkpoint.json")).unwrap(),
            b"original"
        );
        for name in ["read-fifo", "checkpoint.lock"] {
            assert!(
                std::fs::symlink_metadata(temp.path().join(name))
                    .unwrap()
                    .file_type()
                    .is_fifo()
            );
        }
    }
}

/// Maximum number of layer-2 sub-tabs retained per project.
pub const MAX_PROJECT_SUBTABS: usize = 32;
const MAX_WORKTREE_OUTPUT_BYTES: usize = 256 * 1024;
const WORKTREE_GIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// One git worktree discovered for a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeEntry {
    pub path: std::path::PathBuf,
    pub branch: Option<String>,
    pub detached: bool,
}

fn run_git(project: &Path, args: &[&str], deadline: std::time::Instant) -> Result<String, String> {
    let mut command = std::process::Command::new("git");
    command.arg("-C").arg(project).args(args).env_clear();
    for (key, value) in crate::security::child_environment() {
        command.env(key, value);
    }
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null");
    bounded_worktree_git(&mut command, deadline)
}

#[cfg(unix)]
fn bounded_worktree_git(
    command: &mut std::process::Command,
    deadline: std::time::Instant,
) -> Result<String, String> {
    use std::{io::Read, os::unix::process::CommandExt, process::Stdio, time::Instant};
    let expired =
        || "git deadline exceeded; filesystem outcome may require reconciliation".to_owned();
    if Instant::now() >= deadline {
        return Err(expired());
    }
    command
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = crate::tools::SupervisedChild::new(command.spawn().map_err(|e| e.to_string())?);
    let mut stdout = child.child.stdout.take().ok_or("git stdout unavailable")?;
    let mut stderr = child.child.stderr.take().ok_or("git stderr unavailable")?;
    crate::tools::make_pipe_nonblocking(&stdout).map_err(|e| e.to_string())?;
    crate::tools::make_pipe_nonblocking(&stderr).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut out_eof = false;
    let mut err_eof = false;
    loop {
        if Instant::now() >= deadline {
            return Err(expired());
        }
        // Alternate bounded reads so a flooding stream cannot starve its peer
        // or the deadline. The owned group is reaped on every return path.
        for (pipe, bytes, eof) in [
            (&mut stdout as &mut dyn Read, &mut out, &mut out_eof),
            (&mut stderr as &mut dyn Read, &mut err, &mut err_eof),
        ] {
            if *eof {
                continue;
            }
            for _ in 0..8 {
                let mut chunk = [0u8; 8192];
                match pipe.read(&mut chunk) {
                    Ok(0) => {
                        *eof = true;
                        break;
                    }
                    Ok(n) => {
                        if n > MAX_WORKTREE_OUTPUT_BYTES.saturating_sub(bytes.len()) {
                            return Err("git output exceeded the bound; filesystem outcome may require reconciliation".into());
                        }
                        bytes.extend_from_slice(&chunk[..n]);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e.to_string()),
                }
            }
        }
        if out_eof
            && err_eof
            && let Some(status) = child.child.try_wait().map_err(|e| e.to_string())?
        {
            child.cleanup();
            if !status.success() {
                let reason = String::from_utf8_lossy(&err).trim().to_owned();
                return Err(if reason.is_empty() {
                    format!("git failed with {status}")
                } else {
                    reason
                });
            }
            return String::from_utf8(out).map_err(|_| "git output is not UTF-8".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

#[cfg(not(unix))]
fn bounded_worktree_git(
    _command: &mut std::process::Command,
    _deadline: std::time::Instant,
) -> Result<String, String> {
    Err("managed worktree Git requires Unix process containment".into())
}

/// Read-only `git worktree list` for one project.
pub fn list_worktrees(project: &Path) -> Result<Vec<WorktreeEntry>, String> {
    list_worktrees_before(project, std::time::Instant::now() + WORKTREE_GIT_TIMEOUT)
}

fn list_worktrees_before(
    project: &Path,
    deadline: std::time::Instant,
) -> Result<Vec<WorktreeEntry>, String> {
    let text = run_git(project, &["worktree", "list", "--porcelain"], deadline)?;
    let mut entries = Vec::new();
    let mut path: Option<std::path::PathBuf> = None;
    let mut detached = false;
    let mut branch: Option<String> = None;
    let flush = |path: &mut Option<std::path::PathBuf>,
                 detached: &mut bool,
                 branch: &mut Option<String>,
                 entries: &mut Vec<WorktreeEntry>| {
        if let Some(path) = path.take() {
            entries.push(WorktreeEntry {
                path,
                branch: branch.take(),
                detached: std::mem::take(detached),
            });
        }
    };
    for line in text.lines().chain(std::iter::once("")) {
        if let Some(value) = line.strip_prefix("worktree ") {
            flush(&mut path, &mut detached, &mut branch, &mut entries);
            path = Some(std::path::PathBuf::from(value));
        } else if line.strip_prefix("branch ").is_some() {
            branch = line
                .strip_prefix("branch ")
                .map(|value| value.trim_start_matches("refs/heads/").to_owned());
        } else if line == "detached" {
            detached = true;
        }
    }
    flush(&mut path, &mut detached, &mut branch, &mut entries);
    Ok(entries)
}

/// Create a new worktree at `path` on a fresh branch `branch`.
pub fn add_worktree(project: &Path, path: &Path, branch: &str) -> Result<(), String> {
    let deadline = std::time::Instant::now() + WORKTREE_GIT_TIMEOUT;
    let (project, path) = managed_worktree_path(project, path, false)?;
    if branch.is_empty()
        || branch.len() > 128
        || branch.starts_with('-')
        || branch.chars().any(char::is_control)
    {
        return Err("invalid worktree branch".into());
    }
    run_git(
        &project,
        &["check-ref-format", "--branch", branch],
        deadline,
    )?;
    let path = path.to_str().ok_or("worktree path is not utf-8")?;
    run_git(&project, &["worktree", "add", "-b", branch, path], deadline).map(|_| ())
}

/// Remove only a registered, clean, unlocked managed worktree. Never force data loss.
pub fn remove_worktree(project: &Path, path: &Path) -> Result<(), String> {
    let deadline = std::time::Instant::now() + WORKTREE_GIT_TIMEOUT;
    validate_worktree_removal_before(project, path, deadline)?;
    let (project, path) = managed_worktree_path(project, path, true)?;
    let path = path.to_str().ok_or("worktree path is not utf-8")?;
    run_git(&project, &["worktree", "remove", "--", path], deadline).map(|_| ())
}

pub fn validate_worktree_removal(project: &Path, path: &Path) -> Result<(), String> {
    validate_worktree_removal_before(
        project,
        path,
        std::time::Instant::now() + WORKTREE_GIT_TIMEOUT,
    )
}

fn validate_worktree_removal_before(
    project: &Path,
    path: &Path,
    deadline: std::time::Instant,
) -> Result<(), String> {
    validate_registered_worktree_before(project, path, deadline)?;
    // Git's normal cleanliness check may ignore ignored files; retain those too.
    if !run_git(
        path,
        &[
            "status",
            "--porcelain",
            "--untracked-files=all",
            "--ignored",
        ],
        deadline,
    )?
    .trim()
    .is_empty()
    {
        return Err("worktree contains modified, staged, untracked or ignored files".into());
    }
    Ok(())
}

#[cfg(feature = "remote")]
fn validate_registered_worktree(project: &Path, path: &Path) -> Result<(), String> {
    validate_registered_worktree_before(
        project,
        path,
        std::time::Instant::now() + WORKTREE_GIT_TIMEOUT,
    )
}

fn validate_registered_worktree_before(
    project: &Path,
    path: &Path,
    deadline: std::time::Instant,
) -> Result<(), String> {
    let (project, path) = managed_worktree_path(project, path, true)?;
    if !list_worktrees_before(&project, deadline)?
        .iter()
        .any(|entry| entry.path == path)
    {
        return Err("directory is not a registered project worktree".into());
    }
    let project_common = run_git(
        &project,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        deadline,
    )?;
    let target_common = run_git(
        &path,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        deadline,
    )?;
    if Path::new(project_common.trim())
        .canonicalize()
        .map_err(|e| e.to_string())?
        != Path::new(target_common.trim())
            .canonicalize()
            .map_err(|e| e.to_string())?
    {
        return Err("worktree belongs to another repository".into());
    }
    Ok(())
}

fn managed_worktree_path(
    project: &Path,
    path: &Path,
    must_exist: bool,
) -> Result<(PathBuf, PathBuf), String> {
    let original_project = project;
    let project = project.canonicalize().map_err(|e| e.to_string())?;
    let path = if let Ok(relative) = path.strip_prefix(original_project) {
        project.join(relative)
    } else {
        path.to_path_buf()
    };
    validate_path_text(&project).map_err(|e| e.to_string())?;
    validate_path_text(&path).map_err(|e| e.to_string())?;
    let parent = project.join(".zenpi-worktrees");
    if !path.is_absolute()
        || path.parent() != Some(parent.as_path())
        || path.components().any(|value| {
            matches!(
                value,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_none_or(|name| name.is_empty() || name.len() > 128 || name.starts_with('-'))
    {
        return Err("worktree path must be a direct child of the managed project directory".into());
    }
    let metadata = std::fs::symlink_metadata(&parent).map_err(|e| e.to_string())?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || parent.canonicalize().map_err(|e| e.to_string())? != parent
    {
        return Err("managed worktree directory must not be a symlink".into());
    }
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if !must_exist {
                return Err("worktree destination already exists".into());
            }
            if !metadata.is_dir()
                || metadata.file_type().is_symlink()
                || path.canonicalize().map_err(|e| e.to_string())? != path
            {
                return Err("worktree root must be a canonical directory, not a symlink".into());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !must_exist => {}
        Err(error) => return Err(error.to_string()),
    }
    Ok((project, path))
}

#[cfg(all(test, feature = "remote"))]
mod quota_intent_tests {
    use super::*;
    use crate::remote::protocol::{Channel, Envelope, Message, Operation};
    use std::sync::{Arc, Mutex};
    fn fixture(
        root: &Path,
    ) -> (
        ProjectOwnerPool,
        Envelope,
        crate::worktree::subtab::SubtabState,
        String,
    ) {
        let owner = crate::core::Agent::with_echo(
            crate::session::SessionStore::open_in_workspace(root.join("quota.jsonl"), root)
                .unwrap(),
        );
        let mut pool = ProjectOwnerPool::new(Arc::new(Mutex::new(owner))).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let before = pool.subtab_state(&project).unwrap();
        pool.commit_subtabs(
            &project,
            before.revision,
            before.rename(&before.active_id, "persisted").unwrap(),
        )
        .unwrap();
        let before = pool.subtab_state(&project).unwrap();
        let candidate = before.set_concurrency(&before.active_id, 3).unwrap();
        let mut request = Envelope::new(
            Channel::Control,
            Message::Request {
                operation: Operation::SetConcurrency,
                arguments: serde_json::json!({"subtab_id":before.active_id,"subtab_revision":before.revision,"concurrency":3}),
            },
        );
        request.id = Some("quota-intent".into());
        request.project_id = Some(project);
        request.session_id = Some(pool.active_context().session_id.clone());
        request.required_owner = Some(pool.workspace_owner().into());
        request.expected_revision = Some(pool.workspace_revision());
        let digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&request).unwrap())
        );
        (pool, request, candidate, digest)
    }
    #[test]
    fn unresolved_quota_serializes_new_identities_across_restart_until_explicit_repair() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, mut request, candidate, _) = fixture(&root);
        let other_root = root.join("other-project");
        std::fs::create_dir(&other_root).unwrap();
        let OpenOutcome::Opened(other_project) = pool.open(Some(&other_root)).unwrap() else {
            panic!("expected a separate project");
        };
        request.expected_revision = Some(pool.workspace_revision());
        let digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&request).unwrap())
        );
        let device = "a".repeat(64);
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        let mut competing = request.clone();
        competing.id = Some("second-device-quota".into());
        let other = pool.subtab_state(other_project.as_str()).unwrap();
        let mut cross_project = competing.clone();
        cross_project.project_id = Some(other_project.as_str().into());
        cross_project.session_id = Some(
            pool.context(other_project.as_str())
                .unwrap()
                .session_id
                .clone(),
        );
        if let Message::Request { arguments, .. } = &mut cross_project.payload {
            arguments["subtab_id"] = serde_json::json!(other.active_id);
            arguments["subtab_revision"] = serde_json::json!(other.revision);
        }
        let cross_digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&cross_project).unwrap())
        );
        let cross_candidate = other.set_concurrency(&other.active_id, 3).unwrap();
        let competing_digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&competing).unwrap())
        );
        let owner = pool.owner(request.project_id.as_deref().unwrap()).unwrap();
        let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
        for restart in [false, true] {
            if restart {
                drop(pool);
                pool = ProjectOwnerPool::new(owner.clone()).unwrap();
                pool.restore_checkpoint().unwrap();
            }
            assert!(pool.quota_intent_pending());
            assert!(
                pool.reserve_remote_quota(
                    &"b".repeat(64),
                    &cross_project,
                    &cross_digest,
                    &cross_candidate
                )
                .is_err()
            );
            assert!(
                pool.reserve_remote_quota(
                    &"b".repeat(64),
                    &competing,
                    &competing_digest,
                    &candidate
                )
                .is_err()
            );
            assert_eq!(pool.unresolved_quota_intents().len(), 1);
            assert_eq!(
                std::fs::read(root.join("project-workspace.json")).unwrap(),
                checkpoint
            );
            assert_eq!(
                pool.workspace_receipt(&device, "quota-intent", &digest)
                    .unwrap(),
                Some(serde_json::Value::Null)
            );
        }
        pool.reconcile_quota_intent(&device, "quota-intent")
            .unwrap();
        assert!(!pool.quota_intent_pending());
        let before = pool
            .subtab_state(request.project_id.as_deref().unwrap())
            .unwrap();
        competing.expected_revision = Some(pool.workspace_revision());
        competing.required_owner = Some(pool.workspace_owner().into());
        if let Message::Request { arguments, .. } = &mut competing.payload {
            arguments["subtab_revision"] = serde_json::json!(before.revision);
            arguments["concurrency"] = serde_json::json!(2);
        }
        let candidate = before.set_concurrency(&before.active_id, 2).unwrap();
        let digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&competing).unwrap())
        );
        pool.reserve_remote_quota(&"b".repeat(64), &competing, &digest, &candidate)
            .unwrap();
        assert!(pool.quota_intent_pending());
    }
    #[test]
    fn reservation_is_unknown_durable_and_only_local_idle_reconciliation_commits_configuration() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, request, candidate, digest) = fixture(&root);
        let device = "a".repeat(64);
        let project = request.project_id.as_deref().unwrap();
        let before = pool.subtab_state(project).unwrap();
        let revision = pool.workspace_revision();
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        assert!(
            pool.commit_remote_subtabs(
                project,
                before.rename(&before.active_id, "wrong-candidate").unwrap(),
                &device,
                "quota-intent",
                &digest
            )
            .is_err()
        );
        assert_eq!(pool.subtab_state(project).unwrap(), before);
        assert_eq!(pool.workspace_revision(), revision);
        assert_eq!(
            pool.workspace_receipt(&device, "quota-intent", &digest)
                .unwrap(),
            Some(serde_json::Value::Null)
        );
        assert!(
            pool.reserve_remote_quota(&device, &request, &digest, &candidate)
                .is_err()
        );
        assert_eq!(pool.unresolved_quota_intents().len(), 1);
        assert!(pool.unresolved_worktree_intents().is_empty());
        let owner = pool.owner(project).unwrap();
        drop(pool);
        let mut pool = ProjectOwnerPool::new(owner.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        assert_eq!(pool.unresolved_quota_intents()[0]["concurrency"], 3);
        let lock = owner.lock().unwrap();
        assert!(
            pool.reconcile_quota_intent(&device, "quota-intent")
                .is_err()
        );
        drop(lock);
        let runner = crate::worktree::worktree_runner::WorktreeRunner::spawn(
            &mut pool,
            project,
            &before.active_id,
            crate::runtime::RuntimeConfig::default(),
            |_, _: (), _| Ok::<_, String>(()),
        )
        .unwrap();
        assert!(
            pool.reconcile_quota_intent(&device, "quota-intent")
                .is_err()
        );
        let mut other = ProjectOwnerPool::new(owner.clone()).unwrap();
        other.restore_checkpoint().unwrap();
        assert!(other.worker_handles(project).is_empty());
        assert!(
            other
                .reconcile_quota_intent(&device, "quota-intent")
                .is_err(),
            "foreign runtime lease must fence local repair"
        );
        drop(other);
        runner.shutdown_and_join().unwrap();
        let result = pool
            .reconcile_quota_intent(&device, "quota-intent")
            .unwrap();
        assert_eq!(pool.subtab_state(project).unwrap(), candidate);
        assert!(pool.unresolved_quota_intents().is_empty());
        assert_eq!(result["required_owner"], request.required_owner.unwrap());
        assert_eq!(pool.workspace_revision(), revision + 1);
        assert_eq!(
            pool.reconcile_quota_intent(&device, "quota-intent")
                .unwrap(),
            result
        );
        assert!(owner.lock().unwrap().session().turns().is_empty());
    }
    #[test]
    fn failed_reservation_and_stale_intent_cannot_change_quota_or_overwrite_newer_state() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, request, candidate, digest) = fixture(&root);
        let device = "a".repeat(64);
        let project = request.project_id.as_deref().unwrap();
        let before = pool.subtab_state(project).unwrap();
        let path = root.join("project-workspace.json");
        let saved = std::fs::read(&path).unwrap();
        std::fs::write(&path, [saved.as_slice(), b"\n"].concat()).unwrap();
        assert!(
            pool.reserve_remote_quota(&device, &request, &digest, &candidate)
                .is_err()
        );
        assert!(pool.unresolved_quota_intents().is_empty());
        assert_eq!(pool.subtab_state(project).unwrap(), before);
        std::fs::write(&path, &saved).unwrap();
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        pool.commit_subtabs(
            project,
            before.revision,
            before.rename(&before.active_id, "newer").unwrap(),
        )
        .unwrap();
        let saved = std::fs::read(&path).unwrap();
        assert!(
            pool.reconcile_quota_intent(&device, "quota-intent")
                .is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), saved);
        assert_eq!(pool.unresolved_quota_intents().len(), 1);
    }
    #[test]
    fn remote_state_stays_out_of_the_project_checkpoint_and_legacy_state_moves_out() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, request, candidate, digest) = fixture(&root);
        let device = "a".repeat(64);
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        let core = root.join("project-workspace.json");
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&core).unwrap()).unwrap();
        let keys: Vec<_> = saved.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            [
                "schema_version",
                "sessions",
                "subtabs",
                "workspace",
                "workspace_revision"
            ]
        );
        let remote: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join(REMOTE_STATE_FILE)).unwrap()).unwrap();
        assert_eq!(remote["receipts"][0]["request_id"], "quota-intent");
        let owner = pool.active();
        drop(pool);
        // A pre-split checkpoint carries the same state inline; restoring it
        // moves the state out before the inline copy is dropped.
        let mut legacy = saved.clone();
        let revision = legacy
            .as_object_mut()
            .unwrap()
            .remove("workspace_revision")
            .unwrap();
        let mut inline = remote.clone();
        inline["revision"] = revision;
        legacy["remote_workspace"] = inline;
        std::fs::write(&core, serde_json::to_vec(&legacy).unwrap()).unwrap();
        std::fs::remove_file(root.join(REMOTE_STATE_FILE)).unwrap();
        let mut restored = ProjectOwnerPool::new(owner).unwrap();
        assert!(restored.restore_checkpoint().unwrap());
        assert_eq!(restored.unresolved_quota_intents().len(), 1);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(&core).unwrap()).unwrap(),
            saved
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                &std::fs::read(root.join(REMOTE_STATE_FILE)).unwrap()
            )
            .unwrap(),
            remote
        );
    }
    #[test]
    fn checkpoint_refuses_corrupt_quota_intents_without_starting_workers() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, request, candidate, digest) = fixture(&root);
        let device = "a".repeat(64);
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        let owner = pool.active();
        let path = root.join(REMOTE_STATE_FILE);
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        drop(pool);
        for field in ["candidate", "request", "before", "unknown"] {
            let mut corrupt = saved.clone();
            let intent = &mut corrupt["receipts"][0]["quota_intent"];
            match field {
                "candidate" => intent["candidate"]["tabs"][0]["concurrency"] = serde_json::json!(4),
                "request" => {
                    intent["request"]["payload"]["arguments"]["concurrency"] =
                        serde_json::json!("3")
                }
                "before" => intent["before"]["revision"] = serde_json::json!(999),
                _ => intent["unknown"] = serde_json::json!(true),
            }
            std::fs::write(&path, serde_json::to_vec(&corrupt).unwrap()).unwrap();
            let mut restored = ProjectOwnerPool::new(owner.clone()).unwrap();
            assert!(restored.restore_checkpoint().is_err(), "{field}");
            assert!(
                restored
                    .worker_handles(request.project_id.as_deref().unwrap())
                    .is_empty()
            );
        }
        assert!(!root.join(".zenpi/workers").exists());
    }
    #[test]
    fn reserved_growth_claims_new_writers_before_raising_a_live_scheduler() {
        use crate::runtime::{JobOutcome, RuntimeConfig, RuntimeEvent};
        use std::{sync::mpsc, time::Duration};
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, request, candidate, digest) = fixture(&root);
        let device = "a".repeat(64);
        let project = request.project_id.as_deref().unwrap();
        let before = pool.subtab_state(project).unwrap();
        let (entered, starts) = mpsc::channel();
        let released = Arc::new((Mutex::new(vec![false; 4]), std::sync::Condvar::new()));
        let gates = released.clone();
        let mut runner = crate::worktree::worktree_runner::WorktreeRunner::spawn(
            &mut pool,
            project,
            &before.active_id,
            RuntimeConfig::default(),
            move |worker, index: usize, token| {
                let mut agent = worker.agent().lock().unwrap();
                entered
                    .send((index, agent.session().session_id().to_owned()))
                    .unwrap();
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                let mut ready = gates.0.lock().unwrap();
                while !ready[index] && !token.is_cancelled() {
                    assert!(std::time::Instant::now() < deadline);
                    ready = gates
                        .1
                        .wait_timeout(ready, Duration::from_millis(5))
                        .unwrap()
                        .0;
                }
                drop(ready);
                agent
                    .process_with_cancel(
                        crate::core::TurnInputRequest::new(format!("growth-job-{index}")),
                        || token.is_cancelled(),
                    )
                    .map(|_| index)
                    .map_err(|error| error.to_string())
            },
        )
        .unwrap();
        let ids: Vec<_> = (0..4)
            .map(|index| runner.try_submit(index).unwrap())
            .collect();
        let first = starts.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(first.0, 0);
        while !matches!(runner.recv_timeout(Duration::from_secs(3)).unwrap(), RuntimeEvent::Queued { id, depth: 3 } if id == ids[3])
        {
        }
        assert!(
            runner
                .grow_reserved(&mut pool, &device, "quota-intent", &digest)
                .is_err()
        );
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
        assert!(
            runner
                .grow_reserved(&mut pool, "foreign", "quota-intent", &digest)
                .is_err()
        );
        assert!(
            runner
                .grow_reserved(&mut pool, &device, "quota-intent", &"0".repeat(64))
                .is_err()
        );
        runner
            .grow_reserved(&mut pool, &device, "quota-intent", &digest)
            .unwrap();
        assert_eq!(runner.quota(), 3);
        assert_eq!(runner.applied_concurrency(), 1);
        assert_eq!(pool.worker_handles(project).len(), 3);
        assert!(
            starts.recv_timeout(Duration::from_millis(30)).is_err(),
            "preparing ownership must not apply scheduler quota"
        );
        assert!(
            runner
                .grow_reserved(&mut pool, &device, "quota-intent", &digest)
                .is_err()
        );
        assert!(
            pool.reconcile_quota_intent(&device, "quota-intent")
                .is_err()
        );
        runner
            .try_set_concurrency(3)
            .unwrap()
            .recv_timeout(Duration::from_secs(3))
            .unwrap();
        assert_eq!(runner.applied_concurrency(), 3);
        let added: Vec<_> = (0..2)
            .map(|_| starts.recv_timeout(Duration::from_secs(3)).unwrap())
            .collect();
        assert_eq!(
            added
                .iter()
                .map(|(index, _)| *index)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([1, 2])
        );
        let sessions: BTreeSet<_> = std::iter::once(first.1)
            .chain(added.into_iter().map(|(_, session)| session))
            .collect();
        assert_eq!(sessions.len(), 3);
        assert!(
            starts.recv_timeout(Duration::from_millis(30)).is_err(),
            "fourth task must remain queued at quota three"
        );
        assert_eq!(
            std::fs::read(root.join("project-workspace.json")).unwrap(),
            checkpoint
        );
        assert_eq!(
            pool.subtab_state(project).unwrap(),
            before,
            "runtime application is not a configuration commit"
        );
        released.0.lock().unwrap().fill(true);
        released.1.notify_all();
        let mut done = BTreeSet::new();
        while done.len() < 4 {
            if let RuntimeEvent::Completed { id, outcome } =
                runner.recv_timeout(Duration::from_secs(3)).unwrap()
            {
                assert!(matches!(outcome, JobOutcome::Succeeded(_)));
                assert!(done.insert(id.get()));
            }
        }
        assert!(sessions.contains(&starts.recv_timeout(Duration::from_secs(3)).unwrap().1));
        runner.shutdown_and_join().unwrap();
        pool.reconcile_quota_intent(&device, "quota-intent")
            .unwrap();
        assert_eq!(pool.subtab_state(project).unwrap(), candidate);
        assert!(
            pool.owner(project)
                .unwrap()
                .lock()
                .unwrap()
                .session()
                .turns()
                .is_empty()
        );
        pool.retire_worker_owners(project).unwrap();
    }

    #[test]
    fn failed_growth_rolls_back_staged_claims_and_preserves_original_live_capacity() {
        use crate::runtime::{RuntimeConfig, RuntimeEvent};
        use std::time::Duration;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, request, candidate, digest) = fixture(&root);
        let device = "a".repeat(64);
        let project = request.project_id.as_deref().unwrap();
        let before = pool.subtab_state(project).unwrap();
        let mut runner = crate::worktree::worktree_runner::WorktreeRunner::spawn(
            &mut pool,
            project,
            &before.active_id,
            RuntimeConfig::default(),
            |_, index: usize, _| Ok::<_, String>(index),
        )
        .unwrap();
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        let key = crate::worktree::worker::key(
            project,
            pool.active_context().session_id.as_str(),
            &before.active_id,
            2,
        )
        .unwrap();
        let master = pool.owner(project).unwrap();
        let guard = master.lock().unwrap();
        assert!(
            runner
                .grow_reserved(&mut pool, &device, "quota-intent", &digest)
                .is_err()
        );
        assert_eq!(pool.worker_handles(project).len(), 1);
        drop(guard);
        let alias = root.join(format!(".zenpi/workers/{key}.lease"));
        let outside = root.join("outside-lease");
        std::fs::write(&outside, b"preserve").unwrap();
        std::os::unix::fs::symlink(&outside, &alias).unwrap();
        assert!(
            runner
                .grow_reserved(&mut pool, &device, "quota-intent", &digest)
                .is_err()
        );
        assert_eq!(runner.quota(), 1);
        assert_eq!(runner.applied_concurrency(), 1);
        assert_eq!(
            pool.worker_handles(project).len(),
            1,
            "partial growth must not publish owners"
        );
        assert_eq!(std::fs::read(&outside).unwrap(), b"preserve");
        runner.try_submit(7).unwrap();
        loop {
            if let RuntimeEvent::Completed { outcome, .. } =
                runner.recv_timeout(Duration::from_secs(3)).unwrap()
            {
                assert!(matches!(outcome, crate::runtime::JobOutcome::Succeeded(7)));
                break;
            }
        }
        std::fs::remove_file(alias).unwrap();
        runner
            .grow_reserved(&mut pool, &device, "quota-intent", &digest)
            .unwrap();
        assert_eq!(runner.quota(), 3);
        assert_eq!(runner.applied_concurrency(), 1);
        runner
            .try_set_concurrency(3)
            .unwrap()
            .recv_timeout(Duration::from_secs(3))
            .unwrap();
        runner.shutdown_and_join().unwrap();
        pool.reconcile_quota_intent(&device, "quota-intent")
            .unwrap();
        pool.retire_worker_owners(project).unwrap();
    }

    #[test]
    fn stale_growth_intent_cannot_prepare_new_owners_or_advance_runner_scope() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, request, candidate, digest) = fixture(&root);
        let device = "a".repeat(64);
        let project = request.project_id.as_deref().unwrap();
        let before = pool.subtab_state(project).unwrap();
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        pool.commit_subtabs(
            project,
            before.revision,
            before.rename(&before.active_id, "newer").unwrap(),
        )
        .unwrap();
        let mut runner = crate::worktree::worktree_runner::WorktreeRunner::spawn(
            &mut pool,
            project,
            &before.active_id,
            crate::runtime::RuntimeConfig::default(),
            |_, _: (), _| Ok::<_, String>(()),
        )
        .unwrap();
        let revision = runner.scope().3;
        let saved = std::fs::read(root.join("project-workspace.json")).unwrap();
        assert!(
            runner
                .grow_reserved(&mut pool, &device, "quota-intent", &digest)
                .is_err()
        );
        assert_eq!(runner.quota(), 1);
        assert_eq!(runner.applied_concurrency(), 1);
        assert_eq!(runner.scope().3, revision);
        assert_eq!(pool.worker_handles(project).len(), 1);
        assert_eq!(
            std::fs::read(root.join("project-workspace.json")).unwrap(),
            saved
        );
        runner.shutdown_and_join().unwrap();
        pool.retire_worker_owners(project).unwrap();
    }

    #[test]
    fn newly_grown_detached_writer_keeps_all_claims_until_actual_exit() {
        use crate::runtime::{RuntimeConfig, RuntimeEvent};
        use std::{
            sync::mpsc,
            time::{Duration, Instant},
        };
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, request, candidate, digest) = fixture(&root);
        let device = "a".repeat(64);
        let project = request.project_id.as_deref().unwrap();
        let before = pool.subtab_state(project).unwrap();
        let owner = pool.owner(project).unwrap();
        let (entered, starts) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let gate = Mutex::new(gate);
        let (finished, ends) = mpsc::channel();
        let mut runner = crate::worktree::worktree_runner::WorktreeRunner::spawn(
            &mut pool,
            project,
            &before.active_id,
            RuntimeConfig::default(),
            move |worker, index: usize, _| {
                let _guard = worker.agent().lock().unwrap();
                entered.send(index).unwrap();
                if index == 1 {
                    gate.lock()
                        .unwrap()
                        .recv_timeout(Duration::from_secs(5))
                        .unwrap();
                    finished.send(()).unwrap();
                }
                Ok::<_, String>(index)
            },
        )
        .unwrap();
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        runner
            .grow_reserved(&mut pool, &device, "quota-intent", &digest)
            .unwrap();
        runner
            .try_set_concurrency(3)
            .unwrap()
            .recv_timeout(Duration::from_secs(3))
            .unwrap();
        runner.try_submit(1).unwrap();
        assert_eq!(starts.recv_timeout(Duration::from_secs(3)).unwrap(), 1);
        // Complete a different slot without freeing the detached writer's claims.
        runner.try_submit(0).unwrap();
        assert_eq!(starts.recv_timeout(Duration::from_secs(3)).unwrap(), 0);
        loop {
            if matches!(
                runner.recv_timeout(Duration::from_secs(3)).unwrap(),
                RuntimeEvent::Completed {
                    outcome: crate::runtime::JobOutcome::Succeeded(0),
                    ..
                }
            ) {
                break;
            }
        }
        runner
            .shutdown_and_join_with_events(Duration::from_millis(20), |_| {})
            .unwrap();
        assert!(pool.retire_worker_owners(project).is_err());
        drop(pool);
        let mut restored = ProjectOwnerPool::new(owner).unwrap();
        restored.restore_checkpoint().unwrap();
        assert!(
            restored
                .reconcile_quota_intent(&device, "quota-intent")
                .is_err(),
            "restart must not ignore detached new-slot leases"
        );
        release.send(()).unwrap();
        ends.recv_timeout(Duration::from_secs(3)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while restored
            .reconcile_quota_intent(&device, "quota-intent")
            .is_err()
        {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(restored.subtab_state(project).unwrap(), candidate);
        assert!(restored.worker_handles(project).is_empty());
    }

    #[test]
    fn post_reservation_commit_failure_remains_unknown_until_explicit_repair() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, request, candidate, digest) = fixture(&root);
        let device = "a".repeat(64);
        let project = request.project_id.as_deref().unwrap();
        let before = pool.subtab_state(project).unwrap();
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        let path = root.join("project-workspace.json");
        let saved = std::fs::read(&path).unwrap();
        std::fs::write(&path, [saved.as_slice(), b"\n"].concat()).unwrap();
        assert!(
            pool.commit_remote_subtabs(project, candidate, &device, "quota-intent", &digest)
                .is_err()
        );
        assert_eq!(pool.subtab_state(project).unwrap(), before);
        assert_eq!(
            pool.workspace_receipt(&device, "quota-intent", &digest)
                .unwrap(),
            Some(serde_json::Value::Null)
        );
        let owner = pool.active();
        drop(pool);
        let mut restored = ProjectOwnerPool::new(owner).unwrap();
        restored.restore_checkpoint().unwrap();
        assert_eq!(restored.unresolved_quota_intents().len(), 1);
        assert_eq!(
            restored
                .workspace_receipt(&device, "quota-intent", &digest)
                .unwrap(),
            Some(serde_json::Value::Null)
        );
        assert!(restored.worker_handles(project).is_empty());
        restored
            .reconcile_quota_intent(&device, "quota-intent")
            .unwrap();
        assert_eq!(
            restored.subtab_state(project).unwrap().tabs[0].concurrency,
            3
        );
    }

    #[test]
    fn reserved_runtime_settlement_commits_only_after_ack_and_supports_lower_then_grow_again() {
        use crate::runtime::{RuntimeConfig, RuntimeEvent};
        use std::{
            sync::mpsc,
            time::{Duration, Instant},
        };
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, request, candidate, digest) = fixture(&root);
        let device = "a".repeat(64);
        let project = request.project_id.as_deref().unwrap();
        let before = pool.subtab_state(project).unwrap();
        let (started, starts) = mpsc::channel();
        let release = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let gates = release.clone();
        let mut runner = crate::worktree::worktree_runner::WorktreeRunner::spawn(
            &mut pool,
            project,
            &before.active_id,
            RuntimeConfig {
                event_capacity: 1,
                ..RuntimeConfig::default()
            },
            move |worker, index: usize, token| {
                let mut agent = worker.agent().lock().unwrap();
                started.send(index).unwrap();
                let deadline = Instant::now() + Duration::from_secs(5);
                while !gates.load(std::sync::atomic::Ordering::Acquire) && !token.is_cancelled() {
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(1));
                }
                agent
                    .process_with_cancel(
                        crate::core::TurnInputRequest::new(format!("settled-{index}")),
                        || token.is_cancelled(),
                    )
                    .map(|_| index)
                    .map_err(|error| error.to_string())
            },
        )
        .unwrap();
        runner.try_submit(0).unwrap();
        starts.recv_timeout(Duration::from_secs(3)).unwrap();
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        runner
            .begin_reserved_quota(&mut pool, &device, "quota-intent", &digest)
            .unwrap();
        assert!(runner.quota_change_pending());
        assert_eq!(runner.configured_quota(), 1);
        assert_eq!(runner.quota(), 3);
        std::thread::sleep(Duration::from_millis(30));
        assert!(runner.poll_reserved_quota(&mut pool).unwrap().is_none());
        assert_eq!(pool.subtab_state(project).unwrap(), before);
        assert_eq!(runner.applied_concurrency(), 1);
        assert_eq!(
            runner.try_submit(1).unwrap_err(),
            crate::runtime::SubmitError::QueueFull
        );
        assert!(
            runner
                .begin_reserved_quota(&mut pool, &device, "quota-intent", &digest)
                .is_err()
        );
        assert!(runner.try_set_concurrency(2).is_err());
        assert!(matches!(
            runner.recv_timeout(Duration::from_secs(3)).unwrap(),
            RuntimeEvent::Accepted { .. }
        ));
        assert!(matches!(
            runner.recv_timeout(Duration::from_secs(3)).unwrap(),
            RuntimeEvent::Started { .. }
        ));
        let deadline = Instant::now() + Duration::from_secs(3);
        let result = loop {
            if let Some(value) = runner.poll_reserved_quota(&mut pool).unwrap() {
                break value;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        };
        assert!(!runner.quota_change_pending());
        assert_eq!(runner.configured_quota(), 3);
        assert_eq!(runner.scope().3, candidate.revision);
        assert_eq!(pool.subtab_state(project).unwrap(), candidate);
        assert_eq!(
            pool.workspace_receipt(&device, "quota-intent", &digest)
                .unwrap(),
            Some(result)
        );
        assert!(
            runner
                .begin_reserved_quota(&mut pool, &device, "quota-intent", &digest)
                .is_err(),
            "settled identity cannot apply twice"
        );
        // Lower while a real writer is held; retain all claims and capacity.
        let reserve = |pool: &mut ProjectOwnerPool, count: u16, id: &str| {
            let state = pool.subtab_state(project).unwrap();
            let mut req = request.clone();
            req.id = Some(id.into());
            req.expected_revision = Some(pool.workspace_revision());
            req.payload = Message::Request {
                operation: Operation::SetConcurrency,
                arguments: serde_json::json!({"subtab_id":state.active_id,"subtab_revision":state.revision,"concurrency":count}),
            };
            let hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&req).unwrap()));
            pool.reserve_remote_quota(
                &device,
                &req,
                &hash,
                &state.set_concurrency(&state.active_id, count).unwrap(),
            )
            .unwrap();
            hash
        };
        for (id, count) in [("lower-live", 1), ("raise-live", 3), ("grow-again", 4)] {
            let hash = reserve(&mut pool, count, id);
            runner
                .begin_reserved_quota(&mut pool, &device, id, &hash)
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            while runner.poll_reserved_quota(&mut pool).unwrap().is_none() {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            assert_eq!(runner.configured_quota(), count);
            assert_eq!(runner.applied_concurrency(), count);
            assert_eq!(
                pool.subtab_state(project).unwrap().tabs[0].concurrency,
                count
            );
            assert_eq!(runner.quota(), if count == 4 { 4 } else { 3 });
        }
        assert!(pool.retire_worker_owners(project).is_err());
        runner.try_submit(1).unwrap();
        assert_eq!(starts.recv_timeout(Duration::from_secs(3)).unwrap(), 1);
        release.store(true, std::sync::atomic::Ordering::Release);
        let mut completed = 0;
        while completed < 2 {
            if let RuntimeEvent::Completed { outcome, .. } =
                runner.recv_timeout(Duration::from_secs(3)).unwrap()
            {
                assert!(matches!(outcome, crate::runtime::JobOutcome::Succeeded(_)));
                completed += 1;
            }
        }
        runner.shutdown_and_join().unwrap();
        pool.retire_worker_owners(project).unwrap();
    }

    #[test]
    fn applied_runtime_checkpoint_conflict_keeps_unknown_receipt_and_submission_fence() {
        use crate::runtime::RuntimeConfig;
        use std::time::{Duration, Instant};
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, request, candidate, digest) = fixture(&root);
        let device = "a".repeat(64);
        let project = request.project_id.as_deref().unwrap();
        let before = pool.subtab_state(project).unwrap();
        let mut runner = crate::worktree::worktree_runner::WorktreeRunner::spawn(
            &mut pool,
            project,
            &before.active_id,
            RuntimeConfig::default(),
            |_, _: (), _| Ok::<_, String>(()),
        )
        .unwrap();
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        runner
            .begin_reserved_quota(&mut pool, &device, "quota-intent", &digest)
            .unwrap();
        let path = root.join("project-workspace.json");
        let saved = std::fs::read(&path).unwrap();
        std::fs::write(&path, [saved.as_slice(), b"\n"].concat()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while runner.applied_concurrency() != 3 {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(runner.poll_reserved_quota(&mut pool).is_err());
        assert_eq!(runner.scope().3, before.revision);
        assert_eq!(runner.configured_quota(), 1);
        assert!(runner.quota_change_pending());
        assert!(runner.try_submit(()).is_err());
        assert!(
            runner.try_set_concurrency(1).is_err(),
            "unresolved application cannot be overwritten by a second runtime change"
        );
        assert_eq!(pool.subtab_state(project).unwrap(), before);
        assert_eq!(
            pool.workspace_receipt(&device, "quota-intent", &digest)
                .unwrap(),
            Some(serde_json::Value::Null)
        );
        runner.shutdown_and_join().unwrap();
        pool.retire_worker_owners(project).unwrap();
        let owner = pool.active();
        drop(pool);
        let mut restored = ProjectOwnerPool::new(owner).unwrap();
        restored.restore_checkpoint().unwrap();
        assert_eq!(restored.subtab_state(project).unwrap(), before);
        assert_eq!(restored.unresolved_quota_intents().len(), 1);
        restored
            .reconcile_quota_intent(&device, "quota-intent")
            .unwrap();
        assert_eq!(restored.subtab_state(project).unwrap(), candidate);
    }

    #[test]
    fn shutdown_does_not_settle_an_unapplied_reserved_quota_command() {
        use crate::runtime::RuntimeConfig;
        use std::{sync::mpsc, time::Duration};
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, request, candidate, digest) = fixture(&root);
        let device = "a".repeat(64);
        let project = request.project_id.as_deref().unwrap();
        let before = pool.subtab_state(project).unwrap();
        let (started, starts) = mpsc::channel();
        let mut runner = crate::worktree::worktree_runner::WorktreeRunner::spawn(
            &mut pool,
            project,
            &before.active_id,
            RuntimeConfig {
                event_capacity: 1,
                ..RuntimeConfig::default()
            },
            move |_, _: (), token| {
                started.send(()).unwrap();
                while !token.is_cancelled() {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok::<_, String>(())
            },
        )
        .unwrap();
        runner.try_submit(()).unwrap();
        starts.recv_timeout(Duration::from_secs(3)).unwrap();
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        runner
            .try_shutdown_with_grace(Duration::from_secs(1))
            .unwrap();
        runner
            .begin_reserved_quota(&mut pool, &device, "quota-intent", &digest)
            .unwrap();
        assert!(runner.poll_reserved_quota(&mut pool).unwrap().is_none());
        runner.shutdown_and_join().unwrap();
        assert_eq!(pool.subtab_state(project).unwrap(), before);
        assert_eq!(
            pool.workspace_receipt(&device, "quota-intent", &digest)
                .unwrap(),
            Some(serde_json::Value::Null)
        );
        pool.retire_worker_owners(project).unwrap();
    }

    #[test]
    fn scope_refresh_refuses_changed_execution_quota_session_regression_and_pending_intent() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (mut pool, request, candidate, digest) = fixture(&root);
        let device = "a".repeat(64);
        let project = request.project_id.as_deref().unwrap();
        let before = pool.subtab_state(project).unwrap();
        let mut runner = crate::worktree::worktree_runner::WorktreeRunner::spawn(
            &mut pool,
            project,
            &before.active_id,
            crate::runtime::RuntimeConfig::default(),
            |_, _: (), _| Ok::<_, String>(()),
        )
        .unwrap();
        let captured = runner.scope().3;
        for change in ["quota", "root", "kind", "regression", "removed"] {
            let mut changed = before.clone();
            changed.revision += 1;
            match change {
                "quota" => changed.tabs[0].concurrency = 2,
                "root" => changed.tabs[0].root = root.join("different"),
                "kind" => changed.tabs[0].kind = crate::worktree::subtab::SubtabKind::InPlace,
                "regression" => changed.revision = captured - 1,
                _ => changed.tabs.clear(),
            }
            pool.subtabs.insert(project.into(), changed);
            assert!(runner.refresh_unchanged_scope(&pool).is_err(), "{change}");
            assert_eq!(runner.scope().3, captured);
        }
        pool.subtabs.insert(project.into(), before.clone());
        let session = pool.contexts.get(project).unwrap().session_id.clone();
        pool.contexts.get_mut(project).unwrap().session_id = "different-session".into();
        assert!(runner.refresh_unchanged_scope(&pool).is_err());
        pool.contexts.get_mut(project).unwrap().session_id = session;
        pool.reserve_remote_quota(&device, &request, &digest, &candidate)
            .unwrap();
        runner
            .begin_reserved_quota(&mut pool, &device, "quota-intent", &digest)
            .unwrap();
        assert!(runner.refresh_unchanged_scope(&pool).is_err());
        assert_eq!(runner.scope().3, captured);
        runner.shutdown_and_join().unwrap();
        pool.retire_worker_owners(project).unwrap();
    }
}

#[cfg(all(test, unix))]
mod worktree_git_tests {
    use super::*;
    use std::{
        process::Command,
        time::{Duration, Instant},
    };

    fn shell(script: &str, timeout: Duration) -> Result<String, String> {
        bounded_worktree_git(
            Command::new("/bin/sh").args(["-c", script]),
            Instant::now() + timeout,
        )
    }

    #[test]
    fn worktree_git_preserves_output_and_failure_reason() {
        assert_eq!(
            shell("printf 'one\\ntwo\\n'", Duration::from_secs(2)).unwrap(),
            "one\ntwo\n"
        );
        assert_eq!(
            shell("printf refused >&2; exit 7", Duration::from_secs(2)).unwrap_err(),
            "refused"
        );
        assert!(
            shell("exit 9", Duration::from_secs(2))
                .unwrap_err()
                .contains("git failed")
        );
        assert!(
            shell("printf '\\377'", Duration::from_secs(2))
                .unwrap_err()
                .contains("UTF-8")
        );
    }

    #[test]
    fn worktree_git_enforces_both_stream_limits_before_allocation() {
        for redirect in ["", " >&2"] {
            let script = format!(
                "head -c {} /dev/zero{redirect}",
                MAX_WORKTREE_OUTPUT_BYTES + 1
            );
            assert!(
                shell(&script, Duration::from_secs(2))
                    .unwrap_err()
                    .contains("output exceeded")
            );
        }
        let script = format!("head -c {} /dev/zero", MAX_WORKTREE_OUTPUT_BYTES);
        assert_eq!(
            shell(&script, Duration::from_secs(2)).unwrap().len(),
            MAX_WORKTREE_OUTPUT_BYTES
        );
    }

    #[test]
    fn worktree_git_drains_both_pipes_without_deadlock() {
        let script = "i=0; while [ $i -lt 4000 ]; do printf abcdefghijklmnop; printf ABCDEFGHIJKLMNOP >&2; i=$((i+1)); done";
        assert_eq!(
            shell(script, Duration::from_secs(3)).unwrap(),
            "abcdefghijklmnop".repeat(4000)
        );
    }

    #[test]
    fn worktree_git_deadline_covers_silent_child_and_inherited_pipe() {
        for script in ["sleep 30", "sleep 30 & exit 0"] {
            let started = Instant::now();
            assert!(
                shell(script, Duration::from_millis(150))
                    .unwrap_err()
                    .contains("deadline exceeded")
            );
            assert!(started.elapsed() < Duration::from_secs(2));
        }
    }

    #[test]
    fn worktree_git_expired_deadline_does_not_spawn() {
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("spawned");
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "touch \"$1\"", "fixture"]).arg(&marker);
        assert!(
            bounded_worktree_git(&mut command, Instant::now())
                .unwrap_err()
                .contains("deadline exceeded")
        );
        assert!(!marker.exists());
    }

    #[test]
    fn worktree_git_reaps_leader_and_terminates_descendants() {
        let temp = tempfile::tempdir().unwrap();
        for script in [
            "echo $$ > \"$1/leader\"; (trap '' TERM; sleep 30) & echo $! > \"$1/descendant\"; wait",
            "echo $$ > \"$1/leader\"; (trap '' TERM; sleep 30) >/dev/null 2>&1 & echo $! > \"$1/descendant\"; printf ok",
        ] {
            let mut command = Command::new("/bin/sh");
            command.args(["-c", script, "fixture"]).arg(temp.path());
            let _ = bounded_worktree_git(&mut command, Instant::now() + Duration::from_millis(250));
            for name in ["leader", "descendant"] {
                let pid: i32 = std::fs::read_to_string(temp.path().join(name))
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();
                let until = Instant::now() + Duration::from_secs(2);
                loop {
                    let exists = unsafe { libc::kill(pid, 0) } == 0;
                    // Linux can retain a killed orphan as an init-owned zombie.
                    let zombie =
                        std::fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|text| {
                            text.rsplit_once(") ")
                                .is_some_and(|(_, fields)| fields.starts_with('Z'))
                        });
                    if !exists || zombie {
                        break;
                    }
                    assert!(
                        Instant::now() < until,
                        "owned {name} {pid} is still running"
                    );
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }
}
