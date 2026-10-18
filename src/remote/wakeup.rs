//! Content-free, bounded session change notifications. Not an event cursor or journal.

use std::{
    collections::BTreeMap,
    sync::{Arc, Condvar, Mutex, OnceLock, Weak},
    time::{Duration, Instant},
};

const MAX_WATCHED_SESSIONS: usize = 128;
#[derive(Default)]
pub struct SessionWakeup {
    revision: Mutex<u64>,
    changed: Condvar,
}
impl SessionWakeup {
    pub fn revision(&self) -> u64 {
        *self
            .revision
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
    /// Capture revision before reading the owner so concurrent changes cannot be lost.
    pub fn wait(&self, observed: u64, timeout: Duration) -> bool {
        let revision = self
            .revision
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let (revision, _) = self
            .changed
            .wait_timeout_while(revision, timeout, |revision| *revision == observed)
            .unwrap_or_else(|error| error.into_inner());
        *revision != observed
    }
    fn notify(&self) {
        let mut revision = self
            .revision
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *revision = revision.wrapping_add(1);
        self.changed.notify_all();
    }
}

#[derive(Default)]
struct Watchers(Mutex<BTreeMap<String, Weak<SessionWakeup>>>);
impl Watchers {
    fn watch(&self, session: &str) -> Option<Arc<SessionWakeup>> {
        if !super::auth::valid_identifier(session) {
            return None;
        }
        let mut values = self.0.lock().unwrap_or_else(|error| error.into_inner());
        values.retain(|_, value| value.strong_count() != 0);
        if let Some(signal) = values.get(session).and_then(Weak::upgrade) {
            return Some(signal);
        }
        if values.len() >= MAX_WATCHED_SESSIONS {
            return None;
        }
        let signal = Arc::new(SessionWakeup::default());
        values.insert(session.into(), Arc::downgrade(&signal));
        Some(signal)
    }
    fn notify(&self, session: &str) {
        let signal = self
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(session)
            .and_then(Weak::upgrade);
        if let Some(signal) = signal {
            signal.notify();
        }
    }
}
fn watchers() -> &'static Watchers {
    static VALUES: OnceLock<Watchers> = OnceLock::new();
    VALUES.get_or_init(Watchers::default)
}
pub fn watch(session: &str) -> Option<Arc<SessionWakeup>> {
    watchers().watch(session)
}
pub fn notify(session: &str) {
    watchers().notify(session);
}
pub fn wait(signal: Option<&SessionWakeup>, observed: u64, deadline: Instant) {
    let timeout =
        Duration::from_millis(250).min(deadline.saturating_duration_since(Instant::now()));
    if let Some(signal) = signal {
        signal.wait(observed, timeout);
    } else {
        std::thread::sleep(timeout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scope_capacity_and_dropped_watchers_do_not_create_unbounded_retention() {
        let values = Watchers::default();
        assert!(values.watch("").is_none());
        let first = values.watch("first").unwrap();
        let same = values.watch("first").unwrap();
        assert!(Arc::ptr_eq(&first, &same));
        let second = values.watch("second").unwrap();
        values.notify("first");
        assert_eq!(first.revision(), 1);
        assert_eq!(second.revision(), 0);
        let held: Vec<_> = (2..MAX_WATCHED_SESSIONS)
            .map(|id| values.watch(&format!("session-{id}")).unwrap())
            .collect();
        assert!(values.watch("excess").is_none());
        drop(held);
        assert!(values.watch("replacement").is_some());
    }
    #[test]
    fn changes_during_read_and_while_waiting_are_not_lost() {
        let signal = Arc::new(SessionWakeup::default());
        let observed = signal.revision();
        signal.notify();
        assert!(signal.wait(observed, Duration::ZERO));
        let observed = signal.revision();
        let other = signal.clone();
        let worker = std::thread::spawn(move || {
            other.notify();
        });
        assert!(signal.wait(observed, Duration::from_secs(1)));
        worker.join().unwrap();
        assert!(!signal.wait(signal.revision(), Duration::ZERO));
    }
}
