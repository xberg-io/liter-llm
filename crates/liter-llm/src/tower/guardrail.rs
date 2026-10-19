//! Tower middleware layer that enforces guardrail checks at each request stage.
//!
//! [`GuardrailLayer`] wraps any [`Service<LlmRequest>`] and runs the registered
//! guardrails at three lifecycle points:
//!
//! - **`Input`** — before forwarding the request to the inner service. A
//!   `Block` decision returns [`LiterLlmError::HookRejected`] immediately;
//!   `Mutate` rewrites the request that is forwarded.
//! - **`Output`** — after the inner service returns a non-streaming response.
//!   A `Block` decision returns an error; `Mutate` replaces the response JSON.
//! - **`OutputChunk`** — for each streaming chunk. A `Block` decision
//!   terminates the stream; `Mutate` replaces the chunk text.
//!
//! # Example
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use liter_llm::guardrail::registry::GuardrailRegistry;
//! use liter_llm::tower::guardrail::GuardrailLayer;
//! use tower::ServiceBuilder;
//!
//! let registry = Arc::new(GuardrailRegistry::new());
//! let service = ServiceBuilder::new()
//!     .layer(GuardrailLayer::new(registry, Default::default()))
//!     .service(inner_service);
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_core::Stream;
use tower::Layer;
use tower::Service;

use crate::client::{BoxFuture, BoxStream};
use crate::error::{LiterLlmError, Result};
use crate::guardrail::registry::GuardrailRegistry;
use crate::guardrail::{GuardrailContext, GuardrailDecision, GuardrailStage};
use crate::types::ChatCompletionChunk;

use super::types::{LlmRequest, LlmRequestKind, LlmResponse};

/// Serialize a guardrail-inspectable response body, failing closed.
///
/// ~keep A response that cannot be serialized is treated as blocked rather
/// ~keep than passed through unchecked: silently falling back to `Ok(response)`
/// ~keep here would let arbitrary un-inspected content reach the caller
/// ~keep whenever a response happens to serialize badly, defeating the
/// ~keep purpose of the Output guardrail stage.
fn serialize_for_guardrail<T: serde::Serialize>(value: &T) -> Result<serde_json::Value> {
    serde_json::to_value(value).map_err(|e| LiterLlmError::InternalError {
        message: format!("guardrail: failed to serialize response for output-stage inspection: {e}"),
    })
}

/// Build the JSON payload the `Input` guardrail stage inspects.
///
/// `LlmRequest` serializes as its `kind` alone, so this is the provider
/// payload only — `tenant_id` and `idempotency_key` are never shown to a
/// guardrail and so can never be rewritten by one. ~keep
fn request_to_guardrail_json(request: &LlmRequest) -> Result<serde_json::Value> {
    serde_json::to_value(request).map_err(|e| LiterLlmError::InternalError {
        message: format!("guardrail: failed to serialize request: {e}"),
    })
}

/// `GuardrailContext::metadata` key carrying the request's tenant identity.
///
/// ~keep Part of the guardrail public contract: `AllowListGuardrail` /
/// ~keep `DenyListGuardrail` configured on this field name rely on it being
/// ~keep populated whenever `LlmRequest::tenant_id` is set. Renaming this
/// ~keep constant is a breaking change for any deployed guardrail expression.
/// ~keep Exported so a caller that assembles the context itself — the proxy's realtime
/// ~keep relay proxies raw frames and has no `LlmRequest` to derive one from — uses this
/// ~keep exact key rather than its own copy of the string. Two spellings would make a
/// ~keep configured deny-list live on one path and silently dead on the other.
pub const TENANT_ID_METADATA_KEY: &str = "tenant_id";

/// Merge the layer's static metadata with per-call facts derived from `request`.
///
/// ~keep Only `tenant_id` is populated from the request today. `model` and
/// ~keep `user` were considered and rejected: `LlmRequest::model()` returns
/// ~keep `None` for `ListModels`, and the provider-level `user` field exists on
/// ~keep only some request kinds — either one would be structurally absent for
/// ~keep some request shapes rather than merely unset by the caller, which
/// ~keep reproduces the exact fail-open/fail-closed ambiguity this fix closes
/// ~keep for `tenant_id`, just triggered by request shape instead of caller
/// ~keep omission. Both remain fully inspectable by content-based guardrails
/// ~keep (Regex, LengthCap, CEL) via `ctx.request`, so there is no coverage gap
/// ~keep in leaving them out of `metadata`. `idempotency_key` is excluded as a
/// ~keep matter of policy: it is an infra-only dedup token with no bearing on
/// ~keep content or identity, already deliberately kept out of the guardrail's
/// ~keep view of the request by `request_to_guardrail_json`.
///
/// ~keep Static (layer-configured) values always win on key collision: an
/// ~keep operator who set a key via `GuardrailLayer::new`'s metadata map made a
/// ~keep deliberate choice that per-call plumbing must not silently override.
/// ~keep The collision is still surfaced via a WARN trace event rather than
/// ~keep swallowed, so a static default that unexpectedly shadows a real
/// ~keep per-call fact is diagnosable instead of silently wrong — the same
/// ~keep failure mode this function exists to close for `tenant_id` itself.
///
/// Returns the layer's `Arc` unchanged, without allocating, whenever the
/// request contributes no per-call facts (no `tenant_id` set) or the one fact
/// it contributes collides with an existing static key.
fn build_call_metadata(
    layer_metadata: &Arc<HashMap<String, String>>,
    request: &LlmRequest,
) -> Arc<HashMap<String, String>> {
    let Some(tenant_id) = request.tenant_id() else {
        return Arc::clone(layer_metadata);
    };

    if layer_metadata.contains_key(TENANT_ID_METADATA_KEY) {
        tracing::warn!(
            metadata_key = TENANT_ID_METADATA_KEY,
            "guardrail: static per-layer metadata already defines this key; discarding the per-call value"
        );
        return Arc::clone(layer_metadata);
    }

    let mut merged = (**layer_metadata).clone();
    merged.insert(TENANT_ID_METADATA_KEY.to_owned(), tenant_id.as_ref().to_owned());
    Arc::new(merged)
}

/// Apply an `Input`-stage `Mutate` decision to the request.
///
/// ~keep Fails closed. Forwarding the original request when the replacement
/// ~keep cannot be applied is how a redaction guardrail comes to leak exactly
/// ~keep the content it was installed to remove, so a payload that does not
/// ~keep deserialize aborts the request instead.
///
/// ~keep The operation type is pinned to the original: a guardrail may rewrite
/// ~keep the payload of the call being made, not turn a chat completion into a
/// ~keep different operation. `tenant_id` and `idempotency_key` are carried
/// ~keep over from the original for the same reason.
fn apply_request_mutation(request: LlmRequest, new_payload: serde_json::Value) -> Result<LlmRequest> {
    let mutated: LlmRequestKind = serde_json::from_value(new_payload).map_err(|e| LiterLlmError::InternalError {
        message: format!("guardrail: Input stage Mutate payload is not a valid request: {e}"),
    })?;

    if std::mem::discriminant(&mutated) != std::mem::discriminant(&request.kind) {
        return Err(LiterLlmError::InternalError {
            message: "guardrail: Input stage Mutate payload changed the operation type".to_owned(),
        });
    }

    Ok(LlmRequest {
        kind: mutated,
        tenant_id: request.tenant_id,
        idempotency_key: request.idempotency_key,
    })
}

/// Apply an `Output`-stage `Mutate` decision to the response.
///
/// Deserializes into the same variant the original response carried, and fails
/// closed for the same reason as [`apply_request_mutation`]. ~keep
fn apply_response_mutation(response: LlmResponse, new_payload: serde_json::Value) -> Result<LlmResponse> {
    fn parse<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Result<T> {
        serde_json::from_value(value).map_err(|e| LiterLlmError::InternalError {
            message: format!("guardrail: Output stage Mutate payload is not a valid response: {e}"),
        })
    }

    match response {
        LlmResponse::Chat(_) => parse(new_payload).map(LlmResponse::Chat),
        LlmResponse::Embed(_) => parse(new_payload).map(LlmResponse::Embed),
        LlmResponse::ListModels(_) => parse(new_payload).map(LlmResponse::ListModels),
        LlmResponse::ImageGenerate(_) => parse(new_payload).map(LlmResponse::ImageGenerate),
        LlmResponse::Transcribe(_) => parse(new_payload).map(LlmResponse::Transcribe),
        LlmResponse::Moderate(_) => parse(new_payload).map(LlmResponse::Moderate),
        LlmResponse::Rerank(_) => parse(new_payload).map(LlmResponse::Rerank),
        LlmResponse::Search(_) => parse(new_payload).map(LlmResponse::Search),
        LlmResponse::Ocr(_) => parse(new_payload).map(LlmResponse::Ocr),
        // ~keep Speech is inspected as a synthetic {"byte_len": N} summary rather than
        // ~keep the audio itself, so there is no payload a guardrail could rewrite;
        // ~keep ChatStream is guarded per-chunk and never reaches the Output stage.
        LlmResponse::Speech(_) | LlmResponse::ChatStream(_) => Err(LiterLlmError::InternalError {
            message: "guardrail: Output stage Mutate is not supported for this response type".to_owned(),
        }),
    }
}

/// Build the JSON payload the `Output` guardrail stage inspects for a given
/// response, or `None` when no aggregate body is available yet to inspect.
fn response_to_guardrail_json(response: &LlmResponse) -> Result<Option<serde_json::Value>> {
    match response {
        LlmResponse::Chat(r) => serialize_for_guardrail(r).map(Some),
        LlmResponse::Embed(r) => serialize_for_guardrail(r).map(Some),
        LlmResponse::ListModels(r) => serialize_for_guardrail(r).map(Some),
        LlmResponse::ImageGenerate(r) => serialize_for_guardrail(r).map(Some),
        LlmResponse::Transcribe(r) => serialize_for_guardrail(r).map(Some),
        LlmResponse::Moderate(r) => serialize_for_guardrail(r).map(Some),
        LlmResponse::Rerank(r) => serialize_for_guardrail(r).map(Some),
        LlmResponse::Search(r) => serialize_for_guardrail(r).map(Some),
        LlmResponse::Ocr(r) => serialize_for_guardrail(r).map(Some),
        LlmResponse::Speech(audio_bytes) => Ok(Some(serde_json::json!({
            // ~keep Speech returns raw audio bytes, not a serializable struct.
            // ~keep Emitting the bytes verbatim would blow up the guardrail
            // ~keep payload; exposing the length is still enough for e.g.
            // ~keep length-cap guardrails to act on.
            "byte_len": audio_bytes.len(),
        }))),
        LlmResponse::ChatStream(_) => Ok(None),
    }
}

/// Extract the inspectable text of a single streamed chunk by joining the
/// `content` delta of every choice in the chunk.
///
/// Returns an empty string for chunks that carry no textual delta (e.g. a
/// role-only first chunk, a tool-call delta, or a trailing usage-only
/// chunk) — callers should treat an empty result as "nothing to inspect".
fn chunk_text(chunk: &ChatCompletionChunk) -> String {
    chunk
        .choices
        .iter()
        .filter_map(|choice| choice.delta.content.as_deref())
        .collect::<Vec<_>>()
        .join("")
}

/// Run the `OutputChunk` guardrail stage against a single streamed chunk.
///
/// ~keep `GuardrailContext::chunk` is a single `&str`, not per-choice, so a
/// ~keep `Mutate` decision replaces the `content` delta of every choice in the
/// ~keep chunk with the same redacted text. This matches the common `n == 1`
/// ~keep streaming case; multi-choice (`n > 1`) streams are guarded jointly.
async fn apply_output_chunk_guardrail(
    mut chunk: ChatCompletionChunk,
    registry: &GuardrailRegistry,
    request_json: &serde_json::Value,
    metadata: &HashMap<String, String>,
) -> Result<ChatCompletionChunk> {
    let text = chunk_text(&chunk);
    if text.is_empty() {
        return Ok(chunk);
    }

    let ctx = GuardrailContext {
        request: request_json,
        response: None,
        chunk: Some(&text),
        metadata,
    };

    match registry.run_stage(GuardrailStage::OutputChunk, &ctx).await {
        GuardrailDecision::Block { reason, code } => Err(LiterLlmError::HookRejected {
            message: format!("guardrail blocked output chunk [code={code}]: {reason}"),
        }),
        GuardrailDecision::Mutate { new_payload } => {
            let replacement = new_payload.as_str().unwrap_or_default().to_owned();
            for choice in &mut chunk.choices {
                if choice.delta.content.is_some() {
                    choice.delta.content = Some(replacement.clone());
                }
            }
            Ok(chunk)
        }
        GuardrailDecision::Allow => Ok(chunk),
    }
}

/// `Stream` adapter that runs the `OutputChunk` guardrail stage over each
/// chunk of a `ChatStream` response as it is polled.
///
/// # Blocking policy
///
/// ~keep A blocked chunk terminates the stream: the block is yielded once as
/// ~keep `Err(HookRejected)`, and every subsequent poll returns `None` rather
/// ~keep than continuing to yield later chunks. Chunks already handed to the
/// ~keep caller cannot be recalled — but this at minimum guarantees no further
/// ~keep content, blocked or not, reaches the caller after a violation is
/// ~keep detected, which mirrors the fail-closed policy this layer already
/// ~keep applies to the non-streaming `Output` stage. Emitting a redacted
/// ~keep replacement chunk and continuing was considered and rejected: it
/// ~keep would let the stream keep running past a detected violation on the
/// ~keep hope that only that one chunk was bad, silently downgrading a
/// ~keep "block" decision to a "redact" decision the guardrail never made.
struct GuardedChunkStream {
    inner: BoxStream<'static, Result<ChatCompletionChunk>>,
    registry: Arc<GuardrailRegistry>,
    request_json: Arc<serde_json::Value>,
    metadata: Arc<HashMap<String, String>>,
    pending: Option<Pin<Box<dyn Future<Output = Result<ChatCompletionChunk>> + Send>>>,
    blocked: bool,
}

impl Stream for GuardedChunkStream {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if this.blocked {
            return Poll::Ready(None);
        }

        loop {
            if let Some(fut) = this.pending.as_mut() {
                return match fut.as_mut().poll(cx) {
                    Poll::Ready(result) => {
                        this.pending = None;
                        if result.is_err() {
                            this.blocked = true;
                        }
                        Poll::Ready(Some(result))
                    }
                    Poll::Pending => Poll::Pending,
                };
            }

            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    let registry = Arc::clone(&this.registry);
                    let request_json = Arc::clone(&this.request_json);
                    let metadata = Arc::clone(&this.metadata);
                    this.pending = Some(Box::pin(async move {
                        apply_output_chunk_guardrail(chunk, &registry, &request_json, &metadata).await
                    }));
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Wrap a `ChatStream` response so each chunk is passed through the
/// `OutputChunk` guardrail stage as it is polled. See [`GuardedChunkStream`]
/// for the blocking policy.
fn guard_output_chunk_stream(
    stream: BoxStream<'static, Result<ChatCompletionChunk>>,
    registry: Arc<GuardrailRegistry>,
    request_json: Arc<serde_json::Value>,
    metadata: Arc<HashMap<String, String>>,
) -> BoxStream<'static, Result<ChatCompletionChunk>> {
    Box::pin(GuardedChunkStream {
        inner: stream,
        registry,
        request_json,
        metadata,
        pending: None,
        blocked: false,
    })
}

/// Tower [`Layer`] that enforces guardrail checks around an inner service.
///
/// `registry` holds the ordered list of guardrails to evaluate.
/// `metadata` provides per-layer static tags (e.g., route, deployment) that are
/// merged with per-call metadata derived from each [`LlmRequest`] — currently
/// just `tenant_id` (see `build_call_metadata`). Static values win on a key
/// collision.
#[cfg_attr(alef, alef(skip))]
#[derive(Clone)]
pub struct GuardrailLayer {
    registry: Arc<GuardrailRegistry>,
    metadata: Arc<HashMap<String, String>>,
}

impl GuardrailLayer {
    /// Create a new [`GuardrailLayer`] with the given registry and static metadata.
    ///
    /// `metadata` is merged into the [`GuardrailContext`] for every request,
    /// alongside per-call facts this layer derives automatically from the
    /// [`LlmRequest`] being served (currently `tenant_id`). This constructor
    /// accepts the layer-level static tags only; static keys take precedence
    /// over same-named per-call facts.
    #[must_use]
    pub fn new(registry: Arc<GuardrailRegistry>, metadata: HashMap<String, String>) -> Self {
        Self {
            registry,
            metadata: Arc::new(metadata),
        }
    }

    /// Create a new [`GuardrailLayer`] with no static metadata.
    #[must_use]
    pub fn with_registry(registry: Arc<GuardrailRegistry>) -> Self {
        Self::new(registry, HashMap::new())
    }
}

impl<S> Layer<S> for GuardrailLayer {
    type Service = GuardrailService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GuardrailService {
            inner,
            registry: Arc::clone(&self.registry),
            metadata: Arc::clone(&self.metadata),
        }
    }
}

/// Tower service produced by [`GuardrailLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct GuardrailService<S> {
    inner: S,
    registry: Arc<GuardrailRegistry>,
    metadata: Arc<HashMap<String, String>>,
}

impl<S: Clone> Clone for GuardrailService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            registry: Arc::clone(&self.registry),
            metadata: Arc::clone(&self.metadata),
        }
    }
}

impl<S> Service<LlmRequest> for GuardrailService<S>
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: LlmRequest) -> Self::Future {
        let registry = Arc::clone(&self.registry);
        // ~keep Skip the per-call metadata merge entirely when no guardrail is
        // ~keep registered: with the feature compiled in but unused (the common
        // ~keep shape today — see GuardrailLayer's own docs), this avoids a
        // ~keep HashMap clone + Arc allocation on every request that carries a
        // ~keep tenant_id, for a metadata map nothing would ever read.
        let metadata = if registry.is_empty() {
            Arc::clone(&self.metadata)
        } else {
            build_call_metadata(&self.metadata, &req)
        };

        // ~keep The Input stage can rewrite the request, so the inner call must be made
        // ~keep inside the future, after that decision is known. Consume the polled-ready
        // ~keep instance and leave a fresh standby clone, matching BudgetLedgerService.
        let standby = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, standby);

        Box::pin(async move {
            let request_json = Arc::new(request_to_guardrail_json(&req)?);

            let input_ctx = GuardrailContext {
                request: &request_json,
                response: None,
                chunk: None,
                metadata: &metadata,
            };

            let input_decision = registry.run_stage(GuardrailStage::Input, &input_ctx).await;
            let request_json = match input_decision {
                GuardrailDecision::Block { reason, code } => {
                    return Err(LiterLlmError::HookRejected {
                        message: format!("guardrail blocked [code={code}]: {reason}"),
                    });
                }
                GuardrailDecision::Mutate { new_payload } => {
                    req = apply_request_mutation(req, new_payload)?;
                    // ~keep Re-serialize so the later stages inspect the request that was
                    // ~keep actually sent rather than the pre-mutation original.
                    Arc::new(request_to_guardrail_json(&req)?)
                }
                GuardrailDecision::Allow => request_json,
            };

            let response = inner.call(req).await?;

            // ~keep ChatStream: no aggregate body exists yet at this point to run an
            // ~keep Output-stage guardrail against — instead, each chunk is passed
            // ~keep through the OutputChunk stage as it is polled, via
            // ~keep guard_output_chunk_stream. See GuardedChunkStream's doc comment
            // ~keep for the mid-stream blocking policy.
            if let LlmResponse::ChatStream(stream) = response {
                let guarded = guard_output_chunk_stream(
                    stream,
                    Arc::clone(&registry),
                    Arc::clone(&request_json),
                    Arc::clone(&metadata),
                );
                return Ok(LlmResponse::ChatStream(guarded));
            }

            let Some(response_json) = response_to_guardrail_json(&response)? else {
                return Ok(response);
            };

            let output_ctx = GuardrailContext {
                request: &request_json,
                response: Some(&response_json),
                chunk: None,
                metadata: &metadata,
            };

            let output_decision = registry.run_stage(GuardrailStage::Output, &output_ctx).await;
            match output_decision {
                GuardrailDecision::Block { reason, code } => Err(LiterLlmError::HookRejected {
                    message: format!("guardrail blocked output [code={code}]: {reason}"),
                }),
                GuardrailDecision::Mutate { new_payload } => apply_response_mutation(response, new_payload),
                GuardrailDecision::Allow => Ok(response),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    use tower::{Layer, Service};

    use super::*;
    use crate::guardrail::Guardrail;
    use crate::guardrail::builtin::{AllowListGuardrail, DenyListGuardrail};
    use crate::guardrail::registry::GuardrailRegistry;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req, make_chat_response};
    use crate::tower::types::LlmRequest;
    use crate::types::audio::{CreateSpeechRequest, CreateTranscriptionRequest, TranscriptionResponse};
    use crate::types::common::{AssistantContent, Message, UserMessage};
    use crate::types::image::{CreateImageRequest, ImagesResponse};
    use crate::types::moderation::{ModerationRequest, ModerationResponse};
    use crate::types::ocr::{OcrRequest, OcrResponse};
    use crate::types::rerank::{RerankRequest, RerankResponse};
    use crate::types::search::{SearchRequest, SearchResponse};

    #[tokio::test]
    async fn guardrail_layer_allows_when_registry_is_empty() {
        let registry = Arc::new(GuardrailRegistry::new());
        let inner = LlmService::new(MockClient::ok());
        let mut svc = GuardrailLayer::with_registry(registry).layer(inner);

        let result = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(result.is_ok(), "empty registry should allow all requests");
    }

    #[tokio::test]
    async fn guardrail_layer_input_block_prevents_inner_call() {
        let mut registry = GuardrailRegistry::new();
        let list: HashSet<String> = ["banned-user"].iter().map(|s| s.to_string()).collect();
        registry.register(Arc::new(DenyListGuardrail::new("ban", list, "user_id")));

        let mock = MockClient::ok();
        let call_count = Arc::clone(&mock.call_count);
        let inner = LlmService::new(mock);

        let mut meta = HashMap::new();
        meta.insert("user_id".to_string(), "banned-user".to_string());

        let mut svc = GuardrailLayer::new(Arc::new(registry), meta).layer(inner);
        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("banned user should be blocked");

        assert!(
            matches!(err, LiterLlmError::HookRejected { .. }),
            "guardrail block should surface as HookRejected"
        );
        assert_eq!(call_count.load(Ordering::SeqCst), 0, "inner service must not be called");
    }

    #[tokio::test]
    async fn guardrail_layer_allows_non_blocked_user() {
        let mut registry = GuardrailRegistry::new();
        let list: HashSet<String> = ["banned-user"].iter().map(|s| s.to_string()).collect();
        registry.register(Arc::new(DenyListGuardrail::new("ban", list, "user_id")));

        let inner = LlmService::new(MockClient::ok());
        let mut meta = HashMap::new();
        meta.insert("user_id".to_string(), "good-user".to_string());

        let mut svc = GuardrailLayer::new(Arc::new(registry), meta).layer(inner);
        let result = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(result.is_ok(), "non-blocked user should pass through");
    }

    /// A trivial inner service that always returns a preset response,
    /// regardless of the request. Used to exercise every `LlmResponse`
    /// variant through the guardrail layer without depending on
    /// `MockClient`'s per-endpoint coverage (it doesn't implement every
    /// endpoint — e.g. `search`/`ocr` always return `EndpointNotSupported`).
    #[derive(Clone)]
    struct CannedService {
        build: Arc<dyn Fn() -> LlmResponse + Send + Sync>,
    }

    impl Service<LlmRequest> for CannedService {
        type Response = LlmResponse;
        type Error = LiterLlmError;
        type Future = BoxFuture<'static, Result<LlmResponse>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: LlmRequest) -> Self::Future {
            let resp = (self.build)();
            Box::pin(async move { Ok(resp) })
        }
    }

    /// A guardrail that unconditionally blocks whenever it runs at the
    /// `Output` stage. Used to prove the stage is actually invoked for a
    /// given response type — before the fix, most response variants never
    /// reached `run_stage` at all, so even an always-blocking guardrail
    /// would silently never fire for them.
    struct AlwaysBlockOutput;

    impl Guardrail for AlwaysBlockOutput {
        fn name(&self) -> &'static str {
            "always-block-output"
        }

        fn supported_stages(&self) -> &'static [GuardrailStage] {
            &[GuardrailStage::Output]
        }

        fn check<'a>(
            &'a self,
            _stage: GuardrailStage,
            _ctx: &'a GuardrailContext<'a>,
        ) -> Pin<Box<dyn std::future::Future<Output = GuardrailDecision> + Send + 'a>> {
            Box::pin(async move {
                GuardrailDecision::Block {
                    reason: "test: always blocks output".into(),
                    code: 9999,
                }
            })
        }
    }

    /// Wrap a `CannedService` that always returns a response built by
    /// `build_response` behind a `GuardrailLayer` containing only
    /// `AlwaysBlockOutput`, and assert the call is blocked — proving the
    /// Output stage actually inspected this response type instead of
    /// silently skipping it via the old `_ => return Ok(response)` catch-all.
    async fn assert_output_stage_inspects<F>(request: LlmRequest, build_response: F)
    where
        F: Fn() -> LlmResponse + Send + Sync + 'static,
    {
        let mut registry = GuardrailRegistry::new();
        registry.register(Arc::new(AlwaysBlockOutput));

        let inner = CannedService {
            build: Arc::new(build_response),
        };
        let mut svc = GuardrailLayer::with_registry(Arc::new(registry)).layer(inner);

        let err = svc
            .call(request)
            .await
            .expect_err("Output-stage guardrail should have blocked this response type");

        assert!(
            matches!(err, LiterLlmError::HookRejected { .. }),
            "expected HookRejected, got {err:?}"
        );
    }

    #[tokio::test]
    async fn guardrail_output_stage_inspects_image_generate_response() {
        assert_output_stage_inspects(LlmRequest::ImageGenerate(CreateImageRequest::default()), || {
            LlmResponse::ImageGenerate(ImagesResponse::default())
        })
        .await;
    }

    #[tokio::test]
    async fn guardrail_output_stage_inspects_speech_response() {
        assert_output_stage_inspects(LlmRequest::Speech(CreateSpeechRequest::default()), || {
            LlmResponse::Speech(bytes::Bytes::from_static(b"audio"))
        })
        .await;
    }

    #[tokio::test]
    async fn guardrail_output_stage_inspects_transcribe_response() {
        assert_output_stage_inspects(LlmRequest::Transcribe(CreateTranscriptionRequest::default()), || {
            LlmResponse::Transcribe(TranscriptionResponse::default())
        })
        .await;
    }

    #[tokio::test]
    async fn guardrail_output_stage_inspects_moderate_response() {
        assert_output_stage_inspects(LlmRequest::Moderate(ModerationRequest::default()), || {
            LlmResponse::Moderate(ModerationResponse {
                id: String::new(),
                model: String::new(),
                results: vec![],
            })
        })
        .await;
    }

    #[tokio::test]
    async fn guardrail_output_stage_inspects_rerank_response() {
        assert_output_stage_inspects(LlmRequest::Rerank(RerankRequest::default()), || {
            LlmResponse::Rerank(RerankResponse {
                id: None,
                results: vec![],
                meta: None,
            })
        })
        .await;
    }

    #[tokio::test]
    async fn guardrail_output_stage_inspects_search_response() {
        assert_output_stage_inspects(LlmRequest::Search(SearchRequest::default()), || {
            LlmResponse::Search(SearchResponse {
                results: vec![],
                model: "test-model".into(),
            })
        })
        .await;
    }

    #[tokio::test]
    async fn guardrail_output_stage_inspects_ocr_response() {
        assert_output_stage_inspects(LlmRequest::Ocr(OcrRequest::default()), || {
            LlmResponse::Ocr(OcrResponse {
                pages: vec![],
                model: "test-model".into(),
                usage: None,
            })
        })
        .await;
    }

    /// A type whose `Serialize` impl always fails, used to exercise the
    /// guardrail's fail-closed path for a response body that cannot be
    /// serialized into JSON.
    struct AlwaysFailsToSerialize;

    impl serde::Serialize for AlwaysFailsToSerialize {
        fn serialize<S: serde::Serializer>(&self, _serializer: S) -> std::result::Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("intentional failure for test"))
        }
    }

    /// Regression test for the guardrail's fail-open bug: previously, a
    /// response that failed to serialize to JSON silently returned
    /// `Ok(response)`, letting un-inspected content reach the caller
    /// unchecked. It must now fail closed (return `Err`).
    ///
    /// None of `LlmResponse`'s concrete payload types can be coaxed into a
    /// real `serde_json::to_value` failure (no non-string map keys, and
    /// non-finite floats serialize to JSON `null` rather than erroring), so
    /// this exercises the extracted `serialize_for_guardrail` primitive
    /// directly with a type whose `Serialize` impl is built to fail.
    #[test]
    fn serialize_for_guardrail_fails_closed_on_serialization_error() {
        let result = serialize_for_guardrail(&AlwaysFailsToSerialize);
        assert!(
            result.is_err(),
            "a response body that cannot be serialized must fail closed (Err), not silently pass through"
        );
    }

    // --- OutputChunk streaming guardrail tests ---------------------------

    use crate::guardrail::builtin::{OnMatch, RegexGuardrail};
    use crate::types::{ChatCompletionChunk, StreamChoice, StreamDelta};
    use futures_util::StreamExt as _;

    /// Build a chunk carrying the given `content` in choice 0's delta.
    fn content_chunk(content: &str) -> Result<ChatCompletionChunk> {
        Ok(ChatCompletionChunk {
            id: "chunk".into(),
            object: "chat.completion.chunk".into(),
            created: 0,
            model: "test-model".into(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    content: Some(content.to_owned()),
                    ..Default::default()
                },
                finish_reason: None,
            }],
            usage: None,
            system_fingerprint: None,
            service_tier: None,
        })
    }

    /// A stream that yields a fixed, owned sequence of chunk results.
    struct VecChunkStream {
        items: std::collections::VecDeque<Result<ChatCompletionChunk>>,
    }

    impl futures_core::Stream for VecChunkStream {
        type Item = Result<ChatCompletionChunk>;
        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.items.pop_front())
        }
    }

    /// A `RegexGuardrail` registered at `OutputChunk` only, blocking on the word "SECRET".
    fn blocking_output_chunk_registry() -> GuardrailRegistry {
        let mut registry = GuardrailRegistry::new();
        static STAGES: &[GuardrailStage] = &[GuardrailStage::OutputChunk];
        registry.register(Arc::new(RegexGuardrail::new(
            "block-secret",
            regex::Regex::new("SECRET").expect("valid regex"),
            OnMatch::Block {
                code: 1042,
                reason_prefix: "secret leaked".into(),
            },
            STAGES,
        )));
        registry
    }

    /// Regression test for the core bug this fix addresses: before wiring
    /// `OutputChunk` into the streaming path, a guardrail that blocks a phrase
    /// in a normal completion did nothing when the same content was streamed,
    /// because `GuardrailStage::OutputChunk` was never invoked. A chunk
    /// carrying the blocked phrase must now surface as `Err(HookRejected)`
    /// when the caller polls the `ChatStream`.
    #[tokio::test]
    async fn guardrail_output_chunk_stage_blocks_streamed_phrase() {
        let registry = blocking_output_chunk_registry();

        let inner = CannedService {
            build: Arc::new(|| {
                let stream: crate::client::BoxStream<'static, Result<ChatCompletionChunk>> = Box::pin(VecChunkStream {
                    items: std::collections::VecDeque::from([
                        content_chunk("hello "),
                        content_chunk("this is SECRET data"),
                    ]),
                });
                LlmResponse::ChatStream(stream)
            }),
        };

        let mut svc = GuardrailLayer::with_registry(Arc::new(registry)).layer(inner);
        let response = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("ChatStream response itself must not be rejected up front");

        let LlmResponse::ChatStream(mut stream) = response else {
            panic!("expected ChatStream response");
        };

        let first = stream.next().await.expect("first chunk must be yielded").expect(
            "first chunk contains no blocked phrase and must pass through \
             the OutputChunk guardrail unchanged",
        );
        assert_eq!(first.choices[0].delta.content.as_deref(), Some("hello "));

        let second = stream.next().await.expect("second chunk must be yielded");
        assert!(
            matches!(second, Err(LiterLlmError::HookRejected { .. })),
            "chunk containing the blocked phrase must surface as HookRejected, got {second:?}"
        );
    }

    /// After a chunk is blocked, no further chunks may be yielded — even if
    /// the underlying stream still has more items queued up. This proves the chosen
    /// mid-stream blocking policy (terminate) actually terminates, rather
    /// than merely erroring on the offending chunk and continuing.
    #[tokio::test]
    async fn guardrail_output_chunk_stage_terminates_stream_after_block() {
        let registry = blocking_output_chunk_registry();

        let inner = CannedService {
            build: Arc::new(|| {
                let stream: crate::client::BoxStream<'static, Result<ChatCompletionChunk>> = Box::pin(VecChunkStream {
                    items: std::collections::VecDeque::from([
                        content_chunk("this is SECRET data"),
                        content_chunk("more content after the violation"),
                    ]),
                });
                LlmResponse::ChatStream(stream)
            }),
        };

        let mut svc = GuardrailLayer::with_registry(Arc::new(registry)).layer(inner);
        let response = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("ChatStream response itself must not be rejected up front");

        let LlmResponse::ChatStream(mut stream) = response else {
            panic!("expected ChatStream response");
        };

        let first = stream
            .next()
            .await
            .expect("blocked chunk must still be yielded once, as an Err");
        assert!(matches!(first, Err(LiterLlmError::HookRejected { .. })));

        let second = stream.next().await;
        assert!(
            second.is_none(),
            "stream must terminate after a block, not yield the remaining queued chunk; got {second:?}"
        );
    }

    /// A `Mutate` decision at `OutputChunk` must redact the chunk's content
    /// in place while allowing the stream to continue, distinguishing it
    /// from a `Block` decision.
    #[tokio::test]
    async fn guardrail_output_chunk_stage_mutate_redacts_and_continues() {
        let mut registry = GuardrailRegistry::new();
        static STAGES: &[GuardrailStage] = &[GuardrailStage::OutputChunk];
        registry.register(Arc::new(RegexGuardrail::new(
            "redact-secret",
            regex::Regex::new("SECRET").expect("valid regex"),
            OnMatch::Redact {
                replacement: "[REDACTED]".into(),
            },
            STAGES,
        )));

        let inner = CannedService {
            build: Arc::new(|| {
                let stream: crate::client::BoxStream<'static, Result<ChatCompletionChunk>> = Box::pin(VecChunkStream {
                    items: std::collections::VecDeque::from([content_chunk("this is SECRET data")]),
                });
                LlmResponse::ChatStream(stream)
            }),
        };

        let mut svc = GuardrailLayer::with_registry(Arc::new(registry)).layer(inner);
        let response = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("call must succeed");

        let LlmResponse::ChatStream(mut stream) = response else {
            panic!("expected ChatStream response");
        };

        let first = stream
            .next()
            .await
            .expect("chunk must be yielded")
            .expect("mutate decision must not error");
        assert_eq!(
            first.choices[0].delta.content.as_deref(),
            Some("this is [REDACTED] data"),
            "matched text must be redacted in place"
        );

        assert!(stream.next().await.is_none(), "stream must end after the single chunk");
    }

    /// Records the request it was called with so a test can assert on what
    /// actually reached the inner service.
    #[derive(Clone)]
    struct RecordingService {
        seen: Arc<std::sync::Mutex<Option<LlmRequest>>>,
    }

    impl Service<LlmRequest> for RecordingService {
        type Response = LlmResponse;
        type Error = LiterLlmError;
        type Future = BoxFuture<'static, Result<LlmResponse>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: LlmRequest) -> Self::Future {
            *self.seen.lock().expect("lock") = Some(req);
            Box::pin(async move { Ok(LlmResponse::Chat(make_chat_response("gpt-4"))) })
        }
    }

    /// The text of the single user message on a recorded chat request.
    fn recorded_prompt(request: &LlmRequest) -> String {
        let LlmRequestKind::Chat(chat) = &request.kind else {
            panic!("expected a Chat request");
        };
        serde_json::to_string(&chat.messages).expect("messages must serialize")
    }

    /// A `Mutate` decision at the `Input` stage must rewrite the request that
    /// reaches the inner service.  It previously logged at DEBUG and forwarded
    /// the *original* request, so a redaction guardrail sent the provider
    /// exactly the content it was installed to strip.
    #[tokio::test]
    async fn guardrail_input_stage_mutate_rewrites_the_forwarded_request() {
        let mut registry = GuardrailRegistry::new();
        static STAGES: &[GuardrailStage] = &[GuardrailStage::Input];
        registry.register(Arc::new(RegexGuardrail::new(
            "redact-secret",
            regex::Regex::new("SECRET").expect("valid regex"),
            OnMatch::Redact {
                replacement: "[REDACTED]".into(),
            },
            STAGES,
        )));

        let seen = Arc::new(std::sync::Mutex::new(None));
        let inner = RecordingService {
            seen: Arc::clone(&seen),
        };

        let mut chat = chat_req("gpt-4");
        chat.messages = vec![Message::User(UserMessage {
            content: "my password is SECRET".into(),
            name: None,
        })];

        let mut svc = GuardrailLayer::with_registry(Arc::new(registry)).layer(inner);
        svc.call(LlmRequest::Chat(chat)).await.expect("call must succeed");

        let forwarded = seen
            .lock()
            .expect("lock")
            .clone()
            .expect("inner service must be called");
        let prompt = recorded_prompt(&forwarded);

        assert!(
            prompt.contains("[REDACTED]"),
            "the mutated request must reach the inner service; got {prompt}"
        );
        assert!(
            !prompt.contains("SECRET"),
            "the original unredacted content must not reach the inner service; got {prompt}"
        );
    }

    /// An `Input`-stage `Mutate` must not be able to rewrite the tenant a
    /// request is scoped to.  `LlmRequest` serializes as its payload alone, so
    /// the guardrail never sees `tenant_id` — this pins that the surrounding
    /// code carries it over rather than reading it back from the payload.
    #[tokio::test]
    async fn guardrail_input_stage_mutate_preserves_tenant_scope() {
        let mut registry = GuardrailRegistry::new();
        static STAGES: &[GuardrailStage] = &[GuardrailStage::Input];
        registry.register(Arc::new(RegexGuardrail::new(
            "redact-secret",
            regex::Regex::new("SECRET").expect("valid regex"),
            OnMatch::Redact {
                replacement: "[REDACTED]".into(),
            },
            STAGES,
        )));

        let seen = Arc::new(std::sync::Mutex::new(None));
        let inner = RecordingService {
            seen: Arc::clone(&seen),
        };

        let mut chat = chat_req("gpt-4");
        chat.messages = vec![Message::User(UserMessage {
            content: "my password is SECRET".into(),
            name: None,
        })];

        let mut svc = GuardrailLayer::with_registry(Arc::new(registry)).layer(inner);
        svc.call(
            LlmRequest::Chat(chat)
                .with_tenant_id("tenant-A")
                .with_idempotency_key("idem-1"),
        )
        .await
        .expect("call must succeed");

        let forwarded = seen
            .lock()
            .expect("lock")
            .clone()
            .expect("inner service must be called");

        assert_eq!(
            forwarded.tenant_id().map(|t| t.as_ref().to_owned()),
            Some("tenant-A".to_owned()),
            "tenant must survive an Input-stage mutation"
        );
        assert_eq!(
            forwarded.idempotency_key.as_deref(),
            Some("idem-1"),
            "idempotency key must survive an Input-stage mutation"
        );
    }

    /// A `Mutate` decision at the `Output` stage must rewrite the response the
    /// caller receives.  It previously returned the original response, so a
    /// redaction guardrail handed the caller the unredacted body.
    #[tokio::test]
    async fn guardrail_output_stage_mutate_rewrites_the_returned_response() {
        let mut registry = GuardrailRegistry::new();
        static STAGES: &[GuardrailStage] = &[GuardrailStage::Output];
        registry.register(Arc::new(RegexGuardrail::new(
            "redact-secret",
            regex::Regex::new("SECRET").expect("valid regex"),
            OnMatch::Redact {
                replacement: "[REDACTED]".into(),
            },
            STAGES,
        )));

        let inner = CannedService {
            build: Arc::new(|| {
                let mut resp = make_chat_response("gpt-4");
                resp.choices[0].message.content = Some("the answer is SECRET".into());
                LlmResponse::Chat(resp)
            }),
        };

        let mut svc = GuardrailLayer::with_registry(Arc::new(registry)).layer(inner);
        let response = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("call must succeed");

        let LlmResponse::Chat(chat) = response else {
            panic!("expected a Chat response");
        };
        let Some(AssistantContent::Text(text)) = &chat.choices[0].message.content else {
            panic!("expected text content on the returned response");
        };
        assert_eq!(
            text, "the answer is [REDACTED]",
            "the mutated response must be what the caller receives"
        );
    }

    /// A `Mutate` payload that cannot be applied must fail the call rather than
    /// fall through to the original.  Silently forwarding the unmutated request
    /// is the exact failure mode this whole path exists to prevent, so a
    /// malformed rewrite has to fail closed.
    #[tokio::test]
    async fn guardrail_input_stage_inapplicable_mutate_fails_closed() {
        struct GarbageMutate;

        impl Guardrail for GarbageMutate {
            fn name(&self) -> &'static str {
                "garbage-mutate"
            }

            fn supported_stages(&self) -> &'static [GuardrailStage] {
                static STAGES: &[GuardrailStage] = &[GuardrailStage::Input];
                STAGES
            }

            fn check<'a>(
                &'a self,
                _stage: GuardrailStage,
                _ctx: &'a GuardrailContext<'a>,
            ) -> Pin<Box<dyn Future<Output = GuardrailDecision> + Send + 'a>> {
                Box::pin(async {
                    GuardrailDecision::Mutate {
                        new_payload: serde_json::json!({ "NotAVariant": 1 }),
                    }
                })
            }
        }

        let mut registry = GuardrailRegistry::new();
        registry.register(Arc::new(GarbageMutate));

        let seen = Arc::new(std::sync::Mutex::new(None));
        let inner = RecordingService {
            seen: Arc::clone(&seen),
        };

        let mut svc = GuardrailLayer::with_registry(Arc::new(registry)).layer(inner);
        let result = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;

        assert!(result.is_err(), "an inapplicable Mutate must fail the call");
        assert!(
            seen.lock().expect("lock").is_none(),
            "the original request must not be forwarded when the mutation cannot be applied"
        );
    }

    // --- Per-call metadata: tenant_id plumbing ----------------------------

    /// A guardrail that records the full `metadata` map it was invoked with,
    /// then always allows. Used to assert on exactly what
    /// `GuardrailContext::metadata` contained for a given call, independent
    /// of any single field's block/allow semantics.
    struct RecordingMetadataGuardrail {
        seen: Arc<std::sync::Mutex<Option<HashMap<String, String>>>>,
    }

    impl Guardrail for RecordingMetadataGuardrail {
        fn name(&self) -> &'static str {
            "recording-metadata"
        }

        fn supported_stages(&self) -> &'static [GuardrailStage] {
            static STAGES: &[GuardrailStage] = &[GuardrailStage::Input];
            STAGES
        }

        fn check<'a>(
            &'a self,
            _stage: GuardrailStage,
            ctx: &'a GuardrailContext<'a>,
        ) -> Pin<Box<dyn Future<Output = GuardrailDecision> + Send + 'a>> {
            let seen = Arc::clone(&self.seen);
            let metadata = ctx.metadata.clone();
            Box::pin(async move {
                *seen.lock().expect("lock") = Some(metadata);
                GuardrailDecision::Allow
            })
        }
    }

    /// The core security-control proof: a `DenyListGuardrail` configured on
    /// `tenant_id` must actually block a request whose tenant is on the
    /// list. Before per-call metadata was wired up, `ctx.metadata` never
    /// carried `tenant_id`, so `DenyListGuardrail` always read `None` and
    /// (fail-open) allowed every request regardless of the list.
    ///
    /// Revert: replace `build_call_metadata(&self.metadata, &req)` in
    /// `GuardrailService::call` with `Arc::clone(&self.metadata)` — the
    /// tenant never reaches `metadata`, the deny-list sees no field to
    /// match, and this test fails (`result` becomes `Ok`, `call_count` becomes 1).
    #[tokio::test]
    async fn deny_list_guardrail_blocks_request_whose_tenant_is_on_the_list() {
        let mut registry = GuardrailRegistry::new();
        let list: HashSet<String> = ["evil-tenant"].iter().map(|s| s.to_string()).collect();
        registry.register(Arc::new(DenyListGuardrail::new("tenant-ban", list, "tenant_id")));

        let mock = MockClient::ok();
        let call_count = Arc::clone(&mock.call_count);
        let inner = LlmService::new(mock);

        let mut svc = GuardrailLayer::with_registry(Arc::new(registry)).layer(inner);
        let result = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")).with_tenant_id("evil-tenant"))
            .await;

        let err = result.expect_err("a tenant on the deny-list must be blocked");
        assert!(
            matches!(err, LiterLlmError::HookRejected { .. }),
            "guardrail block should surface as HookRejected, got {err:?}"
        );
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            0,
            "inner service must not be called for a denied tenant"
        );
    }

    /// Counterpart to the block test: a tenant absent from the deny-list must
    /// still be let through, and the per-call metadata the guardrail actually
    /// saw must carry the real tenant id (not merely "some map or other").
    ///
    /// Revert: same line as above — with `Arc::clone(&self.metadata)` in
    /// place of `build_call_metadata`, `recorded.get("tenant_id")` is `None`
    /// instead of `Some("good-tenant")`, failing the final assertion.
    #[tokio::test]
    async fn deny_list_guardrail_allows_tenant_absent_from_list() {
        let mut registry = GuardrailRegistry::new();
        let list: HashSet<String> = ["evil-tenant"].iter().map(|s| s.to_string()).collect();
        registry.register(Arc::new(DenyListGuardrail::new("tenant-ban", list, "tenant_id")));

        let seen = Arc::new(std::sync::Mutex::new(None));
        registry.register(Arc::new(RecordingMetadataGuardrail {
            seen: Arc::clone(&seen),
        }));

        let mock = MockClient::ok();
        let call_count = Arc::clone(&mock.call_count);
        let inner = LlmService::new(mock);

        let mut svc = GuardrailLayer::with_registry(Arc::new(registry)).layer(inner);
        let result = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")).with_tenant_id("good-tenant"))
            .await;

        assert!(
            result.is_ok(),
            "a tenant absent from the deny-list must be allowed through"
        );
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "inner service must be called exactly once for an allowed tenant"
        );

        let recorded = seen
            .lock()
            .expect("lock")
            .clone()
            .expect("recording guardrail must have run");
        assert_eq!(
            recorded.get("tenant_id").map(String::as_str),
            Some("good-tenant"),
            "the per-call tenant_id must reach GuardrailContext::metadata; got {recorded:?}"
        );
    }

    /// An `AllowListGuardrail` on `tenant_id` must permit a request whose
    /// tenant is on the list. Before the fix, the field was always absent
    /// from `metadata`, so `AllowListGuardrail` (fail-closed on an absent
    /// field) blocked every request regardless of the list.
    ///
    /// Revert: same as above — with the tenant never reaching `metadata`,
    /// `AllowListGuardrail` blocks (code 1002, field absent) instead of
    /// allowing, and `result.is_ok()` fails.
    #[tokio::test]
    async fn allow_list_guardrail_permits_listed_tenant() {
        let mut registry = GuardrailRegistry::new();
        let list: HashSet<String> = ["good-tenant"].iter().map(|s| s.to_string()).collect();
        registry.register(Arc::new(AllowListGuardrail::new("tenant-allow", list, "tenant_id")));

        let mock = MockClient::ok();
        let call_count = Arc::clone(&mock.call_count);
        let inner = LlmService::new(mock);

        let mut svc = GuardrailLayer::with_registry(Arc::new(registry)).layer(inner);
        let result = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")).with_tenant_id("good-tenant"))
            .await;

        assert!(
            result.is_ok(),
            "a tenant on the allow-list must be permitted, got {result:?}"
        );
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "inner service must be called once"
        );
    }

    /// An `AllowListGuardrail` on `tenant_id` must block a request whose
    /// tenant is *not* on the list — and it must block it for the right
    /// reason (value rejected, code 1001), not merely because the field was
    /// absent (code 1002), which is what happened for every tenant before
    /// per-call metadata was wired up.
    ///
    /// Revert: same line as above. With the tenant never reaching `metadata`,
    /// the block still happens (fail-closed either way) but with code 1002
    /// and the message "required field 'tenant_id' is absent from metadata"
    /// instead of code 1001 / "is not permitted" — the `contains("code=1001")`
    /// assertion fails.
    #[tokio::test]
    async fn allow_list_guardrail_blocks_unlisted_tenant() {
        let mut registry = GuardrailRegistry::new();
        let list: HashSet<String> = ["good-tenant"].iter().map(|s| s.to_string()).collect();
        registry.register(Arc::new(AllowListGuardrail::new("tenant-allow", list, "tenant_id")));

        let mock = MockClient::ok();
        let call_count = Arc::clone(&mock.call_count);
        let inner = LlmService::new(mock);

        let mut svc = GuardrailLayer::with_registry(Arc::new(registry)).layer(inner);
        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")).with_tenant_id("bad-tenant"))
            .await
            .expect_err("a tenant absent from the allow-list must be blocked");

        let LiterLlmError::HookRejected { message } = err else {
            panic!("expected HookRejected, got {err:?}");
        };
        assert!(
            message.contains("code=1001") && message.contains("is not permitted"),
            "block must be an evaluated value rejection, not a missing-field fail-closed; got {message}"
        );
        assert_eq!(call_count.load(Ordering::SeqCst), 0, "inner service must not be called");
    }

    /// Static per-layer metadata (an existing, pre-dating feature) must still
    /// reach every guardrail alongside the newly-populated per-call
    /// `tenant_id`, side by side, both with their exact values.
    ///
    /// Revert: same line as the block/allow tests above — `recorded` would be
    /// missing the `tenant_id` entry entirely, failing that assertion (the
    /// `route` assertion alone would still pass, which is why both are
    /// checked).
    #[tokio::test]
    async fn static_layer_metadata_reaches_guardrail_alongside_per_call_tenant_id() {
        let mut registry = GuardrailRegistry::new();
        let seen = Arc::new(std::sync::Mutex::new(None));
        registry.register(Arc::new(RecordingMetadataGuardrail {
            seen: Arc::clone(&seen),
        }));

        let mut static_meta = HashMap::new();
        static_meta.insert("route".to_string(), "prod-us-east".to_string());

        let inner = LlmService::new(MockClient::ok());
        let mut svc = GuardrailLayer::new(Arc::new(registry), static_meta).layer(inner);
        svc.call(LlmRequest::Chat(chat_req("gpt-4")).with_tenant_id("tenant-A"))
            .await
            .expect("call must succeed");

        let recorded = seen
            .lock()
            .expect("lock")
            .clone()
            .expect("recording guardrail must have run");
        assert_eq!(
            recorded.get("route").map(String::as_str),
            Some("prod-us-east"),
            "static per-layer metadata must survive the merge; got {recorded:?}"
        );
        assert_eq!(
            recorded.get("tenant_id").map(String::as_str),
            Some("tenant-A"),
            "per-call tenant_id must be merged in alongside static metadata; got {recorded:?}"
        );
    }

    /// On a key collision between static per-layer metadata and a per-call
    /// fact, the static (operator-configured) value must win — silently
    /// dropping an operator's explicit configuration in favour of automatic
    /// per-call plumbing would be a new instance of the exact bug class this
    /// fix closes.
    ///
    /// Revert: remove the `layer_metadata.contains_key(TENANT_ID_METADATA_KEY)`
    /// guard in `build_call_metadata` (i.e. always insert the per-call value).
    /// `recorded.get("tenant_id")` becomes `Some("request-tenant")` instead of
    /// `Some("static-tenant")`, failing the assertion.
    #[tokio::test]
    async fn static_metadata_wins_on_key_collision_with_per_call_tenant_id() {
        let mut registry = GuardrailRegistry::new();
        let seen = Arc::new(std::sync::Mutex::new(None));
        registry.register(Arc::new(RecordingMetadataGuardrail {
            seen: Arc::clone(&seen),
        }));

        let mut static_meta = HashMap::new();
        static_meta.insert("tenant_id".to_string(), "static-tenant".to_string());

        let inner = LlmService::new(MockClient::ok());
        let mut svc = GuardrailLayer::new(Arc::new(registry), static_meta).layer(inner);
        svc.call(LlmRequest::Chat(chat_req("gpt-4")).with_tenant_id("request-tenant"))
            .await
            .expect("call must succeed");

        let recorded = seen
            .lock()
            .expect("lock")
            .clone()
            .expect("recording guardrail must have run");
        assert_eq!(
            recorded.get("tenant_id").map(String::as_str),
            Some("static-tenant"),
            "the static per-layer value must win on collision, not the per-call value; got {recorded:?}"
        );
    }
}
