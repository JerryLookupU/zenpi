use zenpi::security::{
    SecretError, SecretHandle, SecretScope, child_environment, redact_json, redact_text,
};

const FIXTURE_SECRET: &str = "sk-fixture-secret-123456";

#[test]
fn text_and_nested_json_withhold_private_pem_body_until_matching_end() {
    for label in [
        "PRIVATE KEY",
        "RSA PRIVATE KEY",
        "EC PRIVATE KEY",
        "OPENSSH PRIVATE KEY",
        "ENCRYPTED PRIVATE KEY",
    ] {
        let input = format!(
            "before\n-----BEGIN {label}-----\nPRIVATE_BODY_7Xq9\n-----END PUBLIC KEY-----\nSTILL_PRIVATE_BODY_9Kd2\n-----END {label}-----\nafter\n"
        );
        let redacted = redact_text(&input, &[]);
        assert!(redacted.starts_with("before\n"));
        assert!(redacted.ends_with("after\n"));
        assert!(!redacted.contains("PRIVATE_BODY"), "{label}: {redacted}");
        let nested = redact_json(&serde_json::json!({"transcript":[{"content":input}]}), &[]);
        assert!(!nested.to_string().contains("PRIVATE_BODY"));
    }
}

#[test]
fn unterminated_private_pem_and_inline_quoted_body_are_withheld() {
    for input in [
        "-----begin rsa private key-----\r\nPRIVATE_BODY_UNFINISHED\r\n",
        "-----BEGIN PRIVATE KEY-----\nPRIVATE_BODY_UNFINISHED",
        "quoted: -----BEGIN EC PRIVATE KEY-----PRIVATE_BODY_UNFINISHED-----END EC PRIVATE KEY-----\npublic tail",
    ] {
        let redacted = redact_text(input, &[]);
        assert!(!redacted.contains("PRIVATE_BODY"));
        assert!(redacted.contains("<redacted private key>"));
    }
    assert!(
        redact_text(
            "-----BEGIN PRIVATE KEY-----\r\nbody\r\n-----END PRIVATE KEY-----\r\n",
            &[]
        )
        .ends_with("\r\n")
    );
}

#[test]
fn public_pem_and_ordinary_line_endings_are_preserved() {
    for input in [
        "ordinary\ntext\n",
        "ordinary\r\ntext",
        "-----BEGIN PUBLIC KEY-----\nPUBLIC_BODY\n-----END PUBLIC KEY-----\n",
        "-----BEGIN CERTIFICATE-----\nPUBLIC_CERT\n-----END CERTIFICATE-----",
    ] {
        assert_eq!(redact_text(input, &[]), input);
    }
}

#[test]
fn nested_and_mismatched_private_pem_ends_do_not_release_outer_body() {
    let input = "-----BEGIN RSA PRIVATE KEY-----\nPRIVATE_BODY_OUTER\n-----BEGIN EC PRIVATE KEY-----\nPRIVATE_BODY_INNER\n-----END RSA PRIVATE KEY-----\nPRIVATE_BODY_STILL_INNER\n-----END EC PRIVATE KEY-----\nPRIVATE_BODY_STILL_OUTER\n-----END RSA PRIVATE KEY-----\nvisible\n";
    let redacted = redact_text(input, &[]);
    assert!(!redacted.contains("PRIVATE_BODY"));
    assert!(redacted.ends_with("visible\n"));
    assert_eq!(redact_text(&redacted, &[]), redacted);
    let diff = "+-----BEGIN PRIVATE KEY-----\n+PRIVATE_BODY_ADDED\n+-----END PRIVATE KEY-----\n+safe\n-ordinary\n";
    assert!(!redact_text(diff, &[]).contains("PRIVATE_BODY"));
    assert!(redact_text(diff, &[]).ends_with("+safe\n-ordinary\n"));
}

#[test]
fn revocation_debug_never_exposes_secret_bytes() {
    let secret = "opaque-revocation-fixture-4871";
    let (handle, revocation) = SecretHandle::new(secret, "b".repeat(64)).unwrap();
    let debug = format!("{handle:?} {revocation:?}");
    assert!(!debug.contains(secret));
    assert!(!debug.contains(&format!("{:?}", secret.as_bytes())));
    assert!(debug.contains("<redacted>"));
}

#[test]
fn redaction_covers_headers_urls_nested_json_and_known_values() {
    let text = format!(
        "Authorization: Bearer {FIXTURE_SECRET} https://user:{FIXTURE_SECRET}@example.test/v1?api_key={FIXTURE_SECRET}"
    );
    let redacted = redact_text(&text, &[FIXTURE_SECRET]);
    assert!(!redacted.contains(FIXTURE_SECRET));
    assert!(!redacted.contains("user:"));
    assert!(redacted.contains("<redacted>"));

    let value = serde_json::json!({
        "authorization": format!("Bearer {FIXTURE_SECRET}"),
        "nested": {"api_key": FIXTURE_SECRET, "message": format!("key={FIXTURE_SECRET}")},
    });
    let encoded = serde_json::to_string(&redact_json(&value, &[FIXTURE_SECRET])).unwrap();
    assert!(!encoded.contains(FIXTURE_SECRET));
    assert!(encoded.contains("<redacted>"));

    let diagnostic = format!("OPENAI_API_KEY={FIXTURE_SECRET} password='{FIXTURE_SECRET}'");
    let diagnostic = redact_text(&diagnostic, &[]);
    assert!(!diagnostic.contains(FIXTURE_SECRET));
}

#[test]
fn redaction_covers_cookie_proxy_and_provider_specific_key_headers() {
    let value = serde_json::json!({
        "Cookie": "session=fixture-cookie",
        "Set-Cookie": "sid=fixture-cookie",
        "Proxy-Authorization": "Basic fixture-proxy",
        "X-Api-Key": "fixture-x-key",
        "nested": {"private_key": "fixture-private"},
    });
    let encoded = serde_json::to_string(&redact_json(&value, &[])).unwrap();
    for secret in [
        "fixture-cookie",
        "fixture-proxy",
        "fixture-x-key",
        "fixture-private",
    ] {
        assert!(!encoded.contains(secret), "leaked {secret}: {encoded}");
    }
}

#[test]
fn text_redaction_covers_header_colons_and_query_credentials() {
    let text = "X-Api-Key: fixture-header\nCookie: sid=fixture-cookie\nhttps://example.test/v1?api_key=fixture-query";
    let redacted = redact_text(text, &[]);
    for secret in ["fixture-header", "fixture-cookie", "fixture-query"] {
        assert!(!redacted.contains(secret), "leaked {secret}: {redacted}");
    }
}

#[test]
fn filtered_child_environment_contains_no_credential_names() {
    let environment = child_environment();
    assert!(environment.iter().any(|(key, _)| key == "PATH"));
    assert!(environment.iter().all(|(key, _)| {
        !key.contains("KEY")
            && !key.contains("TOKEN")
            && !key.contains("AUTH")
            && !key.contains("PASSWORD")
    }));
}

#[test]
fn debug_output_redacts_backend_credentials() {
    let backend = zenpi::backend::OpenAiCompatibleBackend::new(
        "https://example.test/v1",
        Some(FIXTURE_SECRET.into()),
        "fixture",
    )
    .unwrap();
    let debug = format!("{backend:?}");
    assert!(!debug.contains(FIXTURE_SECRET));
    assert!(debug.contains("<redacted>"));
}

#[test]
fn secret_handle_is_opaque_and_policy_bound() {
    let digest = "a".repeat(64);
    let (handle, revoke) = SecretHandle::new(FIXTURE_SECRET, digest.clone()).unwrap();
    assert_eq!(handle.policy_digest(), digest);
    assert!(format!("{handle:?}").contains("present: true"));
    assert!(!format!("{handle:?}").contains(FIXTURE_SECRET));
    assert!(handle.verify_policy_digest(&"b".repeat(64)).is_err());
    assert_eq!(
        handle.verify_policy_digest(&"b".repeat(64)).unwrap_err(),
        SecretError::PolicyMismatch
    );
    revoke.revoke();
    assert_eq!(
        handle.verify_policy_digest(&digest).unwrap_err(),
        SecretError::RevokedOrExpired
    );
}

#[test]
fn active_secret_handles_are_redacted_without_explicit_secret_lists() {
    let digest = "c".repeat(64);
    let (handle, _revoke) = SecretHandle::new(FIXTURE_SECRET, digest).unwrap();
    let text = format!("tool returned {FIXTURE_SECRET}");
    assert!(!redact_text(&text, &[]).contains(FIXTURE_SECRET));
    drop(handle);
}

#[test]
fn scoped_handle_verifies_public_scope_without_exposing_material() {
    let policy = "d".repeat(64);
    let scope = SecretScope {
        route_digest: "route-fixture".into(),
        identity_scope: "account-fixture".into(),
    };
    let (handle, revoke) = SecretHandle::new_scoped(
        "scoped-public-secret-fixture",
        &policy,
        scope.clone(),
        u64::MAX,
    )
    .unwrap();
    handle.verify_scope(&policy, &scope).unwrap();
    let mut other = scope.clone();
    other.identity_scope = "other-account".into();
    assert_eq!(
        handle.verify_scope(&policy, &other),
        Err(SecretError::ScopeMismatch)
    );
    assert!(!format!("{handle:?} {revoke:?}").contains("scoped-public-secret-fixture"));
    revoke.revoke();
    assert_eq!(
        handle.verify_scope(&policy, &scope),
        Err(SecretError::RevokedOrExpired)
    );
}

#[test]
fn scoped_handle_rejects_expired_or_unbounded_scope_input() {
    let scope = SecretScope {
        route_digest: "route-fixture".into(),
        identity_scope: "account-fixture".into(),
    };
    assert!(matches!(
        SecretHandle::new_scoped("fixture", "a".repeat(64), scope.clone(), 0),
        Err(SecretError::RevokedOrExpired)
    ));
    let mut invalid = scope;
    invalid.route_digest.clear();
    assert!(matches!(
        SecretHandle::new_scoped("fixture", "a".repeat(64), invalid, u64::MAX),
        Err(SecretError::InvalidScope)
    ));
}
