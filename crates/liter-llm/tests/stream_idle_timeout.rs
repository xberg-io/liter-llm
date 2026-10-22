//! A stream is limited by the gap between chunks, not by its total length.
//!
//! The mock server writes each chunk after a pause. `timeout` on the client config
//! is shorter than the whole stream and longer than every pause.

use std::time::{Duration, Instant};

use futures_util::StreamExt;
use liter_llm::client::{BoxStream, ResponseClient};
use liter_llm::types::responses::{CreateResponseRequest, ResponseStreamEvent};
use liter_llm::{
    ChatCompletionChunk, ChatCompletionRequest, ClientConfigBuilder, DefaultClient, LiterLlmError, LlmClient,
    LlmClientRaw,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Idle limit used by the silent stream tests.
const SILENT_TIMEOUT: Duration = Duration::from_millis(500);

/// How long the mock server stays silent. The server holds the socket this long and then drops it,
/// so a client that has no idle limit sees a closed stream, not a `Timeout` error.
const SILENT_STALL: Duration = Duration::from_secs(10);

/// A limit that works ends its call within this many times the configured limit. A slow or loaded
/// process adds well under one limit of delay to a call; a limit set 10 times too long misses the
/// ceiling by a factor of two.
const CEILING_FACTOR: u32 = 5;

/// The ceiling for a call that has a configured `limit`.
fn ceiling(limit: Duration) -> Duration {
    limit * CEILING_FACTOR
}

/// Read a silent stream to its error and check the idle limit.
///
/// The gap is the time since the stream last yielded an item, so it excludes the time that setup
/// takes on a slow process. The client starts its timer when it reads the last bytes, a moment before
/// this function sees the item, so the gap can be a little under the limit: the floor is half the
/// limit. The ceiling is five limits.
async fn assert_silent_stream_ends_at_the_idle_limit(mut stream: BoxStream<'static, Result<String, LiterLlmError>>) {
    let mut quiet_since = Instant::now();
    let mut last_err = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => quiet_since = Instant::now(),
            Err(e) => {
                last_err = Some(e);
                break;
            }
        }
    }
    let gap = quiet_since.elapsed();
    let err = last_err.expect("a silent stream must fail");
    assert!(matches!(err, LiterLlmError::Timeout), "got {err:?}");
    assert!(
        gap >= SILENT_TIMEOUT / 2,
        "the stream ended well before the idle limit, after {gap:?}"
    );
    assert!(
        gap < ceiling(SILENT_TIMEOUT),
        "the idle limit did not fire within {CEILING_FACTOR} limits, the stream was quiet for {gap:?}"
    );
}

/// What the mock server does after it reads the request.
#[derive(Clone, Copy)]
struct Script {
    /// Pause before the response headers.
    header_pause: Duration,
    /// Pause before each content chunk.
    chunk_pause: Duration,
    /// Number of content chunks.
    chunks: usize,
    /// When true the server sends the first chunk, then stays silent for `SILENT_STALL`.
    stall_after_first: bool,
    /// How the events are framed on the wire.
    wire: Wire,
}

/// The three wire formats the streaming entry points read.
#[derive(Clone, Copy)]
enum Wire {
    /// Chat completion chunks as server-sent events.
    ChatSse,
    /// Responses API events as server-sent events.
    ResponsesSse,
    /// Bedrock binary event stream frames.
    EventStream,
}

fn chunked(data: &[u8]) -> Vec<u8> {
    let mut out = format!("{:x}\r\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\r\n");
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// One Bedrock event stream frame with string headers.
fn frame(event_type: &str, payload: &str) -> Vec<u8> {
    let mut headers = Vec::new();
    for (name, value) in [(":message-type", "event"), (":event-type", event_type)] {
        headers.push(name.len() as u8);
        headers.extend_from_slice(name.as_bytes());
        headers.push(7);
        headers.extend_from_slice(&(value.len() as u16).to_be_bytes());
        headers.extend_from_slice(value.as_bytes());
    }
    let total = 12 + headers.len() + payload.len() + 4;
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&(total as u32).to_be_bytes());
    out.extend_from_slice(&(headers.len() as u32).to_be_bytes());
    let prelude_crc = crc32(&out[..8]);
    out.extend_from_slice(&prelude_crc.to_be_bytes());
    out.extend_from_slice(&headers);
    out.extend_from_slice(payload.as_bytes());
    let message_crc = crc32(&out);
    out.extend_from_slice(&message_crc.to_be_bytes());
    out
}

/// A content event carrying `text`, in the wire format of the script.
fn text_event(wire: Wire, text: &str) -> Vec<u8> {
    match wire {
        Wire::ChatSse => {
            let chunk = serde_json::json!({
                "id": "c1", "object": "chat.completion.chunk", "created": 1, "model": "m",
                "choices": [{"index": 0, "delta": {"content": text}, "finish_reason": null}],
            });
            chunked(format!("data: {chunk}\n\n").as_bytes())
        }
        Wire::ResponsesSse => {
            let event = serde_json::json!({
                "type": "response.output_text.delta",
                "item_id": "i1", "output_index": 0, "content_index": 0, "delta": text,
            });
            chunked(format!("data: {event}\n\n").as_bytes())
        }
        Wire::EventStream => {
            let payload = serde_json::json!({"contentBlockIndex": 0, "delta": {"text": text}});
            chunked(&frame("contentBlockDelta", &payload.to_string()))
        }
    }
}

/// The closing events and the end of the chunked body.
fn closing(wire: Wire) -> Vec<u8> {
    let mut out = match wire {
        Wire::ChatSse => {
            let chunk = serde_json::json!({
                "id": "c1", "object": "chat.completion.chunk", "created": 1, "model": "m",
                "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
            });
            let mut o = chunked(format!("data: {chunk}\n\n").as_bytes());
            o.extend(chunked(b"data: [DONE]\n\n"));
            o
        }
        Wire::ResponsesSse => chunked(b"data: [DONE]\n\n"),
        Wire::EventStream => chunked(&frame("messageStop", r#"{"stopReason":"end_turn"}"#)),
    };
    out.extend_from_slice(b"0\r\n\r\n");
    out
}

fn content_type(wire: Wire) -> &'static str {
    match wire {
        Wire::ChatSse | Wire::ResponsesSse => "text/event-stream",
        Wire::EventStream => "application/vnd.amazon.eventstream",
    }
}

async fn serve_one(mut sock: TcpStream, script: Script) {
    let mut buf = vec![0u8; 16384];
    let mut seen = Vec::new();
    // Read until the end of the request headers; the body is small and arrives with them.
    while !seen.windows(4).any(|w| w == b"\r\n\r\n") {
        match sock.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => seen.extend_from_slice(&buf[..n]),
        }
    }
    tokio::time::sleep(script.header_pause).await;
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nTransfer-Encoding: chunked\r\n\r\n",
        content_type(script.wire)
    );
    if sock.write_all(head.as_bytes()).await.is_err() {
        return;
    }
    let _ = sock.write_all(&text_event(script.wire, "")).await;
    if script.stall_after_first {
        tokio::time::sleep(SILENT_STALL).await;
        return;
    }
    for i in 0..script.chunks {
        tokio::time::sleep(script.chunk_pause).await;
        if sock
            .write_all(&text_event(script.wire, &format!("w{i} ")))
            .await
            .is_err()
        {
            return;
        }
    }
    let _ = sock.write_all(&closing(script.wire)).await;
}

async fn start(script: Script) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((sock, _)) = listener.accept().await {
            tokio::spawn(serve_one(sock, script));
        }
    });
    format!("http://{addr}/v1")
}

fn client(base: String, timeout: Duration) -> DefaultClient {
    let config = ClientConfigBuilder::new("k")
        .base_url(base)
        .timeout(timeout)
        .max_retries(0)
        .build();
    DefaultClient::new(config, Some("openai")).unwrap()
}

fn request() -> ChatCompletionRequest {
    serde_json::from_value(serde_json::json!({
        "model": "m",
        "messages": [{"role": "user", "content": "hi"}],
    }))
    .unwrap()
}

#[tokio::test]
async fn stream_outlives_the_timeout_while_chunks_keep_arriving() {
    // 6 chunks, 400 ms apart: about 2.4 s in total, against a 1 s timeout.
    let base = start(Script {
        header_pause: Duration::ZERO,
        chunk_pause: Duration::from_millis(400),
        chunks: 6,
        stall_after_first: false,
        wire: Wire::ChatSse,
    })
    .await;
    let c = client(base, Duration::from_secs(1));
    let started = Instant::now();
    let mut stream = c.chat_stream(request()).await.expect("stream opens");
    let mut text = String::new();
    while let Some(item) = stream.next().await {
        let chunk = item.expect("a healthy stream must not fail");
        if let Some(t) = chunk.choices.first().and_then(|ch| ch.delta.content.as_deref()) {
            text.push_str(t);
        }
    }
    assert_eq!(text, "w0 w1 w2 w3 w4 w5 ");
    assert!(
        started.elapsed() > Duration::from_secs(2),
        "the stream must outlast the timeout"
    );
}

#[tokio::test]
async fn stream_that_goes_silent_fails_with_the_timeout_error() {
    let base = start(Script {
        header_pause: Duration::ZERO,
        chunk_pause: Duration::ZERO,
        chunks: 0,
        stall_after_first: true,
        wire: Wire::ChatSse,
    })
    .await;
    let c = client(base, SILENT_TIMEOUT);
    let stream = c.chat_stream(request()).await.expect("stream opens");
    assert_silent_stream_ends_at_the_idle_limit(Box::pin(stream.map(|r| r.map(|c| chunk_text(&c))))).await;
}

#[tokio::test]
async fn stream_with_no_response_headers_fails_with_the_timeout_error() {
    let base = start(Script {
        header_pause: Duration::from_secs(5),
        chunk_pause: Duration::ZERO,
        chunks: 0,
        stall_after_first: false,
        wire: Wire::ChatSse,
    })
    .await;
    let c = client(base, Duration::from_millis(500));
    let result = c.chat_stream(request()).await;
    assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {:?}", result.err());
}

/// The streaming entry points besides `chat_stream`.
#[derive(Clone, Copy)]
enum Entry {
    ChatRaw,
    Responses,
    #[cfg(feature = "bedrock")]
    Bedrock,
    #[cfg(feature = "bedrock")]
    BedrockRaw,
}

const BEDROCK_MODEL: &str = "bedrock/anthropic.claude-3-haiku-20240307-v1:0";

fn chunk_text(chunk: &ChatCompletionChunk) -> String {
    chunk
        .choices
        .first()
        .and_then(|c| c.delta.content.clone())
        .unwrap_or_default()
}

#[cfg(feature = "bedrock")]
fn bedrock_client(base: String, timeout: Duration) -> DefaultClient {
    let mut config = ClientConfigBuilder::new("").load_env(false).base_url(base).build();
    config.timeout = timeout;
    config.max_retries = 0;
    config.bedrock_region = Some("us-east-1".into());
    config.bedrock_access_key_id = Some("key".into());
    config.bedrock_secret_access_key = Some("secret".into());
    DefaultClient::new(config, Some(BEDROCK_MODEL)).unwrap()
}

/// Open the stream of `entry` and map every item to its text.
async fn open(entry: Entry, base: String, timeout: Duration) -> BoxStream<'static, Result<String, LiterLlmError>> {
    match entry {
        Entry::ChatRaw => {
            let raw = client(base, timeout)
                .chat_stream_raw(request())
                .await
                .expect("stream opens");
            Box::pin(raw.stream.map(|r| r.map(|c| chunk_text(&c))))
        }
        Entry::Responses => {
            let req = CreateResponseRequest {
                model: "m".into(),
                input: serde_json::json!("hi"),
                instructions: None,
                tools: None,
                temperature: None,
                max_output_tokens: None,
                metadata: None,
                extra_body: None,
                stream: None,
            };
            let stream = client(base, timeout)
                .create_response_stream(req)
                .await
                .expect("stream opens");
            Box::pin(stream.map(|r| {
                r.map(|e| match e {
                    ResponseStreamEvent::OutputTextDelta(d) => d.delta,
                    _ => String::new(),
                })
            }))
        }
        #[cfg(feature = "bedrock")]
        Entry::Bedrock => {
            let mut req = request();
            req.model = BEDROCK_MODEL.into();
            let stream = bedrock_client(base, timeout)
                .chat_stream(req)
                .await
                .expect("stream opens");
            Box::pin(stream.map(|r| r.map(|c| chunk_text(&c))))
        }
        #[cfg(feature = "bedrock")]
        Entry::BedrockRaw => {
            let mut req = request();
            req.model = BEDROCK_MODEL.into();
            let raw = bedrock_client(base, timeout)
                .chat_stream_raw(req)
                .await
                .expect("stream opens");
            Box::pin(raw.stream.map(|r| r.map(|c| chunk_text(&c))))
        }
    }
}

async fn healthy_stream_outlives_the_timeout(entry: Entry, wire: Wire) {
    // 6 chunks, 400 ms apart: about 2.4 s in total, against a 1 s timeout.
    let base = start(Script {
        header_pause: Duration::ZERO,
        chunk_pause: Duration::from_millis(400),
        chunks: 6,
        stall_after_first: false,
        wire,
    })
    .await;
    let started = Instant::now();
    let mut stream = open(entry, base, Duration::from_secs(1)).await;
    let mut text = String::new();
    while let Some(item) = stream.next().await {
        text.push_str(&item.expect("a healthy stream must not fail"));
    }
    assert_eq!(text, "w0 w1 w2 w3 w4 w5 ");
    assert!(
        started.elapsed() > Duration::from_secs(2),
        "the stream must outlast the timeout"
    );
}

async fn silent_stream_fails_with_the_timeout_error(entry: Entry, wire: Wire) {
    let base = start(Script {
        header_pause: Duration::ZERO,
        chunk_pause: Duration::ZERO,
        chunks: 0,
        stall_after_first: true,
        wire,
    })
    .await;
    let stream = open(entry, base, SILENT_TIMEOUT).await;
    assert_silent_stream_ends_at_the_idle_limit(stream).await;
}

macro_rules! idle_tests {
    ($healthy:ident, $silent:ident, $entry:expr, $wire:expr) => {
        #[tokio::test]
        async fn $healthy() {
            healthy_stream_outlives_the_timeout($entry, $wire).await;
        }

        #[tokio::test]
        async fn $silent() {
            silent_stream_fails_with_the_timeout_error($entry, $wire).await;
        }
    };
}

idle_tests!(
    raw_chat_stream_outlives_the_timeout,
    raw_chat_stream_that_goes_silent_fails,
    Entry::ChatRaw,
    Wire::ChatSse
);
idle_tests!(
    responses_stream_outlives_the_timeout,
    responses_stream_that_goes_silent_fails,
    Entry::Responses,
    Wire::ResponsesSse
);
#[cfg(feature = "bedrock")]
idle_tests!(
    bedrock_stream_outlives_the_timeout,
    bedrock_stream_that_goes_silent_fails,
    Entry::Bedrock,
    Wire::EventStream
);
#[cfg(feature = "bedrock")]
idle_tests!(
    bedrock_raw_stream_outlives_the_timeout,
    bedrock_raw_stream_that_goes_silent_fails,
    Entry::BedrockRaw,
    Wire::EventStream
);

const UNARY_BODY: &str = r#"{"id":"x","object":"chat.completion","created":0,"model":"m","choices":[{"index":0,"message":{"role":"assistant","content":"hi"},"finish_reason":"stop"}]}"#;

/// Pieces in the dripped reply.
const DRIP_PIECES: usize = 40;

/// Whole-request limit of the dripped-reply tests.
const WHOLE_REQUEST_LIMIT: Duration = Duration::from_secs(1);

/// Serve one complete reply with a `Content-Length`, dripped in `DRIP_PIECES` pieces 200 ms apart (about 8 s, longer than the ceiling of the 1 s limit).
async fn drip_unary(mut sock: TcpStream) {
    let mut buf = vec![0u8; 16384];
    let mut seen = Vec::new();
    while !seen.windows(4).any(|w| w == b"\r\n\r\n") {
        match sock.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => seen.extend_from_slice(&buf[..n]),
        }
    }
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        UNARY_BODY.len()
    );
    if sock.write_all(head.as_bytes()).await.is_err() {
        return;
    }
    for piece in UNARY_BODY.as_bytes().chunks(UNARY_BODY.len().div_ceil(DRIP_PIECES)) {
        tokio::time::sleep(Duration::from_millis(200)).await;
        if sock.write_all(piece).await.is_err() || sock.flush().await.is_err() {
            return;
        }
    }
}

async fn start_unary_drip() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((sock, _)) = listener.accept().await {
            tokio::spawn(drip_unary(sock));
        }
    });
    format!("http://{addr}/v1")
}

/// The whole-request limit ended a call whose bytes kept arriving: not before the limit, and within
/// the ceiling. The reply drips for longer than the ceiling, so a call with no limit succeeds.
fn assert_whole_request_limit_ended_the_call(elapsed: Duration) {
    assert!(
        elapsed >= WHOLE_REQUEST_LIMIT,
        "the call ended before the limit, after {elapsed:?}"
    );
    assert!(
        elapsed < ceiling(WHOLE_REQUEST_LIMIT),
        "the call did not end within {CEILING_FACTOR} limits, ended at {elapsed:?}"
    );
}

#[tokio::test]
async fn chat_keeps_the_whole_request_limit() {
    // Bytes keep arriving, so only a whole-request limit ends this call at the 1 s timeout.
    let c = client(start_unary_drip().await, WHOLE_REQUEST_LIMIT);
    let started = Instant::now();
    let result = c.chat(request()).await;
    assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {:?}", result.err());
    assert_whole_request_limit_ended_the_call(started.elapsed());
}

#[tokio::test]
async fn create_response_keeps_the_whole_request_limit() {
    let c = client(start_unary_drip().await, WHOLE_REQUEST_LIMIT);
    let req = CreateResponseRequest {
        model: "m".into(),
        input: serde_json::json!("hi"),
        instructions: None,
        tools: None,
        temperature: None,
        max_output_tokens: None,
        metadata: None,
        extra_body: None,
        stream: None,
    };
    let started = Instant::now();
    let result = c.create_response(req).await;
    assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {:?}", result.err());
    assert_whole_request_limit_ended_the_call(started.elapsed());
}
