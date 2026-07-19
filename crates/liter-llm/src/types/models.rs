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
    /// Defaults to empty when a provider omits the field.
    #[serde(default)]
    pub object: String,
    /// Unix timestamp of model creation (or release date).
    ///
    /// Defaults to `0` when a provider omits it — DeepSeek and some other
    /// OpenAI-compatible providers do not return `created` from `/v1/models`.
    #[serde(default)]
    pub created: u64,
    /// Organization or entity that owns the model.
    /// Defaults to empty when a provider omits the field.
    #[serde(default)]
    pub owned_by: String,
}
