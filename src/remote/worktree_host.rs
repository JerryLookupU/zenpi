//! Controller-owned lazy worktree lanes; no worker is reconstructed for a receipt query.

use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use super::{
    auth::{AuthenticatedDevice, DeviceRegistry},
    host_adapter::HostAdapter,
    protocol::{Envelope, ErrorCode, Message, Operation},
    worker_progress::WorkerProgress,
    worktree_runner::{WorkerPrompt, WorktreeRunner},
};
use crate::{
    core::{AgentError, ProcessResult},
    project_workspace::ProjectOwnerPool,
    runtime::RuntimeConfig,
};

type Runner = WorktreeRunner<WorkerPrompt, ProcessResult, AgentError>;
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LocalCancelReport {
    pub requested: usize,
    pub deferred: usize,
}
struct Lane {
    runner: Runner,
    progress: WorkerProgress,
    live: Arc<super::live_text::LivePromptText>,
}
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::Agent, session::SessionStore};
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };

    fn phone(registry: &DeviceRegistry, project: &str) -> AuthenticatedDevice {
        let random = ring::rand::SystemRandom::new();
        let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
        let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random)
            .unwrap();
        let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
        let record = registry
            .verify_pair_and_register(
                "token",
                "pair",
                "phone",
                &public,
                &URL_SAFE_NO_PAD.encode(
                    key.sign(
                        &random,
                        &super::super::auth::pairing_proof_message(
                            &"a".repeat(64),
                            "token",
                            "pair",
                            "phone",
                            &public,
                        ),
                    )
                    .unwrap()
                    .as_ref(),
                ),
                || true,
            )
            .unwrap();
        registry
            .set_permissions(
                &record.id,
                record.revision,
                vec![
                    super::super::auth::Capability::Read,
                    super::super::auth::Capability::Control,
                ],
                vec![project.into()],
            )
            .unwrap();
        let mut challenge = super::super::auth::DeviceChallenge::new("a".repeat(64)).unwrap();
        challenge
            .authenticate(
                registry,
                &record.id,
                &URL_SAFE_NO_PAD.encode(
                    key.sign(&random, &challenge.proof_message(&record.id))
                        .unwrap()
                        .as_ref(),
                ),
            )
            .unwrap()
    }

    #[test]
    fn desktop_cancel_targets_one_subtab_across_devices_and_retains_noncooperative_ownership() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::create_dir(root.join("sibling")).unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let initial = pool.subtab_state(&project).unwrap();
        let main = initial.active_id.clone();
        let two = initial
            .add_directory(&project, &root, &root.join("sibling"), "sibling")
            .unwrap();
        let sibling = two.active_id.clone();
        pool.commit_subtabs(&project, initial.revision, two)
            .unwrap();
        let state = pool.subtab_state(&project).unwrap();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let devices = [phone(&registry, &project), phone(&registry, &project)];
        let release = Arc::new(AtomicBool::new(false));
        let entered = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cancelled = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut controller = WorktreeHost::default();
        for tab in [&main, &sibling] {
            let gate = release.clone();
            let starts = entered.clone();
            let cancels = cancelled.clone();
            let runner = Runner::spawn(
                &mut pool,
                &project,
                tab,
                RuntimeConfig::default(),
                move |_, _: WorkerPrompt, token| {
                    starts.fetch_add(1, Ordering::AcqRel);
                    let deadline = Instant::now() + Duration::from_secs(10);
                    let mut saw_cancel = false;
                    while !gate.load(Ordering::Acquire) {
                        if token.is_cancelled() && !saw_cancel {
                            cancels.fetch_add(1, Ordering::AcqRel);
                            saw_cancel = true;
                        }
                        assert!(Instant::now() < deadline);
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(AgentError::InvalidTurn("controlled worker exit".into()))
                },
            )
            .unwrap();
            controller.lanes.insert(
                (project.clone(), tab.clone()),
                Lane {
                    runner,
                    progress: WorkerProgress::default(),
                    live: Arc::new(super::super::live_text::LivePromptText::default()),
                },
            );
        }
        let mut receipts = Vec::new();
        for (device, id, tab) in [
            (&devices[0], "same-id", &main),
            (&devices[1], "same-id", &main),
            (&devices[0], "sibling-job", &sibling),
        ] {
            let prompt = request(
                &pool,
                id,
                Operation::Prompt,
                json!({"text":"wait", "subtab_id":tab, "subtab_revision":state.revision}),
            );
            let response = controller.dispatch_prompt(&mut pool, &registry, device, &prompt, false);
            assert_eq!(
                response.execution_state,
                Some(super::super::protocol::ExecutionState::Accepted)
            );
            receipts.push((prompt, device.clone(), response));
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while entered.load(Ordering::Acquire) != 2 {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        let before = std::fs::read(root.join("project-workspace.json")).unwrap();
        let local = controller.local_status(&pool, &project, &main).unwrap();
        assert_eq!(local.len(), 2);
        assert!(
            local
                .iter()
                .all(|task| task.request == "same-id" && task.runtime_active)
        );
        assert_ne!(local[0].device, local[1].device);
        assert_eq!(
            controller
                .local_status(&pool, &project, &sibling)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            controller.cancel_local(&pool, "foreign", &main),
            Err(ErrorCode::InvalidRequest)
        );
        assert_eq!(
            controller.cancel_local(&pool, &project, "foreign"),
            Err(ErrorCode::InvalidRequest)
        );
        let held_master = master.lock().unwrap();
        assert_eq!(
            controller.local_status(&pool, &project, &main),
            Err(ErrorCode::Busy)
        );
        assert_eq!(
            controller.cancel_local(&pool, &project, &main).unwrap(),
            LocalCancelReport {
                requested: 2,
                deferred: 0
            }
        );
        drop(held_master);
        while cancelled.load(Ordering::Acquire) != 1 {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        // The queued job is cancelled without starting; the running writer ignores cancellation until release.
        controller.poll(&mut pool);
        assert_eq!(entered.load(Ordering::Acquire), 2);
        assert!(controller.busy(&project));
        let pending = controller.listing(&pool, devices[0].id(), &project);
        assert!(
            pending
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["request_id"] == "same-id" && item["state"] == "running"),
            "cancel admission must not synthesize a terminal state"
        );
        assert_eq!(pool.worker_handles(&project).len(), 2);
        assert_eq!(
            std::fs::read(root.join("project-workspace.json")).unwrap(),
            before
        );
        for (prompt, device, response) in &receipts {
            assert_eq!(
                controller.dispatch_prompt(&mut pool, &registry, device, prompt, false),
                *response
            );
        }
        release.store(true, Ordering::Release);
        while controller.busy(&project) {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        let main_status = super::super::worker_progress::listing(&pool, devices[0].id(), &project);
        assert!(
            main_status
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["request_id"] == "same-id" && item["state"] == "cancelled")
        );
        assert!(
            main_status
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["request_id"] == "sibling-job" && item["state"] == "failed")
        );
        assert_eq!(
            super::super::worker_progress::listing(&pool, devices[1].id(), &project)[0]["state"],
            "cancelled"
        );
        assert!(pool.worker_handles(&project).is_empty());
        let final_status = controller.local_status(&pool, &project, &main).unwrap();
        assert_eq!(final_status.len(), 2);
        assert!(final_status.iter().all(|task| task.state
            == super::super::protocol::ExecutionState::Cancelled
            && !task.runtime_active));
        assert_eq!(
            controller.cancel_local(&pool, &project, &main).unwrap(),
            LocalCancelReport::default()
        );
        assert_eq!(cancelled.load(Ordering::Acquire), 1);
    }

    #[test]
    fn phone_listing_retains_later_lane_live_work_before_earlier_lane_terminal_tail() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::create_dir(root.join("sibling")).unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let initial = pool.subtab_state(&project).unwrap();
        let next = initial
            .add_directory(&project, &root, &root.join("sibling"), "sibling")
            .unwrap();
        let mut tabs = [initial.active_id.clone(), next.active_id.clone()];
        tabs.sort();
        pool.commit_subtabs(&project, initial.revision, next)
            .unwrap();
        let state = pool.subtab_state(&project).unwrap();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let release = Arc::new(AtomicBool::new(false));
        let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut controller = WorktreeHost::default();
        for (index, tab) in tabs.iter().enumerate() {
            let gate = release.clone();
            let done = completed.clone();
            let runner = Runner::spawn(
                &mut pool,
                &project,
                tab,
                RuntimeConfig {
                    command_capacity: 256,
                    event_capacity: 512,
                    max_pending: 128,
                    ..RuntimeConfig::default()
                },
                move |_, _: WorkerPrompt, token| {
                    if index == 1 {
                        let deadline = Instant::now() + Duration::from_secs(10);
                        while !gate.load(Ordering::Acquire) && !token.is_cancelled() {
                            assert!(Instant::now() < deadline);
                            std::thread::sleep(Duration::from_millis(1));
                        }
                    }
                    done.fetch_add(1, Ordering::AcqRel);
                    Err(AgentError::InvalidTurn("controlled terminal".into()))
                },
            )
            .unwrap();
            controller.lanes.insert(
                (project.clone(), tab.clone()),
                Lane {
                    runner,
                    progress: WorkerProgress::default(),
                    live: Arc::new(super::super::live_text::LivePromptText::default()),
                },
            );
        }
        for index in 0..17 {
            let tab = &tabs[usize::from(index == 16)];
            let prompt = request(
                &pool,
                &format!("task-{index}"),
                Operation::Prompt,
                json!({"text":"hold", "subtab_id":tab,"subtab_revision":state.revision}),
            );
            assert_eq!(
                controller
                    .dispatch_prompt(&mut pool, &registry, &device, &prompt, false)
                    .execution_state,
                Some(super::super::protocol::ExecutionState::Accepted)
            );
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while completed.load(Ordering::Acquire) < 16 {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        let guard = master.lock().unwrap();
        loop {
            controller.poll(&mut pool);
            if controller.lanes[&(project.clone(), tabs[0].clone())]
                .progress
                .local_status(
                    &project,
                    pool.active_context().session_id.as_str(),
                    &tabs[0],
                )
                .iter()
                .all(|task| !task.runtime_active)
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        let wire = controller.listing(&pool, device.id(), &project);
        assert_eq!(wire.as_array().unwrap().len(), 16);
        assert_eq!(wire[0]["request_id"], "task-16");
        assert_eq!(wire[0]["runtime_active"], true);
        assert!(
            wire.as_array().unwrap()[1..]
                .iter()
                .all(|task| task["state"] == "failed" && task.get("runtime_active").is_none())
        );
        drop(guard);
        release.store(true, Ordering::Release);
        while controller.busy(&project) {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(pool.worker_handles(&project).is_empty());
    }

    #[test]
    fn phone_can_cancel_ambiguous_progress_only_while_original_runtime_is_alive() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let tabs = pool.subtab_state(&project).unwrap();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let foreign = phone(&registry, &project);
        let entered = Arc::new(AtomicBool::new(false));
        let started = entered.clone();
        let release = Arc::new(AtomicBool::new(false));
        let gate = release.clone();
        let runner = Runner::spawn(
            &mut pool,
            &project,
            &tabs.active_id,
            RuntimeConfig::default(),
            move |_, _: WorkerPrompt, token| {
                started.store(true, Ordering::Release);
                let deadline = Instant::now() + Duration::from_secs(10);
                while !token.is_cancelled() && !gate.load(Ordering::Acquire) {
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(AgentError::InvalidTurn("controlled exit".into()))
            },
        )
        .unwrap();
        let mut controller = WorktreeHost::default();
        controller.lanes.insert(
            (project.clone(), tabs.active_id.clone()),
            Lane {
                runner,
                progress: WorkerProgress::default(),
                live: Arc::new(super::super::live_text::LivePromptText::default()),
            },
        );
        let prompt = request(
            &pool,
            "ambiguous-runtime",
            Operation::Prompt,
            json!({"text":"hold", "subtab_id":tabs.active_id,"subtab_revision":tabs.revision}),
        );
        let admitted = controller.dispatch_prompt(&mut pool, &registry, &device, &prompt, false);
        assert_eq!(
            admitted.execution_state,
            Some(super::super::protocol::ExecutionState::Accepted)
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while !entered.load(Ordering::Acquire) {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        // Replace the path while preserving the original inode to force a progress append refusal.
        let path = master.lock().unwrap().session().path().to_owned();
        let original = path.with_extension("original");
        std::fs::rename(&path, &original).unwrap();
        std::fs::write(&path, "foreign journal\n").unwrap();
        controller.poll(&mut pool);
        let statuses = controller.listing(&pool, device.id(), &project);
        assert_eq!(statuses[0]["state"], "unknown_outcome");
        assert_eq!(statuses[0]["runtime_active"], true);
        assert!(
            controller
                .listing(&pool, foreign.id(), &project)
                .as_array()
                .unwrap()
                .is_empty()
        );
        std::fs::remove_file(&path).unwrap();
        std::fs::rename(&original, &path).unwrap();
        let cancel = request(
            &pool,
            "cancel-ambiguous-runtime",
            Operation::Cancel,
            json!({"request_id":"ambiguous-runtime"}),
        );
        assert_eq!(
            controller
                .cancel(&mut pool, &registry, &foreign, &cancel)
                .unwrap()
                .code,
            Some(ErrorCode::OwnerMismatch)
        );
        let receipt = controller
            .cancel(&mut pool, &registry, &device, &cancel)
            .unwrap();
        assert_eq!(
            receipt.execution_state,
            Some(super::super::protocol::ExecutionState::Accepted)
        );
        assert_eq!(
            controller
                .cancel(&mut pool, &registry, &device, &cancel)
                .unwrap(),
            receipt
        );
        while controller.busy(&project) {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            controller.dispatch_prompt(&mut pool, &registry, &device, &prompt, false),
            admitted
        );
        let restored = controller.listing(&pool, device.id(), &project);
        assert_eq!(restored[0]["state"], "unknown_outcome");
        assert!(restored[0].get("runtime_active").is_none());
        assert!(
            controller
                .cancel(&mut pool, &registry, &device, &cancel)
                .is_none()
        );
        assert!(pool.worker_handles(&project).is_empty());
        let session = pool.active_context().session_id.clone();
        master.lock().unwrap().session_mut().append_event(json!({"type":"remote_worker_progress_v1","device_id":device.id(),
            "request_id":"stale-forged-live","project_id":project,"session_id":session,"subtab_id":tabs.active_id,
            "state":"unknown_outcome","runtime_active":true})).unwrap();
        let stale = controller.listing(&pool, device.id(), &project);
        assert!(
            stale
                .as_array()
                .unwrap()
                .iter()
                .all(|status| status.get("runtime_active").is_none()),
            "durable flags cannot prove runtime ownership"
        );
        release.store(true, Ordering::Release);
    }

    #[test]
    fn exact_slot_reads_ignore_oversized_sibling_journals_and_refuse_wrong_runtime_identity() {
        use crate::backend::{
            Backend, BackendError, Completion, CompletionRequest, ProviderEvent, RequestControl,
        };
        struct Held(Arc<AtomicBool>, Arc<AtomicBool>);
        impl Backend for Held {
            fn complete(&self, _: CompletionRequest<'_>) -> Result<Completion, BackendError> {
                Ok(Completion::text("held answer"))
            }
            fn complete_with_request_control(
                &self,
                request: CompletionRequest<'_>,
                control: &mut RequestControl<'_>,
                sink: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
            ) -> Result<Completion, BackendError> {
                self.1.store(true, Ordering::Release);
                let deadline = Instant::now() + Duration::from_secs(10);
                while !self.0.load(Ordering::Acquire) {
                    control.check_cancelled()?;
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(1));
                }
                self.complete_with_control(request, &|| control.check_cancelled().is_err(), sink)
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let state = pool.subtab_state(&project).unwrap();
        pool.commit_subtabs(
            &project,
            state.revision,
            state.set_concurrency(&state.active_id, 2).unwrap(),
        )
        .unwrap();
        let state = pool.subtab_state(&project).unwrap();
        let worker = pool.worker_owner(&project, &state.active_id, 0).unwrap();
        let sibling_path = worker.agent().lock().unwrap().session().path().to_owned();
        let release = Arc::new(AtomicBool::new(false));
        let entered = Arc::new(AtomicBool::new(false));
        *worker.agent().lock().unwrap() = Agent::new(
            SessionStore::open_existing_writable(&sibling_path).unwrap(),
            Box::new(Held(release.clone(), entered.clone())),
        );
        worker.agent().lock().unwrap().set_tools(
            crate::tools::ToolRegistry::with_all_builtins().unwrap(),
            crate::tools::ToolContext::new(&root).unwrap(),
            crate::tools::SideEffectPolicy::read_only(),
        );
        drop(worker);
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let foreign = phone(&registry, &project);
        let mut controller = WorktreeHost::default();
        for (index, phone) in [(&foreign, "held-sibling"), (&device, "slot-one-output")]
            .into_iter()
            .enumerate()
        {
            let (phone, id) = phone;
            let prompt = request(
                &pool,
                id,
                Operation::Prompt,
                json!({"text":id,"subtab_id":state.active_id,"subtab_revision":state.revision}),
            );
            assert_eq!(
                controller
                    .dispatch_prompt(&mut pool, &registry, phone, &prompt, false)
                    .execution_state,
                Some(super::super::protocol::ExecutionState::Accepted)
            );
            if index == 0 {
                let deadline = Instant::now() + Duration::from_secs(3);
                while !entered.load(Ordering::Acquire) {
                    controller.poll(&mut pool);
                    assert!(
                        Instant::now() < deadline,
                        "{}",
                        controller.listing(&pool, foreign.id(), &project)
                    );
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            controller.poll(&mut pool);
            let values = controller.listing(&pool, device.id(), &project);
            if values
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value["state"] == "succeeded")
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        release.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_secs(3);
        while controller.busy(&project) {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        let sibling_bytes = std::fs::read(&sibling_path).unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&sibling_path)
            .unwrap()
            .set_len(8 * 1024 * 1024 + 1)
            .unwrap();
        let mut read = request(
            &pool,
            "exact-slot-read",
            Operation::TranscriptRead,
            json!({"request_id":"slot-one-output","view":"history","offset":0,"digest":null}),
        );
        read.required_owner = None;
        read.expected_revision = None;
        let page = super::super::transcript::read(&pool, &registry, &device, &read).unwrap();
        assert_eq!(page["total_turns"], 2);
        let projected = controller.listing(&pool, device.id(), &project);
        for field in ["worker_slot", "worker_session_id", "history_digest"] {
            assert!(projected[0].get(field).is_none());
        }
        assert!(page.to_string().contains("slot-one-output"));
        assert!(!page.to_string().contains("held-sibling"));
        let original = master
            .lock()
            .unwrap()
            .session()
            .events()
            .iter()
            .rev()
            .find(|event| {
                event["request_id"] == "slot-one-output"
                    && event["type"] == "remote_worker_progress_v1"
            })
            .unwrap()
            .clone();
        assert_eq!(original["worker_slot"], 1);
        for value in [json!(64), json!(1.0), json!("1"), Value::Null] {
            let mut wrong = original.clone();
            wrong["worker_slot"] = value;
            master
                .lock()
                .unwrap()
                .session_mut()
                .append_event(wrong)
                .unwrap();
            assert_eq!(
                super::super::transcript::read(&pool, &registry, &device, &read),
                Err(ErrorCode::InvalidRequest)
            );
        }
        let mut wrong = original.clone();
        wrong["worker_session_id"] = json!("different-worker-session");
        master
            .lock()
            .unwrap()
            .session_mut()
            .append_event(wrong)
            .unwrap();
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &read),
            Err(ErrorCode::InvalidRequest)
        );
        master
            .lock()
            .unwrap()
            .session_mut()
            .append_event(original)
            .unwrap();
        drop(pool);
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &read).unwrap(),
            page
        );
        std::fs::write(&sibling_path, sibling_bytes).unwrap();
        assert!(pool.worker_handles(&project).is_empty());
    }

    #[test]
    #[ignore = "requires current Android JVM classes; executed by source checker"]
    fn android_reads_production_multi_page_tool_loop() {
        use crate::backend::{
            Backend, BackendError, Completion, CompletionRequest, ProviderEvent, RequestControl,
        };
        use crate::tools::{SideEffectPolicy, ToolCall, ToolContext, ToolRegistry};
        struct Scripted;
        impl Backend for Scripted {
            fn complete(&self, request: CompletionRequest<'_>) -> Result<Completion, BackendError> {
                let count = request
                    .turns
                    .iter()
                    .filter(|turn| turn.role == crate::core::TurnRole::Tool)
                    .count();
                if count == 9 {
                    return Ok(Completion::text("Final tool-backed answer"));
                }
                let mut completion = Completion::text(format!("Reading file {count}"));
                completion.tool_calls.push(ToolCall {
                    id: format!("read-{count}"),
                    name: "read_file".into(),
                    arguments: json!({"path":format!("{}evidence-{count}.txt", "./".repeat(300))}),
                });
                Ok(completion)
            }
            fn complete_with_request_control(
                &self,
                request: CompletionRequest<'_>,
                control: &mut RequestControl<'_>,
                sink: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
            ) -> Result<Completion, BackendError> {
                control.check_cancelled()?;
                self.complete_with_control(request, &|| control.check_cancelled().is_err(), sink)
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        for index in 0..9 {
            std::fs::write(
                root.join(format!("evidence-{index}.txt")),
                "Tool content\nAPI_KEY=PRIVATE_TOOL_LOOP_SECRET",
            )
            .unwrap();
        }
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let tabs = pool.subtab_state(&project).unwrap();
        let worker = pool.worker_owner(&project, &tabs.active_id, 0).unwrap();
        let path = worker.agent().lock().unwrap().session().path().to_owned();
        let mut agent = Agent::new(
            SessionStore::open_existing_writable(&path).unwrap(),
            Box::new(Scripted),
        );
        agent.set_tools(
            ToolRegistry::with_all_builtins().unwrap(),
            ToolContext::new(&root).unwrap(),
            SideEffectPolicy::read_only(),
        );
        *worker.agent().lock().unwrap() = agent;
        drop(worker);
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let foreign = phone(&registry, &project);
        let prompt = request(
            &pool,
            "tool-loop",
            Operation::Prompt,
            json!({"text":"Read evidence.txt nine times", "subtab_id":tabs.active_id,"subtab_revision":tabs.revision}),
        );
        let mut controller = WorktreeHost::default();
        let accepted = controller.dispatch_prompt(&mut pool, &registry, &device, &prompt, false);
        assert_eq!(
            accepted.execution_state,
            Some(super::super::protocol::ExecutionState::Accepted)
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while controller.busy(&project) {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
        let journal = std::fs::read(&path).unwrap();
        let master_journal = std::fs::read(master.lock().unwrap().session().path()).unwrap();
        let mut listing_request = request(&pool, "listing", Operation::WorkspaceList, json!({}));
        listing_request.required_owner = None;
        listing_request.expected_revision = None;
        let listing = super::super::host_adapter::HostAdapter::dispatch_owned(
            &mut pool,
            &registry,
            &device,
            listing_request,
        );
        let Message::Response { mut result } = listing.payload else {
            panic!("listing")
        };
        result["projects"][0]["worker_statuses"] = controller.listing(&pool, device.id(), &project);
        let mut snapshot = request(&pool, "snapshot", Operation::Inspect, json!({}));
        snapshot.required_owner = None;
        snapshot.expected_revision = None;
        let snapshot = super::super::host_adapter::HostAdapter::dispatch_owned(
            &mut pool, &registry, &device, snapshot,
        );
        let Message::Response { result: snapshot } = snapshot.payload else {
            panic!("snapshot")
        };
        let java = std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap();
        let cp = std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap();
        let seed_path = root.join("seed.json");
        let command_path = root.join("command.frame");
        let result_path = root.join("result.json");
        let run_java = |phase: &str| {
            let mut child = std::process::Command::new(&java)
                .args([
                    format!("-Dzenpi.workertool.phase={phase}"),
                    format!("-Dzenpi.workertool.seed={}", seed_path.display()),
                    format!("-Dzenpi.workertool.command={}", command_path.display()),
                    format!("-Dzenpi.workertool.results={}", result_path.display()),
                    "-cp".into(),
                    cp.clone(),
                    "org.junit.runner.JUnitCore".into(),
                    "zenpi.AndroidRustWorkerToolTest".into(),
                ])
                .spawn()
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(15);
            loop {
                if let Some(status) = child.try_wait().unwrap() {
                    assert!(status.success(), "Android {phase}");
                    break;
                }
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("Android deadline");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        let mut histories = Vec::<Value>::new();
        let mut evidence = Vec::new();
        for mode in ["history", "history", "arguments"] {
            let arguments = serde_json::to_string(
                &json!({"path":format!("{}evidence-0.txt","./".repeat(300))}),
            )
            .unwrap();
            std::fs::write(&seed_path,serde_json::to_vec(&json!({"mode":mode,"listing":result,"snapshot":snapshot,"histories":histories,"arguments":arguments})).unwrap()).unwrap();
            run_java("generate");
            let bytes = std::fs::read(&command_path).unwrap();
            let mut input = bytes.as_slice();
            let command = super::super::frame::read_frame(&mut input)
                .unwrap()
                .unwrap();
            assert!(input.is_empty());
            let value =
                super::super::transcript::read(&pool, &registry, &device, &command).unwrap();
            assert!(!value.to_string().contains("PRIVATE_TOOL_LOOP_SECRET"));
            if mode == "history" {
                assert_eq!(value["total_turns"], 20);
                for turn in value["turns"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|turn| turn["role"] == "tool")
                {
                    assert!(
                        turn["content"].as_str().unwrap().contains("Tool content"),
                        "real read_file did not succeed: {}",
                        turn["content"]
                    );
                }
            }
            let mut exchanges = Vec::new();
            let mut retain = |sent: &Envelope, receipt: &Envelope| {
                use base64::engine::general_purpose::STANDARD;
                let mut wire = Vec::new();
                super::super::frame::write_frame(&mut wire, receipt).unwrap();
                exchanges.push(
                    json!({"request":sent,"code":receipt.code,"wire_base64":STANDARD.encode(wire)}),
                );
            };
            let mut response = Envelope::new(
                super::super::protocol::Channel::Control,
                Message::Response {
                    result: value.clone(),
                },
            );
            response.id = command.id.clone();
            response.execution_state = Some(super::super::protocol::ExecutionState::Succeeded);
            retain(&command, &response);
            let code =
                super::super::transcript::read(&pool, &registry, &foreign, &command).unwrap_err();
            assert_eq!(code, ErrorCode::InvalidRequest);
            retain(&command, &HostAdapter::failure(&command, code));
            if mode == "arguments" {
                let mut changed = command.clone();
                if let Message::Request { arguments, .. } = &mut changed.payload {
                    arguments["tool_call_id"] = json!("foreign");
                }
                let code = super::super::transcript::read(&pool, &registry, &device, &changed)
                    .unwrap_err();
                assert_eq!(code, ErrorCode::InvalidRequest);
                retain(&changed, &HostAdapter::failure(&changed, code));
            }
            if mode == "history" && !histories.is_empty() {
                let mut stale = command.clone();
                if let Message::Request { arguments, .. } = &mut stale.payload {
                    arguments["digest"] = json!("0".repeat(64));
                }
                let code =
                    super::super::transcript::read(&pool, &registry, &device, &stale).unwrap_err();
                assert_eq!(code, ErrorCode::RevisionConflict);
                retain(&stale, &HostAdapter::failure(&stale, code));
            }
            std::fs::write(
                &result_path,
                serde_json::to_vec(&json!({"exchanges":exchanges})).unwrap(),
            )
            .unwrap();
            run_java("validate");
            evidence.push(json!({"mode":mode,"exchanges":exchanges}));
            if mode == "history" {
                histories.push(value);
            }
            assert_eq!(std::fs::read(&path).unwrap(), journal);
            assert_eq!(
                std::fs::read(master.lock().unwrap().session().path()).unwrap(),
                master_journal
            );
        }
        controller.shutdown(&pool, Instant::now() + Duration::from_secs(2));
        assert!(master.lock().unwrap().session().turns().is_empty());
        if let Ok(path) = std::env::var("ZENPI_ANDROID_WORKER_TOOL_OUTPUT") {
            std::fs::write(path,serde_json::to_vec_pretty(&json!({"fixture_only":true,"steps":evidence,"real_read_file_calls":9,"worker_journal_unchanged":true})).unwrap()).unwrap();
        }
    }

    #[test]
    fn production_worker_history_contains_real_tool_loop_without_other_task_messages() {
        use crate::backend::{
            Backend, BackendError, Completion, CompletionRequest, ProviderEvent, RequestControl,
        };
        use crate::tools::{SideEffectPolicy, ToolCall, ToolContext, ToolRegistry};
        struct Scripted;
        impl Backend for Scripted {
            fn complete(&self, request: CompletionRequest<'_>) -> Result<Completion, BackendError> {
                if request
                    .turns
                    .iter()
                    .any(|turn| turn.role == crate::core::TurnRole::Tool)
                {
                    Ok(Completion::text("Final tool-backed answer"))
                } else {
                    let mut completion = Completion::text("Reading the task file");
                    completion.tool_calls.push(ToolCall {
                        id: "read-task-file".into(),
                        name: "read_file".into(),
                        arguments: json!({"path":"evidence.txt"}),
                    });
                    Ok(completion)
                }
            }
            fn complete_with_request_control(
                &self,
                request: CompletionRequest<'_>,
                control: &mut RequestControl<'_>,
                sink: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
            ) -> Result<Completion, BackendError> {
                control.check_cancelled()?;
                self.complete_with_control(request, &|| control.check_cancelled().is_err(), sink)
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::write(
            root.join("evidence.txt"),
            "Tool content\nAPI_KEY=PRIVATE_TOOL_HISTORY",
        )
        .unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let state = pool.subtab_state(&project).unwrap();
        let worker = pool.worker_owner(&project, &state.active_id, 0).unwrap();
        let path = worker.agent().lock().unwrap().session().path().to_owned();
        let mut agent = Agent::new(
            SessionStore::open_existing_writable(&path).unwrap(),
            Box::new(Scripted),
        );
        agent.set_tools(
            ToolRegistry::with_all_builtins().unwrap(),
            ToolContext::new(&root).unwrap(),
            SideEffectPolicy::read_only(),
        );
        *worker.agent().lock().unwrap() = agent;
        drop(worker);
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let prompt = request(
            &pool,
            "tool-history-task",
            Operation::Prompt,
            json!({"text":"Read evidence.txt","subtab_id":state.active_id,"subtab_revision":state.revision}),
        );
        let mut controller = WorktreeHost::default();
        assert_eq!(
            controller
                .dispatch_prompt(&mut pool, &registry, &device, &prompt, false)
                .execution_state,
            Some(super::super::protocol::ExecutionState::Accepted)
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while controller.busy(&project) {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        let mut read = request(
            &pool,
            "read-tool-history",
            Operation::TranscriptRead,
            json!({"request_id":"tool-history-task","view":"history","offset":0,"digest":null}),
        );
        read.required_owner = None;
        read.expected_revision = None;
        let page = super::super::transcript::read(&pool, &registry, &device, &read).unwrap();
        let turns = page["turns"].as_array().unwrap();
        assert_eq!(
            turns
                .iter()
                .map(|turn| turn["role"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["user", "assistant", "tool", "assistant"]
        );
        assert!(
            turns[2]["content"]
                .as_str()
                .unwrap()
                .contains("Tool content")
        );
        assert!(!page.to_string().contains("PRIVATE_TOOL_HISTORY"));
        assert_eq!(turns[1]["tool_calls"][0]["name"], "read_file");
        assert_eq!(turns[1]["tool_calls"][0]["id"], "read-task-file");
        assert!(
            turns[1]["tool_calls"][0]["arguments_preview"]
                .as_str()
                .unwrap()
                .contains("evidence.txt")
        );
        for field in [
            "execution",
            "policy_digest",
            "worker_binding",
            "metadata_digest",
        ] {
            assert!(!page.to_string().contains(field));
        }
        assert_eq!(turns[3]["content"], "Final tool-backed answer");
        assert!(pool.worker_handles(&project).is_empty());
        assert!(master.lock().unwrap().session().turns().is_empty());
        let mut arguments_read = read.clone();
        arguments_read.payload = Message::Request {
            operation: Operation::TranscriptRead,
            arguments: json!({"request_id":"tool-history-task","turn_id":turns[1]["id"],"tool_call_id":"read-task-file","offset":0,"digest":null}),
        };
        let arguments_page =
            super::super::transcript::read(&pool, &registry, &device, &arguments_read).unwrap();
        assert_eq!(arguments_page["text"], "{\"path\":\"evidence.txt\"}");
        assert_eq!(arguments_page["tool_name"], "read_file");
        let foreign = phone(&registry, &project);
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &foreign, &arguments_read),
            Err(ErrorCode::InvalidRequest)
        );
        for change in [
            json!({"tool_call_id":"foreign"}),
            json!({"turn_id":turns[0]["id"]}),
            json!({"turn_id":turns[3]["id"]}),
            json!({"view":"history"}),
            json!({"extra":true}),
        ] {
            let mut invalid = arguments_read.clone();
            if let Message::Request { arguments, .. } = &mut invalid.payload {
                arguments
                    .as_object_mut()
                    .unwrap()
                    .extend(change.as_object().unwrap().clone());
            }
            assert_eq!(
                super::super::transcript::read(&pool, &registry, &device, &invalid),
                Err(ErrorCode::InvalidRequest)
            );
        }
        let original = std::fs::read(&path).unwrap();
        let mut records: Vec<Value> = original
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect();
        let call_id = turns[1]["id"].as_str().unwrap();
        records
            .iter_mut()
            .find(|record| record["turn"]["id"] == call_id)
            .unwrap()["turn"]["metadata"]["tool_calls"][0]["arguments"]["path"] =
            json!("FOREIGN_TOOL_CALL");
        let mut changed = Vec::new();
        for record in &records {
            serde_json::to_writer(&mut changed, &record).unwrap();
            changed.push(b'\n');
        }
        std::fs::write(&path, changed).unwrap();
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &arguments_read),
            Err(ErrorCode::InvalidRequest)
        );
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &read),
            Err(ErrorCode::InvalidRequest)
        );
        let mut selected = read.clone();
        selected.payload = Message::Request {
            operation: Operation::TranscriptRead,
            arguments: json!({"request_id":"tool-history-task","turn_id":call_id,"offset":0,"digest":null}),
        };
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &selected),
            Err(ErrorCode::InvalidRequest)
        );
        // Rehashing the altered worker manifest cannot change the master's evidence.
        use sha2::{Digest, Sha256};
        let replacement = records
            .iter()
            .find(|record| record["turn"]["id"] == call_id)
            .unwrap()["turn"]["metadata"]
            .clone();
        let binding = records
            .iter_mut()
            .find(|record| record["event"]["type"] == "remote_worker_output_v1")
            .unwrap();
        binding["event"]["history"][1]["metadata_digest"] = json!(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&replacement).unwrap())
        ));
        binding["event"]["history_digest"] = json!(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&binding["event"]["history"]).unwrap())
        ));
        let mut changed = Vec::new();
        for record in &records {
            serde_json::to_writer(&mut changed, record).unwrap();
            changed.push(b'\n');
        }
        std::fs::write(&path, changed).unwrap();
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &read),
            Err(ErrorCode::InvalidRequest)
        );
        std::fs::write(&path, original).unwrap();
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &read).unwrap(),
            page
        );
        let worker_before = std::fs::read(&path).unwrap();
        let master_path = master.lock().unwrap().session().path().to_owned();
        let master_before = std::fs::read(&master_path).unwrap();
        drop(controller);
        drop(pool);
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &arguments_read).unwrap(),
            arguments_page
        );
        assert_eq!(std::fs::read(&path).unwrap(), worker_before);
        assert_eq!(std::fs::read(&master_path).unwrap(), master_before);
        assert!(pool.worker_handles(&project).is_empty());
        registry.revoke(device.id()).unwrap();
        assert!(
            super::super::transcript::read(&pool, &registry, &device, &arguments_read).is_err()
        );
    }

    #[test]
    fn worker_output_pages_remain_device_scoped_readonly_after_retirement_and_restart() {
        use super::super::protocol::ExecutionState;
        use sha2::Digest;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let state = pool.subtab_state(&project).unwrap();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let foreign = phone(&registry, &project);
        let raw = format!(
            "API_KEY=PRIVATE_PAGED_WORKER\n{}",
            "\u{4e2d} answer\n".repeat(1800)
        );
        let prompt = request(
            &pool,
            "paged-worker",
            Operation::Prompt,
            json!({"text":raw,"subtab_id":state.active_id,"subtab_revision":state.revision}),
        );
        let mut controller = WorktreeHost::default();
        assert_eq!(
            controller
                .dispatch_prompt(&mut pool, &registry, &device, &prompt, false)
                .execution_state,
            Some(ExecutionState::Accepted)
        );
        let deadline = Instant::now() + Duration::from_secs(3);
        while controller.busy(&project) {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(pool.worker_handles(&project).is_empty());
        let another = request(
            &pool,
            "other-device-task",
            Operation::Prompt,
            json!({"text":"OTHER_DEVICE_HISTORY_ONLY","subtab_id":state.active_id,"subtab_revision":state.revision}),
        );
        assert_eq!(
            controller
                .dispatch_prompt(&mut pool, &registry, &foreign, &another, false)
                .execution_state,
            Some(ExecutionState::Accepted)
        );
        let deadline = Instant::now() + Duration::from_secs(3);
        while controller.busy(&project) {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        let status = super::super::worker_progress::listing(&pool, device.id(), &project);
        assert_eq!(status[0]["state"], "succeeded");
        assert_eq!(status[0]["output_truncated"], true);
        assert!(
            status[0].get("output_turn_id").is_none(),
            "internal correlation is not a status field"
        );
        let mut read = request(
            &pool,
            "read-worker",
            Operation::TranscriptRead,
            json!({"request_id":"paged-worker","offset":0,"digest":null}),
        );
        read.required_owner = None;
        read.expected_revision = None;
        let mut history = read.clone();
        history.payload = Message::Request {
            operation: Operation::TranscriptRead,
            arguments: json!({"request_id":"paged-worker","view":"history","offset":0,"digest":null}),
        };
        let list = super::super::transcript::read(&pool, &registry, &device, &history).unwrap();
        assert_eq!(list["turns"].as_array().unwrap().len(), 2);
        assert_eq!(list["turns"][0]["role"], "user");
        assert_eq!(list["turns"][1]["role"], "assistant");
        assert!(!list.to_string().contains("PRIVATE_PAGED_WORKER"));
        assert!(!list.to_string().contains("OTHER_DEVICE_HISTORY_ONLY"));
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &foreign, &history),
            Err(ErrorCode::InvalidRequest)
        );
        let user_id = list["turns"][0]["id"].as_str().unwrap();
        let mut user_read = read.clone();
        user_read.payload = Message::Request {
            operation: Operation::TranscriptRead,
            arguments: json!({"request_id":"paged-worker","turn_id":user_id,"offset":0,"digest":null}),
        };
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &user_read).unwrap()["role"],
            "user"
        );
        if let Message::Request { arguments, .. } = &mut user_read.payload {
            arguments["turn_id"] = json!("foreign-turn");
        }
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &user_read),
            Err(ErrorCode::InvalidRequest)
        );
        let first = super::super::transcript::read(&pool, &registry, &device, &read).unwrap();
        assert_eq!(first["request_id"], "paged-worker");
        assert_eq!(first["subtab_id"], state.active_id);
        assert!(first["has_more"].as_bool().unwrap());
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &foreign, &read),
            Err(ErrorCode::InvalidRequest)
        );
        let journal_path = std::fs::read_dir(root.join(".zenpi/workers"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| path.extension().is_some_and(|value| value == "jsonl"))
            .unwrap();
        let original = std::fs::read(&journal_path).unwrap();
        let records: Vec<Value> = original
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect();
        for field in [
            "device_id",
            "request_id",
            "project_id",
            "session_id",
            "subtab_id",
            "worker_session_id",
            "worker_key",
            "worker_slot",
            "request_fingerprint",
            "output_digest",
            "type",
        ] {
            let mut changed = records.clone();
            let binding = changed
                .iter_mut()
                .find(|record| record["event"]["type"] == "remote_worker_output_v1")
                .unwrap();
            binding["event"][field] = json!("foreign-binding");
            let mut bytes = Vec::new();
            for record in changed {
                serde_json::to_writer(&mut bytes, &record).unwrap();
                bytes.push(b'\n');
            }
            std::fs::write(&journal_path, bytes).unwrap();
            assert_eq!(
                super::super::transcript::read(&pool, &registry, &device, &read),
                Err(ErrorCode::InvalidRequest),
                "{field}"
            );
            std::fs::write(&journal_path, &original).unwrap();
        }
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &read).unwrap(),
            first
        );
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &history).unwrap(),
            list
        );
        let mut changed = records.clone();
        let output_id = first["turn_id"].as_str().unwrap();
        let output = changed
            .iter_mut()
            .find(|record| record["turn"]["id"] == output_id)
            .unwrap();
        output["turn"]["content"] = json!("unrelated replacement answer");
        let mut bytes = Vec::new();
        for record in changed {
            serde_json::to_writer(&mut bytes, &record).unwrap();
            bytes.push(b'\n');
        }
        std::fs::write(&journal_path, bytes).unwrap();
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &read),
            Err(ErrorCode::InvalidRequest)
        );
        let mut changed = records.clone();
        changed
            .iter_mut()
            .find(|record| record["turn"]["id"] == output_id)
            .unwrap()["turn"]["content"] = json!("unrelated replacement answer");
        changed
            .iter_mut()
            .find(|record| record["event"]["type"] == "remote_worker_output_v1")
            .unwrap()["event"]["output_digest"] = json!(format!(
            "{:x}",
            sha2::Sha256::digest(b"unrelated replacement answer")
        ));
        let mut bytes = Vec::new();
        for record in changed {
            serde_json::to_writer(&mut bytes, &record).unwrap();
            bytes.push(b'\n');
        }
        std::fs::write(&journal_path, bytes).unwrap();
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &read),
            Err(ErrorCode::InvalidRequest)
        );
        std::fs::write(&journal_path, &original).unwrap();
        {
            let mut journal = SessionStore::open_existing_writable(&journal_path).unwrap();
            let binding = journal
                .events()
                .iter()
                .find(|event| event["type"] == "remote_worker_output_v1")
                .unwrap()
                .clone();
            journal.append_event(binding).unwrap();
        }
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &read),
            Err(ErrorCode::InvalidRequest)
        );
        std::fs::write(&journal_path, &original).unwrap();
        let mut changed = records.clone();
        changed
            .iter_mut()
            .find(|record| record["turn"]["id"] == user_id)
            .unwrap()["turn"]["content"] = json!("REPLACED_INPUT_NOT_THIS_TASK");
        let mut bytes = Vec::new();
        for record in changed {
            serde_json::to_writer(&mut bytes, &record).unwrap();
            bytes.push(b'\n');
        }
        std::fs::write(&journal_path, bytes).unwrap();
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &history),
            Err(ErrorCode::InvalidRequest)
        );
        std::fs::write(&journal_path, &original).unwrap();
        let mut changed = records.clone();
        let binding = changed
            .iter_mut()
            .find(|record| record["event"]["type"] == "remote_worker_output_v1")
            .unwrap();
        binding["event"]["history"][0]["digest"] = json!("0".repeat(64));
        let forged_digest = format!(
            "{:x}",
            sha2::Sha256::digest(serde_json::to_vec(&binding["event"]["history"]).unwrap())
        );
        binding["event"]["history_digest"] = json!(forged_digest);
        let mut bytes = Vec::new();
        for record in changed {
            serde_json::to_writer(&mut bytes, &record).unwrap();
            bytes.push(b'\n');
        }
        std::fs::write(&journal_path, bytes).unwrap();
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &history),
            Err(ErrorCode::InvalidRequest)
        );
        std::fs::write(&journal_path, &original).unwrap();
        let before: BTreeMap<_, _> = std::fs::read_dir(root.join(".zenpi/workers"))
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let bytes = std::fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect();
        drop(pool);
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &history).unwrap(),
            list
        );
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &read).unwrap(),
            first
        );
        let master_before = std::fs::read(master.lock().unwrap().session().path()).unwrap();
        let checkpoint_before = std::fs::read(root.join("project-workspace.json")).unwrap();
        let routed = HostAdapter::dispatch_owned(&mut pool, &registry, &device, read.clone());
        assert!(matches!(routed.payload, Message::Response { result } if result == first));
        let mut joined = first["text"].as_str().unwrap().to_owned();
        let mut page = first;
        while page["has_more"] == true {
            read.payload = Message::Request {
                operation: Operation::TranscriptRead,
                arguments: json!({"request_id":"paged-worker","offset":page["next_offset"],"digest":page["digest"]}),
            };
            page = super::super::transcript::read(&pool, &registry, &device, &read).unwrap();
            assert!(
                !page["text"]
                    .as_str()
                    .unwrap()
                    .contains("PRIVATE_PAGED_WORKER")
            );
            joined.push_str(page["text"].as_str().unwrap());
        }
        assert_eq!(joined, crate::security::redact_text(&raw, &[]));
        if let Message::Request { arguments, .. } = &mut read.payload {
            arguments["digest"] = json!("0".repeat(64));
        }
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &read),
            Err(ErrorCode::RevisionConflict)
        );
        for (path, bytes) in before {
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
        assert!(pool.worker_handles(&project).is_empty());
        assert_eq!(
            std::fs::read(master.lock().unwrap().session().path()).unwrap(),
            master_before
        );
        assert_eq!(
            std::fs::read(root.join("project-workspace.json")).unwrap(),
            checkpoint_before
        );
        registry.revoke(device.id()).unwrap();
        assert_eq!(
            super::super::transcript::read(&pool, &registry, &device, &read),
            Err(ErrorCode::Unauthorized)
        );
        assert!(master.lock().unwrap().session().turns().is_empty());
    }

    #[test]
    fn live_growth_at_global_capacity_refuses_before_reservation_then_fits_after_retirement() {
        use super::super::protocol::ExecutionState;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let other_root = root.join("other-project");
        std::fs::create_dir(&other_root).unwrap();
        let crate::project_workspace::OpenOutcome::Opened(other_project) =
            pool.open(Some(&other_root)).unwrap()
        else {
            panic!("expected another project");
        };
        let other = pool.subtab_state(other_project.as_str()).unwrap();
        pool.commit_subtabs(
            other_project.as_str(),
            other.revision,
            other.set_concurrency(&other.active_id, 63).unwrap(),
        )
        .unwrap();
        for slot in 0..63 {
            pool.worker_owner(other_project.as_str(), &other.active_id, slot)
                .unwrap();
        }
        pool.select(&project).unwrap();
        let state = pool.subtab_state(&project).unwrap();
        let runner = Runner::spawn(
            &mut pool,
            &project,
            &state.active_id,
            RuntimeConfig::default(),
            |worker, job: WorkerPrompt, token| {
                worker
                    .agent()
                    .lock()
                    .unwrap()
                    .process_with_cancel(crate::core::TurnInputRequest::new(job.text), || {
                        token.is_cancelled()
                    })
            },
        )
        .unwrap();
        let mut controller = WorktreeHost::default();
        let key = (project.clone(), state.active_id.clone());
        controller.lanes.insert(
            key.clone(),
            Lane {
                runner,
                progress: WorkerProgress::default(),
                live: Arc::new(super::super::live_text::LivePromptText::default()),
            },
        );
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let quota = request(
            &pool,
            "global-capacity-growth",
            Operation::SetConcurrency,
            json!({"subtab_id":state.active_id,"subtab_revision":state.revision,"concurrency":2}),
        );
        let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
        let journals = std::fs::read_dir(root.join(".zenpi/workers"))
            .unwrap()
            .count();
        assert_eq!(
            controller
                .dispatch_quota(&mut pool, &registry, &device, &quota, false)
                .unwrap()
                .code,
            Some(ErrorCode::ResourceLimit)
        );
        assert!(!pool.quota_intent_pending());
        assert_eq!(pool.worker_handles(&project).len(), 1);
        assert_eq!(pool.worker_handles(other_project.as_str()).len(), 63);
        assert_eq!(
            std::fs::read(root.join("project-workspace.json")).unwrap(),
            checkpoint
        );
        assert_eq!(
            std::fs::read_dir(root.join(".zenpi/workers"))
                .unwrap()
                .count(),
            journals
        );
        assert_eq!(controller.lanes[&key].runner.applied_concurrency(), 1);
        assert!(!controller.lanes[&key].runner.quota_change_pending());
        pool.retire_worker_owners(other_project.as_str()).unwrap();
        for slot in 0..62 {
            pool.worker_owner(other_project.as_str(), &other.active_id, slot)
                .unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let result = controller
                .dispatch_quota(&mut pool, &registry, &device, &quota, false)
                .unwrap();
            if result.execution_state == Some(ExecutionState::Succeeded) {
                break;
            }
            assert_eq!(result.execution_state, Some(ExecutionState::UnknownOutcome));
            assert!(Instant::now() < deadline, "{result:?}");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(controller.lanes[&key].runner.quota(), 2);
        assert_eq!(pool.worker_handles(other_project.as_str()).len(), 62);
        assert_eq!(pool.worker_handles(&project).len(), 2);
        assert_eq!(controller.lanes[&key].runner.applied_concurrency(), 2);
        assert_eq!(pool.subtab_state(&project).unwrap().tabs[0].concurrency, 2);
        assert!(!pool.quota_intent_pending());
        controller.shutdown(&pool, Instant::now() + Duration::from_secs(1));
        pool.retire_worker_owners(&project).unwrap();
        pool.retire_worker_owners(other_project.as_str()).unwrap();
        assert!(master.lock().unwrap().session().turns().is_empty());
    }

    #[test]
    fn remote_live_quota_retries_settle_once_and_new_prompts_use_committed_scope() {
        use super::super::protocol::ExecutionState;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
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
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let revoked = phone(&registry, &project);
        let release = Arc::new(AtomicBool::new(false));
        let gates = release.clone();
        let entered = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let starts = entered.clone();
        let live = Arc::new(super::super::live_text::LivePromptText::default());
        let runner = Runner::spawn(
            &mut pool,
            &project,
            &before.active_id,
            RuntimeConfig {
                event_capacity: 1,
                ..RuntimeConfig::default()
            },
            move |worker, job: WorkerPrompt, token| {
                let mut agent = worker.agent().lock().unwrap();
                starts.fetch_add(1, Ordering::Release);
                let deadline = Instant::now() + Duration::from_secs(5);
                while !gates.load(Ordering::Acquire) && !token.is_cancelled() {
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(1));
                }
                agent.process_with_cancel(crate::core::TurnInputRequest::new(job.text), || {
                    token.is_cancelled()
                })
            },
        )
        .unwrap();
        let mut controller = WorktreeHost::default();
        let key = (project.clone(), before.active_id.clone());
        controller.lanes.insert(
            key.clone(),
            Lane {
                runner,
                progress: WorkerProgress::default(),
                live,
            },
        );
        let prompt = request(
            &pool,
            "held",
            Operation::Prompt,
            json!({"text":"held-task", "subtab_id":before.active_id,"subtab_revision":before.revision}),
        );
        let accepted = controller.dispatch_prompt(&mut pool, &registry, &device, &prompt, false);
        assert_eq!(accepted.execution_state, Some(ExecutionState::Accepted));
        let deadline = Instant::now() + Duration::from_secs(3);
        while entered.load(Ordering::Acquire) < 1 {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        let quota = request(
            &pool,
            "live-quota",
            Operation::SetConcurrency,
            json!({"subtab_id":before.active_id,"subtab_revision":before.revision,"concurrency":3}),
        );
        let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
        let record = registry.lookup(revoked.id()).unwrap();
        registry
            .set_permissions(
                revoked.id(),
                record.revision,
                vec![super::super::auth::Capability::Read],
                vec![project.clone()],
            )
            .unwrap();
        assert_eq!(
            controller
                .dispatch_quota(&mut pool, &registry, &revoked, &quota, false)
                .unwrap()
                .code,
            Some(ErrorCode::Unauthorized)
        );
        assert_eq!(
            controller
                .dispatch_quota(&mut pool, &registry, &device, &quota, true)
                .unwrap()
                .code,
            Some(ErrorCode::Busy)
        );
        for value in [json!(0), json!(65), json!(1.0), json!("3")] {
            let mut invalid = quota.clone();
            if let Message::Request { arguments, .. } = &mut invalid.payload {
                arguments["concurrency"] = value;
            }
            assert_eq!(
                controller
                    .dispatch_quota(&mut pool, &registry, &device, &invalid, false)
                    .unwrap()
                    .code,
                Some(ErrorCode::InvalidRequest)
            );
        }
        assert_eq!(
            std::fs::read(root.join("project-workspace.json")).unwrap(),
            checkpoint
        );
        let pending = controller
            .dispatch_quota(&mut pool, &registry, &device, &quota, false)
            .unwrap();
        assert_eq!(
            pending.execution_state,
            Some(ExecutionState::UnknownOutcome)
        );
        assert_eq!(pool.subtab_state(&project).unwrap(), before);
        assert_eq!(
            controller
                .dispatch_quota(&mut pool, &registry, &device, &quota, false)
                .unwrap(),
            pending
        );
        let mut other = quota.clone();
        other.id = Some("another-quota".into());
        assert_eq!(
            controller
                .dispatch_quota(&mut pool, &registry, &device, &other, false)
                .unwrap()
                .code,
            Some(ErrorCode::Busy)
        );
        let new_prompt = request(
            &pool,
            "pending-fenced-task",
            Operation::Prompt,
            json!({"text":"no-new-task", "subtab_id":before.active_id,"subtab_revision":before.revision}),
        );
        assert_eq!(
            controller
                .dispatch_prompt(&mut pool, &registry, &device, &new_prompt, false)
                .code,
            Some(ErrorCode::ResourceLimit)
        );
        let mut changed = quota.clone();
        if let Message::Request { arguments, .. } = &mut changed.payload {
            arguments["concurrency"] = json!(2);
        }
        assert_eq!(
            controller
                .dispatch_quota(&mut pool, &registry, &device, &changed, false)
                .unwrap()
                .code,
            Some(ErrorCode::RequestIdConflict)
        );
        controller.poll(&mut pool);
        let deadline = Instant::now() + Duration::from_secs(3);
        let receipt = loop {
            let value = controller
                .dispatch_quota(&mut pool, &registry, &device, &quota, false)
                .unwrap();
            if value.execution_state == Some(ExecutionState::Succeeded) {
                break value;
            }
            assert!(Instant::now() < deadline, "{value:?}");
            controller.poll(&mut pool);
            std::thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(
            controller
                .dispatch_quota(&mut pool, &registry, &device, &quota, false)
                .unwrap(),
            receipt
        );
        assert_eq!(pool.subtab_state(&project).unwrap().tabs[0].concurrency, 3);
        assert_eq!(controller.lanes[&key].runner.configured_quota(), 3);
        assert_eq!(
            controller.dispatch_prompt(&mut pool, &registry, &device, &prompt, false),
            accepted
        );
        let state = pool.subtab_state(&project).unwrap();
        let next = request(
            &pool,
            "after-resize",
            Operation::Prompt,
            json!({"text":"after-resize-task","subtab_id":state.active_id,"subtab_revision":state.revision}),
        );
        assert_eq!(
            controller
                .dispatch_prompt(&mut pool, &registry, &device, &next, false)
                .execution_state,
            Some(ExecutionState::Accepted)
        );
        release.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_secs(3);
        while controller.busy(&project) {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(entered.load(Ordering::Acquire), 2);
        assert!(master.lock().unwrap().session().turns().is_empty());
        assert_eq!(
            HostAdapter::dispatch_owned(&mut pool, &registry, &device, quota.clone()),
            receipt
        );
        registry.revoke(device.id()).unwrap();
        assert_eq!(
            HostAdapter::dispatch_owned(&mut pool, &registry, &device, quota).code,
            Some(ErrorCode::Unauthorized)
        );
    }

    #[test]
    fn revoke_after_runtime_submission_cannot_commit_on_retry_or_background_poll() {
        use super::super::protocol::ExecutionState;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master).unwrap();
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
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let started = Arc::new(AtomicBool::new(false));
        let entered = started.clone();
        let runner = Runner::spawn(
            &mut pool,
            &project,
            &before.active_id,
            RuntimeConfig {
                event_capacity: 1,
                ..RuntimeConfig::default()
            },
            move |_, _: WorkerPrompt, token| {
                entered.store(true, Ordering::Release);
                let deadline = Instant::now() + Duration::from_secs(5);
                while !token.is_cancelled() {
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(AgentError::InvalidTurn("cancelled fixture".into()))
            },
        )
        .unwrap();
        let mut controller = WorktreeHost::default();
        let key = (project.clone(), before.active_id.clone());
        controller.lanes.insert(
            key.clone(),
            Lane {
                runner,
                progress: WorkerProgress::default(),
                live: Arc::new(super::super::live_text::LivePromptText::default()),
            },
        );
        let prompt = request(
            &pool,
            "held",
            Operation::Prompt,
            json!({"text":"held","subtab_id":before.active_id,"subtab_revision":before.revision}),
        );
        controller.dispatch_prompt(&mut pool, &registry, &device, &prompt, false);
        let deadline = Instant::now() + Duration::from_secs(3);
        while !started.load(Ordering::Acquire) {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        let quota = request(
            &pool,
            "revoked-quota",
            Operation::SetConcurrency,
            json!({"subtab_id":before.active_id,"subtab_revision":before.revision,"concurrency":2}),
        );
        assert_eq!(
            controller
                .dispatch_quota(&mut pool, &registry, &device, &quota, false)
                .unwrap()
                .execution_state,
            Some(ExecutionState::UnknownOutcome)
        );
        registry.revoke(device.id()).unwrap();
        controller.poll(&mut pool);
        let deadline = Instant::now() + Duration::from_secs(3);
        while controller.lanes[&key].runner.applied_concurrency() != 2 {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            controller
                .dispatch_quota(&mut pool, &registry, &device, &quota, false)
                .unwrap()
                .code,
            Some(ErrorCode::Unauthorized)
        );
        assert_eq!(pool.subtab_state(&project).unwrap(), before);
        assert_eq!(pool.unresolved_quota_intents().len(), 1);
        assert!(controller.lanes[&key].runner.quota_change_pending());
        let replacement_device = phone(&registry, &project);
        let mut competing = quota.clone();
        competing.id = Some("replacement-device-quota".into());
        let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
        assert_eq!(
            super::super::subtab::mutate(&mut pool, &registry, &replacement_device, &competing),
            Err(ErrorCode::Busy)
        );
        assert_eq!(
            std::fs::read(root.join("project-workspace.json")).unwrap(),
            checkpoint
        );
        controller.shutdown(&pool, Instant::now() + Duration::from_secs(1));
        pool.retire_worker_owners(&project).unwrap();
        pool.reconcile_quota_intent(device.id(), "revoked-quota")
            .unwrap();
        assert_eq!(pool.subtab_state(&project).unwrap().tabs[0].concurrency, 2);
    }

    #[test]
    fn completed_tasks_do_not_retire_an_unconfirmed_quota_and_exact_retry_releases_lane() {
        use super::super::protocol::ExecutionState;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let initial = pool.subtab_state(&project).unwrap();
        pool.commit_subtabs(
            &project,
            initial.revision,
            initial.rename(&initial.active_id, "persisted").unwrap(),
        )
        .unwrap();
        let state = pool.subtab_state(&project).unwrap();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let live = Arc::new(super::super::live_text::LivePromptText::default());
        let runner = Runner::spawn(
            &mut pool,
            &project,
            &state.active_id,
            RuntimeConfig {
                event_capacity: 1,
                ..RuntimeConfig::default()
            },
            |worker, job: WorkerPrompt, token| {
                worker
                    .agent()
                    .lock()
                    .unwrap()
                    .process_with_cancel(crate::core::TurnInputRequest::new(job.text), || {
                        token.is_cancelled()
                    })
            },
        )
        .unwrap();
        let key = (project.clone(), state.active_id.clone());
        let mut controller = WorktreeHost::default();
        controller.lanes.insert(
            key.clone(),
            Lane {
                runner,
                progress: WorkerProgress::default(),
                live,
            },
        );
        let prompt = request(
            &pool,
            "quick-task",
            Operation::Prompt,
            json!({"text":"quick-task-output","subtab_id":state.active_id,"subtab_revision":state.revision}),
        );
        assert_eq!(
            controller
                .dispatch_prompt(&mut pool, &registry, &device, &prompt, false)
                .execution_state,
            Some(ExecutionState::Accepted)
        );
        let quota = request(
            &pool,
            "after-completion-quota",
            Operation::SetConcurrency,
            json!({"subtab_id":state.active_id,"subtab_revision":state.revision,"concurrency":2}),
        );
        assert_eq!(
            controller
                .dispatch_quota(&mut pool, &registry, &device, &quota, false)
                .unwrap()
                .execution_state,
            Some(ExecutionState::UnknownOutcome)
        );
        let deadline = Instant::now() + Duration::from_secs(3);
        while !controller.lanes[&key].progress.finished() {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            controller.busy(&project),
            "finished tasks must not destroy pending application receipt"
        );
        assert_eq!(pool.subtab_state(&project).unwrap(), state);
        assert!(controller.lanes[&key].runner.quota_change_pending());
        assert_eq!(pool.unresolved_quota_intents().len(), 1);
        let receipt = loop {
            let value = controller
                .dispatch_quota(&mut pool, &registry, &device, &quota, false)
                .unwrap();
            if value.execution_state == Some(ExecutionState::Succeeded) {
                break value;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        };
        controller.poll(&mut pool);
        assert!(!controller.busy(&project));
        assert!(pool.worker_handles(&project).is_empty());
        assert!(pool.unresolved_quota_intents().is_empty());
        assert_eq!(
            HostAdapter::dispatch_owned(&mut pool, &registry, &device, quota.clone()),
            receipt
        );
        let values = controller.listing(&pool, device.id(), &project);
        assert!(
            values
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value["request_id"] == "quick-task" && value["state"] == "succeeded")
        );
        let bytes = std::fs::read(root.join("project-workspace.json")).unwrap();
        let owner = pool.active();
        drop(controller);
        drop(pool);
        let mut restored = ProjectOwnerPool::new(owner).unwrap();
        restored.restore_checkpoint().unwrap();
        assert_eq!(
            restored.subtab_state(&project).unwrap().tabs[0].concurrency,
            2
        );
        assert_eq!(
            HostAdapter::dispatch_owned(&mut restored, &registry, &device, quota),
            receipt
        );
        assert_eq!(
            std::fs::read(root.join("project-workspace.json")).unwrap(),
            bytes
        );
        assert!(master.lock().unwrap().session().turns().is_empty());
    }

    #[test]
    fn remote_lowering_preserves_active_writers_and_queued_jobs_follow_new_quota() {
        use super::super::protocol::ExecutionState;
        use std::sync::mpsc;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let initial = pool.subtab_state(&project).unwrap();
        pool.commit_subtabs(
            &project,
            initial.revision,
            initial.set_concurrency(&initial.active_id, 3).unwrap(),
        )
        .unwrap();
        let state = pool.subtab_state(&project).unwrap();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let gates = Arc::new((Mutex::new(vec![false; 5]), std::sync::Condvar::new()));
        let releases = gates.clone();
        let (entered, starts) = mpsc::channel();
        let runner = Runner::spawn(
            &mut pool,
            &project,
            &state.active_id,
            RuntimeConfig::default(),
            move |worker, job: WorkerPrompt, token| {
                let index: usize = job.text.parse().unwrap();
                let mut agent = worker.agent().lock().unwrap();
                entered
                    .send((index, agent.session().session_id().to_owned()))
                    .unwrap();
                let deadline = Instant::now() + Duration::from_secs(5);
                let mut ready = releases.0.lock().unwrap();
                while !ready[index] && !token.is_cancelled() {
                    assert!(Instant::now() < deadline);
                    ready = releases
                        .1
                        .wait_timeout(ready, Duration::from_millis(5))
                        .unwrap()
                        .0;
                }
                drop(ready);
                agent.process_with_cancel(
                    crate::core::TurnInputRequest::new(format!("lowered-job-{index}")),
                    || token.is_cancelled(),
                )
            },
        )
        .unwrap();
        let key = (project.clone(), state.active_id.clone());
        let mut controller = WorktreeHost::default();
        controller.lanes.insert(
            key.clone(),
            Lane {
                runner,
                progress: WorkerProgress::default(),
                live: Arc::new(super::super::live_text::LivePromptText::default()),
            },
        );
        let requests: Vec<_> = (0..5).map(|index| request(&pool, &format!("job-{index}"), Operation::Prompt,
            json!({"text":index.to_string(),"subtab_id":state.active_id,"subtab_revision":state.revision}))).collect();
        for prompt in &requests {
            assert_eq!(
                controller
                    .dispatch_prompt(&mut pool, &registry, &device, prompt, false)
                    .execution_state,
                Some(ExecutionState::Accepted)
            );
        }
        let initial_starts: Vec<_> = (0..3)
            .map(|_| starts.recv_timeout(Duration::from_secs(3)).unwrap())
            .collect();
        assert_eq!(
            initial_starts
                .iter()
                .map(|(_, session)| session)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            3
        );
        let quota = request(
            &pool,
            "lower-to-one",
            Operation::SetConcurrency,
            json!({"subtab_id":state.active_id,"subtab_revision":state.revision,"concurrency":1}),
        );
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let value = controller
                .dispatch_quota(&mut pool, &registry, &device, &quota, false)
                .unwrap();
            if value.execution_state == Some(ExecutionState::Succeeded) {
                break;
            }
            assert_eq!(value.execution_state, Some(ExecutionState::UnknownOutcome));
            assert!(Instant::now() < deadline);
            controller.poll(&mut pool);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(controller.lanes[&key].runner.applied_concurrency(), 1);
        assert_eq!(controller.lanes[&key].runner.quota(), 3);
        assert_eq!(pool.worker_handles(&project).len(), 3);
        let release = |index| {
            gates.0.lock().unwrap()[index] = true;
            gates.1.notify_all();
        };
        for index in 0..2 {
            release(index);
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                controller.poll(&mut pool);
                let values = controller.listing(&pool, device.id(), &project);
                if values.as_array().unwrap().iter().any(|value| {
                    value["request_id"] == format!("job-{index}") && value["state"] == "succeeded"
                }) {
                    break;
                }
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(
                starts.recv_timeout(Duration::from_millis(30)).is_err(),
                "new starts wait while live count >= lowered limit"
            );
        }
        release(2);
        assert_eq!(starts.recv_timeout(Duration::from_secs(3)).unwrap().0, 3);
        assert!(starts.recv_timeout(Duration::from_millis(30)).is_err());
        release(3);
        assert_eq!(starts.recv_timeout(Duration::from_secs(3)).unwrap().0, 4);
        release(4);
        let deadline = Instant::now() + Duration::from_secs(3);
        while controller.busy(&project) {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        let values = controller.listing(&pool, device.id(), &project);
        for index in 0..5 {
            assert!(
                values
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| value["request_id"] == format!("job-{index}")
                        && value["state"] == "succeeded")
            );
        }
        assert!(pool.worker_handles(&project).is_empty());
        assert_eq!(pool.subtab_state(&project).unwrap().tabs[0].concurrency, 1);
        assert!(master.lock().unwrap().session().turns().is_empty());
    }

    #[test]
    #[ignore = "requires current Kotlin JVM classes; executed by source checker"]
    fn android_cancel_frame_stops_one_running_worker_and_restores_receipt() {
        use super::super::protocol::{Channel, ExecutionState};
        use crate::backend::{
            Backend, BackendError, Completion, CompletionRequest, ProviderEvent, RequestControl,
        };
        use base64::engine::general_purpose::STANDARD;
        use std::sync::atomic::AtomicUsize;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::create_dir(root.join("sibling")).unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let initial = pool.subtab_state(&project).unwrap();
        let main = initial.active_id.clone();
        let two = initial
            .add_directory(&project, &root, &root.join("sibling"), "sibling")
            .unwrap();
        let sibling = two.active_id.clone();
        pool.commit_subtabs(&project, initial.revision, two)
            .unwrap();
        let tabs = pool.subtab_state(&project).unwrap();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let foreign = phone(&registry, &project);
        let starts = Arc::new(AtomicUsize::new(0));
        let cancels = Arc::new(AtomicUsize::new(0));
        let release = Arc::new(AtomicBool::new(false));
        struct ControlledBackend {
            starts: Arc<AtomicUsize>,
            cancels: Arc<AtomicUsize>,
            release: Arc<AtomicBool>,
        }
        impl Backend for ControlledBackend {
            fn complete(&self, _: CompletionRequest<'_>) -> Result<Completion, BackendError> {
                panic!("controlled request boundary required")
            }
            fn complete_with_control(
                &self,
                _: CompletionRequest<'_>,
                cancelled: &dyn Fn() -> bool,
                sink: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
            ) -> Result<Completion, BackendError> {
                self.starts.fetch_add(1, Ordering::SeqCst);
                sink(ProviderEvent::TextDelta {
                    delta: "ACTUAL_WORKER_PREVIEW\n".into(),
                })?;
                let deadline = Instant::now() + Duration::from_secs(30);
                while !self.release.load(Ordering::Acquire) && !cancelled() {
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(1));
                }
                if cancelled() {
                    self.cancels.fetch_add(1, Ordering::SeqCst);
                    return Err(BackendError::Cancelled);
                }
                Ok(Completion::text("SIBLING_COMPLETED_AFTER_TARGET_CANCEL"))
            }
            fn complete_with_request_control(
                &self,
                request: CompletionRequest<'_>,
                control: &mut RequestControl<'_>,
                sink: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
            ) -> Result<Completion, BackendError> {
                control.check_cancelled()?;
                self.complete_with_control(request, &|| control.check_cancelled().is_err(), sink)
            }
        }
        let mut worker_paths = Vec::new();
        let seed_path = root.join("seed.json");
        let command_path = root.join("command.frame");
        let result_path = root.join("results.json");
        let run_java = |phase| run_worker_cancel_kotlin(&root, phase);
        let mut controller = WorktreeHost::default();
        for tab in [&main, &sibling] {
            let worker = pool.worker_owner(&project, tab, 0).unwrap();
            let path = worker.agent().lock().unwrap().session().path().to_owned();
            let mut agent = Agent::new(
                SessionStore::open_existing_writable(&path).unwrap(),
                Box::new(ControlledBackend {
                    starts: starts.clone(),
                    cancels: cancels.clone(),
                    release: release.clone(),
                }),
            );
            agent.set_attachment_workspace(
                crate::tools::ToolContext::new(worker.execution().tab.root.as_path()).unwrap(),
            );
            *worker.agent().lock().unwrap() = agent;
            worker_paths.push(path);
            drop(worker);
            let live = Arc::new(super::super::live_text::LivePromptText::default());
            let runner = Runner::spawn_observed_prompts(
                &mut pool,
                &project,
                tab,
                RuntimeConfig::default(),
                live.clone(),
            )
            .unwrap();
            controller.lanes.insert(
                (project.clone(), tab.clone()),
                Lane {
                    runner,
                    progress: WorkerProgress::default(),
                    live,
                },
            );
        }
        let mut submissions = Vec::new();
        for (id, tab) in [("cancel-target", &main), ("sibling-task", &sibling)] {
            let mut seed = serde_json::Map::new();
            for (name, operation) in [
                ("listing", Operation::WorkspaceList),
                ("snapshot", Operation::Inspect),
                ("subtabs", Operation::SubtabList),
            ] {
                let mut read = request(&pool, name, operation, json!({}));
                read.required_owner = None;
                read.expected_revision = None;
                let response = HostAdapter::dispatch_owned(&mut pool, &registry, &device, read);
                assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
                let Message::Response { result } = response.payload else {
                    panic!("submission view")
                };
                seed.insert(name.into(), result);
            }
            seed.get_mut("listing").unwrap()["projects"][0]["worker_control"] = json!(true);
            seed.insert("target".into(), json!(id));
            seed.insert("submit_tab".into(), json!(tab));
            std::fs::write(&seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
            run_java("submit");
            let bytes = std::fs::read(&command_path).unwrap();
            let mut input = bytes.as_slice();
            let prompt = super::super::frame::read_frame(&mut input)
                .unwrap()
                .unwrap();
            assert!(input.is_empty());
            assert_eq!(prompt.id.as_deref(), Some(id));
            assert!(
                matches!(&prompt.payload,Message::Request { operation:Operation::Prompt,arguments }
                if arguments==&json!({"text":"wait","subtab_id":tab,"subtab_revision":tabs.revision}))
            );
            let accepted =
                controller.dispatch_prompt(&mut pool, &registry, &device, &prompt, false);
            assert_eq!(accepted.execution_state, Some(ExecutionState::Accepted));
            let retry = controller.dispatch_prompt(&mut pool, &registry, &device, &prompt, false);
            assert_eq!(retry, accepted);
            submissions.push((prompt, accepted));
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        while starts.load(Ordering::SeqCst) != 2 {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
        controller.poll(&mut pool);
        let view = |pool: &mut ProjectOwnerPool, operation| {
            let response = HostAdapter::dispatch_owned(
                pool,
                &registry,
                &device,
                Envelope {
                    required_owner: None,
                    expected_revision: None,
                    ..request(pool, "view", operation, json!({}))
                },
            );
            assert_eq!(response.execution_state, Some(ExecutionState::Succeeded));
            let Message::Response { result } = response.payload else {
                panic!("view")
            };
            result
        };
        let mut listed = view(&mut pool, Operation::WorkspaceList);
        listed["projects"][0]["worker_control"] = json!(true);
        listed["projects"][0]["worker_statuses"] = controller.listing(&pool, device.id(), &project);
        let snapshot = view(&mut pool, Operation::Inspect);
        std::fs::write(
            &seed_path,
            serde_json::to_vec(
                &json!({"listing":listed,"snapshot":snapshot,"target":"cancel-target"}),
            )
            .unwrap(),
        )
        .unwrap();
        run_java("generate");
        let bytes = std::fs::read(&command_path).unwrap();
        let mut input = bytes.as_slice();
        let cancel = super::super::frame::read_frame(&mut input)
            .unwrap()
            .unwrap();
        assert!(input.is_empty());
        assert_eq!(cancel.channel, Channel::Control);
        let mut exchanges = Vec::new();
        let mut retain = |command: &Envelope, response: &Envelope| {
            let mut wire = Vec::new();
            super::super::frame::write_frame(&mut wire, response).unwrap();
            let wire = STANDARD.encode(wire);
            exchanges.push(json!({"request":command,"code":response.code,"wire_base64":wire}));
            wire
        };
        let refused = controller
            .cancel(&mut pool, &registry, &foreign, &cancel)
            .unwrap();
        assert_eq!(refused.code, Some(ErrorCode::OwnerMismatch));
        retain(&cancel, &refused);
        let mut stale = cancel.clone();
        stale.expected_revision = Some(pool.workspace_revision() + 1);
        let refused = controller
            .cancel(&mut pool, &registry, &device, &stale)
            .unwrap();
        assert_eq!(refused.code, Some(ErrorCode::RevisionConflict));
        retain(&stale, &refused);
        let receipt = controller
            .cancel(&mut pool, &registry, &device, &cancel)
            .unwrap();
        assert_eq!(receipt.execution_state, Some(ExecutionState::Accepted));
        let first = retain(&cancel, &receipt);
        let retry = controller
            .cancel(&mut pool, &registry, &device, &cancel)
            .unwrap();
        assert_eq!(retry, receipt);
        let second = retain(&cancel, &retry);
        loop {
            controller.poll(&mut pool);
            let values = controller.listing(&pool, device.id(), &project);
            if values
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["request_id"] == "cancel-target" && item["state"] == "cancelled")
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
        let values = controller.listing(&pool, device.id(), &project);
        assert!(
            values
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["request_id"] == "sibling-task" && item["state"] == "running")
        );
        assert_eq!(cancels.load(Ordering::SeqCst), 1);
        assert_eq!(starts.load(Ordering::SeqCst), 2);
        assert!(
            values
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["request_id"] == "sibling-task"
                    && item["stream_text"] == "ACTUAL_WORKER_PREVIEW\n"),
            "actual worker preview missing: {values}"
        );
        listed["projects"][0]["worker_statuses"] = values;
        release.store(true, Ordering::Release);
        while controller.busy(&project) {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
        controller.shutdown(&pool, Instant::now() + Duration::from_secs(2));
        let finished = super::super::worker_progress::listing(&pool, device.id(), &project);
        assert!(
            finished
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["request_id"] == "sibling-task" && item["state"] == "succeeded")
        );
        let target_journal = SessionStore::open_existing(&worker_paths[0]).unwrap();
        assert_eq!(
            target_journal
                .turns()
                .iter()
                .filter(|turn| turn.role == crate::core::TurnRole::User)
                .count(),
            1
        );
        assert!(
            target_journal
                .turns()
                .iter()
                .all(|turn| turn.role != crate::core::TurnRole::Assistant)
        );
        let sibling_journal = SessionStore::open_existing(&worker_paths[1]).unwrap();
        assert_eq!(
            sibling_journal
                .turns()
                .iter()
                .filter(|turn| turn.role == crate::core::TurnRole::Assistant
                    && turn.content == "SIBLING_COMPLETED_AFTER_TARGET_CANCEL")
                .count(),
            1
        );
        assert!(master.lock().unwrap().session().turns().is_empty());
        drop(pool);
        drop(master);
        let recovered_master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_existing_writable(root.join("session.jsonl")).unwrap(),
        )));
        let mut restored = ProjectOwnerPool::new(recovered_master.clone()).unwrap();
        restored.restore_checkpoint().unwrap();
        let journal = std::fs::read(root.join("session.jsonl")).unwrap();
        let retry = super::super::command::CancelAdmissions.dispatch(
            &mut restored,
            &registry,
            &device,
            &cancel,
            None,
            || panic!("receipt recovery must not cancel a replacement"),
        );
        assert_eq!(retry, receipt);
        let third = retain(&cancel, &retry);
        assert_eq!(std::fs::read(root.join("session.jsonl")).unwrap(), journal);
        assert!(restored.worker_handles(&project).is_empty());
        let mut replay_controller = WorktreeHost::default();
        for (prompt, accepted) in &submissions {
            assert_eq!(
                replay_controller.dispatch_prompt(&mut restored, &registry, &device, prompt, false),
                *accepted
            );
        }
        assert!(restored.worker_handles(&project).is_empty());
        assert_eq!(starts.load(Ordering::SeqCst), 2);
        assert_eq!(std::fs::read(root.join("session.jsonl")).unwrap(), journal);
        replay_controller.shutdown(&restored, Instant::now() + Duration::from_secs(2));
        let mut security_exchanges = Vec::new();
        let mut capture_security = |command: &Envelope, response: &Envelope| {
            let mut wire = Vec::new();
            super::super::frame::write_frame(&mut wire, response).unwrap();
            security_exchanges.push(
                json!({"request":command,"code":response.code,"wire_base64":STANDARD.encode(wire)}),
            );
        };
        let mut replay_controller = WorktreeHost::default();
        for (prompt, _) in &submissions {
            let mut changed = prompt.clone();
            let Message::Request { arguments, .. } = &mut changed.payload else {
                panic!("prompt")
            };
            arguments["text"] = json!("different task with reused identity");
            let refused = replay_controller.dispatch_prompt(
                &mut restored,
                &registry,
                &device,
                &changed,
                false,
            );
            assert_eq!(refused.code, Some(ErrorCode::RequestIdConflict));
            capture_security(&changed, &refused);
        }
        let mut changed_cancel = cancel.clone();
        let Message::Request { arguments, .. } = &mut changed_cancel.payload else {
            panic!("cancel")
        };
        arguments["request_id"] = json!("sibling-task");
        let refused = super::super::command::CancelAdmissions.dispatch(
            &mut restored,
            &registry,
            &device,
            &changed_cancel,
            None,
            || panic!("conflicting retry must not cancel"),
        );
        assert_eq!(refused.code, Some(ErrorCode::RequestIdConflict));
        capture_security(&changed_cancel, &refused);
        registry.revoke(device.id()).unwrap();
        for (prompt, _) in &submissions {
            let refused =
                replay_controller.dispatch_prompt(&mut restored, &registry, &device, prompt, false);
            assert_eq!(refused.code, Some(ErrorCode::Unauthorized));
            capture_security(prompt, &refused);
        }
        let refused = super::super::command::CancelAdmissions.dispatch(
            &mut restored,
            &registry,
            &device,
            &cancel,
            None,
            || panic!("revoked retry must not cancel"),
        );
        assert_eq!(refused.code, Some(ErrorCode::Unauthorized));
        capture_security(&cancel, &refused);
        assert!(restored.worker_handles(&project).is_empty());
        assert_eq!(starts.load(Ordering::SeqCst), 2);
        assert_eq!(cancels.load(Ordering::SeqCst), 1);
        assert_eq!(std::fs::read(root.join("session.jsonl")).unwrap(), journal);
        replay_controller.shutdown(&restored, Instant::now() + Duration::from_secs(2));
        let submission_wires = submissions
            .iter()
            .map(|(prompt, accepted)| {
                let mut wire = Vec::new();
                super::super::frame::write_frame(&mut wire, accepted).unwrap();
                json!({"request":prompt,"wire_base64":STANDARD.encode(wire)})
            })
            .collect::<Vec<_>>();
        let evidence = json!({"request":cancel,"exchanges":exchanges,"receipts":[first,second,third],"after":listed,
            "starts":starts.load(Ordering::SeqCst),"cancel_observations":cancels.load(Ordering::SeqCst),"fixture_only":true,
            "actual_agent_execution":true,"target_user_turns":1,"target_assistant_turns":0,"sibling_assistant_turns":1,"finished":finished,
            "kotlin_submissions":submission_wires,"submission_restart_equal":true,"security_exchanges":security_exchanges,
            "post_security_journal_unchanged":true,"post_security_workers_absent":true});
        std::fs::write(&result_path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
        run_java("validate");
        if let Ok(path) = std::env::var("ZENPI_ANDROID_WORKER_CANCEL_OUTPUT") {
            std::fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
        }
    }

    fn run_worker_cancel_kotlin(root: &std::path::Path, phase: &str) {
        let mut child =
            std::process::Command::new(std::env::var("ZENPI_ANDROID_INTEROP_JAVA").unwrap())
                .args([
                    format!("-Dzenpi.workercancel.phase={phase}"),
                    format!(
                        "-Dzenpi.workercancel.seed={}",
                        root.join("seed.json").display()
                    ),
                    format!(
                        "-Dzenpi.workercancel.command={}",
                        root.join("command.frame").display()
                    ),
                    format!(
                        "-Dzenpi.workercancel.results={}",
                        root.join("results.json").display()
                    ),
                    "-cp".into(),
                    std::env::var("ZENPI_ANDROID_INTEROP_CLASSPATH").unwrap(),
                    "org.junit.runner.JUnitCore".into(),
                    "zenpi.AndroidRustWorkerCancelTest".into(),
                ])
                .spawn()
                .unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "Kotlin {phase}");
                break;
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("Kotlin deadline");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn request(
        pool: &ProjectOwnerPool,
        id: &str,
        operation: Operation,
        arguments: Value,
    ) -> Envelope {
        let mut req = Envelope::new(
            super::super::protocol::Channel::Control,
            Message::Request {
                operation,
                arguments,
            },
        );
        req.id = Some(id.into());
        req.project_id = Some(pool.active_context().project_id.clone());
        req.session_id = Some(pool.active_context().session_id.clone());
        req.required_owner = Some(pool.workspace_owner().into());
        req.expected_revision = Some(pool.workspace_revision());
        req
    }

    #[test]
    fn paged_worker_status_is_device_scoped_digest_fenced_and_strips_output() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let state = pool.subtab_state(&project).unwrap();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let foreign = phone(&registry, &project);
        let mut controller = WorktreeHost::default();
        let runner = Runner::spawn(
            &mut pool,
            &project,
            &state.active_id,
            RuntimeConfig::default(),
            move |_, _: WorkerPrompt, _| Err(AgentError::InvalidTurn("status fixture".into())),
        )
        .unwrap();
        controller.lanes.insert(
            (project.clone(), state.active_id.clone()),
            Lane {
                runner,
                progress: WorkerProgress::default(),
                live: Arc::new(super::super::live_text::LivePromptText::default()),
            },
        );
        for index in 0..18 {
            let prompt = request(
                &pool,
                &format!("page-task-{index}"),
                Operation::Prompt,
                json!({"text":"private prompt", "subtab_id":state.active_id,"subtab_revision":state.revision}),
            );
            assert_eq!(
                controller
                    .dispatch_prompt(&mut pool, &registry, &device, &prompt, false)
                    .execution_state,
                Some(super::super::protocol::ExecutionState::Accepted)
            );
        }
        controller.poll(&mut pool);
        let mut first = request(
            &pool,
            "page-first",
            Operation::WorkspaceList,
            json!({"view":"worker_status","offset":0,"digest":null}),
        );
        first.required_owner = None;
        first.expected_revision = None;
        let page = controller
            .status_page(&pool, &registry, &device, &first)
            .unwrap();
        assert_eq!(
            page.execution_state,
            Some(super::super::protocol::ExecutionState::Succeeded)
        );
        let payload = match page.payload {
            Message::Response { result } => result,
            _ => panic!("missing status result"),
        };
        assert_eq!(payload["offset"], 0);
        assert_eq!(payload["next_offset"], 16);
        assert_eq!(payload["total_tasks"], 18);
        assert_eq!(payload["tasks"].as_array().unwrap().len(), 16);
        assert!(!payload.to_string().contains("private prompt"));
        let digest = payload["digest"].as_str().unwrap().to_owned();
        first.id = Some("page-second".into());
        if let Message::Request { arguments, .. } = &mut first.payload {
            arguments["offset"] = json!(16);
            arguments["digest"] = json!(digest);
        }
        let second = controller
            .status_page(&pool, &registry, &device, &first)
            .unwrap();
        let second_result = match second.payload {
            Message::Response { result } => result,
            _ => panic!("missing second"),
        };
        assert_eq!(second_result["offset"], 16);
        assert_eq!(second_result["next_offset"], 18);
        assert_eq!(second_result["has_more"], false);
        assert_eq!(
            controller
                .status_page(&pool, &registry, &foreign, &first)
                .unwrap()
                .code,
            Some(ErrorCode::RevisionConflict)
        );
        if let Message::Request { arguments, .. } = &mut first.payload {
            arguments["digest"] = json!("0".repeat(64));
        }
        assert_eq!(
            controller
                .status_page(&pool, &registry, &device, &first)
                .unwrap()
                .code,
            Some(ErrorCode::RevisionConflict)
        );
        let mut extra = request(
            &pool,
            "bad-page",
            Operation::WorkspaceList,
            json!({"view":"worker_status","offset":0,"digest":null,"extra":true}),
        );
        extra.required_owner = None;
        extra.expected_revision = None;
        assert_eq!(
            controller
                .status_page(&pool, &registry, &device, &extra)
                .unwrap()
                .code,
            Some(ErrorCode::InvalidRequest)
        );
        controller.shutdown(&pool, Instant::now() + Duration::from_secs(1));
    }

    #[test]
    fn completed_task_outside_summary_remains_readable_from_later_page_without_resubmission() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let tabs = pool.subtab_state(&project).unwrap();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let foreign = phone(&registry, &project);
        let mut controller = WorktreeHost::default();
        for index in (0..18).rev() {
            let prompt = request(
                &pool,
                &format!("completed-page-{index:02}"),
                Operation::Prompt,
                json!({"text":format!("page-output-{index:02}"),"subtab_id":tabs.active_id,"subtab_revision":tabs.revision}),
            );
            let admitted =
                controller.dispatch_prompt(&mut pool, &registry, &device, &prompt, false);
            assert_eq!(
                admitted.execution_state,
                Some(super::super::protocol::ExecutionState::Accepted),
                "task {index}: {admitted:?}"
            );
            let deadline = Instant::now() + Duration::from_secs(5);
            while controller.busy(&project) {
                controller.poll(&mut pool);
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        let summary = controller.listing(&pool, device.id(), &project);
        assert_eq!(summary.as_array().unwrap().len(), 16);
        let mut page_request = request(
            &pool,
            "completed-first",
            Operation::WorkspaceList,
            json!({"view":"worker_status","offset":0,"digest":null}),
        );
        page_request.required_owner = None;
        page_request.expected_revision = None;
        let Message::Response { result: first } = controller
            .status_page(&pool, &registry, &device, &page_request)
            .unwrap()
            .payload
        else {
            panic!("missing first page");
        };
        page_request.id = Some("completed-second".into());
        page_request.payload = Message::Request {
            operation: Operation::WorkspaceList,
            arguments: json!({"view":"worker_status","offset":16,"digest":first["digest"]}),
        };
        let Message::Response { result: second } = controller
            .status_page(&pool, &registry, &device, &page_request)
            .unwrap()
            .payload
        else {
            panic!("missing second page");
        };
        assert_eq!(second["total_tasks"], 18);
        assert_eq!(second["tasks"].as_array().unwrap().len(), 2);
        // The summary keeps recent completions, whereas the stable pager sorts IDs.
        // Select an actual paged identity absent from the summary, not a presumed index.
        let target = second["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|task| {
                !summary
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["request_id"] == task["request_id"])
            })
            .expect("task outside bounded summary");
        assert_eq!(target["state"], "succeeded");
        let before_master = std::fs::read(master.lock().unwrap().session().path()).unwrap();
        let revision = pool.workspace_revision();
        let workers_before = std::fs::read_dir(root.join(".zenpi/workers"))
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                (path.clone(), std::fs::read(path).unwrap())
            })
            .collect::<BTreeMap<_, _>>();
        for history in [false, true] {
            let mut args = json!({"request_id":target["request_id"],"offset":0,"digest":null});
            if history {
                args["view"] = json!("history");
            }
            let mut read = request(
                &pool,
                "read-paged-completion",
                Operation::TranscriptRead,
                args,
            );
            read.required_owner = None;
            read.expected_revision = None;
            let result = super::super::transcript::read(&pool, &registry, &device, &read).unwrap();
            assert_eq!(result["request_id"], target["request_id"]);
            assert_eq!(result["subtab_id"], target["subtab_id"]);
            assert!(result.to_string().contains("page-output-"));
            assert_eq!(
                super::super::transcript::read(&pool, &registry, &foreign, &read),
                Err(ErrorCode::InvalidRequest)
            );
            assert_eq!(
                super::super::transcript::read(&pool, &registry, &device, &read).unwrap(),
                result
            );
        }
        assert_eq!(pool.workspace_revision(), revision);
        assert_eq!(
            std::fs::read(master.lock().unwrap().session().path()).unwrap(),
            before_master
        );
        for (path, bytes) in workers_before {
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
        assert!(pool.worker_handles(&project).is_empty());
        assert!(!controller.busy(&project));
        controller.shutdown(&pool, Instant::now() + Duration::from_secs(1));
    }

    #[test]
    fn sibling_live_lanes_keep_executing_and_refresh_only_unchanged_scope_after_quota_commit() {
        use super::super::protocol::ExecutionState;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let child = root.join("child");
        std::fs::create_dir(&child).unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let state = pool.subtab_state(&project).unwrap();
        pool.commit_subtabs(
            &project,
            state.revision,
            state
                .add_directory(&project, &root, &child, "child")
                .unwrap(),
        )
        .unwrap();
        let state = pool.subtab_state(&project).unwrap();
        let tabs = [state.tabs[0].id.clone(), state.tabs[1].id.clone()];
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let other_device = phone(&registry, &project);
        let release = Arc::new(AtomicBool::new(false));
        let entered = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut controller = WorktreeHost::default();
        for tab in &tabs {
            let gates = release.clone();
            let starts = entered.clone();
            let runner = Runner::spawn(
                &mut pool,
                &project,
                tab,
                RuntimeConfig {
                    event_capacity: 1,
                    ..RuntimeConfig::default()
                },
                move |worker, job: WorkerPrompt, token| {
                    let mut agent = worker.agent().lock().unwrap();
                    starts.fetch_add(1, Ordering::Release);
                    let deadline = Instant::now() + Duration::from_secs(5);
                    while !gates.load(Ordering::Acquire) && !token.is_cancelled() {
                        assert!(Instant::now() < deadline);
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    agent.process_with_cancel(crate::core::TurnInputRequest::new(job.text), || {
                        token.is_cancelled()
                    })
                },
            )
            .unwrap();
            controller.lanes.insert(
                (project.clone(), tab.clone()),
                Lane {
                    runner,
                    progress: WorkerProgress::default(),
                    live: Arc::new(super::super::live_text::LivePromptText::default()),
                },
            );
        }
        for (index, tab) in tabs.iter().enumerate() {
            let req = request(
                &pool,
                &format!("original-{index}"),
                Operation::Prompt,
                json!({"text":format!("original-{index}"),"subtab_id":tab,"subtab_revision":state.revision}),
            );
            assert_eq!(
                controller
                    .dispatch_prompt(&mut pool, &registry, &device, &req, false)
                    .execution_state,
                Some(ExecutionState::Accepted)
            );
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        while entered.load(Ordering::Acquire) != 2 {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        // Both roots have active writers; changing one quota must not take the other's Agent lock.
        for (index, tab) in tabs.iter().enumerate() {
            let current = pool.subtab_state(&project).unwrap();
            let quota = request(
                &pool,
                &format!("quota-{index}"),
                Operation::SetConcurrency,
                json!({"subtab_id":tab,"subtab_revision":current.revision,"concurrency":2}),
            );
            if index == 0 {
                assert_eq!(
                    controller
                        .dispatch_quota(&mut pool, &registry, &device, &quota, false)
                        .unwrap()
                        .execution_state,
                    Some(ExecutionState::UnknownOutcome)
                );
                let competing = request(
                    &pool,
                    "competing-sibling-quota",
                    Operation::SetConcurrency,
                    json!({"subtab_id":tabs[1],"subtab_revision":current.revision,"concurrency":2}),
                );
                let checkpoint = std::fs::read(root.join("project-workspace.json")).unwrap();
                let owners = pool.worker_handles(&project).len();
                assert_eq!(
                    controller
                        .dispatch_quota(&mut pool, &registry, &other_device, &competing, false)
                        .unwrap()
                        .code,
                    Some(ErrorCode::Busy)
                );
                assert_eq!(
                    super::super::subtab::mutate(&mut pool, &registry, &other_device, &competing),
                    Err(ErrorCode::Busy)
                );
                assert_eq!(pool.unresolved_quota_intents().len(), 1);
                assert_eq!(pool.worker_handles(&project).len(), owners);
                assert_eq!(
                    std::fs::read(root.join("project-workspace.json")).unwrap(),
                    checkpoint
                );
                assert_eq!(
                    controller.lanes[&(project.clone(), tabs[1].clone())]
                        .runner
                        .quota(),
                    1
                );
                assert!(
                    !controller.lanes[&(project.clone(), tabs[1].clone())]
                        .runner
                        .quota_change_pending()
                );
            }
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                let value = controller
                    .dispatch_quota(&mut pool, &registry, &device, &quota, false)
                    .unwrap();
                if value.execution_state == Some(ExecutionState::Succeeded) {
                    break;
                }
                assert_eq!(value.execution_state, Some(ExecutionState::UnknownOutcome));
                controller.poll(&mut pool);
                assert!(Instant::now() < deadline, "{value:?}");
                std::thread::sleep(Duration::from_millis(1));
            }
            let current = pool.subtab_state(&project).unwrap();
            let sibling = &tabs[1 - index];
            let prompt = request(
                &pool,
                &format!("fresh-sibling-{index}"),
                Operation::Prompt,
                json!({"text":format!("fresh-sibling-{index}"),"subtab_id":sibling,"subtab_revision":current.revision}),
            );
            assert_eq!(
                controller
                    .dispatch_prompt(&mut pool, &registry, &device, &prompt, false)
                    .execution_state,
                Some(ExecutionState::Accepted)
            );
            assert_eq!(
                controller.lanes[&(project.clone(), sibling.clone())]
                    .runner
                    .scope()
                    .3,
                current.revision
            );
        }
        assert_eq!(
            pool.subtab_state(&project)
                .unwrap()
                .tabs
                .iter()
                .map(|tab| tab.concurrency)
                .collect::<Vec<_>>(),
            vec![2, 2]
        );
        assert_eq!(pool.worker_handles(&project).len(), 4);
        release.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_secs(3);
        while controller.busy(&project) {
            controller.poll(&mut pool);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(entered.load(Ordering::Acquire), 4);
        assert!(pool.worker_handles(&project).is_empty());
        let values = controller.listing(&pool, device.id(), &project);
        assert_eq!(
            values
                .as_array()
                .unwrap()
                .iter()
                .filter(|value| value["state"] == "succeeded")
                .count(),
            4
        );
        assert!(master.lock().unwrap().session().turns().is_empty());
    }

    #[test]
    fn production_worker_prompt_pipeline_emits_redacted_text_and_retains_completed_result() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master.clone()).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let state = pool.subtab_state(&project).unwrap();
        let live = Arc::new(super::super::live_text::LivePromptText::default());
        let runner = Runner::spawn_observed_prompts(
            &mut pool,
            &project,
            &state.active_id,
            RuntimeConfig::default(),
            live.clone(),
        )
        .unwrap();
        let req = request(
            &pool,
            "actual-stream",
            Operation::Prompt,
            json!({"text":"public\nAPI_KEY=PRIVATE_ACTUAL_STREAM\n", "subtab_id":state.active_id,"subtab_revision":state.revision}),
        );
        runner
            .try_submit(WorkerPrompt {
                text: "public\nAPI_KEY=PRIVATE_ACTUAL_STREAM\n".into(),
                request: req.clone(),
                device: "device".into(),
            })
            .unwrap();
        loop {
            if let crate::runtime::RuntimeEvent::Completed { outcome, .. } =
                runner.recv_timeout(Duration::from_secs(3)).unwrap()
            {
                assert!(matches!(outcome, crate::runtime::JobOutcome::Succeeded(_)));
                break;
            }
        }
        let text = live
            .snapshot(
                "device",
                &project,
                &pool.active_context().session_id,
                "actual-stream",
            )
            .unwrap()
            .text;
        assert!(text.contains("public"));
        assert!(!text.contains("PRIVATE_ACTUAL_STREAM"));
        assert!(
            live.snapshot(
                "foreign",
                &project,
                &pool.active_context().session_id,
                "actual-stream"
            )
            .is_none()
        );
        assert!(master.lock().unwrap().session().turns().is_empty());
        runner.shutdown_and_join().unwrap();
    }

    #[test]
    fn concurrent_live_worker_text_is_request_device_scoped_and_removed_at_terminal() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let initial = pool.subtab_state(&project).unwrap();
        pool.commit_subtabs(
            &project,
            initial.revision,
            initial.set_concurrency(&initial.active_id, 2).unwrap(),
        )
        .unwrap();
        let state = pool.subtab_state(&project).unwrap();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let other = phone(&registry, &project);
        let live = Arc::new(super::super::live_text::LivePromptText::default());
        let emitting = live.clone();
        let entered = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let started = entered.clone();
        let runner = Runner::spawn(
            &mut pool,
            &project,
            &state.active_id,
            RuntimeConfig::default(),
            move |_, job: WorkerPrompt, token| {
                emitting.update(
                    &job.request,
                    &job.device,
                    &crate::backend::ProviderEvent::TextDelta {
                        delta: format!("{}\nAPI_KEY=PRIVATE", job.text),
                    },
                );
                emitting.update(
                    &job.request,
                    &job.device,
                    &crate::backend::ProviderEvent::TextDelta {
                        delta: "_WORKER_STREAM_SECRET\n".into(),
                    },
                );
                started.fetch_add(1, Ordering::Release);
                let deadline = Instant::now() + Duration::from_secs(5);
                while !token.is_cancelled() {
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(AgentError::InvalidTurn("PRIVATE_DIAGNOSTIC".into()))
            },
        )
        .unwrap();
        let mut controller = WorktreeHost::default();
        controller.lanes.insert(
            (project.clone(), state.active_id.clone()),
            Lane {
                runner,
                progress: WorkerProgress::default(),
                live: live.clone(),
            },
        );
        let requests: Vec<_> = (0..2).map(|index| request(&pool, &format!("live-{index}"), Operation::Prompt,
            json!({"text":format!("worker-{index}"), "subtab_id":state.active_id,"subtab_revision":state.revision}))).collect();
        for (device, request) in [(&device, &requests[0]), (&other, &requests[1])] {
            assert_eq!(
                controller
                    .dispatch_prompt(&mut pool, &registry, device, request, false)
                    .execution_state,
                Some(super::super::protocol::ExecutionState::Accepted)
            );
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        while entered.load(Ordering::Acquire) != 2 {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        controller.poll(&mut pool);
        for (device, index) in [(&device, 0), (&other, 1)] {
            let values = controller.listing(&pool, device.id(), &project);
            assert_eq!(values.as_array().unwrap().len(), 1);
            let text = values[0]["stream_text"].as_str().unwrap();
            assert!(text.contains(&format!("worker-{index}")));
            assert!(!text.contains(&format!("worker-{}", 1 - index)));
            assert!(!text.contains("PRIVATE_WORKER_STREAM_SECRET"));
            assert!(
                super::super::worker_progress::listing(&pool, device.id(), &project)[0]
                    .get("stream_text")
                    .is_none()
            );
        }
        let cancel = request(
            &pool,
            "cancel-live-0",
            Operation::Cancel,
            json!({"request_id":"live-0"}),
        );
        controller
            .cancel(&mut pool, &registry, &device, &cancel)
            .unwrap();
        loop {
            controller.poll(&mut pool);
            let values = controller.listing(&pool, device.id(), &project);
            if values[0]["state"] == "cancelled" {
                assert!(values[0].get("stream_text").is_none());
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            live.snapshot(
                device.id(),
                &project,
                &pool.active_context().session_id,
                "live-0"
            )
            .is_none()
        );
        assert!(
            controller.listing(&pool, other.id(), &project)[0]["stream_text"]
                .as_str()
                .unwrap()
                .contains("worker-1")
        );
        controller.shutdown(&pool, Instant::now() + Duration::from_secs(1));
    }

    #[test]
    fn controller_routes_one_queued_cancel_and_shutdown_records_other_job_unknown() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let master = Arc::new(Mutex::new(Agent::with_echo(
            SessionStore::open_in_workspace(root.join("session.jsonl"), &root).unwrap(),
        )));
        let mut pool = ProjectOwnerPool::new(master).unwrap();
        pool.restore_checkpoint().unwrap();
        let project = pool.active_context().project_id.clone();
        let state = pool.subtab_state(&project).unwrap();
        let registry = DeviceRegistry::new(root.join("devices.json"), "a".repeat(64)).unwrap();
        let device = phone(&registry, &project);
        let other = phone(&registry, &project);
        let entered = Arc::new(AtomicBool::new(false));
        let started = entered.clone();
        let runner = Runner::spawn(
            &mut pool,
            &project,
            &state.active_id,
            RuntimeConfig::default(),
            move |_, _: WorkerPrompt, token| {
                started.store(true, Ordering::Release);
                let deadline = Instant::now() + Duration::from_secs(5);
                while !token.is_cancelled() {
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(AgentError::InvalidTurn(
                    "PRIVATE_EXECUTION_DIAGNOSTIC".into(),
                ))
            },
        )
        .unwrap();
        let mut controller = WorktreeHost::default();
        controller.lanes.insert(
            (project.clone(), state.active_id.clone()),
            Lane {
                runner,
                progress: WorkerProgress::default(),
                live: Arc::new(super::super::live_text::LivePromptText::default()),
            },
        );
        for index in 0..2 {
            let req = request(
                &pool,
                &format!("job-{index}"),
                Operation::Prompt,
                json!({"text":"wait", "subtab_id":state.active_id, "subtab_revision":state.revision}),
            );
            assert_eq!(
                controller
                    .dispatch_prompt(&mut pool, &registry, &device, &req, false)
                    .execution_state,
                Some(super::super::protocol::ExecutionState::Accepted)
            );
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        while !entered.load(Ordering::Acquire) {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        let cancel = request(
            &pool,
            "cancel-job-1",
            Operation::Cancel,
            json!({"request_id":"job-1"}),
        );
        assert_eq!(
            controller
                .cancel(&mut pool, &registry, &other, &cancel)
                .unwrap()
                .code,
            Some(ErrorCode::OwnerMismatch)
        );
        let receipt = controller
            .cancel(&mut pool, &registry, &device, &cancel)
            .unwrap();
        assert_eq!(
            receipt.execution_state,
            Some(super::super::protocol::ExecutionState::Accepted)
        );
        assert_eq!(
            controller
                .cancel(&mut pool, &registry, &device, &cancel)
                .unwrap(),
            receipt
        );
        loop {
            controller.poll(&mut pool);
            let values = controller.listing(&pool, device.id(), &project);
            if values
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value["request_id"] == "job-1" && value["state"] == "cancelled")
            {
                break;
            }
            assert!(Instant::now() < deadline, "{values}");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(controller.busy(&project));
        controller.shutdown(&pool, Instant::now() + Duration::from_secs(1));
        let values = super::super::worker_progress::listing(&pool, device.id(), &project);
        assert!(
            values
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value["request_id"] == "job-0" && value["state"] == "unknown_outcome")
        );
        assert!(!values.to_string().contains("PRIVATE_EXECUTION_DIAGNOSTIC"));
    }
}

#[derive(Default)]
pub(crate) struct WorktreeHost {
    lanes: BTreeMap<(String, String), Lane>,
}

impl WorktreeHost {
    pub fn status_page(
        &self,
        pool: &ProjectOwnerPool,
        registry: &DeviceRegistry,
        device: &AuthenticatedDevice,
        request: &Envelope,
    ) -> Option<Envelope> {
        let Message::Request {
            operation: Operation::WorkspaceList,
            arguments,
        } = &request.payload
        else {
            return None;
        };
        if arguments.get("view").is_none() {
            return None;
        }
        let result = (|| {
            if request.validate().is_err()
                || request.id.is_none()
                || request.required_owner.is_some()
                || request.expected_revision.is_some()
                || request.turn_id.is_some()
                || request.sequence.is_some()
            {
                return Err(ErrorCode::InvalidRequest);
            }
            let args = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
            if args.len() != 3
                || args["view"] != "worker_status"
                || !args.contains_key("offset")
                || !args.contains_key("digest")
            {
                return Err(ErrorCode::InvalidRequest);
            }
            let offset = args["offset"]
                .as_u64()
                .filter(|n| *n <= 4096)
                .ok_or(ErrorCode::InvalidRequest)? as usize;
            let digest = if args["digest"].is_null() {
                None
            } else {
                Some(
                    args["digest"]
                        .as_str()
                        .filter(|s| {
                            s.len() == 64
                                && s.bytes()
                                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                        })
                        .ok_or(ErrorCode::InvalidRequest)?,
                )
            };
            if offset > 0 && digest.is_none() {
                return Err(ErrorCode::InvalidRequest);
            }
            let project = request
                .project_id
                .as_deref()
                .ok_or(ErrorCode::InvalidRequest)?;
            registry.with_authorized_projects(device, &[project], Operation::WorkspaceList, |_| {
                let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
                if request.session_id.as_deref() != Some(context.session_id.as_str()) { return Err(ErrorCode::OwnerMismatch); }
                let owner = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
                let master = owner.try_lock().map_err(|_| ErrorCode::Busy)?;
                if master.session().session_id() != context.session_id { return Err(ErrorCode::OwnerMismatch); }
                drop(master);
                let mut values = Vec::new();
                for lane in self.lanes.values().filter(|lane| lane.runner.scope().0 == project && lane.runner.scope().1 == context.session_id) {
                    if let Value::Array(statuses) = lane.progress.paged_listing(pool, device.id(), project) { values.extend(statuses); }
                }
                if let Value::Array(statuses) = super::worker_progress::durable_listing(pool, device.id(), project, 4097)? { values.extend(statuses); }
                let mut seen = std::collections::BTreeSet::new();
                values.retain(|value| seen.insert(value["request_id"].as_str().unwrap_or_default().to_owned()));
                if values.len() > 4096 { return Err(ErrorCode::ResourceLimit); }
                values.sort_by_key(|value| (value["runtime_active"] != true, value["request_id"].as_str().unwrap_or_default().to_owned()));
                // Only stable task identity/outcome fields participate; streamed previews never churn page cursors.
                let values = values.into_iter().map(|value| {
                    let mut out = serde_json::Map::new();
                    for field in ["request_id", "project_id", "session_id", "subtab_id", "state", "failure_code", "runtime_active"] {
                        if let Some(value) = value.get(field) { out.insert(field.into(), value.clone()); }
                    }
                    Value::Object(out)
                }).collect::<Vec<_>>();
                use sha2::{Digest, Sha256};
                let actual = format!("{:x}", Sha256::digest(serde_json::to_vec(&values).map_err(|_| ErrorCode::InternalError)?));
                if digest.is_some_and(|expected| expected != actual) { return Err(ErrorCode::RevisionConflict); }
                if offset > values.len() { return Err(ErrorCode::InvalidRequest); }
                let end = (offset + 16).min(values.len());
                Ok(json!({"project_id":project,"session_id":context.session_id,"digest":actual,"offset":offset,"next_offset":end,
                    "total_tasks":values.len(),"has_more":end < values.len(),"tasks":&values[offset..end]}))
            }).map_err(|error| error.remote_code())?
        })();
        Some(match result {
            Ok(result) => {
                let mut response = Envelope::new(
                    super::protocol::Channel::Control,
                    Message::Response { result },
                );
                response.id = request.id.clone();
                response.project_id = request.project_id.clone();
                response.session_id = request.session_id.clone();
                response.execution_state = Some(super::protocol::ExecutionState::Succeeded);
                response
            }
            Err(code) => HostAdapter::failure(request, code),
        })
    }

    /// Live-lane quota writes settle only under the requesting device's
    /// current revoke lock. Ordinary idle writes use the shared subtab path.
    pub fn dispatch_quota(
        &mut self,
        pool: &mut ProjectOwnerPool,
        registry: &DeviceRegistry,
        device: &AuthenticatedDevice,
        request: &Envelope,
        master_busy: bool,
    ) -> Option<Envelope> {
        use super::protocol::{Channel, ExecutionState};
        use sha2::{Digest, Sha256};
        let Message::Request {
            operation: Operation::SetConcurrency,
            arguments,
        } = &request.payload
        else {
            return None;
        };
        let project = request.project_id.as_deref()?;
        let tab = arguments.get("subtab_id").and_then(Value::as_str)?;
        let lane = self.lanes.get_mut(&(project.into(), tab.into()))?;
        let result = (|| {
            if request.validate().is_err()
                || request.turn_id.is_some()
                || request.sequence.is_some()
            {
                return Err(ErrorCode::InvalidRequest);
            }
            let id = request.id.as_deref().ok_or(ErrorCode::InvalidRequest)?;
            let fingerprint = format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(request).map_err(|_| ErrorCode::InternalError)?)
            );
            registry
                .with_authorized_projects(
                    device,
                    &[project],
                    Operation::SetConcurrency,
                    |authorization| {
                        authorization
                            .authorize(project, Operation::Inspect)
                            .map_err(|_| ErrorCode::Unauthorized)?;
                        if let Some(value) =
                            pool.workspace_receipt(device.id(), id, &fingerprint)?
                        {
                            if !value.is_null() {
                                return Ok(value);
                            }
                            // A reservation restored from disk without this original
                            // runtime receipt is never automatically reapplied.
                            if !lane
                                .runner
                                .pending_quota_matches(device.id(), id, &fingerprint)
                            {
                                return Ok(Value::Null);
                            }
                            return Ok(lane
                                .runner
                                .poll_reserved_quota(pool)
                                .ok()
                                .flatten()
                                .unwrap_or(Value::Null));
                        }
                        if master_busy
                            || lane.runner.quota_change_pending()
                            || pool.quota_intent_pending()
                        {
                            return Err(ErrorCode::Busy);
                        }
                        if request.required_owner.as_deref() != Some(pool.workspace_owner()) {
                            return Err(ErrorCode::OwnerMismatch);
                        }
                        if request.expected_revision != Some(pool.workspace_revision()) {
                            return Err(ErrorCode::RevisionConflict);
                        }
                        if request.session_id.as_deref() != Some(lane.runner.scope().1)
                            || pool
                                .context(project)
                                .map(|context| context.session_id.as_str())
                                != request.session_id.as_deref()
                        {
                            return Err(ErrorCode::OwnerMismatch);
                        }
                        let args = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
                        if args.len() != 3
                            || !["subtab_id", "subtab_revision", "concurrency"]
                                .iter()
                                .all(|key| args.contains_key(*key))
                        {
                            return Err(ErrorCode::InvalidRequest);
                        }
                        let revision = args["subtab_revision"]
                            .as_u64()
                            .ok_or(ErrorCode::InvalidRequest)?;
                        let limit = args["concurrency"]
                            .as_u64()
                            .filter(|value| (1..=64).contains(value))
                            .ok_or(ErrorCode::InvalidRequest)?
                            as u16;
                        let before = pool
                            .subtab_state(project)
                            .map_err(|_| ErrorCode::InvalidRequest)?;
                        lane.runner
                            .refresh_unchanged_scope(pool)
                            .map_err(|_| ErrorCode::RevisionConflict)?;
                        if before.revision != revision || lane.runner.scope().3 != revision {
                            return Err(ErrorCode::RevisionConflict);
                        }
                        if limit > lane.runner.quota()
                            && !pool
                                .worker_capacity_available(project, tab, lane.runner.quota(), limit)
                                .map_err(|_| ErrorCode::InternalError)?
                        {
                            return Err(ErrorCode::ResourceLimit);
                        }
                        let mut candidate = before
                            .set_concurrency(tab, limit)
                            .map_err(|_| ErrorCode::InvalidRequest)?;
                        candidate.revision = before
                            .revision
                            .checked_add(1)
                            .ok_or(ErrorCode::ResourceLimit)?;
                        let master = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
                        let guard = master.try_lock().map_err(|_| ErrorCode::Busy)?;
                        if !super::worker::WorkerOwner::idle(&guard) {
                            return Err(ErrorCode::Busy);
                        }
                        let arch = pool.existing_arch_owner(project);
                        let arch_guard = arch
                            .as_ref()
                            .map(|owner| owner.try_lock().map_err(|_| ErrorCode::Busy))
                            .transpose()?;
                        if arch_guard
                            .as_ref()
                            .is_some_and(|agent| !super::worker::WorkerOwner::idle(agent))
                        {
                            return Err(ErrorCode::Busy);
                        }
                        pool.reserve_remote_quota(device.id(), request, &fingerprint, &candidate)
                            .map_err(|_| ErrorCode::InternalError)?;
                        drop(arch_guard);
                        drop(guard);
                        if lane
                            .runner
                            .begin_reserved_quota(pool, device.id(), id, &fingerprint)
                            .is_err()
                        {
                            return Ok(Value::Null);
                        }
                        Ok(lane
                            .runner
                            .poll_reserved_quota(pool)
                            .ok()
                            .flatten()
                            .unwrap_or(Value::Null))
                    },
                )
                .map_err(|error| error.remote_code())?
        })();
        Some(match result {
            Ok(value) => {
                let unknown = value.is_null();
                let mut response =
                    Envelope::new(Channel::Control, Message::Response { result: value });
                response.id = request.id.clone();
                response.project_id = request.project_id.clone();
                response.execution_state = Some(if unknown {
                    ExecutionState::UnknownOutcome
                } else {
                    ExecutionState::Succeeded
                });
                response
            }
            Err(code) => HostAdapter::failure(request, code),
        })
    }

    pub fn is_worker_prompt(request: &Envelope) -> bool {
        matches!(&request.payload, Message::Request { operation: Operation::Prompt, arguments }
            if arguments.get("subtab_id").is_some() || arguments.get("subtab_revision").is_some())
    }

    pub fn busy(&self, project: &str) -> bool {
        self.lanes.keys().any(|(id, _)| id == project)
    }

    pub fn dispatch_prompt(
        &mut self,
        pool: &mut ProjectOwnerPool,
        registry: &DeviceRegistry,
        device: &AuthenticatedDevice,
        request: &Envelope,
        master_busy: bool,
    ) -> Envelope {
        let Message::Request { arguments, .. } = &request.payload else {
            return HostAdapter::failure(request, ErrorCode::InvalidRequest);
        };
        let Some(project) = request.project_id.as_deref() else {
            return HostAdapter::failure(request, ErrorCode::InvalidRequest);
        };
        let Some(tab) = arguments.get("subtab_id").and_then(Value::as_str) else {
            return HostAdapter::failure(request, ErrorCode::InvalidRequest);
        };
        let key = (project.to_owned(), tab.to_owned());
        if let Some(lane) = self.lanes.get_mut(&key) {
            if !lane.runner.quota_change_pending() {
                let _ = lane.runner.refresh_unchanged_scope(pool);
            }
            let may_submit = lane
                .progress
                .matches_scope(lane.runner.scope(), lane.runner.instance())
                && !lane.progress.is_full()
                && !lane.runner.quota_change_pending();
            return super::worker_admission::dispatch_scoped(
                pool,
                registry,
                device,
                request,
                (lane.runner.scope(), lane.runner.configured_quota()),
                may_submit,
                |_, text| {
                    let job = lane
                        .runner
                        .try_submit(WorkerPrompt {
                            text,
                            request: request.clone(),
                            device: device.id().to_owned(),
                        })
                        .map_err(|_| ErrorCode::Busy)?;
                    assert!(
                        lane.progress
                            .bind_scope(lane.runner.scope(), lane.runner.instance())
                    );
                    lane.progress
                        .track(job, request.clone(), device.id().to_owned());
                    Ok(job)
                },
            );
        }
        super::worker_admission::dispatch_managed(
            pool,
            registry,
            device,
            request,
            !master_busy && self.lanes.len() < 64,
            |pool, text| {
                let live = Arc::new(super::live_text::LivePromptText::default());
                let runner = Runner::spawn_observed_prompts(
                    pool,
                    project,
                    tab,
                    RuntimeConfig::default(),
                    live.clone(),
                )
                .map_err(|_| ErrorCode::Busy)?;
                let job = match runner.try_submit(WorkerPrompt {
                    text,
                    request: request.clone(),
                    device: device.id().to_owned(),
                }) {
                    Ok(job) => job,
                    Err(_) => {
                        let _ = runner.shutdown_and_join();
                        return Err(ErrorCode::Busy);
                    }
                };
                let mut progress = WorkerProgress::default();
                assert!(progress.bind_scope(runner.scope(), runner.instance()));
                progress.track(job, request.clone(), device.id().to_owned());
                self.lanes.insert(
                    key,
                    Lane {
                        runner,
                        progress,
                        live,
                    },
                );
                Ok(job)
            },
        )
    }

    /// Local desktop authority targets only live jobs in the selected host subtab.
    /// Admission is not terminal evidence: ordinary polling retains ownership until exit.
    pub fn cancel_local(
        &self,
        pool: &ProjectOwnerPool,
        project: &str,
        subtab: &str,
    ) -> Result<LocalCancelReport, ErrorCode> {
        let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
        let state = pool
            .subtab_state(project)
            .map_err(|_| ErrorCode::InvalidRequest)?;
        if !state.tabs.iter().any(|tab| tab.id == subtab) {
            return Err(ErrorCode::InvalidRequest);
        }
        let mut report = LocalCancelReport::default();
        let Some(lane) = self.lanes.get(&(project.into(), subtab.into())) else {
            return Ok(report);
        };
        let scope = lane.runner.scope();
        if scope.0 != project
            || scope.1 != context.session_id
            || scope.2 != subtab
            || scope.3 > state.revision
            || !lane.progress.matches_scope(scope, lane.runner.instance())
        {
            return Err(ErrorCode::OwnerMismatch);
        }
        for job in lane.progress.active_jobs() {
            match lane.runner.try_cancel(job) {
                Ok(()) => report.requested += 1,
                Err(_) => report.deferred += 1,
            }
        }
        Ok(report)
    }

    pub fn local_status(
        &self,
        pool: &ProjectOwnerPool,
        project: &str,
        subtab: &str,
    ) -> Result<Vec<super::worker_progress::LocalTaskStatus>, ErrorCode> {
        let context = pool.context(project).ok_or(ErrorCode::InvalidRequest)?;
        let tabs = pool
            .subtab_state(project)
            .map_err(|_| ErrorCode::InvalidRequest)?;
        if !tabs.tabs.iter().any(|tab| tab.id == subtab) {
            return Err(ErrorCode::InvalidRequest);
        }
        let mut values = Vec::new();
        if let Some(lane) = self.lanes.get(&(project.into(), subtab.into())) {
            if lane.runner.scope().1 != context.session_id
                || !lane
                    .progress
                    .matches_scope(lane.runner.scope(), lane.runner.instance())
            {
                return Err(ErrorCode::OwnerMismatch);
            }
            values = lane
                .progress
                .local_status(project, &context.session_id, subtab);
        }
        // Do not invent an empty/successful durable view when the master is contended.
        let durable = super::worker_progress::local_durable_status(pool, project, subtab)?;
        let mut seen = values
            .iter()
            .map(|value| (value.device.clone(), value.request.clone()))
            .collect::<std::collections::BTreeSet<_>>();
        values.extend(
            durable
                .into_iter()
                .filter(|value| seen.insert((value.device.clone(), value.request.clone()))),
        );
        values.truncate(super::worker_progress::LOCAL_STATUS_LIMIT);
        Ok(values)
    }

    pub fn cancel(
        &self,
        pool: &mut ProjectOwnerPool,
        registry: &DeviceRegistry,
        device: &AuthenticatedDevice,
        request: &Envelope,
    ) -> Option<Envelope> {
        let lane = self
            .lanes
            .values()
            .find(|lane| lane.progress.has_target(request, Some(device.id())))
            .or_else(|| {
                self.lanes
                    .values()
                    .find(|lane| lane.progress.has_target(request, None))
            })?;
        Some(
            lane.progress
                .cancel(pool, registry, device, request, &lane.runner),
        )
    }

    pub fn listing(&self, pool: &ProjectOwnerPool, device: &str, project: &str) -> Value {
        let mut values = Vec::new();
        for lane in self
            .lanes
            .values()
            .filter(|lane| lane.runner.scope().0 == project)
        {
            if let Value::Array(live) = lane.progress.live_listing(pool, device, project) {
                values.extend(live.into_iter().map(|mut value| {
                    if matches!(value["state"].as_str(), Some("accepted" | "running"))
                        && let (Some(session), Some(id)) =
                            (value["session_id"].as_str(), value["request_id"].as_str())
                        && let Some(snapshot) = lane.live.snapshot(device, project, session, id)
                    {
                        value["stream_text"] = json!(snapshot.text);
                        if snapshot.truncated {
                            value["stream_truncated"] = json!(true);
                        }
                    }
                    value
                }));
            }
        }
        if let Value::Array(durable) = super::worker_progress::listing(pool, device, project) {
            values.extend(durable);
        }
        let mut seen = std::collections::BTreeSet::new();
        values.retain(|value| {
            seen.insert(value["request_id"].as_str().unwrap_or_default().to_owned())
        });
        // A terminal entry in an earlier lane must not hide another lane's live task.
        values.sort_by_key(|value| value["runtime_active"] != true);
        values.truncate(16);
        json!(values)
    }

    pub fn poll(&mut self, pool: &mut ProjectOwnerPool) {
        let mut finished = Vec::new();
        for (key, lane) in &mut self.lanes {
            while let Ok(event) = lane.runner.try_next_event() {
                if let crate::runtime::RuntimeEvent::Completed { id, .. }
                | crate::runtime::RuntimeEvent::Rejected { id, .. } = &event
                    && let Some((request, device)) = lane.progress.identity(*id)
                {
                    lane.live.remove(request, device);
                }
                lane.progress.observe(event, false);
            }
            lane.progress.flush(pool);
            if lane.progress.finished() && !lane.runner.quota_change_pending() {
                finished.push(key.clone());
            }
        }
        for key in finished {
            if let Some(lane) = self.lanes.remove(&key) {
                let _ = lane.runner.shutdown_and_join();
            }
            if !self.busy(&key.0) {
                let _ = pool.retire_worker_owners(&key.0);
            }
        }
    }

    pub fn request_shutdown(&self, deadline: Instant) {
        for lane in self.lanes.values() {
            let _ = lane
                .runner
                .try_shutdown_with_grace(deadline.saturating_duration_since(Instant::now()));
        }
    }

    pub fn shutdown(self, pool: &ProjectOwnerPool, deadline: Instant) {
        self.request_shutdown(deadline);
        let mut pending = Vec::new();
        for lane in self.lanes.into_values() {
            let mut progress = lane.progress;
            let _ = lane.runner.shutdown_and_join_with_events(
                deadline.saturating_duration_since(Instant::now()),
                |event| progress.observe(event, true),
            );
            progress.finish_shutdown();
            pending.push(progress);
        }
        let persist_deadline = Instant::now() + Duration::from_millis(250);
        loop {
            for progress in &mut pending {
                progress.flush(pool);
            }
            if pending.iter().all(WorkerProgress::finished) || Instant::now() >= persist_deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
