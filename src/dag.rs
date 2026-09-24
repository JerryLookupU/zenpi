//! DAG coordination for execution skills (ZS1-199).
//!
//! A worker owns exactly one DAG node and communicates with its parent,
//! grandparent, direct siblings and direct children through a shared,
//! file-locked JSON store. A node may only be closed when it is green and
//! every descendant is green; otherwise the worker stays alive and spawns
//! workers for the new work.
//!
//! The store is bounded: nodes and messages are capped, bodies are truncated,
//! and every read is a locked snapshot so concurrent workers cannot corrupt it.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Environment that identifies the node a headless worker owns.
pub const DAG_NODE_ENV: &str = "ZENPI_DAG_NODE";
/// Environment that points at the shared store (default: `<cwd>/.zenpi/dag.json`).
pub const DAG_STORE_ENV: &str = "ZENPI_DAG_STORE";
/// Override for the worker binary spawned by `dag_spawn` (tests, hosts).
pub const DAG_WORKER_ENV: &str = "ZENPI_DAG_WORKER";

pub const MAX_DAG_NODES: usize = 8_192;
pub const MAX_DAG_MESSAGES: usize = 16_384;
pub const MAX_DAG_BODY_BYTES: usize = 4_096;
pub const MAX_DAG_STATUS_BYTES: usize = 64;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn default_status() -> String {
    "open".to_owned()
}

fn valid_id(id: &str) -> bool {
    !id.trim().is_empty()
        && id.len() <= 128
        && !id.chars().any(|character| character.is_control())
        && !id.contains(char::is_whitespace)
}

fn valid_status(status: &str) -> bool {
    matches!(status, "open" | "green" | "red")
}

/// One DAG node. `status` is `open`, `green` or `red`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DagNode {
    pub id: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub worker: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub updated_ms: u64,
    /// Last keep-alive from the owning worker (learn: heartbeat-based
    /// liveness lets a parent see a stale/idle worker without IPC).
    #[serde(default)]
    pub worker_heartbeat_ms: u64,
}

/// Typed neighbour relation used by tools and hosts (learn: codex/opencode
/// agents address neighbours by role, not by ad-hoc strings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DagRelation {
    Parent,
    Grandparent,
    Sibling,
    Child,
    All,
}

impl DagRelation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Parent => "parent",
            Self::Grandparent => "grandparent",
            Self::Sibling => "sibling",
            Self::Child => "child",
            Self::All => "all",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "parent" => Some(Self::Parent),
            "grandparent" => Some(Self::Grandparent),
            "sibling" | "siblings" => Some(Self::Sibling),
            "child" | "children" => Some(Self::Child),
            "all" => Some(Self::All),
            _ => None,
        }
    }
}

/// One mailbox message between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagMessage {
    #[serde(default)]
    pub id: u64,
    pub from: String,
    pub to: String,
    pub body: String,
    #[serde(default)]
    pub ts_ms: u64,
    /// Claim/ack semantics (learn: codex + opencode agents acknowledge work so
    /// a restarted reader gets unacked messages redelivered instead of losing
    /// them).
    #[serde(default)]
    pub claimed_by: Option<String>,
    #[serde(default)]
    pub claimed_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DagFile {
    #[serde(default)]
    nodes: BTreeMap<String, DagNode>,
    #[serde(default)]
    messages: Vec<DagMessage>,
    #[serde(default)]
    next_message_id: u64,
}

/// File-locked view of the shared DAG store.
#[derive(Debug, Clone)]
pub struct DagStore {
    path: PathBuf,
}

impl DagStore {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// `ZENPI_DAG_STORE`, else `<cwd>/.zenpi/dag.json`.
    pub fn default_store() -> Self {
        if let Ok(path) = std::env::var(DAG_STORE_ENV) {
            let path = path.trim();
            if !path.is_empty() {
                return Self::at(path);
            }
        }
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::at(cwd.join(".zenpi").join("dag.json"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn ensure_parent(&self) -> Result<(), String> {
        let Some(parent) = self.path.parent() else {
            return Err("dag store has no parent directory".into());
        };
        fs::create_dir_all(parent).map_err(|error| format!("dag store dir: {error}"))
    }

    fn read_locked(&self) -> Result<DagFile, String> {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(DagFile::default());
            }
            Err(error) => return Err(format!("dag store open: {error}")),
        };
        lock_shared(&file)?;
        let mut text = String::new();
        file.read_to_string(&mut text)
            .map_err(|error| format!("dag store read: {error}"))?;
        unlock(&file);
        if text.trim().is_empty() {
            return Ok(DagFile::default());
        }
        serde_json::from_str(&text).map_err(|error| format!("dag store parse: {error}"))
    }

    /// Locked read-modify-write. The closure must keep the file bounded.
    fn update<T>(
        &self,
        apply: impl FnOnce(&mut DagFile) -> Result<T, String>,
    ) -> Result<T, String> {
        self.ensure_parent()?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)
            .map_err(|error| format!("dag store open: {error}"))?;
        lock_exclusive(&file)?;
        let mut text = String::new();
        file.read_to_string(&mut text)
            .map_err(|error| format!("dag store read: {error}"))?;
        let mut state: DagFile = if text.trim().is_empty() {
            DagFile::default()
        } else {
            serde_json::from_str(&text).map_err(|error| format!("dag store parse: {error}"))?
        };
        let result = apply(&mut state)?;
        state.messages.truncate(MAX_DAG_MESSAGES);
        let bytes =
            serde_json::to_vec(&state).map_err(|error| format!("dag store encode: {error}"))?;
        file.set_len(0)
            .map_err(|error| format!("dag store truncate: {error}"))?;
        file.seek(SeekFrom::Start(0))
            .map_err(|error| format!("dag store seek: {error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("dag store write: {error}"))?;
        unlock(&file);
        Ok(result)
    }

    pub fn nodes(&self) -> Result<Vec<DagNode>, String> {
        Ok(self.read_locked()?.nodes.into_values().collect())
    }

    pub fn node(&self, id: &str) -> Result<Option<DagNode>, String> {
        Ok(self.read_locked()?.nodes.get(id).cloned())
    }

    /// Create or re-parent a node. A missing parent is created as a root.
    pub fn upsert(&self, id: &str, parent: Option<&str>) -> Result<DagNode, String> {
        if !valid_id(id) {
            return Err("dag node id is invalid".into());
        }
        if let Some(parent) = parent
            && !valid_id(parent)
        {
            return Err("dag parent id is invalid".into());
        }
        self.update(|state| {
            if !state.nodes.contains_key(id) && state.nodes.len() >= MAX_DAG_NODES {
                return Err("dag store node limit exceeded".into());
            }
            if let Some(parent) = parent
                && !state.nodes.contains_key(parent)
                && state.nodes.len() >= MAX_DAG_NODES
            {
                return Err("dag store node limit exceeded".into());
            }
            if let Some(parent) = parent {
                state
                    .nodes
                    .entry(parent.to_owned())
                    .or_insert_with(|| DagNode {
                        id: parent.to_owned(),
                        parent: None,
                        children: Vec::new(),
                        status: default_status(),
                        worker: None,
                        summary: None,
                        updated_ms: now_ms(),
                        worker_heartbeat_ms: 0,
                    });
                let children = &mut state.nodes.get_mut(parent).expect("inserted").children;
                if !children.iter().any(|child| child == id) {
                    children.push(id.to_owned());
                }
            }
            let node = state.nodes.entry(id.to_owned()).or_insert_with(|| DagNode {
                id: id.to_owned(),
                parent: parent.map(str::to_owned),
                children: Vec::new(),
                status: default_status(),
                worker: None,
                summary: None,
                updated_ms: now_ms(),
                worker_heartbeat_ms: 0,
            });
            if parent.is_some() {
                node.parent = parent.map(str::to_owned);
            }
            node.updated_ms = now_ms();
            Ok(node.clone())
        })
    }

    pub fn set_status(
        &self,
        id: &str,
        status: &str,
        summary: Option<&str>,
    ) -> Result<DagNode, String> {
        if !valid_status(status) {
            return Err("dag status must be open, green or red".into());
        }
        self.update(|state| {
            let Some(node) = state.nodes.get_mut(id) else {
                return Err(format!("dag node not found: {id}"));
            };
            node.status = status.to_owned();
            if let Some(summary) = summary {
                let summary = summary.trim();
                node.summary = Some(summary.chars().take(MAX_DAG_STATUS_BYTES).collect());
            }
            node.updated_ms = now_ms();
            Ok(node.clone())
        })
    }

    pub fn assign_worker(&self, id: &str, worker: &str) -> Result<(), String> {
        self.update(|state| {
            let Some(node) = state.nodes.get_mut(id) else {
                return Err(format!("dag node not found: {id}"));
            };
            node.worker = Some(worker.to_owned());
            node.updated_ms = now_ms();
            Ok(())
        })
    }

    /// Keep-alive from the owning worker. `dag_status`/`dag_recv` call it so a
    /// parent can see whether a node's worker is still alive.
    pub fn touch_worker(&self, id: &str) -> Result<(), String> {
        self.update(|state| {
            let Some(node) = state.nodes.get_mut(id) else {
                return Err(format!("dag node not found: {id}"));
            };
            node.worker_heartbeat_ms = now_ms();
            Ok(())
        })
    }

    /// Resolve a `to` target: a node id or one of `parent`, `grandparent`,
    /// `sibling`, `child`/`children`, `all` (parent + grandparent + direct
    /// siblings + direct children).
    pub fn recipients(&self, from: &str, to: &str) -> Result<Vec<String>, String> {
        let state = self.read_locked()?;
        if !state.nodes.contains_key(from) {
            return Err(format!("dag node not found: {from}"));
        }
        let node = state.nodes.get(from).expect("checked");
        let mut out = Vec::new();
        match to.trim() {
            "parent" => {
                if let Some(parent) = &node.parent {
                    out.push(parent.clone());
                }
            }
            "grandparent" => {
                if let Some(parent) = node.parent.as_ref().and_then(|id| state.nodes.get(id)) {
                    if let Some(grand) = &parent.parent {
                        out.push(grand.clone());
                    }
                }
            }
            "sibling" | "siblings" => {
                if let Some(parent) = node.parent.as_ref().and_then(|id| state.nodes.get(id)) {
                    out.extend(
                        parent
                            .children
                            .iter()
                            .filter(|child| child.as_str() != from)
                            .cloned(),
                    );
                }
            }
            "child" | "children" => out.extend(node.children.iter().cloned()),
            "all" => {
                if let Some(parent) = &node.parent {
                    out.push(parent.clone());
                    if let Some(grand) = state
                        .nodes
                        .get(parent)
                        .and_then(|parent| parent.parent.clone())
                    {
                        out.push(grand.clone());
                    }
                    if let Some(parent) = state.nodes.get(parent) {
                        out.extend(
                            parent
                                .children
                                .iter()
                                .filter(|child| child.as_str() != from)
                                .cloned(),
                        );
                    }
                }
                out.extend(node.children.iter().cloned());
            }
            other => {
                if !valid_id(other) {
                    return Err("dag message target is invalid".into());
                }
                out.push(other.to_owned());
            }
        }
        out.sort();
        out.dedup();
        out.retain(|target| target != from);
        Ok(out)
    }

    /// Append one bounded message to each resolved recipient.
    pub fn send(&self, from: &str, to: &str, body: &str) -> Result<Vec<String>, String> {
        let targets = self.recipients(from, to)?;
        let body = body.trim();
        if body.is_empty() {
            return Err("dag message body is empty".into());
        }
        let body: String = body.chars().take(MAX_DAG_BODY_BYTES).collect();
        self.update(|state| {
            for target in &targets {
                state.next_message_id = state.next_message_id.max(
                    state
                        .messages
                        .iter()
                        .map(|message| message.id)
                        .max()
                        .unwrap_or(0),
                ) + 1;
                state.messages.push(DagMessage {
                    id: state.next_message_id,
                    from: from.to_owned(),
                    to: target.clone(),
                    body: body.clone(),
                    ts_ms: now_ms(),
                    claimed_by: None,
                    claimed_ms: 0,
                });
            }
            Ok(targets.clone())
        })
    }

    /// Claim unread messages for one node: new messages are marked claimed by
    /// the node and returned; messages already claimed by the same node (not
    /// yet acked) are redelivered. Callers ack with [`DagStore::ack`].
    pub fn claim_inbox(&self, node: &str) -> Result<Vec<DagMessage>, String> {
        self.update(|state| {
            let now = now_ms();
            let mut out = Vec::new();
            for message in &mut state.messages {
                if message.to != node {
                    continue;
                }
                match &message.claimed_by {
                    None => {
                        message.claimed_by = Some(node.to_owned());
                        message.claimed_ms = now;
                        out.push(message.clone());
                    }
                    Some(owner) if owner == node => out.push(message.clone()),
                    Some(_) => {}
                }
            }
            Ok(out)
        })
    }

    /// Acknowledge processed messages (only ones claimed by `node`).
    pub fn ack(&self, node: &str, ids: &[u64]) -> Result<usize, String> {
        self.update(|state| {
            let before = state.messages.len();
            state.messages.retain(|message| {
                !(ids.contains(&message.id) && message.claimed_by.as_deref() == Some(node))
            });
            Ok(before - state.messages.len())
        })
    }

    /// Claimed-but-unacked messages for one node.
    pub fn claimed_count(&self, node: &str) -> Result<usize, String> {
        Ok(self
            .read_locked()?
            .messages
            .iter()
            .filter(|message| message.claimed_by.as_deref() == Some(node))
            .count())
    }

    /// Unread messages for one node; `mark_read` consumes them.
    #[allow(dead_code)]
    pub fn inbox(&self, node: &str, mark_read: bool) -> Result<Vec<DagMessage>, String> {
        self.update(|state| {
            let mut out = Vec::new();
            for message in &mut state.messages {
                if message.to == node {
                    out.push(message.clone());
                }
            }
            if mark_read {
                state.messages.retain(|message| message.to != node);
            }
            Ok(out)
        })
    }

    pub fn unread_count(&self, node: &str) -> Result<usize, String> {
        Ok(self
            .read_locked()?
            .messages
            .iter()
            .filter(|message| message.to == node && message.claimed_by.is_none())
            .count())
    }

    /// Close gate: the node is green and every descendant is green. Returns
    /// the unfinished descendants when it is not closable.
    pub fn can_close(&self, id: &str) -> Result<(bool, Vec<String>), String> {
        let state = self.read_locked()?;
        let Some(node) = state.nodes.get(id) else {
            return Err(format!("dag node not found: {id}"));
        };
        let mut unfinished = Vec::new();
        if node.status != "green" {
            unfinished.push(id.to_owned());
        }
        let mut stack: Vec<String> = node.children.clone();
        let mut seen = 0usize;
        while let Some(child) = stack.pop() {
            seen += 1;
            if seen > MAX_DAG_NODES {
                break;
            }
            let Some(child_node) = state.nodes.get(&child) else {
                unfinished.push(child);
                continue;
            };
            if child_node.status != "green" {
                unfinished.push(child.clone());
            }
            stack.extend(child_node.children.iter().cloned());
        }
        unfinished.sort();
        unfinished.dedup();
        Ok((unfinished.is_empty(), unfinished))
    }
}

/// Node identity of the current process (headless worker or owner).
pub fn current_node() -> Option<String> {
    std::env::var(DAG_NODE_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Spawned direct children held by this process, keyed by node id, with the
/// prompt sequence used for follow-up messages.
static SPAWNED: Mutex<Option<BTreeMap<String, SpawnedWorker>>> = Mutex::new(None);

struct SpawnedWorker {
    child: Child,
    sequence: u64,
    log: PathBuf,
}

/// Spawn a headless worker for one new DAG node and keep its stdin open, so
/// `dag_send` can deliver follow-up work without restarting the process.
pub fn spawn_worker(
    store: &DagStore,
    node: &str,
    prompt: &str,
    session_dir: &Path,
) -> Result<u64, String> {
    if !valid_id(node) {
        return Err("dag node id is invalid".into());
    }
    let binary = std::env::var(DAG_WORKER_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "zenpi-dev".to_owned());
    fs::create_dir_all(session_dir).map_err(|error| format!("dag session dir: {error}"))?;
    let log_path = session_dir.join(format!("{node}.log"));
    let log = File::create(&log_path).map_err(|error| format!("dag worker log: {error}"))?;
    let session = session_dir.join(format!("{node}.jsonl"));
    let mut child = Command::new(&binary)
        .arg("--auto")
        .arg("--max-tool-iterations")
        .arg("200")
        .arg("--mode")
        .arg("headless")
        .arg("--session")
        .arg(&session)
        .env(DAG_NODE_ENV, node)
        .env(DAG_STORE_ENV, store.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::from(
            log.try_clone()
                .map_err(|error| format!("dag worker log clone: {error}"))?,
        ))
        .stderr(Stdio::from(log))
        .spawn()
        .map_err(|error| format!("dag worker spawn: {error}"))?;
    let pid = child.id();
    write_prompt(&mut child, node, 1, prompt)?;
    let mut guard = SPAWNED.lock().map_err(|_| "dag spawn registry poisoned")?;
    let registry = guard.get_or_insert_with(BTreeMap::new);
    registry.insert(
        node.to_owned(),
        SpawnedWorker {
            child,
            sequence: 1,
            log: log_path,
        },
    );
    Ok(u64::from(pid))
}

fn write_prompt(child: &mut Child, node: &str, sequence: u64, prompt: &str) -> Result<(), String> {
    let Some(stdin) = child.stdin.as_mut() else {
        return Err("dag worker has no stdin".into());
    };
    let frame = json!({
        "type": "prompt",
        "id": format!("{node}-{sequence}"),
        "text": prompt,
    });
    let mut line = serde_json::to_string(&frame).map_err(|error| error.to_string())?;
    line.push('\n');
    stdin
        .write_all(line.as_bytes())
        .map_err(|error| format!("dag worker write: {error}"))?;
    stdin
        .flush()
        .map_err(|error| format!("dag worker flush: {error}"))
}

/// Deliver follow-up work to a direct child spawned by this process. Returns
/// false when the node was not spawned here (the message still lands in the
/// store mailbox).
pub fn send_to_worker(node: &str, prompt: &str) -> Result<bool, String> {
    let mut guard = SPAWNED.lock().map_err(|_| "dag spawn registry poisoned")?;
    let Some(registry) = guard.as_mut() else {
        return Ok(false);
    };
    let Some(worker) = registry.get_mut(node) else {
        return Ok(false);
    };
    worker.sequence += 1;
    let sequence = worker.sequence;
    write_prompt(&mut worker.child, node, sequence, prompt)?;
    Ok(true)
}

pub fn spawned_nodes() -> Vec<String> {
    SPAWNED
        .lock()
        .ok()
        .and_then(|guard| {
            guard
                .as_ref()
                .map(|registry| registry.keys().cloned().collect())
        })
        .unwrap_or_default()
}

/// Bounded poll for the next unread message; cancellation-aware.
pub fn wait_for_message(
    store: &DagStore,
    node: &str,
    wait: Duration,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<DagMessage>, String> {
    let deadline = std::time::Instant::now() + wait.min(Duration::from_secs(30));
    loop {
        if cancelled() {
            return Err("cancelled".into());
        }
        let messages = store.claim_inbox(node)?;
        if !messages.is_empty() || std::time::Instant::now() >= deadline {
            return Ok(messages);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Snapshot for `dag_status`.
pub fn status_view(store: &DagStore, node: &str) -> Result<Value, String> {
    let state = store.read_locked()?;
    let Some(current) = state.nodes.get(node) else {
        return Err(format!("dag node not found: {node}"));
    };
    let (can_close, unfinished) = store.can_close(node)?;
    let children: Vec<Value> = current
        .children
        .iter()
        .map(|child| {
            let status = state
                .nodes
                .get(child)
                .map(|node| node.status.as_str())
                .unwrap_or("missing");
            json!({"id": child, "status": status})
        })
        .collect();
    let heartbeat = current.worker_heartbeat_ms;
    let idle_seconds = if heartbeat == 0 {
        None
    } else {
        Some(now_ms().saturating_sub(heartbeat) / 1000)
    };
    Ok(json!({
        "node": current.id,
        "parent": current.parent,
        "status": current.status,
        "children": children,
        "can_close": can_close,
        "unfinished": unfinished,
        "unread": store.unread_count(node)?,
        "claimed_unacked": store.claimed_count(node)?,
        "worker": current.worker,
        "worker_idle_seconds": idle_seconds,
        "relations": ["parent","grandparent","sibling","child","all"],
        "spawned_here": spawned_nodes().contains(&node.to_owned()),
    }))
}

#[cfg(unix)]
fn lock_exclusive(file: &File) -> Result<(), String> {
    use std::os::unix::io::AsRawFd;
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    if result == 0 {
        Ok(())
    } else {
        Err("dag store lock failed".into())
    }
}

#[cfg(unix)]
fn lock_shared(file: &File) -> Result<(), String> {
    use std::os::unix::io::AsRawFd;
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_SH) };
    if result == 0 {
        Ok(())
    } else {
        Err("dag store shared lock failed".into())
    }
}

#[cfg(unix)]
fn unlock(file: &File) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        libc::flock(file.as_raw_fd(), libc::LOCK_UN);
    }
}

#[cfg(not(unix))]
fn lock_exclusive(_: &File) -> Result<(), String> {
    Ok(())
}

#[cfg(not(unix))]
fn lock_shared(_: &File) -> Result<(), String> {
    Ok(())
}

#[cfg(not(unix))]
fn unlock(_: &File) {}
