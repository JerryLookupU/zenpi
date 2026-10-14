//! Host-owned layer-2 identities. Display names and vector indices are not identities.

#[cfg(feature = "remote")]
use crate::project_workspace::ProjectOwnerPool;
#[cfg(feature = "remote")]
use crate::remote::{
    auth::{AuthenticatedDevice, DeviceRegistry},
    protocol::{Envelope, ErrorCode, Message, Operation},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[cfg(feature = "remote")]
pub fn listing(
    pool: &ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
) -> Result<serde_json::Value, ErrorCode> {
    if request.validate().is_err()
        || !matches!(&request.payload, Message::Request { operation: Operation::SubtabList, arguments } if arguments.as_object().is_some_and(|v| v.is_empty()))
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let project = request
        .project_id
        .as_deref()
        .ok_or(ErrorCode::InvalidRequest)?;
    registry.with_authorized_projects(device, &[project], Operation::SubtabList, |authorization| {
        let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
        if request.session_id.as_deref().is_some_and(|id| id != context.session_id) { return Err(ErrorCode::OwnerMismatch); }
        let state = pool.subtab_state(project).map_err(|_| ErrorCode::InternalError)?;
        let tabs: Vec<_> = state.tabs.iter().map(|tab| serde_json::json!({"id":tab.id,
            "name":crate::security::redact_text(&tab.name, &[]), "kind":tab.kind, "concurrency":tab.concurrency})).collect();
        Ok(serde_json::json!({"project_id":project, "session_id":context.session_id, "schema_version":SUBTAB_SCHEMA_VERSION,
            "required_owner":pool.workspace_owner(), "workspace_revision":pool.workspace_revision(), "subtab_revision":state.revision,
            "active_subtab_id":state.active_id, "tabs":tabs,
            "subtab_control":pool.has_workspace_checkpoint() && authorization.authorize(project, Operation::SubtabSelect).is_ok(),
            "directory_tab_control":pool.has_workspace_checkpoint() && authorization.authorize(project, Operation::SubtabAdd).is_ok(),
            "worktree_control":pool.has_workspace_checkpoint() && authorization.authorize(project, Operation::SubtabAdd).is_ok(),
            "concurrency_control":pool.has_workspace_checkpoint() && authorization.authorize(project, Operation::SetConcurrency).is_ok()}))
    }).map_err(|error| error.remote_code())?
}

#[cfg(feature = "remote")]
pub fn mutate(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
) -> Result<serde_json::Value, ErrorCode> {
    let Message::Request {
        operation,
        arguments,
    } = &request.payload
    else {
        return Err(ErrorCode::InvalidRequest);
    };
    if !matches!(
        operation,
        Operation::SubtabSelect
            | Operation::SubtabRename
            | Operation::SubtabMove
            | Operation::SubtabAdd
            | Operation::SubtabClose
            | Operation::SetConcurrency
    ) || request.validate().is_err()
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let project = request
        .project_id
        .as_deref()
        .ok_or(ErrorCode::InvalidRequest)?;
    let request_id = request.id.as_deref().ok_or(ErrorCode::InvalidRequest)?;
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(request).map_err(|_| ErrorCode::InternalError)?)
    );
    registry
        .with_authorized_projects(device, &[project], *operation, |authorization| {
            authorization
                .authorize(project, Operation::Inspect)
                .map_err(|_| ErrorCode::Unauthorized)?;
            if let Some(result) = pool.workspace_receipt(device.id(), request_id, &fingerprint)? {
                return Ok(result);
            }
            if *operation == Operation::SetConcurrency && pool.quota_intent_pending() {
                return Err(ErrorCode::Busy);
            }
            if request.required_owner.as_deref() != Some(pool.workspace_owner()) {
                return Err(ErrorCode::OwnerMismatch);
            }
            if request.expected_revision != Some(pool.workspace_revision()) {
                return Err(ErrorCode::RevisionConflict);
            }
            if !pool.has_workspace_checkpoint() {
                return Err(ErrorCode::InvalidRequest);
            }
            let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
            if request.session_id.as_deref() != Some(context.session_id.as_str()) {
                return Err(ErrorCode::OwnerMismatch);
            }
            let values = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
            let expected_keys: &[&str] = match operation {
                Operation::SubtabSelect => &["subtab_id", "subtab_revision"],
                Operation::SubtabRename => &["subtab_id", "subtab_revision", "name"],
                Operation::SubtabMove => &["subtab_id", "subtab_revision", "before_id"],
                Operation::SubtabAdd
                    if values.get("kind").and_then(serde_json::Value::as_str)
                        == Some("worktree") =>
                {
                    &["kind", "name", "subtab_revision"]
                }
                Operation::SubtabAdd => &["kind", "name", "cwd", "subtab_revision"],
                Operation::SubtabClose => &["subtab_id", "subtab_revision"],
                Operation::SetConcurrency => &["subtab_id", "subtab_revision", "concurrency"],
                _ => unreachable!(),
            };
            if values.len() != expected_keys.len()
                || expected_keys.iter().any(|key| !values.contains_key(*key))
            {
                return Err(ErrorCode::InvalidRequest);
            }
            let id = if *operation == Operation::SubtabAdd {
                ""
            } else {
                values["subtab_id"]
                    .as_str()
                    .ok_or(ErrorCode::InvalidRequest)?
            };
            let revision = values["subtab_revision"]
                .as_u64()
                .ok_or(ErrorCode::InvalidRequest)?;
            let state = pool
                .subtab_state(project)
                .map_err(|_| ErrorCode::InvalidRequest)?;
            if revision != state.revision {
                return Err(ErrorCode::RevisionConflict);
            }
            let base = pool
                .workspace()
                .tabs()
                .iter()
                .find(|tab| tab.id().as_str() == project)
                .ok_or(ErrorCode::InvalidRequest)?
                .cwd()
                .to_path_buf();
            let worktree_add = *operation == Operation::SubtabAdd && values["kind"] == "worktree";
            let worktree_close = *operation == Operation::SubtabClose
                && state
                    .tabs
                    .iter()
                    .any(|tab| tab.id == id && tab.kind == SubtabKind::Worktree);
            let mut git_action = None;
            let mut candidate = match operation {
                Operation::SetConcurrency => state.set_concurrency(
                    id,
                    values["concurrency"]
                        .as_u64()
                        .filter(|value| (1..=64).contains(value))
                        .ok_or(ErrorCode::InvalidRequest)? as u16,
                ),
                Operation::SubtabSelect => state.select(id),
                Operation::SubtabRename => state.rename(
                    id,
                    values["name"].as_str().ok_or(ErrorCode::InvalidRequest)?,
                ),
                Operation::SubtabMove => {
                    let before = if values["before_id"].is_null() {
                        None
                    } else {
                        Some(
                            values["before_id"]
                                .as_str()
                                .ok_or(ErrorCode::InvalidRequest)?,
                        )
                    };
                    state.move_before(id, before)
                }
                Operation::SubtabAdd => {
                    if worktree_add {
                        let name = values["name"].as_str().ok_or(ErrorCode::InvalidRequest)?;
                        validate_worktree_name(name).map_err(|_| ErrorCode::InvalidRequest)?;
                        let path = base.join(".zenpi-worktrees").join(name);
                        git_action = Some((path.clone(), Some(name.to_owned())));
                        state.plan_worktree(project, &base, &path, name)
                    } else {
                        if values["kind"].as_str() != Some("in_place") {
                            return Err(ErrorCode::InvalidRequest);
                        }
                        let cwd = values["cwd"].as_str().ok_or(ErrorCode::InvalidRequest)?;
                        if cwd.is_empty() || cwd.len() > 4096 || cwd.chars().any(char::is_control) {
                            return Err(ErrorCode::InvalidRequest);
                        }
                        let base = pool
                            .workspace()
                            .tabs()
                            .iter()
                            .find(|tab| tab.id().as_str() == project)
                            .ok_or(ErrorCode::InvalidRequest)?
                            .cwd();
                        let input = Path::new(cwd);
                        if input
                            .components()
                            .any(|v| matches!(v, std::path::Component::ParentDir))
                        {
                            return Err(ErrorCode::InvalidRequest);
                        }
                        let path = if input.is_absolute() {
                            input.to_path_buf()
                        } else {
                            base.join(input)
                        };
                        let path = path.canonicalize().map_err(|_| ErrorCode::InvalidRequest)?;
                        if !path.is_dir() || path == base || !path.starts_with(base) {
                            return Err(ErrorCode::InvalidRequest);
                        }
                        state.add_directory(
                            project,
                            base,
                            &path,
                            values["name"].as_str().ok_or(ErrorCode::InvalidRequest)?,
                        )
                    }
                }
                Operation::SubtabClose if worktree_close => {
                    let tab = state
                        .tabs
                        .iter()
                        .find(|tab| tab.id == id)
                        .ok_or(ErrorCode::InvalidRequest)?;
                    git_action = Some((tab.root.clone(), None));
                    state.close_secondary(id)
                }
                Operation::SubtabClose => state.close_directory(id),
                _ => unreachable!(),
            }
            .map_err(|_| ErrorCode::InvalidRequest)?;
            // Even a logical no-op gets one durable receipt and one revision, so exact retry is unambiguous.
            candidate.revision = state
                .revision
                .checked_add(1)
                .ok_or(ErrorCode::ResourceLimit)?;
            let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
            let guard = owner.try_lock().map_err(|_| ErrorCode::Busy)?;
            if guard.phase() == crate::core::AgentPhase::Running
                || guard.input_port().has_pending()
                || guard.input_port().is_preparing()
            {
                return Err(ErrorCode::Busy);
            }
            let arch = pool.existing_arch_owner(project);
            let arch_guard = arch
                .as_ref()
                .map(|owner| owner.try_lock().map_err(|_| ErrorCode::Busy))
                .transpose()?;
            if arch_guard.as_ref().is_some_and(|agent| {
                agent.phase() == crate::core::AgentPhase::Running
                    || agent.input_port().has_pending()
                    || agent.input_port().is_preparing()
            }) {
                return Err(ErrorCode::Busy);
            }
            let workers = pool.worker_handles(project);
            let mut worker_guards = Vec::new();
            for worker in &workers {
                let guard = worker.agent().try_lock().map_err(|_| ErrorCode::Busy)?;
                if worker.runtime_owned() || !super::worker::WorkerOwner::idle(&guard) {
                    return Err(ErrorCode::Busy);
                }
                worker_guards.push(guard);
            }
            if let Some((path, branch)) = git_action {
                if branch.is_none() {
                    crate::project_workspace::validate_worktree_removal(&base, &path)
                        .map_err(|_| ErrorCode::InvalidRequest)?;
                } else {
                    // No remote path is accepted; only a direct managed child.
                    let parent = base.join(".zenpi-worktrees");
                    std::fs::create_dir(&parent)
                        .or_else(|error| {
                            if error.kind() == std::io::ErrorKind::AlreadyExists {
                                Ok(())
                            } else {
                                Err(error)
                            }
                        })
                        .map_err(|_| ErrorCode::InternalError)?;
                    let metadata = std::fs::symlink_metadata(&parent)
                        .map_err(|_| ErrorCode::InvalidRequest)?;
                    if !metadata.is_dir()
                        || metadata.file_type().is_symlink()
                        || path.exists()
                        || std::fs::symlink_metadata(&path).is_ok()
                    {
                        return Err(ErrorCode::InvalidRequest);
                    }
                }
                pool.reserve_remote_subtab_git(
                    device.id(),
                    request,
                    &fingerprint,
                    &candidate,
                    &path,
                    branch.as_deref(),
                )
                .map_err(|_| ErrorCode::InternalError)?;
                let result = match branch {
                    Some(branch) => crate::project_workspace::add_worktree(&base, &path, &branch),
                    None => crate::project_workspace::remove_worktree(&base, &path),
                };
                if result.is_err() {
                    return Ok(serde_json::Value::Null);
                }
                drop(worker_guards);
                // A failed post-Git checkpoint cannot truthfully report failed/no effect.
                return Ok(pool
                    .commit_remote_subtabs(
                        project,
                        candidate,
                        device.id(),
                        request_id,
                        &fingerprint,
                    )
                    .unwrap_or(serde_json::Value::Null));
            }
            drop(worker_guards);
            if *operation == Operation::SetConcurrency {
                pool.reserve_remote_quota(device.id(), request, &fingerprint, &candidate)
                    .map_err(|_| ErrorCode::InternalError)?;
                // Reservation fences exact retries even when the later commit
                // becomes ambiguous. Never claim failed/no effect afterwards.
                return Ok(pool
                    .commit_remote_subtabs(
                        project,
                        candidate,
                        device.id(),
                        request_id,
                        &fingerprint,
                    )
                    .unwrap_or(serde_json::Value::Null));
            }
            pool.commit_remote_subtabs(project, candidate, device.id(), request_id, &fingerprint)
                .map_err(|_| ErrorCode::InternalError)
        })
        .map_err(|error| error.remote_code())?
}
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub const SUBTAB_SCHEMA_VERSION: u16 = 2;
pub const MAX_SUBTABS: usize = crate::project_workspace::MAX_PROJECT_SUBTABS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtabKind {
    Main,
    Worktree,
    InPlace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostSubtab {
    pub id: String,
    pub name: String,
    pub root: PathBuf,
    pub kind: SubtabKind,
    pub concurrency: u16,
}

/// Immutable execution selection captured by the host before a job is queued.
#[derive(Debug, Clone)]
pub struct SubtabExecution {
    pub tab: HostSubtab,
    pub project_root: PathBuf,
}
impl SubtabExecution {
    pub fn validate(&self) -> Result<(), String> {
        validate_root(&self.project_root)?;
        validate_root(&self.tab.root)?;
        if !self.project_root.is_dir() || !self.tab.root.is_dir() {
            return Err("execution workspace is unavailable".into());
        }
        match self.tab.kind {
            SubtabKind::Main if self.tab.root == self.project_root => Ok(()),
            SubtabKind::InPlace
                if self.tab.root != self.project_root
                    && self.tab.root.starts_with(&self.project_root) =>
            {
                Ok(())
            }
            SubtabKind::Worktree => {
                if crate::project_workspace::list_worktrees(&self.project_root)?
                    .iter()
                    .any(|entry| entry.path == self.tab.root)
                {
                    Ok(())
                } else {
                    Err("execution worktree is not registered with this project".into())
                }
            }
            _ => Err("execution workspace does not belong to this project".into()),
        }
    }
    pub fn run<R>(
        &self,
        agent: &mut crate::core::Agent,
        run: impl FnOnce(&mut crate::core::Agent) -> R,
    ) -> Result<R, crate::core::AgentError> {
        self.run_cancelable(agent, &|| false, run)
    }
    pub fn run_cancelable<R>(
        &self,
        agent: &mut crate::core::Agent,
        cancelled: &dyn Fn() -> bool,
        run: impl FnOnce(&mut crate::core::Agent) -> R,
    ) -> Result<R, crate::core::AgentError> {
        self.validate()
            .map_err(crate::core::AgentError::InvalidTurn)?;
        agent.with_tool_workspace_cancelable(&self.tab.root, cancelled, run)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubtabState {
    pub schema_version: u16,
    pub revision: u64,
    pub tabs: Vec<HostSubtab>,
    pub active_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacySubtab {
    pub name: String,
    pub root: String,
    pub kind: SubtabKind,
    #[serde(default = "one")]
    pub concurrency: u16,
}
fn one() -> u16 {
    1
}
fn identity(project: &str, kind: &SubtabKind, root: &Path) -> Result<String, String> {
    let path = root.to_str().ok_or("subtab root must be UTF-8")?;
    let kind = match kind {
        SubtabKind::Main => "main",
        SubtabKind::Worktree => "worktree",
        SubtabKind::InPlace => "in_place",
    };
    let bytes =
        serde_json::to_vec(&("zenpi-subtab-v2", project, kind, path)).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.trim() != name
        || name.len() > 128
        || name.chars().any(char::is_control)
    {
        return Err("invalid subtab name".into());
    }
    Ok(())
}
fn validate_worktree_name(name: &str) -> Result<(), String> {
    validate_name(name)?;
    if name.starts_with('-')
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        return Err("invalid worktree branch name".into());
    }
    Ok(())
}
fn validate_root(root: &Path) -> Result<(), String> {
    let text = root.to_str().ok_or("subtab root must be UTF-8")?;
    if !root.is_absolute()
        || text.len() > 4096
        || text.chars().any(char::is_control)
        || root.components().any(|v| {
            matches!(
                v,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err("invalid subtab root".into());
    }
    // Legacy in-place roots can be planned but not created. Existing roots must be canonical.
    if root.exists() && root.canonicalize().map_err(|e| e.to_string())? != root {
        return Err("subtab root is not canonical".into());
    }
    Ok(())
}

impl SubtabState {
    pub fn main(project: &str, root: &Path) -> Result<Self, String> {
        Self::migrate(
            project,
            root,
            &[LegacySubtab {
                name: "main".into(),
                root: root.display().to_string(),
                kind: SubtabKind::Main,
                concurrency: 1,
            }],
            0,
        )
    }
    pub fn migrate(
        project: &str,
        root: &Path,
        legacy: &[LegacySubtab],
        active: usize,
    ) -> Result<Self, String> {
        if legacy.is_empty() || legacy.len() > MAX_SUBTABS || active >= legacy.len() {
            return Err("invalid legacy subtab selection or capacity".into());
        }
        let tabs = legacy
            .iter()
            .map(|tab| {
                let path = PathBuf::from(&tab.root);
                Ok(HostSubtab {
                    id: identity(project, &tab.kind, &path)?,
                    name: tab.name.clone(),
                    root: path,
                    kind: tab.kind.clone(),
                    concurrency: tab.concurrency,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let state = Self {
            schema_version: SUBTAB_SCHEMA_VERSION,
            revision: 0,
            active_id: tabs[active].id.clone(),
            tabs,
        };
        state.validate(project, root)?;
        Ok(state)
    }
    pub fn validate(&self, project: &str, root: &Path) -> Result<(), String> {
        if self.schema_version != SUBTAB_SCHEMA_VERSION
            || self.tabs.is_empty()
            || self.tabs.len() > MAX_SUBTABS
        {
            return Err("invalid subtab schema or capacity".into());
        }
        let mut ids = BTreeSet::new();
        let mut roots = BTreeSet::new();
        let mut names = BTreeSet::new();
        for (index, tab) in self.tabs.iter().enumerate() {
            validate_name(&tab.name)?;
            validate_root(&tab.root)?;
            if !(1..=64).contains(&tab.concurrency)
                || !ids.insert(&tab.id)
                || !roots.insert(&tab.root)
                || !names.insert(&tab.name)
                || tab.id != identity(project, &tab.kind, &tab.root)?
                || (index == 0) != (tab.kind == SubtabKind::Main)
                || (index == 0 && tab.root != root)
            {
                return Err("invalid subtab identity, root, name or concurrency".into());
            }
        }
        if !ids.contains(&self.active_id) {
            return Err("unknown active subtab".into());
        }
        Ok(())
    }
    fn index(&self, id: &str) -> Result<usize, String> {
        self.tabs
            .iter()
            .position(|tab| tab.id == id)
            .ok_or("unknown subtab".into())
    }
    fn changed(&self, mut next: Self) -> Result<Self, String> {
        if &next != self {
            next.revision = self
                .revision
                .checked_add(1)
                .ok_or("subtab revision exhausted")?;
        }
        Ok(next)
    }
    pub fn select(&self, id: &str) -> Result<Self, String> {
        self.index(id)?;
        let mut next = self.clone();
        next.active_id = id.into();
        self.changed(next)
    }
    pub fn rename(&self, id: &str, name: &str) -> Result<Self, String> {
        validate_name(name)?;
        let index = self.index(id)?;
        if self.tabs.iter().any(|tab| tab.id != id && tab.name == name) {
            return Err("duplicate subtab name".into());
        }
        let mut next = self.clone();
        next.tabs[index].name = name.into();
        self.changed(next)
    }
    pub fn move_before(&self, id: &str, before: Option<&str>) -> Result<Self, String> {
        let source = self.index(id)?;
        if source == 0 {
            return Err("main subtab cannot move".into());
        }
        if before == Some(id) {
            return Ok(self.clone());
        }
        if before.is_some_and(|id| self.tabs.first().is_some_and(|tab| tab.id == id)) {
            return Err("main subtab must stay first".into());
        }
        let mut next = self.clone();
        let tab = next.tabs.remove(source);
        let target = match before {
            Some(id) => next.index(id)?,
            None => next.tabs.len(),
        };
        next.tabs.insert(target, tab);
        self.changed(next)
    }
    pub fn set_concurrency(&self, id: &str, value: u16) -> Result<Self, String> {
        if !(1..=64).contains(&value) {
            return Err("subtab concurrency must be 1..64".into());
        }
        let index = self.index(id)?;
        let mut next = self.clone();
        next.tabs[index].concurrency = value;
        self.changed(next)
    }
    pub fn add_directory(
        &self,
        project: &str,
        base: &Path,
        root: &Path,
        name: &str,
    ) -> Result<Self, String> {
        validate_name(name)?;
        validate_root(root)?;
        if self.tabs.len() >= MAX_SUBTABS
            || !root.is_dir()
            || root == base
            || !root.starts_with(base)
            || self
                .tabs
                .iter()
                .any(|tab| tab.root == root || tab.name == name)
        {
            return Err("invalid or duplicate directory tab".into());
        }
        let id = identity(project, &SubtabKind::InPlace, root)?;
        let mut next = self.clone();
        next.tabs.push(HostSubtab {
            id: id.clone(),
            name: name.into(),
            root: root.into(),
            kind: SubtabKind::InPlace,
            concurrency: 1,
        });
        next.active_id = id;
        next.validate(project, base)?;
        self.changed(next)
    }
    pub fn close_directory(&self, id: &str) -> Result<Self, String> {
        let index = self.index(id)?;
        if self.tabs[index].kind != SubtabKind::InPlace {
            return Err("only directory tabs can be closed here".into());
        }
        self.close_secondary(id)
    }
    pub fn plan_worktree(
        &self,
        project: &str,
        base: &Path,
        root: &Path,
        name: &str,
    ) -> Result<Self, String> {
        validate_worktree_name(name)?;
        if root != base.join(".zenpi-worktrees").join(name)
            || self.tabs.len() >= MAX_SUBTABS
            || self
                .tabs
                .iter()
                .any(|tab| tab.root == root || tab.name == name)
        {
            return Err("invalid or duplicate worktree tab".into());
        }
        let mut next = self.clone();
        let id = identity(project, &SubtabKind::Worktree, root)?;
        next.tabs.push(HostSubtab {
            id: id.clone(),
            name: name.into(),
            root: root.into(),
            kind: SubtabKind::Worktree,
            concurrency: 1,
        });
        next.active_id = id;
        next.validate(project, base)?;
        self.changed(next)
    }
    pub(crate) fn close_secondary(&self, id: &str) -> Result<Self, String> {
        let index = self.index(id)?;
        if index == 0 {
            return Err("main subtab cannot close".into());
        }
        let mut next = self.clone();
        next.tabs.remove(index);
        if next.active_id == id {
            next.active_id = next.tabs[index.min(next.tabs.len() - 1)].id.clone();
        }
        self.changed(next)
    }
}
