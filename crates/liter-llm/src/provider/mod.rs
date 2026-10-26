use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use web_time::{SystemTime, UNIX_EPOCH};

use crate::error::{LiterLlmError, Result};

/// Return the current Unix epoch timestamp in seconds.
///
/// Used by provider transformers to populate the `created` field in
/// OpenAI-compatible response objects. Falls back to `0` if the system
/// clock is before the epoch (should never happen in practice).
///
/// Reads the clock through `web_time`, which is `std::time` on native targets
/// and a JavaScript `Date` clock on `wasm32-unknown-unknown`, where
/// `std::time::SystemTime::now()` panics.
pub(crate) fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Validate that a sampling parameter present in the request body falls within a
/// provider's documented range, rejecting it with a [`LiterLlmError::BadRequest`]
/// when it does not.
///
/// `ChatCompletionRequest::temperature` and `top_p` are documented at the widest
/// range accepted by any supported provider, so a value that is legal per that
/// doc can still be illegal for a specific provider (e.g. Anthropic caps
/// `temperature` at `1.0`, not OpenAI's `2.0`). Rejecting the value here, before
/// the request reaches the wire, turns an opaque provider-side 400 into a local,
/// actionable error instead of silently rewriting the caller's requested
/// sampling behaviour, which a clamp would do. A missing field is not an error;
/// only a present, out-of-range value is rejected.
pub(crate) fn validate_sampling_param_range(
    body: &serde_json::Value,
    field: &str,
    provider: &str,
    min: f64,
    max: f64,
) -> Result<()> {
    let Some(value) = body.get(field).and_then(serde_json::Value::as_f64) else {
        return Ok(());
    };
    if value < min || value > max {
        return Err(LiterLlmError::BadRequest {
            message: format!(
                "{field}={value} is outside {provider}'s supported range [{min}, {max}]; lower \
                 the requested value or omit `{field}` to use the provider default"
            ),
            status: 400,
        });
    }
    Ok(())
}

/// The streaming wire format a provider uses for its response stream.
///
/// Most providers use standard Server-Sent Events (SSE).  AWS Bedrock uses
/// a proprietary binary EventStream framing.  Ollama's native API streams
/// newline-delimited JSON.
///
/// Deserialized from the `streaming_format` JSON field via [`serde`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamFormat {
    /// Standard Server-Sent Events (text/event-stream).
    #[default]
    Sse,
    /// AWS EventStream binary framing (application/vnd.amazon.eventstream).
    AwsEventStream,
    /// Newline-delimited JSON (application/x-ndjson): one JSON object per line.
    Ndjson,
}

/// Static capability flags for a provider.
///
/// Each flag indicates whether the provider's models *generally* support that
/// feature.  For providers that aggregate many underlying models (e.g. Bedrock,
/// OpenRouter, vLLM) the flags reflect the superset of available model
/// capabilities — a flag being `true` means at least one model supports the
/// feature, not every model.
///
/// All flags default to `false` so that newly added providers are safe.
///
/// Access via the crate-level [`capabilities`] function:
///
/// ```rust
/// use liter_llm::capabilities;
///
/// let caps = capabilities("openai");
/// assert!(caps.function_calling);
/// assert!(caps.vision);
///
/// // Unknown providers return a default-all-false reference.
/// let unknown = capabilities("my-private-model");
/// assert!(!unknown.function_calling);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProviderCapabilities {
    /// The provider accepts image input in chat messages.
    pub vision: bool,
    /// The provider supports extended-thinking / reasoning tokens.
    pub reasoning: bool,
    /// The provider supports JSON-mode or `response_format` structured output.
    pub structured_output: bool,
    /// The provider supports tool / function calling.
    pub function_calling: bool,
    /// The provider accepts audio as input.
    pub audio_in: bool,
    /// The provider can generate audio / TTS output.
    pub audio_out: bool,
    /// The provider accepts video as input.
    pub video_in: bool,
}

/// Static all-false sentinel returned by [`capabilities`] for unknown providers.
static DEFAULT_CAPABILITIES: ProviderCapabilities = ProviderCapabilities {
    vision: false,
    reasoning: false,
    structured_output: false,
    function_calling: false,
    audio_in: false,
    audio_out: false,
    video_in: false,
};

/// Return the capability flags for a named provider.
///
/// Performs an O(n) linear scan over the embedded registry (174 entries).
/// Returns an owned value so bindings can pass capability data without
/// borrowing registry internals.
///
/// For unknown `provider_name` values the function returns an all-`false`
/// sentinel so callers never need to handle `Option`.
pub fn capabilities(provider_name: &str) -> ProviderCapabilities {
    let Ok(reg) = REGISTRY.as_ref() else {
        return DEFAULT_CAPABILITIES;
    };
    for entry in &reg.providers {
        if entry.config.name == provider_name {
            return entry.capabilities;
        }
    }
    DEFAULT_CAPABILITIES
}

const PROVIDERS_JSON: &str = include_str!("../../schemas/providers.json");

/// Lazy-initialised registry parsed from the embedded JSON.
/// Stores a `Result` so that parse failures surface at call time rather than
/// panicking the process (fix for the `.expect()` on LazyLock).
static REGISTRY: LazyLock<std::result::Result<ProviderRegistry, String>> = LazyLock::new(|| {
    serde_json::from_str::<ProviderRegistryRaw>(PROVIDERS_JSON)
        .map(ProviderRegistry::from_raw)
        .map_err(|e| {
            // ~keep Logged once here (LazyLock evaluates its closure exactly once) rather
            // than at every `detect_provider`/`capabilities` call site that silently
            // falls back to "no provider found" on this error.
            tracing::error!(error = %e, "embedded schemas/providers.json failed to parse");
            e.to_string()
        })
});

/// Access the registry, returning an error if the embedded JSON was invalid.
fn registry() -> Result<&'static ProviderRegistry> {
    REGISTRY.as_ref().map_err(|e| LiterLlmError::ServerError {
        message: format!("embedded schemas/providers.json is invalid: {e}"),
        status: 500,
    })
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct RegionalEndpointConfig {
    region: String,
    openai_base_url: String,
    anthropic_base_url: String,
    docs_root: String,
}

/// A kind of API call that the generic OpenAI-shaped route can carry.
///
/// Names the endpoints a provider can list in `no_route_endpoints` in providers.json.
/// `Audio` covers both speech and transcription; `Image` covers image generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Endpoint {
    Chat,
    Embedding,
    Image,
    Audio,
    Moderation,
    Rerank,
    Search,
    Ocr,
}

impl Endpoint {
    fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Embedding => "embedding",
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Moderation => "moderation",
            Self::Rerank => "rerank",
            Self::Search => "search",
            Self::Ocr => "ocr",
        }
    }
}

/// Internal JSON shape: each provider entry with capability flags and streaming
/// format, stored separately from the public [`ProviderConfig`] schema.
#[derive(Debug, Deserialize)]
struct ProviderEntry {
    #[serde(flatten)]
    config: ProviderConfig,
    /// Alternative `alias/` namespaces that route to this provider and are stripped like
    /// `name/` (e.g. `lm_studio` for `lmstudio`). Kept off the public [`ProviderConfig`].
    #[serde(default)]
    aliases: Vec<String>,
    /// Absolute URLs for endpoints that live outside `base_url` (e.g. DashScope rerank under
    /// `/compatible-api/v1/reranks` while chat is under `/compatible-mode/v1`). Keyed by
    /// endpoint kind; currently only `rerank` is consulted.
    #[serde(default)]
    endpoint_urls: HashMap<String, String>,
    /// Endpoints this provider has no route for, although its other endpoints work (for
    /// example a provider with an OpenAI-compatible chat endpoint and a native search API).
    /// A call to one of them is refused before any request. Only the provider built from this
    /// registry entry carries the list: a `base_url` or a custom provider never does.
    #[serde(default)]
    no_route_endpoints: Vec<Endpoint>,
    #[serde(default)]
    capabilities: ProviderCapabilities,
    /// Protocol-specific base URLs available in each supported region.
    #[allow(dead_code)]
    #[serde(default)]
    regional_endpoints: Vec<RegionalEndpointConfig>,
    /// Streaming wire format for this provider.
    ///
    /// Deserialized from `providers.json` and available to future workstreams
    /// via `ProviderEntry` (e.g. to drive per-provider streaming-format routing
    /// without hardcoding the Bedrock special-case in `detect_provider`).
    #[allow(dead_code)]
    #[serde(default, rename = "streaming_format")]
    stream_format: StreamFormat,
}

/// Intermediate deserialization target for `providers.json`.
#[derive(Debug, Deserialize)]
struct ProviderRegistryRaw {
    providers: Vec<ProviderEntry>,
    /// Set of complex provider names for O(1) lookup.
    ///
    /// Deserialized from a JSON array; converted to a `HashSet` for fast
    /// membership tests in the hot `detect_provider` path.
    #[serde(default, deserialize_with = "deserialize_hashset")]
    complex_providers: HashSet<String>,
}

/// The parsed and indexed registry.
///
/// `providers` holds the full entries including capability flags and streaming
/// format (for internal routing and capability lookup).  `configs` is a
/// pre-extracted public-config slice returned by [`all_providers`].
#[derive(Debug)]
struct ProviderRegistry {
    providers: Vec<ProviderEntry>,
    configs: Vec<ProviderConfig>,
    complex_providers: HashSet<String>,
}

impl ProviderRegistry {
    fn from_raw(raw: ProviderRegistryRaw) -> Self {
        let configs = raw.providers.iter().map(|e| e.config.clone()).collect();
        Self {
            configs,
            providers: raw.providers,
            complex_providers: raw.complex_providers,
        }
    }
}

fn deserialize_hashset<'de, D>(deserializer: D) -> std::result::Result<HashSet<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let vec = Vec::<String>::deserialize(deserializer)?;
    Ok(vec.into_iter().collect())
}

/// Static configuration for a single provider entry in providers.json.
///
/// This struct deliberately does not include capability flags or streaming
/// format, which are accessed via the [`capabilities`] function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider identifier (matches the entry key in providers.json).
    pub name: String,
    /// Human-readable provider name shown in UIs.
    pub display_name: Option<String>,
    /// Base URL used as the default for this provider's HTTP client.
    pub base_url: Option<String>,
    /// Authentication scheme metadata (auth type + env var holding the key).
    pub auth: Option<AuthConfig>,
    /// Supported endpoint kinds (e.g. `chat`, `embeddings`).
    pub endpoints: Option<Vec<String>>,
    /// Model-name prefixes claimed by this provider (e.g. `["gpt-", "o1-"]`).
    pub model_prefixes: Option<Vec<String>>,
    /// Parameter key renaming for this provider.
    ///
    /// Each entry maps an OpenAI-spec field name (e.g. `"max_completion_tokens"`)
    /// to the name this provider expects (e.g. `"max_tokens"`).  Applied
    /// automatically by `ConfigDrivenProvider::transform_request`.
    pub param_mappings: Option<HashMap<String, String>>,
}

/// Auth scheme used by a provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AuthType {
    /// Standard `Authorization: Bearer <key>` header.
    Bearer,
    /// `x-api-key: <key>` header (also handles `"header"` and `"x-api-key"` aliases).
    #[serde(alias = "header", alias = "x-api-key")]
    ApiKey,
    /// No authentication header required.
    None,
    /// Unrecognised auth scheme — falls back to bearer.
    #[serde(other)]
    Unknown,
}

/// Auth configuration block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Auth scheme classification.
    #[serde(rename = "type")]
    pub auth_type: AuthType,
    /// Name of the environment variable that holds the API key (e.g. `"OPENAI_API_KEY"`).
    /// Holds the variable name, never the secret value.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub env_var: Option<String>,
}

/// The HTTP status an error event stands for when its payload has no usable
/// `status_code`. The error code decides first, then the error type; an error
/// with neither reads as 400, like a request the provider refused.
fn status_for_error_kind(code: Option<&str>, kind: Option<&str>) -> u16 {
    match code {
        Some("rate_limit_exceeded" | "insufficient_quota") => return 429,
        Some("invalid_api_key") => return 401,
        Some("model_not_found") => return 404,
        Some("server_error") => return 500,
        _ => {}
    }
    match kind {
        Some("rate_limit_error") => 429,
        Some("authentication_error") => 401,
        Some("billing_error") => 402,
        Some("permission_error") => 403,
        Some("not_found_error") => 404,
        Some("request_too_large") => 413,
        Some("server_error" | "api_error") => 500,
        Some("timeout_error") => 504,
        Some("overloaded_error") => 529,
        _ => 400,
    }
}

/// A JSON number, or a string that holds a whole number, as a `u16`.
fn u16_field(value: &serde_json::Value) -> Option<u16> {
    let number = match value {
        serde_json::Value::String(s) => s.trim().parse::<u64>().ok()?,
        other => other.as_u64()?,
    };
    u16::try_from(number).ok()
}

/// The payload `status_code` of an error event, if it fits a `u16`. The error
/// object is read first, as a number or a string that holds a number, and its
/// value is kept as is, even outside the error range. The top level of the event
/// is the fallback and counts only as an error status (400 to 599), because the
/// top level is not part of any documented error shape.
pub(crate) fn status_code_field(event: &serde_json::Value) -> Option<u16> {
    event.pointer("/error/status_code").and_then(u16_field).or_else(|| {
        event
            .get("status_code")
            .and_then(u16_field)
            .filter(|s| (400..=599).contains(s))
    })
}

/// The HTTP status of a Google error object (Gemini and Vertex AI). The numeric
/// `code` decides when it is an error status (400 to 599), as a number or a
/// string. Otherwise the canonical `status` name decides, as in Google's error
/// model. `None` when neither gives an error status.
pub(crate) fn google_error_status(event: &serde_json::Value) -> Option<u16> {
    let from_code = event
        .pointer("/error/code")
        .and_then(u16_field)
        .filter(|s| (400..=599).contains(s));
    from_code.or_else(
        || match event.pointer("/error/status").and_then(serde_json::Value::as_str)? {
            "INVALID_ARGUMENT" | "FAILED_PRECONDITION" | "OUT_OF_RANGE" => Some(400),
            "UNAUTHENTICATED" => Some(401),
            "PERMISSION_DENIED" => Some(403),
            "NOT_FOUND" => Some(404),
            "ABORTED" | "ALREADY_EXISTS" => Some(409),
            "RESOURCE_EXHAUSTED" => Some(429),
            "CANCELLED" => Some(499),
            "UNKNOWN" | "INTERNAL" | "DATA_LOSS" => Some(500),
            "UNIMPLEMENTED" => Some(501),
            "UNAVAILABLE" => Some(503),
            "DEADLINE_EXCEEDED" => Some(504),
            _ => None,
        },
    )
}

/// The `retry_after` of an error event, in seconds, read from the error object
/// first, then from the top level of the event. A value is a number or a string
/// that holds a number, like the `Retry-After` header. A negative, non-finite or
/// unreadable value is skipped, so the next place still gets read.
fn retry_after_field(event: &serde_json::Value) -> Option<std::time::Duration> {
    let seconds = |value: &serde_json::Value| -> Option<std::time::Duration> {
        let secs = match value {
            serde_json::Value::String(s) => s.trim().parse::<f64>().ok()?,
            other => other.as_f64()?,
        };
        std::time::Duration::try_from_secs_f64(secs).ok()
    };
    [event.pointer("/error/retry_after"), event.get("retry_after")]
        .into_iter()
        .flatten()
        .find_map(seconds)
}

/// Build the error for an error event inside a stream, or `None` when the event
/// carries no `error` object. Every provider's stream parser calls this, so an
/// error event is the same kind mid-stream as the same body in an HTTP reply.
///
/// `status` is the HTTP status the provider put in the payload. With `None`, the
/// error code or type gives one (see `status_for_error_kind`).
pub(crate) fn stream_error_event(event: &serde_json::Value, status: Option<u16>) -> Option<LiterLlmError> {
    let error = event.get("error")?;
    let text = |key: &str| error.get(key).and_then(serde_json::Value::as_str);
    let message = text("message").unwrap_or("provider returned an error mid-stream");
    let status = status.unwrap_or_else(|| status_for_error_kind(text("code"), text("type")));
    if !(400..=599).contains(&status) {
        return Some(LiterLlmError::BadRequest {
            message: message.to_string(),
            status,
        });
    }
    // ~keep `details` carries the Google `API_KEY_INVALID` reason that `from_status` reads.
    let body =
        serde_json::json!({ "error": { "message": message, "code": text("code"), "details": error.get("details") } });
    Some(LiterLlmError::from_status(
        status,
        &body.to_string(),
        retry_after_field(event),
    ))
}

/// A provider defines how to reach an LLM API endpoint.
pub(crate) trait Provider: Send + Sync {
    /// Validate provider configuration at construction time.
    ///
    /// Called by [`DefaultClient::new`] immediately after the provider is
    /// resolved.  Returning an error here surfaces misconfiguration early
    /// (e.g. missing Azure `base_url`) rather than on the first request.
    ///
    /// The default implementation is a no-op; providers with required
    /// configuration fields (like Azure) override this.
    fn validate(&self) -> Result<()> {
        Ok(())
    }

    /// Name of the environment variable that holds the API key for this provider.
    ///
    /// Returns `None` for providers that do not use an API key (e.g. auth type
    /// `none`), or for providers whose key source is handled out-of-band (e.g.
    /// AWS Bedrock credentials resolved via the AWS SDK).
    ///
    /// Used by [`DefaultClient::new`] to auto-load the API key from the
    /// environment when `load_env` is enabled and no explicit key was provided.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    fn env_var(&self) -> Option<&str> {
        None
    }

    /// Provider name (e.g., "openai").
    fn name(&self) -> &str;

    /// Endpoints this provider has no route for, although its other endpoints work.
    ///
    /// Only a provider built from a registry entry lists any. A custom provider, and a
    /// provider pinned by a `base_url`, keep the default empty list, whatever their name.
    fn unrouted_endpoints(&self) -> &[Endpoint] {
        &[]
    }

    /// Base URL (e.g., "https://api.openai.com/v1").
    fn base_url(&self) -> &str;

    /// This provider, sent to `base_url` instead of its own host.
    ///
    /// A `base_url` on the client changes only the host that receives the request. The
    /// request body, the path and the reply handling stay those of this provider.
    /// A provider whose URL is not `{base_url}{path}` (Azure, Google AI Studio, Vertex AI,
    /// Bedrock) or whose base URL is private state overrides this method. The default
    /// returns `None`, and [`on_base_url`] then wraps the provider so that only its host changes.
    fn at_base_url(&self, base_url: &str) -> Option<Box<dyn Provider>> {
        let _ = base_url;
        None
    }

    /// Build the authorization header as `Some((header-name, header-value))`.
    ///
    /// Returns `None` when the provider requires no authentication header
    /// (e.g. local models or providers with `auth: none`).  Callers must skip
    /// inserting any header when `None` is returned.
    ///
    /// When `Some`, returns a static header name and a borrowed-or-owned value
    /// to avoid allocating the header name string on every request.
    fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)>;

    /// Additional static headers required by this provider beyond the auth header.
    ///
    /// Most providers return an empty slice.  Use this for provider-mandated
    /// headers like Anthropic's `anthropic-version`.
    fn extra_headers(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }

    /// Compute request-dependent headers based on the request body.
    ///
    /// Called by the client for each request. Use this for headers that
    /// vary per-request, like Anthropic's `anthropic-beta` which depends
    /// on whether thinking or hosted tools are enabled.
    ///
    /// The default implementation returns an empty vector.
    fn dynamic_headers(&self, _body: &serde_json::Value) -> Vec<(String, String)> {
        vec![]
    }

    /// Whether this provider matches a given model string.
    fn matches_model(&self, model: &str) -> bool;

    /// Strip any provider-routing prefix from a model name before sending it
    /// in the request body.
    ///
    /// E.g. `"groq/llama3-70b"` → `"llama3-70b"`.
    /// Returns the model name unchanged when no prefix is present.
    fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
        if let Some(rest) = model.strip_prefix(self.name())
            && let Some(stripped) = rest.strip_prefix('/')
        {
            return stripped;
        }
        model
    }

    /// Path for chat completions endpoint.
    fn chat_completions_path(&self) -> &str {
        "/chat/completions"
    }

    /// Path for embeddings endpoint.
    fn embeddings_path(&self) -> &str {
        "/embeddings"
    }

    /// Path for list models endpoint.
    fn models_path(&self) -> &str {
        "/models"
    }

    /// URL of the next page of a list models reply, or `None` when the reply is the last page.
    ///
    /// `url` is the URL that returned `page`, and `page` is the reply before
    /// [`Provider::transform_response`]. The client follows the URLs in order and joins the
    /// pages. The default is `None`: the provider lists every model in one reply.
    fn next_models_page_url(&self, _url: &str, _page: &serde_json::Value) -> Option<String> {
        None
    }

    /// Path for image generations endpoint.
    fn image_generations_path(&self) -> &str {
        "/images/generations"
    }

    /// Path for text-to-speech endpoint.
    fn audio_speech_path(&self) -> &str {
        "/audio/speech"
    }

    /// Path for audio transcription endpoint.
    fn audio_transcriptions_path(&self) -> &str {
        "/audio/transcriptions"
    }

    /// Path for content moderation endpoint.
    fn moderations_path(&self) -> &str {
        "/moderations"
    }

    /// Path for document reranking endpoint.
    fn rerank_path(&self) -> &str {
        "/rerank"
    }

    /// Path for the files management endpoint (e.g. POST /files, GET /files/{id}).
    fn files_path(&self) -> &str {
        "/files"
    }

    /// Path for the batches management endpoint (e.g. POST /batches, GET /batches/{id}).
    fn batches_path(&self) -> &str {
        "/batches"
    }

    /// Path for the responses endpoint (e.g. POST /responses).
    fn responses_path(&self) -> &str {
        "/responses"
    }

    /// Path for the web/document search endpoint.
    fn search_path(&self) -> &str {
        "/search"
    }

    /// Path for the OCR (optical character recognition) endpoint.
    fn ocr_path(&self) -> &str {
        "/ocr"
    }

    /// Whether streaming is supported.
    #[allow(dead_code)]
    fn supports_streaming(&self) -> bool {
        true
    }

    /// Transform the request body before sending, if needed.
    fn transform_request(&self, body: &mut serde_json::Value) -> Result<()> {
        let _ = body;
        Ok(())
    }

    /// Transform the body of an embedding request before sending.
    ///
    /// The client calls this, and not [`Self::transform_request`], for the embeddings endpoint.
    /// The default runs `transform_request`. A provider with its own embedding body overrides
    /// it, so the choice depends on the endpoint and never on the shape of the body.
    fn transform_embedding_request(&self, body: &mut serde_json::Value) -> Result<()> {
        self.transform_request(body)
    }

    /// Transform the raw response JSON of an embedding request before deserialization.
    ///
    /// The client calls this, and not [`Self::transform_response`], for the embeddings
    /// endpoint, after the request body is built. `request_model` is the model of the request.
    /// The default runs `transform_response` and leaves the reply `model` as the provider sent
    /// it. A provider whose embed reply has no `model` overrides it.
    fn transform_embedding_response(&self, body: &mut serde_json::Value, request_model: &str) -> Result<()> {
        let _ = request_model;
        self.transform_response(body)
    }

    /// Transform the raw response JSON before deserialization into canonical types.
    ///
    /// Providers returning non-OpenAI formats (Anthropic, Bedrock, Vertex) override
    /// this to normalize their native response into OpenAI-compatible JSON.
    /// The default implementation is a no-op (OpenAI-compatible responses pass through
    /// unchanged).
    fn transform_response(&self, _body: &mut serde_json::Value) -> Result<()> {
        Ok(())
    }

    /// Rewrite an error built from an HTTP status reply before the caller sees it.
    ///
    /// The default implementation returns the error unchanged.
    fn transform_error(&self, error: crate::error::LiterLlmError) -> crate::error::LiterLlmError {
        error
    }

    /// Transform the raw rerank reply before deserialization into `RerankResponse`.
    ///
    /// The canonical shape is a top-level `results` array. Providers whose rerank
    /// reply differs (a different key, a different `document` form) override this
    /// to reshape it. Rerank replies do not pass through [`Provider::transform_response`]:
    /// that step reshapes chat replies, and a chat transform applied to a rerank
    /// reply discards the `results` array. The default is a no-op.
    fn transform_rerank_response(&self, _body: &mut serde_json::Value) -> Result<()> {
        Ok(())
    }

    /// Transform the rerank request body after the common fields are set.
    ///
    /// Providers that name a rerank field differently from the canonical request (for
    /// example the field that limits the number of results) override this to rename it.
    /// It runs for rerank requests only, so it never sees a chat or embedding body.
    /// The default is a no-op.
    fn transform_rerank_request(&self, _body: &mut serde_json::Value) -> Result<()> {
        Ok(())
    }

    /// Build the full URL for a specific endpoint and model.
    ///
    /// Default: `{base_url}{endpoint_path}`.  Providers like Azure and Bedrock
    /// override this to embed deployment names, model IDs, or query parameters
    /// into the URL.
    fn build_url(&self, endpoint_path: &str, _model: &str) -> String {
        join_url(self.base_url(), endpoint_path)
    }

    /// Parse a single SSE event data string into a `ChatCompletionChunk`.
    ///
    /// Default: OpenAI format (straight JSON parse).
    /// Anthropic and Vertex override for their native streaming event formats.
    ///
    /// The `[DONE]` sentinel is handled at the SSE parser level before this
    /// method is called, so implementations do not need to check for it.
    ///
    /// Returns `Ok(Some(chunk))` for a successfully parsed event.
    /// Returns `Ok(None)` to skip this event (continue reading the stream).
    /// Returns `Err` when the event cannot be parsed.
    fn parse_stream_event(&self, event_data: &str) -> Result<Option<crate::types::ChatCompletionChunk>> {
        // ~keep An OpenAI-compatible provider may accept a stream (HTTP 200,
        // `text/event-stream`) and then abort mid-stream by sending a `data:`
        // line carrying an error object — `{"error": {"message": ..., "code":
        // ..., "status_code": ...}}` — before closing the connection. Groq does
        // this when a model emits a malformed tool call. The status lives in the
        // payload, not the response code, so it must be recovered here.
        //
        // This check must run BEFORE deserialization: `ChatCompletionChunk`
        // defaults every field (so providers may omit `id`/`choices`/etc. on
        // legitimate events — see #155), which means an error object would
        // otherwise decode into a valid-looking, content-free chunk and the
        // failure would vanish, leaving callers awaiting content that is never
        // sent.
        //
        // ~keep The error kind comes from `LiterLlmError::from_status`, the same
        // classifier the HTTP path uses, so an error event is the same kind
        // mid-stream as the same body in an HTTP reply. A payload status in
        // 400..=599 goes to that classifier as is. A payload status outside that
        // range keeps its raw number in a bad-request error. With no usable status,
        // `status_for_error_kind` reads one from the error code or type.
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(event_data)
            && let Some(err) = stream_error_event(&value, status_code_field(&value))
        {
            return Err(err);
        }

        serde_json::from_str::<crate::types::ChatCompletionChunk>(event_data)
            .map(Some)
            .map_err(|e| LiterLlmError::Streaming {
                message: format!("failed to parse SSE data: {e}"),
            })
    }

    /// The streaming wire format this provider uses.
    ///
    /// Default: [`StreamFormat::Sse`].  Override for providers that use
    /// non-SSE framing (e.g. AWS Bedrock EventStream).
    fn stream_format(&self) -> StreamFormat {
        StreamFormat::Sse
    }

    /// Whether every complete chat stream from this provider ends with an event that carries a
    /// finish reason, so a stream that ends without one was cut off.
    ///
    /// Default: `false`. The client then treats a clean end of the stream as a normal end, as
    /// OpenAI-compatible servers need (#155, #295). A provider whose own stream protocol always
    /// closes with a final event overrides this, and the client reports a stream that ends
    /// without one as [`LiterLlmError::Streaming`](crate::error::LiterLlmError::Streaming).
    fn stream_has_final_event(&self) -> bool {
        false
    }

    /// Whether each chat stream event numbers its tool calls from 0.
    ///
    /// Default: `false`. A provider that overrides this sends each tool call once, complete, in
    /// an event of its own, and the client continues the numbering across events and ends a
    /// stream that made tool calls with `tool_calls`.
    fn stream_tool_call_indexes_restart(&self) -> bool {
        false
    }

    /// Build the full URL for a streaming request.
    ///
    /// Default: delegates to [`Provider::build_url`].  Providers whose
    /// streaming endpoint differs from the non-streaming one (e.g. Bedrock
    /// uses `/converse-stream` vs `/converse`) override this.
    fn build_stream_url(&self, endpoint_path: &str, model: &str) -> String {
        self.build_url(endpoint_path, model)
    }

    /// Compute dynamic signing headers for the outgoing request.
    ///
    /// Called by the client just before sending each request.  The default
    /// implementation returns an empty vector (no extra signing required).
    ///
    /// Providers that use request-signing (e.g. AWS Bedrock with SigV4) override
    /// this to return the computed `Authorization`, `x-amz-date`, and
    /// `x-amz-security-token` headers.  The returned headers are merged with the
    /// provider's static [`Provider::extra_headers`] before the request is sent.
    ///
    /// # Arguments
    ///
    /// - `method`: HTTP method string, e.g. `"POST"`.
    /// - `url`: Full request URL including path and query string.
    /// - `body`: Serialised request body bytes (used in the payload hash).
    ///
    /// # Errors
    ///
    /// Returns an error if the signing headers cannot be computed (e.g. an
    /// internal signing-library failure). The default implementation never fails.
    fn signing_headers(&self, method: &str, url: &str, body: &[u8]) -> Result<Vec<(String, String)>> {
        let _ = (method, url, body);
        Ok(vec![])
    }

    /// Sign with the credential resolved for this request, when configured.
    ///
    /// The default preserves static signing for providers without dynamic signing support.
    ///
    /// # Errors
    ///
    /// Returns an error when credentials are unsupported or request signing fails.
    fn signing_headers_with_credential(
        &self,
        method: &str,
        url: &str,
        body: &[u8],
        credential: Option<&crate::auth::Credential>,
    ) -> Result<Vec<(String, String)>> {
        let _ = credential;
        self.signing_headers(method, url, body)
    }
}

/// Keep the caller's `extra_body` on a body a provider transform rebuilds.
///
/// A rebuild drops every OpenAI field. The client merges `extra_body` into the
/// body after the provider transform, so the caller's keys must survive it.
pub(crate) fn carry_extra_body(from: &mut serde_json::Value, to: &mut serde_json::Value) {
    if let Some(extra_body) = from.get_mut("extra_body").map(serde_json::Value::take) {
        to["extra_body"] = extra_body;
    }
}

pub(crate) mod anthropic;
pub(crate) mod azure;
pub(crate) mod bedrock;
pub(crate) mod cohere;
pub mod custom;
pub(crate) mod github_copilot;
pub(crate) mod google_ai;
pub(crate) mod mistral;
pub(crate) mod ollama;
pub mod outbound_policy;
pub(crate) mod rebased;
pub(crate) mod rerank_shape;
pub(crate) mod vertex;
pub(crate) mod voyage;

#[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
pub use outbound_policy::configure_outbound_client_builder;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub(crate) use outbound_policy::outbound_forbidden_from_reqwest;
pub use outbound_policy::{
    OutboundPolicy, current_policy, set_outbound_policy, validate_outbound_url, validate_outbound_url_sync,
};
#[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
pub(crate) use outbound_policy::{authenticated_outbound_client, configure_credential_free_outbound_client_builder};

/// Built-in OpenAI provider.
pub(crate) struct OpenAiProvider;

impl Provider for OpenAiProvider {
    /// OpenAI's request format at another host is the generic OpenAI-compatible provider.
    ///
    /// The `max_tokens` rename to `max_completion_tokens` belongs to OpenAI's own service. A
    /// local server or a gateway at `base_url` keeps the `max_tokens` the caller sent.
    fn at_base_url(&self, base_url: &str) -> Option<Box<dyn Provider>> {
        Some(Box::new(OpenAiCompatibleProvider {
            name: "custom".into(),
            base_url: base_url.to_owned(),
            env_var: None,
            model_prefixes: vec![],
            hint_provider: Some("openai".into()),
        }))
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn base_url(&self) -> &str {
        "https://api.openai.com/v1"
    }

    fn env_var(&self) -> Option<&str> {
        Some("OPENAI_API_KEY")
    }

    fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
        Some((Cow::Borrowed("Authorization"), Cow::Owned(format!("Bearer {api_key}"))))
    }

    fn matches_model(&self, model: &str) -> bool {
        model.starts_with("gpt-")
            || model.starts_with("o1-")
            || model.starts_with("o3-")
            || model.starts_with("o4-")
            || model == "o1"
            || model == "o3"
            || model == "o4"
            || model.starts_with("dall-e-")
            || model.starts_with("whisper-")
            || model.starts_with("tts-")
            || model.starts_with("text-embedding-")
            || model.starts_with("chatgpt-")
            || model.starts_with("openai/")
    }

    fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
        model.strip_prefix("openai/").unwrap_or(model)
    }

    /// Hoist OpenAI audio response data into the `content` field.
    ///
    /// OpenAI audio-preview models return `choices[].message.audio = { data,
    /// transcript, format, ... }` alongside a null `content`.  Normalise this
    /// into `choices[].message.content` as a parts array so that
    /// [`AssistantContent::Parts`] deserialization works correctly.
    ///
    /// Layout emitted:
    /// - A `{ "type": "text", "text": transcript }` part when `transcript` is
    ///   present and non-empty.
    /// - A `{ "type": "output_audio", "audio": { "data": …, "format": … } }` part.
    ///
    /// For non-audio responses this is a no-op.
    fn transform_response(&self, body: &mut serde_json::Value) -> Result<()> {
        transform_openai_audio_response(body)
    }
}

/// Hoist an OpenAI audio response's `message.audio` field into `message.content`.
///
/// Mutates each choice in-place; returns `Ok(())` unconditionally.
pub(crate) fn transform_openai_audio_response(body: &mut serde_json::Value) -> Result<()> {
    use serde_json::json;

    let choices = match body.get_mut("choices").and_then(|c| c.as_array_mut()) {
        Some(c) => c,
        None => return Ok(()),
    };

    for choice in choices {
        let audio = match choice.pointer("/message/audio") {
            Some(a) if !a.is_null() => a.clone(),
            _ => continue,
        };

        let data = audio.get("data").and_then(|v| v.as_str()).unwrap_or("").to_owned();
        let format = audio.get("format").and_then(|v| v.as_str()).unwrap_or("wav").to_owned();
        let transcript = audio
            .get("transcript")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();

        let mut parts: Vec<serde_json::Value> = vec![];
        if !transcript.is_empty() {
            parts.push(json!({"type": "text", "text": transcript}));
        }
        if !data.is_empty() {
            parts.push(json!({
                "type": "output_audio",
                "audio": {"data": data, "format": format}
            }));
        }

        if !parts.is_empty()
            && let Some(message) = choice.get_mut("message")
        {
            message["content"] = json!(parts);
        }
    }

    Ok(())
}

/// A generic OpenAI-compatible provider (configurable base_url + bearer auth).
pub(crate) struct OpenAiCompatibleProvider {
    pub name: String,
    pub base_url: String,
    /// Environment variable name for the API key, if known.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub env_var: Option<&'static str>,
    pub model_prefixes: Vec<String>,
    /// Provider named by the client's `model_hint`; its `name/` prefix is stripped from models.
    pub hint_provider: Option<String>,
}

impl Provider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn env_var(&self) -> Option<&str> {
        self.env_var
    }

    fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
        Some((Cow::Borrowed("Authorization"), Cow::Owned(format!("Bearer {api_key}"))))
    }

    fn matches_model(&self, model: &str) -> bool {
        self.model_prefixes
            .iter()
            .any(|prefix| model.starts_with(prefix.as_str()))
    }

    /// Strip a leading `X/` only when `X` is this provider's own name or the provider
    /// named by the client's `model_hint`.
    ///
    /// This provider fronts a user-supplied `base_url`. Gateways there (OpenRouter, a
    /// liter-llm or LiteLLM proxy) route on the prefix, so `openai/gpt-4o` must reach them
    /// verbatim unless the caller declared via `model_hint` that the endpoint is that
    /// provider. Only the first segment is removed.
    fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
        if let Some((prefix, rest)) = model.split_once('/')
            && !rest.is_empty()
            && (prefix == self.name || self.hint_provider.as_deref() == Some(prefix))
        {
            return rest;
        }
        model
    }
}

/// Provider name designated by a `model_hint` (`"openai"` or `"openai/gpt-4o-mini"` -> `"openai"`).
///
/// Uses the same `provider/` split as [`detect_provider`].
pub(crate) fn hint_provider_name(model_hint: &str) -> Option<String> {
    let name = model_hint.split_once('/').map_or(model_hint, |(prefix, _)| prefix);
    (!name.is_empty()).then(|| name.to_owned())
}

/// A data-driven provider backed by a [`ProviderConfig`] entry from providers.json.
///
/// Used for simple providers that are fully described by their JSON config.
/// Complex providers (AWS Bedrock, Vertex AI, etc.) use dedicated implementations.
///
/// # Construction
///
/// Construct only via [`ConfigDrivenProvider::new`], which is intentionally
/// `pub(crate)` — callers outside this crate must go through [`detect_provider`].
///
/// # `base_url` contract
///
/// [`Provider::base_url`] returns an empty string when the provider config has
/// no `base_url` entry.  This is safe because [`detect_provider`] guards the
/// `base_url.is_some()` condition before constructing a `ConfigDrivenProvider`,
/// so a correctly-routed request will never produce an empty URL.  A manually
/// constructed instance (hypothetically) would produce a clearly-broken URL
/// (`/chat/completions`) that fails immediately at the HTTP layer.
pub(crate) struct ConfigDrivenProvider {
    config: &'static ProviderConfig,
    aliases: &'static [String],
    endpoint_urls: Option<&'static HashMap<String, String>>,
    unrouted_endpoints: &'static [Endpoint],
}

impl ConfigDrivenProvider {
    #[cfg(test)]
    #[must_use]
    pub(crate) fn new(config: &'static ProviderConfig) -> Self {
        Self {
            config,
            aliases: &[],
            endpoint_urls: None,
            unrouted_endpoints: &[],
        }
    }

    fn from_entry(entry: &'static ProviderEntry) -> Self {
        Self {
            config: &entry.config,
            aliases: &entry.aliases,
            endpoint_urls: Some(&entry.endpoint_urls),
            unrouted_endpoints: &entry.no_route_endpoints,
        }
    }
}

impl Provider for ConfigDrivenProvider {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn unrouted_endpoints(&self) -> &[Endpoint] {
        self.unrouted_endpoints
    }

    fn base_url(&self) -> &str {
        self.config.base_url.as_deref().unwrap_or("")
    }

    fn env_var(&self) -> Option<&str> {
        self.config.auth.as_ref().and_then(|a| a.env_var.as_deref())
    }

    /// The rerank path of the provider: `/ranking` for NVIDIA NIM, `/rerank` for the rest.
    fn rerank_path(&self) -> &str {
        rerank_shape::rerank_path_of(&self.config.name)
    }

    fn build_url(&self, endpoint_path: &str, _model: &str) -> String {
        if endpoint_path == self.rerank_path()
            && let Some(url) = self.endpoint_urls.and_then(|urls| urls.get("rerank"))
        {
            return url.clone();
        }
        format!("{}{}", self.base_url(), endpoint_path)
    }

    fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
        match model.split_once('/') {
            Some((prefix, rest)) if prefix == self.config.name || self.aliases.iter().any(|alias| alias == prefix) => {
                rest
            }
            _ => model,
        }
    }

    /// Reshape the rerank reply of a provider that has a row in [`rerank_shape`].
    fn transform_rerank_response(&self, body: &mut serde_json::Value) -> Result<()> {
        rerank_shape::reshape_rerank_reply(&self.config.name, body);
        Ok(())
    }

    /// Word the rerank request as the provider reads it.
    fn transform_rerank_request(&self, body: &mut serde_json::Value) -> Result<()> {
        rerank_shape::reshape_rerank_request(&self.config.name, body);
        Ok(())
    }

    fn transform_request(&self, body: &mut serde_json::Value) -> Result<()> {
        if let Some(mappings) = &self.config.param_mappings
            && let Some(obj) = body.as_object_mut()
        {
            for (from, to) in mappings {
                if let Some(val) = obj.remove(from.as_str()) {
                    obj.insert(to.clone(), val);
                }
            }
        }
        Ok(())
    }

    fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
        let auth_type = self
            .config
            .auth
            .as_ref()
            .map(|a| &a.auth_type)
            .unwrap_or(&AuthType::Bearer);

        match auth_type {
            AuthType::None => None,
            AuthType::ApiKey => Some((Cow::Borrowed("x-api-key"), Cow::Borrowed(api_key))),
            AuthType::Bearer | AuthType::Unknown => {
                Some((Cow::Borrowed("Authorization"), Cow::Owned(format!("Bearer {api_key}"))))
            }
        }
    }

    fn matches_model(&self, model: &str) -> bool {
        if let Some(prefixes) = &self.config.model_prefixes {
            prefixes.iter().any(|p| model.starts_with(p.as_str()))
        } else {
            false
        }
    }
}

/// Find the registry entry named by a `provider/` prefix, by name or alias.
fn registry_entry_for_prefix<'r>(reg: &'r ProviderRegistry, prefix: &str) -> Option<&'r ProviderEntry> {
    reg.providers
        .iter()
        .find(|e| e.config.name == prefix)
        .or_else(|| reg.providers.iter().find(|e| e.aliases.iter().any(|a| a == prefix)))
}

/// Whether the first `/`-separated segment of `model` names a provider: a registry entry by
/// name or alias, a built-in provider, or a registered custom provider.
///
/// A bare model has none, and a slash alone makes no prefix: `hf.co/user/model` is bare, and
/// so is `acme/m` until a custom provider named `acme` is registered; `openai/m` and
/// `snowflake/m` are not.
pub(crate) fn has_provider_prefix(model: &str) -> bool {
    let Some((prefix, _)) = model.split_once('/') else {
        return false;
    };
    // ~keep `google_ai/` is the one built-in prefix without a registry entry: it is the second
    // name of the `gemini` provider in `detect_builtin_provider`.
    prefix == "google_ai"
        || REGISTRY
            .as_ref()
            .is_ok_and(|reg| registry_entry_for_prefix(reg, prefix).is_some())
        || custom::is_custom_provider_prefix(prefix, model)
}

/// A registry entry has a route of its own when it names a `base_url` and is not a
/// complex provider (complex providers need dedicated auth and request code, matched
/// by prefix before the registry lookup in [`detect_provider`]).
fn has_route(reg: &ProviderRegistry, entry: &ProviderEntry) -> bool {
    entry.config.base_url.is_some() && !reg.complex_providers.contains(&entry.config.name)
}

/// Like [`detect_provider`], but refuses a model whose `provider/` prefix names a registry
/// provider that has no route.
///
/// Such a model would otherwise fall back to [`OpenAiProvider`], sending another
/// provider's requests to the OpenAI endpoint. Callers that have no `base_url` use this
/// instead of [`detect_provider`] and fall back to OpenAI only on `Ok(None)`, which means
/// the model names no registry provider at all.
///
/// # Errors
///
/// Returns [`LiterLlmError::BadRequest`] naming the provider and asking for a `base_url`.
pub(crate) fn route_model(model: &str) -> Result<Option<Box<dyn Provider>>> {
    route_detected(model, detect_provider(model))
}

/// Like [`route_model`], but skips custom providers: only a built-in provider is selected.
///
/// # Errors
///
/// Returns [`LiterLlmError::BadRequest`] naming the provider and asking for a `base_url`.
fn route_builtin_model(model: &str) -> Result<Option<Box<dyn Provider>>> {
    route_detected(model, detect_builtin_provider(model))
}

/// `detected`, or the refusal for a model whose prefix names a registry provider with no route.
fn route_detected(model: &str, detected: Option<Box<dyn Provider>>) -> Result<Option<Box<dyn Provider>>> {
    if let Some(provider) = detected {
        return Ok(Some(provider));
    }
    if let Ok(reg) = REGISTRY.as_ref()
        && let Some(error) = unrouted_provider_error(reg, model)
    {
        return Err(error);
    }
    Ok(None)
}

/// The provider that a client's `model_hint` selects, or `None` when the hint names none.
///
/// This is the one rule that turns a hint into a route. A bare provider name (`cohere`) and
/// the `provider/model` form (`cohere/command-r-plus`) select the same provider, and so does
/// a model name that [`detect_provider`] knows (`command-r-plus`). A `base_url` plays no part
/// in the choice: it changes only the host, see [`on_base_url`].
///
/// # Errors
///
/// Returns [`LiterLlmError::BadRequest`] when the hint names a registry provider that has no
/// route of its own.
pub(crate) fn route_hint(hint: &str) -> Result<Option<Box<dyn Provider>>> {
    route_hint_with(hint, route_model)
}

/// Like [`route_hint`], but selects only a built-in provider.
///
/// A client with a `base_url` uses this: the `base_url` names the host, and the hint names
/// only the wire format. A custom provider brings its own host, so it never answers a hint there.
///
/// # Errors
///
/// Returns [`LiterLlmError::BadRequest`] when the hint names a registry provider that has no
/// route of its own. A caller with a `base_url` treats that as "no built-in provider".
pub(crate) fn route_builtin_hint(hint: &str) -> Result<Option<Box<dyn Provider>>> {
    route_hint_with(hint, route_builtin_model)
}

/// Selects the provider that a model name designates.
type ModelRoute = fn(&str) -> Result<Option<Box<dyn Provider>>>;

/// The hint read as a model name first, then a bare hint read as the `hint/` prefix.
fn route_hint_with(hint: &str, route: ModelRoute) -> Result<Option<Box<dyn Provider>>> {
    if let Some(provider) = route(hint)? {
        return Ok(Some(provider));
    }
    if hint.contains('/') {
        return Ok(None);
    }
    route(&format!("{hint}/"))
}

/// Send `provider` to `base_url`: the provider keeps its request body, its paths and its
/// reply handling, and only the host changes.
pub(crate) fn on_base_url(provider: Box<dyn Provider>, base_url: &str) -> Box<dyn Provider> {
    match provider.at_base_url(base_url) {
        Some(rebased) => rebased,
        None => Box::new(rebased::BaseUrlOverride::new(provider, base_url)),
    }
}

/// `{base_url}{endpoint_path}`, with one `/` between the path of `base_url` and `endpoint_path`.
pub(crate) fn join_url(base_url: &str, endpoint_path: &str) -> String {
    let Ok(mut base) = url::Url::parse(base_url) else {
        return format!("{base_url}{endpoint_path}");
    };
    let path = format!(
        "{}/{}",
        base.path().trim_end_matches('/'),
        endpoint_path.trim_start_matches('/')
    );
    base.set_path(&path);
    base.into()
}

/// The refusal for a model whose `provider/` prefix names a registry entry with no route.
fn unrouted_provider_error(reg: &ProviderRegistry, model: &str) -> Option<LiterLlmError> {
    let (prefix, _) = model.split_once('/')?;
    let entry = registry_entry_for_prefix(reg, prefix)?;
    if has_route(reg, entry) {
        return None;
    }
    Some(LiterLlmError::BadRequest {
        message: format!(
            "provider `{}` has no built-in route; set base_url to its endpoint",
            entry.config.name
        ),
        status: 400,
    })
}

/// Refuse a call to an endpoint that the resolved provider has no route for.
///
/// The provider's other endpoints keep working. The list of refused endpoints belongs to the
/// provider built from a registry entry, so a custom provider or a provider pinned by a
/// `base_url` is never refused, whatever its name.
///
/// # Errors
///
/// Returns [`LiterLlmError::BadRequest`] naming the provider and the endpoint and asking for
/// a `base_url`.
pub(crate) fn refuse_unrouted_endpoint(provider: &dyn Provider, endpoint: Endpoint) -> Result<()> {
    if !provider.unrouted_endpoints().contains(&endpoint) {
        return Ok(());
    }
    Err(LiterLlmError::BadRequest {
        message: format!(
            "provider `{}` has no built-in route for {}; set base_url to its endpoint",
            provider.name(),
            endpoint.as_str()
        ),
        status: 400,
    })
}

/// Detect which provider to use based on model name.
///
/// Strategy:
/// 1. OpenAI hardcoded patterns (gpt-*, o1-*, text-embedding-*, …).
/// 2. Anthropic: `claude-*` model names or `anthropic/` prefix.
/// 3. Azure: `azure/` prefix.
/// 4. Google AI Studio: `gemini/` or `google_ai/` prefix.
/// 5. Vertex AI: `vertex_ai/` prefix.
/// 6. AWS Bedrock: `bedrock/` prefix.
/// 7. Ollama (native API): `ollama/` or `ollama_chat/` prefix.
/// 8. `"provider/"` prefix — look up the prefix in the registry.
/// 9. Walk all registry entries and check their `model_prefixes`.
///
/// Returns `None` when no built-in provider matches.  Callers without a `base_url` use
/// [`route_model`], which refuses a registry provider that has no route before the caller
/// defaults to [`OpenAiProvider`].
///
/// Complex providers (those listed in `complex_providers` in providers.json)
/// are excluded from config-driven routing because they require custom
/// auth/request logic beyond simple bearer tokens.
pub(crate) fn detect_provider(model: &str) -> Option<Box<dyn Provider>> {
    custom::detect_custom_provider(model).or_else(|| detect_builtin_provider(model))
}

/// Detect the built-in provider that a model name designates, skipping custom providers.
///
/// A custom provider decides where a request goes and how it is signed. The wire format
/// of a reply belongs to the model name, so reply handling asks this function.
///
/// Every prefix test here is a case-sensitive `starts_with` (or an exact match on the
/// registry name), so `Voyage/rerank-2` names no provider.
pub(crate) fn detect_builtin_provider(model: &str) -> Option<Box<dyn Provider>> {
    let openai = OpenAiProvider;
    if openai.matches_model(model) {
        return Some(Box::new(openai));
    }

    let anthropic = anthropic::AnthropicProvider::default();
    if anthropic.matches_model(model) {
        return Some(Box::new(anthropic));
    }

    if model.starts_with("azure/") {
        return Some(Box::new(azure::AzureProvider::new()));
    }

    if model.starts_with("gemini/") || model.starts_with("google_ai/") {
        return Some(Box::new(google_ai::GoogleAiProvider::default()));
    }

    if model.starts_with("vertex_ai/") {
        return Some(Box::new(vertex::VertexAiProvider::from_env()));
    }

    if model.starts_with("bedrock/") {
        return Some(Box::new(bedrock::BedrockProvider::from_env()));
    }

    if cohere::CohereProvider.matches_model(model) {
        return Some(Box::new(cohere::CohereProvider));
    }

    if model.starts_with("mistral-")
        || model.starts_with("codestral-")
        || model.starts_with("pixtral-")
        || model.starts_with("mistral/")
    {
        return Some(Box::new(mistral::MistralProvider));
    }

    if ollama::OllamaProvider::owns(model) {
        return Some(Box::new(ollama::OllamaProvider::default()));
    }

    if model.starts_with("voyage/") {
        return Some(Box::new(voyage::VoyageProvider));
    }

    if model.starts_with("github_copilot/") {
        return Some(Box::new(github_copilot::GithubCopilotProvider::from_env()));
    }

    let reg = match REGISTRY.as_ref() {
        Ok(r) => r,
        Err(_) => return None,
    };

    if let Some((prefix, _)) = model.split_once('/')
        && let Some(entry) = registry_entry_for_prefix(reg, prefix)
        && has_route(reg, entry)
    {
        return Some(Box::new(ConfigDrivenProvider::from_entry(entry)));
    }

    for entry in &reg.providers {
        if reg.complex_providers.contains(&entry.config.name) {
            continue;
        }
        if let Some(prefixes) = &entry.config.model_prefixes {
            let matches = prefixes
                .iter()
                .any(|p| model.starts_with(p.as_str()) && !p.ends_with('/'));
            if matches && entry.config.base_url.is_some() {
                return Some(Box::new(ConfigDrivenProvider::from_entry(entry)));
            }
        }
    }

    None
}

/// Return all provider configs from the registry.
///
/// Useful for tooling, documentation generation, or runtime enumeration.
/// Returns the public [`ProviderConfig`] slice (without capability flags).
/// To query capability flags for a specific provider use [`capabilities`].
pub fn all_providers() -> Result<&'static [ProviderConfig]> {
    Ok(&registry()?.configs)
}

/// Return the set of complex provider names.
///
/// Complex providers require custom auth/routing logic beyond simple bearer
/// tokens (e.g. AWS Bedrock SigV4, Vertex AI OAuth2).
///
/// The returned reference points into the static registry — no allocation.
pub fn complex_provider_names() -> Result<&'static HashSet<String>> {
    Ok(&registry()?.complex_providers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct RenamingProvider;

    impl Provider for RenamingProvider {
        fn name(&self) -> &str {
            "renaming"
        }
        fn base_url(&self) -> &str {
            "https://example.invalid"
        }
        fn auth_header<'a>(&'a self, _api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
            None
        }
        fn matches_model(&self, _model: &str) -> bool {
            false
        }
        fn transform_request(&self, body: &mut serde_json::Value) -> Result<()> {
            body["renamed"] = json!(true);
            Ok(())
        }
    }

    #[test]
    fn transform_embedding_response_keeps_the_reply_model_by_default() {
        let mut body = json!({"object": "list", "data": [], "model": ""});
        RenamingProvider
            .transform_embedding_response(&mut body, "request-model")
            .expect("the default hook should not fail");
        assert_eq!(body["model"], "");
    }

    #[test]
    fn transform_embedding_request_runs_transform_request_by_default() {
        let mut body = json!({"model": "m", "input": "x"});
        RenamingProvider
            .transform_embedding_request(&mut body)
            .expect("the default hook should not fail");
        assert_eq!(body["renamed"], true);
    }

    /// Revert line: delete the `if value < min || value > max { ... }` check inside
    /// `validate_sampling_param_range` (or make it always return `Ok(())`) to make
    /// this test fail.
    #[test]
    fn validate_sampling_param_range_rejects_out_of_range_value() {
        let body = json!({"temperature": 1.8});
        let err = validate_sampling_param_range(&body, "temperature", "Anthropic", 0.0, 1.0)
            .expect_err("1.8 is outside [0.0, 1.0]");
        assert_eq!(err.status_code(), 400);
        assert_eq!(
            err.to_string(),
            "bad request: temperature=1.8 is outside Anthropic's supported range [0, 1]; lower \
             the requested value or omit `temperature` to use the provider default"
        );
    }

    #[test]
    fn validate_sampling_param_range_accepts_in_range_value() {
        let body = json!({"temperature": 0.5});
        assert!(validate_sampling_param_range(&body, "temperature", "Anthropic", 0.0, 1.0).is_ok());
    }

    #[test]
    fn validate_sampling_param_range_ignores_absent_field() {
        let body = json!({"model": "claude-3-5-sonnet"});
        assert!(validate_sampling_param_range(&body, "temperature", "Anthropic", 0.0, 1.0).is_ok());
    }

    /// Every provider that authenticates with a static API key must name the
    /// environment variable that key is conventionally read from.
    ///
    /// `env_var` is the sole gate in `DefaultClient::new`, and it is checked
    /// with `if let Some(..)` — so a provider returning `None` does not merely
    /// skip auto-loading, it skips the "no API key" error too.  The client is
    /// built with an empty key and the failure only surfaces as a 401 on the
    /// first request.  Bedrock, Vertex AI and GitHub Copilot legitimately
    /// return `None`: their credentials come from SigV4, ADC and an OAuth
    /// device-flow exchange respectively, not from a key variable.
    #[test]
    fn static_key_providers_declare_their_env_var() {
        let cases: Vec<(&str, Box<dyn Provider>)> = vec![
            ("OPENAI_API_KEY", Box::new(OpenAiProvider)),
            (
                "ANTHROPIC_API_KEY",
                Box::new(super::anthropic::AnthropicProvider::new()),
            ),
            ("COHERE_API_KEY", Box::new(super::cohere::CohereProvider)),
            ("MISTRAL_API_KEY", Box::new(super::mistral::MistralProvider)),
            (
                "GEMINI_API_KEY",
                Box::new(super::google_ai::GoogleAiProvider::default()),
            ),
            (
                "AZURE_OPENAI_API_KEY",
                Box::new(super::azure::AzureProvider::with_base_url("https://x.openai.azure.com")),
            ),
        ];

        for (expected, provider) in cases {
            assert_eq!(
                provider.env_var(),
                Some(expected),
                "{} must declare {expected}",
                provider.name()
            );
        }
    }

    #[test]
    fn minimax_regional_endpoints_are_registered() {
        let registry = registry().expect("registry should load");
        let provider = registry
            .providers
            .iter()
            .find(|entry| entry.config.name == "minimax")
            .expect("MiniMax provider should be registered");

        assert!(provider.capabilities.reasoning);
        assert_eq!(
            provider.regional_endpoints,
            vec![
                RegionalEndpointConfig {
                    region: "global_en".into(),
                    openai_base_url: "https://api.minimax.io/v1".into(),
                    anthropic_base_url: "https://api.minimax.io/anthropic".into(),
                    docs_root: "https://platform.minimax.io/docs".into(),
                },
                RegionalEndpointConfig {
                    region: "cn_zh".into(),
                    openai_base_url: "https://api.minimaxi.com/v1".into(),
                    anthropic_base_url: "https://api.minimaxi.com/anthropic".into(),
                    docs_root: "https://platform.minimaxi.com/docs".into(),
                },
            ]
        );
    }

    /// `OpenAiProvider::transform_request` is a no-op — the wire body is exactly what
    /// serde produces from the typed `Message`/`ToolMessage` API. Asserts the exact
    /// serialized JSON shape of a `Parts` tool result: OpenAI's chat-completions
    /// `content` array accepts `image_url` parts inside a tool message verbatim.
    #[test]
    fn openai_tool_result_image_part_serializes_to_exact_content_array() {
        use crate::types::{ContentPart, Message, ToolMessage, UserContent};

        let message = Message::Tool(ToolMessage {
            content: UserContent::Parts(vec![
                ContentPart::text("Here is a screenshot:"),
                ContentPart::image_data_url("data:image/png;base64,abc123"),
            ]),
            tool_call_id: "call_shot".into(),
            name: None,
        });
        let mut body = json!({"model": "gpt-4o", "messages": [message]});

        OpenAiProvider
            .transform_request(&mut body)
            .expect("transform_request should not fail");

        assert_eq!(
            body["messages"][0],
            json!({
                "role": "tool",
                "content": [
                    {"type": "text", "text": "Here is a screenshot:"},
                    {"type": "image_url", "image_url": {"url": "data:image/png;base64,abc123"}}
                ],
                "tool_call_id": "call_shot"
            })
        );
    }

    #[test]
    fn transform_response_audio_message_hoisted() {
        let mut body = json!({
            "id": "chatcmpl-audio-123",
            "object": "chat.completion",
            "created": 1700000000u64,
            "model": "gpt-4o-audio-preview",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "audio": {
                        "id": "audio-abc",
                        "data": "aGVsbG8=",
                        "transcript": "hello",
                        "format": "wav",
                        "expires_at": 9999999999u64
                    }
                },
                "finish_reason": "stop"
            }]
        });

        transform_openai_audio_response(&mut body).expect("transform must succeed");

        let content = body
            .pointer("/choices/0/message/content")
            .expect("content must be present");
        assert!(content.is_array(), "content must be a parts array, got: {content}");
        let parts = content.as_array().expect("array");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0]["type"], "text");
        assert_eq!(parts[0]["text"], "hello");
        assert_eq!(parts[1]["type"], "output_audio");
        assert_eq!(parts[1]["audio"]["data"], "aGVsbG8=");
        assert_eq!(parts[1]["audio"]["format"], "wav");
    }

    #[test]
    fn transform_response_audio_no_op_when_no_audio_field() {
        let mut body = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "plain text"
                }
            }]
        });
        let original = body.clone();
        transform_openai_audio_response(&mut body).expect("transform must succeed");
        assert_eq!(body, original, "body must be unchanged when no audio field is present");
    }

    #[test]
    fn parse_stream_event_tolerates_trailing_metadata_event_without_id() {
        // ~keep OpenCode Zen/Go emits an `inference-cost` event (no `id`, empty
        // `choices`) right before `[DONE]`; the default parser must yield an empty
        // chunk rather than aborting the stream with `missing field 'id'` (#155).
        let payload = r#"{"choices":[],"x-opencode-type":"inference-cost","cost":"0.00001400","normalizedUsage":{"inputTokens":84,"outputTokens":8}}"#;
        let chunk = OpenAiProvider
            .parse_stream_event(payload)
            .expect("trailing metadata event must not error")
            .expect("event should decode to a chunk, not be skipped");
        assert!(chunk.id.is_empty());
        assert!(chunk.choices.is_empty());
    }

    /// Revert line: delete the `if let Ok(value) = ... && let Some(error) = ...`
    /// block in `parse_stream_event` to make this test fail.
    #[test]
    fn parse_stream_event_surfaces_mid_stream_error_object() {
        // ~keep Groq answers HTTP 200 + `text/event-stream`, then aborts the
        // stream with an error object when a model emits a malformed tool call.
        // The real status lives in the payload, so it must be recovered from
        // there rather than from the (successful) response code.
        let payload = r#"{"error":{"message":"tool call validation failed","type":"invalid_request_error","code":"tool_use_failed","status_code":400}}"#;
        let err = OpenAiProvider
            .parse_stream_event(payload)
            .expect_err("a mid-stream error object must not decode as a chunk");
        assert_eq!(err.status_code(), 400);
        assert!(
            err.to_string().contains("tool call validation failed"),
            "the provider's own message must reach the caller, got: {err}"
        );
    }

    /// Revert line: change the `status_code` fallback in `parse_stream_event`
    /// from `unwrap_or(400)` to `unwrap_or(200)` to make this test fail.
    #[test]
    fn parse_stream_event_defaults_status_when_error_object_omits_it() {
        // ~keep Not every provider includes `status_code` in the error object;
        // absent one, a 4xx is the safe reading — the request is over and it did
        // not succeed, so it must not be reported as a 2xx.
        let payload = r#"{"error":{"message":"upstream capacity exceeded"}}"#;
        let err = OpenAiProvider
            .parse_stream_event(payload)
            .expect_err("an error object without a status must still be an error");
        assert_eq!(err.status_code(), 400);
        assert!(err.to_string().contains("upstream capacity exceeded"));
    }

    /// Parse a mid-stream error event and return the error it raises.
    fn mid_stream_error(payload: &str) -> LiterLlmError {
        OpenAiProvider
            .parse_stream_event(payload)
            .expect_err("a mid-stream error object must not decode as a chunk")
    }

    /// Revert line: drop the `retry_after_field` argument of `from_status` in
    /// `stream_error_event` to make this test fail. The delay is read from the error
    /// object, then from the top level of the event.
    #[test]
    fn parse_stream_event_reads_retry_after_from_error_event() {
        use std::time::Duration;
        let cases = [
            (
                r#"{"error":{"message":"slow down","type":"rate_limit_error","retry_after":7}}"#,
                Some(Duration::from_secs(7)),
            ),
            (
                r#"{"error":{"message":"slow down","type":"rate_limit_error"},"retry_after":3}"#,
                Some(Duration::from_secs(3)),
            ),
            (
                r#"{"error":{"message":"slow down","type":"rate_limit_error","retry_after":1.5}}"#,
                Some(Duration::from_millis(1500)),
            ),
            // The error object wins over the top level.
            (
                r#"{"error":{"message":"x","type":"rate_limit_error","retry_after":7},"retry_after":3}"#,
                Some(Duration::from_secs(7)),
            ),
            (r#"{"error":{"message":"x","type":"rate_limit_error"}}"#, None),
            (
                r#"{"error":{"message":"x","type":"rate_limit_error","retry_after":-4}}"#,
                None,
            ),
            (
                r#"{"error":{"message":"x","type":"rate_limit_error","retry_after":"soon"}}"#,
                None,
            ),
            // A string that holds a number reads like the `Retry-After` header.
            (
                r#"{"error":{"message":"x","type":"rate_limit_error","retry_after":"30"}}"#,
                Some(Duration::from_secs(30)),
            ),
            // An unusable value in the error object does not hide the top level.
            (
                r#"{"error":{"message":"x","type":"rate_limit_error","retry_after":-4},"retry_after":5}"#,
                Some(Duration::from_secs(5)),
            ),
            (
                r#"{"error":{"message":"x","type":"rate_limit_error","retry_after":"soon"},"retry_after":"6"}"#,
                Some(Duration::from_secs(6)),
            ),
        ];
        for (payload, want) in cases {
            let err = mid_stream_error(payload);
            assert!(
                matches!(err, LiterLlmError::RateLimited { .. }),
                "{payload}: got {err:?}"
            );
            assert_eq!(err.retry_after(), want, "{payload}");
        }
    }

    /// A null, string or negative `retry_after` in the error object does not hide a valid
    /// top-level value, and a numeric string reads as seconds. Huge values are not capped,
    /// like a `Retry-After` header on the HTTP path.
    /// Revert line: change `find_map(seconds)` to `next().and_then(seconds)` to make this
    /// test fail.
    #[test]
    fn parse_stream_event_retry_after_edge_inputs() {
        use std::time::Duration;
        let cases = [
            (r#""retry_after":null"#, Some(Duration::from_secs(9))),
            (r#""retry_after":"soon""#, Some(Duration::from_secs(9))),
            (r#""retry_after":-1"#, Some(Duration::from_secs(9))),
            (r#""retry_after":"30""#, Some(Duration::from_secs(30))),
            (r#""retry_after":"2.5""#, Some(Duration::from_millis(2500))),
        ];
        for (inner, want) in cases {
            let payload = format!(r#"{{"error":{{"message":"x","type":"rate_limit_error",{inner}}},"retry_after":9}}"#);
            let err = mid_stream_error(&payload);
            // The strings "30" and "2.5" sit in the object and win; the others fall through to 9 s.
            assert_eq!(err.retry_after(), want, "{payload}");
        }
        let err = mid_stream_error(r#"{"error":{"message":"x","type":"rate_limit_error","retry_after":1e12}}"#);
        assert_eq!(err.retry_after(), Some(Duration::from_secs(1_000_000_000_000)));
    }

    /// The top-level `status_code` of the OpenAI-compatible stream path counts only as an
    /// error status, and the error object wins over it. A string status may carry spaces.
    /// Revert line: remove `.trim()` in `u16_field` to make the padded case fail.
    /// Revert line: remove the `filter` on the top-level value in `status_code_field` to
    /// make this test fail.
    #[test]
    fn parse_stream_event_top_level_status_code_edge_inputs() {
        let cases = [
            (
                r#"{"error":{"message":"x","type":"rate_limit_error"},"status_code":200}"#,
                429,
            ),
            (r#"{"error":{"message":"x"},"status_code":503}"#, 503),
            (r#"{"error":{"message":"x","status_code":"429"}}"#, 429),
            (r#"{"error":{"message":"x","status_code":" 429 "}}"#, 429),
            (r#"{"error":{"message":"x","status_code":401},"status_code":503}"#, 401),
        ];
        for (payload, status) in cases {
            let err = mid_stream_error(payload);
            assert_eq!(err.status_code(), status, "{payload}: {err:?}");
        }
    }

    /// Revert line: remove the `billing_error`, `request_too_large` and `timeout_error`
    /// arms of `status_for_error_kind` to make this test fail. Each Anthropic error type
    /// gives the status that the Anthropic API documents for it.
    #[test]
    fn parse_stream_event_anthropic_error_types_keep_their_status() {
        for (kind, status) in [
            ("billing_error", 402),
            ("request_too_large", 413),
            ("timeout_error", 504),
        ] {
            let err = mid_stream_error(&format!(r#"{{"error":{{"message":"m","type":"{kind}"}}}}"#));
            assert_eq!(err.status_code(), status, "{kind}: {err:?}");
        }
        let err = mid_stream_error(r#"{"error":{"message":"m","type":"timeout_error"}}"#);
        assert!(matches!(err, LiterLlmError::ServiceUnavailable { .. }), "got: {err:?}");
    }

    /// An overloaded error event keeps its delay too: the 529 kind carries `retry_after`.
    #[test]
    fn parse_stream_event_reads_retry_after_on_overloaded_event() {
        let err = mid_stream_error(r#"{"error":{"message":"busy","type":"overloaded_error","retry_after":2}}"#);
        assert!(matches!(err, LiterLlmError::ServiceUnavailable { .. }), "got: {err:?}");
        assert_eq!(err.retry_after(), Some(std::time::Duration::from_secs(2)));
    }

    /// Revert line: in `parse_stream_event`, build `BadRequest` directly instead of
    /// calling `LiterLlmError::from_status` to make this test fail.
    #[test]
    fn parse_stream_event_classifies_rate_limit_error_without_status() {
        let err = mid_stream_error(
            r#"{"error":{"message":"slow down","type":"rate_limit_error","code":"rate_limit_exceeded"}}"#,
        );
        assert!(matches!(err, LiterLlmError::RateLimited { .. }), "got: {err:?}");
        assert_eq!(err.status_code(), 429);
        assert!(err.is_transient());
        assert!(err.to_string().contains("slow down"));
    }

    /// Revert line: as above, build `BadRequest` directly to make this test fail.
    #[test]
    fn parse_stream_event_classifies_rate_limit_error_with_status() {
        let err = mid_stream_error(
            r#"{"error":{"message":"slow down","type":"rate_limit_error","code":"rate_limit_exceeded","status_code":429}}"#,
        );
        assert!(matches!(err, LiterLlmError::RateLimited { .. }), "got: {err:?}");
        assert_eq!(err.status_code(), 429);
    }

    /// Revert line: as above, build `BadRequest` directly to make this test fail.
    #[test]
    fn parse_stream_event_classifies_context_length_error_like_http() {
        let err = mid_stream_error(
            r#"{"error":{"message":"too long","type":"invalid_request_error","code":"context_length_exceeded"}}"#,
        );
        assert!(
            matches!(err, LiterLlmError::ContextWindowExceeded { .. }),
            "got: {err:?}"
        );
    }

    /// Revert line: as above, build `BadRequest` directly to make this test fail.
    #[test]
    fn parse_stream_event_classifies_authentication_error_from_status() {
        let err = mid_stream_error(r#"{"error":{"message":"bad key","code":"invalid_api_key","status_code":401}}"#);
        assert!(
            matches!(err, LiterLlmError::Authentication { status: 401, .. }),
            "got: {err:?}"
        );
    }

    /// Revert line: change the upper bound `599` of the range check in
    /// `parse_stream_event` to `598`, or the lower bound `400` to `401`, to make
    /// this test fail.
    #[test]
    fn parse_stream_event_classifies_the_edges_of_the_error_range() {
        let err = mid_stream_error(r#"{"error":{"message":"edge","status_code":599}}"#);
        assert!(
            matches!(err, LiterLlmError::ServerError { status: 599, .. }),
            "got: {err:?}"
        );
        let err =
            mid_stream_error(r#"{"error":{"message":"edge","code":"context_length_exceeded","status_code":400}}"#);
        assert!(
            matches!(err, LiterLlmError::ContextWindowExceeded { .. }),
            "got: {err:?}"
        );
    }

    /// Revert line: remove the range check on the payload status in
    /// `parse_stream_event` to make this test fail.
    #[test]
    fn parse_stream_event_keeps_raw_payload_status_outside_error_range() {
        for status in [0_u16, 99, 200, 399, 600] {
            let payload = format!(r#"{{"error":{{"message":"odd","status_code":{status}}}}}"#);
            let err = mid_stream_error(&payload);
            assert!(
                matches!(&err, LiterLlmError::BadRequest { status: got, .. } if *got == status),
                "status {status}, got: {err:?}"
            );
            assert!(err.to_string().contains("odd"), "status {status}, got: {err}");
        }
    }

    /// Revert line: replace `u16::try_from(s).ok()` in `parse_stream_event` with an
    /// `as u16` cast to make this test fail (65965 wraps to 429).
    #[test]
    fn parse_stream_event_ignores_payload_status_above_u16() {
        let err = mid_stream_error(r#"{"error":{"message":"huge","status_code":65965}}"#);
        assert!(
            matches!(err, LiterLlmError::BadRequest { status: 400, .. }),
            "got: {err:?}"
        );
    }

    /// Revert line: drop the rate-limit code arm of `status_for_error_kind` to make
    /// this test fail.
    #[test]
    fn parse_stream_event_classifies_rate_limit_code_alone() {
        let err = mid_stream_error(r#"{"error":{"message":"slow down","code":"rate_limit_exceeded"}}"#);
        assert!(matches!(err, LiterLlmError::RateLimited { .. }), "got: {err:?}");
        assert_eq!(err.status_code(), 429);
    }

    /// Revert line: drop the rate-limit type arm of `status_for_error_kind` to make
    /// this test fail.
    #[test]
    fn parse_stream_event_classifies_rate_limit_type_alone() {
        let err = mid_stream_error(r#"{"error":{"message":"slow down","type":"rate_limit_error"}}"#);
        assert!(matches!(err, LiterLlmError::RateLimited { .. }), "got: {err:?}");
        assert_eq!(err.status_code(), 429);
    }

    /// Revert line: read the error type before the error code in
    /// `status_for_error_kind` to make this test fail. When the two disagree,
    /// the code decides.
    #[test]
    fn parse_stream_event_prefers_error_code_over_type() {
        let cases = [
            (r#""code":"rate_limit_exceeded","type":"invalid_request_error""#, 429),
            (r#""code":"rate_limit_exceeded","type":"authentication_error""#, 429),
        ];
        for (kind, status) in cases {
            let err = mid_stream_error(&format!(r#"{{"error":{{"message":"x",{kind}}}}}"#));
            assert!(matches!(err, LiterLlmError::RateLimited { .. }), "{kind}: got {err:?}");
            assert_eq!(err.status_code(), status, "{kind}");
        }
    }

    /// Revert line: drop an arm of `status_for_error_kind` to make this test fail.
    /// Each event has no status, and must give the kind that the same body gives
    /// as an HTTP reply at the status `status_for_error_kind` reads from it.
    #[test]
    fn parse_stream_event_classifies_error_code_and_type_without_status() {
        let cases = [
            (r#""code":"insufficient_quota""#, 429),
            (r#""code":"invalid_api_key""#, 401),
            (r#""code":"model_not_found""#, 404),
            (r#""code":"server_error""#, 500),
            (r#""type":"authentication_error""#, 401),
            (r#""type":"permission_error""#, 403),
            (r#""type":"not_found_error""#, 404),
            (r#""type":"server_error""#, 500),
            (r#""type":"api_error""#, 500),
            (r#""type":"overloaded_error""#, 529),
        ];
        for (fields, status) in cases {
            let err = mid_stream_error(&format!(r#"{{"error":{{"message":"boom",{fields}}}}}"#));
            let http_body = format!(r#"{{"error":{{"message":"boom",{fields}}}}}"#);
            let http = LiterLlmError::from_status(status, &http_body, None);
            assert_eq!(
                std::mem::discriminant(&err),
                std::mem::discriminant(&http),
                "{fields}: stream gave {err:?}, HTTP {status} gives {http:?}"
            );
            assert_eq!(err.status_code(), http.status_code(), "{fields}");
        }
    }

    /// Revert line: drop the `insufficient_quota` code arm of `status_for_error_kind`
    /// to make this test fail.
    #[test]
    fn parse_stream_event_classifies_insufficient_quota_as_quota_exceeded() {
        let err = mid_stream_error(r#"{"error":{"message":"out of credit","code":"insufficient_quota"}}"#);
        assert!(
            matches!(err, LiterLlmError::ProviderQuotaExceeded { .. }),
            "got: {err:?}"
        );
        assert!(!err.is_transient());
    }

    /// An error with no status, code or type reads as a plain 400.
    #[test]
    fn parse_stream_event_reads_unknown_error_kind_as_bad_request() {
        let err = mid_stream_error(r#"{"error":{"message":"odd","code":"something_else","type":"other"}}"#);
        assert!(
            matches!(err, LiterLlmError::BadRequest { status: 400, .. }),
            "got: {err:?}"
        );
    }

    /// Revert line: remove the fallback message in `parse_stream_event` to make this
    /// test fail.
    #[test]
    fn parse_stream_event_keeps_fallback_message_when_error_has_none() {
        let err = mid_stream_error(r#"{"error":{"code":"rate_limit_exceeded"}}"#);
        assert!(
            err.to_string().contains("provider returned an error mid-stream"),
            "got: {err}"
        );
    }

    /// Revert line: delete `#[serde(default)]` from `ChatCompletionChunk::choices`
    /// in `types/chat.rs` to make this test fail.
    #[test]
    fn parse_stream_event_tolerates_metadata_event_without_choices() {
        // ~keep The #155 trailing-metadata event omits `id`; a provider may also
        // omit `choices` entirely on such an event. Defaulting the field keeps
        // the stream alive, and the error-object guard above runs first so this
        // tolerance cannot swallow a real failure.
        let payload =
            r#"{"object":"chat.completion.chunk","usage":{"prompt_tokens":9,"completion_tokens":1,"total_tokens":10}}"#;
        let chunk = OpenAiProvider
            .parse_stream_event(payload)
            .expect("a metadata event without `choices` must not error")
            .expect("event should decode to a chunk, not be skipped");
        assert!(chunk.choices.is_empty());
    }

    #[test]
    fn parse_stream_event_parses_normal_chunk_with_id() {
        let payload = r#"{"id":"chatcmpl-1","object":"chat.completion.chunk","created":1700000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"hi"},"finish_reason":null}]}"#;
        let chunk = OpenAiProvider
            .parse_stream_event(payload)
            .expect("valid chunk must parse")
            .expect("valid chunk must yield Some");
        assert_eq!(chunk.id, "chatcmpl-1");
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("hi"));
    }
}

#[cfg(test)]
mod route_model_tests {
    use super::*;

    /// Registry providers without a route of their own: complex providers that no
    /// dedicated implementation answers for, and the gateways that need a `base_url`.
    const UNROUTED: &[&str] = &[
        "a2a",
        "amazon_nova",
        "anthropic_text",
        "assemblyai",
        "auto_router",
        "aws_polly",
        "azure_ai",
        "azure_text",
        "baseten",
        "bedrock_mantle",
        "black_forest_labs",
        "chatgpt",
        "cloudflare",
        "cursor",
        "databricks",
        "dataforseo",
        "datarobot",
        "deepgram",
        "elevenlabs",
        "fal_ai",
        "gdc",
        "gigachat",
        "heroku",
        "manus",
        "nvidia_riva",
        "oci",
        "petals",
        "ragflow",
        "reducto",
        "runwayml",
        "sagemaker",
        "sagemaker_chat",
        "sap",
        "searxng",
        "serper",
        "snowflake",
        "soniox",
        "stability",
        "tavily",
        "triton",
        "watsonx",
        "watsonx_text",
    ];

    const GATEWAYS: &[&str] = &[
        "custom",
        "custom_openai",
        "hosted_vllm",
        "infinity",
        "litellm_proxy",
        "openai_like",
    ];

    fn refusal(model: &str) -> String {
        match route_model(model) {
            Err(LiterLlmError::BadRequest { message, status }) => {
                assert_eq!(status, 400, "{model}: refusal must be a 400");
                message
            }
            Err(other) => panic!("{model}: expected BadRequest, got {other:?}"),
            Ok(found) => panic!(
                "{model}: expected a refusal, got provider {:?}",
                found.map(|p| p.name().to_owned())
            ),
        }
    }

    #[test]
    fn provider_without_route_is_refused_with_its_name_and_base_url() {
        for name in UNROUTED.iter().chain(GATEWAYS) {
            let message = refusal(&format!("{name}/some-model"));
            assert!(message.contains(&format!("`{name}`")), "{name}: message {message:?}");
            assert!(message.contains("base_url"), "{name}: message {message:?}");
        }
    }

    #[test]
    fn nested_provider_namespace_is_refused_by_its_first_segment() {
        let message = refusal("azure_ai/agents/some-agent");
        assert!(message.contains("`azure_ai`"), "{message:?}");
    }

    #[test]
    fn no_registry_provider_falls_through_to_the_default() {
        let reg = REGISTRY.as_ref().expect("registry should load");
        let mut checked = 0;
        for entry in &reg.providers {
            let names = std::iter::once(entry.config.name.as_str()).chain(entry.aliases.iter().map(String::as_str));
            for name in names {
                let model = format!("{name}/some-model");
                // A provider named by `provider/` either has a route or is refused.
                // `Ok(None)` is the silent fall back to the OpenAI endpoint.
                assert!(
                    !matches!(route_model(&model), Ok(None)),
                    "{model}: falls through to the default provider"
                );
                checked += 1;
            }
        }
        assert!(checked >= reg.providers.len(), "sweep covered {checked} names");
    }

    #[test]
    fn providers_with_a_route_keep_routing() {
        for (model, expected) in [
            ("gpt-4o", "openai"),
            ("claude-3-5-sonnet-20241022", "anthropic"),
            ("anthropic/claude-3-5-sonnet-20241022", "anthropic"),
            ("ollama/llama3", "ollama"),
            ("ollama_chat/llama3", "ollama"),
            ("groq/llama3-70b", "groq"),
            ("deepseek/deepseek-chat", "deepseek"),
            ("mistral/mistral-large-latest", "mistral"),
        ] {
            let provider = route_model(model)
                .unwrap_or_else(|e| panic!("{model}: {e}"))
                .unwrap_or_else(|| panic!("{model}: no provider"));
            assert_eq!(provider.name(), expected, "{model}");
        }
    }

    fn synthetic_registry(providers: serde_json::Value, complex: &[&str]) -> ProviderRegistry {
        let raw: ProviderRegistryRaw =
            serde_json::from_value(serde_json::json!({"providers": providers, "complex_providers": complex}))
                .expect("synthetic registry should parse");
        ProviderRegistry::from_raw(raw)
    }

    #[test]
    fn registry_entry_without_base_url_is_refused_even_when_not_complex() {
        let reg = synthetic_registry(serde_json::json!([{"name": "plain", "aliases": ["plain_alias"]}]), &[]);
        for model in ["plain/m", "plain_alias/m"] {
            match unrouted_provider_error(&reg, model) {
                Some(LiterLlmError::BadRequest { message, status }) => {
                    assert_eq!(status, 400, "{model}");
                    assert!(message.contains("`plain`") && message.contains("base_url"), "{message}");
                }
                other => panic!("{model}: expected a refusal, got {other:?}"),
            }
        }
    }

    /// The provider that a registry entry builds, as the router builds it.
    fn provider_for(providers: serde_json::Value, name: &str) -> ConfigDrivenProvider {
        let reg: &'static ProviderRegistry = Box::leak(Box::new(synthetic_registry(providers, &[])));
        let entry = reg
            .providers
            .iter()
            .find(|e| e.config.name == name)
            .expect("entry should exist");
        ConfigDrivenProvider::from_entry(entry)
    }

    #[test]
    fn endpoint_listed_in_no_route_endpoints_is_refused_and_the_others_are_not() {
        let mixed = provider_for(
            serde_json::json!([{
                "name": "mixed",
                "base_url": "https://mixed.example/v1",
                "no_route_endpoints": ["search", "audio"]
            }]),
            "mixed",
        );
        for endpoint in [Endpoint::Search, Endpoint::Audio] {
            match refuse_unrouted_endpoint(&mixed, endpoint) {
                Err(LiterLlmError::BadRequest { message, status }) => {
                    assert_eq!(status, 400);
                    assert_eq!(
                        message,
                        format!(
                            "provider `mixed` has no built-in route for {}; set base_url to its endpoint",
                            endpoint.as_str()
                        )
                    );
                }
                other => panic!("{endpoint:?}: expected a refusal, got {other:?}"),
            }
        }
        for endpoint in [Endpoint::Chat, Endpoint::Image, Endpoint::Embedding] {
            assert!(refuse_unrouted_endpoint(&mixed, endpoint).is_ok(), "{endpoint:?}");
        }
        let other = provider_for(
            serde_json::json!([{"name": "other", "base_url": "https://other.example/v1"}]),
            "other",
        );
        assert!(refuse_unrouted_endpoint(&other, Endpoint::Search).is_ok());
    }

    fn builtin(name: &str) -> Box<dyn Provider> {
        let model = if name == "openai" {
            "gpt-4o".to_owned()
        } else {
            format!("{name}/m")
        };
        detect_builtin_provider(&model).expect("shipped provider should route")
    }

    #[test]
    fn shipped_registry_marks_each_provider_endpoint_pair_without_a_route() {
        for (provider, endpoint) in [
            ("brave", Endpoint::Search),
            ("perplexity", Endpoint::Search),
            ("exa_ai", Endpoint::Search),
            ("firecrawl", Endpoint::Search),
            ("linkup", Endpoint::Search),
            ("parallel_ai", Endpoint::Search),
            ("google_pse", Endpoint::Search),
            ("duckduckgo", Endpoint::Search),
            ("edenai", Endpoint::Audio),
            ("edenai", Endpoint::Image),
        ] {
            assert!(
                refuse_unrouted_endpoint(builtin(provider).as_ref(), endpoint).is_err(),
                "{provider} {endpoint:?}"
            );
        }
        for (provider, endpoint) in [
            ("brave", Endpoint::Chat),
            ("perplexity", Endpoint::Chat),
            ("edenai", Endpoint::Chat),
            ("edenai", Endpoint::Embedding),
            ("modelscope", Endpoint::Image),
            ("openai", Endpoint::Search),
        ] {
            assert!(
                refuse_unrouted_endpoint(builtin(provider).as_ref(), endpoint).is_ok(),
                "{provider} {endpoint:?}"
            );
        }
    }

    #[test]
    fn registry_entry_with_base_url_that_is_not_complex_has_a_route() {
        let reg = synthetic_registry(
            serde_json::json!([{"name": "plain", "base_url": "https://plain.example/v1"}]),
            &[],
        );
        assert!(unrouted_provider_error(&reg, "plain/m").is_none());
    }

    #[test]
    fn complex_registry_entry_with_base_url_is_refused() {
        let reg = synthetic_registry(
            serde_json::json!([{"name": "fancy", "base_url": "https://fancy.example/v1"}]),
            &["fancy"],
        );
        assert!(unrouted_provider_error(&reg, "fancy/m").is_some());
    }

    #[test]
    fn prefix_that_names_no_registry_entry_is_not_refused() {
        let reg = synthetic_registry(serde_json::json!([{"name": "plain"}]), &[]);
        assert!(unrouted_provider_error(&reg, "other/m").is_none());
        assert!(unrouted_provider_error(&reg, "no-slash").is_none());
    }

    #[test]
    fn bare_hint_is_refused_like_its_prefix() {
        for hint in ["snowflake", "watsonx", "hosted_vllm"] {
            match route_hint(hint) {
                Err(LiterLlmError::BadRequest { message, .. }) => {
                    assert!(message.contains(&format!("`{hint}`")), "{message}");
                }
                Err(error) => panic!("{hint}: expected a bad request, got {error:?}"),
                Ok(routed) => panic!(
                    "{hint}: expected a refusal, got {:?}",
                    routed.map(|provider| provider.name().to_owned())
                ),
            }
        }
    }

    #[test]
    fn bare_hint_with_a_route_or_no_registry_entry_is_accepted() {
        for hint in ["anthropic", "groq", "ollama", "gpt-4o", "some-local-model"] {
            assert!(route_hint(hint).is_ok(), "{hint}");
        }
    }

    #[test]
    fn bare_hint_and_slash_hint_select_the_same_provider() {
        for (bare, slash) in [
            ("cohere", "cohere/command-r-plus"),
            ("mistral", "mistral/mistral-large-latest"),
            ("voyage", "voyage/rerank-2"),
            ("anthropic", "anthropic/claude-sonnet-4"),
            ("gemini", "gemini/gemini-2.0-flash"),
            ("vertex_ai", "vertex_ai/gemini-2.0-flash"),
            ("ollama", "ollama/llama3"),
            ("groq", "groq/llama3-8b"),
        ] {
            let bare_name = route_hint(bare).expect("no refusal").map(|p| p.name().to_owned());
            let slash_name = route_hint(slash).expect("no refusal").map(|p| p.name().to_owned());
            assert!(bare_name.is_some(), "{bare} selects a provider");
            assert_eq!(bare_name, slash_name, "{bare} and {slash}");
        }
    }

    #[test]
    fn openai_at_a_base_url_is_the_generic_provider() {
        let provider = route_hint("gpt-5").expect("no refusal").expect("gpt-5 names OpenAI");
        let rebased = on_base_url(provider, "http://localhost:8080/v1");
        assert_eq!(rebased.name(), "custom");
        assert_eq!(rebased.strip_model_prefix("openai/gpt-5"), "gpt-5");
        assert_eq!(rebased.strip_model_prefix("custom/my-model"), "my-model");
        assert_eq!(rebased.strip_model_prefix("anthropic/claude-x"), "anthropic/claude-x");
    }

    #[test]
    fn hint_that_names_no_provider_selects_none() {
        for hint in ["llama3.1:8b", "some-local-model", "not-a-provider/model"] {
            assert!(route_hint(hint).expect("no refusal").is_none(), "{hint}");
        }
    }

    #[test]
    fn on_base_url_changes_only_the_host() {
        let provider = route_hint("cohere").expect("no refusal").expect("cohere is built in");
        let rebased = on_base_url(provider, "http://gateway.test/v1/");
        assert_eq!(rebased.name(), "cohere");
        assert_eq!(rebased.chat_completions_path(), "/chat");
        assert_eq!(rebased.build_url("/chat", "m"), "http://gateway.test/v1/chat");
    }

    /// Revert line: drop the `next_models_page_url` forward in `BaseUrlOverride`, and a
    /// wrapped provider that pages its model list stops after the first page.
    #[test]
    fn a_rebased_provider_keeps_its_model_list_paging() {
        let paging = Box::new(super::vertex::VertexAiProvider::with_base_url("http://vertex.test"));
        let rebased = rebased::BaseUrlOverride::new(paging, "http://gateway.test");
        let page = serde_json::json!({"nextPageToken": "next"});
        assert_eq!(
            rebased
                .next_models_page_url("http://gateway.test/models", &page)
                .as_deref(),
            Some("http://gateway.test/models?pageToken=next")
        );
    }

    #[test]
    fn provider_without_auth_sends_the_key_as_bearer_at_a_base_url() {
        let local = route_hint("lmstudio")
            .expect("no refusal")
            .expect("lmstudio is built in");
        assert!(
            local.auth_header("k").is_none(),
            "lmstudio sends no key at its own host"
        );
        let rebased = on_base_url(local, "http://gateway.test/v1");
        let (name, value) = rebased.auth_header("k").expect("a key at a base_url is sent");
        assert_eq!((name.as_ref(), value.as_ref()), ("Authorization", "Bearer k"));
    }

    #[test]
    fn provider_with_auth_keeps_its_header_at_a_base_url() {
        let provider = route_hint("cohere").expect("no refusal").expect("cohere is built in");
        let rebased = on_base_url(provider, "http://gateway.test/v1");
        let (name, value) = rebased.auth_header("k").expect("cohere sends a key");
        assert_eq!((name.as_ref(), value.as_ref()), ("Authorization", "Bearer k"));
    }

    #[test]
    fn model_that_names_no_registry_provider_is_left_to_the_caller() {
        assert!(route_model("some-random-model").expect("no refusal").is_none());
        assert!(route_model("not-a-provider/model").expect("no refusal").is_none());
    }
}
