//! A chat stream cut off before the provider's final event must not look like
//! a complete response (#295). Protocols that always end with a finish reason
//! surface truncation as an error; OpenAI-compatible streams stay lenient
//! because some servers omit `[DONE]` (#155).
//!
//! Anthropic and Gemini are exercised end to end here. Cohere and Bedrock are
//! covered by the unit tests next to the adapter, because a `base_url`
//! override routes them through the OpenAI-compatible provider.

#![cfg(feature = "native-http")]

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;

use futures_util::StreamExt;
use liter_llm::LiterLlmError;
use liter_llm::client::{ClientConfigBuilder, DefaultClient, LlmClient};
use liter_llm::types::{ChatCompletionChunk, ChatCompletionRequest, FinishReason, Message, UserMessage};

/// Serve one SSE response with `body`, then close the connection.
fn client(model: &str, body: String) -> (DefaultClient, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind response server");
    let base_url = format!("http://{}", listener.local_addr().expect("server address"));
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("read request headers");
            if line.trim().is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                content_length = value.trim().parse().expect("request content length");
            }
        }
        reader
            .read_exact(&mut vec![0; content_length])
            .expect("read request body");
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("write response");
    });
    let config = ClientConfigBuilder::new("test-key").base_url(base_url).build();
    let client = DefaultClient::new(config, Some(model)).expect("create client");
    (client, handle)
}

fn request(model: &str) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: model.to_owned(),
        messages: vec![Message::User(UserMessage {
            content: "Hello".into(),
            ..Default::default()
        })],
        ..Default::default()
    }
}

/// Drain the stream, returning every item it yielded.
async fn collect(model: &str, body: String) -> Vec<Result<ChatCompletionChunk, LiterLlmError>> {
    let (client, server) = client(model, body);
    let stream = client.chat_stream(request(model)).await.expect("stream opens");
    let items = stream.collect::<Vec<_>>().await;
    server.join().expect("response server completed");
    items
}

fn sse(events: &[&str]) -> String {
    events.iter().map(|event| format!("data: {event}\n\n")).collect()
}

fn assert_truncated(items: &[Result<ChatCompletionChunk, LiterLlmError>]) {
    let (last, content) = items.split_last().expect("stream yields items");
    assert!(
        content.iter().all(Result::is_ok),
        "content before the cut must still arrive: {items:?}"
    );
    assert!(
        matches!(last, Err(LiterLlmError::Streaming { message }) if message.contains("final event")),
        "a stream cut before its final event must end in a Streaming error, got {last:?}"
    );
}

fn assert_complete(items: &[Result<ChatCompletionChunk, LiterLlmError>]) {
    assert!(!items.is_empty(), "stream yields items");
    assert!(
        items.iter().all(Result::is_ok),
        "complete stream must not error: {items:?}"
    );
}

const ANTHROPIC_MODEL: &str = "anthropic/claude-sonnet-4-5";
const ANTHROPIC_START: &str = r#"{"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[],"usage":{"input_tokens":5,"output_tokens":1}}}"#;
const ANTHROPIC_DELTA: &str =
    r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
const ANTHROPIC_END: &str =
    r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":2}}"#;

#[tokio::test]
async fn anthropic_stream_cut_before_message_delta_is_an_error() {
    assert_truncated(&collect(ANTHROPIC_MODEL, sse(&[ANTHROPIC_START, ANTHROPIC_DELTA])).await);
}

#[tokio::test]
async fn anthropic_stream_with_final_event_completes() {
    let body = sse(&[
        ANTHROPIC_START,
        ANTHROPIC_DELTA,
        ANTHROPIC_END,
        r#"{"type":"message_stop"}"#,
    ]);
    assert_complete(&collect(ANTHROPIC_MODEL, body).await);
}

const GEMINI_MODEL: &str = "gemini/gemini-2.5-flash";

#[tokio::test]
async fn gemini_stream_cut_before_finish_reason_is_an_error() {
    let body = sse(&[r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"Hello"}]}}]}"#]);
    assert_truncated(&collect(GEMINI_MODEL, body).await);
}

#[tokio::test]
async fn gemini_intermediate_chunks_carry_no_finish_reason() {
    let body = sse(&[
        r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"Hello"}]}}]}"#,
        r#"{"candidates":[{"content":{"role":"model","parts":[{"text":" world"}]},"finishReason":"MAX_TOKENS"}]}"#,
    ]);
    let items = collect(GEMINI_MODEL, body).await;
    let reasons: Vec<_> = items
        .iter()
        .map(|item| item.as_ref().expect("complete stream").choices[0].finish_reason.clone())
        .collect();
    assert_eq!(reasons, [None, Some(FinishReason::Length)]);
}

#[tokio::test]
async fn gemini_stream_with_finish_reason_completes() {
    let body = sse(&[
        r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"Hello"}]}}]}"#,
        r#"{"candidates":[{"content":{"role":"model","parts":[{"text":" world"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":3,"candidatesTokenCount":2}}"#,
    ]);
    assert_complete(&collect(GEMINI_MODEL, body).await);
}

#[tokio::test]
async fn openai_compatible_stream_without_done_stays_lenient() {
    let body = sse(&[
        r#"{"id":"c1","object":"chat.completion.chunk","created":0,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Hi"},"finish_reason":null}]}"#,
    ]);
    assert_complete(&collect("openai/gpt-4o", body).await);
}
