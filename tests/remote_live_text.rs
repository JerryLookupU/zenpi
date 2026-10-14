use zenpi::backend::ProviderEvent;
use zenpi::remote::{
    live_text::{LivePromptText, MAX_LIVE_TEXT_BYTES},
    protocol::{Channel, Envelope, Message, Operation},
};

fn request() -> Envelope {
    let mut request = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::Prompt,
            arguments: serde_json::json!({"text":"prompt"}),
        },
    );
    request.id = Some("request".into());
    request.project_id = Some("project".into());
    request.session_id = Some("session".into());
    request
}

#[test]
fn private_pem_is_withheld_at_every_stream_split_and_text_done() {
    let input = "before\n-----BEGIN OPENSSH PRIVATE KEY-----\nPRIVATE_BODY_7Xq9\n-----END OPENSSH PRIVATE KEY-----\nafter\n";
    for split in 0..=input.len() {
        let texts = LivePromptText::default();
        let request = request();
        for delta in [&input[..split], &input[split..]] {
            texts.update(
                &request,
                "device",
                &ProviderEvent::TextDelta {
                    delta: delta.into(),
                },
            );
            if let Some(text) = texts.projected("device", "project", "session", "request") {
                assert!(!text.contains("PRIVATE_BODY"), "split {split}: {text}");
            }
        }
        texts.update(
            &request,
            "device",
            &ProviderEvent::TextDone { text: input.into() },
        );
        let text = texts
            .projected("device", "project", "session", "request")
            .unwrap();
        assert!(!text.contains("PRIVATE_BODY"));
        assert!(text.starts_with("before\n"));
        assert!(text.ends_with("after\n"));
    }
}

#[test]
fn desktop_private_pem_without_end_never_exposes_body() {
    use zenpi::remote::live_text::LiveDesktopText;
    let text = LiveDesktopText::default();
    let guard = text.begin("project", "session");
    for delta in [
        "safe\n-----BEGIN EC PRIVATE KEY-----\n",
        "PRIVATE_BODY_FIRST\n",
        "PRIVATE_BODY_SECOND\n",
    ] {
        guard.update(&ProviderEvent::TextDelta {
            delta: delta.into(),
        });
        let value = text.projected("project", "session").unwrap();
        assert!(!value.contains("PRIVATE_BODY"));
    }
    guard.update(&ProviderEvent::TextDone {
        text: "-----BEGIN PRIVATE KEY-----\nPRIVATE_BODY_NO_NEWLINE".into(),
    });
    assert!(
        !text
            .projected("project", "session")
            .unwrap()
            .contains("PRIVATE_BODY")
    );
    drop(guard);
    assert!(text.projected("project", "session").is_none());
}

#[test]
fn complete_lines_are_redacted_across_chunks_and_private_events_ignored() {
    let texts = LivePromptText::default();
    let request = request();
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDelta {
            delta: "api_".into(),
        },
    );
    assert!(
        texts
            .projected("device", "project", "session", "request")
            .is_none()
    );
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDelta {
            delta: "key=STREAM_SECRET\nvisible\nunfinished".into(),
        },
    );
    let text = texts
        .projected("device", "project", "session", "request")
        .unwrap();
    assert!(!text.contains("STREAM_SECRET"));
    assert!(!text.contains("unfinished"));
    assert!(text.contains("visible"));
    texts.update(
        &request,
        "device",
        &ProviderEvent::ReasoningDelta {
            delta: "PRIVATE_REASONING\n".into(),
        },
    );
    texts.update(
        &request,
        "device",
        &ProviderEvent::ToolCallDelta {
            call_id: None,
            name: None,
            arguments_delta: "PRIVATE_ARGUMENTS\n".into(),
        },
    );
    assert_eq!(
        texts
            .projected("device", "project", "session", "request")
            .unwrap(),
        text
    );
    for (device, project, session, id) in [
        ("other", "project", "session", "request"),
        ("device", "other", "session", "request"),
        ("device", "project", "other", "request"),
        ("device", "project", "session", "other"),
    ] {
        assert!(texts.projected(device, project, session, id).is_none());
    }
}

#[test]
fn text_done_replaces_deltas_overflow_retains_safe_preview_and_terminal_removes() {
    let texts = LivePromptText::default();
    let request = request();
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDelta {
            delta: "old\n".into(),
        },
    );
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDone {
            text: "final".into(),
        },
    );
    assert_eq!(
        texts
            .projected("device", "project", "session", "request")
            .as_deref(),
        Some("final")
    );
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDelta {
            delta: "x".repeat(MAX_LIVE_TEXT_BYTES),
        },
    );
    let snapshot = texts
        .snapshot("device", "project", "session", "request")
        .unwrap();
    assert_eq!(snapshot.text, "final");
    assert!(snapshot.truncated);
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDone {
            text: "too late".into(),
        },
    );
    assert_eq!(
        texts
            .projected("device", "project", "session", "request")
            .as_deref(),
        Some("final")
    );
    texts.remove(&request, "device");
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDone {
            text: "fresh".into(),
        },
    );
    assert_eq!(
        texts
            .projected("device", "project", "session", "request")
            .as_deref(),
        Some("fresh")
    );
}

#[test]
fn exact_text_cap_and_slot_cap_do_not_evict_other_active_prompts() {
    let texts = LivePromptText::default();
    let limit = zenpi::project_workspace::MAX_PROJECT_TABS;
    for index in 0..=limit {
        let mut request = request();
        request.id = Some(format!("request-{index}"));
        texts.update(
            &request,
            "device",
            &ProviderEvent::TextDone {
                text: "x".repeat(MAX_LIVE_TEXT_BYTES),
            },
        );
    }
    for index in 0..limit {
        assert_eq!(
            texts
                .projected("device", "project", "session", &format!("request-{index}"))
                .unwrap()
                .len(),
            MAX_LIVE_TEXT_BYTES
        );
    }
    assert!(
        texts
            .projected("device", "project", "session", &format!("request-{limit}"))
            .is_none()
    );
    let mut request = request();
    request.id = Some("request-0".into());
    texts.remove(&request, "device");
    request.id = Some(format!("request-{limit}"));
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDone {
            text: "admitted after cleanup".into(),
        },
    );
    assert!(
        texts
            .projected("device", "project", "session", &format!("request-{limit}"))
            .is_some()
    );
}

#[test]
fn overflow_never_promotes_an_unfinished_secret_line_and_response_reset_recovers() {
    let texts = LivePromptText::default();
    let request = request();
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDelta {
            delta: "visible\napi_".into(),
        },
    );
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDelta {
            delta: format!("key=PRIVATE_OVERFLOW{}\n", "x".repeat(MAX_LIVE_TEXT_BYTES)),
        },
    );
    let snapshot = texts
        .snapshot("device", "project", "session", "request")
        .unwrap();
    assert_eq!(snapshot.text, "visible\n");
    assert!(snapshot.truncated);
    texts.update(
        &request,
        "device",
        &ProviderEvent::ResponseCreated {
            response_id: None,
            model: None,
        },
    );
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDone {
            text: "next response".into(),
        },
    );
    let snapshot = texts
        .snapshot("device", "project", "session", "request")
        .unwrap();
    assert_eq!(snapshot.text, "next response");
    assert!(!snapshot.truncated);
}

#[test]
fn oversized_first_chunk_reports_truncation_without_copying_or_slicing_it() {
    let texts = LivePromptText::default();
    let request = request();
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDone {
            text: "PRIVATE_OVERFLOW".repeat(4096),
        },
    );
    let snapshot = texts
        .snapshot("device", "project", "session", "request")
        .unwrap();
    assert!(snapshot.text.is_empty());
    assert!(snapshot.truncated);
    texts.remove(&request, "device");
    assert!(
        texts
            .snapshot("device", "project", "session", "request")
            .is_none()
    );
}

#[test]
fn multibyte_overflow_keeps_complete_utf8_and_desktop_does_not_claim_full_preview() {
    let texts = LivePromptText::default();
    let request = request();
    let prefix = format!("{}\n", "\u{4e2d}".repeat(1365));
    assert_eq!(prefix.len(), MAX_LIVE_TEXT_BYTES);
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDelta {
            delta: prefix.clone(),
        },
    );
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDelta {
            delta: "\u{4e2d}".into(),
        },
    );
    let snapshot = texts
        .snapshot("device", "project", "session", "request")
        .unwrap();
    assert_eq!(snapshot.text, prefix);
    assert!(snapshot.truncated);
    let desktop = zenpi::remote::live_text::LiveDesktopText::default();
    let guard = desktop.begin("project", "session");
    guard.update(&ProviderEvent::TextDone {
        text: "safe".into(),
    });
    assert_eq!(
        desktop.projected("project", "session").as_deref(),
        Some("safe")
    );
    guard.update(&ProviderEvent::TextDelta {
        delta: "x".repeat(MAX_LIVE_TEXT_BYTES),
    });
    assert!(desktop.projected("project", "session").is_none());
}

#[test]
fn subsequent_provider_response_resets_old_text_and_nul_withholds() {
    let texts = LivePromptText::default();
    let request = request();
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDone { text: "old".into() },
    );
    texts.update(
        &request,
        "device",
        &ProviderEvent::ResponseCreated {
            response_id: None,
            model: None,
        },
    );
    assert!(
        texts
            .projected("device", "project", "session", "request")
            .is_none()
    );
    texts.update(
        &request,
        "device",
        &ProviderEvent::TextDone {
            text: "new\0unsafe".into(),
        },
    );
    assert!(
        texts
            .projected("device", "project", "session", "request")
            .is_none()
    );
}

#[test]
fn desktop_preview_is_session_scoped_and_guard_clears_on_return_and_unwind() {
    use zenpi::remote::live_text::LiveDesktopText;
    let text = LiveDesktopText::default();
    {
        let guard = text.begin("project", "session");
        guard.update(&ProviderEvent::TextDelta {
            delta: "LOCAL_DESKTOP_PARTIAL\n".into(),
        });
        assert_eq!(
            text.projected("project", "session").as_deref(),
            Some("LOCAL_DESKTOP_PARTIAL\n")
        );
        assert!(text.projected("other", "session").is_none());
        assert!(text.projected("project", "other").is_none());
    }
    assert!(text.projected("project", "session").is_none());
    let _ = std::panic::catch_unwind(|| {
        let guard = text.begin("project", "session");
        guard.update(&ProviderEvent::TextDone {
            text: "PANIC_PARTIAL".into(),
        });
        panic!("fixture unwind");
    });
    assert!(text.projected("project", "session").is_none());
}

#[test]
fn deltas_publish_each_whole_token_and_hold_back_the_unfinished_one() {
    let texts = LivePromptText::default();
    let request = request();
    let mut published = Vec::new();
    for delta in [
        "Hel",
        "lo wor",
        "ld, api_",
        "key=STREAM",
        "_SECRET and",
        " 中文",
        "\n",
    ] {
        texts.update(
            &request,
            "device",
            &ProviderEvent::TextDelta {
                delta: delta.into(),
            },
        );
        published.push(texts.projected("device", "project", "session", "request"));
    }
    let expected = [
        None,
        Some("Hello "),
        Some("Hello world, "),
        Some("Hello world, "),
        Some("Hello world, api_key=<redacted> "),
        Some("Hello world, api_key=<redacted> and "),
        Some("Hello world, api_key=<redacted> and 中文\n"),
    ];
    assert_eq!(published, expected.map(|value| value.map(str::to_owned)));
}
