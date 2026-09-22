//! Central redaction and private-file helpers.

use std::{
    fs, io,
    path::Path,
    sync::{Arc, Mutex, OnceLock, Weak},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

const MAX_REGISTERED_SECRETS: usize = 128;
const MAX_SECRET_BYTES: usize = 16 * 1024;
const REGISTERED_SECRET_TTL_MS: u64 = 10 * 60 * 1000;

struct SecretMaterial {
    value: Mutex<Vec<u8>>,
    policy_digest: String,
    expires_at_ms: Option<u64>,
    scope: Option<SecretScope>,
    revoked: std::sync::atomic::AtomicBool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretScope {
    pub route_digest: String,
    pub identity_scope: String,
}

impl SecretScope {
    fn validate(&self) -> Result<(), SecretError> {
        for value in [&self.route_digest, &self.identity_scope] {
            if value.is_empty()
                || value.len() > 2048
                || value.chars().any(char::is_whitespace)
                || value.chars().any(char::is_control)
            {
                return Err(SecretError::InvalidScope);
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for SecretMaterial {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SecretMaterial")
            .field("value", &"<redacted>")
            .field("policy_digest", &self.policy_digest)
            .field("expires_at_ms", &self.expires_at_ms)
            .finish_non_exhaustive()
    }
}

impl Drop for SecretMaterial {
    fn drop(&mut self) {
        if let Ok(value) = self.value.get_mut() {
            for byte in value.iter_mut() {
                *byte = 0;
            }
        }
    }
}

/// An opaque, non-serializable credential capability.  A worker can carry the
/// handle but cannot inspect or export its value; only the host-side backend
/// adapter can borrow it for the duration of an authenticated request.
#[derive(Clone)]
pub struct SecretHandle(Arc<SecretMaterial>);

impl std::fmt::Debug for SecretHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SecretHandle")
            .field("policy_digest", &self.0.policy_digest)
            .field("present", &true)
            .finish()
    }
}

/// Host-owned revocation capability.  It is intentionally a separate type so
/// granting a handle to a worker does not grant permission to revoke it.
#[derive(Clone, Debug)]
pub struct SecretRevocation(Arc<SecretMaterial>);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SecretError {
    #[error("secret is empty or contains control characters")]
    Invalid,
    #[error("policy digest must be a lowercase SHA-256")]
    InvalidPolicyDigest,
    #[error("secret handle is revoked or expired")]
    RevokedOrExpired,
    #[error("secret handle policy digest mismatch")]
    PolicyMismatch,
    #[error("secret handle scope is invalid")]
    InvalidScope,
    #[error("secret handle requires route and identity verification")]
    ScopeRequired,
    #[error("secret handle route or identity scope mismatch")]
    ScopeMismatch,
}

impl SecretHandle {
    /// Create a host-owned handle bound to one immutable Blueprint policy.
    /// Callers should pass a value read from a private auth source, never a
    /// value supplied by model/tool input.
    pub fn new(
        value: impl Into<String>,
        policy_digest: impl Into<String>,
    ) -> Result<(Self, SecretRevocation), SecretError> {
        Self::new_material(value.into(), policy_digest.into(), None, None)
    }

    pub fn new_scoped(
        value: impl Into<String>,
        policy_digest: impl Into<String>,
        scope: SecretScope,
        expires_at_ms: u64,
    ) -> Result<(Self, SecretRevocation), SecretError> {
        Self::new_scoped_at(
            value.into(),
            policy_digest.into(),
            scope,
            expires_at_ms,
            now_ms(),
        )
    }

    fn new_scoped_at(
        value: String,
        policy_digest: String,
        scope: SecretScope,
        expires_at_ms: u64,
        now: u64,
    ) -> Result<(Self, SecretRevocation), SecretError> {
        scope.validate()?;
        if expires_at_ms <= now {
            return Err(SecretError::RevokedOrExpired);
        }
        Self::new_material(value, policy_digest, Some(scope), Some(expires_at_ms))
    }

    fn new_material(
        value: String,
        policy_digest: String,
        scope: Option<SecretScope>,
        expires_at_ms: Option<u64>,
    ) -> Result<(Self, SecretRevocation), SecretError> {
        if value.is_empty() || value.len() > MAX_SECRET_BYTES || value.chars().any(char::is_control)
        {
            return Err(SecretError::Invalid);
        }
        validate_policy_digest(&policy_digest)?;
        let material = Arc::new(SecretMaterial {
            value: Mutex::new(value.into_bytes()),
            policy_digest,
            expires_at_ms,
            scope,
            revoked: std::sync::atomic::AtomicBool::new(false),
        });
        register_secret(&material);
        let revoke = SecretRevocation(Arc::clone(&material));
        Ok((Self(material), revoke))
    }

    /// Return the non-sensitive policy binding for evidence and receipts.
    pub fn policy_digest(&self) -> &str {
        &self.0.policy_digest
    }

    /// Verify that a handle is usable under an exact policy digest.  Digest
    /// comparison is constant-time for equal-length values and fail-closed for
    /// malformed input.
    pub fn verify_policy_digest(&self, expected: &str) -> Result<(), SecretError> {
        self.verify_policy_at(expected, now_ms())
    }

    fn verify_policy_at(&self, expected: &str, now: u64) -> Result<(), SecretError> {
        validate_policy_digest(expected)?;
        if !constant_time_equal(self.policy_digest().as_bytes(), expected.as_bytes()) {
            return Err(SecretError::PolicyMismatch);
        }
        if self.0.revoked.load(std::sync::atomic::Ordering::Acquire)
            || self.0.expires_at_ms.is_some_and(|expiry| now >= expiry)
        {
            return Err(SecretError::RevokedOrExpired);
        }
        Ok(())
    }

    pub fn verify_scope(
        &self,
        expected_policy_digest: &str,
        expected: &SecretScope,
    ) -> Result<(), SecretError> {
        self.verify_scope_at(expected_policy_digest, expected, now_ms())
    }

    fn verify_scope_at(
        &self,
        policy: &str,
        expected: &SecretScope,
        now: u64,
    ) -> Result<(), SecretError> {
        self.verify_policy_at(policy, now)?;
        expected.validate()?;
        let actual = self.0.scope.as_ref().ok_or(SecretError::ScopeRequired)?;
        if !constant_time_equal(
            actual.route_digest.as_bytes(),
            expected.route_digest.as_bytes(),
        ) || !constant_time_equal(
            actual.identity_scope.as_bytes(),
            expected.identity_scope.as_bytes(),
        ) {
            return Err(SecretError::ScopeMismatch);
        }
        Ok(())
    }

    pub(crate) fn is_scoped(&self) -> bool {
        self.0.scope.is_some()
    }

    /// Borrow the secret only inside this crate's host adapter.  No public API
    /// converts a handle to a String or serializes the underlying bytes.
    pub(crate) fn with_secret<T>(
        &self,
        expected_policy_digest: &str,
        f: impl FnOnce(&str) -> T,
    ) -> Result<T, SecretError> {
        self.borrow_secret(expected_policy_digest, None, &now_ms, f)
    }

    pub(crate) fn with_scoped_secret<T>(
        &self,
        expected_policy_digest: &str,
        expected_scope: &SecretScope,
        f: impl FnOnce(&str) -> T,
    ) -> Result<T, SecretError> {
        self.borrow_secret(expected_policy_digest, Some(expected_scope), &now_ms, f)
    }

    fn borrow_secret<T>(
        &self,
        policy: &str,
        scope: Option<&SecretScope>,
        clock: &dyn Fn() -> u64,
        f: impl FnOnce(&str) -> T,
    ) -> Result<T, SecretError> {
        let verify = || match scope {
            Some(scope) => self.verify_scope_at(policy, scope, clock()),
            None if self.is_scoped() => Err(SecretError::ScopeRequired),
            None => self.verify_policy_at(policy, clock()),
        };
        verify()?;
        let value = self
            .0
            .value
            .lock()
            .map_err(|_| SecretError::RevokedOrExpired)?;
        // A waiting borrower must not outlive a revoke or expiry while queued.
        verify()?;
        let value = std::str::from_utf8(&value).map_err(|_| SecretError::RevokedOrExpired)?;
        Ok(f(value))
    }
}

impl SecretRevocation {
    pub fn revoke(&self) {
        self.0
            .revoked
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

fn validate_policy_digest(value: &str) -> Result<(), SecretError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(SecretError::InvalidPolicyDigest)
    }
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0_u8;
    for (&a, &b) in left.iter().zip(right) {
        difference |= a ^ b;
    }
    difference == 0
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().min(u128::from(u64::MAX)) as u64
        })
}

fn secret_registry() -> &'static Mutex<Vec<Weak<SecretMaterial>>> {
    static REGISTRY: OnceLock<Mutex<Vec<Weak<SecretMaterial>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

type LegacySecretRegistry = Mutex<Vec<(Arc<SecretMaterial>, u64)>>;

fn legacy_secret_registry() -> &'static LegacySecretRegistry {
    static REGISTRY: OnceLock<LegacySecretRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

fn register_secret(material: &Arc<SecretMaterial>) {
    let Ok(mut registry) = secret_registry().lock() else {
        return;
    };
    registry.retain(|entry| entry.strong_count() > 0);
    if registry.len() < MAX_REGISTERED_SECRETS {
        registry.push(Arc::downgrade(material));
    }
}

/// Register a credential held by a legacy host API (for example the existing
/// `Option<String>` backend constructor) for bounded process-local redaction.
/// New worker paths should prefer [`SecretHandle`], which does not need this
/// compatibility registry.
pub(crate) fn register_secret_value(value: &str) {
    if value.len() < 4 || value.len() > MAX_SECRET_BYTES || value.chars().any(char::is_control) {
        return;
    }
    let material = Arc::new(SecretMaterial {
        value: Mutex::new(value.as_bytes().to_vec()),
        policy_digest: "0".repeat(64),
        expires_at_ms: Some(now_ms().saturating_add(REGISTERED_SECRET_TTL_MS)),
        scope: None,
        revoked: std::sync::atomic::AtomicBool::new(false),
    });
    let Ok(mut registry) = legacy_secret_registry().lock() else {
        return;
    };
    let now = now_ms();
    registry.retain(|(_, expiry)| *expiry > now);
    if registry.len() < MAX_REGISTERED_SECRETS {
        registry.push((material, now.saturating_add(REGISTERED_SECRET_TTL_MS)));
    }
}

pub(crate) fn registered_secret_values() -> Vec<String> {
    let Ok(mut registry) = secret_registry().lock() else {
        return Vec::new();
    };
    let mut values = Vec::new();
    registry.retain(|entry| {
        let Some(material) = entry.upgrade() else {
            return false;
        };
        if let Ok(value) = material.value.lock()
            && let Ok(value) = std::str::from_utf8(&value)
        {
            values.push(value.to_owned());
        }
        true
    });
    if let Ok(mut registry) = legacy_secret_registry().lock() {
        let now = now_ms();
        registry.retain(|(material, expiry)| {
            if *expiry <= now {
                return false;
            }
            if let Ok(value) = material.value.lock()
                && let Ok(value) = std::str::from_utf8(&value)
            {
                values.push(value.to_owned());
            }
            true
        });
    }
    values
}

const SECRET_KEYS: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "api_key",
    "api-key",
    "x-api-key",
    "access_token",
    "refresh_token",
    "client_secret",
    "password",
    "private_key",
    "secret_key",
];

pub fn redact_text(input: &str, known_secrets: &[&str]) -> String {
    let mut output = input.to_owned();
    for secret in known_secrets.iter().filter(|secret| secret.len() >= 4) {
        output = output.replace(secret, "<redacted>");
    }
    for secret in registered_secret_values()
        .iter()
        .filter(|secret| secret.len() >= 4)
    {
        output = output.replace(secret, "<redacted>");
    }
    output = redact_assignments(&output);
    output = redact_header_values(&output);
    output = redact_bearer_tokens(&output);
    output = redact_url_credentials(&output);
    output
}

fn redact_header_values(value: &str) -> String {
    const MARKERS: &[&str] = &[
        "authorization:",
        "proxy-authorization:",
        "cookie:",
        "set-cookie:",
        "api-key:",
        "x-api-key:",
        "access-token:",
        "refresh-token:",
        "client-secret:",
    ];
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;
    while !remaining.is_empty() {
        let lower = remaining.to_ascii_lowercase();
        let Some((index, marker)) = MARKERS
            .iter()
            .filter_map(|marker| lower.find(marker).map(|index| (index, *marker)))
            .min_by_key(|(index, _)| *index)
        else {
            output.push_str(remaining);
            break;
        };
        output.push_str(&remaining[..index + marker.len()]);
        let tail = remaining[index + marker.len()..].trim_start_matches([' ', '\t']);
        output.push_str(" <redacted>");
        let end = tail.find(['\r', '\n']).unwrap_or(tail.len());
        remaining = &tail[end..];
    }
    output
}

/// Redact conventional key/value diagnostics even when the caller did not
/// register the value first. This catches environment dumps and provider
/// error strings such as `OPENAI_API_KEY=...` without changing the key name.
fn redact_assignments(value: &str) -> String {
    const MARKERS: &[&str] = &[
        "proxy-authorization=",
        "cookie=",
        "set-cookie=",
        "api_key=",
        "api-key=",
        "x-api-key=",
        "access_token=",
        "refresh_token=",
        "password=",
        "client_secret=",
        "private_key=",
        "secret_key=",
        "secret=",
    ];
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;
    while !remaining.is_empty() {
        let lower = remaining.to_ascii_lowercase();
        let Some((index, marker)) = MARKERS
            .iter()
            .filter_map(|marker| lower.find(marker).map(|index| (index, *marker)))
            .min_by_key(|(index, _)| *index)
        else {
            output.push_str(remaining);
            break;
        };
        output.push_str(&remaining[..index + marker.len()]);
        let tail = &remaining[index + marker.len()..];
        let (quoted, start) = match tail.as_bytes().first() {
            Some(b'"') | Some(b'\'') => (Some(tail.as_bytes()[0]), 1),
            _ => (None, 0),
        };
        output.push_str("<redacted>");
        let content = &tail[start..];
        let end = if let Some(quote) = quoted {
            content
                .find(char::from(quote))
                .map_or(content.len(), |index| index + 1)
        } else {
            content
                .find(|character: char| {
                    character.is_whitespace() || matches!(character, ',' | '}' | ']')
                })
                .unwrap_or(content.len())
        };
        remaining = &content[end..];
    }
    output
}

pub fn redact_json(value: &Value, known_secrets: &[&str]) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    if is_secret_key(key) {
                        (key.clone(), Value::String("<redacted>".into()))
                    } else {
                        (key.clone(), redact_json(value, known_secrets))
                    }
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| redact_json(value, known_secrets))
                .collect(),
        ),
        Value::String(value) => Value::String(redact_text(value, known_secrets)),
        other => other.clone(),
    }
}

pub fn is_secret_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    SECRET_KEYS
        .iter()
        .any(|candidate| normalized == *candidate || normalized.ends_with(candidate))
}

pub fn child_environment() -> Vec<(String, String)> {
    [
        ("PATH", "/usr/bin:/bin"),
        ("LANG", "C.UTF-8"),
        ("LC_ALL", "C.UTF-8"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect()
}

pub fn restrict_private_file(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        // `metadata` follows links. Refuse a link before changing mode so a
        // caller cannot accidentally chmod a file outside its ownership
        // boundary (session journals are opened from user-supplied paths).
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "refusing to change permissions on a symbolic link",
                ));
            }
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        if let Some(metadata) = metadata {
            let mut permissions = metadata.permissions();
            if permissions.mode() & 0o777 != 0o600 {
                permissions.set_mode(0o600);
                fs::set_permissions(path, permissions)?;
            }
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn redact_bearer_tokens(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;
    loop {
        let lower = remaining.to_ascii_lowercase();
        let Some(index) = lower.find("bearer ") else {
            output.push_str(remaining);
            break;
        };
        output.push_str(&remaining[..index]);
        output.push_str(&remaining[index..index + "bearer ".len()]);
        output.push_str("<redacted>");
        let token = &remaining[index + "bearer ".len()..];
        let end = token
            .find(|character: char| {
                character.is_whitespace() || matches!(character, ',' | '}' | ']')
            })
            .unwrap_or(token.len());
        remaining = &token[end..];
    }
    output
}

fn redact_url_credentials(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;
    loop {
        let Some(scheme) = remaining.find("://") else {
            output.push_str(remaining);
            break;
        };
        let authority_start = scheme + 3;
        let authority_end = remaining[authority_start..]
            .find(['/', '?', '#', ' '])
            .map_or(remaining.len(), |index| authority_start + index);
        let authority = &remaining[authority_start..authority_end];
        let Some(at) = authority.rfind('@') else {
            output.push_str(&remaining[..authority_end]);
            remaining = &remaining[authority_end..];
            continue;
        };
        output.push_str(&remaining[..authority_start]);
        output.push_str("<redacted>@");
        output.push_str(&authority[at + 1..]);
        remaining = &remaining[authority_end..];
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn scope() -> SecretScope {
        SecretScope {
            route_digest: "r".repeat(64),
            identity_scope: "identity-fixture".into(),
        }
    }

    fn scoped(expiry: u64) -> (SecretHandle, SecretRevocation) {
        SecretHandle::new_scoped_at(
            "scope-fixture-secret".into(),
            "a".repeat(64),
            scope(),
            expiry,
            100,
        )
        .unwrap()
    }

    #[test]
    fn scoped_handle_checks_exact_expiry_and_rejects_legacy_borrow() {
        let (handle, _) = scoped(200);
        assert!(
            handle
                .verify_scope_at(&"a".repeat(64), &scope(), 199)
                .is_ok()
        );
        assert_eq!(
            handle.verify_scope_at(&"a".repeat(64), &scope(), 200),
            Err(SecretError::RevokedOrExpired)
        );
        assert_eq!(
            handle.with_secret(&"a".repeat(64), |_| ()),
            Err(SecretError::ScopeRequired)
        );
        assert!(matches!(
            SecretHandle::new_scoped_at("fixture".into(), "a".repeat(64), scope(), 100, 100),
            Err(SecretError::RevokedOrExpired)
        ));
    }

    #[test]
    fn scoped_handle_rejects_route_identity_and_policy_changes() {
        let (handle, _) = scoped(200);
        let mut wrong = scope();
        wrong.route_digest.push('x');
        assert_eq!(
            handle.verify_scope_at(&"a".repeat(64), &wrong, 101),
            Err(SecretError::ScopeMismatch)
        );
        wrong = scope();
        wrong.identity_scope.push('x');
        assert_eq!(
            handle.verify_scope_at(&"a".repeat(64), &wrong, 101),
            Err(SecretError::ScopeMismatch)
        );
        assert_eq!(
            handle.verify_scope_at(&"b".repeat(64), &scope(), 101),
            Err(SecretError::PolicyMismatch)
        );
    }

    #[test]
    fn borrower_rechecks_expiry_with_clock_inside_the_value_lock() {
        let (handle, _) = scoped(200);
        let calls = Cell::new(0);
        let clock = || {
            calls.set(calls.get() + 1);
            if calls.get() == 1 { 199 } else { 200 }
        };
        let borrowed = Cell::new(false);
        let result = handle.borrow_secret(&"a".repeat(64), Some(&scope()), &clock, |_| {
            borrowed.set(true)
        });
        assert_eq!(result, Err(SecretError::RevokedOrExpired));
        assert_eq!(calls.get(), 2);
        assert!(!borrowed.get());
    }

    #[test]
    fn waiting_borrower_rechecks_revocation_after_acquiring_value_lock() {
        let (handle, revoke) = scoped(200);
        let lock = handle.0.value.lock().unwrap();
        let borrower = handle.clone();
        let (checked, ready) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let clock = || {
                checked.send(()).unwrap();
                101
            };
            borrower.borrow_secret(&"a".repeat(64), Some(&scope()), &clock, |_| ())
        });
        ready
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        revoke.revoke();
        drop(lock);
        assert_eq!(worker.join().unwrap(), Err(SecretError::RevokedOrExpired));
    }

    #[test]
    fn successful_scoped_borrow_stays_bounded_to_the_callback() {
        let (handle, _) = scoped(200);
        let length = handle
            .borrow_secret(&"a".repeat(64), Some(&scope()), &|| 101, str::len)
            .unwrap();
        assert_eq!(length, "scope-fixture-secret".len());
    }
}
