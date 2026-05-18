use serde::{Deserialize, Serialize};

/// Request to create a structured response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateResponseRequest {
    /// Model ID.
    pub model: String,
    /// Input data to process (e.g., a document to extract from).
    pub input: serde_json::Value,
    /// Instructions for processing the input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// Available tools the model can use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ResponseTool>>,
    /// Sampling temperature in `[0.0, 2.0]`. Defaults to 1.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Maximum output tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,
    /// Optional metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// A tool available for the response request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResponseTool {
    /// Tool type (e.g., "extractor", "search").
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Tool configuration (flattened into the object).
    #[serde(flatten)]
    pub config: serde_json::Value,
}

/// Response from a structured response request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResponseObject {
    /// Unique response ID.
    pub id: String,
    /// Object type (e.g., `"response"`).
    pub object: String,
    /// Unix timestamp of response creation.
    pub created_at: u64,
    /// Model used to generate the response.
    pub model: String,
    /// Status (e.g., `"succeeded"`, `"failed"`).
    pub status: String,
    /// Output items from the response.
    pub output: Vec<ResponseOutputItem>,
    /// Token usage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<ResponseUsage>,
    /// Error details (if status is "failed").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

/// A single output item from the response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResponseOutputItem {
    /// Output type (e.g., `"text"`, `"object"`, `"error"`).
    #[serde(rename = "type")]
    pub item_type: String,
    /// Output content (flattened into the object).
    #[serde(flatten)]
    pub content: serde_json::Value,
}

/// Token usage for a response.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseUsage {
    /// Input tokens used.
    pub input_tokens: u64,
    /// Output tokens used.
    pub output_tokens: u64,
    /// Total tokens used.
    pub total_tokens: u64,
}
