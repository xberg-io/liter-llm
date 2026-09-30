/// Type-state builder for [`DefaultClient`] ([`ClientBuilder`]).
pub mod builder;
/// Client builder configuration ([`ClientConfig`] and related helpers).
pub mod config;
/// On-disk client configuration schema (TOML / JSON / YAML).
#[allow(missing_docs)]
pub mod config_file;
/// Canonical, binding-friendly [`LlmConfig`] and its conversion into [`ClientConfigBuilder`].
pub mod llm_config;
/// Tower-backed managed client wired with rate limit, cache, routing, etc.
#[cfg(all(feature = "native-http", feature = "tower"))]
pub mod managed;

use std::future::Future;
use std::pin::Pin;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use std::sync::Arc;

use futures_core::Stream;

use crate::error::{LiterLlmError, Result};
use crate::types::audio::{CreateSpeechRequest, CreateTranscriptionRequest, TranscriptionResponse};
use crate::types::batch::{BatchListQuery, BatchListResponse, BatchObject, CreateBatchRequest};
use crate::types::files::{CreateFileRequest, DeleteResponse, FileListQuery, FileListResponse, FileObject};
use crate::types::image::{CreateImageRequest, ImagesResponse};
use crate::types::moderation::{ModerationRequest, ModerationResponse};
use crate::types::ocr::{OcrRequest, OcrResponse};
use crate::types::raw::{RawExchange, RawStreamExchange};
use crate::types::rerank::{RerankRequest, RerankResponse};
use crate::types::responses::{CreateResponseRequest, ResponseObject, ResponseStreamEvent};
use crate::types::search::{SearchRequest, SearchResponse};
use crate::types::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse,
    ModelsListResponse,
};

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use crate::auth::Credential;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use crate::http;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use crate::provider::{self, OpenAiCompatibleProvider, OpenAiProvider, Provider};
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use secrecy::ExposeSecret;

pub use builder::{ClientBuilder, NoApiKey, NoProvider, WithApiKey, WithProvider};
pub use config::{ClientConfig, ClientConfigBuilder};
pub use config_file::FileConfig;
pub use llm_config::{
    BedrockConfig, LlmBudgetConfig, LlmCacheConfig, LlmConfig, LlmInFlightLimitConfig, LlmProviderConfig,
    LlmRateLimitConfig,
};

use crate::types::batch::BatchStatus;
use std::time::Duration;

/// Configuration for polling a batch until terminal status.
///
/// All time values are in seconds as `f64` so the struct bridges across FFI
/// boundaries without requiring a `Duration` shim.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WaitForBatchConfig {
    /// Initial interval between polls, in seconds.
    pub initial_interval_secs: f64,
    /// Maximum interval between polls (backoff plateau), in seconds.
    pub max_interval_secs: f64,
    /// Exponential backoff multiplier (e.g., 1.5 increases delay by 50% each poll).
    pub backoff_multiplier: f32,
    /// Optional timeout in seconds — polling fails if this duration is exceeded.
    pub timeout_secs: Option<f64>,
}

impl Default for WaitForBatchConfig {
    fn default() -> Self {
        Self {
            initial_interval_secs: 5.0,
            max_interval_secs: 60.0,
            backoff_multiplier: 1.5,
            timeout_secs: None,
        }
    }
}

/// Error type for batch polling operations.
///
/// All fields use FFI-friendly types so the error can be represented across
/// every language binding without a shim. `Duration` fields are expressed
/// as `f64` seconds; `LiterLlmError` is flattened to `message` + `code`.
#[derive(Debug, thiserror::Error)]
pub enum BatchWaitError {
    /// Batch reached a terminal failure state.
    #[error("batch reached terminal failure state: {status:?}")]
    Failed {
        /// Terminal batch status (Failed, Expired, or Cancelled).
        status: BatchStatus,
    },

    /// Polling timed out before reaching terminal status.
    #[error("polling timed out after {timeout_secs:.1}s")]
    Timeout {
        /// Configured timeout in seconds.
        timeout_secs: f64,
    },

    /// Underlying client error, flattened to `message` + numeric `code`.
    #[error("client error (code {code}): {message}")]
    Client {
        /// Human-readable error description.
        message: String,
        /// Numeric error code (HTTP status, or 0 for non-HTTP errors).
        code: u32,
    },
}

impl From<LiterLlmError> for BatchWaitError {
    fn from(err: LiterLlmError) -> Self {
        Self::Client {
            code: u32::from(err.status_code()),
            message: err.to_string(),
        }
    }
}

/// A boxed future returning `T`.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A boxed future returning `T` (WASM variant — not `Send` because JS is single-threaded).
#[cfg(target_arch = "wasm32")]
#[cfg_attr(alef, alef(skip))]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// A boxed stream of `T`.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + Send + 'a>>;

/// A boxed stream of `T` (WASM variant — not `Send` because JS is single-threaded).
#[cfg(target_arch = "wasm32")]
#[cfg_attr(alef, alef(skip))]
pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + 'a>>;

/// Result of [`DefaultClient::prepare_request`].
///
/// The body is pre-serialized into `bytes::Bytes` so it is serialized exactly
/// once — the same bytes are used for signing headers and for the HTTP request
/// body.  On retry, cloning `Bytes` is a zero-copy ref-count bump.
///
/// `body_json` is the pre-serialization JSON value, retained so that
/// [`Provider::dynamic_headers`] can inspect request fields without
/// re-parsing.
///
/// The `provider` is the resolved provider for this specific request — it may
/// differ from `self.provider` when the model prefix identifies a different
/// provider.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
struct PreparedRequest {
    url: String,
    provider: Arc<dyn Provider>,
    body_json: serde_json::Value,
    body_bytes: bytes::Bytes,
}

/// Convert an owned `(String, String)` auth header pair to `(&str, &str)` borrows.
///
/// Centralises the four identical `map(|(n, v)| (n.as_str(), v.as_str()))` expressions
/// that appear wherever we hand headers to the HTTP layer.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn str_pair(pair: &(String, String)) -> (&str, &str) {
    (pair.0.as_str(), pair.1.as_str())
}

/// Shallow-merge a top-level `"extra_body"` key into `body`, OpenAI-Python-SDK
/// style: `{**body, **extra_body}` — extra_body keys override identically
/// named top-level keys.
///
/// On the chat path, providers that consume `extra_body` themselves (Anthropic,
/// Vertex, Bedrock) already strip it inside their own `transform_request`, so by the
/// time this runs there is nothing left for them to merge; there it only has an
/// effect for OpenAI-compatible providers, which pass `extra_body` through
/// unchanged. That caveat is chat-path-only: the Responses path
/// ([`response_request_body`]) runs no provider transform at all, so the merged keys
/// always reach the wire as literal OpenAI Responses fields.
///
/// A non-object `extra_body` cannot be merged into the body root, so it is dropped
/// with a warning rather than sent to the wire.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn merge_extra_body(body: &mut serde_json::Value) {
    let Some(obj) = body.as_object_mut() else {
        return;
    };
    let Some(extra_body) = obj.remove("extra_body") else {
        return;
    };
    match extra_body {
        serde_json::Value::Object(extra_fields) => {
            obj.extend(extra_fields);
        }
        other => {
            tracing::warn!(
                extra_body_type = json_value_type_name(&other),
                "ignoring non-object extra_body; it cannot be merged into the request body"
            );
        }
    }
}

/// Serialize a Responses API request body with `extra_body` merged in,
/// via [`merge_extra_body`] — the same merge semantics as the chat path.
///
/// The merge is the *only* thing shared with the chat path. There is deliberately no
/// [`Provider::transform_request`] call here: the Responses body is a different wire
/// shape from the chat body, and the provider transforms are written against the chat
/// shape, so applying them would mangle the request rather than adapt it. The Responses
/// path is OpenAI-only — see [`ResponseClient`](trait@ResponseClient).
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn response_request_body(req: &CreateResponseRequest) -> Result<serde_json::Value> {
    let mut body = serde_json::to_value(req)?;
    merge_extra_body(&mut body);
    Ok(body)
}

/// Like [`response_request_body`], but for `create_response_stream`: forces
/// `stream: true` in the request, both before and after the `extra_body`
/// merge.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn response_stream_request_body(mut req: CreateResponseRequest) -> Result<serde_json::Value> {
    req.stream = Some(true);
    let mut body = response_request_body(&req)?;
    // ~keep extra_body must never override `stream`: the transport path (post_json_raw vs
    // post_stream) is chosen by the calling method, not the body, so a mismatched wire flag
    // would desync request from response parsing.
    if let Some(obj) = body.as_object_mut() {
        obj.insert("stream".into(), serde_json::Value::Bool(true));
    }
    Ok(body)
}

/// Human-readable JSON type name, used only for diagnostic tracing fields.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn json_value_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Core LLM client trait.
///
/// Provides unified access to LLM and multimodal APIs across 165 providers.
/// Requests are routed to the correct provider based on the model name prefix
/// (e.g. `anthropic/claude-3-5-sonnet` routes to Anthropic) or via explicit
/// `base_url` override.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub trait LlmClient: Send + Sync {
    /// Send a chat completion request.
    ///
    /// Routes the request to the detected provider based on the model prefix
    /// in the `ChatCompletionRequest`. Provider-specific transformations
    /// (request normalization, header signing) are applied automatically.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if the model is empty.
    /// Returns `LiterLlmError::Authentication` if credentials are missing or invalid.
    /// Returns `LiterLlmError::Http` for network or HTTP-level errors.
    /// Returns `LiterLlmError::ProviderError` if the provider rejects the request.
    fn chat(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>>;

    /// Send a streaming chat completion request.
    ///
    /// Returns a stream of `ChatCompletionChunk` items, each representing a
    /// single token delta from the provider. The stream terminates when the
    /// model reaches `stop_reason = "stop"` or `"length"`.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`chat`](Self::chat).
    /// Stream errors are returned as `Err` items in the stream itself.
    ///
    /// # Notes
    ///
    /// Chunks are yielded as soon as they arrive; the stream is not buffered.
    fn chat_stream(
        &self,
        req: ChatCompletionRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>>;

    /// Send an embedding request.
    ///
    /// Computes dense vector representations for semantic search, clustering, or similarity.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if input is empty.
    /// Returns `LiterLlmError::Http` for network errors.
    fn embed(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>>;

    /// List available models.
    ///
    /// Queries the provider's model list endpoint.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::Http` for network errors.
    /// Returns `LiterLlmError::Authentication` if the API key lacks list permissions.
    fn list_models(&self) -> BoxFuture<'_, Result<ModelsListResponse>>;

    /// Generate an image.
    ///
    /// Creates one or more images based on a text prompt.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if the prompt is empty.
    /// Returns `LiterLlmError::Http` for network errors.
    /// Returns `LiterLlmError::ProviderError` if the prompt violates content policy.
    fn image_generate(&self, req: CreateImageRequest) -> BoxFuture<'_, Result<ImagesResponse>>;

    /// Generate speech audio from text.
    ///
    /// Converts text to speech (TTS) using the specified voice model.
    /// Returns raw audio bytes in the requested format.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if text is empty.
    /// Returns `LiterLlmError::Http` for network errors.
    fn speech(&self, req: CreateSpeechRequest) -> BoxFuture<'_, Result<bytes::Bytes>>;

    /// Transcribe audio to text.
    ///
    /// Converts audio files to text using automatic speech recognition (ASR).
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if the audio file is missing.
    /// Returns `LiterLlmError::Http` for network errors.
    fn transcribe(&self, req: CreateTranscriptionRequest) -> BoxFuture<'_, Result<TranscriptionResponse>>;

    /// Check content against moderation policies.
    ///
    /// Evaluates text or images for potentially harmful content.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if input is empty.
    /// Returns `LiterLlmError::Http` for network errors.
    fn moderate(&self, req: ModerationRequest) -> BoxFuture<'_, Result<ModerationResponse>>;

    /// Rerank documents by relevance to a query.
    ///
    /// Orders a list of documents by their relevance to a search query.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if query or documents are empty.
    /// Returns `LiterLlmError::Http` for network errors.
    fn rerank(&self, req: RerankRequest) -> BoxFuture<'_, Result<RerankResponse>>;

    /// Perform a web/document search.
    ///
    /// Searches the web or a provider's document index for results matching the query.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if the query is empty.
    /// Returns `LiterLlmError::Http` for network errors.
    fn search(&self, req: SearchRequest) -> BoxFuture<'_, Result<SearchResponse>>;

    /// Extract text from a document via OCR.
    ///
    /// Performs optical character recognition (OCR) on images or scanned PDFs.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if the document is missing.
    /// Returns `LiterLlmError::Http` for network errors.
    fn ocr(&self, req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>>;
}

/// Core LLM client trait (WASM variant — no `Send + Sync` because JS is single-threaded).
#[cfg(target_arch = "wasm32")]
#[cfg_attr(alef, alef(skip))]
pub trait LlmClient {
    /// Send a chat completion request.
    fn chat(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>>;

    /// Send a streaming chat completion request.
    fn chat_stream(
        &self,
        req: ChatCompletionRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>>;

    /// Send an embedding request.
    fn embed(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>>;

    /// List available models.
    fn list_models(&self) -> BoxFuture<'_, Result<ModelsListResponse>>;

    /// Generate an image.
    fn image_generate(&self, req: CreateImageRequest) -> BoxFuture<'_, Result<ImagesResponse>>;

    /// Generate speech audio from text.
    fn speech(&self, req: CreateSpeechRequest) -> BoxFuture<'_, Result<bytes::Bytes>>;

    /// Transcribe audio to text.
    fn transcribe(&self, req: CreateTranscriptionRequest) -> BoxFuture<'_, Result<TranscriptionResponse>>;

    /// Check content against moderation policies.
    fn moderate(&self, req: ModerationRequest) -> BoxFuture<'_, Result<ModerationResponse>>;

    /// Rerank documents by relevance to a query.
    fn rerank(&self, req: RerankRequest) -> BoxFuture<'_, Result<RerankResponse>>;

    /// Perform a web/document search.
    fn search(&self, req: SearchRequest) -> BoxFuture<'_, Result<SearchResponse>>;

    /// Extract text from a document via OCR.
    fn ocr(&self, req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>>;
}

/// Extension of [`LlmClient`] that returns raw request/response data
/// alongside the typed response.
///
/// Every `_raw` method mirrors its counterpart on [`LlmClient`] but wraps the
/// result in a [`RawExchange`] that exposes the final request body (after
/// `transform_request`) and the raw provider response (before
/// `transform_response`). This is useful for debugging provider-specific
/// transformations, capturing wire-level data, or implementing custom parsing.
#[cfg_attr(alef, alef(skip))]
pub trait LlmClientRaw: LlmClient {
    /// Send a chat completion request and return the raw exchange.
    ///
    /// The `raw_request` field contains the final JSON body sent to the
    /// provider; `raw_response` contains the provider JSON before
    /// normalization.
    fn chat_raw(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<RawExchange<ChatCompletionResponse>>>;

    /// Send a streaming chat completion request and return the raw exchange.
    ///
    /// Only `raw_request` is available upfront — the stream itself is
    /// returned in `stream` and consumed incrementally.
    fn chat_stream_raw(
        &self,
        req: ChatCompletionRequest,
    ) -> BoxFuture<'_, Result<RawStreamExchange<BoxStream<'static, Result<ChatCompletionChunk>>>>>;

    /// Send an embedding request and return the raw exchange.
    fn embed_raw(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<RawExchange<EmbeddingResponse>>>;

    /// Generate an image and return the raw exchange.
    fn image_generate_raw(&self, req: CreateImageRequest) -> BoxFuture<'_, Result<RawExchange<ImagesResponse>>>;

    /// Transcribe audio to text and return the raw exchange.
    fn transcribe_raw(
        &self,
        req: CreateTranscriptionRequest,
    ) -> BoxFuture<'_, Result<RawExchange<TranscriptionResponse>>>;

    /// Check content against moderation policies and return the raw exchange.
    fn moderate_raw(&self, req: ModerationRequest) -> BoxFuture<'_, Result<RawExchange<ModerationResponse>>>;

    /// Rerank documents by relevance to a query and return the raw exchange.
    fn rerank_raw(&self, req: RerankRequest) -> BoxFuture<'_, Result<RawExchange<RerankResponse>>>;

    /// Perform a web/document search and return the raw exchange.
    fn search_raw(&self, req: SearchRequest) -> BoxFuture<'_, Result<RawExchange<SearchResponse>>>;

    /// Extract text from a document via OCR and return the raw exchange.
    fn ocr_raw(&self, req: OcrRequest) -> BoxFuture<'_, Result<RawExchange<OcrResponse>>>;
}

/// File management operations (upload, list, retrieve, delete).
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub trait FileClient: Send + Sync {
    /// Upload a file.
    fn create_file(&self, req: CreateFileRequest) -> BoxFuture<'_, Result<FileObject>>;

    /// Retrieve metadata for a file.
    fn retrieve_file(&self, file_id: &str) -> BoxFuture<'_, Result<FileObject>>;

    /// Delete a file.
    fn delete_file(&self, file_id: &str) -> BoxFuture<'_, Result<DeleteResponse>>;

    /// List files, optionally filtered by query parameters.
    fn list_files(&self, query: Option<FileListQuery>) -> BoxFuture<'_, Result<FileListResponse>>;

    /// Retrieve the raw content of a file.
    fn file_content(&self, file_id: &str) -> BoxFuture<'_, Result<bytes::Bytes>>;
}

/// File management operations (upload, list, retrieve, delete) (WASM variant).
#[cfg(target_arch = "wasm32")]
#[cfg_attr(alef, alef(skip))]
pub trait FileClient {
    /// Upload a file.
    fn create_file(&self, req: CreateFileRequest) -> BoxFuture<'_, Result<FileObject>>;

    /// Retrieve metadata for a file.
    fn retrieve_file(&self, file_id: &str) -> BoxFuture<'_, Result<FileObject>>;

    /// Delete a file.
    fn delete_file(&self, file_id: &str) -> BoxFuture<'_, Result<DeleteResponse>>;

    /// List files, optionally filtered by query parameters.
    fn list_files(&self, query: Option<FileListQuery>) -> BoxFuture<'_, Result<FileListResponse>>;

    /// Retrieve the raw content of a file.
    fn file_content(&self, file_id: &str) -> BoxFuture<'_, Result<bytes::Bytes>>;
}

/// Batch processing operations (create, list, retrieve, cancel).
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub trait BatchClient: Send + Sync {
    /// Create a new batch job.
    fn create_batch(&self, req: CreateBatchRequest) -> BoxFuture<'_, Result<BatchObject>>;

    /// Retrieve a batch by ID.
    fn retrieve_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>>;

    /// List batches, optionally filtered by query parameters.
    fn list_batches(&self, query: Option<BatchListQuery>) -> BoxFuture<'_, Result<BatchListResponse>>;

    /// Cancel an in-progress batch.
    fn cancel_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>>;
}

/// Batch processing operations (create, list, retrieve, cancel) (WASM variant).
#[cfg(target_arch = "wasm32")]
#[cfg_attr(alef, alef(skip))]
pub trait BatchClient {
    /// Create a new batch job.
    fn create_batch(&self, req: CreateBatchRequest) -> BoxFuture<'_, Result<BatchObject>>;

    /// Retrieve a batch by ID.
    fn retrieve_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>>;

    /// List batches, optionally filtered by query parameters.
    fn list_batches(&self, query: Option<BatchListQuery>) -> BoxFuture<'_, Result<BatchListResponse>>;

    /// Cancel an in-progress batch.
    fn cancel_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>>;
}

/// OpenAI Responses API operations (create, retrieve, cancel).
///
/// # Provider support: OpenAI only
///
/// Every other client trait in this module is provider-agnostic — requests go through
/// a shared `prepare_request` step that resolves the provider from the model prefix,
/// strips that prefix, and applies [`Provider::transform_request`] /
/// [`Provider::transform_response`]. **This trait does none of that.** The
/// implementation sends [`CreateResponseRequest`] to
/// `{base_url}` + [`Provider::responses_path`] verbatim and deserializes the reply
/// straight into [`ResponseObject`], so both directions assume the OpenAI Responses
/// wire format.
///
/// Consequently:
///
/// - No provider overrides [`Provider::responses_path`], and no entry in
///   `schemas/providers.json` declares a `responses` endpoint — provider Responses
///   support is not modeled anywhere in the registry.
/// - Requests stay pinned to the provider the client was constructed with. A
///   `provider/model` prefix in [`CreateResponseRequest::model`] is neither stripped
///   nor used for routing; it travels to the wire intact.
/// - Providers that rewrite the request or normalize the response for chat (Anthropic,
///   Vertex, Bedrock, Cohere, Google AI) get no such chance here. Their transforms are
///   written against the Chat Completions body shape and would corrupt a Responses body,
///   which is why they are not applied rather than merely forgotten.
/// - Bedrock, Vertex, and Google AI embed the model into `build_url`, but only for the
///   `chat/completions` and `embeddings` paths they special-case; `responses_path()` falls
///   through to their plain `{base}{endpoint_path}` branch, so the empty model passed here is
///   harmless for them. Azure embeds the model unconditionally, so the empty model yields
///   `.../openai/deployments//responses` — an empty path segment. The four call sites below
///   detect that shape via `reject_malformed_responses_url` and fail fast with
///   [`LiterLlmError::EndpointNotSupported`] instead of sending it.
///
/// Use these methods with OpenAI, or with a gateway that serves OpenAI's `/responses`
/// contract natively. For cross-provider work use [`LlmClient::chat`].
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub trait ResponseClient: Send + Sync {
    /// Create a new response.
    fn create_response(&self, req: CreateResponseRequest) -> BoxFuture<'_, Result<ResponseObject>>;

    /// Create a new response and stream its SSE events as they arrive.
    ///
    /// Returns a stream of `ResponseStreamEvent` items. The stream terminates
    /// when a terminal event (`response.completed`, `response.incomplete`, or
    /// `response.failed`) is yielded and the underlying connection closes.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`create_response`](Self::create_response).
    /// Stream errors are returned as `Err` items in the stream itself.
    fn create_response_stream(
        &self,
        req: CreateResponseRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ResponseStreamEvent>>>>;

    /// Retrieve a response by ID.
    fn retrieve_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>>;

    /// Cancel an in-progress response.
    fn cancel_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>>;
}

/// OpenAI Responses API operations (create, retrieve, cancel) (WASM variant).
///
/// # Provider support: OpenAI only
///
/// See the non-WASM `ResponseClient` for the full rationale. In short: this path sends
/// `CreateResponseRequest` verbatim and parses the reply as the OpenAI Responses wire
/// format. It applies no provider request/response transform, performs no model-prefix
/// routing, and no provider in `schemas/providers.json` declares a `responses`
/// endpoint. Use the chat path for cross-provider work.
#[cfg(target_arch = "wasm32")]
#[cfg_attr(alef, alef(skip))]
pub trait ResponseClient {
    /// Create a new response.
    fn create_response(&self, req: CreateResponseRequest) -> BoxFuture<'_, Result<ResponseObject>>;

    /// Create a new response and stream its SSE events as they arrive.
    ///
    /// Returns a stream of `ResponseStreamEvent` items. The stream terminates
    /// when a terminal event (`response.completed`, `response.incomplete`, or
    /// `response.failed`) is yielded and the underlying connection closes.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`create_response`](Self::create_response).
    /// Stream errors are returned as `Err` items in the stream itself.
    fn create_response_stream(
        &self,
        req: CreateResponseRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ResponseStreamEvent>>>>;

    /// Retrieve a response by ID.
    fn retrieve_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>>;

    /// Cancel an in-progress response.
    fn cancel_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>>;
}

/// Default client implementation backed by `reqwest`.
///
/// Sends requests to 165 LLM providers with automatic provider detection
/// and per-request routing. The provider is resolved at construction time
/// from `model_hint` (or defaults to OpenAI), but individual requests can
/// override the provider via model name prefix (e.g. `"anthropic/claude-3-5-sonnet"`
/// routes to Anthropic regardless of construction-time setting).
///
/// When the model prefix does not match any known provider, the construction-time
/// provider is used as the fallback. This enables seamless migration between
/// providers by changing only the model name.
///
/// The provider is stored behind an [`Arc`] so it can be shared cheaply into
/// async closures and streaming tasks. Pre-computed auth headers and extra
/// headers are cached at construction to avoid redundant encoding on every request.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
#[derive(Clone)]
pub struct DefaultClient {
    config: ClientConfig,
    http: reqwest::Client,
    /// Provider resolved at construction; shared via Arc so streaming closures
    /// can capture an owned reference without requiring `unsafe`.
    provider: Arc<dyn Provider>,
    /// Pre-computed auth header `(name, value)` — avoids `format!("Bearer {key}")`
    /// on every request.  `None` when the provider requires no authentication.
    cached_auth_header: Option<(String, String)>,
    /// Pre-computed static extra headers — avoids converting `&'static str` pairs
    /// to `(String, String)` on every request.
    cached_extra_headers: Vec<(String, String)>,
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl DefaultClient {
    fn response_read_options(&self) -> http::request::ResponseReadOptions {
        http::request::ResponseReadOptions {
            max_retries: self.config.max_retries,
            max_response_bytes: self.config.max_response_bytes,
        }
    }

    /// Build a client.
    ///
    /// Constructs an HTTP client with the given configuration and provider hint.
    /// If `model_hint` is provided, its prefix determines the default provider
    /// (e.g. `"groq/llama3-70b"` selects Groq; `"claude-3-5-sonnet"` defaults to OpenAI).
    /// Pass `None` to use OpenAI as the default. The hint does not constrain
    /// per-request routing — individual requests can override the provider via
    /// their own model prefix.
    ///
    /// When `config.load_env` is true (the default), and no API key was provided,
    /// the client reads the provider's designated environment variable
    /// (e.g. `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`) at construction time.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::Authentication` if `load_env` is enabled, no explicit
    /// API key was provided, and the provider's environment variable is unset or empty.
    /// Returns `LiterLlmError::InvalidHeader` if pre-validated headers somehow
    /// become invalid during client construction (extremely rare; indicates a bug).
    /// Returns `LiterLlmError::Http` if the underlying HTTP client cannot be constructed.
    pub fn new(config: ClientConfig, model_hint: Option<&str>) -> Result<Self> {
        config::validate_response_limit(config.max_response_bytes)?;
        let provider = build_provider(&config, model_hint);
        provider.validate()?;

        #[cfg(not(target_arch = "wasm32"))]
        let mut config = config;
        #[cfg(not(target_arch = "wasm32"))]
        if config.load_env
            && config.api_key.expose_secret().is_empty()
            && let Some(env_var_name) = provider.env_var()
        {
            match std::env::var(env_var_name) {
                Ok(val) if !val.is_empty() => {
                    config.api_key = secrecy::SecretString::from(val);
                }
                _ => {
                    return Err(LiterLlmError::Authentication {
                        message: format!("no API key provided and environment variable {env_var_name} is not set"),
                        status: 401,
                    });
                }
            }
        }

        // ~keep Auto-install Vertex ADC only when Vertex AI has no explicit api_key or credential_provider.
        // ~keep Explicit tokens and credential providers stay authoritative over Workload Identity defaults.
        #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
        if config.credential_provider.is_none()
            && config.api_key.expose_secret().is_empty()
            && provider.name() == "vertex_ai"
        {
            config.credential_provider = Some(Arc::new(crate::auth::vertex_adc::VertexAdcCredentialProvider::new()));
        }

        let mut header_map = reqwest::header::HeaderMap::new();
        for (k, v) in config.headers() {
            let name =
                reqwest::header::HeaderName::from_bytes(k.as_bytes()).map_err(|_| LiterLlmError::InvalidHeader {
                    name: k.clone(),
                    reason: "pre-validated header name became invalid".into(),
                })?;
            let val = reqwest::header::HeaderValue::from_str(v).map_err(|_| LiterLlmError::InvalidHeader {
                name: k.clone(),
                reason: "pre-validated header value became invalid".into(),
            })?;
            header_map.insert(name, val);
        }

        let http = {
            #[cfg(feature = "native-http")]
            crate::ensure_crypto_provider();
            let builder = reqwest::Client::builder().default_headers(header_map);
            #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
            let builder = crate::provider::configure_outbound_client_builder(builder, config.transport.dns_cache_ttl);
            #[cfg(not(target_arch = "wasm32"))]
            let builder = builder.timeout(config.timeout);
            #[cfg(not(target_arch = "wasm32"))]
            let builder = config.transport.apply_to_builder(builder);
            builder.build().map_err(LiterLlmError::from)?
        };

        let cached_auth_header = provider
            .auth_header(config.api_key.expose_secret())
            .map(|(name, value)| (name.into_owned(), value.into_owned()));

        let cached_extra_headers = provider
            .extra_headers()
            .iter()
            .map(|&(name, value)| (name.to_owned(), value.to_owned()))
            .collect();

        Ok(Self {
            config,
            http,
            provider,
            cached_auth_header,
            cached_extra_headers,
        })
    }

    /// Resolve the provider for a specific request based on the model string.
    ///
    /// If the model prefix clearly identifies a provider that differs from the
    /// construction-time default, the detected provider is returned.  Otherwise
    /// the construction-time provider is reused (zero allocation).
    fn resolve_provider_for_model(&self, model: &str) -> Arc<dyn Provider> {
        // ~keep A base_url override pins the construction-time provider for mock/proxy endpoints.
        if self.config.base_url.is_some() {
            return Arc::clone(&self.provider);
        }
        // ~keep Keep the construction-time provider when it already matches the model.
        if self.provider.matches_model(model) {
            return Arc::clone(&self.provider);
        }
        if model.starts_with("bedrock/") {
            return build_bedrock_provider(&self.config);
        }
        if let Some(detected) = provider::detect_provider(model) {
            return Arc::from(detected);
        }
        Arc::clone(&self.provider)
    }

    /// Compute the auth header for a given provider (potentially different from
    /// the construction-time cached one).
    async fn resolve_auth_header_for_provider(&self, prov: &dyn Provider) -> Result<Option<(String, String)>> {
        if let Some(ref cp) = self.config.credential_provider {
            let credential = cp.resolve().await?;
            match credential {
                Credential::BearerToken(token) => Ok(Some((
                    "Authorization".to_owned(),
                    format!("Bearer {}", token.expose_secret()),
                ))),
                Credential::AwsCredentials { .. } => Ok(None),
            }
        } else {
            // ~keep Auth headers must be recomputed after per-request provider resolution.
            Ok(prov
                .auth_header(self.config.api_key.expose_secret())
                .map(|(name, value)| (name.into_owned(), value.into_owned())))
        }
    }

    /// Build the combined header list for a request using a specific provider.
    ///
    /// # Errors
    ///
    /// Propagates a signing failure from [`Provider::signing_headers`] (e.g. an
    /// internal Bedrock SigV4 error) instead of sending an unsigned request.
    fn all_headers_for_provider(
        &self,
        prov: &dyn Provider,
        method: &str,
        url: &str,
        body_json: &serde_json::Value,
        body_bytes: &[u8],
    ) -> Result<Vec<(String, String)>> {
        let mut headers = prov.signing_headers(method, url, body_bytes)?;
        headers.extend(
            prov.extra_headers()
                .iter()
                .map(|&(name, value)| (name.to_owned(), value.to_owned())),
        );
        headers.extend(prov.dynamic_headers(body_json));
        Ok(headers)
    }

    /// Shared helper: resolve the per-request provider, build the URL, strip
    /// model prefix from the request body, set the `stream` flag, apply provider
    /// transform, and return everything needed to fire a request.
    ///
    /// `endpoint_fn` receives the resolved provider and returns the endpoint
    /// path (e.g. `|p| p.chat_completions_path()`), ensuring the path comes
    /// from the correct provider when per-request routing overrides the default.
    ///
    /// `stream` is inserted into the body **before** `transform_request` runs,
    /// so providers can inspect the final body state in one pass.
    fn prepare_request(
        &self,
        serializable: &impl serde::Serialize,
        endpoint_fn: impl FnOnce(&dyn Provider) -> &str,
        model: &str,
        stream: Option<bool>,
    ) -> Result<PreparedRequest> {
        if model.is_empty() {
            return Err(LiterLlmError::BadRequest {
                message: "model must not be empty".into(),
                status: 400,
            });
        }

        let prov = self.resolve_provider_for_model(model);
        let bare_model = prov.strip_model_prefix(model).to_owned();
        // ~keep Providers such as Azure and Bedrock embed model/deployment identifiers in URLs.
        let endpoint_path = endpoint_fn(prov.as_ref());
        let url = prov.build_url(endpoint_path, &bare_model);

        let mut body = serde_json::to_value(serializable)?;
        if let Some(obj) = body.as_object_mut() {
            obj.insert("model".into(), serde_json::Value::String(bare_model));
            if let Some(s) = stream {
                obj.insert("stream".into(), serde_json::Value::Bool(s));
            }
        }
        prov.transform_request(&mut body)?;
        // ~keep Whether the provider's wire format carries `stream` at all is the provider's
        // decision, recorded here by whether the key survived `transform_request`. Providers that
        // rebuild the body into a native format (Vertex/Gemini `generateContent`, Bedrock
        // `converse`) drop it deliberately -- those APIs select streaming by endpoint and reject
        // the field outright ("Unknown name \"stream\": Cannot find field", HTTP 400). Providers
        // whose format does carry it (OpenAI-compatible, Anthropic, Cohere) keep it.
        let provider_wire_carries_stream = body.get("stream").is_some();
        merge_extra_body(&mut body);
        // ~keep extra_body must never override `stream`: the transport path
        // (post_json_raw vs post_stream) is chosen by the calling method, not the
        // body, so a mismatched wire flag would desync request from response parsing.
        // Restoring it is therefore scoped to providers that had it after the transform.
        if provider_wire_carries_stream
            && let Some(s) = stream
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert("stream".into(), serde_json::Value::Bool(s));
        }

        // ~keep Serialize once so signing bytes and request body bytes are identical.
        let body_bytes = bytes::Bytes::from(serde_json::to_vec(&body)?);

        Ok(PreparedRequest {
            url,
            provider: prov,
            body_json: body,
            body_bytes,
        })
    }

    /// Resolve the auth header for a request using the construction-time provider.
    ///
    /// Uses the pre-computed cached auth header for efficiency.  When a
    /// [`CredentialProvider`] is configured, it is called to obtain a fresh
    /// credential which overrides the cached header.
    async fn resolve_auth_header(&self) -> Result<Option<(String, String)>> {
        if let Some(ref cp) = self.config.credential_provider {
            let credential = cp.resolve().await?;
            match credential {
                Credential::BearerToken(token) => Ok(Some((
                    "Authorization".to_owned(),
                    format!("Bearer {}", token.expose_secret()),
                ))),
                Credential::AwsCredentials { .. } => Ok(None),
            }
        } else {
            Ok(self.cached_auth_header.clone())
        }
    }

    /// Build the combined header list using the construction-time provider.
    ///
    /// Uses pre-computed cached extra headers for efficiency.
    ///
    /// # Errors
    ///
    /// Propagates a signing failure from [`Provider::signing_headers`] (e.g. an
    /// internal Bedrock SigV4 error) instead of sending an unsigned request.
    fn all_headers(
        &self,
        method: &str,
        url: &str,
        body_json: &serde_json::Value,
        body_bytes: &[u8],
    ) -> Result<Vec<(String, String)>> {
        let mut headers = self.provider.signing_headers(method, url, body_bytes)?;
        headers.extend(self.cached_extra_headers.iter().cloned());
        headers.extend(self.provider.dynamic_headers(body_json));
        Ok(headers)
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
/// Resolve the provider to use for all requests on this client.
///
/// Priority:
/// 1. Explicit `base_url` in config:
///    - If `model_hint` identifies a provider with a non-standard URL format
///      (Azure embeds the deployment name and `?api-version=…`), construct
///      that provider with the override (issue #83).
///    - Otherwise, treat the override as a generic OpenAI-compatible endpoint
///      (LM Studio, Ollama, vLLM, etc.).
/// 2. `model_hint` -> auto-detect by model name prefix.
/// 3. Default -> OpenAI.
fn build_provider(config: &ClientConfig, model_hint: Option<&str>) -> Arc<dyn Provider> {
    if let Some(ref base_url) = config.base_url {
        if let Some(model) = model_hint
            && model.starts_with("azure/")
        {
            return Arc::new(provider::azure::AzureProvider::with_base_url(base_url.clone()));
        }
        if let Some(model) = model_hint
            && model.starts_with("anthropic/")
        {
            return Arc::new(provider::anthropic::AnthropicProvider::with_base_url(base_url.clone()));
        }
        return Arc::new(OpenAiCompatibleProvider {
            name: "custom".into(),
            base_url: base_url.clone(),
            env_var: None,
            model_prefixes: vec![],
        });
    }

    if let Some(model) = model_hint {
        if model.starts_with("bedrock/") {
            return build_bedrock_provider(config);
        }
        if let Some(p) = provider::detect_provider(model) {
            return Arc::from(p);
        }
    }

    Arc::new(OpenAiProvider)
}

/// Build a [`provider::bedrock::BedrockProvider`] from `config`, threading
/// through any explicit region, cross-region prefix, and credentials set on
/// [`ClientConfig`]. Falls back to the environment for anything left unset,
/// matching [`provider::bedrock::BedrockProvider::from_env`].
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn build_bedrock_provider(config: &ClientConfig) -> Arc<dyn Provider> {
    Arc::new(provider::bedrock::BedrockProvider::from_config(
        config.bedrock_region.clone(),
        config.bedrock_cross_region_prefix.clone(),
        config.bedrock_access_key_id.clone(),
        config.bedrock_secret_access_key.clone(),
        config.bedrock_session_token.clone(),
    ))
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl LlmClient for DefaultClient {
    fn chat(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>> {
        Box::pin(async move {
            // ~keep Pass stream=false so providers can transform non-streaming chat correctly.
            let prepared = self.prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))?;

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<ChatCompletionResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn chat_stream(
        &self,
        req: ChatCompletionRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>> {
        Box::pin(async move {
            // ~keep Prepare first for validation/transforms, then override with provider stream URL.
            let prepared = self.prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(true))?;

            // ~keep Streaming endpoints can differ from non-streaming provider URLs.
            let bare_model = prepared.provider.strip_model_prefix(&req.model);
            let url = prepared
                .provider
                .build_stream_url(prepared.provider.chat_completions_path(), bare_model);

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            match prepared.provider.stream_format() {
                provider::StreamFormat::Sse => {
                    let provider = Arc::clone(&prepared.provider);
                    let parse_event = move |data: &str| provider.parse_stream_event(data);
                    let stream = http::streaming::post_stream_bounded(
                        &self.http,
                        http::request::StreamingPost {
                            url: &url,
                            auth_header: auth,
                            extra_headers: &extra,
                            body: prepared.body_bytes,
                        },
                        parse_event,
                        self.response_read_options(),
                    )
                    .await?;
                    Ok(stream)
                }
                provider::StreamFormat::AwsEventStream => {
                    let stream = http::eventstream::post_eventstream_bounded(
                        &self.http,
                        http::request::StreamingPost {
                            url: &url,
                            auth_header: auth,
                            extra_headers: &extra,
                            body: prepared.body_bytes,
                        },
                        provider::bedrock::parse_bedrock_stream_event,
                        self.response_read_options(),
                    )
                    .await?;
                    Ok(stream)
                }
            }
        })
    }

    fn embed(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>> {
        Box::pin(async move {
            // ~keep Embeddings have no stream flag; passing None prevents inserting one.
            let prepared = self.prepare_request(&req, |p| p.embeddings_path(), &req.model, None)?;

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<EmbeddingResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn list_models(&self) -> BoxFuture<'_, Result<ModelsListResponse>> {
        Box::pin(async move {
            // ~keep list_models has no model string, so use the construction-time provider.
            let url = self.provider.build_url(self.provider.models_path(), "");
            let auth_header = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let mut raw = http::request::get_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                self.config.max_retries,
                self.config.max_response_bytes,
            )
            .await?;
            self.provider.transform_response(&mut raw)?;
            serde_json::from_value::<ModelsListResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn image_generate(&self, req: CreateImageRequest) -> BoxFuture<'_, Result<ImagesResponse>> {
        Box::pin(async move {
            let model = req.model.as_deref().unwrap_or_default();
            let prepared = self.prepare_request(&req, |p| p.image_generations_path(), model, None)?;

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<ImagesResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn speech(&self, req: CreateSpeechRequest) -> BoxFuture<'_, Result<bytes::Bytes>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.audio_speech_path(), &req.model, None)?;

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            http::request::post_binary_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await
        })
    }

    fn transcribe(&self, req: CreateTranscriptionRequest) -> BoxFuture<'_, Result<TranscriptionResponse>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.audio_transcriptions_path(), &req.model, None)?;

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<TranscriptionResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn moderate(&self, req: ModerationRequest) -> BoxFuture<'_, Result<ModerationResponse>> {
        Box::pin(async move {
            let model = req.model.as_deref().unwrap_or_default();
            let prepared = self.prepare_request(&req, |p| p.moderations_path(), model, None)?;

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<ModerationResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn rerank(&self, req: RerankRequest) -> BoxFuture<'_, Result<RerankResponse>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.rerank_path(), &req.model, None)?;

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<RerankResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn search(&self, req: SearchRequest) -> BoxFuture<'_, Result<SearchResponse>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.search_path(), &req.model, None)?;

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<SearchResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn ocr(&self, req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.ocr_path(), &req.model, None)?;

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<OcrResponse>(raw).map_err(LiterLlmError::from)
        })
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl LlmClientRaw for DefaultClient {
    fn chat_raw(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<RawExchange<ChatCompletionResponse>>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))?;
            let raw_request = prepared.body_json.clone();

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let data = serde_json::from_value::<ChatCompletionResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn chat_stream_raw(
        &self,
        req: ChatCompletionRequest,
    ) -> BoxFuture<'_, Result<RawStreamExchange<BoxStream<'static, Result<ChatCompletionChunk>>>>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(true))?;
            let raw_request = prepared.body_json.clone();

            let bare_model = prepared.provider.strip_model_prefix(&req.model);
            let url = prepared
                .provider
                .build_stream_url(prepared.provider.chat_completions_path(), bare_model);

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let stream = match prepared.provider.stream_format() {
                provider::StreamFormat::Sse => {
                    let provider = Arc::clone(&prepared.provider);
                    let parse_event = move |data: &str| provider.parse_stream_event(data);
                    http::streaming::post_stream_bounded(
                        &self.http,
                        http::request::StreamingPost {
                            url: &url,
                            auth_header: auth,
                            extra_headers: &extra,
                            body: prepared.body_bytes,
                        },
                        parse_event,
                        self.response_read_options(),
                    )
                    .await?
                }
                provider::StreamFormat::AwsEventStream => {
                    http::eventstream::post_eventstream_bounded(
                        &self.http,
                        http::request::StreamingPost {
                            url: &url,
                            auth_header: auth,
                            extra_headers: &extra,
                            body: prepared.body_bytes,
                        },
                        provider::bedrock::parse_bedrock_stream_event,
                        self.response_read_options(),
                    )
                    .await?
                }
            };

            Ok(RawStreamExchange { stream, raw_request })
        })
    }

    fn embed_raw(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<RawExchange<EmbeddingResponse>>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.embeddings_path(), &req.model, None)?;
            let raw_request = prepared.body_json.clone();

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let data = serde_json::from_value::<EmbeddingResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn image_generate_raw(&self, req: CreateImageRequest) -> BoxFuture<'_, Result<RawExchange<ImagesResponse>>> {
        Box::pin(async move {
            let model = req.model.as_deref().unwrap_or_default();
            let prepared = self.prepare_request(&req, |p| p.image_generations_path(), model, None)?;
            let raw_request = prepared.body_json.clone();

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let data = serde_json::from_value::<ImagesResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn transcribe_raw(
        &self,
        req: CreateTranscriptionRequest,
    ) -> BoxFuture<'_, Result<RawExchange<TranscriptionResponse>>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.audio_transcriptions_path(), &req.model, None)?;
            let raw_request = prepared.body_json.clone();

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let data = serde_json::from_value::<TranscriptionResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn moderate_raw(&self, req: ModerationRequest) -> BoxFuture<'_, Result<RawExchange<ModerationResponse>>> {
        Box::pin(async move {
            let model = req.model.as_deref().unwrap_or_default();
            let prepared = self.prepare_request(&req, |p| p.moderations_path(), model, None)?;
            let raw_request = prepared.body_json.clone();

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let data = serde_json::from_value::<ModerationResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn rerank_raw(&self, req: RerankRequest) -> BoxFuture<'_, Result<RawExchange<RerankResponse>>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.rerank_path(), &req.model, None)?;
            let raw_request = prepared.body_json.clone();

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let data = serde_json::from_value::<RerankResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn search_raw(&self, req: SearchRequest) -> BoxFuture<'_, Result<RawExchange<SearchResponse>>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.search_path(), &req.model, None)?;
            let raw_request = prepared.body_json.clone();

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let data = serde_json::from_value::<SearchResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn ocr_raw(&self, req: OcrRequest) -> BoxFuture<'_, Result<RawExchange<OcrResponse>>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.ocr_path(), &req.model, None)?;
            let raw_request = prepared.body_json.clone();

            let auth_header = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let data = serde_json::from_value::<OcrResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl FileClient for DefaultClient {
    fn create_file(&self, req: CreateFileRequest) -> BoxFuture<'_, Result<FileObject>> {
        Box::pin(async move {
            let url = self.provider.build_url(self.provider.files_path(), "");
            let auth_header = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers("POST", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            use base64::Engine;
            let file_bytes = base64::engine::general_purpose::STANDARD
                .decode(&req.file)
                .map_err(|e| LiterLlmError::BadRequest {
                    message: format!("invalid base64 file data: {e}"),
                    status: 400,
                })?;

            let filename = req.filename.unwrap_or_else(|| "upload".to_owned());
            let file_part = reqwest::multipart::Part::bytes(file_bytes).file_name(filename);
            let purpose_str = serde_json::to_value(&req.purpose)?
                .as_str()
                .unwrap_or_default()
                .to_owned();
            let form = reqwest::multipart::Form::new()
                .part("file", file_part)
                .text("purpose", purpose_str);

            let raw = http::request::post_multipart_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                form,
                self.config.max_response_bytes,
            )
            .await?;
            serde_json::from_value::<FileObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn retrieve_file(&self, file_id: &str) -> BoxFuture<'_, Result<FileObject>> {
        let file_id = file_id.to_owned();
        Box::pin(async move {
            let url = format!(
                "{}/{}",
                self.provider.build_url(self.provider.files_path(), ""),
                file_id
            );
            let auth_header = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                self.config.max_retries,
                self.config.max_response_bytes,
            )
            .await?;
            serde_json::from_value::<FileObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn delete_file(&self, file_id: &str) -> BoxFuture<'_, Result<DeleteResponse>> {
        let file_id = file_id.to_owned();
        Box::pin(async move {
            let url = format!(
                "{}/{}",
                self.provider.build_url(self.provider.files_path(), ""),
                file_id
            );
            let auth_header = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers("DELETE", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::delete_json_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                self.config.max_retries,
                self.config.max_response_bytes,
            )
            .await?;
            serde_json::from_value::<DeleteResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn list_files(&self, query: Option<FileListQuery>) -> BoxFuture<'_, Result<FileListResponse>> {
        Box::pin(async move {
            let base_url = self.provider.build_url(self.provider.files_path(), "");
            let url = if let Some(ref q) = query {
                let mut params = Vec::new();
                if let Some(ref purpose) = q.purpose {
                    params.push(format!("purpose={purpose}"));
                }
                if let Some(limit) = q.limit {
                    params.push(format!("limit={limit}"));
                }
                if let Some(ref after) = q.after {
                    params.push(format!("after={after}"));
                }
                if params.is_empty() {
                    base_url
                } else {
                    format!("{base_url}?{}", params.join("&"))
                }
            } else {
                base_url
            };
            let auth_header = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                self.config.max_retries,
                self.config.max_response_bytes,
            )
            .await?;
            serde_json::from_value::<FileListResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn file_content(&self, file_id: &str) -> BoxFuture<'_, Result<bytes::Bytes>> {
        let file_id = file_id.to_owned();
        Box::pin(async move {
            let url = format!(
                "{}/{}/content",
                self.provider.build_url(self.provider.files_path(), ""),
                file_id
            );
            let auth_header = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            http::request::get_binary_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                self.config.max_retries,
                self.config.max_response_bytes,
            )
            .await
        })
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl BatchClient for DefaultClient {
    fn create_batch(&self, req: CreateBatchRequest) -> BoxFuture<'_, Result<BatchObject>> {
        Box::pin(async move {
            let url = self.provider.build_url(self.provider.batches_path(), "");
            let body_bytes = bytes::Bytes::from(serde_json::to_vec(&req)?);
            let body_json = serde_json::to_value(&req)?;

            let auth_header = self.resolve_auth_header().await?;
            let all_headers = self.all_headers("POST", &url, &body_json, &body_bytes)?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let raw = http::request::post_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                body_bytes,
                self.response_read_options(),
            )
            .await?;
            serde_json::from_value::<BatchObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn retrieve_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>> {
        let batch_id = batch_id.to_owned();
        Box::pin(async move {
            let url = format!(
                "{}/{}",
                self.provider.build_url(self.provider.batches_path(), ""),
                batch_id
            );
            let auth_header = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                self.config.max_retries,
                self.config.max_response_bytes,
            )
            .await?;
            serde_json::from_value::<BatchObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn list_batches(&self, query: Option<BatchListQuery>) -> BoxFuture<'_, Result<BatchListResponse>> {
        Box::pin(async move {
            let base_url = self.provider.build_url(self.provider.batches_path(), "");
            let url = if let Some(ref q) = query {
                let mut params = Vec::new();
                if let Some(limit) = q.limit {
                    params.push(format!("limit={limit}"));
                }
                if let Some(ref after) = q.after {
                    params.push(format!("after={after}"));
                }
                if params.is_empty() {
                    base_url
                } else {
                    format!("{base_url}?{}", params.join("&"))
                }
            } else {
                base_url
            };
            let auth_header = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                self.config.max_retries,
                self.config.max_response_bytes,
            )
            .await?;
            serde_json::from_value::<BatchListResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn cancel_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>> {
        let batch_id = batch_id.to_owned();
        Box::pin(async move {
            let url = format!(
                "{}/{}/cancel",
                self.provider.build_url(self.provider.batches_path(), ""),
                batch_id
            );
            let auth_header = self.resolve_auth_header().await?;
            let body_json = serde_json::Value::Null;
            let body_bytes = bytes::Bytes::new();
            let all_headers = self.all_headers("POST", &url, &body_json, &body_bytes)?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let raw = http::request::post_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                body_bytes,
                self.response_read_options(),
            )
            .await?;
            serde_json::from_value::<BatchObject>(raw).map_err(LiterLlmError::from)
        })
    }
}

/// Internal trait for batch retrieval, used to abstract polling logic for testability.
///
/// Method name avoids collision with the inherent `DefaultClient::retrieve_batch`
/// so alef-generated bindings don't need to import this trait into scope.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[doc(hidden)]
#[cfg_attr(alef, alef(skip))]
pub trait BatchRetriever {
    /// Retrieve a batch by ID for polling purposes.
    async fn fetch_batch_for_polling(&self, batch_id: &str) -> Result<BatchObject>;
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl BatchRetriever for DefaultClient {
    async fn fetch_batch_for_polling(&self, batch_id: &str) -> Result<BatchObject> {
        self.retrieve_batch(batch_id).await
    }
}

/// Poll a batch until it reaches a terminal status.
///
/// This is the internal implementation shared by tests and `DefaultClient::wait_for_batch`.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
#[doc(hidden)]
#[cfg_attr(alef, alef(skip))]
pub async fn wait_for_batch_impl<R: BatchRetriever>(
    retriever: &R,
    batch_id: &str,
    config: WaitForBatchConfig,
) -> std::result::Result<BatchObject, BatchWaitError> {
    #[cfg(not(target_arch = "wasm32"))]
    let started = tokio::time::Instant::now();
    #[cfg(target_arch = "wasm32")]
    let started = web_time::Instant::now();
    let mut interval_secs = config.initial_interval_secs;

    loop {
        let batch = retriever.fetch_batch_for_polling(batch_id).await?;

        match batch.status {
            BatchStatus::Completed => return Ok(batch),
            BatchStatus::Failed | BatchStatus::Expired | BatchStatus::Cancelled => {
                return Err(BatchWaitError::Failed { status: batch.status });
            }
            BatchStatus::Validating | BatchStatus::InProgress | BatchStatus::Finalizing | BatchStatus::Cancelling => {
                if let Some(timeout_secs) = config.timeout_secs {
                    let timeout = Duration::from_secs_f64(timeout_secs);
                    if started.elapsed() >= timeout {
                        return Err(BatchWaitError::Timeout { timeout_secs });
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(Duration::from_secs_f64(interval_secs)).await;
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::sleep(Duration::from_secs_f64(interval_secs)).await;
                let next =
                    (interval_secs as f32 * config.backoff_multiplier).min(config.max_interval_secs as f32) as f64;
                interval_secs = next;
            }
        }
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl DefaultClient {
    /// Poll a batch until it reaches a terminal status (Completed, Failed, Expired, Cancelled).
    ///
    /// Uses exponential backoff with configurable initial interval, maximum interval, and backoff multiplier.
    /// Optionally supports a timeout that aborts polling if exceeded.
    ///
    /// # Errors
    ///
    /// Returns `BatchWaitError::Failed` if the batch reaches a failure terminal status.
    /// Returns `BatchWaitError::Timeout` if the configured timeout is exceeded.
    /// Returns `BatchWaitError::Client` for underlying client errors.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use liter_llm::client::{DefaultClient, ClientConfig, WaitForBatchConfig};
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = DefaultClient::new(ClientConfig::new("api-key"), None)?;
    /// let batch = client.wait_for_batch("b-123", WaitForBatchConfig::default()).await?;
    /// println!("Batch completed: {:?}", batch.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_batch(
        &self,
        batch_id: &str,
        config: WaitForBatchConfig,
    ) -> std::result::Result<BatchObject, BatchWaitError> {
        wait_for_batch_impl(self, batch_id, config).await
    }
}

/// Parse a single Responses API SSE `data:` payload into a [`ResponseStreamEvent`].
///
/// Unlike [`Provider::parse_stream_event`], this has no provider dispatch: it always
/// decodes the OpenAI Responses `response.*` event vocabulary. That is not evidence
/// that the format is portable — the Responses path is OpenAI-only (see
/// [`ResponseClient`](trait@ResponseClient)), so there is no second format to dispatch
/// on. Unrecognized `type` values decode into
/// [`ResponseStreamEvent::Unknown`] rather than failing (see
/// `ResponseStreamEvent`'s `Deserialize` impl), so this only returns `Err`
/// for payloads that are not valid JSON at all.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn parse_response_stream_event(data: &str) -> Result<Option<ResponseStreamEvent>> {
    serde_json::from_str::<ResponseStreamEvent>(data)
        .map(Some)
        .map_err(|e| LiterLlmError::Streaming {
            message: format!("failed to parse Responses SSE data: {e}"),
        })
}

/// Reject a Responses URL that contains an empty path segment (`//`).
///
/// The four `ResponseClient` call sites pass `""` as the model to
/// [`Provider::build_url`] / [`Provider::build_stream_url`], because none of `create_response`,
/// `retrieve_response`, or `cancel_response` has a model available to give the URL-embedding
/// providers (see the `ResponseClient` trait doc). Most providers' `build_url` ignores the model
/// for the `/responses` path — it only fires for `chat/completions` and `embeddings` — so an
/// empty model is harmless there. Azure's `build_url` embeds the model unconditionally, so an
/// empty model turns into an empty path segment: `.../openai/deployments//responses`.
///
/// This checks the URL shape rather than `provider.name()` on purpose: naming a specific
/// provider would either allow-list `"openai"` and reject supported OpenAI-compatible gateways
/// running under a different provider name, or deny-list `"azure"` and silently miss any future
/// provider with the same URL-embedding bug. Checking for the empty segment catches exactly the
/// malformed case, from whichever provider produces it, and never fires for a provider whose
/// `build_url` output is well-formed. ~keep
///
/// One other configuration can trip this, so do not read the error as "this provider cannot do
/// Responses" without checking: [`ClientConfig::base_url`] is a public field, so a value set
/// directly rather than through [`ClientConfigBuilder::base_url`] or
/// [`ClientConfig::with_base_url`] skips their `trim_end_matches('/')`, and a trailing slash
/// then yields `.../v1//responses` here too. That configuration is already broken for every
/// other endpoint as well, so it should surface long before this check — but if this fires for
/// a provider that ought to work, inspect `base_url` before suspecting the provider. ~keep
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn reject_malformed_responses_url(url: &str, provider_name: &str) -> Result<()> {
    let path = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = path.split_once('/').map_or("", |(_, rest)| rest);
    let path = path.split(['?', '#']).next().unwrap_or("");
    if format!("/{path}").contains("//") {
        return Err(LiterLlmError::EndpointNotSupported {
            endpoint: "responses".into(),
            provider: provider_name.into(),
        });
    }
    Ok(())
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl ResponseClient for DefaultClient {
    fn create_response(&self, req: CreateResponseRequest) -> BoxFuture<'_, Result<ResponseObject>> {
        Box::pin(async move {
            let url = self.provider.build_url(self.provider.responses_path(), "");
            reject_malformed_responses_url(&url, self.provider.name())?;
            let body_json = response_request_body(&req)?;
            let body_bytes = bytes::Bytes::from(serde_json::to_vec(&body_json)?);

            let auth_header = self.resolve_auth_header().await?;
            let all_headers = self.all_headers("POST", &url, &body_json, &body_bytes)?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let raw = http::request::post_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                body_bytes,
                self.response_read_options(),
            )
            .await?;
            serde_json::from_value::<ResponseObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn create_response_stream(
        &self,
        req: CreateResponseRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ResponseStreamEvent>>>> {
        Box::pin(async move {
            // ~keep Force streaming on the wire regardless of what the caller (or extra_body) set.
            let url = self.provider.build_stream_url(self.provider.responses_path(), "");
            reject_malformed_responses_url(&url, self.provider.name())?;
            let body_json = response_stream_request_body(req)?;
            let body_bytes = bytes::Bytes::from(serde_json::to_vec(&body_json)?);

            let auth_header = self.resolve_auth_header().await?;
            let all_headers = self.all_headers("POST", &url, &body_json, &body_bytes)?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            http::streaming::post_stream_bounded(
                &self.http,
                http::request::StreamingPost {
                    url: &url,
                    auth_header: auth,
                    extra_headers: &extra,
                    body: body_bytes,
                },
                parse_response_stream_event,
                self.response_read_options(),
            )
            .await
        })
    }

    fn retrieve_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>> {
        let response_id = response_id.to_owned();
        Box::pin(async move {
            let base_url = self.provider.build_url(self.provider.responses_path(), "");
            reject_malformed_responses_url(&base_url, self.provider.name())?;
            let url = format!("{base_url}/{response_id}");
            let auth_header = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                self.config.max_retries,
                self.config.max_response_bytes,
            )
            .await?;
            serde_json::from_value::<ResponseObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn cancel_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>> {
        let response_id = response_id.to_owned();
        Box::pin(async move {
            let base_url = self.provider.build_url(self.provider.responses_path(), "");
            reject_malformed_responses_url(&base_url, self.provider.name())?;
            let url = format!("{base_url}/{response_id}/cancel");
            let auth_header = self.resolve_auth_header().await?;
            let body_json = serde_json::Value::Null;
            let body_bytes = bytes::Bytes::new();
            let all_headers = self.all_headers("POST", &url, &body_json, &body_bytes)?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let raw = http::request::post_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                body_bytes,
                self.response_read_options(),
            )
            .await?;
            serde_json::from_value::<ResponseObject>(raw).map_err(LiterLlmError::from)
        })
    }
}

#[cfg(all(test, any(feature = "native-http", feature = "wasm-http")))]
mod build_provider_tests {
    use super::*;
    use crate::client::config::ClientConfigBuilder;

    /// Revert line: reverting `reject_malformed_responses_url` in `crates/liter-llm/src/client/
    /// mod.rs` to always return `Ok(())` makes this test fail — it would no longer catch the
    /// empty `/openai/deployments//responses` segment Azure produces when no deployment name
    /// (model) is available, which is the case at all four `ResponseClient` call sites.
    #[test]
    fn reject_malformed_responses_url_rejects_azure_empty_deployment() {
        let provider = provider::azure::AzureProvider::with_base_url("https://resourceA.openai.azure.com");
        let url = provider.build_url(provider.responses_path(), "");

        assert!(
            url.contains("/openai/deployments//responses"),
            "test setup assumption broken, url = {url}"
        );

        let result = reject_malformed_responses_url(&url, provider.name());
        match result {
            Err(LiterLlmError::EndpointNotSupported {
                endpoint,
                provider: rejected_provider,
            }) => {
                assert_eq!(endpoint, "responses");
                assert_eq!(rejected_provider, "azure");
            }
            other => panic!("expected Err(EndpointNotSupported), got {other:?}"),
        }
    }

    /// Azure base URLs that already pin a deployment (`/openai/deployments/{name}`) never reach
    /// the model-embedding branch of `AzureProvider::build_url`, so the produced URL has no empty
    /// segment and must not be rejected.
    #[test]
    fn reject_malformed_responses_url_allows_azure_with_pinned_deployment() {
        let base_url = "https://resourceA.openai.azure.com/openai/deployments/my-gpt4-deployment";
        let provider = provider::azure::AzureProvider::with_base_url(base_url);
        let url = provider.build_url(provider.responses_path(), "");

        assert!(
            !url.contains("deployments//"),
            "test setup assumption broken: pinned-deployment URL should have no empty segment, url = {url}"
        );

        assert!(
            reject_malformed_responses_url(&url, provider.name()).is_ok(),
            "a pinned-deployment Azure URL must not be rejected, url = {url}"
        );
    }

    /// Bedrock, Vertex, and Google AI embed the model into `build_url` only for the
    /// `chat/completions` and `embeddings` paths; `responses_path()` matches neither, so the
    /// empty model passed at the four `ResponseClient` call sites must not be rejected for them.
    #[test]
    fn reject_malformed_responses_url_allows_bedrock_google_ai_and_vertex() {
        let bedrock = provider::bedrock::BedrockProvider::from_config(
            Some("us-east-1".into()),
            None,
            Some("test-access-key".into()),
            Some("test-secret-key".into()),
            None,
        );
        let bedrock_url = bedrock.build_url(bedrock.responses_path(), "");
        assert!(
            reject_malformed_responses_url(&bedrock_url, bedrock.name()).is_ok(),
            "bedrock url = {bedrock_url}"
        );

        let google_ai = provider::google_ai::GoogleAiProvider;
        let google_ai_url = google_ai.build_url(google_ai.responses_path(), "");
        assert!(
            reject_malformed_responses_url(&google_ai_url, google_ai.name()).is_ok(),
            "google ai url = {google_ai_url}"
        );

        let vertex = provider::vertex::VertexAiProvider::new("my-project", "us-central1");
        let vertex_url = vertex.build_url(vertex.responses_path(), "");
        assert!(
            reject_malformed_responses_url(&vertex_url, vertex.name()).is_ok(),
            "vertex url = {vertex_url}"
        );
    }

    #[test]
    fn azure_model_with_per_model_base_url_uses_azure_provider() {
        let config = ClientConfigBuilder::new("test-key")
            .base_url("https://resourceA.cognitiveservices.azure.com")
            .build();
        let p = build_provider(&config, Some("azure/gpt-5-mini"));
        assert_eq!(p.name(), "azure");
        let url = p.build_url("/chat/completions", "gpt-5-mini");
        assert!(
            url.starts_with("https://resourceA.cognitiveservices.azure.com/openai/deployments/gpt-5-mini/chat/completions?api-version="),
            "url = {url}"
        );
    }

    #[test]
    fn non_azure_model_with_base_url_uses_openai_compatible() {
        let config = ClientConfigBuilder::new("test-key")
            .base_url("http://localhost:11434/v1")
            .build();
        let p = build_provider(&config, Some("llama3.1:8b"));
        assert_eq!(p.name(), "custom");
        let url = p.build_url("/chat/completions", "llama3.1:8b");
        assert_eq!(url, "http://localhost:11434/v1/chat/completions");
    }

    #[test]
    fn no_base_url_falls_through_to_detect_provider() {
        let config = ClientConfigBuilder::new("test-key").build();
        let p = build_provider(&config, Some("azure/gpt-4o"));
        assert_eq!(p.name(), "azure");
    }

    #[test]
    fn anthropic_model_with_per_model_base_url_uses_anthropic_provider() {
        let config = ClientConfigBuilder::new("test-key")
            .base_url("https://proxy.internal/anthropic/")
            .build();
        let p = build_provider(&config, Some("anthropic/claude-3-5-sonnet-20241022"));
        assert_eq!(p.name(), "anthropic");
        assert_eq!(p.base_url(), "https://proxy.internal/anthropic");
    }

    #[test]
    fn default_anthropic_provider_uses_official_base_url() {
        assert_eq!(
            provider::anthropic::AnthropicProvider::new().base_url(),
            "https://api.anthropic.com/v1"
        );
        assert_eq!(
            provider::anthropic::AnthropicProvider::default().base_url(),
            "https://api.anthropic.com/v1"
        );
    }

    #[test]
    fn extra_body_object_is_merged_and_overrides_existing_key() {
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some("gpt-4"))
            .expect("client construction should succeed");
        let req = ChatCompletionRequest {
            model: "gpt-4".into(),
            messages: vec![],
            extra_body: Some(serde_json::json!({
                "thinking": {"type": "enabled"},
                "model": "extra-body-override"
            })),
            ..Default::default()
        };

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))
            .expect("prepare_request should not fail");

        assert!(
            prepared.body_json.get("extra_body").is_none(),
            "extra_body key must be removed from the final body"
        );
        assert_eq!(prepared.body_json["thinking"], serde_json::json!({"type": "enabled"}));
        assert_eq!(
            prepared.body_json["model"], "extra-body-override",
            "extra_body keys must override identically named top-level keys"
        );
    }

    #[test]
    fn extra_body_cannot_override_the_transport_controlled_stream_flag() {
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some("gpt-4"))
            .expect("client construction should succeed");
        let req = ChatCompletionRequest {
            model: "gpt-4".into(),
            messages: vec![],
            extra_body: Some(serde_json::json!({"stream": false})),
            ..Default::default()
        };

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(true))
            .expect("prepare_request should not fail");

        assert_eq!(
            prepared.body_json["stream"],
            serde_json::json!(true),
            "the caller-selected stream flag must win over extra_body"
        );
    }

    #[test]
    fn extra_body_non_object_is_dropped_without_reaching_the_body() {
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some("gpt-4"))
            .expect("client construction should succeed");
        let req = ChatCompletionRequest {
            model: "gpt-4".into(),
            messages: vec![],
            extra_body: Some(serde_json::json!(["not", "an", "object"])),
            ..Default::default()
        };

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))
            .expect("prepare_request should not fail");

        assert!(
            prepared.body_json.get("extra_body").is_none(),
            "non-object extra_body must never reach the wire"
        );
    }

    #[test]
    fn anthropic_provider_leaves_no_extra_body_key_in_final_body() {
        let client = DefaultClient::new(
            ClientConfigBuilder::new("test-key").build(),
            Some("claude-3-5-sonnet-20241022"),
        )
        .expect("client construction should succeed");
        let req = ChatCompletionRequest {
            model: "claude-3-5-sonnet-20241022".into(),
            messages: vec![crate::types::Message::User(crate::types::UserMessage {
                content: crate::types::UserContent::Text("Hi".into()),
                name: None,
            })],
            max_tokens: Some(100),
            extra_body: Some(serde_json::json!({"reasoning_effort": "high"})),
            ..Default::default()
        };

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))
            .expect("prepare_request should not fail");

        assert!(
            prepared.body_json.get("extra_body").is_none(),
            "anthropic's own transform_request already strips extra_body"
        );
        assert_eq!(prepared.body_json["thinking"]["budget_tokens"], 16384);
    }

    fn base_response_request() -> CreateResponseRequest {
        CreateResponseRequest {
            model: "gpt-5".into(),
            input: serde_json::json!("What is the capital of France?"),
            ..Default::default()
        }
    }

    #[test]
    fn responses_extra_body_object_is_merged_and_overrides_existing_key() {
        let mut req = base_response_request();
        req.extra_body = Some(serde_json::json!({
            "reasoning": {"effort": "none"},
            "model": "extra-body-override"
        }));

        let body = response_request_body(&req).expect("body should serialize");

        assert!(
            body.get("extra_body").is_none(),
            "extra_body key must be removed from the final Responses body"
        );
        assert_eq!(body["reasoning"], serde_json::json!({"effort": "none"}));
        assert_eq!(
            body["model"], "extra-body-override",
            "extra_body keys must override identically named top-level keys, matching chat-path semantics"
        );
    }

    #[test]
    fn responses_extra_body_non_object_is_dropped_without_reaching_the_body() {
        let mut req = base_response_request();
        req.extra_body = Some(serde_json::json!(["not", "an", "object"]));

        let body = response_request_body(&req).expect("body should serialize");

        assert!(
            body.get("extra_body").is_none(),
            "non-object extra_body must never reach the wire"
        );
        assert_eq!(
            body["model"], "gpt-5",
            "a dropped extra_body must leave the real fields untouched"
        );
    }

    #[test]
    fn responses_streaming_path_also_merges_extra_body_and_keeps_stream_forced() {
        let mut req = base_response_request();
        req.extra_body = Some(serde_json::json!({
            "reasoning": {"effort": "none"},
            "stream": false
        }));

        let body = response_stream_request_body(req).expect("body should serialize");

        assert!(
            body.get("extra_body").is_none(),
            "extra_body key must be removed from the streaming Responses body too"
        );
        assert_eq!(body["reasoning"], serde_json::json!({"effort": "none"}));
        assert_eq!(
            body["stream"],
            serde_json::json!(true),
            "the streaming transport's stream flag must win over extra_body"
        );
    }

    /// When the resolved provider is Vertex AI and the caller supplied neither
    /// an explicit `api_key` nor a `credential_provider`, `DefaultClient::new`
    /// auto-installs `VertexAdcCredentialProvider`. This is the canonical auth
    /// path for GKE / Workload Identity deployments where short-lived OAuth2
    /// tokens come from the metadata server.
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_ai_auto_installs_adc_provider_when_no_credentials_configured() {
        // ~keep SAFETY: serial_test::serial prevents concurrent env mutation.
        // ~keep Prior values are restored by the guard below.
        let prior_project = std::env::var("VERTEXAI_PROJECT").ok();
        let prior_location = std::env::var("VERTEXAI_LOCATION").ok();
        struct EnvGuard {
            prior_project: Option<String>,
            prior_location: Option<String>,
        }
        impl Drop for EnvGuard {
            fn drop(&mut self) {
                // ~keep SAFETY: single-threaded restoration during test teardown.
                unsafe {
                    match &self.prior_project {
                        Some(v) => std::env::set_var("VERTEXAI_PROJECT", v),
                        None => std::env::remove_var("VERTEXAI_PROJECT"),
                    }
                    match &self.prior_location {
                        Some(v) => std::env::set_var("VERTEXAI_LOCATION", v),
                        None => std::env::remove_var("VERTEXAI_LOCATION"),
                    }
                }
            }
        }
        let _guard = EnvGuard {
            prior_project,
            prior_location,
        };
        // ~keep SAFETY: serial_test::serial ensures exclusive env access.
        unsafe {
            std::env::set_var("VERTEXAI_PROJECT", "test-project");
            std::env::set_var("VERTEXAI_LOCATION", "us-central1");
        }

        let config = ClientConfigBuilder::new("").load_env(false).build();
        assert!(
            config.credential_provider.is_none(),
            "input config should have no credential_provider"
        );

        let client = DefaultClient::new(config, Some("vertex_ai/gemini-2.5-flash-lite"))
            .expect("DefaultClient::new should succeed for vertex with empty api_key");

        assert!(
            client.config.credential_provider.is_some(),
            "DefaultClient::new should auto-install VertexAdcCredentialProvider for vertex_ai when no credentials are configured"
        );
    }

    /// When the caller already supplied an `api_key`, the auto-install does
    /// not fire — pre-obtained tokens take precedence over ADC discovery.
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_ai_explicit_api_key_skips_auto_install() {
        let prior_project = std::env::var("VERTEXAI_PROJECT").ok();
        struct EnvGuard(Option<String>);
        impl Drop for EnvGuard {
            fn drop(&mut self) {
                unsafe {
                    match &self.0 {
                        Some(v) => std::env::set_var("VERTEXAI_PROJECT", v),
                        None => std::env::remove_var("VERTEXAI_PROJECT"),
                    }
                }
            }
        }
        let _guard = EnvGuard(prior_project);
        unsafe {
            std::env::set_var("VERTEXAI_PROJECT", "test-project");
        }

        let config = ClientConfigBuilder::new("ya29.pre-obtained-token")
            .load_env(false)
            .build();

        let client = DefaultClient::new(config, Some("vertex_ai/gemini-2.5-flash-lite"))
            .expect("DefaultClient::new should succeed");

        assert!(
            client.config.credential_provider.is_none(),
            "auto-install must not fire when api_key is non-empty"
        );
    }

    /// When the caller explicitly supplied a `credential_provider`, the
    /// auto-install does not overwrite it.
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_ai_explicit_credential_provider_skips_auto_install() {
        use std::sync::Arc;

        use crate::auth::{Credential, CredentialProvider, StaticTokenProvider};
        use secrecy::SecretString;

        let prior_project = std::env::var("VERTEXAI_PROJECT").ok();
        struct EnvGuard(Option<String>);
        impl Drop for EnvGuard {
            fn drop(&mut self) {
                unsafe {
                    match &self.0 {
                        Some(v) => std::env::set_var("VERTEXAI_PROJECT", v),
                        None => std::env::remove_var("VERTEXAI_PROJECT"),
                    }
                }
            }
        }
        let _guard = EnvGuard(prior_project);
        unsafe {
            std::env::set_var("VERTEXAI_PROJECT", "test-project");
        }

        let explicit: Arc<dyn CredentialProvider> =
            Arc::new(StaticTokenProvider::new(SecretString::from("static-token".to_owned())));
        let explicit_marker = Arc::as_ptr(&explicit) as *const ();

        let config = ClientConfigBuilder::new("")
            .load_env(false)
            .credential_provider(Arc::clone(&explicit))
            .build();

        let client = DefaultClient::new(config, Some("vertex_ai/gemini-2.5-flash-lite"))
            .expect("DefaultClient::new should succeed");

        let installed = client
            .config
            .credential_provider
            .as_ref()
            .expect("explicit provider should survive auto-install path");
        let installed_marker = Arc::as_ptr(installed) as *const ();
        assert_eq!(
            installed_marker, explicit_marker,
            "auto-install must not overwrite an explicitly-supplied credential_provider"
        );

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let credential = rt.block_on(installed.resolve()).expect("resolve");
        match credential {
            Credential::BearerToken(t) => {
                use secrecy::ExposeSecret;
                assert_eq!(t.expose_secret(), "static-token");
            }
            _ => panic!("expected BearerToken"),
        }
    }

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    fn vertex_client() -> DefaultClient {
        DefaultClient::new(
            ClientConfigBuilder::new("ya29.pre-obtained-token")
                .load_env(false)
                .build(),
            Some("vertex_ai/gemini-2.5-flash"),
        )
        .expect("DefaultClient::new should succeed")
    }

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    struct VertexProjectGuard(Option<String>);

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    impl VertexProjectGuard {
        fn set() -> Self {
            let prior = std::env::var("VERTEXAI_PROJECT").ok();
            unsafe {
                std::env::set_var("VERTEXAI_PROJECT", "test-project");
            }
            Self(prior)
        }
    }

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    impl Drop for VertexProjectGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VERTEXAI_PROJECT", value),
                    None => std::env::remove_var("VERTEXAI_PROJECT"),
                }
            }
        }
    }

    /// Vertex/Gemini rebuilds the body into native `generateContent` shape, which has no
    /// `stream` field. Restoring the transport flag after `transform_request` therefore shipped
    /// `"stream": false` on every non-streaming call, and Vertex rejects the whole request with
    /// HTTP 400 `Invalid JSON payload received. Unknown name "stream": Cannot find field.`
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_non_streaming_request_carries_no_stream_key() {
        let _guard = VertexProjectGuard::set();
        let client = vertex_client();
        let req = ChatCompletionRequest {
            model: "vertex_ai/gemini-2.5-flash".into(),
            messages: vec![],
            ..Default::default()
        };

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))
            .expect("prepare_request should not fail");

        assert!(
            prepared.body_json.get("stream").is_none(),
            "generateContent has no `stream` field; sending it is a hard 400, body was: {}",
            prepared.body_json
        );
    }

    /// Streaming against Vertex is selected by endpoint (`streamGenerateContent`), not by a body
    /// flag, so the streaming path must not reintroduce the key either.
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_streaming_request_carries_no_stream_key() {
        let _guard = VertexProjectGuard::set();
        let client = vertex_client();
        let req = ChatCompletionRequest {
            model: "vertex_ai/gemini-2.5-flash".into(),
            messages: vec![],
            ..Default::default()
        };

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(true))
            .expect("prepare_request should not fail");

        assert!(
            prepared.body_json.get("stream").is_none(),
            "streaming is endpoint-selected on Vertex; body was: {}",
            prepared.body_json
        );
    }

    /// Guard against over-correcting: providers whose wire format does carry `stream` must still
    /// get the transport-controlled flag, so the request cannot desync from the response parser.
    #[test]
    fn openai_shaped_provider_still_carries_the_stream_flag() {
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some("gpt-4"))
            .expect("client construction should succeed");
        let req = ChatCompletionRequest {
            model: "gpt-4".into(),
            messages: vec![],
            ..Default::default()
        };

        for streaming in [false, true] {
            let prepared = client
                .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(streaming))
                .expect("prepare_request should not fail");
            assert_eq!(
                prepared.body_json["stream"], streaming,
                "OpenAI-shaped bodies must keep the transport-controlled stream flag"
            );
        }
    }
}
