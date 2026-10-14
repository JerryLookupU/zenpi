//! Host-owned persistent device permissions and connection-bound proof of possession.

use super::{pairing::unix_seconds, protocol::Operation};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use thiserror::Error;

const MAX_DEVICES: usize = 128;
const MAX_REGISTRY_BYTES: u64 = 256 * 1024;
const MAX_PROJECTS: usize = 128;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("device registry I/O: {0}")]
    Io(#[from] io::Error),
    #[error("device registry unavailable or malformed")]
    InvalidRegistry,
    #[error("invalid device identity or proof")]
    InvalidProof,
    #[error("device unauthorized or revoked")]
    Unauthorized,
    #[error("device registry is busy")]
    Busy,
    #[error("device registry limit reached")]
    Limit,
}

impl AuthError {
    /// Contention is not evidence that a valid device lost its permissions.
    pub fn remote_code(&self) -> super::protocol::ErrorCode {
        if matches!(self, Self::Busy) {
            super::protocol::ErrorCode::Busy
        } else {
            super::protocol::ErrorCode::Unauthorized
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Read,
    Control,
    Approval,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeviceRecord {
    pub id: String,
    pub name: String,
    pub public_key: String,
    pub created_at: u64,
    pub revision: u64,
    pub revoked: bool,
    pub capabilities: Vec<Capability>,
    pub projects: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoke_request_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryData {
    version: u16,
    host_fingerprint: String,
    devices: Vec<DeviceRecord>,
}

/// Returned only after proof and current durable registry admission. Not a wire credential.
#[derive(Debug, Clone)]
pub struct AuthenticatedDevice {
    id: String,
    revision: u64,
}
pub(crate) struct AuthorizedProjects(DeviceRecord);
impl AuthorizedProjects {
    pub(crate) fn authorize(&self, project: &str, operation: Operation) -> Result<(), AuthError> {
        authorize_record(&self.0, project, operation)
    }
}
fn authorize_record(
    record: &DeviceRecord,
    project: &str,
    operation: Operation,
) -> Result<(), AuthError> {
    let capability = if operation == Operation::Approve {
        Capability::Approval
    } else if operation.is_write() {
        Capability::Control
    } else {
        Capability::Read
    };
    if record.revoked
        || !record.projects.iter().any(|value| value == project)
        || !record.capabilities.contains(&capability)
    {
        return Err(AuthError::Unauthorized);
    }
    Ok(())
}
impl AuthenticatedDevice {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

pub use crate::protocol::valid_identifier;

fn decode_key(public_key: &str) -> Result<Vec<u8>, AuthError> {
    if public_key.len() != 87 {
        return Err(AuthError::InvalidProof);
    }
    let key = URL_SAFE_NO_PAD
        .decode(public_key)
        .map_err(|_| AuthError::InvalidProof)?;
    if key.len() != 65 || key[0] != 4 || URL_SAFE_NO_PAD.encode(&key) != public_key {
        return Err(AuthError::InvalidProof);
    }
    Ok(key)
}

pub fn device_id(public_key: &str) -> Result<String, AuthError> {
    Ok(format!("{:x}", Sha256::digest(decode_key(public_key)?)))
}

fn verify(public_key: &str, message: &[u8], signature: &str) -> Result<(), AuthError> {
    let key = decode_key(public_key)?;
    if signature.len() > 104 {
        return Err(AuthError::InvalidProof);
    }
    let signature = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| AuthError::InvalidProof)?;
    ring::signature::UnparsedPublicKey::new(&ring::signature::ECDSA_P256_SHA256_ASN1, key)
        .verify(message, &signature)
        .map_err(|_| AuthError::InvalidProof)
}

pub fn pairing_proof_message(
    fingerprint: &str,
    token: &str,
    request_id: &str,
    name: &str,
    public_key: &str,
) -> Vec<u8> {
    serde_json::to_vec(&[
        "zenpi-device-pair-v1",
        fingerprint,
        token,
        request_id,
        name,
        public_key,
    ])
    .expect("string array serialization")
}

pub fn verify_pairing_proof(
    fingerprint: &str,
    token: &str,
    request_id: &str,
    name: &str,
    public_key: &str,
    signature: &str,
) -> Result<(), AuthError> {
    if !valid_identifier(request_id) || !valid_identifier(name) {
        return Err(AuthError::InvalidProof);
    }
    verify(
        public_key,
        &pairing_proof_message(fingerprint, token, request_id, name, public_key),
        signature,
    )
}

/// The nonce is created per TLS connection and consumed even on a failed attempt.
pub struct DeviceChallenge {
    nonce: String,
    fingerprint: String,
    deadline: Instant,
    consumed: bool,
}
impl DeviceChallenge {
    pub fn new(fingerprint: String) -> Result<Self, AuthError> {
        let mut nonce = [0; 32];
        getrandom::getrandom(&mut nonce).map_err(|_| AuthError::InvalidProof)?;
        Ok(Self {
            nonce: URL_SAFE_NO_PAD.encode(nonce),
            fingerprint,
            deadline: Instant::now() + Duration::from_secs(30),
            consumed: false,
        })
    }
    pub fn nonce(&self) -> &str {
        &self.nonce
    }
    pub fn proof_message(&self, device_id: &str) -> Vec<u8> {
        serde_json::to_vec(&[
            "zenpi-device-auth-v1",
            &self.fingerprint,
            &self.nonce,
            device_id,
        ])
        .expect("string array")
    }
    pub fn authenticate(
        &mut self,
        registry: &DeviceRegistry,
        id: &str,
        signature: &str,
    ) -> Result<AuthenticatedDevice, AuthError> {
        if self.consumed || Instant::now() >= self.deadline {
            return Err(AuthError::Unauthorized);
        }
        self.consumed = true;
        let record = registry.lookup(id)?;
        if record.revoked {
            return Err(AuthError::Unauthorized);
        }
        verify(&record.public_key, &self.proof_message(id), signature)?;
        Ok(AuthenticatedDevice {
            id: record.id,
            revision: record.revision,
        })
    }
    pub fn revoke_status_proof(&self, device: &str, request: &str, query: &str) -> Vec<u8> {
        serde_json::to_vec(&[
            "zenpi-device-revoke-status-v1",
            &self.fingerprint,
            &self.nonce,
            device,
            request,
            query,
        ])
        .expect("string array")
    }
    /// A revoked key may prove possession for this receipt only, never authenticate.
    pub fn revoke_status(
        &mut self,
        registry: &DeviceRegistry,
        device: &str,
        request: &str,
        query: &str,
        signature: &str,
    ) -> Result<bool, AuthError> {
        if self.consumed
            || Instant::now() >= self.deadline
            || !valid_identifier(request)
            || !valid_identifier(query)
        {
            return Err(AuthError::Unauthorized);
        }
        self.consumed = true;
        let record = registry.lookup(device)?;
        verify(
            &record.public_key,
            &self.revoke_status_proof(device, request, query),
            signature,
        )?;
        Ok(record.revoked && record.revoke_request_id.as_deref() == Some(request))
    }
}

pub struct DeviceRegistry {
    path: PathBuf,
    fingerprint: String,
}

struct RegistryLock(File);
impl Drop for RegistryLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            unsafe {
                libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
            }
        }
    }
}

fn private_open(path: &Path, create: bool) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(create).create(create);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "registry file required",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o077 != 0
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.nlink() != 1
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "private owned registry file required",
            ));
        }
    }
    Ok(file)
}

impl DeviceRegistry {
    pub fn new(path: PathBuf, fingerprint: String) -> Result<Self, AuthError> {
        if fingerprint.len() != 64
            || !fingerprint
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(AuthError::InvalidRegistry);
        }
        Ok(Self { path, fingerprint })
    }
    fn lock(&self) -> Result<RegistryLock, AuthError> {
        let parent = self.path.parent().ok_or(AuthError::InvalidRegistry)?;
        fs::create_dir_all(parent)?;
        let file = private_open(&self.path.with_extension("lock"), true)?;
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            let deadline = Instant::now() + Duration::from_millis(100);
            loop {
                if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                    break;
                }
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::WouldBlock {
                    return Err(error.into());
                }
                if Instant::now() >= deadline {
                    return Err(AuthError::Busy);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        #[cfg(not(unix))]
        {
            return Err(AuthError::InvalidRegistry);
        }
        Ok(RegistryLock(file))
    }
    fn load(&self) -> Result<RegistryData, AuthError> {
        let file = match private_open(&self.path, false) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(RegistryData {
                    version: 1,
                    host_fingerprint: self.fingerprint.clone(),
                    devices: vec![],
                });
            }
            Err(error) => return Err(error.into()),
        };
        if file.metadata()?.len() > MAX_REGISTRY_BYTES {
            return Err(AuthError::InvalidRegistry);
        }
        let mut bytes = Vec::new();
        file.take(MAX_REGISTRY_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_REGISTRY_BYTES {
            return Err(AuthError::InvalidRegistry);
        }
        let data: RegistryData =
            serde_json::from_slice(&bytes).map_err(|_| AuthError::InvalidRegistry)?;
        if data.version != 1
            || data.host_fingerprint != self.fingerprint
            || data.devices.len() > MAX_DEVICES
        {
            return Err(AuthError::InvalidRegistry);
        }
        let mut ids = BTreeSet::new();
        for record in &data.devices {
            if !valid_identifier(&record.name)
                || record.id != device_id(&record.public_key)?
                || !ids.insert(&record.id)
                || record.revision == 0
                || record
                    .revoke_request_id
                    .as_ref()
                    .is_some_and(|id| !record.revoked || !valid_identifier(id))
                || record.capabilities.is_empty()
                || record.capabilities.len() > 3
                || record.projects.len() > MAX_PROJECTS
                || record.projects.iter().any(|value| !valid_identifier(value))
                || record.projects.iter().collect::<BTreeSet<_>>().len() != record.projects.len()
                || record
                    .capabilities
                    .iter()
                    .map(|value| *value as u8)
                    .collect::<BTreeSet<_>>()
                    .len()
                    != record.capabilities.len()
            {
                return Err(AuthError::InvalidRegistry);
            }
        }
        Ok(data)
    }
    fn save(&self, data: &RegistryData) -> Result<(), AuthError> {
        let bytes = serde_json::to_vec(data).map_err(|_| AuthError::InvalidRegistry)?;
        if bytes.len() as u64 > MAX_REGISTRY_BYTES {
            return Err(AuthError::Limit);
        }
        let parent = self.path.parent().ok_or(AuthError::InvalidRegistry)?;
        let mut random = [0; 16];
        getrandom::getrandom(&mut random).map_err(|_| AuthError::InvalidRegistry)?;
        let temp = parent.join(format!(".devices-{}.tmp", URL_SAFE_NO_PAD.encode(random)));
        let result = (|| -> Result<(), AuthError> {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temp, &self.path)?;
            File::open(parent)?.sync_all()?;
            Ok(())
        })();
        let _ = fs::remove_file(temp);
        result
    }
    /// Caller must first consume a valid QR invite and verify proof of possession.
    /// Re-pair never silently restores a revoked key or expands an existing grant.
    pub(crate) fn register(&self, name: &str, public_key: &str) -> Result<DeviceRecord, AuthError> {
        if !valid_identifier(name) {
            return Err(AuthError::InvalidProof);
        }
        let id = device_id(public_key)?;
        let _lock = self.lock()?;
        let mut data = self.load()?;
        if let Some(record) = data.devices.iter().find(|record| record.id == id) {
            return if record.revoked {
                Err(AuthError::Unauthorized)
            } else {
                Ok(record.clone())
            };
        }
        if data.devices.len() >= MAX_DEVICES {
            return Err(AuthError::Limit);
        }
        let record = DeviceRecord {
            id,
            name: name.into(),
            public_key: public_key.into(),
            created_at: unix_seconds().map_err(|_| AuthError::InvalidRegistry)?,
            revision: 1,
            revoked: false,
            capabilities: vec![Capability::Read],
            projects: vec![],
            revoke_request_id: None,
        };
        data.devices.push(record.clone());
        self.save(&data)?;
        Ok(record)
    }
    pub fn list(&self) -> Result<Vec<DeviceRecord>, AuthError> {
        let _lock = self.lock()?;
        Ok(self.load()?.devices)
    }
    pub fn lookup(&self, id: &str) -> Result<DeviceRecord, AuthError> {
        self.list()?
            .into_iter()
            .find(|record| record.id == id)
            .ok_or(AuthError::Unauthorized)
    }
    /// Local host operator only; this method is never exposed as a remote request.
    pub fn set_permissions(
        &self,
        id: &str,
        revision: u64,
        capabilities: Vec<Capability>,
        projects: Vec<String>,
    ) -> Result<(), AuthError> {
        if capabilities.is_empty()
            || capabilities.len() > 3
            || projects.len() > MAX_PROJECTS
            || projects.iter().any(|value| !valid_identifier(value))
            || capabilities
                .iter()
                .map(|value| *value as u8)
                .collect::<BTreeSet<_>>()
                .len()
                != capabilities.len()
            || projects.iter().collect::<BTreeSet<_>>().len() != projects.len()
        {
            return Err(AuthError::InvalidRegistry);
        }
        let _lock = self.lock()?;
        let mut data = self.load()?;
        let record = data
            .devices
            .iter_mut()
            .find(|record| record.id == id)
            .ok_or(AuthError::Unauthorized)?;
        if record.revoked || record.revision != revision {
            return Err(AuthError::Unauthorized);
        }
        record.revision = record.revision.checked_add(1).ok_or(AuthError::Limit)?;
        record.capabilities = capabilities;
        record.projects = projects;
        self.save(&data)
    }
    pub fn revoke(&self, id: &str) -> Result<(), AuthError> {
        let _lock = self.lock()?;
        let mut data = self.load()?;
        let record = data
            .devices
            .iter_mut()
            .find(|record| record.id == id)
            .ok_or(AuthError::Unauthorized)?;
        if !record.revoked {
            record.revoked = true;
            record.revision = record.revision.checked_add(1).ok_or(AuthError::Limit)?;
            self.save(&data)?;
        }
        Ok(())
    }
    /// Only the connection-authenticated identity, fenced by its registry revision.
    pub fn revoke_self(&self, device: &AuthenticatedDevice) -> Result<(), AuthError> {
        self.revoke_self_request(device, None)
    }
    fn revoke_self_request(
        &self,
        device: &AuthenticatedDevice,
        request: Option<&str>,
    ) -> Result<(), AuthError> {
        let _lock = self.lock()?;
        let mut data = self.load()?;
        let record = data
            .devices
            .iter_mut()
            .find(|record| record.id == device.id)
            .ok_or(AuthError::Unauthorized)?;
        if record.revoked || record.revision != device.revision {
            return Err(AuthError::Unauthorized);
        }
        record.revision = record.revision.checked_add(1).ok_or(AuthError::Limit)?;
        record.revoked = true;
        record.revoke_request_id = request.map(str::to_owned);
        self.save(&data)
    }
    pub fn dispatch_self_revoke(
        &self,
        device: &AuthenticatedDevice,
        request: &super::protocol::Envelope,
    ) -> super::protocol::Envelope {
        use super::protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message};
        let valid = request.validate().is_ok()
            && matches!(&request.payload, Message::Request { operation: Operation::DeviceRevokeSelf, arguments } if arguments.as_object().is_some_and(|v| v.is_empty()))
            && request.required_owner.as_deref() == Some(device.id())
            && request.expected_revision == Some(device.revision())
            && request.project_id.is_none()
            && request.session_id.is_none()
            && request.turn_id.is_none()
            && request.sequence.is_none();
        if !valid {
            return super::host_adapter::HostAdapter::failure(request, ErrorCode::InvalidRequest);
        }
        match self.revoke_self_request(device, request.id.as_deref()) {
            Ok(()) => {
                let mut response = Envelope::new(
                    Channel::Control,
                    Message::Response {
                        result: serde_json::json!({"device_id":device.id(), "revoked":true}),
                    },
                );
                response.id = request.id.clone();
                response.execution_state = Some(ExecutionState::Succeeded);
                response
            }
            Err(AuthError::Unauthorized) => {
                super::host_adapter::HostAdapter::failure(request, ErrorCode::Unauthorized)
            }
            Err(AuthError::Busy) => {
                super::host_adapter::HostAdapter::failure(request, ErrorCode::Busy)
            }
            Err(_) => {
                let mut response =
                    super::host_adapter::HostAdapter::failure(request, ErrorCode::InternalError);
                // Storage failure can occur after atomic replacement. Never claim no side effect.
                response.execution_state = Some(ExecutionState::UnknownOutcome);
                response
            }
        }
    }
    pub fn authorize(
        &self,
        device: &AuthenticatedDevice,
        project: &str,
        operation: Operation,
    ) -> Result<(), AuthError> {
        let record = self.lookup(&device.id)?;
        if record.revision != device.revision {
            return Err(AuthError::Unauthorized);
        }
        authorize_record(&record, project, operation)
    }
    /// Hold the same cross-process registry lock used by revoke/grant across a short mutation commit.
    pub(crate) fn with_authorized_projects<T>(
        &self,
        device: &AuthenticatedDevice,
        projects: &[&str],
        operation: Operation,
        commit: impl FnOnce(&AuthorizedProjects) -> T,
    ) -> Result<T, AuthError> {
        let _lock = self.lock()?;
        let record = self
            .load()?
            .devices
            .into_iter()
            .find(|record| record.id == device.id)
            .ok_or(AuthError::Unauthorized)?;
        if record.revoked || record.revision != device.revision {
            return Err(AuthError::Unauthorized);
        }
        let authorization = AuthorizedProjects(record);
        for project in projects {
            authorization.authorize(project, operation)?;
        }
        Ok(commit(&authorization))
    }
    pub fn is_current(&self, device: &AuthenticatedDevice) -> Result<bool, AuthError> {
        let record = self.lookup(&device.id)?;
        Ok(!record.revoked && record.revision == device.revision)
    }
    pub fn verify_pair_and_register(
        &self,
        token: &str,
        request_id: &str,
        name: &str,
        public_key: &str,
        signature: &str,
        consume: impl FnOnce() -> bool,
    ) -> Result<DeviceRecord, AuthError> {
        verify_pairing_proof(
            &self.fingerprint,
            token,
            request_id,
            name,
            public_key,
            signature,
        )?;
        // Invalid, revoked or malformed storage must be refused before burning a token.
        let id = device_id(public_key)?;
        let records = self.list()?;
        if records
            .iter()
            .any(|record| record.id == id && record.revoked)
        {
            return Err(AuthError::Unauthorized);
        }
        if records.len() >= MAX_DEVICES && !records.iter().any(|record| record.id == id) {
            return Err(AuthError::Limit);
        }
        if !consume() {
            return Err(AuthError::Unauthorized);
        }
        self.register(name, public_key)
    }
}
