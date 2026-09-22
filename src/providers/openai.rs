//! OpenAI API service rules, not ChatGPT subscription authentication.
use super::{AuthHeaderPolicy, EndpointOperation, Protocol, ProviderDefinition, prefix_route};

static DEFINITION: ProviderDefinition = ProviderDefinition {
    id: "openai",
    definition_version: 1,
    routes: &[
        prefix_route(
            Protocol::ChatCompletions,
            Some("https://api.openai.com/v1"),
            EndpointOperation::ChatCompletions,
            AuthHeaderPolicy::Bearer,
            &[AuthHeaderPolicy::Bearer],
        ),
        prefix_route(
            Protocol::Responses,
            Some("https://api.openai.com/v1"),
            EndpointOperation::Responses,
            AuthHeaderPolicy::Bearer,
            &[AuthHeaderPolicy::Bearer],
        ),
    ],
};

pub(crate) fn definition() -> &'static ProviderDefinition {
    &DEFINITION
}
