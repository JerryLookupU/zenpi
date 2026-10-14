//! Worktree, worker and subtab subsystem shared by the TUI and the remote host
//! (frozen by §7 of the execution blueprint; moved out of `remote` by ZE-511).

pub mod subtab;
pub mod worker;
pub mod worker_admission;
pub mod worker_progress;
pub(crate) mod worktree_host;
pub mod worktree_runner;
