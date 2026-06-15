/// Client builder configuration ([`ClientConfig`] and related helpers).
pub mod config;
/// On-disk client configuration schema (TOML / JSON / YAML).
#[allow(missing_docs)]
pub mod config_file;
/// Tower-backed managed client wired with rate limit, cache, routing, etc.
#[cfg(all(feature = "native-http", feature = "tower"))]
pub mod managed;

use std::future::Future;
use std::pin::Pin;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use std::sync::Arc;

use futures_core::Stream;

use crate::error::Result;
use crate::types::audio::{CreateSpeechRequest, CreateTranscriptionRequest, TranscriptionResponse};
use crate::types::batch::{BatchListQuery, BatchListResponse, BatchObject, CreateBatchRequest};
use crate::types::files::{CreateFileRequest, DeleteResponse, FileListQuery, FileListResponse, FileObject};
use crate::types::image::{CreateImageRequest, ImagesResponse};
use crate::types::moderation::{ModerationRequest, ModerationResponse};
use crate::types::ocr::{OcrRequest, OcrResponse};
use crate::types::raw::{RawExchange, RawStreamExchange};
use crate::types::rerank::{RerankRequest, RerankResponse};
use crate::types::responses::{CreateResponseRequest, ResponseObject};
use crate::types::search::{SearchRequest, SearchResponse};
use crate::types::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse,
    ModelsListResponse,
};

// DefaultClient and its LlmClient impl require reqwest + tokio.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use crate::auth::Credential;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use crate::error::LiterLlmError;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use crate::http;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use crate::provider::{self, OpenAiCompatibleProvider, OpenAiProvider, Provider};
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use secrecy::ExposeSecret;

pub use config::{ClientConfig, ClientConfigBuilder};
pub use config_file::FileConfig;

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

/// Core LLM client trait.
///
/// Provides unified access to LLM and multimodal APIs across 140+ providers.
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

/// Responses API operations (create, retrieve, cancel).
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub trait ResponseClient: Send + Sync {
    /// Create a new response.
    fn create_response(&self, req: CreateResponseRequest) -> BoxFuture<'_, Result<ResponseObject>>;

    /// Retrieve a response by ID.
    fn retrieve_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>>;

    /// Cancel an in-progress response.
    fn cancel_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>>;
}

/// Responses API operations (create, retrieve, cancel) (WASM variant).
#[cfg(target_arch = "wasm32")]
#[cfg_attr(alef, alef(skip))]
pub trait ResponseClient {
    /// Create a new response.
    fn create_response(&self, req: CreateResponseRequest) -> BoxFuture<'_, Result<ResponseObject>>;

    /// Retrieve a response by ID.
    fn retrieve_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>>;

    /// Cancel an in-progress response.
    fn cancel_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>>;
}

/// Default client implementation backed by `reqwest`.
///
/// Sends requests to 140+ LLM providers with automatic provider detection
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
        let provider = build_provider(&config, model_hint);
        // Validate configuration eagerly so callers get a clear error at
        // construction time rather than on the first request.
        provider.validate()?;

        // Auto-load the API key from the environment when no explicit key was
        // provided and `load_env` is enabled.  Skipped on WASM where
        // `std::env::var` is unavailable.
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

        // Auto-install VertexAdcCredentialProvider when the resolved provider is
        // Vertex AI and the caller supplied neither an explicit api_key nor a
        // credential_provider. The ADC provider obtains short-lived OAuth2 tokens
        // from the GKE / Compute Engine metadata server (or via gcp_auth's ADC
        // discovery chain for local development), which is the canonical auth
        // path for Workload Identity deployments. Callers that supply a
        // pre-obtained access token via api_key or explicitly set a
        // credential_provider continue to take precedence.
        #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
        if config.credential_provider.is_none()
            && config.api_key.expose_secret().is_empty()
            && provider.name() == "vertex_ai"
        {
            config.credential_provider = Some(Arc::new(crate::auth::vertex_adc::VertexAdcCredentialProvider::new()));
        }

        // Build the header map from pre-validated headers stored in the config.
        // The builder already validated each header name/value, so these
        // conversions are expected to succeed; return a proper error if they
        // somehow fail rather than panicking.
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
            // Install the guarded DNS resolver when the outbound policy is not
            // Off.  This provides defense-in-depth against DNS rebinding: even
            // if a hostname initially passed the sync registration-time check,
            // the resolver re-validates every resolved address at connect time.
            // WASM uses the browser fetch API; DNS happens in the browser and
            // cannot be intercepted from Rust, so this is native-only.
            #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
            let builder = {
                if !matches!(crate::provider::current_policy(), crate::provider::OutboundPolicy::Off) {
                    builder.dns_resolver(crate::provider::outbound_policy::guarded_resolver())
                } else {
                    builder
                }
            };
            // reqwest's WASM backend uses the browser fetch API and does not
            // support per-client timeout configuration.
            #[cfg(not(target_arch = "wasm32"))]
            let builder = builder.timeout(config.timeout);
            // Apply transport config (connection pool, TCP keepalive, HTTP
            // version negotiation).  WASM uses the browser fetch API which
            // controls these settings independently.
            #[cfg(not(target_arch = "wasm32"))]
            let builder = config.transport.apply_to_builder(builder);
            builder.build().map_err(LiterLlmError::from)?
        };

        // Pre-compute the auth header once at construction time to avoid
        // `format!("Bearer {key}")` on every request.
        let cached_auth_header = provider
            .auth_header(config.api_key.expose_secret())
            .map(|(name, value)| (name.into_owned(), value.into_owned()));

        // Pre-compute static extra headers once to avoid `&'static str` ->
        // `String` conversion on every request.
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
        // When a base_url override is set, always use the construction-time
        // provider — the user explicitly pointed the client at a specific
        // endpoint (e.g. a mock server or custom proxy).
        if self.config.base_url.is_some() {
            return Arc::clone(&self.provider);
        }
        // If the construction-time provider already matches this model, keep it.
        if self.provider.matches_model(model) {
            return Arc::clone(&self.provider);
        }
        // Attempt per-request detection from the model prefix.
        if let Some(detected) = provider::detect_provider(model) {
            return Arc::from(detected);
        }
        // Fall back to the construction-time provider.
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
            // Re-compute auth header for the resolved provider.
            Ok(prov
                .auth_header(self.config.api_key.expose_secret())
                .map(|(name, value)| (name.into_owned(), value.into_owned())))
        }
    }

    /// Build the combined header list for a request using a specific provider.
    fn all_headers_for_provider(
        &self,
        prov: &dyn Provider,
        method: &str,
        url: &str,
        body_json: &serde_json::Value,
        body_bytes: &[u8],
    ) -> Vec<(String, String)> {
        let mut headers = prov.signing_headers(method, url, body_bytes);
        headers.extend(
            prov.extra_headers()
                .iter()
                .map(|&(name, value)| (name.to_owned(), value.to_owned())),
        );
        headers.extend(prov.dynamic_headers(body_json));
        headers
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
        // Use build_url so providers like Azure and Bedrock can embed the model
        // name or deployment identifier into the URL.
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

        // Serialize exactly once — the same bytes are used for signing and for
        // the HTTP request body.  `Bytes` is reference-counted, so cloning on
        // retry is a zero-copy bump.
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
    fn all_headers(
        &self,
        method: &str,
        url: &str,
        body_json: &serde_json::Value,
        body_bytes: &[u8],
    ) -> Vec<(String, String)> {
        let mut headers = self.provider.signing_headers(method, url, body_bytes);
        headers.extend(self.cached_extra_headers.iter().cloned());
        headers.extend(self.provider.dynamic_headers(body_json));
        headers
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
        return Arc::new(OpenAiCompatibleProvider {
            name: "custom".into(),
            base_url: base_url.clone(),
            env_var: None,
            model_prefixes: vec![],
        });
    }

    if let Some(model) = model_hint
        && let Some(p) = provider::detect_provider(model)
    {
        // detect_provider returns Box<dyn Provider>; convert to Arc.
        return Arc::from(p);
    }

    Arc::new(OpenAiProvider)
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl LlmClient for DefaultClient {
    fn chat(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>> {
        Box::pin(async move {
            // Pass stream=false so providers can inspect the flag in transform_request.
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            // Use prepare_request for validation, model-prefix stripping, and
            // transform_request — then override the URL via build_stream_url.
            let prepared = self.prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(true))?;

            // Always use build_stream_url for the streaming endpoint.
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            match prepared.provider.stream_format() {
                provider::StreamFormat::Sse => {
                    let provider = Arc::clone(&prepared.provider);
                    let parse_event = move |data: &str| provider.parse_stream_event(data);
                    let stream = http::streaming::post_stream(
                        &self.http,
                        &url,
                        auth,
                        &extra,
                        prepared.body_bytes,
                        self.config.max_retries,
                        parse_event,
                    )
                    .await?;
                    Ok(stream)
                }
                provider::StreamFormat::AwsEventStream => {
                    let stream = http::eventstream::post_eventstream(
                        &self.http,
                        &url,
                        auth,
                        &extra,
                        prepared.body_bytes,
                        self.config.max_retries,
                        provider::bedrock::parse_bedrock_stream_event,
                    )
                    .await?;
                    Ok(stream)
                }
            }
        })
    }

    fn embed(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>> {
        Box::pin(async move {
            // Embeddings have no stream flag; pass None so it is not inserted.
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<EmbeddingResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn list_models(&self) -> BoxFuture<'_, Result<ModelsListResponse>> {
        Box::pin(async move {
            // list_models has no model string — use the construction-time provider.
            let url = self.provider.build_url(self.provider.models_path(), "");
            let auth_header = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[]);
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let mut raw = http::request::get_json_raw(&self.http, &url, auth, &extra, self.config.max_retries).await?;
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            http::request::post_binary(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let stream = match prepared.provider.stream_format() {
                provider::StreamFormat::Sse => {
                    let provider = Arc::clone(&prepared.provider);
                    let parse_event = move |data: &str| provider.parse_stream_event(data);
                    http::streaming::post_stream(
                        &self.http,
                        &url,
                        auth,
                        &extra,
                        prepared.body_bytes,
                        self.config.max_retries,
                        parse_event,
                    )
                    .await?
                }
                provider::StreamFormat::AwsEventStream => {
                    http::eventstream::post_eventstream(
                        &self.http,
                        &url,
                        auth,
                        &extra,
                        prepared.body_bytes,
                        self.config.max_retries,
                        provider::bedrock::parse_bedrock_stream_event,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            );
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.config.max_retries,
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
            let all_headers = self.all_headers("POST", &url, &serde_json::Value::Null, &[]);
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            // Decode the base64-encoded file data into raw bytes for the multipart upload.
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

            let raw = http::request::post_multipart(&self.http, &url, auth, &extra, form).await?;
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
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[]);
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw(&self.http, &url, auth, &extra, self.config.max_retries).await?;
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
            let all_headers = self.all_headers("DELETE", &url, &serde_json::Value::Null, &[]);
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::delete_json(&self.http, &url, auth, &extra, self.config.max_retries).await?;
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
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[]);
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw(&self.http, &url, auth, &extra, self.config.max_retries).await?;
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
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[]);
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            http::request::get_binary(&self.http, &url, auth, &extra, self.config.max_retries).await
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
            let all_headers = self.all_headers("POST", &url, &body_json, &body_bytes);
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let raw = http::request::post_json_raw(&self.http, &url, auth, &extra, body_bytes, self.config.max_retries)
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
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[]);
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw(&self.http, &url, auth, &extra, self.config.max_retries).await?;
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
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[]);
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw(&self.http, &url, auth, &extra, self.config.max_retries).await?;
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
            let all_headers = self.all_headers("POST", &url, &body_json, &body_bytes);
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let raw = http::request::post_json_raw(&self.http, &url, auth, &extra, body_bytes, self.config.max_retries)
                .await?;
            serde_json::from_value::<BatchObject>(raw).map_err(LiterLlmError::from)
        })
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl ResponseClient for DefaultClient {
    fn create_response(&self, req: CreateResponseRequest) -> BoxFuture<'_, Result<ResponseObject>> {
        Box::pin(async move {
            let url = self.provider.build_url(self.provider.responses_path(), "");
            let body_bytes = bytes::Bytes::from(serde_json::to_vec(&req)?);
            let body_json = serde_json::to_value(&req)?;

            let auth_header = self.resolve_auth_header().await?;
            let all_headers = self.all_headers("POST", &url, &body_json, &body_bytes);
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let raw = http::request::post_json_raw(&self.http, &url, auth, &extra, body_bytes, self.config.max_retries)
                .await?;
            serde_json::from_value::<ResponseObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn retrieve_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>> {
        let response_id = response_id.to_owned();
        Box::pin(async move {
            let url = format!(
                "{}/{}",
                self.provider.build_url(self.provider.responses_path(), ""),
                response_id
            );
            let auth_header = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers("GET", &url, &serde_json::Value::Null, &[]);
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw(&self.http, &url, auth, &extra, self.config.max_retries).await?;
            serde_json::from_value::<ResponseObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn cancel_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>> {
        let response_id = response_id.to_owned();
        Box::pin(async move {
            let url = format!(
                "{}/{}/cancel",
                self.provider.build_url(self.provider.responses_path(), ""),
                response_id
            );
            let auth_header = self.resolve_auth_header().await?;
            let body_json = serde_json::Value::Null;
            let body_bytes = bytes::Bytes::new();
            let all_headers = self.all_headers("POST", &url, &body_json, &body_bytes);
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let raw = http::request::post_json_raw(&self.http, &url, auth, &extra, body_bytes, self.config.max_retries)
                .await?;
            serde_json::from_value::<ResponseObject>(raw).map_err(LiterLlmError::from)
        })
    }
}

#[cfg(all(test, any(feature = "native-http", feature = "wasm-http")))]
mod build_provider_tests {
    use super::*;
    use crate::client::config::ClientConfigBuilder;

    #[test]
    fn azure_model_with_per_model_base_url_uses_azure_provider() {
        // Regression test for issue #83: when `[[models]]` pins a per-model
        // `base_url` AND the provider_model is azure/..., the resolved
        // provider must be Azure (which embeds the deployment name and
        // ?api-version=… in the URL), NOT a naive OpenAI-compatible URL.
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
        // The Azure carve-out must not regress the LM Studio / Ollama / vLLM
        // path, which legitimately uses the naive base_url + endpoint shape.
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
        // Without an explicit per-model base_url, Azure provider is still
        // detected — but base_url comes from env vars (likely empty in CI),
        // so validate() would fail. We only assert the name here.
        assert_eq!(p.name(), "azure");
    }

    // ── Vertex AI ADC auto-install ───────────────────────────────────────────

    /// When the resolved provider is Vertex AI and the caller supplied neither
    /// an explicit `api_key` nor a `credential_provider`, `DefaultClient::new`
    /// auto-installs `VertexAdcCredentialProvider`. This is the canonical auth
    /// path for GKE / Workload Identity deployments where short-lived OAuth2
    /// tokens come from the metadata server.
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_ai_auto_installs_adc_provider_when_no_credentials_configured() {
        // SAFETY: serial_test::serial guarantees no other test mutates these
        // env vars concurrently. We restore the prior values on drop below.
        let prior_project = std::env::var("VERTEXAI_PROJECT").ok();
        let prior_location = std::env::var("VERTEXAI_LOCATION").ok();
        struct EnvGuard {
            prior_project: Option<String>,
            prior_location: Option<String>,
        }
        impl Drop for EnvGuard {
            fn drop(&mut self) {
                // SAFETY: single-threaded restoration during test teardown.
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
        // SAFETY: serial_test::serial ensures exclusive access.
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

        // Sanity check: the explicit provider still resolves to its static token.
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
}
