//! Semantic routing cascade — trait, built-in classifiers, and verdict cache.
//!
//! # Architecture
//!
//! This module implements a three-tier intent-based routing cascade that sits
//! *behind* [`super::router::RoutingStrategy::Semantic`].  When the router
//! needs to select a model it calls [`RouteClassifier::classify`]; the first
//! classifier that returns `Some(model_id)` with confidence ≥ its threshold
//! wins.
//!
//! ## Tier order (fastest → most expensive)
//!
//! 1. [`KeywordClassifier`] — regex match against prompt; `O(k·n)` per rule.
//! 2. [`EmbeddingSimilarityClassifier`] — cosine similarity between the prompt
//!    embedding and registered intent prototypes; requires one embedding call.
//! 3. [`LlmClassifier`] — delegates to an LLM to classify the intent;
//!    most flexible but costliest.
//!
//! Compose all three via [`CascadeClassifier`].  Wrap any cascade in
//! [`ClassifierVerdictCache`] to memoise results keyed on a hash of
//! `(prompt, system_prompt)`.
//!
//! ## OTel metrics
//!
//! When the `otel` feature is enabled the helpers in [`super::metrics`] emit:
//! - `gen_ai.route.classify.duration` histogram (seconds, attribute
//!   `route.classifier.tier ∈ {keyword, embedding, llm, cache}`)
//! - `gen_ai.route.classify.tier.hit` counter with the same attribute.
//!
//! Both instruments are no-ops when `otel` is disabled.

use std::collections::HashMap;
use std::future::Future;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use regex::Regex;

use crate::client::LlmClient;
use crate::types::{ChatCompletionRequest, EmbeddingInput, EmbeddingRequest, Message, SystemMessage, UserMessage};

/// Emit `gen_ai.route.classify.duration` histogram if the `otel` feature is
/// active; otherwise a zero-cost no-op.
#[inline]
fn record_classify_duration(tier: &'static str, duration_secs: f64) {
    #[cfg(feature = "otel")]
    {
        use opentelemetry::KeyValue;
        if let Some(meter) = super::metrics::global_meter() {
            meter
                .f64_histogram("gen_ai.route.classify.duration")
                .with_description("Semantic routing classifier latency per tier")
                .with_unit("s")
                .build()
                .record(duration_secs, &[KeyValue::new("route.classifier.tier", tier)]);
        }
    }
    let _ = (tier, duration_secs);
}

/// Emit `gen_ai.route.classify.tier.hit` counter if the `otel` feature is
/// active; otherwise a zero-cost no-op.
#[inline]
fn record_classify_hit(tier: &'static str) {
    #[cfg(feature = "otel")]
    {
        use opentelemetry::KeyValue;
        if let Some(meter) = super::metrics::global_meter() {
            meter
                .u64_counter("gen_ai.route.classify.tier.hit")
                .with_description("Semantic routing classifier tier hits")
                .build()
                .add(1, &[KeyValue::new("route.classifier.tier", tier)]);
        }
    }
    let _ = tier;
}

/// Immutable context passed to every [`RouteClassifier::classify`] call.
#[cfg_attr(alef, alef(skip))]
pub struct ClassifyContext<'a> {
    /// The user-facing prompt text.
    pub prompt: &'a str,
    /// Optional system prompt from the request.
    pub system_prompt: Option<&'a str>,
    /// Arbitrary metadata attached to the request (e.g. tenant, session ID).
    pub metadata: &'a HashMap<String, String>,
    /// The set of model identifiers the router currently considers available.
    pub available_models: &'a [String],
}

/// A single tier in the semantic routing cascade.
///
/// Each classifier inspects the [`ClassifyContext`] and either returns the
/// recommended model identifier or `None` to defer to the next tier.
///
/// # Object safety
///
/// The trait is object-safe — it returns `Pin<Box<dyn Future<…>>>` rather than
/// an associated `Future` type.  This allows mixing classifier implementations
/// behind `Arc<dyn RouteClassifier>` inside [`CascadeClassifier`].
///
/// # Confidence
///
/// [`RouteClassifier::confidence_threshold`] sets the minimum confidence level
/// at which this classifier commits to its verdict.  If a classifier's
/// internal confidence (e.g. cosine similarity score) is below its own
/// threshold it must return `None`.
pub trait RouteClassifier: Send + Sync + 'static {
    /// Classify a request and return the recommended model identifier.
    ///
    /// Return `None` to defer to the next classifier in the cascade.
    fn classify<'a>(
        &'a self,
        ctx: &'a ClassifyContext<'a>,
    ) -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>>;

    /// Confidence threshold below which this classifier defers to the next.
    ///
    /// The default is `0.0`, meaning the classifier always commits when it
    /// returns a non-None verdict.  Override this for score-based classifiers
    /// such as [`EmbeddingSimilarityClassifier`].
    fn confidence_threshold(&self) -> f32 {
        0.0
    }
}

/// Tier-1 classifier: regex matching against the user prompt.
///
/// Rules are evaluated in order; the first match wins.  The classifier returns
/// confidence `1.0` on any match and `0.0` (i.e. `None`) when no rule fires.
///
/// # Example
///
/// ```rust
/// use regex::Regex;
/// use liter_llm::tower::route_classify::KeywordClassifier;
///
/// let kw = KeywordClassifier::new(vec![
///     (Regex::new(r"(?i)sql|database").unwrap(), "gpt-4o".into()),
///     (Regex::new(r"(?i)poem|haiku").unwrap(),   "claude-3-5-sonnet".into()),
/// ]);
/// ```
pub struct KeywordClassifier {
    /// `(compiled pattern, model_id)` pairs evaluated in declaration order.
    rules: Vec<(Regex, String)>,
}

impl KeywordClassifier {
    /// Create a classifier from a list of `(pattern, model_id)` pairs.
    pub fn new(rules: Vec<(Regex, String)>) -> Self {
        Self { rules }
    }
}

impl RouteClassifier for KeywordClassifier {
    fn classify<'a>(
        &'a self,
        ctx: &'a ClassifyContext<'a>,
    ) -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>> {
        let start = Instant::now();
        let result = self
            .rules
            .iter()
            .find(|(pattern, _)| pattern.is_match(ctx.prompt))
            .map(|(_, model)| model.clone());

        if result.is_some() {
            record_classify_duration("keyword", start.elapsed().as_secs_f64());
            record_classify_hit("keyword");
        }

        Box::pin(async move { result })
    }

    /// Keyword matches are always decisive — confidence is 1.0 on match.
    fn confidence_threshold(&self) -> f32 {
        0.0
    }
}

/// An intent prototype: `(intent_name, prototype_embedding, target_model_id)`.
pub struct IntentPrototype {
    /// Human-readable name for the intent (used in logs/metrics).
    pub name: String,
    /// Pre-computed embedding vector for this intent.
    pub embedding: Vec<f64>,
    /// Model to route to when this intent is detected.
    pub model: String,
}

/// Tier-2 classifier: nearest-prototype cosine similarity.
///
/// On each call the classifier:
/// 1. Embeds `ctx.prompt` via the provided [`LlmClient`].
/// 2. Computes cosine similarity between the prompt embedding and every
///    registered [`IntentPrototype`].
/// 3. Returns the model for the most similar prototype if the similarity score
///    is ≥ [`confidence_threshold`](Self::confidence_threshold).
///
/// # Embedding model
///
/// Pass any model string understood by the embedding provider, e.g.
/// `"text-embedding-3-small"`.
pub struct EmbeddingSimilarityClassifier {
    /// Client used to embed the incoming prompt.
    client: Arc<dyn LlmClient>,
    /// Embedding model identifier (e.g. `"text-embedding-3-small"`).
    embedding_model: String,
    /// Registered intent prototypes.
    prototypes: Vec<IntentPrototype>,
    /// Minimum cosine similarity for the classifier to commit.
    threshold: f64,
}

impl EmbeddingSimilarityClassifier {
    /// Build a classifier.
    ///
    /// - `client` — any `LlmClient` implementation; used only for `.embed()`.
    /// - `embedding_model` — model string passed to the embedding endpoint.
    /// - `prototypes` — list of intent prototypes; can be added later via
    ///   [`add_prototype`](Self::add_prototype) if the classifier is wrapped
    ///   in a mutex.
    /// - `threshold` — cosine similarity in `[0, 1]` below which the
    ///   classifier defers.
    pub fn new(
        client: Arc<dyn LlmClient>,
        embedding_model: impl Into<String>,
        prototypes: Vec<IntentPrototype>,
        threshold: f64,
    ) -> Self {
        Self {
            client,
            embedding_model: embedding_model.into(),
            prototypes,
            threshold,
        }
    }
}

/// Compute cosine similarity between two vectors.
///
/// Returns `0.0` when either vector has zero magnitude.
fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

impl RouteClassifier for EmbeddingSimilarityClassifier {
    fn classify<'a>(
        &'a self,
        ctx: &'a ClassifyContext<'a>,
    ) -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>> {
        Box::pin(async move {
            let start = Instant::now();

            let embed_req = EmbeddingRequest {
                model: self.embedding_model.clone(),
                input: EmbeddingInput::Single(ctx.prompt.to_owned()),
                encoding_format: None,
                dimensions: None,
                user: None,
            };

            let resp = match self.client.embed(embed_req).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(error = %e, "embedding classifier: embed request failed; deferring");
                    return None;
                }
            };

            // ~keep Embedding responses arrive as 32-bit floats; widen them to 64-bit
            // ~keep for the cosine comparison against the higher-precision prototypes.
            let prompt_vec: Vec<f64> = match resp.data.into_iter().next() {
                Some(obj) => obj.embedding.into_iter().map(f64::from).collect(),
                None => {
                    tracing::warn!("embedding classifier: empty embedding response; deferring");
                    return None;
                }
            };

            let best = self
                .prototypes
                .iter()
                .map(|p| (cosine_similarity(&prompt_vec, &p.embedding), p))
                .max_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            match best {
                Some((score, prototype)) if score >= self.threshold => {
                    record_classify_duration("embedding", start.elapsed().as_secs_f64());
                    record_classify_hit("embedding");
                    tracing::debug!(
                        intent = %prototype.name,
                        model = %prototype.model,
                        score,
                        "embedding classifier: routed to intent prototype"
                    );
                    Some(prototype.model.clone())
                }
                Some((score, _)) => {
                    tracing::debug!(
                        score,
                        threshold = self.threshold,
                        "embedding classifier: best score below threshold; deferring"
                    );
                    None
                }
                None => None,
            }
        })
    }

    fn confidence_threshold(&self) -> f32 {
        #[allow(clippy::cast_possible_truncation)]
        let t = self.threshold as f32;
        t
    }
}

/// Tier-3 classifier: LLM-based intent classification.
///
/// Sends a structured prompt to a chat model asking it to identify which of
/// the `available_models` should handle the request.  The model is expected
/// to reply with a single JSON object `{"model": "<model_id>"}`.
///
/// This is the most flexible but also the most expensive tier and should be
/// placed last in the cascade.
pub struct LlmClassifier {
    /// Client used for the classification chat call.
    client: Arc<dyn LlmClient>,
    /// Model to use for the classification call (e.g. `"gpt-4o-mini"`).
    model: String,
    /// System prompt instructing the model to act as a router.
    system_prompt: String,
}

impl LlmClassifier {
    /// Create an LLM classifier.
    ///
    /// - `client` — `LlmClient` used for the classification chat call.
    /// - `model` — identifier of the model that performs classification.
    /// - `system_prompt` — instructions for the classification model.
    pub fn new(client: Arc<dyn LlmClient>, model: impl Into<String>, system_prompt: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
            system_prompt: system_prompt.into(),
        }
    }

    /// Build the routing prompt for the classification call.
    fn build_routing_prompt(ctx: &ClassifyContext<'_>) -> String {
        let models = ctx.available_models.join(", ");
        format!(
            "Available models: [{models}]\n\
             User prompt: {prompt}\n\n\
             Respond with ONLY a JSON object in this exact format: {{\"model\": \"<model_id>\"}}\n\
             Choose the most appropriate model from the available models list.",
            models = models,
            prompt = ctx.prompt,
        )
    }

    /// Parse the model ID from a JSON string like `{"model":"gpt-4o"}`.
    fn parse_model_from_response(text: &str) -> Option<String> {
        let start = text.find('{')?;
        let end = text.rfind('}')?;
        if end < start {
            return None;
        }
        let json_str = &text[start..=end];

        let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
        value.get("model").and_then(|v| v.as_str()).map(ToOwned::to_owned)
    }
}

impl RouteClassifier for LlmClassifier {
    fn classify<'a>(
        &'a self,
        ctx: &'a ClassifyContext<'a>,
    ) -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>> {
        Box::pin(async move {
            let start = Instant::now();

            let routing_prompt = Self::build_routing_prompt(ctx);
            let req = ChatCompletionRequest {
                model: self.model.clone(),
                messages: vec![
                    Message::System(SystemMessage {
                        content: crate::types::UserContent::Text(self.system_prompt.clone()),
                        name: None,
                    }),
                    Message::User(UserMessage {
                        content: crate::types::UserContent::Text(routing_prompt),
                        name: None,
                    }),
                ],
                ..Default::default()
            };

            let resp = match self.client.chat(req).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(error = %e, "llm classifier: chat call failed; deferring");
                    return None;
                }
            };

            let text = resp.choices.into_iter().next().and_then(|c| c.message.text())?;

            let model_id = Self::parse_model_from_response(&text);

            if model_id.is_some() {
                record_classify_duration("llm", start.elapsed().as_secs_f64());
                record_classify_hit("llm");
                tracing::debug!(
                    model = ?model_id,
                    "llm classifier: parsed routing decision"
                );
            } else {
                tracing::warn!(
                    raw_response = %text,
                    "llm classifier: could not parse model from response; deferring"
                );
            }

            model_id.filter(|m| ctx.available_models.contains(m))
        })
    }

    fn confidence_threshold(&self) -> f32 {
        0.0
    }
}

/// Composes N classifiers in priority order.
///
/// Calls each classifier in sequence.  The first one that returns
/// `Some(model_id)` wins.  Returns `None` when all classifiers defer.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use liter_llm::tower::route_classify::{
///     CascadeClassifier, KeywordClassifier, LlmClassifier,
/// };
/// use regex::Regex;
///
/// let cascade = CascadeClassifier::new(vec![
///     Arc::new(KeywordClassifier::new(vec![
///         (Regex::new(r"sql").unwrap(), "gpt-4o".into()),
///     ])),
///     Arc::new(LlmClassifier::new(client.clone(), "gpt-4o-mini", "…")),
/// ]);
/// ```
pub struct CascadeClassifier {
    classifiers: Vec<Arc<dyn RouteClassifier>>,
}

impl CascadeClassifier {
    /// Create a cascade from an ordered list of classifiers.
    pub fn new(classifiers: Vec<Arc<dyn RouteClassifier>>) -> Self {
        Self { classifiers }
    }
}

impl RouteClassifier for CascadeClassifier {
    fn classify<'a>(
        &'a self,
        ctx: &'a ClassifyContext<'a>,
    ) -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>> {
        Box::pin(async move {
            for classifier in &self.classifiers {
                if let Some(model) = classifier.classify(ctx).await {
                    return Some(model);
                }
            }
            None
        })
    }

    fn confidence_threshold(&self) -> f32 {
        0.0
    }
}

/// A cached classifier verdict.
struct VerdictEntry {
    model: String,
    inserted_at: Instant,
}

/// Wraps any [`RouteClassifier`] with a TTL-based in-memory verdict cache.
///
/// Cache key: a 64-bit hash of `(prompt, system_prompt)`.
/// Cache TTL: configurable, default 1 hour.
///
/// A cache hit short-circuits the inner classifier entirely.  The cache does
/// **not** store `None` verdicts — a deferred result is never cached.
pub struct ClassifierVerdictCache<C> {
    inner: C,
    ttl: Duration,
    /// `key → VerdictEntry` under an RwLock for concurrent access.
    cache: Arc<RwLock<HashMap<u64, VerdictEntry>>>,
}

impl<C: RouteClassifier> ClassifierVerdictCache<C> {
    /// Default cache TTL (1 hour).
    pub const DEFAULT_TTL: Duration = Duration::from_secs(3600);

    /// Wrap a classifier with the default 1-hour TTL.
    pub fn new(inner: C) -> Self {
        Self::with_ttl(inner, Self::DEFAULT_TTL)
    }

    /// Wrap a classifier with a custom TTL.
    pub fn with_ttl(inner: C, ttl: Duration) -> Self {
        Self {
            inner,
            ttl,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Compute the cache key for a classify context.
    fn cache_key(ctx: &ClassifyContext<'_>) -> u64 {
        let mut h = DefaultHasher::new();
        ctx.prompt.hash(&mut h);
        ctx.system_prompt.hash(&mut h);
        h.finish()
    }

    /// Look up a cached verdict, returning `None` on miss or expiry.
    fn get_cached(&self, key: u64) -> Option<String> {
        let cache = self.cache.read().ok()?;
        let entry = cache.get(&key)?;
        if entry.inserted_at.elapsed() > self.ttl {
            return None;
        }
        Some(entry.model.clone())
    }

    /// Store a verdict in the cache.
    fn put_cached(&self, key: u64, model: String) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(
                key,
                VerdictEntry {
                    model,
                    inserted_at: Instant::now(),
                },
            );
        }
    }
}

impl<C: RouteClassifier> RouteClassifier for ClassifierVerdictCache<C> {
    fn classify<'a>(
        &'a self,
        ctx: &'a ClassifyContext<'a>,
    ) -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>> {
        Box::pin(async move {
            let key = Self::cache_key(ctx);

            if let Some(model) = self.get_cached(key) {
                record_classify_hit("cache");
                tracing::debug!(%model, "classifier verdict cache hit");
                return Some(model);
            }

            let result = self.inner.classify(ctx).await;

            if let Some(ref model) = result {
                self.put_cached(key, model.clone());
            }

            result
        })
    }

    fn confidence_threshold(&self) -> f32 {
        self.inner.confidence_threshold()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use regex::Regex;

    use super::*;
    use crate::client::{BoxFuture, BoxStream, LlmClient};
    use crate::error::{LiterLlmError, Result};
    use crate::types::audio::{CreateSpeechRequest, CreateTranscriptionRequest, TranscriptionResponse};
    use crate::types::image::{CreateImageRequest, ImagesResponse};
    use crate::types::moderation::{ModerationRequest, ModerationResponse};
    use crate::types::ocr::{OcrRequest, OcrResponse};
    use crate::types::rerank::{RerankRequest, RerankResponse};
    use crate::types::search::{SearchRequest, SearchResponse};
    use crate::types::{
        AssistantMessage, ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, Choice, EmbeddingObject,
        EmbeddingResponse, FinishReason, ModelsListResponse, Usage,
    };

    fn empty_meta() -> HashMap<String, String> {
        HashMap::new()
    }

    fn models(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn ctx<'a>(prompt: &'a str, meta: &'a HashMap<String, String>, available: &'a [String]) -> ClassifyContext<'a> {
        ClassifyContext {
            prompt,
            system_prompt: None,
            metadata: meta,
            available_models: available,
        }
    }

    /// A minimal mock that controls chat and embed responses independently.
    #[derive(Clone)]
    struct MockLlmClient {
        chat_response: Option<String>,
        embed_vec: Option<Vec<f32>>,
        chat_call_count: Arc<AtomicUsize>,
        embed_call_count: Arc<AtomicUsize>,
    }

    impl MockLlmClient {
        fn with_chat_response(response: impl Into<String>) -> Self {
            Self {
                chat_response: Some(response.into()),
                embed_vec: None,
                chat_call_count: Arc::new(AtomicUsize::new(0)),
                embed_call_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn with_embed_vec(vec: Vec<f32>) -> Self {
            Self {
                chat_response: None,
                embed_vec: Some(vec),
                chat_call_count: Arc::new(AtomicUsize::new(0)),
                embed_call_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn no_response() -> Self {
            Self {
                chat_response: None,
                embed_vec: None,
                chat_call_count: Arc::new(AtomicUsize::new(0)),
                embed_call_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        #[allow(dead_code)]
        fn chat_calls(&self) -> usize {
            self.chat_call_count.load(Ordering::SeqCst)
        }

        #[allow(dead_code)]
        fn embed_calls(&self) -> usize {
            self.embed_call_count.load(Ordering::SeqCst)
        }
    }

    impl LlmClient for MockLlmClient {
        fn chat(&self, _req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>> {
            self.chat_call_count.fetch_add(1, Ordering::SeqCst);
            let response = self.chat_response.clone();
            Box::pin(async move {
                match response {
                    Some(text) => Ok(ChatCompletionResponse {
                        id: "test".into(),
                        object: "chat.completion".into(),
                        created: 0,
                        model: "classifier-model".into(),
                        choices: vec![Choice {
                            index: 0,
                            message: AssistantMessage {
                                content: Some(text.into()),
                                name: None,
                                tool_calls: None,
                                refusal: None,
                                function_call: None,
                                reasoning_content: None,
                            },
                            finish_reason: Some(FinishReason::Stop),
                        }],
                        usage: Some(Usage {
                            prompt_tokens: 10,
                            completion_tokens: 5,
                            total_tokens: 15,
                            prompt_tokens_details: None,
                        }),
                        system_fingerprint: None,
                        service_tier: None,
                    }),
                    None => Err(LiterLlmError::ServerError {
                        message: "no response configured".into(),
                        status: 500,
                    }),
                }
            })
        }

        fn chat_stream(
            &self,
            _req: ChatCompletionRequest,
        ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>> {
            Box::pin(async move {
                Err(LiterLlmError::EndpointNotSupported {
                    endpoint: "chat_stream".into(),
                    provider: "mock".into(),
                })
            })
        }

        fn embed(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>> {
            self.embed_call_count.fetch_add(1, Ordering::SeqCst);
            let vec = self.embed_vec.clone();
            Box::pin(async move {
                match vec {
                    Some(embedding) => Ok(EmbeddingResponse {
                        object: "list".into(),
                        data: vec![EmbeddingObject {
                            object: "embedding".into(),
                            embedding,
                            index: 0,
                        }],
                        model: req.model,
                        usage: None,
                    }),
                    None => Err(LiterLlmError::ServerError {
                        message: "no embed vec configured".into(),
                        status: 500,
                    }),
                }
            })
        }

        fn list_models(&self) -> BoxFuture<'_, Result<ModelsListResponse>> {
            Box::pin(async move {
                Ok(ModelsListResponse {
                    object: "list".into(),
                    data: vec![],
                })
            })
        }

        fn image_generate(&self, _req: CreateImageRequest) -> BoxFuture<'_, Result<ImagesResponse>> {
            Box::pin(async move {
                Ok(ImagesResponse {
                    created: 0,
                    data: vec![],
                })
            })
        }

        fn speech(&self, _req: CreateSpeechRequest) -> BoxFuture<'_, Result<bytes::Bytes>> {
            Box::pin(async move { Ok(bytes::Bytes::new()) })
        }

        fn transcribe(&self, _req: CreateTranscriptionRequest) -> BoxFuture<'_, Result<TranscriptionResponse>> {
            Box::pin(async move {
                Ok(TranscriptionResponse {
                    text: String::new(),
                    language: None,
                    duration: None,
                    segments: None,
                })
            })
        }

        fn moderate(&self, _req: ModerationRequest) -> BoxFuture<'_, Result<ModerationResponse>> {
            Box::pin(async move {
                Ok(ModerationResponse {
                    id: String::new(),
                    model: String::new(),
                    results: vec![],
                })
            })
        }

        fn rerank(&self, _req: RerankRequest) -> BoxFuture<'_, Result<RerankResponse>> {
            Box::pin(async move {
                Ok(RerankResponse {
                    id: None,
                    results: vec![],
                    meta: None,
                })
            })
        }

        fn search(&self, _req: SearchRequest) -> BoxFuture<'_, Result<SearchResponse>> {
            Box::pin(async move {
                Err(LiterLlmError::EndpointNotSupported {
                    endpoint: "search".into(),
                    provider: "mock".into(),
                })
            })
        }

        fn ocr(&self, _req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>> {
            Box::pin(async move {
                Err(LiterLlmError::EndpointNotSupported {
                    endpoint: "ocr".into(),
                    provider: "mock".into(),
                })
            })
        }
    }

    #[tokio::test]
    async fn keyword_classifier_first_match_wins() {
        let rules = vec![
            (Regex::new(r"(?i)image").unwrap(), "dall-e-3".into()),
            (Regex::new(r"(?i)python|code").unwrap(), "gpt-4o".into()),
            (Regex::new(r"(?i)essay|write").unwrap(), "claude-3-5-sonnet".into()),
        ];
        let kw = KeywordClassifier::new(rules);
        let meta = empty_meta();
        let avail = models(&["dall-e-3", "gpt-4o", "claude-3-5-sonnet"]);
        let ctx = ctx("Write me python code", &meta, &avail);

        let result = kw.classify(&ctx).await;
        assert_eq!(result, Some("gpt-4o".into()));
    }

    #[tokio::test]
    async fn keyword_classifier_no_match_returns_none() {
        let rules = vec![
            (Regex::new(r"(?i)image").unwrap(), "dall-e-3".into()),
            (Regex::new(r"(?i)code").unwrap(), "gpt-4o".into()),
        ];
        let kw = KeywordClassifier::new(rules);
        let meta = empty_meta();
        let avail = models(&["dall-e-3", "gpt-4o"]);
        let ctx = ctx("Tell me a joke", &meta, &avail);

        let result = kw.classify(&ctx).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn embedding_classifier_returns_nearest_above_threshold() {
        let prototypes = vec![
            IntentPrototype {
                name: "coding".into(),
                embedding: vec![1.0, 0.0, 0.0],
                model: "gpt-4o".into(),
            },
            IntentPrototype {
                name: "creative".into(),
                embedding: vec![0.0, 1.0, 0.0],
                model: "claude-3-5-sonnet".into(),
            },
        ];

        let client = Arc::new(MockLlmClient::with_embed_vec(vec![0.9, 0.1, 0.0]));
        let classifier = EmbeddingSimilarityClassifier::new(client, "text-embedding-3-small", prototypes, 0.5);

        let meta = empty_meta();
        let avail = models(&["gpt-4o", "claude-3-5-sonnet"]);
        let ctx = ctx("Debug my Rust code", &meta, &avail);

        let result = classifier.classify(&ctx).await;
        assert_eq!(result, Some("gpt-4o".into()));
    }

    #[tokio::test]
    async fn embedding_classifier_below_threshold_returns_none() {
        let prototypes = vec![IntentPrototype {
            name: "coding".into(),
            embedding: vec![1.0, 0.0, 0.0],
            model: "gpt-4o".into(),
        }];

        let client = Arc::new(MockLlmClient::with_embed_vec(vec![0.0, 1.0, 0.0]));
        let classifier = EmbeddingSimilarityClassifier::new(client, "text-embedding-3-small", prototypes, 0.8);

        let meta = empty_meta();
        let avail = models(&["gpt-4o"]);
        let ctx = ctx("What is the weather?", &meta, &avail);

        let result = classifier.classify(&ctx).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn llm_classifier_parses_model_id_from_response() {
        let client = Arc::new(MockLlmClient::with_chat_response(r#"{"model":"gpt-4o"}"#));
        let classifier = LlmClassifier::new(
            client,
            "gpt-4o-mini",
            "You are a routing assistant. Reply with JSON only.",
        );

        let meta = empty_meta();
        let avail = models(&["gpt-4o", "claude-3-5-sonnet"]);
        let ctx = ctx("Explain quantum computing", &meta, &avail);

        let result = classifier.classify(&ctx).await;
        assert_eq!(result, Some("gpt-4o".into()));
    }

    #[tokio::test]
    async fn llm_classifier_ignores_unavailable_model() {
        let client = Arc::new(MockLlmClient::with_chat_response(r#"{"model":"unknown-model"}"#));
        let classifier = LlmClassifier::new(client, "gpt-4o-mini", "Route the request.");

        let meta = empty_meta();
        let avail = models(&["gpt-4o", "claude-3-5-sonnet"]);
        let ctx = ctx("Hello", &meta, &avail);

        let result = classifier.classify(&ctx).await;
        assert_eq!(result, None);
    }

    /// Keyword hits should short-circuit the cascade — embedding and LLM must
    /// not be called.
    #[tokio::test]
    async fn cascade_keyword_hit_short_circuits_embedding() {
        let embed_client = Arc::new(MockLlmClient::with_embed_vec(vec![1.0, 0.0]));
        let llm_client = Arc::new(MockLlmClient::with_chat_response(r#"{"model":"gpt-4o"}"#));

        let embed_call_count = Arc::clone(&embed_client.embed_call_count);
        let llm_call_count = Arc::clone(&llm_client.chat_call_count);

        let kw = KeywordClassifier::new(vec![(Regex::new(r"(?i)code").unwrap(), "gpt-4o".into())]);
        let embedding_cls = EmbeddingSimilarityClassifier::new(
            embed_client,
            "text-embedding-3-small",
            vec![IntentPrototype {
                name: "test".into(),
                embedding: vec![1.0, 0.0],
                model: "gpt-4o".into(),
            }],
            0.5,
        );
        let llm_cls = LlmClassifier::new(llm_client, "gpt-4o-mini", "Route.");

        let cascade = CascadeClassifier::new(vec![
            Arc::new(kw) as Arc<dyn RouteClassifier>,
            Arc::new(embedding_cls) as Arc<dyn RouteClassifier>,
            Arc::new(llm_cls) as Arc<dyn RouteClassifier>,
        ]);

        let meta = empty_meta();
        let avail = models(&["gpt-4o"]);
        let ctx = ctx("Write some code", &meta, &avail);

        let result = cascade.classify(&ctx).await;
        assert_eq!(result, Some("gpt-4o".into()));
        assert_eq!(
            embed_call_count.load(Ordering::SeqCst),
            0,
            "embedding should not be called"
        );
        assert_eq!(llm_call_count.load(Ordering::SeqCst), 0, "llm should not be called");
    }

    /// Keyword miss → embedding hit → LLM not called.
    #[tokio::test]
    async fn cascade_keyword_miss_falls_to_embedding() {
        let embed_client = Arc::new(MockLlmClient::with_embed_vec(vec![1.0, 0.0]));
        let llm_client = Arc::new(MockLlmClient::with_chat_response(r#"{"model":"gpt-4o"}"#));

        let llm_call_count = Arc::clone(&llm_client.chat_call_count);

        let kw = KeywordClassifier::new(vec![(Regex::new(r"(?i)image").unwrap(), "dall-e-3".into())]);
        let embedding_cls = EmbeddingSimilarityClassifier::new(
            embed_client,
            "text-embedding-3-small",
            vec![IntentPrototype {
                name: "coding".into(),
                embedding: vec![1.0, 0.0],
                model: "gpt-4o".into(),
            }],
            0.5,
        );
        let llm_cls = LlmClassifier::new(llm_client, "gpt-4o-mini", "Route.");

        let cascade = CascadeClassifier::new(vec![
            Arc::new(kw) as Arc<dyn RouteClassifier>,
            Arc::new(embedding_cls) as Arc<dyn RouteClassifier>,
            Arc::new(llm_cls) as Arc<dyn RouteClassifier>,
        ]);

        let meta = empty_meta();
        let avail = models(&["gpt-4o", "dall-e-3"]);
        let ctx = ctx("Debug my Rust program", &meta, &avail);

        let result = cascade.classify(&ctx).await;
        assert_eq!(result, Some("gpt-4o".into()));
        assert_eq!(llm_call_count.load(Ordering::SeqCst), 0);
    }

    /// All tiers defer → cascade returns None.
    #[tokio::test]
    async fn cascade_all_defer_returns_none() {
        let kw = KeywordClassifier::new(vec![]);

        let embed_client = Arc::new(MockLlmClient::with_embed_vec(vec![0.0, 1.0]));
        let embedding_cls = EmbeddingSimilarityClassifier::new(
            embed_client,
            "text-embedding-3-small",
            vec![IntentPrototype {
                name: "coding".into(),
                embedding: vec![1.0, 0.0],
                model: "gpt-4o".into(),
            }],
            0.99,
        );

        let llm_client = Arc::new(MockLlmClient::no_response());
        let llm_cls = LlmClassifier::new(llm_client, "gpt-4o-mini", "Route.");

        let cascade = CascadeClassifier::new(vec![
            Arc::new(kw) as Arc<dyn RouteClassifier>,
            Arc::new(embedding_cls) as Arc<dyn RouteClassifier>,
            Arc::new(llm_cls) as Arc<dyn RouteClassifier>,
        ]);

        let meta = empty_meta();
        let avail = models(&["gpt-4o"]);
        let ctx = ctx("What is 2+2?", &meta, &avail);

        let result = cascade.classify(&ctx).await;
        assert_eq!(result, None);
    }

    /// A simple counting classifier that records how many times it was called.
    struct CountingClassifier {
        model: String,
        call_count: Arc<AtomicUsize>,
    }

    impl RouteClassifier for CountingClassifier {
        fn classify<'a>(
            &'a self,
            _ctx: &'a ClassifyContext<'a>,
        ) -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let model = self.model.clone();
            Box::pin(async move { Some(model) })
        }
    }

    #[tokio::test]
    async fn classifier_verdict_cache_returns_cached_verdict_within_ttl() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let inner = CountingClassifier {
            model: "gpt-4o".into(),
            call_count: Arc::clone(&call_count),
        };
        let cached = ClassifierVerdictCache::new(inner);

        let meta = empty_meta();
        let avail = models(&["gpt-4o"]);
        let prompt = "Tell me a joke";

        let ctx1 = ClassifyContext {
            prompt,
            system_prompt: None,
            metadata: &meta,
            available_models: &avail,
        };
        let r1 = cached.classify(&ctx1).await;
        assert_eq!(r1, Some("gpt-4o".into()));
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        let ctx2 = ClassifyContext {
            prompt,
            system_prompt: None,
            metadata: &meta,
            available_models: &avail,
        };
        let r2 = cached.classify(&ctx2).await;
        assert_eq!(r2, Some("gpt-4o".into()));
        assert_eq!(call_count.load(Ordering::SeqCst), 1, "inner should not be called again");
    }

    #[tokio::test]
    async fn classifier_verdict_cache_expires_after_ttl() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let inner = CountingClassifier {
            model: "gpt-4o".into(),
            call_count: Arc::clone(&call_count),
        };
        let cached = ClassifierVerdictCache::with_ttl(inner, Duration::from_millis(10));

        let meta = empty_meta();
        let avail = models(&["gpt-4o"]);
        let prompt = "Expire me";

        let ctx1 = ClassifyContext {
            prompt,
            system_prompt: None,
            metadata: &meta,
            available_models: &avail,
        };
        let r1 = cached.classify(&ctx1).await;
        assert_eq!(r1, Some("gpt-4o".into()));
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        tokio::time::sleep(Duration::from_millis(20)).await;

        let ctx2 = ClassifyContext {
            prompt,
            system_prompt: None,
            metadata: &meta,
            available_models: &avail,
        };
        let r2 = cached.classify(&ctx2).await;
        assert_eq!(r2, Some("gpt-4o".into()));
        assert_eq!(call_count.load(Ordering::SeqCst), 2, "inner should be called after TTL");
    }

    #[test]
    fn cosine_similarity_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6, "identical vectors should have similarity 1.0");
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6, "orthogonal vectors should have similarity ~0");
    }

    #[test]
    fn cosine_similarity_zero_vector() {
        let zero = vec![0.0, 0.0];
        let other = vec![1.0, 1.0];
        let sim = cosine_similarity(&zero, &other);
        assert_eq!(sim, 0.0, "zero vector should yield similarity 0.0");
    }

    #[test]
    fn llm_classifier_parse_with_preamble() {
        let text = r#"Sure! Here is my answer: {"model": "claude-3-5-sonnet"} Hope that helps!"#;
        let result = LlmClassifier::parse_model_from_response(text);
        assert_eq!(result, Some("claude-3-5-sonnet".into()));
    }

    #[test]
    fn llm_classifier_parse_malformed_returns_none() {
        let result = LlmClassifier::parse_model_from_response("I cannot decide.");
        assert_eq!(result, None);
    }
}
