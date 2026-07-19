//! Serde mirror of the upstream models.dev Zod schema
//! (`packages/core/src/schema.ts` in <https://github.com/anomalyco/models.dev>).
//!
//! Every field defined by the upstream `Provider`/`ModelShape` types is
//! represented here so that `#[serde(deny_unknown_fields)]` only fires when
//! upstream adds a genuinely new field — that mismatch is the drift signal
//! this generator is meant to surface. See the crate-level docs for the
//! handful of deliberate fidelity trade-offs (documented per-field below).
//!
//! Only the *served* shape (`Model` = `refineModel(ModelShape)`) is modeled;
//! `AuthoredModel`/`ModelMetadata` are authoring-time types used by
//! models.dev's own build pipeline and never appear in `api.json`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::CatalogGenError;

/// The full upstream catalog: `{ providerId: Provider }`.
///
/// A `BTreeMap` (rather than `HashMap`) keeps provider iteration order
/// deterministic, which in turn makes bare-alias collisions
/// (see [`crate::transform::PRIMARY_PROVIDERS`]) resolve the same way on
/// every run.
pub type Catalog = BTreeMap<String, Provider>;

/// One upstream provider entry.
///
/// Mirrors the Zod `Provider` schema. `.strict()` in the source maps to
/// `#[serde(deny_unknown_fields)]` here.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Provider {
    /// Provider id, duplicated from the map key upstream.
    pub id: String,
    /// Environment variable names accepted for provider credentials.
    /// Zod requires at least one entry.
    pub env: Vec<String>,
    /// The npm package implementing this provider's AI SDK integration.
    pub npm: String,
    /// Optional base API URL override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    /// Human-readable provider name.
    pub name: String,
    /// Link to provider documentation listing models.
    pub doc: String,
    /// Models offered by this provider, keyed by model id.
    pub models: BTreeMap<String, Model>,
}

/// One upstream model entry (the `Model` = `refineModel(ModelShape)` type).
///
/// Mirrors `ModelShape.strict()`. `family` is deliberately widened to
/// `Option<String>` rather than mirroring the ~200-variant closed
/// `ModelFamily` enum: that taxonomy grows with nearly every models.dev
/// data update and is unrelated to pricing/limit fields, so treating it as
/// a closed enum would produce constant validation failures that are not
/// genuine schema drift. Date fields (`knowledge`, `release_date`,
/// `last_updated`) are similarly widened from the regex-validated
/// `DateString` to plain `String` — format drift there is not a pricing
/// concern.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    /// Model id, duplicated from the map key upstream.
    pub id: String,
    /// Human-readable model name.
    pub name: String,
    /// Model description.
    pub description: String,
    /// Model family taxonomy tag. See struct docs for why this is a plain
    /// string rather than the closed upstream enum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// Whether the model accepts file/image attachments.
    pub attachment: bool,
    /// Whether the model supports reasoning.
    pub reasoning: bool,
    /// Reasoning configuration options; required by the upstream schema
    /// whenever `reasoning` is `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_options: Option<Vec<ReasoningOption>>,
    /// Whether the model supports tool/function calling.
    pub tool_call: bool,
    /// Interleaved-thinking support declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interleaved: Option<Interleaved>,
    /// Whether the model supports structured/JSON-schema output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<bool>,
    /// Whether the model accepts a `temperature` sampling parameter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<bool>,
    /// Training data cutoff, `YYYY-MM` or `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge: Option<String>,
    /// Model release date.
    pub release_date: String,
    /// Date this catalog entry was last updated.
    pub last_updated: String,
    /// Supported input/output content modalities.
    pub modalities: Modalities,
    /// Whether model weights are open.
    pub open_weights: bool,
    /// Context window and per-request token limits.
    pub limit: ProviderModelLimit,
    /// Lifecycle status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ModelStatus>,
    /// Experimental per-mode overrides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Experimental>,
    /// Provider-specific routing overrides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<ModelProviderOverride>,
    /// Pricing. Absent entirely for free/unpriced models — those are
    /// skipped by the transform step (see `transform::transform_catalog`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<OutputCost>,
}

/// Lifecycle status, mirrors `z.enum(["alpha", "beta", "deprecated"])`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    /// Newly released, may change without notice.
    Alpha,
    /// Pre-release, stabilizing.
    Beta,
    /// No longer recommended for new integrations.
    Deprecated,
}

/// `attachment: true | { field: "reasoning_content" | "reasoning_details" }`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Interleaved {
    /// Bare `true` — interleaved thinking supported, no extra detail.
    Enabled(bool),
    /// Interleaved thinking supported, surfaced via a specific field.
    Detail {
        /// Which response field carries interleaved reasoning content.
        field: InterleavedField,
    },
}

/// The response field carrying interleaved reasoning content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InterleavedField {
    /// `reasoning_content`.
    ReasoningContent,
    /// `reasoning_details`.
    ReasoningDetails,
}

/// A single reasoning-effort value, or `null`/absent meaning "no explicit
/// effort level".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    /// No reasoning.
    None,
    /// Minimal reasoning effort.
    Minimal,
    /// Low reasoning effort.
    Low,
    /// Medium reasoning effort.
    Medium,
    /// High reasoning effort.
    High,
    /// Extra-high reasoning effort.
    Xhigh,
    /// Maximum reasoning effort.
    Max,
    /// Provider default reasoning effort.
    Default,
}

/// `ReasoningOption`: a discriminated union on `type`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReasoningOption {
    /// A simple on/off reasoning toggle.
    Toggle,
    /// A discrete set of effort levels.
    Effort {
        /// Allowed effort values, `null` entries permitted upstream.
        values: Vec<Option<ReasoningEffort>>,
    },
    /// A numeric reasoning token budget range.
    BudgetTokens {
        /// Minimum budget, `>= -1` upstream.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        /// Maximum budget, `>= 0` upstream.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
    },
}

/// `Modality`: `"text" | "audio" | "image" | "video" | "pdf"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Modality {
    /// Plain text.
    Text,
    /// Audio content.
    Audio,
    /// Still images.
    Image,
    /// Video content.
    Video,
    /// PDF documents.
    Pdf,
}

/// Supported input/output modalities. Mirrors `Modalities.strict()`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Modalities {
    /// Modalities accepted as input.
    pub input: Vec<Modality>,
    /// Modalities the model can produce as output.
    pub output: Vec<Modality>,
}

/// Token limits as served on a live provider model. Mirrors
/// `ProviderModelLimit.strict()` (`output` is required here, unlike the
/// authoring-time `ModelLimit`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderModelLimit {
    /// Total context window, in tokens.
    pub context: f64,
    /// Maximum input/prompt tokens, when narrower than `context`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,
    /// Maximum output/completion tokens.
    pub output: f64,
}

/// Base cost fields shared by [`OutputCost`] and a [`CostTier`] override, and
/// by the (non-strict) `context_over_200k` nested cost object. Upstream's
/// `Cost` Zod object is never itself `.strict()`-flagged, so this type
/// intentionally does *not* deny unknown fields — only the types built by
/// extending and then explicitly `.strict()`-ing it (like [`CostTier`]) do.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cost {
    /// USD per 1,000,000 input tokens.
    pub input: f64,
    /// USD per 1,000,000 output tokens.
    pub output: f64,
    /// USD per 1,000,000 reasoning tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<f64>,
    /// USD per 1,000,000 cache-read tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    /// USD per 1,000,000 cache-write tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
    /// USD per 1,000,000 input audio tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_audio: Option<f64>,
    /// USD per 1,000,000 output audio tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_audio: Option<f64>,
}

/// `size`/`type` discriminator for a [`CostTier`]. Mirrors the strict
/// `{ type: "context", size: number.int() }` sub-object.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CostTierBoundary {
    /// Always `"context"` today; upstream defaults it when absent.
    #[serde(rename = "type", default = "default_tier_type")]
    pub tier_type: String,
    /// Context-token threshold at/above which this tier's costs apply.
    /// Upstream constrains this to an integer (`z.number().int()`), which
    /// this field's `u64` type enforces structurally.
    pub size: u64,
}

fn default_tier_type() -> String {
    "context".to_string()
}

/// A context-tiered cost override. Mirrors `CostTier = Cost.extend({
/// tier }).strict()`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CostTier {
    /// USD per 1,000,000 input tokens at/above this tier's threshold.
    pub input: f64,
    /// USD per 1,000,000 output tokens at/above this tier's threshold.
    pub output: f64,
    /// USD per 1,000,000 reasoning tokens at/above this tier's threshold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<f64>,
    /// USD per 1,000,000 cache-read tokens at/above this tier's threshold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    /// USD per 1,000,000 cache-write tokens at/above this tier's threshold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
    /// USD per 1,000,000 input audio tokens at/above this tier's threshold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_audio: Option<f64>,
    /// USD per 1,000,000 output audio tokens at/above this tier's threshold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_audio: Option<f64>,
    /// The threshold this tier applies above.
    pub tier: CostTierBoundary,
}

/// The cost shape actually served on a live model (`OutputCost`, i.e.
/// `Cost.extend({ context_over_200k, tiers })`, not `.strict()`-flagged
/// upstream).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutputCost {
    /// USD per 1,000,000 input tokens.
    pub input: f64,
    /// USD per 1,000,000 output tokens.
    pub output: f64,
    /// USD per 1,000,000 reasoning tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<f64>,
    /// USD per 1,000,000 cache-read tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    /// USD per 1,000,000 cache-write tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
    /// USD per 1,000,000 input audio tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_audio: Option<f64>,
    /// USD per 1,000,000 output audio tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_audio: Option<f64>,
    /// A redundant duplicate of the 200k-token tier already present in
    /// `tiers`; intentionally ignored by the transform step per the
    /// pricing.json record contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_over_200k: Option<Cost>,
    /// Context-tiered cost overrides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiers: Option<Vec<CostTier>>,
}

/// `experimental.modes` entry.
///
/// Upstream's `experimental.modes` record value is a plain `z.object` (no
/// `.strict()`), so unknown keys are silently stripped upstream — this mirror
/// must not `deny_unknown_fields`, or CI would hard-fail the first time
/// models.dev adds an unrelated field here.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExperimentalMode {
    /// Cost override for this experimental mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<Cost>,
    /// Provider request overrides for this experimental mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<ExperimentalModeProvider>,
}

/// Provider request overrides nested under an experimental mode.
///
/// Upstream this is a plain `z.object` (no `.strict()`); see
/// [`ExperimentalMode`] for why this mirror does not `deny_unknown_fields`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExperimentalModeProvider {
    /// Extra body fields merged into the outgoing request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Map<String, serde_json::Value>>,
    /// Extra headers merged into the outgoing request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
}

/// `experimental` block on a model.
///
/// Upstream this is a plain `z.object` (no `.strict()`); see
/// [`ExperimentalMode`] for why this mirror does not `deny_unknown_fields`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Experimental {
    /// Named experimental mode overrides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modes: Option<BTreeMap<String, ExperimentalMode>>,
}

/// Request shape upstream uses to talk to this provider (`"responses" |
/// "completions"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderShape {
    /// The OpenAI Responses API shape.
    Responses,
    /// The OpenAI Chat Completions API shape.
    Completions,
}

/// Model-level provider routing overrides.
///
/// Upstream this is a plain `z.object` (no `.strict()`); see
/// [`ExperimentalMode`] for why this mirror does not `deny_unknown_fields`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelProviderOverride {
    /// npm module override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    /// API base URL override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    /// Wire shape override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<ProviderShape>,
    /// Extra body fields merged into the outgoing request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Map<String, serde_json::Value>>,
    /// Extra headers merged into the outgoing request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
}

/// Validate structural invariants beyond what serde's type system already
/// enforces: non-negative costs, non-empty identifying strings, and a
/// positive context window. Returns the first violation found; provider and
/// model iteration follows the deterministic `BTreeMap` order.
pub fn validate_catalog(catalog: &Catalog) -> Result<(), CatalogGenError> {
    for (provider_id, provider) in catalog {
        validate_provider(provider_id, provider)?;
        for (model_id, model) in &provider.models {
            validate_model(provider_id, model_id, model)?;
        }
    }
    Ok(())
}

fn validation_error(provider: &str, model: &str, field: &str, message: impl Into<String>) -> CatalogGenError {
    CatalogGenError::Validation {
        provider: provider.to_string(),
        model: model.to_string(),
        field: field.to_string(),
        message: message.into(),
    }
}

fn validate_provider(provider_id: &str, provider: &Provider) -> Result<(), CatalogGenError> {
    if provider.id.is_empty() {
        return Err(validation_error(
            provider_id,
            provider_id,
            "id",
            "provider id must not be empty",
        ));
    }
    if provider.name.is_empty() {
        return Err(validation_error(
            provider_id,
            provider_id,
            "name",
            "provider name must not be empty",
        ));
    }
    if provider.env.is_empty() {
        return Err(validation_error(
            provider_id,
            provider_id,
            "env",
            "provider env must have at least one entry",
        ));
    }
    if provider.npm.is_empty() {
        return Err(validation_error(
            provider_id,
            provider_id,
            "npm",
            "provider npm module must not be empty",
        ));
    }
    if provider.doc.is_empty() {
        return Err(validation_error(
            provider_id,
            provider_id,
            "doc",
            "provider doc link must not be empty",
        ));
    }
    Ok(())
}

fn validate_model(provider_id: &str, model_id: &str, model: &Model) -> Result<(), CatalogGenError> {
    if model.id.is_empty() {
        return Err(validation_error(
            provider_id,
            model_id,
            "id",
            "model id must not be empty",
        ));
    }
    if model.name.is_empty() {
        return Err(validation_error(
            provider_id,
            model_id,
            "name",
            "model name must not be empty",
        ));
    }
    // ~keep Upstream constrains this to `z.number().min(0)` — 0 is valid
    // ~keep (image models such as `azure/gpt-image-1` have no text context
    // ~keep window), so reject only negative or non-finite values.
    if model.limit.context < 0.0 || !model.limit.context.is_finite() {
        return Err(validation_error(
            provider_id,
            model_id,
            "limit.context",
            format!("context window must be non-negative, got {}", model.limit.context),
        ));
    }
    if let Some(input) = model.limit.input
        && (input < 0.0 || !input.is_finite())
    {
        return Err(validation_error(
            provider_id,
            model_id,
            "limit.input",
            format!("input token limit must be non-negative, got {input}"),
        ));
    }
    if model.limit.output < 0.0 || !model.limit.output.is_finite() {
        return Err(validation_error(
            provider_id,
            model_id,
            "limit.output",
            format!("output token limit must be non-negative, got {}", model.limit.output),
        ));
    }
    if let Some(cost) = &model.cost {
        validate_cost(
            provider_id,
            model_id,
            "cost",
            cost.input,
            cost.output,
            &cost.reasoning,
            &cost.cache_read,
            &cost.cache_write,
            &cost.input_audio,
            &cost.output_audio,
        )?;
        if let Some(tiers) = &cost.tiers {
            let mut seen_sizes = std::collections::BTreeSet::new();
            for (index, tier) in tiers.iter().enumerate() {
                let field_prefix = format!("cost.tiers[{index}]");
                validate_cost(
                    provider_id,
                    model_id,
                    &field_prefix,
                    tier.input,
                    tier.output,
                    &tier.reasoning,
                    &tier.cache_read,
                    &tier.cache_write,
                    &tier.input_audio,
                    &tier.output_audio,
                )?;
                // ~keep Upstream's `refineModel` rejects duplicate tier sizes;
                // ~keep enforce it here too, since `cost.rs::select_tier` would
                // ~keep otherwise silently pick an arbitrary duplicate.
                if !seen_sizes.insert(tier.tier.size) {
                    return Err(validation_error(
                        provider_id,
                        model_id,
                        &format!("{field_prefix}.tier.size"),
                        format!("duplicate cost tier size {}", tier.tier.size),
                    ));
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_cost(
    provider_id: &str,
    model_id: &str,
    field_prefix: &str,
    input: f64,
    output: f64,
    reasoning: &Option<f64>,
    cache_read: &Option<f64>,
    cache_write: &Option<f64>,
    input_audio: &Option<f64>,
    output_audio: &Option<f64>,
) -> Result<(), CatalogGenError> {
    let checks: [(&str, f64); 2] = [("input", input), ("output", output)];
    for (name, value) in checks {
        if value < 0.0 || !value.is_finite() {
            return Err(validation_error(
                provider_id,
                model_id,
                &format!("{field_prefix}.{name}"),
                format!("cost must be non-negative, got {value}"),
            ));
        }
    }
    let optional_checks: [(&str, &Option<f64>); 5] = [
        ("reasoning", reasoning),
        ("cache_read", cache_read),
        ("cache_write", cache_write),
        ("input_audio", input_audio),
        ("output_audio", output_audio),
    ];
    for (name, value) in optional_checks {
        if let Some(value) = value
            && (*value < 0.0 || !value.is_finite())
        {
            return Err(validation_error(
                provider_id,
                model_id,
                &format!("{field_prefix}.{name}"),
                format!("cost must be non-negative, got {value}"),
            ));
        }
    }
    Ok(())
}
