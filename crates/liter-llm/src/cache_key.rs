//! Pluggable cache key derivation strategies.
//!
//! [`CacheKeyStrategy`] is the extension point for controlling how a cache key
//! (a `u64` hash) and a canonical body string are derived from an incoming
//! request.  The canonical body is stored alongside the cached response and
//! compared on lookup to guard against 64-bit hash collisions.
//!
//! # Built-in implementations
//!
//! | Strategy | Description |
//! |---|---|
//! | [`ExactHashStrategy`] | Hash the full serialized request body (default). |
//! | [`SystemPromptAwareStrategy`] | Hash includes the resolved system prompt but ignores per-call metadata noise. |
//! | [`TenantScopedStrategy`] | Adds a tenant prefix so two tenants requesting the same prompt get separate cache entries. |

use std::hash::{DefaultHasher, Hash, Hasher};

// ── CacheKeyInput ─────────────────────────────────────────────────────────────

/// Input to a [`CacheKeyStrategy`].
///
/// Constructed by [`CacheService`][crate::tower::cache::CacheService] from the
/// incoming [`LlmRequest`][crate::tower::types::LlmRequest] before the strategy
/// is consulted.
pub struct CacheKeyInput<'a> {
    /// Model identifier, e.g. `"openai/gpt-4o"`.
    pub model: &'a str,
    /// JSON-serialized messages array extracted from the request.
    pub messages_json: &'a str,
    /// JSON-serialized inference parameters (temperature, top_p, etc.).
    pub params_json: &'a str,
    /// Optional tenant identifier for multi-tenant deployments.
    pub tenant_id: Option<&'a str>,
    /// Optional system prompt that has been resolved (expanded, loaded from a
    /// file, etc.) before the request reaches the cache layer.
    pub system_prompt: Option<&'a str>,
}

// ── CacheKeyStrategy trait ────────────────────────────────────────────────────

/// Pluggable key derivation strategy for the response cache.
///
/// Implement this trait to control how a `(u64, String)` pair is derived from
/// a [`CacheKeyInput`].  The `u64` is used as the primary cache index; the
/// `String` is the canonical serialized body stored alongside the cached
/// response and compared on lookup to guard against hash collisions.
///
/// # Object safety
///
/// The trait is object-safe; implementations can be stored behind
/// `Arc<dyn CacheKeyStrategy>`.
pub trait CacheKeyStrategy: Send + Sync + 'static {
    /// Derive a `u64` hash key and a canonical serialized request body for
    /// collision-guard comparison.
    ///
    /// Determinism is required: identical inputs must produce identical outputs
    /// across calls (though not necessarily across process restarts, since
    /// `DefaultHasher` is not stable across Rust versions).
    fn key_for(&self, input: &CacheKeyInput<'_>) -> (u64, String);
}

// ── ExactHashStrategy ─────────────────────────────────────────────────────────

/// Hash the entire serialized request (model + messages + params).
///
/// This is the default behavior: two requests are cache-equivalent only when
/// their full serialization is byte-identical.  Tenant and system-prompt fields
/// are included verbatim.
#[derive(Debug, Clone, Default)]
pub struct ExactHashStrategy;

impl CacheKeyStrategy for ExactHashStrategy {
    fn key_for(&self, input: &CacheKeyInput<'_>) -> (u64, String) {
        // Canonical body: join all relevant fields with a delimiter that cannot
        // appear inside a JSON string without escaping.
        let body = format!(
            "{}|{}|{}|{}|{}",
            input.model,
            input.messages_json,
            input.params_json,
            input.tenant_id.unwrap_or(""),
            input.system_prompt.unwrap_or(""),
        );
        let mut hasher = DefaultHasher::new();
        body.hash(&mut hasher);
        (hasher.finish(), body)
    }
}

// ── SystemPromptAwareStrategy ─────────────────────────────────────────────────

/// Hash includes the resolved system prompt but ignores per-call metadata
/// noise (tenant ID).
///
/// Useful when a resolved system prompt should be part of cache identity but
/// the caller wants tenant isolation to be handled at a higher layer, or when
/// per-call metadata such as a request trace ID would otherwise fragment the
/// cache unnecessarily.
#[derive(Debug, Clone, Default)]
pub struct SystemPromptAwareStrategy;

impl CacheKeyStrategy for SystemPromptAwareStrategy {
    fn key_for(&self, input: &CacheKeyInput<'_>) -> (u64, String) {
        // Omit tenant_id so two tenants with the same prompt share a cache slot.
        let body = format!(
            "{}|{}|{}|{}",
            input.model,
            input.messages_json,
            input.params_json,
            input.system_prompt.unwrap_or(""),
        );
        let mut hasher = DefaultHasher::new();
        body.hash(&mut hasher);
        (hasher.finish(), body)
    }
}

// ── TenantScopedStrategy ──────────────────────────────────────────────────────

/// Adds a tenant prefix so two tenants requesting the same prompt receive
/// separate cache entries.
///
/// When no `tenant_id` is present in the input, this strategy behaves
/// identically to [`ExactHashStrategy`].
#[derive(Debug, Clone, Default)]
pub struct TenantScopedStrategy;

impl CacheKeyStrategy for TenantScopedStrategy {
    fn key_for(&self, input: &CacheKeyInput<'_>) -> (u64, String) {
        // Tenant prefix is included first so the same body under different
        // tenants always produces a different key.
        let body = format!(
            "tenant:{}|{}|{}|{}|{}",
            input.tenant_id.unwrap_or("__global__"),
            input.model,
            input.messages_json,
            input.params_json,
            input.system_prompt.unwrap_or(""),
        );
        let mut hasher = DefaultHasher::new();
        body.hash(&mut hasher);
        (hasher.finish(), body)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn input<'a>(
        model: &'a str,
        messages_json: &'a str,
        params_json: &'a str,
        tenant_id: Option<&'a str>,
        system_prompt: Option<&'a str>,
    ) -> CacheKeyInput<'a> {
        CacheKeyInput {
            model,
            messages_json,
            params_json,
            tenant_id,
            system_prompt,
        }
    }

    // ── ExactHashStrategy ─────────────────────────────────────────────────────

    #[test]
    fn exact_hash_strategy_is_deterministic() {
        let s = ExactHashStrategy;
        let i = input("gpt-4", r#"[{"role":"user","content":"hi"}]"#, "{}", None, None);
        let (k1, b1) = s.key_for(&i);
        let (k2, b2) = s.key_for(&i);
        assert_eq!(k1, k2, "key must be deterministic");
        assert_eq!(b1, b2, "body must be deterministic");
    }

    #[test]
    fn exact_hash_strategy_distinct_inputs_produce_distinct_keys() {
        let s = ExactHashStrategy;
        let i1 = input("gpt-4", r#"[{"role":"user","content":"hello"}]"#, "{}", None, None);
        let i2 = input("gpt-4", r#"[{"role":"user","content":"world"}]"#, "{}", None, None);
        let (k1, _) = s.key_for(&i1);
        let (k2, _) = s.key_for(&i2);
        assert_ne!(k1, k2, "distinct prompts must produce distinct keys");
    }

    #[test]
    fn exact_hash_strategy_different_models_produce_distinct_keys() {
        let s = ExactHashStrategy;
        let msgs = r#"[{"role":"user","content":"hi"}]"#;
        let i1 = input("gpt-4", msgs, "{}", None, None);
        let i2 = input("gpt-4o", msgs, "{}", None, None);
        let (k1, _) = s.key_for(&i1);
        let (k2, _) = s.key_for(&i2);
        assert_ne!(k1, k2);
    }

    #[test]
    fn exact_hash_strategy_different_tenants_produce_distinct_keys() {
        let s = ExactHashStrategy;
        let msgs = r#"[{"role":"user","content":"hi"}]"#;
        let i1 = input("gpt-4", msgs, "{}", Some("tenant-a"), None);
        let i2 = input("gpt-4", msgs, "{}", Some("tenant-b"), None);
        let (k1, _) = s.key_for(&i1);
        let (k2, _) = s.key_for(&i2);
        assert_ne!(k1, k2, "exact hash must differentiate tenants");
    }

    // ── SystemPromptAwareStrategy ─────────────────────────────────────────────

    #[test]
    fn system_prompt_aware_strategy_is_deterministic() {
        let s = SystemPromptAwareStrategy;
        let i = input(
            "gpt-4",
            r#"[{"role":"user","content":"hi"}]"#,
            "{}",
            None,
            Some("be helpful"),
        );
        let (k1, b1) = s.key_for(&i);
        let (k2, b2) = s.key_for(&i);
        assert_eq!(k1, k2);
        assert_eq!(b1, b2);
    }

    #[test]
    fn system_prompt_aware_strategy_different_system_prompts_produce_distinct_keys() {
        let s = SystemPromptAwareStrategy;
        let msgs = r#"[{"role":"user","content":"hi"}]"#;
        let i1 = input("gpt-4", msgs, "{}", None, Some("be helpful"));
        let i2 = input("gpt-4", msgs, "{}", None, Some("be concise"));
        let (k1, _) = s.key_for(&i1);
        let (k2, _) = s.key_for(&i2);
        assert_ne!(k1, k2);
    }

    #[test]
    fn system_prompt_aware_strategy_ignores_tenant_id() {
        let s = SystemPromptAwareStrategy;
        let msgs = r#"[{"role":"user","content":"hi"}]"#;
        let i1 = input("gpt-4", msgs, "{}", Some("tenant-a"), None);
        let i2 = input("gpt-4", msgs, "{}", Some("tenant-b"), None);
        let (k1, _) = s.key_for(&i1);
        let (k2, _) = s.key_for(&i2);
        assert_eq!(k1, k2, "system-prompt-aware strategy should ignore tenant_id");
    }

    // ── TenantScopedStrategy ──────────────────────────────────────────────────

    #[test]
    fn tenant_scoped_strategy_is_deterministic() {
        let s = TenantScopedStrategy;
        let i = input("gpt-4", r#"[{"role":"user","content":"hi"}]"#, "{}", Some("acme"), None);
        let (k1, b1) = s.key_for(&i);
        let (k2, b2) = s.key_for(&i);
        assert_eq!(k1, k2);
        assert_eq!(b1, b2);
    }

    #[test]
    fn tenant_scoped_strategy_different_tenants_same_prompt_produce_distinct_keys() {
        let s = TenantScopedStrategy;
        let msgs = r#"[{"role":"user","content":"hi"}]"#;
        let i1 = input("gpt-4", msgs, "{}", Some("acme"), None);
        let i2 = input("gpt-4", msgs, "{}", Some("globex"), None);
        let (k1, _) = s.key_for(&i1);
        let (k2, _) = s.key_for(&i2);
        assert_ne!(k1, k2, "different tenants must produce different keys");
    }

    #[test]
    fn tenant_scoped_strategy_no_tenant_uses_global_prefix() {
        let s = TenantScopedStrategy;
        let msgs = r#"[{"role":"user","content":"hi"}]"#;
        // Both have no tenant — should be equal.
        let i1 = input("gpt-4", msgs, "{}", None, None);
        let i2 = input("gpt-4", msgs, "{}", None, None);
        let (k1, _) = s.key_for(&i1);
        let (k2, _) = s.key_for(&i2);
        assert_eq!(
            k1, k2,
            "two requests without tenant_id should share a key under tenant-scoped strategy"
        );
    }

    // ── Cross-strategy isolation ───────────────────────────────────────────────

    #[test]
    fn strategies_can_produce_different_keys_for_same_input() {
        let exact = ExactHashStrategy;
        let tenant = TenantScopedStrategy;
        let msgs = r#"[{"role":"user","content":"hi"}]"#;
        let i = input("gpt-4", msgs, "{}", Some("acme"), None);
        let (ke, _) = exact.key_for(&i);
        let (kt, _) = tenant.key_for(&i);
        // Exact and tenant-scoped strategies produce different bodies, so their
        // keys may differ (hash of different strings).
        // We assert the bodies differ since the prefixing changes the content.
        let (_, be) = exact.key_for(&i);
        let (_, bt) = tenant.key_for(&i);
        assert_ne!(be, bt, "bodies produced by different strategies must differ");
        // Keys derived from different bodies will very likely differ too, but
        // we cannot assert inequality absolutely (hash collisions, however rare).
        let _ = (ke, kt); // suppress unused warning
    }
}
