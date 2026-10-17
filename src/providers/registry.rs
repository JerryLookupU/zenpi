//! Bounded, local model catalogue. Identity is the exact `(provider, id)` pair;
//! a wire protocol never proves a model's capabilities. No discovery HTTP,
//! credentials, shell interpolation, or model-name heuristics live here.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::backend::ProviderCapabilities;

pub const CATALOG_VERSION: &str = "zenpi-models-2026-09-11.1";
pub const MAX_OVERRIDES: usize = 256;
pub const UNKNOWN_CONTEXT_WINDOW: u64 = 32_768;
pub const UNKNOWN_MAX_OUTPUT: u64 = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningLevel {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
    Ultra,
}

impl ReasoningLevel {
    pub fn parse(value: &str) -> Result<Self, RegistryError> {
        serde_json::from_value(serde_json::Value::String(value.into()))
            .map_err(|_| RegistryError::Invalid("unknown reasoning level".into()))
    }
}

/// Integer millionths of one USD per million text tokens. Missing prices mean
/// unknown, not free. A quotation belongs to its own source/version snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPrice {
    pub input_micro_usd_per_million: u64,
    pub output_micro_usd_per_million: u64,
    pub cached_input_micro_usd_per_million: Option<u64>,
    pub source: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum FieldSource {
    Builtin { reference: String, version: String },
    UserOverride { version: String },
    Conservative { version: String },
}

/// Lifecycle label of one catalog model (ZS1-186). Built-ins and unknown
/// models are `Active`; overrides may mark alpha/beta/deprecated so a selected
/// model's metadata change is visible through the descriptor digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    Alpha,
    Beta,
    Deprecated,
    Active,
}

fn default_model_status() -> ModelStatus {
    ModelStatus::Active
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelDescriptor {
    pub provider: String,
    pub id: String,
    #[serde(default = "default_model_status")]
    pub status: ModelStatus,
    pub context_window: u64,
    pub max_output_tokens: u64,
    pub capabilities: ProviderCapabilities,
    pub reasoning_levels: BTreeSet<ReasoningLevel>,
    pub price: Option<ModelPrice>,
    pub sources: BTreeMap<String, FieldSource>,
}

impl ModelDescriptor {
    pub fn effective_capabilities(&self, wire: ProviderCapabilities) -> ProviderCapabilities {
        let m = self.capabilities;
        ProviderCapabilities {
            text: m.text && wire.text,
            images: m.images && wire.images,
            files: m.files && wire.files,
            tools: m.tools && wire.tools,
            structured_output: m.structured_output && wire.structured_output,
            streaming: m.streaming && wire.streaming,
            reasoning: m.reasoning && wire.reasoning,
        }
    }

    pub fn validate_reasoning(
        &self,
        wire: ProviderCapabilities,
        effort: Option<&str>,
    ) -> Result<(), RegistryError> {
        if let Some(effort) = effort {
            let level = ReasoningLevel::parse(effort)?;
            if !self.effective_capabilities(wire).reasoning
                || !self.reasoning_levels.contains(&level)
            {
                return Err(RegistryError::Unsupported("reasoning effort"));
            }
        }
        Ok(())
    }

    /// Keep explicit user limits, clamp them to the selected model and reserve
    /// the configured output tokens. This is a local estimate, not tokenization.
    pub fn context_budget(
        &self,
        user: crate::context::ContextBudget,
    ) -> crate::context::ContextBudget {
        crate::context::ContextBudget {
            max_tokens: user.max_tokens.min(self.context_window),
            reserved_output_tokens: user.reserved_output_tokens.min(self.max_output_tokens),
        }
    }

    pub fn digest(&self) -> String {
        // All fields are finite integers, strings and ordered maps/sets.
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).expect("model metadata serializes"))
        )
    }
}

/// Only fields supplied by a user replace the builtin/conservative value.
/// Entries require an exact identity; switching models cannot accidentally
/// carry an override for the previous model to a new name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelOverride {
    pub provider: String,
    pub id: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ModelStatus>,
    pub context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub text: Option<bool>,
    pub images: Option<bool>,
    pub files: Option<bool>,
    pub tools: Option<bool>,
    pub structured_output: Option<bool>,
    pub streaming: Option<bool>,
    pub reasoning_levels: Option<BTreeSet<ReasoningLevel>>,
    pub price: Option<ModelPrice>,
}

#[derive(Debug, Clone)]
pub struct ModelRegistry {
    entries: BTreeMap<(String, String), ModelDescriptor>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        let mut registry = Self {
            entries: BTreeMap::new(),
        };
        // Small, explicitly verified catalogue; unknown aliases are not guessed.
        for (id, window, output, reasoning) in [
            ("gpt-4o-mini", 128_000, 16_384, false),
            ("gpt-4o-mini-2024-07-18", 128_000, 16_384, false),
            ("gpt-4.1", 1_047_576, 32_768, false),
            ("gpt-4.1-2025-04-14", 1_047_576, 32_768, false),
            ("gpt-5.2", 400_000, 128_000, true),
            ("gpt-5.2-2025-12-11", 400_000, 128_000, true),
        ] {
            let page = if id.starts_with("gpt-4o-mini") {
                "gpt-4o-mini"
            } else if id.starts_with("gpt-4.1") {
                "gpt-4.1"
            } else {
                "gpt-5.2"
            };
            let source = FieldSource::Builtin {
                reference: format!("https://developers.openai.com/api/docs/models/{page}"),
                version: CATALOG_VERSION.into(),
            };
            let mut model = unknown("openai", id);
            model.context_window = window;
            model.max_output_tokens = output;
            model.capabilities = ProviderCapabilities {
                text: true,
                images: true,
                files: true,
                tools: true,
                structured_output: true,
                streaming: true,
                reasoning,
            };
            if reasoning {
                model.reasoning_levels = [
                    ReasoningLevel::None,
                    ReasoningLevel::Low,
                    ReasoningLevel::Medium,
                    ReasoningLevel::High,
                    ReasoningLevel::Xhigh,
                ]
                .into();
            }
            for field in [
                "context_window",
                "max_output_tokens",
                "text",
                "images",
                "tools",
                "structured_output",
                "streaming",
                "reasoning_levels",
            ] {
                model.sources.insert(field.into(), source.clone());
            }
            model.sources.insert(
                "files".into(),
                FieldSource::Builtin {
                    reference: "https://developers.openai.com/api/docs/guides/file-inputs".into(),
                    version: CATALOG_VERSION.into(),
                },
            );
            registry.entries.insert(("openai".into(), id.into()), model);
        }
        let id = "claude-sonnet-4-6";
        let mut model = unknown("anthropic", id);
        model.context_window = 1_000_000;
        model.max_output_tokens = 128_000;
        model.capabilities = crate::providers::anthropic::CAPABILITIES;
        model.reasoning_levels = [
            ReasoningLevel::None,
            ReasoningLevel::Low,
            ReasoningLevel::Medium,
            ReasoningLevel::High,
            ReasoningLevel::Max,
        ]
        .into();
        for field in [
            "context_window",
            "max_output_tokens",
            "text",
            "images",
            "tools",
            "streaming",
        ] {
            model.sources.insert(
                field.into(),
                FieldSource::Builtin {
                    reference: "https://platform.claude.com/docs/en/models/sonnet-4-6/overview"
                        .into(),
                    version: "zenpi-anthropic-2026-09-11.1".into(),
                },
            );
        }
        model.sources.insert(
            "reasoning_levels".into(),
            FieldSource::Builtin {
                reference: "https://platform.claude.com/docs/en/build-with-claude/effort".into(),
                version: "zenpi-anthropic-2026-09-11.1; none=thinking.disabled".into(),
            },
        );
        registry
            .entries
            .insert(("anthropic".into(), id.into()), model);
        let id = "gemini-2.5-flash";
        let mut model = unknown("google", id);
        model.context_window = 1_048_576;
        model.max_output_tokens = 65_536;
        model.capabilities = crate::providers::google::CAPABILITIES;
        model.reasoning_levels = [
            ReasoningLevel::None,
            ReasoningLevel::Minimal,
            ReasoningLevel::Low,
            ReasoningLevel::Medium,
            ReasoningLevel::High,
        ]
        .into();
        for field in [
            "context_window",
            "max_output_tokens",
            "text",
            "images",
            "tools",
            "streaming",
        ] {
            model.sources.insert(
                field.into(),
                FieldSource::Builtin {
                    reference: "https://ai.google.dev/gemini-api/docs/models/gemini-2.5-flash"
                        .into(),
                    version: "zenpi-google-2026-09-11.1".into(),
                },
            );
        }
        model.sources.insert("reasoning_levels".into(),FieldSource::Builtin{reference:"https://ai.google.dev/gemini-api/docs/generate-content/thinking".into(),version:"zenpi-google-2026-09-11.1; budgets none=0 minimal=128 low=2048 medium=8192 high=24576".into()});
        registry.entries.insert(("google".into(), id.into()), model);
        // DeepSeek is an OpenAI-compatible service whose route rules declare
        // tool support, but an explicit connection only trusts a rich field
        // whose source is builtin or a user override (`connection.rs` disables
        // the rest).  Left uncatalogued, every DeepSeek field resolves through
        // `unknown()` as `Conservative`, which silently removes `tools` and
        // with it the whole tool and approval subsystem from the session.
        for id in ["deepseek-flash", "deepseek-reasoner"] {
            let source = FieldSource::Builtin {
                reference: "https://api-docs.deepseek.com/".into(),
                version: CATALOG_VERSION.into(),
            };
            let mut model = unknown("deepseek", id);
            model.context_window = 131_072;
            model.max_output_tokens = 65_536;
            // Some OpenAI-compatible DeepSeek gateways run in thinking mode by
            // default and require the previous `reasoning_text` to be replayed
            // on tool continuations. The gateway may not return that field at
            // all, which makes every tool call fail with HTTP 400. Advertising
            // `none` lets the host disable thinking explicitly instead.
            model.capabilities.reasoning = true;
            model.reasoning_levels = [
                ReasoningLevel::None,
                ReasoningLevel::Low,
                ReasoningLevel::Medium,
                ReasoningLevel::High,
            ]
            .into();
            for field in [
                "context_window",
                "max_output_tokens",
                "text",
                "tools",
                "streaming",
                "reasoning_levels",
            ] {
                model.sources.insert(field.into(), source.clone());
            }
            registry
                .entries
                .insert(("deepseek".into(), id.into()), model);
        }
        for entry in BUILTINS {
            registry
                .entries
                .entry((entry.provider.into(), entry.id.into()))
                .or_insert_with(|| entry.descriptor());
        }
        registry
    }
}

use ReasoningLevel::{High, Low, Max, Medium, Minimal, None as NoReasoning, Xhigh};

/// Models in everyday use (ZE-222). Context windows and output limits are the
/// documented ones; reasoning levels are the efforts the adapters can encode
/// for that model (an empty set means the model's own default, no effort sent).
struct Builtin {
    provider: &'static str,
    id: &'static str,
    context_window: u64,
    max_output_tokens: u64,
    capabilities: ProviderCapabilities,
    levels: &'static [ReasoningLevel],
    reference: &'static str,
}

const CODEX_CAPS: ProviderCapabilities = ProviderCapabilities {
    text: true,
    images: true,
    files: false,
    tools: true,
    structured_output: false,
    streaming: true,
    reasoning: true,
};
const OPENAI_CAPS: ProviderCapabilities = ProviderCapabilities {
    text: true,
    images: true,
    files: true,
    tools: true,
    structured_output: true,
    streaming: true,
    reasoning: true,
};
const ANTHROPIC_CAPS: ProviderCapabilities = crate::protocols::anthropic::CAPABILITIES;
const ANTHROPIC_PLAIN_CAPS: ProviderCapabilities = ProviderCapabilities {
    reasoning: false,
    ..ANTHROPIC_CAPS
};
const GOOGLE_CAPS: ProviderCapabilities = crate::protocols::google::CAPABILITIES;

const CODEX_REF: &str = "https://developers.openai.com/codex/models";
const OPENAI_REF: &str = "https://developers.openai.com/api/docs/models";
const ANTHROPIC_REF: &str = "https://platform.claude.com/docs/en/about-claude/models/overview";
const GOOGLE_REF: &str = "https://ai.google.dev/gemini-api/docs/models";

const GPT5_LEVELS: &[ReasoningLevel] = &[Minimal, Low, Medium, High];
const GPT51_LEVELS: &[ReasoningLevel] = &[NoReasoning, Low, Medium, High];
const GPT52_LEVELS: &[ReasoningLevel] = &[NoReasoning, Low, Medium, High, Xhigh];
const CODEX_LEVELS: &[ReasoningLevel] = &[Low, Medium, High];
const CODEX_X_LEVELS: &[ReasoningLevel] = &[Low, Medium, High, Xhigh];
const O_LEVELS: &[ReasoningLevel] = &[Low, Medium, High];
/// Adaptive thinking with `output_config.effort`; `none` is `thinking.disabled`.
const CLAUDE_46_LEVELS: &[ReasoningLevel] = &[NoReasoning, Low, Medium, High, Max];
const CLAUDE_47_LEVELS: &[ReasoningLevel] = &[NoReasoning, Low, Medium, High, Xhigh, Max];
/// Thinking cannot be disabled on these models.
const CLAUDE_ALWAYS_LEVELS: &[ReasoningLevel] = &[Low, Medium, High, Xhigh, Max];
/// `thinkingBudget` levels for Gemini 2.5; Pro cannot turn thinking off.
const GEMINI_LITE_LEVELS: &[ReasoningLevel] = &[NoReasoning, Low, Medium, High];
const GEMINI_PRO_LEVELS: &[ReasoningLevel] = &[Minimal, Low, Medium, High];
/// `thinkingLevel` values for Gemini 3.
const GEMINI3_PRO_LEVELS: &[ReasoningLevel] = &[Low, High];
const GEMINI3_FLASH_LEVELS: &[ReasoningLevel] = &[Minimal, Low, Medium, High];

macro_rules! builtin {
    ($provider:expr, $id:expr, $window:expr, $output:expr, $caps:expr, $levels:expr, $reference:expr) => {
        Builtin {
            provider: $provider,
            id: $id,
            context_window: $window,
            max_output_tokens: $output,
            capabilities: $caps,
            levels: $levels,
            reference: $reference,
        }
    };
}

#[rustfmt::skip]
const BUILTINS: &[Builtin] = &[
    builtin!("openai-codex", "gpt-5-codex", 272_000, 128_000, CODEX_CAPS, CODEX_LEVELS, CODEX_REF),
    builtin!("openai-codex", "gpt-5.1", 272_000, 128_000, CODEX_CAPS, GPT51_LEVELS, CODEX_REF),
    builtin!("openai-codex", "gpt-5.1-codex", 272_000, 128_000, CODEX_CAPS, CODEX_LEVELS, CODEX_REF),
    builtin!("openai-codex", "gpt-5.1-codex-mini", 272_000, 128_000, CODEX_CAPS, CODEX_LEVELS, CODEX_REF),
    builtin!("openai-codex", "gpt-5.1-codex-max", 272_000, 128_000, CODEX_CAPS, CODEX_X_LEVELS, CODEX_REF),
    builtin!("openai-codex", "gpt-5.2", 272_000, 128_000, CODEX_CAPS, GPT52_LEVELS, CODEX_REF),
    builtin!("openai-codex", "gpt-5.2-codex", 272_000, 128_000, CODEX_CAPS, CODEX_X_LEVELS, CODEX_REF),
    builtin!("openai-codex", "gpt-5.3-codex", 272_000, 128_000, CODEX_CAPS, CODEX_X_LEVELS, CODEX_REF),
    builtin!("openai-codex", "gpt-5.4", 272_000, 128_000, CODEX_CAPS, GPT52_LEVELS, CODEX_REF),
    builtin!("openai-codex", "gpt-5.4-mini", 272_000, 128_000, CODEX_CAPS, GPT52_LEVELS, CODEX_REF),
    builtin!("openai", "gpt-5", 400_000, 128_000, OPENAI_CAPS, GPT5_LEVELS, OPENAI_REF),
    builtin!("openai", "gpt-5-mini", 400_000, 128_000, OPENAI_CAPS, GPT5_LEVELS, OPENAI_REF),
    builtin!("openai", "gpt-5-codex", 400_000, 128_000, OPENAI_CAPS, CODEX_LEVELS, OPENAI_REF),
    builtin!("openai", "gpt-5.1", 400_000, 128_000, OPENAI_CAPS, GPT51_LEVELS, OPENAI_REF),
    builtin!("openai", "gpt-5.1-codex", 400_000, 128_000, OPENAI_CAPS, CODEX_LEVELS, OPENAI_REF),
    builtin!("openai", "gpt-5.1-codex-max", 400_000, 128_000, OPENAI_CAPS, CODEX_X_LEVELS, OPENAI_REF),
    builtin!("openai", "gpt-5.2-codex", 400_000, 128_000, OPENAI_CAPS, CODEX_X_LEVELS, OPENAI_REF),
    builtin!("openai", "gpt-5.3-codex", 400_000, 128_000, OPENAI_CAPS, CODEX_X_LEVELS, OPENAI_REF),
    builtin!("openai", "gpt-5.4", 272_000, 128_000, OPENAI_CAPS, GPT52_LEVELS, OPENAI_REF),
    builtin!("openai", "gpt-5.4-mini", 400_000, 128_000, OPENAI_CAPS, GPT52_LEVELS, OPENAI_REF),
    builtin!("openai", "o3", 200_000, 100_000, OPENAI_CAPS, O_LEVELS, OPENAI_REF),
    builtin!("openai", "o4-mini", 200_000, 100_000, OPENAI_CAPS, O_LEVELS, OPENAI_REF),
    builtin!("anthropic", "claude-opus-5-5", 1_000_000, 128_000, ANTHROPIC_CAPS, CLAUDE_ALWAYS_LEVELS, ANTHROPIC_REF),
    builtin!("anthropic", "claude-opus-5", 1_000_000, 128_000, ANTHROPIC_CAPS, CLAUDE_47_LEVELS, ANTHROPIC_REF),
    builtin!("anthropic", "claude-opus-4-8", 1_000_000, 128_000, ANTHROPIC_CAPS, CLAUDE_47_LEVELS, ANTHROPIC_REF),
    builtin!("anthropic", "claude-opus-4-7", 1_000_000, 128_000, ANTHROPIC_CAPS, CLAUDE_47_LEVELS, ANTHROPIC_REF),
    builtin!("anthropic", "claude-opus-4-6", 1_000_000, 128_000, ANTHROPIC_CAPS, CLAUDE_46_LEVELS, ANTHROPIC_REF),
    builtin!("anthropic", "claude-sonnet-5-5", 1_000_000, 128_000, ANTHROPIC_CAPS, CLAUDE_ALWAYS_LEVELS, ANTHROPIC_REF),
    builtin!("anthropic", "claude-sonnet-5", 1_000_000, 128_000, ANTHROPIC_CAPS, CLAUDE_47_LEVELS, ANTHROPIC_REF),
    builtin!("anthropic", "claude-haiku-5-5", 1_000_000, 128_000, ANTHROPIC_CAPS, CLAUDE_47_LEVELS, ANTHROPIC_REF),
    // Haiku 4.5 thinks only with `budget_tokens`, which this adapter does not send.
    builtin!("anthropic", "claude-haiku-4-5", 200_000, 64_000, ANTHROPIC_PLAIN_CAPS, &[], ANTHROPIC_REF),
    builtin!("google", "gemini-2.5-pro", 1_048_576, 65_536, GOOGLE_CAPS, GEMINI_PRO_LEVELS, GOOGLE_REF),
    builtin!("google", "gemini-2.5-flash-lite", 1_048_576, 65_536, GOOGLE_CAPS, GEMINI_LITE_LEVELS, GOOGLE_REF),
    builtin!("google", "gemini-3-pro-preview", 1_048_576, 65_536, GOOGLE_CAPS, GEMINI3_PRO_LEVELS, GOOGLE_REF),
    builtin!("google", "gemini-3.1-pro-preview", 1_048_576, 65_536, GOOGLE_CAPS, GEMINI3_PRO_LEVELS, GOOGLE_REF),
    builtin!("google", "gemini-3-flash-preview", 1_048_576, 65_536, GOOGLE_CAPS, GEMINI3_FLASH_LEVELS, GOOGLE_REF),
];

impl Builtin {
    fn descriptor(&self) -> ModelDescriptor {
        let mut model = unknown(self.provider, self.id);
        model.context_window = self.context_window;
        model.max_output_tokens = self.max_output_tokens;
        model.capabilities = self.capabilities;
        model.reasoning_levels = self.levels.iter().copied().collect();
        let source = FieldSource::Builtin {
            reference: self.reference.into(),
            version: CATALOG_VERSION.into(),
        };
        for field in [
            "context_window",
            "max_output_tokens",
            "text",
            "images",
            "files",
            "tools",
            "structured_output",
            "streaming",
            "reasoning_levels",
        ] {
            model.sources.insert(field.into(), source.clone());
        }
        model
    }
}

/// The model a profile uses when it names none (pi `defaultModelPerProvider`).
pub fn default_model(provider: &str) -> Option<&'static str> {
    match provider {
        "openai-codex" | "openai" => Some("gpt-5.4"),
        "anthropic" => Some("claude-opus-4-6"),
        "google" => Some("gemini-2.5-pro"),
        _ => None,
    }
}

/// Reasoning levels the native adapters may encode for a catalogued model.
pub(crate) fn builtin_reasoning_levels(provider: &str, id: &str) -> BTreeSet<ReasoningLevel> {
    static DEFAULT: std::sync::OnceLock<ModelRegistry> = std::sync::OnceLock::new();
    DEFAULT
        .get_or_init(ModelRegistry::default)
        .entries
        .get(&(provider.into(), id.into()))
        .map(|model| model.reasoning_levels.clone())
        .unwrap_or_default()
}

impl ModelRegistry {
    /// Build a candidate and publish only on success at the caller. Duplicates
    /// are rejected rather than silently changing precedence within one layer.
    pub fn with_overrides(overrides: &[ModelOverride]) -> Result<Self, RegistryError> {
        if overrides.len() > MAX_OVERRIDES {
            return Err(RegistryError::Invalid("too many model overrides".into()));
        }
        let mut registry = Self::default();
        let mut seen = BTreeSet::new();
        for entry in overrides {
            validate_identity(&entry.provider, &entry.id)?;
            bounded_text(&entry.version, 256)?;
            if !seen.insert((&entry.provider, &entry.id)) {
                return Err(RegistryError::Invalid("duplicate model override".into()));
            }
            let mut model = registry.resolve(&entry.provider, &entry.id)?;
            let source = FieldSource::UserOverride {
                version: entry.version.clone(),
            };
            macro_rules! field {
                ($field:ident) => {
                    if let Some(value) = entry.$field {
                        model.$field = value;
                        model
                            .sources
                            .insert(stringify!($field).into(), source.clone());
                    }
                };
            }
            field!(status);
            field!(context_window);
            field!(max_output_tokens);
            macro_rules! capability {
                ($field:ident) => {
                    if let Some(value) = entry.$field {
                        model.capabilities.$field = value;
                        model
                            .sources
                            .insert(stringify!($field).into(), source.clone());
                    }
                };
            }
            capability!(text);
            capability!(images);
            capability!(files);
            capability!(tools);
            capability!(structured_output);
            capability!(streaming);
            if let Some(levels) = &entry.reasoning_levels {
                model.reasoning_levels = levels.clone();
                model.capabilities.reasoning = !levels.is_empty();
                model
                    .sources
                    .insert("reasoning_levels".into(), source.clone());
            }
            if let Some(price) = &entry.price {
                bounded_text(&price.source, 2048)?;
                bounded_text(&price.version, 256)?;
                model.price = Some(price.clone());
                model.sources.insert("price".into(), source);
            }
            if !(256..=16_777_216).contains(&model.context_window)
                || model.max_output_tokens == 0
                || model.max_output_tokens >= model.context_window
            {
                return Err(RegistryError::Invalid(
                    "model context/output limits are invalid".into(),
                ));
            }
            registry
                .entries
                .insert((entry.provider.clone(), entry.id.clone()), model);
        }
        Ok(registry)
    }

    pub fn resolve(&self, provider: &str, id: &str) -> Result<ModelDescriptor, RegistryError> {
        validate_identity(provider, id)?;
        Ok(self
            .entries
            .get(&(provider.into(), id.into()))
            .cloned()
            .unwrap_or_else(|| unknown(provider, id)))
    }

    pub fn list(&self, provider: &str) -> Vec<ModelDescriptor> {
        self.entries
            .values()
            .filter(|model| model.provider == provider)
            .cloned()
            .collect()
    }
}

fn unknown(provider: &str, id: &str) -> ModelDescriptor {
    ModelDescriptor {
        provider: provider.into(),
        id: id.into(),
        status: default_model_status(),
        context_window: UNKNOWN_CONTEXT_WINDOW,
        max_output_tokens: UNKNOWN_MAX_OUTPUT,
        // Provider openness: an uncatalogued model defaults to the wire's
        // capabilities instead of a closed text-only profile, so any
        // OpenAI-compatible (chat completions / responses) or Anthropic-
        // compatible endpoint works without a hand-written override.
        // `effective_capabilities` still intersects this with the wire and an
        // explicit `model_override` can restrict any field.
        capabilities: ProviderCapabilities {
            text: true,
            images: true,
            files: true,
            tools: true,
            structured_output: true,
            streaming: true,
            // The wire's default: a reasoning wire may return thinking, but
            // with no catalogued levels no effort is ever sent.
            reasoning: true,
        },
        reasoning_levels: BTreeSet::new(),
        price: None,
        sources: [
            "context_window",
            "max_output_tokens",
            "text",
            "images",
            "files",
            "tools",
            "structured_output",
            "streaming",
            "reasoning_levels",
            "price",
        ]
        .into_iter()
        .map(|field| {
            (
                field.into(),
                FieldSource::Conservative {
                    version: CATALOG_VERSION.into(),
                },
            )
        })
        .collect(),
    }
}

fn bounded_text(value: &str, max: usize) -> Result<(), RegistryError> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(RegistryError::Invalid(
            "model metadata string is invalid".into(),
        ));
    }
    Ok(())
}

pub fn validate_identity(provider: &str, id: &str) -> Result<(), RegistryError> {
    bounded_text(provider, 128)?;
    bounded_text(id, 256)?;
    if provider.chars().any(char::is_whitespace) || id.chars().any(char::is_whitespace) {
        return Err(RegistryError::Invalid(
            "model identity contains whitespace".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RegistryError {
    #[error("invalid model metadata: {0}")]
    Invalid(String),
    #[error("selected model or wire does not support {0}")]
    Unsupported(&'static str),
}
