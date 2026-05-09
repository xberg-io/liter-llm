//! Types for document OCR (optical character recognition) requests and responses.

use serde::{Deserialize, Serialize};

use super::common::Usage;

/// An OCR request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OcrRequest {
    /// The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`).
    pub model: String,
    /// The document to process.
    pub document: OcrDocument,
    /// Specific pages to process (1-indexed). `None` means all pages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<u32>>,
    /// Whether to include base64-encoded images of each page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_image_base64: Option<bool>,
}

/// Document input for OCR — either a URL or inline base64 data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OcrDocument {
    /// A publicly accessible document URL.
    #[serde(rename = "document_url")]
    Url {
        /// The document URL.
        url: String,
    },
    /// Inline base64-encoded document data.
    #[serde(rename = "base64")]
    Base64 {
        /// Base64-encoded document content.
        data: String,
        /// MIME type (e.g. `"application/pdf"`, `"image/png"`).
        media_type: String,
    },
}

impl Default for OcrDocument {
    fn default() -> Self {
        Self::Url { url: String::new() }
    }
}

/// An OCR response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResponse {
    /// Extracted pages.
    pub pages: Vec<OcrPage>,
    /// The model used.
    pub model: String,
    /// Token usage, if reported by the provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// A single page of OCR output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrPage {
    /// Page index (0-based).
    pub index: u32,
    /// Extracted content as Markdown.
    pub markdown: String,
    /// Extracted images, if `include_image_base64` was set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<OcrImage>>,
    /// Page dimensions in pixels, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<PageDimensions>,
}

/// An image extracted from an OCR page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrImage {
    /// Unique image identifier.
    pub id: String,
    /// Base64-encoded image data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_base64: Option<String>,
}

/// Page dimensions in pixels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageDimensions {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}
