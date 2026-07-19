//! Unit tests for [`super::refresh`]: the disabled no-op path, overlay
//! install/clear via [`super::refresh::install_catalog_overlay_from_str`],
//! the air-gap fallback (a failed network refresh must never disturb the
//! active registry), and the `https`-only source URL guard.
//!
//! # Test independence
//!
//! The overlay ([`super::refresh::overlay_registry`]) is a process-global
//! `static` shared by every `completion_cost` / `completion_cost_with_cache`
//! / `model_info` call in the whole test binary — including the pre-existing
//! tests in `cost::tests`, which run concurrently on other test threads and
//! know nothing about this module. Two measures make that safe:
//!
//! 1. Every overlay-mutating test in *this* module `.await`s
//!    [`OVERLAY_TEST_LOCK`] for its full duration (an async-aware
//!    `tokio::sync::Mutex`, since the guard is held across the `.await` in
//!    `refresh_catalog` — a `std::sync::Mutex` guard held across an `.await`
//!    is a clippy `await_holding_lock` violation and a latent deadlock risk
//!    under a multi-threaded runtime) and clears the overlay both before (in
//!    case a previous run panicked while holding the lock, leaving a stale
//!    overlay installed) and after itself, so tests in this module never
//!    interleave with each other.
//! 2. The overlay fixture ([`build_overlay_fixture`]) is the **full embedded
//!    catalog, byte-for-byte, plus one new marker provider/model injected**
//!    (see [`MARKER_PROVIDER`]) rather than a reprice of an existing model.
//!    Every model any test in `cost::tests` looks up therefore resolves to
//!    the exact same price whether or not this module's overlay happens to
//!    be installed on another thread at that moment — so no explicit
//!    coordination with `cost::tests` is needed.

// ~keep `cost` re-exports `CatalogRefreshConfig`, `CatalogRefreshError`, `RefreshOutcome`,
// ~keep `clear_catalog_overlay`, `install_catalog_overlay_from_str`, and `refresh_catalog`,
// ~keep so this glob import covers the public surface. The `overlay_registry` seam is
// ~keep `pub(crate)` on `refresh` itself and is called via
// ~keep `super::refresh::overlay_registry()` below.
use super::*;

/// Serializes every test in this module that touches the process-global
/// overlay. See the module doc for why this is an async-aware `Mutex`
/// (`tokio::sync`, not `std::sync`) rather than a plain `std::sync::Mutex`.
/// `tokio::sync::Mutex` has no poisoning concept, so a guard from a
/// previously panicked test simply releases on drop — no recovery needed.
static OVERLAY_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Provider id for the marker model injected by [`build_overlay_fixture`].
/// Deliberately absent from the embedded catalog and not a
/// `PRIMARY_PROVIDERS` id, so it resolves only via the exact combined
/// `"{provider}/{model}"` key returned by [`marker_key`] — never
/// colliding with a real model.
const MARKER_PROVIDER: &str = "zzz-catalog-refresh-test-provider";
/// Model id for the marker model injected by [`build_overlay_fixture`].
const MARKER_MODEL: &str = "zzz-catalog-refresh-test-model";
/// Input cost of the marker model, distinctive enough to be unmistakable.
const MARKER_INPUT_COST: f64 = 0.05;
/// Output cost of the marker model, distinctive enough to be unmistakable.
const MARKER_OUTPUT_COST: f64 = 0.1;

/// The combined `"{provider}/{model}"` key for the marker model.
fn marker_key() -> String {
    format!("{MARKER_PROVIDER}/{MARKER_MODEL}")
}

/// Build an overlay fixture: the full embedded catalog, verbatim, plus one
/// new provider/model injected with distinctive test pricing. See the
/// module doc and [`MARKER_PROVIDER`] for why this — rather than repricing
/// an existing model — is what keeps these tests race-safe against
/// unrelated tests running concurrently elsewhere in the crate.
fn build_overlay_fixture() -> String {
    let mut value: serde_json::Value =
        serde_json::from_str(super::CATALOG_JSON).expect("embedded catalog.json must parse as JSON");
    let providers = value
        .get_mut("providers")
        .and_then(serde_json::Value::as_object_mut)
        .expect("embedded catalog.json must have a top-level `providers` object");

    let marker_provider_json = format!(
        r#"{{
            "models": {{
                "{MARKER_MODEL}": {{
                    "pricing": {{
                        "input_cost_per_token": {MARKER_INPUT_COST},
                        "output_cost_per_token": {MARKER_OUTPUT_COST}
                    }},
                    "limit": {{ "context": 128000, "output": 16384 }},
                    "mode": "chat",
                    "capabilities": {{
                        "vision": false, "function_calling": false, "reasoning": false,
                        "structured_output": false, "audio_input": false, "audio_output": false,
                        "prompt_caching": false
                    }}
                }}
            }}
        }}"#
    );
    let marker_provider_value: serde_json::Value =
        serde_json::from_str(&marker_provider_json).expect("marker provider fixture JSON must parse");
    providers.insert(MARKER_PROVIDER.to_string(), marker_provider_value);

    serde_json::to_string(&value).expect("modified catalog value must serialize")
}

/// Fixed (not random, not date-derived), test-unique cache path so this
/// suite never collides with a real refresh cache or with concurrent test
/// binaries, while staying deterministic across runs.
fn unique_cache_path(suffix: &str) -> String {
    std::env::temp_dir()
        .join(format!("liter-llm-catalog-refresh-test-{suffix}.json"))
        .to_string_lossy()
        .into_owned()
}

/// Best-effort cleanup of a test cache file; a missing file is not an error.
fn remove_cache_file(path: &str) {
    let _ = std::fs::remove_file(path);
}

/// Proves the embedded-only baseline: with runtime refresh disabled via
/// config (`enabled: false`, the crate default), `refresh_catalog` does not
/// touch the network, filesystem, or overlay, and `completion_cost` keeps
/// returning the exact embedded price.
#[tokio::test]
async fn refresh_disabled_returns_disabled_and_leaves_embedded_pricing_active() {
    let _guard = OVERLAY_TEST_LOCK.lock().await;
    clear_catalog_overlay();

    let config = CatalogRefreshConfig::default();
    assert!(
        !config.enabled,
        "CatalogRefreshConfig::default() must be opt-in (enabled: false)"
    );

    let outcome = refresh_catalog(&config).await;
    assert!(
        matches!(outcome, Ok(RefreshOutcome::Disabled)),
        "expected Ok(Disabled), got {outcome:?}"
    );

    assert!(
        super::refresh::overlay_registry().is_none(),
        "a disabled refresh must never install an overlay"
    );

    let cost = completion_cost("gpt-4", 100, 50).expect("gpt-4 must be in the embedded registry");
    let expected = 100.0 * 0.00003 + 50.0 * 0.00006;
    assert!(
        (cost - expected).abs() < 1e-12,
        "expected embedded price {expected}, got {cost}"
    );

    clear_catalog_overlay();
}

/// Proves the overlay path: installing a fixture that adds a marker model
/// makes `completion_cost` and `model_info` resolve it at the overlay's
/// price (it does not exist in the embedded catalog at all), and clearing
/// the overlay reverts to "unknown model" for that key. Exercised
/// before/after to prove `clear_catalog_overlay` is idempotent and tests
/// remain independent regardless of execution order.
#[tokio::test]
async fn install_overlay_from_str_overrides_pricing_then_clear_reverts_to_embedded() {
    let _guard = OVERLAY_TEST_LOCK.lock().await;
    clear_catalog_overlay();

    let key = marker_key();
    assert!(
        completion_cost(&key, 1_000, 500).is_none(),
        "the marker model must not exist in the embedded catalog"
    );
    assert!(
        model_info(&key).is_none(),
        "the marker model must not exist in the embedded catalog"
    );

    let fixture = build_overlay_fixture();
    install_catalog_overlay_from_str(&fixture).expect("fixture catalog JSON must parse");

    let overlay_cost =
        completion_cost(&key, 1_000, 500).expect("marker model must resolve once the overlay is installed");
    let overlay_expected = 1_000.0 * MARKER_INPUT_COST + 500.0 * MARKER_OUTPUT_COST;
    assert!(
        (overlay_cost - overlay_expected).abs() < 1e-9,
        "expected overlay price {overlay_expected}, got {overlay_cost}"
    );

    let info = model_info(&key).expect("marker model must resolve once the overlay is installed");
    assert!(
        (info.input_cost_per_token - MARKER_INPUT_COST).abs() < 1e-12,
        "model_info must reflect the overlay's input_cost_per_token, got {}",
        info.input_cost_per_token
    );
    assert!(
        (info.output_cost_per_token - MARKER_OUTPUT_COST).abs() < 1e-12,
        "model_info must reflect the overlay's output_cost_per_token, got {}",
        info.output_cost_per_token
    );

    // ~keep The overlay is the full embedded catalog plus the marker: an
    // ~keep unrelated, real embedded model must still resolve to its normal
    // ~keep (embedded) price through the overlay, proving the overlay is a
    // ~keep faithful superset rather than a stripped-down replacement.
    let gpt4o_cost = completion_cost("gpt-4o", 1_000, 500).expect("gpt-4o must still resolve through the overlay");
    let gpt4o_expected = 1_000.0 * 0.0000025 + 500.0 * 0.00001;
    assert!(
        (gpt4o_cost - gpt4o_expected).abs() < 1e-12,
        "expected unrelated embedded model to keep its embedded price {gpt4o_expected}, got {gpt4o_cost}"
    );

    clear_catalog_overlay();

    assert!(
        completion_cost(&key, 1_000, 500).is_none(),
        "clearing the overlay must revert the marker model to unresolved"
    );
    assert!(
        model_info(&key).is_none(),
        "clearing the overlay must revert the marker model to unresolved"
    );
}

/// Air-gap fallback: an enabled refresh against an unreachable `https` host
/// with no usable cache must return `Err`, but must NOT disturb the active
/// registry — `completion_cost` keeps returning the embedded value. This is
/// the core air-gap-safety guarantee: a refresh attempt in an offline or
/// firewalled environment degrades gracefully instead of breaking pricing.
#[tokio::test]
async fn refresh_against_unreachable_host_fails_but_embedded_pricing_still_works() {
    let _guard = OVERLAY_TEST_LOCK.lock().await;
    clear_catalog_overlay();

    let cache_path = unique_cache_path("air-gap-fallback");
    remove_cache_file(&cache_path);

    let config = CatalogRefreshConfig {
        enabled: true,
        // ~keep Port 1 is not a listening HTTP service; this must fail fast with a
        // ~keep connection error rather than hang or succeed.
        source_url: "https://127.0.0.1:1/catalog.json".to_string(),
        ttl_seconds: 86_400,
        cache_path: Some(cache_path.clone()),
    };

    let outcome = refresh_catalog(&config).await;
    assert!(
        outcome.is_err(),
        "refresh against an unreachable host must fail, got {outcome:?}"
    );
    assert!(
        matches!(outcome, Err(CatalogRefreshError::Fetch { .. })),
        "expected Fetch error, got {outcome:?}"
    );

    assert!(
        super::refresh::overlay_registry().is_none(),
        "a failed refresh must never install an overlay (air-gap safety)"
    );

    let cost = completion_cost("gpt-4o", 1_000, 500).expect("gpt-4o must still resolve via the embedded catalog");
    let expected = 1_000.0 * 0.0000025 + 500.0 * 0.00001;
    assert!(
        (cost - expected).abs() < 1e-12,
        "expected embedded fallback price {expected}, got {cost}"
    );

    remove_cache_file(&cache_path);
    clear_catalog_overlay();
}

/// A non-`https` `source_url` is rejected before any network activity, with
/// [`CatalogRefreshError::InsecureUrl`].
#[tokio::test]
async fn refresh_with_non_https_url_returns_insecure_url_error() {
    let _guard = OVERLAY_TEST_LOCK.lock().await;
    clear_catalog_overlay();

    let cache_path = unique_cache_path("insecure-url");
    remove_cache_file(&cache_path);

    let config = CatalogRefreshConfig {
        enabled: true,
        source_url: "http://example.com/catalog.json".to_string(),
        ttl_seconds: 86_400,
        cache_path: Some(cache_path.clone()),
    };

    let outcome = refresh_catalog(&config).await;
    assert!(
        matches!(outcome, Err(CatalogRefreshError::InsecureUrl { .. })),
        "expected InsecureUrl error, got {outcome:?}"
    );
    assert!(
        super::refresh::overlay_registry().is_none(),
        "a rejected URL must never install an overlay"
    );

    remove_cache_file(&cache_path);
    clear_catalog_overlay();
}
