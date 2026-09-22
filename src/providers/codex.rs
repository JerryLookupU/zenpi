//! Fixed ChatGPT product destination; token exchange belongs to auth.
use super::{
    AuthHeaderPolicy, Dialect, EndpointRule, OptionPolicy, Protocol, ProviderDefinition, RouteRule,
};
use crate::backend::{OpenAiWireApi, ProviderCapabilities};

pub(crate) const RESPONSES_ENDPOINT: &str = "https://chatgpt.com/backend-api/codex/responses";
pub(crate) const ISSUER: &str = "https://auth.openai.com";
pub(crate) const AUTHORIZE_ENDPOINT: &str = "https://auth.openai.com/oauth/authorize";
pub(crate) const TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";
pub(crate) const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub(crate) const SCOPE: &str = "openid profile email offline_access";
pub(crate) const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
pub(crate) const DEVICE_BEGIN_ENDPOINT: &str =
    "https://auth.openai.com/api/accounts/deviceauth/usercode";
pub(crate) const DEVICE_POLL_ENDPOINT: &str =
    "https://auth.openai.com/api/accounts/deviceauth/token";
pub(crate) const DEVICE_VERIFICATION_URI: &str = "https://auth.openai.com/codex/device";
pub(crate) const DEVICE_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";
pub(crate) const ORIGINATOR: &str = "zenpi";

static DEFINITION: ProviderDefinition = ProviderDefinition {
    id: "openai-codex",
    definition_version: 1,
    routes: &[RouteRule {
        protocol: Protocol::OpenAiCodexResponses,
        dialect: Dialect::Codex,
        endpoint: EndpointRule::Fixed(RESPONSES_ENDPOINT),
        header_policy: AuthHeaderPolicy::Codex,
        allowed_headers: &[AuthHeaderPolicy::Codex],
        capabilities: ProviderCapabilities {
            files: false,
            structured_output: false,
            ..ProviderCapabilities::for_wire_api(OpenAiWireApi::Responses)
        },
        options: OptionPolicy {
            reasoning_effort: true,
            output_token_limit: false,
            tool_choice: true,
            response_format: false,
        },
    }],
};

pub(crate) fn definition() -> &'static ProviderDefinition {
    &DEFINITION
}
