use serde::{Deserialize, Serialize};

/// Request to create images from a text prompt.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateImageRequest {
    /// Text description of the image to generate.
    pub prompt: String,
    /// Model ID (e.g., `"dall-e-3"`). Optional; API may use default if unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Number of images to generate. Defaults to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    /// Image size (e.g., `"1024x1024"`, `"1792x1024"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    /// Image quality: `"standard"` or `"hd"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    /// Style: `"natural"` or `"vivid"` (DALL-E 3 only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Response format: `"url"` or `"b64_json"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
    /// User identifier for request tracking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Response containing generated images.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImagesResponse {
    /// Unix timestamp of image creation.
    pub created: u64,
    /// List of generated images.
    pub data: Vec<Image>,
}

/// A single generated image, returned as either a URL or base64 data.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Image {
    /// Image URL (if response_format was "url").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Base64-encoded image data (if response_format was "b64_json").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b64_json: Option<String>,
    /// The final prompt used to generate the image (DALL-E 3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,
}
