use std::task::{Context, Poll};

use tower::Layer;
use tower::Service;

use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};

/// Tower [`Layer`] that routes to a fallback service when the primary service
/// returns an error.
///
/// Only transient errors trigger the fallback — specifically:
/// [`LiterLlmError::RateLimited`], [`LiterLlmError::ServiceUnavailable`],
/// [`LiterLlmError::Timeout`], and [`LiterLlmError::ServerError`].
/// Authentication or bad-request errors are propagated directly without
/// consulting the fallback because retrying on a different service would
/// produce the same result.
#[cfg_attr(alef, alef(skip))]
pub struct FallbackLayer<F> {
    fallback: F,
}

#[cfg_attr(alef, alef(skip))]
impl<F> FallbackLayer<F> {
    /// Create a new fallback layer with the given fallback service.
    #[must_use]
    pub fn new(fallback: F) -> Self {
        Self { fallback }
    }
}

impl<S, F> Layer<S> for FallbackLayer<F>
where
    F: Clone,
{
    type Service = FallbackService<S, F>;

    fn layer(&self, primary: S) -> Self::Service {
        FallbackService {
            primary,
            fallback: self.fallback.clone(),
        }
    }
}

/// Tower service produced by [`FallbackLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct FallbackService<S, F> {
    primary: S,
    fallback: F,
}

impl<S, F> Clone for FallbackService<S, F>
where
    S: Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            primary: self.primary.clone(),
            fallback: self.fallback.clone(),
        }
    }
}

impl<S, F> Service<LlmRequest> for FallbackService<S, F>
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + 'static,
    S::Future: Send + 'static,
    F: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Clone + Send + 'static,
    F::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        // ~keep Tower requires one ready service per call, but `call` returns a 'static future.
        // ~keep Polling both services is valid only for always-ready DefaultClient-style services.
        match self.primary.poll_ready(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) => {}
        }
        self.fallback.poll_ready(cx)
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        let fallback_req = req.clone();
        let primary_fut = self.primary.call(req);

        // ~keep Move the readied fallback into the 'static future and leave a fresh clone for next cycle.
        // ~keep Tower permits at most one call per poll_ready, so the fresh clone is not used early.
        let fresh = self.fallback.clone();
        let mut readied_fallback = std::mem::replace(&mut self.fallback, fresh);

        Box::pin(async move {
            match primary_fut.await {
                Ok(resp) => Ok(resp),
                Err(e) if e.is_transient() => {
                    tracing::warn!(
                        error = %e,
                        "primary service failed with transient error; trying fallback"
                    );
                    readied_fallback.call(fallback_req).await
                }
                Err(e) => Err(e),
            }
        })
    }
}
