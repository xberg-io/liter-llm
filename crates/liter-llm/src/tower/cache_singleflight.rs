//! Singleflight deduplication middleware.
//!
//! Under concurrent bursts, multiple callers may issue identical requests
//! simultaneously.  Without coordination, each caller independently hits
//! the upstream LLM provider, multiplying cost and saturating rate limits.
//!
//! [`SingleflightLayer`] collapses concurrent identical requests into a single
//! upstream call.  The *leader* — the first caller for a given key — performs
//! the real work; all subsequent *followers* await the leader's result and
//! receive the same value.
//!
//! # Design
//!
//! The [`SingleflightCoordinator`] trait is the extension point.  The default
//! implementation ([`InMemorySingleflight`]) uses a [`dashmap::DashMap`] of
//! Tokio broadcast channels.  Broadcast (rather than a single `oneshot`) lets
//! an arbitrary number of followers subscribe without any follower needing to
//! hold a unique receiver slot — the channel retains the last value and late
//! subscribers obtain it via `resubscribe`.
//!
//! # Recommended layer order
//!
//! See [`crate::tower::cache`] module documentation for the full recommended
//! layer composition order.
//!
//! # Panics
//!
//! `SingleflightService` does not panic in normal operation.  `unwrap` calls
//! inside the implementation are guarded by invariants documented in `SAFETY`
//! comments.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use dashmap::DashMap;
use tokio::sync::broadcast;
use tower::{Layer, Service};

use super::cache::CachedResponse;
use super::types::{LlmRequest, LlmRequestKind, LlmResponse};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};

// Type alias for the shared in-flight map.
type InFlightMap = Arc<DashMap<u64, broadcast::Sender<SingleflightResult>>>;

// ─── SingleflightResult ───────────────────────────────────────────────────────

/// The value broadcast from a singleflight leader to all followers.
///
/// `Arc<LiterLlmError>` is used because `LiterLlmError` is not `Clone` and
/// broadcast channels require `T: Clone`.  The `Arc` adds only a reference-count
/// bump per follower, which is negligible under the burst loads this layer targets.
pub type SingleflightResult = std::result::Result<CachedResponse, Arc<LiterLlmError>>;

// ─── SingleflightHandle ───────────────────────────────────────────────────────

/// Outcome of [`SingleflightCoordinator::join`].
///
/// - A [`SingleflightHandle::Leader`] performs the upstream call and delivers
///   the result by calling the `complete` closure.
/// - A [`SingleflightHandle::Follower`] awaits the leader's result via the
///   broadcast receiver.
pub enum SingleflightHandle {
    /// First caller for this key.  Caller is responsible for performing the
    /// upstream work and signalling completion via `complete`.
    Leader {
        /// Deliver the result to all waiting followers.
        ///
        /// Calling `complete` is mandatory.  Dropping it without calling causes
        /// all followers to receive a `RecvError` (channel closed), which the
        /// `SingleflightService` maps to an `InternalError`.
        complete: Box<dyn FnOnce(SingleflightResult) + Send>,
    },
    /// Subsequent caller.  Awaits the leader's broadcast result.
    Follower {
        /// Receiver for the leader's result.  Call `.await` to block until the
        /// leader completes.
        recv: broadcast::Receiver<SingleflightResult>,
    },
}

// ─── SingleflightCoordinator trait ────────────────────────────────────────────

/// Pluggable singleflight coordination strategy.
///
/// Implement this trait to provide distributed singleflight coordination (e.g.
/// via Redis `SET NX` / pub-sub) without modifying library code.
///
/// The default in-process implementation is [`InMemorySingleflight`].
#[cfg_attr(alef, alef(skip))]
pub trait SingleflightCoordinator: Send + Sync + 'static {
    /// Register the caller's interest in `key`.
    ///
    /// Returns a [`SingleflightHandle`] that indicates whether this caller is
    /// the leader (must do upstream work) or a follower (must await the leader).
    fn join<'a>(&'a self, key: u64) -> Pin<Box<dyn Future<Output = SingleflightHandle> + Send + 'a>>;
}

// ─── InMemorySingleflight ─────────────────────────────────────────────────────

/// In-memory singleflight coordinator backed by a [`DashMap`] of broadcast channels.
///
/// Each in-flight key maps to a `broadcast::Sender<SingleflightResult>`.  The
/// first caller for a key creates the sender (becoming the leader).  Subsequent
/// callers subscribe to the same sender (becoming followers).  When the leader
/// calls `complete`, the result is broadcast to all subscribers.
///
/// Entries are removed from the map by the `complete` closure immediately after
/// broadcasting, so that the next distinct request for the same key starts a
/// fresh singleflight round.
#[cfg_attr(alef, alef(skip))]
pub struct InMemorySingleflight {
    /// Shared in-flight map, wrapped in `Arc` so it can be moved into the
    /// `complete` closure without lifetime constraints.
    ///
    /// A broadcast channel capacity of 1 is sufficient: the channel carries a
    /// single result event.  Late subscribers (followers that join after the
    /// leader completes) receive the stored value from the channel's ring buffer.
    in_flight: InFlightMap,
}

impl Default for InMemorySingleflight {
    fn default() -> Self {
        Self {
            in_flight: Arc::new(DashMap::new()),
        }
    }
}

impl InMemorySingleflight {
    /// Create a new coordinator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SingleflightCoordinator for InMemorySingleflight {
    fn join<'a>(&'a self, key: u64) -> Pin<Box<dyn Future<Output = SingleflightHandle> + Send + 'a>> {
        Box::pin(async move {
            use dashmap::mapref::entry::Entry;

            match self.in_flight.entry(key) {
                Entry::Vacant(slot) => {
                    // This caller is the leader: create the channel and claim the entry.
                    let (tx, _) = broadcast::channel::<SingleflightResult>(1);
                    let tx_for_map = tx.clone();
                    slot.insert(tx_for_map);

                    // Clone the `Arc` so the `complete` closure can own a reference
                    // to the map independently of the coordinator's lifetime.
                    let map = Arc::clone(&self.in_flight);

                    let complete = Box::new(move |result: SingleflightResult| {
                        // Send BEFORE removing the map entry (bug 5 fix).
                        //
                        // With the old remove-then-send order, a new caller arriving
                        // between the remove and the send sees a Vacant slot, becomes
                        // a leader, and starts a duplicate upstream call.  Sending
                        // first ensures any subscriber that joined before complete()
                        // receives the result before the entry is removed.
                        let _ = tx.send(result);
                        // Remove after broadcasting so the next distinct request
                        // starts a fresh singleflight round.
                        map.remove(&key);
                    });

                    SingleflightHandle::Leader { complete }
                }
                Entry::Occupied(entry) => {
                    // Subsequent caller: subscribe to the existing channel.
                    let recv = entry.get().subscribe();
                    SingleflightHandle::Follower { recv }
                }
            }
        })
    }
}

// ─── SingleflightLayer ────────────────────────────────────────────────────────

/// Tower [`Layer`] that collapses concurrent identical requests into one
/// upstream call via a [`SingleflightCoordinator`].
#[cfg_attr(alef, alef(skip))]
pub struct SingleflightLayer<C: SingleflightCoordinator> {
    coordinator: Arc<C>,
}

impl<C: SingleflightCoordinator> SingleflightLayer<C> {
    /// Create a new singleflight layer with the given coordinator.
    #[must_use]
    pub fn new(coordinator: Arc<C>) -> Self {
        Self { coordinator }
    }
}

impl<C: SingleflightCoordinator, S> Layer<S> for SingleflightLayer<C> {
    type Service = SingleflightService<C, S>;

    fn layer(&self, inner: S) -> Self::Service {
        SingleflightService {
            coordinator: Arc::clone(&self.coordinator),
            inner,
        }
    }
}

// ─── SingleflightService ──────────────────────────────────────────────────────

/// Tower service produced by [`SingleflightLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct SingleflightService<C: SingleflightCoordinator, S> {
    coordinator: Arc<C>,
    inner: S,
}

impl<C: SingleflightCoordinator, S: Clone> Clone for SingleflightService<C, S> {
    fn clone(&self) -> Self {
        Self {
            coordinator: Arc::clone(&self.coordinator),
            inner: self.inner.clone(),
        }
    }
}

/// Derive the singleflight key from a request.
///
/// Only `Chat` and `Embed` requests are deduplicated; other variants are
/// passed through without coordination.  Returns `None` for non-cacheable
/// variants.
fn singleflight_key(req: &LlmRequest) -> Option<u64> {
    use std::hash::{DefaultHasher, Hash, Hasher};

    let json = match &req.kind {
        LlmRequestKind::Chat(r) => serde_json::to_string(r).ok()?,
        LlmRequestKind::Embed(r) => serde_json::to_string(r).ok()?,
        _ => return None,
    };
    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    Some(hasher.finish())
}

impl<C, S> Service<LlmRequest> for SingleflightService<C, S>
where
    C: SingleflightCoordinator,
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        let key = singleflight_key(&req);

        // Non-deduplicatable requests pass straight through.
        // The `async move { fut.await }` would normally trigger `redundant_async_block`
        // but is required here because `Self::Future` is `BoxFuture<'static, ...>` while
        // `S::Future` is the inner service's concrete future type — they are distinct types.
        let Some(key) = key else {
            let fut = self.inner.call(req);
            #[allow(clippy::redundant_async_block)]
            return Box::pin(async move { fut.await });
        };

        let coordinator = Arc::clone(&self.coordinator);

        // Tower contract: `poll_ready` readied `self.inner` for exactly one call.
        // We must consume that readied slot for the leader path and leave `self.inner`
        // in a fresh (un-readied) state for the next `poll_ready`/`call` cycle.
        //
        // Pattern: clone the service to obtain a fresh standby, then `mem::replace`
        // so that `inner` holds the poll_ready'd instance and `self.inner` holds the
        // fresh clone.  Only the leader ever invokes `inner.call(req)`; followers drop
        // `inner` without calling it, which is safe because `call` was never invoked.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            match coordinator.join(key).await {
                SingleflightHandle::Leader { complete } => {
                    // Leader is the sole caller of `inner.call`.  This satisfies Tower's
                    // contract: exactly one `call` per `poll_ready`.
                    let result = inner.call(req).await;
                    // Convert the upstream result into a `SingleflightResult` to
                    // broadcast.  Success path clones the inner response into a
                    // `CachedResponse` so followers receive the same value.
                    let sf_result: SingleflightResult = match &result {
                        Ok(resp) => match resp {
                            LlmResponse::Chat(r) => Ok(CachedResponse::Chat(r.clone())),
                            LlmResponse::Embed(r) => Ok(CachedResponse::Embed(r.clone())),
                            // For non-cacheable response variants (should not reach here
                            // given the key derivation guard above), broadcast a synthetic
                            // error and return the real response to the leader only.
                            _ => Err(Arc::new(LiterLlmError::InternalError {
                                message: "singleflight: non-cacheable response variant in leader".into(),
                            })),
                        },
                        Err(e) => Err(Arc::new(LiterLlmError::InternalError { message: e.to_string() })),
                    };
                    complete(sf_result);
                    result
                }
                SingleflightHandle::Follower { mut recv } => {
                    // Follower never calls `inner.call(req)` — safe to drop because
                    // Tower only prohibits calling after poll_ready; skipping the call
                    // is always allowed.
                    drop(inner);
                    match recv.recv().await {
                        Ok(Ok(cached)) => cached.into_llm_response(),
                        Ok(Err(arc_err)) => {
                            Err(
                                Arc::try_unwrap(arc_err).unwrap_or_else(|arc| LiterLlmError::InternalError {
                                    message: arc.to_string(),
                                }),
                            )
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            // Ring-buffer overflow: resubscribe to drain the latest value.
                            tracing::debug!(skipped = n, "singleflight follower lagged; resubscribing");
                            let mut rx2 = recv.resubscribe();
                            match rx2.recv().await {
                                Ok(Ok(cached)) => cached.into_llm_response(),
                                Ok(Err(arc_err)) => {
                                    Err(
                                        Arc::try_unwrap(arc_err).unwrap_or_else(|arc| LiterLlmError::InternalError {
                                            message: arc.to_string(),
                                        }),
                                    )
                                }
                                Err(_) => Err(LiterLlmError::InternalError {
                                    message: "singleflight: follower lagged and retry also failed".into(),
                                }),
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => Err(LiterLlmError::InternalError {
                            message: "singleflight: leader closed channel without sending a result".into(),
                        }),
                    }
                }
            }
        })
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    use tower::{Layer as _, Service as _};

    use super::*;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::LlmRequest;

    /// A slow inner service that introduces an artificial delay so that all
    /// concurrent callers can arrive at the singleflight coordinator before the
    /// leader completes.
    ///
    /// Without a delay, `MockClient` returns synchronously and the leader
    /// completes before follower tasks are scheduled, defeating deduplication.
    #[derive(Clone)]
    struct SlowClient {
        inner: MockClient,
        delay: std::time::Duration,
    }

    impl SlowClient {
        fn ok_with_delay(delay: std::time::Duration) -> Self {
            Self {
                inner: MockClient::ok(),
                delay,
            }
        }
    }

    impl crate::client::LlmClient for SlowClient {
        fn chat(
            &self,
            req: crate::types::ChatCompletionRequest,
        ) -> crate::client::BoxFuture<'_, crate::error::Result<crate::types::ChatCompletionResponse>> {
            let delay = self.delay;
            let inner_fut = self.inner.chat(req);
            Box::pin(async move {
                tokio::time::sleep(delay).await;
                inner_fut.await
            })
        }

        fn chat_stream(
            &self,
            req: crate::types::ChatCompletionRequest,
        ) -> crate::client::BoxFuture<
            '_,
            crate::error::Result<
                crate::client::BoxStream<'static, crate::error::Result<crate::types::ChatCompletionChunk>>,
            >,
        > {
            self.inner.chat_stream(req)
        }

        fn embed(
            &self,
            req: crate::types::EmbeddingRequest,
        ) -> crate::client::BoxFuture<'_, crate::error::Result<crate::types::EmbeddingResponse>> {
            self.inner.embed(req)
        }

        fn list_models(&self) -> crate::client::BoxFuture<'_, crate::error::Result<crate::types::ModelsListResponse>> {
            self.inner.list_models()
        }

        fn image_generate(
            &self,
            req: crate::types::image::CreateImageRequest,
        ) -> crate::client::BoxFuture<'_, crate::error::Result<crate::types::image::ImagesResponse>> {
            self.inner.image_generate(req)
        }

        fn speech(
            &self,
            req: crate::types::audio::CreateSpeechRequest,
        ) -> crate::client::BoxFuture<'_, crate::error::Result<bytes::Bytes>> {
            self.inner.speech(req)
        }

        fn transcribe(
            &self,
            req: crate::types::audio::CreateTranscriptionRequest,
        ) -> crate::client::BoxFuture<'_, crate::error::Result<crate::types::audio::TranscriptionResponse>> {
            self.inner.transcribe(req)
        }

        fn moderate(
            &self,
            req: crate::types::moderation::ModerationRequest,
        ) -> crate::client::BoxFuture<'_, crate::error::Result<crate::types::moderation::ModerationResponse>> {
            self.inner.moderate(req)
        }

        fn rerank(
            &self,
            req: crate::types::rerank::RerankRequest,
        ) -> crate::client::BoxFuture<'_, crate::error::Result<crate::types::rerank::RerankResponse>> {
            self.inner.rerank(req)
        }

        fn search(
            &self,
            req: crate::types::search::SearchRequest,
        ) -> crate::client::BoxFuture<'_, crate::error::Result<crate::types::search::SearchResponse>> {
            self.inner.search(req)
        }

        fn ocr(
            &self,
            req: crate::types::ocr::OcrRequest,
        ) -> crate::client::BoxFuture<'_, crate::error::Result<crate::types::ocr::OcrResponse>> {
            self.inner.ocr(req)
        }
    }

    /// Spawn `n` concurrent requests for the same key via *independent service clones*
    /// that share an `Arc<InMemorySingleflight>`, then assert inner was called exactly once.
    ///
    /// Using independent clones is critical: a single `&mut self` service can only
    /// handle one request at a time (Tower's contract), so sharing a single service
    /// behind a `Mutex` would serialize all calls and defeat singleflight.  Each clone
    /// calls `poll_ready` + `call` independently, but the shared coordinator collapses
    /// them into one upstream call.
    ///
    /// A slow inner service ensures all 100 tasks arrive at the coordinator
    /// while the leader is still awaiting its upstream call.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn singleflight_leader_runs_upstream_once_under_burst() {
        let client = SlowClient::ok_with_delay(std::time::Duration::from_millis(50));
        let call_count = Arc::clone(&client.inner.call_count);
        let coordinator = Arc::new(InMemorySingleflight::new());
        let layer = SingleflightLayer::new(Arc::clone(&coordinator));

        // Use a barrier so all spawned tasks arrive at `call` simultaneously.
        let barrier = Arc::new(tokio::sync::Barrier::new(100));

        let handles: Vec<_> = (0..100)
            .map(|_| {
                // Each task gets its own clone that shares the coordinator Arc.
                let svc = layer.layer(LlmService::new(client.clone()));
                let barrier = Arc::clone(&barrier);
                tokio::spawn(async move {
                    barrier.wait().await;
                    let mut svc = svc;
                    // Tower contract: call poll_ready before call.
                    use tower::Service as _;
                    futures_util::future::poll_fn(|cx| svc.poll_ready(cx)).await.unwrap();
                    svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await
                })
            })
            .collect();

        let results: Vec<_> = futures_util::future::join_all(handles).await;
        let success_count = results.iter().filter(|r| r.as_ref().unwrap().is_ok()).count();
        assert_eq!(success_count, 100, "all 100 callers should get a successful response");

        let calls = call_count.load(Ordering::SeqCst);
        // With a 50ms delay in the upstream, all 100 tasks arrive while the
        // leader awaits — singleflight should collapse to exactly 1 call.
        assert_eq!(
            calls, 1,
            "inner service must be called exactly once under burst; got {calls}"
        );
    }

    /// 10 concurrent requests via independent service clones all receive the same result.
    ///
    /// Uses `SlowClient` (50 ms delay) so all 10 tasks reach the coordinator as
    /// followers before the leader's upstream call completes.  Without the delay
    /// the leader may complete before followers subscribe, causing spurious second
    /// leader rounds.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn singleflight_followers_get_same_result() {
        let client = SlowClient::ok_with_delay(std::time::Duration::from_millis(50));
        let coordinator = Arc::new(InMemorySingleflight::new());
        let layer = SingleflightLayer::new(Arc::clone(&coordinator));

        let barrier = Arc::new(tokio::sync::Barrier::new(10));
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let svc = layer.layer(LlmService::new(client.clone()));
                let barrier = Arc::clone(&barrier);
                tokio::spawn(async move {
                    barrier.wait().await;
                    let mut svc = svc;
                    futures_util::future::poll_fn(|cx| svc.poll_ready(cx)).await.unwrap();
                    svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await
                })
            })
            .collect();

        let results: Vec<_> = futures_util::future::join_all(handles).await;
        // Extract the model field from each response to verify they are identical.
        let models: Vec<String> = results
            .into_iter()
            .map(|join_result| {
                let llm_resp = join_result
                    .expect("task did not panic")
                    .expect("service call succeeded");
                match llm_resp {
                    LlmResponse::Chat(r) => r.model,
                    _ => panic!("expected Chat response"),
                }
            })
            .collect();

        // All responses should carry the same model string set by MockClient.
        let first = &models[0];
        assert!(
            models.iter().all(|m| m == first),
            "all followers must receive the same result"
        );
    }

    /// When the leader returns an error, all followers receive that error.
    ///
    /// A `SlowClient` with a 50 ms delay ensures all 10 tasks subscribe as followers
    /// before the leader's future resolves — otherwise the fast `MockClient` would
    /// complete before followers arrive, causing multiple "leader" rounds.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn singleflight_leader_error_propagates_to_followers() {
        // Use a slow failing client so all 10 tasks arrive while the leader is still
        // awaiting its upstream call.
        let inner_client = MockClient::failing_rate_limited();
        let slow_client = SlowClient {
            inner: inner_client,
            delay: std::time::Duration::from_millis(50),
        };
        let call_count = Arc::clone(&slow_client.inner.call_count);
        let coordinator = Arc::new(InMemorySingleflight::new());
        let layer = SingleflightLayer::new(Arc::clone(&coordinator));

        let barrier = Arc::new(tokio::sync::Barrier::new(10));
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let svc = layer.layer(LlmService::new(slow_client.clone()));
                let barrier = Arc::clone(&barrier);
                tokio::spawn(async move {
                    barrier.wait().await;
                    let mut svc = svc;
                    futures_util::future::poll_fn(|cx| svc.poll_ready(cx)).await.unwrap();
                    svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await
                })
            })
            .collect();

        let results: Vec<_> = futures_util::future::join_all(handles).await;
        let error_count = results.iter().filter(|r| r.as_ref().unwrap().is_err()).count();

        // All callers should receive an error.
        assert_eq!(error_count, 10, "all callers must receive the leader's error");

        // With a 50 ms delay, all 10 tasks arrive while the leader is awaiting;
        // inner should be called exactly once.
        let calls = call_count.load(Ordering::SeqCst);
        assert_eq!(
            calls, 1,
            "inner should be called exactly once under singleflight; got {calls}"
        );
    }

    /// Followers must never invoke `inner.call` — only the leader does.
    ///
    /// Wire a slow mock with a call counter, fire 10 concurrent requests for the
    /// same key, and assert the inner counter is exactly 1 (the leader) even though
    /// all 10 callers received a successful response.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn singleflight_follower_does_not_call_inner_service() {
        let client = SlowClient::ok_with_delay(std::time::Duration::from_millis(50));
        let call_count = Arc::clone(&client.inner.call_count);
        let coordinator = Arc::new(InMemorySingleflight::new());
        let layer = SingleflightLayer::new(Arc::clone(&coordinator));

        let barrier = Arc::new(tokio::sync::Barrier::new(10));
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let svc = layer.layer(LlmService::new(client.clone()));
                let barrier = Arc::clone(&barrier);
                tokio::spawn(async move {
                    barrier.wait().await;
                    let mut svc = svc;
                    use tower::Service as _;
                    futures_util::future::poll_fn(|cx| svc.poll_ready(cx)).await.unwrap();
                    svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await
                })
            })
            .collect();

        let results: Vec<_> = futures_util::future::join_all(handles).await;
        let success_count = results.iter().filter(|r| r.as_ref().unwrap().is_ok()).count();
        assert_eq!(success_count, 10, "all 10 callers should succeed");

        let calls = call_count.load(Ordering::SeqCst);
        assert_eq!(
            calls, 1,
            "inner service must be called exactly once (leader only); followers must not call it; got {calls}"
        );
    }

    /// Requests with distinct keys must not be deduplicated — each key triggers its
    /// own upstream call.
    ///
    /// Fire 10 concurrent requests with 10 different model names (which produces
    /// 10 different cache keys) and assert the inner service call counter equals 10.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn singleflight_concurrent_keys_dont_dedupe() {
        let client = SlowClient::ok_with_delay(std::time::Duration::from_millis(20));
        let call_count = Arc::clone(&client.inner.call_count);
        let coordinator = Arc::new(InMemorySingleflight::new());
        let layer = SingleflightLayer::new(Arc::clone(&coordinator));

        let barrier = Arc::new(tokio::sync::Barrier::new(10));
        let handles: Vec<_> = (0..10u32)
            .map(|i| {
                let svc = layer.layer(LlmService::new(client.clone()));
                let barrier = Arc::clone(&barrier);
                tokio::spawn(async move {
                    barrier.wait().await;
                    let mut svc = svc;
                    use tower::Service as _;
                    futures_util::future::poll_fn(|cx| svc.poll_ready(cx)).await.unwrap();
                    // Each task uses a distinct model name → distinct cache key.
                    svc.call(LlmRequest::Chat(chat_req(&format!("gpt-4-model-{i}")))).await
                })
            })
            .collect();

        let results: Vec<_> = futures_util::future::join_all(handles).await;
        let success_count = results.iter().filter(|r| r.as_ref().unwrap().is_ok()).count();
        assert_eq!(success_count, 10, "all 10 distinct-key callers should succeed");

        let calls = call_count.load(Ordering::SeqCst);
        assert_eq!(
            calls, 10,
            "each distinct key must produce its own upstream call; got {calls}"
        );
    }

    /// Bug 5 fix: send-before-remove ordering in `complete` closure.
    ///
    /// A follower that subscribes BEFORE the leader calls `complete` must
    /// receive the result — not a `RecvError::Closed`.
    #[tokio::test]
    async fn singleflight_no_duplicate_upstream_on_late_arrival() {
        let coordinator = Arc::new(InMemorySingleflight::new());
        let key: u64 = 0xC0FF_EE00;

        // Leader joins first.
        let complete = match coordinator.join(key).await {
            SingleflightHandle::Leader { complete } => complete,
            SingleflightHandle::Follower { .. } => panic!("first join must be Leader"),
        };

        // Follower joins while the entry is still in the map.
        let mut recv = match coordinator.join(key).await {
            SingleflightHandle::Follower { recv } => recv,
            SingleflightHandle::Leader { .. } => panic!("second join must be Follower"),
        };

        // Leader completes: send first, remove second.
        complete(Ok(CachedResponse::Chat(
            crate::tower::tests_common::make_chat_response("gpt-4"),
        )));

        // Follower must receive the result (not RecvError::Closed).
        let received = recv.recv().await.expect("follower must receive leader result");
        assert!(received.is_ok(), "follower must receive success result");
    }
}
