use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
use zenpi::remote::{
    auth::{Capability, DeviceChallenge, DeviceRegistry, pairing_proof_message},
    protocol::Operation,
};

fn key() -> EcdsaKeyPair {
    let random = ring::rand::SystemRandom::new();
    let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &random).unwrap();
    EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, doc.as_ref(), &random).unwrap()
}
fn sign(key: &EcdsaKeyPair, message: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(
        key.sign(&ring::rand::SystemRandom::new(), message)
            .unwrap()
            .as_ref(),
    )
}
fn register(registry: &DeviceRegistry, key: &EcdsaKeyPair) -> zenpi::remote::auth::DeviceRecord {
    let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    registry
        .verify_pair_and_register(
            "test-token",
            "r1",
            "phone",
            &public,
            &sign(
                key,
                &pairing_proof_message(&"a".repeat(64), "test-token", "r1", "phone", &public),
            ),
            || true,
        )
        .unwrap()
}

#[test]
fn authenticated_same_device_repair_preserves_exact_local_grant_across_registry_restart() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("devices.json");
    let fingerprint = "a".repeat(64);
    let registry = DeviceRegistry::new(path.clone(), fingerprint.clone()).unwrap();
    let signing = key();
    let original = register(&registry, &signing);
    registry
        .set_permissions(
            &original.id,
            original.revision,
            vec![Capability::Read, Capability::Control, Capability::Approval],
            vec!["project".into()],
        )
        .unwrap();
    let granted = registry.lookup(&original.id).unwrap();
    let before = std::fs::read(&path).unwrap();
    let public = URL_SAFE_NO_PAD.encode(signing.public_key().as_ref());
    let restored = DeviceRegistry::new(path.clone(), fingerprint.clone()).unwrap();
    let mut consumed = 0;
    let paired = restored
        .verify_pair_and_register(
            "new-lan-invite",
            "new-pair",
            "renamed-phone",
            &public,
            &sign(
                &signing,
                &pairing_proof_message(
                    &fingerprint,
                    "new-lan-invite",
                    "new-pair",
                    "renamed-phone",
                    &public,
                ),
            ),
            || {
                consumed += 1;
                true
            },
        )
        .unwrap();
    assert_eq!(consumed, 1);
    assert_eq!(paired, granted);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let mut challenge = DeviceChallenge::new(fingerprint).unwrap();
    let device = challenge
        .authenticate(
            &restored,
            &paired.id,
            &sign(&signing, &challenge.proof_message(&paired.id)),
        )
        .unwrap();
    for operation in [Operation::Inspect, Operation::Prompt, Operation::Approve] {
        restored.authorize(&device, "project", operation).unwrap();
        assert!(restored.authorize(&device, "foreign", operation).is_err());
    }
    restored.revoke(&paired.id).unwrap();
    let revoked = std::fs::read(&path).unwrap();
    assert!(
        restored
            .verify_pair_and_register(
                "after-revoke",
                "pair",
                "phone",
                &public,
                &sign(
                    &signing,
                    &pairing_proof_message(
                        &"a".repeat(64),
                        "after-revoke",
                        "pair",
                        "phone",
                        &public
                    )
                ),
                || panic!("revoked identity must not consume an invite")
            )
            .is_err()
    );
    assert_eq!(std::fs::read(path).unwrap(), revoked);
}

#[test]
fn shared_android_proof_vectors_match_rust_signed_bytes() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../android/app/src/test/resources/device-proof-v1.json"
    ))
    .unwrap();
    for value in vectors.as_array().unwrap() {
        let fields: Vec<_> = value["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let actual = match value["kind"].as_str().unwrap() {
            "pair" => pairing_proof_message(fields[0], fields[1], fields[2], fields[3], fields[4]),
            "authenticate" => {
                serde_json::to_vec(&["zenpi-device-auth-v1", fields[0], fields[1], fields[2]])
                    .unwrap()
            }
            "revoke_status" => serde_json::to_vec(&[
                "zenpi-device-revoke-status-v1",
                fields[0],
                fields[1],
                fields[2],
                fields[3],
                fields[4],
            ])
            .unwrap(),
            _ => panic!("unknown vector"),
        };
        assert_eq!(actual, value["message"].as_str().unwrap().as_bytes());
    }
}

#[test]
fn literal_unicode_separator_pairing_proof_is_not_an_equivalent_escaped_json_signature() {
    let signing = key();
    let public = URL_SAFE_NO_PAD.encode(signing.public_key().as_ref());
    let fingerprint = "a".repeat(64);
    let name = "Phone\u{2028}desk\u{2029}end";
    let canonical = pairing_proof_message(&fingerprint, "token", "request", name, &public);
    let proof = sign(&signing, &canonical);
    zenpi::remote::auth::verify_pairing_proof(
        &fingerprint,
        "token",
        "request",
        name,
        &public,
        &proof,
    )
    .unwrap();
    let escaped = String::from_utf8(canonical)
        .unwrap()
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    assert!(
        zenpi::remote::auth::verify_pairing_proof(
            &fingerprint,
            "token",
            "request",
            name,
            &public,
            &sign(&signing, escaped.as_bytes())
        )
        .is_err()
    );
}

#[test]
fn self_revocation_is_identity_revision_fenced_and_durable_without_project_grant() {
    use zenpi::remote::protocol::{Channel, Envelope, ExecutionState, Message};
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("devices.json");
    let registry = DeviceRegistry::new(path.clone(), "a".repeat(64)).unwrap();
    let first = key();
    let second = key();
    let one = register(&registry, &first);
    let two = register(&registry, &second);
    let authenticate = |key: &EcdsaKeyPair, id: &str| {
        let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
        challenge
            .authenticate(&registry, id, &sign(key, &challenge.proof_message(id)))
            .unwrap()
    };
    let device = authenticate(&first, &one.id);
    let mut request = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::DeviceRevokeSelf,
            arguments: serde_json::json!({}),
        },
    );
    request.id = Some("self-revoke".into());
    request.required_owner = Some(one.id.clone());
    request.expected_revision = Some(device.revision());
    for invalid in [
        {
            let mut v = request.clone();
            v.required_owner = Some(two.id.clone());
            v
        },
        {
            let mut v = request.clone();
            v.expected_revision = Some(0);
            v
        },
        {
            let mut v = request.clone();
            v.project_id = Some("p1".into());
            v
        },
        {
            let mut v = request.clone();
            v.payload = Message::Request {
                operation: Operation::DeviceRevokeSelf,
                arguments: serde_json::json!({"device_id":two.id}),
            };
            v
        },
    ] {
        assert_eq!(
            registry
                .dispatch_self_revoke(&device, &invalid)
                .execution_state,
            Some(ExecutionState::Failed)
        );
        assert!(!registry.lookup(&one.id).unwrap().revoked);
    }
    registry
        .set_permissions(&one.id, 1, vec![Capability::Read], vec![])
        .unwrap();
    assert!(
        registry.revoke_self(&device).is_err(),
        "stale authenticated handle cannot revoke after ACL revision changes"
    );
    let device = authenticate(&first, &one.id);
    request.expected_revision = Some(device.revision());
    assert_eq!(
        registry
            .dispatch_self_revoke(&device, &request)
            .execution_state,
        Some(ExecutionState::Succeeded)
    );
    assert!(!registry.is_current(&device).unwrap());
    assert!(registry.revoke_self(&device).is_err());
    let restored = DeviceRegistry::new(path, "a".repeat(64)).unwrap();
    assert!(restored.lookup(&one.id).unwrap().revoked);
    assert!(!restored.lookup(&two.id).unwrap().revoked);
    assert_eq!(restored.lookup(&one.id).unwrap().revision, 3);
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    assert!(
        challenge
            .authenticate(
                &restored,
                &one.id,
                &sign(&first, &challenge.proof_message(&one.id))
            )
            .is_err()
    );
}

#[test]
fn revoked_identity_can_query_only_its_signed_request_receipt() {
    use zenpi::remote::protocol::{Channel, Envelope, Message};
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("devices.json");
    let registry = DeviceRegistry::new(path.clone(), "a".repeat(64)).unwrap();
    let signing = key();
    let other = key();
    let record = register(&registry, &signing);
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let device = challenge
        .authenticate(
            &registry,
            &record.id,
            &sign(&signing, &challenge.proof_message(&record.id)),
        )
        .unwrap();
    let mut request = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::DeviceRevokeSelf,
            arguments: serde_json::json!({}),
        },
    );
    request.id = Some("original-revoke".into());
    request.required_owner = Some(record.id.clone());
    request.expected_revision = Some(device.revision());
    registry.dispatch_self_revoke(&device, &request);
    let registry = DeviceRegistry::new(path, "a".repeat(64)).unwrap();
    let mut query = DeviceChallenge::new("a".repeat(64)).unwrap();
    let signature = sign(
        &signing,
        &query.revoke_status_proof(&record.id, "original-revoke", "query1"),
    );
    assert!(
        query
            .revoke_status(
                &registry,
                &record.id,
                "original-revoke",
                "query1",
                &signature
            )
            .unwrap()
    );
    assert!(
        query
            .revoke_status(
                &registry,
                &record.id,
                "original-revoke",
                "query1",
                &signature
            )
            .is_err()
    );
    let mut wrong = DeviceChallenge::new("a".repeat(64)).unwrap();
    assert!(
        wrong
            .revoke_status(
                &registry,
                &record.id,
                "original-revoke",
                "query1",
                &signature
            )
            .is_err(),
        "connection challenge replay"
    );
    let mut wrong = DeviceChallenge::new("a".repeat(64)).unwrap();
    let signature = sign(
        &other,
        &wrong.revoke_status_proof(&record.id, "original-revoke", "query1"),
    );
    assert!(
        wrong
            .revoke_status(
                &registry,
                &record.id,
                "original-revoke",
                "query1",
                &signature
            )
            .is_err()
    );
    let mut missing = DeviceChallenge::new("a".repeat(64)).unwrap();
    let signature = sign(
        &signing,
        &missing.revoke_status_proof(&record.id, "different", "query2"),
    );
    assert!(
        !missing
            .revoke_status(&registry, &record.id, "different", "query2", &signature)
            .unwrap()
    );
    let mut auth = DeviceChallenge::new("a".repeat(64)).unwrap();
    assert!(
        auth.authenticate(
            &registry,
            &record.id,
            &sign(&signing, &auth.proof_message(&record.id))
        )
        .is_err()
    );
    let mut changed = DeviceChallenge::new("a".repeat(64)).unwrap();
    let proof = sign(
        &signing,
        &changed.revoke_status_proof(&record.id, "original-revoke", "query3"),
    );
    assert!(
        changed
            .revoke_status(&registry, &record.id, "different", "query3", &proof)
            .is_err(),
        "proof binds original request"
    );
    let mut changed = DeviceChallenge::new("a".repeat(64)).unwrap();
    let proof = sign(
        &signing,
        &changed.revoke_status_proof(&record.id, "original-revoke", "query3"),
    );
    assert!(
        changed
            .revoke_status(&registry, &record.id, "original-revoke", "query4", &proof)
            .is_err(),
        "proof binds query ID"
    );
}

#[test]
fn persistent_registry_default_denial_and_capability_project_matrix() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("devices.json");
    let registry = DeviceRegistry::new(path.clone(), "a".repeat(64)).unwrap();
    let key = key();
    let device = register(&registry, &key);
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let authenticated = challenge
        .authenticate(
            &registry,
            &device.id,
            &sign(&key, &challenge.proof_message(&device.id)),
        )
        .unwrap();
    assert!(
        registry
            .authorize(&authenticated, "p1", Operation::Inspect)
            .is_err()
    );
    registry
        .set_permissions(
            &device.id,
            1,
            vec![Capability::Read, Capability::Control],
            vec!["p1".into()],
        )
        .unwrap();
    assert!(
        registry
            .authorize(&authenticated, "p1", Operation::Inspect)
            .is_err(),
        "old revision fenced"
    );
    let restored = DeviceRegistry::new(path, "a".repeat(64)).unwrap();
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let authenticated = challenge
        .authenticate(
            &restored,
            &device.id,
            &sign(&key, &challenge.proof_message(&device.id)),
        )
        .unwrap();
    assert!(
        restored
            .authorize(&authenticated, "p1", Operation::Inspect)
            .is_ok()
    );
    assert!(
        restored
            .authorize(&authenticated, "p1", Operation::Prompt)
            .is_ok()
    );
    assert!(
        restored
            .authorize(&authenticated, "p1", Operation::TerminalOpen)
            .is_ok()
    );
    assert!(
        restored
            .authorize(&authenticated, "p1", Operation::Approve)
            .is_err()
    );
    assert!(
        restored
            .authorize(&authenticated, "p2", Operation::Inspect)
            .is_err()
    );
    assert!(
        restored
            .set_permissions(&device.id, 1, vec![Capability::Approval], vec!["p1".into()])
            .is_err()
    );
    restored
        .set_permissions(
            &device.id,
            2,
            vec![Capability::Read, Capability::Approval],
            vec!["p1".into()],
        )
        .unwrap();
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let authenticated = challenge
        .authenticate(
            &restored,
            &device.id,
            &sign(&key, &challenge.proof_message(&device.id)),
        )
        .unwrap();
    assert!(
        restored
            .authorize(&authenticated, "p1", Operation::Approve)
            .is_ok()
    );
    assert!(
        restored
            .authorize(&authenticated, "p1", Operation::Prompt)
            .is_err()
    );
    assert!(
        restored
            .authorize(&authenticated, "p1", Operation::TerminalOpen)
            .is_err()
    );
    restored.revoke(&device.id).unwrap();
    restored.revoke(&device.id).unwrap();
    assert!(
        restored
            .authorize(&authenticated, "p1", Operation::Inspect)
            .is_err()
    );
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    assert!(
        challenge
            .authenticate(
                &restored,
                &device.id,
                &sign(&key, &challenge.proof_message(&device.id))
            )
            .is_err()
    );
    let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    assert!(
        restored
            .verify_pair_and_register(
                "test-token",
                "r1",
                "phone",
                &public,
                &sign(
                    &key,
                    &pairing_proof_message(&"a".repeat(64), "test-token", "r1", "phone", &public)
                ),
                || panic!("revoked device consumed token")
            )
            .is_err()
    );
}

#[test]
fn challenge_is_one_use_connection_bound_and_wrong_key_proof_denied() {
    let temp = tempfile::tempdir().unwrap();
    let registry = DeviceRegistry::new(temp.path().join("devices.json"), "a".repeat(64)).unwrap();
    let key = key();
    let device = register(&registry, &key);
    let mut first = DeviceChallenge::new("a".repeat(64)).unwrap();
    let signature = sign(&key, &first.proof_message(&device.id));
    first
        .authenticate(&registry, &device.id, &signature)
        .unwrap();
    assert!(
        first
            .authenticate(&registry, &device.id, &signature)
            .is_err()
    );
    let mut other = DeviceChallenge::new("a".repeat(64)).unwrap();
    assert!(
        other
            .authenticate(&registry, &device.id, &signature)
            .is_err()
    );
    let correct = sign(&key, &other.proof_message(&device.id));
    assert!(
        other.authenticate(&registry, &device.id, &correct).is_err(),
        "failed proof consumes challenge"
    );
    let mut wrong_host = DeviceChallenge::new("b".repeat(64)).unwrap();
    assert!(
        wrong_host
            .authenticate(&registry, &device.id, &signature)
            .is_err()
    );
}

#[test]
fn invalid_pair_proof_never_consumes_invite_and_name_key_token_are_bound() {
    let temp = tempfile::tempdir().unwrap();
    let registry = DeviceRegistry::new(temp.path().join("devices.json"), "a".repeat(64)).unwrap();
    let key = key();
    let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
    let signature = sign(
        &key,
        &pairing_proof_message(&"a".repeat(64), "token", "r1", "phone", &public),
    );
    for (token, id, name) in [
        ("wrong", "r1", "phone"),
        ("token", "wrong", "phone"),
        ("token", "r1", "wrong"),
    ] {
        assert!(
            registry
                .verify_pair_and_register(token, id, name, &public, &signature, || panic!(
                    "invalid proof consumed token"
                ))
                .is_err()
        );
    }
    assert!(registry.list().unwrap().is_empty());
    assert!(
        registry
            .verify_pair_and_register("token", "r1", "phone", &public, &signature, || false)
            .is_err()
    );
    assert!(registry.list().unwrap().is_empty());
}

#[test]
fn corrupt_registry_symlink_and_identity_rotation_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("devices.json");
    let registry = DeviceRegistry::new(path.clone(), "a".repeat(64)).unwrap();
    register(&registry, &key());
    assert!(
        DeviceRegistry::new(path.clone(), "b".repeat(64))
            .unwrap()
            .list()
            .is_err()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let link = temp.path().join("link.json");
        symlink(&path, &link).unwrap();
        assert!(
            DeviceRegistry::new(link, "a".repeat(64))
                .unwrap()
                .list()
                .is_err()
        );
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(registry.list().is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    std::fs::write(&path, "broken").unwrap();
    assert!(registry.list().is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken");
}

#[cfg(unix)]
#[test]
fn registry_contention_is_bounded_busy_not_revocation_and_fresh_proof_is_required() {
    use std::os::fd::AsRawFd;
    use std::time::{Duration, Instant};
    use zenpi::remote::{
        auth::AuthError,
        protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message},
    };
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("devices.json");
    let registry = DeviceRegistry::new(path.clone(), "a".repeat(64)).unwrap();
    let key = key();
    let record = register(&registry, &key);
    registry
        .set_permissions(
            &record.id,
            record.revision,
            vec![Capability::Read, Capability::Control],
            vec!["project".into()],
        )
        .unwrap();
    let authenticate = || {
        let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
        let signed = sign(&key, &challenge.proof_message(&record.id));
        challenge
            .authenticate(&registry, &record.id, &signed)
            .unwrap()
    };
    let device = authenticate();
    let before = std::fs::read(&path).unwrap();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path.with_extension("lock"))
        .unwrap();
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    let start = Instant::now();
    assert!(matches!(
        registry.authorize(&device, "project", Operation::Prompt),
        Err(AuthError::Busy)
    ));
    assert!(start.elapsed() >= Duration::from_millis(90));
    assert!(start.elapsed() < Duration::from_secs(1));
    assert_eq!(AuthError::Busy.remote_code(), ErrorCode::Busy);
    assert_eq!(
        AuthError::InvalidRegistry.remote_code(),
        ErrorCode::Unauthorized
    );
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let signed = sign(&key, &challenge.proof_message(&record.id));
    assert!(matches!(
        challenge.authenticate(&registry, &record.id, &signed),
        Err(AuthError::Busy)
    ));
    let mut revoke = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: Operation::DeviceRevokeSelf,
            arguments: serde_json::json!({}),
        },
    );
    revoke.id = Some("revoke".into());
    revoke.required_owner = Some(device.id().into());
    revoke.expected_revision = Some(device.revision());
    let refused = registry.dispatch_self_revoke(&device, &revoke);
    assert_eq!(refused.code, Some(ErrorCode::Busy));
    assert_eq!(refused.execution_state, Some(ExecutionState::Failed));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    drop(lock);
    // Busy consumes the connection nonce; never reuse it after unlocking.
    assert!(matches!(
        challenge.authenticate(&registry, &record.id, &signed),
        Err(AuthError::Unauthorized)
    ));
    let fresh = authenticate();
    assert!(
        registry
            .authorize(&fresh, "project", Operation::Prompt)
            .is_ok()
    );
    registry.revoke(&record.id).unwrap();
    assert!(!registry.is_current(&fresh).unwrap());
    assert!(matches!(
        registry.authorize(&fresh, "project", Operation::Prompt),
        Err(AuthError::Unauthorized)
    ));
    assert_eq!(
        registry.dispatch_self_revoke(&fresh, &revoke).code,
        Some(ErrorCode::Unauthorized)
    );
}

#[cfg(unix)]
#[test]
fn short_lock_release_reloads_current_permissions_instead_of_a_cached_grant() {
    use std::os::fd::AsRawFd;
    use std::time::Duration;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("devices.json");
    let registry = DeviceRegistry::new(path.clone(), "a".repeat(64)).unwrap();
    let key = key();
    let record = register(&registry, &key);
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path.with_extension("lock"))
        .unwrap();
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    let current = std::fs::read(&path).unwrap();
    let mut changed: serde_json::Value = serde_json::from_slice(&current).unwrap();
    changed["devices"][0]["revoked"] = serde_json::json!(true);
    changed["devices"][0]["revision"] = serde_json::json!(2);
    let writer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20));
        // Explicit fixture models another local owner committing under the shared lock.
        std::fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
        drop(lock);
    });
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let signed = sign(&key, &challenge.proof_message(&record.id));
    assert!(matches!(
        challenge.authenticate(&registry, &record.id, &signed),
        Err(zenpi::remote::auth::AuthError::Unauthorized)
    ));
    writer.join().unwrap();
}

#[test]
fn concurrent_read_connections_do_not_invent_unauthorized_or_change_registry() {
    use std::sync::{Arc, Barrier};
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("devices.json");
    let registry = Arc::new(DeviceRegistry::new(path.clone(), "a".repeat(64)).unwrap());
    let key = key();
    let record = register(&registry, &key);
    registry
        .set_permissions(
            &record.id,
            record.revision,
            vec![Capability::Read],
            vec!["project".into()],
        )
        .unwrap();
    let mut challenge = DeviceChallenge::new("a".repeat(64)).unwrap();
    let device = challenge
        .authenticate(
            &registry,
            &record.id,
            &sign(&key, &challenge.proof_message(&record.id)),
        )
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let start = Arc::new(Barrier::new(8));
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let registry = registry.clone();
            let device = device.clone();
            let start = start.clone();
            scope.spawn(move || {
                start.wait();
                for _ in 0..50 {
                    assert!(
                        registry
                            .authorize(&device, "project", Operation::Inspect)
                            .is_ok()
                    );
                    assert!(registry.is_current(&device).unwrap());
                }
            });
        }
    });
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
