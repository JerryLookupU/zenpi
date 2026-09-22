//! Static service policies and pure connection routing, separate from wire codecs.
pub mod anthropic;
pub mod connection;
pub mod google;
pub mod registry;

pub(crate) mod codex;
pub(crate) mod deepseek;
pub(crate) mod openai;

use crate::backend::{BackendError, OpenAiWireApi, ProviderCapabilities};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Protocol {
    ChatCompletions,
    Responses,
    AnthropicMessages,
    GoogleGenerativeAi,
    OpenAiCodexResponses,
}

impl Protocol {
    pub(crate) fn parse(value: &str) -> Result<Self, BackendError> {
        if value == "openai_codex_responses" {
            return Ok(Self::OpenAiCodexResponses);
        }
        value
            .parse::<OpenAiWireApi>()
            .map(Self::from)
            .map_err(|_| BackendError::Configuration("unsupported provider protocol".into()))
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ChatCompletions => "chat_completions",
            Self::Responses => "responses",
            Self::AnthropicMessages => "anthropic_messages",
            Self::GoogleGenerativeAi => "google_generative_ai",
            Self::OpenAiCodexResponses => "openai_codex_responses",
        }
    }

    pub(crate) const fn wire_api(self) -> OpenAiWireApi {
        match self {
            Self::ChatCompletions => OpenAiWireApi::ChatCompletions,
            Self::Responses | Self::OpenAiCodexResponses => OpenAiWireApi::Responses,
            Self::AnthropicMessages => OpenAiWireApi::AnthropicMessages,
            Self::GoogleGenerativeAi => OpenAiWireApi::GoogleGenerativeAi,
        }
    }
}

impl From<OpenAiWireApi> for Protocol {
    fn from(value: OpenAiWireApi) -> Self {
        match value {
            OpenAiWireApi::ChatCompletions => Self::ChatCompletions,
            OpenAiWireApi::Responses => Self::Responses,
            OpenAiWireApi::AnthropicMessages => Self::AnthropicMessages,
            OpenAiWireApi::GoogleGenerativeAi => Self::GoogleGenerativeAi,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dialect {
    Default,
    Codex,
    DeepSeek,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthHeaderPolicy {
    Bearer,
    XApiKey,
    GoogleApiKey,
    Codex,
    None,
}

impl AuthHeaderPolicy {
    pub(crate) fn parse(value: &str) -> Result<Self, BackendError> {
        match value {
            "bearer" => Ok(Self::Bearer),
            "x_api_key" => Ok(Self::XApiKey),
            "google_api_key" => Ok(Self::GoogleApiKey),
            _ => Err(BackendError::Configuration(
                "unsupported API key header policy".into(),
            )),
        }
    }

    pub(crate) const fn headers(self) -> &'static [&'static str] {
        match self {
            Self::Bearer => &["authorization"],
            Self::XApiKey => &["x-api-key"],
            Self::GoogleApiKey => &["x-goog-api-key"],
            Self::Codex => &["authorization", "chatgpt-account-id"],
            Self::None => &[],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum EndpointOperation {
    ChatCompletions,
    Responses,
    Messages,
    GoogleGenerateContent,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum EndpointRule {
    Fixed(&'static str),
    PrefixAndOperation {
        default_prefix: Option<&'static str>,
        canonical_prefix: Option<&'static str>,
        operation: EndpointOperation,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OptionPolicy {
    pub reasoning_effort: bool,
    pub output_token_limit: bool,
    pub tool_choice: bool,
    pub response_format: bool,
}

pub(crate) const DEFAULT_OPTIONS: OptionPolicy = OptionPolicy {
    reasoning_effort: true,
    output_token_limit: true,
    tool_choice: true,
    response_format: true,
};

pub(crate) struct RouteRule {
    pub protocol: Protocol,
    pub dialect: Dialect,
    pub endpoint: EndpointRule,
    pub header_policy: AuthHeaderPolicy,
    pub allowed_headers: &'static [AuthHeaderPolicy],
    pub capabilities: ProviderCapabilities,
    pub options: OptionPolicy,
}

pub(crate) struct ProviderDefinition {
    pub id: &'static str,
    pub definition_version: u32,
    pub routes: &'static [RouteRule],
}

pub(crate) fn get_provider_definition(id: &str) -> Option<&'static ProviderDefinition> {
    match id {
        "openai" => Some(openai::definition()),
        "openai-codex" => Some(codex::definition()),
        "deepseek" => Some(deepseek::definition()),
        "anthropic" => Some(anthropic::definition()),
        "google" => Some(google::definition()),
        _ => None,
    }
}

const fn prefix_route(
    protocol: Protocol,
    prefix: Option<&'static str>,
    operation: EndpointOperation,
    header: AuthHeaderPolicy,
    allowed_headers: &'static [AuthHeaderPolicy],
) -> RouteRule {
    RouteRule {
        protocol,
        dialect: Dialect::Default,
        endpoint: EndpointRule::PrefixAndOperation {
            default_prefix: prefix,
            canonical_prefix: prefix,
            operation,
        },
        header_policy: header,
        allowed_headers,
        capabilities: ProviderCapabilities::for_wire_api(protocol.wire_api()),
        options: DEFAULT_OPTIONS,
    }
}

const CUSTOM_HEADERS: &[AuthHeaderPolicy] = &[
    AuthHeaderPolicy::Bearer,
    AuthHeaderPolicy::XApiKey,
    AuthHeaderPolicy::GoogleApiKey,
];

pub(crate) static CUSTOM: ProviderDefinition = ProviderDefinition {
    id: "custom",
    definition_version: 1,
    routes: &[
        prefix_route(
            Protocol::ChatCompletions,
            None,
            EndpointOperation::ChatCompletions,
            AuthHeaderPolicy::Bearer,
            CUSTOM_HEADERS,
        ),
        prefix_route(
            Protocol::Responses,
            None,
            EndpointOperation::Responses,
            AuthHeaderPolicy::Bearer,
            CUSTOM_HEADERS,
        ),
        prefix_route(
            Protocol::AnthropicMessages,
            None,
            EndpointOperation::Messages,
            AuthHeaderPolicy::XApiKey,
            CUSTOM_HEADERS,
        ),
        prefix_route(
            Protocol::GoogleGenerativeAi,
            None,
            EndpointOperation::GoogleGenerateContent,
            AuthHeaderPolicy::GoogleApiKey,
            CUSTOM_HEADERS,
        ),
    ],
};
