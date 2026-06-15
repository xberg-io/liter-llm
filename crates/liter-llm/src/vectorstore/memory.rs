//! In-memory vector store backed by a [`DashMap`].
//!
//! Suitable for ≤ 10 k entries.  Uses brute-force cosine similarity — O(n)
//! per query.  For larger corpora use [`OpenDalVectorStore`] or a dedicated
//! vector database.

use std::future::Future;
use std::pin::Pin;

use dashmap::DashMap;

use super::{VectorMatch, VectorMetadata, VectorStore};
use crate::error::{LiterLlmError, Result};

// ── Cosine similarity helper ──────────────────────────────────────────────────

/// Compute the cosine similarity between two vectors of equal length.
///
/// Returns `0.0` when either vector is the zero vector (to avoid division by
/// zero) or when the lengths differ.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

// ── Entry type ────────────────────────────────────────────────────────────────

struct Entry {
    vec: Vec<f32>,
    metadata: VectorMetadata,
}

// ── InMemoryVectorStore ───────────────────────────────────────────────────────

/// In-memory vector store using brute-force cosine similarity.
///
/// All operations are O(n) over the number of stored vectors.  The store is
/// thread-safe and can be shared across tasks via `Arc`.
///
/// # Example
///
/// ```rust,ignore
/// use liter_llm::vectorstore::{InMemoryVectorStore, VectorMetadata, VectorStore};
/// use std::collections::HashMap;
/// use std::sync::Arc;
/// use std::time::SystemTime;
///
/// let store = Arc::new(InMemoryVectorStore::new(3));
/// store.upsert(
///     "entry-1".into(),
///     vec![1.0, 0.0, 0.0],
///     VectorMetadata {
///         cache_key: 42,
///         tenant_id: None,
///         inserted_at: SystemTime::now(),
///         extra: HashMap::new(),
///     },
/// );
/// ```
pub struct InMemoryVectorStore {
    entries: DashMap<String, Entry>,
    dim: usize,
}

impl InMemoryVectorStore {
    /// Create a new in-memory vector store expecting vectors of dimension `dim`.
    #[must_use]
    pub fn new(dim: usize) -> Self {
        Self {
            entries: DashMap::new(),
            dim,
        }
    }
}

impl VectorStore for InMemoryVectorStore {
    fn search<'a>(
        &'a self,
        query_vec: &'a [f32],
        k: usize,
        threshold: f32,
    ) -> Pin<Box<dyn Future<Output = Vec<VectorMatch>> + Send + 'a>> {
        // Collect and sort synchronously, wrap in a ready future.
        let mut matches: Vec<VectorMatch> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let sim = cosine_similarity(query_vec, &entry.vec);
                if sim >= threshold {
                    Some(VectorMatch {
                        id: entry.key().clone(),
                        similarity: sim,
                        metadata: entry.metadata.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity descending, then truncate to k.
        matches.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches.truncate(k);

        Box::pin(std::future::ready(matches))
    }

    fn upsert<'a>(
        &'a self,
        id: String,
        vec: Vec<f32>,
        metadata: VectorMetadata,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        if vec.len() != self.dim {
            return Box::pin(std::future::ready(Err(LiterLlmError::InternalError {
                message: format!(
                    "vector dimension mismatch: store expects {} but received {}",
                    self.dim,
                    vec.len()
                ),
            })));
        }
        self.entries.insert(id, Entry { vec, metadata });
        Box::pin(std::future::ready(Ok(())))
    }

    fn delete<'a>(&'a self, id: &'a str) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        self.entries.remove(id);
        Box::pin(std::future::ready(Ok(())))
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::SystemTime;

    use super::*;

    fn meta(cache_key: u64) -> VectorMetadata {
        VectorMetadata {
            cache_key,
            tenant_id: None,
            inserted_at: SystemTime::now(),
            extra: HashMap::new(),
        }
    }

    #[test]
    fn cosine_similarity_identical_vectors() {
        let v = vec![1.0_f32, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0_f32, 0.0, 0.0];
        let b = vec![0.0_f32, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_zero_vector_returns_zero() {
        let a = vec![0.0_f32, 0.0, 0.0];
        let b = vec![1.0_f32, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn cosine_similarity_length_mismatch_returns_zero() {
        let a = vec![1.0_f32, 0.0];
        let b = vec![1.0_f32, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[tokio::test]
    async fn upsert_and_search_returns_match_above_threshold() {
        let store = InMemoryVectorStore::new(3);
        store.upsert("v1".into(), vec![1.0, 0.0, 0.0], meta(42)).await.unwrap();

        let results = store.search(&[1.0, 0.0, 0.0], 5, 0.99).await;
        assert_eq!(results.len(), 1, "should find the identical vector");
        assert_eq!(results[0].id, "v1");
        assert!((results[0].similarity - 1.0).abs() < 1e-5);
        assert_eq!(results[0].metadata.cache_key, 42);
    }

    #[tokio::test]
    async fn search_filters_below_threshold() {
        let store = InMemoryVectorStore::new(3);
        store.upsert("v1".into(), vec![1.0, 0.0, 0.0], meta(1)).await.unwrap();
        store.upsert("v2".into(), vec![0.0, 1.0, 0.0], meta(2)).await.unwrap();

        // Query close to v1; v2 is orthogonal (similarity = 0) — below threshold 0.9.
        let results = store.search(&[1.0, 0.0, 0.0], 5, 0.9).await;
        assert_eq!(results.len(), 1, "orthogonal vector should be filtered out");
        assert_eq!(results[0].id, "v1");
    }

    #[tokio::test]
    async fn search_returns_k_nearest() {
        let store = InMemoryVectorStore::new(2);
        store.upsert("a".into(), vec![1.0, 0.0], meta(1)).await.unwrap();
        store.upsert("b".into(), vec![0.9, 0.1], meta(2)).await.unwrap();
        store.upsert("c".into(), vec![0.8, 0.2], meta(3)).await.unwrap();

        let results = store.search(&[1.0, 0.0], 2, 0.0).await;
        assert_eq!(results.len(), 2, "should return exactly k results");
        // Results should be sorted by similarity descending.
        assert!(results[0].similarity >= results[1].similarity);
    }

    #[tokio::test]
    async fn search_returns_empty_when_store_is_empty() {
        let store = InMemoryVectorStore::new(3);
        let results = store.search(&[1.0, 0.0, 0.0], 5, 0.0).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn delete_removes_entry() {
        let store = InMemoryVectorStore::new(3);
        store.upsert("v1".into(), vec![1.0, 0.0, 0.0], meta(1)).await.unwrap();
        store.delete("v1").await.unwrap();
        let results = store.search(&[1.0, 0.0, 0.0], 5, 0.0).await;
        assert!(results.is_empty(), "deleted entry must not appear in search results");
    }

    #[tokio::test]
    async fn delete_nonexistent_is_noop() {
        let store = InMemoryVectorStore::new(3);
        let result = store.delete("does-not-exist").await;
        assert!(result.is_ok(), "deleting a non-existent entry should not error");
    }

    #[tokio::test]
    async fn upsert_replaces_existing_entry() {
        let store = InMemoryVectorStore::new(3);
        store.upsert("v1".into(), vec![1.0, 0.0, 0.0], meta(1)).await.unwrap();
        store.upsert("v1".into(), vec![0.0, 1.0, 0.0], meta(99)).await.unwrap();

        // Search for the new vector — should find it.
        let results = store.search(&[0.0, 1.0, 0.0], 5, 0.99).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.cache_key, 99, "upsert should replace metadata");
    }

    #[tokio::test]
    async fn upsert_dimension_mismatch_returns_error() {
        let store = InMemoryVectorStore::new(3);
        let result = store.upsert("bad".into(), vec![1.0, 0.0], meta(1)).await;
        assert!(result.is_err(), "dimension mismatch must return an error");
    }

    #[test]
    fn dim_returns_configured_dimension() {
        let store = InMemoryVectorStore::new(512);
        assert_eq!(store.dim(), 512);
    }
}
