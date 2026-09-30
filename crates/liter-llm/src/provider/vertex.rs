use std::borrow::Cow;
use std::sync::atomic::AtomicU64;

use crate::error::{LiterLlmError, Result};
use crate::provider::Provider;
use crate::types::ChatCompletionChunk;

mod gemini_request;
mod gemini_response;
mod gemini_stream;

pub(crate) use gemini_request::transform_gemini_request;
pub(crate) use gemini_response::transform_gemini_response;
pub(crate) use gemini_stream::parse_gemini_stream_event;

/// Default Vertex AI location when none is specified.
const DEFAULT_LOCATION: &str = "us-central1";

/// Global counter for generating unique tool call IDs.
static TOOL_CALL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Map reasoning effort levels to a Gemini `thinkingConfig.thinkingBudget` token count.
///
/// Mirrors the tiers used by [`super::bedrock::reasoning_effort_to_budget_tokens`] for
/// Claude-on-Bedrock extended thinking, adapted to Gemini's budget semantics (`0`
/// disables thinking on models that allow it).
fn reasoning_effort_to_thinking_budget(effort: &str) -> i64 {
    match effort {
        "minimal" => 0,
        "low" => 1024,
        "medium" => 4096,
        "high" => 16384,
        "max" => 24576,
        _ => 4096,
    }
}

/// Google Vertex AI / Gemini provider.
///
/// Differences from the OpenAI-compatible baseline:
/// - Auth uses `Authorization: Bearer <token>` where the token is a Google
///   Cloud OAuth2 access token (obtained via ADC, service account, or
///   `gcloud auth print-access-token`).
/// - The base URL is constructed from `VERTEXAI_PROJECT` and `VERTEXAI_LOCATION`
///   environment variables, or can be overridden via `base_url` in [`ClientConfig`].
///   The resulting URL follows the pattern:
///   `https://{location}-aiplatform.googleapis.com/v1/projects/{project}/locations/{location}`
/// - Model names are routed via the `vertex_ai/` prefix which is stripped
///   before being sent in the request body.
/// - The native Gemini `generateContent` format is used, not the OpenAI
///   `/chat/completions` path. Request and response are translated accordingly.
/// - Streaming uses SSE with `?alt=sse`; each chunk is a full `generateContent`
///   response JSON wrapped in a standard SSE `data:` line.
///
/// # Token management
///
/// Three options, listed by preference:
///
/// 1. **Automatic ADC** (recommended for GKE / Cloud Run / Compute Engine).
///    Construct the client with no `api_key` and no `credential_provider`;
///    `DefaultClient::new` will auto-install [`VertexAdcCredentialProvider`]
///    which obtains short-lived OAuth2 tokens from the metadata server, with
///    a `gcp_auth` ADC discovery fallback for local development.
///
/// 2. **Explicit credential provider.** Supply your own
///    [`CredentialProvider`] (e.g. [`VertexOAuthCredentialProvider`] for the
///    service-account JWT flow) via
///    `ClientConfigBuilder::credential_provider`. The client calls
///    `resolve()` before each request and uses the returned bearer token.
///
/// 3. **Pre-obtained access token.** Supply a token as the `api_key`
///    parameter. The caller is responsible for refresh before expiry.
///
/// [`VertexAdcCredentialProvider`]: crate::auth::vertex_adc::VertexAdcCredentialProvider
/// [`VertexOAuthCredentialProvider`]: crate::auth::vertex_oauth::VertexOAuthCredentialProvider
/// [`CredentialProvider`]: crate::auth::CredentialProvider
///
/// # Environment variables
///
/// - `VERTEXAI_PROJECT` (required): Google Cloud project ID.
/// - `VERTEXAI_LOCATION` (optional): GCP region, defaults to `us-central1`.
///
/// # Configuration
///
/// ```rust,ignore
/// // Option 1: GKE Workload Identity / ADC — empty api_key, no credential_provider.
/// // export VERTEXAI_PROJECT=my-project
/// // export VERTEXAI_LOCATION=us-central1
/// let config = ClientConfigBuilder::new("").build();
/// let client = DefaultClient::new(config, Some("vertex_ai/gemini-2.5-flash-lite"))?;
///
/// // Option 2: Pre-obtained token.
/// let config = ClientConfigBuilder::new("ya29.your-access-token").build();
/// let client = DefaultClient::new(config, Some("vertex_ai/gemini-2.0-flash"))?;
///
/// // Option 3: Explicit base_url override (bypasses env var resolution).
/// let config = ClientConfigBuilder::new("ya29.your-access-token")
///     .base_url(
///         "https://us-central1-aiplatform.googleapis.com/v1/\
///          projects/my-project/locations/us-central1",
///     )
///     .build();
/// let client = DefaultClient::new(config, Some("vertex_ai/gemini-2.0-flash"))?;
/// ```
pub struct VertexAiProvider {
    /// Cached base URL: `https://{location}-aiplatform.googleapis.com/v1/projects/{project}/locations/{location}`.
    base_url: String,
}

impl VertexAiProvider {
    /// Construct with an explicit project and location.
    #[must_use]
    pub fn new(project: impl Into<String>, location: impl Into<String>) -> Self {
        let project = project.into();
        let location = location.into();
        let base_url =
            format!("https://{location}-aiplatform.googleapis.com/v1/projects/{project}/locations/{location}");
        Self { base_url }
    }

    /// Construct from environment variables.
    ///
    /// Reads `VERTEXAI_PROJECT` and `VERTEXAI_LOCATION` (defaults to `us-central1`).
    /// If `VERTEXAI_PROJECT` is not set, the base URL will be empty and
    /// [`validate`] will return an error.
    #[must_use]
    pub fn from_env() -> Self {
        let project = std::env::var("VERTEXAI_PROJECT").unwrap_or_default();
        let location = std::env::var("VERTEXAI_LOCATION").unwrap_or_else(|_| DEFAULT_LOCATION.to_owned());
        if project.is_empty() {
            return Self {
                base_url: String::new(),
            };
        }
        Self::new(project, location)
    }
}

impl Provider for VertexAiProvider {
    fn name(&self) -> &str {
        "vertex_ai"
    }

    /// Vertex AI base URL constructed from project and location.
    ///
    /// Returns an empty string when the provider was constructed without a
    /// valid project (e.g. `VERTEXAI_PROJECT` not set). The [`validate`]
    /// method catches this at client construction time.
    fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Validate that required configuration is present.
    ///
    /// Checks that the base URL was successfully constructed from environment
    /// variables (`VERTEXAI_PROJECT` is required, `VERTEXAI_LOCATION` defaults
    /// to `us-central1`).
    fn validate(&self) -> Result<()> {
        if self.base_url.is_empty() {
            return Err(LiterLlmError::BadRequest {
                message: "Vertex AI requires a project ID. \
                          Set VERTEXAI_PROJECT (and optionally VERTEXAI_LOCATION) \
                          in the environment, or provide an explicit base_url in \
                          ClientConfig."
                    .into(),
                status: 400,
            });
        }
        Ok(())
    }

    fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
        Some((Cow::Borrowed("Authorization"), Cow::Owned(format!("Bearer {api_key}"))))
    }

    fn matches_model(&self, model: &str) -> bool {
        model.starts_with("vertex_ai/")
    }

    fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
        model.strip_prefix("vertex_ai/").unwrap_or(model)
    }

    /// Build the full URL for a Gemini API request.
    ///
    /// Chat completions → `{base}/publishers/google/models/{model}:generateContent`
    /// Embeddings       → `{base}/publishers/google/models/{model}:predict`
    /// Other paths      → `{base}{endpoint_path}`
    fn build_url(&self, endpoint_path: &str, model: &str) -> String {
        let base = self.base_url();
        if base.is_empty() {
            return String::new();
        }
        let base = base.trim_end_matches('/');
        if endpoint_path.contains("chat/completions") {
            format!("{base}/publishers/google/models/{model}:generateContent")
        } else if endpoint_path.contains("embeddings") {
            format!("{base}/publishers/google/models/{model}:predict")
        } else {
            format!("{base}{endpoint_path}")
        }
    }

    fn transform_request(&self, body: &mut serde_json::Value) -> Result<()> {
        if body.get("input").is_some() && body.get("messages").is_none() {
            return transform_vertex_embed_request(body);
        }
        transform_gemini_request(body)
    }

    fn transform_response(&self, body: &mut serde_json::Value) -> Result<()> {
        transform_gemini_response(body)
    }

    /// Build the streaming URL: `:streamGenerateContent` plus `?alt=sse`.
    ///
    /// ~keep Streaming is a DISTINCT method, not `generateContent` with a flag.
    /// ~keep `:generateContent` ignores `alt=sse` and returns one ordinary JSON
    /// ~keep body, which the SSE parser cannot frame — the caller sees a single
    /// ~keep `Streaming { "SSE stream truncated" }` error whose message gives no
    /// ~keep hint that the endpoint was wrong.
    fn build_stream_url(&self, endpoint_path: &str, model: &str) -> String {
        let url = self.build_url(endpoint_path, model);
        if url.is_empty() {
            return url;
        }
        let url = url.replace(":generateContent", ":streamGenerateContent");
        format!("{url}?alt=sse")
    }

    fn parse_stream_event(&self, event_data: &str) -> Result<Option<ChatCompletionChunk>> {
        parse_gemini_stream_event(event_data)
    }
}

/// Transform an OpenAI embedding request to Gemini `embedContent` format.
///
/// OpenAI: `{"model": "...", "input": "text"}`
/// Gemini: `{"content": {"parts": [{"text": "text"}]}}`
fn transform_gemini_embed_request(body: &mut serde_json::Value) -> Result<()> {
    use crate::error::LiterLlmError;
    use serde_json::json;

    let input = body.get("input").cloned().unwrap_or_default();

    let text = match &input {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) if arr.iter().all(serde_json::Value::is_string) => {
            arr.first().and_then(|v| v.as_str()).unwrap_or("").to_string()
        }
        _ => {
            return Err(LiterLlmError::BadRequest {
                message: "Google AI embedding adapters support text input only; use a multimodal-compatible custom provider for image embeddings".into(),
                status: 400,
            });
        }
    };

    let new_body = json!({
        "content": {
            "parts": [{"text": text}]
        }
    });

    *body = new_body;
    Ok(())
}

/// Transform an OpenAI embedding request to Vertex AI `:predict` format.
///
/// OpenAI: `{"model": "...", "input": "text"}`
/// Vertex: `{"instances": [{"content": "text"}]}`
fn transform_vertex_embed_request(body: &mut serde_json::Value) -> Result<()> {
    use crate::error::LiterLlmError;
    use serde_json::json;

    let input = body.get("input").cloned().unwrap_or_default();

    let text = match &input {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) if arr.iter().all(serde_json::Value::is_string) => {
            arr.first().and_then(|v| v.as_str()).unwrap_or("").to_string()
        }
        _ => {
            return Err(LiterLlmError::BadRequest {
                message: "Vertex AI embedding adapters support text input only; use a multimodal-compatible custom provider for image embeddings".into(),
                status: 400,
            });
        }
    };

    *body = json!({
        "instances": [{"content": text}]
    });
    Ok(())
}

/// Convert OpenAI user content (string or content-part array) to Gemini parts.
///
/// Handles four cases:
/// 1. Plain string -> single text part.
/// 2. Array of content parts -> each part converted to Gemini format.
/// 3. `ContentPart::Document` -> Gemini `inlineData` with the document's MIME type.
/// 4. None/null -> single empty text part.
pub(crate) fn convert_user_content_to_gemini(content: Option<&serde_json::Value>) -> Vec<serde_json::Value> {
    use serde_json::json;

    match content {
        Some(serde_json::Value::String(s)) => vec![json!({"text": s})],
        Some(serde_json::Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| {
                let part_type = part.get("type").and_then(|t| t.as_str())?;
                match part_type {
                    "text" => {
                        let text = part.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        Some(json!({"text": text}))
                    }
                    "image_url" => {
                        let url = part.pointer("/image_url/url").and_then(|u| u.as_str())?;
                        if url.starts_with("data:")
                            && let Some((header, data)) = url.split_once(',')
                        {
                            let mime_type = header.trim_start_matches("data:").trim_end_matches(";base64");
                            return Some(json!({
                                "inlineData": {
                                    "mimeType": mime_type,
                                    "data": data
                                }
                            }));
                        }
                        Some(json!({
                            "fileData": {
                                "mimeType": "image/jpeg",
                                "fileUri": url
                            }
                        }))
                    }
                    "document" => {
                        let doc = part.get("document")?;
                        let data = doc.get("data").and_then(|d| d.as_str())?;
                        let media_type = doc
                            .get("media_type")
                            .and_then(|m| m.as_str())
                            .unwrap_or("application/pdf");
                        Some(json!({
                            "inlineData": {
                                "mimeType": media_type,
                                "data": data
                            }
                        }))
                    }
                    _ => {
                        let text = part.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        if text.is_empty() {
                            None
                        } else {
                            Some(json!({"text": text}))
                        }
                    }
                }
            })
            .collect(),
        _ => vec![json!({"text": ""})],
    }
}

/// Translate OpenAI `tool_choice` to Gemini `toolConfig.functionCallingConfig`.
///
/// OpenAI `tool_choice` values:
/// - `"none"` -> `NONE`
/// - `"auto"` -> `AUTO`
/// - `"required"` -> `ANY`
/// - `{"type": "function", "function": {"name": "..."}}` -> `ANY` with `allowedFunctionNames`
fn translate_tool_choice(tool_choice: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    use serde_json::json;

    let tc = tool_choice?;

    if let Some(s) = tc.as_str() {
        let mode = match s {
            "none" => "NONE",
            "auto" => "AUTO",
            "required" => "ANY",
            _ => return None,
        };
        return Some(json!({
            "functionCallingConfig": {
                "mode": mode
            }
        }));
    }

    if let Some(name) = tc.pointer("/function/name").and_then(|n| n.as_str()) {
        return Some(json!({
            "functionCallingConfig": {
                "mode": "ANY",
                "allowedFunctionNames": [name]
            }
        }));
    }

    None
}

#[cfg(test)]
mod response_tests;
#[cfg(test)]
mod tests;
