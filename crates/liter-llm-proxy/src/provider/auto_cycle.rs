//! Auto-cycle Tower layer: rotates credentials on 429 / 5xx rate-limit responses.
//!
//! # How it works
//!
//! [`AutoCycleLayer`] wraps an inner `Service<LlmRequest>` and intercepts
//! [`liter_llm::error::LiterLlmError::RateLimited`] and
//! [`liter_llm::error::LiterLlmError::ServerError`] / `ServiceUnavailable`
//! (5xx) responses:
//!
//! 1. On a rate-limit or 5xx error for provider `P`, it calls
//!    [`CredentialPool::mark_exhausted`] with the `Retry-After` cooldown
//!    (or a configurable default of 60 s).
//! 2. The next credential from [`CredentialPool::current`] is passed to the
//!    `ServiceFactory` to produce a fresh inner service wired with the new key.
//! 3. If all credentials are exhausted, [`liter_llm::error::LiterLlmError::ServiceUnavailable`]
//!    is returned immediately — this **is** a transient error and will trip the
//!    Phase 1 circuit breaker only if it happens `N` consecutive times.
//!    A single rotation (one credential exhausted, another active) does NOT
//!    record a circuit failure because the retry call succeeds.
//!
//! # Model allowlist enforcement
//!
//! `next_credential_for_model` scans past any credential whose
//! `model_allowlist` does not include the requested model.
//!
//! # Integration point
//!
//! `AutoCycleLayer` sits between the circuit breaker and the base `LlmService`
//! in the Tower stack built by [`crate::service_pool`].  The circuit breaker
//! sees:
//!
//! - A successful response (rotation transparent): no failure recorded.
//! - `ServiceUnavailable` from all-exhausted: one failure recorded.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use tower::{Layer, Service};

use liter_llm::error::{LiterLlmError, Result};
use liter_llm::tower::types::{LlmRequest, LlmResponse};

use super::credential_pool::{CredentialError, CredentialHandle, CredentialPool};

/// Default cooldown when no `Retry-After` header is present.
const DEFAULT_COOLDOWN: Duration = Duration::from_secs(60);

/// A factory that produces a new `tower::Service<LlmRequest>` for a given
/// API key.  Used by [`AutoCycleService`] to rebuild the inner service when a
/// credential is rotated.
///
/// The factory must be `Send + Sync + 'static` so it can be shared across
/// Tokio tasks.  The returned service must also be `Send + 'static`.
pub trait ServiceFactory: Send + Sync + 'static {
    /// The concrete service type produced by this factory.
    type Service: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + Clone + 'static;

    /// Build a new service wired with the supplied API key.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError` if the underlying client cannot be constructed
    /// (e.g. invalid header).
    fn build(&self, api_key: &secrecy::SecretString) -> Result<Self::Service>;
}

/// Tower [`Layer`] that wraps a service with credential auto-cycling.
pub struct AutoCycleLayer<P, F> {
    pool: Arc<P>,
    factory: Arc<F>,
    /// Human-readable provider label (e.g. `"openai"`).
    provider: String,
}

impl<P: CredentialPool, F: ServiceFactory> AutoCycleLayer<P, F> {
    /// Create a new auto-cycle layer.
    #[must_use]
    pub fn new(pool: Arc<P>, factory: Arc<F>, provider: impl Into<String>) -> Self {
        Self {
            pool,
            factory,
            provider: provider.into(),
        }
    }
}

impl<P: CredentialPool, F: ServiceFactory, S> Layer<S> for AutoCycleLayer<P, F>
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + Clone + 'static,
    S::Future: Send + 'static,
{
    type Service = AutoCycleService<P, F, S>;

    fn layer(&self, inner: S) -> Self::Service {
        AutoCycleService {
            inner,
            pool: Arc::clone(&self.pool),
            factory: Arc::clone(&self.factory),
            provider: self.provider.clone(),
        }
    }
}

/// Tower service produced by [`AutoCycleLayer`].
pub struct AutoCycleService<P, F, S> {
    inner: S,
    pool: Arc<P>,
    factory: Arc<F>,
    provider: String,
}

impl<P: CredentialPool, F: ServiceFactory, S: Clone> Clone for AutoCycleService<P, F, S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            pool: Arc::clone(&self.pool),
            factory: Arc::clone(&self.factory),
            provider: self.provider.clone(),
        }
    }
}

impl<P, F, S> Service<LlmRequest> for AutoCycleService<P, F, S>
where
    P: CredentialPool + 'static,
    F: ServiceFactory + 'static,
    F::Service: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + Clone + 'static,
    <F::Service as Service<LlmRequest>>::Future: Send + 'static,
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + Clone + 'static,
    S::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = Pin<Box<dyn Future<Output = Result<LlmResponse>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        let pool = Arc::clone(&self.pool);
        let factory = Arc::clone(&self.factory);
        let provider = self.provider.clone();
        let model = req.model().unwrap_or("").to_owned();

        // ~keep Tower readiness contract: consume the polled-ready instance.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let initial_handle = pool.current(&provider).await.ok();

            let first_result = inner.call(req.clone()).await;

            match first_result {
                Ok(resp) => Ok(resp),
                Err(err) if needs_rotation(&err) => {
                    let cooldown = cooldown_from_error(&err);

                    if let Some(ref handle) = initial_handle {
                        pool.mark_exhausted(&provider, handle, cooldown).await;
                    }

                    let next_handle = next_credential_for_model(&pool, &provider, &model).await;

                    match next_handle {
                        Some(handle) => {
                            let mut rotated_svc = factory.build(&handle.api_key)?;

                            // ~keep poll_ready still runs on rotated services so permit-based layers stay correct.
                            std::future::poll_fn(|cx| rotated_svc.poll_ready(cx)).await?;

                            rotated_svc.call(req).await
                        }
                        None => Err(LiterLlmError::ServiceUnavailable {
                            message: format!(
                                "all credentials exhausted for provider '{provider}' \
                                     (model: '{model}')"
                            ),
                            status: 503,
                        }),
                    }
                }
                Err(other) => Err(other),
            }
        })
    }
}

/// Return `true` for errors that should trigger a credential rotation.
///
/// - `RateLimited` (429): quota exhausted on this key.
/// - `ServerError` / `ServiceUnavailable` (5xx): may indicate per-key limits.
///
/// Auth errors (401/403) are excluded — bad keys cannot be fixed by rotation.
fn needs_rotation(err: &LiterLlmError) -> bool {
    matches!(
        err,
        LiterLlmError::RateLimited { .. }
            | LiterLlmError::ServerError { .. }
            | LiterLlmError::ServiceUnavailable { .. }
    )
}

/// Extract the cooldown duration from a `RateLimited` error's `retry_after`
/// field, or fall back to [`DEFAULT_COOLDOWN`].
fn cooldown_from_error(err: &LiterLlmError) -> Duration {
    if let LiterLlmError::RateLimited {
        retry_after: Some(d), ..
    } = err
    {
        *d
    } else {
        DEFAULT_COOLDOWN
    }
}

/// Return the next credential that satisfies the model allowlist, or `None`
/// when all available credentials either are exhausted or disallow the model.
///
/// Polls the pool in a loop, skipping any credential with a `model_allowlist`
/// that does not include `model`.  Each disallowed credential is temporarily
/// parked with a 1-second cooldown so the cursor advances.
async fn next_credential_for_model<P: CredentialPool>(
    pool: &Arc<P>,
    provider: &str,
    model: &str,
) -> Option<CredentialHandle> {
    const MAX_ATTEMPTS: usize = 128;

    for _ in 0..MAX_ATTEMPTS {
        match pool.current(provider).await {
            Ok(handle) => {
                if handle
                    .model_allowlist
                    .as_ref()
                    .is_some_and(|allowlist| !allowlist.iter().any(|m| m == model))
                {
                    pool.mark_exhausted(provider, &handle, Duration::from_secs(1)).await;
                    continue;
                }
                return Some(handle);
            }
            Err(CredentialError::AllExhausted | CredentialError::PoolEmpty) => return None,
            Err(_) => return None,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll};
    use std::time::Duration;

    use tower::{Layer as _, Service as _};

    use liter_llm::client::BoxFuture;
    use liter_llm::error::{LiterLlmError, Result};
    use liter_llm::tower::types::{LlmRequest, LlmResponse};
    use liter_llm::types::chat::ChatCompletionRequest;
    use liter_llm::types::common::{AssistantMessage, Message};

    use crate::provider::credential_pool_memory::InMemoryCredentialPool;

    use super::*;

    fn chat_req(model: &str) -> LlmRequest {
        use liter_llm::types::common::{UserContent, UserMessage};
        LlmRequest::Chat(ChatCompletionRequest {
            model: model.to_owned(),
            messages: vec![Message::User(UserMessage {
                content: UserContent::Text("hello".into()),
                name: None,
            })],
            ..Default::default()
        })
    }

    fn pool_with_keys(provider: &str, keys: &[(&str, Option<Vec<String>>)]) -> Arc<InMemoryCredentialPool> {
        let p = Arc::new(InMemoryCredentialPool::new());
        for (i, (key, allowlist)) in keys.iter().enumerate() {
            p.add_credential(provider, format!("k{i}"), *key, allowlist.clone());
        }
        p
    }

    fn ok_response() -> LlmResponse {
        use liter_llm::types::chat::{ChatCompletionResponse, Choice, FinishReason};
        LlmResponse::Chat(ChatCompletionResponse {
            id: "test".into(),
            object: "chat.completion".into(),
            created: 0,
            model: "test-model".into(),
            choices: vec![Choice {
                index: 0,
                message: AssistantMessage {
                    content: Some("hi".into()),
                    ..Default::default()
                },
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: None,
            system_fingerprint: None,
            service_tier: None,
        })
    }

    type ScriptEntry = Box<dyn Fn() -> Result<LlmResponse> + Send + Sync>;

    /// An inner service that returns a preset sequence of results.
    #[derive(Clone)]
    struct ScriptedService {
        script: Arc<Mutex<Vec<ScriptEntry>>>,
        call_count: Arc<AtomicU32>,
    }

    impl ScriptedService {
        fn ok_entries(n: usize) -> Vec<ScriptEntry> {
            (0..n)
                .map(|_| {
                    let f: ScriptEntry = Box::new(|| Ok(ok_response()));
                    f
                })
                .collect()
        }

        fn rate_limit_entry(retry_after: Option<Duration>) -> ScriptEntry {
            Box::new(move || {
                Err(LiterLlmError::RateLimited {
                    message: "rate limited".into(),
                    retry_after,
                })
            })
        }

        fn new(entries: Vec<ScriptEntry>) -> Self {
            Self {
                script: Arc::new(Mutex::new(entries)),
                call_count: Arc::new(AtomicU32::new(0)),
            }
        }
    }

    impl Service<LlmRequest> for ScriptedService {
        type Response = LlmResponse;
        type Error = LiterLlmError;
        type Future = BoxFuture<'static, Result<LlmResponse>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: LlmRequest) -> Self::Future {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let result = {
                let guard = self.script.lock().unwrap();
                if guard.is_empty() {
                    Ok(ok_response())
                } else if guard.len() == 1 {
                    (guard[0])()
                } else {
                    drop(guard);
                    let mut guard = self.script.lock().unwrap();
                    (guard.remove(0))()
                }
            };
            Box::pin(async move { result })
        }
    }

    /// A `ServiceFactory` that returns a scripted service with a given key recorder.
    #[derive(Clone)]
    struct ScriptedFactory {
        /// Records which API key was used in `build()`.
        last_key: Arc<Mutex<Option<String>>>,
        /// Counts calls made by factory-produced services.
        factory_call_count: Arc<AtomicU32>,
    }

    impl ScriptedFactory {
        fn new() -> Self {
            Self {
                last_key: Arc::new(Mutex::new(None)),
                factory_call_count: Arc::new(AtomicU32::new(0)),
            }
        }
    }

    impl ServiceFactory for ScriptedFactory {
        type Service = ScriptedService;

        fn build(&self, api_key: &secrecy::SecretString) -> Result<ScriptedService> {
            use secrecy::ExposeSecret;
            *self.last_key.lock().unwrap() = Some(api_key.expose_secret().to_owned());
            Ok(ScriptedService {
                script: Arc::new(Mutex::new(ScriptedService::ok_entries(10))),
                call_count: Arc::clone(&self.factory_call_count),
            })
        }
    }

    /// auto_cycle_on_429_retries_with_fresh_credential
    ///
    /// The initial call returns 429; the rotation should produce a service
    /// built with the second credential ("sk-b"), and the retry should succeed.
    #[tokio::test]
    async fn auto_cycle_on_429_retries_with_fresh_credential() {
        let pool = pool_with_keys("openai", &[("sk-a", None), ("sk-b", None)]);

        let factory = Arc::new(ScriptedFactory::new());
        let last_key = Arc::clone(&factory.last_key);
        let factory_call_count = Arc::clone(&factory.factory_call_count);

        let layer = AutoCycleLayer::new(Arc::clone(&pool), Arc::clone(&factory), "openai");

        let initial_inner =
            ScriptedService::new(vec![ScriptedService::rate_limit_entry(Some(Duration::from_millis(1)))]);
        let initial_call_count = Arc::clone(&initial_inner.call_count);

        let mut svc = layer.layer(initial_inner);

        let resp = svc
            .call(chat_req("openai/gpt-4o"))
            .await
            .expect("should succeed on retry");
        assert!(matches!(resp, LlmResponse::Chat(_)), "expected Chat response");

        assert_eq!(initial_call_count.load(Ordering::SeqCst), 1);
        assert_eq!(factory_call_count.load(Ordering::SeqCst), 1);

        let key = last_key.lock().unwrap().clone();
        assert_eq!(key.as_deref(), Some("sk-b"), "retry should use second credential");
    }

    /// auto_cycle_all_exhausted_surfaces_error
    #[tokio::test]
    async fn auto_cycle_all_exhausted_surfaces_error() {
        let pool = pool_with_keys("openai", &[("sk-only", None)]);
        let factory = Arc::new(ScriptedFactory::new());

        let layer = AutoCycleLayer::new(Arc::clone(&pool), Arc::clone(&factory), "openai");

        let initial_inner = ScriptedService::new(vec![ScriptedService::rate_limit_entry(None)]);

        let mut svc = layer.layer(initial_inner);
        let err = svc.call(chat_req("openai/gpt-4o")).await.expect_err("should fail");
        assert!(
            matches!(err, LiterLlmError::ServiceUnavailable { .. }),
            "expected ServiceUnavailable when all credentials exhausted, got {err:?}"
        );
    }

    /// auto_cycle_does_not_trip_circuit_on_single_exhaustion
    ///
    /// One 429 triggers rotation; the retry succeeds.  The circuit breaker
    /// must see only one successful call and remain Closed.
    #[tokio::test]
    async fn auto_cycle_does_not_trip_circuit_on_single_exhaustion() {
        use liter_llm::tower::CircuitPolicy as _;
        use liter_llm::tower::circuit::{CircuitLayer, CircuitState, ExponentialBackoffCircuit};

        let pool = pool_with_keys("openai", &[("sk-a", None), ("sk-b", None)]);
        let policy = Arc::new(ExponentialBackoffCircuit::new(3, Duration::from_millis(50)));
        let policy_ref = Arc::clone(&policy);

        let factory = Arc::new(ScriptedFactory::new());
        let auto_layer = AutoCycleLayer::new(Arc::clone(&pool), Arc::clone(&factory), "openai");
        let circuit_layer = CircuitLayer::new(Arc::clone(&policy), "openai");

        let initial_inner =
            ScriptedService::new(vec![ScriptedService::rate_limit_entry(Some(Duration::from_millis(1)))]);

        let auto_svc = auto_layer.layer(initial_inner);
        let mut svc = circuit_layer.layer(auto_svc);

        let resp = svc
            .call(chat_req("openai/gpt-4o"))
            .await
            .expect("should succeed after rotation");
        assert!(matches!(resp, LlmResponse::Chat(_)));

        assert_eq!(
            policy_ref.state(),
            CircuitState::Closed,
            "circuit should remain Closed after a single credential rotation"
        );
    }

    /// Credential A allows only "gpt-4o"; credential B allows "claude-3".
    /// next_credential_for_model("claude-3") should skip A and return B.
    #[tokio::test]
    async fn credential_pool_model_allowlist_skip() {
        let pool_arc = Arc::new(InMemoryCredentialPool::new());
        pool_arc.add_credential("multi", "k0", "sk-a", Some(vec!["gpt-4o".to_owned()]));
        pool_arc.add_credential("multi", "k1", "sk-b", Some(vec!["claude-3".to_owned()]));

        let handle = next_credential_for_model(&pool_arc, "multi", "claude-3")
            .await
            .expect("should find a credential that allows claude-3");

        assert_eq!(handle.id, "k1", "expected k1 (claude-3 allowlist), got {}", handle.id);
    }
}
