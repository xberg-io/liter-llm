use std::cell::RefCell;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use futures_core::Stream;
use memchr::memchr;
use pin_project_lite::pin_project;

use crate::error::{LiterLlmError, Result};
use crate::http::request::with_retry;
use crate::types::ChatCompletionChunk;

/// Maximum number of bytes buffered before declaring a streaming error.
const MAX_BUFFER_BYTES: usize = 1024 * 1024; // 1 MiB

/// Maximum capacity a reclaimed `BytesMut` buffer may have before it is
/// discarded rather than returned to the pool.  Prevents unbounded memory
/// accumulation on idle clients that previously processed very large chunks.
const MAX_POOL_BUFFER_CAPACITY: usize = 64 * 1024; // 64 KiB

// ---------------------------------------------------------------------------
// Threadlocal BytesMut pool
// ---------------------------------------------------------------------------

thread_local! {
    /// Per-thread pool of reusable `BytesMut` scratch buffers.
    ///
    /// Only one buffer is kept per thread to bound memory.  Buffers that have
    /// grown beyond [`MAX_POOL_BUFFER_CAPACITY`] are discarded so that a single
    /// large response doesn't permanently inflate per-thread memory.
    static BYTES_POOL: RefCell<Option<BytesMut>> = const { RefCell::new(None) };
}

/// Acquire a `BytesMut` scratch buffer from the per-thread pool.
///
/// Returns a recycled buffer when one is available; otherwise allocates a new
/// one pre-sized to 4 KiB (a reasonable first-chunk size for most SSE streams).
fn pool_acquire() -> BytesMut {
    BYTES_POOL.with(|cell| {
        cell.borrow_mut()
            .take()
            .map(|mut buf| {
                buf.clear();
                buf
            })
            .unwrap_or_else(|| BytesMut::with_capacity(4096))
    })
}

/// Return a `BytesMut` buffer to the per-thread pool.
///
/// Buffers exceeding [`MAX_POOL_BUFFER_CAPACITY`] are dropped instead of
/// being pooled to prevent memory growth after large-response processing.
fn pool_release(buf: BytesMut) {
    if buf.capacity() <= MAX_POOL_BUFFER_CAPACITY {
        BYTES_POOL.with(|cell| {
            *cell.borrow_mut() = Some(buf);
        });
    }
    // Buffers larger than the cap are silently dropped here.
}

// ---------------------------------------------------------------------------
// CancellationToken re-export (native-http only)
// ---------------------------------------------------------------------------

/// A token that can be used to cancel an in-progress streaming response.
///
/// Pass to [`post_stream_with_cancel`] to abort the upstream SSE connection
/// when the downstream client disconnects.
///
/// Only available when the `native-http` feature is enabled.
#[cfg(feature = "native-http")]
pub use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Send a streaming POST request and return an SSE stream of
/// `ChatCompletionChunk`s.
///
/// Before opening the stream, retries on 429 / 500 / 502 / 503 / 504 up to
/// `max_retries` times honouring any `Retry-After` header.  Once the stream
/// is open, individual chunk errors are yielded as `Err` items rather than
/// causing a retry.
///
/// `auth_header` is `Some((name, value))` when the provider requires
/// authentication, or `None` when no auth header should be added.
///
/// `extra_headers` carries provider-specific mandatory headers (e.g.
/// `anthropic-version`) beyond the single auth header.
///
/// `parse_event` translates a raw SSE `data:` payload string into a
/// `ChatCompletionChunk`.  Pass the provider's `parse_stream_event` method
/// to support non-OpenAI SSE formats.
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(
        skip_all,
        fields(
            http.method = "POST",
            http.url = %url,
            http.status_code = tracing::field::Empty,
            http.retry_count = tracing::field::Empty,
        )
    )
)]
pub async fn post_stream<P>(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    body: Bytes,
    max_retries: u32,
    parse_event: P,
) -> Result<crate::client::BoxStream<'static, Result<ChatCompletionChunk>>>
where
    P: Fn(&str) -> Result<Option<ChatCompletionChunk>> + Send + 'static,
{
    let mut retry_count = 0u32;

    let resp = with_retry(max_retries, || {
        // Clone is a zero-copy ref-count bump on `Bytes`.
        let mut builder = client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.clone());
        if let Some((name, value)) = auth_header {
            builder = builder.header(name, value);
        }
        for (name, value) in extra_headers {
            builder = builder.header(*name, *value);
        }
        retry_count += 1;
        builder.send()
    })
    .await?;

    #[cfg(feature = "tracing")]
    {
        let span = tracing::Span::current();
        span.record("http.status_code", resp.status().as_u16());
        span.record("http.retry_count", retry_count.saturating_sub(1));
    }

    let byte_stream = resp.bytes_stream();
    let stream = SseParser::new(byte_stream, parse_event, None);
    Ok(Box::pin(stream))
}

/// Send a streaming POST request with end-to-end cancellation support.
///
/// Identical to [`post_stream`] but accepts a [`CancellationToken`].  When the
/// token is cancelled (e.g. because the downstream client disconnected), the
/// SSE stream is aborted cleanly and no further chunks are yielded.
#[cfg(feature = "native-http")]
#[allow(dead_code)] // Public API; not yet wired to provider call sites.
#[allow(clippy::too_many_arguments)] // The cancel token is the necessary 8th arg.
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(
        skip_all,
        fields(
            http.method = "POST",
            http.url = %url,
            http.status_code = tracing::field::Empty,
            http.retry_count = tracing::field::Empty,
        )
    )
)]
pub async fn post_stream_with_cancel<P>(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    body: Bytes,
    max_retries: u32,
    parse_event: P,
    cancel: CancellationToken,
) -> Result<crate::client::BoxStream<'static, Result<ChatCompletionChunk>>>
where
    P: Fn(&str) -> Result<Option<ChatCompletionChunk>> + Send + 'static,
{
    let mut retry_count = 0u32;

    let resp = with_retry(max_retries, || {
        let mut builder = client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.clone());
        if let Some((name, value)) = auth_header {
            builder = builder.header(name, value);
        }
        for (name, value) in extra_headers {
            builder = builder.header(*name, *value);
        }
        retry_count += 1;
        builder.send()
    })
    .await?;

    #[cfg(feature = "tracing")]
    {
        let span = tracing::Span::current();
        span.record("http.status_code", resp.status().as_u16());
        span.record("http.retry_count", retry_count.saturating_sub(1));
    }

    let byte_stream = resp.bytes_stream();
    let stream = SseParser::new(byte_stream, parse_event, Some(cancel));
    Ok(Box::pin(stream))
}

// ---------------------------------------------------------------------------
// SSE parser
// ---------------------------------------------------------------------------
//
// `pin_project_lite` does not support `#[cfg(...)]` attributes on individual
// struct fields.  We work around this by always including the `cancel` field
// but typing it as `Option<CancellationToken>` only on the `native-http` path
// and as `Option<std::convert::Infallible>` on the WASM path (zero size, never
// constructed with `Some`).

#[cfg(feature = "native-http")]
type CancelField = Option<CancellationToken>;

#[cfg(not(feature = "native-http"))]
type CancelField = Option<std::convert::Infallible>;

pin_project! {
    /// Wraps a `bytes::Bytes` stream and yields parsed `ChatCompletionChunk`s.
    ///
    /// The `P` type parameter is the parse function used to translate a raw
    /// SSE `data:` payload string into a `ChatCompletionChunk`.  This allows
    /// non-OpenAI SSE formats (e.g. Anthropic, Vertex) to plug in their own
    /// event parsers without duplicating the byte-buffering and line-splitting
    /// logic.
    ///
    /// # BytesMut pool
    ///
    /// Incoming byte chunks are decoded from UTF-8 into the main `String`
    /// buffer directly.  A per-thread `BytesMut` scratch allocation is
    /// reserved for the lifetime of the stream and returned to the pool on
    /// `Drop` to amortise allocation cost across successive streams.
    struct SseParser<S, P> {
        #[pin]
        inner: S,
        buffer: String,
        // Read cursor into `buffer`.  All bytes before `cursor` have already
        // been processed.  We compact (drain) only when the cursor exceeds
        // half the buffer length, amortising memmove cost to O(total_bytes).
        cursor: usize,
        // Set to true once the inner stream is exhausted or cancelled.
        done: bool,
        // Provider-supplied event parser; translates raw SSE data payloads.
        parse_event: P,
        // Scratch buffer from the per-thread pool, held for the stream
        // lifetime and returned to the pool on Drop.
        scratch: Option<BytesMut>,
        // Optional cancellation signal.
        //
        // On native-http: `Option<CancellationToken>`.
        // On wasm-http: `Option<Infallible>` (always None, zero size).
        cancel: CancelField,
    }

    impl<S, P> PinnedDrop for SseParser<S, P> {
        fn drop(this: Pin<&mut Self>) {
            // Return the scratch buffer to the thread-local pool.
            let this = this.project();
            if let Some(buf) = this.scratch.take() {
                pool_release(buf);
            }
        }
    }
}

impl<S, P> SseParser<S, P>
where
    P: Fn(&str) -> Result<Option<ChatCompletionChunk>>,
{
    fn new(inner: S, parse_event: P, cancel: CancelField) -> Self {
        Self {
            inner,
            // Pre-allocate 4 KiB — a reasonable size for SSE lines to
            // reduce reallocations during the first few chunks.
            buffer: String::with_capacity(4096),
            cursor: 0,
            done: false,
            parse_event,
            scratch: Some(pool_acquire()),
            cancel,
        }
    }
}

impl<S, P> Stream for SseParser<S, P>
where
    S: Stream<Item = std::result::Result<Bytes, reqwest::Error>>,
    P: Fn(&str) -> Result<Option<ChatCompletionChunk>>,
{
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Check cancellation before doing any work.
        #[cfg(feature = "native-http")]
        if this.cancel.as_ref().is_some_and(|t| t.is_cancelled()) {
            #[cfg(feature = "tracing")]
            tracing::debug!("SSE stream cancelled by downstream disconnect");
            *this.done = true;
            return Poll::Ready(None);
        }

        loop {
            // --- Process any complete lines already in the buffer ---
            // Search for `\n` only in the unprocessed portion (from cursor onward).
            if let Some(offset) = memchr(b'\n', &this.buffer.as_bytes()[*this.cursor..]) {
                let newline_pos = *this.cursor + offset;

                // Borrow the line slice from cursor..newline_pos — zero allocation
                // on the hot path.  All decisions (empty check, prefix match, JSON
                // parse) operate on this borrowed `&str`.
                let line = this.buffer[*this.cursor..newline_pos].trim_end_matches('\r').trim();

                // Skip empty lines and SSE comments.
                if line.is_empty() || line.starts_with(':') {
                    *this.cursor = newline_pos + 1;
                    compact_if_needed(this.buffer, this.cursor);
                    continue;
                }

                if let Some(raw) = line.strip_prefix("data:") {
                    // Strip exactly one optional leading space (RFC 8895 §3.3).
                    let data = raw.strip_prefix(' ').unwrap_or(raw).trim();

                    // Handle the OpenAI `[DONE]` sentinel at the SSE parser
                    // level — this terminates the stream regardless of provider.
                    if data == "[DONE]" {
                        *this.cursor = newline_pos + 1;
                        compact_if_needed(this.buffer, this.cursor);
                        return Poll::Ready(None);
                    }

                    // Delegate to the provider-supplied parser.
                    // - `Ok(Some(chunk))` → yield the chunk.
                    // - `Ok(None)` → skip this event (e.g. Anthropic ping,
                    //   content_block_stop, message_stop) and continue parsing.
                    // - `Err(e)` → yield the error to the consumer.
                    let result = (this.parse_event)(data);
                    *this.cursor = newline_pos + 1;
                    compact_if_needed(this.buffer, this.cursor);
                    match result {
                        Ok(None) => continue,
                        Ok(Some(chunk)) => return Poll::Ready(Some(Ok(chunk))),
                        Err(e) => return Poll::Ready(Some(Err(e))),
                    }
                }

                // Ignore other SSE fields (event:, id:, retry:).
                *this.cursor = newline_pos + 1;
                compact_if_needed(this.buffer, this.cursor);
                continue;
            }

            // --- Buffer has only a partial line (or nothing unprocessed); fetch more bytes ---

            if *this.done {
                // Any bytes remaining in the buffer after the stream ends were
                // not terminated by a newline — they form an incomplete SSE
                // line that would be silently dropped.  Emit a warning so that
                // protocol bugs or truncated responses are visible in logs.
                let remaining = this.buffer.len() - *this.cursor;
                if remaining > 0 {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        leftover_bytes = remaining,
                        preview = &this.buffer[*this.cursor..(*this.cursor + remaining.min(64))],
                        "SSE stream ended with unterminated data in buffer; dropping partial line"
                    );
                    this.buffer.clear();
                    *this.cursor = 0;
                }
                return Poll::Ready(None);
            }

            // Re-check cancellation before blocking on the inner stream.
            #[cfg(feature = "native-http")]
            if this.cancel.as_ref().is_some_and(|t| t.is_cancelled()) {
                #[cfg(feature = "tracing")]
                tracing::debug!("SSE stream cancelled while waiting for next chunk");
                *this.done = true;
                return Poll::Ready(None);
            }

            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    // Guard against unbounded growth.
                    if this.buffer.len() + bytes.len() > MAX_BUFFER_BYTES {
                        // Mark done so subsequent polls don't continue reading.
                        *this.done = true;
                        return Poll::Ready(Some(Err(LiterLlmError::Streaming {
                            message: format!("SSE buffer exceeded {MAX_BUFFER_BYTES} bytes; stream aborted"),
                        })));
                    }
                    // Decode directly from the incoming `Bytes` slice into the
                    // main `String` buffer.
                    match std::str::from_utf8(&bytes) {
                        Ok(s) => this.buffer.push_str(s),
                        Err(e) => {
                            // Mark done so the next poll does not try to read
                            // more data from the (now-corrupt) stream.
                            *this.done = true;
                            return Poll::Ready(Some(Err(LiterLlmError::Streaming {
                                message: format!("invalid UTF-8 in SSE stream: {e}"),
                            })));
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(LiterLlmError::from(e))));
                }
                Poll::Ready(None) => {
                    *this.done = true;
                    // Loop once more to flush any remaining buffered line.
                    continue;
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}

/// Compact the buffer when the cursor has advanced past half the buffer length.
///
/// This amortises the O(n) memmove cost: instead of shifting bytes on every
/// line, we only compact when at least half the buffer is consumed, giving
/// amortised O(total_bytes) cost across the entire stream.
fn compact_if_needed(buffer: &mut String, cursor: &mut usize) {
    if *cursor > buffer.len() / 2 {
        buffer.drain(..*cursor);
        *cursor = 0;
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Parse a single SSE `data:` line into a `ChatCompletionChunk`.
///
/// Returns `None` for the terminal `[DONE]` sentinel.
///
/// Only used in crate-internal tests; external consumers should use the
/// streaming API instead.
#[cfg(test)]
pub(crate) fn parse_sse_line(line: &str) -> Option<Result<ChatCompletionChunk>> {
    // Strip "data:" then optionally one leading space (RFC 8895 §3.3).
    let raw = line.strip_prefix("data:")?;
    let data = raw.strip_prefix(' ').unwrap_or(raw).trim();
    if data == "[DONE]" {
        return None;
    }
    Some(serde_json::from_str(data).map_err(|e| LiterLlmError::Streaming {
        message: format!("failed to parse SSE data: {e}"),
    }))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ─── BytesMut pool tests ──────────────────────────────────────────────────

    #[test]
    fn bytes_pool_reuses_buffer() {
        // Acquire and release a buffer.
        let buf = pool_acquire();
        let ptr_before = buf.as_ptr();
        pool_release(buf);

        // A second acquire on the same thread should get the same backing store.
        let buf2 = pool_acquire();
        let ptr_after = buf2.as_ptr();

        // The pointer should be the same, confirming reuse.
        assert_eq!(ptr_before, ptr_after, "pool should reuse the same BytesMut allocation");
        pool_release(buf2);
    }

    #[test]
    fn bytes_pool_discards_oversized_buffers() {
        // Create a buffer that exceeds the pool cap.
        let mut big = BytesMut::with_capacity(MAX_POOL_BUFFER_CAPACITY + 1);
        // Write enough to actually allocate at least MAX_POOL_BUFFER_CAPACITY + 1.
        big.resize(MAX_POOL_BUFFER_CAPACITY + 1, 0u8);

        pool_release(big);

        // A subsequent acquire should not get the oversized buffer back.
        // It should instead allocate a fresh 4 KiB buffer.
        let acquired = pool_acquire();
        assert!(
            acquired.capacity() <= 4096,
            "oversized buffer should have been discarded; got capacity {}",
            acquired.capacity()
        );
        pool_release(acquired);
    }

    // ─── Cancellation tests ───────────────────────────────────────────────────

    #[cfg(feature = "native-http")]
    #[tokio::test]
    async fn cancellation_aborts_upstream() {
        use std::pin::Pin;
        use std::task::{Context, Poll};

        use bytes::Bytes;
        use futures_core::Stream;

        use crate::types::ChatCompletionChunk;

        /// A stream that never terminates and always returns `Poll::Pending`.
        struct InfiniteStream;

        impl Stream for InfiniteStream {
            type Item = std::result::Result<Bytes, reqwest::Error>;

            fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                Poll::Pending
            }
        }

        let token = CancellationToken::new();
        let token_clone = token.clone();

        // Build an SseParser wrapping the infinite stream.
        let mut parser: Pin<Box<SseParser<_, _>>> = Box::pin(SseParser::new(
            InfiniteStream,
            |_data: &str| -> Result<Option<ChatCompletionChunk>> { Ok(None) },
            Some(token_clone),
        ));

        // Cancel the token.
        token.cancel();

        // The stream should immediately return None (clean termination).
        let result = futures_util::StreamExt::next(&mut parser).await;
        assert!(result.is_none(), "cancelled stream should return None immediately");
    }
}
