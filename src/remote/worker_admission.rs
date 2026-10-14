//! Durable request admission before work enters a worktree runner.

use super::{
    auth::{AuthenticatedDevice, DeviceRegistry},
    host_adapter::HostAdapter,
    protocol::{Envelope, ErrorCode, ExecutionState, Message, Operation},
    worktree_runner::WorktreeRunner,
};
use crate::{
    core::{AgentError, ProcessResult},
    project_workspace::ProjectOwnerPool,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const RECEIPT: &str = "remote_worker_admission_v1";

pub fn dispatch(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    runner: &WorktreeRunner<String, ProcessResult, AgentError>,
) -> Envelope {
    dispatch_observed(
        pool,
        registry,
        device,
        request,
        Some((runner.scope(), runner.configured_quota())),
        !runner.quota_change_pending(),
        |_, text| runner.try_submit(text).map_err(|_| ErrorCode::Busy),
    )
}

pub fn dispatch_tracked(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    runner: &WorktreeRunner<String, ProcessResult, AgentError>,
    progress: &mut super::worker_progress::WorkerProgress,
) -> Envelope {
    let may_submit = progress.matches_scope(runner.scope(), runner.instance())
        && !progress.is_full()
        && !runner.quota_change_pending();
    dispatch_observed(
        pool,
        registry,
        device,
        request,
        Some((runner.scope(), runner.configured_quota())),
        may_submit,
        |_, text| {
            let job = runner.try_submit(text).map_err(|_| ErrorCode::Busy)?;
            assert!(progress.bind_scope(runner.scope(), runner.instance()));
            progress.track(job, request.clone(), device.id().to_owned());
            Ok(job)
        },
    )
}

pub(crate) fn dispatch_managed(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    may_submit: bool,
    submit: impl FnOnce(&mut ProjectOwnerPool, String) -> Result<crate::runtime::JobId, ErrorCode>,
) -> Envelope {
    dispatch_observed(pool, registry, device, request, None, may_submit, submit)
}

pub(crate) fn dispatch_scoped(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    scope: ((&str, &str, &str, u64), u16),
    may_submit: bool,
    submit: impl FnOnce(&mut ProjectOwnerPool, String) -> Result<crate::runtime::JobId, ErrorCode>,
) -> Envelope {
    dispatch_observed(
        pool,
        registry,
        device,
        request,
        Some(scope),
        may_submit,
        submit,
    )
}

fn dispatch_observed(
    pool: &mut ProjectOwnerPool,
    registry: &DeviceRegistry,
    device: &AuthenticatedDevice,
    request: &Envelope,
    runner: Option<((&str, &str, &str, u64), u16)>,
    may_submit: bool,
    submit: impl FnOnce(&mut ProjectOwnerPool, String) -> Result<crate::runtime::JobId, ErrorCode>,
) -> Envelope {
    let result = (|| {
        if request.validate().is_err() || request.turn_id.is_some() || request.sequence.is_some() {
            return Err(ErrorCode::InvalidRequest);
        }
        let Message::Request {
            operation: Operation::Prompt,
            arguments,
        } = &request.payload
        else {
            return Err(ErrorCode::InvalidRequest);
        };
        let args = arguments.as_object().ok_or(ErrorCode::InvalidRequest)?;
        let text = args
            .get("text")
            .and_then(Value::as_str)
            .ok_or(ErrorCode::InvalidRequest)?;
        let subtab = args
            .get("subtab_id")
            .and_then(Value::as_str)
            .ok_or(ErrorCode::InvalidRequest)?;
        let revision = args
            .get("subtab_revision")
            .and_then(Value::as_u64)
            .ok_or(ErrorCode::InvalidRequest)?;
        if args.len() != 3
            || text.trim().is_empty()
            || text.len() > 64 * 1024
            || text.contains('\0')
            || text.trim_start().starts_with('!')
        {
            return Err(ErrorCode::InvalidRequest);
        }
        super::command::validate_remote_file_references(text)
            .map_err(|_| ErrorCode::InvalidRequest)?;
        let project = request
            .project_id
            .as_deref()
            .ok_or(ErrorCode::InvalidRequest)?;
        let id = request.id.as_deref().ok_or(ErrorCode::InvalidRequest)?;
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(request).map_err(|_| ErrorCode::InternalError)?)
        );
        registry
            .with_authorized_projects(device, &[project], Operation::Prompt, |authorization| {
                authorization
                    .authorize(project, Operation::Inspect)
                    .map_err(|_| ErrorCode::Unauthorized)?;
                let bound = pool.prompt_binding(device.id(), id, &fingerprint)?;
                let master = pool.owner(project).ok_or(ErrorCode::InvalidRequest)?;
                let mut agent = match master.try_lock() {
                    Ok(agent) => agent,
                    Err(_) => return Ok((ExecutionState::UnknownOutcome, Value::Null, None)),
                };
                if request.session_id.as_deref() != Some(agent.session().session_id()) {
                    return Err(ErrorCode::OwnerMismatch);
                }
                let receipts: Vec<_> = agent
                    .session()
                    .events()
                    .iter()
                    .filter(|event| event["type"] == RECEIPT)
                    .collect();
                if let Some(event) = receipts
                    .iter()
                    .rev()
                    .find(|event| event["device_id"] == device.id() && event["request_id"] == id)
                {
                    if event["fingerprint"] != fingerprint {
                        return Err(ErrorCode::RequestIdConflict);
                    }
                    return Ok(match event["state"].as_str() {
                        Some("accepted") => {
                            (ExecutionState::Accepted, event["result"].clone(), None)
                        }
                        Some("failed") => {
                            (ExecutionState::Failed, Value::Null, Some(ErrorCode::Busy))
                        }
                        _ => (ExecutionState::UnknownOutcome, Value::Null, None),
                    });
                }
                if bound {
                    return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
                }
                if !may_submit {
                    return Err(ErrorCode::ResourceLimit);
                }
                if receipts.len() >= 256 {
                    return Err(ErrorCode::ResourceLimit);
                }
                if request.required_owner.as_deref() != Some(pool.workspace_owner()) {
                    return Err(ErrorCode::OwnerMismatch);
                }
                if request.expected_revision != Some(pool.workspace_revision()) {
                    return Err(ErrorCode::RevisionConflict);
                }
                let state = pool
                    .subtab_state(project)
                    .map_err(|_| ErrorCode::InvalidRequest)?;
                if state.revision != revision {
                    return Err(ErrorCode::RevisionConflict);
                }
                if let Some((scope, quota)) = runner {
                    let (runner_project, runner_session, runner_tab, runner_revision) = scope;
                    if runner_project != project
                        || Some(runner_session) != request.session_id.as_deref()
                        || runner_tab != subtab
                    {
                        return Err(ErrorCode::OwnerMismatch);
                    }
                    if runner_revision != revision
                        || !state
                            .tabs
                            .iter()
                            .any(|tab| tab.id == subtab && tab.concurrency == quota)
                    {
                        return Err(ErrorCode::RevisionConflict);
                    }
                } else if !state.tabs.iter().any(|tab| tab.id == subtab) {
                    return Err(ErrorCode::InvalidRequest);
                }
                if !super::worker::WorkerOwner::idle(&agent) {
                    return Err(ErrorCode::Busy);
                }
                // Checkpoint binding precedes journal reservation and any runtime submission.
                if pool
                    .bind_remote_prompt(device.id(), request, &fingerprint)
                    .is_err()
                {
                    return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
                }
                let event = |state: &str, result: Value| {
                    json!({"type":RECEIPT, "device_id":device.id(),
                "request_id":id,"fingerprint":fingerprint,"state":state,"result":result,
                "project_id":project,"session_id":request.session_id,"subtab_id":subtab})
                };
                if agent
                    .session_mut()
                    .append_event(event("reserved", Value::Null))
                    .is_err()
                {
                    return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
                }
                // A lazy host runner must prepare independent owners without
                // holding their master's execution mutex. Reservation is durable.
                drop(agent);
                let (state, value, code) = match submit(pool, text.to_owned()) {
                    Ok(job) => (
                        ExecutionState::Accepted,
                        json!({"request_id":id,"job_id":job.get().to_string(),
                    "project_id":project,"session_id":request.session_id,"subtab_id":subtab}),
                        None,
                    ),
                    Err(_) => (ExecutionState::Failed, Value::Null, Some(ErrorCode::Busy)),
                };
                let mut agent = match master.try_lock() {
                    Ok(agent)
                        if request.session_id.as_deref() == Some(agent.session().session_id()) =>
                    {
                        agent
                    }
                    _ => return Ok((ExecutionState::UnknownOutcome, Value::Null, None)),
                };
                if agent
                    .session_mut()
                    .append_event(event(
                        if state == ExecutionState::Accepted {
                            "accepted"
                        } else {
                            "failed"
                        },
                        value.clone(),
                    ))
                    .is_err()
                {
                    return Ok((ExecutionState::UnknownOutcome, Value::Null, None));
                }
                Ok((state, value, code))
            })
            .map_err(|error| error.remote_code())?
    })();
    match result {
        Err(code) => HostAdapter::failure(request, code),
        Ok((state, result, code)) => {
            let mut response = HostAdapter::failure(request, ErrorCode::InternalError);
            response.session_id = request.session_id.clone();
            response.execution_state = Some(state);
            response.code = code;
            response.payload = Message::Response { result };
            response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::Agent, runtime::RuntimeConfig, session::SessionStore};
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
    use std::sync::{Arc, Mutex};

    #[test]
    fn checkpoint_binding_without_journal_admission_never_submits_even_on_same_host() {
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
                1,
                vec![
                    super::super::auth::Capability::Read,
                    super::super::auth::Capability::Control,
                ],
                vec![project.clone()],
            )
            .unwrap();
        let mut challenge = super::super::auth::DeviceChallenge::new("a".repeat(64)).unwrap();
        let device = challenge
            .authenticate(
                &registry,
                &record.id,
                &URL_SAFE_NO_PAD.encode(
                    key.sign(&random, &challenge.proof_message(&record.id))
                        .unwrap()
                        .as_ref(),
                ),
            )
            .unwrap();
        let runner = WorktreeRunner::spawn_prompts(
            &mut pool,
            &project,
            &state.active_id,
            RuntimeConfig::default(),
        )
        .unwrap();
        let mut request = Envelope::new(
            super::super::protocol::Channel::Control,
            Message::Request {
                operation: Operation::Prompt,
                arguments: json!({"text":"never-resubmit", "subtab_id":state.active_id, "subtab_revision":state.revision}),
            },
        );
        request.id = Some("checkpoint-only".into());
        request.project_id = Some(project);
        request.session_id = Some(pool.active_context().session_id.clone());
        request.required_owner = Some(pool.workspace_owner().into());
        request.expected_revision = Some(pool.workspace_revision());
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&request).unwrap())
        );
        pool.bind_remote_prompt(device.id(), &request, &fingerprint)
            .unwrap();
        let path = master.lock().unwrap().session().path().to_owned();
        let before = std::fs::read(path.clone()).unwrap();
        assert_eq!(
            dispatch_managed(&mut pool, &registry, &device, &request, true, |_, _| {
                panic!("checkpoint-only retry must not construct a worker runner")
            })
            .execution_state,
            Some(ExecutionState::UnknownOutcome)
        );
        assert_eq!(
            dispatch(&mut pool, &registry, &device, &request, &runner).execution_state,
            Some(ExecutionState::UnknownOutcome)
        );
        assert!(
            runner
                .recv_timeout(std::time::Duration::from_millis(30))
                .is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), before);
        runner.shutdown_and_join().unwrap();
    }
}
