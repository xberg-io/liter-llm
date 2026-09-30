use std::borrow::Cow;

use crate::error::{LiterLlmError, Result};
use crate::provider::{Provider, StreamFormat};

mod embed;
mod request;
mod response;
mod stream;

use embed::{transform_bedrock_embed_request, transform_bedrock_embed_response};
pub(crate) use stream::parse_bedrock_stream_event;

/// Default AWS region for Bedrock when none is specified.
const DEFAULT_REGION: &str = "us-east-1";

/// Map reasoning effort levels to budget_tokens for Claude-on-Bedrock extended thinking.
fn reasoning_effort_to_budget_tokens(effort: &str) -> u64 {
    match effort {
        "low" => 1024,
        "medium" => 4096,
        "high" => 16384,
        _ => 4096,
    }
}

/// Extract a document format from a MIME type string.
///
/// E.g. `"application/pdf"` → `"pdf"`, `"text/csv"` → `"csv"`.
fn format_from_media_type(media_type: &str) -> &str {
    media_type.split('/').nth(1).unwrap_or("pdf")
}

/// Convert OpenAI-format message content (plain string or content-part array) to
/// Bedrock Converse content blocks (`text` / `image` / `document`).
///
/// Shared by both user messages and tool results: Converse's `toolResult.content`
/// accepts the same block shapes as a user turn's `content`, so this is reused
/// rather than duplicated for the `"tool"` role. ~keep
fn convert_content_to_bedrock_blocks(content: Option<&serde_json::Value>) -> Vec<serde_json::Value> {
    use serde_json::json;

    if let Some(text) = content.and_then(|c| c.as_str()) {
        vec![json!({"text": text})]
    } else if let Some(array) = content.and_then(|c| c.as_array()) {
        array
            .iter()
            .filter_map(|part| {
                let part_type = part.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match part_type {
                    "text" => {
                        let text = part.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        Some(json!({"text": text}))
                    }
                    "image_url" => {
                        let url = part.pointer("/image_url/url").and_then(|u| u.as_str()).unwrap_or("");
                        if let Some(data_part) = url.strip_prefix("data:") {
                            let mut iter = data_part.splitn(2, ';');
                            let media_type = iter.next().unwrap_or("image/jpeg");
                            let b64 = iter.next().and_then(|s| s.strip_prefix("base64,")).unwrap_or("");
                            Some(json!({
                                "image": {
                                    "format": media_type.split('/').nth(1).unwrap_or("jpeg"),
                                    "source": {"bytes": b64}
                                }
                            }))
                        } else {
                            Some(json!({"text": url}))
                        }
                    }
                    "document" => {
                        let data = part.pointer("/document/data").and_then(|d| d.as_str()).unwrap_or("");
                        let media_type = part
                            .pointer("/document/media_type")
                            .and_then(|m| m.as_str())
                            .unwrap_or("application/pdf");
                        let format = format_from_media_type(media_type);
                        Some(json!({
                            "document": {
                                "name": "doc",
                                "format": format,
                                "source": {"bytes": data}
                            }
                        }))
                    }
                    _ => None,
                }
            })
            .collect()
    } else {
        vec![json!({"text": ""})]
    }
}

/// Determine the DNS suffix for a given AWS region.
///
/// - Standard/GovCloud regions: `amazonaws.com`
/// - European Sovereign Cloud (EUSC, `eusc-*`): `amazonaws.eu`
/// - China (`cn-*`): `amazonaws.com.cn`
fn dns_suffix_for_region(region: &str) -> &'static str {
    if region.starts_with("eusc-") {
        "amazonaws.eu"
    } else if region.starts_with("cn-") {
        "amazonaws.com.cn"
    } else {
        "amazonaws.com"
    }
}

/// Percent-encode a model ID for use in a URL path segment.
///
/// Bedrock model IDs can contain colons and slashes that must be encoded.
fn percent_encode_model(model: &str) -> String {
    let mut encoded = String::with_capacity(model.len());
    for byte in model.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            other => {
                encoded.push('%');
                let hi = char::from_digit(u32::from(other >> 4), 16).unwrap_or('0');
                let lo = char::from_digit(u32::from(other & 0xf), 16).unwrap_or('0');
                encoded.push(hi.to_ascii_uppercase());
                encoded.push(lo.to_ascii_uppercase());
            }
        }
    }
    encoded
}

/// AWS Bedrock provider.
///
/// Differences from the OpenAI-compatible baseline:
/// - Routes `bedrock/` prefixed model names to the Bedrock runtime endpoint.
/// - The model prefix is stripped before the model ID is sent in the request.
/// - Requests carry a credential when one is configured; with none, and a
///   `base_url` override, the provider is usable against a mock server.
///
/// # Authentication
///
/// Either a Bedrock API key in `AWS_BEARER_TOKEN_BEDROCK`, sent as
/// `Authorization: Bearer <token>` and needing no signing (so it works with the
/// `bedrock` feature off), or a SigV4 access-key pair from explicit config (see
/// [`BedrockProvider::with_credentials`]) or the environment
/// (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`), which
/// needs the feature. Precedence: any explicitly configured credential field, then
/// the token, then environment credentials. Without the feature, only the token
/// can authenticate, so it is used whatever else is configured.
///
/// # Region resolution
///
/// The region is resolved in priority order:
/// 1. Explicit value passed to [`BedrockProvider::new`] or [`BedrockProvider::from_config`].
/// 2. `AWS_DEFAULT_REGION` environment variable.
/// 3. `AWS_REGION` environment variable.
/// 4. Hard-coded default: `us-east-1`.
///
/// # Configuration
///
/// ```rust,ignore
/// let config = ClientConfigBuilder::new("unused-for-sigv4")
///     .build();
/// let client = DefaultClient::new(config, Some("bedrock/anthropic.claude-3-sonnet-20240229-v1:0"))?;
/// ```
pub struct BedrockProvider {
    region: String,
    /// Cached base URL: `https://bedrock-runtime.{region}.{dns_suffix}`.
    base_url: String,
    /// Cached cross-region prefix from `BEDROCK_CROSS_REGION` env var at
    /// construction time (e.g. `Some("us.")`) so we avoid reading the
    /// environment on every request.
    cross_region_prefix: Option<String>,
    /// Explicit AWS access key ID, overriding `AWS_ACCESS_KEY_ID` when set.
    access_key_id: Option<String>,
    /// Explicit AWS secret access key, overriding `AWS_SECRET_ACCESS_KEY` when set.
    secret_access_key: Option<String>,
    /// Explicit AWS session token, overriding `AWS_SESSION_TOKEN` when set.
    session_token: Option<String>,
}

impl BedrockProvider {
    /// Construct with the given AWS region.
    ///
    /// The base URL is derived from the region's DNS suffix. To override it
    /// entirely, set `BEDROCK_BASE_URL` in the environment.
    #[must_use]
    pub fn new(region: impl Into<String>) -> Self {
        let region = region.into();
        let custom_base_url = std::env::var("BEDROCK_BASE_URL")
            .ok()
            .filter(|v| !v.is_empty())
            .map(|v| v.trim_end_matches('/').to_string());
        let base_url = custom_base_url.clone().unwrap_or_else(|| {
            let dns_suffix = dns_suffix_for_region(&region);
            format!("https://bedrock-runtime.{region}.{dns_suffix}")
        });
        let cross_region_prefix = if custom_base_url.is_some() {
            None
        } else {
            std::env::var("BEDROCK_CROSS_REGION")
                .ok()
                .filter(|v| !v.is_empty())
                .map(|v| format!("{v}."))
        };
        Self {
            region,
            base_url,
            cross_region_prefix,
            access_key_id: None,
            secret_access_key: None,
            session_token: None,
        }
    }

    /// Construct using region from the environment, falling back to `us-east-1`.
    ///
    /// Reads `AWS_DEFAULT_REGION` then `AWS_REGION`.
    #[must_use]
    pub fn from_env() -> Self {
        let region = std::env::var("AWS_DEFAULT_REGION")
            .or_else(|_| std::env::var("AWS_REGION"))
            .unwrap_or_else(|_| DEFAULT_REGION.to_owned());
        Self::new(region)
    }

    /// Construct from explicit, optional config values, falling back to the
    /// environment for anything left unset.
    ///
    /// Region resolution order: `region` -> `AWS_DEFAULT_REGION` ->
    /// `AWS_REGION` -> `us-east-1`. Credentials and the cross-region prefix
    /// fall back to their respective environment variables at request time
    /// (see [`BedrockProvider::with_credentials`] and
    /// [`BedrockProvider::with_cross_region_prefix`]).
    #[must_use]
    pub fn from_config(
        region: Option<String>,
        cross_region_prefix: Option<String>,
        access_key_id: Option<String>,
        secret_access_key: Option<String>,
        session_token: Option<String>,
    ) -> Self {
        let region = region
            .or_else(|| std::env::var("AWS_DEFAULT_REGION").ok())
            .or_else(|| std::env::var("AWS_REGION").ok())
            .unwrap_or_else(|| DEFAULT_REGION.to_owned());
        Self::new(region)
            .with_cross_region_prefix(cross_region_prefix)
            .with_credentials(access_key_id, secret_access_key, session_token)
    }

    /// Override the cross-region inference profile prefix (e.g. `"us"`).
    ///
    /// When `None`, the prefix cached from `BEDROCK_CROSS_REGION` at
    /// construction time (if any) is left untouched.
    #[must_use]
    pub fn with_cross_region_prefix(mut self, prefix: Option<String>) -> Self {
        if let Some(prefix) = prefix {
            let prefix = if prefix.ends_with('.') {
                prefix
            } else {
                format!("{prefix}.")
            };
            self.cross_region_prefix = Some(prefix);
        }
        self
    }

    /// Set explicit AWS credentials for SigV4 signing, overriding the
    /// `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` / `AWS_SESSION_TOKEN`
    /// environment variables when present.
    #[must_use]
    pub fn with_credentials(
        mut self,
        access_key_id: Option<String>,
        secret_access_key: Option<String>,
        session_token: Option<String>,
    ) -> Self {
        self.access_key_id = access_key_id;
        self.secret_access_key = secret_access_key;
        self.session_token = session_token;
        self
    }

    /// Return the AWS region this provider is configured for.
    #[must_use]
    #[allow(dead_code)]
    pub fn region(&self) -> &str {
        &self.region
    }

    /// The Bedrock API key from `AWS_BEARER_TOKEN_BEDROCK`, when it applies.
    ///
    /// Resolved per request, so a rotated key needs no new client — the same reason
    /// the SigV4 path resolves `AWS_ACCESS_KEY_ID` at signing time.
    ///
    /// With the `bedrock` feature, any credential field set through
    /// [`BedrockProvider::with_credentials`] wins. Each field falls back to the
    /// environment on its own, so an explicit access key with an ambient secret is a
    /// working SigV4 configuration, and an ambient token must not redirect it to another
    /// principal. Against *environment* credentials the token wins, matching the AWS
    /// SDKs. Without the feature nothing can sign, so a usable token always wins. ~keep
    fn bearer_token(&self) -> Option<String> {
        #[cfg(feature = "bedrock")]
        {
            if self.has_explicit_credential() {
                return None;
            }
        }
        std::env::var("AWS_BEARER_TOKEN_BEDROCK")
            .ok()
            .filter(|token| !token.is_empty())
    }

    /// Whether any credential field was set in code rather than left to the environment.
    fn has_explicit_credential(&self) -> bool {
        // ~keep Empty is not set: a blank value is no credential anyone configured, so it
        // must not suppress the token.
        [&self.access_key_id, &self.secret_access_key, &self.session_token]
            .into_iter()
            .any(|field| field.as_deref().is_some_and(|value| !value.is_empty()))
    }
}

impl Provider for BedrockProvider {
    fn name(&self) -> &str {
        "bedrock"
    }

    /// Base URL for the Bedrock runtime service.
    ///
    /// When a `base_url` override is set in [`ClientConfig`] (as in tests),
    /// the override takes precedence and this value is never used.
    fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Bedrock uses SigV4 signing rather than a static authorization header.
    ///
    /// Returns `None` so the HTTP layer skips adding an `Authorization` header.
    /// Authentication headers are injected by [`BedrockProvider::signing_headers`]:
    /// a Bearer token when one is set, otherwise SigV4 when the `bedrock` feature
    /// is enabled.
    fn auth_header<'a>(&'a self, _api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
        None
    }

    fn matches_model(&self, model: &str) -> bool {
        model.starts_with("bedrock/")
    }

    fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
        model.strip_prefix("bedrock/").unwrap_or(model)
    }

    /// Validate that the provider is usable in the current environment.
    ///
    /// When the `bedrock` feature is enabled, checks that a usable credential is
    /// available: either a Bedrock API key in `AWS_BEARER_TOKEN_BEDROCK`, or both
    /// an AWS access key and secret key (explicit config, or `AWS_ACCESS_KEY_ID`
    /// and `AWS_SECRET_ACCESS_KEY` in the environment). With neither, no request
    /// can be authenticated, so this returns an error instead of letting the
    /// request continue toward an unsigned or malformed send.
    ///
    /// Called once at client construction and again on every request from
    /// [`BedrockProvider::transform_request`], since credentials can become
    /// unavailable between the two.
    ///
    /// When the `bedrock` feature is disabled, a provider with no credentials at
    /// all passes, so callers can connect to a mock server via a `base_url`
    /// override. Explicit credentials without a token are rejected: nothing in
    /// that build can sign them, so the request would go out unauthenticated.
    fn validate(&self) -> Result<()> {
        #[cfg(not(feature = "bedrock"))]
        {
            if self.has_explicit_credential() && self.bearer_token().is_none() {
                return Err(LiterLlmError::Authentication {
                    message: "AWS Bedrock access keys need the `bedrock` feature to sign requests. \
                              Enable it, or set a Bedrock API key in AWS_BEARER_TOKEN_BEDROCK."
                        .into(),
                    status: 401,
                });
            }
        }
        #[cfg(feature = "bedrock")]
        {
            // ~keep A Bedrock API key authenticates on its own; the SigV4 pair is not required.
            if self.bearer_token().is_some() {
                return Ok(());
            }

            let has_access_key = self.access_key_id.is_some() || std::env::var("AWS_ACCESS_KEY_ID").is_ok();
            let has_secret_key = self.secret_access_key.is_some() || std::env::var("AWS_SECRET_ACCESS_KEY").is_ok();
            // ~keep Both keys are required: sigv4_sign has no instance-profile/SSO fallback,
            // so a request missing either one can never actually be signed.
            if !has_access_key || !has_secret_key {
                return Err(LiterLlmError::Authentication {
                    message: "AWS Bedrock requires credentials. \
                              Set a Bedrock API key in AWS_BEARER_TOKEN_BEDROCK, or supply an \
                              access-key pair explicitly via config or as AWS_ACCESS_KEY_ID and \
                              AWS_SECRET_ACCESS_KEY (and optionally AWS_SESSION_TOKEN) in the \
                              environment."
                        .into(),
                    status: 401,
                });
            }
        }
        Ok(())
    }

    /// Bedrock uses AWS EventStream binary framing, not SSE.
    fn stream_format(&self) -> StreamFormat {
        StreamFormat::AwsEventStream
    }

    /// Build the full URL for a Bedrock Converse API request.
    ///
    /// Chat completions map to `/model/{encoded_model}/converse`.
    /// Embeddings map to `/model/{encoded_model}/invoke`.
    /// All other paths are passed through unchanged.
    ///
    /// When the `BEDROCK_CROSS_REGION` environment variable is set, the
    /// cross-region inference profile prefix is prepended to the model ID.
    /// For example, with `BEDROCK_CROSS_REGION=us`, model
    /// `anthropic.claude-3-sonnet-20240229-v1:0` becomes
    /// `us.anthropic.claude-3-sonnet-20240229-v1:0`.
    fn build_url(&self, endpoint_path: &str, model: &str) -> String {
        let base = self.base_url();
        let effective_model = self.apply_cross_region_prefix(model);
        let encoded_model = percent_encode_model(&effective_model);
        if endpoint_path.contains("chat/completions") {
            format!("{base}/model/{encoded_model}/converse")
        } else if endpoint_path.contains("embeddings") {
            format!("{base}/model/{encoded_model}/invoke")
        } else {
            format!("{base}{endpoint_path}")
        }
    }

    /// Build the streaming URL: `/model/{id}/converse-stream`.
    fn build_stream_url(&self, endpoint_path: &str, model: &str) -> String {
        let base = self.base_url();
        let effective_model = self.apply_cross_region_prefix(model);
        let encoded_model = percent_encode_model(&effective_model);
        if endpoint_path.contains("chat/completions") {
            format!("{base}/model/{encoded_model}/converse-stream")
        } else {
            self.build_url(endpoint_path, model)
        }
    }

    /// Convert an OpenAI-style chat request to Bedrock Converse API format.
    ///
    /// Key differences from the OpenAI format:
    /// - System messages are extracted to a top-level `system` array.
    /// - Messages use `content` arrays with typed blocks (`text`, `toolUse`, `toolResult`).
    /// - Generation parameters live in `inferenceConfig`.
    /// - Tools are described in `toolConfig.tools[].toolSpec`.
    /// - `temperature` and `top_p` outside Bedrock's documented `[0.0, 1.0]` range are
    ///   rejected with a `BadRequest` error rather than forwarded and left for Bedrock to reject.
    ///
    /// Embedding requests (`input` present, no `messages`) are routed to
    /// [`transform_bedrock_embed_request`] instead: Bedrock has no Converse-style
    /// unified embeddings API, so this dispatch mirrors the same
    /// `input`/`messages` discriminator used by [`super::vertex`].
    fn transform_request(&self, body: &mut serde_json::Value) -> Result<()> {
        // ~keep Re-checked on every request (not just at client construction): credentials
        // can become unavailable between construction and send. `signing_headers` now also
        // hard-errors on a signing failure (see #42), but this precheck fails fast with a
        // clearer, missing-credentials-specific message before any signing work happens.
        self.validate()?;

        // ~keep Embedding bodies have `input` and no `messages`. Without this branch they
        // ~keep fell through to the Converse transform below, which unconditionally removes
        // ~keep `messages` (absent here) via `unwrap_or_default()` and rebuilds the body as
        // ~keep `{"messages": []}`, silently discarding the entire `input`.
        if body.get("input").is_some() && body.get("messages").is_none() {
            return transform_bedrock_embed_request(body);
        }

        request::transform_converse_request(body)
    }

    /// Normalize a Bedrock Converse API response to OpenAI chat completion format.
    ///
    /// Bedrock wraps the assistant's message in `output.message.content[]` blocks.
    /// Stop reasons use Bedrock terminology (`end_turn`, `tool_use`, etc.) and are
    /// mapped to the OpenAI `finish_reason` set.
    ///
    /// **Known limitation:** The `model` field in the normalized response is
    /// always `""`.  Bedrock does not include the model name in its response
    /// body — the model is only present in the request URL path.  Threading
    /// the model through would require a signature change to `transform_response`.
    ///
    /// Embedding responses (Titan's `{"embedding": [...]}` or Cohere's
    /// `{"embeddings": [[...], ...]}`) are routed to
    /// [`transform_bedrock_embed_response`] instead. Unlike the request side,
    /// this dispatch cannot use the model ID — `transform_response` has no
    /// model parameter (see the limitation above) and an `InvokeModel`
    /// response body carries no model field either — so it sniffs the
    /// response shape instead, same as [`super::vertex::transform_gemini_response`].
    fn transform_response(&self, body: &mut serde_json::Value) -> Result<()> {
        if body.get("embedding").is_some() || body.get("embeddings").is_some() {
            return transform_bedrock_embed_response(body);
        }

        response::transform_converse_response(body)
    }

    /// Compute the authentication headers for the request.
    ///
    /// With a Bedrock API key in `AWS_BEARER_TOKEN_BEDROCK`, returns one
    /// `Authorization: Bearer <token>` header and skips signing.
    ///
    /// Otherwise, when the `bedrock` feature is enabled, derives the `Authorization`,
    /// `x-amz-date`, and (when a session token is present) `x-amz-security-token`
    /// headers from the current request parameters and AWS credentials.
    ///
    /// When the `bedrock` feature is disabled, returns an empty vector so
    /// requests work against override base-URLs (e.g. mock servers in tests).
    ///
    /// # Errors
    ///
    /// `Provider::transform_request` (called earlier in the send path, see
    /// [`BedrockProvider::transform_request`]) re-validates that credentials are
    /// present on every request and hard-errors before any network I/O, which
    /// closes the realistic failure mode (missing or revoked credentials). The
    /// only way `sigv4_sign` can still fail here despite that precheck is an
    /// internal SigV4 library error (malformed signing params). Fix for #42:
    /// that failure is now propagated as a hard error instead of silently
    /// falling back to an unsigned request. ~keep
    fn signing_headers(&self, method: &str, url: &str, body: &[u8]) -> Result<Vec<(String, String)>> {
        // ~keep Outside the `bedrock` feature gate deliberately: a Bearer header needs no
        // signing machinery, so a default build authenticates without pulling in aws-sigv4.
        if let Some(token) = self.bearer_token() {
            return Ok(vec![("authorization".to_owned(), format!("Bearer {token}"))]);
        }

        #[cfg(feature = "bedrock")]
        {
            sigv4_sign(
                method,
                url,
                body,
                &self.region,
                SigV4Credentials {
                    access_key_id: self.access_key_id.as_deref(),
                    secret_access_key: self.secret_access_key.as_deref(),
                    session_token: self.session_token.as_deref(),
                },
            )
            .map_err(|error| {
                tracing::error!(
                    error = %error,
                    %method,
                    "Bedrock SigV4 signing failed after credential precheck passed"
                );
                error
            })
        }

        #[cfg(not(feature = "bedrock"))]
        {
            let _ = (method, url, body);
            Ok(vec![])
        }
    }
}

/// Map a Bedrock Converse `stopReason` to an OpenAI `finish_reason`. Shared by
/// the non-streaming response and the `messageStop` stream event.
fn converse_finish_reason(stop_reason: &str) -> &'static str {
    match stop_reason {
        "end_turn" => "stop",
        "tool_use" => "tool_calls",
        "max_tokens" => "length",
        "stop_sequence" => "stop",
        "content_filtered" | "guardrail_intervened" => "content_filter",
        _ => "stop",
    }
}

/// Apply the cross-region inference profile prefix using the value cached at
/// construction time from the `BEDROCK_CROSS_REGION` environment variable.
///
/// When the prefix is set (e.g. `"us."`), the model ID
/// `anthropic.claude-3-sonnet-20240229-v1:0` becomes
/// `us.anthropic.claude-3-sonnet-20240229-v1:0`.
///
/// If the model already starts with the cross-region prefix, it is returned
/// unchanged to avoid double-prefixing.
impl BedrockProvider {
    fn apply_cross_region_prefix(&self, model: &str) -> String {
        match &self.cross_region_prefix {
            Some(prefix) => {
                if model.starts_with(prefix.as_str()) {
                    model.to_owned()
                } else {
                    format!("{prefix}{model}")
                }
            }
            None => model.to_owned(),
        }
    }
}

/// Legacy free function kept for existing tests. Reads the env var directly.
///
/// Production code uses [`BedrockProvider::apply_cross_region_prefix`] which
/// reads the env var once at construction time.
#[cfg(test)]
fn apply_cross_region_prefix(model: &str) -> String {
    match std::env::var("BEDROCK_CROSS_REGION") {
        Ok(region) if !region.is_empty() => {
            let prefix = format!("{region}.");
            if model.starts_with(&prefix) {
                model.to_owned()
            } else {
                format!("{prefix}{model}")
            }
        }
        _ => model.to_owned(),
    }
}

/// Explicit AWS credentials for [`sigv4_sign`]; each `None` falls back to its
/// standard environment variable.
#[cfg(feature = "bedrock")]
struct SigV4Credentials<'a> {
    access_key_id: Option<&'a str>,
    secret_access_key: Option<&'a str>,
    session_token: Option<&'a str>,
}

/// Compute AWS SigV4 signing headers using the `aws-sigv4` crate.
///
/// Each credential falls back to the standard AWS environment variable when
/// the corresponding explicit argument is `None`:
/// - `access_key_id` -> `AWS_ACCESS_KEY_ID` (required)
/// - `secret_access_key` -> `AWS_SECRET_ACCESS_KEY` (required)
/// - `session_token` -> `AWS_SESSION_TOKEN` (optional, for temporary credentials)
///
/// Returns a vector of `(header-name, header-value)` pairs to inject into the
/// outgoing HTTP request.
#[cfg(feature = "bedrock")]
fn sigv4_sign(
    method: &str,
    url: &str,
    body: &[u8],
    region: &str,
    credentials: SigV4Credentials<'_>,
) -> Result<Vec<(String, String)>> {
    use aws_credential_types::Credentials;
    use aws_sigv4::http_request::{SignableBody, SignableRequest, SigningSettings, sign};
    use aws_sigv4::sign::v4::SigningParams;

    let access_key = credentials
        .access_key_id
        .map(str::to_owned)
        .or_else(|| std::env::var("AWS_ACCESS_KEY_ID").ok())
        .ok_or_else(|| LiterLlmError::BadRequest {
            message: "AWS access key ID is required for Bedrock requests: set it explicitly via config or \
                      the AWS_ACCESS_KEY_ID environment variable"
                .into(),
            status: 400,
        })?;
    let secret_key = credentials
        .secret_access_key
        .map(str::to_owned)
        .or_else(|| std::env::var("AWS_SECRET_ACCESS_KEY").ok())
        .ok_or_else(|| LiterLlmError::BadRequest {
            message: "AWS secret access key is required for Bedrock requests: set it explicitly via config or \
                      the AWS_SECRET_ACCESS_KEY environment variable"
                .into(),
            status: 400,
        })?;
    let session_token = credentials
        .session_token
        .map(str::to_owned)
        .or_else(|| std::env::var("AWS_SESSION_TOKEN").ok());

    let credentials = Credentials::new(access_key, secret_key, session_token, None, "env");

    let identity = credentials.into();

    let signing_settings = SigningSettings::default();
    let now = std::time::SystemTime::now();

    let params = SigningParams::builder()
        .identity(&identity)
        .region(region)
        .name("bedrock")
        .time(now)
        .settings(signing_settings)
        .build()
        .map_err(|e| LiterLlmError::BadRequest {
            message: format!("failed to build SigV4 signing params: {e}"),
            status: 400,
        })?;

    let signable = SignableRequest::new(
        method,
        url,
        std::iter::empty::<(&str, &str)>(),
        SignableBody::Bytes(body),
    )
    .map_err(|e| LiterLlmError::BadRequest {
        message: format!("failed to create signable request: {e}"),
        status: 400,
    })?;

    let signing_output = sign(signable, &params.into()).map_err(|e| LiterLlmError::BadRequest {
        message: format!("SigV4 signing failed: {e}"),
        status: 400,
    })?;

    let instructions = signing_output.output();
    let signed_headers: Vec<(String, String)> = instructions
        .headers()
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect();

    Ok(signed_headers)
}

#[cfg(test)]
mod provider_tests;
#[cfg(test)]
mod tests;
