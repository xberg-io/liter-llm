//! Integration tests for the fetch-free pipeline: parse -> validate ->
//! transform -> render. All tests run entirely offline against the
//! committed `tests/fixtures/sample_api.json` fixture (or inline JSON
//! literals for malformed-input cases) — no network access.

use std::collections::BTreeSet;

use liter_llm_catalog_gen::transform::{
    CatalogCapabilities, CatalogDocument, CatalogLimit, CatalogModalities, CatalogModel, CatalogPricing,
    CatalogPricingTier, transform_catalog,
};
use liter_llm_catalog_gen::{
    CatalogGenError, Provenance, build_catalog_document, build_provenance, default_output_paths, fetch_catalog_text,
    parse_and_validate, stale_paths, write_document,
};

/// Block on a future without pulling in tokio's test macros. The allowlist
/// guards under test reject before any network I/O, so a current-thread
/// runtime never actually touches the network.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("current-thread runtime builds")
        .block_on(future)
}

#[test]
fn should_reject_a_non_https_fetch_url() {
    let error = block_on(fetch_catalog_text("http://models.dev/api.json"))
        .expect_err("plaintext http must be refused before any network I/O");
    assert!(matches!(error, CatalogGenError::InsecureUrl(_)), "got {error:?}");
}

#[test]
fn should_reject_a_fetch_url_whose_host_is_not_allowlisted() {
    let error = block_on(fetch_catalog_text("https://evil.example/api.json"))
        .expect_err("a non-allowlisted host must be refused before any network I/O");
    match error {
        CatalogGenError::DisallowedHost { host, .. } => assert_eq!(host, "evil.example"),
        other => panic!("expected DisallowedHost, got {other:?}"),
    }
}

const FIXTURE: &str = include_str!("fixtures/sample_api.json");

/// Total number of models across every provider in the fixture, now that
/// unpriced models are included rather than skipped.
const FIXTURE_MODEL_COUNT: usize = 6;

fn per_token(cost_per_million: f64) -> f64 {
    cost_per_million / 1_000_000.0
}

fn fixture_document() -> CatalogDocument {
    let catalog = parse_and_validate(FIXTURE).expect("fixture must parse and validate cleanly");
    transform_catalog(&catalog)
}

fn fixture_provenance(fetched_on: &str) -> Provenance {
    build_provenance(FIXTURE, fetched_on)
}

fn model<'a>(document: &'a CatalogDocument, provider_id: &str, model_id: &str) -> &'a CatalogModel {
    document
        .get(provider_id)
        .unwrap_or_else(|| panic!("provider `{provider_id}` must be present"))
        .models
        .get(model_id)
        .unwrap_or_else(|| panic!("model `{provider_id}/{model_id}` must be present"))
}

#[test]
fn should_parse_and_validate_the_fixture_without_error() {
    let catalog = parse_and_validate(FIXTURE).expect("fixture must be valid");
    assert_eq!(catalog.len(), 4, "fixture defines four providers");
    assert!(catalog.contains_key("anthropic"));
    assert!(catalog.contains_key("google"));
    assert!(catalog.contains_key("openai"));
    assert!(catalog.contains_key("unlisted-provider"));
}

#[test]
fn should_include_every_model_including_ones_with_no_cost_object() {
    let document = fixture_document();
    let total_models: usize = document.values().map(|provider| provider.models.len()).sum();
    assert_eq!(
        total_models, FIXTURE_MODEL_COUNT,
        "every model in the fixture must appear in the catalog, priced or not"
    );

    let free_model = model(&document, "unlisted-provider", "free-model");
    assert!(
        free_model.pricing.is_none(),
        "free-model has no cost.input and must omit the pricing object, not be dropped"
    );
}

#[test]
fn should_transform_a_plain_chat_model_exactly() {
    let document = fixture_document();
    let record = model(&document, "openai", "gpt-plain");

    let expected = CatalogModel {
        id: "gpt-plain".to_string(),
        name: "GPT Plain".to_string(),
        pricing: Some(CatalogPricing {
            input_cost_per_token: per_token(2.5),
            output_cost_per_token: per_token(10.0),
            cache_read_input_token_cost: None,
            cache_creation_input_token_cost: None,
            input_cost_per_audio_token: None,
            output_cost_per_audio_token: None,
            output_cost_per_reasoning_token: None,
            tiers: vec![],
        }),
        limit: CatalogLimit {
            context: 128_000,
            input: Some(120_000),
            output: 8_000,
        },
        modalities: CatalogModalities {
            input: vec!["text".to_string()],
            output: vec!["text".to_string()],
        },
        mode: Some("chat".to_string()),
        capabilities: CatalogCapabilities {
            vision: false,
            function_calling: true,
            reasoning: false,
            structured_output: true,
            audio_input: false,
            audio_output: false,
            prompt_caching: false,
            attachment: false,
            open_weights: false,
        },
        shape: None,
        family: None,
        knowledge: None,
        release_date: "2024-01-01".to_string(),
        last_updated: "2024-01-01".to_string(),
    };
    assert_eq!(record, &expected);
}

#[test]
fn should_transform_a_cost_only_model_with_identity_fields_and_no_mode() {
    let document = fixture_document();
    let record = model(&document, "openai", "gpt-cost-only");

    let expected = CatalogModel {
        id: "gpt-cost-only".to_string(),
        name: "GPT Cost Only".to_string(),
        pricing: Some(CatalogPricing {
            input_cost_per_token: per_token(1.0),
            output_cost_per_token: per_token(2.0),
            cache_read_input_token_cost: None,
            cache_creation_input_token_cost: None,
            input_cost_per_audio_token: None,
            output_cost_per_audio_token: None,
            output_cost_per_reasoning_token: None,
            tiers: vec![],
        }),
        limit: CatalogLimit {
            context: 4_096,
            input: None,
            output: 4_096,
        },
        modalities: CatalogModalities {
            input: vec!["text".to_string()],
            output: vec![],
        },
        mode: None,
        capabilities: CatalogCapabilities {
            vision: false,
            function_calling: false,
            reasoning: false,
            structured_output: false,
            audio_input: false,
            audio_output: false,
            prompt_caching: false,
            attachment: false,
            open_weights: false,
        },
        shape: Some("responses".to_string()),
        family: Some("gpt-4".to_string()),
        knowledge: Some("2023-04".to_string()),
        release_date: "2023-06-01".to_string(),
        last_updated: "2023-06-01".to_string(),
    };
    assert_eq!(record, &expected);
}

#[test]
fn should_omit_pricing_entirely_for_a_model_with_no_cost() {
    let document = fixture_document();
    let record = model(&document, "unlisted-provider", "free-model");

    let expected = CatalogModel {
        id: "free-model".to_string(),
        name: "Free Model".to_string(),
        pricing: None,
        limit: CatalogLimit {
            context: 8_192,
            input: None,
            output: 8_192,
        },
        modalities: CatalogModalities {
            input: vec!["text".to_string()],
            output: vec!["text".to_string()],
        },
        mode: Some("chat".to_string()),
        capabilities: CatalogCapabilities {
            vision: false,
            function_calling: false,
            reasoning: false,
            structured_output: false,
            audio_input: false,
            audio_output: false,
            prompt_caching: false,
            attachment: false,
            open_weights: true,
        },
        shape: None,
        family: None,
        knowledge: None,
        release_date: "2024-01-01".to_string(),
        last_updated: "2024-01-01".to_string(),
    };
    assert_eq!(record, &expected);
}

#[test]
fn should_transform_a_reasoning_cost_model_exactly() {
    let document = fixture_document();
    let record = model(&document, "anthropic", "claude-reasoning");
    let pricing = record.pricing.as_ref().expect("claude-reasoning must be priced");

    assert_eq!(pricing.output_cost_per_reasoning_token, Some(per_token(20.0)));
    assert_eq!(pricing.input_cost_per_token, per_token(5.0));
    assert_eq!(pricing.output_cost_per_token, per_token(20.0));
    assert!(pricing.tiers.is_empty());
    assert!(
        record.capabilities.reasoning,
        "reasoning: true upstream must set capabilities.reasoning"
    );
    assert_eq!(record.mode.as_deref(), Some("chat"));
}

#[test]
fn should_transform_tiered_pricing_and_ignore_context_over_200k() {
    let document = fixture_document();
    let record = model(&document, "anthropic", "claude-tiered");
    let pricing = record.pricing.as_ref().expect("claude-tiered must be priced");

    assert!(record.capabilities.vision, "modalities.input contains image");
    assert!(
        record.capabilities.prompt_caching,
        "cost.cache_read/cache_write are present"
    );
    assert!(record.capabilities.attachment, "attachment: true upstream");
    assert_eq!(record.limit.context, 400_000);
    assert_eq!(record.limit.input, Some(272_000));
    assert_eq!(record.limit.output, 128_000);

    let expected_tier = CatalogPricingTier {
        min_context_tokens: 200_000,
        input_cost_per_token: per_token(6.0),
        output_cost_per_token: per_token(22.5),
        cache_read_input_token_cost: Some(per_token(0.6)),
        cache_creation_input_token_cost: Some(per_token(7.5)),
        input_cost_per_audio_token: None,
        output_cost_per_audio_token: None,
        output_cost_per_reasoning_token: None,
    };
    assert_eq!(
        pricing.tiers,
        vec![expected_tier],
        "cost.context_over_200k must be ignored, only cost.tiers used"
    );
}

#[test]
fn should_transform_audio_costs_and_infer_audio_speech_mode() {
    let document = fixture_document();
    let record = model(&document, "google", "gemini-audio");
    let pricing = record.pricing.as_ref().expect("gemini-audio must be priced");

    assert_eq!(pricing.input_cost_per_audio_token, Some(per_token(2.5)));
    assert_eq!(pricing.output_cost_per_audio_token, Some(per_token(10.0)));
    assert!(record.capabilities.audio_input);
    assert!(record.capabilities.audio_output);
    assert!(record.capabilities.attachment, "attachment: true upstream");
    assert_eq!(record.mode.as_deref(), Some("audio_speech"));
}

#[test]
fn should_always_render_a_fully_populated_capabilities_object_for_every_model() {
    let catalog = parse_and_validate(FIXTURE).expect("fixture must be valid");
    let provenance = fixture_provenance("2024-01-01");
    let document = build_catalog_document(&catalog, &provenance).expect("render must succeed");
    let value: serde_json::Value = serde_json::from_str(&document).expect("rendered document must be valid JSON");

    let expected_keys: BTreeSet<&str> = [
        "vision",
        "function_calling",
        "reasoning",
        "structured_output",
        "audio_input",
        "audio_output",
        "prompt_caching",
        "attachment",
        "open_weights",
    ]
    .into_iter()
    .collect();

    let providers = value["providers"].as_object().expect("providers must be an object");
    let mut visited = 0;
    for provider in providers.values() {
        let models = provider["models"].as_object().expect("models must be an object");
        for model in models.values() {
            visited += 1;
            let capabilities = model["capabilities"]
                .as_object()
                .expect("capabilities must always be present as an object");
            let keys: BTreeSet<&str> = capabilities.keys().map(String::as_str).collect();
            assert_eq!(
                keys, expected_keys,
                "capabilities must always carry all nine boolean flags, priced or not"
            );
            for flag_value in capabilities.values() {
                assert!(flag_value.is_boolean(), "every capability flag must be a JSON boolean");
            }
        }
    }
    assert_eq!(visited, FIXTURE_MODEL_COUNT);
}

#[test]
fn should_render_deterministically_byte_identical_across_runs() {
    let catalog = parse_and_validate(FIXTURE).expect("fixture must be valid");
    let provenance = fixture_provenance("2024-01-01");
    let first = build_catalog_document(&catalog, &provenance).expect("first render must succeed");
    let second = build_catalog_document(&catalog, &provenance).expect("second render must succeed");
    assert_eq!(first, second, "rendering the same catalog twice must be byte-identical");
}

#[test]
fn should_render_two_space_indentation_not_tabs() {
    let catalog = parse_and_validate(FIXTURE).expect("fixture must be valid");
    let provenance = fixture_provenance("2024-01-01");
    let document = build_catalog_document(&catalog, &provenance).expect("render must succeed");
    assert!(
        !document.contains('\t'),
        "rendered output must not contain tab characters"
    );
    assert!(
        document.contains("  \"$schema_version\": 1,\n"),
        "top-level fields must be 2-space indented"
    );
    assert!(document.starts_with("{\n  \"$provenance\": {\n"));
}

#[test]
fn should_round_trip_the_rendered_document_through_serde_json() {
    let catalog = parse_and_validate(FIXTURE).expect("fixture must be valid");
    let provenance = fixture_provenance("2024-01-01");
    let document = build_catalog_document(&catalog, &provenance).expect("render must succeed");
    let value: serde_json::Value = serde_json::from_str(&document).expect("rendered document must be valid JSON");
    assert_eq!(value["$schema_version"], 1);
    assert_eq!(value["$provenance"]["fetched"], "2024-01-01");
    assert_eq!(value["$provenance"]["library_version"], env!("CARGO_PKG_VERSION"));

    let providers = value["providers"].as_object().expect("providers must be a JSON object");
    let total_models: usize = providers
        .values()
        .map(|provider| provider["models"].as_object().expect("models must be an object").len())
        .sum();
    assert_eq!(total_models, FIXTURE_MODEL_COUNT);
}

#[test]
fn should_reject_a_negative_cost() {
    let malformed = r#"{
        "openai": {
            "id": "openai",
            "env": ["OPENAI_API_KEY"],
            "npm": "@ai-sdk/openai",
            "name": "OpenAI",
            "doc": "https://platform.openai.com/docs/models",
            "models": {
                "gpt-negative": {
                    "id": "gpt-negative",
                    "name": "GPT Negative",
                    "description": "A model with an invalid negative input cost.",
                    "attachment": false,
                    "reasoning": false,
                    "tool_call": false,
                    "release_date": "2024-01-01",
                    "last_updated": "2024-01-01",
                    "modalities": { "input": ["text"], "output": ["text"] },
                    "open_weights": false,
                    "limit": { "context": 4096, "output": 4096 },
                    "cost": { "input": -1.0, "output": 2.0 }
                }
            }
        }
    }"#;

    let result = parse_and_validate(malformed);
    let error = result.expect_err("negative cost.input must fail validation");
    match error {
        CatalogGenError::Validation {
            provider, model, field, ..
        } => {
            assert_eq!(provider, "openai");
            assert_eq!(model, "gpt-negative");
            assert_eq!(field, "cost.input");
        }
        other => panic!("expected CatalogGenError::Validation, got {other:?}"),
    }
}

#[test]
fn should_reject_a_negative_context_window() {
    // ~keep Upstream allows `context: 0` (image models have no text context
    // ~keep window), so only a negative value is a genuine validation failure.
    let malformed = r#"{
        "openai": {
            "id": "openai",
            "env": ["OPENAI_API_KEY"],
            "npm": "@ai-sdk/openai",
            "name": "OpenAI",
            "doc": "https://platform.openai.com/docs/models",
            "models": {
                "gpt-negative-context": {
                    "id": "gpt-negative-context",
                    "name": "GPT Negative Context",
                    "description": "A model with an invalid negative context window.",
                    "attachment": false,
                    "reasoning": false,
                    "tool_call": false,
                    "release_date": "2024-01-01",
                    "last_updated": "2024-01-01",
                    "modalities": { "input": ["text"], "output": ["text"] },
                    "open_weights": false,
                    "limit": { "context": -1, "output": 4096 },
                    "cost": { "input": 1.0, "output": 2.0 }
                }
            }
        }
    }"#;

    let result = parse_and_validate(malformed);
    let error = result.expect_err("negative context window must fail validation");
    match error {
        CatalogGenError::Validation { field, .. } => assert_eq!(field, "limit.context"),
        other => panic!("expected CatalogGenError::Validation, got {other:?}"),
    }
}

#[test]
fn should_accept_a_zero_context_window_for_image_models() {
    // ~keep Regression: real upstream data (e.g. `azure/gpt-image-1`) declares
    // ~keep `limit.context: 0`. Upstream's `z.number().min(0)` allows it, so
    // ~keep the generator must not reject it.
    let catalog = r#"{
        "azure": {
            "id": "azure",
            "env": ["AZURE_API_KEY"],
            "npm": "@ai-sdk/azure",
            "name": "Azure",
            "doc": "https://learn.microsoft.com/azure/ai-services/openai/",
            "models": {
                "gpt-image-1": {
                    "id": "gpt-image-1",
                    "name": "GPT Image 1",
                    "description": "An image-generation model with no text context window.",
                    "attachment": false,
                    "reasoning": false,
                    "tool_call": false,
                    "release_date": "2024-01-01",
                    "last_updated": "2024-01-01",
                    "modalities": { "input": ["text"], "output": ["image"] },
                    "open_weights": false,
                    "limit": { "context": 0, "output": 0 },
                    "cost": { "input": 5.0, "output": 40.0 }
                }
            }
        }
    }"#;

    let parsed = parse_and_validate(catalog).expect("zero context window must be accepted");
    let document = transform_catalog(&parsed);
    let record = model(&document, "azure", "gpt-image-1");
    assert_eq!(record.mode.as_deref(), Some("image_generation"));
    assert_eq!(record.limit.context, 0);
}

#[test]
fn should_accept_unknown_fields_on_non_strict_provider_override() {
    // ~keep The model-level `provider` object is a plain `z.object` upstream
    // ~keep (no `.strict()`), so an unrelated new key must NOT fail parsing —
    // ~keep otherwise `generate:catalog:check` breaks on a non-event.
    let catalog = r#"{
        "openai": {
            "id": "openai",
            "env": ["OPENAI_API_KEY"],
            "npm": "@ai-sdk/openai",
            "name": "OpenAI",
            "doc": "https://platform.openai.com/docs/models",
            "models": {
                "gpt-future": {
                    "id": "gpt-future",
                    "name": "GPT Future",
                    "description": "A model carrying a new upstream provider-override field.",
                    "attachment": false,
                    "reasoning": false,
                    "tool_call": false,
                    "release_date": "2024-01-01",
                    "last_updated": "2024-01-01",
                    "modalities": { "input": ["text"], "output": ["text"] },
                    "open_weights": false,
                    "limit": { "context": 8192, "output": 4096 },
                    "provider": { "npm": "@ai-sdk/openai", "brand_new_field": "tolerated" },
                    "cost": { "input": 1.0, "output": 2.0 }
                }
            }
        }
    }"#;

    parse_and_validate(catalog).expect("unknown field on a non-strict provider object must be tolerated");
}

#[test]
fn should_reject_duplicate_cost_tier_sizes() {
    // ~keep Upstream's `refineModel` forbids duplicate tier sizes; the
    // ~keep generator must catch it too, or `select_tier` picks arbitrarily.
    let malformed = r#"{
        "anthropic": {
            "id": "anthropic",
            "env": ["ANTHROPIC_API_KEY"],
            "npm": "@ai-sdk/anthropic",
            "name": "Anthropic",
            "doc": "https://docs.anthropic.com/en/docs/about-claude/models",
            "models": {
                "claude-dup-tiers": {
                    "id": "claude-dup-tiers",
                    "name": "Claude Dup Tiers",
                    "description": "A model with duplicate context tier sizes.",
                    "attachment": false,
                    "reasoning": false,
                    "tool_call": false,
                    "release_date": "2024-01-01",
                    "last_updated": "2024-01-01",
                    "modalities": { "input": ["text"], "output": ["text"] },
                    "open_weights": false,
                    "limit": { "context": 1000000, "output": 64000 },
                    "cost": {
                        "input": 3.0,
                        "output": 15.0,
                        "tiers": [
                            { "input": 6.0, "output": 22.5, "tier": { "type": "context", "size": 200000 } },
                            { "input": 7.0, "output": 25.0, "tier": { "type": "context", "size": 200000 } }
                        ]
                    }
                }
            }
        }
    }"#;

    let error = parse_and_validate(malformed).expect_err("duplicate tier sizes must fail validation");
    match error {
        CatalogGenError::Validation { field, .. } => assert_eq!(field, "cost.tiers[1].tier.size"),
        other => panic!("expected CatalogGenError::Validation, got {other:?}"),
    }
}

#[test]
fn should_reject_an_unknown_top_level_provider_field() {
    let malformed = r#"{
        "openai": {
            "id": "openai",
            "env": ["OPENAI_API_KEY"],
            "npm": "@ai-sdk/openai",
            "name": "OpenAI",
            "doc": "https://platform.openai.com/docs/models",
            "models": {},
            "totally_unexpected_new_field": true
        }
    }"#;

    let result = parse_and_validate(malformed);
    assert!(
        matches!(result, Err(CatalogGenError::Parse(_))),
        "unknown provider field must surface as a Parse (deny_unknown_fields) error, got {result:?}"
    );
}

#[test]
fn should_reject_an_unknown_field_on_a_model() {
    let malformed = r#"{
        "openai": {
            "id": "openai",
            "env": ["OPENAI_API_KEY"],
            "npm": "@ai-sdk/openai",
            "name": "OpenAI",
            "doc": "https://platform.openai.com/docs/models",
            "models": {
                "gpt-drifted": {
                    "id": "gpt-drifted",
                    "name": "GPT Drifted",
                    "description": "A model with an unknown new field.",
                    "attachment": false,
                    "reasoning": false,
                    "tool_call": false,
                    "release_date": "2024-01-01",
                    "last_updated": "2024-01-01",
                    "modalities": { "input": ["text"], "output": ["text"] },
                    "open_weights": false,
                    "limit": { "context": 4096, "output": 4096 },
                    "cost": { "input": 1.0, "output": 2.0 },
                    "some_brand_new_upstream_field": "surprise"
                }
            }
        }
    }"#;

    let result = parse_and_validate(malformed);
    assert!(
        matches!(result, Err(CatalogGenError::Parse(_))),
        "unknown model field must surface as a Parse (deny_unknown_fields) error, got {result:?}"
    );
}

#[test]
fn should_dual_write_identical_content_to_both_output_paths() {
    let temp_root = tempfile::tempdir().expect("must create a temp directory");
    let catalog = parse_and_validate(FIXTURE).expect("fixture must be valid");
    let provenance = fixture_provenance("2024-01-01");
    let document = build_catalog_document(&catalog, &provenance).expect("render must succeed");

    let paths = default_output_paths(temp_root.path());
    for path in &paths {
        write_document(path, &document).expect("write must succeed");
    }

    let [schemas_copy, crate_copy] = paths;
    assert!(schemas_copy.ends_with("schemas/catalog.json"));
    assert!(crate_copy.ends_with("crates/liter-llm/schemas/catalog.json"));

    let written_schemas = std::fs::read_to_string(&schemas_copy).expect("schemas copy must exist");
    let written_crate = std::fs::read_to_string(&crate_copy).expect("crate copy must exist");
    assert_eq!(written_schemas, document);
    assert_eq!(
        written_crate, document,
        "both dual-write targets must contain byte-identical content"
    );
}

#[test]
fn should_report_no_drift_once_both_output_paths_are_written() {
    let temp_root = tempfile::tempdir().expect("must create a temp directory");
    let catalog = parse_and_validate(FIXTURE).expect("fixture must be valid");
    let provenance = fixture_provenance("2024-01-01");
    let document = build_catalog_document(&catalog, &provenance).expect("render must succeed");
    let paths = default_output_paths(temp_root.path());

    let stale_before = stale_paths(&paths, &document).expect("stale check must succeed on missing files");
    assert_eq!(
        stale_before.len(),
        2,
        "both paths must be reported stale before anything is written"
    );

    for path in &paths {
        write_document(path, &document).expect("write must succeed");
    }

    let stale_after = stale_paths(&paths, &document).expect("stale check must succeed once written");
    assert!(
        stale_after.is_empty(),
        "no drift once both paths match the freshly rendered document"
    );
}

#[test]
fn should_ignore_provenance_drift_when_only_provenance_fields_change() {
    let temp_root = tempfile::tempdir().expect("must create a temp directory");
    let catalog = parse_and_validate(FIXTURE).expect("fixture must be valid");
    let paths = default_output_paths(temp_root.path());

    let old_provenance = build_provenance(FIXTURE, "2020-01-01");
    let old_document = build_catalog_document(&catalog, &old_provenance).expect("render must succeed");
    for path in &paths {
        write_document(path, &old_document).expect("write must succeed");
    }

    // ~keep A completely different provenance block (different source bytes
    // ~keep hashed, different fetch date) over the *same* underlying catalog
    // ~keep data must still report no drift — $provenance is wholly excluded
    // ~keep from the comparison, not just the fetch date within it.
    let new_provenance = build_provenance("entirely different upstream bytes for hashing purposes", "2020-06-15");
    let new_document = build_catalog_document(&catalog, &new_provenance).expect("render must succeed");
    assert_ne!(
        old_provenance, new_provenance,
        "the two provenance blocks must actually differ for this test to be meaningful"
    );

    let stale = stale_paths(&paths, &new_document).expect("stale check must succeed");
    assert!(
        stale.is_empty(),
        "a changed $provenance block alone must not be reported as drift"
    );
}

#[test]
fn should_report_drift_when_the_underlying_providers_payload_changes() {
    let temp_root = tempfile::tempdir().expect("must create a temp directory");
    let catalog = parse_and_validate(FIXTURE).expect("fixture must be valid");
    let provenance = fixture_provenance("2024-01-01");
    let paths = default_output_paths(temp_root.path());

    let full_document = build_catalog_document(&catalog, &provenance).expect("render must succeed");
    for path in &paths {
        write_document(path, &full_document).expect("write must succeed");
    }

    let mut trimmed_catalog = catalog.clone();
    trimmed_catalog.remove("google");
    let trimmed_document = build_catalog_document(&trimmed_catalog, &provenance).expect("render must succeed");

    let stale = stale_paths(&paths, &trimmed_document).expect("stale check must succeed");
    assert_eq!(
        stale.len(),
        2,
        "a changed providers payload must be reported as drift on both output paths"
    );
}
