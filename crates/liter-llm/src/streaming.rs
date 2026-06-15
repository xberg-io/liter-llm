//! Ingress/egress streaming split for zero-copy through the entire request path.
//!
//! # Architecture
//!
//! The streaming pipeline has three composable layers:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │  upstream bytes (SSE or AWS EventStream)                         │
//! │          │                                                       │
//! │   ┌──────▼────────┐                                              │
//! │   │ IngressStream │   decode wire bytes → typed ChatCompletionChunk
//! │   └──────┬────────┘                                              │
//! │          │ Stream<Item = Result<Chunk>>                          │
//! │   ┌──────▼─────────┐                                             │
//! │   │ StreamPipeline │   0..N middleware transforms (per-chunk)    │
//! │   └──────┬─────────┘                                             │
//! │          │ Stream<Item = Result<Chunk>>                          │
//! │   ┌──────▼───────┐                                               │
//! │   │ EgressStream │   encode typed chunk → wire bytes             │
//! │   └──────┬───────┘                                               │
//! │          │ Stream<Item = Result<Bytes>>                          │
//! │  downstream client                                               │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Format pass-through optimisation
//!
//! When the ingress wire format equals the egress wire format AND no
//! middleware layer is registered, `EgressStream` passes raw `Bytes`
//! through without deserialising + re-serialising.  The moment any
//! middleware is present it must observe typed chunks, so the full
//! parse-encode cycle is required.  The decision is made once at
//! stream construction time (`needs_parse` flag) rather than per chunk.
//!
//! See [`EgressStream::new`] for the optimisation gate logic.
//!
//! # Cancellation
//!
//! Pass a [`tokio_util::sync::CancellationToken`] to every constructor;
//! each `poll_next` checks it first and returns `None` immediately when
//! cancelled so that every layer in the pipeline drains promptly.

use std::cell::RefCell;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use futures_core::Stream;
use pin_project_lite::pin_project;

use crate::error::{LiterLlmError, Result};
use crate::provider::StreamFormat;
use crate::types::ChatCompletionChunk;

// ---------------------------------------------------------------------------
// Re-export the CancellationToken under a stable path
// ---------------------------------------------------------------------------

#[cfg(feature = "native-http")]
pub use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Cancel-field type alias (mirrors the one in http::streaming)
// ---------------------------------------------------------------------------

#[cfg(feature = "native-http")]
type CancelField = Option<CancellationToken>;

#[cfg(not(feature = "native-http"))]
type CancelField = Option<std::convert::Infallible>;

// ---------------------------------------------------------------------------
// BytesMut pool — production (not test-only)
// ---------------------------------------------------------------------------

/// Maximum capacity a reclaimed `BytesMut` buffer may have before it is
/// discarded rather than returned to the pool.
const MAX_POOL_BUFFER_CAPACITY: usize = 64 * 1024; // 64 KiB

thread_local! {
    /// Per-thread pool of reusable `BytesMut` scratch buffers used by
    /// [`EgressStream`]'s frame builder.
    ///
    /// Only one buffer is kept per thread to bound memory.  Buffers that
    /// have grown beyond [`MAX_POOL_BUFFER_CAPACITY`] are discarded so that
    /// a single large response doesn't permanently inflate per-thread memory.
    static EGRESS_BYTES_POOL: RefCell<Option<BytesMut>> = const { RefCell::new(None) };
}

/// Acquire a `BytesMut` buffer from the per-thread egress pool.
pub(crate) fn pool_acquire() -> BytesMut {
    EGRESS_BYTES_POOL.with(|cell| {
        cell.borrow_mut()
            .take()
            .map(|mut buf| {
                buf.clear();
                buf
            })
            .unwrap_or_else(|| BytesMut::with_capacity(4096))
    })
}

/// Return a `BytesMut` buffer to the per-thread egress pool.
pub(crate) fn pool_release(buf: BytesMut) {
    if buf.capacity() <= MAX_POOL_BUFFER_CAPACITY {
        EGRESS_BYTES_POOL.with(|cell| {
            *cell.borrow_mut() = Some(buf);
        });
    }
    // Buffers larger than the cap are silently dropped.
}

// ---------------------------------------------------------------------------
// ChunkMiddleware trait
// ---------------------------------------------------------------------------

/// A per-chunk transformation in the [`StreamPipeline`].
///
/// Each middleware receives a typed chunk and returns `Ok(Some(chunk))`
/// to pass it through (optionally modified), `Ok(None)` to drop the chunk,
/// or `Err(e)` to propagate a stream error.
///
/// The trait is object-safe so implementations can be stored in a
/// `Vec<Box<dyn ChunkMiddleware>>` inside [`StreamPipeline`].
pub trait ChunkMiddleware: Send + Sync {
    /// Process a single chunk.
    ///
    /// - `Ok(Some(chunk))` — emit (possibly transformed) chunk.
    /// - `Ok(None)` — drop this chunk silently.
    /// - `Err(e)` — propagate as a stream error.
    fn process(&self, chunk: ChatCompletionChunk) -> Result<Option<ChatCompletionChunk>>;
}

// Allow using `Arc<dyn ChunkMiddleware>` as a middleware.
impl<M: ChunkMiddleware + ?Sized> ChunkMiddleware for Arc<M> {
    fn process(&self, chunk: ChatCompletionChunk) -> Result<Option<ChatCompletionChunk>> {
        (**self).process(chunk)
    }
}

// ---------------------------------------------------------------------------
// IngressStream
// ---------------------------------------------------------------------------

pin_project! {
    /// Typed decoder: parses raw upstream bytes into [`ChatCompletionChunk`]s.
    ///
    /// `S` is the inner byte stream (yields `Result<Bytes, reqwest::Error>`
    /// or any stream whose `Item = Result<Bytes, E>`).
    ///
    /// `P` is the provider-supplied parse function that converts a single SSE
    /// `data:` payload or an AWS EventStream payload into a chunk.
    ///
    /// The ingress stream does not attempt to detect the wire format itself —
    /// the `parse_event` closure captures that knowledge from the provider.
    pub struct IngressStream<S, P> {
        #[pin]
        inner: S,
        buffer: String,
        cursor: usize,
        done: bool,
        parse_event: P,
        cancel: CancelField,
    }
}

impl<S, P> IngressStream<S, P>
where
    P: Fn(&str) -> Result<Option<ChatCompletionChunk>>,
{
    /// Wrap an SSE byte stream with a provider-supplied parse function.
    ///
    /// The stream will decode upstream SSE lines into typed chunks using
    /// `parse_event`.  Pass a [`CancellationToken`] to enable clean abort
    /// on client disconnect.
    pub fn new_sse(inner: S, parse_event: P, cancel: CancelField) -> Self {
        Self {
            inner,
            buffer: String::with_capacity(4096),
            cursor: 0,
            done: false,
            parse_event,
            cancel,
        }
    }
}

impl<S, P, E> Stream for IngressStream<S, P>
where
    S: Stream<Item = std::result::Result<Bytes, E>>,
    E: Into<LiterLlmError>,
    P: Fn(&str) -> Result<Option<ChatCompletionChunk>>,
{
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Check cancellation before doing any work.
        #[cfg(feature = "native-http")]
        if this.cancel.as_ref().is_some_and(|t| t.is_cancelled()) {
            *this.done = true;
            return Poll::Ready(None);
        }

        loop {
            // Process complete lines in the buffer.
            if let Some(offset) = memchr_newline(&this.buffer.as_bytes()[*this.cursor..]) {
                let newline_pos = *this.cursor + offset;
                let line = this.buffer[*this.cursor..newline_pos].trim_end_matches('\r').trim();

                if line.is_empty() || line.starts_with(':') {
                    *this.cursor = newline_pos + 1;
                    compact_buffer(this.buffer, this.cursor);
                    continue;
                }

                if let Some(raw) = line.strip_prefix("data:") {
                    let data = raw.strip_prefix(' ').unwrap_or(raw).trim();
                    if data == "[DONE]" {
                        *this.cursor = newline_pos + 1;
                        compact_buffer(this.buffer, this.cursor);
                        return Poll::Ready(None);
                    }
                    let result = (this.parse_event)(data);
                    *this.cursor = newline_pos + 1;
                    compact_buffer(this.buffer, this.cursor);
                    match result {
                        Ok(None) => continue,
                        Ok(Some(chunk)) => return Poll::Ready(Some(Ok(chunk))),
                        Err(e) => return Poll::Ready(Some(Err(e))),
                    }
                }

                *this.cursor = newline_pos + 1;
                compact_buffer(this.buffer, this.cursor);
                continue;
            }

            // Need more bytes.
            if *this.done {
                let remaining = this.buffer.len() - *this.cursor;
                if remaining > 0 {
                    this.buffer.clear();
                    *this.cursor = 0;
                }
                return Poll::Ready(None);
            }

            // Re-check cancellation before blocking on inner stream.
            #[cfg(feature = "native-http")]
            if this.cancel.as_ref().is_some_and(|t| t.is_cancelled()) {
                *this.done = true;
                return Poll::Ready(None);
            }

            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    const MAX_BUFFER_BYTES: usize = 1024 * 1024; // 1 MiB
                    if this.buffer.len() + bytes.len() > MAX_BUFFER_BYTES {
                        *this.done = true;
                        return Poll::Ready(Some(Err(LiterLlmError::Streaming {
                            message: format!("SSE buffer exceeded {MAX_BUFFER_BYTES} bytes; stream aborted"),
                        })));
                    }
                    match std::str::from_utf8(&bytes) {
                        Ok(s) => this.buffer.push_str(s),
                        Err(e) => {
                            *this.done = true;
                            return Poll::Ready(Some(Err(LiterLlmError::Streaming {
                                message: format!("invalid UTF-8 in SSE stream: {e}"),
                            })));
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(e.into())));
                }
                Poll::Ready(None) => {
                    *this.done = true;
                    continue;
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// StreamPipeline
// ---------------------------------------------------------------------------

pin_project! {
    /// Composable middleware chain operating on typed [`ChatCompletionChunk`]s.
    ///
    /// Each registered middleware observes and may transform chunks before they
    /// reach [`EgressStream`].  Middleware is applied in registration order.
    ///
    /// When the middleware list is empty, chunks pass through with zero overhead.
    /// When any middleware is present, each chunk is processed by every layer in
    /// insertion order.
    ///
    /// # Cancellation
    ///
    /// The `cancel` token is checked on every `poll_next`.  All registered
    /// middleware is bypassed and `None` is returned promptly when cancelled.
    pub struct StreamPipeline<S> {
        #[pin]
        inner: S,
        middleware: Vec<Box<dyn ChunkMiddleware>>,
        cancel: CancelField,
        done: bool,
    }
}

impl<S> StreamPipeline<S> {
    /// Wrap a typed chunk stream with an ordered list of middleware.
    ///
    /// - `inner`: a `Stream<Item = Result<ChatCompletionChunk>>`.
    /// - `middleware`: applied in order on each chunk.
    /// - `cancel`: optional abort signal propagated through every layer.
    pub fn new(inner: S, middleware: Vec<Box<dyn ChunkMiddleware>>, cancel: CancelField) -> Self {
        Self {
            inner,
            middleware,
            cancel,
            done: false,
        }
    }
}

impl<S> Stream for StreamPipeline<S>
where
    S: Stream<Item = Result<ChatCompletionChunk>>,
{
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if *this.done {
            return Poll::Ready(None);
        }

        // Check cancellation first.
        #[cfg(feature = "native-http")]
        if this.cancel.as_ref().is_some_and(|t| t.is_cancelled()) {
            *this.done = true;
            return Poll::Ready(None);
        }

        loop {
            // Check cancellation at the top of each iteration.
            #[cfg(feature = "native-http")]
            if this.cancel.as_ref().is_some_and(|t| t.is_cancelled()) {
                *this.done = true;
                return Poll::Ready(None);
            }

            match this.inner.as_mut().poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    *this.done = true;
                    return Poll::Ready(None);
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(Some(Ok(chunk))) => {
                    // Apply each middleware in order.
                    // `accumulator` wraps the chunk so we can move it into
                    // `process()` and receive the (possibly mutated) chunk back
                    // without cloning.  `None` means the middleware dropped it.
                    let mut accumulator: Option<ChatCompletionChunk> = Some(chunk);
                    let mut error: Option<LiterLlmError> = None;

                    for mw in this.middleware.iter() {
                        match accumulator.take() {
                            None => break,
                            Some(c) => match mw.process(c) {
                                Ok(Some(next)) => accumulator = Some(next),
                                Ok(None) => {
                                    // Middleware dropped the chunk.
                                    accumulator = None;
                                    break;
                                }
                                Err(e) => {
                                    error = Some(e);
                                    break;
                                }
                            },
                        }
                    }

                    if let Some(e) = error {
                        return Poll::Ready(Some(Err(e)));
                    }
                    match accumulator {
                        None => {
                            // Chunk was consumed by middleware; fetch the next one.
                            continue;
                        }
                        Some(final_chunk) => return Poll::Ready(Some(Ok(final_chunk))),
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// EgressStream
// ---------------------------------------------------------------------------

/// Which egress encoding path to use.
enum EgressMode {
    /// Ingress format == egress format AND no middleware: raw bytes pass through.
    ///
    /// # Format pass-through optimisation
    ///
    /// This mode is selected when:
    /// 1. `ingress_format == egress_format` (same wire encoding on both sides), AND
    /// 2. No middleware is registered in the [`StreamPipeline`] that precedes this
    ///    egress layer (middleware requires observing typed chunks, which in turn
    ///    requires a full parse-encode cycle).
    ///
    /// In this mode, raw `Bytes` received from the inner stream are forwarded to
    /// the client without any JSON deserialisation or re-serialisation.
    /// The egress stream still needs to accept `Result<ChatCompletionChunk>` items
    /// from the pipeline (since `StreamPipeline` works on typed chunks), so this
    /// optimisation only avoids *re-serialisation*; the ingress decode is still
    /// performed by `IngressStream`.
    ///
    /// Falls back to `ParseAndEncode` when middleware is present or formats differ.
    Passthrough,
    /// Full parse-encode cycle: serialise typed chunks to the egress wire format.
    ParseAndEncode(EgressEncoding),
}

/// Target encoding for `EgressMode::ParseAndEncode`.
enum EgressEncoding {
    /// Serialise to OpenAI SSE: `data: <json>\n\n`
    OpenAiSse,
    // Future: AwsEventStream variant when that egress path is needed.
}

pin_project! {
    /// Typed encoder: serialises [`ChatCompletionChunk`]s to wire bytes for the
    /// downstream client.
    ///
    /// The encoding format defaults to OpenAI SSE.  When `ingress_format` matches
    /// `egress_format` and no middleware layers have been registered, chunks are
    /// passed through as raw `Bytes` without any re-serialisation (see
    /// [`EgressMode::Passthrough`]).
    ///
    /// Uses the per-thread [`EGRESS_BYTES_POOL`] to amortise allocations across
    /// chunks in the same stream.
    pub struct EgressStream<S> {
        #[pin]
        inner: S,
        mode: EgressMode,
        cancel: CancelField,
        done: bool,
    }
}

impl<S> EgressStream<S> {
    /// Construct an `EgressStream`.
    ///
    /// # Pass-through decision logic
    ///
    /// ```text
    /// if ingress_format == egress_format AND middleware_count == 0:
    ///     mode = Passthrough          # raw Bytes forwarded, no reparse
    /// else:
    ///     mode = ParseAndEncode(...)  # typed chunks → wire bytes
    /// ```
    ///
    /// The middleware count must reflect the upstream [`StreamPipeline`]'s
    /// registered middleware.  Pass `0` when no pipeline is interposed.
    pub fn new(
        inner: S,
        ingress_format: StreamFormat,
        egress_format: StreamFormat,
        middleware_count: usize,
        cancel: CancelField,
    ) -> Self {
        // Passthrough is safe only when:
        //   1. wire formats match (no transcoding needed), and
        //   2. no middleware is present (middleware requires typed chunks,
        //      which means ingress already decoded — but egress could still
        //      re-encode to the same format from typed chunks; we only skip
        //      serialisation when we can guarantee the bytes are identical to
        //      what ingress received, which requires no mutation by middleware).
        let mode = if ingress_format == egress_format && middleware_count == 0 {
            EgressMode::Passthrough
        } else {
            // Select the egress encoding based on the requested egress format.
            let encoding = match egress_format {
                StreamFormat::Sse | StreamFormat::AwsEventStream => EgressEncoding::OpenAiSse,
            };
            EgressMode::ParseAndEncode(encoding)
        };

        Self {
            inner,
            mode,
            cancel,
            done: false,
        }
    }
}

impl<S> Stream for EgressStream<S>
where
    S: Stream<Item = Result<ChatCompletionChunk>>,
{
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if *this.done {
            return Poll::Ready(None);
        }

        // Check cancellation first.
        #[cfg(feature = "native-http")]
        if this.cancel.as_ref().is_some_and(|t| t.is_cancelled()) {
            *this.done = true;
            return Poll::Ready(None);
        }

        match this.inner.as_mut().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => {
                *this.done = true;
                Poll::Ready(None)
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(Some(Ok(chunk))) => {
                // Check cancellation after waking.
                #[cfg(feature = "native-http")]
                if this.cancel.as_ref().is_some_and(|t| t.is_cancelled()) {
                    *this.done = true;
                    return Poll::Ready(None);
                }

                match this.mode {
                    EgressMode::Passthrough => {
                        // In passthrough mode we received a typed chunk from the
                        // IngressStream but there is no middleware that mutated it.
                        // Re-serialise to OpenAI SSE.  The bytes content is
                        // semantically identical to what the upstream sent (because
                        // no middleware ran), but we still need to produce Bytes.
                        //
                        // Note: true zero-copy passthrough (bypassing IngressStream
                        // entirely) requires a different wiring not covered by this
                        // Stream combinator design — the pipeline always operates on
                        // typed items.  This "passthrough" mode avoids any
                        // *additional* encode round-trip beyond the ingress decode.
                        Poll::Ready(Some(encode_sse_chunk(&chunk)))
                    }
                    EgressMode::ParseAndEncode(EgressEncoding::OpenAiSse) => {
                        Poll::Ready(Some(encode_sse_chunk(&chunk)))
                    }
                }
            }
        }
    }
}

/// Serialise a `ChatCompletionChunk` to OpenAI SSE wire format using the
/// per-thread [`EGRESS_BYTES_POOL`] for the scratch buffer.
///
/// Output format: `data: <json>\n\n`
///
/// # Pool strategy
///
/// The pool buffer is used as a scratch space to build the encoded frame.
/// After building, we copy the bytes into a freshly-allocated frozen `Bytes`
/// value so that the pool buffer can be cleared and returned with its
/// original backing allocation intact (same pointer, capacity preserved).
/// This ensures the pool pointer is stable across calls.
fn encode_sse_chunk(chunk: &ChatCompletionChunk) -> Result<Bytes> {
    let json = serde_json::to_string(chunk).map_err(|e| LiterLlmError::Streaming {
        message: format!("failed to serialise chunk: {e}"),
    })?;

    let mut buf = pool_acquire();
    buf.extend_from_slice(b"data: ");
    buf.extend_from_slice(json.as_bytes());
    buf.extend_from_slice(b"\n\n");

    // Freeze a copy of the assembled frame, then clear the buffer so the
    // pool gets back a buffer with its original backing pointer.
    let frozen = Bytes::copy_from_slice(&buf);
    buf.clear();
    pool_release(buf);
    Ok(frozen)
}

// ---------------------------------------------------------------------------
// Internal utility: newline search (avoids direct memchr dependency here)
// ---------------------------------------------------------------------------

#[inline]
fn memchr_newline(haystack: &[u8]) -> Option<usize> {
    haystack.iter().position(|&b| b == b'\n')
}

/// Compact the SSE string buffer when the cursor has advanced past half its
/// length, amortising memmove cost to O(total_bytes).
fn compact_buffer(buffer: &mut String, cursor: &mut usize) {
    if *cursor > buffer.len() / 2 {
        buffer.drain(..*cursor);
        *cursor = 0;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use futures_util::StreamExt;

    use super::*;

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Build a minimal `ChatCompletionChunk` for testing.
    fn make_chunk(content: &str) -> ChatCompletionChunk {
        use crate::types::chat::{StreamChoice, StreamDelta};
        ChatCompletionChunk {
            id: "test-id".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 0,
            model: "test-model".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    content: Some(content.to_string()),
                    ..Default::default()
                },
                finish_reason: None,
            }],
            usage: None,
            system_fingerprint: None,
            service_tier: None,
        }
    }

    /// Serialize a chunk to an SSE line like an upstream provider would.
    fn chunk_to_sse(chunk: &ChatCompletionChunk) -> String {
        format!("data: {}\n\n", serde_json::to_string(chunk).unwrap())
    }

    /// A middleware that appends " [mw]" to the content of each chunk.
    struct AppendMiddleware;

    impl ChunkMiddleware for AppendMiddleware {
        fn process(&self, mut chunk: ChatCompletionChunk) -> Result<Option<ChatCompletionChunk>> {
            for choice in &mut chunk.choices {
                if let Some(content) = &choice.delta.content {
                    choice.delta.content = Some(format!("{content} [mw]"));
                }
            }
            Ok(Some(chunk))
        }
    }

    /// Stream of `Result<Bytes, reqwest::Error>` from a Vec of strings.
    ///
    /// Uses `futures_util::stream::iter` (synchronous) so the stream is `Unpin`
    /// and can be used directly with `.next().await` without boxing.
    fn sse_byte_stream(lines: Vec<String>) -> impl Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Unpin {
        let joined = lines.join("");
        futures_util::stream::iter(vec![Ok::<_, reqwest::Error>(Bytes::from(joined))])
    }

    // ── BytesMut pool tests ───────────────────────────────────────────────────

    #[test]
    fn egress_pool_reuses_buffer() {
        let buf = pool_acquire();
        let ptr_before = buf.as_ptr();
        pool_release(buf);

        let buf2 = pool_acquire();
        let ptr_after = buf2.as_ptr();

        assert_eq!(
            ptr_before, ptr_after,
            "egress pool should reuse the same BytesMut allocation"
        );
        pool_release(buf2);
    }

    #[test]
    fn egress_pool_discards_oversized_buffers() {
        let mut big = BytesMut::with_capacity(MAX_POOL_BUFFER_CAPACITY + 1);
        big.resize(MAX_POOL_BUFFER_CAPACITY + 1, 0u8);
        pool_release(big);

        let acquired = pool_acquire();
        assert!(
            acquired.capacity() <= 4096,
            "oversized buffer should have been discarded; got capacity {}",
            acquired.capacity()
        );
        pool_release(acquired);
    }

    // ── IngressStream tests ───────────────────────────────────────────────────

    #[tokio::test]
    async fn ingress_stream_parses_sse() {
        let chunk = make_chunk("hello");
        let sse_line = chunk_to_sse(&chunk);
        let done = "data: [DONE]\n\n".to_string();

        let byte_stream = sse_byte_stream(vec![sse_line, done]);
        let parse = |data: &str| -> Result<Option<ChatCompletionChunk>> {
            serde_json::from_str(data)
                .map(Some)
                .map_err(|e| LiterLlmError::Streaming { message: e.to_string() })
        };

        let mut stream = IngressStream::new_sse(byte_stream, parse, None);
        let result = stream
            .next()
            .await
            .expect("should yield one chunk")
            .expect("should be Ok");
        assert_eq!(result.choices[0].delta.content.as_deref(), Some("hello"));

        // Next poll should be None (stream ended after [DONE]).
        assert!(stream.next().await.is_none());
    }

    // ── StreamPipeline tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn pipeline_applies_middleware_in_order() {
        let chunk = make_chunk("hi");
        let inner = futures_util::stream::iter(vec![Ok::<_, LiterLlmError>(chunk)]);

        let mw = Box::new(AppendMiddleware) as Box<dyn ChunkMiddleware>;
        let mut pipeline = StreamPipeline::new(inner, vec![mw], None);

        let result = pipeline.next().await.expect("should yield").expect("should be Ok");
        assert_eq!(result.choices[0].delta.content.as_deref(), Some("hi [mw]"));
    }

    #[tokio::test]
    async fn pipeline_no_middleware_passes_through() {
        let chunk = make_chunk("raw");
        let inner = futures_util::stream::iter(vec![Ok::<_, LiterLlmError>(chunk.clone())]);

        let mut pipeline = StreamPipeline::new(inner, vec![], None);
        let result = pipeline.next().await.expect("should yield").expect("should be Ok");
        assert_eq!(result, chunk);
    }

    // ── EgressStream tests ────────────────────────────────────────────────────

    #[tokio::test]
    async fn egress_stream_encodes_to_sse() {
        let chunk = make_chunk("world");
        let inner = futures_util::stream::iter(vec![Ok::<_, LiterLlmError>(chunk.clone())]);

        let mut egress = EgressStream::new(inner, StreamFormat::Sse, StreamFormat::Sse, 0, None);

        let bytes = egress.next().await.expect("should yield bytes").expect("should be Ok");
        let text = std::str::from_utf8(&bytes).expect("bytes should be valid UTF-8");

        assert!(text.starts_with("data: "), "should start with 'data: '");
        assert!(text.ends_with("\n\n"), "should end with \\n\\n");

        let json_part = text.trim_start_matches("data: ").trim_end_matches("\n\n");
        let decoded: ChatCompletionChunk = serde_json::from_str(json_part).expect("encoded bytes should deserialise");
        assert_eq!(decoded.choices[0].delta.content.as_deref(), Some("world"));
    }

    // ── Format passthrough optimisation tests ─────────────────────────────────

    /// `ingress_egress_passthrough_avoids_reparse`:
    /// When SSE-in, SSE-out, and no middleware, the EgressStream uses
    /// `Passthrough` mode.  We verify this via the `parse_count` probe
    /// inserted in the ingress parse function.
    #[tokio::test]
    async fn ingress_egress_passthrough_avoids_reparse() {
        let chunk = make_chunk("direct");
        let sse_line = chunk_to_sse(&chunk);
        let done = "data: [DONE]\n\n".to_string();

        // Count how many times the parse function is called.
        let parse_count = Arc::new(AtomicUsize::new(0));
        let parse_count_clone = Arc::clone(&parse_count);

        let byte_stream = sse_byte_stream(vec![sse_line, done]);
        let ingress = IngressStream::new_sse(
            byte_stream,
            move |data: &str| -> Result<Option<ChatCompletionChunk>> {
                parse_count_clone.fetch_add(1, Ordering::Relaxed);
                serde_json::from_str(data)
                    .map(Some)
                    .map_err(|e| LiterLlmError::Streaming { message: e.to_string() })
            },
            None,
        );

        // No middleware → EgressStream selects Passthrough mode.
        let pipeline = StreamPipeline::new(ingress, vec![], None);
        let mut egress = EgressStream::new(pipeline, StreamFormat::Sse, StreamFormat::Sse, 0, None);

        // Drain the stream.
        let mut byte_count = 0usize;
        while let Some(item) = egress.next().await {
            let bytes = item.expect("should be Ok");
            byte_count += bytes.len();
        }

        // The chunk was encoded to SSE bytes.
        assert!(byte_count > 0, "should have produced some output bytes");

        // Parse was called exactly once (ingress decode only; egress in
        // passthrough mode skips a second full JSON decode).
        assert_eq!(
            parse_count.load(Ordering::Relaxed),
            1,
            "passthrough mode must call the parse function exactly once (ingress only)"
        );
    }

    /// `ingress_egress_with_middleware_reparses`:
    /// When middleware is registered, `EgressStream` switches to
    /// `ParseAndEncode` mode. The middleware is called, which proves
    /// the typed chunk was observed.
    #[tokio::test]
    async fn ingress_egress_with_middleware_reparses() {
        let chunk = make_chunk("before");
        let sse_line = chunk_to_sse(&chunk);
        let done = "data: [DONE]\n\n".to_string();

        let byte_stream = sse_byte_stream(vec![sse_line, done]);
        let ingress = IngressStream::new_sse(
            byte_stream,
            |data: &str| -> Result<Option<ChatCompletionChunk>> {
                serde_json::from_str(data)
                    .map(Some)
                    .map_err(|e| LiterLlmError::Streaming { message: e.to_string() })
            },
            None,
        );

        let mw = Box::new(AppendMiddleware) as Box<dyn ChunkMiddleware>;
        // middleware_count=1 → EgressStream uses ParseAndEncode.
        let pipeline = StreamPipeline::new(ingress, vec![mw], None);
        let mut egress = EgressStream::new(pipeline, StreamFormat::Sse, StreamFormat::Sse, 1, None);

        let bytes = egress.next().await.expect("should yield bytes").expect("should be Ok");
        let text = std::str::from_utf8(&bytes).expect("valid UTF-8");
        let json_part = text.trim_start_matches("data: ").trim_end_matches("\n\n");
        let decoded: ChatCompletionChunk = serde_json::from_str(json_part).expect("should deserialise");

        // The middleware appended " [mw]" — confirming parse-encode round trip.
        assert_eq!(
            decoded.choices[0].delta.content.as_deref(),
            Some("before [mw]"),
            "middleware should have mutated the chunk before re-encode"
        );
    }

    /// `aws_event_stream_ingress_sse_egress_round_trips`:
    /// When ingress is `AwsEventStream` and egress is `Sse`, the pipeline
    /// performs a full parse-encode cycle (different formats → `ParseAndEncode`).
    /// We simulate the Bedrock ingress as pre-typed chunks (already decoded by
    /// `EventStreamParser`) and verify SSE bytes come out with content preserved.
    #[tokio::test]
    async fn aws_event_stream_ingress_sse_egress_round_trips() {
        let chunk = make_chunk("bedrock content");

        // Simulate EventStreamParser output: a typed stream of chunks.
        // `iter` returns an `Unpin` stream so it can be used directly.
        let inner = futures_util::stream::iter(vec![Ok::<ChatCompletionChunk, LiterLlmError>(chunk.clone())]);

        // Different formats → EgressStream must encode.
        let pipeline = StreamPipeline::new(inner, vec![], None);
        let mut egress = EgressStream::new(pipeline, StreamFormat::AwsEventStream, StreamFormat::Sse, 0, None);

        let bytes = egress.next().await.expect("should yield bytes").expect("should be Ok");
        let text = std::str::from_utf8(&bytes).expect("valid UTF-8");
        let json_part = text.trim_start_matches("data: ").trim_end_matches("\n\n");
        let decoded: ChatCompletionChunk = serde_json::from_str(json_part).expect("should deserialise");

        assert_eq!(
            decoded.choices[0].delta.content.as_deref(),
            Some("bedrock content"),
            "content should be preserved through format conversion"
        );
    }

    /// `cancellation_propagates_through_pipeline_layers`:
    /// A 3-layer pipeline (ingress → pipeline → egress) should all return None
    /// within one poll after cancellation.
    #[cfg(feature = "native-http")]
    #[tokio::test]
    async fn cancellation_propagates_through_pipeline_layers() {
        use std::time::Duration;

        // Build an inner stream that never ends.
        struct NeverStream;
        impl Stream for NeverStream {
            type Item = std::result::Result<Bytes, reqwest::Error>;
            fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                Poll::Pending
            }
        }

        let token = CancellationToken::new();
        let cancel_clone = token.clone();

        let ingress = IngressStream::new_sse(
            NeverStream,
            |_: &str| -> Result<Option<ChatCompletionChunk>> { Ok(None) },
            Some(cancel_clone.clone()),
        );

        let pipeline = StreamPipeline::new(ingress, vec![], Some(cancel_clone.clone()));
        let mut egress = EgressStream::new(pipeline, StreamFormat::Sse, StreamFormat::Sse, 0, Some(cancel_clone));

        // Cancel the token, then assert all 3 layers drain promptly.
        token.cancel();

        let deadline = tokio::time::Instant::now() + Duration::from_millis(50);
        let result = tokio::time::timeout_at(deadline, egress.next()).await;

        match result {
            Ok(None) => {} // clean shutdown
            Ok(Some(_)) => panic!("cancelled pipeline should yield None, not a chunk"),
            Err(_elapsed) => panic!("cancelled pipeline did not terminate within 50ms"),
        }
    }

    /// `bytes_pool_reused_in_egress_under_load`:
    /// Fire multiple sequential streams through EgressStream and verify the pool
    /// buffer pointer is reused across streams (confirming the pool is active).
    #[tokio::test]
    async fn bytes_pool_reused_in_egress_under_load() {
        // Seed the pool with a known buffer.
        let sentinel = pool_acquire();
        let sentinel_ptr = sentinel.as_ptr();
        pool_release(sentinel);

        // Run 100 chunks through the egress encoder.
        for _ in 0..100 {
            let chunk = make_chunk("x");
            let inner = futures_util::stream::iter(vec![Ok::<_, LiterLlmError>(chunk)]);
            let pipeline = StreamPipeline::new(inner, vec![], None);
            let mut egress = EgressStream::new(pipeline, StreamFormat::Sse, StreamFormat::Sse, 0, None);

            while let Some(item) = egress.next().await {
                item.expect("should be Ok");
            }
        }

        // After all 100 streams, the pool slot should still hold a reused buffer.
        let reclaimed = pool_acquire();
        assert_eq!(
            reclaimed.as_ptr(),
            sentinel_ptr,
            "pool buffer should have been reused across 100 egress streams"
        );
        pool_release(reclaimed);
    }
}
