//! A request that runs out of time is not sent again. The server already has
//! the request and may still be working on it, so a second try does not help
//! and the server would run (and bill) the request twice. A connection that
//! closes without a reply and 5xx replies keep their retries.

#![cfg(feature = "native-http")]

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use liter_llm::LiterLlmError;
use liter_llm::client::{ClientConfigBuilder, DefaultClient, LlmClient};
use liter_llm::types::{ChatCompletionRequest, Message, UserMessage};

const REPLY: &str = r#"{"id":"c","object":"chat.completion","created":1,"model":"m","choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"late"}}]}"#;

/// How the server treats one connection, chosen by the attempt number.
type Handler = dyn Fn(usize, TcpStream) + Send + Sync;

/// A server that counts the requests it receives and answers each one with `handler`.
struct Server {
    base_url: String,
    requests: Arc<AtomicUsize>,
}

fn read_request(stream: &TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut content_length = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).expect("read request headers") == 0 || line.trim().is_empty() {
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
}

fn respond(mut stream: TcpStream, status: &str, body: &str) {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("write response");
}

fn serve(handler: impl Fn(usize, TcpStream) + Send + Sync + 'static) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind server");
    let base_url = format!("http://{}", listener.local_addr().expect("server address"));
    let requests = Arc::new(AtomicUsize::new(0));
    let handler: Arc<Handler> = Arc::new(handler);
    let counter = Arc::clone(&requests);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { return };
            let handler = Arc::clone(&handler);
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                read_request(&stream);
                let attempt = counter.fetch_add(1, Ordering::SeqCst);
                handler(attempt, stream);
            });
        }
    });
    Server { base_url, requests }
}

/// A server that holds every request for `hold` and never answers it.
fn silent_server(hold: Duration) -> Server {
    serve(move |_, stream| {
        thread::sleep(hold);
        drop(stream);
    })
}

fn client(server: &Server, timeout: Duration, max_retries: u32) -> DefaultClient {
    let config = ClientConfigBuilder::new("test-key")
        .base_url(server.base_url.clone())
        .timeout(timeout)
        .max_retries(max_retries)
        .build();
    DefaultClient::new(config, Some("openai/m")).expect("create client")
}

fn request() -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: "m".to_owned(),
        messages: vec![Message::User(UserMessage {
            content: "hi".into(),
            ..Default::default()
        })],
        ..Default::default()
    }
}

#[tokio::test]
async fn a_chat_call_that_times_out_is_sent_once() {
    let server = silent_server(Duration::from_secs(8));
    let client = client(&server, Duration::from_secs(1), 3);

    let start = Instant::now();
    let result = client.chat(request()).await;
    let elapsed = start.elapsed();

    assert!(
        matches!(result, Err(LiterLlmError::Timeout)),
        "a reply that never comes ends in a Timeout error, got {result:?}"
    );
    assert!(
        elapsed < Duration::from_millis(2500),
        "the caller gets the Timeout after about one timeout, not one per retry: {elapsed:?}"
    );
    assert_eq!(
        server.requests.load(Ordering::SeqCst),
        1,
        "the server must receive the request once"
    );
}

#[tokio::test]
async fn a_stream_that_times_out_before_its_reply_is_sent_once() {
    let server = silent_server(Duration::from_secs(8));
    let client = client(&server, Duration::from_secs(1), 3);

    let start = Instant::now();
    let result = client.chat_stream(request()).await;
    let elapsed = start.elapsed();

    assert!(
        matches!(result, Err(LiterLlmError::Timeout)),
        "a stream that gets no reply ends in a Timeout error, got {:?}",
        result.err()
    );
    assert!(
        elapsed < Duration::from_millis(2500),
        "the caller gets the Timeout after about one timeout: {elapsed:?}"
    );
    assert_eq!(
        server.requests.load(Ordering::SeqCst),
        1,
        "the server must receive the request once"
    );
}

#[tokio::test]
async fn a_timeout_after_a_server_error_ends_the_call() {
    // The first attempt gets a 503 (retried, as before). The second attempt
    // times out and must end the call: no third request.
    let server = serve(|attempt, stream| {
        if attempt == 0 {
            respond(stream, "503 Service Unavailable", r#"{"error":{"message":"busy"}}"#);
        } else {
            thread::sleep(Duration::from_secs(8));
            drop(stream);
        }
    });
    let client = client(&server, Duration::from_secs(1), 3);

    let result = client.chat(request()).await;

    assert!(
        matches!(result, Err(LiterLlmError::Timeout)),
        "the timed-out retry ends the call with a Timeout error, got {result:?}"
    );
    assert_eq!(
        server.requests.load(Ordering::SeqCst),
        2,
        "the 503 is retried once and the timeout is not"
    );
}

#[tokio::test]
async fn a_server_error_is_still_sent_again() {
    let server = serve(|attempt, stream| {
        if attempt == 0 {
            respond(stream, "503 Service Unavailable", r#"{"error":{"message":"busy"}}"#);
        } else {
            respond(stream, "200 OK", REPLY);
        }
    });
    let client = client(&server, Duration::from_secs(5), 1);

    let result = client.chat(request()).await;

    assert!(result.is_ok(), "the retry after a 503 succeeds, got {result:?}");
    assert_eq!(server.requests.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_connection_closed_without_a_reply_is_still_sent_again() {
    let server = serve(|attempt, stream| {
        if attempt == 0 {
            drop(stream);
        } else {
            respond(stream, "200 OK", REPLY);
        }
    });
    let client = client(&server, Duration::from_secs(5), 1);

    let result = client.chat(request()).await;

    assert!(
        result.is_ok(),
        "the retry after a closed connection succeeds, got {result:?}"
    );
    assert_eq!(server.requests.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_rate_limit_reply_is_still_sent_again() {
    let server = serve(|attempt, stream| {
        if attempt == 0 {
            respond(stream, "429 Too Many Requests", r#"{"error":{"message":"slow down"}}"#);
        } else {
            respond(stream, "200 OK", REPLY);
        }
    });
    let client = client(&server, Duration::from_secs(5), 1);

    let result = client.chat(request()).await;

    assert!(result.is_ok(), "the retry after a 429 succeeds, got {result:?}");
    assert_eq!(server.requests.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_refused_connection_is_still_tried_again() {
    // Nothing listens at first, so the first attempt fails before any request
    // is sent. A server starts on the same port while the client backs off
    // (the first back-off is at least 500 ms).
    let probe = TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let address = probe.local_addr().expect("probe address");
    drop(probe);
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&requests);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        let listener = TcpListener::bind(address).expect("bind server on the probe port");
        let (stream, _) = listener.accept().expect("accept the retried request");
        read_request(&stream);
        counter.fetch_add(1, Ordering::SeqCst);
        respond(stream, "200 OK", REPLY);
    });
    let server = Server {
        base_url: format!("http://{address}"),
        requests,
    };
    let client = client(&server, Duration::from_secs(5), 2);

    let result = client.chat(request()).await;

    assert!(
        result.is_ok(),
        "the retry after a refused connection succeeds, got {result:?}"
    );
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_connection_dropped_on_every_attempt_uses_up_the_retries() {
    // The server closes every connection with no reply. With max_retries 2 the
    // client sends the first request and two retries, then returns the
    // transport error. The outer timeout makes a loop that never stops fail
    // instead of hang.
    let server = serve(|_, stream| drop(stream));
    let client = client(&server, Duration::from_secs(5), 2);

    let result = tokio::time::timeout(Duration::from_secs(20), client.chat(request()))
        .await
        .expect("the call ends once the retries are used up");

    assert!(
        matches!(result, Err(LiterLlmError::Network { .. })),
        "a connection that always closes ends in a Network error, got {result:?}"
    );
    assert_eq!(
        server.requests.load(Ordering::SeqCst),
        3,
        "the server receives the first request and two retries"
    );
}
