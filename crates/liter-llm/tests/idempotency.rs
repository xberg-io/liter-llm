//! Integration tests for [`liter_llm::tower::IdempotencyLayer`].
//!
//! Each test uses a `tower::service_fn` inner service backed by an
//! `AtomicUsize` call counter to verify that the layer correctly suppresses
//! or forwards calls to the inner service.

#![cfg(feature = "tower")]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use liter_llm::error::LiterLlmError;
use liter_llm::tower::idempotency::{IdempotencyLayer, InMemoryIdempotencyStore};
use liter_llm::tower::types::{LlmRequest, LlmResponse};
use tower::{Layer as _, Service, ServiceExt as _};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a mock chat completion request for the given model name.
fn chat_req(model: &str) -> liter_llm::types::ChatCompletionRequest {
    use liter_llm::types::{Message, SystemMessage};
    liter_llm::types::ChatCompletionRequest {
        model: model.into(),
        messages: vec![Message::System(SystemMessage {
            content: "test".into(),
            name: None,
        })],
        ..Default::default()
    }
}

/// Build a mock `LlmResponse::Chat` carrying the given model string.
fn make_chat_response(model: &str) -> LlmResponse {
    use liter_llm::types::{AssistantMessage, ChatCompletionResponse, Choice, FinishReason, Usage};
    LlmResponse::Chat(ChatCompletionResponse {
        id: "test-id".into(),
        object: "chat.completion".into(),
        created: 0,
        model: model.into(),
        choices: vec![Choice {
            index: 0,
            message: AssistantMessage {
                content: Some("Hello!".into()),
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
    })
}

/// Wrap an `AtomicUsize`-counted inner that always succeeds.
fn ok_inner(
    call_count: Arc<AtomicUsize>,
    model: &'static str,
) -> impl Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError, Future: Send> + Clone + Send + 'static {
    tower::service_fn(move |_req: LlmRequest| {
        let count = Arc::clone(&call_count);
        let model = model;
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(make_chat_response(model))
        }
    })
}

/// Wrap an `AtomicUsize`-counted inner that always fails with `RateLimited`.
fn failing_inner(
    call_count: Arc<AtomicUsize>,
) -> impl Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError, Future: Send> + Clone + Send + 'static {
    tower::service_fn(move |_req: LlmRequest| {
        let count = Arc::clone(&call_count);
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Err(LiterLlmError::RateLimited {
                message: "rate limited".into(),
                retry_after: None,
            })
        }
    })
}

fn req_with_key(model: &str, key: &str) -> LlmRequest {
    LlmRequest::Chat(chat_req(model)).with_idempotency_key(key)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// First request with a new key must invoke the inner service exactly once.
#[tokio::test]
async fn first_request_hits_inner() {
    let count = Arc::new(AtomicUsize::new(0));
    let layer = IdempotencyLayer::new(InMemoryIdempotencyStore::new());
    let mut svc = layer.layer(ok_inner(Arc::clone(&count), "gpt-4"));

    svc.ready()
        .await
        .unwrap()
        .call(req_with_key("gpt-4", "k-1"))
        .await
        .unwrap();

    assert_eq!(
        count.load(Ordering::SeqCst),
        1,
        "inner must be called once for the first request"
    );
}

/// Second request with the same key and body must return the cached response
/// WITHOUT invoking the inner service again.
#[tokio::test]
async fn repeat_same_key_same_body_returns_cached() {
    let count = Arc::new(AtomicUsize::new(0));
    let layer = IdempotencyLayer::new(InMemoryIdempotencyStore::new());
    let mut svc = layer.layer(ok_inner(Arc::clone(&count), "gpt-4"));

    // First call — populates the store.
    svc.ready()
        .await
        .unwrap()
        .call(req_with_key("gpt-4", "k-2"))
        .await
        .expect("first call must succeed");
    assert_eq!(count.load(Ordering::SeqCst), 1);

    // Second call — same key + same body → must return stored response.
    let resp = svc
        .ready()
        .await
        .unwrap()
        .call(req_with_key("gpt-4", "k-2"))
        .await
        .expect("second call must succeed");
    assert_eq!(
        count.load(Ordering::SeqCst),
        1,
        "inner must NOT be called again when returning cached response"
    );

    // Verify the returned response is the stored one (model string matches).
    match resp {
        LlmResponse::Chat(r) => assert_eq!(r.model, "gpt-4"),
        _ => panic!("expected Chat response"),
    }
}

/// Second request with the same key but a different body must return
/// `LiterLlmError::IdempotencyConflict`.
#[tokio::test]
async fn repeat_same_key_different_body_returns_conflict() {
    let count = Arc::new(AtomicUsize::new(0));
    let layer = IdempotencyLayer::new(InMemoryIdempotencyStore::new());
    let mut svc = layer.layer(ok_inner(Arc::clone(&count), "gpt-4"));

    // First call with model-a.
    svc.ready()
        .await
        .unwrap()
        .call(req_with_key("gpt-4", "k-3"))
        .await
        .expect("first call must succeed");

    // Second call — same key, different model → different body hash.
    let result = svc
        .ready()
        .await
        .unwrap()
        .call(req_with_key("gpt-3.5-turbo", "k-3"))
        .await;

    assert!(
        matches!(result, Err(LiterLlmError::IdempotencyConflict { .. })),
        "different body for same key must return IdempotencyConflict, got: {result:?}"
    );
    // Inner must not be called for the conflicting second request.
    assert_eq!(count.load(Ordering::SeqCst), 1, "inner must not be invoked on conflict");
}

/// A request without an idempotency key must pass through to the inner service
/// and the inner service must be invoked.
#[tokio::test]
async fn no_key_passes_through() {
    let count = Arc::new(AtomicUsize::new(0));
    let layer = IdempotencyLayer::new(InMemoryIdempotencyStore::new());
    let mut svc = layer.layer(ok_inner(Arc::clone(&count), "gpt-4"));

    let result = svc
        .ready()
        .await
        .unwrap()
        .call(LlmRequest::Chat(chat_req("gpt-4")))
        .await;
    assert!(result.is_ok(), "keyless request must succeed");
    assert_eq!(
        count.load(Ordering::SeqCst),
        1,
        "inner must be called for keyless request"
    );

    // A second keyless call must also hit inner (no dedup without a key).
    svc.ready()
        .await
        .unwrap()
        .call(LlmRequest::Chat(chat_req("gpt-4")))
        .await
        .unwrap();
    assert_eq!(
        count.load(Ordering::SeqCst),
        2,
        "each keyless call must hit inner independently"
    );
}

/// When the inner service fails, the placeholder entry must be removed so
/// subsequent calls with the same key+body retry the operation.
#[tokio::test]
async fn inner_error_does_not_cache() {
    let count = Arc::new(AtomicUsize::new(0));
    let layer = IdempotencyLayer::new(InMemoryIdempotencyStore::new());
    let mut svc = layer.layer(failing_inner(Arc::clone(&count)));

    // First call — inner fails.
    let first = svc.ready().await.unwrap().call(req_with_key("gpt-4", "k-err")).await;
    assert!(first.is_err(), "first call must fail");
    assert_eq!(count.load(Ordering::SeqCst), 1);

    // Second call — placeholder was removed; inner must be called again.
    let second = svc.ready().await.unwrap().call(req_with_key("gpt-4", "k-err")).await;
    assert!(second.is_err(), "second call must also fail");
    assert_eq!(
        count.load(Ordering::SeqCst),
        2,
        "inner must be called again after first failed call (error must not be cached)"
    );
}

/// TTL expiry allows a new invocation after the cached entry expires.
///
/// This test uses a very short TTL (1 ns) and verifies that after expiry the
/// store treats the key as unseen.  The service-level TTL expiry path is
/// covered by `InMemoryIdempotencyStore` unit tests; this test validates that
/// the service correctly re-calls inner when the store signals a miss.
#[tokio::test]
#[ignore = "wall-clock timing is flaky in CI; TTL expiry covered by store unit tests"]
async fn ttl_expiry_allows_new_invocation() {
    let count = Arc::new(AtomicUsize::new(0));
    let layer = IdempotencyLayer::with_ttl(InMemoryIdempotencyStore::new(), Duration::from_nanos(1));
    let mut svc = layer.layer(ok_inner(Arc::clone(&count), "gpt-4"));

    svc.ready()
        .await
        .unwrap()
        .call(req_with_key("gpt-4", "k-ttl"))
        .await
        .expect("first call");
    assert_eq!(count.load(Ordering::SeqCst), 1);

    // Wait past the TTL.
    tokio::time::sleep(Duration::from_millis(5)).await;

    // Entry expired — inner must be called again.
    svc.ready()
        .await
        .unwrap()
        .call(req_with_key("gpt-4", "k-ttl"))
        .await
        .expect("second call after expiry");
    assert_eq!(
        count.load(Ordering::SeqCst),
        2,
        "inner must be called again after TTL expiry"
    );
}
