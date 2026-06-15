//! Tower middleware that invokes user-defined hooks before and after requests.
//!
//! [`HooksLayer`] wraps any [`Service<LlmRequest>`] and calls registered
//! [`LlmHook`] implementations at three lifecycle points:
//!
//! - **`on_request`** — before the request is forwarded to the inner service.
//!   Returning `Err` from any hook short-circuits the chain (guardrail
//!   rejection).
//! - **`on_response`** — after a successful response from the inner service.
//! - **`on_error`** — when the inner service returns an error.
//!
//! Hooks are invoked sequentially in registration order.
//!
//! Attach a [`crate::observability::UsageSink`] via
//! [`HooksLayer::with_usage_sink`] to receive one
//! [`crate::observability::UsageEvent`] per completed request (success or
//! error). The sink call is best-effort: errors are logged and do not affect
//! the caller's response.
//!
//! # Example
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use liter_llm::tower::{HooksLayer, LlmHook, LlmService, TracingLayer};
//! use tower::ServiceBuilder;
//!
//! let hook: Arc<dyn LlmHook> = Arc::new(MyAuditHook);
//! let service = ServiceBuilder::new()
//!     .layer(HooksLayer::single(hook))
//!     .service(LlmService::new(client));
//! ```

use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::task::{Context, Poll};
use std::time::Instant;

use futures_util::FutureExt as _;
use tower::Layer;
use tower::Service;

use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};
use crate::observability::usage::{CacheState, UsageEvent, UsageEventOutcome, UsageSinkErased};

// Process-scoped counter used as a fallback request-id when no idempotency
// key is attached. Monotonically increasing; uniqueness across processes is
// not guaranteed — this is a best-effort correlation aid.
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

// ─── Hook Trait ──────────────────────────────────────────────────────────────

/// Callback trait for observing and guarding LLM requests.
///
/// All methods have default no-op implementations, so consumers only need to
/// override the lifecycle points they care about.
#[cfg_attr(alef, alef(skip))]
pub trait LlmHook: Send + Sync + 'static {
    /// Called before the request is sent to the inner service.
    ///
    /// Return `Err` to short-circuit the entire service chain — this enables
    /// guardrail patterns such as content filtering or budget enforcement.
    fn on_request(&self, _req: &LlmRequest) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    /// Called after the inner service returns a successful response.
    fn on_response(&self, _req: &LlmRequest, _resp: &LlmResponse) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async {})
    }

    /// Called when the inner service returns an error.
    fn on_error(&self, _req: &LlmRequest, _err: &LiterLlmError) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async {})
    }
}

// ─── Layer ───────────────────────────────────────────────────────────────────

/// Tower [`Layer`] that attaches [`LlmHook`] callbacks to a service.
///
/// Hooks are stored behind `Arc` so that the layer and all services it
/// produces share the same hook instances without cloning them.
#[derive(Clone)]
#[cfg_attr(alef, alef(skip))]
pub struct HooksLayer {
    hooks: Arc<Vec<Arc<dyn LlmHook>>>,
    usage_sink: Option<Arc<dyn UsageSinkErased>>,
}

impl HooksLayer {
    /// Create a new layer with the given list of hooks.
    ///
    /// Hooks are invoked sequentially in the order they appear in the vector.
    #[must_use]
    pub fn new(hooks: Vec<Arc<dyn LlmHook>>) -> Self {
        Self {
            hooks: Arc::new(hooks),
            usage_sink: None,
        }
    }

    /// Convenience constructor for a single hook.
    #[must_use]
    pub fn single(hook: Arc<dyn LlmHook>) -> Self {
        Self::new(vec![hook])
    }

    /// Attach a [`crate::observability::UsageSink`] that receives one
    /// [`crate::observability::UsageEvent`] per completed request.
    ///
    /// The sink is invoked after all post-hooks complete, on both the success
    /// and error paths. Emit errors are logged and do not propagate.
    #[must_use]
    pub fn with_usage_sink<S: crate::observability::UsageSink>(mut self, sink: Arc<S>) -> Self {
        self.usage_sink = Some(sink as Arc<dyn UsageSinkErased>);
        self
    }
}

impl<S> Layer<S> for HooksLayer {
    type Service = HooksService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HooksService {
            inner,
            hooks: Arc::clone(&self.hooks),
            usage_sink: self.usage_sink.clone(),
        }
    }
}

// ─── Service ─────────────────────────────────────────────────────────────────

/// Tower service produced by [`HooksLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct HooksService<S> {
    inner: S,
    hooks: Arc<Vec<Arc<dyn LlmHook>>>,
    usage_sink: Option<Arc<dyn UsageSinkErased>>,
}

impl<S: Clone> Clone for HooksService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            hooks: Arc::clone(&self.hooks),
            usage_sink: self.usage_sink.clone(),
        }
    }
}

impl<S> Service<LlmRequest> for HooksService<S>
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
        let hooks = Arc::clone(&self.hooks);
        let usage_sink = self.usage_sink.clone();
        // Clone the request so we can pass it to post-hooks after the inner
        // service consumes the original.
        let req_clone = req.clone();
        let fut = self.inner.call(req);

        Box::pin(async move {
            // Pre-hooks: run sequentially; short-circuit on first Err or panic.
            for hook in hooks.iter() {
                let result = AssertUnwindSafe(hook.on_request(&req_clone)).catch_unwind().await;
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => return Err(e),
                    Err(_panic) => {
                        tracing::error!("hook panicked during on_request");
                        return Err(LiterLlmError::HookRejected {
                            message: "hook panicked".into(),
                        });
                    }
                }
            }

            let start = Instant::now();

            // Arm the cancellation guard: if this future is dropped before
            // reaching the normal completion path (e.g. the caller drops the
            // future mid-flight), the guard's `Drop` impl fires a detached
            // `Cancelled` event to the sink.
            let mut cancel_guard = usage_sink
                .as_ref()
                .map(|s| CancellationGuard::new(Arc::clone(s), req_clone.clone(), start));

            match fut.await {
                Ok(resp) => {
                    let latency_ms = start.elapsed().as_millis() as u64;

                    // Post-hooks (success path) — panics are logged but do not
                    // propagate so the caller still receives the response.
                    for hook in hooks.iter() {
                        if AssertUnwindSafe(hook.on_response(&req_clone, &resp))
                            .catch_unwind()
                            .await
                            .is_err()
                        {
                            tracing::error!("hook panicked during on_response");
                        }
                    }

                    // Disarm the cancellation guard before the sink emit so that
                    // `Drop` does not also fire a `Cancelled` event.
                    if let Some(guard) = cancel_guard.take() {
                        guard.disarm();
                    }

                    if let Some(sink) = usage_sink {
                        let event = build_usage_event(&req_clone, &resp, latency_ms, UsageEventOutcome::Success);
                        // Detach the sink call so slow backends don't add to
                        // caller-observed latency.  Errors are logged by the task.
                        tokio::spawn(async move {
                            if let Err(err) = sink.emit_erased(event).await {
                                tracing::warn!(
                                    target: "gen_ai.usage",
                                    error = %err,
                                    "usage sink emit failed"
                                );
                            }
                        });
                    }

                    Ok(resp)
                }
                Err(err) => {
                    let latency_ms = start.elapsed().as_millis() as u64;

                    // Post-hooks (error path) — panics are logged but do not
                    // replace the original error.
                    for hook in hooks.iter() {
                        if AssertUnwindSafe(hook.on_error(&req_clone, &err))
                            .catch_unwind()
                            .await
                            .is_err()
                        {
                            tracing::error!("hook panicked during on_error");
                        }
                    }

                    // Disarm before emitting the real error event.
                    if let Some(guard) = cancel_guard.take() {
                        guard.disarm();
                    }

                    if let Some(sink) = usage_sink {
                        let outcome = classify_error_outcome(&err);
                        let event = build_error_usage_event(&req_clone, latency_ms, outcome);
                        // Detach: slow sink must not add to error-path latency.
                        tokio::spawn(async move {
                            if let Err(sink_err) = sink.emit_erased(event).await {
                                tracing::warn!(
                                    target: "gen_ai.usage",
                                    error = %sink_err,
                                    "usage sink emit failed on error path"
                                );
                            }
                        });
                    }

                    Err(err)
                }
            }
        })
    }
}

// ─── CancellationGuard ───────────────────────────────────────────────────────

/// RAII guard that emits a [`UsageEventOutcome::Cancelled`] event when dropped
/// while still armed.
///
/// The guard is created just before the inner service future is awaited and
/// disarmed immediately before the normal sink emit on both success and error
/// paths.  If the enclosing future is dropped mid-flight (caller cancellation),
/// the guard fires a fire-and-forget `Cancelled` event via [`tokio::spawn`].
struct CancellationGuard {
    /// The sink to emit to, wrapped in an `Option` so `disarm` can take it.
    inner: Option<CancellationGuardInner>,
}

struct CancellationGuardInner {
    sink: Arc<dyn UsageSinkErased>,
    req: LlmRequest,
    start: Instant,
}

impl CancellationGuard {
    fn new(sink: Arc<dyn UsageSinkErased>, req: LlmRequest, start: Instant) -> Self {
        Self {
            inner: Some(CancellationGuardInner { sink, req, start }),
        }
    }

    /// Disarm: drop the guard without firing the cancellation event.
    fn disarm(mut self) {
        self.inner = None;
    }
}

impl Drop for CancellationGuard {
    fn drop(&mut self) {
        let Some(inner) = self.inner.take() else { return };

        let latency_ms = inner.start.elapsed().as_millis() as u64;
        let event = build_error_usage_event(&inner.req, latency_ms, UsageEventOutcome::Cancelled);
        let sink = inner.sink;

        // Fire-and-forget: if a Tokio runtime is active, spawn the emit.
        // If not (e.g. a synchronous test context drops the future), the
        // event is silently discarded — correct behavior since there is no
        // runtime to deliver it to anyway.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Err(err) = sink.emit_erased(event).await {
                    tracing::warn!(
                        target: "gen_ai.usage",
                        error = %err,
                        "usage sink emit failed for cancelled request"
                    );
                }
            });
        }
    }
}

// ─── UsageEvent construction helpers ─────────────────────────────────────────

/// Derive a stable `request_id` from the request's idempotency key,
/// falling back to a process-scoped counter.
fn request_id(req: &LlmRequest) -> String {
    req.idempotency_key
        .clone()
        .unwrap_or_else(|| REQUEST_COUNTER.fetch_add(1, AtomicOrdering::Relaxed).to_string())
}

/// Extract the provider prefix from a model string (the part before `/`).
fn provider_from_model(model: &str) -> String {
    model.split_once('/').map(|(prefix, _)| prefix).unwrap_or("").to_owned()
}

/// Map a `LiterLlmError` to a `UsageEventOutcome`.
fn classify_error_outcome(err: &LiterLlmError) -> UsageEventOutcome {
    match err {
        LiterLlmError::Timeout => UsageEventOutcome::TimedOut,
        _ => UsageEventOutcome::Error,
    }
}

/// Build a `UsageEvent` from a successful response.
fn build_usage_event(req: &LlmRequest, resp: &LlmResponse, latency_ms: u64, outcome: UsageEventOutcome) -> UsageEvent {
    let model = req.model().unwrap_or("").to_owned();
    let provider = provider_from_model(&model);

    let (prompt_tokens, completion_tokens, cached_tokens, total_tokens) = resp
        .usage()
        .map(|u| {
            let cached = u.prompt_tokens_details.as_ref().map_or(0, |d| d.cached_tokens);
            (u.prompt_tokens, u.completion_tokens, cached, u.total_tokens)
        })
        .unwrap_or((0, 0, 0, 0));

    // cost::completion_cost_with_cache returns Option<f64>; convert to Decimal.
    let cost_usd = crate::cost::completion_cost_with_cache(&model, prompt_tokens, cached_tokens, completion_tokens)
        .and_then(|f| rust_decimal::Decimal::try_from(f).ok())
        .unwrap_or(rust_decimal::Decimal::ZERO);

    let finish_reason = match resp {
        LlmResponse::Chat(r) => r
            .choices
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .map(|fr| format!("{fr:?}").to_lowercase()),
        _ => None,
    };

    UsageEvent {
        tenant_id: req.tenant_id.clone(),
        request_id: request_id(req),
        model,
        provider,
        prompt_tokens,
        completion_tokens,
        cached_tokens,
        total_tokens,
        cost_usd,
        // TODO: wire real cache state once the cache layer exposes hit-type
        // metadata through a task-local or response extension.
        cache_state: CacheState::Bypass,
        finish_reason,
        outcome,
        latency_ms,
        metadata: std::collections::HashMap::new(),
        received_at: std::time::SystemTime::now(),
    }
}

/// Build a `UsageEvent` for the error path (no response available).
fn build_error_usage_event(req: &LlmRequest, latency_ms: u64, outcome: UsageEventOutcome) -> UsageEvent {
    let model = req.model().unwrap_or("").to_owned();
    let provider = provider_from_model(&model);

    UsageEvent {
        tenant_id: req.tenant_id.clone(),
        request_id: request_id(req),
        model,
        provider,
        prompt_tokens: 0,
        completion_tokens: 0,
        cached_tokens: 0,
        total_tokens: 0,
        cost_usd: rust_decimal::Decimal::ZERO,
        cache_state: CacheState::Bypass,
        finish_reason: None,
        outcome,
        latency_ms,
        metadata: std::collections::HashMap::new(),
        received_at: std::time::SystemTime::now(),
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tower::Layer as _;
    use tower::Service as _;

    use super::*;
    use crate::observability::usage::{UsageEvent, UsageSinkError};
    use crate::observability::UsageSink;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::{LlmRequest, LlmResponse};

    // ── Shared sink helpers for new tests ────────────────────────────────────

    /// In-test sink that records every received event.
    #[derive(Default)]
    struct VecSink {
        events: Arc<std::sync::Mutex<Vec<UsageEvent>>>,
    }

    impl VecSink {
        #[allow(dead_code)]
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

    /// Sink that waits 500 ms before completing — used to verify non-blocking
    /// behaviour on the response path.
    #[derive(Default)]
    struct SlowSink {
        events: Arc<std::sync::Mutex<Vec<UsageEvent>>>,
    }

    impl UsageSink for SlowSink {
        async fn emit(&self, event: UsageEvent) -> std::result::Result<(), UsageSinkError> {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            self.events.lock().expect("lock poisoned").push(event);
            Ok(())
        }
    }

    // ── Test hook implementations ────────────────────────────────────────────

    /// A hook that records how many times each callback was invoked.
    struct CountingHook {
        on_request_count: AtomicUsize,
        on_response_count: AtomicUsize,
        on_error_count: AtomicUsize,
    }

    impl CountingHook {
        fn new() -> Self {
            Self {
                on_request_count: AtomicUsize::new(0),
                on_response_count: AtomicUsize::new(0),
                on_error_count: AtomicUsize::new(0),
            }
        }
    }

    impl LlmHook for CountingHook {
        fn on_request(&self, _req: &LlmRequest) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
            self.on_request_count.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(()) })
        }

        fn on_response(&self, _req: &LlmRequest, _resp: &LlmResponse) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
            self.on_response_count.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {})
        }

        fn on_error(&self, _req: &LlmRequest, _err: &LiterLlmError) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
            self.on_error_count.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {})
        }
    }

    /// A hook that rejects all requests (guardrail).
    struct RejectAllHook;

    impl LlmHook for RejectAllHook {
        fn on_request(&self, _req: &LlmRequest) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
            Box::pin(async {
                Err(LiterLlmError::HookRejected {
                    message: "rejected by guardrail".into(),
                })
            })
        }
    }

    /// A hook that records its invocation order into a shared vector.
    struct OrderTrackingHook {
        id: usize,
        order: Arc<std::sync::Mutex<Vec<usize>>>,
    }

    impl LlmHook for OrderTrackingHook {
        fn on_request(&self, _req: &LlmRequest) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
            self.order.lock().expect("lock poisoned").push(self.id);
            Box::pin(async { Ok(()) })
        }

        fn on_response(&self, _req: &LlmRequest, _resp: &LlmResponse) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
            self.order.lock().expect("lock poisoned").push(self.id + 100);
            Box::pin(async {})
        }
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn on_request_hook_is_called() {
        let hook = Arc::new(CountingHook::new());
        let inner = LlmService::new(MockClient::ok());
        let mut svc = HooksLayer::single(Arc::clone(&hook) as Arc<dyn LlmHook>).layer(inner);

        let _resp = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("should succeed");

        assert_eq!(hook.on_request_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn on_response_hook_is_called_on_success() {
        let hook = Arc::new(CountingHook::new());
        let inner = LlmService::new(MockClient::ok());
        let mut svc = HooksLayer::single(Arc::clone(&hook) as Arc<dyn LlmHook>).layer(inner);

        let _resp = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("should succeed");

        assert_eq!(hook.on_response_count.load(Ordering::SeqCst), 1);
        assert_eq!(hook.on_error_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn on_error_hook_is_called_on_failure() {
        let hook = Arc::new(CountingHook::new());
        let inner = LlmService::new(MockClient::failing_timeout());
        let mut svc = HooksLayer::single(Arc::clone(&hook) as Arc<dyn LlmHook>).layer(inner);

        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should fail");

        assert!(matches!(err, LiterLlmError::Timeout));
        assert_eq!(hook.on_error_count.load(Ordering::SeqCst), 1);
        assert_eq!(hook.on_response_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn guardrail_rejection_short_circuits_inner_service() {
        let mock = MockClient::ok();
        let call_count = Arc::clone(&mock.call_count);
        let inner = LlmService::new(mock);
        let mut svc = HooksLayer::single(Arc::new(RejectAllHook) as Arc<dyn LlmHook>).layer(inner);

        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should be rejected by guardrail");

        assert!(matches!(err, LiterLlmError::HookRejected { .. }));
        // The inner service must NOT have been called.
        assert_eq!(call_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn multiple_hooks_called_in_registration_order() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));

        let hooks: Vec<Arc<dyn LlmHook>> = vec![
            Arc::new(OrderTrackingHook {
                id: 1,
                order: Arc::clone(&order),
            }),
            Arc::new(OrderTrackingHook {
                id: 2,
                order: Arc::clone(&order),
            }),
            Arc::new(OrderTrackingHook {
                id: 3,
                order: Arc::clone(&order),
            }),
        ];

        let inner = LlmService::new(MockClient::ok());
        let mut svc = HooksLayer::new(hooks).layer(inner);

        let _resp = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("should succeed");

        let recorded = order.lock().expect("lock poisoned").clone();
        // Pre-hooks: 1, 2, 3 then post-hooks: 101, 102, 103
        assert_eq!(recorded, vec![1, 2, 3, 101, 102, 103]);
    }

    /// Verify that a slow sink does not block the caller's response path.
    ///
    /// The `SlowSink` sleeps 500 ms in `emit`; the request must return in
    /// under 100 ms because the sink is spawned detached.
    #[tokio::test]
    async fn sink_does_not_block_response_path() {
        let sink = Arc::new(SlowSink::default());
        let inner = LlmService::new(MockClient::ok());
        let mut svc = HooksLayer::new(vec![]).with_usage_sink(Arc::clone(&sink)).layer(inner);

        let t0 = tokio::time::Instant::now();
        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("should succeed");
        let elapsed = t0.elapsed();

        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "response should return in <100 ms even with a slow sink; got {elapsed:?}"
        );
    }

    /// When the response future is dropped before completion, the
    /// `CancellationGuard` must fire exactly one `Cancelled` event.
    #[tokio::test]
    async fn cancelled_outcome_emitted_when_future_dropped() {
        use std::task::{Context, Poll};

        use tower::Service as _;

        // An inner service that never resolves — its future just parks forever.
        #[derive(Clone)]
        struct PendingService;

        impl tower::Service<LlmRequest> for PendingService {
            type Response = LlmResponse;
            type Error = LiterLlmError;
            type Future = BoxFuture<'static, Result<LlmResponse>>;

            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
                Poll::Ready(Ok(()))
            }

            fn call(&mut self, _req: LlmRequest) -> Self::Future {
                Box::pin(std::future::pending())
            }
        }

        let sink = Arc::new(VecSink::default());
        let events_ref = Arc::clone(&sink.events);

        let mut svc = HooksLayer::new(vec![]).with_usage_sink(sink).layer(PendingService);

        // Spawn the future so we can abort it to simulate cancellation.
        let handle = tokio::spawn(async move {
            svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await
        });

        // Give the future a tick to start (so the CancellationGuard is armed).
        tokio::task::yield_now().await;

        // Drop / abort the future before it completes.
        handle.abort();
        // Wait for the abort to complete and the guard's Drop to fire.
        let _ = handle.await;

        // Give the spawned emit task a moment to complete.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let collected = events_ref.lock().expect("lock poisoned").clone();
        assert_eq!(collected.len(), 1, "exactly one Cancelled event must be emitted");
        assert_eq!(
            collected[0].outcome,
            crate::observability::UsageEventOutcome::Cancelled,
            "outcome must be Cancelled"
        );
    }
}
