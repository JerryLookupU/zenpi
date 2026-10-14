use serde_json::json;
use zenpi::remote::protocol::{
    Channel, Envelope, ErrorCode, EventCursor, ExecutionState, MAX_CONTROL_BYTES, MAX_ID_BYTES,
    MAX_JSON_DEPTH, MAX_JSON_NODES, MAX_VERSIONS, Message, Operation, ProtocolError,
    SCHEMA_VERSION, negotiate_version,
};

fn request(operation: Operation) -> Envelope {
    let mut envelope = Envelope::new(
        Channel::Control,
        Message::Request {
            operation,
            arguments: json!({}),
        },
    );
    envelope.id = Some("request-1".into());
    envelope
}

#[test]
fn negotiation_is_explicit_and_bounded() {
    assert_eq!(
        negotiate_version(&[99, SCHEMA_VERSION]).unwrap(),
        SCHEMA_VERSION
    );
    assert!(matches!(
        negotiate_version(&[99]),
        Err(ProtocolError::NoCommonVersion)
    ));
    assert!(negotiate_version(&[]).is_err());
    assert!(negotiate_version(&[1; MAX_VERSIONS + 1]).is_err());
    let mut hello = Envelope::new(
        Channel::Control,
        Message::Hello {
            supported_versions: vec![SCHEMA_VERSION],
        },
    );
    assert!(hello.validate().is_ok());
    hello.schema_version = 99;
    assert!(matches!(
        hello.validate(),
        Err(ProtocolError::UnsupportedVersion(99))
    ));
}

#[test]
fn connection_handshakes_refuse_all_routing_and_explicit_null_authority() {
    let public = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        [vec![4], vec![0; 64]].concat(),
    );
    let payloads = [
        json!({"type":"hello","supported_versions":[1]}),
        json!({"type":"welcome","selected_version":1}),
        json!({"type":"challenge","nonce":"A".repeat(43),"host_fingerprint":"a".repeat(64)}),
        json!({"type":"pair","token":"A".repeat(43),"fingerprint":"a".repeat(64),
            "safety_code":"123456","device_name":"phone","public_key":public,"signature":"proof"}),
        json!({"type":"authenticate","device_id":"a".repeat(64),"signature":"proof"}),
        json!({"type":"revoke_status","device_id":"a".repeat(64),"request_id":"original","signature":"proof"}),
    ];
    for (index, payload) in payloads.into_iter().enumerate() {
        let mut valid = json!({"schema_version":1,"channel":"control","payload":payload});
        if index >= 3 {
            valid["id"] = json!("handshake");
        }
        let frame = Envelope::decode(&serde_json::to_vec(&valid).unwrap()).unwrap();
        for field in [
            "project_id",
            "session_id",
            "turn_id",
            "sequence",
            "required_owner",
            "expected_revision",
            "code",
            "execution_state",
            "id",
        ] {
            if field == "id" && index >= 3 {
                continue;
            }
            for value in [
                serde_json::Value::Null,
                match field {
                    "sequence" | "expected_revision" => json!(0),
                    "code" => json!("unauthorized"),
                    "execution_state" => json!("succeeded"),
                    _ => json!("foreign"),
                },
            ] {
                let mut malformed = valid.clone();
                malformed[field] = value;
                assert!(
                    Envelope::decode(&serde_json::to_vec(&malformed).unwrap()).is_err(),
                    "{index}:{field}"
                );
            }
        }
        for field in ["project", "session", "turn"] {
            let mut routed = frame.clone();
            match field {
                "project" => routed.project_id = Some("p".into()),
                "session" => routed.session_id = Some("s".into()),
                _ => routed.turn_id = Some("t".into()),
            }
            assert!(routed.validate().is_err());
            let mut output = Vec::new();
            assert!(zenpi::remote::frame::write_frame(&mut output, &routed).is_err());
            assert!(output.is_empty());
        }
    }
}

#[test]
fn challenge_nonce_requires_canonical_encoding_of_exactly_thirty_two_bytes() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    for last in 0..=255u8 {
        let mut bytes = [0u8; 32];
        bytes[31] = last;
        let canonical = URL_SAFE_NO_PAD.encode(bytes);
        let mut frame = Envelope::new(
            Channel::Control,
            Message::Challenge {
                nonce: canonical.clone(),
                host_fingerprint: "a".repeat(64),
            },
        );
        assert!(frame.validate().is_ok());
        assert_eq!(
            Envelope::decode(&serde_json::to_vec(&frame).unwrap()).unwrap(),
            frame
        );
        let index = alphabet
            .iter()
            .position(|byte| *byte == canonical.as_bytes()[42])
            .unwrap();
        for padding in 1..=3 {
            let mut noncanonical = canonical.as_bytes().to_vec();
            noncanonical[42] = alphabet[index + padding];
            if let Message::Challenge { nonce, .. } = &mut frame.payload {
                *nonce = String::from_utf8(noncanonical).unwrap();
            }
            assert!(frame.validate().is_err());
            assert!(Envelope::decode(&serde_json::to_vec(&frame).unwrap()).is_err());
        }
    }
}

#[test]
fn writes_require_request_identity_owner_and_revision() {
    for operation in [
        Operation::WorkspaceOpen,
        Operation::WorkspaceSelect,
        Operation::WorkspaceClose,
        Operation::SubtabAdd,
        Operation::SubtabSelect,
        Operation::SubtabMove,
        Operation::SubtabRename,
        Operation::SubtabClose,
        Operation::SetConcurrency,
        Operation::ModelSelect,
        Operation::ReasoningSelect,
        Operation::ResourcesReload,
        Operation::TerminalOpen,
        Operation::TerminalInput,
        Operation::TerminalResize,
        Operation::SessionNew,
        Operation::SessionResume,
        Operation::SessionRename,
        Operation::Prompt,
        Operation::Steer,
        Operation::Approve,
        Operation::Cancel,
    ] {
        let mut envelope = request(operation);
        assert!(envelope.validate().is_err(), "{operation:?}");
        envelope.required_owner = Some("owner-epoch-1".into());
        assert!(envelope.validate().is_err());
        envelope.expected_revision = Some(0);
        assert!(envelope.validate().is_ok());
        envelope.id = None;
        assert!(envelope.validate().is_err());
    }
    for operation in [
        Operation::Inspect,
        Operation::TranscriptRead,
        Operation::TerminalRead,
        Operation::SessionList,
        Operation::FilesList,
        Operation::ResourcesList,
        Operation::ModelStatus,
        Operation::WorkspaceList,
        Operation::SubtabList,
    ] {
        assert!(request(operation).validate().is_ok());
    }
}

#[test]
fn strict_decode_preserves_correlation_and_refuses_unknown_fields() {
    let mut envelope = request(Operation::Prompt);
    envelope.required_owner = Some("owner-1".into());
    envelope.expected_revision = Some(4);
    envelope.session_id = Some("session-1".into());
    envelope.project_id = Some("project-1".into());
    envelope.turn_id = Some("turn-1".into());
    let bytes = serde_json::to_vec(&envelope).unwrap();
    assert_eq!(Envelope::decode(&bytes).unwrap(), envelope);
    let mut value = serde_json::to_value(&envelope).unwrap();
    value["is_admin"] = json!(true);
    assert!(Envelope::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    value.as_object_mut().unwrap().remove("is_admin");
    value["payload"]["is_write"] = json!(false);
    assert!(Envelope::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    value["payload"].as_object_mut().unwrap().remove("is_write");
    value["payload"]["operation"] = json!("shell");
    assert!(Envelope::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(Envelope::decode(br#"{"schema_version":1}"#).is_err());
    assert!(Envelope::decode(&[0xff]).is_err());
    assert!(Envelope::decode(br#"{"schema_version":1,"schema_version":1}"#).is_err());
}

#[test]
fn channels_and_host_only_fields_are_not_client_authority() {
    let mut envelope = request(Operation::Inspect);
    envelope.channel = Channel::Bulk;
    assert!(envelope.validate().is_err());
    envelope.channel = Channel::Control;
    envelope.execution_state = Some(ExecutionState::Succeeded);
    assert!(envelope.validate().is_err());
    envelope.execution_state = None;
    envelope.code = Some(ErrorCode::Unauthorized);
    assert!(envelope.validate().is_err());
    envelope.code = None;
    envelope.sequence = Some(0);
    assert!(envelope.validate().is_err());
}

#[test]
fn responses_preserve_unknown_outcome_instead_of_false_success() {
    let mut envelope = Envelope::new(Channel::Control, Message::Response { result: json!({}) });
    assert!(envelope.validate().is_err());
    envelope.id = Some("request-1".into());
    assert!(envelope.validate().is_err());
    envelope.execution_state = Some(ExecutionState::UnknownOutcome);
    assert_eq!(
        Envelope::decode(&serde_json::to_vec(&envelope).unwrap()).unwrap(),
        envelope
    );
    envelope.execution_state = Some(ExecutionState::Failed);
    assert!(envelope.validate().is_err());
    envelope.code = Some(ErrorCode::RevisionConflict);
    assert!(envelope.validate().is_ok());
    envelope.required_owner = Some("client-supplied".into());
    assert!(envelope.validate().is_err());
}

#[test]
fn events_and_replay_use_a_session_bound_transport_cursor() {
    let cursor = EventCursor {
        session_id: "s1".into(),
        next_sequence: 8,
    };
    for payload in [
        Message::Ack {
            cursor: cursor.clone(),
        },
        Message::Resume {
            cursor: cursor.clone(),
            limit: 128,
        },
        Message::Snapshot {
            cursor: cursor.clone(),
            state: json!({}),
        },
    ] {
        let channel = if matches!(payload, Message::Snapshot { .. }) {
            Channel::Bulk
        } else {
            Channel::Control
        };
        let mut envelope = Envelope::new(channel, payload);
        assert!(envelope.validate().is_err());
        envelope.session_id = Some("s1".into());
        assert!(envelope.validate().is_ok());
        envelope.session_id = Some("s2".into());
        assert!(envelope.validate().is_err());
    }
    for limit in [0, 129, u16::MAX] {
        let mut envelope = Envelope::new(
            Channel::Control,
            Message::Resume {
                cursor: cursor.clone(),
                limit,
            },
        );
        envelope.session_id = Some("s1".into());
        assert!(envelope.validate().is_err());
    }
    let mut event = Envelope::new(
        Channel::Event,
        Message::Event {
            name: "turn_started".into(),
            data: json!({}),
        },
    );
    event.session_id = Some("s1".into());
    assert!(event.validate().is_err());
    event.sequence = Some(0);
    assert!(event.validate().is_ok());
}

#[test]
fn identifiers_json_complexity_and_channel_bytes_are_bounded() {
    let mut envelope = request(Operation::Inspect);
    for bad in [String::new(), "x".repeat(MAX_ID_BYTES + 1), "x\ny".into()] {
        envelope.id = Some(bad);
        assert!(envelope.validate().is_err());
    }
    envelope.id = Some("x".repeat(MAX_ID_BYTES));
    assert!(envelope.validate().is_ok());
    if let Message::Request { arguments, .. } = &mut envelope.payload {
        *arguments = json!({"items": vec![0; MAX_JSON_NODES]});
    }
    assert!(envelope.validate().is_err());
    let mut nested = json!(null);
    for _ in 0..MAX_JSON_DEPTH + 1 {
        nested = json!([nested]);
    }
    if let Message::Request { arguments, .. } = &mut envelope.payload {
        *arguments = json!({"nested": nested});
    }
    assert!(envelope.validate().is_err());
    if let Message::Request { arguments, .. } = &mut envelope.payload {
        *arguments = json!({"text": "x".repeat(MAX_CONTROL_BYTES)});
    }
    assert!(Envelope::decode(&serde_json::to_vec(&envelope).unwrap()).is_err());
}

#[test]
fn local_jsonl_compatibility_is_unchanged() {
    assert_eq!(zenpi::protocol::LEGACY_PROTOCOL_VERSION, 1);
    assert_eq!(zenpi::protocol::PROTOCOL_VERSION, 2);
    let legacy: zenpi::protocol::StdioRequest =
        serde_json::from_value(json!({"type": "status"})).unwrap();
    assert_eq!(legacy.schema_version, 1);
    assert!(Envelope::decode(br#"{"type":"status"}"#).is_err());
}

#[test]
fn duplicate_remote_members_are_refused_at_every_json_scope() {
    for payload in [
        r#"{"type":"request","operation":"inspect","arguments":{"path":"one","path":"two"}}"#,
        r#"{"type":"request","operation":"inspect","arguments":{"nested":[{"path":"one","pa\u0074h":"two"}]}}"#,
        r#"{"type":"response","result":{"access":false,"access":true}}"#,
        r#"{"type":"event","name":"turn","data":{"text":"one","text":"two"}}"#,
        r#"{"type":"snapshot","cursor":{"session_id":"s","next_sequence":0},"state":{"nested":{"value":1,"value":2}}}"#,
    ] {
        let bytes =
            format!(r#"{{"schema_version":1,"channel":"control","id":"r","payload":{payload}}}"#);
        let error = Envelope::decode(bytes.as_bytes()).unwrap_err().to_string();
        assert!(error.contains("duplicate remote JSON member"), "{error}");
    }
    for bytes in [
        r#"{"schema_version":1,"schema_version":1,"channel":"control","payload":{"type":"hello","supported_versions":[1]}}"#,
        r#"{"schema_version":1,"channel":"control","payload":{"type":"hello","type":"hello","supported_versions":[1]}}"#,
    ] {
        assert!(Envelope::decode(bytes.as_bytes()).is_err());
    }
}

#[test]
fn repeated_names_in_distinct_objects_remain_valid() {
    let bytes = br#"{"schema_version":1,"channel":"control","id":"r","payload":{"type":"request","operation":"inspect","arguments":{"items":[{"name":"one"},{"name":"two"}],"name":"root"}}}"#;
    assert!(Envelope::decode(bytes).is_ok());
}

#[test]
fn preflight_prevents_dynamic_payload_last_member_wins() {
    let bytes = br#"{"schema_version":1,"channel":"control","id":"r","payload":{"type":"request","operation":"inspect","arguments":{"limit":1,"limit":128}}}"#;
    let lossy: Envelope = serde_json::from_slice(bytes).unwrap();
    assert!(lossy.validate().is_ok());
    let Message::Request { arguments, .. } = lossy.payload else {
        panic!("request")
    };
    assert_eq!(arguments["limit"], 128);
    assert!(Envelope::decode(bytes).is_err());
}

#[test]
fn shared_android_rust_raw_wire_corpus_has_identical_acceptance() {
    let corpus = include_str!("../fuzz/remote_protocol/corpus.jsonl");
    let mut names = std::collections::BTreeSet::new();
    let mut accepted = 0;
    let mut refused = 0;
    for line in corpus.lines() {
        let case: serde_json::Value = serde_json::from_str(line).unwrap();
        let name = case["name"].as_str().unwrap();
        assert!(names.insert(name.to_owned()));
        let expected = case["accept"].as_bool().unwrap();
        let wire = case["wire"].as_str().unwrap().as_bytes();
        let decoded = Envelope::decode(wire);
        assert_eq!(decoded.is_ok(), expected, "{name}: {decoded:?}");
        if let Ok(envelope) = decoded {
            accepted += 1;
            let mut framed = Vec::new();
            zenpi::remote::frame::write_frame(&mut framed, &envelope).unwrap();
            assert_eq!(
                zenpi::remote::frame::read_frame(&mut std::io::Cursor::new(framed)).unwrap(),
                Some(envelope)
            );
        } else {
            refused += 1;
        }
    }
    assert!(accepted >= 10 && refused >= 25);
}

#[test]
fn shared_schema_contract_exercises_every_operation_and_outcome() {
    let contract: serde_json::Value =
        serde_json::from_str(include_str!("../fuzz/remote_protocol/schema-contract.json")).unwrap();
    assert_eq!(contract["schema_version"], SCHEMA_VERSION);
    let mut names = std::collections::BTreeSet::new();
    for (field, write) in [("read_operations", false), ("write_operations", true)] {
        for value in contract[field].as_array().unwrap() {
            let name = value.as_str().unwrap();
            assert!(names.insert(name));
            let operation: Operation = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(operation.is_write(), write, "{name}");
            let mut frame = json!({"schema_version":1,"channel":"control","id":"matrix",
                "payload":{"type":"request","operation":name,"arguments":{}}});
            assert_eq!(
                Envelope::decode(&serde_json::to_vec(&frame).unwrap()).is_ok(),
                !write,
                "{name}"
            );
            frame["required_owner"] = json!("owner");
            assert_eq!(
                Envelope::decode(&serde_json::to_vec(&frame).unwrap()).is_ok(),
                !write,
                "{name} owner only"
            );
            frame["expected_revision"] = json!(u64::MAX);
            let decoded = Envelope::decode(&serde_json::to_vec(&frame).unwrap()).unwrap();
            assert_eq!(decoded.expected_revision, Some(u64::MAX));
            frame.as_object_mut().unwrap().remove("required_owner");
            assert_eq!(
                Envelope::decode(&serde_json::to_vec(&frame).unwrap()).is_ok(),
                !write,
                "{name} revision only"
            );
            frame["required_owner"] = json!("owner");
            for channel in ["event", "bulk"] {
                frame["channel"] = json!(channel);
                assert!(
                    Envelope::decode(&serde_json::to_vec(&frame).unwrap()).is_err(),
                    "{name} {channel}"
                );
            }
        }
    }
    assert_eq!(names.len(), 33);
    let mut states = std::collections::BTreeSet::new();
    for state in contract["execution_states"].as_array().unwrap() {
        assert!(states.insert(state.as_str().unwrap()));
        let mut frame = json!({"schema_version":1,"channel":"control","id":"matrix",
            "execution_state":state,"payload":{"type":"response","result":{}}});
        assert_eq!(
            Envelope::decode(&serde_json::to_vec(&frame).unwrap()).is_ok(),
            state != "failed"
        );
        if state == "failed" {
            let mut codes = std::collections::BTreeSet::new();
            for code in contract["error_codes"].as_array().unwrap() {
                assert!(codes.insert(code.as_str().unwrap()));
                frame["code"] = code.clone();
                let envelope = Envelope::decode(&serde_json::to_vec(&frame).unwrap()).unwrap();
                assert_eq!(serde_json::to_value(envelope.code.unwrap()).unwrap(), *code);
            }
            assert_eq!(codes.len(), 12);
            frame["code"] = json!("unknown_error");
            assert!(Envelope::decode(&serde_json::to_vec(&frame).unwrap()).is_err());
        }
    }
    assert_eq!(states.len(), 6);
    for frame in [
        json!({"schema_version":1,"channel":"control","id":"matrix","payload":{"type":"request","operation":"unknown_operation","arguments":{}}}),
        json!({"schema_version":1,"channel":"control","id":"matrix","execution_state":"unknown_state","payload":{"type":"response","result":{}}}),
    ] {
        assert!(Envelope::decode(&serde_json::to_vec(&frame).unwrap()).is_err());
    }
}

#[test]
fn shared_message_contract_checks_fields_channels_and_required_members() {
    let contract: serde_json::Value =
        serde_json::from_str(include_str!("../fuzz/remote_protocol/schema-contract.json")).unwrap();
    let messages = contract["messages"].as_object().unwrap();
    assert_eq!(messages.len(), 13);
    for (name, message) in messages {
        let fields: std::collections::BTreeSet<_> = message["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|field| field.as_str().unwrap())
            .collect();
        assert_eq!(fields.len(), message["fields"].as_array().unwrap().len());
        assert_eq!(
            fields,
            message["payload"]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect()
        );
        assert_eq!(message["payload"]["type"], name.as_str());
        let mut frame = message["envelope"].clone();
        frame["schema_version"] = contract["schema_version"].clone();
        frame["channel"] = message["channel"].clone();
        frame["payload"] = message["payload"].clone();
        let decoded = Envelope::decode(&serde_json::to_vec(&frame).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(decoded.payload).unwrap()["type"],
            name.as_str()
        );
        for field in &fields {
            let mut missing = frame.clone();
            missing["payload"].as_object_mut().unwrap().remove(*field);
            assert!(
                Envelope::decode(&serde_json::to_vec(&missing).unwrap()).is_err(),
                "{name} missing {field}"
            );
        }
        let mut extra = frame.clone();
        extra["payload"]["unknown_field"] = json!(true);
        assert!(
            Envelope::decode(&serde_json::to_vec(&extra).unwrap()).is_err(),
            "{name} extra field"
        );
        for channel in ["control", "event", "bulk"] {
            let mut changed = frame.clone();
            changed["channel"] = json!(channel);
            assert_eq!(
                Envelope::decode(&serde_json::to_vec(&changed).unwrap()).is_ok(),
                message["channel"] == channel,
                "{name} {channel}"
            );
        }
    }
}

#[test]
fn bounded_seeded_wire_mutations_never_panic_and_accepted_frames_roundtrip() {
    let corpus: Vec<Vec<u8>> = include_str!("../fuzz/remote_protocol/corpus.jsonl")
        .lines()
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line).unwrap()["wire"]
                .as_str()
                .unwrap()
                .as_bytes()
                .to_vec()
        })
        .collect();
    let mut state = 0x72656d6f746531u64;
    let mut random = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for index in 0..2048 {
        let mut bytes = corpus[random() as usize % corpus.len()].clone();
        let at = random() as usize % bytes.len();
        match index % 5 {
            0 => bytes[at] ^= 1 << (random() % 8),
            1 => {
                bytes.remove(at);
            }
            2 => {
                bytes.insert(at, random() as u8);
            }
            3 => bytes.truncate(at),
            _ => {
                let tail = bytes[at..].to_vec();
                bytes.extend_from_slice(&tail);
            }
        }
        assert!(bytes.len() <= 4096);
        let decoded = Envelope::decode(&bytes);
        let mut framed = (bytes.len() as u32).to_be_bytes().to_vec();
        framed.extend_from_slice(&bytes);
        let read = zenpi::remote::frame::read_frame(&mut std::io::Cursor::new(framed));
        assert_eq!(decoded.is_ok(), read.is_ok(), "mutation {index}");
        if let Ok(envelope) = decoded {
            assert_eq!(read.unwrap(), Some(envelope.clone()));
            let canonical = serde_json::to_vec(&envelope).unwrap();
            assert_eq!(Envelope::decode(&canonical).unwrap(), envelope);
        }
    }
}

#[test]
fn payload_node_and_depth_limits_accept_the_boundary_and_refuse_one_more() {
    for (nodes, expected) in [(MAX_JSON_NODES, true), (MAX_JSON_NODES + 1, false)] {
        let mut envelope = request(Operation::Inspect);
        if let Message::Request { arguments, .. } = &mut envelope.payload {
            *arguments = json!({"items":vec![0;nodes-2]});
        }
        assert_eq!(
            Envelope::decode(&serde_json::to_vec(&envelope).unwrap()).is_ok(),
            expected,
            "nodes {nodes}"
        );
    }
    for (depth, expected) in [(MAX_JSON_DEPTH, true), (MAX_JSON_DEPTH + 1, false)] {
        let mut nested = json!(null);
        for _ in 1..depth {
            nested = json!([nested]);
        }
        let mut envelope = request(Operation::Inspect);
        if let Message::Request { arguments, .. } = &mut envelope.payload {
            *arguments = json!({"nested":nested});
        }
        assert_eq!(
            Envelope::decode(&serde_json::to_vec(&envelope).unwrap()).is_ok(),
            expected,
            "depth {depth}"
        );
    }
}
