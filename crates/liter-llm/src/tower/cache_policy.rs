//! Per-request cache tier selection and bypass policy.
//!
//! [`CachePolicy`] is consulted for every incoming request to decide which
//! tiers to try (exact hash, semantic, streaming-replay), whether to bypass the
//! cache entirely (e.g. to honour a `cache: no-store` directive), and whether
//! to apply a TTL or similarity-threshold override for this request.
//!
//! # Built-in implementation
//!
//! [`StandardCachePolicy`] is the default implementation.  It enables exact
//! and semantic tiers, honours `cache: no-store`, and applies configurable
//! TTL and similarity-threshold values.

use std::collections::HashMap;
use std::time::Duration;

/// Context passed to [`CachePolicy::decide`].
pub struct CachePolicyContext<'a> {
    /// Model identifier (e.g. `"openai/gpt-4o"`).
    pub model: &'a str,
    /// Optional tenant identifier.
    pub tenant_id: Option<&'a str>,
    /// Whether this is a streaming request.
    pub stream: bool,
    /// Arbitrary per-request metadata (headers, custom flags, etc.).
    ///
    /// Implementations may inspect `"cache"`, `"x-cache-bypass"`, or any
    /// other key injected by the caller before the layer is reached.
    pub metadata: &'a HashMap<String, String>,
}

/// Decision produced by [`CachePolicy::decide`].
///
/// All fields default to their conservative values (tiers enabled, no bypass,
/// no overrides).
#[derive(Debug, Clone)]
pub struct CacheDecision {
    /// Try the exact-hash tier.
    pub use_exact: bool,
    /// Try the semantic-similarity tier.
    pub use_semantic: bool,
    /// Attempt to join an in-progress streaming response as a follower
    /// (requires the singleflight coordinator to be wired up).
    pub use_streaming_replay: bool,
    /// Bypass the cache entirely for this request (read and write).
    pub bypass: bool,
    /// Per-request TTL override (overrides the global `CacheConfig::ttl`).
    pub ttl_override: Option<Duration>,
    /// Minimum cosine similarity for the semantic tier to count as a hit.
    pub similarity_threshold: f32,
    /// Serve a stale entry while re-fetching in the background.
    ///
    /// `None` disables stale-while-revalidate.  `Some(window)` allows serving
    /// a stale entry for up to `window` duration after its TTL has expired.
    pub stale_while_revalidate: Option<Duration>,
}

impl Default for CacheDecision {
    fn default() -> Self {
        Self {
            use_exact: true,
            use_semantic: false,
            use_streaming_replay: false,
            bypass: false,
            ttl_override: None,
            similarity_threshold: 0.95,
            stale_while_revalidate: None,
        }
    }
}

/// Pluggable per-request cache decision maker.
///
/// Implement this trait to control which cache tiers are consulted, when the
/// cache should be bypassed, and how TTL and similarity thresholds are set on
/// a per-request basis.
///
/// # Object safety
///
/// The trait is object-safe; implementations can be stored behind
/// `Arc<dyn CachePolicy>`.
pub trait CachePolicy: Send + Sync + 'static {
    /// Decide how to handle caching for the given request context.
    fn decide(&self, ctx: &CachePolicyContext<'_>) -> CacheDecision;
}

/// Standard cache policy with configurable tier selection and bypass logic.
///
/// # Behaviour
///
/// - Always enables the exact-hash tier.
/// - Enables the semantic tier when `semantic_ttl` is `Some`.
/// - Bypasses the cache when `bypass_on_no_store` is `true` and the request
///   metadata contains `"cache": "no-store"`.
/// - Applies `similarity_threshold` to the semantic tier.
#[derive(Debug, Clone)]
pub struct StandardCachePolicy {
    /// TTL for exact-cache entries.
    pub exact_ttl: Duration,
    /// TTL for semantic-cache entries (`None` disables the semantic tier).
    pub semantic_ttl: Option<Duration>,
    /// Minimum cosine similarity for a semantic cache hit.
    pub similarity_threshold: f32,
    /// When `true`, requests with `metadata["cache"] == "no-store"` bypass the cache.
    pub bypass_on_no_store: bool,
}

impl Default for StandardCachePolicy {
    fn default() -> Self {
        Self {
            exact_ttl: Duration::from_secs(300),
            semantic_ttl: None,
            similarity_threshold: 0.95,
            bypass_on_no_store: true,
        }
    }
}

impl CachePolicy for StandardCachePolicy {
    fn decide(&self, ctx: &CachePolicyContext<'_>) -> CacheDecision {
        let bypass = self.bypass_on_no_store
            && ctx
                .metadata
                .get("cache")
                .is_some_and(|v| v.eq_ignore_ascii_case("no-store"));

        CacheDecision {
            use_exact: true,
            use_semantic: self.semantic_ttl.is_some(),
            use_streaming_replay: ctx.stream,
            bypass,
            ttl_override: if bypass { None } else { Some(self.exact_ttl) },
            similarity_threshold: self.similarity_threshold,
            stale_while_revalidate: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>(model: &'a str, stream: bool, metadata: &'a HashMap<String, String>) -> CachePolicyContext<'a> {
        CachePolicyContext {
            model,
            tenant_id: None,
            stream,
            metadata,
        }
    }

    #[test]
    fn standard_policy_exact_tier_enabled_by_default() {
        let policy = StandardCachePolicy::default();
        let meta = HashMap::new();
        let decision = policy.decide(&ctx("gpt-4", false, &meta));
        assert!(decision.use_exact);
    }

    #[test]
    fn standard_policy_semantic_tier_disabled_when_no_semantic_ttl() {
        let policy = StandardCachePolicy::default();
        let meta = HashMap::new();
        let decision = policy.decide(&ctx("gpt-4", false, &meta));
        assert!(
            !decision.use_semantic,
            "semantic tier should be off when semantic_ttl is None"
        );
    }

    #[test]
    fn standard_policy_semantic_tier_enabled_when_semantic_ttl_set() {
        let policy = StandardCachePolicy {
            semantic_ttl: Some(Duration::from_secs(120)),
            ..Default::default()
        };
        let meta = HashMap::new();
        let decision = policy.decide(&ctx("gpt-4", false, &meta));
        assert!(decision.use_semantic);
    }

    #[test]
    fn standard_policy_bypass_on_no_store_header() {
        let policy = StandardCachePolicy::default();
        let mut meta = HashMap::new();
        meta.insert("cache".into(), "no-store".into());
        let decision = policy.decide(&ctx("gpt-4", false, &meta));
        assert!(decision.bypass, "should bypass when cache=no-store is present");
        assert!(
            decision.ttl_override.is_none(),
            "TTL override should be cleared when bypassing"
        );
    }

    #[test]
    fn standard_policy_bypass_on_no_store_case_insensitive() {
        let policy = StandardCachePolicy::default();
        let mut meta = HashMap::new();
        meta.insert("cache".into(), "No-Store".into());
        let decision = policy.decide(&ctx("gpt-4", false, &meta));
        assert!(decision.bypass, "bypass should be case-insensitive");
    }

    #[test]
    fn standard_policy_no_bypass_when_bypass_on_no_store_is_false() {
        let policy = StandardCachePolicy {
            bypass_on_no_store: false,
            ..Default::default()
        };
        let mut meta = HashMap::new();
        meta.insert("cache".into(), "no-store".into());
        let decision = policy.decide(&ctx("gpt-4", false, &meta));
        assert!(!decision.bypass, "should not bypass when bypass_on_no_store=false");
    }

    #[test]
    fn standard_policy_ttl_override_populated_when_not_bypassing() {
        let policy = StandardCachePolicy {
            exact_ttl: Duration::from_secs(42),
            ..Default::default()
        };
        let meta = HashMap::new();
        let decision = policy.decide(&ctx("gpt-4", false, &meta));
        assert_eq!(decision.ttl_override, Some(Duration::from_secs(42)));
    }

    #[test]
    fn standard_policy_similarity_threshold_forwarded() {
        let policy = StandardCachePolicy {
            similarity_threshold: 0.88,
            ..Default::default()
        };
        let meta = HashMap::new();
        let decision = policy.decide(&ctx("gpt-4", false, &meta));
        assert!((decision.similarity_threshold - 0.88).abs() < f32::EPSILON);
    }

    #[test]
    fn standard_policy_streaming_replay_when_stream_is_true() {
        let policy = StandardCachePolicy::default();
        let meta = HashMap::new();
        let decision = policy.decide(&ctx("gpt-4", true, &meta));
        assert!(decision.use_streaming_replay);
    }
}
