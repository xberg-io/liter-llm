//! Transforms a validated models.dev [`Catalog`](crate::schema::Catalog)
//! into liter-llm's nested `catalog.json` shape: `provider -> model ->
//! {pricing?, limit, modalities, mode?, capabilities, shape?, family?,
//! knowledge?, release_date, last_updated}`.
//!
//! Every model in the upstream catalog is represented, including models with
//! no `cost` object — those simply omit the `pricing` sub-object rather than
//! being dropped, so the catalog stays a complete identifier/limit/capability
//! index even for unpriced models.

use std::collections::BTreeMap;

use crate::schema::{Catalog, Modality, Model, OutputCost, Provider, ProviderShape};

/// USD-per-1M-tokens -> USD-per-token.
const TOKENS_PER_UNIT: f64 = 1_000_000.0;

/// A single context-tiered cost override in the transformed record.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogPricingTier {
    /// Context-token threshold at/above which this tier's costs apply.
    pub min_context_tokens: u64,
    /// USD per input token at/above the threshold.
    pub input_cost_per_token: f64,
    /// USD per output token at/above the threshold.
    pub output_cost_per_token: f64,
    /// USD per cache-read token at/above the threshold.
    pub cache_read_input_token_cost: Option<f64>,
    /// USD per cache-creation token at/above the threshold.
    pub cache_creation_input_token_cost: Option<f64>,
    /// USD per input audio token at/above the threshold.
    pub input_cost_per_audio_token: Option<f64>,
    /// USD per output audio token at/above the threshold.
    pub output_cost_per_audio_token: Option<f64>,
    /// USD per reasoning token at/above the threshold.
    pub output_cost_per_reasoning_token: Option<f64>,
}

/// Per-token pricing for a single model, omitted entirely on the model when
/// the upstream entry has no `cost` object.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogPricing {
    /// USD per input token.
    pub input_cost_per_token: f64,
    /// USD per output token.
    pub output_cost_per_token: f64,
    /// USD per cache-read token.
    pub cache_read_input_token_cost: Option<f64>,
    /// USD per cache-creation token.
    pub cache_creation_input_token_cost: Option<f64>,
    /// USD per input audio token.
    pub input_cost_per_audio_token: Option<f64>,
    /// USD per output audio token.
    pub output_cost_per_audio_token: Option<f64>,
    /// USD per reasoning token.
    pub output_cost_per_reasoning_token: Option<f64>,
    /// Context-tiered cost overrides, sorted ascending by
    /// `min_context_tokens`.
    pub tiers: Vec<CatalogPricingTier>,
}

/// Token limits for a single model.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogLimit {
    /// Total context window, in tokens.
    pub context: u64,
    /// Maximum input/prompt tokens, when narrower than `context`.
    pub input: Option<u64>,
    /// Maximum output/completion tokens.
    pub output: u64,
}

/// Supported input/output content modalities, as lowercase strings (`"text"`,
/// `"audio"`, `"image"`, `"video"`, `"pdf"`).
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogModalities {
    /// Modalities accepted as input.
    pub input: Vec<String>,
    /// Modalities the model can produce as output.
    pub output: Vec<String>,
}

/// Boolean capability flags. Every field is always emitted (never
/// selectively omitted) so the catalog is self-describing.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogCapabilities {
    /// Whether the model accepts image input.
    pub vision: bool,
    /// Whether the model supports tool/function calling.
    pub function_calling: bool,
    /// Whether the model supports reasoning.
    pub reasoning: bool,
    /// Whether the model supports structured/JSON-schema output.
    pub structured_output: bool,
    /// Whether the model accepts audio input.
    pub audio_input: bool,
    /// Whether the model can produce audio output.
    pub audio_output: bool,
    /// Whether the model supports prompt caching.
    pub prompt_caching: bool,
    /// Whether the model accepts file/image attachments.
    pub attachment: bool,
    /// Whether model weights are open.
    pub open_weights: bool,
}

/// One transformed catalog model entry.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogModel {
    /// Model id, duplicated from the map key.
    pub id: String,
    /// Human-readable model name.
    pub name: String,
    /// Pricing, omitted entirely for unpriced models.
    pub pricing: Option<CatalogPricing>,
    /// Context window and per-request token limits.
    pub limit: CatalogLimit,
    /// Supported input/output content modalities.
    pub modalities: CatalogModalities,
    /// Best-effort inferred usage mode (`"chat"`, `"image_generation"`, ...).
    pub mode: Option<String>,
    /// Boolean capability flags.
    pub capabilities: CatalogCapabilities,
    /// Request shape override (`"responses"` | `"completions"`).
    pub shape: Option<String>,
    /// Model family taxonomy tag.
    pub family: Option<String>,
    /// Training data cutoff.
    pub knowledge: Option<String>,
    /// Model release date.
    pub release_date: String,
    /// Date this catalog entry was last updated.
    pub last_updated: String,
}

/// One transformed catalog provider entry.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogProvider {
    /// Human-readable provider name.
    pub name: String,
    /// Environment variable names accepted for provider credentials.
    pub env: Vec<String>,
    /// Link to provider documentation listing models.
    pub doc: String,
    /// Optional base API URL override.
    pub api: Option<String>,
    /// The npm package implementing this provider's AI SDK integration.
    pub npm: String,
    /// Models offered by this provider, keyed by model id.
    pub models: BTreeMap<String, CatalogModel>,
}

/// The full transformed catalog: `{providerId: CatalogProvider}`.
pub type CatalogDocument = BTreeMap<String, CatalogProvider>;

/// Transform every provider and model in `catalog` into the nested
/// `catalog.json` shape. Every model is represented, priced or not.
/// Iteration follows the catalog's deterministic `BTreeMap` order.
pub fn transform_catalog(catalog: &Catalog) -> CatalogDocument {
    catalog
        .iter()
        .map(|(provider_id, provider)| (provider_id.clone(), transform_provider(provider)))
        .collect()
}

fn transform_provider(provider: &Provider) -> CatalogProvider {
    CatalogProvider {
        name: provider.name.clone(),
        env: provider.env.clone(),
        doc: provider.doc.clone(),
        api: provider.api.clone(),
        npm: provider.npm.clone(),
        models: provider
            .models
            .iter()
            .map(|(model_id, model)| (model_id.clone(), transform_model(model)))
            .collect(),
    }
}

fn transform_model(model: &Model) -> CatalogModel {
    CatalogModel {
        id: model.id.clone(),
        name: model.name.clone(),
        pricing: model.cost.as_ref().map(transform_pricing),
        limit: CatalogLimit {
            context: limit_u64(model.limit.context),
            input: model.limit.input.map(limit_u64),
            output: limit_u64(model.limit.output),
        },
        modalities: CatalogModalities {
            input: model
                .modalities
                .input
                .iter()
                .map(modality_str)
                .map(str::to_string)
                .collect(),
            output: model
                .modalities
                .output
                .iter()
                .map(modality_str)
                .map(str::to_string)
                .collect(),
        },
        mode: infer_mode(&model.modalities),
        capabilities: CatalogCapabilities {
            vision: model.modalities.input.contains(&Modality::Image),
            function_calling: model.tool_call,
            reasoning: model.reasoning,
            structured_output: model.structured_output.unwrap_or(false),
            audio_input: model.modalities.input.contains(&Modality::Audio),
            audio_output: model.modalities.output.contains(&Modality::Audio),
            prompt_caching: supports_prompt_caching(model.cost.as_ref()),
            attachment: model.attachment,
            open_weights: model.open_weights,
        },
        shape: model
            .provider
            .as_ref()
            .and_then(|provider_override| provider_override.shape)
            .map(shape_str)
            .map(str::to_string),
        family: model.family.clone(),
        knowledge: model.knowledge.clone(),
        release_date: model.release_date.clone(),
        last_updated: model.last_updated.clone(),
    }
}

fn transform_pricing(cost: &OutputCost) -> CatalogPricing {
    let mut tiers: Vec<CatalogPricingTier> = cost
        .tiers
        .as_ref()
        .map(|tiers| tiers.iter().map(transform_tier).collect())
        .unwrap_or_default();
    tiers.sort_by_key(|tier| tier.min_context_tokens);

    CatalogPricing {
        input_cost_per_token: to_per_token(cost.input),
        output_cost_per_token: to_per_token(cost.output),
        cache_read_input_token_cost: cost.cache_read.map(to_per_token),
        cache_creation_input_token_cost: cost.cache_write.map(to_per_token),
        input_cost_per_audio_token: cost.input_audio.map(to_per_token),
        output_cost_per_audio_token: cost.output_audio.map(to_per_token),
        output_cost_per_reasoning_token: cost.reasoning.map(to_per_token),
        tiers,
    }
}

fn transform_tier(tier: &crate::schema::CostTier) -> CatalogPricingTier {
    CatalogPricingTier {
        min_context_tokens: tier.tier.size,
        input_cost_per_token: to_per_token(tier.input),
        output_cost_per_token: to_per_token(tier.output),
        cache_read_input_token_cost: tier.cache_read.map(to_per_token),
        cache_creation_input_token_cost: tier.cache_write.map(to_per_token),
        input_cost_per_audio_token: tier.input_audio.map(to_per_token),
        output_cost_per_audio_token: tier.output_audio.map(to_per_token),
        output_cost_per_reasoning_token: tier.reasoning.map(to_per_token),
    }
}

fn to_per_token(cost_per_million: f64) -> f64 {
    cost_per_million / TOKENS_PER_UNIT
}

/// Convert a limit value to a token count, saturating non-finite or negative
/// values to zero (already rejected by `schema::validate_catalog`, but
/// defensive here too since this module has no `Result` to propagate
/// through).
fn limit_u64(value: f64) -> u64 {
    if value.is_finite() && value >= 0.0 {
        value.round() as u64
    } else {
        0
    }
}

fn supports_prompt_caching(cost: Option<&OutputCost>) -> bool {
    cost.is_some_and(|cost| cost.cache_read.is_some() || cost.cache_write.is_some())
}

fn modality_str(modality: &Modality) -> &'static str {
    match modality {
        Modality::Text => "text",
        Modality::Audio => "audio",
        Modality::Image => "image",
        Modality::Video => "video",
        Modality::Pdf => "pdf",
    }
}

fn shape_str(shape: ProviderShape) -> &'static str {
    match shape {
        ProviderShape::Responses => "responses",
        ProviderShape::Completions => "completions",
    }
}

/// Best-effort usage-mode inference from declared modalities. There is no
/// explicit "mode" field upstream: image output first, then audio output,
/// then audio-in/text-out (transcription), then chat; no match omits the
/// field entirely.
///
/// Deviation: an "embedding" branch is not represented because the vendored
/// [`Modality`] enum (mirroring `schema.ts` exactly —
/// text/audio/image/video/pdf) has no `embedding` variant; embedding models
/// simply fall through to the final "no match" `None`.
fn infer_mode(modalities: &crate::schema::Modalities) -> Option<String> {
    if modalities.output.contains(&Modality::Image) {
        return Some("image_generation".to_string());
    }
    if modalities.output.contains(&Modality::Audio) {
        return Some("audio_speech".to_string());
    }
    if modalities.input.contains(&Modality::Audio) && modalities.output == [Modality::Text] {
        return Some("audio_transcription".to_string());
    }
    if modalities.output.contains(&Modality::Text) {
        return Some("chat".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Modalities;

    fn modalities(input: &[Modality], output: &[Modality]) -> Modalities {
        Modalities {
            input: input.to_vec(),
            output: output.to_vec(),
        }
    }

    #[test]
    fn should_infer_chat_mode_from_text_output() {
        let modalities = modalities(&[Modality::Text], &[Modality::Text]);
        assert_eq!(infer_mode(&modalities), Some("chat".to_string()));
    }

    #[test]
    fn should_infer_image_generation_mode_when_output_contains_image() {
        let modalities = modalities(&[Modality::Text], &[Modality::Image, Modality::Text]);
        assert_eq!(infer_mode(&modalities), Some("image_generation".to_string()));
    }

    #[test]
    fn should_prefer_image_generation_over_audio_speech_when_both_present() {
        let modalities = modalities(&[Modality::Text], &[Modality::Image, Modality::Audio]);
        assert_eq!(infer_mode(&modalities), Some("image_generation".to_string()));
    }

    #[test]
    fn should_infer_audio_speech_mode_when_output_contains_audio_without_image() {
        let modalities = modalities(&[Modality::Text], &[Modality::Audio, Modality::Text]);
        assert_eq!(infer_mode(&modalities), Some("audio_speech".to_string()));
    }

    #[test]
    fn should_infer_audio_transcription_mode_for_audio_in_text_out() {
        let modalities = modalities(&[Modality::Audio], &[Modality::Text]);
        assert_eq!(infer_mode(&modalities), Some("audio_transcription".to_string()));
    }

    #[test]
    fn should_not_infer_audio_transcription_when_text_output_has_extra_modalities() {
        let modalities = modalities(&[Modality::Audio], &[Modality::Text, Modality::Pdf]);
        assert_eq!(infer_mode(&modalities), Some("chat".to_string()));
    }

    #[test]
    fn should_omit_mode_when_no_heuristic_branch_matches() {
        let modalities = modalities(&[Modality::Text], &[]);
        assert_eq!(infer_mode(&modalities), None);
    }

    #[test]
    fn should_include_every_model_nested_under_its_provider() {
        let mut catalog = Catalog::new();
        let mut provider = crate::schema::Provider {
            id: "openai".to_string(),
            env: vec!["OPENAI_API_KEY".to_string()],
            npm: "@ai-sdk/openai".to_string(),
            api: None,
            name: "OpenAI".to_string(),
            doc: "https://example.com".to_string(),
            models: BTreeMap::new(),
        };
        provider.models.insert(
            "free-model".to_string(),
            crate::schema::Model {
                id: "free-model".to_string(),
                name: "Free Model".to_string(),
                description: "no cost".to_string(),
                family: None,
                attachment: false,
                reasoning: false,
                reasoning_options: None,
                tool_call: false,
                interleaved: None,
                structured_output: None,
                temperature: None,
                knowledge: None,
                release_date: "2024-01-01".to_string(),
                last_updated: "2024-01-01".to_string(),
                modalities: modalities(&[Modality::Text], &[Modality::Text]),
                open_weights: false,
                limit: crate::schema::ProviderModelLimit {
                    context: 4096.0,
                    input: None,
                    output: 4096.0,
                },
                status: None,
                experimental: None,
                provider: None,
                cost: None,
            },
        );
        catalog.insert("openai".to_string(), provider);

        let document = transform_catalog(&catalog);
        let openai = document.get("openai").expect("provider must be present");
        let model = openai
            .models
            .get("free-model")
            .expect("unpriced model must still appear");
        assert!(model.pricing.is_none(), "unpriced model must omit pricing");
        assert_eq!(model.limit.context, 4096);
    }
}
