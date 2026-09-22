//! Compatibility surface for the native Gemini wire capabilities.

pub use crate::protocols::google::CAPABILITIES;

use super::{AuthHeaderPolicy, EndpointOperation, Protocol, ProviderDefinition, prefix_route};

static DEFINITION: ProviderDefinition = ProviderDefinition {
    id: "google",
    definition_version: 1,
    routes: &[prefix_route(
        Protocol::GoogleGenerativeAi,
        Some("https://generativelanguage.googleapis.com/v1beta"),
        EndpointOperation::GoogleGenerateContent,
        AuthHeaderPolicy::GoogleApiKey,
        &[AuthHeaderPolicy::GoogleApiKey],
    )],
};

pub(crate) fn definition() -> &'static ProviderDefinition {
    &DEFINITION
}
