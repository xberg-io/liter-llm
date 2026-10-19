use std::sync::Arc;
use std::task::{Context, Poll};

use tower::Service;

use super::types::{LlmRequest, LlmRequestKind, LlmResponse};
use crate::client::{BoxFuture, LlmClient};
use crate::error::{LiterLlmError, Result};

/// A thin tower [`Service`] wrapper around any [`LlmClient`] implementation.
///
/// Because [`LlmClient`] methods take `&self`, the inner client is stored
/// behind an [`Arc`] so the service can be cloned without owning a unique
/// reference.  `tower::Service::call` takes `&mut self`, but the actual
/// async work is dispatched through the shared reference inside the arc.
///
/// # Streaming behaviour
///
/// The client already returns an owned `'static` stream. Forward it directly:
/// downstream polling supplies backpressure, and dropping the response closes
/// the upstream stream. Cancelling `call` before headers drops initialization.
/// No background task or whole-response buffer is needed.
#[cfg_attr(alef, alef(skip))]
pub struct LlmService<C> {
    inner: Arc<C>,
}

impl<C> LlmService<C> {
    /// Wrap `client` in a tower-compatible service.
    #[must_use]
    pub fn new(client: C) -> Self {
        Self {
            inner: Arc::new(client),
        }
    }

    /// Wrap a client that is already behind an `Arc`.
    ///
    /// This avoids a redundant `Arc` layer when the caller (e.g.
    /// [`ManagedClient`](crate::client::managed::ManagedClient)) already
    /// owns an `Arc<C>`.
    #[must_use]
    pub fn new_from_arc(client: Arc<C>) -> Self {
        Self { inner: client }
    }

    /// Return a reference to the inner client.
    pub fn inner(&self) -> &C {
        &self.inner
    }
}

impl<C> Clone for LlmService<C> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<C> Service<LlmRequest> for LlmService<C>
where
    C: LlmClient + Send + Sync + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        let client = Arc::clone(&self.inner);
        Box::pin(async move {
            match req.kind {
                LlmRequestKind::Chat(r) => {
                    let resp = client.chat(r).await?;
                    Ok(LlmResponse::Chat(resp))
                }
                LlmRequestKind::ChatStream(r) => {
                    let stream = client.chat_stream(r).await?;
                    Ok(LlmResponse::ChatStream(stream))
                }
                LlmRequestKind::Embed(r) => {
                    let resp = client.embed(r).await?;
                    Ok(LlmResponse::Embed(resp))
                }
                LlmRequestKind::ListModels => {
                    let resp = client.list_models().await?;
                    Ok(LlmResponse::ListModels(resp))
                }
                LlmRequestKind::ImageGenerate(r) => {
                    let resp = client.image_generate(r).await?;
                    Ok(LlmResponse::ImageGenerate(resp))
                }
                LlmRequestKind::Speech(r) => {
                    let resp = client.speech(r).await?;
                    Ok(LlmResponse::Speech(resp))
                }
                LlmRequestKind::Transcribe(r) => {
                    let resp = client.transcribe(r).await?;
                    Ok(LlmResponse::Transcribe(resp))
                }
                LlmRequestKind::Moderate(r) => {
                    let resp = client.moderate(r).await?;
                    Ok(LlmResponse::Moderate(resp))
                }
                LlmRequestKind::Rerank(r) => {
                    let resp = client.rerank(r).await?;
                    Ok(LlmResponse::Rerank(resp))
                }
                LlmRequestKind::Search(r) => {
                    let resp = client.search(r).await?;
                    Ok(LlmResponse::Search(resp))
                }
                LlmRequestKind::Ocr(r) => {
                    let resp = client.ocr(r).await?;
                    Ok(LlmResponse::Ocr(resp))
                }
            }
        })
    }
}

#[cfg(all(test, feature = "native-http"))]
mod live_stream_tests {
    use std::time::Duration;

    use futures_util::StreamExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use tokio::time::timeout;

    use super::*;
    use crate::client::{ClientConfigBuilder, DefaultClient};

    // A real HTTP response deliberately remains unfinished. Whole-response
    // buffering cannot pass these checks, even if finite mock streams pass.
    async fn upstream(send_first_chunk: bool) -> (String, oneshot::Receiver<()>, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (seen, request_seen) = oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 4096];
            assert!(socket.read(&mut request).await.unwrap() > 0);
            if send_first_chunk {
                socket.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 99999\r\n\r\ndata: {\"id\":\"first\",\"choices\":[]}\n\n",
                ).await.unwrap();
            }
            let _ = seen.send(());
            // Cancellation must close the socket even though the provider has
            // not finished the response (or has not sent headers at all).
            let _ = socket.read_to_end(&mut Vec::new()).await;
        });
        (format!("http://{address}/v1"), request_seen, server)
    }

    fn service(url: String) -> LlmService<DefaultClient> {
        let config = ClientConfigBuilder::new("synthetic-key")
            .base_url(url)
            .max_retries(0)
            .build();
        LlmService::new(DefaultClient::new(config, Some("openai/synthetic-model")).unwrap())
    }

    fn request() -> LlmRequest {
        LlmRequest::ChatStream(
            serde_json::from_value(serde_json::json!({
                "model": "synthetic-model", "messages": [{"role": "user", "content": "Synthetic fixture"}]
            }))
            .unwrap(),
        )
    }

    #[tokio::test]
    async fn returns_first_chunk_before_upstream_finishes_and_drop_closes_socket() {
        let (url, _seen, server) = upstream(true).await;
        let response = timeout(Duration::from_secs(2), service(url).call(request()))
            .await
            .expect("response must not wait for the whole stream")
            .unwrap();
        let LlmResponse::ChatStream(mut stream) = response else {
            panic!("expected stream")
        };
        let first = timeout(Duration::from_secs(2), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(first.id, "first");
        drop(stream);
        timeout(Duration::from_secs(2), server)
            .await
            .expect("drop must close upstream")
            .unwrap();
    }

    #[tokio::test]
    async fn cancellation_before_headers_closes_upstream() {
        let (url, seen, server) = upstream(false).await;
        let caller = tokio::spawn(async move { service(url).call(request()).await });
        timeout(Duration::from_secs(2), seen).await.unwrap().unwrap();
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        timeout(Duration::from_secs(2), server)
            .await
            .expect("cancel must close upstream")
            .unwrap();
    }

    #[tokio::test]
    async fn upstream_failure_after_first_chunk_reaches_consumer() {
        let (url, _seen, server) = upstream(true).await;
        let response = timeout(Duration::from_secs(2), service(url).call(request()))
            .await
            .unwrap()
            .unwrap();
        let LlmResponse::ChatStream(mut stream) = response else {
            panic!("expected stream")
        };
        assert_eq!(stream.next().await.unwrap().unwrap().id, "first");
        server.abort();
        let _ = server.await;
        let next = timeout(Duration::from_secs(2), stream.next()).await.unwrap();
        assert!(next.expect("truncated response must report failure").is_err());
    }
}
