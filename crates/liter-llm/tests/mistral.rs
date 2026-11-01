#![cfg(feature = "native-http")]

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use futures_util::StreamExt;
use liter_llm::client::{ClientConfigBuilder, DefaultClient, LlmClient};
use liter_llm::types::ChatCompletionRequest;
use serde_json::{Value, json};

struct Seen {
    request_line: String,
    body: Value,
}

#[derive(Clone, Copy)]
enum Mode {
    Chat,
    Stream,
}

fn client(mode: Mode) -> (DefaultClient, thread::JoinHandle<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind response server");
    let base_url = format!("http://{}/v1", listener.local_addr().expect("server address"));
    let server = thread::spawn(move || serve_request(listener, mode));
    let config = ClientConfigBuilder::new("test-key")
        .base_url(base_url)
        .load_env(false)
        .max_retries(0)
        .build();
    let client = DefaultClient::new(config, Some("mistral/mistral-small-latest")).expect("create client");
    (client, server)
}

fn serve_request(listener: TcpListener, mode: Mode) -> Seen {
    let (mut stream, _) = listener.accept().expect("accept request");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("set read timeout");
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut request_line = String::new();
    reader.read_line(&mut request_line).expect("read request line");
    let mut content_length = None;
    loop {
        let mut line = String::new();
        assert!(reader.read_line(&mut line).expect("read request headers") > 0);
        if line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            content_length = Some(value.trim().parse().expect("request content length"));
        }
    }
    let mut raw = vec![0; content_length.expect("content length header")];
    reader.read_exact(&mut raw).expect("read request body");
    let choice = if matches!(mode, Mode::Stream) {
        json!({"index": 0, "delta": {"role": "assistant", "content": "Hello"}, "finish_reason": "stop"})
    } else {
        json!({"index": 0, "message": {"role": "assistant", "content": "Hello"}, "finish_reason": "stop"})
    };
    let reply = json!({
        "id": "fixture", "object": if matches!(mode, Mode::Stream) { "chat.completion.chunk" } else { "chat.completion" },
        "created": 1, "model": "mistral-small-latest", "choices": [choice]
    });
    let (content_type, body) = if matches!(mode, Mode::Stream) {
        ("text/event-stream", format!("data: {reply}\n\ndata: [DONE]\n\n"))
    } else {
        ("application/json", reply.to_string())
    };
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("write response");
    Seen {
        request_line: request_line.trim().to_owned(),
        body: serde_json::from_slice(&raw).expect("request body is JSON"),
    }
}

fn settings() -> [Value; 3] {
    [
        json!({"parallel_tool_calls": false, "presence_penalty": 0.4, "frequency_penalty": -0.3}),
        json!({"parallel_tool_calls": true, "presence_penalty": 0.0, "frequency_penalty": 0.0}),
        json!({}),
    ]
}

fn request(settings: &Value) -> ChatCompletionRequest {
    let mut body = json!({
        "model": "mistral/mistral-small-latest",
        "messages": [{"role": "user", "content": "Hello"}],
        "temperature": 0.2,
        "tool_choice": "required",
        "tools": [{"type": "function", "function": {
            "name": "weather", "parameters": {"type": "object", "properties": {}}
        }}],
        "logit_bias": {"1": 1.0}
    });
    body.as_object_mut()
        .expect("fixture request")
        .extend(settings.as_object().expect("fixture settings").clone());
    serde_json::from_value(body).expect("chat request")
}

fn assert_request(seen: Seen, settings: &Value, mode: Mode) {
    assert_eq!(seen.request_line, "POST /v1/chat/completions HTTP/1.1");
    assert_eq!(seen.body["model"], "mistral-small-latest");
    assert_eq!(seen.body["stream"], matches!(mode, Mode::Stream));
    assert_eq!(seen.body["temperature"], 0.2);
    assert_eq!(seen.body["tool_choice"], "any");
    assert_eq!(seen.body["tools"][0]["function"]["name"], "weather");
    assert!(seen.body.get("logit_bias").is_none());
    for field in ["parallel_tool_calls", "presence_penalty", "frequency_penalty"] {
        assert_eq!(seen.body.get(field), settings.get(field), "{field}");
    }
}

#[tokio::test]
async fn public_chat_preserves_supported_mistral_settings() {
    for settings in settings() {
        let (client, server) = client(Mode::Chat);
        let response = client.chat(request(&settings)).await.expect("chat succeeds");
        assert_eq!(response.choices[0].message.text().as_deref(), Some("Hello"));
        assert_request(server.join().expect("response server completed"), &settings, Mode::Chat);
    }
}

#[tokio::test]
async fn public_chat_stream_preserves_supported_mistral_settings() {
    for settings in settings() {
        let (client, server) = client(Mode::Stream);
        let chunks = client
            .chat_stream(request(&settings))
            .await
            .expect("stream opens")
            .collect::<Vec<_>>()
            .await;
        assert_eq!(chunks.len(), 1);
        let chunk = chunks[0].as_ref().expect("valid chunk");
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hello"));
        assert_request(
            server.join().expect("response server completed"),
            &settings,
            Mode::Stream,
        );
    }
}
