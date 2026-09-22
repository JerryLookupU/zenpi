//! Request-local, scoped credentials. Only committed store results can mint handles.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::backend::{BackendError, HttpRequestKind, RequestControl};
use crate::providers::{
    codex as definition,
    connection::{ValidatedRoute, revalidate_route_auth},
};
use crate::security::{SecretError, SecretHandle, SecretRevocation, SecretScope};

use super::store::{
    CommittedCredential, CredentialKind, CredentialMetadata, CredentialState, CredentialStore,
    LockWait, RefreshGuard, RefreshResolution, RefreshTicket, RefreshTokens, StoreError,
};
use super::{AuthBinding, AuthError, AuthIdentitySnapshot, codex};

const REFRESH_SKEW_MS: u64 = 5 * 60 * 1000;
const HANDLE_TTL_MS: u64 = 60_000;

pub(crate) struct AuthContext {
    handle: Option<SecretHandle>,
    revocation: Option<SecretRevocation>,
    scope: SecretScope,
    owner_id: String,
    session_id: String,
    policy_digest: String,
    credential_revision: Option<u64>,
    account_id: Option<String>,
    minimum_validity_ms: u64,
    expires_at_ms: u64,
}

impl AuthContext {
    pub(crate) fn credential_revision(&self) -> Option<u64> {
        self.credential_revision
    }
    pub(crate) fn account_id(&self) -> Option<&str> {
        self.account_id.as_deref()
    }
    pub(crate) fn policy_digest(&self) -> &str {
        &self.policy_digest
    }

    pub(crate) fn with_secret<T>(
        &self,
        expected_policy: &str,
        expected_scope: &SecretScope,
        action: impl FnOnce(Option<&str>) -> T,
    ) -> Result<T, AuthError> {
        if expected_policy != self.policy_digest || expected_scope != &self.scope {
            return Err(AuthError::ScopeMismatch);
        }
        if now_ms()? >= self.expires_at_ms {
            return Err(AuthError::ContextExpired);
        }
        match &self.handle {
            Some(handle) => handle
                .with_scoped_secret(expected_policy, expected_scope, |secret| {
                    action(Some(secret))
                })
                .map_err(secret_error),
            None => Ok(action(None)),
        }
    }

    fn revoke(&self) {
        if let Some(revocation) = &self.revocation {
            revocation.revoke();
        }
    }
}

impl Drop for AuthContext {
    fn drop(&mut self) {
        self.revoke();
    }
}

pub(crate) struct AuthResolver {
    store: CredentialStore,
}

impl AuthResolver {
    pub(crate) fn new(store: CredentialStore) -> Self {
        Self { store }
    }

    /// Prepare a fixed identity without issuing a handle or repairing auth.
    /// A live peer's InFlight state is handled by the later controlled resolve.
    pub(crate) fn initial_identity(
        &self,
        binding: &AuthBinding,
    ) -> Result<Option<AuthIdentitySnapshot>, AuthError> {
        let id = match binding {
            AuthBinding::Anonymous => return Ok(None),
            AuthBinding::LegacyApiKey => return Err(AuthError::ModeMismatch),
            AuthBinding::StoredApiKey { credential_id }
            | AuthBinding::CodexOAuth { credential_id } => credential_id,
        };
        let metadata = self.store.metadata(id, &LockWait::default())?;
        validate_binding_metadata(binding, &metadata)?;
        if metadata.state != CredentialState::RefreshInFlight {
            require_active(&metadata)?;
        }
        Ok(Some(metadata.identity))
    }

    pub(crate) fn resolve(
        &self,
        route: &ValidatedRoute,
        control: &mut RequestControl<'_>,
        minimum_validity_ms: u64,
    ) -> Result<AuthContext, AuthError> {
        let result = self.resolve_with(
            route,
            control,
            minimum_validity_ms,
            None,
            &mut codex::send_refresh,
        );
        normalize_lock_error(result, control)
    }

    pub(crate) fn recover_unauthorized(
        &self,
        route: &ValidatedRoute,
        previous_revision: u64,
        control: &mut RequestControl<'_>,
        minimum_validity_ms: u64,
    ) -> Result<AuthContext, AuthError> {
        if !matches!(route.auth(), AuthBinding::CodexOAuth { .. }) {
            return Err(AuthError::LoginRequired);
        }
        let result = self.resolve_with(
            route,
            control,
            minimum_validity_ms,
            Some(previous_revision),
            &mut codex::send_refresh,
        );
        normalize_lock_error(result, control)
    }

    fn resolve_with(
        &self,
        route: &ValidatedRoute,
        control: &mut RequestControl<'_>,
        minimum_validity_ms: u64,
        unauthorized_revision: Option<u64>,
        send: &mut dyn FnMut(
            &codex::RefreshRequest,
            &RequestControl<'_>,
        ) -> Result<RefreshTokens, AuthError>,
    ) -> Result<AuthContext, AuthError> {
        let policy = request_policy(route, control)?;
        let id = match route.auth() {
            AuthBinding::Anonymous => {
                return self.issue(route, control, policy, None, minimum_validity_ms);
            }
            AuthBinding::LegacyApiKey => return Err(AuthError::ModeMismatch),
            AuthBinding::StoredApiKey { credential_id }
            | AuthBinding::CodexOAuth { credential_id } => credential_id,
        };
        let observed = self.store.metadata(id, &wait(control))?;
        validate_metadata(route, &observed)?;
        if observed.kind == CredentialKind::ApiKey {
            require_active(&observed)?;
            let committed = self.store.read_committed(
                id,
                observed.identity.credential_revision,
                &wait(control),
            )?;
            return self.issue(route, control, policy, Some(committed), minimum_validity_ms);
        }

        // A peer may be refreshing even when the old token still looks fresh.
        // The account lock, never the global file lock, spans the token HTTP.
        let guard = self.store.lock_refresh(id, &wait(control))?;
        let current = self.store.metadata(id, &wait(control))?;
        validate_metadata(route, &current)?;
        if current.state == CredentialState::RefreshInFlight {
            self.store
                .recover_abandoned_refresh(&guard, &wait(control))?;
            return Err(AuthError::RefreshUncertain);
        }
        require_active(&current)?;
        let previous = unauthorized_revision.unwrap_or(observed.identity.credential_revision);
        let revision = current.identity.credential_revision;
        if revision < previous {
            return Err(AuthError::Conflict);
        }
        let now = now_ms()?;
        let peer_committed = revision > previous;
        let sufficient = valid_for(&current, now, minimum_validity_ms);
        let fresh = valid_for(&current, now, REFRESH_SKEW_MS.saturating_add(1));
        if sufficient && (peer_committed || (unauthorized_revision.is_none() && fresh)) {
            let committed = self.store.read_committed(id, revision, &wait(control))?;
            return self.issue(route, control, policy, Some(committed), minimum_validity_ms);
        }

        let ticket = self
            .store
            .begin_refresh(&guard, revision, now, &wait(control))?;
        // Verify the durable attempt before admission. No raw refresh getter is
        // exposed; the only copy produced here is an opaque fixed-endpoint form.
        let request = match self.store.with_refresh_token(
            &guard,
            &ticket,
            &wait(control),
            codex::refresh_request,
        ) {
            Ok(Ok(request)) => request,
            Ok(Err(error)) => {
                self.finish_failure(&guard, &ticket, RefreshResolution::NotSent, control)?;
                return Err(error);
            }
            Err(error) => return Err(error.into()),
        };
        if let Err(error) = control.before_send(HttpRequestKind::AuthRefresh) {
            self.finish_failure(&guard, &ticket, RefreshResolution::NotSent, control)?;
            return Err(control_error(error));
        }
        // Admission hooks can revoke/replace the credential. Recheck after the
        // hook and release the global store lock before entering the transport.
        let result = self
            .store
            .with_refresh_token(&guard, &ticket, &wait(control), |_| send(&request, control));
        let tokens = match result {
            Err(error) => return Err(error.into()),
            Ok(Ok(tokens)) => tokens,
            Ok(Err(error)) => {
                let resolution =
                    if matches!(error, AuthError::LoginRequired | AuthError::AccessDenied) {
                        RefreshResolution::LoginRequired
                    } else {
                        RefreshResolution::Uncertain
                    };
                self.finish_failure(&guard, &ticket, resolution, control)?;
                return Err(match error {
                    AuthError::LoginRequired
                    | AuthError::AccessDenied
                    | AuthError::Cancelled
                    | AuthError::DeadlineExceeded => error,
                    _ => AuthError::RefreshUncertain,
                });
            }
        };
        let committed = match self.store.finish_refresh(
            &guard,
            &ticket,
            RefreshResolution::Tokens(tokens),
            &cleanup_wait(control),
        ) {
            Ok(committed) => committed,
            Err(StoreError::IdentityMismatch) => {
                self.finish_failure(&guard, &ticket, RefreshResolution::LoginRequired, control)?;
                return Err(AuthError::IdentityMismatch);
            }
            Err(error) => {
                if !matches!(error, StoreError::CommitUncertain | StoreError::Conflict) {
                    let _ =
                        self.finish_failure(&guard, &ticket, RefreshResolution::Uncertain, control);
                }
                return Err(error.into());
            }
        };
        // Do not apply the five-minute skew again: one refresh per resolve.
        self.issue(route, control, policy, Some(committed), minimum_validity_ms)
    }

    fn finish_failure(
        &self,
        guard: &RefreshGuard,
        ticket: &RefreshTicket,
        resolution: RefreshResolution,
        control: &RequestControl<'_>,
    ) -> Result<(), AuthError> {
        self.store
            .finish_refresh(guard, ticket, resolution, &cleanup_wait(control))
            .map(|_| ())
            .map_err(Into::into)
    }

    fn issue(
        &self,
        route: &ValidatedRoute,
        control: &RequestControl<'_>,
        policy: String,
        committed: Option<CommittedCredential>,
        minimum_validity_ms: u64,
    ) -> Result<AuthContext, AuthError> {
        control.check_cancelled().map_err(control_error)?;
        let now = now_ms()?;
        let mut expires_at_ms = now
            .checked_add(HANDLE_TTL_MS)
            .ok_or(AuthError::ContextExpired)?;
        let scope = SecretScope {
            route_digest: route.route_digest().into(),
            identity_scope: route.identity_scope().into(),
        };
        let (handle, revocation, revision, account_id) = match committed {
            Some(committed) => {
                let metadata = committed.metadata();
                validate_metadata(route, &metadata)?;
                require_active(&metadata)?;
                if !valid_for(&metadata, now, minimum_validity_ms) {
                    return Err(AuthError::LoginRequired);
                }
                if let Some(expiry) = metadata.expires_at_ms {
                    expires_at_ms = expires_at_ms.min(expiry);
                }
                let (handle, revocation) = committed
                    .with_request_secret(|secret| {
                        SecretHandle::new_scoped(secret, &policy, scope.clone(), expires_at_ms)
                    })?
                    .map_err(secret_error)?;
                (
                    Some(handle),
                    Some(revocation),
                    Some(metadata.identity.credential_revision),
                    metadata.identity.account_id,
                )
            }
            None => (None, None, None, None),
        };
        Ok(AuthContext {
            handle,
            revocation,
            scope,
            owner_id: control.scope.owner_id.clone(),
            session_id: control.scope.session_id.clone(),
            policy_digest: policy,
            credential_revision: revision,
            account_id,
            minimum_validity_ms,
            expires_at_ms,
        })
    }

    pub(crate) fn final_preflight(
        &self,
        route: &ValidatedRoute,
        context: &AuthContext,
        control: &RequestControl<'_>,
    ) -> Result<(), AuthError> {
        let result = (|| {
            let policy = request_policy(route, control)?;
            if context.owner_id != control.scope.owner_id
                || context.session_id != control.scope.session_id
                || context.policy_digest != policy
                || context.scope.route_digest != route.route_digest()
                || context.scope.identity_scope != route.identity_scope()
            {
                return Err(AuthError::ScopeMismatch);
            }
            let id = match route.auth() {
                AuthBinding::Anonymous
                    if context.handle.is_none() && context.credential_revision.is_none() =>
                {
                    None
                }
                AuthBinding::StoredApiKey { credential_id }
                | AuthBinding::CodexOAuth { credential_id } => Some(credential_id),
                _ => return Err(AuthError::ModeMismatch),
            };
            if let Some(id) = id {
                let metadata = self.store.metadata(id, &wait(control))?;
                validate_metadata(route, &metadata)?;
                require_active(&metadata)?;
                if Some(metadata.identity.credential_revision) != context.credential_revision {
                    return Err(AuthError::Conflict);
                }
                if !valid_for(&metadata, now_ms()?, context.minimum_validity_ms) {
                    return Err(AuthError::ContextExpired);
                }
            }
            context.with_secret(&policy, &context.scope, |_| ())
        })();
        if result.is_err() {
            context.revoke();
        }
        normalize_lock_error(result, control)
    }
}

fn request_policy(
    route: &ValidatedRoute,
    control: &RequestControl<'_>,
) -> Result<String, AuthError> {
    control.check_cancelled().map_err(control_error)?;
    let scope = &control.scope;
    if scope.route_digest != route.route_digest()
        || scope.identity_scope != route.identity_scope()
        || [&scope.owner_id, &scope.session_id].iter().any(|value| {
            value.is_empty() || value.len() > 2048 || value.chars().any(char::is_control)
        })
    {
        return Err(AuthError::ScopeMismatch);
    }
    match &scope.policy_digest {
        Some(policy)
            if policy.len() == 64
                && policy
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
        {
            Ok(policy.clone())
        }
        Some(_) => Err(AuthError::ScopeMismatch),
        None if scope.lease_id.is_some() => Err(AuthError::ScopeMismatch),
        None => {
            let bytes =
                serde_json::to_vec(&("zenpi-auth-owner-v1", &scope.owner_id, &scope.session_id))
                    .map_err(|_| AuthError::ScopeMismatch)?;
            Ok(format!("{:x}", Sha256::digest(bytes)))
        }
    }
}

fn validate_metadata(
    route: &ValidatedRoute,
    metadata: &CredentialMetadata,
) -> Result<(), AuthError> {
    validate_binding_metadata(route.auth(), metadata)?;
    if metadata.definition_version != route.definition_version() {
        return Err(AuthError::ModeMismatch);
    }
    let identity = &metadata.identity;
    let bytes = serde_json::to_vec(&(
        &identity.provider,
        &identity.credential_id,
        &identity.account_id,
        &identity.identity_generation,
    ))
    .map_err(|_| AuthError::IdentityMismatch)?;
    if format!("{:x}", Sha256::digest(bytes)) != route.identity_scope() {
        return Err(AuthError::IdentityMismatch);
    }
    revalidate_route_auth(route, identity).map_err(|_| AuthError::DestinationDenied)
}

fn validate_binding_metadata(
    binding: &AuthBinding,
    metadata: &CredentialMetadata,
) -> Result<(), AuthError> {
    let expected_kind = match binding {
        AuthBinding::StoredApiKey { .. } => CredentialKind::ApiKey,
        AuthBinding::CodexOAuth { .. } => CredentialKind::Oauth,
        _ => return Err(AuthError::ModeMismatch),
    };
    if metadata.kind != expected_kind {
        return Err(AuthError::ModeMismatch);
    }
    if metadata.kind == CredentialKind::Oauth
        && (metadata.issuer != definition::ISSUER || metadata.client_id != definition::CLIENT_ID)
    {
        return Err(AuthError::IdentityMismatch);
    }
    Ok(())
}

fn require_active(metadata: &CredentialMetadata) -> Result<(), AuthError> {
    match metadata.state {
        CredentialState::Active => Ok(()),
        CredentialState::Revoked => Err(AuthError::Revoked),
        CredentialState::RefreshInFlight | CredentialState::RefreshUncertain => {
            Err(AuthError::RefreshUncertain)
        }
        CredentialState::LoginRequired => Err(AuthError::LoginRequired),
    }
}

fn valid_for(metadata: &CredentialMetadata, now: u64, minimum: u64) -> bool {
    metadata.kind == CredentialKind::ApiKey
        || metadata.expires_at_ms.is_some_and(|expiry| {
            expiry > now
                && now
                    .checked_add(minimum)
                    .is_some_and(|required| expiry >= required)
        })
}

fn wait<'a>(control: &'a RequestControl<'_>) -> LockWait<'a> {
    let deadline = Instant::now() + Duration::from_secs(10);
    LockWait {
        deadline: control
            .deadline
            .map_or(deadline, |value| value.min(deadline)),
        cancelled: control.cancelled,
    }
}

fn cleanup_wait(control: &RequestControl<'_>) -> LockWait<'static> {
    // Cancellation must not prevent recording an already known remote result.
    // An expired deadline still leaves the durable InFlight marker fail-closed.
    LockWait {
        deadline: wait(control).deadline,
        cancelled: &|| false,
    }
}

fn normalize_lock_error<T>(
    result: Result<T, AuthError>,
    control: &RequestControl<'_>,
) -> Result<T, AuthError> {
    result.map_err(|error| {
        if error == AuthError::LockTimeout
            && control
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
        {
            AuthError::DeadlineExceeded
        } else {
            error
        }
    })
}

fn control_error(error: BackendError) -> AuthError {
    match error {
        BackendError::Cancelled | BackendError::Steered => AuthError::Cancelled,
        BackendError::DeadlineExceeded => AuthError::DeadlineExceeded,
        BackendError::AdmissionDenied(_) => AuthError::SendDenied,
        _ => AuthError::Transport,
    }
}

fn secret_error(error: SecretError) -> AuthError {
    match error {
        SecretError::RevokedOrExpired => AuthError::ContextExpired,
        _ => AuthError::ScopeMismatch,
    }
}

fn now_ms() -> Result<u64, AuthError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| value.as_millis().try_into().ok())
        .ok_or(AuthError::ContextExpired)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::auth::{
        AllowedDestination,
        store::{Mutation, PendingCredential, Replacement},
    };
    use crate::backend::{RequestPurpose, RequestScope};
    use crate::providers::{
        Protocol,
        connection::{ProviderConnection, resolve_connection},
        registry::ModelRegistry,
    };
    use std::cell::Cell;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    };

    fn pending(oauth: bool, expiry: u64) -> PendingCredential {
        PendingCredential {
            kind: if oauth {
                CredentialKind::Oauth
            } else {
                CredentialKind::ApiKey
            },
            provider: if oauth { "openai-codex" } else { "openai" }.into(),
            definition_version: 1,
            issuer: if oauth { definition::ISSUER } else { "" }.into(),
            client_id: if oauth { definition::CLIENT_ID } else { "" }.into(),
            account_id: oauth.then(|| "synthetic-account".into()),
            user_id: oauth.then(|| "synthetic-user".into()),
            allowed_destinations: vec![AllowedDestination {
                origin: if oauth {
                    "https://chatgpt.com"
                } else {
                    "https://api.openai.com"
                }
                .into(),
                path_prefix: if oauth { "/backend-api/codex" } else { "/v1" }.into(),
                protocols: vec![
                    if oauth {
                        "openai_codex_responses"
                    } else {
                        "responses"
                    }
                    .into(),
                ],
                headers: if oauth {
                    vec!["authorization".into(), "chatgpt-account-id".into()]
                } else {
                    vec!["authorization".into()]
                },
            }],
            api_key: (!oauth).then(|| "synthetic-api-key".into()),
            access_token: oauth.then(|| "synthetic-access".into()),
            refresh_token: oauth.then(|| "synthetic-refresh".into()),
            expires_at_ms: oauth.then_some(expiry),
        }
    }
    fn tokens(ttl: u64) -> RefreshTokens {
        RefreshTokens {
            issuer: definition::ISSUER.into(),
            client_id: definition::CLIENT_ID.into(),
            account_id: "synthetic-account".into(),
            user_id: Some("synthetic-user".into()),
            access_token: "synthetic-new-access".into(),
            refresh_token: None,
            expires_at_ms: now_ms().unwrap() + ttl,
        }
    }
    fn fixture(oauth: bool, ttl: u64) -> (tempfile::TempDir, CredentialStore, ValidatedRoute) {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let store =
            CredentialStore::new(dir.path().canonicalize().unwrap().join("auth.json")).unwrap();
        let identity = store
            .modify(
                "credential",
                None,
                Mutation::Replace(Replacement::Login(pending(oauth, now_ms().unwrap() + ttl))),
                &LockWait::default(),
            )
            .unwrap()
            .identity()
            .unwrap();
        let connection = ProviderConnection {
            profile: "synthetic".into(),
            provider: identity.provider.clone(),
            protocol: if oauth {
                Protocol::OpenAiCodexResponses
            } else {
                Protocol::Responses
            },
            base_url: None,
            auth: if oauth {
                AuthBinding::CodexOAuth {
                    credential_id: "credential".into(),
                }
            } else {
                AuthBinding::StoredApiKey {
                    credential_id: "credential".into(),
                }
            },
            header_policy: None,
            config_revision: 1,
            model_routes: vec![],
        };
        let route = resolve_connection(
            &connection,
            "test-model",
            &ModelRegistry::default(),
            Some(&identity),
            true,
        )
        .unwrap();
        (dir, store, route)
    }
    fn scope(route: &ValidatedRoute) -> RequestScope {
        RequestScope {
            owner_id: "owner".into(),
            session_id: "session".into(),
            operation_id: "operation".into(),
            purpose: RequestPurpose::Turn,
            route_digest: route.route_digest().into(),
            identity_scope: route.identity_scope().into(),
            policy_digest: None,
            lease_id: None,
        }
    }
    fn control<'a>(
        route: &ValidatedRoute,
        sends: &'a mut dyn FnMut(HttpRequestKind, &RequestScope) -> Result<(), BackendError>,
    ) -> RequestControl<'a> {
        RequestControl {
            cancelled: &|| false,
            deadline: Some(Instant::now() + Duration::from_secs(5)),
            scope: scope(route),
            before_send: sends,
        }
    }
    fn no_send(
        _: &codex::RefreshRequest,
        _: &RequestControl<'_>,
    ) -> Result<RefreshTokens, AuthError> {
        panic!("unexpected token send")
    }
    fn metadata(store: &CredentialStore) -> CredentialMetadata {
        store.metadata("credential", &LockWait::default()).unwrap()
    }
    fn secret(context: &AuthContext) -> String {
        context
            .with_secret(context.policy_digest(), &context.scope, |value| {
                value.unwrap().to_owned()
            })
            .unwrap()
    }

    #[test]
    fn stored_key_uses_scoped_committed_handle_and_drop_revokes() {
        let (_dir, store, route) = fixture(false, 0);
        let resolver = AuthResolver::new(store);
        let mut admission = |_, _: &RequestScope| panic!("key resolution has no HTTP");
        let mut control = control(&route, &mut admission);
        let context = resolver.resolve(&route, &mut control, u64::MAX).unwrap();
        assert_eq!(secret(&context), "synthetic-api-key");
        assert_eq!(context.credential_revision(), Some(1));
        assert_eq!(context.account_id(), None);
        assert_eq!(resolver.final_preflight(&route, &context, &control), Ok(()));
        let leaked = context.handle.as_ref().unwrap().clone();
        let policy = context.policy_digest.clone();
        let scope = context.scope.clone();
        drop(context);
        assert_eq!(
            leaked.verify_scope(&policy, &scope),
            Err(SecretError::RevokedOrExpired)
        );
    }

    #[test]
    fn anonymous_never_reads_store_and_only_it_yields_none() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::new(dir.path().join("never-created/auth.json")).unwrap();
        let connection = ProviderConnection {
            profile: "local".into(),
            provider: "local".into(),
            protocol: Protocol::Responses,
            base_url: Some("http://127.0.0.1:3456/v1".into()),
            auth: AuthBinding::Anonymous,
            header_policy: None,
            config_revision: 1,
            model_routes: vec![],
        };
        let route = resolve_connection(
            &connection,
            "test-model",
            &ModelRegistry::default(),
            None,
            true,
        )
        .unwrap();
        let resolver = AuthResolver::new(store);
        let mut admission = |_, _: &RequestScope| panic!("anonymous auth has no HTTP");
        let mut control = control(&route, &mut admission);
        let context = resolver.resolve(&route, &mut control, 0).unwrap();
        assert!(
            context
                .with_secret(context.policy_digest(), &context.scope, |value| value
                    .is_none())
                .unwrap()
        );
        assert_eq!(resolver.final_preflight(&route, &context, &control), Ok(()));
        assert!(!dir.path().join("never-created").exists());
    }

    #[test]
    fn scope_owner_and_worker_policy_are_enforced() {
        let (_dir, store, route) = fixture(false, 0);
        let resolver = AuthResolver::new(store);
        let mut admission = |_, _: &RequestScope| Ok(());
        let mut control = control(&route, &mut admission);
        let context = resolver.resolve(&route, &mut control, 0).unwrap();
        assert_eq!(
            context.with_secret(&"a".repeat(64), &context.scope, |_| ()),
            Err(AuthError::ScopeMismatch)
        );
        control.scope.owner_id = "other-owner".into();
        assert_eq!(
            resolver.final_preflight(&route, &context, &control),
            Err(AuthError::ScopeMismatch)
        );
        assert_eq!(
            context.with_secret(context.policy_digest(), &context.scope, |_| ()),
            Err(AuthError::ContextExpired)
        );
        control.scope.lease_id = Some("worker-lease".into());
        assert!(matches!(
            resolver.resolve(&route, &mut control, 0),
            Err(AuthError::ScopeMismatch)
        ));
        control.scope.policy_digest = Some("b".repeat(64));
        let worker = resolver.resolve(&route, &mut control, 0).unwrap();
        assert_eq!(worker.policy_digest(), "b".repeat(64));
        control.scope.identity_scope = "wrong".into();
        assert!(matches!(
            resolver.resolve(&route, &mut control, 0),
            Err(AuthError::ScopeMismatch)
        ));
    }

    #[test]
    fn fresh_oauth_uses_no_send_and_short_refreshed_token_is_not_refreshed_twice() {
        let (_dir, store, route) = fixture(true, REFRESH_SKEW_MS + 60_000);
        let resolver = AuthResolver::new(store);
        let mut admission = |_, _: &RequestScope| panic!("fresh auth has no HTTP");
        let mut control = control(&route, &mut admission);
        let context = resolver
            .resolve_with(&route, &mut control, 1000, None, &mut no_send)
            .unwrap();
        assert_eq!(secret(&context), "synthetic-access");
        assert_eq!(context.account_id(), Some("synthetic-account"));

        let (_dir, store, route) = fixture(true, REFRESH_SKEW_MS);
        let resolver = AuthResolver::new(store.clone());
        let sends = Cell::new(0);
        let mut admission = |kind, _: &RequestScope| {
            assert_eq!(kind, HttpRequestKind::AuthRefresh);
            sends.set(sends.get() + 1);
            Ok(())
        };
        let mut control = self::control(&route, &mut admission);
        let mut http = |_: &codex::RefreshRequest, _: &RequestControl<'_>| Ok(tokens(30_000));
        let context = resolver
            .resolve_with(&route, &mut control, 1000, None, &mut http)
            .unwrap();
        assert_eq!(sends.get(), 1);
        assert_eq!(context.credential_revision(), Some(3));
        assert_eq!(secret(&context), "synthetic-new-access");
        assert_eq!(metadata(&store).state, CredentialState::Active);
        assert!(context.expires_at_ms <= metadata(&store).expires_at_ms.unwrap());
    }

    #[test]
    fn refreshed_token_must_meet_minimum_without_looping() {
        let (_dir, store, route) = fixture(true, 1);
        let resolver = AuthResolver::new(store.clone());
        let sends = Cell::new(0);
        let mut admission = |_, _: &RequestScope| {
            sends.set(sends.get() + 1);
            Ok(())
        };
        let mut control = control(&route, &mut admission);
        let mut http = |_: &codex::RefreshRequest, _: &RequestControl<'_>| Ok(tokens(2000));
        assert!(matches!(
            resolver.resolve_with(&route, &mut control, 5000, None, &mut http),
            Err(AuthError::LoginRequired)
        ));
        assert_eq!(sends.get(), 1);
        assert_eq!(metadata(&store).state, CredentialState::Active);
    }

    #[test]
    fn unauthorized_reloads_peer_revision_without_refresh() {
        let (_dir, store, route) = fixture(true, 1000);
        let guard = store
            .lock_refresh("credential", &LockWait::default())
            .unwrap();
        let ticket = store
            .begin_refresh(&guard, 1, now_ms().unwrap(), &LockWait::default())
            .unwrap();
        store
            .finish_refresh(
                &guard,
                &ticket,
                RefreshResolution::Tokens(tokens(30_000)),
                &LockWait::default(),
            )
            .unwrap();
        drop(guard);
        let resolver = AuthResolver::new(store);
        let mut admission = |_, _: &RequestScope| panic!("peer reload has no HTTP");
        let mut control = control(&route, &mut admission);
        let context = resolver
            .recover_unauthorized(&route, 1, &mut control, 1000)
            .unwrap();
        assert_eq!(context.credential_revision(), Some(3));
        assert_eq!(secret(&context), "synthetic-new-access");
    }

    #[test]
    fn unauthorized_forces_one_refresh_but_never_refreshes_api_keys() {
        let (_dir, store, route) = fixture(true, 600_000);
        let resolver = AuthResolver::new(store);
        let count = Cell::new(0);
        let mut admission = |_, _: &RequestScope| {
            count.set(count.get() + 1);
            Ok(())
        };
        let mut control = control(&route, &mut admission);
        let context = resolver
            .resolve_with(&route, &mut control, 1000, Some(1), &mut |_, _| {
                Ok(tokens(60_000))
            })
            .unwrap();
        assert_eq!(context.credential_revision(), Some(3));
        assert_eq!(count.get(), 1);
        let (_dir, store, route) = fixture(false, 0);
        let resolver = AuthResolver::new(store);
        assert!(matches!(
            resolver.recover_unauthorized(&route, 1, &mut control, 0),
            Err(AuthError::LoginRequired)
        ));
    }

    #[test]
    fn replaced_identity_never_follows_new_login() {
        let (_dir, store, route) = fixture(true, 1000);
        store
            .modify(
                "credential",
                Some(1),
                Mutation::Replace(Replacement::Login(pending(
                    true,
                    now_ms().unwrap() + 600_000,
                ))),
                &LockWait::default(),
            )
            .unwrap();
        let resolver = AuthResolver::new(store);
        let mut admission = |_, _: &RequestScope| panic!("wrong identity has no HTTP");
        let mut control = control(&route, &mut admission);
        assert!(matches!(
            resolver.resolve_with(&route, &mut control, 0, None, &mut no_send),
            Err(AuthError::IdentityMismatch)
        ));
    }

    #[test]
    fn final_preflight_rejects_revoke_revision_and_destination_changes() {
        for mutation in 0..3 {
            let (_dir, store, route) = fixture(false, 0);
            let resolver = AuthResolver::new(store.clone());
            let mut admission = |_, _: &RequestScope| Ok(());
            let mut control = control(&route, &mut admission);
            let context = resolver.resolve(&route, &mut control, 0).unwrap();
            let (change, expected) = match mutation {
                0 => (Mutation::Revoke, AuthError::Revoked),
                1 => (
                    Mutation::Replace(Replacement::AllowedDestinations(
                        metadata(&store).identity.allowed_destinations,
                    )),
                    AuthError::Conflict,
                ),
                _ => (
                    Mutation::Replace(Replacement::AllowedDestinations(vec![])),
                    AuthError::DestinationDenied,
                ),
            };
            store
                .modify("credential", Some(1), change, &LockWait::default())
                .unwrap();
            assert_eq!(
                resolver.final_preflight(&route, &context, &control),
                Err(expected)
            );
            assert_eq!(
                context.with_secret(context.policy_digest(), &context.scope, |_| ()),
                Err(AuthError::ContextExpired)
            );
        }
    }

    #[test]
    fn admission_denial_or_cancellation_clears_only_unsent_marker() {
        for cancel in [false, true] {
            let (_dir, store, route) = fixture(true, 1000);
            let resolver = AuthResolver::new(store.clone());
            let cancelled = Cell::new(false);
            let check = || cancelled.get();
            let mut admission = |_, _: &RequestScope| {
                if cancel {
                    cancelled.set(true);
                    Ok(())
                } else {
                    Err(BackendError::AdmissionDenied("synthetic".into()))
                }
            };
            let mut control = control(&route, &mut admission);
            control.cancelled = &check;
            let expected = if cancel {
                AuthError::Cancelled
            } else {
                AuthError::SendDenied
            };
            assert!(
                matches!(resolver.resolve_with(&route, &mut control, 0, None, &mut no_send), Err(error) if error == expected)
            );
            assert_eq!(metadata(&store).state, CredentialState::Active);
            assert_eq!(metadata(&store).identity.credential_revision, 3);
        }
    }

    #[test]
    fn dispatched_failure_is_uncertain_and_never_retries_old_refresh() {
        for error in [
            AuthError::Transport,
            AuthError::InvalidResponse,
            AuthError::Cancelled,
        ] {
            let (_dir, store, route) = fixture(true, 1000);
            let resolver = AuthResolver::new(store.clone());
            let mut admission = |_, _: &RequestScope| Ok(());
            let mut control = control(&route, &mut admission);
            let result =
                resolver.resolve_with(&route, &mut control, 0, None, &mut |_, _| Err(error));
            assert!(
                matches!(result, Err(actual) if actual == if error == AuthError::Cancelled { error } else { AuthError::RefreshUncertain })
            );
            assert_eq!(metadata(&store).state, CredentialState::RefreshUncertain);
            assert!(matches!(
                resolver.resolve_with(&route, &mut control, 0, None, &mut no_send),
                Err(AuthError::RefreshUncertain)
            ));
        }
    }

    #[test]
    fn abandoned_marker_is_uncertain_not_replayed() {
        let (_dir, store, route) = fixture(true, 600_000);
        let guard = store
            .lock_refresh("credential", &LockWait::default())
            .unwrap();
        store
            .begin_refresh(&guard, 1, now_ms().unwrap(), &LockWait::default())
            .unwrap();
        drop(guard);
        let resolver = AuthResolver::new(store.clone());
        let mut admission = |_, _: &RequestScope| panic!("abandoned attempt has no HTTP");
        let mut control = control(&route, &mut admission);
        assert!(matches!(
            resolver.resolve_with(&route, &mut control, 0, None, &mut no_send),
            Err(AuthError::RefreshUncertain)
        ));
        assert_eq!(metadata(&store).state, CredentialState::RefreshUncertain);
    }

    #[test]
    fn invalid_grant_and_changed_refresh_identity_require_login() {
        for identity in [false, true] {
            let (_dir, store, route) = fixture(true, 1000);
            let resolver = AuthResolver::new(store.clone());
            let mut admission = |_, _: &RequestScope| Ok(());
            let mut control = control(&route, &mut admission);
            let mut http = |_: &codex::RefreshRequest, _: &RequestControl<'_>| {
                if !identity {
                    return Err(AuthError::LoginRequired);
                }
                let mut changed = tokens(60_000);
                changed.account_id = "other-account".into();
                Ok(changed)
            };
            let expected = if identity {
                AuthError::IdentityMismatch
            } else {
                AuthError::LoginRequired
            };
            assert!(
                matches!(resolver.resolve_with(&route, &mut control, 0, None, &mut http), Err(error) if error == expected)
            );
            assert_eq!(metadata(&store).state, CredentialState::LoginRequired);
        }
    }

    #[test]
    fn concurrent_revoke_after_admission_prevents_token_http() {
        let (_dir, store, route) = fixture(true, 1000);
        let resolver = AuthResolver::new(store.clone());
        let mut admission = |_, _: &RequestScope| {
            store
                .modify(
                    "credential",
                    Some(2),
                    Mutation::Revoke,
                    &LockWait::default(),
                )
                .unwrap();
            Ok(())
        };
        let mut control = control(&route, &mut admission);
        assert!(matches!(
            resolver.resolve_with(&route, &mut control, 0, None, &mut no_send),
            Err(AuthError::Conflict)
        ));
        assert_eq!(metadata(&store).state, CredentialState::Revoked);
    }

    #[test]
    fn network_does_not_hold_global_lock_and_cas_cannot_overwrite_revoke_or_login() {
        for login in [false, true] {
            let (_dir, store, route) = fixture(true, 1000);
            let resolver = AuthResolver::new(store.clone());
            let mut admission = |_, _: &RequestScope| Ok(());
            let mut control = control(&route, &mut admission);
            let mut http = |_: &codex::RefreshRequest, _: &RequestControl<'_>| {
                let change = if login {
                    Mutation::Replace(Replacement::Login(pending(
                        true,
                        now_ms().unwrap() + 600_000,
                    )))
                } else {
                    Mutation::Revoke
                };
                store
                    .modify("credential", Some(2), change, &LockWait::default())
                    .unwrap();
                Ok(tokens(60_000))
            };
            assert!(matches!(
                resolver.resolve_with(&route, &mut control, 0, None, &mut http),
                Err(AuthError::Conflict)
            ));
            assert_eq!(metadata(&store).identity.credential_revision, 3);
            assert_eq!(
                metadata(&store).state,
                if login {
                    CredentialState::Active
                } else {
                    CredentialState::Revoked
                }
            );
        }
    }

    #[test]
    fn marker_commit_uncertainty_and_token_persistence_failure_never_mint_handles() {
        let (_dir, store, route) = fixture(true, 1000);
        let resolver = AuthResolver::new(store.with_uncertain_commit_for_test());
        let mut admission = |_, _: &RequestScope| panic!("uncertain marker has no HTTP");
        let mut control = control(&route, &mut admission);
        assert!(matches!(
            resolver.resolve_with(&route, &mut control, 0, None, &mut no_send),
            Err(AuthError::CommitUncertain)
        ));
        assert_eq!(metadata(&store).state, CredentialState::RefreshInFlight);

        let (dir, store, route) = fixture(true, 1000);
        let resolver = AuthResolver::new(store.clone());
        let path = dir.path().join("auth.json");
        let mut admission = |_, _: &RequestScope| Ok(());
        let mut control = self::control(&route, &mut admission);
        let mut http = |_: &codex::RefreshRequest, _: &RequestControl<'_>| {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
            Ok(tokens(60_000))
        };
        assert!(matches!(
            resolver.resolve_with(&route, &mut control, 0, None, &mut http),
            Err(AuthError::StorageFailed)
        ));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(metadata(&store).state, CredentialState::RefreshInFlight);
    }

    #[test]
    fn deadline_and_cancel_before_resolve_have_zero_store_changes() {
        let (_dir, store, route) = fixture(true, 1000);
        let resolver = AuthResolver::new(store.clone());
        let mut admission = |_, _: &RequestScope| panic!("cancelled request has no HTTP");
        let mut control = control(&route, &mut admission);
        control.deadline = Some(Instant::now());
        assert!(matches!(
            resolver.resolve_with(&route, &mut control, 0, None, &mut no_send),
            Err(AuthError::DeadlineExceeded)
        ));
        control.cancelled = &|| true;
        assert!(matches!(
            resolver.resolve_with(&route, &mut control, 0, None, &mut no_send),
            Err(AuthError::Cancelled)
        ));
        assert_eq!(metadata(&store).identity.credential_revision, 1);
    }

    #[test]
    fn concurrent_resolvers_wait_for_peer_and_send_once() {
        let (_dir, store, route) = fixture(true, 1000);
        let started = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let sends = Arc::new(AtomicUsize::new(0));
        let first = {
            let (store, route, started, release, sends) = (
                store.clone(),
                route.clone(),
                started.clone(),
                release.clone(),
                sends.clone(),
            );
            std::thread::spawn(move || {
                let resolver = AuthResolver::new(store);
                let mut admission = |_, _: &RequestScope| {
                    sends.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                };
                let mut control = control(&route, &mut admission);
                resolver
                    .resolve_with(&route, &mut control, 1000, None, &mut |_, _| {
                        started.wait();
                        release.wait();
                        Ok(tokens(30_000))
                    })
                    .unwrap()
                    .credential_revision()
            })
        };
        started.wait();
        let second = {
            let (store, route, sends) = (store.clone(), route.clone(), sends.clone());
            std::thread::spawn(move || {
                let resolver = AuthResolver::new(store);
                let mut admission = |_, _: &RequestScope| {
                    sends.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                };
                let mut control = control(&route, &mut admission);
                resolver
                    .resolve_with(&route, &mut control, 1000, Some(1), &mut no_send)
                    .unwrap()
                    .credential_revision()
            })
        };
        release.wait();
        assert_eq!(first.join().unwrap(), Some(3));
        assert_eq!(second.join().unwrap(), Some(3));
        assert_eq!(sends.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn validity_arithmetic_and_lock_deadlines_are_bounded() {
        let (_dir, store, route) = fixture(true, 600_000);
        let mut metadata = metadata(&store);
        metadata.expires_at_ms = Some(301_000);
        assert!(valid_for(&metadata, 1000, REFRESH_SKEW_MS));
        assert!(!valid_for(&metadata, 1000, REFRESH_SKEW_MS + 1));
        assert!(!valid_for(&metadata, 301_000, 0));
        assert!(!valid_for(&metadata, 1000, u64::MAX));
        let mut admission = |_, _: &RequestScope| Ok(());
        let mut control = control(&route, &mut admission);
        control.deadline = Some(Instant::now() + Duration::from_secs(3600));
        assert!(wait(&control).deadline <= Instant::now() + Duration::from_secs(10));
        control.deadline = Some(Instant::now() + Duration::from_millis(20));
        assert_eq!(wait(&control).deadline, control.deadline.unwrap());
        control.deadline = Some(Instant::now());
        assert_eq!(
            normalize_lock_error::<()>(Err(AuthError::LockTimeout), &control),
            Err(AuthError::DeadlineExceeded)
        );
    }

    #[test]
    fn lock_wait_obeys_shorter_request_deadline_without_sending() {
        let (_dir, store, route) = fixture(true, 1000);
        let _guard = store
            .lock_refresh("credential", &LockWait::default())
            .unwrap();
        let resolver = AuthResolver::new(store.clone());
        let mut admission = |_, _: &RequestScope| panic!("lock waiting does not send");
        let mut control = control(&route, &mut admission);
        control.deadline = Some(Instant::now() + Duration::from_millis(30));
        let start = Instant::now();
        assert!(matches!(
            resolver.resolve(&route, &mut control, 0),
            Err(AuthError::DeadlineExceeded)
        ));
        assert!(start.elapsed() < Duration::from_secs(1));
        assert_eq!(metadata(&store).identity.credential_revision, 1);
    }

    #[test]
    fn refreshed_commit_uncertainty_only_allows_guarded_reload_not_token_replay() {
        let (_dir, store, route) = fixture(true, 1000);
        let resolver = AuthResolver::new(store.with_uncertain_revision_for_test(3));
        let sends = Cell::new(0);
        let mut admission = |_, _: &RequestScope| {
            sends.set(sends.get() + 1);
            Ok(())
        };
        let mut control = control(&route, &mut admission);
        assert!(matches!(
            resolver.resolve_with(&route, &mut control, 0, None, &mut |_, _| Ok(tokens(
                600_000
            ))),
            Err(AuthError::CommitUncertain)
        ));
        assert_eq!(sends.get(), 1);
        assert_eq!(metadata(&store).state, CredentialState::Active);
        let reloaded = resolver
            .recover_unauthorized(&route, 1, &mut control, 1000)
            .unwrap();
        assert_eq!(reloaded.credential_revision(), Some(3));
        assert_eq!(secret(&reloaded), "synthetic-new-access");
        assert_eq!(sends.get(), 1);
    }

    #[test]
    fn peer_relogin_is_not_disabled_by_invalid_grant_from_old_attempt() {
        let (_dir, store, route) = fixture(true, 1000);
        let resolver = AuthResolver::new(store.clone());
        let mut admission = |_, _: &RequestScope| Ok(());
        let mut control = control(&route, &mut admission);
        let mut send = |_: &codex::RefreshRequest, _: &RequestControl<'_>| {
            store
                .modify(
                    "credential",
                    Some(2),
                    Mutation::Replace(Replacement::Login(pending(
                        true,
                        now_ms().unwrap() + 600_000,
                    ))),
                    &LockWait::default(),
                )
                .unwrap();
            Err(AuthError::LoginRequired)
        };
        assert!(matches!(
            resolver.resolve_with(&route, &mut control, 0, None, &mut send),
            Err(AuthError::Conflict)
        ));
        assert_eq!(metadata(&store).state, CredentialState::Active);
        assert_eq!(metadata(&store).identity.credential_revision, 3);
    }

    #[test]
    fn context_and_committed_credentials_have_no_secret_debug_or_serialization() {
        macro_rules! no_trait {
            ($ty:ty, $bound:path) => {{
                trait Ambiguous<A> {
                    fn check() {}
                }
                impl<T: ?Sized> Ambiguous<()> for T {}
                struct Bound;
                impl<T: ?Sized + $bound> Ambiguous<Bound> for T {}
                let _ = <$ty as Ambiguous<_>>::check;
            }};
        }
        no_trait!(AuthContext, std::fmt::Debug);
        no_trait!(AuthContext, serde::Serialize);
        no_trait!(AuthContext, Clone);
        no_trait!(CommittedCredential, std::fmt::Debug);
        no_trait!(CommittedCredential, serde::Serialize);
        assert_eq!(AuthError::RefreshUncertain.code(), "auth_refresh_uncertain");
        assert_eq!(AuthError::Revoked.code(), "auth_revoked");
        assert_eq!(AuthError::IdentityMismatch.code(), "auth_identity_mismatch");
    }

    #[test]
    fn initial_anonymous_and_legacy_do_not_read_or_create_store() {
        let directory = tempfile::tempdir().unwrap();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory
            .path()
            .canonicalize()
            .unwrap()
            .join("not-created/auth.json");
        let resolver = AuthResolver::new(CredentialStore::new(path).unwrap());
        assert_eq!(resolver.initial_identity(&AuthBinding::Anonymous), Ok(None));
        assert_eq!(
            resolver.initial_identity(&AuthBinding::LegacyApiKey),
            Err(AuthError::ModeMismatch)
        );
        assert_eq!(
            resolver.initial_identity(&AuthBinding::StoredApiKey {
                credential_id: "missing".into()
            }),
            Err(AuthError::NotConfigured)
        );
        assert!(!directory.path().join("not-created").exists());
    }

    #[test]
    fn initial_identity_preserves_revoked_uncertain_and_login_required() {
        for state in [
            CredentialState::Revoked,
            CredentialState::RefreshUncertain,
            CredentialState::LoginRequired,
        ] {
            let (_directory, store, route) = fixture(true, 1000);
            let expected = match state {
                CredentialState::Revoked => {
                    store
                        .modify(
                            "credential",
                            Some(1),
                            Mutation::Revoke,
                            &LockWait::default(),
                        )
                        .unwrap();
                    AuthError::Revoked
                }
                _ => {
                    let guard = store
                        .lock_refresh("credential", &LockWait::default())
                        .unwrap();
                    let ticket = store
                        .begin_refresh(&guard, 1, now_ms().unwrap(), &LockWait::default())
                        .unwrap();
                    let (resolution, error) = if state == CredentialState::RefreshUncertain {
                        (RefreshResolution::Uncertain, AuthError::RefreshUncertain)
                    } else {
                        (RefreshResolution::LoginRequired, AuthError::LoginRequired)
                    };
                    store
                        .finish_refresh(&guard, &ticket, resolution, &LockWait::default())
                        .unwrap();
                    error
                }
            };
            let before = metadata(&store);
            let resolver = AuthResolver::new(store.clone());
            assert_eq!(resolver.initial_identity(route.auth()), Err(expected));
            let after = metadata(&store);
            assert_eq!(after.state, state);
            assert_eq!(after.identity, before.identity);
        }
    }

    #[test]
    fn initial_active_and_peer_inflight_snapshots_do_not_refresh_or_wait_for_account_lock() {
        let (_directory, store, route) = fixture(true, 1);
        let resolver = AuthResolver::new(store.clone());
        let initial = resolver.initial_identity(route.auth()).unwrap().unwrap();
        assert_eq!(initial.credential_revision, 1);
        let guard = store
            .lock_refresh("credential", &LockWait::default())
            .unwrap();
        store
            .begin_refresh(&guard, 1, now_ms().unwrap(), &LockWait::default())
            .unwrap();
        let start = Instant::now();
        let inflight = resolver.initial_identity(route.auth()).unwrap().unwrap();
        assert!(start.elapsed() < Duration::from_secs(1));
        assert_eq!(inflight.credential_revision, 2);
        assert_eq!(inflight.identity_generation, initial.identity_generation);
        assert_eq!(metadata(&store).state, CredentialState::RefreshInFlight);

        let (_directory, store, route) = fixture(false, 0);
        let resolver = AuthResolver::new(store);
        assert_eq!(
            resolver
                .initial_identity(route.auth())
                .unwrap()
                .unwrap()
                .credential_revision,
            1
        );
    }

    #[test]
    fn initial_identity_checks_binding_kind_and_fixed_oauth_issuer_client() {
        for case in 0..4 {
            let oauth = case != 1;
            let (_directory, store, route) = fixture(oauth, 600_000);
            let binding = if case == 0 {
                AuthBinding::StoredApiKey {
                    credential_id: "credential".into(),
                }
            } else if case == 1 {
                AuthBinding::CodexOAuth {
                    credential_id: "credential".into(),
                }
            } else {
                let mut replacement = pending(true, now_ms().unwrap() + 600_000);
                if case == 2 {
                    replacement.issuer = "https://other-issuer.invalid".into();
                } else {
                    replacement.client_id = "other-client".into();
                }
                store
                    .modify(
                        "credential",
                        Some(1),
                        Mutation::Replace(Replacement::Login(replacement)),
                        &LockWait::default(),
                    )
                    .unwrap();
                route.auth().clone()
            };
            let resolver = AuthResolver::new(store);
            assert_eq!(
                resolver.initial_identity(&binding),
                Err(if case < 2 {
                    AuthError::ModeMismatch
                } else {
                    AuthError::IdentityMismatch
                })
            );
        }
    }
}
