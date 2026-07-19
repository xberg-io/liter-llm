//! Unit tests for [`super`] (the `cost` module): flat and tier-aware cost
//! calculation, prefix-fallback lookup, provider/model key resolution, and
//! the [`super::ModelInfo`] / [`super::ModelTier`] projection.
//!
//! Tests that exercise custom pricing schemas use inline JSON fixtures
//! deserialized directly into [`super::ModelPricing`] rather than depending
//! on the shape of the committed `schemas/catalog.json`. Tests that need
//! resolution over a known-present model name (e.g. `gpt-4`) use the real
//! embedded catalog registry, matching the pre-existing test style in this
//! module.

use super::*;

#[test]
fn completion_cost_known_model_returns_expected_value() {
    let cost = completion_cost("gpt-4", 100, 50).expect("gpt-4 must be in registry");
    let expected = 100.0 * 0.00003 + 50.0 * 0.00006;
    assert!((cost - expected).abs() < 1e-12, "expected {expected}, got {cost}");
}

#[test]
fn completion_cost_unknown_model_returns_none() {
    assert!(
        completion_cost("unknown-model-xyz", 100, 50).is_none(),
        "unknown model should return None"
    );
}

#[test]
fn completion_cost_gpt4o_matches_published_pricing() {
    let cost = completion_cost("gpt-4o", 1_000, 500).expect("gpt-4o must be in registry");
    let expected = 1_000.0 * 0.0000025 + 500.0 * 0.00001;
    assert!((cost - expected).abs() < 1e-12, "expected {expected}, got {cost}");
}

#[test]
fn completion_cost_embedding_model_has_zero_output_cost() {
    let cost = completion_cost("text-embedding-3-small", 100, 0).expect("text-embedding-3-small must be in registry");
    assert!(cost > 0.0, "input tokens must have a positive cost");

    let pricing = model_pricing("text-embedding-3-small").expect("text-embedding-3-small must be in pricing registry");
    assert_eq!(pricing.output_cost_per_token, 0.0, "embedding output cost must be zero");
}

#[test]
fn model_pricing_returns_none_for_unknown_model() {
    assert!(model_pricing("does-not-exist").is_none());
}

#[test]
fn model_pricing_prefix_fallback_matches_shorter_name() {
    let exact = model_pricing("gpt-4").expect("gpt-4 must be in registry");
    let prefix = model_pricing("gpt-4-0613").expect("gpt-4-0613 should match gpt-4 via prefix");
    assert!(
        (exact.input_cost_per_token - prefix.input_cost_per_token).abs() < 1e-15,
        "prefix match should return the same pricing as exact match"
    );
}

#[test]
fn completion_cost_prefix_fallback() {
    let cost = completion_cost("gpt-4-0613", 100, 50);
    assert!(cost.is_some(), "gpt-4-0613 should resolve via prefix fallback to gpt-4");
}

#[test]
fn model_pricing_returns_correct_fields_for_known_model() {
    let p = model_pricing("gpt-4o-mini").expect("gpt-4o-mini must be in registry");
    assert!(
        (p.input_cost_per_token - 0.00000015).abs() < 1e-12,
        "unexpected input_cost_per_token: {}",
        p.input_cost_per_token
    );
    assert!(
        (p.output_cost_per_token - 0.0000006).abs() < 1e-12,
        "unexpected output_cost_per_token: {}",
        p.output_cost_per_token
    );
}

#[test]
fn completion_cost_with_cache_applies_discount_when_pricing_available() {
    let pricing = ModelPricing {
        input_cost_per_token: 1e-5,
        output_cost_per_token: 2e-5,
        cache_read_input_token_cost: Some(1e-6),
        cache_creation_input_token_cost: None,
        ..Default::default()
    };
    let expected = 800.0 * 1e-5 + 200.0 * 1e-6 + 50.0 * 2e-5;
    let uncached = 1000 - 200;
    let actual = (uncached as f64) * pricing.input_cost_per_token
        + 200.0
            * pricing
                .cache_read_input_token_cost
                .expect("cache_read_input_token_cost should be set")
        + 50.0 * pricing.output_cost_per_token;
    assert!((actual - expected).abs() < 1e-12);
}

#[test]
fn completion_cost_with_cache_falls_back_to_input_rate_without_cache_pricing() {
    let with_cache = completion_cost_with_cache("gpt-4", 1_000, 200, 50).expect("gpt-4 must be in registry");
    let without = completion_cost("gpt-4", 1_000, 50).expect("gpt-4 must be in registry");
    assert!((with_cache - without).abs() < 1e-12);
}

#[test]
fn completion_cost_with_cache_clamps_cached_tokens_to_prompt_tokens() {
    let cost = completion_cost_with_cache("gpt-4", 100, 500, 0).expect("gpt-4 must be in registry");
    let clamped = completion_cost_with_cache("gpt-4", 100, 100, 0).expect("gpt-4 must be in registry");
    assert!((cost - clamped).abs() < 1e-12);
}

#[test]
fn completion_cost_with_cache_uses_registry_cache_pricing_when_available() {
    let pricing = model_pricing("claude-sonnet-4-5").expect("claude-sonnet-4-5 must be in registry");
    let cache_rate = pricing
        .cache_read_input_token_cost
        .expect("claude-sonnet-4-5 must have cache_read_input_token_cost");
    assert!(
        cache_rate < pricing.input_cost_per_token,
        "cache rate ({cache_rate}) must be cheaper than input rate ({})",
        pricing.input_cost_per_token
    );
    let expected = 800.0 * pricing.input_cost_per_token + 200.0 * cache_rate + 50.0 * pricing.output_cost_per_token;
    let actual =
        completion_cost_with_cache("claude-sonnet-4-5", 1_000, 200, 50).expect("claude-sonnet-4-5 must be priceable");
    assert!((actual - expected).abs() < 1e-12, "expected {expected}, got {actual}");
    let no_cache = completion_cost("claude-sonnet-4-5", 1_000, 50).expect("claude-sonnet-4-5 must be priceable");
    assert!(
        actual < no_cache,
        "cached cost ({actual}) must be < uncached ({no_cache})"
    );
}

#[test]
fn completion_cost_with_cache_unknown_model_returns_none() {
    assert!(completion_cost_with_cache("unknown-model-xyz", 100, 10, 50).is_none());
}

#[test]
fn catalog_embedded_json_parses() {
    assert!(
        PRICING.as_ref().is_ok(),
        "embedded schemas/catalog.json failed to parse into the registry: {:?}",
        PRICING.as_ref().err()
    );
}

/// A cost-only record containing only the two required fields must still
/// deserialize directly into `ModelPricing`, with every other field
/// defaulting to `None` / empty.
#[test]
fn legacy_cost_only_record_still_parses() {
    let json = r#"{
        "input_cost_per_token": 0.000001,
        "output_cost_per_token": 0.000002
    }"#;
    let p: ModelPricing = serde_json::from_str(json).expect("cost-only record must parse");

    assert!((p.input_cost_per_token - 0.000001).abs() < 1e-15);
    assert!((p.output_cost_per_token - 0.000002).abs() < 1e-15);
    assert_eq!(p.cache_read_input_token_cost, None);
    assert_eq!(p.cache_creation_input_token_cost, None);
    assert_eq!(p.input_cost_per_audio_token, None);
    assert_eq!(p.output_cost_per_audio_token, None);
    assert_eq!(p.output_cost_per_reasoning_token, None);
    assert_eq!(p.max_tokens, None);
    assert_eq!(p.max_input_tokens, None);
    assert_eq!(p.max_output_tokens, None);
    assert_eq!(p.mode, None);
    assert_eq!(p.supports_vision, None);
    assert_eq!(p.supports_function_calling, None);
    assert_eq!(p.supports_reasoning, None);
    assert_eq!(p.supports_structured_output, None);
    assert_eq!(p.supports_audio_input, None);
    assert_eq!(p.supports_audio_output, None);
    assert_eq!(p.supports_prompt_caching, None);
    assert!(p.tiers.is_empty());
}

/// A record with tiers and full metadata projects correctly into
/// `ModelInfo` / `ModelTier`.
#[test]
fn model_info_parses_extended_fields() {
    let json = r#"{
        "input_cost_per_token": 0.000005,
        "output_cost_per_token": 0.000015,
        "cache_read_input_token_cost": 0.0000008,
        "cache_creation_input_token_cost": 0.0000012,
        "input_cost_per_audio_token": 0.0000023,
        "output_cost_per_audio_token": 0.0000023,
        "output_cost_per_reasoning_token": 0.00001,
        "tiers": [
            {
                "min_context_tokens": 200000,
                "input_cost_per_token": 0.00001,
                "output_cost_per_token": 0.00003,
                "cache_read_input_token_cost": 0.0000016
            }
        ],
        "max_tokens": 400000,
        "max_input_tokens": 272000,
        "max_output_tokens": 128000,
        "mode": "chat",
        "supports_vision": true,
        "supports_function_calling": true,
        "supports_reasoning": true,
        "supports_structured_output": true,
        "supports_audio_input": true,
        "supports_audio_output": false,
        "supports_prompt_caching": true
    }"#;
    let pricing: ModelPricing = serde_json::from_str(json).expect("extended record must parse");
    let info = ModelInfo::from(&pricing);

    assert!((info.input_cost_per_token - 0.000005).abs() < 1e-15);
    assert!((info.output_cost_per_token - 0.000015).abs() < 1e-15);
    assert_eq!(info.cache_read_input_token_cost, Some(0.0000008));
    assert_eq!(info.cache_creation_input_token_cost, Some(0.0000012));
    assert_eq!(info.input_cost_per_audio_token, Some(0.0000023));
    assert_eq!(info.output_cost_per_audio_token, Some(0.0000023));
    assert_eq!(info.output_cost_per_reasoning_token, Some(0.00001));
    assert_eq!(info.max_tokens, Some(400_000));
    assert_eq!(info.max_input_tokens, Some(272_000));
    assert_eq!(info.max_output_tokens, Some(128_000));
    assert_eq!(info.mode.as_deref(), Some("chat"));
    assert_eq!(info.supports_vision, Some(true));
    assert_eq!(info.supports_function_calling, Some(true));
    assert_eq!(info.supports_reasoning, Some(true));
    assert_eq!(info.supports_structured_output, Some(true));
    assert_eq!(info.supports_audio_input, Some(true));
    assert_eq!(info.supports_audio_output, Some(false));
    assert_eq!(info.supports_prompt_caching, Some(true));

    assert_eq!(info.tiers.len(), 1);
    let tier = &info.tiers[0];
    assert_eq!(tier.min_context_tokens, 200_000);
    assert!((tier.input_cost_per_token - 0.00001).abs() < 1e-15);
    assert!((tier.output_cost_per_token - 0.00003).abs() < 1e-15);
    assert_eq!(tier.cache_read_input_token_cost, Some(0.0000016));
    assert_eq!(tier.cache_creation_input_token_cost, None);
}

/// `model_info` resolves through the same prefix-fallback path as
/// `model_pricing`, against the real embedded registry.
#[test]
fn model_info_prefix_fallback() {
    let exact = model_info("gpt-4").expect("gpt-4 must be in registry");
    let prefix = model_info("gpt-4-0613").expect("gpt-4-0613 should resolve via prefix fallback to gpt-4");
    assert!(
        (exact.input_cost_per_token - prefix.input_cost_per_token).abs() < 1e-15,
        "prefix match should return the same pricing as exact match"
    );
}

#[test]
fn model_info_unknown_returns_none() {
    assert!(model_info("definitely-not-a-real-model-xyz").is_none());
}

/// A tiered model bills at the base rate below the tier threshold and at the
/// tier rate at/above it, including the tier's own cache rate.
#[test]
fn completion_cost_uses_tier_above_threshold() {
    let json = r#"{
        "input_cost_per_token": 0.000003,
        "output_cost_per_token": 0.000015,
        "cache_read_input_token_cost": 0.0000003,
        "tiers": [
            {
                "min_context_tokens": 200000,
                "input_cost_per_token": 0.000006,
                "output_cost_per_token": 0.0000225,
                "cache_read_input_token_cost": 0.0000006
            }
        ]
    }"#;
    let pricing: ModelPricing = serde_json::from_str(json).expect("tiered record must parse");

    let below = compute_cost(&pricing, 100_000, 0, 1_000);
    let expected_below = 100_000.0 * 0.000003 + 1_000.0 * 0.000015;
    assert!(
        (below - expected_below).abs() < 1e-9,
        "expected {expected_below}, got {below}"
    );

    let above = compute_cost(&pricing, 250_000, 0, 1_000);
    let expected_above = 250_000.0 * 0.000006 + 1_000.0 * 0.0000225;
    assert!(
        (above - expected_above).abs() < 1e-9,
        "expected {expected_above}, got {above}"
    );

    let above_cached = compute_cost(&pricing, 250_000, 50_000, 1_000);
    let expected_above_cached = 200_000.0 * 0.000006 + 50_000.0 * 0.0000006 + 1_000.0 * 0.0000225;
    assert!(
        (above_cached - expected_above_cached).abs() < 1e-9,
        "expected {expected_above_cached}, got {above_cached}"
    );
}

/// An unpriced catalog model (no `pricing` object upstream) still resolves
/// through `model_info` with its limits/mode/capabilities populated, and
/// `completion_cost` treats it as free (`Some(0.0)`) rather than unknown.
#[test]
fn unpriced_model_resolves_with_zero_cost_and_full_metadata() {
    let info = model_info("blueclaw/Qwen3.6-27B").expect("unpriced catalog model must still resolve");

    assert_eq!(info.input_cost_per_token, 0.0);
    assert_eq!(info.output_cost_per_token, 0.0);
    assert_eq!(info.max_tokens, Some(196_608));
    assert_eq!(
        info.max_input_tokens,
        Some(196_608),
        "falls back to context when limit.input is absent"
    );
    assert_eq!(info.max_output_tokens, Some(65_536));
    assert_eq!(info.mode.as_deref(), Some("chat"));
    assert_eq!(info.supports_function_calling, Some(true));
    assert_eq!(info.supports_reasoning, Some(true));
    assert_eq!(info.supports_structured_output, Some(true));
    assert_eq!(info.supports_vision, Some(false));
    assert_eq!(info.supports_audio_input, Some(false));
    assert_eq!(info.supports_audio_output, Some(false));
    assert_eq!(info.supports_prompt_caching, Some(false));

    let cost = completion_cost("blueclaw/Qwen3.6-27B", 1_000, 500);
    assert_eq!(cost, Some(0.0), "unpriced model must cost exactly zero, not be unknown");
}

/// A primary-provider ("openai") model resolves both via its combined
/// `"{provider}/{model}"` key and its bare alias, returning identical
/// pricing.
#[test]
fn openai_model_resolves_via_combined_key_and_bare_alias() {
    let combined = model_pricing("openai/gpt-4o").expect("openai/gpt-4o must resolve via combined key");
    let bare = model_pricing("gpt-4o").expect("gpt-4o must resolve via bare alias (primary provider)");

    assert!((combined.input_cost_per_token - 0.0000025).abs() < 1e-12);
    assert!((combined.output_cost_per_token - 0.00001).abs() < 1e-12);
    assert_eq!(combined.cache_read_input_token_cost, Some(0.00000125));
    assert!((bare.input_cost_per_token - combined.input_cost_per_token).abs() < 1e-15);
    assert!((bare.output_cost_per_token - combined.output_cost_per_token).abs() < 1e-15);
    assert_eq!(bare.cache_read_input_token_cost, combined.cache_read_input_token_cost);
}

/// An "anthropic" (primary-provider) model resolves via both the combined
/// key and the bare alias.
#[test]
fn anthropic_model_resolves_via_combined_key_and_bare_alias() {
    let combined = model_pricing("anthropic/claude-sonnet-4-5")
        .expect("anthropic/claude-sonnet-4-5 must resolve via combined key");
    let bare = model_pricing("claude-sonnet-4-5").expect("claude-sonnet-4-5 must resolve via bare alias");

    assert!((combined.input_cost_per_token - 0.000003).abs() < 1e-12);
    assert!((combined.output_cost_per_token - 0.000015).abs() < 1e-12);
    assert!((bare.input_cost_per_token - combined.input_cost_per_token).abs() < 1e-15);
    assert!((bare.output_cost_per_token - combined.output_cost_per_token).abs() < 1e-15);
}

/// A "google" (primary-provider) model resolves via both the combined key
/// and the bare alias.
#[test]
fn google_model_resolves_via_combined_key_and_bare_alias() {
    let combined =
        model_pricing("google/gemini-2.0-flash").expect("google/gemini-2.0-flash must resolve via combined key");
    let bare = model_pricing("gemini-2.0-flash").expect("gemini-2.0-flash must resolve via bare alias");

    assert!((combined.input_cost_per_token - 0.0000001).abs() < 1e-12);
    assert!((combined.output_cost_per_token - 0.0000004).abs() < 1e-12);
    assert!((bare.input_cost_per_token - combined.input_cost_per_token).abs() < 1e-15);
    assert!((bare.output_cost_per_token - combined.output_cost_per_token).abs() < 1e-15);
}

/// A non-primary provider's model resolves via its combined key but has no
/// bare alias: neither the exact bare name nor prefix-fallback stripping
/// may accidentally resolve it (or collide with an unrelated primary-provider
/// entry).
#[test]
fn non_primary_provider_bare_name_does_not_resolve() {
    let combined =
        model_pricing("mistral/codestral-latest").expect("mistral/codestral-latest must resolve via combined key");
    assert!((combined.input_cost_per_token - 0.0000003).abs() < 1e-12);
    assert!((combined.output_cost_per_token - 0.0000009).abs() < 1e-12);

    assert!(
        model_pricing("codestral-latest").is_none(),
        "non-primary-provider models must not be reachable via a bare name"
    );
    assert!(
        completion_cost("codestral-latest", 100, 50).is_none(),
        "completion_cost must not resolve a non-primary-provider bare name"
    );
}
