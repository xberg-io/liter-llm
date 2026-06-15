//! Integration tests for `UsageEvent` / `UsageSink` plumbing.
//!
//! Covers:
//! - `VecSink` collecting events
//! - `HooksLayer::with_usage_sink` emitting on success
//! - `HooksLayer::with_usage_sink` emitting on error (timeout)
//! - `MultiUsageSink` fan-out to multiple sinks
//! - `LoggingUsageSink` never returning an error

use std::sync::{Arc, Mutex};

use liter_llm::observability::{
    CacheState, LoggingUsageSink, MultiUsageSink, UsageEvent, UsageEventOutcome, UsageSink, UsageSinkError,
};
use liter_llm::tenant::TenantId;
use liter_llm::tower::types::LlmRequest;
use liter_llm::tower::{HooksLayer, LlmService};

// ─── VecSink ─────────────────────────────────────────────────────────────────

/// In-process sink that collects events for test assertions.
#[derive(Default)]
struct VecSink {
    events: Arc<Mutex<Vec<UsageEvent>>>,
}

impl VecSink {
    fn collected(&self) -> Vec<UsageEvent> {
        self.events.lock().expect("lock poisoned").clone()
    }
}

impl UsageSink for VecSink {
    async fn emit(&self, event: UsageEvent) -> Result<(), UsageSinkError> {
        self.events.lock().expect("lock poisoned").push(event);
        Ok(())
    }
}

// ─── Stub inner service helpers ───────────────────────────────────────────────

mod helpers {
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::{Context, Poll};

    use futures_core::Stream;

    use liter_llm::client::{BoxFuture, BoxStream, LlmClient};
    use liter_llm::error::{LiterLlmError, Result};
    use liter_llm::types::audio::{CreateSpeechRequest, CreateTranscriptionRequest, TranscriptionResponse};
    use liter_llm::types::image::{CreateImageRequest, ImagesResponse};
    use liter_llm::types::moderation::{ModerationRequest, ModerationResponse};
    use liter_llm::types::ocr::{OcrRequest, OcrResponse};
    use liter_llm::types::rerank::{RerankRequest, RerankResponse};
    use liter_llm::types::search::{SearchRequest, SearchResponse};
    use liter_llm::types::{
        AssistantMessage, ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, Choice, EmbeddingObject,
        EmbeddingRequest, EmbeddingResponse, FinishReason, Message, ModelsListResponse, SystemMessage, Usage,
    };

    pub struct EmptyStream;
    impl Stream for EmptyStream {
        type Item = Result<ChatCompletionChunk>;
        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(None)
        }
    }

    /// A client that always returns a successful chat response.
    #[derive(Clone)]
    pub struct OkClient {
        pub call_count: Arc<AtomicUsize>,
    }

    impl OkClient {
        pub fn new() -> Self {
            Self { call_count: Arc::new(AtomicUsize::new(0)) }
        }
    }

    fn ok_response(model: &str) -> ChatCompletionResponse {
        ChatCompletionResponse {
            id: "id-1".into(),
            object: "chat.completion".into(),
            created: 0,
            model: model.into(),
            choices: vec![Choice {
                index: 0,
                message: AssistantMessage {
                    content: Some("hello".into()),
                    name: None,
                    tool_calls: None,
                    refusal: None,
                    function_call: None,
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
        }
    }

    impl LlmClient for OkClient {
        fn chat(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let resp = ok_response(&req.model);
            Box::pin(async move { Ok(resp) })
        }
        fn chat_stream(&self, _req: ChatCompletionRequest) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>> {
            Box::pin(async move { Ok(Box::pin(EmptyStream) as BoxStream<'_, _>) })
        }
        fn embed(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>> {
            let resp = EmbeddingResponse {
                object: "list".into(),
                data: vec![EmbeddingObject { object: "embedding".into(), embedding: vec![0.1], index: 0 }],
                model: req.model.clone(),
                usage: None,
            };
            Box::pin(async move { Ok(resp) })
        }
        fn list_models(&self) -> BoxFuture<'_, Result<ModelsListResponse>> {
            Box::pin(async move { Ok(ModelsListResponse { object: "list".into(), data: vec![] }) })
        }
        fn image_generate(&self, _req: CreateImageRequest) -> BoxFuture<'_, Result<ImagesResponse>> {
            Box::pin(async move { Ok(ImagesResponse { created: 0, data: vec![] }) })
        }
        fn speech(&self, _req: CreateSpeechRequest) -> BoxFuture<'_, Result<bytes::Bytes>> {
            Box::pin(async move { Ok(bytes::Bytes::new()) })
        }
        fn transcribe(&self, _req: CreateTranscriptionRequest) -> BoxFuture<'_, Result<TranscriptionResponse>> {
            Box::pin(async move { Ok(TranscriptionResponse { text: String::new(), language: None, duration: None, segments: None }) })
        }
        fn moderate(&self, _req: ModerationRequest) -> BoxFuture<'_, Result<ModerationResponse>> {
            Box::pin(async move { Ok(ModerationResponse { id: String::new(), model: String::new(), results: vec![] }) })
        }
        fn rerank(&self, _req: RerankRequest) -> BoxFuture<'_, Result<RerankResponse>> {
            Box::pin(async move { Ok(RerankResponse { id: None, results: vec![], meta: None }) })
        }
        fn search(&self, _req: SearchRequest) -> BoxFuture<'_, Result<SearchResponse>> {
            Box::pin(async { Err(LiterLlmError::EndpointNotSupported { endpoint: "search".into(), provider: "mock".into() }) })
        }
        fn ocr(&self, _req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>> {
            Box::pin(async { Err(LiterLlmError::EndpointNotSupported { endpoint: "ocr".into(), provider: "mock".into() }) })
        }
    }

    /// A client that always returns `LiterLlmError::Timeout`.
    #[derive(Clone)]
    pub struct TimeoutClient;

    impl LlmClient for TimeoutClient {
        fn chat(&self, _req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>> {
            Box::pin(async { Err(LiterLlmError::Timeout) })
        }
        fn chat_stream(&self, _req: ChatCompletionRequest) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>> {
            Box::pin(async move { Ok(Box::pin(EmptyStream) as BoxStream<'_, _>) })
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
            Box::pin(async { Err(LiterLlmError::EndpointNotSupported { endpoint: "search".into(), provider: "mock".into() }) })
        }
        fn ocr(&self, _req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>> {
            Box::pin(async { Err(LiterLlmError::EndpointNotSupported { endpoint: "ocr".into(), provider: "mock".into() }) })
        }
    }

    pub fn chat_req(model: &str) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: model.into(),
            messages: vec![Message::System(SystemMessage { content: "hi".into(), name: None })],
            ..Default::default()
        }
    }
}

use helpers::{OkClient, TimeoutClient, chat_req};
use tower::Layer as _;
use tower::Service as _;

// ─── Tests ────────────────────────────────────────────────────────────────────

/// VecSink accumulates events correctly.
#[tokio::test]
async fn vec_sink_collects_events() {
    let sink = Arc::new(VecSink::default());

    let event = UsageEvent {
        tenant_id: Some(TenantId::from("acme-corp")),
        request_id: "req-1".into(),
        model: "gpt-4".into(),
        provider: "openai".into(),
        prompt_tokens: 10,
        completion_tokens: 5,
        cached_tokens: 0,
        total_tokens: 15,
        cost_usd: rust_decimal::Decimal::ZERO,
        cache_state: CacheState::Miss,
        finish_reason: Some("stop".into()),
        outcome: UsageEventOutcome::Success,
        latency_ms: 42,
        metadata: Default::default(),
        received_at: std::time::SystemTime::now(),
    };

    sink.emit(event).await.expect("sink must not error");

    let collected = sink.collected();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].tenant_id, Some(TenantId::from("acme-corp")));
    assert_eq!(collected[0].request_id, "req-1");
    assert_eq!(collected[0].outcome, UsageEventOutcome::Success);
}

/// HooksLayer emits a UsageEvent with outcome=Success on the happy path.
#[tokio::test]
async fn hooks_layer_emits_usage_event_on_success() {
    let sink = Arc::new(VecSink::default());
    let inner = LlmService::new(OkClient::new());
    let mut svc = HooksLayer::new(vec![]).with_usage_sink(Arc::clone(&sink)).layer(inner);

    let req = LlmRequest::Chat(chat_req("openai/gpt-4o")).with_tenant_id("t-1");
    svc.call(req).await.expect("should succeed");

    let events = sink.collected();
    assert_eq!(events.len(), 1, "exactly one event per request");

    let ev = &events[0];
    assert_eq!(ev.outcome, UsageEventOutcome::Success);
    assert_eq!(ev.tenant_id, Some(TenantId::from("t-1")));
    assert_eq!(ev.model, "openai/gpt-4o");
    assert_eq!(ev.provider, "openai");
    assert_eq!(ev.prompt_tokens, 10);
    assert_eq!(ev.completion_tokens, 5);
    assert_eq!(ev.total_tokens, 15);
    assert_eq!(ev.cache_state, CacheState::Bypass);
}

/// HooksLayer emits an event with outcome=TimedOut on the error path.
#[tokio::test]
async fn hooks_layer_emits_usage_event_on_timeout() {
    let sink = Arc::new(VecSink::default());
    let inner = LlmService::new(TimeoutClient);
    let mut svc = HooksLayer::new(vec![]).with_usage_sink(Arc::clone(&sink)).layer(inner);

    let req = LlmRequest::Chat(chat_req("openai/gpt-4o"));
    let err = svc.call(req).await.expect_err("should fail with timeout");
    assert!(matches!(err, liter_llm::LiterLlmError::Timeout));

    let events = sink.collected();
    assert_eq!(events.len(), 1, "one event even on error");
    assert_eq!(events[0].outcome, UsageEventOutcome::TimedOut);
    assert_eq!(events[0].prompt_tokens, 0);
    assert_eq!(events[0].cost_usd, rust_decimal::Decimal::ZERO);
}

/// HooksLayer uses the idempotency key as request_id when one is set.
#[tokio::test]
async fn hooks_layer_uses_idempotency_key_as_request_id() {
    let sink = Arc::new(VecSink::default());
    let inner = LlmService::new(OkClient::new());
    let mut svc = HooksLayer::new(vec![]).with_usage_sink(Arc::clone(&sink)).layer(inner);

    let req = LlmRequest::Chat(chat_req("gpt-4")).with_idempotency_key("my-idem-key");
    svc.call(req).await.expect("should succeed");

    let events = sink.collected();
    assert_eq!(events[0].request_id, "my-idem-key");
}

/// MultiUsageSink fans out to every inner sink.
#[tokio::test]
async fn multi_usage_sink_fans_out() {
    let a = Arc::new(VecSink::default());
    let b = Arc::new(VecSink::default());

    let mut multi = MultiUsageSink::from_sinks(vec![Arc::clone(&a)]);
    multi.push(Arc::clone(&b));

    let event = UsageEvent {
        tenant_id: None,
        request_id: "r".into(),
        model: "m".into(),
        provider: "p".into(),
        prompt_tokens: 0,
        completion_tokens: 0,
        cached_tokens: 0,
        total_tokens: 0,
        cost_usd: rust_decimal::Decimal::ZERO,
        cache_state: CacheState::Bypass,
        finish_reason: None,
        outcome: UsageEventOutcome::Success,
        latency_ms: 0,
        metadata: Default::default(),
        received_at: std::time::SystemTime::now(),
    };

    multi.emit(event).await.expect("multi sink must not error");

    assert_eq!(a.collected().len(), 1, "sink a received event");
    assert_eq!(b.collected().len(), 1, "sink b received event");
}

/// LoggingUsageSink never errors.
#[tokio::test]
async fn logging_sink_does_not_error() {
    let sink = LoggingUsageSink;
    let event = UsageEvent {
        tenant_id: None,
        request_id: "x".into(),
        model: "gpt-4".into(),
        provider: "openai".into(),
        prompt_tokens: 1,
        completion_tokens: 2,
        cached_tokens: 0,
        total_tokens: 3,
        cost_usd: rust_decimal::Decimal::ZERO,
        cache_state: CacheState::Miss,
        finish_reason: None,
        outcome: UsageEventOutcome::Success,
        latency_ms: 10,
        metadata: Default::default(),
        received_at: std::time::SystemTime::now(),
    };
    sink.emit(event).await.expect("logging sink must not error");
}

/// No usage event is emitted when no sink is attached.
#[tokio::test]
async fn no_sink_attached_does_not_panic() {
    let inner = LlmService::new(OkClient::new());
    let mut svc = HooksLayer::new(vec![]).layer(inner);
    svc.call(LlmRequest::Chat(chat_req("gpt-4")))
        .await
        .expect("should succeed without a sink");
}
