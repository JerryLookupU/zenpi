//! Worktree, worker and subtab subsystem shared by the TUI and the remote host
//! (frozen by §7 of the execution blueprint; moved out of `remote` by ZE-511).

pub mod subtab;
#[cfg(feature = "remote")]
pub mod worker;
#[cfg(feature = "remote")]
pub mod worker_admission;
#[cfg(feature = "remote")]
pub mod worker_progress;
#[cfg(feature = "remote")]
pub(crate) mod worktree_host;
#[cfg(feature = "remote")]
pub mod worktree_runner;
