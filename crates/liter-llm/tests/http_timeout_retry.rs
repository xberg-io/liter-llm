//! A request that runs out of time is not sent again. The server already has
//! the request and may still be working on it, so a second try does not help
//! and the server would run (and bill) the request twice. A connection that
//! closes without a reply and 5xx replies keep their retries.
//!
//! No test bounds how long a call takes from above by a margin that a loaded host can use up.
//! Each time bound separates the behaviour under test from the alternative by many seconds,
//! and the retry count is asserted from the requests the server receives.

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

/// How long a silent server holds a connection. A call that ends because the server closed the
/// connection (a Network error, after `HOLD`) cannot pass for a call that timed out.
const HOLD: Duration = Duration::from_secs(60);

/// A bound that no call under test reaches, however busy the host is: it is far above every
/// timeout the tests set and far below `HOLD`.
const NEVER_REACHED: Duration = Duration::from_secs(30);

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
    let server = silent_server(HOLD);
    let client = client(&server, Duration::from_secs(1), 3);

    let start = Instant::now();
    let result = client.chat(request()).await;
    let elapsed = start.elapsed();

    assert!(
        matches!(result, Err(LiterLlmError::Timeout)),
        "a reply that never comes ends in a Timeout error, got {result:?}"
    );
    assert!(
        elapsed < NEVER_REACHED,
        "the request timeout ends the call, not the end of the held connection: {elapsed:?}"
    );
    assert_eq!(
        server.requests.load(Ordering::SeqCst),
        1,
        "the server must receive the request once"
    );
}

#[tokio::test]
async fn a_stream_that_times_out_before_its_reply_is_sent_once() {
    let server = silent_server(HOLD);
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
        elapsed < NEVER_REACHED,
        "the request timeout ends the call, not the end of the held connection: {elapsed:?}"
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
            thread::sleep(HOLD);
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

// macOS drops the SYN to a bound, non-listening socket, so this test runs on Linux only.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn a_refused_connection_is_still_tried_again() {
    // Nothing listens at first, so the first attempt fails before any request
    // is sent. A server starts on the same port while the client backs off
    // (the first back-off is at least 500 ms).
    //
    // The socket is bound at once and listens only later. A bound socket that does not listen
    // refuses connections, and it keeps the port, so no other process can take it in between.
    let socket = tokio::net::TcpSocket::new_v4().expect("create socket");
    socket.set_reuseaddr(false).expect("disable address reuse");
    socket
        .bind("127.0.0.1:0".parse().expect("loopback address"))
        .expect("bind the refusing port");
    let address = socket.local_addr().expect("refusing port address");
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&requests);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let listener = socket.listen(16).expect("listen on the refusing port");
        let (stream, _) = listener.accept().await.expect("accept the retried request");
        let stream = stream.into_std().expect("convert to a blocking stream");
        stream.set_nonblocking(false).expect("blocking stream");
        tokio::task::spawn_blocking(move || {
            read_request(&stream);
            counter.fetch_add(1, Ordering::SeqCst);
            respond(stream, "200 OK", REPLY);
        })
        .await
        .expect("answer the retried request");
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

/// An address that drops every packet, so a connection to it never opens.
///
/// The tests that use it need a route to it. On a host with no route, the connect fails at once
/// with an error that is not a `Timeout`, and these tests fail. They do not pass by mistake.
const UNROUTABLE: &str = "http://10.255.255.1";

fn unroutable_client(timeout: Duration, connect_timeout: Duration, max_retries: u32) -> DefaultClient {
    let config = ClientConfigBuilder::new("test-key")
        .base_url(UNROUTABLE)
        .timeout(timeout)
        .connect_timeout(connect_timeout)
        .max_retries(max_retries)
        .build();
    DefaultClient::new(config, Some("openai/m")).expect("create client")
}

#[tokio::test]
async fn a_connect_timeout_is_tried_again() {
    // Each of the three attempts waits 500 ms for a connection, and the two back-offs add at
    // least 0.5 s and 1 s. A connect timeout that is sent once ends in about 500 ms. The
    // request timeout is long, so only the connect timeout can end an attempt.
    let client = unroutable_client(Duration::from_secs(30), Duration::from_millis(500), 2);

    let start = Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(120), client.chat(request()))
        .await
        .expect("the call ends once the retries are used up");
    let elapsed = start.elapsed();

    assert!(
        matches!(result, Err(LiterLlmError::Timeout)),
        "a connection that never opens ends in a Timeout error, got {result:?}"
    );
    // Only a lower bound: a busy host makes the call longer, never shorter.
    assert!(
        elapsed >= Duration::from_millis(2500),
        "the connect timeout is retried twice: {elapsed:?}"
    );
}

#[tokio::test]
async fn a_stream_connect_timeout_is_tried_again() {
    // The stream client is a second reqwest client. If it loses the connect timeout, the call
    // waits for its read timeout, which is the request timeout, or for the operating system to
    // give up on the connection, and the outer timeout fires. The request timeout is far above
    // the outer timeout so that no other timer can end the call first and pass for the connect
    // timeout. If the stream path does not retry, the call ends after one 500 ms attempt.
    let client = unroutable_client(Duration::from_secs(600), Duration::from_millis(500), 2);

    let start = Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(60), client.chat_stream(request()))
        .await
        .expect("the stream call ends once the retries are used up");
    let elapsed = start.elapsed();

    assert!(
        matches!(result, Err(LiterLlmError::Timeout)),
        "a stream connection that never opens ends in a Timeout error, got {:?}",
        result.err()
    );
    assert!(
        elapsed >= Duration::from_millis(2500),
        "the stream connect timeout is retried twice: {elapsed:?}"
    );
}

#[tokio::test]
async fn a_connect_timeout_with_no_retries_is_sent_once() {
    let client = unroutable_client(Duration::from_secs(30), Duration::from_millis(500), 0);

    let start = Instant::now();
    let result = client.chat(request()).await;
    let elapsed = start.elapsed();

    assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
    assert!(
        elapsed < NEVER_REACHED,
        "the connect timeout ends the call, not the 30 s request timeout: {elapsed:?}"
    );
}

#[tokio::test]
async fn a_timeout_after_the_connection_opened_is_sent_once_with_a_connect_timeout_set() {
    // The server accepts the connection at once, so the connect timeout never fires. The reply
    // never comes, so the request timeout does, and the request is not sent again.
    let server = silent_server(HOLD);
    let config = ClientConfigBuilder::new("test-key")
        .base_url(server.base_url.clone())
        .timeout(Duration::from_secs(1))
        .connect_timeout(Duration::from_secs(5))
        .max_retries(2)
        .build();
    let client = DefaultClient::new(config, Some("openai/m")).expect("create client");

    let start = Instant::now();
    let result = client.chat(request()).await;
    let elapsed = start.elapsed();

    assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
    assert!(
        elapsed < NEVER_REACHED,
        "the request timeout ends the call, not the end of the held connection: {elapsed:?}"
    );
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn the_connect_timeout_option_reaches_the_client() {
    // A client built from ClientOptions with a 1 s connect timeout gives up on a connection that
    // never opens after 1 s. With the 10 s default it would take 10 s, so the bound sits between.
    let client = liter_llm::create_client_with_options(liter_llm::ClientOptions {
        api_key: "test-key".into(),
        base_url: Some(UNROUTABLE.into()),
        timeout_secs: Some(30),
        connect_timeout_secs: Some(1),
        max_retries: Some(0),
        model_hint: Some("openai/m".into()),
        ..Default::default()
    })
    .expect("create client");

    let start = Instant::now();
    let result = client.chat(request()).await;
    let elapsed = start.elapsed();

    assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
    assert!(
        elapsed < Duration::from_secs(9),
        "the option sets the connect timeout, the default is 10 s: {elapsed:?}"
    );
}

fn assert_zero_connect_timeout_error<T>(result: liter_llm::Result<T>, entry_point: &str) {
    match result.err() {
        Some(LiterLlmError::BadRequest { message, status: 400 }) => {
            assert!(
                message.contains("connect_timeout"),
                "{entry_point}: message = {message}"
            );
        }
        other => panic!("{entry_point}: a zero connect timeout must be a BadRequest error, got {other:?}"),
    }
}

#[test]
fn a_zero_connect_timeout_is_an_error_in_the_config_builder() {
    let config = ClientConfigBuilder::new("test-key")
        .connect_timeout(Duration::ZERO)
        .build();
    assert_zero_connect_timeout_error(DefaultClient::new(config, Some("openai/m")), "ClientConfigBuilder");
}

#[test]
fn a_zero_connect_timeout_is_an_error_in_the_typed_builder() {
    let result = liter_llm::client::ClientBuilder::new()
        .api_key("test-key")
        .provider("openai")
        .connect_timeout(Duration::ZERO)
        .build();
    assert_zero_connect_timeout_error(result, "ClientBuilder");
}

#[test]
fn a_zero_connect_timeout_is_an_error_in_the_client_options() {
    let result = liter_llm::create_client_with_options(liter_llm::ClientOptions {
        api_key: "test-key".into(),
        connect_timeout_secs: Some(0),
        ..Default::default()
    });
    assert_zero_connect_timeout_error(result, "ClientOptions");
}

#[test]
fn a_zero_connect_timeout_is_an_error_in_the_json_config() {
    let result = liter_llm::create_client_from_json(r#"{"api_key":"test-key","connect_timeout_secs":0}"#);
    assert_zero_connect_timeout_error(result, "JSON config");
}

#[test]
fn a_zero_connect_timeout_is_an_error_in_the_toml_config() {
    let config = liter_llm::FileConfig::from_toml_str("api_key = \"test-key\"\nconnect_timeout_secs = 0\n")
        .expect("the config parses")
        .into_builder()
        .build();
    assert_zero_connect_timeout_error(DefaultClient::new(config, None), "TOML config");
}

#[test]
fn a_zero_connect_timeout_is_an_error_in_the_llm_config() {
    let config = liter_llm::LlmConfig {
        model: "openai/m".to_owned(),
        api_key: Some("test-key".to_owned()),
        connect_timeout_secs: Some(0),
        ..Default::default()
    }
    .into_client_builder()
    .build();
    assert_zero_connect_timeout_error(DefaultClient::new(config, Some("openai/m")), "LlmConfig");
}

#[test]
fn a_one_second_connect_timeout_is_accepted() {
    let result = liter_llm::create_client_with_options(liter_llm::ClientOptions {
        api_key: "test-key".into(),
        connect_timeout_secs: Some(1),
        ..Default::default()
    });
    assert!(result.is_ok(), "got {:?}", result.err());
}
