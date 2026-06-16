//! Integration tests for [`liter_llm::tower::FallbackChainLayer`].

mod common;

use std::future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use tower::{Layer, Service};

use liter_llm::error::{LiterLlmError, Result};
use liter_llm::tower::fallback_chain::{
    DefaultRetryPolicy, FallbackChainLayer, FallbackChainService, RetryClass, RetryPolicy,
};
use liter_llm::tower::types::{LlmRequest, LlmResponse};
use liter_llm::types::{
    AssistantMessage, ChatCompletionRequest, ChatCompletionResponse, Choice, FinishReason, Message, SystemMessage,
    Usage,
};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn chat_req() -> LlmRequest {
    LlmRequest::Chat(ChatCompletionRequest {
        model: "gpt-4".into(),
        messages: vec![Message::System(SystemMessage {
            content: "ping".into(),
            name: None,
        })],
        ..Default::default()
    })
}

fn ok_response(tag: &str) -> LlmResponse {
    LlmResponse::Chat(ChatCompletionResponse {
        id: tag.into(),
        object: "chat.completion".into(),
        created: 0,
        model: tag.into(),
        choices: vec![Choice {
            index: 0,
            message: AssistantMessage {
                content: Some(tag.into()),
                name: None,
                tool_calls: None,
                refusal: None,
                function_call: None,
            },
            finish_reason: Some(FinishReason::Stop),
        }],
        usage: Some(Usage {
            prompt_tokens: 1,
            completion_tokens: 1,
            total_tokens: 2,
            prompt_tokens_details: None,
        }),
        system_fingerprint: None,
        service_tier: None,
    })
}

fn transient_err() -> LiterLlmError {
    LiterLlmError::ServiceUnavailable {
        message: "503".into(),
        status: 503,
    }
}

fn terminal_err() -> LiterLlmError {
    LiterLlmError::Authentication {
        message: "401".into(),
        status: 401,
    }
}

/// A stub `Service<LlmRequest>` that returns a canned result on every call
/// and counts how many times it was invoked.
#[derive(Clone)]
struct StubService {
    call_count: Arc<AtomicUsize>,
    result: StubResult,
}

#[derive(Clone)]
enum StubResult {
    Ok(String),
    Transient,
    Terminal,
}

impl StubService {
    fn succeeding(tag: &str) -> Self {
        Self {
            call_count: Arc::new(AtomicUsize::new(0)),
            result: StubResult::Ok(tag.into()),
        }
    }

    fn transient() -> Self {
        Self {
            call_count: Arc::new(AtomicUsize::new(0)),
            result: StubResult::Transient,
        }
    }

    fn terminal() -> Self {
        Self {
            call_count: Arc::new(AtomicUsize::new(0)),
            result: StubResult::Terminal,
        }
    }
}

impl Service<LlmRequest> for StubService {
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = future::Ready<Result<LlmResponse>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: LlmRequest) -> Self::Future {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        let result = match &self.result {
            StubResult::Ok(tag) => Ok(ok_response(tag)),
            StubResult::Transient => Err(transient_err()),
            StubResult::Terminal => Err(terminal_err()),
        };
        future::ready(result)
    }
}

// Convenience: build a `FallbackChainService` directly from a `Vec<StubService>`.
fn make_chain(services: Vec<StubService>) -> FallbackChainService<StubService> {
    let layer = FallbackChainLayer::<StubService, DefaultRetryPolicy>::new(services);
    // Layer<()> — inner is ignored, chain is the source of services.
    layer.layer(())
}

// ─── tests ────────────────────────────────────────────────────────────────────

/// Primary returns transient, secondary returns success → final OK.
#[tokio::test]
async fn walks_to_second_deployment_on_transient_failure() {
    let primary = StubService::transient();
    let secondary = StubService::succeeding("secondary");
    let primary_calls = Arc::clone(&primary.call_count);
    let secondary_calls = Arc::clone(&secondary.call_count);

    let mut svc = make_chain(vec![primary, secondary]);

    let resp = svc.call(chat_req()).await.expect("should succeed via secondary");

    assert_eq!(primary_calls.load(Ordering::SeqCst), 1, "primary invoked once");
    assert_eq!(secondary_calls.load(Ordering::SeqCst), 1, "secondary invoked once");

    match resp {
        LlmResponse::Chat(r) => assert_eq!(r.id, "secondary"),
        _ => panic!("unexpected response variant"),
    }
}

/// Primary returns terminal → secondary not invoked → error propagated.
#[tokio::test]
async fn aborts_on_terminal_error() {
    let primary = StubService::terminal();
    let secondary = StubService::succeeding("secondary");
    let primary_calls = Arc::clone(&primary.call_count);
    let secondary_calls = Arc::clone(&secondary.call_count);

    let mut svc = make_chain(vec![primary, secondary]);

    let err = svc.call(chat_req()).await.expect_err("should propagate terminal error");

    assert_eq!(primary_calls.load(Ordering::SeqCst), 1, "primary invoked once");
    assert_eq!(
        secondary_calls.load(Ordering::SeqCst),
        0,
        "secondary must not be invoked"
    );
    assert!(
        matches!(err, LiterLlmError::Authentication { .. }),
        "terminal error variant must be preserved"
    );
}

/// All N services return transient → final Err with the last transient error.
#[tokio::test]
async fn exhausts_chain_when_all_transient() {
    let a = StubService::transient();
    let b = StubService::transient();
    let c = StubService::transient();
    let calls_a = Arc::clone(&a.call_count);
    let calls_b = Arc::clone(&b.call_count);
    let calls_c = Arc::clone(&c.call_count);

    let mut svc = make_chain(vec![a, b, c]);

    let err = svc.call(chat_req()).await.expect_err("all transient → error");

    assert_eq!(calls_a.load(Ordering::SeqCst), 1);
    assert_eq!(calls_b.load(Ordering::SeqCst), 1);
    assert_eq!(calls_c.load(Ordering::SeqCst), 1);
    assert!(
        matches!(err, LiterLlmError::ServiceUnavailable { .. }),
        "last transient error must be propagated"
    );
}

/// Chain of length 1 — single service succeeding works.
#[tokio::test]
async fn single_deployment_chain_success() {
    let only = StubService::succeeding("only");
    let calls = Arc::clone(&only.call_count);

    let mut svc = make_chain(vec![only]);

    let resp = svc.call(chat_req()).await.expect("single-service chain must succeed");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    match resp {
        LlmResponse::Chat(r) => assert_eq!(r.id, "only"),
        _ => panic!("unexpected response variant"),
    }
}

/// Chain of length 1 — single service with transient error returns that error.
#[tokio::test]
async fn single_deployment_chain_transient_error_propagated() {
    let mut svc = make_chain(vec![StubService::transient()]);
    let err = svc.call(chat_req()).await.expect_err("single transient → error");
    assert!(matches!(err, LiterLlmError::ServiceUnavailable { .. }));
}

/// Empty chain returns an error without panicking.
#[tokio::test]
async fn empty_chain_returns_server_error() {
    let mut svc = make_chain(vec![]);
    let err = svc.call(chat_req()).await.expect_err("empty chain → error");
    assert!(matches!(err, LiterLlmError::ServerError { .. }));
}

/// Custom `RetryPolicy` can reclassify errors (here: treat ALL errors as terminal).
#[tokio::test]
async fn custom_retry_policy_overrides_classification() {
    #[derive(Clone, Copy)]
    struct AlwaysTerminalPolicy;

    impl RetryPolicy for AlwaysTerminalPolicy {
        fn classify(&self, _error: &LiterLlmError) -> RetryClass {
            RetryClass::Terminal
        }
    }

    let primary = StubService::transient();
    let secondary = StubService::succeeding("secondary");
    let primary_calls = Arc::clone(&primary.call_count);
    let secondary_calls = Arc::clone(&secondary.call_count);

    let layer = FallbackChainLayer::with_policy(vec![primary, secondary], AlwaysTerminalPolicy);
    let mut svc = layer.layer(());

    let err = svc
        .call(chat_req())
        .await
        .expect_err("policy classifies transient as terminal");

    assert_eq!(primary_calls.load(Ordering::SeqCst), 1, "primary invoked");
    assert_eq!(
        secondary_calls.load(Ordering::SeqCst),
        0,
        "secondary skipped due to policy"
    );
    assert!(matches!(err, LiterLlmError::ServiceUnavailable { .. }));
}

/// Multiple transient failures before a mid-chain success.
#[tokio::test]
async fn reaches_third_service_after_two_transient_failures() {
    let a = StubService::transient();
    let b = StubService::transient();
    let c = StubService::succeeding("third");
    let calls_a = Arc::clone(&a.call_count);
    let calls_b = Arc::clone(&b.call_count);
    let calls_c = Arc::clone(&c.call_count);

    let mut svc = make_chain(vec![a, b, c]);

    let resp = svc.call(chat_req()).await.expect("should succeed on third service");

    assert_eq!(calls_a.load(Ordering::SeqCst), 1);
    assert_eq!(calls_b.load(Ordering::SeqCst), 1);
    assert_eq!(calls_c.load(Ordering::SeqCst), 1);
    match resp {
        LlmResponse::Chat(r) => assert_eq!(r.id, "third"),
        _ => panic!("unexpected response variant"),
    }
}

/// `DefaultRetryPolicy` treats rate-limited (429) as transient.
#[tokio::test]
async fn default_policy_treats_rate_limited_as_transient() {
    #[derive(Clone)]
    struct RateLimitedService {
        call_count: Arc<AtomicUsize>,
    }

    impl Service<LlmRequest> for RateLimitedService {
        type Response = LlmResponse;
        type Error = LiterLlmError;
        type Future = future::Ready<Result<LlmResponse>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: LlmRequest) -> Self::Future {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            future::ready(Err(LiterLlmError::RateLimited {
                message: "429".into(),
                retry_after: None,
            }))
        }
    }

    let rate_limited = RateLimitedService {
        call_count: Arc::new(AtomicUsize::new(0)),
    };
    let calls = Arc::clone(&rate_limited.call_count);
    let backup = StubService::succeeding("backup");
    let backup_calls = Arc::clone(&backup.call_count);

    // To compose heterogeneous service types, box them.
    #[derive(Clone)]
    struct ErasedService(Arc<dyn Fn(LlmRequest) -> Result<LlmResponse> + Send + Sync>);

    impl Service<LlmRequest> for ErasedService {
        type Response = LlmResponse;
        type Error = LiterLlmError;
        type Future = future::Ready<Result<LlmResponse>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: LlmRequest) -> Self::Future {
            future::ready((self.0)(req))
        }
    }

    let calls2 = Arc::clone(&calls);
    let backup_calls2 = Arc::clone(&backup_calls);

    let rate_limited_erased = ErasedService(Arc::new(move |_req| {
        calls2.fetch_add(1, Ordering::SeqCst);
        Err(LiterLlmError::RateLimited {
            message: "429".into(),
            retry_after: None,
        })
    }));
    let backup_erased = ErasedService(Arc::new(move |_req| {
        backup_calls2.fetch_add(1, Ordering::SeqCst);
        Ok(ok_response("backup"))
    }));

    let layer = FallbackChainLayer::<ErasedService, DefaultRetryPolicy>::new(vec![rate_limited_erased, backup_erased]);
    let mut svc = layer.layer(());

    let resp = svc
        .call(chat_req())
        .await
        .expect("backup must handle rate-limited primary");
    assert_eq!(calls.load(Ordering::SeqCst), 1, "rate-limited service invoked");
    assert_eq!(
        backup_calls.load(Ordering::SeqCst),
        1,
        "backup invoked after transient 429"
    );
    match resp {
        LlmResponse::Chat(r) => assert_eq!(r.id, "backup"),
        _ => panic!("unexpected response variant"),
    }
}

/// `DefaultRetryPolicy` treats auth errors (401) as terminal — backup not invoked.
#[tokio::test]
async fn default_policy_treats_auth_error_as_terminal() {
    let policy = DefaultRetryPolicy;
    let err = LiterLlmError::Authentication {
        message: "401".into(),
        status: 401,
    };
    assert_eq!(policy.classify(&err), RetryClass::Terminal);
}

/// `FallbackChainLayer` is `Clone`.
#[test]
fn fallback_chain_layer_is_clone() {
    let layer = FallbackChainLayer::<StubService, DefaultRetryPolicy>::new(vec![StubService::transient()]);
    let _cloned = layer.clone();
}

/// `FallbackChainService` is `Clone`.
#[test]
fn fallback_chain_service_is_clone() {
    let svc = make_chain(vec![StubService::transient()]);
    let _cloned = svc.clone();
}
