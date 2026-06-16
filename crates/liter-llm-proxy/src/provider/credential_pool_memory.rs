//! In-memory [`CredentialPool`] implementation.
//!
//! [`InMemoryCredentialPool`] stores credentials in a `DashMap` keyed by
//! provider name.  Each provider's credentials are kept in a `Vec` behind a
//! `Mutex`.  The active index advances on every `mark_exhausted` call so that
//! the next `current` call returns the subsequent credential
//! (round-robin rotation).
//!
//! # Recovery
//!
//! `mark_exhausted` spawns a lightweight Tokio task that sleeps for
//! `cooldown` and then reactivates the credential.  No external scheduler is
//! required.
//!
//! # OTel metrics
//!
//! Rotation / exhaustion / active-count / recovery events are recorded via the
//! `gen_ai.credential.*` instruments when the `otel` feature is active.

use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use dashmap::DashMap;
use secrecy::SecretString;
use tokio::time::Instant;

use super::credential_pool::{CredentialError, CredentialHandle, CredentialPool, PoolSnapshot};
use super::metrics as cred_metrics;

// ─── CredentialState ──────────────────────────────────────────────────────────

/// Internal lifecycle state of a single credential entry.
#[derive(Debug, Clone)]
pub(super) enum CredentialState {
    /// The credential is available for use.
    Active,
    /// The credential is parked until `until` has elapsed.
    Exhausted {
        /// Monotonic instant at which this credential becomes available again.
        until: Instant,
    },
}

// ─── CredentialEntry ─────────────────────────────────────────────────────────

/// Internal pool entry pairing a handle with its lifecycle state.
#[derive(Debug, Clone)]
pub(super) struct CredentialEntry {
    pub(super) handle: CredentialHandle,
    pub(super) state: CredentialState,
}

// ─── ProviderBucket ──────────────────────────────────────────────────────────

/// Per-provider credential collection with an active-index cursor.
struct ProviderBucket {
    /// Ordered list of credentials for this provider.
    entries: Vec<CredentialEntry>,
    /// Index of the last-returned credential.  `current()` starts scanning
    /// from `(cursor + 1) % len` and returns the first `Active` entry it finds.
    cursor: usize,
}

impl ProviderBucket {
    fn new(entries: Vec<CredentialEntry>) -> Self {
        Self { entries, cursor: 0 }
    }

    /// Return the first `Active` credential, advancing the cursor past the
    /// previously returned position.  Returns `None` if all credentials are
    /// exhausted.
    fn next_active(&mut self) -> Option<&CredentialHandle> {
        let len = self.entries.len();
        if len == 0 {
            return None;
        }

        // Scan all positions starting at the current cursor (inclusive).
        // The cursor points to the most-recently returned entry; we honour it
        // as-is on the first call (cursor = 0), and on subsequent calls we
        // cycle through from where we left off.
        for offset in 0..len {
            let idx = (self.cursor + offset) % len;
            if matches!(self.entries[idx].state, CredentialState::Active) {
                self.cursor = idx;
                return Some(&self.entries[idx].handle);
            }
        }
        None
    }

    /// Mark the credential with `id` as exhausted.  Returns the index that was
    /// updated, or `None` if the id was not found.
    fn mark_exhausted(&mut self, id: &str, until: Instant) -> Option<usize> {
        for (idx, entry) in self.entries.iter_mut().enumerate() {
            if entry.handle.id == id {
                entry.state = CredentialState::Exhausted { until };
                // Advance the cursor past the exhausted entry so the next
                // `next_active()` skips it automatically.
                let len = self.entries.len();
                self.cursor = (idx + 1) % len;
                return Some(idx);
            }
        }
        None
    }

    /// Reactivate the credential with `id`.
    fn reactivate(&mut self, id: &str) {
        for entry in self.entries.iter_mut() {
            if entry.handle.id == id {
                entry.state = CredentialState::Active;
                return;
            }
        }
    }

    /// Count active and exhausted credentials, and find the earliest recovery.
    fn stats(&self) -> (usize, usize, Option<Instant>) {
        let mut active = 0usize;
        let mut exhausted = 0usize;
        let mut earliest: Option<Instant> = None;
        for entry in &self.entries {
            match entry.state {
                CredentialState::Active => active += 1,
                CredentialState::Exhausted { until } => {
                    exhausted += 1;
                    earliest = Some(earliest.map_or(until, |e: Instant| e.min(until)));
                }
            }
        }
        (active, exhausted, earliest)
    }
}

// ─── InMemoryCredentialPool ───────────────────────────────────────────────────

/// Default credential pool backed by an in-memory `DashMap`.
///
/// Thread-safe via `DashMap` (shard-level locking) + `Mutex<ProviderBucket>`
/// for per-provider mutation.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use liter_llm_proxy::provider::credential_pool_memory::InMemoryCredentialPool;
///
/// let pool = Arc::new(InMemoryCredentialPool::new());
/// pool.add_credential("openai", "key-1", "sk-abc", None);
/// pool.add_credential("openai", "key-2", "sk-def", Some(vec!["gpt-4o".into()]));
/// ```
pub struct InMemoryCredentialPool {
    /// provider_name -> ProviderBucket (wrapped in Arc+Mutex for shared mutation)
    buckets: DashMap<String, Arc<Mutex<ProviderBucket>>>,
}

impl InMemoryCredentialPool {
    /// Create an empty pool.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buckets: DashMap::new(),
        }
    }

    /// Add a credential for `provider`.
    ///
    /// - `id` — opaque identifier for this credential (used in rotation bookkeeping).
    /// - `api_key` — raw API key string; stored behind `secrecy::SecretString`.
    /// - `model_allowlist` — optional set of model names this credential may serve.
    pub fn add_credential(
        &self,
        provider: &str,
        id: impl Into<String>,
        api_key: impl Into<String>,
        model_allowlist: Option<Vec<String>>,
    ) {
        let entry = CredentialEntry {
            handle: CredentialHandle {
                id: id.into(),
                api_key: SecretString::from(api_key.into()),
                model_allowlist,
            },
            state: CredentialState::Active,
        };

        let bucket = self
            .buckets
            .entry(provider.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(ProviderBucket::new(vec![]))));

        bucket
            .lock()
            .expect("provider bucket mutex poisoned")
            .entries
            .push(entry);
    }

    /// Internal: obtain a cloned `Arc` to the provider's bucket, or `None`.
    fn bucket(&self, provider: &str) -> Option<Arc<Mutex<ProviderBucket>>> {
        self.buckets.get(provider).map(|r| Arc::clone(r.value()))
    }
}

impl Default for InMemoryCredentialPool {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialPool for InMemoryCredentialPool {
    fn current<'a>(
        &'a self,
        provider: &'a str,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<CredentialHandle, CredentialError>> + Send + 'a>> {
        Box::pin(async move {
            let Some(bucket_arc) = self.bucket(provider) else {
                return Err(CredentialError::PoolEmpty);
            };

            let mut bucket = bucket_arc.lock().expect("provider bucket mutex poisoned");
            if bucket.entries.is_empty() {
                return Err(CredentialError::PoolEmpty);
            }

            // Re-check Exhausted entries to see if any has recovered by now.
            // This handles the case where the Tokio recovery task hasn't fired
            // yet but the cooldown has elapsed on the wall clock.
            let now = Instant::now();
            for entry in bucket.entries.iter_mut() {
                if let CredentialState::Exhausted { until } = entry.state
                    && now >= until
                {
                    entry.state = CredentialState::Active;
                }
            }

            match bucket.next_active() {
                Some(handle) => Ok(handle.clone()),
                None => Err(CredentialError::AllExhausted),
            }
        })
    }

    fn mark_exhausted<'a>(
        &'a self,
        provider: &'a str,
        handle: &'a CredentialHandle,
        cooldown: Duration,
    ) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let Some(bucket_arc) = self.bucket(provider) else {
                return;
            };

            let until = Instant::now() + cooldown;
            let id = handle.id.clone();
            let provider_owned = provider.to_owned();

            {
                let mut bucket = bucket_arc.lock().expect("provider bucket mutex poisoned");
                if bucket.mark_exhausted(&id, until).is_none() {
                    // Credential not found in this pool — no-op.
                    return;
                }

                cred_metrics::record_credential_exhausted(&provider_owned);

                let (active, _, _) = bucket.stats();
                cred_metrics::record_credential_pool_active(&provider_owned, active);
            }

            cred_metrics::record_credential_rotation(&provider_owned);

            // Spawn a recovery task.  The task holds an `Arc` clone of the
            // bucket so it does not keep `&self` alive across the await point.
            let bucket_arc_clone = Arc::clone(&bucket_arc);
            let id_clone = id.clone();
            let provider_clone = provider_owned.clone();

            tokio::spawn(async move {
                tokio::time::sleep(cooldown).await;

                let mut bucket = bucket_arc_clone.lock().expect("provider bucket mutex poisoned");
                bucket.reactivate(&id_clone);

                cred_metrics::record_credential_recovery(&provider_clone);

                let (active, _, _) = bucket.stats();
                cred_metrics::record_credential_pool_active(&provider_clone, active);
            });
        })
    }

    fn snapshot(&self, provider: &str) -> PoolSnapshot {
        let Some(bucket_arc) = self.bucket(provider) else {
            return PoolSnapshot {
                total: 0,
                active: 0,
                exhausted: 0,
                next_recovery: None,
            };
        };

        let bucket = bucket_arc.lock().expect("provider bucket mutex poisoned");
        let total = bucket.entries.len();
        let (active, exhausted, earliest_instant) = bucket.stats();

        // Convert monotonic `Instant` to `SystemTime` for the public API.
        let next_recovery = earliest_instant.map(|instant| {
            let remaining = instant.saturating_duration_since(Instant::now());
            SystemTime::now() + remaining
        });

        PoolSnapshot {
            total,
            active,
            exhausted,
            next_recovery,
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;
    use crate::provider::credential_pool::CredentialError;

    fn pool_with(provider: &str, keys: &[(&str, Option<Vec<String>>)]) -> Arc<InMemoryCredentialPool> {
        let p = Arc::new(InMemoryCredentialPool::new());
        for (i, (key, allowlist)) in keys.iter().enumerate() {
            p.add_credential(provider, format!("key-{i}"), *key, allowlist.clone());
        }
        p
    }

    // ── 1 ────────────────────────────────────────────────────────────────────
    /// current() returns the first active credential when all three are active.
    #[tokio::test]
    async fn credential_pool_current_returns_first_active() {
        let pool = pool_with("openai", &[("sk-a", None), ("sk-b", None), ("sk-c", None)]);

        let handle = pool.current("openai").await.expect("should return a handle");
        // First active credential has id "key-0".
        assert_eq!(handle.id, "key-0");
    }

    // ── 2 ────────────────────────────────────────────────────────────────────
    /// After exhausting the first credential, the next current() returns the second.
    #[tokio::test]
    async fn credential_pool_mark_exhausted_advances_round_robin() {
        let pool = pool_with("openai", &[("sk-a", None), ("sk-b", None), ("sk-c", None)]);

        let first = pool.current("openai").await.expect("first credential");
        assert_eq!(first.id, "key-0");

        pool.mark_exhausted("openai", &first, Duration::from_secs(60)).await;

        let second = pool.current("openai").await.expect("second credential");
        assert_eq!(second.id, "key-1");
    }

    // ── 3 ────────────────────────────────────────────────────────────────────
    /// When all credentials are exhausted, current() returns AllExhausted.
    #[tokio::test]
    async fn credential_pool_all_exhausted_returns_error() {
        let pool = pool_with("openai", &[("sk-a", None), ("sk-b", None)]);

        let h0 = pool.current("openai").await.expect("first");
        pool.mark_exhausted("openai", &h0, Duration::from_secs(60)).await;

        let h1 = pool.current("openai").await.expect("second");
        pool.mark_exhausted("openai", &h1, Duration::from_secs(60)).await;

        let err = pool.current("openai").await.expect_err("should be exhausted");
        assert!(
            matches!(err, CredentialError::AllExhausted),
            "expected AllExhausted, got {err:?}"
        );
    }

    // ── 4 ────────────────────────────────────────────────────────────────────
    /// After the cooldown elapses, current() returns the reactivated credential.
    ///
    /// The inline wall-clock re-check inside `current()` handles the case where
    /// the recovery task hasn't fired yet but the cooldown has elapsed — no
    /// explicit yielding to the spawned task is required.
    #[tokio::test(start_paused = true)]
    async fn credential_pool_recovery_reactivates_after_cooldown() {
        let pool = pool_with("openai", &[("sk-only", None)]);

        let h = pool.current("openai").await.expect("initial");
        pool.mark_exhausted("openai", &h, Duration::from_millis(100)).await;

        // Credential is exhausted immediately.
        let err = pool.current("openai").await.expect_err("should be exhausted");
        assert!(matches!(err, CredentialError::AllExhausted));

        // Advance the virtual clock past the cooldown.  The inline `Instant::now()`
        // check inside `current()` will see the advanced time and treat the
        // credential as recovered without needing the spawned task to fire.
        tokio::time::advance(Duration::from_millis(150)).await;

        let recovered = pool.current("openai").await.expect("should recover");
        assert_eq!(recovered.id, "key-0");
    }

    // ── 5 ────────────────────────────────────────────────────────────────────
    /// When credential A only allows "gpt-4o" and credential B allows "claude-3",
    /// current() still returns A (the first active one regardless of model).
    /// The model-allowlist filtering happens at the auto-cycle layer, not in the pool.
    /// This test verifies the allowlist data is preserved through the pool.
    #[tokio::test]
    async fn credential_pool_model_allowlist_preserved() {
        let pool = pool_with(
            "multi",
            &[
                ("sk-a", Some(vec!["gpt-4o".to_owned()])),
                ("sk-b", Some(vec!["claude-3".to_owned()])),
            ],
        );

        let h = pool.current("multi").await.expect("first");
        assert_eq!(h.id, "key-0");
        assert_eq!(h.model_allowlist, Some(vec!["gpt-4o".to_owned()]));
    }

    // ── snapshot ──────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn snapshot_counts_are_accurate() {
        let pool = pool_with("openai", &[("sk-a", None), ("sk-b", None), ("sk-c", None)]);

        let snap = pool.snapshot("openai");
        assert_eq!(snap.total, 3);
        assert_eq!(snap.active, 3);
        assert_eq!(snap.exhausted, 0);

        let h0 = pool.current("openai").await.expect("first");
        pool.mark_exhausted("openai", &h0, Duration::from_secs(60)).await;

        let snap2 = pool.snapshot("openai");
        assert_eq!(snap2.total, 3);
        assert_eq!(snap2.active, 2);
        assert_eq!(snap2.exhausted, 1);
        assert!(snap2.next_recovery.is_some());
    }

    #[tokio::test]
    async fn empty_provider_returns_pool_empty() {
        let pool = Arc::new(InMemoryCredentialPool::new());
        let err = pool.current("nonexistent").await.expect_err("should be empty");
        assert!(matches!(err, CredentialError::PoolEmpty));
    }
}
