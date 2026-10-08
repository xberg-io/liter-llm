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
//! # Wiring this layer in
//!
//! Two separate steps are required before any metric declared here reaches an
//! OTel backend; skipping either leaves the whole surface permanently inert:
//!
//! 1. **Install the meter.** Call [`init_meter`] once at application startup
//!    with a `Meter` obtained from your configured `MeterProvider` (e.g.
//!    `global::meter("liter-llm")`). This mirrors the crate's `tracing`
//!    convention — the library only emits, the embedding application installs
//!    the provider — so `MetricsLayer` deliberately does not call
//!    `global::meter` itself. Until `init_meter` runs, every recorder in this
//!    module, and [`MetricsService`] itself, is a silent no-op.
//! 2. **Add the layer to the request pipeline.** Unlike step 1, this is not
//!    an OTel-wide convention: `MetricsLayer` is a regular `tower::Layer` and,
//!    like every other layer in this module, is only applied where a caller
//!    explicitly adds it — `LlmService` and the other layers here never wrap
//!    it in automatically. Add `.layer(MetricsLayer)` wherever the
//!    application assembles its `tower::ServiceBuilder` stack, typically
//!    alongside [`super::tracing::TracingLayer`].
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

    // ~keep This module's Meter/Instruments/attribute-cache statics are process-global
    // ~keep state, which the repo's no-global-state convention otherwise forbids. True
    // ~keep dependency injection would mean every Tower layer that records a metric
    // ~keep (CacheLayer, CircuitLayer, the budget ledger, hedge, realtime sessions, ...)
    // ~keep carries an `Arc<Instruments>` handle and every one of their public
    // ~keep constructors accepts (or is given a setter for) it — a breaking change to
    // ~keep the public API of roughly a dozen types across this crate. This mirrors how
    // ~keep `opentelemetry::global` itself works (a single process-wide meter
    // ~keep provider), so it is left as-is here rather than half-migrated.
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
    #[cfg_attr(alef, alef(skip))]
    pub fn init_meter(meter: Meter) {
        let _ = METER.set(meter);
    }

    /// Return the global meter, or `None` when [`init_meter`] has not been called.
    /// Exposed to downstream crates (e.g. `liter-llm-proxy`) so they can record
    /// metrics against the same shared meter without re-initialising OTel.
    ///
    /// Hidden from alef extraction — the `opentelemetry::metrics::Meter` return
    /// type does not bridge to any host language; this is an internal Rust API.
    #[cfg_attr(alef, alef(skip))]
    pub fn global_meter() -> Option<&'static Meter> {
        METER.get()
    }

    /// Cached OTel instruments for recording metrics.
    ///
    /// Initialized once via [`init_meter`] and shared across all requests and
    /// helper functions via `Arc` to avoid repeated instrument construction.
    struct Instruments {
        op_duration: Histogram<f64>,
        token_usage: Histogram<u64>,
        /// Cost histogram — populated by [`record_cost_usd`], called from
        /// [`crate::tower::cost::CostTrackingService`] once a completion's cost is known.
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

    /// Upper bound on distinct (system, model) label pairs cached by
    /// [`BASE_ATTRS_CACHE`] and [`TOKEN_ATTRS_CACHE`].
    ///
    /// ~keep Without a cap, a long-running proxy that sees unbounded label
    /// ~keep cardinality (e.g. per-tenant or freeform model strings from
    /// ~keep untrusted callers) grows these caches forever — an unbounded
    /// ~keep memory leak. `DashMap` has no built-in LRU, so the eviction
    /// ~keep policy on hitting the cap is a full clear: crude, but O(1) and
    /// ~keep correct, and cache misses only cost a few `KeyValue` clones.
    const MAX_METRICS_ATTR_CACHE_ENTRIES: usize = 4096;

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

    /// Retrieve or build the STATIC subset of GenAI attributes for a (system,
    /// model) pair: `gen_ai.system` and `gen_ai.request.model`.
    ///
    /// ~keep Only these two fields are safe to key a process-wide cache on.
    /// ~keep `gen_ai.response.model` and `gen_ai.operation.name` vary *per call*
    /// ~keep even for the same (system, model) — the response model can differ
    /// ~keep from the request model, and `req.model()` is `None` (so `model` is
    /// ~keep `""`) for `ListModels` and for `ImageGenerate`/`Moderate` requests
    /// ~keep with no model set. Baking either into the cached value (as a
    /// ~keep previous version of this function did) meant the first observation
    /// ~keep for a given key permanently overwrote every later one's response
    /// ~keep model and operation name. Callers must combine this with the
    /// ~keep per-call fields via [`build_base_attrs`] / [`build_token_attrs`]
    /// ~keep instead of caching the combined result.
    fn get_or_build_static_attrs(system: &str, model: &str) -> Arc<[KeyValue]> {
        let system_arc = Arc::<str>::from(system);
        let model_arc = Arc::<str>::from(model);
        let key = (Arc::clone(&system_arc), Arc::clone(&model_arc));

        let cache = base_attrs_cache();

        if let Some(entry) = cache.get(&key) {
            return Arc::clone(&entry);
        }

        let attrs: Arc<[KeyValue]> = Arc::from(
            vec![
                KeyValue::new("gen_ai.system", system_arc.to_string()),
                KeyValue::new("gen_ai.request.model", model_arc.to_string()),
            ]
            .into_boxed_slice(),
        );

        evict_if_at_cap(cache, "base_attrs");
        cache.entry(key).or_insert_with(|| Arc::clone(&attrs));

        attrs
    }

    /// Build the full per-observation attribute set for a single call: the
    /// cached static (system, model) pair plus this call's own
    /// `gen_ai.response.model` and `gen_ai.operation.name`, which are never
    /// cached (see [`get_or_build_static_attrs`]).
    fn build_base_attrs(system: &str, model: &str, response_model: &str, operation: &str) -> Vec<KeyValue> {
        let mut attrs = get_or_build_static_attrs(system, model).to_vec();
        attrs.push(KeyValue::new("gen_ai.response.model", response_model.to_owned()));
        attrs.push(KeyValue::new("gen_ai.operation.name", operation.to_owned()));
        attrs
    }

    /// Clear `cache` and log a warning if it has reached
    /// [`MAX_METRICS_ATTR_CACHE_ENTRIES`], so an unbounded label-cardinality
    /// source degrades to "occasional cache misses" instead of unbounded
    /// memory growth.
    fn evict_if_at_cap<K, V>(cache: &DashMap<K, V>, cache_name: &'static str)
    where
        K: Eq + std::hash::Hash,
    {
        if cache.len() >= MAX_METRICS_ATTR_CACHE_ENTRIES {
            tracing::warn!(
                cache = cache_name,
                cap = MAX_METRICS_ATTR_CACHE_ENTRIES,
                "metrics: attribute cache reached its label-cardinality cap; evicting all cached entries"
            );
            cache.clear();
        }
    }

    /// Retrieve or build the STATIC subset of per-token-type attributes for a
    /// (system, model) pair: the [`get_or_build_static_attrs`] pair plus
    /// `gen_ai.token.type`. Like its base counterpart, this deliberately
    /// excludes `gen_ai.response.model` / `gen_ai.operation.name` — see
    /// [`get_or_build_static_attrs`] for why baking those into a cached value
    /// is unsound.
    fn get_or_build_static_token_attrs(system: &str, model: &str) -> CachedTokenAttrs {
        let system_arc = Arc::<str>::from(system);
        let model_arc = Arc::<str>::from(model);
        let key = (Arc::clone(&system_arc), Arc::clone(&model_arc));

        let cache = token_attrs_cache();

        if let Some(entry) = cache.get(&key) {
            return CachedTokenAttrs {
                input: Arc::clone(&entry.input),
                output: Arc::clone(&entry.output),
            };
        }

        let base = get_or_build_static_attrs(&system_arc, &model_arc);

        let mut input_attrs = base.to_vec();
        input_attrs.push(KeyValue::new("gen_ai.token.type", "input"));
        let input_arc: Arc<[KeyValue]> = Arc::from(input_attrs.into_boxed_slice());

        let mut output_attrs = base.to_vec();
        output_attrs.push(KeyValue::new("gen_ai.token.type", "output"));
        let output_arc: Arc<[KeyValue]> = Arc::from(output_attrs.into_boxed_slice());

        let cached = CachedTokenAttrs {
            input: Arc::clone(&input_arc),
            output: Arc::clone(&output_arc),
        };

        evict_if_at_cap(cache, "token_attrs");
        cache.entry(key).or_insert_with(|| CachedTokenAttrs {
            input: Arc::clone(&input_arc),
            output: Arc::clone(&output_arc),
        });

        cached
    }

    /// Build the full per-observation (input, output) token attribute pair for
    /// a single call: the cached static token-type attrs plus this call's own
    /// `gen_ai.response.model` and `gen_ai.operation.name`.
    fn build_token_attrs(
        system: &str,
        model: &str,
        response_model: &str,
        operation: &str,
    ) -> (Vec<KeyValue>, Vec<KeyValue>) {
        let cached = get_or_build_static_token_attrs(system, model);

        let mut input = cached.input.to_vec();
        input.push(KeyValue::new("gen_ai.response.model", response_model.to_owned()));
        input.push(KeyValue::new("gen_ai.operation.name", operation.to_owned()));

        let mut output = cached.output.to_vec();
        output.push(KeyValue::new("gen_ai.response.model", response_model.to_owned()));
        output.push(KeyValue::new("gen_ai.operation.name", operation.to_owned()));

        (input, output)
    }

    static INSTRUMENTS: OnceLock<Arc<Instruments>> = OnceLock::new();

    /// Return the cached instruments, initializing them if the meter is available.
    /// Returns `None` if the meter has not yet been initialized.
    fn instruments() -> Option<Arc<Instruments>> {
        if let Some(cached) = INSTRUMENTS.get() {
            return Some(Arc::clone(cached));
        }

        if let Some(meter) = global_meter() {
            let new_instruments = Arc::new(Instruments::new(meter));
            let result = INSTRUMENTS
                .set(Arc::clone(&new_instruments))
                .ok()
                .map(|_| Arc::clone(&new_instruments));
            return result.or_else(|| INSTRUMENTS.get().map(Arc::clone));
        }

        None
    }

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

                let Some(instr) = instruments() else {
                    return result;
                };

                let response_model = match &result {
                    Ok(LlmResponse::Chat(r)) => r.model.clone(),
                    Ok(LlmResponse::Embed(r)) => r.model.clone(),
                    _ => model_str.clone(),
                };

                let base_attrs = build_base_attrs(&system, &model_str, &response_model, operation);
                instr.op_duration.record(elapsed, &base_attrs);

                match result {
                    // ~keep `LlmResponse::usage()` always returns `None` for `ChatStream` —
                    // ~keep usage only arrives on the stream's final chunk once fully consumed.
                    // ~keep Without this branch every streamed request silently dropped its
                    // ~keep `gen_ai.client.token.usage` observations while cost tracking (which
                    // ~keep already wraps the stream the same way, see `cost::observe_stream_usage`)
                    // ~keep kept reporting non-zero `gen_ai.client.cost.usd`.
                    Ok(LlmResponse::ChatStream(stream)) => {
                        let instr_for_cb = Arc::clone(&instr);
                        let wrapped = crate::tower::cost::observe_stream_usage(stream, move |usage| {
                            let Some(usage) = usage else { return };
                            let (input_attrs, output_attrs) =
                                build_token_attrs(&system, &model_str, &response_model, operation);
                            instr_for_cb.token_usage.record(usage.prompt_tokens, &input_attrs);
                            instr_for_cb.token_usage.record(usage.completion_tokens, &output_attrs);
                        });
                        Ok(LlmResponse::ChatStream(wrapped))
                    }
                    Ok(resp) => {
                        if let Some(usage) = resp.usage() {
                            let (input_attrs, output_attrs) =
                                build_token_attrs(&system, &model_str, &response_model, operation);
                            instr.token_usage.record(usage.prompt_tokens, &input_attrs);
                            instr.token_usage.record(usage.completion_tokens, &output_attrs);
                        }
                        Ok(resp)
                    }
                    Err(e) => Err(e),
                }
            })
        }
    }

    /// Record a stale cache metric.
    ///
    /// Emits `gen_ai.cache.stale`. Not currently called anywhere in this
    /// crate: the cache layer (`cache.rs`) tracks `"exact"` / `"semantic"`
    /// hits and misses via [`record_cache_tier_hit`] / [`record_cache_tier_miss`]
    /// but has no call site for a stale-but-served response. Wiring it up
    /// requires touching `cache.rs`'s stale-serving branch, which is outside
    /// this module. If the meter has not been initialized, this call is a
    /// no-op.
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

    /// Record a circuit breaker rejection.
    ///
    /// Emits `gen_ai.circuit.trip`. Despite the metric name, [`super::circuit::CircuitLayer`]'s
    /// sole call site increments this once per request rejected while the
    /// circuit is open, not once per open transition — so `rate(gen_ai_circuit_trip[5m])`
    /// measures rejected traffic volume, not trip frequency. Fixing that requires
    /// moving the call in `circuit.rs` to the state-transition point (outside
    /// this module) and is a distinct concern from what this function currently,
    /// correctly, measures. The metric name is left as-is rather than renamed
    /// out from under it: it is part of the crate's public OTel surface, and
    /// renaming it here would not by itself fix the semantics. If the meter
    /// has not been initialized, this call is a no-op.
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

    /// Record the estimated USD cost of a completion.
    ///
    /// Call from [`crate::tower::cost::CostTrackingService`] once a
    /// completion's cost has been computed. Emits `gen_ai.client.cost.usd`.
    /// If the meter has not been initialized, this call is a no-op.
    pub fn record_cost_usd(system: &str, model: &str, operation: &str, cost_usd: f64) {
        if let Some(instr) = instruments() {
            instr.cost_usd.record(
                cost_usd,
                &[
                    KeyValue::new("gen_ai.system", system.to_owned()),
                    KeyValue::new("gen_ai.request.model", model.to_owned()),
                    KeyValue::new("gen_ai.operation.name", operation.to_owned()),
                ],
            );
        }
    }

    /// Resolve the effective `gen_ai.system` value: prefer the caller-supplied
    /// `system`, falling back to the provider prefix parsed from `model`
    /// (`"provider/model"`) when `system` is empty.
    ///
    /// ~keep All current [`record_cache_tier_hit`] / [`record_cache_tier_miss`]
    /// ~keep callers (`cache.rs`) pass `""` for `system` and the full
    /// ~keep `"provider/model"` string for `model`, which made `gen_ai.system`
    /// ~keep permanently empty on every cache-tier observation. Deriving it
    /// ~keep here — inside the recorder, using data it already receives — fixes
    /// ~keep that without requiring a `cache.rs` change.
    fn resolve_system<'a>(system: &'a str, model: &'a str) -> &'a str {
        if !system.is_empty() {
            return system;
        }
        model.split_once('/').map_or("", |(prefix, _)| prefix)
    }

    /// Record a per-tier cache hit.
    ///
    /// `tier` should be one of `"exact"`, `"semantic"`, or `"streaming_replay"`.
    /// Emits `gen_ai.cache.hit` with a `gen_ai.cache.tier` attribute. `system`
    /// is resolved via [`resolve_system`], so an empty `system` with a
    /// `"provider/model"`-shaped `model` still yields a populated
    /// `gen_ai.system`. If the meter has not been initialized, this call is a
    /// no-op.
    pub fn record_cache_tier_hit(system: &str, model: &str, tier: &str) {
        if let Some(instr) = instruments() {
            instr.cache_hit.add(
                1,
                &[
                    KeyValue::new("gen_ai.system", resolve_system(system, model).to_owned()),
                    KeyValue::new("gen_ai.request.model", model.to_owned()),
                    KeyValue::new("gen_ai.cache.tier", tier.to_owned()),
                ],
            );
        }
    }

    /// Record a per-tier cache miss.
    ///
    /// `tier` should be one of `"exact"`, `"semantic"`, or `"streaming_replay"`.
    /// Emits `gen_ai.cache.miss` with a `gen_ai.cache.tier` attribute. `system`
    /// is resolved via [`resolve_system`], so an empty `system` with a
    /// `"provider/model"`-shaped `model` still yields a populated
    /// `gen_ai.system`. If the meter has not been initialized, this call is a
    /// no-op.
    pub fn record_cache_tier_miss(system: &str, model: &str, tier: &str) {
        if let Some(instr) = instruments() {
            instr.cache_miss.add(
                1,
                &[
                    KeyValue::new("gen_ai.system", resolve_system(system, model).to_owned()),
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
    /// Emits `gen_ai.budget.rejection` with the triggering dimension. Call
    /// from [`super::budget::InMemoryBudgetLedger::check`] when returning
    /// [`super::budget::BudgetVerdict::Reject`] — as of this writing that
    /// call site does not exist yet: `budget.rs` only calls
    /// [`record_budget_spend`], so `gen_ai.budget.rejection` never fires even
    /// though `check` does return `Reject`. This is the metric an operator
    /// would alert on (rejected spend), so wiring `budget.rs`'s reject path
    /// to this function is the highest-value follow-up outside this module.
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

    #[cfg(test)]
    mod tests {
        use std::pin::Pin;
        use std::task::{Context as StdContext, Poll as StdPoll};

        use futures_core::Stream;

        use super::*;
        use crate::client::{BoxStream, LlmClient};
        use crate::tower::service::LlmService;
        use crate::tower::tests_common::{MockClient, chat_req};
        use crate::tower::types::LlmRequest;
        use crate::types::audio::{CreateSpeechRequest, CreateTranscriptionRequest, TranscriptionResponse};
        use crate::types::image::{CreateImageRequest, ImagesResponse};
        use crate::types::moderation::{ModerationRequest, ModerationResponse};
        use crate::types::ocr::{OcrRequest, OcrResponse};
        use crate::types::rerank::{RerankRequest, RerankResponse};
        use crate::types::search::{SearchRequest, SearchResponse};
        use crate::types::{
            ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse,
            ModelsListResponse, Usage,
        };

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

            let meter = global::meter("liter-llm-test");

            init_meter(meter.clone());

            let instr1 = instruments().expect("instruments should be cached");

            let meter2 = global::meter("liter-llm-test-2");
            init_meter(meter2);

            let instr2 = instruments().expect("instruments should still be cached");

            assert!(Arc::ptr_eq(&instr1, &instr2), "instruments should be reused");
        }

        /// Verify that metric record helpers are no-ops before the meter is initialized.
        /// These should not panic even when called without `init_meter`.
        #[test]
        fn metrics_record_helpers_no_op_without_meter() {
            record_cache_stale("openai", "gpt-4", "chat");
            record_circuit_trip("openai", "gpt-4");
            record_retry_attempt("openai", "gpt-4", "chat");
        }

        /// `resolve_system` must derive `gen_ai.system` from the model's
        /// `"provider/model"` prefix whenever the caller passes an empty
        /// `system` — the case for every current `record_cache_tier_hit` /
        /// `record_cache_tier_miss` call site (`cache.rs` always passes `""`).
        ///
        /// Revert line: replacing the `if !system.is_empty() { return system }
        /// ... model.split_once(...)` body with a bare `system` makes the
        /// first assertion fail (`resolve_system("", "openai/gpt-4")` would
        /// return `""` instead of `"openai"`).
        #[test]
        fn resolve_system_derives_from_model_when_system_is_empty() {
            assert_eq!(resolve_system("", "openai/gpt-4"), "openai");
            assert_eq!(resolve_system("anthropic", "openai/gpt-4"), "anthropic");
            assert_eq!(resolve_system("", "no-slash-model"), "");
        }

        /// Verify that repeated lookups for the same (system, model) pair reuse the
        /// same cached `Arc`, proving the caching layer avoids rebuilding attribute
        /// slices on every call rather than merely appearing to (see
        /// `base_attrs_do_not_poison_across_calls_with_same_key` for why the old
        /// version of this test, which inspected `Arc::strong_count` after routing
        /// through the full `MetricsService`, could never actually fail).
        ///
        /// Revert line: making `get_or_build_static_attrs` always rebuild (deleting
        /// its `if let Some(entry) = cache.get(&key) { return ... }` fast path)
        /// allocates a fresh `Arc` on the second call, so `Arc::ptr_eq` returns
        /// `false` and the test fails.
        #[test]
        fn base_attrs_cache_reuses_same_arc_for_same_key() {
            let first = get_or_build_static_attrs("openai", "gpt-4-cache-test-key");
            let second = get_or_build_static_attrs("openai", "gpt-4-cache-test-key");
            assert!(
                Arc::ptr_eq(&first, &second),
                "expected the second lookup to reuse the cached Arc, not allocate a new one"
            );
        }

        /// Regression for the attribute-cache poisoning bug: the cache used to be
        /// keyed only on (system, model) but baked `response.model` /
        /// `gen_ai.operation.name` — which vary per call for the same (system,
        /// model) — into the *cached* value. The first observation for a given key
        /// therefore permanently mislabelled every later one's response model and
        /// operation name.
        ///
        /// Revert line: changing `build_base_attrs` back to caching the combined
        /// four-field slice under the (system, model) key (the old
        /// `get_or_build_base_attrs` behaviour) makes the second call below return
        /// the first call's `"error"` / `"chat"` labels instead of its own, and the
        /// second `assert_eq!` fails.
        #[test]
        fn base_attrs_do_not_poison_across_calls_with_same_key() {
            fn response_model(attrs: &[KeyValue]) -> Option<String> {
                attrs
                    .iter()
                    .find(|kv| kv.key.as_str() == "gen_ai.response.model")
                    .map(|kv| kv.value.as_str().into_owned())
            }

            let first = build_base_attrs("openai", "gpt-4-poison-test-key", "error", "chat");
            let second = build_base_attrs("openai", "gpt-4-poison-test-key", "gpt-4-0613", "chat");

            assert_eq!(response_model(&first).as_deref(), Some("error"));
            assert_eq!(response_model(&second).as_deref(), Some("gpt-4-0613"));
        }

        /// A stream that yields a fixed sequence of chunks then ends, used to drive
        /// `LlmResponse::ChatStream` through `MetricsService` in
        /// `streaming_chat_records_token_usage_via_stream_completion` below.
        struct VecStream {
            items: std::collections::VecDeque<Result<ChatCompletionChunk>>,
        }

        impl Stream for VecStream {
            type Item = Result<ChatCompletionChunk>;
            fn poll_next(mut self: Pin<&mut Self>, _cx: &mut StdContext<'_>) -> StdPoll<Option<Self::Item>> {
                StdPoll::Ready(self.items.pop_front())
            }
        }

        /// Delegates every [`LlmClient`] method to an inner [`MockClient`] except
        /// `chat_stream`, which yields two chunks: one with no usage, then one
        /// carrying `usage` on the final chunk — mirroring how a real provider only
        /// reports usage once the stream completes.
        #[derive(Clone)]
        struct StreamingMockClient {
            inner: MockClient,
            usage: Usage,
        }

        impl LlmClient for StreamingMockClient {
            fn chat(&self, req: ChatCompletionRequest) -> crate::client::BoxFuture<'_, Result<ChatCompletionResponse>> {
                self.inner.chat(req)
            }

            fn chat_stream(
                &self,
                req: ChatCompletionRequest,
            ) -> crate::client::BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>> {
                let usage = self.usage.clone();
                let model = req.model.clone();
                Box::pin(async move {
                    let chunk = |usage: Option<Usage>| {
                        Ok(ChatCompletionChunk {
                            id: "chunk".into(),
                            object: "chat.completion.chunk".into(),
                            created: 0,
                            model: model.clone(),
                            choices: vec![],
                            usage,
                            system_fingerprint: None,
                            service_tier: None,
                        })
                    };
                    let stream: BoxStream<'static, Result<ChatCompletionChunk>> = Box::pin(VecStream {
                        items: std::collections::VecDeque::from([chunk(None), chunk(Some(usage))]),
                    });
                    Ok(stream)
                })
            }

            fn embed(&self, req: EmbeddingRequest) -> crate::client::BoxFuture<'_, Result<EmbeddingResponse>> {
                self.inner.embed(req)
            }

            fn list_models(&self) -> crate::client::BoxFuture<'_, Result<ModelsListResponse>> {
                self.inner.list_models()
            }

            fn image_generate(&self, req: CreateImageRequest) -> crate::client::BoxFuture<'_, Result<ImagesResponse>> {
                self.inner.image_generate(req)
            }

            fn speech(&self, req: CreateSpeechRequest) -> crate::client::BoxFuture<'_, Result<bytes::Bytes>> {
                self.inner.speech(req)
            }

            fn transcribe(
                &self,
                req: CreateTranscriptionRequest,
            ) -> crate::client::BoxFuture<'_, Result<TranscriptionResponse>> {
                self.inner.transcribe(req)
            }

            fn moderate(&self, req: ModerationRequest) -> crate::client::BoxFuture<'_, Result<ModerationResponse>> {
                self.inner.moderate(req)
            }

            fn rerank(&self, req: RerankRequest) -> crate::client::BoxFuture<'_, Result<RerankResponse>> {
                self.inner.rerank(req)
            }

            fn search(&self, req: SearchRequest) -> crate::client::BoxFuture<'_, Result<SearchResponse>> {
                self.inner.search(req)
            }

            fn ocr(&self, req: OcrRequest) -> crate::client::BoxFuture<'_, Result<OcrResponse>> {
                self.inner.ocr(req)
            }
        }

        /// Regression for "streaming bypasses accounting": `MetricsService::call`
        /// used to gate token-usage recording on `resp.usage()`, which is always
        /// `None` for `LlmResponse::ChatStream` (usage only arrives on the stream's
        /// final chunk). A 100%-streaming deployment therefore recorded zero
        /// `gen_ai.client.token.usage` observations. There is no OTel SDK reader
        /// wired into this test binary, so the histogram's recorded values cannot be
        /// read back directly; instead this asserts the externally observable proxy
        /// for "the recorder ran": the token-attrs cache gains an entry for the
        /// (system, model) key used here, which only happens inside
        /// `build_token_attrs`, which only `MetricsService::call` invokes once it
        /// has resolved a `Usage`.
        ///
        /// Revert line: reverting the `Ok(LlmResponse::ChatStream(stream))` arm back
        /// to falling into the `Ok(resp) => { if let Some(usage) = resp.usage() ...
        /// }` branch (i.e. removing the `observe_stream_usage` wrap) makes
        /// `resp.usage()` return `None` for `ChatStream`, so `build_token_attrs` is
        /// never called for this key and the `expect` below panics.
        #[tokio::test]
        async fn streaming_chat_records_token_usage_via_stream_completion() {
            use futures_util::StreamExt as _;
            use opentelemetry::global;

            init_meter(global::meter("liter-llm-test-streaming"));

            let usage = Usage {
                prompt_tokens: 42,
                completion_tokens: 7,
                total_tokens: 49,
                prompt_tokens_details: None,
                completion_tokens_details: None,
            };

            let client = StreamingMockClient {
                inner: MockClient::ok(),
                usage: usage.clone(),
            };
            let inner = LlmService::new(client);
            let mut svc = MetricsLayer.layer(inner);

            let model = "streamtest/streaming-token-usage-model";
            let resp = svc
                .call(LlmRequest::ChatStream(chat_req(model)))
                .await
                .expect("should succeed");

            let crate::tower::types::LlmResponse::ChatStream(mut stream) = resp else {
                panic!("expected a ChatStream response");
            };

            while stream.next().await.is_some() {}

            let key = (Arc::<str>::from("streamtest"), Arc::<str>::from(model));
            let entry = token_attrs_cache()
                .get(&key)
                .expect("streamed usage must populate the token-attrs cache once the stream completes");
            assert!(entry.input.iter().any(|kv| kv.key.as_str() == "gen_ai.token.type"));
            assert!(entry.output.iter().any(|kv| kv.key.as_str() == "gen_ai.token.type"));
        }
    }
}

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

    /// No-op cache-stale helper.
    #[inline]
    pub fn record_cache_stale(_system: &str, _model: &str, _operation: &str) {}

    /// No-op circuit-trip helper.
    #[inline]
    pub fn record_circuit_trip(_system: &str, _model: &str) {}

    /// No-op retry-attempt helper.
    #[inline]
    pub fn record_retry_attempt(_system: &str, _model: &str, _operation: &str) {}

    /// No-op cost helper.
    #[inline]
    pub fn record_cost_usd(_system: &str, _model: &str, _operation: &str, _cost_usd: f64) {}

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

// ~keep Explicit re-exports, not `pub use inner::*`: alef's `validate_no_private_path_leaks`
// cannot resolve a glob re-export out of a private module, so every item must be named
// individually for the generated bindings to receive a public path.
#[cfg(feature = "otel")]
pub use inner::{
    MetricsLayer, MetricsService, global_meter, init_meter, record_budget_rejection, record_budget_spend,
    record_cache_stale, record_cache_tier_hit, record_cache_tier_miss, record_circuit_trip, record_cost_usd,
    record_realtime_bytes, record_realtime_event, record_realtime_session_duration, record_retry_attempt,
};
#[cfg(not(feature = "otel"))]
pub use inner::{
    MetricsLayer, MetricsService, record_budget_rejection, record_budget_spend, record_cache_stale,
    record_cache_tier_hit, record_cache_tier_miss, record_circuit_trip, record_cost_usd, record_realtime_bytes,
    record_realtime_event, record_realtime_session_duration, record_retry_attempt,
};

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
        record_cache_stale("openai", "gpt-4", "chat");
        record_circuit_trip("openai", "gpt-4");
        record_retry_attempt("openai", "gpt-4", "chat");
    }
}
