//! Tower middleware that records estimated cost as a tracing span attribute.
//!
//! [`CostTrackingLayer`] wraps any [`Service<LlmRequest>`] and, after each
//! successful response, calculates the USD cost from the embedded pricing
//! registry and records it as `gen_ai.usage.cost` on the current tracing span.
//!
//! The layer is a no-op (zero overhead) for models not present in the pricing
//! registry — the span attribute is simply not recorded.
//!
//! # Example
//!
//! ```rust,ignore
//! use liter_llm::tower::{CostTrackingLayer, LlmService, TracingLayer};
//! use tower::ServiceBuilder;
//!
//! let client = liter_llm::DefaultClient::new(config, None)?;
//! let service = ServiceBuilder::new()
//!     .layer(TracingLayer)
//!     .layer(CostTrackingLayer)
//!     .service(LlmService::new(client));
//! ```

use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use futures_core::Stream;
use tower::Layer;
use tower::Service;

use super::types::{LlmRequest, LlmResponse};
use crate::client::{BoxFuture, BoxStream};
use crate::cost;
use crate::error::{LiterLlmError, Result};
use crate::types::{ChatCompletionChunk, Usage};

/// Wrap a `ChatStream` response so that `on_complete` runs once, with the last
/// [`Usage`] value observed across all chunks (`None` if the stream never
/// carried usage), when the stream is exhausted.
///
/// # Why this closes the streaming-accounting gap
///
/// [`LlmResponse::usage`][super::types::LlmResponse::usage] always returns
/// `None` for the `ChatStream` variant — usage is only known once the stream
/// has been fully consumed (typically arriving on the final chunk, when the
/// caller set `stream_options.include_usage`). Accounting layers that only
/// inspect `resp.usage()` immediately after the inner service returns
/// (budget enforcement, cost tracking, rate limiting) therefore silently
/// skip every streamed request.
///
/// [`super::service::LlmService`] fully buffers `ChatStream` responses before
/// returning them (see its module docs), so wrapping the already-buffered
/// stream here adds no additional buffering — only a `poll_next` hook that
/// watches for a `usage` field and fires `on_complete` when the stream ends.
///
/// `on_complete` runs synchronously inside `poll_next`; callers that need to
/// perform async work (e.g. writing to a remote [`crate::tower::budget::BudgetLedger`])
/// must spawn their own task rather than block here.
///
/// # Caller contract
///
/// The returned stream drains itself in `Drop` (with a no-op waker) so that an
/// abandoned stream still reports the spend it already incurred. That is only
/// safe because every current caller wraps [`super::service::LlmService`]'s
/// fully-buffered stream, whose `poll_next` is a synchronous read off a
/// `VecDeque` and touches no I/O driver.
///
/// Do not compose this over a live provider-backed stream. Polling real HTTP
/// body I/O from `Drop` can panic when no reactor is running (an aborted task,
/// a drop during runtime shutdown), and a panic raised while unwinding aborts
/// the process instead of propagating. `pub(crate)` visibility is what keeps
/// this contract enforceable — it is a convention, not a type-level guarantee,
/// so preserve it. ~keep
pub(crate) fn observe_stream_usage<F>(
    stream: BoxStream<'static, Result<ChatCompletionChunk>>,
    on_complete: F,
) -> BoxStream<'static, Result<ChatCompletionChunk>>
where
    F: FnOnce(Option<Usage>) + Send + 'static,
{
    Box::pin(UsageObservingStream {
        inner: stream,
        last_usage: None,
        on_complete: Some(Box::new(on_complete)),
    })
}

/// `Stream` adapter backing [`observe_stream_usage`].
///
/// All fields are `Unpin` (`BoxStream` is `Pin<Box<..>>`, which is always
/// `Unpin`; `Option<Usage>` and `Option<Box<dyn FnOnce..>>` are both `Unpin`),
/// so `poll_next` can use `self.get_mut()` without pin-projection machinery.
struct UsageObservingStream {
    inner: BoxStream<'static, Result<ChatCompletionChunk>>,
    last_usage: Option<Usage>,
    on_complete: Option<Box<dyn FnOnce(Option<Usage>) + Send>>,
}

impl Stream for UsageObservingStream {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                if chunk.usage.is_some() {
                    this.last_usage.clone_from(&chunk.usage);
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => {
                if let Some(cb) = this.on_complete.take() {
                    cb(this.last_usage.take());
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Upper bound on chunks drained in [`UsageObservingStream::drop`].
///
/// Only a stream that is both infinite and always-ready could reach this; the
/// bound exists so `Drop` cannot spin forever. ~keep
const DROP_DRAIN_CHUNK_LIMIT: usize = 100_000;

impl Drop for UsageObservingStream {
    fn drop(&mut self) {
        let Some(on_complete) = self.on_complete.take() else {
            // ~keep The stream ran to completion and poll_next already settled.
            return;
        };

        // ~keep The caller abandoned the stream, so poll_next never reached its
        // ~keep terminal arm and this request's spend would go unrecorded entirely —
        // ~keep letting a client consume provider tokens for free by repeatedly
        // ~keep starting streams and dropping them.
        //
        // ~keep Usage normally arrives only in the final chunk, but LlmService buffers
        // ~keep the provider stream before any middleware sees it, so that chunk is
        // ~keep already in memory here and can be recovered without awaiting: drain
        // ~keep whatever is immediately ready. A genuinely lazy stream returns Pending
        // ~keep on the first poll and is left untouched, so this never blocks.
        let mut context = Context::from_waker(Waker::noop());
        for _ in 0..DROP_DRAIN_CHUNK_LIMIT {
            match self.inner.as_mut().poll_next(&mut context) {
                Poll::Ready(Some(Ok(chunk))) => {
                    if chunk.usage.is_some() {
                        self.last_usage = chunk.usage;
                    }
                }
                Poll::Ready(Some(Err(_)) | None) | Poll::Pending => break,
            }
        }

        if self.last_usage.is_none() {
            tracing::warn!("streamed usage unavailable on abandoned stream; spend not recorded");
        }

        on_complete(self.last_usage.take());
    }
}

/// Tower [`Layer`] that records estimated USD cost on the current tracing span.
///
/// After each successful response the layer calls [`cost::completion_cost`] and
/// records the result as `gen_ai.usage.cost` using
/// [`tracing::Span::record`].  If the model is not in the pricing registry the
/// attribute is simply omitted.
#[cfg_attr(alef, alef(skip))]
pub struct CostTrackingLayer;

impl<S> Layer<S> for CostTrackingLayer {
    type Service = CostTrackingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CostTrackingService { inner }
    }
}

/// Tower service produced by [`CostTrackingLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct CostTrackingService<S> {
    inner: S,
}

impl<S> Clone for CostTrackingService<S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<S> Service<LlmRequest> for CostTrackingService<S>
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        let model = req.model().map(ToOwned::to_owned);
        let operation = req.operation_name();
        let fut = self.inner.call(req);

        Box::pin(async move {
            let resp = fut.await?;
            // ~keep Capture the span while still inside the caller's instrumented scope (e.g. TracingLayer's
            // ~keep `gen_ai` span); the ChatStream branch below records onto this handle after that scope exits.
            let span = tracing::Span::current();

            match resp {
                LlmResponse::ChatStream(stream) => {
                    let model_for_completion = model.clone();
                    let wrapped = observe_stream_usage(stream, move |usage| {
                        record_cost_for_usage(model_for_completion.as_deref(), usage.as_ref(), operation, &span);
                    });
                    Ok(LlmResponse::ChatStream(wrapped))
                }
                other => {
                    record_cost(&model, &other, operation, &span);
                    Ok(other)
                }
            }
        })
    }
}

/// Extract usage from the response and record an estimated cost on `span` as
/// `gen_ai.usage.cost`.
fn record_cost(model: &Option<String>, resp: &LlmResponse, operation: &str, span: &tracing::Span) {
    record_cost_for_usage(model.as_deref(), resp.usage(), operation, span);
}

/// Compute and record an estimated cost from an already-extracted `(model,
/// usage)` pair. Shared by the non-streaming path (`record_cost`, usage
/// available immediately) and the `ChatStream` completion callback (usage
/// only known once the stream has been fully consumed).
// ~keep `operation` is only read by the otel metrics call below, so it is genuinely unused
// ~keep when that feature is off; the allow is scoped to that case rather than renaming the
// ~keep parameter, which would obscure its meaning in the enabled build.
#[cfg_attr(not(feature = "otel"), allow(unused_variables))]
fn record_cost_for_usage(model: Option<&str>, usage: Option<&Usage>, operation: &str, span: &tracing::Span) {
    let Some(model_name) = model else { return };
    let Some(usage) = usage else { return };

    let cached = usage.prompt_tokens_details.as_ref().map_or(0, |d| d.cached_tokens);
    if let Some(usd) =
        cost::completion_cost_with_cache(model_name, usage.prompt_tokens, cached, usage.completion_tokens)
    {
        span.record("gen_ai.usage.cost", usd);

        #[cfg(feature = "otel")]
        {
            // ~keep `model` here may carry a "provider/model" prefix (e.g. "openai/gpt-4"),
            // ~keep matching the convention MetricsService uses for the `gen_ai.system` label.
            let system = model_name.split_once('/').map_or("", |(prefix, _)| prefix);
            crate::tower::metrics::record_cost_usd(system, model_name, operation, usd);
        }
    }
}

#[cfg(test)]
mod tests {
    use tower::Layer as _;
    use tower::Service as _;

    use crate::tower::service::LlmService;
    use crate::tower::types::{LlmRequest, LlmResponse};
    use crate::types::audio::{CreateSpeechRequest, CreateTranscriptionRequest, TranscriptionResponse};
    use crate::types::image::{CreateImageRequest, ImagesResponse};
    use crate::types::moderation::{ModerationRequest, ModerationResponse};
    use crate::types::ocr::{OcrRequest, OcrResponse};
    use crate::types::rerank::{RerankRequest, RerankResponse};
    use crate::types::search::{SearchRequest, SearchResponse};
    use crate::types::{
        AssistantMessage, ChatCompletionRequest, ChatCompletionResponse, Choice, EmbeddingObject, EmbeddingRequest,
        EmbeddingResponse, FinishReason, Message, ModelsListResponse, SystemMessage, Usage,
    };
    use crate::{
        client::{BoxFuture, BoxStream, LlmClient},
        error::{LiterLlmError, Result},
        types::ChatCompletionChunk,
    };

    use std::pin::Pin;
    use std::task::{Context, Poll};

    use futures_core::Stream;

    use super::CostTrackingLayer;

    struct EmptyStream;

    impl Stream for EmptyStream {
        type Item = Result<ChatCompletionChunk>;
        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(None)
        }
    }

    #[derive(Clone)]
    struct PricedMockClient {
        #[allow(dead_code)]
        model: String,
    }

    impl LlmClient for PricedMockClient {
        fn chat(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>> {
            let model = req.model.clone();
            let resp = ChatCompletionResponse {
                id: "test".into(),
                object: "chat.completion".into(),
                created: 0,
                model,
                choices: vec![Choice {
                    index: 0,
                    message: AssistantMessage {
                        content: Some("hi".into()),
                        name: None,
                        tool_calls: None,
                        refusal: None,
                        function_call: None,
                        reasoning_content: None,
                    },
                    finish_reason: Some(FinishReason::Stop),
                    logprobs: None,
                }],
                usage: Some(Usage {
                    prompt_tokens: 100,
                    completion_tokens: 50,
                    total_tokens: 150,
                    prompt_tokens_details: None,
                    completion_tokens_details: None,
                }),
                system_fingerprint: None,
                service_tier: None,
            };
            Box::pin(async move { Ok(resp) })
        }

        fn chat_stream(
            &self,
            _req: ChatCompletionRequest,
        ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>> {
            Box::pin(async move {
                let stream: BoxStream<'static, Result<ChatCompletionChunk>> = Box::pin(EmptyStream);
                Ok(stream)
            })
        }

        fn embed(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>> {
            let model = req.model.clone();
            let resp = EmbeddingResponse {
                object: "list".into(),
                data: vec![EmbeddingObject {
                    object: "embedding".into(),
                    embedding: vec![0.1],
                    index: 0,
                }],
                model,
                usage: Some(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 0,
                    total_tokens: 10,
                    prompt_tokens_details: None,
                    completion_tokens_details: None,
                }),
            };
            Box::pin(async move { Ok(resp) })
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
            Box::pin(async {
                Err(LiterLlmError::EndpointNotSupported {
                    endpoint: "search".into(),
                    provider: "mock".into(),
                })
            })
        }

        fn ocr(&self, _req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>> {
            Box::pin(async {
                Err(LiterLlmError::EndpointNotSupported {
                    endpoint: "ocr".into(),
                    provider: "mock".into(),
                })
            })
        }
    }

    fn chat_req(model: &str) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: model.into(),
            messages: vec![Message::System(SystemMessage {
                content: "test".into(),
                name: None,
            })],
            ..Default::default()
        }
    }

    /// CostTrackingLayer passes through the response unchanged for a known model.
    #[tokio::test]
    async fn cost_tracking_passes_through_chat_response_for_known_model() {
        let inner = LlmService::new(PricedMockClient { model: "gpt-4".into() });
        let mut svc = CostTrackingLayer.layer(inner);
        let resp = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("should succeed");
        match resp {
            LlmResponse::Chat(r) => {
                assert_eq!(r.model, "gpt-4");
                let cost = r.estimated_cost().expect("gpt-4 must have pricing");
                assert!((cost - 0.006).abs() < 1e-9, "unexpected cost: {cost}");
            }
            other => panic!("expected Chat response, got {:?}", std::mem::discriminant(&other)),
        }
    }

    /// CostTrackingLayer is a no-op (does not panic) for unknown models.
    #[tokio::test]
    async fn cost_tracking_no_op_for_unknown_model() {
        let inner = LlmService::new(PricedMockClient {
            model: "unknown-model".into(),
        });
        let mut svc = CostTrackingLayer.layer(inner);
        let resp = svc
            .call(LlmRequest::Chat(chat_req("unknown-model")))
            .await
            .expect("should succeed without error");
        assert!(matches!(resp, LlmResponse::Chat(_)));
    }

    /// CostTrackingLayer propagates errors from the inner service.
    #[tokio::test]
    async fn cost_tracking_propagates_inner_errors() {
        use crate::client::{BoxFuture, BoxStream, LlmClient};
        use crate::tower::service::LlmService;

        #[derive(Clone)]
        struct AlwaysErrorClient;

        impl LlmClient for AlwaysErrorClient {
            fn chat(&self, _req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>> {
                Box::pin(async { Err(LiterLlmError::Timeout) })
            }
            fn chat_stream(
                &self,
                _req: ChatCompletionRequest,
            ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>> {
                Box::pin(async move {
                    let stream: BoxStream<'static, Result<ChatCompletionChunk>> = Box::pin(EmptyStream);
                    Ok(stream)
                })
            }
            fn embed(&self, _req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>> {
                Box::pin(async { Err(LiterLlmError::Timeout) })
            }
            fn list_models(&self) -> BoxFuture<'_, Result<ModelsListResponse>> {
                Box::pin(async { Err(LiterLlmError::Timeout) })
            }
            fn image_generate(&self, _req: CreateImageRequest) -> BoxFuture<'_, Result<ImagesResponse>> {
                Box::pin(async { Err(LiterLlmError::Timeout) })
            }
            fn speech(&self, _req: CreateSpeechRequest) -> BoxFuture<'_, Result<bytes::Bytes>> {
                Box::pin(async { Err(LiterLlmError::Timeout) })
            }
            fn transcribe(&self, _req: CreateTranscriptionRequest) -> BoxFuture<'_, Result<TranscriptionResponse>> {
                Box::pin(async { Err(LiterLlmError::Timeout) })
            }
            fn moderate(&self, _req: ModerationRequest) -> BoxFuture<'_, Result<ModerationResponse>> {
                Box::pin(async { Err(LiterLlmError::Timeout) })
            }
            fn rerank(&self, _req: RerankRequest) -> BoxFuture<'_, Result<RerankResponse>> {
                Box::pin(async { Err(LiterLlmError::Timeout) })
            }

            fn search(&self, _req: SearchRequest) -> BoxFuture<'_, Result<SearchResponse>> {
                Box::pin(async {
                    Err(LiterLlmError::EndpointNotSupported {
                        endpoint: "search".into(),
                        provider: "mock".into(),
                    })
                })
            }

            fn ocr(&self, _req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>> {
                Box::pin(async {
                    Err(LiterLlmError::EndpointNotSupported {
                        endpoint: "ocr".into(),
                        provider: "mock".into(),
                    })
                })
            }
        }

        let inner = LlmService::new(AlwaysErrorClient);
        let mut svc = CostTrackingLayer.layer(inner);
        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should propagate inner error");
        assert!(matches!(err, LiterLlmError::Timeout));
    }

    /// A stream that yields a fixed sequence of items then ends.
    struct VecStream<T> {
        items: std::collections::VecDeque<T>,
    }

    impl<T: Unpin> Stream for VecStream<T> {
        type Item = T;
        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.items.pop_front())
        }
    }

    fn chunk(usage: Option<Usage>) -> Result<ChatCompletionChunk> {
        Ok(ChatCompletionChunk {
            id: "chunk".into(),
            object: "chat.completion.chunk".into(),
            created: 0,
            model: "gpt-4".into(),
            choices: vec![],
            usage,
            system_fingerprint: None,
            service_tier: None,
        })
    }

    /// Regression for the "streaming bypasses accounting" bug: `LlmResponse::usage()`
    /// always returns `None` for `ChatStream`, so any layer that only inspects the
    /// immediate response silently skips every streamed request. `observe_stream_usage`
    /// must still surface the usage carried on the final chunk to its completion callback.
    #[tokio::test]
    async fn observe_stream_usage_reports_usage_from_final_chunk() {
        use futures_util::StreamExt as _;

        let usage = Usage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        };

        let inner: BoxStream<'static, Result<ChatCompletionChunk>> = Box::pin(VecStream {
            items: std::collections::VecDeque::from([chunk(None), chunk(None), chunk(Some(usage.clone()))]),
        });

        let observed = std::sync::Arc::new(std::sync::Mutex::new(None));
        let observed_for_cb = std::sync::Arc::clone(&observed);
        let mut wrapped = super::observe_stream_usage(inner, move |u| {
            *observed_for_cb.lock().expect("test mutex must not be poisoned") = Some(u);
        });

        let mut yielded = 0;
        while wrapped.next().await.is_some() {
            yielded += 1;
        }

        assert_eq!(yielded, 3, "all chunks must still be yielded to the caller");
        let recorded = observed.lock().expect("test mutex must not be poisoned").clone();
        assert_eq!(
            recorded,
            Some(Some(usage)),
            "on_complete must fire exactly once with the usage from the final chunk"
        );
    }

    /// When no chunk carries usage, `on_complete` must still fire (exactly once)
    /// with `None`, so callers can distinguish "stream observed, no usage reported"
    /// from "callback never ran".
    #[tokio::test]
    async fn observe_stream_usage_reports_none_when_no_chunk_has_usage() {
        use futures_util::StreamExt as _;

        let inner: BoxStream<'static, Result<ChatCompletionChunk>> = Box::pin(VecStream {
            items: std::collections::VecDeque::from([chunk(None), chunk(None)]),
        });

        let observed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let observed_for_cb = std::sync::Arc::clone(&observed);
        let mut wrapped = super::observe_stream_usage(inner, move |u| {
            observed_for_cb.lock().expect("test mutex must not be poisoned").push(u);
        });

        while wrapped.next().await.is_some() {}

        let recorded = observed.lock().expect("test mutex must not be poisoned").clone();
        assert_eq!(recorded, vec![None], "on_complete must fire exactly once, with None");
    }
}
