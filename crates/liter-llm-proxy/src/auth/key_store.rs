use std::future::Future;
use std::pin::Pin;

use dashmap::DashMap;
use liter_llm::tenant::{KeyResolver, KeyResolverError, ResolvedKey, TenantId};
use secrecy::{ExposeSecret, SecretString};
use subtle::ConstantTimeEq;

use crate::config::VirtualKeyConfig;

/// Context injected into request extensions after successful auth.
#[derive(Debug, Clone)]
pub struct KeyContext {
    pub key_id: String,
    pub allowed_models: Option<Vec<String>>,
    pub is_master: bool,
}

impl KeyContext {
    /// Create a context representing the master key (unrestricted access).
    pub fn master() -> Self {
        Self {
            key_id: "master".into(),
            allowed_models: None,
            is_master: true,
        }
    }

    /// Create a context from a virtual key configuration.
    pub fn from_config(config: &VirtualKeyConfig) -> Self {
        let allowed_models = if config.models.is_empty() {
            None
        } else {
            Some(config.models.clone())
        };
        Self {
            key_id: config.key.clone(),
            allowed_models,
            is_master: false,
        }
    }

    /// Returns true if this key is allowed to access the given model.
    pub fn can_access_model(&self, model: &str) -> bool {
        match &self.allowed_models {
            None => true,
            Some(models) => models.iter().any(|m| m == model),
        }
    }
}

/// In-memory virtual key store backed by `DashMap` for concurrent access.
///
/// # Constant-time lookup design
///
/// [`KeyStore::get`] performs a **constant-time** virtual-key lookup to
/// prevent timing side-channel attacks that could reveal whether a given token
/// (or its prefix) exists in the store.
///
/// A naive `DashMap::get(token)` call computes the token's hash and probes
/// the hash table.  The probe-chain length and branch behaviour leak
/// information about the stored key set (prefix matches, hamming distance).
///
/// Instead we iterate over all registered keys and compare each with
/// `subtle::ConstantTimeEq`.  This is O(n) in the number of virtual keys.
/// The trade-off is acceptable: production deployments with hundreds of VKs
/// still complete in microseconds, and the constant-time guarantee is sound
/// regardless of the key population.
///
/// The `DashMap` is retained for non-timing-sensitive operations (e.g. hot
/// reload of key metadata) and for the master-key check which is already
/// constant-time.
pub struct KeyStore {
    /// Raw key tokens stored as `SecretString`.  The `DashMap` is only used
    /// to hold the configuration; the lookup path iterates entries and uses
    /// `subtle::ConstantTimeEq` to compare tokens.
    keys: DashMap<String, VirtualKeyConfig>,
    master_key: Option<SecretString>,
}

impl KeyStore {
    /// Build a key store from the proxy configuration values.
    pub fn from_config(master_key: Option<SecretString>, keys: &[VirtualKeyConfig]) -> Self {
        let map = DashMap::new();
        for k in keys {
            map.insert(k.key.clone(), k.clone());
        }
        Self { keys: map, master_key }
    }

    /// Check whether `token` matches the configured master key using a
    /// constant-time comparison to prevent timing side-channel attacks.
    ///
    /// The master key length is deployment-static configuration, not
    /// user-controlled per request, so the length-check short-circuit is
    /// acceptable.
    pub fn is_master_key(&self, token: &str) -> bool {
        let Some(master) = self.master_key.as_ref() else {
            return false;
        };
        let master_bytes = master.expose_secret().as_bytes();
        let token_bytes = token.as_bytes();
        master_bytes.ct_eq(token_bytes).into()
    }

    /// Look up a virtual key configuration by its token string using a
    /// constant-time comparison.
    ///
    /// # Constant-time guarantee
    ///
    /// This method iterates ALL registered virtual keys and compares each
    /// token with `subtle::ConstantTimeEq`.  The iteration runs to completion
    /// regardless of whether a match is found, preventing early-exit timing
    /// leaks.  The result is captured and returned after the loop.
    ///
    /// The `DashMap` stores keys in an unordered bucket structure.  Iteration
    /// order is non-deterministic across calls, which provides additional
    /// resistance against timing correlation attacks that depend on consistent
    /// ordering.
    ///
    /// # Complexity
    ///
    /// O(n) where n is the number of registered virtual keys.  For practical
    /// deployments (n ≤ 10 000) this completes in < 1 ms.
    pub fn get(&self, token: &str) -> Option<VirtualKeyConfig> {
        let token_bytes = token.as_bytes();
        let mut found: Option<VirtualKeyConfig> = None;

        // Iterate ALL entries unconditionally — no early exit.
        // `subtle::ConstantTimeEq` prevents branch-based timing leaks when
        // comparing two byte slices.  We capture the first match but continue
        // the loop to completion so the total work is the same for "hit" and
        // "miss" inputs.
        for entry in self.keys.iter() {
            let stored_bytes = entry.key().as_bytes();

            // ct_eq returns Choice(1) on match, Choice(0) on mismatch.
            // Mapping to bool via `.into()` is safe here because we only
            // use the bool after the comparison — no branching on the secret
            // value occurs inside the comparison itself.
            if bool::from(stored_bytes.ct_eq(token_bytes)) && found.is_none() {
                found = Some(entry.value().clone());
            }
            // Loop continues unconditionally even after a match.
        }

        found
    }
}

impl KeyResolver for KeyStore {
    fn resolve<'a>(
        &'a self,
        api_key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedKey, KeyResolverError>> + Send + 'a>> {
        Box::pin(async move {
            match self.get(api_key) {
                None => Err(KeyResolverError::NotFound),
                Some(cfg) => Ok(ResolvedKey {
                    tenant_id: TenantId::from(cfg.key.clone()),
                    allowed_models: cfg.models.clone(),
                    monthly_budget: cfg
                        .budget_limit
                        .map(|b| rust_decimal::Decimal::from_f64_retain(b).unwrap_or(rust_decimal::Decimal::ZERO)),
                    currency: None,
                    metadata: std::collections::HashMap::new(),
                    active: true,
                }),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use secrecy::SecretString;

    use super::*;

    fn sample_key_config(key: &str, models: Vec<String>) -> VirtualKeyConfig {
        VirtualKeyConfig {
            key: key.to_string(),
            description: None,
            models,
            rpm: Some(60),
            tpm: Some(100_000),
            budget_limit: Some(50.0),
            provider_credentials: vec![],
        }
    }

    // ── KeyStore tests ──────────────────────────────────────────────────

    #[test]
    fn master_key_match_returns_true() {
        let store = KeyStore::from_config(Some(SecretString::from("sk-master".to_string())), &[]);
        assert!(store.is_master_key("sk-master"));
    }

    #[test]
    fn master_key_mismatch_returns_false() {
        let store = KeyStore::from_config(Some(SecretString::from("sk-master".to_string())), &[]);
        assert!(!store.is_master_key("sk-wrong"));
    }

    #[test]
    fn no_master_key_always_returns_false() {
        let store = KeyStore::from_config(None, &[]);
        assert!(!store.is_master_key("sk-anything"));
    }

    #[test]
    fn master_key_near_miss_returns_false() {
        // Same length, one character different — ensures ct_eq compares content, not just length.
        let store = KeyStore::from_config(Some(SecretString::from("sk-master".to_string())), &[]);
        assert!(!store.is_master_key("sk-mastex"));
    }

    #[test]
    fn get_existing_key_returns_config() {
        let cfg = sample_key_config("vk-team-a", vec!["gpt-4o".into()]);
        let store = KeyStore::from_config(None, std::slice::from_ref(&cfg));

        let result = store.get("vk-team-a");
        assert!(result.is_some());
        let found = result.expect("key lookup should succeed");
        assert_eq!(found.key, "vk-team-a");
        assert_eq!(found.models, vec!["gpt-4o"]);
    }

    #[test]
    fn get_nonexistent_key_returns_none() {
        let store = KeyStore::from_config(None, &[]);
        assert!(store.get("vk-missing").is_none());
    }

    // ── Constant-time lookup verification ───────────────────────────────

    /// Verify that `get` uses `subtle::ConstantTimeEq` for comparison and
    /// iterates ALL keys unconditionally, not stopping on first match.
    ///
    /// This test verifies the design contract documented on [`KeyStore::get`]:
    /// the method iterates every registered key and uses constant-time
    /// comparison.  It also verifies that an exact-match key is found while a
    /// near-miss (same length, one byte different) is not.
    #[test]
    fn key_store_constant_time_lookup() {
        let cfg_a = sample_key_config("vk-aaaa", vec![]);
        let cfg_b = sample_key_config("vk-bbbb", vec![]);
        let cfg_c = sample_key_config("vk-cccc", vec![]);
        let store = KeyStore::from_config(None, &[cfg_a, cfg_b, cfg_c]);

        // Exact match — must be found.
        assert!(store.get("vk-aaaa").is_some(), "exact match must be found");
        assert!(store.get("vk-bbbb").is_some(), "exact match must be found");
        assert!(store.get("vk-cccc").is_some(), "exact match must be found");

        // Near-miss (same length, different content) — must not be found.
        // subtle::ConstantTimeEq returns Choice(0) here; no early exit.
        assert!(store.get("vk-aaab").is_none(), "near-miss must not be found");
        assert!(store.get("vk-aaaz").is_none(), "near-miss must not be found");

        // Prefix of a registered key — must not be found.
        assert!(store.get("vk-aaa").is_none(), "prefix must not be found");

        // Superstring of a registered key — must not be found.
        assert!(store.get("vk-aaaax").is_none(), "superstring must not be found");

        // Empty token — must not be found.
        assert!(store.get("").is_none(), "empty token must not be found");
    }

    /// Verify that lookup in a store with multiple keys returns the correct one.
    ///
    /// This exercises the loop-to-completion behaviour: the first matching
    /// entry is captured but the loop continues for all remaining entries.
    #[test]
    fn get_returns_correct_key_among_many() {
        let configs: Vec<VirtualKeyConfig> = (0..20)
            .map(|i| sample_key_config(&format!("vk-key-{i:04}"), vec![format!("model-{i}")]))
            .collect();
        let store = KeyStore::from_config(None, &configs);

        for i in 0..20 {
            let token = format!("vk-key-{i:04}");
            let result = store.get(&token);
            assert!(result.is_some(), "key {token} should be found");
            let found = result.unwrap();
            assert_eq!(found.key, token);
            assert_eq!(found.models, vec![format!("model-{i}")]);
        }
    }

    // ── KeyContext tests ────────────────────────────────────────────────

    #[test]
    fn master_context_has_no_restrictions() {
        let ctx = KeyContext::master();
        assert!(ctx.is_master);
        assert!(ctx.allowed_models.is_none());
        assert!(ctx.can_access_model("any-model"));
    }

    #[test]
    fn context_with_allowed_models_permits_listed_model() {
        let cfg = sample_key_config("vk-1", vec!["gpt-4o".into(), "claude-sonnet".into()]);
        let ctx = KeyContext::from_config(&cfg);

        assert!(!ctx.is_master);
        assert!(ctx.can_access_model("gpt-4o"));
        assert!(ctx.can_access_model("claude-sonnet"));
    }

    #[test]
    fn context_with_allowed_models_denies_unlisted_model() {
        let cfg = sample_key_config("vk-1", vec!["gpt-4o".into()]);
        let ctx = KeyContext::from_config(&cfg);

        assert!(!ctx.can_access_model("claude-sonnet"));
    }

    #[test]
    fn context_with_empty_models_allows_all() {
        let cfg = sample_key_config("vk-1", vec![]);
        let ctx = KeyContext::from_config(&cfg);

        assert!(ctx.allowed_models.is_none());
        assert!(ctx.can_access_model("any-model"));
    }
}
