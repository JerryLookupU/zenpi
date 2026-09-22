//! One DeepSeek service with three explicit wire routes and shared credentials.
use super::{
    AuthHeaderPolicy, DEFAULT_OPTIONS, Dialect, EndpointOperation, EndpointRule, Protocol,
    ProviderDefinition, RouteRule,
};
use crate::backend::ProviderCapabilities;

const fn route(
    protocol: Protocol,
    prefix: &'static str,
    operation: EndpointOperation,
    header_policy: AuthHeaderPolicy,
    allowed_headers: &'static [AuthHeaderPolicy],
) -> RouteRule {
    RouteRule {
        protocol,
        dialect: Dialect::DeepSeek,
        endpoint: EndpointRule::PrefixAndOperation {
            default_prefix: Some(prefix),
            canonical_prefix: Some(prefix),
            operation,
        },
        header_policy,
        allowed_headers,
        capabilities: ProviderCapabilities {
            files: false,
            structured_output: matches!(protocol, Protocol::Responses),
            reasoning: true,
            ..ProviderCapabilities::for_wire_api(protocol.wire_api())
        },
        options: super::OptionPolicy {
            response_format: !matches!(protocol, Protocol::AnthropicMessages),
            ..DEFAULT_OPTIONS
        },
    }
}

static DEFINITION: ProviderDefinition = ProviderDefinition {
    id: "deepseek",
    definition_version: 1,
    routes: &[
        route(
            Protocol::ChatCompletions,
            "https://api.deepseek.com",
            EndpointOperation::ChatCompletions,
            AuthHeaderPolicy::Bearer,
            &[AuthHeaderPolicy::Bearer],
        ),
        route(
            Protocol::Responses,
            "https://api.deepseek.com",
            EndpointOperation::Responses,
            AuthHeaderPolicy::Bearer,
            &[AuthHeaderPolicy::Bearer],
        ),
        route(
            Protocol::AnthropicMessages,
            "https://api.deepseek.com/anthropic/v1",
            EndpointOperation::Messages,
            AuthHeaderPolicy::XApiKey,
            &[AuthHeaderPolicy::XApiKey],
        ),
    ],
};

pub(crate) fn definition() -> &'static ProviderDefinition {
    &DEFINITION
}
