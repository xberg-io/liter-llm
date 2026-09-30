use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use crate::tower::service::LlmService;
use crate::tower::tests_common::{MockClient, chat_req};
use crate::tower::types::LlmRequest;

/// Helper: build a budget layer + service with the given config.
fn build_service(config: BudgetConfig, state: Arc<BudgetState>) -> BudgetService<LlmService<MockClient>> {
    let layer = BudgetLayer::new(config, state);
    let inner = LlmService::new(MockClient::ok());
    layer.layer(inner)
}

#[tokio::test]
async fn hard_enforcement_rejects_when_global_limit_exceeded() {
    let state = Arc::new(BudgetState::new());
    state.global_spend.store(usd_to_microcents(10.0), Ordering::Relaxed);

    let config = BudgetConfig {
        global_limit: Some(5.0),
        enforcement: Enforcement::Hard,
        ..Default::default()
    };

    let mut svc = build_service(config, state);
    let err = svc
        .call(LlmRequest::Chat(chat_req("gpt-4")))
        .await
        .expect_err("should reject over-budget request");
    assert!(matches!(err, LiterLlmError::BudgetExceeded { .. }));
}

#[tokio::test]
async fn hard_enforcement_rejects_when_model_limit_exceeded() {
    let state = Arc::new(BudgetState::new());
    state
        .model_spend
        .entry("gpt-4".to_owned())
        .or_insert_with(|| AtomicU64::new(0))
        .store(usd_to_microcents(2.0), Ordering::Relaxed);

    let mut limits = HashMap::new();
    limits.insert("gpt-4".into(), 1.0);

    let config = BudgetConfig {
        global_limit: None,
        model_limits: limits,
        enforcement: Enforcement::Hard,
    };

    let mut svc = build_service(config, state);
    let err = svc
        .call(LlmRequest::Chat(chat_req("gpt-4")))
        .await
        .expect_err("should reject over-budget model request");

    match &err {
        LiterLlmError::BudgetExceeded { model, .. } => {
            assert_eq!(model.as_deref(), Some("gpt-4"));
        }
        other => panic!("expected BudgetExceeded, got {other:?}"),
    }
}

#[tokio::test]
async fn hard_enforcement_allows_requests_under_limit() {
    let state = Arc::new(BudgetState::new());
    let config = BudgetConfig {
        global_limit: Some(100.0),
        enforcement: Enforcement::Hard,
        ..Default::default()
    };

    let mut svc = build_service(config, state);
    let resp = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
    assert!(resp.is_ok(), "request under budget should succeed");
}

#[tokio::test]
async fn soft_enforcement_allows_requests_over_global_limit() {
    let state = Arc::new(BudgetState::new());
    state.global_spend.store(usd_to_microcents(100.0), Ordering::Relaxed);

    let config = BudgetConfig {
        global_limit: Some(5.0),
        enforcement: Enforcement::Soft,
        ..Default::default()
    };

    let mut svc = build_service(config, state);
    let resp = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
    assert!(resp.is_ok(), "soft mode should never reject");
}

#[tokio::test]
async fn soft_enforcement_allows_requests_over_model_limit() {
    let state = Arc::new(BudgetState::new());
    state
        .model_spend
        .entry("gpt-4".to_owned())
        .or_insert_with(|| AtomicU64::new(0))
        .store(usd_to_microcents(10.0), Ordering::Relaxed);

    let mut limits = HashMap::new();
    limits.insert("gpt-4".into(), 1.0);

    let config = BudgetConfig {
        global_limit: None,
        model_limits: limits,
        enforcement: Enforcement::Soft,
    };

    let mut svc = build_service(config, state);
    let resp = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
    assert!(resp.is_ok(), "soft mode should never reject");
}

#[tokio::test]
async fn accumulates_cost_after_response() {
    let state = Arc::new(BudgetState::new());
    let config = BudgetConfig {
        global_limit: Some(100.0),
        enforcement: Enforcement::Hard,
        ..Default::default()
    };

    let mut svc = build_service(config, Arc::clone(&state));
    svc.call(LlmRequest::Chat(chat_req("gpt-4")))
        .await
        .expect("service call should not fail");

    assert!(state.global_spend() > 0.0, "global spend should be recorded");
    assert!(state.model_spend("gpt-4") > 0.0, "model spend should be recorded");
}

#[tokio::test]
async fn per_model_limits_are_independent() {
    let state = Arc::new(BudgetState::new());
    state
        .model_spend
        .entry("gpt-4".to_owned())
        .or_insert_with(|| AtomicU64::new(0))
        .store(usd_to_microcents(5.0), Ordering::Relaxed);

    let mut limits = HashMap::new();
    limits.insert("gpt-4".into(), 1.0);

    let config = BudgetConfig {
        global_limit: None,
        model_limits: limits,
        enforcement: Enforcement::Hard,
    };

    let mut svc = build_service(config, state);

    let err = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
    assert!(err.is_err(), "gpt-4 should be rejected");

    let ok = svc.call(LlmRequest::Chat(chat_req("gpt-3.5-turbo"))).await;
    assert!(ok.is_ok(), "gpt-3.5-turbo should not be limited");
}

#[tokio::test]
async fn reset_clears_all_counters() {
    let state = Arc::new(BudgetState::new());
    state.global_spend.store(usd_to_microcents(50.0), Ordering::Relaxed);
    state
        .model_spend
        .entry("gpt-4".to_owned())
        .or_insert_with(|| AtomicU64::new(0))
        .store(usd_to_microcents(25.0), Ordering::Relaxed);

    assert!(state.global_spend() > 0.0);
    assert!(state.model_spend("gpt-4") > 0.0);

    state.reset();

    assert_eq!(state.global_spend(), 0.0, "global spend should be zero after reset");
    assert_eq!(
        state.model_spend("gpt-4"),
        0.0,
        "model spend should be zero after reset"
    );
}

#[tokio::test]
async fn reset_allows_previously_blocked_requests() {
    let state = Arc::new(BudgetState::new());
    state.global_spend.store(usd_to_microcents(10.0), Ordering::Relaxed);

    let config = BudgetConfig {
        global_limit: Some(5.0),
        enforcement: Enforcement::Hard,
        ..Default::default()
    };

    let mut svc = build_service(config, Arc::clone(&state));

    let err = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
    assert!(err.is_err());

    state.reset();
    let ok = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
    assert!(ok.is_ok(), "should succeed after reset");
}

#[tokio::test]
async fn unlimited_config_allows_all_requests() {
    let state = Arc::new(BudgetState::new());
    let config = BudgetConfig::default();

    let mut svc = build_service(config, state);
    for _ in 0..20 {
        assert!(svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await.is_ok());
    }
}

#[tokio::test]
async fn propagates_inner_service_errors() {
    let state = Arc::new(BudgetState::new());
    let config = BudgetConfig {
        global_limit: Some(100.0),
        enforcement: Enforcement::Hard,
        ..Default::default()
    };

    let layer = BudgetLayer::new(config, state);
    let inner = LlmService::new(MockClient::failing_timeout());
    let mut svc = layer.layer(inner);

    let err = svc
        .call(LlmRequest::Chat(chat_req("gpt-4")))
        .await
        .expect_err("should propagate inner error");
    assert!(matches!(err, LiterLlmError::Timeout));
}

/// A minimal inner `Service` that returns a `ChatStream` response carrying
/// usage on its final chunk, so `BudgetService`'s streaming-accounting path
/// can be exercised without pulling in the full `LlmClient` mock surface.
#[derive(Clone)]
struct StreamingUsageService {
    prompt_tokens: u64,
    completion_tokens: u64,
}

fn usage_chunk(model: &str, usage: Option<Usage>) -> crate::types::ChatCompletionChunk {
    crate::types::ChatCompletionChunk {
        id: "chunk".into(),
        object: "chat.completion.chunk".into(),
        created: 0,
        model: model.into(),
        choices: vec![],
        usage,
        system_fingerprint: None,
        service_tier: None,
    }
}

struct ChunkStream(std::collections::VecDeque<crate::types::ChatCompletionChunk>);

impl futures_core::Stream for ChunkStream {
    type Item = Result<crate::types::ChatCompletionChunk>;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::task::Poll::Ready(self.0.pop_front().map(Ok))
    }
}

impl tower::Service<LlmRequest> for StreamingUsageService {
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        let model = req.model().unwrap_or("gpt-4").to_owned();
        let usage = Usage {
            prompt_tokens: self.prompt_tokens,
            completion_tokens: self.completion_tokens,
            total_tokens: self.prompt_tokens + self.completion_tokens,
            prompt_tokens_details: None,
        };
        Box::pin(async move {
            let chunks =
                std::collections::VecDeque::from([usage_chunk(&model, None), usage_chunk(&model, Some(usage))]);
            let stream: crate::client::BoxStream<'static, Result<crate::types::ChatCompletionChunk>> =
                Box::pin(ChunkStream(chunks));
            Ok(LlmResponse::ChatStream(stream))
        })
    }
}

/// Regression for the "streaming bypasses budget accounting" bug:
/// `LlmResponse::usage()` always returns `None` for `ChatStream`, so a
/// naive post-response check (`resp.usage()`) never sees the tokens a
/// streamed call actually consumed, and `BudgetState` is never updated —
/// a caller could stream unlimited tokens through a budget-limited
/// endpoint at zero recorded cost.
#[tokio::test]
async fn budget_service_records_cost_for_streamed_response() {
    use futures_util::StreamExt as _;

    let state = Arc::new(BudgetState::new());
    let config = BudgetConfig {
        global_limit: Some(1000.0),
        enforcement: Enforcement::Hard,
        ..Default::default()
    };
    let layer = BudgetLayer::new(config, Arc::clone(&state));
    let mut svc = layer.layer(StreamingUsageService {
        prompt_tokens: 1000,
        completion_tokens: 500,
    });

    let resp = svc
        .call(LlmRequest::ChatStream(chat_req("gpt-4")))
        .await
        .expect("streamed call should succeed");
    let LlmResponse::ChatStream(mut stream) = resp else {
        panic!("expected a ChatStream response");
    };

    // Drain the stream fully so the completion callback (which records cost) fires.
    while stream.next().await.is_some() {}

    assert!(
        state.global_spend() > 0.0,
        "cost of a streamed response must be recorded in global spend"
    );
    assert!(
        state.model_spend("gpt-4") > 0.0,
        "cost of a streamed response must be recorded per-model"
    );
}

/// Spend must still be recorded when the caller abandons the stream.
///
/// Accounting is settle-only, and settlement used to happen exclusively in
/// the stream's terminal poll — so a client that started a stream and
/// dropped it consumed real provider tokens (the whole completion is
/// generated and buffered before the caller sees a byte) while the ledger
/// stayed at zero.  Repeating that in a loop is unmetered usage.
#[tokio::test]
async fn abandoned_stream_still_records_spend() {
    use futures_util::StreamExt as _;

    let state = Arc::new(BudgetState::new());
    let config = BudgetConfig {
        global_limit: Some(1000.0),
        enforcement: Enforcement::Hard,
        ..Default::default()
    };
    let layer = BudgetLayer::new(config, Arc::clone(&state));
    let mut svc = layer.layer(StreamingUsageService {
        prompt_tokens: 1000,
        completion_tokens: 500,
    });

    let resp = svc
        .call(LlmRequest::ChatStream(chat_req("gpt-4")))
        .await
        .expect("streamed call should succeed");
    let LlmResponse::ChatStream(mut stream) = resp else {
        panic!("expected a ChatStream response");
    };

    // ~keep Take one chunk and walk away — the usage-bearing final chunk is never polled.
    let _ = stream.next().await;
    drop(stream);

    assert!(
        state.global_spend() > 0.0,
        "spend must be recorded even though the stream was dropped before it ended"
    );
    assert!(
        state.model_spend("gpt-4") > 0.0,
        "per-model spend must be recorded for an abandoned stream"
    );
}

/// Dropping without polling at all must also settle.
#[tokio::test]
async fn stream_dropped_without_a_single_poll_records_spend() {
    let state = Arc::new(BudgetState::new());
    let config = BudgetConfig {
        global_limit: Some(1000.0),
        enforcement: Enforcement::Hard,
        ..Default::default()
    };
    let layer = BudgetLayer::new(config, Arc::clone(&state));
    let mut svc = layer.layer(StreamingUsageService {
        prompt_tokens: 1000,
        completion_tokens: 500,
    });

    let resp = svc
        .call(LlmRequest::ChatStream(chat_req("gpt-4")))
        .await
        .expect("streamed call should succeed");
    let LlmResponse::ChatStream(stream) = resp else {
        panic!("expected a ChatStream response");
    };

    drop(stream);

    assert!(
        state.global_spend() > 0.0,
        "spend must be recorded for a stream that was never polled"
    );
}
