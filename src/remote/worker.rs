//! Independent worker journals, never aliases of a project's master writer.

use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use super::subtab::SubtabExecution;
use crate::{core::Agent, project_workspace::OwnerControl};

pub const MAX_WORKER_OWNERS: usize = 64;

#[derive(Clone)]
pub struct WorkerOwner(Arc<WorkerState>);

#[derive(Debug, Clone)]
pub(crate) struct HistoryEvidence {
    pub digest: String,
    pub slot: u16,
    pub session: String,
}

struct WorkerState {
    agent: Mutex<Agent>,
    execution: SubtabExecution,
    control: OwnerControl,
    // The lease outlives the pool when an executing caller retains its handle.
    _lease: WorkerLease,
    runtime_owned: AtomicBool,
    master_session_id: String,
    slot: u16,
}

pub(crate) struct WorkerLease(File);

impl Drop for WorkerLease {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;
        // Closing only our descriptor may retain flock through a concurrent
        // fork's inherited descriptor until exec. Release on every owner path,
        // including recovery probes and preparation failures.
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

pub(crate) struct WorkerClaim(WorkerOwner);

impl Drop for WorkerClaim {
    fn drop(&mut self) {
        self.0.0.runtime_owned.store(false, Ordering::Release);
    }
}

impl WorkerOwner {
    pub(crate) fn slot(&self) -> u16 {
        self.0.slot
    }
    pub fn master_session_id(&self) -> &str {
        &self.0.master_session_id
    }
    pub(crate) fn runtime_owned(&self) -> bool {
        self.0.runtime_owned.load(Ordering::Acquire)
    }

    pub(crate) fn claim_runtime(&self) -> Result<WorkerClaim, String> {
        let agent = self.agent().try_lock().map_err(|_| "worker is busy")?;
        if !Self::idle(&agent)
            || self
                .0
                .runtime_owned
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return Err("worker already has an execution runtime".into());
        }
        Ok(WorkerClaim(self.clone()))
    }
    pub fn agent(&self) -> &Mutex<Agent> {
        &self.0.agent
    }

    pub fn execution(&self) -> &SubtabExecution {
        &self.0.execution
    }

    pub fn control(&self) -> OwnerControl {
        self.0.control.clone()
    }

    pub(crate) fn idle(agent: &Agent) -> bool {
        agent.phase() == crate::core::AgentPhase::Idle
            && !agent.input_port().has_pending()
            && !agent.input_port().is_preparing()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare(
        project: &str,
        master_session_id: &str,
        master_path: &Path,
        execution: SubtabExecution,
        slot: u16,
        overrides: crate::config::ConfigOverrides,
        echo_fixture: bool,
        auto_approve: bool,
    ) -> Result<Self, String> {
        execution.validate()?;
        let key = key(project, master_session_id, &execution.tab.id, slot)?;
        let private = directory(
            &master_path
                .parent()
                .ok_or("master journal has no parent")?
                .join(".zenpi"),
            false,
        )?;
        let directory = directory(&private.join("workers"), true)?;
        let lease_path = directory.join(format!("{key}.lease"));
        let lease = lease(&lease_path)?;
        let journal = directory.join(format!("{key}.jsonl"));
        validate_journal(&journal)?;
        if journal.exists() {
            let saved = crate::session::SessionStore::open_existing(&journal)
                .map_err(|error| error.to_string())?;
            if saved.header().cwd != execution.tab.root.to_string_lossy() {
                return Err("worker journal workspace differs from its subtab".into());
            }
        }
        let agent = Agent::prepare_project_with_approval(
            &journal,
            &execution.tab.root,
            overrides,
            echo_fixture,
            auto_approve,
        )
        .map_err(|error| error.to_string())?;
        if agent.session().header().cwd != execution.tab.root.to_string_lossy() {
            return Err("worker journal workspace differs from its subtab".into());
        }
        validate_journal(&journal)?;
        let control = OwnerControl {
            project: project.into(),
            input_port: agent.input_port(),
            approval: agent.approval_coordinator(),
        };
        Ok(Self(Arc::new(WorkerState {
            agent: Mutex::new(agent),
            execution,
            control,
            _lease: lease,
            runtime_owned: AtomicBool::new(false),
            master_session_id: master_session_id.into(),
            slot,
        })))
    }
}

pub(crate) fn key(project: &str, session: &str, tab: &str, slot: u16) -> Result<String, String> {
    if !super::auth::valid_identifier(project)
        || !super::auth::valid_identifier(session)
        || !super::auth::valid_identifier(tab)
        || slot >= crate::runtime::MAX_CONCURRENCY
    {
        return Err("invalid worker project/session/subtab/slot identity".into());
    }
    let bytes = serde_json::to_vec(&("zenpi-worker-v1", project, session, tab, slot))
        .map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn directory(path: &Path, private: bool) -> Result<PathBuf, String> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};
    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700);
    match builder.create(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.to_string()),
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & (if private { 0o077 } else { 0o022 }) != 0
        || path.canonicalize().map_err(|error| error.to_string())? != path
    {
        return Err("worker journal directory must be canonical, private and user-owned".into());
    }
    Ok(path.to_owned())
}

fn private_file(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.is_file()
        && !metadata.file_type().is_symlink()
        && metadata.nlink() == 1
        && metadata.uid() == unsafe { libc::geteuid() }
        && metadata.mode() & 0o077 == 0
}

fn validate_journal(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if private_file(&metadata) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err("worker journal must be a private regular file without aliases".into()),
    }
}

/// Read a retired worker journal without preparing an Agent, lease or directory.
pub(crate) fn read_existing_journal(path: &Path) -> Result<crate::session::SessionStore, String> {
    use std::os::unix::fs::MetadataExt;
    let parent = path.parent().ok_or("worker journal has no directory")?;
    let directory = std::fs::symlink_metadata(parent).map_err(|error| error.to_string())?;
    if !directory.is_dir()
        || directory.file_type().is_symlink()
        || directory.uid() != unsafe { libc::geteuid() }
        || directory.mode() & 0o077 != 0
        || parent.canonicalize().map_err(|error| error.to_string())? != parent
    {
        return Err("unsafe worker journal directory".into());
    }
    validate_journal(path)?;
    let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > 8 * 1024 * 1024 {
        return Err("worker transcript journal limit".into());
    }
    let journal = crate::session::SessionStore::open_existing_bounded(path, 8 * 1024 * 1024)
        .map_err(|error| error.to_string())?;
    validate_journal(path)?;
    let after = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.dev() != after.dev()
        || metadata.ino() != after.ino()
        || metadata.len() != after.len()
        || metadata.mtime() != after.mtime()
        || metadata.mtime_nsec() != after.mtime_nsec()
    {
        return Err("worker journal changed during observation".into());
    }
    Ok(journal)
}

fn lease(path: &Path) -> Result<WorkerLease, String> {
    open_lease(path, true)
}

pub(crate) fn existing_lease(path: &Path) -> Result<Option<WorkerLease>, String> {
    use std::os::unix::fs::MetadataExt;
    let parent = path.parent().ok_or("worker lease has no directory")?;
    match std::fs::symlink_metadata(parent) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
        Ok(metadata)
            if metadata.is_dir()
                && !metadata.file_type().is_symlink()
                && metadata.uid() == unsafe { libc::geteuid() }
                && metadata.mode() & 0o077 == 0
                && parent.canonicalize().map_err(|error| error.to_string())? == parent => {}
        _ => return Err("worker lease directory must be canonical, private and user-owned".into()),
    }
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
        Ok(_) => open_lease(path, false).map(Some),
    }
}

fn open_lease(path: &Path, create: bool) -> Result<WorkerLease, String> {
    use std::os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    };
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(create)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|error| error.to_string())?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    let actual = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !private_file(&metadata)
        || !private_file(&actual)
        || metadata.dev() != actual.dev()
        || metadata.ino() != actual.ino()
    {
        return Err("worker lease must be a private regular file without aliases".into());
    }
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err("worker journal already has an execution owner".into());
    }
    Ok(WorkerLease(file))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{project_workspace::ProjectOwnerPool, session::SessionStore};
    use std::os::fd::{AsRawFd, FromRawFd};

    #[test]
    fn transcript_observation_creates_nothing_and_refuses_nonprivate_or_aliased_journals() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let directory = root.join("workers");
        let path = directory.join("journal.jsonl");
        assert!(read_existing_journal(&path).is_err());
        assert!(!directory.exists());
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let session = SessionStore::open_in_workspace(&path, &root).unwrap();
        drop(session);
        let bytes = std::fs::read(&path).unwrap();
        assert!(read_existing_journal(&path).is_ok());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let alias = directory.join("alias");
        std::fs::hard_link(&path, &alias).unwrap();
        assert!(read_existing_journal(&path).is_err());
        std::fs::remove_file(&alias).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read_existing_journal(&path).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(read_existing_journal(&path).is_err());
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::rename(&path, &alias).unwrap();
        std::os::unix::fs::symlink(&alias, &path).unwrap();
        assert!(read_existing_journal(&path).is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::rename(&alias, &path).unwrap();
        let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_len(8 * 1024 * 1024 + 1).unwrap();
        assert!(read_existing_journal(&path).is_err());
    }

    #[test]
    fn recovery_lease_probe_creates_nothing_and_refuses_live_or_aliased_ownership() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let directory = root.join("workers");
        let path = directory.join("slot.lease");
        assert!(existing_lease(&path).unwrap().is_none());
        assert!(!directory.exists());
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(existing_lease(&path).unwrap().is_none());
        assert!(!path.exists());
        let held = lease(&path).unwrap();
        assert!(existing_lease(&path).is_err());
        drop(held);
        drop(existing_lease(&path).unwrap().unwrap());
        let alias = directory.join("alias.lease");
        std::fs::hard_link(&path, &alias).unwrap();
        assert!(existing_lease(&path).is_err());
        std::fs::remove_file(alias).unwrap();
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(root.join("outside"), &path).unwrap();
        assert!(existing_lease(&path).is_err());
        assert!(!root.join("outside").exists());
        std::fs::remove_file(&path).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(existing_lease(&path).is_err());
        std::fs::remove_dir(&directory).unwrap();
        std::os::unix::fs::symlink(&root, &directory).unwrap();
        assert!(existing_lease(&path).is_err());
        assert!(!root.join("slot.lease").exists());
    }

    #[test]
    fn recovery_and_preparation_leases_unlock_without_waiting_for_inherited_descriptor_close() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let directory = directory(&root.join("workers"), true).unwrap();
        let path = directory.join("slot.lease");
        for recovery in [false, true] {
            let held = if recovery {
                existing_lease(&path).unwrap().unwrap()
            } else {
                lease(&path).unwrap()
            };
            let descriptor = unsafe { libc::dup(held.0.as_raw_fd()) };
            assert!(descriptor >= 0);
            let inherited = unsafe { File::from_raw_fd(descriptor) };
            assert!(existing_lease(&path).is_err());
            drop(held);
            let replacement = existing_lease(&path).unwrap().unwrap();
            drop(inherited);
            assert!(
                existing_lease(&path).is_err(),
                "retired descriptor cannot unlock replacement"
            );
            drop(replacement);
            assert!(existing_lease(&path).unwrap().is_some());
        }
    }

    #[test]
    fn last_worker_handle_unlocks_even_when_an_inherited_descriptor_is_still_open() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let new_pool = || {
            let master = Arc::new(Mutex::new(Agent::with_echo(
                SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
            )));
            let mut pool = ProjectOwnerPool::new(master).unwrap();
            pool.restore_checkpoint().unwrap();
            pool
        };
        let mut pool = new_pool();
        let project = pool.active_context().project_id.clone();
        let tab = pool.subtab_state(&project).unwrap().active_id;
        let worker = pool.worker_owner(&project, &tab, 0).unwrap();
        // dup has the same open-file description as fork inheritance, without
        // invoking non-async-signal-safe Rust code in a multithreaded child.
        let descriptor = unsafe { libc::dup(worker.0._lease.0.as_raw_fd()) };
        assert!(descriptor >= 0);
        let inherited = unsafe { File::from_raw_fd(descriptor) };
        drop(pool);
        let mut second = new_pool();
        assert!(second.worker_owner(&project, &tab, 0).is_err());
        drop(worker);
        let worker = second.worker_owner(&project, &tab, 0).unwrap();
        drop(inherited);
        let mut third = new_pool();
        assert!(
            third.worker_owner(&project, &tab, 0).is_err(),
            "old descriptor close must not unlock the new writer"
        );
        drop(worker);
        drop(second);
        assert!(third.worker_owner(&project, &tab, 0).is_ok());
    }
}
