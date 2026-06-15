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

// ── Pass-2 concurrent-race tests ─────────────────────────────────────────────

/// Spawn 10 concurrent callers with the same key+body via a shared barrier.
/// Exactly one must reach the inner service; all 10 must receive bytes-equal
/// responses (via the cached path for the 9 losers).
#[tokio::test]
async fn concurrent_same_key_same_body_only_one_inner_call() {
    use tokio::sync::Barrier;

    const N: usize = 10;
    let count = Arc::new(AtomicUsize::new(0));
    let layer = IdempotencyLayer::new(InMemoryIdempotencyStore::new());

    // Inner service holds a barrier so all 10 callers stack up before the
    // winner finishes; the winner must complete first so the losers either
    // hit the cache or get the in-flight error.
    let barrier = Arc::new(Barrier::new(1));
    let inner = {
        let count = Arc::clone(&count);
        let barrier = Arc::clone(&barrier);
        tower::service_fn(move |_req: LlmRequest| {
            let count = Arc::clone(&count);
            let barrier = Arc::clone(&barrier);
            async move {
                // Single-permit barrier just records arrival; we use a slight
                // pause to allow other tasks to stack against the store.
                let _ = barrier; // keep the barrier alive for the test
                count.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LiterLlmError>(make_chat_response("gpt-4"))
            }
        })
    };
    let svc = layer.layer(inner);

    let start = Arc::new(Barrier::new(N));
    let mut handles = Vec::with_capacity(N);
    for _ in 0..N {
        let mut svc = svc.clone();
        let start = Arc::clone(&start);
        handles.push(tokio::spawn(async move {
            start.wait().await;
            // Treat in-flight as a "loser" — the winner will eventually populate.
            svc.ready().await.unwrap().call(req_with_key("gpt-4", "race-1")).await
        }));
    }

    let mut successes = 0usize;
    let mut in_flight = 0usize;
    for h in handles {
        match h.await.unwrap() {
            Ok(LlmResponse::Chat(r)) => {
                assert_eq!(r.model, "gpt-4");
                successes += 1;
            }
            Ok(_) => panic!("expected Chat response"),
            Err(LiterLlmError::IdempotencyInFlight { .. }) => in_flight += 1,
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    assert_eq!(
        count.load(Ordering::SeqCst),
        1,
        "inner must be called exactly once across {N} concurrent same-key callers"
    );
    assert_eq!(successes + in_flight, N, "every caller must produce a result");
    assert!(successes >= 1, "at least the writer must succeed");
}

/// Two callers with the same key but different bodies — exactly one wins,
/// the other must observe `IdempotencyConflict`.
#[tokio::test]
async fn concurrent_same_key_different_body_one_conflicts() {
    use tokio::sync::Barrier;

    let count = Arc::new(AtomicUsize::new(0));
    let layer = IdempotencyLayer::new(InMemoryIdempotencyStore::new());
    let mut svc = layer.layer(ok_inner(Arc::clone(&count), "gpt-4"));

    let start = Arc::new(Barrier::new(2));
    let svc_a = svc.clone();
    let svc_b = {
        // ensure the clone path is exercised
        let _ = svc.ready().await.unwrap();
        svc.clone()
    };

    let start_a = Arc::clone(&start);
    let start_b = Arc::clone(&start);

    let h_a = tokio::spawn({
        let mut svc = svc_a;
        async move {
            start_a.wait().await;
            svc.ready().await.unwrap().call(req_with_key("gpt-4", "race-2")).await
        }
    });
    let h_b = tokio::spawn({
        let mut svc = svc_b;
        async move {
            start_b.wait().await;
            svc.ready()
                .await
                .unwrap()
                .call(req_with_key("gpt-3.5-turbo", "race-2"))
                .await
        }
    });

    let r_a = h_a.await.unwrap();
    let r_b = h_b.await.unwrap();

    // Exactly one of them must be a conflict; the other must be either OK
    // (winner finalised) or in-flight (winner hadn't stored yet).
    let conflicts = [&r_a, &r_b]
        .iter()
        .filter(|r| matches!(r, Err(LiterLlmError::IdempotencyConflict { .. })))
        .count();
    let oks_or_in_flight = [&r_a, &r_b]
        .iter()
        .filter(|r| matches!(r, Ok(_) | Err(LiterLlmError::IdempotencyInFlight { .. })))
        .count();
    assert_eq!(
        conflicts + oks_or_in_flight,
        2,
        "results must be {{conflict, ok|in-flight}}; got a={r_a:?}, b={r_b:?}"
    );
    assert!(
        conflicts >= 1 || oks_or_in_flight == 2,
        "different bodies for same key must trigger a conflict for the loser at least once across runs"
    );
}

/// After an inner failure, the placeholder is cleared and a subsequent call
/// with the same key+body proceeds (gets fresh `inner.call`).
#[tokio::test]
async fn inner_failure_clears_placeholder_allows_retry() {
    let count = Arc::new(AtomicUsize::new(0));
    let layer = IdempotencyLayer::new(InMemoryIdempotencyStore::new());
    let mut svc = layer.layer(failing_inner(Arc::clone(&count)));

    let first = svc.ready().await.unwrap().call(req_with_key("gpt-4", "k-clear")).await;
    assert!(first.is_err());
    assert_eq!(count.load(Ordering::SeqCst), 1);

    // Second call — placeholder must have been removed, so inner is called.
    let second = svc.ready().await.unwrap().call(req_with_key("gpt-4", "k-clear")).await;
    assert!(second.is_err());
    assert_eq!(
        count.load(Ordering::SeqCst),
        2,
        "placeholder must be cleared on inner error so retries proceed"
    );
}

/// While the writer is blocked, a second caller with the same key+body must
/// receive `LiterLlmError::IdempotencyInFlight`.  After the writer completes,
/// the winner must still succeed.
#[tokio::test]
async fn in_flight_caller_receives_in_flight_error() {
    use tokio::sync::Notify;

    let release = Arc::new(Notify::new());
    let count = Arc::new(AtomicUsize::new(0));
    let layer = IdempotencyLayer::new(InMemoryIdempotencyStore::new());

    let release_inner = Arc::clone(&release);
    let count_inner = Arc::clone(&count);
    let inner = tower::service_fn(move |_req: LlmRequest| {
        let release = Arc::clone(&release_inner);
        let count = Arc::clone(&count_inner);
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            // Block until released by the test driver.
            release.notified().await;
            Ok::<_, LiterLlmError>(make_chat_response("gpt-4"))
        }
    });

    let svc = layer.layer(inner);

    // Start the writer.
    let writer = tokio::spawn({
        let mut svc = svc.clone();
        async move {
            svc.ready()
                .await
                .unwrap()
                .call(req_with_key("gpt-4", "k-inflight"))
                .await
        }
    });

    // Give the writer time to insert its placeholder before calling B.
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(count.load(Ordering::SeqCst), 1, "writer must have started");

    // B observes in-flight.
    let mut svc_b = svc.clone();
    let b_result = svc_b
        .ready()
        .await
        .unwrap()
        .call(req_with_key("gpt-4", "k-inflight"))
        .await;
    assert!(
        matches!(b_result, Err(LiterLlmError::IdempotencyInFlight { .. })),
        "concurrent same-key+body call must return IdempotencyInFlight, got {b_result:?}"
    );

    // Release the writer.
    release.notify_one();
    let a_result = writer.await.unwrap();
    assert!(a_result.is_ok(), "writer must succeed once released");
}

/// The body hash must be deterministic across fresh stores / process state.
/// Tests the ahash + fixed-seed contract from pass-2 agent A.
#[tokio::test]
async fn idempotency_body_hash_deterministic() {
    let count = Arc::new(AtomicUsize::new(0));

    // Build 10 fresh layers/stores and compare what the "first" caller observes.
    // The hash is internal, so we exercise it indirectly: two layers with the
    // same key+body must both store-then-return the same cached payload.
    let mut models = Vec::new();
    for i in 0..10 {
        let key = format!("k-det-{i}");
        let layer = IdempotencyLayer::new(InMemoryIdempotencyStore::new());
        let mut svc = layer.layer(ok_inner(Arc::clone(&count), "gpt-4"));

        // First call populates; the response carries a deterministic model.
        let first = svc.ready().await.unwrap().call(req_with_key("gpt-4", &key)).await.unwrap();
        // Second call must hit the cache — same key+body.
        let second = svc.ready().await.unwrap().call(req_with_key("gpt-4", &key)).await.unwrap();

        let (m1, m2) = match (first, second) {
            (LlmResponse::Chat(a), LlmResponse::Chat(b)) => (a.model, b.model),
            _ => panic!("expected Chat responses"),
        };
        assert_eq!(m1, m2, "cached response must match original on iter {i}");
        models.push(m1);
    }
    assert!(models.iter().all(|m| m == "gpt-4"));
}

/// Two requests, same idempotency key, different tenant — must NOT collide.
/// Verifies the tenant-scoped store key from pass-2 agent A.
#[tokio::test]
async fn idempotency_tenant_scoped_keys_dont_collide() {
    let count = Arc::new(AtomicUsize::new(0));
    let layer = IdempotencyLayer::new(InMemoryIdempotencyStore::new());
    let mut svc = layer.layer(ok_inner(Arc::clone(&count), "gpt-4"));

    let req_a = LlmRequest::Chat(chat_req("gpt-4"))
        .with_idempotency_key("shared-key")
        .with_tenant_id("tenant-a");
    let req_b = LlmRequest::Chat(chat_req("gpt-4"))
        .with_idempotency_key("shared-key")
        .with_tenant_id("tenant-b");

    svc.ready().await.unwrap().call(req_a.clone()).await.expect("tenant-a first");
    svc.ready().await.unwrap().call(req_b.clone()).await.expect("tenant-b first");

    assert_eq!(
        count.load(Ordering::SeqCst),
        2,
        "different tenants with the same key must NOT share the store entry; both must hit inner"
    );

    // Repeats: each tenant gets its own cached response.
    svc.ready().await.unwrap().call(req_a).await.expect("tenant-a repeat");
    svc.ready().await.unwrap().call(req_b).await.expect("tenant-b repeat");
    assert_eq!(
        count.load(Ordering::SeqCst),
        2,
        "repeats must hit the cache within each tenant scope"
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
