#[cfg(test)]
use std::cell::RefCell;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
#[cfg(test)]
use bytes::BytesMut;
use futures_core::Stream;
use memchr::memchr;
use pin_project_lite::pin_project;

use crate::error::{LiterLlmError, Result};
use crate::http::request::{ResponseReadOptions, StreamingPost, with_retry_bounded};
#[cfg(test)]
use crate::types::ChatCompletionChunk;

/// Maximum number of bytes buffered before declaring a streaming error.
const MAX_BUFFER_BYTES: usize = 1024 * 1024;

/// Maximum number of characters of truncated/leftover data logged in a
/// truncation error message, to avoid dumping an unbounded payload.
const TRUNCATION_PREVIEW_CHARS: usize = 64;

/// Maximum capacity a reclaimed `BytesMut` buffer may have before it is
/// discarded rather than returned to the pool.  Prevents unbounded memory
/// accumulation on idle clients that previously processed very large chunks.
#[cfg(test)]
const MAX_POOL_BUFFER_CAPACITY: usize = 64 * 1024;

#[cfg(test)]
thread_local! {
    /// Per-thread pool of reusable `BytesMut` scratch buffers.
    ///
    /// Only one buffer is kept per thread to bound memory.  Buffers that have
    /// grown beyond `MAX_POOL_BUFFER_CAPACITY` are discarded so that a single
    /// large response doesn't permanently inflate per-thread memory.
    static BYTES_POOL: RefCell<Option<BytesMut>> = const { RefCell::new(None) };
}

/// Acquire a `BytesMut` scratch buffer from the per-thread pool.
///
/// Returns a recycled buffer when one is available; otherwise allocates a new
/// one pre-sized to 4 KiB (a reasonable first-chunk size for most SSE streams).
#[cfg(test)]
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
/// Buffers exceeding `MAX_POOL_BUFFER_CAPACITY` are dropped instead of
/// being pooled to prevent memory growth after large-response processing.
#[cfg(test)]
fn pool_release(buf: BytesMut) {
    if buf.capacity() <= MAX_POOL_BUFFER_CAPACITY {
        BYTES_POOL.with(|cell| {
            *cell.borrow_mut() = Some(buf);
        });
    }
}

/// A token that can be used to cancel an in-progress streaming response.
///
/// Pass to `post_stream_with_cancel` to abort the upstream SSE connection
/// when the downstream client disconnects.
///
/// Only available when the `native-http` feature is enabled.
#[cfg(feature = "native-http")]
pub use tokio_util::sync::CancellationToken;

/// Send a streaming POST request and return an SSE stream of parsed items of
/// type `T`.
///
/// Before opening the stream, retries on 429 / 500 / 502 / 503 / 504 up to
/// `max_retries` times honouring any `Retry-After` header.  Once the stream
/// is open, individual chunk errors are yielded as `Err` items rather than
/// causing a retry.
///
/// `request` carries the target URL, the optional auth header, any
/// provider-specific extra headers, and the body; see [`StreamingPost`].
///
/// `parse_event` translates a raw SSE `data:` payload string into a `T`.
/// Pass the provider's `parse_stream_event` method for chat completion
/// streams, or an endpoint-specific parser (e.g. Responses API events) for
/// other SSE-based endpoints.
#[allow(dead_code, reason = "retain the unconfigured raw request entry point")]
pub async fn post_stream<P, T>(
    client: &reqwest::Client,
    request: StreamingPost<'_>,
    max_retries: u32,
    parse_event: P,
) -> Result<crate::client::BoxStream<'static, Result<T>>>
where
    P: Fn(&str) -> Result<Option<T>> + Send + 'static,
    T: Send + 'static,
{
    post_stream_bounded(
        client,
        request,
        parse_event,
        ResponseReadOptions {
            max_retries,
            max_response_bytes: None,
        },
    )
    .await
}

#[tracing::instrument(
    name = "post_stream",
    level = "debug",
    skip_all,
    fields(
        http.method = "POST",
        http.url = %request.url,
        http.status_code = tracing::field::Empty,
        http.retry_count = tracing::field::Empty,
    )
)]
pub(crate) async fn post_stream_bounded<P, T>(
    client: &reqwest::Client,
    request: StreamingPost<'_>,
    parse_event: P,
    options: ResponseReadOptions,
) -> Result<crate::client::BoxStream<'static, Result<T>>>
where
    P: Fn(&str) -> Result<Option<T>> + Send + 'static,
    T: Send + 'static,
{
    let resp = send_streaming_post(client, &request, options).await?;
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
#[allow(dead_code)]
pub async fn post_stream_with_cancel<P, T>(
    client: &reqwest::Client,
    request: StreamingPost<'_>,
    max_retries: u32,
    parse_event: P,
    cancel: CancellationToken,
) -> Result<crate::client::BoxStream<'static, Result<T>>>
where
    P: Fn(&str) -> Result<Option<T>> + Send + 'static,
    T: Send + 'static,
{
    post_stream_with_cancel_bounded(
        client,
        request,
        parse_event,
        cancel,
        ResponseReadOptions {
            max_retries,
            max_response_bytes: None,
        },
    )
    .await
}

#[tracing::instrument(
    name = "post_stream_with_cancel",
    level = "debug",
    skip_all,
    fields(
        http.method = "POST",
        http.url = %request.url,
        http.status_code = tracing::field::Empty,
        http.retry_count = tracing::field::Empty,
    )
)]
#[cfg(feature = "native-http")]
#[allow(dead_code)]
pub(crate) async fn post_stream_with_cancel_bounded<P, T>(
    client: &reqwest::Client,
    request: StreamingPost<'_>,
    parse_event: P,
    cancel: CancellationToken,
    options: ResponseReadOptions,
) -> Result<crate::client::BoxStream<'static, Result<T>>>
where
    P: Fn(&str) -> Result<Option<T>> + Send + 'static,
    T: Send + 'static,
{
    let resp = send_streaming_post(client, &request, options).await?;
    let byte_stream = resp.bytes_stream();
    let stream = SseParser::new(byte_stream, parse_event, Some(cancel));
    Ok(Box::pin(stream))
}

/// Send `request` as a JSON `POST` under the retry policy in `options`.
///
/// Once a response is obtained, its status and the retry count are recorded on
/// the calling `post_stream*` span; a terminal error returns before anything is
/// recorded.
async fn send_streaming_post(
    client: &reqwest::Client,
    request: &StreamingPost<'_>,
    options: ResponseReadOptions,
) -> Result<reqwest::Response> {
    let ResponseReadOptions {
        max_retries,
        max_response_bytes,
    } = options;
    let mut retry_count = 0u32;

    let resp = with_retry_bounded(request.url, max_retries, max_response_bytes, || {
        let mut builder = client
            .post(request.url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(request.body.clone());
        if let Some((name, value)) = request.auth_header {
            builder = builder.header(name, value);
        }
        for (name, value) in request.extra_headers {
            builder = builder.header(*name, *value);
        }
        retry_count += 1;
        builder.send()
    })
    .await?;

    {
        let span = tracing::Span::current();
        span.record("http.status_code", resp.status().as_u16());
        span.record("http.retry_count", retry_count.saturating_sub(1));
    }

    Ok(resp)
}

// ~keep `pin_project_lite` cannot cfg individual fields; WASM uses a zero-size Infallible cancel field.

#[cfg(feature = "native-http")]
type CancelField = Option<CancellationToken>;

#[cfg(not(feature = "native-http"))]
type CancelField = Option<std::convert::Infallible>;

pin_project! {
    /// Wraps a `bytes::Bytes` stream and yields parsed items of type `T`.
    ///
    /// The `P` type parameter is the parse function used to translate a raw
    /// SSE `data:` payload string into a `T`.  This allows different SSE
    /// payload shapes (chat completion chunks, non-OpenAI chat formats such
    /// as Anthropic/Vertex, or unrelated event types such as Responses API
    /// events) to plug in their own event parsers without duplicating the
    /// byte-buffering and line-splitting logic.
    ///
    /// # Buffer strategy
    ///
    /// Incoming byte chunks are decoded from UTF-8 into a `String` buffer
    /// directly.  A read cursor tracks the processed position; the buffer is
    /// compacted (drained) only when the cursor has advanced past half its
    /// length, amortising memmove cost to O(total_bytes) across the stream.
    ///
    /// `SseParser` does not hold a `BytesMut` scratch allocation.
    /// Acquiring a buffer on construction and returning it on drop would pin
    /// ~4 KiB per idle stream with no benefit since no parsing occurs until
    /// the first byte arrives.
    struct SseParser<S, P, T> {
        #[pin]
        inner: S,
        buffer: String,
        // ~keep Trailing bytes of a multi-byte UTF-8 codepoint that was split
        // across upstream chunks. Holds at most 3 bytes (a UTF-8 codepoint is <=4
        // bytes and the valid prefix is always flushed into `buffer` immediately),
        // so it cannot grow unbounded.
        pending: Vec<u8>,
        cursor: usize,
        done: bool,
        parse_event: P,
        cancel: CancelField,
        // ~keep `fn() -> T` marker keeps `SseParser` Send/Sync regardless of `T`,
        // since the parser never actually stores a `T`; it only produces one per item.
        _marker: std::marker::PhantomData<fn() -> T>,
    }

    impl<S, P, T> PinnedDrop for SseParser<S, P, T> {
        fn drop(this: Pin<&mut Self>) {
            let _ = this;
        }
    }
}

impl<S, P, T> SseParser<S, P, T>
where
    P: Fn(&str) -> Result<Option<T>>,
{
    fn new(inner: S, parse_event: P, cancel: CancelField) -> Self {
        Self {
            inner,
            buffer: String::with_capacity(4096),
            pending: Vec::new(),
            cursor: 0,
            done: false,
            parse_event,
            cancel,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, P, T> Stream for SseParser<S, P, T>
where
    S: Stream<Item = std::result::Result<Bytes, reqwest::Error>>,
    P: Fn(&str) -> Result<Option<T>>,
{
    type Item = Result<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        #[cfg(feature = "native-http")]
        if this.cancel.as_ref().is_some_and(|t| t.is_cancelled()) {
            tracing::debug!("SSE stream cancelled by downstream disconnect");
            *this.done = true;
            return Poll::Ready(None);
        }

        loop {
            match take_buffered_line(this.buffer, this.cursor, &*this.parse_event) {
                BufferedLine::Incomplete => {}
                BufferedLine::Consumed => continue,
                BufferedLine::Done => return Poll::Ready(None),
                BufferedLine::Item(item) => return Poll::Ready(Some(item)),
            }

            if *this.done {
                return Poll::Ready(truncation_error_at_eof(this.buffer, this.cursor, this.pending).map(Err));
            }

            // ~keep Re-check cancellation before blocking on the inner stream.
            #[cfg(feature = "native-http")]
            if this.cancel.as_ref().is_some_and(|t| t.is_cancelled()) {
                tracing::debug!("SSE stream cancelled while waiting for next chunk");
                *this.done = true;
                return Poll::Ready(None);
            }

            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    if let Some(e) = append_chunk(this.buffer, this.pending, &bytes) {
                        *this.done = true;
                        return Poll::Ready(Some(Err(e)));
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(LiterLlmError::from(e))));
                }
                Poll::Ready(None) => {
                    *this.done = true;
                    continue;
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}

/// What the line at the read cursor produced.
enum BufferedLine<T> {
    /// No complete line is buffered yet; more bytes are needed.
    Incomplete,
    /// A line was consumed without yielding anything: a blank line, an SSE
    /// comment, a non-`data:` field, or an event the parser chose to skip.
    Consumed,
    /// The `[DONE]` sentinel was consumed.
    Done,
    /// A `data:` payload was parsed into an item or an error.
    Item(Result<T>),
}

/// Consume the next complete line from `buffer` (starting at `cursor`) and
/// classify it, parsing `data:` payloads with `parse_event`.
fn take_buffered_line<T, P>(buffer: &mut String, cursor: &mut usize, parse_event: &P) -> BufferedLine<T>
where
    P: Fn(&str) -> Result<Option<T>>,
{
    let Some(offset) = memchr(b'\n', &buffer.as_bytes()[*cursor..]) else {
        return BufferedLine::Incomplete;
    };
    let newline_pos = *cursor + offset;

    let line = buffer[*cursor..newline_pos].trim_end_matches('\r').trim();

    let outcome = if line.is_empty() || line.starts_with(':') {
        BufferedLine::Consumed
    } else if let Some(raw) = line.strip_prefix("data:") {
        let data = raw.strip_prefix(' ').unwrap_or(raw).trim();

        // ~keep `[DONE]` terminates at the SSE parser level regardless of provider.
        if data == "[DONE]" {
            BufferedLine::Done
        } else {
            match parse_event(data) {
                Ok(None) => BufferedLine::Consumed,
                Ok(Some(chunk)) => BufferedLine::Item(Ok(chunk)),
                Err(e) => BufferedLine::Item(Err(e)),
            }
        }
    } else {
        BufferedLine::Consumed
    };

    *cursor = newline_pos + 1;
    compact_if_needed(buffer, cursor);
    outcome
}

/// Once the inner stream has ended, report any bytes left in the buffers as a
/// truncation error, clearing the leftover data it finds. `None` means a clean
/// end, in which case there was nothing left to clear.
fn truncation_error_at_eof(buffer: &mut String, cursor: &mut usize, pending: &mut Vec<u8>) -> Option<LiterLlmError> {
    // ~keep Leftover bytes at EOF are an incomplete SSE line: the connection was
    // cut mid-event. That is data loss, not a clean end, so it must surface as
    // an `Err` item rather than a silent `Poll::Ready(None)` (see #44).
    let remaining = buffer.len() - *cursor;
    if remaining > 0 {
        let leftover = buffer[*cursor..].trim();
        if !leftover.is_empty() {
            let preview: String = leftover.chars().take(TRUNCATION_PREVIEW_CHARS).collect();
            tracing::error!(
                leftover_bytes = remaining,
                preview = %preview,
                "SSE stream ended with unterminated data in buffer; stream was truncated"
            );
            buffer.clear();
            *cursor = 0;
            pending.clear();
            return Some(LiterLlmError::Streaming {
                message: format!(
                    "SSE stream truncated: {remaining} bytes of incomplete data at end of stream \
                     (starts with: {preview:?})"
                ),
            });
        }
        buffer.clear();
        *cursor = 0;
    }
    // ~keep Bytes still pending at EOF are a codepoint split by the connection
    // closing mid-character — also truncation, not something to drop silently.
    if !pending.is_empty() {
        let pending_len = pending.len();
        pending.clear();
        tracing::error!(
            pending_bytes = pending_len,
            "SSE stream ended mid-codepoint; stream was truncated"
        );
        return Some(LiterLlmError::Streaming {
            message: format!("SSE stream truncated: {pending_len} bytes of incomplete UTF-8 at end of stream"),
        });
    }
    None
}

/// Decode an upstream chunk into `buffer`, carrying any split trailing
/// codepoint in `pending`. Returns the error that must end the stream when the
/// buffer limit is exceeded or the bytes are not valid UTF-8.
fn append_chunk(buffer: &mut String, pending: &mut Vec<u8>, bytes: &[u8]) -> Option<LiterLlmError> {
    if buffer.len() + pending.len() + bytes.len() > MAX_BUFFER_BYTES {
        return Some(LiterLlmError::Streaming {
            message: format!("SSE buffer exceeded {MAX_BUFFER_BYTES} bytes; stream aborted"),
        });
    }
    // ~keep Prepend any bytes left over from a codepoint that was
    // split across the previous chunk, then decode what is now valid.
    pending.extend_from_slice(bytes);
    match std::str::from_utf8(pending) {
        Ok(s) => {
            buffer.push_str(s);
            pending.clear();
        }
        Err(e) => {
            let valid = e.valid_up_to();
            let complete_error = e.error_len().is_some();
            // SAFETY: `valid_up_to()` bytes are guaranteed to be valid UTF-8.
            buffer.push_str(unsafe { std::str::from_utf8_unchecked(&pending[..valid]) });
            if complete_error {
                // ~keep `error_len()` is `Some` only for genuinely
                // malformed UTF-8, not a codepoint split across a chunk
                // boundary; that corrupts the SSE stream, so stop polling.
                return Some(LiterLlmError::Streaming {
                    message: format!("invalid UTF-8 in SSE stream: {e}"),
                });
            }
            // ~keep Incomplete trailing codepoint: keep the tail for the next chunk.
            pending.drain(..valid);
        }
    }
    None
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

/// Parse a single SSE `data:` line into a `ChatCompletionChunk`.
///
/// Returns `None` for the terminal `[DONE]` sentinel.
///
/// Only used in crate-internal tests; external consumers should use the
/// streaming API instead.
#[cfg(test)]
pub(crate) fn parse_sse_line(line: &str) -> Option<Result<ChatCompletionChunk>> {
    let raw = line.strip_prefix("data:")?;
    let data = raw.strip_prefix(' ').unwrap_or(raw).trim();
    if data == "[DONE]" {
        return None;
    }
    Some(serde_json::from_str(data).map_err(|e| LiterLlmError::Streaming {
        message: format!("failed to parse SSE data: {e}"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_pool_reuses_buffer() {
        let buf = pool_acquire();
        let ptr_before = buf.as_ptr();
        pool_release(buf);

        let buf2 = pool_acquire();
        let ptr_after = buf2.as_ptr();

        assert_eq!(ptr_before, ptr_after, "pool should reuse the same BytesMut allocation");
        pool_release(buf2);
    }

    #[test]
    fn bytes_pool_discards_oversized_buffers() {
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

    #[test]
    fn sse_parser_does_not_hold_idle_buffer_when_stream_idle() {
        use std::pin::Pin;
        use std::task::{Context, Poll};

        use bytes::Bytes;
        use futures_core::Stream;

        struct NeverStream;

        impl Stream for NeverStream {
            type Item = std::result::Result<Bytes, reqwest::Error>;

            fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                Poll::Pending
            }
        }

        let sentinel = pool_acquire();
        let sentinel_ptr = sentinel.as_ptr();
        pool_release(sentinel);

        let parser = SseParser::new(
            NeverStream,
            |_data: &str| -> Result<Option<crate::types::ChatCompletionChunk>> { Ok(None) },
            None,
        );

        let reclaimed = pool_acquire();
        assert_eq!(
            reclaimed.as_ptr(),
            sentinel_ptr,
            "SseParser constructor must not acquire a BytesMut from the pool"
        );
        pool_release(reclaimed);

        drop(parser);

        let after_drop = pool_acquire();
        assert_eq!(
            after_drop.as_ptr(),
            sentinel_ptr,
            "SseParser Drop must not corrupt the pool slot"
        );
        pool_release(after_drop);
    }

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

        let mut parser: Pin<Box<SseParser<_, _, _>>> = Box::pin(SseParser::new(
            InfiniteStream,
            |_data: &str| -> Result<Option<ChatCompletionChunk>> { Ok(None) },
            Some(token_clone),
        ));

        token.cancel();

        let result = futures_util::StreamExt::next(&mut parser).await;
        assert!(result.is_none(), "cancelled stream should return None immediately");
    }

    /// Parse closure over the crate's chunk type for the boundary tests.
    #[cfg(test)]
    fn json_parse(data: &str) -> Result<Option<ChatCompletionChunk>> {
        serde_json::from_str(data)
            .map(Some)
            .map_err(|e| LiterLlmError::Streaming { message: e.to_string() })
    }

    #[tokio::test]
    async fn should_reassemble_multibyte_char_split_across_chunks() {
        use crate::types::chat::{StreamChoice, StreamDelta};

        // Content mixes CJK and an emoji so several codepoints are multi-byte.
        let content = "你好，世界 🌍 café";
        let chunk = ChatCompletionChunk {
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
        };
        let full = format!("data: {}\n\ndata: [DONE]\n\n", serde_json::to_string(&chunk).unwrap());
        let bytes = full.into_bytes();

        // Pick a split index that lands *inside* a multi-byte codepoint.
        let split = (1..bytes.len())
            .find(|&i| std::str::from_utf8(&bytes[..i]).is_err())
            .expect("input contains multi-byte codepoints");

        let inner = futures_util::stream::iter(vec![
            Ok::<_, reqwest::Error>(Bytes::from(bytes[..split].to_vec())),
            Ok::<_, reqwest::Error>(Bytes::from(bytes[split..].to_vec())),
        ]);
        let mut parser: Pin<Box<SseParser<_, _, _>>> = Box::pin(SseParser::new(inner, json_parse, None));

        let result = futures_util::StreamExt::next(&mut parser)
            .await
            .expect("should yield one chunk")
            .expect("split codepoint must not abort the stream");
        assert_eq!(result.choices[0].delta.content.as_deref(), Some(content));

        assert!(futures_util::StreamExt::next(&mut parser).await.is_none());
    }

    #[tokio::test]
    async fn should_error_on_genuinely_invalid_utf8() {
        // A lone 0xFF is never valid UTF-8 anywhere in a sequence.
        let inner = futures_util::stream::iter(vec![
            Ok::<_, reqwest::Error>(Bytes::from_static(b"data: ")),
            Ok::<_, reqwest::Error>(Bytes::from_static(&[0xFF])),
            Ok::<_, reqwest::Error>(Bytes::from_static(b"\n\n")),
        ]);
        let mut parser: Pin<Box<SseParser<_, _, _>>> = Box::pin(SseParser::new(inner, json_parse, None));

        match futures_util::StreamExt::next(&mut parser).await {
            Some(Err(LiterLlmError::Streaming { message })) => {
                assert!(message.contains("invalid UTF-8"), "unexpected message: {message}");
            }
            other => panic!("expected a Streaming UTF-8 error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn truncated_stream_surfaces_as_error_not_clean_eof() {
        // ~keep Regression test for #44: a connection cut mid-event (no trailing "\n\n")
        // must surface as an `Err`, not be silently dropped as `Poll::Ready(None)`.
        let inner = futures_util::stream::iter(vec![Ok::<_, reqwest::Error>(Bytes::from_static(
            br#"data: {"id":"trunc","choices":[{"delta":{"content":"partial"#,
        ))]);
        let mut parser: Pin<Box<SseParser<_, _, _>>> = Box::pin(SseParser::new(inner, json_parse, None));

        match futures_util::StreamExt::next(&mut parser).await {
            Some(Err(LiterLlmError::Streaming { message })) => {
                assert!(message.contains("truncated"), "unexpected message: {message}");
            }
            other => panic!("truncated stream must yield an error, not {other:?}"),
        }

        // ~keep After the truncation error, the stream must end (no repeated errors, no panic).
        assert!(futures_util::StreamExt::next(&mut parser).await.is_none());
    }

    #[tokio::test]
    async fn truncated_stream_mid_codepoint_surfaces_as_error() {
        // ~keep Regression test for #44: a connection cut mid-UTF-8-codepoint must also
        // be reported as truncation, not silently dropped.
        let content = "café"; // "é" is 2 bytes in UTF-8; split right before its second byte.
        let full = format!(r#"data: {{"text":"{content}"#);
        let bytes = full.into_bytes();
        let split = bytes.len() - 1; // leaves the last byte of "é" pending
        assert!(
            std::str::from_utf8(&bytes[..split]).is_err(),
            "split must land inside a codepoint"
        );

        let inner = futures_util::stream::iter(vec![Ok::<_, reqwest::Error>(Bytes::from(bytes[..split].to_vec()))]);
        let mut parser: Pin<Box<SseParser<_, _, _>>> = Box::pin(SseParser::new(inner, json_parse, None));

        match futures_util::StreamExt::next(&mut parser).await {
            Some(Err(LiterLlmError::Streaming { message })) => {
                assert!(message.contains("truncated"), "unexpected message: {message}");
            }
            other => panic!("mid-codepoint truncation must yield an error, not {other:?}"),
        }
    }
}
