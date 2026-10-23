//! Time limits for HTTP calls on wasm32.
//!
//! ~keep reqwest applies `ClientConfig::timeout` through its client builder on native targets only.
//! ~keep The wasm32 client has no builder timeout, so this module supplies the same two limits there:
//! ~keep a deadline for one attempt of a call that returns one reply, and an idle limit for the gap
//! ~keep between the chunks of a stream. Both resolve to `LiterLlmError::Timeout`, the error kind the
//! ~keep native client reports. On native targets every item here passes its input through.

#[cfg(target_arch = "wasm32")]
use std::future::Future;
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
use bytes::Bytes;
#[cfg(target_arch = "wasm32")]
use futures_core::Stream;

#[cfg(target_arch = "wasm32")]
use crate::error::LiterLlmError;

/// The longest delay `setTimeout` honours: a larger value fires at once. A limit above this is no limit.
#[cfg(target_arch = "wasm32")]
const MAX_TIMER: Duration = Duration::from_millis(i32::MAX as u64);

/// The time limit of one attempt, from the moment the attempt starts.
///
/// One value covers the whole attempt: sending the request and reading the reply body. On native
/// targets the client builder enforces the limit, so the value is empty and `race` only awaits.
#[derive(Clone, Copy)]
pub(crate) struct Deadline {
    #[cfg(target_arch = "wasm32")]
    end: Option<web_time::Instant>,
}

/// The attempt outlived its `Deadline`.
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(
    not(target_arch = "wasm32"),
    allow(dead_code, reason = "only the wasm32 build raises it")
)]
pub(crate) struct Elapsed;

impl Deadline {
    /// Start the clock for one attempt. `None` means the attempt has no limit.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        allow(unused_variables, reason = "native clients enforce the limit")
    )]
    pub(crate) fn after(limit: Option<Duration>) -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            end: limit
                .filter(|limit| *limit <= MAX_TIMER)
                .map(|limit| web_time::Instant::now() + limit),
        }
    }

    /// Await `fut` unless the deadline passes first. A reply that is ready wins over the timer.
    ///
    /// Dropping `fut` aborts the request that it holds: reqwest's wasm client aborts its `fetch`
    /// when the send future or the response is dropped.
    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn race<F: Future>(self, fut: F) -> Result<F::Output, Elapsed> {
        use std::pin::pin;
        use std::task::Poll;

        let Some(end) = self.end else {
            return Ok(fut.await);
        };
        let mut fut = pin!(fut);
        let mut timer = pin!(gloo_timers::future::sleep(
            end.saturating_duration_since(web_time::Instant::now())
        ));
        std::future::poll_fn(|cx| {
            if let Poll::Ready(output) = fut.as_mut().poll(cx) {
                return Poll::Ready(Ok(output));
            }
            if timer.as_mut().poll(cx).is_ready() {
                return Poll::Ready(Err(Elapsed));
            }
            Poll::Pending
        })
        .await
    }

    /// Await `fut` unless the deadline passes first.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::unused_async, reason = "same signature as the wasm32 build")]
    pub(crate) async fn race<F: std::future::Future>(self, fut: F) -> Result<F::Output, Elapsed> {
        Ok(fut.await)
    }

    /// Await a fallible `fut`, mapping a passed deadline to `LiterLlmError::Timeout`.
    pub(crate) async fn run<T, F>(self, fut: F) -> crate::error::Result<T>
    where
        F: std::future::Future<Output = crate::error::Result<T>>,
    {
        match self.race(fut).await {
            Ok(result) => result,
            Err(Elapsed) => Err(crate::error::LiterLlmError::Timeout),
        }
    }
}

#[cfg(target_arch = "wasm32")]
pin_project_lite::pin_project! {
    /// A byte stream that ends with `LiterLlmError::Timeout` when no chunk arrives within `limit`.
    ///
    /// The timer restarts after every chunk, as the native `read_timeout` does. After the timeout
    /// the stream is finished, and dropping it aborts the request.
    pub(crate) struct IdleTimeout<S> {
        #[pin]
        inner: S,
        limit: Option<Duration>,
        timer: Option<std::pin::Pin<Box<gloo_timers::future::TimeoutFuture>>>,
        finished: bool,
    }
}

/// Limit the gap between the chunks of `stream` to `limit`. `None` means no limit.
#[cfg(target_arch = "wasm32")]
pub(crate) fn idle_timeout<S>(stream: S, limit: Option<Duration>) -> IdleTimeout<S> {
    IdleTimeout {
        inner: stream,
        limit: limit.filter(|limit| *limit <= MAX_TIMER),
        timer: None,
        finished: false,
    }
}

/// On native targets the client builder enforces the idle limit, so the stream passes through.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn idle_timeout<S>(stream: S, _limit: Option<Duration>) -> S {
    stream
}

#[cfg(target_arch = "wasm32")]
impl<S, E> Stream for IdleTimeout<S>
where
    S: Stream<Item = Result<Bytes, E>>,
    LiterLlmError: From<E>,
{
    type Item = Result<Bytes, LiterLlmError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::future::Future;
        use std::task::Poll;

        let mut this = self.project();
        if *this.finished {
            return Poll::Ready(None);
        }
        match this.inner.as_mut().poll_next(cx) {
            Poll::Ready(item) => {
                *this.timer = None;
                Poll::Ready(item.map(|chunk| chunk.map_err(LiterLlmError::from)))
            }
            Poll::Pending => {
                let Some(limit) = *this.limit else {
                    return Poll::Pending;
                };
                let timer = this
                    .timer
                    .get_or_insert_with(|| Box::pin(gloo_timers::future::sleep(limit)));
                if timer.as_mut().poll(cx).is_ready() {
                    *this.timer = None;
                    *this.finished = true;
                    return Poll::Ready(Some(Err(LiterLlmError::Timeout)));
                }
                Poll::Pending
            }
        }
    }
}
