use serde_json::json;
use std::io;
use zenpi::{
    core::{Turn, TurnRole},
    remote::{
        events,
        host_adapter::HostAdapter,
        protocol::{Channel, Envelope, ErrorCode, EventCursor, ExecutionState, Message},
        subscription,
    },
    session::SessionStore,
};

fn request() -> Envelope {
    let mut request = Envelope::new(
        Channel::Control,
        Message::Subscribe {
            cursor: EventCursor {
                session_id: "session".into(),
                next_sequence: 0,
            },
            limit: 128,
            duration_ms: 1000,
        },
    );
    request.id = Some("subscription".into());
    request.project_id = Some("project".into());
    request.session_id = Some("session".into());
    request
}

#[test]
fn switch_during_metadata_sampling_rechecks_before_returning_refresh() {
    let mut samples = 0;
    let mut dispatches = 0;
    let mut output = Vec::new();
    subscription::serve_with_view(
        &request(),
        |read| {
            dispatches += 1;
            HostAdapter::failure(&read, ErrorCode::Busy)
        },
        || {
            samples += 1;
            Err(if samples == 1 {
                ErrorCode::OwnerMismatch
            } else {
                ErrorCode::SessionChanged
            })
        },
        |frame| {
            output.push(frame);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(samples, 2);
    assert_eq!(dispatches, 0);
    assert_eq!(output[0].code, Some(ErrorCode::SessionChanged));
}

#[test]
fn switch_between_sample_and_replay_rechecks_listing_before_refresh_code() {
    for changed in [true, false] {
        let mut samples = 0;
        let mut output = Vec::new();
        subscription::serve_with_view(
            &request(),
            |resume| HostAdapter::failure(&resume, ErrorCode::OwnerMismatch),
            || {
                samples += 1;
                if samples == 2 && changed {
                    Err(ErrorCode::SessionChanged)
                } else {
                    Ok(None)
                }
            },
            |frame| {
                output.push(frame);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(samples, 2);
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0].code,
            Some(if changed {
                ErrorCode::SessionChanged
            } else {
                ErrorCode::OwnerMismatch
            })
        );
    }
}

#[test]
fn changed_authorized_session_has_distinct_code_but_missing_project_is_unauthorized() {
    use zenpi::remote::protocol::Operation;
    for (project, session, expected) in [
        ("project", "new", ErrorCode::SessionChanged),
        ("foreign", "new", ErrorCode::Unauthorized),
    ] {
        let mut operations = Vec::new();
        let view = subscription::view(&request(), |read| {
            let Message::Request { operation, .. } = read.payload else {
                panic!("request")
            };
            operations.push(operation);
            let mut response = Envelope::new(
                Channel::Control,
                Message::Response {
                    result: json!({"projects":[{"id":project,"session_id":session}]}),
                },
            );
            response.id = read.id;
            response.execution_state = Some(ExecutionState::Succeeded);
            response
        });
        assert_eq!(operations, [Operation::WorkspaceList]);
        assert_eq!(view, Err(expected));
    }
}

#[test]
fn owner_race_recheck_preserves_current_authority_loss_without_replay_or_data() {
    for during_sample in [true, false] {
        for code in [ErrorCode::Unauthorized, ErrorCode::RevokedDevice] {
            let mut samples = 0;
            let mut dispatches = 0;
            let mut output = Vec::new();
            subscription::serve_with_view(
                &request(),
                |read| {
                    dispatches += 1;
                    HostAdapter::failure(&read, ErrorCode::OwnerMismatch)
                },
                || {
                    samples += 1;
                    if samples == 1 {
                        if during_sample {
                            Err(ErrorCode::OwnerMismatch)
                        } else {
                            Ok(Some(json!({"private_preview":"must-not-emit"})))
                        }
                    } else {
                        Err(code)
                    }
                },
                |frame| {
                    output.push(frame);
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(samples, 2);
            assert_eq!(dispatches, usize::from(!during_sample));
            assert_eq!(output.len(), 1);
            assert_eq!(output[0].id.as_deref(), Some("subscription"));
            assert_eq!(output[0].execution_state, Some(ExecutionState::Failed));
            assert_eq!(output[0].code, Some(code));
            assert_eq!(
                output[0].payload,
                Message::Response {
                    result: json!(null)
                }
            );
        }
    }
}

#[test]
fn inconclusive_owner_race_recheck_never_invents_session_change_or_authority() {
    for during_sample in [true, false] {
        for rechecked in [
            Ok(None),
            Ok(Some(json!({"private_preview":"must-not-emit"}))),
            Err(ErrorCode::Busy),
            Err(ErrorCode::InternalError),
            Err(ErrorCode::OwnerMismatch),
        ] {
            let mut samples = 0;
            let mut output = Vec::new();
            subscription::serve_with_view(
                &request(),
                |read| HostAdapter::failure(&read, ErrorCode::OwnerMismatch),
                || {
                    samples += 1;
                    if samples == 1 {
                        if during_sample {
                            Err(ErrorCode::OwnerMismatch)
                        } else {
                            Ok(None)
                        }
                    } else {
                        rechecked.clone()
                    }
                },
                |frame| {
                    output.push(frame);
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(samples, 2);
            assert_eq!(output.len(), 1);
            assert_eq!(output[0].code, Some(ErrorCode::OwnerMismatch));
            assert_eq!(
                output[0].payload,
                Message::Response {
                    result: json!(null)
                }
            );
        }
    }
}

#[test]
fn subscription_bounds_and_scope_are_strict_on_wire() {
    let mut valid = request();
    assert_eq!(
        Envelope::decode(&serde_json::to_vec(&valid).unwrap()).unwrap(),
        valid
    );
    for duration_ms in [0, 999, 5001, u16::MAX] {
        valid.payload = Message::Subscribe {
            cursor: EventCursor {
                session_id: "session".into(),
                next_sequence: 0,
            },
            limit: 128,
            duration_ms,
        };
        assert!(valid.validate().is_err());
    }
    for field in ["id", "project_id", "session_id"] {
        let mut value = serde_json::to_value(request()).unwrap();
        value.as_object_mut().unwrap().remove(field);
        assert!(Envelope::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}

#[test]
fn durable_new_turns_push_over_one_request_and_cursor_is_contiguous() {
    let root = tempfile::tempdir().unwrap();
    let mut session = SessionStore::open(root.path().join("session.jsonl")).unwrap();
    let mut request = request();
    request.session_id = Some(session.session_id().into());
    if let Message::Subscribe { cursor, .. } = &mut request.payload {
        cursor.session_id = session.session_id().into();
    }
    let mut polls = 0;
    let mut output = Vec::new();
    subscription::serve(&request, |resume| {
        polls += 1;
        if polls == 2 { session.append_turn(Turn::new("new-turn", TurnRole::Assistant, "actual durable answer")).unwrap(); }
        let Message::Resume { cursor, limit } = resume.payload else { panic!("resume"); };
        let page = events::replay(&session, "project", &cursor, limit).unwrap();
        let mut response = Envelope::new(Channel::Control, Message::Response { result: json!({
            "cursor":page.cursor, "cursor_scope":"transcript_events_v1", "has_more":page.has_more,
            "snapshot_required":page.snapshot_required, "phase":"idle", "journal_next_sequence":session.next_sequence(), "wire_frames":page.frames
        }) });
        response.id = resume.id; response.execution_state = Some(ExecutionState::Succeeded); response
    }, |frame| { frame.validate().unwrap(); output.push(frame); Ok(()) }).unwrap();
    let events: Vec<_> = output
        .iter()
        .filter(|f| matches!(f.payload, Message::Event { .. }))
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence, Some(0));
    assert_eq!(events[0].turn_id.as_deref(), Some("new-turn"));
    let Message::Response { result } = &output.last().unwrap().payload else {
        panic!("final");
    };
    assert_eq!(result["subscription_done"], true);
    assert_eq!(result["cursor"]["next_sequence"], 1);
    assert!(polls >= 2);
    assert!(polls <= 32);
}

#[test]
fn refusal_and_broken_output_stop_without_retrying_or_acknowledging() {
    let mut dispatches = 0;
    let mut output = Vec::new();
    subscription::serve(
        &request(),
        |resume| {
            dispatches += 1;
            HostAdapter::failure(&resume, ErrorCode::Unauthorized)
        },
        |frame| {
            output.push(frame);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(dispatches, 1);
    assert_eq!(output[0].code, Some(ErrorCode::Unauthorized));
    let error = subscription::serve(
        &request(),
        |resume| HostAdapter::failure(&resume, ErrorCode::Busy),
        |_| {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "fixture disconnect",
            ))
        },
    )
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
}

#[test]
fn bounded_page_finishes_at_gap_or_tail_and_overflow_refuses_before_emitting_data() {
    for (snapshot, has_more, count) in [(true, false, 0), (false, true, 1), (false, false, 129)] {
        let mut output = Vec::new();
        let mut calls = 0;
        subscription::serve(&request(), |resume| {
            calls += 1;
            let frames: Vec<_> = (0..count).map(|index| {
                let mut frame = Envelope::new(Channel::Event, Message::Event { name: "transcript_turn".into(), data: json!({}) });
                frame.sequence = Some(index); frame.session_id = Some("session".into()); frame
            }).collect();
            let mut response = Envelope::new(Channel::Control, Message::Response { result: json!({
                "cursor":{"session_id":"session","next_sequence":count}, "wire_frames":frames,
                "snapshot_required":snapshot, "has_more":has_more
            }) });
            response.id = resume.id; response.execution_state = Some(ExecutionState::Succeeded); response
        }, |frame| { output.push(frame); Ok(()) }).unwrap();
        assert_eq!(calls, 1);
        if count > 128 {
            assert_eq!(output.len(), 1);
            assert_eq!(output[0].code, Some(ErrorCode::ResourceLimit));
        } else {
            let Message::Response { result } = &output.last().unwrap().payload else {
                panic!("response");
            };
            assert_eq!(result["subscription_done"], true);
        }
    }
}

#[test]
fn busy_owner_still_streams_changed_view_without_advancing_or_acknowledging_cursor() {
    let mut samples = 0;
    let mut output = Vec::new();
    subscription::serve_with_view(
        &request(),
        |resume| HostAdapter::failure(&resume, ErrorCode::Busy),
        || {
            samples += 1;
            Ok(Some(
                json!({"preview":if samples == 1 { "first" } else { "second" }}),
            ))
        },
        |frame| {
            frame.validate().unwrap();
            output.push(frame);
            Ok(())
        },
    )
    .unwrap();
    let views: Vec<_> = output
        .iter()
        .filter_map(|frame| {
            let Message::Response { result } = &frame.payload else {
                panic!("batch");
            };
            assert_eq!(result["subscription_transcript"], false);
            assert_eq!(result["cursor"]["next_sequence"], 0);
            assert!(!result.to_string().contains("journal_next_sequence"));
            result.get("subscription_view").cloned()
        })
        .collect();
    assert_eq!(
        views,
        vec![json!({"preview":"first"}), json!({"preview":"second"})]
    );
    let Message::Response { result } = &output.last().unwrap().payload else {
        panic!("completion");
    };
    assert_eq!(result["subscription_done"], true);
}

#[test]
fn failed_view_and_excessive_transient_bytes_never_emit_successful_display() {
    for sample in [
        Err(ErrorCode::Unauthorized),
        Ok(Some(json!({"text":"x".repeat(196_608)}))),
    ] {
        let mut output = Vec::new();
        subscription::serve_with_view(
            &request(),
            |resume| HostAdapter::failure(&resume, ErrorCode::Busy),
            || sample.clone(),
            |frame| {
                output.push(frame);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].execution_state, Some(ExecutionState::Failed));
        assert!(matches!(
            output[0].code,
            Some(ErrorCode::Unauthorized | ErrorCode::ResourceLimit)
        ));
    }
}

#[test]
fn actual_envelope_byte_boundary_accepts_exact_limit_and_refuses_one_more_before_events() {
    fn response(resume: Envelope, padding: usize) -> Envelope {
        let mut event = Envelope::new(
            Channel::Event,
            Message::Event {
                name: "transcript_turn".into(),
                data: json!({"text":"x".repeat(padding)}),
            },
        );
        event.project_id = resume.project_id.clone();
        event.session_id = resume.session_id.clone();
        event.sequence = Some(0);
        let mut response = Envelope::new(
            Channel::Control,
            Message::Response {
                result: json!({
                    "cursor":{"session_id":"session","next_sequence":1}, "wire_frames":[event],
                    "snapshot_required":false, "has_more":true
                }),
            },
        );
        response.id = resume.id;
        response.execution_state = Some(ExecutionState::Succeeded);
        response
    }
    let mut unpadded = response(request(), 0);
    let Message::Response { result } = &mut unpadded.payload else {
        panic!("response")
    };
    result["subscription_transcript"] = json!(true);
    result["subscription_done"] = json!(true);
    let overhead: usize = HostAdapter::wire_frames(unpadded)
        .unwrap()
        .iter()
        .map(|frame| serde_json::to_vec(frame).unwrap().len())
        .sum();
    for extra in [0, 1] {
        let mut output = Vec::new();
        subscription::serve(
            &request(),
            |resume| response(resume, 196_608 - overhead + extra),
            |frame| {
                output.push(frame);
                Ok(())
            },
        )
        .unwrap();
        let actual: usize = output
            .iter()
            .map(|frame| serde_json::to_vec(frame).unwrap().len())
            .sum();
        if extra == 0 {
            assert_eq!(actual, 196_608);
            assert_eq!(output.len(), 2);
            assert_eq!(
                output.last().unwrap().execution_state,
                Some(ExecutionState::Succeeded)
            );
        } else {
            assert_eq!(output.len(), 1);
            assert_eq!(output[0].code, Some(ErrorCode::ResourceLimit));
            assert!(actual < 196_608);
        }
    }
}

#[test]
fn nonterminal_page_reserves_correlated_failure_inside_android_wire_budget() {
    let mut samples = 0;
    let mut output = Vec::new();
    subscription::serve_with_view(
        &request(),
        |resume| HostAdapter::failure(&resume, ErrorCode::Busy),
        || {
            samples += 1;
            if samples == 1 {
                Ok(Some(json!({"text":"x".repeat(196_000)})))
            } else {
                Err(ErrorCode::RevokedDevice)
            }
        },
        |frame| {
            output.push(frame);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(output.len(), 2);
    assert_eq!(output[0].execution_state, Some(ExecutionState::Succeeded));
    assert_eq!(output[1].code, Some(ErrorCode::RevokedDevice));
    let size: usize = output
        .iter()
        .map(|frame| serde_json::to_vec(frame).unwrap().len())
        .sum();
    assert!(size <= 196_608, "{size}");
}

#[test]
fn nonterminal_exact_remaining_budget_reserves_longest_failure_code() {
    let mut initial = HostAdapter::failure(&request(), ErrorCode::Busy);
    initial.execution_state = Some(ExecutionState::Succeeded);
    initial.code = None;
    initial.payload = Message::Response {
        result: json!({
            "cursor":{"session_id":"session","next_sequence":0}, "cursor_scope":"transcript_events_v1",
            "snapshot_required":false, "has_more":false, "subscription_transcript":false,
            "subscription_view":{"text":""}, "subscription_done":false
        }),
    };
    let overhead = serde_json::to_vec(&initial).unwrap().len();
    let refusal = HostAdapter::failure(&request(), ErrorCode::UnsupportedVersion);
    let reserved = serde_json::to_vec(&refusal).unwrap().len();
    for extra in [0, 1] {
        let mut samples = 0;
        let mut output = Vec::new();
        subscription::serve_with_view(
            &request(),
            |resume| HostAdapter::failure(&resume, ErrorCode::Busy),
            || {
                samples += 1;
                if samples == 1 {
                    Ok(Some(
                        json!({"text":"x".repeat(196_608 - overhead - reserved + extra)}),
                    ))
                } else {
                    Err(ErrorCode::UnsupportedVersion)
                }
            },
            |frame| {
                output.push(frame);
                Ok(())
            },
        )
        .unwrap();
        let size: usize = output
            .iter()
            .map(|frame| serde_json::to_vec(frame).unwrap().len())
            .sum();
        if extra == 0 {
            assert_eq!(samples, 2);
            assert_eq!(output.len(), 2);
            assert_eq!(size, 196_608);
            assert_eq!(output[1].code, Some(ErrorCode::UnsupportedVersion));
        } else {
            assert_eq!(samples, 1);
            assert_eq!(output.len(), 1);
            assert_eq!(output[0].code, Some(ErrorCode::ResourceLimit));
            assert!(size < 196_608);
        }
    }
}

#[test]
fn view_requires_same_session_and_matching_workspace_revision_and_owner() {
    let listing = json!({"projects":[{"id":"project","session_id":"session"}],"required_owner":"owner","workspace_revision":1});
    let tabs = json!({"required_owner":"owner","workspace_revision":1});
    for (listed, tabbed, expected) in [
        (listing.clone(), tabs.clone(), None),
        (
            json!({"projects":[]}),
            tabs.clone(),
            Some(ErrorCode::Unauthorized),
        ),
        (
            json!({"projects":[{"id":"project","session_id":"foreign"}]}),
            tabs.clone(),
            Some(ErrorCode::SessionChanged),
        ),
        (
            listing.clone(),
            json!({"required_owner":"owner","workspace_revision":2}),
            Some(ErrorCode::Busy),
        ),
        (
            listing.clone(),
            json!({"required_owner":"foreign","workspace_revision":1}),
            Some(ErrorCode::Busy),
        ),
    ] {
        let result = subscription::view(&request(), |read| {
            let Message::Request { operation, .. } = read.payload else {
                panic!("read-only");
            };
            let mut response = Envelope::new(
                Channel::Control,
                Message::Response {
                    result: if operation == zenpi::remote::protocol::Operation::WorkspaceList {
                        listed.clone()
                    } else if operation == zenpi::remote::protocol::Operation::ModelStatus {
                        json!({"project_id":"project","session_id":"session"})
                    } else {
                        tabbed.clone()
                    },
                },
            );
            response.id = read.id;
            response.execution_state = Some(ExecutionState::Succeeded);
            response
        });
        match expected {
            Some(code) => assert_eq!(result, Err(code)),
            None => assert!(result.unwrap().is_some()),
        }
    }
    let mut invalid = request();
    invalid.session_id = None;
    assert_eq!(
        subscription::view(&invalid, |_| panic!("invalid request must not dispatch")),
        Err(ErrorCode::InvalidRequest)
    );
}

#[test]
fn committed_session_and_provider_changes_wake_only_their_own_session() {
    let root = tempfile::tempdir().unwrap();
    let mut session = SessionStore::open(root.path().join("journal.jsonl")).unwrap();
    let signal = zenpi::remote::wakeup::watch(session.session_id()).unwrap();
    let foreign = zenpi::remote::wakeup::watch("foreign-wakeup-test").unwrap();
    let observed = signal.revision();
    let turn = Turn::new("new", TurnRole::Assistant, "durable");
    session.append_turn(turn.clone()).unwrap();
    assert!(signal.wait(observed, std::time::Duration::ZERO));
    assert_eq!(foreign.revision(), 0);
    let observed = signal.revision();
    session.append_turn(turn).unwrap();
    assert!(!signal.wait(observed, std::time::Duration::ZERO));
    assert!(
        session
            .append_turn(Turn::new("new", TurnRole::Assistant, "conflict"))
            .is_err()
    );
    assert!(!signal.wait(observed, std::time::Duration::ZERO));
    session
        .append_event(json!({"type":"private", "token":"never-in-notification"}))
        .unwrap();
    assert!(signal.wait(observed, std::time::Duration::ZERO));
    let mut request = request();
    request.session_id = Some(session.session_id().into());
    let progress = zenpi::remote::command::PromptProgress::default();
    let observed = signal.revision();
    progress.record(&request, "device", ExecutionState::Running);
    assert!(signal.wait(observed, std::time::Duration::ZERO));
    let observed = signal.revision();
    progress.provider_event(
        &request,
        "device",
        &zenpi::backend::ProviderEvent::TextDelta {
            delta: "line\n".into(),
        },
    );
    assert!(signal.wait(observed, std::time::Duration::ZERO));
    let observed = signal.revision();
    progress.provider_event(
        &request,
        "device",
        &zenpi::backend::ProviderEvent::TextDelta {
            delta: "x".repeat(4096),
        },
    );
    assert!(signal.wait(observed, std::time::Duration::ZERO));
    let observed = signal.revision();
    progress.provider_event(
        &request,
        "device",
        &zenpi::backend::ProviderEvent::TextDelta {
            delta: "ignored after truncation".into(),
        },
    );
    assert!(!signal.wait(observed, std::time::Duration::ZERO));
    let observed = signal.revision();
    progress.record(&request, "device", ExecutionState::Succeeded);
    assert!(signal.wait(observed, std::time::Duration::ZERO));
}

#[test]
fn view_model_busy_is_transient_but_acl_and_scope_errors_refuse_the_entire_view() {
    use zenpi::remote::protocol::Operation;
    for (code, foreign) in [
        (Some(ErrorCode::Busy), false),
        (Some(ErrorCode::Unauthorized), false),
        (Some(ErrorCode::ResourceLimit), false),
        (None, true),
        (None, false),
    ] {
        let mut operations = Vec::new();
        let view = subscription::view(&request(), |read| {
            let Message::Request {
                operation,
                arguments,
            } = &read.payload
            else {
                panic!("read");
            };
            operations.push(*operation);
            assert_eq!(arguments, &json!({}));
            if *operation == Operation::ModelStatus {
                assert_eq!(read.project_id.as_deref(), Some("project"));
                assert_eq!(read.session_id.as_deref(), Some("session"));
                assert!(read.required_owner.is_none());
                if let Some(code) = code {
                    return HostAdapter::failure(&read, code);
                }
            }
            let result = match operation {
                Operation::WorkspaceList => {
                    json!({"projects":[{"id":"project","session_id":"session"}],"required_owner":"owner","workspace_revision":1})
                }
                Operation::SubtabList => json!({"required_owner":"owner","workspace_revision":1}),
                Operation::ModelStatus => {
                    json!({"project_id":"project","session_id":if foreign {"foreign"}else{"session"},"selection_revision":7})
                }
                _ => panic!("subscription must dispatch read operations only"),
            };
            let mut response = Envelope::new(Channel::Control, Message::Response { result });
            response.id = read.id;
            response.execution_state = Some(ExecutionState::Succeeded);
            response
        });
        assert_eq!(
            operations,
            [
                Operation::WorkspaceList,
                Operation::SubtabList,
                Operation::ModelStatus
            ]
        );
        if foreign {
            assert_eq!(view, Err(ErrorCode::OwnerMismatch));
        } else if code == Some(ErrorCode::Busy) {
            assert!(view.unwrap().unwrap()["model_status"].is_null());
        } else if let Some(code) = code {
            assert_eq!(view, Err(code));
        } else {
            assert_eq!(
                view.unwrap().unwrap()["model_status"]["selection_revision"],
                7
            );
        }
    }
}

#[test]
fn continuous_notifications_end_at_client_batch_cap_instead_of_overflowing() {
    let request = request();
    let signal = zenpi::remote::wakeup::watch("session").unwrap();
    let mut output = Vec::new();
    subscription::serve(&request, |resume| {
        zenpi::remote::wakeup::notify("session");
        let mut response = Envelope::new(Channel::Control, Message::Response { result: json!({
            "cursor":{"session_id":"session","next_sequence":0}, "wire_frames":[], "snapshot_required":false,"has_more":false
        }) });
        response.id = resume.id; response.execution_state = Some(ExecutionState::Succeeded); response
    }, |frame| { output.push(frame); Ok(()) }).unwrap();
    assert_eq!(output.len(), 32);
    assert!(signal.revision() >= 32);
    let Message::Response { result } = &output.last().unwrap().payload else {
        panic!("batch");
    };
    assert_eq!(result["subscription_done"], true);
    assert_eq!(result["cursor"]["next_sequence"], 0);
}

#[test]
fn failed_journal_append_never_signals_a_durable_change() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("exhausted.jsonl");
    let session = SessionStore::open(&path).unwrap();
    let id = session.session_id().to_owned();
    drop(session);
    let mut header: serde_json::Value = serde_json::from_str(
        std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    header["seq"] = json!(u64::MAX - 1);
    std::fs::write(&path, format!("{header}\n")).unwrap();
    let mut session = SessionStore::open(&path).unwrap();
    let signal = zenpi::remote::wakeup::watch(&id).unwrap();
    let observed = signal.revision();
    let before = std::fs::read(&path).unwrap();
    assert!(
        session
            .append_event(json!({"type":"must-not-commit"}))
            .is_err()
    );
    assert!(
        session
            .append_turn(Turn::new("refused", TurnRole::Assistant, "never committed"))
            .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(!signal.wait(observed, std::time::Duration::ZERO));
}

/// Android's reducer rule: keep `stream_offset` scalar values of the held
/// text, then append `stream_delta`. Returns each applied (offset, held length).
fn apply_live_deltas(held: &mut Option<String>, frames: &[Envelope]) -> Vec<(usize, usize)> {
    let mut applied = Vec::new();
    for frame in frames {
        let Message::Response { result } = &frame.payload else {
            continue;
        };
        let Some(view) = result.get("subscription_view") else {
            continue;
        };
        let project = &view["listing"]["projects"][0];
        assert!(project["desktop_stream_text"].is_null());
        assert!(project.get("desktop_stream_offset").is_none());
        let status = &project["prompt_statuses"][0];
        assert!(status.get("stream_text").is_none(), "{status}");
        let Some(offset) = status["stream_offset"].as_u64() else {
            *held = None;
            continue;
        };
        let offset = usize::try_from(offset).unwrap();
        let base = held.take().unwrap_or_default();
        let length = base.chars().count();
        assert!(offset <= length, "gap: {offset} > {base:?}");
        applied.push((offset, length));
        *held = Some(
            base.chars().take(offset).collect::<String>()
                + status["stream_delta"].as_str().unwrap(),
        );
    }
    applied
}

#[test]
fn live_text_streams_as_ordered_token_deltas_and_replay_neither_repeats_nor_skips() {
    use zenpi::{backend::ProviderEvent, remote::live_text::LivePromptText};
    let session = "live-text-delta-session";
    let subscribe = |duration_ms| {
        let mut value = request();
        value.session_id = Some(session.into());
        value.payload = Message::Subscribe {
            cursor: EventCursor {
                session_id: session.into(),
                next_sequence: 0,
            },
            limit: 128,
            duration_ms,
        };
        value
    };
    let mut prompt = subscribe(0);
    prompt.id = Some("prompt".into());
    let texts = LivePromptText::default();
    let view = || {
        let mut status = json!({"request_id":"prompt","state":"running"});
        if let Some(text) = texts.projected("device", "project", session, "prompt") {
            status["stream_text"] = json!(text);
        }
        json!({"listing":{"projects":[{"id":"project","desktop_stream_text":null,"prompt_statuses":[status]}]}})
    };
    let serve = |request: &Envelope, mut feed: Box<dyn FnMut() + '_>| {
        let mut frames = Vec::new();
        subscription::serve_with_view(
            request,
            |resume| HostAdapter::failure(&resume, ErrorCode::Busy),
            || {
                feed();
                Ok(Some(view()))
            },
            |frame| {
                frames.push(frame);
                Ok(())
            },
        )
        .unwrap();
        frames
    };
    let tokens = [
        "Stream",
        "ing per",
        " token",
        " api_key=SEC",
        "RET ok",
        " 中文 ",
        "end\n",
    ];
    let mut fed = 0;
    let frames = serve(
        &subscribe(1000),
        Box::new(|| {
            let event = match tokens.get(fed) {
                Some(token) => ProviderEvent::TextDelta {
                    delta: (*token).into(),
                },
                None if fed == tokens.len() => ProviderEvent::TextDone {
                    text: "Streaming per token, final".into(),
                },
                None => return,
            };
            texts.update(&prompt, "device", &event);
            fed += 1;
        }),
    );
    let mut held = None;
    let applied = apply_live_deltas(&mut held, &frames);
    // Every new token boundary arrives once and in order as a pure append;
    // only the final text, which differs after "token", rewrites a suffix.
    // The first token has no boundary yet, so it publishes nothing.
    let (last, appended) = applied.split_last().unwrap();
    assert_eq!(appended.len(), tokens.len() - 1);
    assert!(appended.iter().all(|(offset, length)| offset == length));
    let streamed = "Streaming per token api_key=<redacted> ok 中文 end\n";
    assert_eq!(
        *last,
        ("Streaming per token".len(), streamed.chars().count())
    );
    assert_eq!(held.as_deref(), Some("Streaming per token, final"));
    let wire = serde_json::to_string(&frames).unwrap();
    assert!(!wire.contains("SEC\"") && !wire.contains("SECRET"));
    assert_eq!(wire.matches("Stream").count(), 1);
    assert!(wire.contains("api_key=<redacted> "));

    // A reconnect replays from offset 0: the held copy is replaced, not doubled.
    let frames = serve(&subscribe(1000), Box::new(|| {}));
    assert_eq!(apply_live_deltas(&mut held, &frames), [(0, 26)]);
    assert_eq!(held.as_deref(), Some("Streaming per token, final"));
    // A finished stream disappears from the view and is forgotten on both sides.
    texts.remove(&prompt, "device");
    let frames = serve(&subscribe(1000), Box::new(|| {}));
    assert!(apply_live_deltas(&mut held, &frames).is_empty());
    assert_eq!(held, None);
}
