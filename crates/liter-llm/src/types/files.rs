use serde::{Deserialize, Serialize};

/// Purpose of an uploaded file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FilePurpose {
    /// File for use with Assistants API.
    #[default]
    Assistants,
    /// File for batch processing.
    Batch,
    /// File for fine-tuning.
    FineTune,
    /// File for vision/image tasks.
    Vision,
}

/// Request to upload a file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateFileRequest {
    /// Base64-encoded file data.
    pub file: String,
    /// Purpose for the file.
    pub purpose: FilePurpose,
    /// Optional filename to associate with the upload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// An uploaded file object.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FileObject {
    /// Unique file ID.
    pub id: String,
    /// Object type (always `"file"`).
    pub object: String,
    /// File size in bytes.
    pub bytes: u64,
    /// Unix timestamp of file creation.
    pub created_at: u64,
    /// Filename.
    pub filename: String,
    /// File purpose.
    pub purpose: String,
    /// Processing status (e.g., `"uploaded"`, `"processed"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Response from listing files.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FileListResponse {
    /// Object type (always `"list"`).
    pub object: String,
    /// List of file objects.
    pub data: Vec<FileObject>,
    /// Whether more results are available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

/// Query parameters for listing files.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileListQuery {
    /// Filter by file purpose (e.g., `"batch"`, `"fine-tune"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    /// Maximum number of results to return. Defaults to 20.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Pagination cursor: return results after this file ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// Response from a delete operation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeleteResponse {
    /// ID of the deleted resource.
    pub id: String,
    /// Object type.
    pub object: String,
    /// Confirmation that the resource was deleted.
    pub deleted: bool,
}
