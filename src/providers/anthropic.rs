//! Compatibility surface for the native Messages wire capabilities.

pub use crate::protocols::anthropic::CAPABILITIES;

use super::{AuthHeaderPolicy, EndpointOperation, Protocol, ProviderDefinition, prefix_route};

static DEFINITION: ProviderDefinition = ProviderDefinition {
    id: "anthropic",
    definition_version: 1,
    routes: &[prefix_route(
        Protocol::AnthropicMessages,
        Some("https://api.anthropic.com/v1"),
        EndpointOperation::Messages,
        AuthHeaderPolicy::XApiKey,
        &[AuthHeaderPolicy::XApiKey],
    )],
};

pub(crate) fn definition() -> &'static ProviderDefinition {
    &DEFINITION
}
