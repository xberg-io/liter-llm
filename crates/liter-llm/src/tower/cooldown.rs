//! Deployment cooldown middleware.
//!
//! [`CooldownLayer`] wraps a service and implements a cooldown period after
//! transient errors.  When the inner service returns a transient error (as
//! determined by [`LiterLlmError::is_transient`]), the service is marked as
//! cooling down for a configurable duration.  During the cooldown period,
//! incoming requests are immediately rejected with
//! [`LiterLlmError::ServiceUnavailable`] without calling the inner service.

use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tower::{Layer, Service};

use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};

struct CooldownState {
    /// `None` when not cooling down, `Some(start)` when a cooldown is active.
    cooldown_start: Option<Instant>,
}

/// Tower [`Layer`] that applies a cooldown period after transient errors.
#[cfg_attr(alef, alef(skip))]
pub struct CooldownLayer {
    duration: Duration,
}

impl CooldownLayer {
    /// Create a new cooldown layer.
    ///
    /// After a transient error, the wrapped service will reject all requests
    /// for `duration` before allowing traffic through again.
    #[must_use]
    pub fn new(duration: Duration) -> Self {
        Self { duration }
    }
}

impl<S> Layer<S> for CooldownLayer {
    type Service = CooldownService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CooldownService {
            inner,
            duration: self.duration,
            state: Arc::new(RwLock::new(CooldownState { cooldown_start: None })),
        }
    }
}

/// Tower service produced by [`CooldownLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct CooldownService<S> {
    inner: S,
    duration: Duration,
    state: Arc<RwLock<CooldownState>>,
}

impl<S: Clone> Clone for CooldownService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            duration: self.duration,
            state: Arc::clone(&self.state),
        }
    }
}

impl<S> Service<LlmRequest> for CooldownService<S>
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + Clone + 'static,
    S::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        let state = Arc::clone(&self.state);
        let duration = self.duration;
        let mut inner = self.inner.clone();

        Box::pin(async move {
            {
                let read = state.read().await;
                if let Some(start) = read.cooldown_start {
                    if start.elapsed() < duration {
                        return Err(LiterLlmError::ServiceUnavailable {
                            message: format!(
                                "service is cooling down for {:.0}s after a transient error",
                                duration.as_secs_f64()
                            ),
                            status: 503,
                        });
                    }
                    drop(read);
                    let mut write = state.write().await;
                    if let Some(s) = write.cooldown_start
                        && s.elapsed() >= duration
                    {
                        write.cooldown_start = None;
                    }
                }
            }

            match inner.call(req).await {
                Ok(resp) => Ok(resp),
                Err(e) if e.is_transient() => {
                    let mut write = state.write().await;
                    write.cooldown_start = Some(Instant::now());
                    Err(e)
                }
                Err(e) => Err(e),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use tower::{Layer as _, Service as _};

    use super::*;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::LlmRequest;

    #[tokio::test]
    async fn passes_through_on_success() {
        let layer = CooldownLayer::new(Duration::from_secs(10));
        let inner = LlmService::new(MockClient::ok());
        let mut svc = layer.layer(inner);

        let resp = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(resp.is_ok());
    }

    #[tokio::test]
    async fn enters_cooldown_after_transient_error() {
        let layer = CooldownLayer::new(Duration::from_secs(60));
        let inner = LlmService::new(MockClient::failing_timeout());
        let mut svc = layer.layer(inner);

        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should fail");
        assert!(matches!(err, LiterLlmError::Timeout));

        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should be in cooldown");
        assert!(
            matches!(err, LiterLlmError::ServiceUnavailable { .. }),
            "expected ServiceUnavailable during cooldown, got {err:?}"
        );
    }

    #[tokio::test]
    async fn cooldown_expires_after_duration() {
        let layer = CooldownLayer::new(Duration::from_millis(0));
        let inner = LlmService::new(MockClient::failing_timeout());
        let mut svc = layer.layer(inner);

        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should fail");

        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should fail");
        assert!(
            matches!(err, LiterLlmError::Timeout),
            "expected Timeout (cooldown expired), got {err:?}"
        );
    }

    #[tokio::test]
    async fn non_transient_error_does_not_trigger_cooldown() {
        let layer = CooldownLayer::new(Duration::from_secs(60));
        let inner = LlmService::new(MockClient::failing_auth());
        let mut svc = layer.layer(inner);

        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should fail");

        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should fail with auth, not cooldown");
        assert!(
            matches!(err, LiterLlmError::BadRequest { .. }),
            "expected BadRequest (auth), not ServiceUnavailable, got {err:?}"
        );
    }
}
