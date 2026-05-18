use serde::{Deserialize, Serialize};

use super::common::Usage;
use crate::cost;

// ─── Encoding format ──────────────────────────────────────────────────────────

/// The format in which the embedding vectors are returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingFormat {
    /// 32-bit floating-point numbers (default).
    Float,
    /// Base64-encoded string representation of the floats.
    Base64,
}

// ─── Request ──────────────────────────────────────────────────────────────────

/// Embedding request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingRequest {
    /// Model ID (e.g., `"text-embedding-3-small"`).
    pub model: String,
    /// Text or texts to embed.
    pub input: EmbeddingInput,
    /// Output format: float (native) or base64.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<EmbeddingFormat>,
    /// Requested embedding dimensions (if supported by the model).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    /// User identifier for request tracking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Text or texts to embed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    /// Single text string.
    Single(String),
    /// Multiple text strings (batch embedding).
    Multiple(Vec<String>),
}

#[cfg_attr(alef, alef(skip))]
impl Default for EmbeddingInput {
    fn default() -> Self {
        Self::Single(String::new())
    }
}

// ─── Response ─────────────────────────────────────────────────────────────────

/// Embedding response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// Always `"list"` from OpenAI-compatible APIs.  Stored as a plain
    /// `String` so non-standard provider values do not break deserialization.
    pub object: String,
    /// List of embeddings.
    pub data: Vec<EmbeddingObject>,
    /// Model used to generate embeddings.
    pub model: String,
    /// Token usage (input tokens only; embeddings have zero output tokens).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

impl EmbeddingResponse {
    /// Estimate the cost of this embedding request based on embedded pricing data.
    ///
    /// Returns `None` if:
    /// - the `model` field is not present in the embedded pricing registry, or
    /// - the `usage` field is absent from the response.
    ///
    /// Embedding models only charge for input tokens; output cost is zero.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let cost = response.estimated_cost();
    /// if let Some(usd) = cost {
    ///     println!("Embedding cost: ${usd:.8}");
    /// }
    /// ```
    #[cfg_attr(alef, alef(skip))]
    #[must_use]
    pub fn estimated_cost(&self) -> Option<f64> {
        let usage = self.usage.as_ref()?;
        cost::completion_cost(&self.model, usage.prompt_tokens, usage.completion_tokens)
    }
}

/// A single embedding vector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingObject {
    /// Always `"embedding"` from OpenAI-compatible APIs.  Stored as a plain
    /// `String` so non-standard provider values do not break deserialization.
    pub object: String,
    /// The embedding vector.
    pub embedding: Vec<f64>,
    /// Index in the batch (corresponds to input order).
    pub index: u32,
}
