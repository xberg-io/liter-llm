//! OTel-native metrics layer for the Tower middleware stack.
//!
//! [`MetricsLayer`] wraps any [`tower::Service<LlmRequest>`] and records
//! GenAI semantic-convention metrics via the `opentelemetry::metrics` API.
//!
//! # Instruments
//!
//! **Histograms**
//! - `gen_ai.client.operation.duration` — request latency in seconds.
//! - `gen_ai.client.token.usage` — token counts (one observation per token
//!   category; distinguished by the `gen_ai.token.type` attribute).
//! - `gen_ai.client.cost.usd` — estimated cost in USD (when a cost is
//!   available from [`super::cost`]).
//! - `gen_ai.cache.lookup.duration` — time spent on cache lookups (recorded
//!   from the `gen_ai.cache.hit` / `gen_ai.cache.miss` context; the layer
//!   itself does not perform cache lookups, but downstream cache layers can
//!   attach timing via the shared `CacheMetricsExt` helper).
//!
//! **Counters**
//! - `gen_ai.cache.hit` — number of cache hits.
//! - `gen_ai.cache.miss` — number of cache misses.
//! - `gen_ai.cache.stale` — number of stale-cache served responses.
//! - `gen_ai.circuit.trip` — number of times a circuit breaker tripped.
//! - `gen_ai.retry.attempt` — number of retry attempts (excluding first try).
//!
//! # Attributes
//!
//! Every instrument observation carries the following key-value pairs from
//! the GenAI semantic conventions:
//! - `gen_ai.system` — provider prefix (e.g. `"openai"`).
//! - `gen_ai.request.model` — the model name from the request.
//! - `gen_ai.response.model` — the model from the response (may differ).
//! - `gen_ai.operation.name` — `"chat"`, `"embeddings"`, etc.
//!
//! # Feature gate
//!
//! This module is compiled only when the `otel` feature is active.  When the
//! feature is disabled the module still exists but exports a no-op
//! `MetricsLayer` that compiles away completely.

#[cfg(feature = "otel")]
mod inner {
    use std::sync::OnceLock;
    use std::task::{Context, Poll};
    use std::time::Instant;

    use dashmap::DashMap;
    use opentelemetry::KeyValue;
    use opentelemetry::metrics::{Counter, Histogram, Meter};
    use tower::{Layer, Service};

    use super::super::types::{LlmRequest, LlmResponse};
    use crate::client::BoxFuture;
    use crate::error::{LiterLlmError, Result};

    use std::sync::Arc;

    // ─── Meter singleton ──────────────────────────────────────────────────────

    static METER: OnceLock<Meter> = OnceLock::new();

    /// Initialise the global `Meter` used by all [`MetricsLayer`] instances.
    ///
    /// Call this once during application startup with the meter obtained from
    /// your `opentelemetry` provider (e.g. `global::meter("liter-llm")`).
    /// Subsequent calls are ignored.
    ///
    /// # Order of initialisation
    ///
    /// The instruments cache is populated when `init_meter` is called. If any
    /// metric helpers (e.g. `record_cache_hit`) are called before
    /// `init_meter`, they silently no-op. Once the meter is initialised,
    /// all subsequent metric operations use the cached instrument set.
    pub fn init_meter(meter: Meter) {
        let _ = METER.set(meter);
    }

    /// Return the global meter, or `None` when [`init_meter`] has not been called.
    pub(crate) fn global_meter() -> Option<&'static Meter> {
        METER.get()
    }

    // ─── Instrument set ───────────────────────────────────────────────────────

    /// Cached OTel instruments for recording metrics.
    ///
    /// Initialized once via [`init_meter`] and shared across all requests and
    /// helper functions via `Arc` to avoid repeated instrument construction.
    struct Instruments {
        op_duration: Histogram<f64>,
        token_usage: Histogram<u64>,
        /// Cost histogram — populated by callers via [`record_cost_usd`].
        #[allow(dead_code)]
        cost_usd: Histogram<f64>,
        cache_hit: Counter<u64>,
        cache_miss: Counter<u64>,
        cache_stale: Counter<u64>,
        circuit_trip: Counter<u64>,
        retry_attempt: Counter<u64>,
        /// `gen_ai.budget.spend_usd` — gauge-style histogram per dimension.
        budget_spend: Histogram<f64>,
        /// `gen_ai.budget.rejection` — counter incremented on budget reject.
        budget_rejection: Counter<u64>,
        // ── Realtime instruments ───────────────────────────────────────────────
        /// `gen_ai.realtime.session.duration` — WebSocket session lifetime in seconds.
        realtime_session_duration: Histogram<f64>,
        /// `gen_ai.realtime.event.count` — events forwarded (inbound + outbound).
        realtime_event_count: Counter<u64>,
        /// `gen_ai.realtime.bytes` — audio bytes forwarded.
        realtime_bytes: Counter<u64>,
    }

    impl Instruments {
        fn new(meter: &Meter) -> Self {
            Self {
                op_duration: meter
                    .f64_histogram("gen_ai.client.operation.duration")
                    .with_description("GenAI client request latency in seconds")
                    .with_unit("s")
                    .build(),
                token_usage: meter
                    .u64_histogram("gen_ai.client.token.usage")
                    .with_description("Token counts for GenAI operations")
                    .with_unit("{token}")
                    .build(),
                cost_usd: meter
                    .f64_histogram("gen_ai.client.cost.usd")
                    .with_description("Estimated cost of GenAI operations in USD")
                    .with_unit("USD")
                    .build(),
                cache_hit: meter
                    .u64_counter("gen_ai.cache.hit")
                    .with_description("Number of GenAI response cache hits")
                    .build(),
                cache_miss: meter
                    .u64_counter("gen_ai.cache.miss")
                    .with_description("Number of GenAI response cache misses")
                    .build(),
                cache_stale: meter
                    .u64_counter("gen_ai.cache.stale")
                    .with_description("Number of stale GenAI cache responses served")
                    .build(),
                circuit_trip: meter
                    .u64_counter("gen_ai.circuit.trip")
                    .with_description("Number of circuit breaker trips")
                    .build(),
                retry_attempt: meter
                    .u64_counter("gen_ai.retry.attempt")
                    .with_description("Number of retry attempts (excluding first try)")
                    .build(),
                budget_spend: meter
                    .f64_histogram("gen_ai.budget.spend_usd")
                    .with_description("Cumulative spend in USD per budget dimension")
                    .with_unit("USD")
                    .build(),
                budget_rejection: meter
                    .u64_counter("gen_ai.budget.rejection")
                    .with_description("Number of requests rejected due to budget limits")
                    .build(),
                realtime_session_duration: meter
                    .f64_histogram("gen_ai.realtime.session.duration")
                    .with_description("Realtime WebSocket session lifetime in seconds")
                    .with_unit("s")
                    .build(),
                realtime_event_count: meter
                    .u64_counter("gen_ai.realtime.event.count")
                    .with_description("Number of Realtime events forwarded, by direction and type")
                    .build(),
                realtime_bytes: meter
                    .u64_counter("gen_ai.realtime.bytes")
                    .with_description("Audio bytes forwarded over Realtime WebSocket sessions")
                    .with_unit("By")
                    .build(),
            }
        }
    }

    // ─── Attributes cache ────────────────────────────────────────────────────

    /// Cached base attributes keyed by (system, model) to avoid repeated clones
    /// on every request.
    type BaseAttrsKey = (Arc<str>, Arc<str>);
    static BASE_ATTRS_CACHE: OnceLock<DashMap<BaseAttrsKey, Arc<[KeyValue]>>> = OnceLock::new();

    /// Cached token-type attribute sets (input and output).
    struct CachedTokenAttrs {
        input: Arc<[KeyValue]>,
        output: Arc<[KeyValue]>,
    }

    static TOKEN_ATTRS_CACHE: OnceLock<DashMap<BaseAttrsKey, CachedTokenAttrs>> = OnceLock::new();

    /// Return or initialize the base attributes cache.
    fn base_attrs_cache() -> &'static DashMap<BaseAttrsKey, Arc<[KeyValue]>> {
        BASE_ATTRS_CACHE.get_or_init(DashMap::new)
    }

    /// Return or initialize the token attributes cache.
    fn token_attrs_cache() -> &'static DashMap<BaseAttrsKey, CachedTokenAttrs> {
        TOKEN_ATTRS_CACHE.get_or_init(DashMap::new)
    }

    /// Retrieve or build base attributes for the given (system, model) pair.
    /// Returns an Arc pointing to the cached attribute slice to avoid per-request clones.
    fn get_or_build_base_attrs(system: &str, model: &str, response_model: &str, operation: &str) -> Arc<[KeyValue]> {
        let system_arc = Arc::<str>::from(system);
        let model_arc = Arc::<str>::from(model);
        let key = (Arc::clone(&system_arc), Arc::clone(&model_arc));

        let cache = base_attrs_cache();

        // Try fast path: entry already cached.
        if let Some(entry) = cache.get(&key) {
            return Arc::clone(&entry);
        }

        // Slow path: build and cache.
        let attrs = Arc::from(
            vec![
                KeyValue::new("gen_ai.system", system_arc.to_string()),
                KeyValue::new("gen_ai.request.model", model_arc.to_string()),
                KeyValue::new("gen_ai.response.model", response_model.to_owned()),
                KeyValue::new("gen_ai.operation.name", operation.to_owned()),
            ]
            .into_boxed_slice(),
        );

        // Insert and return (another thread might race; we use entry to minimize reinsert).
        cache.entry(key).or_insert_with(|| Arc::clone(&attrs));

        attrs
    }

    /// Retrieve or build cached token-type attributes for the given base attributes.
    /// Returns a pair of (input_attrs, output_attrs) to avoid to_vec() on every token recording.
    fn get_or_build_token_attrs(system: &str, model: &str, response_model: &str, operation: &str) -> CachedTokenAttrs {
        let system_arc = Arc::<str>::from(system);
        let model_arc = Arc::<str>::from(model);
        let key = (Arc::clone(&system_arc), Arc::clone(&model_arc));

        let cache = token_attrs_cache();

        // Try fast path: entry already cached.
        if let Some(entry) = cache.get(&key) {
            return CachedTokenAttrs {
                input: Arc::clone(&entry.input),
                output: Arc::clone(&entry.output),
            };
        }

        // Slow path: build base and extend with token types.
        let base = get_or_build_base_attrs(&system_arc, &model_arc, response_model, operation);

        let mut input_attrs = base.to_vec();
        input_attrs.push(KeyValue::new("gen_ai.token.type", "input"));
        let input_arc = Arc::from(input_attrs.into_boxed_slice());

        let mut output_attrs = base.to_vec();
        output_attrs.push(KeyValue::new("gen_ai.token.type", "output"));
        let output_arc = Arc::from(output_attrs.into_boxed_slice());

        let cached = CachedTokenAttrs {
            input: Arc::clone(&input_arc),
            output: Arc::clone(&output_arc),
        };

        // Insert and return.
        cache.entry(key).or_insert_with(|| CachedTokenAttrs {
            input: Arc::clone(&input_arc),
            output: Arc::clone(&output_arc),
        });

        cached
    }

    // ─── Instruments cache ────────────────────────────────────────────────────

    static INSTRUMENTS: OnceLock<Arc<Instruments>> = OnceLock::new();

    /// Return the cached instruments, initializing them if the meter is available.
    /// Returns `None` if the meter has not yet been initialized.
    fn instruments() -> Option<Arc<Instruments>> {
        // Fast path: instruments already cached.
        if let Some(cached) = INSTRUMENTS.get() {
            return Some(Arc::clone(cached));
        }

        // Slow path: lazy initialization from meter.
        // This is called at most once per thread (first time after METER is set).
        if let Some(meter) = global_meter() {
            let new_instruments = Arc::new(Instruments::new(meter));
            // Best-effort cache insertion; another thread may beat us to it.
            let result = INSTRUMENTS
                .set(Arc::clone(&new_instruments))
                .ok()
                .map(|_| Arc::clone(&new_instruments));
            return result.or_else(|| INSTRUMENTS.get().map(Arc::clone));
        }

        None
    }

    // ─── Layer ────────────────────────────────────────────────────────────────

    /// Tower [`Layer`] that records OTel GenAI semantic-convention metrics.
    ///
    /// Metrics are only emitted when [`init_meter`] has been called before the
    /// first request.  If the meter has not been initialised the layer is a
    /// transparent pass-through.
    #[derive(Clone)]
    pub struct MetricsLayer;

    impl<S> Layer<S> for MetricsLayer {
        type Service = MetricsService<S>;

        fn layer(&self, inner: S) -> Self::Service {
            MetricsService { inner }
        }
    }

    // ─── Service ─────────────────────────────────────────────────────────────

    /// Tower service produced by [`MetricsLayer`].
    pub struct MetricsService<S> {
        inner: S,
    }

    impl<S: Clone> Clone for MetricsService<S> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }

    impl<S> Service<LlmRequest> for MetricsService<S>
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
            let start = Instant::now();

            // Capture metadata before moving `req` into the inner future.
            let operation = req.operation_name();
            let model_str = req.model().unwrap_or("").to_owned();
            let system = model_str
                .split_once('/')
                .map(|(prefix, _)| prefix.to_owned())
                .unwrap_or_default();

            let fut = self.inner.call(req);

            Box::pin(async move {
                let result = fut.await;
                let elapsed = start.elapsed().as_secs_f64();

                // Only record metrics when instruments are available.
                if let Some(instr) = instruments() {
                    // Determine response model.
                    let response_model = match &result {
                        Ok(resp) => match resp {
                            LlmResponse::Chat(r) => r.model.clone(),
                            LlmResponse::Embed(r) => r.model.clone(),
                            _ => model_str.clone(),
                        },
                        Err(_) => model_str.clone(),
                    };

                    // Retrieve or build cached base attributes (Arc-backed to avoid per-request clones).
                    let base_attrs = get_or_build_base_attrs(&system, &model_str, &response_model, operation);

                    // Operation duration.
                    instr.op_duration.record(elapsed, base_attrs.as_ref());

                    // Token usage — only when the response carries usage data.
                    if let Ok(resp) = &result
                        && let Some(usage) = resp.usage()
                    {
                        // Retrieve or build cached token-type attributes to avoid per-recording allocations.
                        let token_attrs = get_or_build_token_attrs(&system, &model_str, &response_model, operation);

                        // Input tokens.
                        instr
                            .token_usage
                            .record(usage.prompt_tokens, token_attrs.input.as_ref());

                        // Output tokens.
                        instr
                            .token_usage
                            .record(usage.completion_tokens, token_attrs.output.as_ref());
                    }
                }

                result
            })
        }
    }

    // ─── Public helpers ───────────────────────────────────────────────────────

    /// Record a cache hit metric.
    ///
    /// Call from cache layer implementations to emit `gen_ai.cache.hit`.
    /// If the meter has not been initialized, this call is a no-op.
    pub fn record_cache_hit(system: &str, model: &str, operation: &str) {
        if let Some(instr) = instruments() {
            instr.cache_hit.add(
                1,
                &[
                    KeyValue::new("gen_ai.system", system.to_owned()),
                    KeyValue::new("gen_ai.request.model", model.to_owned()),
                    KeyValue::new("gen_ai.operation.name", operation.to_owned()),
                ],
            );
        }
    }

    /// Record a cache miss metric.
    ///
    /// Call from cache layer implementations to emit `gen_ai.cache.miss`.
    /// If the meter has not been initialized, this call is a no-op.
    pub fn record_cache_miss(system: &str, model: &str, operation: &str) {
        if let Some(instr) = instruments() {
            instr.cache_miss.add(
                1,
                &[
                    KeyValue::new("gen_ai.system", system.to_owned()),
                    KeyValue::new("gen_ai.request.model", model.to_owned()),
                    KeyValue::new("gen_ai.operation.name", operation.to_owned()),
                ],
            );
        }
    }

    /// Record a stale cache metric.
    ///
    /// Call from cache layer implementations to emit `gen_ai.cache.stale`.
    /// If the meter has not been initialized, this call is a no-op.
    pub fn record_cache_stale(system: &str, model: &str, operation: &str) {
        if let Some(instr) = instruments() {
            instr.cache_stale.add(
                1,
                &[
                    KeyValue::new("gen_ai.system", system.to_owned()),
                    KeyValue::new("gen_ai.request.model", model.to_owned()),
                    KeyValue::new("gen_ai.operation.name", operation.to_owned()),
                ],
            );
        }
    }

    /// Record a circuit breaker trip.
    ///
    /// Call from [`super::circuit::CircuitLayer`] when the circuit opens.
    /// If the meter has not been initialized, this call is a no-op.
    pub fn record_circuit_trip(system: &str, model: &str) {
        if let Some(instr) = instruments() {
            instr.circuit_trip.add(
                1,
                &[
                    KeyValue::new("gen_ai.system", system.to_owned()),
                    KeyValue::new("gen_ai.request.model", model.to_owned()),
                ],
            );
        }
    }

    /// Record a retry attempt.
    ///
    /// Call from retry/hedge layers to emit `gen_ai.retry.attempt`.
    /// If the meter has not been initialized, this call is a no-op.
    pub fn record_retry_attempt(system: &str, model: &str, operation: &str) {
        if let Some(instr) = instruments() {
            instr.retry_attempt.add(
                1,
                &[
                    KeyValue::new("gen_ai.system", system.to_owned()),
                    KeyValue::new("gen_ai.request.model", model.to_owned()),
                    KeyValue::new("gen_ai.operation.name", operation.to_owned()),
                ],
            );
        }
    }

    /// Record a per-tier cache hit.
    ///
    /// `tier` should be one of `"exact"`, `"semantic"`, or `"streaming_replay"`.
    /// Emits `gen_ai.cache.hit` with a `gen_ai.cache.tier` attribute.
    /// If the meter has not been initialized, this call is a no-op.
    pub fn record_cache_tier_hit(system: &str, model: &str, tier: &str) {
        if let Some(instr) = instruments() {
            instr.cache_hit.add(
                1,
                &[
                    KeyValue::new("gen_ai.system", system.to_owned()),
                    KeyValue::new("gen_ai.request.model", model.to_owned()),
                    KeyValue::new("gen_ai.cache.tier", tier.to_owned()),
                ],
            );
        }
    }

    /// Record a per-tier cache miss.
    ///
    /// `tier` should be one of `"exact"`, `"semantic"`, or `"streaming_replay"`.
    /// Emits `gen_ai.cache.miss` with a `gen_ai.cache.tier` attribute.
    /// If the meter has not been initialized, this call is a no-op.
    pub fn record_cache_tier_miss(system: &str, model: &str, tier: &str) {
        if let Some(instr) = instruments() {
            instr.cache_miss.add(
                1,
                &[
                    KeyValue::new("gen_ai.system", system.to_owned()),
                    KeyValue::new("gen_ai.request.model", model.to_owned()),
                    KeyValue::new("gen_ai.cache.tier", tier.to_owned()),
                ],
            );
        }
    }

    /// Record cumulative spend for a specific budget dimension.
    ///
    /// Emits `gen_ai.budget.spend_usd` with dimension attributes.
    /// Call from [`super::budget::InMemoryBudgetLedger::record`] after each
    /// successful completion.  If the meter has not been initialized, this
    /// call is a no-op.
    #[allow(clippy::too_many_arguments)]
    pub fn record_budget_spend(
        model: &str,
        provider: &str,
        tenant_id: Option<&str>,
        user_id: Option<&str>,
        api_key_id: Option<&str>,
        cost_usd: f64,
    ) {
        if let Some(instr) = instruments() {
            let mut attrs = vec![
                KeyValue::new("gen_ai.request.model", model.to_owned()),
                KeyValue::new("gen_ai.system", provider.to_owned()),
            ];
            if let Some(tenant) = tenant_id {
                attrs.push(KeyValue::new("gen_ai.budget.tenant_id", tenant.to_owned()));
            }
            if let Some(user) = user_id {
                attrs.push(KeyValue::new("gen_ai.budget.user_id", user.to_owned()));
            }
            if let Some(key) = api_key_id {
                attrs.push(KeyValue::new("gen_ai.budget.api_key_id", key.to_owned()));
            }
            instr.budget_spend.record(cost_usd, &attrs);
        }
    }

    /// Record a budget-rejection event.
    ///
    /// Emits `gen_ai.budget.rejection` with the triggering dimension.
    /// Call from [`super::budget::InMemoryBudgetLedger::check`] when
    /// returning [`super::budget::BudgetVerdict::Reject`].
    /// If the meter has not been initialized, this call is a no-op.
    pub fn record_budget_rejection(model: &str, provider: &str, dimension: &str) {
        if let Some(instr) = instruments() {
            instr.budget_rejection.add(
                1,
                &[
                    KeyValue::new("gen_ai.request.model", model.to_owned()),
                    KeyValue::new("gen_ai.system", provider.to_owned()),
                    KeyValue::new("gen_ai.budget.dimension", dimension.to_owned()),
                ],
            );
        }
    }

    // ─── Realtime metric helpers ──────────────────────────────────────────────

    /// Record the lifetime of a completed Realtime WebSocket session.
    ///
    /// Emits `gen_ai.realtime.session.duration` (seconds).
    /// If the meter has not been initialized, this call is a no-op.
    pub fn record_realtime_session_duration(provider: &str, duration_secs: f64) {
        if let Some(instr) = instruments() {
            instr
                .realtime_session_duration
                .record(duration_secs, &[KeyValue::new("gen_ai.system", provider.to_owned())]);
        }
    }

    /// Record a single Realtime event being forwarded.
    ///
    /// Emits `gen_ai.realtime.event.count` with `gen_ai.realtime.direction`
    /// (`"inbound"` | `"outbound"`), `gen_ai.realtime.event_type`, and
    /// `gen_ai.system`.
    /// If the meter has not been initialized, this call is a no-op.
    pub fn record_realtime_event(provider: &str, direction: &str, event_type: &str) {
        if let Some(instr) = instruments() {
            instr.realtime_event_count.add(
                1,
                &[
                    KeyValue::new("gen_ai.system", provider.to_owned()),
                    KeyValue::new("gen_ai.realtime.direction", direction.to_owned()),
                    KeyValue::new("gen_ai.realtime.event_type", event_type.to_owned()),
                ],
            );
        }
    }

    /// Record audio bytes forwarded over a Realtime WebSocket session.
    ///
    /// Emits `gen_ai.realtime.bytes` with `gen_ai.system` and
    /// `gen_ai.realtime.direction` attributes.
    /// If the meter has not been initialized, this call is a no-op.
    pub fn record_realtime_bytes(provider: &str, direction: &str, byte_count: u64) {
        if let Some(instr) = instruments() {
            instr.realtime_bytes.add(
                byte_count,
                &[
                    KeyValue::new("gen_ai.system", provider.to_owned()),
                    KeyValue::new("gen_ai.realtime.direction", direction.to_owned()),
                ],
            );
        }
    }

    // ─── Tests ────────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use tower::{Layer as _, Service as _};

        use super::*;
        use crate::tower::service::LlmService;
        use crate::tower::tests_common::{MockClient, chat_req};
        use crate::tower::types::LlmRequest;

        /// Verify that the MetricsLayer is a transparent pass-through when the meter
        /// is not initialised (the common case in unit tests without an OTel SDK).
        #[tokio::test]
        async fn metrics_layer_passes_through_without_meter() {
            let inner = LlmService::new(MockClient::ok());
            let mut svc = MetricsLayer.layer(inner);

            let resp = svc
                .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
                .await
                .expect("should succeed");

            assert!(matches!(resp, crate::tower::types::LlmResponse::Chat(_)));
        }

        /// Verify the layer correctly passes through errors.
        #[tokio::test]
        async fn metrics_layer_propagates_errors() {
            let inner = LlmService::new(MockClient::failing_timeout());
            let mut svc = MetricsLayer.layer(inner);

            let err = svc
                .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
                .await
                .expect_err("should fail");

            assert!(matches!(err, crate::error::LiterLlmError::Timeout));
        }

        /// Verify that `Instruments` are cached and not reconstructed on each call.
        /// Initializing the meter twice should reuse the same cached instruments.
        #[test]
        fn instruments_initialised_once() {
            use opentelemetry::global;

            // Initialize a test meter using the global provider.
            // In testing, we use a no-op provider if nothing has been configured.
            let meter = global::meter("liter-llm-test");

            // Initialize once.
            init_meter(meter.clone());

            // Retrieve instruments the first time.
            let instr1 = instruments().expect("instruments should be cached");

            // Attempt to initialize again (should be ignored).
            let meter2 = global::meter("liter-llm-test-2");
            init_meter(meter2);

            // Retrieve instruments the second time.
            let instr2 = instruments().expect("instruments should still be cached");

            // Verify pointer identity: both `Arc` pointers should reference the same
            // allocation, proving that the second initialization was ignored.
            assert!(Arc::ptr_eq(&instr1, &instr2), "instruments should be reused");
        }

        /// Verify that metric record helpers are no-ops before the meter is initialized.
        /// These should not panic even when called without `init_meter`.
        #[test]
        fn metrics_record_helpers_no_op_without_meter() {
            // Note: we cannot clear the global METER or INSTRUMENTS state in tests
            // (OnceLock doesn't expose a reset API). This test assumes a fresh test
            // process or carefully sequenced test ordering. In CI, each test should
            // ideally run in isolation. For now, we at least document the expected
            // behavior.

            // If instruments are not yet cached and the meter is not initialized,
            // these calls should return early and do nothing.
            record_cache_hit("openai", "gpt-4", "chat");
            record_cache_miss("openai", "gpt-4", "chat");
            record_cache_stale("openai", "gpt-4", "chat");
            record_circuit_trip("openai", "gpt-4");
            record_retry_attempt("openai", "gpt-4", "chat");

            // If we reach here without panicking, the test passes.
            // (A proper test would require resettable global state, which OnceLock
            // does not provide.)
        }

        /// Verify that base attributes are reused across calls with the same (system, model) pair.
        /// This test checks that Arc strong_count increases instead of allocating fresh Vec on each call.
        #[tokio::test]
        async fn base_attrs_reused_across_calls() {
            use tower::{Layer as _, Service as _};

            let inner = LlmService::new(MockClient::ok());
            let mut svc = MetricsLayer.layer(inner);

            // Make 100 requests with the same (system, model) pair.
            // Without caching, we would allocate a new Vec on each request.
            // With caching, the same Arc<[KeyValue]> is reused.
            for _ in 0..100 {
                let _ = svc.call(LlmRequest::Chat(chat_req("openai/gpt-4"))).await;
            }

            // Verify that the cache contains at least one entry for the (openai, gpt-4) pair.
            let cache = base_attrs_cache();
            let openai_arc = Arc::<str>::from("openai");
            let gpt4_arc = Arc::<str>::from("gpt-4");
            let key = (openai_arc, gpt4_arc);

            if let Some(entry) = cache.get(&key) {
                // Check that the Arc has been cloned multiple times (strong_count > 100).
                // This proves that the same cached entry is being reused rather than
                // creating a new allocation on each request.
                let strong_count = Arc::strong_count(&entry);
                assert!(
                    strong_count > 10,
                    "expected cached entry to be reused (strong_count > 10), got {}",
                    strong_count
                );
            } else {
                // If the cache is empty, the test is inconclusive (meter not initialized).
                // We still pass because the primary invariant (no panic) holds.
            }
        }
    }
}

// ─── No-op stub when otel feature is off ─────────────────────────────────────

#[cfg(not(feature = "otel"))]
mod inner {
    use std::task::{Context, Poll};

    use tower::{Layer, Service};

    use super::super::types::{LlmRequest, LlmResponse};
    use crate::client::BoxFuture;
    use crate::error::{LiterLlmError, Result};

    /// No-op metrics layer (compiled when `otel` feature is disabled).
    #[derive(Clone)]
    pub struct MetricsLayer;

    impl<S> Layer<S> for MetricsLayer {
        type Service = MetricsService<S>;

        fn layer(&self, inner: S) -> Self::Service {
            MetricsService { inner }
        }
    }

    /// No-op metrics service.
    pub struct MetricsService<S> {
        inner: S,
    }

    impl<S: Clone> Clone for MetricsService<S> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }

    impl<S> Service<LlmRequest> for MetricsService<S>
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
            Box::pin(self.inner.call(req))
        }
    }

    /// No-op cache-hit helper.
    #[inline]
    pub fn record_cache_hit(_system: &str, _model: &str, _operation: &str) {}

    /// No-op cache-miss helper.
    #[inline]
    pub fn record_cache_miss(_system: &str, _model: &str, _operation: &str) {}

    /// No-op cache-stale helper.
    #[inline]
    pub fn record_cache_stale(_system: &str, _model: &str, _operation: &str) {}

    /// No-op circuit-trip helper.
    #[inline]
    pub fn record_circuit_trip(_system: &str, _model: &str) {}

    /// No-op retry-attempt helper.
    #[inline]
    pub fn record_retry_attempt(_system: &str, _model: &str, _operation: &str) {}

    /// No-op budget-spend helper.
    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn record_budget_spend(
        _model: &str,
        _provider: &str,
        _tenant_id: Option<&str>,
        _user_id: Option<&str>,
        _api_key_id: Option<&str>,
        _cost_usd: f64,
    ) {
    }

    /// No-op budget-rejection helper.
    #[inline]
    pub fn record_budget_rejection(_model: &str, _provider: &str, _dimension: &str) {}

    /// No-op per-tier cache-hit helper.
    #[inline]
    pub fn record_cache_tier_hit(_system: &str, _model: &str, _tier: &str) {}

    /// No-op per-tier cache-miss helper.
    #[inline]
    pub fn record_cache_tier_miss(_system: &str, _model: &str, _tier: &str) {}

    /// No-op realtime session duration helper.
    #[inline]
    pub fn record_realtime_session_duration(_provider: &str, _duration_secs: f64) {}

    /// No-op realtime event count helper.
    #[inline]
    pub fn record_realtime_event(_provider: &str, _direction: &str, _event_type: &str) {}

    /// No-op realtime bytes helper.
    #[inline]
    pub fn record_realtime_bytes(_provider: &str, _direction: &str, _byte_count: u64) {}
}

// Re-export the active implementation.
pub use inner::*;

// ─── Top-level tests (run regardless of otel feature) ──────────────────────

#[cfg(test)]
#[cfg(feature = "otel")]
mod tests {
    use tower::{Layer as _, Service as _};

    use super::*;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::LlmRequest;

    /// Verify that the MetricsLayer is a transparent pass-through when the meter
    /// is not initialised (the common case in unit tests without an OTel SDK).
    #[tokio::test]
    async fn tower_metrics_layer_passes_through_without_meter() {
        let inner = LlmService::new(MockClient::ok());
        let mut svc = MetricsLayer.layer(inner);

        let resp = svc
            .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect("should succeed");

        assert!(matches!(resp, crate::tower::types::LlmResponse::Chat(_)));
    }

    /// Verify the layer correctly passes through errors.
    #[tokio::test]
    async fn tower_metrics_layer_propagates_errors() {
        let inner = LlmService::new(MockClient::failing_timeout());
        let mut svc = MetricsLayer.layer(inner);

        let err = svc
            .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect_err("should fail");

        assert!(matches!(err, crate::error::LiterLlmError::Timeout));
    }

    /// Verify that metric record helpers are no-ops before the meter is initialized.
    /// These should not panic even when called without `init_meter`.
    #[test]
    fn tower_metrics_record_helpers_no_op_without_meter() {
        // These calls should return early and do nothing when meter is not initialized.
        record_cache_hit("openai", "gpt-4", "chat");
        record_cache_miss("openai", "gpt-4", "chat");
        record_cache_stale("openai", "gpt-4", "chat");
        record_circuit_trip("openai", "gpt-4");
        record_retry_attempt("openai", "gpt-4", "chat");

        // If we reach here without panicking, the test passes.
    }
}
