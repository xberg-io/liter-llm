//! Integration tests verifying that `UsageEvent.cache_state` and
//! `UsageEvent.effective_model` are correctly populated by the Tower stack.

#![cfg(feature = "tower")]

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use liter_llm::client::{BoxFuture, BoxStream, LlmClient};
use liter_llm::error::{LiterLlmError, Result};
use liter_llm::observability::{CacheState, UsageEvent, UsageSink, UsageSinkError};
use liter_llm::tower::cache_policy::StandardCachePolicy;
use liter_llm::tower::types::LlmRequest;
use liter_llm::tower::{
    CacheBackend, CacheConfig, CacheLayer, EmbeddingProvider, HooksLayer, InMemoryVectorStore, LlmService,
    NoOpEmbeddingProvider, VectorMetadata, VectorStore,
};
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
use tower::Layer as _;
use tower::Service as _;

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
    async fn emit(&self, event: UsageEvent) -> std::result::Result<(), UsageSinkError> {
        self.events.lock().expect("lock poisoned").push(event);
        Ok(())
    }
}

/// Client that echoes the requested model name in the response.
#[derive(Clone)]
struct EchoClient;

fn make_chat_response(model: &str) -> ChatCompletionResponse {
    ChatCompletionResponse {
        id: "test".into(),
        object: "chat.completion".into(),
        created: 0,
        model: model.into(),
        choices: vec![Choice {
            index: 0,
            message: AssistantMessage {
                content: Some("hi".into()),
                name: None,
                tool_calls: None,
                refusal: None,
                function_call: None,
            },
            finish_reason: Some(FinishReason::Stop),
        }],
        usage: Some(Usage {
            prompt_tokens: 5,
            completion_tokens: 3,
            total_tokens: 8,
            prompt_tokens_details: None,
        }),
        system_fingerprint: None,
        service_tier: None,
    }
}

impl LlmClient for EchoClient {
    fn chat(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>> {
        let resp = make_chat_response(&req.model);
        Box::pin(async move { Ok(resp) })
    }

    fn chat_stream(
        &self,
        _req: ChatCompletionRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>> {
        Box::pin(async move { Ok(Box::pin(futures_util::stream::empty()) as BoxStream<'_, _>) })
    }

    fn embed(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>> {
        let model = req.model.clone();
        Box::pin(async move {
            Ok(EmbeddingResponse {
                object: "list".into(),
                data: vec![EmbeddingObject {
                    object: "embedding".into(),
                    embedding: vec![0.1],
                    index: 0,
                }],
                model,
                usage: None,
            })
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
        Box::pin(async {
            Err(LiterLlmError::EndpointNotSupported {
                endpoint: "search".into(),
                provider: "echo".into(),
            })
        })
    }

    fn ocr(&self, _req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>> {
        Box::pin(async {
            Err(LiterLlmError::EndpointNotSupported {
                endpoint: "ocr".into(),
                provider: "echo".into(),
            })
        })
    }
}

/// Client that echoes back a *different* model name than requested — simulates
/// provider resolution (e.g. `"gpt-4o"` → `"gpt-4o-2024-08-06"`).
#[derive(Clone)]
struct ResolvedModelClient {
    resolved_model: String,
}

impl LlmClient for ResolvedModelClient {
    fn chat(&self, _req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>> {
        let resp = make_chat_response(&self.resolved_model);
        Box::pin(async move { Ok(resp) })
    }

    fn chat_stream(
        &self,
        _req: ChatCompletionRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>> {
        Box::pin(async move { Ok(Box::pin(futures_util::stream::empty()) as BoxStream<'_, _>) })
    }

    fn embed(&self, _req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>> {
        Box::pin(async {
            Err(LiterLlmError::EndpointNotSupported {
                endpoint: "embed".into(),
                provider: "resolved".into(),
            })
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
        Box::pin(async {
            Err(LiterLlmError::EndpointNotSupported {
                endpoint: "search".into(),
                provider: "resolved".into(),
            })
        })
    }

    fn ocr(&self, _req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>> {
        Box::pin(async {
            Err(LiterLlmError::EndpointNotSupported {
                endpoint: "ocr".into(),
                provider: "resolved".into(),
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

/// Yield to the Tokio executor so detached `tokio::spawn` sink tasks can land.
async fn flush_sink() {
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;
}

/// First request with a CacheLayer in the stack produces `cache_state == Miss`.
#[tokio::test]
async fn miss_records_miss() {
    let sink = Arc::new(VecSink::default());
    let cache_layer = CacheLayer::new(CacheConfig {
        max_entries: 10,
        ttl: Duration::from_secs(60),
        backend: CacheBackend::default(),
    });
    let inner = LlmService::new(EchoClient);
    let mut svc = HooksLayer::new(vec![])
        .with_usage_sink(Arc::clone(&sink))
        .layer(cache_layer.layer(inner));

    svc.call(LlmRequest::Chat(chat_req("gpt-4o")))
        .await
        .expect("should succeed");
    flush_sink().await;

    let events = sink.collected();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].cache_state,
        CacheState::Miss,
        "first call with empty cache must be Miss"
    );
}

/// Second identical request with a primed CacheLayer produces `cache_state == ExactHit`.
#[tokio::test]
async fn exact_hit_records_exact_hit() {
    let sink = Arc::new(VecSink::default());
    let cache_layer = CacheLayer::new(CacheConfig {
        max_entries: 10,
        ttl: Duration::from_secs(60),
        backend: CacheBackend::default(),
    });
    let inner = LlmService::new(EchoClient);
    let mut svc = HooksLayer::new(vec![])
        .with_usage_sink(Arc::clone(&sink))
        .layer(cache_layer.layer(inner));

    svc.call(LlmRequest::Chat(chat_req("gpt-4o")))
        .await
        .expect("should succeed");
    flush_sink().await;

    svc.call(LlmRequest::Chat(chat_req("gpt-4o")))
        .await
        .expect("should succeed");
    flush_sink().await;

    let events = sink.collected();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].cache_state, CacheState::Miss, "first call must be Miss");
    assert_eq!(
        events[1].cache_state,
        CacheState::ExactHit,
        "second identical call must be ExactHit"
    );
}

/// Semantic-similarity hit via the vector store produces `cache_state == SemanticHit`.
#[tokio::test]
async fn semantic_hit_records_semantic_hit() {
    use liter_llm::tower::cache::InMemoryStore;

    let sink = Arc::new(VecSink::default());

    let config = CacheConfig {
        max_entries: 10,
        ttl: Duration::from_secs(60),
        backend: CacheBackend::default(),
    };
    let store: Arc<dyn liter_llm::tower::CacheStore> = Arc::new(InMemoryStore::new(&config));

    use liter_llm::tower::CachedResponse;
    let sentinel_key: u64 = 7777;
    let sentinel_body = "sentinel-body";
    store
        .put(
            sentinel_key,
            sentinel_body.into(),
            CachedResponse::Chat(make_chat_response("gpt-4")),
        )
        .await;

    let vs: Arc<dyn VectorStore> = Arc::new(InMemoryVectorStore::new(1));
    vs.upsert(
        "v".into(),
        vec![0.0],
        VectorMetadata {
            cache_key: sentinel_key,
            original_request_body: sentinel_body.into(),
            tenant_id: None,
            inserted_at: SystemTime::now(),
            extra: HashMap::new(),
        },
    )
    .await
    .expect("vector upsert must succeed");

    let ep: Arc<dyn EmbeddingProvider> = Arc::new(NoOpEmbeddingProvider { dim: 1 });

    let policy = Arc::new(StandardCachePolicy {
        semantic_ttl: Some(Duration::from_secs(60)),
        similarity_threshold: 0.0,
        ..Default::default()
    });

    let cache_layer = CacheLayer::with_store(Arc::clone(&store))
        .with_policy(policy)
        .with_semantic_cache(ep, vs);

    let inner = LlmService::new(EchoClient);
    let mut svc = HooksLayer::new(vec![])
        .with_usage_sink(Arc::clone(&sink))
        .layer(cache_layer.layer(inner));

    svc.call(LlmRequest::Chat(chat_req("gpt-4o")))
        .await
        .expect("should succeed");
    flush_sink().await;

    let events = sink.collected();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].cache_state,
        CacheState::SemanticHit,
        "semantic vector match must produce SemanticHit"
    );
}

/// Without a `CacheLayer` in the stack, `cache_state` stays `Bypass`.
#[tokio::test]
async fn bypass_when_no_cache_layer() {
    let sink = Arc::new(VecSink::default());
    let inner = LlmService::new(EchoClient);
    let mut svc = HooksLayer::new(vec![]).with_usage_sink(Arc::clone(&sink)).layer(inner);

    svc.call(LlmRequest::Chat(chat_req("gpt-4o")))
        .await
        .expect("should succeed");
    flush_sink().await;

    let events = sink.collected();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].cache_state,
        CacheState::Bypass,
        "no CacheLayer → cache_state must be Bypass"
    );
}

/// When the provider echoes a resolved model name, `effective_model` reflects it.
#[tokio::test]
async fn effective_model_populated_from_response() {
    let sink = Arc::new(VecSink::default());
    let inner = LlmService::new(ResolvedModelClient {
        resolved_model: "gpt-4o-2024-08-06".into(),
    });
    let mut svc = HooksLayer::new(vec![]).with_usage_sink(Arc::clone(&sink)).layer(inner);

    svc.call(LlmRequest::Chat(chat_req("gpt-4o")))
        .await
        .expect("should succeed");
    flush_sink().await;

    let events = sink.collected();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].model, "gpt-4o",
        "requested model name must be preserved in `model`"
    );
    assert_eq!(
        events[0].effective_model.as_deref(),
        Some("gpt-4o-2024-08-06"),
        "provider-echoed model must appear in `effective_model`"
    );
}

/// Speech responses return raw bytes with no model field — `effective_model` is `None`.
#[tokio::test]
async fn effective_model_none_for_speech_variant() {
    let sink = Arc::new(VecSink::default());
    let inner = LlmService::new(EchoClient);
    let mut svc = HooksLayer::new(vec![]).with_usage_sink(Arc::clone(&sink)).layer(inner);

    svc.call(LlmRequest::Speech(CreateSpeechRequest {
        model: "tts-1".into(),
        input: "hello".into(),
        voice: "alloy".into(),
        response_format: None,
        speed: None,
    }))
    .await
    .expect("should succeed");
    flush_sink().await;

    let events = sink.collected();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].effective_model, None,
        "Speech variant carries no model echo — effective_model must be None"
    );
}
