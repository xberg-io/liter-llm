use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use regex::Regex;
use tower::{Layer, Service, ServiceExt};

use liter_llm::auth::{Credential, CredentialProvider};
use liter_llm::client::{BoxFuture, ClientConfig, ClientConfigBuilder, DefaultClient};
use liter_llm::error::LiterLlmError;
use liter_llm::guardrail::GuardrailRegistry;
use liter_llm::observability::{MultiUsageSink, UsageSinkErased};
use liter_llm::tenant::TenantId;
use liter_llm::tower::types::{LlmRequest, LlmRequestKind, LlmResponse};
use liter_llm::tower::{
    BudgetConfig, BudgetDimension, BudgetLayer, BudgetLedger, BudgetLedgerLayer, BudgetState, BudgetVerdict,
    CacheConfig, CacheLayer, CooldownLayer, CostCheckContext, CostTrackingLayer, EmbeddingSimilarityClassifier,
    Enforcement, GuardrailLayer, HealthCheckLayer, HooksLayer, IntentPrototype, KeywordClassifier, LlmService,
    ModelRateLimitLayer, RateLimitConfig, RouteClassifier, Router, RoutingStrategy, TracingLayer, Weight,
};

use crate::config::{ClassifierConfig, KeywordRuleConfig, ModelEntry, ProxyConfig, RoutingConfig, VirtualKeyConfig};
use crate::error::ProxyError;
use crate::tenant_limit::{KeyLimitLayer, PerKeyBudgetLedger, build_key_limits};

type Bcs = tower::util::BoxCloneService<LlmRequest, LlmResponse, LiterLlmError>;

/// Thread-safe wrapper around `BoxCloneService`.
///
/// Tower's `BoxCloneService` is `Send` but not `Sync`, because `Service::call`
/// takes `&mut self`. We wrap it in a `Mutex` and clone on each request — the
/// lock is held only for the duration of `Clone::clone` (a handful of `Arc`
/// ref-count bumps).
struct SyncBoxService {
    inner: Mutex<Bcs>,
}

impl SyncBoxService {
    /// Clone the inner service out of the mutex.
    ///
    /// # Errors
    ///
    /// Returns `ProxyError::internal` if the mutex is poisoned.
    fn clone_service(&self) -> Result<Bcs, ProxyError> {
        self.inner
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| ProxyError::internal("service mutex poisoned"))
    }
}

/// A pool of Tower service stacks, one per configured model name.
///
/// Each model name maps to a type-erased `BoxCloneService` with the full
/// middleware stack applied (cache, health check, cooldown, rate limit, cost
/// tracking, budget, tracing).
pub struct ServicePool {
    /// Model name -> Tower service stack.
    services: HashMap<String, SyncBoxService>,
    /// Model name -> raw `DefaultClient` (for File/Batch/Response operations
    /// that bypass the Tower stack).
    clients: HashMap<String, Arc<DefaultClient>>,
    /// The first client inserted during construction, for deterministic
    /// `first_client()` behaviour regardless of `HashMap` iteration order.
    default_client: Option<Arc<DefaultClient>>,
    /// Shared per-key rpm/tpm limiter, retained so a config reload can push
    /// updated limits into every model's Tower stack (see
    /// [`ServicePool::update_key_limits`]) without rebuilding the pool.
    key_limit_layer: Arc<KeyLimitLayer>,
    /// Shared per-key calendar-month budget ledger, retained for the same
    /// reason as `key_limit_layer`.
    budget_ledger: Arc<PerKeyBudgetLedger>,
    /// The guardrail set built from `[[guardrails]]`, layered into every
    /// model's stack above and retained so [`AppState`] can hand the *same*
    /// registry to the realtime WebSocket path.
    ///
    /// [`AppState`]: crate::state::AppState
    guardrails: Arc<GuardrailRegistry>,
}

// ~keep SAFETY: `SyncBoxService` wraps a `Mutex<BoxCloneService>` which is `Send + Sync`.
// ~keep `Arc<DefaultClient>` is `Send + Sync`. The compiler verifies these bounds.

impl ServicePool {
    /// Build a pool from the proxy configuration.
    ///
    /// Groups `config.models` by `name` and creates a Tower service stack for
    /// each unique model name.  When multiple deployments share a name they
    /// form an active-active pool dispatched via [`Router`] with the
    /// [`RoutingStrategy`] configured in `[routing]` (defaulting to
    /// [`RoutingStrategy::RoundRobin`] — see [`build_routing_strategy`] and
    /// [`build_base_service`]); a single entry gets a bare `LlmService` with
    /// no routing overhead.
    ///
    /// `usage_sink`, when `Some`, is wired into `HooksLayer` outermost in
    /// every model's Tower stack so all completions emit a `UsageEvent`.
    ///
    /// # Errors
    ///
    /// Returns an error string if a `DefaultClient` cannot be constructed for
    /// any model entry, or if any `[[guardrails]]` entry cannot be built. The
    /// guardrail failure is deliberately fatal — see
    /// [`crate::guardrail::build_registry`].
    pub fn from_config(config: &ProxyConfig, usage_sink: Option<Arc<dyn UsageSinkErased>>) -> Result<Self, String> {
        // ~keep Built first, before any client or routing setup: a rejected guardrail
        // ~keep config must abort startup, and failing here costs nothing to unwind.
        let guardrails = crate::guardrail::build_registry(&config.guardrails)?;

        let mut grouped: HashMap<String, Vec<&ModelEntry>> = HashMap::new();
        for entry in &config.models {
            grouped.entry(entry.name.clone()).or_default().push(entry);
        }

        // ~keep Built once and reused via `.layer(..)` for every model below so a
        // ~keep key's rpm/tpm/budget limit is shared across all models it can
        // ~keep reach, not reset per model (see `tenant_limit` module docs).
        // ~keep Both are Arc-wrapped and retained on `ServicePool` (not just the
        // ~keep local `.layer(..)` closures) so a config reload can call
        // ~keep `update_key_limits` afterwards — see that method's docs.
        let key_limit_layer = Arc::new(KeyLimitLayer::new(build_key_limits(&config.keys)));
        let budget_ledger = Arc::new(PerKeyBudgetLedger::new(&config.keys));

        // ~keep Built once and cloned per group below: `RoutingStrategy` is global
        // ~keep (see `RoutingConfig` docs), and cloning is cheap (an Arc bump for
        // ~keep `Semantic`, a small Vec copy for `WeightedRandom`) next to rebuilding
        // ~keep a classifier — including its embedding `DefaultClient` — per group.
        let routing_strategy = build_routing_strategy(config)?;

        let mut services = HashMap::new();
        let mut clients = HashMap::new();
        let mut default_client: Option<Arc<DefaultClient>> = None;

        for (name, entries) in &grouped {
            let (base, client_arc) = build_base_service(entries, config, &routing_strategy)?;

            if default_client.is_none() {
                default_client = Some(Arc::clone(&client_arc));
            }

            let svc = build_service_stack(
                config,
                base,
                usage_sink.clone(),
                &key_limit_layer,
                &budget_ledger,
                &guardrails,
            );

            services.insert(name.clone(), SyncBoxService { inner: Mutex::new(svc) });
            clients.insert(name.clone(), client_arc);
        }

        Ok(Self {
            services,
            clients,
            default_client,
            key_limit_layer,
            budget_ledger,
            guardrails,
        })
    }

    /// The guardrail set enforced by every model's Tower stack in this pool.
    ///
    /// [`crate::state::AppState`] stores this handle so the realtime WebSocket
    /// path — which proxies raw frames and therefore has no Tower stack to
    /// layer — enforces the same guardrails as the unary routes. Callers must
    /// take the set from here rather than rebuilding it, so the two paths
    /// cannot be configured apart.
    #[must_use]
    pub fn guardrails(&self) -> Arc<GuardrailRegistry> {
        Arc::clone(&self.guardrails)
    }

    /// Push updated per-key rpm/tpm/budget limits from a reloaded
    /// `ProxyConfig.keys` into every model's live Tower stack.
    ///
    /// Both the rpm/tpm limiter and the budget ledger are shared (via `Arc`)
    /// across every model's stack, so this takes effect for all in-flight and
    /// future requests immediately — no `ServicePool` rebuild needed. Call
    /// this from the config watcher's reload path alongside
    /// `auth::KeyStore::reload`.
    ///
    /// Accumulated state survives: the budget ledger swaps only its configured
    /// caps, and the rpm/tpm sliding-window counters are untouched. A reload
    /// therefore cannot be used to reset a tenant's month-to-date spend.
    pub fn update_key_limits(&self, keys: &[VirtualKeyConfig]) {
        self.key_limit_layer.update_limits(keys);
        self.budget_ledger.update_limits(keys);
    }

    /// Clone and return a Tower service stack for the given model name.
    ///
    /// # Errors
    ///
    /// Returns `ProxyError::not_found` if no model with that name exists.
    pub fn get_service(&self, model: &str) -> Result<Bcs, ProxyError> {
        self.services
            .get(model)
            .ok_or_else(|| ProxyError::not_found(format!("model '{model}' not found")))?
            .clone_service()
    }

    /// Enforce per-key rpm and budget limits at realtime WebSocket session
    /// establishment.
    ///
    /// `routes::realtime` never runs a session through `get_service`'s Tower
    /// stack — it proxies raw WebSocket frames, so there is no discrete
    /// `LlmRequest`/`LlmResponse` per message to run through `KeyLimitLayer`
    /// or `BudgetLedgerLayer`. This method reuses the SAME `key_limit_layer`
    /// and `budget_ledger` instances `build_service_stack` wires into every
    /// model's stack (see `tenant_limit` module docs) rather than a separate
    /// realtime-only implementation, so a key's rpm window and calendar-month
    /// spend are shared across unary calls and realtime sessions. Session
    /// establishment is charged as a single reserved request against the rpm
    /// window; there is no cost-tracking hook for the ongoing audio stream,
    /// so budget enforcement here is a pre-flight gate against already
    /// recorded spend, not continuous metering of the live session.
    ///
    /// # Errors
    ///
    /// Returns [`LiterLlmError::RateLimited`] (converted to a 429 `ProxyError`
    /// via the same `From` impl unary calls use) when the per-key rpm/tpm
    /// window is exhausted, or [`LiterLlmError::BudgetExceeded`] when the
    /// tenant is already over its configured budget — matching
    /// `BudgetLedgerService::call`'s Reject handling exactly, so a rejected
    /// realtime session reports the same error shape as a rejected unary
    /// call.
    pub async fn check_realtime_session_start(&self, tenant_id: &TenantId, model: &str) -> Result<(), ProxyError> {
        self.key_limit_layer.check_and_reserve(tenant_id)?;

        let check_ctx = CostCheckContext {
            model,
            provider: "openai",
            tenant_id: Some(tenant_id.as_ref()),
            user_id: None,
            api_key_id: None,
            timestamp: std::time::SystemTime::now(),
        };

        if let BudgetVerdict::Reject { reason, dimension } = self.budget_ledger.check(&check_ctx).await {
            let model_field = match &dimension {
                BudgetDimension::Model(m) => Some(m.clone()),
                _ => None,
            };
            return Err(LiterLlmError::BudgetExceeded {
                message: reason,
                model: model_field,
            }
            .into());
        }

        Ok(())
    }

    /// Return a reference to the raw `DefaultClient` for the given model.
    ///
    /// Useful for File, Batch, and Response API operations that bypass the
    /// Tower middleware stack.
    ///
    /// # Errors
    ///
    /// Returns `ProxyError::not_found` if no model with that name exists.
    pub fn get_client(&self, model: &str) -> Result<Arc<DefaultClient>, ProxyError> {
        self.clients
            .get(model)
            .cloned()
            .ok_or_else(|| ProxyError::not_found(format!("model '{model}' not found")))
    }

    /// Return the first available raw client.
    ///
    /// Used by File, Batch, and Response API endpoints, which bypass model
    /// routing and talk to a single upstream account. File and Batch carry no
    /// model at all; `CreateResponseRequest` does carry a required `model`, but
    /// it is only forwarded on the wire — it never selects a provider here.
    pub fn first_client(&self) -> Result<Arc<DefaultClient>, ProxyError> {
        self.default_client
            .clone()
            .ok_or_else(|| ProxyError::service_unavailable("no models configured"))
    }

    /// Return the names of all available models.
    pub fn model_names(&self) -> Vec<&str> {
        self.services.keys().map(String::as_str).collect()
    }

    /// Return `true` if the pool contains at least one service.
    pub fn has_any_service(&self) -> bool {
        !self.services.is_empty()
    }
}

/// Extract each entry's real provider model identifier, in the same order as
/// `entries`.
///
/// This is the `Vec` handed to [`Router::with_deployment_models`] — its index
/// must align 1:1 with the `deployments` vec passed to [`Router::new`], since
/// [`RoutingStrategy::Semantic`] resolves a classifier verdict back to a
/// deployment by matching against this list. [`build_base_service`] builds
/// both vecs from the same iteration so they cannot drift apart.
fn deployment_model_ids(entries: &[&ModelEntry]) -> Vec<String> {
    entries.iter().map(|entry| entry.provider_model.clone()).collect()
}

/// Build the base (pre-middleware) service for one model-name group.
///
/// A `name` may be declared by more than one `[[models]]` entry to form an
/// active-active deployment pool (see `proxy-configuration.mdx`). A single
/// entry gets a bare `LlmService` with no routing overhead; multiple entries
/// are wrapped in a [`Router`] using `routing_strategy` (built once per pool
/// by [`build_routing_strategy`] from `[routing]`, defaulting to
/// [`RoutingStrategy::RoundRobin`]) so every configured deployment actually
/// receives traffic, with [`Router::with_deployment_models`] wired to each
/// entry's real `provider_model` so `RoutingStrategy::Semantic` resolves
/// correctly.
///
/// Returns the base service plus the first entry's raw client, which callers
/// use for File/Batch/Response operations that bypass the Tower stack (see
/// `ServicePool::get_client`).
///
/// # Errors
///
/// Returns an error string if any entry's `DefaultClient` fails to build, or
/// if `Router::new` rejects the deployment list — either because `entries`
/// is empty (cannot happen here since callers derive `entries` from a
/// non-empty group) or because a `RoutingStrategy::WeightedRandom` weight
/// count from `[routing]` doesn't match this group's deployment count.
fn build_base_service(
    entries: &[&ModelEntry],
    config: &ProxyConfig,
    routing_strategy: &RoutingStrategy,
) -> Result<(Bcs, Arc<DefaultClient>), String> {
    if entries.len() == 1 {
        let client_arc = Arc::new(build_client(entries[0], config)?);
        let base = deployment_service(Arc::clone(&client_arc), entries[0].provider_model.clone());
        return Ok((base, client_arc));
    }

    let mut deployments = Vec::with_capacity(entries.len());
    let mut first_client: Option<Arc<DefaultClient>> = None;
    for entry in entries.iter().copied() {
        let client_arc = Arc::new(build_client(entry, config)?);
        if first_client.is_none() {
            first_client = Some(Arc::clone(&client_arc));
        }
        deployments.push(deployment_service(client_arc, entry.provider_model.clone()));
    }
    let deployment_models = deployment_model_ids(entries);

    let router = Router::new(deployments, routing_strategy.clone())
        .map_err(|e| format!("failed to build router for deployment pool: {e}"))?
        .with_deployment_models(deployment_models);

    let base = guard_classifier_credentials(tower::util::BoxCloneService::new(router), config)?;
    let client_arc = first_client.ok_or_else(|| "deployment pool must have at least one entry".to_string())?;
    Ok((base, client_arc))
}

fn deployment_service(client: Arc<DefaultClient>, provider_model: String) -> Bcs {
    tower::util::BoxCloneService::new(
        LlmService::new_from_arc(client).map_request(move |request| with_configured_model(request, &provider_model)),
    )
}

fn with_configured_model(mut request: LlmRequest, model: &str) -> LlmRequest {
    match &mut request.kind {
        LlmRequestKind::Chat(r) | LlmRequestKind::ChatStream(r) => model.clone_into(&mut r.model),
        LlmRequestKind::Embed(r) => model.clone_into(&mut r.model),
        LlmRequestKind::ImageGenerate(r) => r.model = Some(model.to_owned()),
        LlmRequestKind::Speech(r) => model.clone_into(&mut r.model),
        LlmRequestKind::Transcribe(r) => model.clone_into(&mut r.model),
        LlmRequestKind::Moderate(r) => r.model = Some(model.to_owned()),
        LlmRequestKind::Rerank(r) => model.clone_into(&mut r.model),
        LlmRequestKind::Search(r) => model.clone_into(&mut r.model),
        LlmRequestKind::Ocr(r) => model.clone_into(&mut r.model),
        LlmRequestKind::ListModels => {}
    }
    request
}

/// Translate `config.routing` (`[routing]`) into the runtime
/// [`RoutingStrategy`] applied to every multi-deployment group.
///
/// Absent `[routing]` returns [`RoutingStrategy::RoundRobin`], preserving the
/// behaviour every existing config relied on before `[routing]` existed.
///
/// # Errors
///
/// Returns an error string if `strategy = "semantic"` and the configured
/// classifier fails to build — an invalid regex pattern for a `keyword`
/// classifier, or a client-construction failure for an `embedding`
/// classifier's credentials.
fn build_routing_strategy(config: &ProxyConfig) -> Result<RoutingStrategy, String> {
    let Some(routing) = &config.routing else {
        return Ok(RoutingStrategy::RoundRobin);
    };

    match routing {
        RoutingConfig::RoundRobin => Ok(RoutingStrategy::RoundRobin),
        RoutingConfig::Fallback => Ok(RoutingStrategy::Fallback),
        RoutingConfig::LatencyBased => Ok(RoutingStrategy::LatencyBased),
        RoutingConfig::CostBased => Ok(RoutingStrategy::CostBased),
        RoutingConfig::WeightedRandom { weights } => Ok(RoutingStrategy::WeightedRandom {
            weights: weights.iter().copied().map(Weight::from_f64).collect(),
        }),
        RoutingConfig::Semantic { classifier } => Ok(RoutingStrategy::Semantic(build_classifier(classifier, config)?)),
    }
}

/// Build the [`RouteClassifier`] behind `RoutingStrategy::Semantic` from
/// `[routing.classifier]`.
///
/// # Errors
///
/// Returns an error string if a `keyword` rule's `pattern` is not a valid
/// regex, or if an `embedding` classifier's `DefaultClient` fails to build
/// (see [`build_classifier_client`]).
fn build_classifier(cfg: &ClassifierConfig, config: &ProxyConfig) -> Result<Arc<dyn RouteClassifier>, String> {
    match cfg {
        ClassifierConfig::Keyword { rules } => {
            let compiled = compile_keyword_rules(rules)?;
            Ok(Arc::new(KeywordClassifier::new(compiled)))
        }
        ClassifierConfig::Embedding {
            embedding_model,
            api_key,
            base_url,
            threshold,
            prototypes,
        } => {
            let client = build_classifier_client(api_key.as_deref(), base_url.as_deref(), embedding_model, config)?;
            let prototypes = prototypes
                .iter()
                .map(|p| IntentPrototype {
                    name: p.name.clone(),
                    embedding: p.embedding.clone(),
                    model: p.model.clone(),
                })
                .collect();
            Ok(Arc::new(EmbeddingSimilarityClassifier::new(
                Arc::new(client),
                embedding_model.clone(),
                prototypes,
                *threshold,
            )))
        }
    }
}

/// Compile each `(pattern, model)` rule's regex, failing on the first
/// invalid pattern with the pattern text and the underlying `regex` error.
fn compile_keyword_rules(rules: &[KeywordRuleConfig]) -> Result<Vec<(Regex, String)>, String> {
    rules
        .iter()
        .map(|rule| {
            Regex::new(&rule.pattern)
                .map(|re| (re, rule.model.clone()))
                .map_err(|e| format!("routing.classifier: invalid regex pattern '{}': {e}", rule.pattern))
        })
        .collect()
}

/// Build the `DefaultClient` an `EmbeddingSimilarityClassifier` uses to embed
/// the live request prompt, applying the same `[general]` timeout/retry
/// defaults [`build_client`] applies to `[[models]]` entries.
fn build_classifier_client(
    api_key: Option<&str>,
    base_url: Option<&str>,
    embedding_model: &str,
    config: &ProxyConfig,
) -> Result<DefaultClient, String> {
    let client_config = classifier_client_config(api_key, base_url, config);
    build_request_authenticated_client(client_config, embedding_model)
        .map_err(|e| format!("failed to build routing.classifier embedding client: {e}"))
}

fn classifier_client_config(api_key: Option<&str>, base_url: Option<&str>, config: &ProxyConfig) -> ClientConfig {
    let mut builder = ClientConfigBuilder::new(api_key.unwrap_or(""));
    if let Some(url) = base_url {
        builder = builder.base_url(url);
    }
    builder = builder.timeout(Duration::from_secs(config.general.default_timeout_secs));
    builder.max_retries(config.general.max_retries).build()
}

fn missing_classifier_credentials(config: &ProxyConfig) -> Result<Option<Arc<MissingProviderCredentials>>, String> {
    let Some(RoutingConfig::Semantic {
        classifier:
            ClassifierConfig::Embedding {
                api_key,
                base_url,
                embedding_model,
                ..
            },
    }) = &config.routing
    else {
        return Ok(None);
    };
    let client_config = classifier_client_config(api_key.as_deref(), base_url.as_deref(), config);
    match DefaultClient::new(client_config, Some(embedding_model)) {
        Err(LiterLlmError::Authentication { message, status }) => {
            Ok(Some(Arc::new(MissingProviderCredentials { message, status })))
        }
        Ok(_) => Ok(None),
        Err(error) => Err(format!("failed to build routing.classifier embedding client: {error}")),
    }
}

// ~keep Classifiers intentionally defer transient failures. A missing configured
// ~keep credential must instead fail routed chat requests before any upstream call.
fn guard_classifier_credentials(base: Bcs, config: &ProxyConfig) -> Result<Bcs, String> {
    let Some(credentials) = missing_classifier_credentials(config)? else {
        return Ok(base);
    };
    Ok(tower::util::BoxCloneService::new(tower::service_fn(
        move |request: LlmRequest| {
            let mut service = base.clone();
            let credentials = Arc::clone(&credentials);
            async move {
                if matches!(&request.kind, LlmRequestKind::Chat(_) | LlmRequestKind::ChatStream(_)) {
                    credentials.resolve().await?;
                }
                service.call(request).await
            }
        },
    )))
}

/// Build a `DefaultClient` from a `ModelEntry` and global config defaults.
fn build_client(entry: &ModelEntry, config: &ProxyConfig) -> Result<DefaultClient, String> {
    let api_key = entry.api_key.as_deref().unwrap_or("");

    let mut builder = ClientConfigBuilder::new(api_key);

    if let Some(ref url) = entry.base_url {
        builder = builder.base_url(url);
    }

    let timeout_secs = entry.timeout_secs.unwrap_or(config.general.default_timeout_secs);
    builder = builder.timeout(Duration::from_secs(timeout_secs));
    builder = builder.max_retries(config.general.max_retries);

    let client = build_request_authenticated_client(builder.build(), &entry.provider_model);
    client.map_err(|e| format!("failed to build client for model '{}': {e}", entry.name))
}

fn build_request_authenticated_client(mut config: ClientConfig, model: &str) -> Result<DefaultClient, LiterLlmError> {
    match DefaultClient::new(config.clone(), Some(model)) {
        Err(LiterLlmError::Authentication { message, status }) => {
            config.load_env = false;
            config.credential_provider = Some(Arc::new(MissingProviderCredentials { message, status }));
            DefaultClient::new(config, Some(model))
        }
        result => result,
    }
}

// ~keep Missing provider keys are request failures; the MCP catalog and local
// ~keep tools must remain available before provider credentials are configured.
struct MissingProviderCredentials {
    message: String,
    status: u16,
}

impl CredentialProvider for MissingProviderCredentials {
    fn resolve(&self) -> BoxFuture<'_, liter_llm::error::Result<Credential>> {
        Box::pin(async move {
            Err(LiterLlmError::Authentication {
                message: self.message.clone(),
                status: self.status,
            })
        })
    }
}

/// Compose the Tower middleware stack, following the same layering order as
/// `managed.rs:build_service_stack`:
///
/// 1. Cache (innermost)
/// 2. HealthCheck
/// 3. Cooldown
/// 4. RateLimit (per-model)
/// 5. KeyLimit (per-virtual-key rpm/tpm — see issue #71)
/// 6. Per-key budget ledger (calendar-month, per-virtual-key — see issue #71)
/// 7. CostTracking
/// 8. Budget (global/per-model, from `[budget]`)
/// 9. Guardrail (from `[[guardrails]]`, conditional on a non-empty set)
/// 10. Tracing
/// 11. HooksLayer with usage sink (outermost, conditional on `usage_sink.is_some()`)
///
/// HooksLayer sits outermost so it observes every request regardless of which
/// inner layer produces the response (cache hit or live upstream).
///
/// Guardrails sit **outside** cache, rate-limit and budget so that: a blocked
/// request never consumes a caller's rpm/tpm window or budget, a cache hit is
/// still inspected at the `Output` stage instead of replaying content that
/// bypassed the check, and an `Input` block costs no upstream call. They sit
/// inside Tracing so a block is still recorded as a request. ~keep
fn build_service_stack(
    config: &ProxyConfig,
    base: Bcs,
    usage_sink: Option<Arc<dyn UsageSinkErased>>,
    key_limit_layer: &KeyLimitLayer,
    budget_ledger: &Arc<PerKeyBudgetLedger>,
    guardrails: &Arc<GuardrailRegistry>,
) -> Bcs {
    let mut svc: Bcs = base;

    if let Some(ref cache_cfg) = config.cache {
        let max_entries = cache_cfg.max_entries.unwrap_or(256);
        let ttl = Duration::from_secs(cache_cfg.ttl_seconds.unwrap_or(300));
        let tower_cache_cfg = CacheConfig {
            max_entries,
            ttl,
            backend: liter_llm::tower::CacheBackend::Memory,
        };
        let layer = CacheLayer::new(tower_cache_cfg);
        svc = tower::util::BoxCloneService::new(layer.layer(svc));
    }

    if let Some(ref health_cfg) = config.health
        && let Some(interval_secs) = health_cfg.interval_secs
    {
        let layer = HealthCheckLayer::new(Duration::from_secs(interval_secs));
        svc = tower::util::BoxCloneService::new(layer.layer(svc));
    }

    if let Some(ref cooldown_cfg) = config.cooldown {
        let layer = CooldownLayer::new(Duration::from_secs(cooldown_cfg.duration_secs));
        svc = tower::util::BoxCloneService::new(layer.layer(svc));
    }

    if let Some(ref rl_cfg) = config.rate_limit {
        let tower_rl_cfg = RateLimitConfig {
            rpm: rl_cfg.rpm,
            tpm: rl_cfg.tpm,
            window: Duration::from_secs(60),
        };
        let layer = ModelRateLimitLayer::new(tower_rl_cfg);
        svc = tower::util::BoxCloneService::new(layer.layer(svc));
    }

    svc = tower::util::BoxCloneService::new(key_limit_layer.layer(svc));

    // ~keep Hard enforcement: mirrors the pre-existing `KeyLimitService` budget
    // ~keep check it replaces, which always rejected (no soft mode for per-key budgets).
    let budget_ledger_layer = BudgetLedgerLayer::new(Arc::clone(budget_ledger), Enforcement::Hard);
    svc = tower::util::BoxCloneService::new(budget_ledger_layer.layer(svc));

    if config.general.enable_cost_tracking {
        svc = tower::util::BoxCloneService::new(CostTrackingLayer.layer(svc));
    }

    if let Some(ref budget_cfg) = config.budget {
        let enforcement = match budget_cfg.enforcement {
            crate::config::EnforcementMode::Soft => Enforcement::Soft,
            crate::config::EnforcementMode::Hard => Enforcement::Hard,
        };
        let tower_budget_cfg = BudgetConfig {
            global_limit: budget_cfg.global_limit,
            model_limits: budget_cfg.model_limits.clone(),
            enforcement,
        };
        let state = Arc::new(BudgetState::new());
        let layer = BudgetLayer::new(tower_budget_cfg, state);
        svc = tower::util::BoxCloneService::new(layer.layer(svc));
    }

    // ~keep Skipped when nothing is configured purely to avoid GuardrailService's
    // ~keep per-request request serialization. This is safe only because
    // ~keep `build_registry` makes an empty registry mean "the operator configured
    // ~keep none" and never "some entry failed to build" — it aborts startup instead.
    if !guardrails.is_empty() {
        let layer = GuardrailLayer::with_registry(Arc::clone(guardrails));
        svc = tower::util::BoxCloneService::new(layer.layer(svc));
    }

    if config.general.enable_tracing {
        svc = tower::util::BoxCloneService::new(TracingLayer.layer(svc));
    }

    // ~keep HooksLayer sits outside Tracing so every request, including cache hits, emits usage.
    // ~keep `UsageSink` uses RPITIT and is not dyn-compatible, so `MultiUsageSink` erases it.
    if let Some(sink) = usage_sink {
        let multi = Arc::new(MultiUsageSink::from_erased(vec![sink]));
        let layer = HooksLayer::new(vec![]).with_usage_sink(multi);
        svc = tower::util::BoxCloneService::new(layer.layer(svc));
    }

    svc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProxyConfig;
    use liter_llm::tower::CostRecordContext;

    #[test]
    fn deployment_mapping_changes_only_model_for_every_model_bearing_request() {
        use serde_json::json;

        let cases = [
            ("Chat", json!({"model":"alias","messages":[],"temperature":0.2})),
            ("ChatStream", json!({"model":"alias","messages":[],"stream":true})),
            ("Embed", json!({"model":"alias","input":"embedding input"})),
            ("ImageGenerate", json!({"model":"alias","prompt":"image input"})),
            ("ImageGenerate", json!({"prompt":"image input"})),
            (
                "Speech",
                json!({"model":"alias","input":"speech input","voice":"alloy"}),
            ),
            ("Transcribe", json!({"model":"alias","file":"eA==","language":"en"})),
            ("Moderate", json!({"model":"alias","input":"moderation input"})),
            ("Moderate", json!({"input":"moderation input"})),
            (
                "Rerank",
                json!({"model":"alias","query":"rerank input","documents":["document"]}),
            ),
            ("Search", json!({"model":"alias","query":"search input","country":"US"})),
            (
                "Ocr",
                json!({"model":"alias","document":{"type":"base64","data":"eA==","media_type":"application/pdf"}}),
            ),
        ];
        assert_eq!(cases.len(), 12);
        for (variant, payload) in cases {
            let kind = serde_json::from_value(json!({variant: payload})).expect("valid request variant");
            let original = LlmRequest {
                kind,
                tenant_id: Some(TenantId::from("synthetic-tenant")),
                idempotency_key: Some("synthetic-idempotency".to_owned()),
            };
            let mut expected = serde_json::to_value(&original.kind).expect("original JSON");
            expected[variant]["model"] = json!("openai/configured-model");
            let mapped = with_configured_model(original.clone(), "openai/configured-model");
            assert_eq!(
                serde_json::to_value(&mapped.kind).expect("mapped JSON"),
                expected,
                "{variant}"
            );
            assert_eq!(mapped.tenant_id, original.tenant_id, "{variant}");
            assert_eq!(mapped.idempotency_key, original.idempotency_key, "{variant}");
            assert_eq!(mapped.model(), Some("openai/configured-model"), "{variant}");
        }
    }

    #[test]
    fn deployment_mapping_preserves_model_less_list_request_and_envelope() {
        let original = LlmRequest::ListModels()
            .with_tenant_id("synthetic-tenant")
            .with_idempotency_key("synthetic-idempotency");
        let mapped = with_configured_model(original.clone(), "openai/configured-model");
        assert_eq!(mapped.model(), None);
        assert_eq!(serde_json::to_value(mapped.kind).expect("mapped JSON"), "ListModels");
        assert_eq!(mapped.tenant_id, original.tenant_id);
        assert_eq!(mapped.idempotency_key, original.idempotency_key);
    }

    fn config_with_one_model() -> ProxyConfig {
        ProxyConfig::from_toml_str(
            r#"
[[models]]
name = "test-model"
provider_model = "openai/gpt-4o"
api_key = "sk-test"
"#,
        )
        .expect("valid TOML")
    }

    fn config_with_two_models() -> ProxyConfig {
        ProxyConfig::from_toml_str(
            r#"
[[models]]
name = "model-a"
provider_model = "openai/gpt-4o"
api_key = "sk-a"

[[models]]
name = "model-b"
provider_model = "anthropic/claude-sonnet-4-20250514"
api_key = "sk-b"
"#,
        )
        .expect("valid TOML")
    }

    fn config_with_unrouted_model(extra: &str) -> ProxyConfig {
        ProxyConfig::from_toml_str(&format!(
            r#"
[[models]]
name = "m1"
provider_model = "snowflake/arctic"
api_key = "sk-test"
{extra}
"#
        ))
        .expect("valid TOML")
    }

    #[test]
    fn model_for_a_provider_without_route_and_without_base_url_is_refused() {
        let config = config_with_unrouted_model("");
        let message = build_client(&config.models[0], &config)
            .err()
            .expect("the client must not build");
        assert!(message.contains("failed to build client for model 'm1'"), "{message}");
        assert!(
            message.contains("provider `snowflake` has no built-in route"),
            "{message}"
        );
        let pool_error = ServicePool::from_config(&config, None)
            .err()
            .expect("the pool must not build");
        assert_eq!(pool_error, message);
    }

    #[test]
    fn model_for_a_provider_without_route_builds_with_a_base_url() {
        let config = config_with_unrouted_model("base_url = \"http://127.0.0.1:9/v1\"");
        build_client(&config.models[0], &config).expect("a base_url lifts the refusal");
        ServicePool::from_config(&config, None).expect("the pool builds");
    }

    #[test]
    fn build_from_empty_config() {
        let config = ProxyConfig::default();
        let pool = ServicePool::from_config(&config, None).expect("empty config should build");
        assert!(pool.services.is_empty());
        assert!(pool.clients.is_empty());
        assert!(!pool.has_any_service());
    }

    #[test]
    fn build_from_config_with_one_model() {
        let config = config_with_one_model();
        let pool = ServicePool::from_config(&config, None).expect("should build");
        assert_eq!(pool.services.len(), 1);
        assert_eq!(pool.clients.len(), 1);
        assert!(pool.has_any_service());
    }

    #[test]
    fn get_service_for_unknown_model_returns_not_found() {
        let config = config_with_one_model();
        let pool = ServicePool::from_config(&config, None).expect("should build");
        let result = pool.get_service("nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn get_service_for_known_model_succeeds() {
        let config = config_with_one_model();
        let pool = ServicePool::from_config(&config, None).expect("should build");
        let result = pool.get_service("test-model");
        assert!(result.is_ok());
    }

    #[test]
    fn get_client_for_known_model_succeeds() {
        let config = config_with_one_model();
        let pool = ServicePool::from_config(&config, None).expect("should build");
        let result = pool.get_client("test-model");
        assert!(result.is_ok());
    }

    #[test]
    fn get_client_for_unknown_model_returns_not_found() {
        let config = config_with_one_model();
        let pool = ServicePool::from_config(&config, None).expect("should build");
        let result = pool.get_client("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn model_names_returns_correct_list() {
        let config = config_with_two_models();
        let pool = ServicePool::from_config(&config, None).expect("should build");
        let mut names = pool.model_names();
        names.sort();
        assert_eq!(names, vec!["model-a", "model-b"]);
    }

    #[test]
    fn has_any_service_returns_false_for_empty_pool() {
        let config = ProxyConfig::default();
        let pool = ServicePool::from_config(&config, None).expect("should build");
        assert!(!pool.has_any_service());
    }

    #[test]
    fn has_any_service_returns_true_for_nonempty_pool() {
        let config = config_with_one_model();
        let pool = ServicePool::from_config(&config, None).expect("should build");
        assert!(pool.has_any_service());
    }

    #[tokio::test]
    async fn build_with_middleware_config() {
        let config = ProxyConfig::from_toml_str(
            r#"
[general]
enable_cost_tracking = true
enable_tracing = true

[[models]]
name = "gpt"
provider_model = "openai/gpt-4o"
api_key = "sk-test"

[cache]
max_entries = 128
ttl_seconds = 60

[rate_limit]
rpm = 100

[budget]
global_limit = 50.0
enforcement = "soft"

[cooldown]
duration_secs = 30

[health]
interval_secs = 10
"#,
        )
        .expect("valid TOML");

        let pool = ServicePool::from_config(&config, None).expect("should build with middleware");
        assert!(pool.has_any_service());
        assert!(pool.get_service("gpt").is_ok());
    }

    /// Regression test for the "extra deployments are silently dropped" gap:
    /// `ServicePool::from_config` must build successfully — via
    /// [`build_base_service`]'s `Router` path — for a name declared by more
    /// than two entries, not just the two-entry case. `get_client` still
    /// resolves to the first entry, per the documented File/Batch bypass
    /// contract.
    #[test]
    fn deployment_pool_builds_router_across_all_entries() {
        let config = ProxyConfig::from_toml_str(
            r#"
[[models]]
name = "gpt"
provider_model = "openai/gpt-4o"
api_key = "sk-1"

[[models]]
name = "gpt"
provider_model = "anthropic/claude-sonnet-4-5"
api_key = "sk-2"

[[models]]
name = "gpt"
provider_model = "openai/gpt-4o-mini"
api_key = "sk-3"
"#,
        )
        .expect("valid TOML");

        let pool = ServicePool::from_config(&config, None).expect("should build");
        assert_eq!(pool.services.len(), 1);
        assert!(pool.get_service("gpt").is_ok());
        assert!(pool.get_client("gpt").is_ok());
    }

    /// The `Vec` passed to `Router::with_deployment_models` must align 1:1
    /// with the `deployments` vec index Router dispatches on — a misordered
    /// list resolves a classifier verdict to the wrong upstream. Both vecs
    /// are built from the same `entries` iteration in `build_base_service`,
    /// so this proves the extraction itself preserves configured order.
    #[test]
    fn deployment_model_ids_preserves_configured_order() {
        let one = ModelEntry {
            name: "gpt".to_string(),
            provider_model: "openai/gpt-4o".to_string(),
            api_key: None,
            base_url: None,
            timeout_secs: None,
            fallbacks: vec![],
        };
        let two = ModelEntry {
            name: "gpt".to_string(),
            provider_model: "azure/gpt-4o".to_string(),
            api_key: None,
            base_url: None,
            timeout_secs: None,
            fallbacks: vec![],
        };
        let entries: Vec<&ModelEntry> = vec![&one, &two];

        assert_eq!(
            deployment_model_ids(&entries),
            vec!["openai/gpt-4o".to_string(), "azure/gpt-4o".to_string()]
        );
    }

    /// Absent `[routing]` must preserve the pre-existing default exactly —
    /// round-robin — so every config written before `[routing]` existed
    /// keeps behaving the same way.
    #[test]
    fn build_routing_strategy_defaults_to_round_robin_when_unconfigured() {
        let config = ProxyConfig::default();
        let strategy = build_routing_strategy(&config).expect("default should build");
        assert!(matches!(strategy, RoutingStrategy::RoundRobin));
    }

    /// Regression guard for the class of bug this task fixes: a configured
    /// `strategy = "semantic"` must actually produce `RoutingStrategy::
    /// Semantic`, not silently fall back to round-robin.
    #[test]
    fn build_routing_strategy_builds_semantic_from_keyword_classifier() {
        let config = ProxyConfig::from_toml_str(
            r#"
[routing]
strategy = "semantic"

[routing.classifier]
kind = "keyword"

[[routing.classifier.rules]]
pattern = "(?i)sql"
model = "gpt-4o"
"#,
        )
        .expect("valid TOML");

        let strategy = build_routing_strategy(&config).expect("classifier should build");
        assert!(matches!(strategy, RoutingStrategy::Semantic(_)));
    }

    #[test]
    fn build_routing_strategy_converts_weighted_random_weights() {
        let config = ProxyConfig::from_toml_str(
            r#"
[routing]
strategy = "weighted_random"
weights = [3.0, 2.0, 1.0]
"#,
        )
        .expect("valid TOML");

        let strategy = build_routing_strategy(&config).expect("should build");
        match strategy {
            RoutingStrategy::WeightedRandom { weights } => {
                assert_eq!(
                    weights,
                    vec![Weight::from_f64(3.0), Weight::from_f64(2.0), Weight::from_f64(1.0)]
                );
            }
            other => panic!("expected WeightedRandom, got {other:?}"),
        }
    }

    /// A `keyword` classifier with an unparsable regex must fail router
    /// construction with a clear, actionable error rather than building a
    /// classifier that always defers (which would look identical to a
    /// working-but-idle semantic router from the outside).
    #[test]
    fn semantic_classifier_with_invalid_regex_fails_clearly() {
        let config = ProxyConfig::from_toml_str(
            r#"
[routing]
strategy = "semantic"

[routing.classifier]
kind = "keyword"

[[routing.classifier.rules]]
pattern = "(unclosed"
model = "gpt-4o"
"#,
        )
        .expect("valid TOML");

        let err = build_routing_strategy(&config).expect_err("invalid regex must fail, not build a dead classifier");
        assert!(err.contains("invalid regex"), "error should name the problem: {err}");
    }

    /// End-to-end regression test for the "semantic routing is silently
    /// inert" bug: a `[routing] strategy = "semantic"` config with a
    /// `keyword` classifier must produce a working `Router` — built via the
    /// same `build_base_service` path `ServicePool::from_config` uses — with
    /// the `Semantic` strategy and the deployment's real `provider_model`
    /// values, not stringified positional indices.
    #[test]
    fn build_base_service_wires_semantic_strategy_with_real_model_ids() {
        let entry_a = ModelEntry {
            name: "gpt".to_string(),
            provider_model: "openai/gpt-4o".to_string(),
            api_key: Some("sk-1".to_string()),
            base_url: None,
            timeout_secs: None,
            fallbacks: vec![],
        };
        let entry_b = ModelEntry {
            name: "gpt".to_string(),
            provider_model: "anthropic/claude-sonnet-4-5".to_string(),
            api_key: Some("sk-2".to_string()),
            base_url: None,
            timeout_secs: None,
            fallbacks: vec![],
        };
        let entries: Vec<&ModelEntry> = vec![&entry_a, &entry_b];

        let config = ProxyConfig::from_toml_str(
            r#"
[routing]
strategy = "semantic"

[routing.classifier]
kind = "keyword"

[[routing.classifier.rules]]
pattern = "(?i)claude"
model = "anthropic/claude-sonnet-4-5"
"#,
        )
        .expect("valid TOML");

        let routing_strategy = build_routing_strategy(&config).expect("classifier should build");
        assert!(
            matches!(routing_strategy, RoutingStrategy::Semantic(_)),
            "must be the configured Semantic strategy, not a silent RoundRobin default"
        );

        let (_base, _client) =
            build_base_service(&entries, &config, &routing_strategy).expect("router should build across both entries");
    }

    /// A `strategy = "embedding"` classifier config must build successfully
    /// end to end through `build_routing_strategy`, wiring a real
    /// `DefaultClient` for the live-prompt embedding call and the
    /// precomputed prototype vectors from TOML.
    #[test]
    fn build_routing_strategy_builds_semantic_from_embedding_classifier() {
        let config = ProxyConfig::from_toml_str(
            r#"
[routing]
strategy = "semantic"

[routing.classifier]
kind = "embedding"
embedding_model = "openai/text-embedding-3-small"
api_key = "sk-embed"
threshold = 0.75

[[routing.classifier.prototypes]]
name = "coding"
model = "gpt-4o"
embedding = [1.0, 0.0, 0.0]
"#,
        )
        .expect("valid TOML");

        let strategy = build_routing_strategy(&config).expect("embedding classifier should build");
        assert!(matches!(strategy, RoutingStrategy::Semantic(_)));
    }

    fn config_with_keyed_model(key_toml: &str) -> ProxyConfig {
        ProxyConfig::from_toml_str(&format!(
            r#"
[[models]]
name = "test-model"
provider_model = "openai/gpt-4o"
api_key = "sk-test"

[[keys]]
key = "vk-a"
models = ["test-model"]
{key_toml}
"#
        ))
        .expect("valid TOML")
    }

    /// Resolve a configured key's tenant id the same way `KeyContext` does.
    ///
    /// ~keep Never hardcode the key string as the tenant id here. The rpm/budget
    /// ~keep lookup fails OPEN on a miss, so a test that passes a literal would
    /// ~keep stop exercising enforcement the moment the derivation changes — it
    /// ~keep would go green by not enforcing anything, which is the exact
    /// ~keep regression these tests exist to catch.
    fn tenant_of(config: &ProxyConfig, key: &str) -> TenantId {
        let cfg = config
            .keys
            .iter()
            .find(|k| k.key == key)
            .expect("key must be present in the test config");
        crate::auth::key_store::resolved_tenant_id(cfg)
    }

    /// Regression test for the "realtime sessions bypass rpm/budget
    /// enforcement" gap: a key with capacity remaining must be allowed to
    /// start a realtime session.
    #[tokio::test]
    async fn check_realtime_session_start_allows_within_limits() {
        let config = config_with_keyed_model("rpm = 5");
        let pool = ServicePool::from_config(&config, None).expect("should build");

        let result = pool
            .check_realtime_session_start(&tenant_of(&config, "vk-a"), "test-model")
            .await;
        assert!(result.is_ok(), "session within rpm should be allowed, got: {result:?}");
    }

    /// A realtime session establishment must consume the SAME rpm counter a
    /// unary call would — proving `check_realtime_session_start` shares
    /// state with `KeyLimitLayer` rather than tracking sessions separately.
    #[tokio::test]
    async fn check_realtime_session_start_rejects_once_rpm_exhausted() {
        let config = config_with_keyed_model("rpm = 1");
        let pool = ServicePool::from_config(&config, None).expect("should build");
        let tenant = tenant_of(&config, "vk-a");

        let first = pool.check_realtime_session_start(&tenant, "test-model").await;
        assert!(first.is_ok(), "first session should be allowed, got: {first:?}");

        let second = pool.check_realtime_session_start(&tenant, "test-model").await;
        assert!(second.is_err(), "rpm=1 must reject the second session");
        assert_eq!(second.unwrap_err().error_type(), "RateLimited");
    }

    /// A tenant already over its configured budget must be denied a new
    /// realtime session — proving `check_realtime_session_start` reads from
    /// the SAME `PerKeyBudgetLedger` unary calls record spend into, not a
    /// separate realtime-only ledger.
    #[tokio::test]
    async fn check_realtime_session_start_rejects_when_budget_exhausted() {
        let config = config_with_keyed_model("budget_limit = 1.0");
        let pool = ServicePool::from_config(&config, None).expect("should build");
        let tenant = tenant_of(&config, "vk-a");

        pool.budget_ledger
            .record(&CostRecordContext {
                model: "test-model",
                provider: "openai",
                tenant_id: Some(tenant.as_ref()),
                user_id: None,
                api_key_id: None,
                cost_usd: 5.0,
                tokens_in: 100,
                tokens_out: 50,
                timestamp: std::time::SystemTime::now(),
            })
            .await;

        let result = pool.check_realtime_session_start(&tenant, "test-model").await;
        assert!(result.is_err(), "spend above budget_limit must reject the session");
        assert_eq!(result.unwrap_err().error_type(), "BudgetExceeded");
    }

    // ~keep Guardrail wiring for the unary Tower path. `[[guardrails]]` had no
    // ~keep effect on any request before this layer was added to build_service_stack.

    fn guarded_config(guardrails_toml: &str) -> Result<ProxyConfig, String> {
        ProxyConfig::from_toml_str(&format!(
            r#"
[[models]]
name = "gpt"
provider_model = "openai/gpt-4o"
api_key = "sk-test"
base_url = "http://127.0.0.1:1"
{guardrails_toml}
"#
        ))
    }

    const SSN_GUARDRAIL: &str = r#"
[[guardrails]]
type = "regex"
name = "block-ssn"
pattern = '\d{3}-\d{2}-\d{4}'
stages = ["input"]
action = { kind = "block", code = 1100, reason_prefix = "SSN detected" }
"#;

    fn chat_with(content: &str) -> LlmRequest {
        let chat = serde_json::from_value(serde_json::json!({
            "model": "gpt",
            "messages": [{ "role": "user", "content": content }],
        }))
        .expect("chat request fixture should deserialize");
        LlmRequest::Chat(chat)
    }

    /// A configured guardrail must block a matching request on the unary path.
    ///
    /// `base_url` points at a closed port, so if the `GuardrailLayer` were
    /// missing from the stack the request would reach the transport and fail
    /// with a connection error instead — a different error type, which is what
    /// makes this assertion sensitive to the layer actually being installed.
    #[tokio::test]
    async fn configured_guardrail_blocks_on_the_unary_path() {
        use tower::ServiceExt;

        let config = guarded_config(SSN_GUARDRAIL).expect("valid TOML");
        let pool = ServicePool::from_config(&config, None).expect("pool should build");
        let service = pool.get_service("gpt").expect("service for configured model");

        let error = service
            .oneshot(chat_with("my ssn is 123-45-6789"))
            .await
            .expect_err("a configured guardrail must block the request");

        match error {
            LiterLlmError::HookRejected { message } => {
                assert!(
                    message.contains("SSN detected"),
                    "the block reason should reach the caller, got: {message}"
                );
            }
            other => panic!("expected HookRejected from the guardrail layer, got: {other:?}"),
        }
    }

    /// The same stack must still forward a request that matches nothing — the
    /// layer blocks on a decision, it does not reject everything.
    #[tokio::test]
    async fn configured_guardrail_forwards_a_clean_request_on_the_unary_path() {
        use tower::ServiceExt;

        let config = guarded_config(SSN_GUARDRAIL).expect("valid TOML");
        let pool = ServicePool::from_config(&config, None).expect("pool should build");
        let service = pool.get_service("gpt").expect("service for configured model");

        let error = service
            .oneshot(chat_with("hello there"))
            .await
            .expect_err("the unreachable base_url makes every forwarded request fail");

        assert!(
            !matches!(error, LiterLlmError::HookRejected { .. }),
            "a clean request must reach the transport, not be blocked: {error:?}"
        );
    }

    /// An absent `[[guardrails]]` section must leave the stack exactly as it
    /// was — no registry entries, and no request blocked.
    #[tokio::test]
    async fn absent_guardrail_config_leaves_the_unary_path_unchanged() {
        use tower::ServiceExt;

        let config = guarded_config("").expect("valid TOML");
        let pool = ServicePool::from_config(&config, None).expect("pool should build");
        assert!(pool.guardrails().is_empty(), "no guardrails should be registered");

        let service = pool.get_service("gpt").expect("service for configured model");
        let error = service
            .oneshot(chat_with("my ssn is 123-45-6789"))
            .await
            .expect_err("the unreachable base_url makes every forwarded request fail");

        assert!(
            !matches!(error, LiterLlmError::HookRejected { .. }),
            "nothing may be blocked when no guardrails are configured: {error:?}"
        );
    }

    /// Fail closed: a guardrail that cannot be constructed must abort pool
    /// construction. `ProxyServer::serve_with_shutdown` propagates this
    /// `Result` with `?` before binding the listener, so the proxy exits
    /// rather than serving with a partial or empty guardrail set.
    #[test]
    fn malformed_guardrail_fails_pool_construction() {
        let config = guarded_config(
            r#"
[[guardrails]]
type = "regex"
name = "broken"
pattern = '([unclosed'
stages = ["input"]
action = { kind = "redact", replacement = "x" }
"#,
        )
        .expect("the TOML itself is valid; the regex is not");

        // ~keep Matched rather than `expect_err`, which would require `ServicePool: Debug`.
        // ~keep The pool holds `ProxyConfig`, whose Debug derive was removed in e5d2f79a0 so a
        // ~keep virtual key token could never reach a debug-formatted span or panic message.
        let error = match ServicePool::from_config(&config, None) {
            Err(error) => error,
            Ok(_) => panic!("an unbuildable guardrail must abort startup, not degrade"),
        };
        assert!(error.contains("broken"), "error should name the entry: {error}");
    }

    /// A guardrail that fails to build must not be skipped in favour of the
    /// ones that do build — the pool refuses rather than running a subset.
    #[test]
    fn one_malformed_guardrail_rejects_the_whole_set() {
        let config = guarded_config(
            r#"
[[guardrails]]
type = "prompt_injection"
name = "good"

[[guardrails]]
type = "regex"
name = "broken"
pattern = '([unclosed'
stages = ["input"]
action = { kind = "redact", replacement = "x" }
"#,
        )
        .expect("the TOML itself is valid; the regex is not");

        assert!(
            ServicePool::from_config(&config, None).is_err(),
            "a partially-buildable guardrail set must abort startup"
        );
    }

    /// The pool's registry is what `AppState` hands the realtime path, so it
    /// must actually carry the configured entries.
    #[test]
    fn pool_exposes_the_configured_guardrail_set() {
        let config = guarded_config(SSN_GUARDRAIL).expect("valid TOML");
        let pool = ServicePool::from_config(&config, None).expect("pool should build");

        let registry = pool.guardrails();
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.iter().next().expect("one guardrail").name(), "block-ssn");
    }
    #[test]
    fn should_defer_missing_provider_credentials_until_requests() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "service_pool::tests::missing_provider_credentials_request_probe",
                "--ignored",
            ])
            .env_remove("OPENAI_API_KEY")
            .env_remove("AZURE_OPENAI_ENDPOINT")
            .env_remove("AZURE_ENDPOINT")
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stdout));
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
    }

    #[tokio::test]
    #[ignore = "invoked with an isolated environment by should_defer_missing_provider_credentials_until_requests"]
    async fn missing_provider_credentials_request_probe() {
        use tower::ServiceExt;
        let config =
            ProxyConfig::from_toml_str("[[models]]\nname = 'gpt'\nprovider_model = 'openai/gpt-4o'\n").unwrap();
        let pool = ServicePool::from_config(&config, None).expect("missing credentials must not prevent startup");
        assert_eq!(pool.model_names(), vec!["gpt"]);
        verify_classifier_credentials_request_only().await;
        let malformed =
            ProxyConfig::from_toml_str("[[models]]\nname = 'broken'\nprovider_model = 'azure/gpt-4o'\n").unwrap();
        let error = ServicePool::from_config(&malformed, None)
            .err()
            .expect("malformed providers must still fail startup");
        assert!(error.contains("requires a base URL"));
        let embedding = serde_json::from_value(serde_json::json!({"model": "gpt", "input": "probe"})).unwrap();
        for request in [chat_with("probe"), LlmRequest::Embed(embedding)] {
            let error = pool.get_service("gpt").unwrap().oneshot(request).await.unwrap_err();
            assert!(matches!(error, LiterLlmError::Authentication { status: 401, .. }));
            assert!(error.to_string().contains("OPENAI_API_KEY"));
        }
    }
    #[tokio::test]
    async fn should_report_missing_credentials_before_contacting_upstream() {
        use tower::ServiceExt;
        let upstream = wiremock::MockServer::start().await;
        let config = ClientConfigBuilder::new("")
            .base_url(upstream.uri())
            .load_env(false)
            .credential_provider(Arc::new(MissingProviderCredentials {
                message: "configure OPENAI_API_KEY".into(),
                status: 401,
            }))
            .build();
        let client = DefaultClient::new(config, Some("openai/gpt-4o")).unwrap();
        let embedding = serde_json::from_value(serde_json::json!({"model": "gpt", "input": "probe"})).unwrap();
        for request in [chat_with("probe"), LlmRequest::Embed(embedding)] {
            let error = LlmService::new_from_arc(Arc::new(client.clone()))
                .oneshot(request)
                .await
                .unwrap_err();
            assert!(matches!(error, LiterLlmError::Authentication { status: 401, .. }));
        }
        assert_eq!(upstream.received_requests().await.unwrap().len(), 0);
    }

    async fn verify_classifier_credentials_request_only() {
        use tower::ServiceExt;
        let classifier = semantic_classifier_config();
        let malformed = build_classifier_client(None, None, "azure/gpt-4o", &classifier)
            .err()
            .expect("malformed classifier providers must still fail startup");
        assert!(malformed.contains("requires a base URL"));
        let client = build_classifier_client(None, None, "openai/text-embedding-3-small", &classifier)
            .expect("missing classifier credentials must not prevent startup");
        let request =
            serde_json::from_value(serde_json::json!({"model": "openai/text-embedding-3-small", "input": "probe"}))
                .unwrap();
        let error = LlmService::new_from_arc(Arc::new(client))
            .oneshot(LlmRequest::Embed(request))
            .await
            .unwrap_err();
        assert!(matches!(error, LiterLlmError::Authentication { status: 401, .. }));
        assert!(error.to_string().contains("OPENAI_API_KEY"));
        assert!(matches!(
            build_routing_strategy(&classifier).unwrap(),
            RoutingStrategy::Semantic(_)
        ));
        let upstream = wiremock::MockServer::start().await;
        let mut routed = classifier.clone();
        let mut deployment = config_with_one_model().models.remove(0);
        deployment.name = "gpt".into();
        deployment.base_url = Some(upstream.uri());
        routed.models = vec![deployment.clone(), deployment];
        let routed_pool =
            ServicePool::from_config(&routed, None).expect("classifier credentials may be absent at startup");
        let LlmRequestKind::Chat(chat) = chat_with("probe").kind else {
            unreachable!()
        };
        for request in [LlmRequest::Chat(chat.clone()), LlmRequest::ChatStream(chat)] {
            let error = routed_pool
                .get_service("gpt")
                .unwrap()
                .oneshot(request)
                .await
                .unwrap_err();
            assert!(matches!(error, LiterLlmError::Authentication { status: 401, .. }));
            assert!(error.to_string().contains("OPENAI_API_KEY"));
        }
        assert_eq!(upstream.received_requests().await.unwrap().len(), 0);
        verify_classifier_bypass_routes(&routed, &upstream).await;
    }

    async fn verify_classifier_bypass_routes(routed: &ProxyConfig, upstream: &wiremock::MockServer) {
        use tower::ServiceExt;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({"object": "list", "data": []})),
            )
            .mount(upstream)
            .await;
        let pool = ServicePool::from_config(routed, None).unwrap();
        assert!(
            pool.get_service("gpt")
                .unwrap()
                .oneshot(LlmRequest::ListModels())
                .await
                .is_ok()
        );
        assert_eq!(upstream.received_requests().await.unwrap().len(), 1);
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "probe", "object": "chat.completion", "created": 0, "model": "gpt-4o",
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
            })))
            .mount(upstream)
            .await;
        let mut single = routed.clone();
        single.models.truncate(1);
        let pool = ServicePool::from_config(&single, None).unwrap();
        assert!(
            pool.get_service("gpt")
                .unwrap()
                .oneshot(chat_with("probe"))
                .await
                .is_ok()
        );
        assert_eq!(upstream.received_requests().await.unwrap().len(), 2);
    }
    fn semantic_classifier_config() -> ProxyConfig {
        ProxyConfig::from_toml_str(
            r#"
[routing]
strategy = "semantic"
[routing.classifier]
kind = "embedding"
embedding_model = "openai/text-embedding-3-small"
threshold = 0.75
[[routing.classifier.prototypes]]
name = "coding"
model = "gpt"
embedding = [1.0, 0.0, 0.0]
"#,
        )
        .unwrap()
    }
}
