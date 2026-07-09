use serde::{Deserialize, Serialize};

/// Response listing available models.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelsListResponse {
    /// Always `"list"` from OpenAI-compatible APIs.  Stored as a plain
    /// `String` so non-standard provider values do not break deserialization.
    pub object: String,
    /// List of available models.
    pub data: Vec<ModelObject>,
}

/// A model available from the API.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelObject {
    /// Model ID (e.g., `"gpt-4o"`, `"claude-3-5-sonnet"`).
    pub id: String,
    /// Always `"model"` from OpenAI-compatible APIs.  Stored as a plain
    /// `String` so non-standard provider values do not break deserialization.
    pub object: String,
    /// Unix timestamp of model creation (or release date).
    pub created: u64,
    /// Organization or entity that owns the model.
    pub owned_by: String,
}
