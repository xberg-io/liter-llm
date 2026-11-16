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
//!
//! # Hash stability
//!
//! All built-in strategies use [`ahash::RandomState`] with four fixed
//! compile-time seeds.  Identical inputs produce identical `u64` keys
//! **across processes using the same build**. Hash stability is not guaranteed
//! across ahash versions, platforms, or compiler configurations; persistent
//! caches may need invalidation when these change.
//!
//! The canonical body string is also returned alongside the key so that
//! callers can guard against the (astronomically rare) 64-bit hash collision
//! by comparing the body on lookup.

use std::hash::{BuildHasher, Hash, Hasher};

use ahash::RandomState;

/// Fixed seeds for the ahash [`RandomState`] used by all built-in cache key
/// strategies.
///
/// These constants MUST NOT be changed after cache entries have been persisted,
/// as changing them would invalidate all existing cache keys.
const CACHE_KEY_SEED_0: u64 = 0x6c69_7465_725f_6c6c;
const CACHE_KEY_SEED_1: u64 = 0x6d5f_6361_6368_655f;
const CACHE_KEY_SEED_2: u64 = 0x6b65_795f_7374_7261;
const CACHE_KEY_SEED_3: u64 = 0x7465_6779_5f76_3100;

/// The process-global deterministic random state.  Constructed once from
/// compile-time-fixed seeds so the same input always yields the same hash,
/// across process restarts with the same build.
fn cache_random_state() -> &'static RandomState {
    use std::sync::OnceLock;
    static STATE: OnceLock<RandomState> = OnceLock::new();
    STATE
        .get_or_init(|| RandomState::with_seeds(CACHE_KEY_SEED_0, CACHE_KEY_SEED_1, CACHE_KEY_SEED_2, CACHE_KEY_SEED_3))
}

/// Construct a deterministic hasher using the fixed-seed [`RandomState`].
#[inline]
fn seeded_hasher() -> impl Hasher {
    cache_random_state().build_hasher()
}

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
#[cfg_attr(alef, alef(skip))]
pub trait CacheKeyStrategy: Send + Sync + 'static {
    /// Derive a `u64` hash key and a canonical serialized request body for
    /// collision-guard comparison.
    ///
    /// Determinism is required: identical inputs must produce identical outputs
    /// across calls **and** across process restarts.  Built-in implementations
    /// satisfy this by using a fixed-seed ahash [`RandomState`] (see
    /// module-level docs).
    fn key_for(&self, input: &CacheKeyInput<'_>) -> (u64, String);
}

/// Hash the entire serialized request (model + messages + params).
///
/// This is the default behavior: two requests are cache-equivalent only when
/// their full serialization is byte-identical.  Tenant and system-prompt fields
/// are included verbatim.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct ExactHashStrategy;

impl CacheKeyStrategy for ExactHashStrategy {
    fn key_for(&self, input: &CacheKeyInput<'_>) -> (u64, String) {
        // ~keep `|` cannot appear unescaped in JSON strings, so it safely delimits canonical fields.
        let body = format!(
            "{}|{}|{}|{}|{}",
            input.model,
            input.messages_json,
            input.params_json,
            input.tenant_id.unwrap_or(""),
            input.system_prompt.unwrap_or(""),
        );
        let mut hasher = seeded_hasher();
        body.hash(&mut hasher);
        (hasher.finish(), body)
    }
}

/// Hash includes the resolved system prompt but ignores per-call metadata
/// noise (tenant ID).
///
/// Useful when a resolved system prompt should be part of cache identity but
/// the caller wants tenant isolation to be handled at a higher layer, or when
/// per-call metadata such as a request trace ID would otherwise fragment the
/// cache unnecessarily.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct SystemPromptAwareStrategy;

impl CacheKeyStrategy for SystemPromptAwareStrategy {
    fn key_for(&self, input: &CacheKeyInput<'_>) -> (u64, String) {
        // ~keep Omit tenant_id here so tenant isolation can be handled by a higher layer.
        let body = format!(
            "{}|{}|{}|{}",
            input.model,
            input.messages_json,
            input.params_json,
            input.system_prompt.unwrap_or(""),
        );
        let mut hasher = seeded_hasher();
        body.hash(&mut hasher);
        (hasher.finish(), body)
    }
}

/// Adds a tenant prefix so two tenants requesting the same prompt receive
/// separate cache entries.
///
/// When no `tenant_id` is present in the input, this strategy behaves
/// identically to [`ExactHashStrategy`].
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct TenantScopedStrategy;

impl CacheKeyStrategy for TenantScopedStrategy {
    fn key_for(&self, input: &CacheKeyInput<'_>) -> (u64, String) {
        // ~keep Tenant prefix comes first so identical request bodies differ across tenants.
        let body = format!(
            "tenant:{}|{}|{}|{}|{}",
            input.tenant_id.unwrap_or("__global__"),
            input.model,
            input.messages_json,
            input.params_json,
            input.system_prompt.unwrap_or(""),
        );
        let mut hasher = seeded_hasher();
        body.hash(&mut hasher);
        (hasher.finish(), body)
    }
}

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

    // Run only in a fresh process so the process-global state is initialized anew.
    #[test]
    #[ignore]
    fn cache_keys_child_process() {
        let i = input(
            "gpt-4",
            r#"[{"role":"user","content":"hi"}]"#,
            "{}",
            Some("acme"),
            Some("help"),
        );
        for strategy in [
            &ExactHashStrategy as &dyn CacheKeyStrategy,
            &SystemPromptAwareStrategy,
            &TenantScopedStrategy,
        ] {
            println!("CACHE_KEY={}", strategy.key_for(&i).0);
        }
    }

    #[test]
    fn cache_keys_are_stable_across_processes() {
        let keys = || {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "cache_key::tests::cache_keys_child_process",
                    "--ignored",
                    "--nocapture",
                ])
                .output()
                .unwrap();
            assert!(output.status.success(), "child process failed: {output:?}");
            let stdout = String::from_utf8(output.stdout).unwrap();
            let keys: Vec<String> = stdout
                .lines()
                .filter(|line| line.starts_with("CACHE_KEY="))
                .map(str::to_owned)
                .collect();
            assert_eq!(keys.len(), 3, "all strategies must be exercised: {stdout}");
            keys
        };
        assert_eq!(keys(), keys(), "cache keys must survive process restarts");
    }

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
        let i1 = input("gpt-4", msgs, "{}", None, None);
        let i2 = input("gpt-4", msgs, "{}", None, None);
        let (k1, _) = s.key_for(&i1);
        let (k2, _) = s.key_for(&i2);
        assert_eq!(
            k1, k2,
            "two requests without tenant_id should share a key under tenant-scoped strategy"
        );
    }

    #[test]
    fn cache_key_deterministic_across_invocations() {
        let reference_input = input(
            "openai/gpt-4o",
            r#"[{"role":"user","content":"hello world"}]"#,
            r#"{"temperature":0.7}"#,
            Some("tenant-x"),
            Some("You are helpful."),
        );

        let (expected_key, expected_body) = ExactHashStrategy.key_for(&reference_input);

        for _ in 0..10 {
            let s = ExactHashStrategy;
            let (k, b) = s.key_for(&reference_input);
            assert_eq!(k, expected_key, "key must be stable across ExactHashStrategy instances");
            assert_eq!(
                b, expected_body,
                "body must be stable across ExactHashStrategy instances"
            );
        }
    }

    #[test]
    fn strategies_can_produce_different_keys_for_same_input() {
        let exact = ExactHashStrategy;
        let tenant = TenantScopedStrategy;
        let msgs = r#"[{"role":"user","content":"hi"}]"#;
        let i = input("gpt-4", msgs, "{}", Some("acme"), None);
        let (ke, _) = exact.key_for(&i);
        let (kt, _) = tenant.key_for(&i);
        let (_, be) = exact.key_for(&i);
        let (_, bt) = tenant.key_for(&i);
        assert_ne!(be, bt, "bodies produced by different strategies must differ");
        let _ = (ke, kt);
    }
}
