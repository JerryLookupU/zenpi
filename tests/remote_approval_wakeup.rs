#![cfg(feature = "remote")]

use std::{
    sync::atomic::{AtomicBool, Ordering},
    sync::{Arc, mpsc},
    time::Duration,
};
use zenpi::{
    approval::{
        ApprovalDecision, ApprovalError, ApprovalPolicy, ApprovalRequest, ApprovalResponse,
    },
    core::Agent,
    remote::wakeup,
    session::SessionStore,
    tools::{ToolContext, ToolOrigin, ToolSideEffect},
};

struct Worker<T> {
    join: Option<std::thread::JoinHandle<T>>,
    stop: Arc<AtomicBool>,
}
impl<T> Worker<T> {
    fn join(mut self) -> T {
        self.join
            .take()
            .unwrap()
            .join()
            .expect("approval fixture panicked")
    }
}
impl<T> Drop for Worker<T> {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn request(id: &str) -> ApprovalRequest {
    ApprovalRequest {
        request_id: id.into(),
        turn_id: "turn".into(),
        call_id: "call".into(),
        tool: "write_file".into(),
        side_effect: ToolSideEffect::WorkspaceWrite,
        arguments: serde_json::json!({"content":"private fixture"}),
        preview: None,
        origin: ToolOrigin::AgentTool,
        policy_digest: None,
        lease_id: None,
    }
}
#[test]
fn approval_lifecycle_notifies_bound_session_and_rebind_preserves_shared_host_handle() {
    let root = tempfile::tempdir().unwrap();
    let session =
        SessionStore::open_in_workspace(root.path().join("old.jsonl"), root.path()).unwrap();
    let old_id = session.session_id().to_owned();
    let mut agent = Agent::with_echo(session);
    agent.set_attachment_workspace(ToolContext::new(root.path()).unwrap());
    let coordinator = agent.approval_coordinator().unwrap();
    let old = wakeup::watch(&old_id).unwrap();
    let foreign = wakeup::watch("approval-notify-foreign").unwrap();
    let observed = old.revision();
    let worker = coordinator.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let task = Worker {
        stop: stop.clone(),
        join: Some(std::thread::spawn(move || {
            worker.request_response(request("approval"), &mut ApprovalPolicy::default(), &|| {
                stopping.load(Ordering::Acquire)
            })
        })),
    };
    assert!(old.wait(observed, Duration::from_secs(2)));
    assert!(coordinator.is_pending("approval"));
    assert_eq!(foreign.revision(), 0);
    let observed = old.revision();
    assert!(
        coordinator
            .respond_with_digest(
                ApprovalResponse {
                    request_id: "approval".into(),
                    decision: ApprovalDecision::Allow,
                    remember: false,
                    message: None
                },
                "wrong-digest"
            )
            .is_err()
    );
    assert!(!old.wait(observed, Duration::ZERO));
    coordinator
        .respond(ApprovalResponse {
            request_id: "approval".into(),
            decision: ApprovalDecision::Deny,
            remember: false,
            message: None,
        })
        .unwrap();
    assert!(old.wait(observed, Duration::ZERO));
    assert_eq!(task.join().unwrap().decision, ApprovalDecision::Deny);
    coordinator.mark_persisted("approval").unwrap();
    stop.store(false, Ordering::Release);

    let next = SessionStore::open_in_workspace(root.path().join("new.jsonl"), root.path()).unwrap();
    let next_id = next.session_id().to_owned();
    drop(next);
    let new = wakeup::watch(&next_id).unwrap();
    let before_new = new.revision();
    let before_old = old.revision();
    assert!(
        agent
            .resume_session(root.path().join("missing.jsonl"))
            .is_err()
    );
    assert!(!new.wait(before_new, Duration::ZERO));
    assert!(!old.wait(before_old, Duration::ZERO));
    agent.resume_session(root.path().join("new.jsonl")).unwrap();
    assert!(new.wait(before_new, Duration::ZERO));
    let before_old = old.revision();
    let before_new = new.revision();
    let worker = agent.approval_coordinator().unwrap();
    let stopping = stop.clone();
    let task = Worker {
        stop: stop.clone(),
        join: Some(std::thread::spawn(move || {
            worker.request_response(
                request("next-approval"),
                &mut ApprovalPolicy::default(),
                &|| stopping.load(Ordering::Acquire),
            )
        })),
    };
    assert!(new.wait(before_new, Duration::from_secs(2)));
    assert!(!old.wait(before_old, Duration::ZERO));
    // The host-held clone still sees and answers the actual coordinator after resume.
    assert!(coordinator.is_pending("next-approval"));
    let observed = new.revision();
    coordinator.cancel_all();
    assert!(new.wait(observed, Duration::ZERO));
    assert_eq!(task.join().unwrap().decision, ApprovalDecision::Deny);
    coordinator.mark_persisted("next-approval").unwrap();
    stop.store(false, Ordering::Release);
    assert_eq!(foreign.revision(), 0);

    let observed = new.revision();
    let worker = coordinator.clone();
    let stopping = stop.clone();
    let task = Worker {
        stop: stop.clone(),
        join: Some(std::thread::spawn(move || {
            worker.request(
                request("cancelled"),
                &mut ApprovalPolicy::default(),
                &|| stopping.load(Ordering::Acquire),
            )
        })),
    };
    assert!(new.wait(observed, Duration::from_secs(2)));
    let observed = new.revision();
    stop.store(true, Ordering::Release);
    assert!(task.join().is_err());
    assert!(new.wait(observed, Duration::ZERO));
    assert!(!coordinator.is_pending("cancelled"));
}

#[test]
fn cancellation_discards_digest_fenced_allow_before_waiter_consumes_it() {
    let root = tempfile::tempdir().unwrap();
    let session =
        SessionStore::open_in_workspace(root.path().join("session.jsonl"), root.path()).unwrap();
    let mut agent = Agent::with_echo(session);
    agent.set_attachment_workspace(ToolContext::new(root.path()).unwrap());
    let coordinator = agent.approval_coordinator().unwrap();
    let watch = wakeup::watch(agent.session().session_id()).unwrap();
    let approval = request("allow-before-cancel");
    let digest = approval.digest().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let worker = coordinator.clone();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let task = Worker {
        stop: stop.clone(),
        join: Some(std::thread::spawn(move || {
            let mut policy = ApprovalPolicy::default();
            worker.request_response(approval, &mut policy, &|| {
                entered_tx.send(()).unwrap();
                // Hold the predicate outside the coordinator lock so the host
                // can accept Allow before cancellation is observed by the waiter.
                release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                stopping.load(Ordering::Acquire)
            })
        })),
    };
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let before_answer = watch.revision();
    let answer = ApprovalResponse {
        request_id: "allow-before-cancel".into(),
        decision: ApprovalDecision::Allow,
        remember: false,
        message: None,
    };
    coordinator
        .respond_with_digest(answer.clone(), &digest)
        .unwrap();
    assert!(watch.wait(before_answer, Duration::ZERO));
    assert!(!coordinator.is_pending("allow-before-cancel"));
    let accepted = coordinator.accepted("allow-before-cancel").unwrap();
    assert_eq!(accepted.response, answer);
    assert!(matches!(
        coordinator.respond_with_digest(answer.clone(), &digest),
        Err(ApprovalError::UnknownRequest)
    ));
    let before_cancel = watch.revision();
    stop.store(true, Ordering::Release);
    release_tx.send(()).unwrap();
    assert!(matches!(task.join(), Err(ApprovalError::Cancelled)));
    assert!(watch.wait(before_cancel, Duration::ZERO));
    assert!(coordinator.accepted("allow-before-cancel").is_none());
    assert!(coordinator.drain_pending().is_empty());
    assert!(matches!(
        coordinator.mark_persisted("allow-before-cancel"),
        Err(ApprovalError::UnknownRequest)
    ));
    assert!(matches!(
        coordinator.respond_with_digest(answer, &digest),
        Err(ApprovalError::UnknownRequest)
    ));
}
