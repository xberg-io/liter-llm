//! A managed LLM client that optionally routes requests through a Tower
//! middleware stack (cache, budget, hooks, cooldown, rate limiting, health
//! checks, cost tracking, tracing) when the corresponding [`ClientConfig`]
//! fields are set.
//!
//! When no middleware is configured the client delegates directly to the
//! underlying [`DefaultClient`], adding zero overhead.  When middleware *is*
//! configured, each [`LlmClient`] method converts its typed request into an
//! [`LlmRequest`], sends it through a cloned Tower service stack, and extracts
//! the typed response from the resulting [`LlmResponse`].
//!
//! # Tower `Service::call` takes `&mut self`
//!
//! The [`LlmClient`] trait requires `&self` receivers but Tower's
//! `Service::call` takes `&mut self`.  All our middleware services are `Clone`
//! (state is behind `Arc`) so we clone the service per call — this is a cheap
//! series of `Arc` reference-count bumps.
//!
//! Tower's [`BoxCloneService`](tower::util::BoxCloneService) is `Send` but not
//! `Sync` (its inner trait object is `dyn ... + Send`).  Since [`LlmClient`]
//! requires `Sync`, we wrap the service in a [`std::sync::Mutex`] that is held
//! only for the brief duration of `Clone::clone` (a few `Arc` ref-count bumps).
//! This makes `ManagedClient` `Sync` with negligible contention.

use std::sync::{Arc, Mutex};

use tower::{Layer, Service};

use super::config::ClientConfig;
use super::{BatchClient, BoxFuture, BoxStream, DefaultClient, FileClient, LlmClient, ResponseClient};
use crate::error::{LiterLlmError, Result};
#[cfg(feature = "opendal-cache")]
use crate::tower::OpenDalCacheStore;
use crate::tower::types::{LlmRequest, LlmResponse};
use crate::tower::{
    BudgetLayer, BudgetState, CacheBackend, CacheLayer, CooldownLayer, CostTrackingLayer, HealthCheckLayer, HooksLayer,
    LlmService, ModelRateLimitLayer, TracingLayer,
};
use crate::types::audio::{CreateSpeechRequest, CreateTranscriptionRequest, TranscriptionResponse};
use crate::types::batch::{BatchListQuery, BatchListResponse, BatchObject, CreateBatchRequest};
use crate::types::files::{CreateFileRequest, DeleteResponse, FileListQuery, FileListResponse, FileObject};
use crate::types::image::{CreateImageRequest, ImagesResponse};
use crate::types::moderation::{ModerationRequest, ModerationResponse};
use crate::types::ocr::{OcrRequest, OcrResponse};
use crate::types::rerank::{RerankRequest, RerankResponse};
use crate::types::responses::{CreateResponseRequest, ResponseObject};
use crate::types::search::{SearchRequest, SearchResponse};
use crate::types::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse,
    ModelsListResponse,
};

/// A `Send + Sync` wrapper around [`tower::util::BoxCloneService`].
///
/// `BoxCloneService` is `Send` but not `Sync` because its inner trait object
/// only requires `Send`.  All our concrete middleware services *are* `Sync`
/// (they store shared state behind `Arc`), so wrapping in a `Mutex` is safe
/// and incurs negligible overhead — the lock is held only for the duration of
/// `Clone::clone` (a handful of `Arc` ref-count bumps).
struct SyncService {
    inner: Mutex<tower::util::BoxCloneService<LlmRequest, LlmResponse, LiterLlmError>>,
}

impl SyncService {
    /// Clone the inner service out of the mutex, returning an owned mutable
    /// service that can be `.call()`-ed.
    ///
    /// If a previous call panicked while holding the lock, the mutex is
    /// poisoned.  Recovery is safe here because the lock guards only the
    /// `Clone::clone` step — the inner `BoxCloneService` state is unchanged
    /// by a panic during cloning.
    fn clone_service(&self) -> tower::util::BoxCloneService<LlmRequest, LlmResponse, LiterLlmError> {
        match self.inner.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }
}

/// A managed LLM client that wraps [`DefaultClient`] with optional Tower
/// middleware (cache, cooldown, rate limiting, health checks, cost tracking,
/// budget, hooks, tracing).
///
/// Construct via [`ManagedClient::new`].  If the provided [`ClientConfig`]
/// contains any middleware configuration the corresponding Tower layers are
/// composed into a service stack.  Otherwise requests pass straight through
/// to the inner [`DefaultClient`].
///
/// `ManagedClient` implements [`LlmClient`] and can be used everywhere a
/// `DefaultClient` is expected.
#[cfg_attr(alef, alef(skip))]
pub struct ManagedClient {
    /// The raw client — used directly when no middleware is configured, and
    /// also wrapped by the Tower service when middleware *is* configured.
    inner: Arc<DefaultClient>,

    /// When `Some`, requests are routed through this Tower service stack
    /// instead of going directly to `inner`.
    service: Option<SyncService>,

    /// Budget state handle, exposed so callers can query accumulated spend.
    /// `None` when no budget middleware is configured.
    budget_state: Option<Arc<BudgetState>>,
}

// ~keep SAFETY: `SyncService` wraps a `Mutex<BoxCloneService>` which is `Send + Sync`.
// ~keep `Arc<DefaultClient>` and `Arc<BudgetState>` are both `Send + Sync`.

impl ManagedClient {
    /// Build a managed client.
    ///
    /// `model_hint` guides provider auto-detection — see
    /// [`DefaultClient::new`] for details.
    ///
    /// If the config contains any middleware settings (cache, budget, hooks,
    /// cooldown, rate limit, health check, cost tracking, tracing) the
    /// corresponding Tower layers are composed into a service stack.
    /// Otherwise requests pass straight through to the inner client.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying [`DefaultClient`] cannot be
    /// constructed (e.g. invalid headers or HTTP client build failure).
    pub fn new(config: ClientConfig, model_hint: Option<&str>) -> Result<Self> {
        let client = DefaultClient::new(config.clone(), model_hint)?;
        let inner = Arc::new(client);

        let (service, budget_state) = build_service_stack(&config, Arc::clone(&inner));

        Ok(Self {
            inner,
            service,
            budget_state,
        })
    }

    /// Return a reference to the underlying [`DefaultClient`].
    #[must_use]
    pub fn inner(&self) -> &DefaultClient {
        &self.inner
    }

    /// Return the budget state handle, if budget middleware is configured.
    ///
    /// Use this to query accumulated spend at runtime.
    #[must_use]
    pub fn budget_state(&self) -> Option<&Arc<BudgetState>> {
        self.budget_state.as_ref()
    }

    /// Return `true` when middleware is active (requests go through the Tower
    /// service stack).
    #[must_use]
    pub fn has_middleware(&self) -> bool {
        self.service.is_some()
    }

    /// Clone the Tower service and call it with `req`, returning the raw
    /// [`LlmResponse`].
    fn call_service(&self, req: LlmRequest) -> BoxFuture<'static, Result<LlmResponse>> {
        let mut svc = match self.service.as_ref() {
            Some(s) => s.clone_service(),
            None => {
                return Box::pin(async {
                    Err(LiterLlmError::InternalError {
                        message: "call_service called without middleware stack".into(),
                    })
                });
            }
        };
        Box::pin(async move { svc.call(req).await })
    }
}

/// Inspect the config and, when at least one middleware option is set,
/// compose a Tower service stack wrapping the given client.
///
/// Returns `(Some(service), budget_state)` when middleware is configured,
/// or `(None, None)` when the config has no middleware.
fn build_service_stack(
    config: &ClientConfig,
    client: Arc<DefaultClient>,
) -> (Option<SyncService>, Option<Arc<BudgetState>>) {
    let has_cache = config.cache_config.is_some();
    let has_budget = config.budget_config.is_some();
    let has_hooks = !config.hooks.is_empty();
    let has_cooldown = config.cooldown_duration.is_some();
    let has_rate_limit = config.rate_limit_config.is_some();
    let has_health_check = config.health_check_interval.is_some();
    let has_cost = config.enable_cost_tracking;
    let has_tracing = config.enable_tracing;

    if !has_cache
        && !has_budget
        && !has_hooks
        && !has_cooldown
        && !has_rate_limit
        && !has_health_check
        && !has_cost
        && !has_tracing
    {
        return (None, None);
    }

    let base = LlmService::new_from_arc(client);

    let mut budget_state: Option<Arc<BudgetState>> = None;

    type Bcs = tower::util::BoxCloneService<LlmRequest, LlmResponse, LiterLlmError>;

    let svc: Bcs = tower::util::BoxCloneService::new(base);

    let svc = if let Some(ref cache_cfg) = config.cache_config {
        let layer = if let Some(ref store) = config.cache_store {
            CacheLayer::with_store(Arc::clone(store))
        } else {
            match &cache_cfg.backend {
                CacheBackend::Memory => CacheLayer::new(cache_cfg.clone()),
                #[cfg(feature = "opendal-cache")]
                CacheBackend::OpenDal {
                    scheme,
                    config: backend_config,
                } => {
                    match OpenDalCacheStore::from_config(scheme, backend_config.clone(), "llm-cache/", cache_cfg.ttl) {
                        Ok(store) => CacheLayer::with_store(Arc::new(store)),
                        Err(e) => {
                            tracing::warn!("Failed to create OpenDAL cache store, falling back to in-memory: {e}");
                            CacheLayer::new(cache_cfg.clone())
                        }
                    }
                }
            }
        };
        tower::util::BoxCloneService::new(layer.layer(svc))
    } else {
        svc
    };

    let svc = if let Some(interval) = config.health_check_interval {
        let layer = HealthCheckLayer::new(interval);
        tower::util::BoxCloneService::new(layer.layer(svc))
    } else {
        svc
    };

    let svc = if let Some(duration) = config.cooldown_duration {
        let layer = CooldownLayer::new(duration);
        tower::util::BoxCloneService::new(layer.layer(svc))
    } else {
        svc
    };

    let svc = if let Some(ref rl_cfg) = config.rate_limit_config {
        let layer = ModelRateLimitLayer::new(rl_cfg.clone());
        tower::util::BoxCloneService::new(layer.layer(svc))
    } else {
        svc
    };

    let svc = if has_cost {
        tower::util::BoxCloneService::new(CostTrackingLayer.layer(svc))
    } else {
        svc
    };

    let svc = if let Some(ref budget_cfg) = config.budget_config {
        let state = Arc::new(BudgetState::new());
        budget_state = Some(Arc::clone(&state));
        let layer = BudgetLayer::new(budget_cfg.clone(), state);
        tower::util::BoxCloneService::new(layer.layer(svc))
    } else {
        svc
    };

    let svc = if has_hooks {
        let layer = HooksLayer::new(config.hooks.clone());
        tower::util::BoxCloneService::new(layer.layer(svc))
    } else {
        svc
    };

    let svc = if has_tracing {
        tower::util::BoxCloneService::new(TracingLayer.layer(svc))
    } else {
        svc
    };

    (Some(SyncService { inner: Mutex::new(svc) }), budget_state)
}

impl LlmClient for ManagedClient {
    fn chat(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>> {
        if self.service.is_none() {
            return self.inner.chat(req);
        }
        let fut = self.call_service(LlmRequest::Chat(req));
        Box::pin(async move {
            match fut.await? {
                LlmResponse::Chat(r) => Ok(r),
                other => Err(LiterLlmError::InternalError {
                    message: format!("expected Chat response, got {other:?}"),
                }),
            }
        })
    }

    fn chat_stream(
        &self,
        req: ChatCompletionRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>> {
        if self.service.is_none() {
            return self.inner.chat_stream(req);
        }
        let fut = self.call_service(LlmRequest::ChatStream(req));
        Box::pin(async move {
            match fut.await? {
                LlmResponse::ChatStream(s) => Ok(s),
                other => Err(LiterLlmError::InternalError {
                    message: format!("expected ChatStream response, got {other:?}"),
                }),
            }
        })
    }

    fn embed(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>> {
        if self.service.is_none() {
            return self.inner.embed(req);
        }
        let fut = self.call_service(LlmRequest::Embed(req));
        Box::pin(async move {
            match fut.await? {
                LlmResponse::Embed(r) => Ok(r),
                other => Err(LiterLlmError::InternalError {
                    message: format!("expected Embed response, got {other:?}"),
                }),
            }
        })
    }

    fn list_models(&self) -> BoxFuture<'_, Result<ModelsListResponse>> {
        if self.service.is_none() {
            return self.inner.list_models();
        }
        let fut = self.call_service(LlmRequest::ListModels());
        Box::pin(async move {
            match fut.await? {
                LlmResponse::ListModels(r) => Ok(r),
                other => Err(LiterLlmError::InternalError {
                    message: format!("expected ListModels response, got {other:?}"),
                }),
            }
        })
    }

    fn image_generate(&self, req: CreateImageRequest) -> BoxFuture<'_, Result<ImagesResponse>> {
        if self.service.is_none() {
            return self.inner.image_generate(req);
        }
        let fut = self.call_service(LlmRequest::ImageGenerate(req));
        Box::pin(async move {
            match fut.await? {
                LlmResponse::ImageGenerate(r) => Ok(r),
                other => Err(LiterLlmError::InternalError {
                    message: format!("expected ImageGenerate response, got {other:?}"),
                }),
            }
        })
    }

    fn speech(&self, req: CreateSpeechRequest) -> BoxFuture<'_, Result<bytes::Bytes>> {
        if self.service.is_none() {
            return self.inner.speech(req);
        }
        let fut = self.call_service(LlmRequest::Speech(req));
        Box::pin(async move {
            match fut.await? {
                LlmResponse::Speech(r) => Ok(r),
                other => Err(LiterLlmError::InternalError {
                    message: format!("expected Speech response, got {other:?}"),
                }),
            }
        })
    }

    fn transcribe(&self, req: CreateTranscriptionRequest) -> BoxFuture<'_, Result<TranscriptionResponse>> {
        if self.service.is_none() {
            return self.inner.transcribe(req);
        }
        let fut = self.call_service(LlmRequest::Transcribe(req));
        Box::pin(async move {
            match fut.await? {
                LlmResponse::Transcribe(r) => Ok(r),
                other => Err(LiterLlmError::InternalError {
                    message: format!("expected Transcribe response, got {other:?}"),
                }),
            }
        })
    }

    fn moderate(&self, req: ModerationRequest) -> BoxFuture<'_, Result<ModerationResponse>> {
        if self.service.is_none() {
            return self.inner.moderate(req);
        }
        let fut = self.call_service(LlmRequest::Moderate(req));
        Box::pin(async move {
            match fut.await? {
                LlmResponse::Moderate(r) => Ok(r),
                other => Err(LiterLlmError::InternalError {
                    message: format!("expected Moderate response, got {other:?}"),
                }),
            }
        })
    }

    fn rerank(&self, req: RerankRequest) -> BoxFuture<'_, Result<RerankResponse>> {
        if self.service.is_none() {
            return self.inner.rerank(req);
        }
        let fut = self.call_service(LlmRequest::Rerank(req));
        Box::pin(async move {
            match fut.await? {
                LlmResponse::Rerank(r) => Ok(r),
                other => Err(LiterLlmError::InternalError {
                    message: format!("expected Rerank response, got {other:?}"),
                }),
            }
        })
    }

    fn search(&self, req: SearchRequest) -> BoxFuture<'_, Result<SearchResponse>> {
        if self.service.is_none() {
            return self.inner.search(req);
        }
        let fut = self.call_service(LlmRequest::Search(req));
        Box::pin(async move {
            match fut.await? {
                LlmResponse::Search(r) => Ok(r),
                other => Err(LiterLlmError::InternalError {
                    message: format!("expected Search response, got {other:?}"),
                }),
            }
        })
    }

    fn ocr(&self, req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>> {
        if self.service.is_none() {
            return self.inner.ocr(req);
        }
        let fut = self.call_service(LlmRequest::Ocr(req));
        Box::pin(async move {
            match fut.await? {
                LlmResponse::Ocr(r) => Ok(r),
                other => Err(LiterLlmError::InternalError {
                    message: format!("expected Ocr response, got {other:?}"),
                }),
            }
        })
    }
}

// ~keep File operations bypass Tower middleware because they are administrative, not model inference.

impl FileClient for ManagedClient {
    fn create_file(&self, req: CreateFileRequest) -> BoxFuture<'_, Result<FileObject>> {
        self.inner.create_file(req)
    }

    fn retrieve_file(&self, file_id: &str) -> BoxFuture<'_, Result<FileObject>> {
        self.inner.retrieve_file(file_id)
    }

    fn delete_file(&self, file_id: &str) -> BoxFuture<'_, Result<DeleteResponse>> {
        self.inner.delete_file(file_id)
    }

    fn list_files(&self, query: Option<FileListQuery>) -> BoxFuture<'_, Result<FileListResponse>> {
        self.inner.list_files(query)
    }

    fn file_content(&self, file_id: &str) -> BoxFuture<'_, Result<bytes::Bytes>> {
        self.inner.file_content(file_id)
    }
}

impl BatchClient for ManagedClient {
    fn create_batch(&self, req: CreateBatchRequest) -> BoxFuture<'_, Result<BatchObject>> {
        self.inner.create_batch(req)
    }

    fn retrieve_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>> {
        self.inner.retrieve_batch(batch_id)
    }

    fn list_batches(&self, query: Option<BatchListQuery>) -> BoxFuture<'_, Result<BatchListResponse>> {
        self.inner.list_batches(query)
    }

    fn cancel_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>> {
        self.inner.cancel_batch(batch_id)
    }
}

impl ResponseClient for ManagedClient {
    fn create_response(&self, req: CreateResponseRequest) -> BoxFuture<'_, Result<ResponseObject>> {
        self.inner.create_response(req)
    }

    fn retrieve_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>> {
        self.inner.retrieve_response(response_id)
    }

    fn cancel_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>> {
        self.inner.cancel_response(response_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::ClientConfigBuilder;

    /// Verify that `ManagedClient` with no middleware config has no service
    /// stack and `has_middleware()` returns false.
    #[test]
    fn no_middleware_when_config_is_plain() {
        let config = ClientConfig::new("test-key");
        let client = ManagedClient::new(config, None).expect("should build");
        assert!(!client.has_middleware());
        assert!(client.budget_state().is_none());
    }

    /// Verify that adding a cache config activates middleware.
    #[test]
    fn middleware_active_with_cache_config() {
        use crate::tower::CacheConfig;
        let config = ClientConfigBuilder::new("test-key")
            .cache(CacheConfig::default())
            .build();
        let client = ManagedClient::new(config, None).expect("should build");
        assert!(client.has_middleware());
    }

    /// Verify that adding a budget config activates middleware and exposes
    /// budget state.
    #[test]
    fn middleware_active_with_budget_config() {
        use crate::tower::BudgetConfig;
        let config = ClientConfigBuilder::new("test-key")
            .budget(BudgetConfig::default())
            .build();
        let client = ManagedClient::new(config, None).expect("should build");
        assert!(client.has_middleware());
        assert!(client.budget_state().is_some());
    }

    /// Verify that cooldown configuration activates middleware.
    #[test]
    fn middleware_active_with_cooldown() {
        use std::time::Duration;
        let config = ClientConfigBuilder::new("test-key")
            .cooldown(Duration::from_secs(30))
            .build();
        let client = ManagedClient::new(config, None).expect("should build");
        assert!(client.has_middleware());
    }

    /// Verify that tracing configuration activates middleware.
    #[test]
    fn middleware_active_with_tracing() {
        let config = ClientConfigBuilder::new("test-key").tracing(true).build();
        let client = ManagedClient::new(config, None).expect("should build");
        assert!(client.has_middleware());
    }

    /// Verify that cost tracking configuration activates middleware.
    #[test]
    fn middleware_active_with_cost_tracking() {
        let config = ClientConfigBuilder::new("test-key").cost_tracking(true).build();
        let client = ManagedClient::new(config, None).expect("should build");
        assert!(client.has_middleware());
    }

    /// Verify that tracing=false alone does not activate middleware.
    #[test]
    fn no_middleware_when_tracing_false() {
        let config = ClientConfigBuilder::new("test-key")
            .tracing(false)
            .cost_tracking(false)
            .build();
        let client = ManagedClient::new(config, None).expect("should build");
        assert!(!client.has_middleware());
    }

    /// Verify the poisoned-mutex recovery pattern used in `clone_service`.
    ///
    /// This documents and proves that `Err(PoisonError).into_inner()` safely
    /// recovers the guarded value after a panic while holding the lock.
    #[test]
    fn poisoned_mutex_recovers_clone() {
        use std::sync::Arc;
        use std::sync::Mutex;
        use std::thread;

        let m = Arc::new(Mutex::new(String::from("inner")));
        let m2 = Arc::clone(&m);

        let _ = thread::spawn(move || {
            let _guard = m2.lock().expect("acquiring test mutex should succeed");
            panic!("intentional panic to poison the mutex");
        })
        .join();

        assert!(m.is_poisoned(), "mutex should be poisoned after thread panic");

        let cloned = match m.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };

        assert_eq!(cloned, "inner", "recovered value must match original");
    }
}
