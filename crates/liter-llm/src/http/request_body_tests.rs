use super::tests::one_shot_server;
use super::*;
use crate::provider::{OutboundPolicy, set_outbound_policy};
use serial_test::serial;
use std::io::{Read, Write};
use std::net::TcpListener;
fn read_request_headers(stream: &mut std::net::TcpStream) {
    const MAX_REQUEST_HEADER_BYTES: usize = 4096;
    let mut headers = Vec::new();
    while !headers.ends_with(b"\r\n\r\n") {
        assert!(
            headers.len() < MAX_REQUEST_HEADER_BYTES,
            "bounded fixture request headers"
        );
        let mut byte = [0_u8; 1];
        stream.read_exact(&mut byte).expect("read fixture request header byte");
        headers.push(byte[0]);
    }
    assert!(headers.starts_with(b"GET "), "expected fixture GET request");
}

#[tokio::test]
#[serial(outbound_policy)]
async fn should_reject_binary_response_above_configured_limit() {
    const RESPONSE_LIMIT: usize = 4;
    set_outbound_policy(OutboundPolicy::Off);
    let (address, server) = one_shot_server("HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nabcde".to_string());
    let result = get_binary_bounded(
        &reqwest::Client::new(),
        &format!("http://{address}/"),
        None,
        &[],
        0,
        Some(RESPONSE_LIMIT),
    )
    .await;
    server.join().expect("join response server");
    assert!(
        result.is_err(),
        "response above {RESPONSE_LIMIT} bytes must be rejected; got {result:?}"
    );
}

const BODY_LIMIT: usize = 2;

#[derive(Clone, Copy, Debug)]
enum BodyOperation {
    PostJson,
    PostBinary,
    Multipart,
    GetJson,
    DeleteJson,
    GetBinary,
}

async fn request_body(operation: BodyOperation, url: &str, limit: Option<usize>) -> Result<Bytes> {
    let client = reqwest::Client::new();
    let body = Bytes::from_static(b"{}");
    match operation {
        BodyOperation::PostBinary => {
            post_binary_bounded(
                &client,
                url,
                None,
                &[],
                body,
                ResponseReadOptions {
                    max_retries: 0,
                    max_response_bytes: limit,
                },
            )
            .await
        }
        BodyOperation::GetBinary => get_binary_bounded(&client, url, None, &[], 0, limit).await,
        operation => {
            let value = match operation {
                BodyOperation::PostJson => {
                    post_json_raw_bounded(
                        &client,
                        url,
                        None,
                        &[],
                        body,
                        ResponseReadOptions {
                            max_retries: 0,
                            max_response_bytes: limit,
                        },
                    )
                    .await
                }
                BodyOperation::Multipart => {
                    post_multipart_bounded(&client, url, None, &[], reqwest::multipart::Form::new(), limit).await
                }
                BodyOperation::GetJson => get_json_raw_bounded(&client, url, None, &[], 0, limit).await,
                BodyOperation::DeleteJson => delete_json_bounded(&client, url, None, &[], 0, limit).await,
                _ => unreachable!(),
            }?;
            Ok(Bytes::from(serde_json::to_vec(&value).expect("serialize response")))
        }
    }
}

fn assert_body_limit(error: LiterLlmError, limit: usize) {
    assert!(
        matches!(error, LiterLlmError::Streaming { ref message }
            if message == &format!("HTTP response body exceeds configured limit of {limit} bytes")),
        "expected exact body-limit error, got {error:?}"
    );
}

#[tokio::test]
#[serial(outbound_policy)]
async fn should_enforce_exact_success_and_final_error_bounds_on_all_six_readers() {
    set_outbound_policy(OutboundPolicy::Off);
    let operations = [
        BodyOperation::PostJson,
        BodyOperation::PostBinary,
        BodyOperation::Multipart,
        BodyOperation::GetJson,
        BodyOperation::DeleteJson,
        BodyOperation::GetBinary,
    ];
    let mut assertions = 0;
    for operation in operations {
        for (status, payload) in [(200, "{}"), (200, "123"), (400, "{}"), (400, "err")] {
            let response = format!(
                "HTTP/1.1 {status} response\r\nContent-Length: {}\r\n\r\n{payload}",
                payload.len()
            );
            let (address, server) = one_shot_server(response);
            let result = request_body(operation, &format!("http://{address}/"), Some(BODY_LIMIT)).await;
            server.join().expect("join body server");
            if payload.len() > BODY_LIMIT {
                assert_body_limit(result.expect_err("oversized body"), BODY_LIMIT);
            } else if status == 200 {
                assert_eq!(result.expect("exact-bound response"), Bytes::from_static(b"{}"));
            } else {
                assert!(
                    matches!(result, Err(LiterLlmError::BadRequest { status: 400, .. })),
                    "{operation:?}: {result:?}"
                );
            }
            assertions += 1;
        }
    }
    assert_eq!(assertions, 24);
}

#[tokio::test]
async fn should_bound_chunked_and_close_delimited_bodies_without_content_length() {
    let responses = [
        (
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1\r\na\r\n1\r\nb\r\n0\r\n\r\n",
            true,
        ),
        (
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1\r\na\r\n2\r\nbc\r\n0\r\n\r\n",
            false,
        ),
        ("HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nab", true),
        ("HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nabc", false),
    ];
    for (response, accepted) in responses {
        let (address, server) = one_shot_server(response.into());
        let response = reqwest::Client::new()
            .get(format!("http://{address}/"))
            .send()
            .await
            .expect("HTTP response");
        assert_eq!(response.content_length(), None);
        let result = read_response_body(response, Some(BODY_LIMIT)).await;
        server.join().expect("join framing server");
        if accepted {
            assert_eq!(result.expect("exact bound"), Bytes::from_static(b"ab"));
        } else {
            assert_body_limit(result.expect_err("over bound"), BODY_LIMIT);
        }
    }
}

#[tokio::test]
async fn should_reject_oversized_headers_before_waiting_for_body() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("header listener");
    let address = listener.local_addr().expect("header address");
    let (release, held) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("header request");
        read_request_headers(&mut stream);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\n")
            .expect("write headers only");
        held.recv_timeout(std::time::Duration::from_secs(2))
            .expect("release held body");
    });
    let response = reqwest::Client::new().get(format!("http://{address}/")).send().await;
    let result = match response {
        Ok(response) => {
            tokio::time::timeout(
                std::time::Duration::from_secs(1),
                read_response_body(response, Some(BODY_LIMIT)),
            )
            .await
        }
        Err(error) => {
            release.send(()).expect("release error");
            server.join().expect("join error");
            panic!("headers: {error}");
        }
    };
    release.send(()).expect("release body");
    server.join().expect("join held body");
    assert_body_limit(
        result
            .expect("must reject with body withheld")
            .expect_err("oversized headers"),
        BODY_LIMIT,
    );
}

#[tokio::test]
#[serial(outbound_policy)]
async fn should_bound_final_stream_errors_without_limiting_successful_streams() {
    set_outbound_policy(OutboundPolicy::Off);
    let (address, server) = one_shot_server("HTTP/1.1 400 error\r\nContent-Length: 3\r\n\r\nerr".into());
    let url = format!("http://{address}/");
    let result = crate::http::streaming::post_stream_bounded::<_, serde_json::Value>(
        &reqwest::Client::new(),
        crate::http::request::StreamingPost {
            url: &url,
            auth_header: None,
            extra_headers: &[],
            body: Bytes::from_static(b"{}"),
        },
        |_| Ok(None),
        ResponseReadOptions {
            max_retries: 0,
            max_response_bytes: Some(BODY_LIMIT),
        },
    )
    .await;
    server.join().expect("join stream error");
    assert_body_limit(result.err().expect("oversized stream error"), BODY_LIMIT);
    let (address, server) = one_shot_server("HTTP/1.1 400 error\r\nContent-Length: 3\r\n\r\nerr".into());
    let url = format!("http://{address}/");
    let result = crate::http::eventstream::post_eventstream_bounded(
        &reqwest::Client::new(),
        crate::http::request::StreamingPost {
            url: &url,
            auth_header: None,
            extra_headers: &[],
            body: Bytes::from_static(b"{}"),
        },
        |_, _| Ok(None),
        ResponseReadOptions {
            max_retries: 0,
            max_response_bytes: Some(BODY_LIMIT),
        },
    )
    .await;
    server.join().expect("join eventstream error");
    assert_body_limit(result.err().expect("oversized eventstream error"), BODY_LIMIT);
}

#[tokio::test]
#[serial(outbound_policy)]
async fn should_propagate_native_client_limit_and_preserve_unconfigured_defaults() {
    use crate::client::{ClientConfig, ClientConfigBuilder, DefaultClient, FileClient};
    set_outbound_policy(OutboundPolicy::Off);
    assert_eq!(ClientConfig::new("synthetic").max_response_bytes, None);
    assert!(ClientConfigBuilder::new("synthetic").max_response_bytes(0).is_err());
    assert!(crate::ClientBuilder::new().max_response_bytes(0).is_err());
    for limit in [None, Some(BODY_LIMIT)] {
        let (address, server) = one_shot_server("HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc".into());
        let mut builder = ClientConfigBuilder::new("synthetic")
            .base_url(format!("http://{address}"))
            .load_env(false);
        if let Some(limit) = limit {
            builder = builder.max_response_bytes(limit).expect("nonzero limit");
        }
        let config = builder.build();
        assert_eq!(config.max_response_bytes, limit);
        let client = DefaultClient::new(config, None).expect("local client");
        let result = client.file_content("synthetic").await;
        server.join().expect("join client server");
        if limit.is_some() {
            assert_body_limit(result.expect_err("configured client"), BODY_LIMIT);
        } else {
            assert_eq!(result.expect("legacy unbounded client"), Bytes::from_static(b"abc"));
        }
    }
}

#[tokio::test]
#[serial(outbound_policy)]
async fn should_preserve_nonzero_limit_through_native_type_state_builder() {
    use crate::client::FileClient;
    set_outbound_policy(OutboundPolicy::Off);
    let (address, server) = one_shot_server("HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc".into());
    let client = crate::ClientBuilder::new()
        .max_response_bytes(BODY_LIMIT)
        .expect("nonzero limit")
        .api_key("synthetic")
        .provider("openai")
        .base_url(format!("http://{address}"))
        .build()
        .expect("local type-state client");
    let result = client.file_content("synthetic").await;
    server.join().expect("join type-state server");
    assert_body_limit(result.expect_err("limit survives both type transitions"), BODY_LIMIT);
    let mut invalid = crate::client::ClientConfig::new("synthetic");
    invalid.load_env = false;
    invalid.max_response_bytes = Some(0);
    assert!(matches!(
        crate::client::DefaultClient::new(invalid, None),
        Err(LiterLlmError::BadRequest { status: 400, .. })
    ));
}

#[tokio::test]
#[serial(outbound_policy)]
async fn should_discard_oversized_retry_body_before_reading_bounded_final_success() {
    set_outbound_policy(OutboundPolicy::Off);
    let listener = TcpListener::bind("127.0.0.1:0").expect("retry listener");
    let address = listener.local_addr().expect("retry address");
    let server = std::thread::spawn(move || {
        for response in [
            "HTTP/1.1 503 unavailable\r\nRetry-After: 0\r\nContent-Length: 3\r\nConnection: close\r\n\r\nerr",
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}",
        ] {
            let (mut stream, _) = listener.accept().expect("retry request");
            read_request_headers(&mut stream);
            stream.write_all(response.as_bytes()).expect("retry response");
        }
    });
    let result = get_json_raw_bounded(
        &reqwest::Client::new(),
        &format!("http://{address}/"),
        None,
        &[],
        1,
        Some(BODY_LIMIT),
    )
    .await;
    server.join().expect("join retry server");
    assert_eq!(result.expect("discard retry body"), serde_json::json!({}));
}

#[tokio::test]
#[serial(outbound_policy)]
async fn should_keep_successful_stream_traffic_above_retained_body_limit() {
    use futures_util::StreamExt;
    set_outbound_policy(OutboundPolicy::Off);
    const SSE_BODY: &str = "data: 123\n\ndata: [DONE]\n\n";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{SSE_BODY}",
        SSE_BODY.len()
    );
    let (address, server) = one_shot_server(response);
    let url = format!("http://{address}/");
    let result = crate::http::streaming::post_stream_bounded::<_, serde_json::Value>(
        &reqwest::Client::new(),
        crate::http::request::StreamingPost {
            url: &url,
            auth_header: None,
            extra_headers: &[],
            body: Bytes::from_static(b"{}"),
        },
        |event| Ok(Some(serde_json::from_str(event)?)),
        ResponseReadOptions {
            max_retries: 0,
            max_response_bytes: Some(BODY_LIMIT),
        },
    )
    .await;
    server.join().expect("join healthy stream");
    let mut stream = result.expect("successful stream ignores aggregate body limit");
    assert_eq!(
        stream.next().await.expect("one event").expect("valid event"),
        serde_json::json!(123)
    );
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn should_bound_decoded_gzip_expansion_and_preserve_unconfigured_payload() {
    const DECODED_LENGTH: usize = 128;
    const DECODED_LIMIT: usize = 32;
    const GZIP_BODY: &[u8] = &[
        31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 75, 76, 28, 88, 0, 0, 140, 54, 43, 241, 128, 0, 0, 0,
    ];
    assert!(GZIP_BODY.len() < DECODED_LIMIT);
    for limit in [None, Some(DECODED_LIMIT)] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("gzip listener");
        let address = listener.local_addr().expect("gzip address");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("gzip request");
            read_request_headers(&mut stream);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\n\r\n",
                GZIP_BODY.len()
            )
            .expect("gzip headers");
            stream.write_all(GZIP_BODY).expect("gzip body");
        });
        let response = reqwest::Client::builder()
            .gzip(true)
            .build()
            .expect("decompression client")
            .get(format!("http://{address}/"))
            .send()
            .await
            .expect("gzip response");
        assert_eq!(response.content_length(), None);
        let result = read_response_body(response, limit).await;
        server.join().expect("join gzip server");
        if limit.is_some() {
            assert_body_limit(result.expect_err("decoded expansion"), DECODED_LIMIT);
        } else {
            assert_eq!(
                result.expect("known decoded payload"),
                Bytes::from(vec![b'a'; DECODED_LENGTH])
            );
        }
    }
}
