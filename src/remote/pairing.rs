//! Persistent TLS identity and short-lived, one-use LAN pairing invitations.

use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use url::Url;

pub const MAX_INVITE_BYTES: usize = 2048;
pub const MAX_INVITE_TTL: Duration = Duration::from_secs(300);
const MAX_IDENTITY_BYTES: u64 = 32 * 1024;

#[derive(Debug, Error)]
pub enum PairingError {
    #[error("pairing storage I/O: {0}")]
    Io(#[from] io::Error),
    #[error("invalid pairing identity")]
    Identity,
    #[error("invalid pairing invitation")]
    InvalidInvite,
    #[error("pairing invitation expired, used, or invalid")]
    Admission,
    #[error("pairing state unavailable")]
    Unavailable,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityRecord {
    version: u16,
    certificate: String,
    private_key: String,
}

/// The private key is deliberately neither public nor Debug/Serialize.
pub struct HostIdentity {
    certificate: Vec<u8>,
    private_key: Vec<u8>,
    host_id: String,
}

/// `host_id = sha256(SPKI)`: the identity is the key, so pins survive a new certificate.
fn host_id(private_key: &[u8]) -> Result<String, PairingError> {
    let key = rustls::crypto::ring::default_provider()
        .key_provider
        .load_private_key(PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
            private_key.to_vec(),
        )))
        .map_err(|_| PairingError::Identity)?;
    let spki = key.public_key().ok_or(PairingError::Identity)?;
    Ok(format!("{:x}", Sha256::digest(spki.as_ref())))
}

impl HostIdentity {
    fn from_parts(certificate: Vec<u8>, private_key: Vec<u8>) -> Result<Self, PairingError> {
        let host_id = host_id(&private_key)?;
        let identity = Self {
            certificate,
            private_key,
            host_id,
        };
        // rustls refuses a certificate whose SPKI does not match the private key.
        identity.server_config()?;
        Ok(identity)
    }

    pub fn generate() -> Result<Self, PairingError> {
        let key = rcgen::generate_simple_self_signed(vec!["zenpi.local".into()])
            .map_err(|_| PairingError::Identity)?;
        Self::from_parts(key.cert.der().to_vec(), key.signing_key.serialize_der())
    }

    /// A fresh self-signed certificate for the same key; `host_id` is unchanged.
    pub fn rotate_certificate(&self) -> Result<Self, PairingError> {
        let key = rcgen::KeyPair::try_from(self.private_key.as_slice())
            .map_err(|_| PairingError::Identity)?;
        let certificate = rcgen::CertificateParams::new(vec!["zenpi.local".into()])
            .and_then(|params| params.self_signed(&key))
            .map_err(|_| PairingError::Identity)?;
        Self::from_parts(certificate.der().to_vec(), self.private_key.clone())
    }

    /// An atomic create publishes one private identity file, never a half key/cert pair.
    /// Concurrent initial creation fails closed instead of replacing host identity.
    pub fn load_or_create(path: &Path) -> Result<Self, PairingError> {
        match Self::load(path) {
            Ok(identity) => return Ok(identity),
            Err(PairingError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let parent = path.parent().ok_or(PairingError::Identity)?;
        fs::create_dir_all(parent)?;
        let identity = Self::generate()?;
        let record = IdentityRecord {
            version: 1,
            certificate: URL_SAFE_NO_PAD.encode(&identity.certificate),
            private_key: URL_SAFE_NO_PAD.encode(&identity.private_key),
        };
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut random = [0; 16];
        getrandom::getrandom(&mut random).map_err(|_| PairingError::Identity)?;
        let temporary = parent.join(format!(".identity-{}.tmp", URL_SAFE_NO_PAD.encode(random)));
        let result = (|| -> Result<(), PairingError> {
            let mut file = options.open(&temporary)?;
            serde_json::to_writer(&mut file, &record).map_err(|_| PairingError::Identity)?;
            file.flush()?;
            file.sync_all()?;
            // Linking a complete temporary file is atomic and refuses an existing target.
            fs::hard_link(&temporary, path)?;
            fs::File::open(parent)?.sync_all()?;
            Ok(())
        })();
        let _ = fs::remove_file(&temporary);
        result?;
        Ok(identity)
    }

    fn load(path: &Path) -> Result<Self, PairingError> {
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let file = options.open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() > MAX_IDENTITY_BYTES {
            return Err(PairingError::Identity);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.mode() & 0o077 != 0
                || metadata.uid() != unsafe { libc::geteuid() }
                || metadata.nlink() != 1
            {
                return Err(PairingError::Identity);
            }
        }
        let mut bytes = Vec::new();
        file.take(MAX_IDENTITY_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_IDENTITY_BYTES {
            return Err(PairingError::Identity);
        }
        let record: IdentityRecord =
            serde_json::from_slice(&bytes).map_err(|_| PairingError::Identity)?;
        if record.version != 1 {
            return Err(PairingError::Identity);
        }
        Self::from_parts(
            URL_SAFE_NO_PAD
                .decode(record.certificate)
                .map_err(|_| PairingError::Identity)?,
            URL_SAFE_NO_PAD
                .decode(record.private_key)
                .map_err(|_| PairingError::Identity)?,
        )
    }

    pub fn certificate(&self) -> &[u8] {
        &self.certificate
    }

    /// The host id, `sha256(SPKI)`: what QR v2, challenges, proofs and the registry bind.
    pub fn fingerprint(&self) -> String {
        self.host_id.clone()
    }

    /// The v1 pin, SHA-256 of the certificate DER; only used to migrate old state.
    pub fn legacy_fingerprint(&self) -> String {
        format!("{:x}", Sha256::digest(&self.certificate))
    }

    pub fn server_config(&self) -> Result<Arc<rustls::ServerConfig>, PairingError> {
        let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|_| PairingError::Identity)?
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(self.certificate.clone())],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.private_key.clone())),
        )
        .map_err(|_| PairingError::Identity)?;
        config.alpn_protocols = vec![b"zenpi/1".to_vec()];
        Ok(Arc::new(config))
    }
}

impl Drop for HostIdentity {
    fn drop(&mut self) {
        self.private_key.fill(0);
    }
}

/// How a hint reaches the host: the local network or the user's own overlay (T1).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HintKind {
    Lan,
    Tailnet,
}

/// One dialable address. Hints are routing advice only; identity is the pinned key.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AddressHint {
    pub kind: HintKind,
    pub address: SocketAddr,
}

impl AddressHint {
    pub fn lan(address: SocketAddr) -> Self {
        Self {
            kind: HintKind::Lan,
            address,
        }
    }

    fn to_query(self) -> String {
        let kind = match self.kind {
            HintKind::Lan => "lan",
            HintKind::Tailnet => "tailnet",
        };
        format!("{kind}:{}", self.address)
    }

    fn from_query(value: &str) -> Result<Self, PairingError> {
        let (kind, address) = value.split_once(':').ok_or(PairingError::InvalidInvite)?;
        let kind = match kind {
            "lan" => HintKind::Lan,
            "tailnet" => HintKind::Tailnet,
            _ => return Err(PairingError::InvalidInvite),
        };
        let address = address.parse().map_err(|_| PairingError::InvalidInvite)?;
        Ok(Self { kind, address })
    }
}

pub const MAX_ADDRESS_HINTS: usize = 4;

/// v1: one address and the certificate pin. v2: `host_id` plus ordered address hints,
/// issued, refreshed and consumed together with the one-use token.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PairingInvite {
    pub schema_version: u16,
    /// The first hint; kept for v1 and for single-address callers.
    pub address: SocketAddr,
    /// v2: host id `sha256(SPKI)`; v1: `sha256(certificate DER)`.
    pub fingerprint: String,
    pub token: String,
    pub expires_at: u64,
    /// Ordered dial hints; `hints[0].address == address`. v1 has exactly one LAN hint.
    pub hints: Vec<AddressHint>,
}

impl std::fmt::Debug for PairingInvite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PairingInvite")
            .field("schema_version", &self.schema_version)
            .field("hints", &self.hints)
            .field("fingerprint", &self.fingerprint)
            .field("token", &"<redacted>")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

pub fn unix_seconds() -> Result<u64, PairingError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .map_err(|_| PairingError::Unavailable)
}

pub(crate) fn valid_address(address: SocketAddr) -> bool {
    let effective = match address.ip() {
        IpAddr::V6(ip) => ip.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(address.ip()),
        ip => ip,
    };
    address.port() != 0
        && !effective.is_unspecified()
        && !effective.is_multicast()
        && effective != IpAddr::V4(std::net::Ipv4Addr::BROADCAST)
        && !matches!(effective, IpAddr::V4(ip) if ip.is_link_local())
        && !matches!(address, SocketAddr::V6(value) if value.scope_id() != 0 || value.flowinfo() != 0 || value.ip().is_unicast_link_local())
}

impl PairingInvite {
    pub fn validate_at(&self, now: u64) -> Result<(), PairingError> {
        let hints_valid = !self.hints.is_empty()
            && self.hints.len() <= MAX_ADDRESS_HINTS
            && self.hints[0].address == self.address
            && self.hints.iter().all(|hint| valid_address(hint.address))
            && self.hints.iter().enumerate().all(|(index, hint)| {
                !self.hints[..index]
                    .iter()
                    .any(|earlier| earlier.address == hint.address)
            })
            && (self.schema_version == 2
                || self.schema_version == 1
                    && self.hints.len() == 1
                    && self.hints[0].kind == HintKind::Lan);
        if !hints_valid
            || !valid_address(self.address)
            || self.fingerprint.len() != 64
            || !self
                .fingerprint
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || self.token.len() != 43
            || URL_SAFE_NO_PAD.decode(&self.token).map_or(true, |bytes| {
                bytes.len() != 32 || URL_SAFE_NO_PAD.encode(bytes) != self.token
            })
            || self.expires_at <= now
            || self.expires_at - now > MAX_INVITE_TTL.as_secs()
        {
            return Err(PairingError::InvalidInvite);
        }
        Ok(())
    }

    pub fn to_uri(&self) -> Result<String, PairingError> {
        self.to_uri_at(unix_seconds()?)
    }

    pub(crate) fn to_uri_at(&self, now: u64) -> Result<String, PairingError> {
        self.validate_at(now)?;
        let mut uri = Url::parse("zenpi://pair").map_err(|_| PairingError::InvalidInvite)?;
        if self.schema_version == 1 {
            uri.query_pairs_mut()
                .append_pair("v", "1")
                .append_pair("address", &self.address.to_string())
                .append_pair("fingerprint", &self.fingerprint)
                .append_pair("token", &self.token)
                .append_pair("expires", &self.expires_at.to_string());
        } else {
            let hints = self
                .hints
                .iter()
                .map(|hint| hint.to_query())
                .collect::<Vec<_>>()
                .join(",");
            uri.query_pairs_mut()
                .append_pair("v", "2")
                .append_pair("host_id", &self.fingerprint)
                .append_pair("hints", &hints)
                .append_pair("token", &self.token)
                .append_pair("expires", &self.expires_at.to_string());
        }
        Ok(uri.into())
    }

    pub fn from_uri(uri: &str, now: u64) -> Result<Self, PairingError> {
        if uri.len() > MAX_INVITE_BYTES
            || uri.chars().any(char::is_control)
            || !uri.starts_with("zenpi://pair?")
        {
            return Err(PairingError::InvalidInvite);
        }
        let uri = Url::parse(uri).map_err(|_| PairingError::InvalidInvite)?;
        if uri.scheme() != "zenpi"
            || uri.host_str() != Some("pair")
            || uri.port().is_some()
            || !uri.username().is_empty()
            || uri.password().is_some()
            || uri.fragment().is_some()
            || !uri.path().is_empty()
        {
            return Err(PairingError::InvalidInvite);
        }
        let mut values = std::collections::BTreeMap::new();
        for (key, value) in uri.query_pairs() {
            if !matches!(
                key.as_ref(),
                "v" | "address" | "fingerprint" | "host_id" | "hints" | "token" | "expires"
            ) || values
                .insert(key.into_owned(), value.into_owned())
                .is_some()
            {
                return Err(PairingError::InvalidInvite);
            }
        }
        let field = |name: &str| values.get(name).ok_or(PairingError::InvalidInvite);
        // Each version has exactly its own field set; nothing is optional.
        let (pin, hints) = match field("v")?.as_str() {
            "1" if values.len() == 5 => {
                let address = field("address")?
                    .parse()
                    .map_err(|_| PairingError::InvalidInvite)?;
                (field("fingerprint")?, vec![AddressHint::lan(address)])
            }
            "2" if values.len() == 5 => (
                field("host_id")?,
                field("hints")?
                    .split(',')
                    .map(AddressHint::from_query)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            _ => return Err(PairingError::InvalidInvite),
        };
        let address = hints.first().ok_or(PairingError::InvalidInvite)?.address;
        if field("expires")?.is_empty()
            || field("expires")?.len() > 19
            || !field("expires")?.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(PairingError::InvalidInvite);
        }
        let invite = Self {
            schema_version: field("v")?
                .parse()
                .map_err(|_| PairingError::InvalidInvite)?,
            address,
            fingerprint: pin.clone(),
            token: field("token")?.clone(),
            expires_at: field("expires")?
                .parse()
                .map_err(|_| PairingError::InvalidInvite)?,
            hints,
        };
        invite.validate_at(now)?;
        Ok(invite)
    }
}

struct ActiveInvite {
    invite: PairingInvite,
    deadline: Instant,
}

/// A new invite invalidates the previous one. Admission consumes atomically;
/// wrong token/fingerprint never consumes a still-valid invitation.
pub struct InviteIssuer {
    fingerprint: String,
    active: Mutex<Option<ActiveInvite>>,
}

impl InviteIssuer {
    pub fn new(identity: &HostIdentity) -> Self {
        Self {
            fingerprint: identity.fingerprint(),
            active: Mutex::new(None),
        }
    }

    /// A v2 invite whose only hint is `address` on the LAN.
    pub fn issue(&self, address: SocketAddr, ttl: Duration) -> Result<PairingInvite, PairingError> {
        self.issue_hints(vec![AddressHint::lan(address)], ttl)
    }

    /// A v2 invite with ordered dial hints, bound to this one-use token.
    pub fn issue_hints(
        &self,
        hints: Vec<AddressHint>,
        ttl: Duration,
    ) -> Result<PairingInvite, PairingError> {
        let address = hints.first().ok_or(PairingError::InvalidInvite)?.address;
        if ttl.is_zero() || ttl > MAX_INVITE_TTL || ttl.subsec_nanos() != 0 {
            return Err(PairingError::InvalidInvite);
        }
        let mut token = [0; 32];
        getrandom::getrandom(&mut token).map_err(|_| PairingError::Unavailable)?;
        let invite = PairingInvite {
            schema_version: 2,
            address,
            fingerprint: self.fingerprint.clone(),
            token: URL_SAFE_NO_PAD.encode(token),
            expires_at: unix_seconds()?
                .checked_add(ttl.as_secs())
                .ok_or(PairingError::Unavailable)?,
            hints,
        };
        invite.validate_at(unix_seconds()?)?;
        *self.active.lock().map_err(|_| PairingError::Unavailable)? = Some(ActiveInvite {
            invite: invite.clone(),
            deadline: Instant::now() + ttl,
        });
        Ok(invite)
    }

    pub fn consume(&self, token: &str, fingerprint: &str) -> Result<(), PairingError> {
        let mut active = self.active.lock().map_err(|_| PairingError::Unavailable)?;
        Self::consume_at(
            &mut active,
            token,
            fingerprint,
            unix_seconds(),
            Instant::now(),
        )
    }

    fn consume_at(
        active: &mut Option<ActiveInvite>,
        token: &str,
        fingerprint: &str,
        now: Result<u64, PairingError>,
        instant: Instant,
    ) -> Result<(), PairingError> {
        let record = active.as_ref().ok_or(PairingError::Admission)?;
        // Retire invalid clock observations as well as expiry; a later clock
        // correction must not revive an invitation the host already refused.
        if instant >= record.deadline
            || !now.is_ok_and(|now| record.invite.validate_at(now).is_ok())
        {
            *active = None;
            return Err(PairingError::Admission);
        }
        if !crate::security::constant_time_equal(record.invite.token.as_bytes(), token.as_bytes())
            || !crate::security::constant_time_equal(
                record.invite.fingerprint.as_bytes(),
                fingerprint.as_bytes(),
            )
        {
            return Err(PairingError::Admission);
        }
        *active = None;
        Ok(())
    }

    pub fn cancel(&self) -> Result<(), PairingError> {
        *self.active.lock().map_err(|_| PairingError::Unavailable)? = None;
        Ok(())
    }
}

#[cfg(test)]
mod admission_clock_tests {
    use super::*;

    fn active() -> (Option<ActiveInvite>, PairingInvite, Instant) {
        let now = Instant::now();
        let invite = PairingInvite {
            schema_version: 1,
            address: "192.168.50.55:7777".parse().unwrap(),
            fingerprint: "a".repeat(64),
            token: URL_SAFE_NO_PAD.encode([0u8; 32]),
            expires_at: 1060,
            hints: vec![AddressHint::lan("192.168.50.55:7777".parse().unwrap())],
        };
        (
            Some(ActiveInvite {
                invite: invite.clone(),
                deadline: now + Duration::from_secs(60),
            }),
            invite,
            now,
        )
    }

    #[test]
    fn wall_expiry_rollback_and_unavailable_clock_permanently_retire_invite() {
        for wall in [
            Ok(1060),
            Ok(u64::MAX),
            Ok(759),
            Err(PairingError::Unavailable),
        ] {
            let (mut active, invite, instant) = active();
            assert!(
                InviteIssuer::consume_at(
                    &mut active,
                    &invite.token,
                    &invite.fingerprint,
                    wall,
                    instant
                )
                .is_err()
            );
            assert!(active.is_none());
            assert!(
                InviteIssuer::consume_at(
                    &mut active,
                    &invite.token,
                    &invite.fingerprint,
                    Ok(1000),
                    instant
                )
                .is_err()
            );
        }
    }

    #[test]
    fn monotonic_expiry_cannot_be_extended_by_a_valid_backdated_wall_clock() {
        let (mut active, invite, instant) = active();
        assert!(
            InviteIssuer::consume_at(
                &mut active,
                &invite.token,
                &invite.fingerprint,
                Ok(990),
                instant + Duration::from_secs(60)
            )
            .is_err()
        );
        assert!(active.is_none());
        assert!(
            InviteIssuer::consume_at(
                &mut active,
                &invite.token,
                &invite.fingerprint,
                Ok(1000),
                instant
            )
            .is_err()
        );
    }

    #[test]
    fn wrong_proof_with_a_valid_clock_preserves_exactly_one_valid_admission() {
        let (mut active, invite, instant) = active();
        for (token, fingerprint) in [
            ("wrong", invite.fingerprint.as_str()),
            (invite.token.as_str(), "wrong"),
        ] {
            assert!(
                InviteIssuer::consume_at(&mut active, token, fingerprint, Ok(1000), instant)
                    .is_err()
            );
            assert!(active.is_some());
        }
        InviteIssuer::consume_at(
            &mut active,
            &invite.token,
            &invite.fingerprint,
            Ok(760),
            instant + Duration::from_secs(59),
        )
        .unwrap();
        assert!(active.is_none());
        assert!(
            InviteIssuer::consume_at(
                &mut active,
                &invite.token,
                &invite.fingerprint,
                Ok(1000),
                instant
            )
            .is_err()
        );
    }

    #[test]
    fn invalid_clock_retires_invite_even_when_the_submitted_proof_is_wrong() {
        let (mut active, _, instant) = active();
        assert!(InviteIssuer::consume_at(&mut active, "wrong", "wrong", Ok(759), instant).is_err());
        assert!(active.is_none());
    }
}
