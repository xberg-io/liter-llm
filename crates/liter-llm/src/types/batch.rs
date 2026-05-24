use serde::{Deserialize, Serialize};

/// Request to create a batch job.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateBatchRequest {
    /// ID of the uploaded input file (JSONL format).
    pub input_file_id: String,
    /// API endpoint (e.g., `"/v1/chat/completions"`).
    pub endpoint: String,
    /// Completion window (e.g., `"24h"`).
    pub completion_window: String,
    /// Optional metadata to attach to the batch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Status of a batch job.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    /// Validating the input file.
    #[default]
    Validating,
    /// Job failed.
    Failed,
    /// Job is running.
    InProgress,
    /// Finalizing results.
    Finalizing,
    /// Job completed successfully.
    Completed,
    /// Job expired before completion.
    Expired,
    /// Job is being cancelled.
    Cancelling,
    /// Job has been cancelled.
    Cancelled,
}

#[cfg_attr(alef, alef(skip))]
impl std::fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        f.write_str(&s)
    }
}

/// A batch job object.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BatchObject {
    /// Unique batch ID.
    pub id: String,
    /// Object type (always `"batch"`).
    pub object: String,
    /// API endpoint (e.g., `"/v1/chat/completions"`).
    pub endpoint: String,
    /// ID of the input file.
    pub input_file_id: String,
    /// Completion window (e.g., `"24h"`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub completion_window: String,
    /// Current job status.
    pub status: BatchStatus,
    /// ID of the output file (present when completed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_file_id: Option<String>,
    /// ID of the error file (present if some requests failed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_file_id: Option<String>,
    /// Unix timestamp of batch creation.
    pub created_at: u64,
    /// Unix timestamp of completion (if completed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<u64>,
    /// Unix timestamp of failure (if failed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<u64>,
    /// Unix timestamp of expiration (if expired).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expired_at: Option<u64>,
    /// Request processing counts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_counts: Option<BatchRequestCounts>,
    /// Metadata attached to the batch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Request processing counts for a batch.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchRequestCounts {
    /// Total requests in the batch.
    pub total: u64,
    /// Completed requests.
    pub completed: u64,
    /// Failed requests.
    pub failed: u64,
}

/// Response from listing batches.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BatchListResponse {
    /// Object type (always `"list"`).
    pub object: String,
    /// List of batch objects.
    pub data: Vec<BatchObject>,
    /// Whether more results are available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
    /// First batch ID in the result set (for pagination).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_id: Option<String>,
    /// Last batch ID in the result set (for pagination).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_id: Option<String>,
}

/// Query parameters for listing batches.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchListQuery {
    /// Maximum number of results to return. Defaults to 20.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Pagination cursor: return results after this batch ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}
