use serde::{Deserialize, Serialize};

/// Request to rerank documents by relevance to a query.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RerankRequest {
    /// Model ID (e.g., `"cohere/rerank-english-v3.0"`).
    pub model: String,
    /// The search query.
    pub query: String,
    /// Documents to rerank.
    pub documents: Vec<RerankDocument>,
    /// Return only the top N results. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_n: Option<u32>,
    /// Include the document content in results. Defaults to false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_documents: Option<bool>,
}

/// A document to be reranked — either a plain string or an object with a text field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RerankDocument {
    /// Plain text document content.
    Text(String),
    /// Document with explicit text field (may include metadata).
    Object { text: String },
}

#[cfg_attr(alef, alef(skip))]
impl Default for RerankDocument {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

/// Response from the rerank endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RerankResponse {
    /// Unique identifier for this rerank request.
    pub id: Option<String>,
    /// Reranked documents in order of relevance.
    pub results: Vec<RerankResult>,
    /// Optional metadata about the reranking operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// A single reranked document with its relevance score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RerankResult {
    /// Original document index in the input list.
    pub index: u32,
    /// Relevance score in `[0, 1]`. Higher indicates more relevant.
    pub relevance_score: f64,
    /// Original document content (if `return_documents` was true).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<RerankResultDocument>,
}

/// The text content of a reranked document, returned when `return_documents` is true.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RerankResultDocument {
    /// Document text.
    pub text: String,
}
