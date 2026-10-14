//! Strict, bounded remote envelopes, separate from the local JSONL protocol.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const SCHEMA_VERSION: u16 = 1;
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub const MAX_CONTROL_BYTES: usize = 256 * 1024;
pub const MAX_ID_BYTES: usize = 128;
pub const MAX_VERSIONS: usize = 8;
pub const MAX_JSON_DEPTH: usize = 32;
pub const MAX_JSON_NODES: usize = 4096;
pub const MAX_REPLAY_EVENTS: u16 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Control,
    Event,
    Bulk,
}

impl Channel {
    pub const fn max_bytes(self) -> usize {
        match self {
            Self::Control | Self::Event => MAX_CONTROL_BYTES,
            Self::Bulk => MAX_FRAME_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    Accepted,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    UnknownOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    UnsupportedVersion,
    Unauthorized,
    RevokedDevice,
    OwnerMismatch,
    SessionChanged,
    RevisionConflict,
    RequestIdConflict,
    Busy,
    ReplayGap,
    ResourceLimit,
    InternalError,
}

/// The host derives write requirements from this enum, never from a client flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Inspect,
    Diff,
    TranscriptRead,
    FilesList,
    ResourcesList,
    ResourcesReload,
    TerminalOpen,
    TerminalInput,
    TerminalResize,
    TerminalRead,
    SessionNew,
    SessionList,
    SessionResume,
    SessionRename,
    ModelStatus,
    ModelSelect,
    ReasoningSelect,
    WorkspaceList,
    WorkspaceOpen,
    WorkspaceSelect,
    WorkspaceClose,
    SubtabList,
    SubtabAdd,
    SubtabSelect,
    SubtabMove,
    SubtabRename,
    SubtabClose,
    SetConcurrency,
    Prompt,
    Steer,
    Approve,
    Cancel,
    DeviceRevokeSelf,
}

impl Operation {
    pub const fn is_write(self) -> bool {
        !matches!(
            self,
            Self::Inspect
                | Self::Diff
                | Self::TranscriptRead
                | Self::TerminalRead
                | Self::SessionList
                | Self::FilesList
                | Self::ResourcesList
                | Self::ModelStatus
                | Self::WorkspaceList
                | Self::SubtabList
        )
    }
}

/// This cursor belongs to remote event projection, not the session journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventCursor {
    pub session_id: String,
    pub next_sequence: u64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Message {
    Hello {
        supported_versions: Vec<u16>,
    },
    Welcome {
        selected_version: u16,
    },
    Challenge {
        nonce: String,
        host_fingerprint: String,
    },
    Authenticate {
        device_id: String,
        signature: String,
    },
    RevokeStatus {
        device_id: String,
        request_id: String,
        signature: String,
    },
    Pair {
        token: String,
        fingerprint: String,
        safety_code: String,
        device_name: String,
        public_key: String,
        signature: String,
    },
    Request {
        operation: Operation,
        arguments: Value,
    },
    Response {
        result: Value,
    },
    Event {
        name: String,
        data: Value,
    },
    Ack {
        cursor: EventCursor,
    },
    Resume {
        cursor: EventCursor,
        limit: u16,
    },
    Subscribe {
        cursor: EventCursor,
        limit: u16,
        duration_ms: u16,
    },
    Snapshot {
        cursor: EventCursor,
        state: Value,
    },
}

impl std::fmt::Debug for Message {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut value = serde_json::to_value(self).map_err(|_| std::fmt::Error)?;
        if matches!(self, Self::Pair { .. }) {
            value["token"] = Value::String("<redacted>".into());
            value["safety_code"] = Value::String("<redacted>".into());
        }
        value.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub schema_version: u16,
    pub channel: Channel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<ErrorCode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_state: Option<ExecutionState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<u64>,
    pub payload: Message,
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("unsupported remote schema version {0}")]
    UnsupportedVersion(u16),
    #[error("no supported remote schema version in offer")]
    NoCommonVersion,
    #[error("invalid remote envelope: {0}")]
    Invalid(&'static str),
    #[error("remote resource limit exceeded: {0}")]
    Limit(&'static str),
    #[error("invalid remote JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn negotiate_version(offered: &[u16]) -> Result<u16, ProtocolError> {
    if offered.is_empty() || offered.len() > MAX_VERSIONS {
        return Err(ProtocolError::Limit("version offer"));
    }
    if offered.contains(&SCHEMA_VERSION) {
        Ok(SCHEMA_VERSION)
    } else {
        Err(ProtocolError::NoCommonVersion)
    }
}

fn identifier(value: &str) -> Result<(), ProtocolError> {
    if value.is_empty() || value.len() > MAX_ID_BYTES || value.chars().any(char::is_control) {
        return Err(ProtocolError::Invalid("identifier"));
    }
    Ok(())
}

fn bounded_value(value: &Value, depth: usize, nodes: &mut usize) -> Result<(), ProtocolError> {
    *nodes += 1;
    if depth > MAX_JSON_DEPTH || *nodes > MAX_JSON_NODES {
        return Err(ProtocolError::Limit("JSON complexity"));
    }
    match value {
        Value::Array(values) => {
            for value in values {
                bounded_value(value, depth + 1, nodes)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                bounded_value(value, depth + 1, nodes)?;
            }
        }
        _ => {}
    }
    Ok(())
}

impl Envelope {
    pub fn new(channel: Channel, payload: Message) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            channel,
            id: None,
            session_id: None,
            project_id: None,
            turn_id: None,
            sequence: None,
            code: None,
            execution_state: None,
            required_owner: None,
            expected_revision: None,
            payload,
        }
    }

    /// Structural checks are not authentication, ACL, CAS or deduplication.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(ProtocolError::UnsupportedVersion(self.schema_version));
        }
        for value in [
            &self.id,
            &self.session_id,
            &self.project_id,
            &self.turn_id,
            &self.required_owner,
        ]
        .into_iter()
        .flatten()
        {
            identifier(value)?;
        }
        let expected_channel = match &self.payload {
            Message::Event { .. } => Channel::Event,
            Message::Snapshot { .. } => Channel::Bulk,
            _ => Channel::Control,
        };
        if self.channel != expected_channel {
            return Err(ProtocolError::Invalid("message/channel mismatch"));
        }
        if !matches!(self.payload, Message::Response { .. })
            && (self.code.is_some() || self.execution_state.is_some())
        {
            return Err(ProtocolError::Invalid("response-only fields"));
        }
        if !matches!(self.payload, Message::Request { .. })
            && (self.expected_revision.is_some() || self.required_owner.is_some())
        {
            return Err(ProtocolError::Invalid("request-only fields"));
        }
        if matches!(
            self.payload,
            Message::Hello { .. }
                | Message::Welcome { .. }
                | Message::Challenge { .. }
                | Message::Pair { .. }
                | Message::Authenticate { .. }
                | Message::RevokeStatus { .. }
        ) && (self.project_id.is_some()
            || self.session_id.is_some()
            || self.turn_id.is_some()
            || self.sequence.is_some()
            || matches!(
                self.payload,
                Message::Hello { .. } | Message::Welcome { .. } | Message::Challenge { .. }
            ) && self.id.is_some())
        {
            return Err(ProtocolError::Invalid(
                "connection handshake routing fields",
            ));
        }
        match &self.payload {
            Message::Hello { supported_versions } => {
                negotiate_version(supported_versions)?;
            }
            Message::Welcome { selected_version } => {
                if *selected_version != SCHEMA_VERSION {
                    return Err(ProtocolError::UnsupportedVersion(*selected_version));
                }
            }
            Message::Challenge {
                nonce,
                host_fingerprint,
            } => {
                if nonce.len() != 43
                    || !nonce.bytes().all(|value| {
                        value.is_ascii_alphanumeric() || value == b'-' || value == b'_'
                    })
                    || URL_SAFE_NO_PAD.decode(nonce).map_or(true, |bytes| {
                        bytes.len() != 32 || URL_SAFE_NO_PAD.encode(bytes) != *nonce
                    })
                    || host_fingerprint.len() != 64
                    || !host_fingerprint
                        .bytes()
                        .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
                {
                    return Err(ProtocolError::Invalid("challenge format"));
                }
            }
            Message::Authenticate {
                device_id,
                signature,
            }
            | Message::RevokeStatus {
                device_id,
                signature,
                ..
            } => {
                if device_id.len() != 64
                    || !device_id
                        .bytes()
                        .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
                    || signature.is_empty()
                    || signature.len() > 104
                    || !signature.bytes().all(|value| {
                        value.is_ascii_alphanumeric() || value == b'-' || value == b'_'
                    })
                    || self.id.is_none()
                {
                    return Err(ProtocolError::Invalid("authentication proof format"));
                }
                if let Message::RevokeStatus { request_id, .. } = &self.payload {
                    identifier(request_id)?;
                    if self.project_id.is_some()
                        || self.session_id.is_some()
                        || self.turn_id.is_some()
                        || self.sequence.is_some()
                    {
                        return Err(ProtocolError::Invalid("revoke status routing fields"));
                    }
                }
            }
            Message::Pair {
                token,
                fingerprint,
                safety_code,
                device_name,
                public_key,
                signature,
            } => {
                if token.len() != 43
                    || !token
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
                    || fingerprint.len() != 64
                    || !fingerprint
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                    || safety_code.len() != 6
                    || !safety_code.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err(ProtocolError::Invalid("pairing proof format"));
                }
                identifier(device_name)?;
                if crate::remote::auth::device_id(public_key).is_err()
                    || signature.is_empty()
                    || signature.len() > 104
                    || !signature
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
                {
                    return Err(ProtocolError::Invalid("device proof format"));
                }
                if self.id.is_none() {
                    return Err(ProtocolError::Invalid("pairing request ID required"));
                }
            }
            Message::Request {
                operation,
                arguments,
            } => {
                if self.id.is_none() {
                    return Err(ProtocolError::Invalid("request ID required"));
                }
                if operation.is_write()
                    && (self.required_owner.is_none() || self.expected_revision.is_none())
                {
                    return Err(ProtocolError::Invalid("write requires owner and revision"));
                }
                if !arguments.is_object() {
                    return Err(ProtocolError::Invalid("arguments must be an object"));
                }
                bounded_value(arguments, 0, &mut 0)?;
            }
            Message::Response { result } => {
                if self.id.is_none() || self.execution_state.is_none() {
                    return Err(ProtocolError::Invalid(
                        "response requires ID and execution state",
                    ));
                }
                if self.execution_state == Some(ExecutionState::Failed) && self.code.is_none() {
                    return Err(ProtocolError::Invalid("failure requires error code"));
                }
                bounded_value(result, 0, &mut 0)?;
            }
            Message::Event { name, data } => {
                identifier(name)?;
                if self.session_id.is_none() || self.sequence.is_none() {
                    return Err(ProtocolError::Invalid(
                        "event requires session and sequence",
                    ));
                }
                bounded_value(data, 0, &mut 0)?;
            }
            Message::Ack { cursor }
            | Message::Resume { cursor, .. }
            | Message::Subscribe { cursor, .. }
            | Message::Snapshot { cursor, .. } => {
                identifier(&cursor.session_id)?;
                if self.session_id.as_deref() != Some(cursor.session_id.as_str()) {
                    return Err(ProtocolError::Invalid("cursor/session mismatch"));
                }
                if let Message::Resume { limit, .. } | Message::Subscribe { limit, .. } =
                    &self.payload
                    && (*limit == 0 || *limit > MAX_REPLAY_EVENTS)
                {
                    return Err(ProtocolError::Limit("replay page"));
                }
                if let Message::Subscribe { duration_ms, .. } = &self.payload
                    && (!(1000..=5000).contains(duration_ms)
                        || self.id.is_none()
                        || self.project_id.is_none()
                        || self.turn_id.is_some())
                {
                    return Err(ProtocolError::Invalid("invalid bounded subscription"));
                }
                if let Message::Snapshot { state, .. } = &self.payload {
                    bounded_value(state, 0, &mut 0)?;
                }
            }
        }
        if self.sequence.is_some() && !matches!(self.payload, Message::Event { .. }) {
            return Err(ProtocolError::Invalid("event-only sequence"));
        }
        Ok(())
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.is_empty() || bytes.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError::Limit("frame bytes"));
        }
        super::strict_json::preflight(bytes)?;
        let envelope: Self = serde_json::from_slice(bytes)?;
        if bytes.len() > envelope.channel.max_bytes() {
            return Err(ProtocolError::Limit("channel bytes"));
        }
        if matches!(
            envelope.payload,
            Message::Hello { .. }
                | Message::Welcome { .. }
                | Message::Challenge { .. }
                | Message::Pair { .. }
                | Message::Authenticate { .. }
                | Message::RevokeStatus { .. }
        ) {
            // Option decoding erases explicit nulls; handshake envelopes require
            // omitted authority fields, not merely values that deserialize to None.
            let value: Value = serde_json::from_slice(bytes)?;
            let fields = value
                .as_object()
                .ok_or(ProtocolError::Invalid("handshake envelope"))?;
            let identified = matches!(
                envelope.payload,
                Message::Pair { .. } | Message::Authenticate { .. } | Message::RevokeStatus { .. }
            );
            if fields.keys().any(|key| {
                !matches!(key.as_str(), "schema_version" | "channel" | "payload")
                    && !(identified && key == "id")
            }) {
                return Err(ProtocolError::Invalid("connection handshake fields"));
            }
        }
        envelope.validate()?;
        Ok(envelope)
    }
}
