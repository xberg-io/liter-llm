use std::future::Future;

use bytes::Bytes;

use crate::error::{LiterLlmError, Result};
use crate::http::retry;

/// Extract an optional `Retry-After` delay from a response.
pub(crate) fn retry_after_from_response(resp: &reqwest::Response) -> Option<std::time::Duration> {
    let value = resp.headers().get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    retry::parse_retry_after(value)
}

/// Sleep for a retry back-off delay, on native or WASM targets.
async fn sleep_for_retry(delay: std::time::Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(delay).await;
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(std::time::Duration::from_millis(delay.as_millis() as u64)).await;
}

#[allow(dead_code, reason = "retain the unconfigured raw request entry point")]
pub(crate) async fn with_retry<F, Fut>(url: &str, max_retries: u32, send: F) -> Result<reqwest::Response>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<reqwest::Response, reqwest::Error>>,
{
    with_retry_bounded(url, max_retries, None, send).await
}

#[derive(Clone, Copy)]
pub(crate) struct ResponseReadOptions {
    pub max_retries: u32,
    pub max_response_bytes: Option<usize>,
}

/// The request half of a streaming JSON `POST`: where it goes, which headers
/// it carries, and its body.
///
/// `auth_header` is `Some((name, value))` when the provider requires
/// authentication, or `None` when no auth header should be added.
/// `extra_headers` carries provider-specific mandatory headers (e.g.
/// `anthropic-version`) beyond the single auth header.
pub(crate) struct StreamingPost<'a> {
    pub url: &'a str,
    pub auth_header: Option<(&'a str, &'a str)>,
    pub extra_headers: &'a [(&'a str, &'a str)],
    pub body: Bytes,
}

impl StreamingPost<'_> {
    /// Send this request as a JSON `POST` under the retry policy in `options`.
    ///
    /// Once a response is obtained, its status and the retry count are recorded
    /// on the calling span (`post_stream*` / `post_eventstream*`), which declares
    /// those fields; a terminal error returns before anything is recorded.
    pub(crate) async fn send(
        &self,
        client: &reqwest::Client,
        options: ResponseReadOptions,
    ) -> Result<reqwest::Response> {
        let ResponseReadOptions {
            max_retries,
            max_response_bytes,
        } = options;
        let mut retry_count = 0u32;

        let resp = with_retry_bounded(self.url, max_retries, max_response_bytes, || {
            let mut builder = client
                .post(self.url)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(self.body.clone());
            if let Some((name, value)) = self.auth_header {
                builder = builder.header(name, value);
            }
            for (name, value) in self.extra_headers {
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
}

#[cfg(not(target_arch = "wasm32"))]
fn response_limit_error(limit: usize) -> LiterLlmError {
    LiterLlmError::Streaming {
        message: format!("HTTP response body exceeds configured limit of {limit} bytes"),
    }
}

pub(crate) async fn read_response_body(response: reqwest::Response, limit: Option<usize>) -> Result<Bytes> {
    let Some(limit) = limit else {
        return response.bytes().await.map_err(LiterLlmError::from);
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        read_limited_response_body(response, limit).await
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (response, limit);
        Err(LiterLlmError::InternalError {
            message: "HTTP response byte limits require native HTTP".into(),
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn read_limited_response_body(mut response: reqwest::Response, limit: usize) -> Result<Bytes> {
    if response.content_length().is_some_and(|length| length > limit as u64) {
        return Err(response_limit_error(limit));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        let length = body
            .len()
            .checked_add(chunk.len())
            .ok_or_else(|| response_limit_error(limit))?;
        if length > limit {
            return Err(response_limit_error(limit));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(body))
}

async fn read_response_json(response: reqwest::Response, limit: Option<usize>) -> Result<serde_json::Value> {
    if limit.is_none() {
        return response.json().await.map_err(LiterLlmError::from);
    }
    let body = read_response_body(response, limit).await?;
    serde_json::from_slice(&body).map_err(LiterLlmError::from)
}

async fn read_error_text(response: reqwest::Response, limit: Option<usize>) -> Result<String> {
    if limit.is_none() {
        return Ok(response
            .text()
            .await
            .unwrap_or_else(|error| format!("(failed to read body: {error})")));
    }
    let body = read_response_body(response, limit).await?;
    Ok(String::from_utf8_lossy(&body).into_owned())
}

/// Drive a single-request closure through the retry / back-off loop.
///
/// `send` is called once per attempt and must return a future that resolves to
/// a raw `reqwest::Response` (or a transport-level error).  The helper handles:
///
/// - Attempt counting and the `max_retries` budget.
/// - Validating the effective request URL against the active outbound policy.
/// - Parsing the `Retry-After` header before consuming the response body.
/// - Exponential back-off via `retry::should_retry`, including for
///   transport-level failures (connection reset, DNS failure, timeout) that
///   never produce a response at all.
/// - Reading the error body and mapping it to `LiterLlmError` on final failure.
///
/// On success the **successful** `Response` is returned so the caller can
/// choose how to consume the body (JSON deserialisation, byte stream, …).
pub(crate) async fn with_retry_bounded<F, Fut>(
    url: &str,
    max_retries: u32,
    max_response_bytes: Option<usize>,
    mut send: F,
) -> Result<reqwest::Response>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<reqwest::Response, reqwest::Error>>,
{
    crate::provider::validate_outbound_url(url).await?;
    let mut attempt = 0u32;

    loop {
        let resp = match send().await {
            Ok(resp) => resp,
            Err(transport_error) => {
                if let Some(policy_error) = crate::provider::outbound_forbidden_from_reqwest(&transport_error) {
                    return Err(policy_error);
                }
                // ~keep A transport-level error means no response was ever received, so it
                // must go through the same retry budget as a 5xx — otherwise a single
                // connection reset fails the whole request even though `max_retries > 0`.
                if let Some(delay) = retry::should_retry_transport_error(attempt, max_retries) {
                    attempt += 1;
                    tracing::warn!(
                        error = %transport_error.without_url(),
                        attempt,
                        max_retries,
                        "transport-level error sending request; retrying"
                    );
                    sleep_for_retry(delay).await;
                    continue;
                }
                return Err(LiterLlmError::from(transport_error));
            }
        };
        let status = resp.status().as_u16();

        if resp.status().is_success() {
            return Ok(resp);
        }

        let server_retry_after = retry_after_from_response(&resp);

        if let Some(delay) = retry::should_retry(status, attempt, max_retries, server_retry_after) {
            attempt += 1;
            sleep_for_retry(delay).await;
            continue;
        }

        let text = read_error_text(resp, max_response_bytes).await?;
        return Err(LiterLlmError::from_status(status, &text, server_retry_after));
    }
}

/// Send a POST request with a JSON body and return the raw response JSON.
///
/// Like `post_json` but returns a `serde_json::Value` instead of deserializing
/// into a typed `T`.  This allows the caller to mutate the response (e.g. via a
/// provider `transform_response`) before deserializing into the canonical type.
///
/// Retries on 429 / 5xx according to `max_retries`.
#[allow(dead_code, reason = "retain the unconfigured raw request entry point")]
pub async fn post_json_raw(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    body: Bytes,
    max_retries: u32,
) -> Result<serde_json::Value> {
    post_json_raw_bounded(
        client,
        url,
        auth_header,
        extra_headers,
        body,
        ResponseReadOptions {
            max_retries,
            max_response_bytes: None,
        },
    )
    .await
}

#[tracing::instrument(
    name = "post_json_raw",
    level = "debug",
    skip_all,
    fields(
        http.method = "POST",
        http.url = %url,
        http.status_code = tracing::field::Empty,
        http.retry_count = tracing::field::Empty,
    )
)]

pub(crate) async fn post_json_raw_bounded(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    body: Bytes,
    options: ResponseReadOptions,
) -> Result<serde_json::Value> {
    let ResponseReadOptions {
        max_retries,
        max_response_bytes,
    } = options;
    let mut retry_count = 0u32;

    let resp = with_retry_bounded(url, max_retries, max_response_bytes, || {
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

    {
        let span = tracing::Span::current();
        span.record("http.status_code", resp.status().as_u16());
        span.record("http.retry_count", retry_count.saturating_sub(1));
    }

    read_response_json(resp, max_response_bytes).await
}

/// Send a POST request with a JSON body and return the raw response bytes.
///
/// Identical to `post_json_raw` except it returns `bytes::Bytes` instead of
/// deserializing JSON.  Useful for endpoints that return binary data (e.g.
/// text-to-speech audio).
///
/// Retries on 429 / 5xx according to `max_retries`.
#[allow(dead_code, reason = "retain the unconfigured raw request entry point")]
pub async fn post_binary(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    body: Bytes,
    max_retries: u32,
) -> Result<Bytes> {
    post_binary_bounded(
        client,
        url,
        auth_header,
        extra_headers,
        body,
        ResponseReadOptions {
            max_retries,
            max_response_bytes: None,
        },
    )
    .await
}

#[tracing::instrument(
    name = "post_binary",
    level = "debug",
    skip_all,
    fields(
        http.method = "POST",
        http.url = %url,
        http.status_code = tracing::field::Empty,
        http.retry_count = tracing::field::Empty,
    )
)]

pub(crate) async fn post_binary_bounded(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    body: Bytes,
    options: ResponseReadOptions,
) -> Result<Bytes> {
    let ResponseReadOptions {
        max_retries,
        max_response_bytes,
    } = options;
    let mut retry_count = 0u32;

    let resp = with_retry_bounded(url, max_retries, max_response_bytes, || {
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

    {
        let span = tracing::Span::current();
        span.record("http.status_code", resp.status().as_u16());
        span.record("http.retry_count", retry_count.saturating_sub(1));
    }

    read_response_body(resp, max_response_bytes).await
}

/// Send a POST request with a multipart form body and return the raw response JSON.
///
/// Used for file uploads (Files API, audio transcription).  Multipart forms are
/// consumed by `send()` and cannot be cheaply cloned, so this function does
/// **not** retry on failure — file uploads are not idempotent anyway.
///
/// `auth_header` is `Some((name, value))` when the provider requires
/// authentication, or `None` when no auth header should be added.
#[allow(dead_code, reason = "retain the unconfigured raw request entry point")]
pub async fn post_multipart(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    form: reqwest::multipart::Form,
) -> Result<serde_json::Value> {
    post_multipart_bounded(client, url, auth_header, extra_headers, form, None).await
}

#[tracing::instrument(
    name = "post_multipart",
    level = "debug",
    skip_all,
    fields(
        http.method = "POST",
        http.url = %url,
        http.status_code = tracing::field::Empty,
    )
)]

pub(crate) async fn post_multipart_bounded(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    form: reqwest::multipart::Form,
    max_response_bytes: Option<usize>,
) -> Result<serde_json::Value> {
    crate::provider::validate_outbound_url(url).await?;
    let mut builder = client.post(url).multipart(form);
    if let Some((name, value)) = auth_header {
        builder = builder.header(name, value);
    }
    for (name, value) in extra_headers {
        builder = builder.header(*name, *value);
    }

    let resp = builder.send().await?;

    {
        let span = tracing::Span::current();
        span.record("http.status_code", resp.status().as_u16());
    }

    let status = resp.status().as_u16();
    if !resp.status().is_success() {
        let server_retry_after = retry_after_from_response(&resp);
        let text = read_error_text(resp, max_response_bytes).await?;
        return Err(LiterLlmError::from_status(status, &text, server_retry_after));
    }

    read_response_json(resp, max_response_bytes).await
}

/// Send a GET request and return the raw response JSON as `serde_json::Value`.
///
/// Returns a raw `serde_json::Value` without deserializing into a typed `T`.
/// Useful for endpoints where the caller needs to inspect or transform the
/// response before deserialization (e.g. GET /files/{id}, GET /batches/{id}).
///
/// Retries on 429 / 5xx according to `max_retries`.
#[allow(dead_code, reason = "retain the unconfigured raw request entry point")]
pub async fn get_json_raw(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    max_retries: u32,
) -> Result<serde_json::Value> {
    get_json_raw_bounded(client, url, auth_header, extra_headers, max_retries, None).await
}

#[tracing::instrument(
    name = "get_json_raw",
    level = "debug",
    skip_all,
    fields(
        http.method = "GET",
        http.url = %url,
        http.status_code = tracing::field::Empty,
        http.retry_count = tracing::field::Empty,
    )
)]

pub(crate) async fn get_json_raw_bounded(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    max_retries: u32,
    max_response_bytes: Option<usize>,
) -> Result<serde_json::Value> {
    let mut retry_count = 0u32;

    let resp = with_retry_bounded(url, max_retries, max_response_bytes, || {
        let mut builder = client.get(url);
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

    {
        let span = tracing::Span::current();
        span.record("http.status_code", resp.status().as_u16());
        span.record("http.retry_count", retry_count.saturating_sub(1));
    }

    read_response_json(resp, max_response_bytes).await
}

/// Send a DELETE request and return the raw response JSON.
///
/// Same retry/auth/header pattern as `get_json_raw` but uses the HTTP DELETE method.
/// Used for resource deletion endpoints (e.g. DELETE /files/{id}).
///
/// Retries on 429 / 5xx according to `max_retries`.
#[allow(dead_code, reason = "retain the unconfigured raw request entry point")]
pub async fn delete_json(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    max_retries: u32,
) -> Result<serde_json::Value> {
    delete_json_bounded(client, url, auth_header, extra_headers, max_retries, None).await
}

#[tracing::instrument(
    name = "delete_json",
    level = "debug",
    skip_all,
    fields(
        http.method = "DELETE",
        http.url = %url,
        http.status_code = tracing::field::Empty,
        http.retry_count = tracing::field::Empty,
    )
)]

pub(crate) async fn delete_json_bounded(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    max_retries: u32,
    max_response_bytes: Option<usize>,
) -> Result<serde_json::Value> {
    let mut retry_count = 0u32;

    let resp = with_retry_bounded(url, max_retries, max_response_bytes, || {
        let mut builder = client.delete(url);
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

    {
        let span = tracing::Span::current();
        span.record("http.status_code", resp.status().as_u16());
        span.record("http.retry_count", retry_count.saturating_sub(1));
    }

    read_response_json(resp, max_response_bytes).await
}

/// Send a GET request and return the raw response bytes.
///
/// Used for endpoints that return binary data (e.g. GET /files/{id}/content
/// for downloading file contents).
///
/// Retries on 429 / 5xx according to `max_retries`.
#[allow(dead_code, reason = "retain the unconfigured raw request entry point")]
pub async fn get_binary(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    max_retries: u32,
) -> Result<Bytes> {
    get_binary_bounded(client, url, auth_header, extra_headers, max_retries, None).await
}

#[tracing::instrument(
    name = "get_binary",
    level = "debug",
    skip_all,
    fields(
        http.method = "GET",
        http.url = %url,
        http.status_code = tracing::field::Empty,
        http.retry_count = tracing::field::Empty,
    )
)]

pub(crate) async fn get_binary_bounded(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
    max_retries: u32,
    max_response_bytes: Option<usize>,
) -> Result<Bytes> {
    let mut retry_count = 0u32;

    let resp = with_retry_bounded(url, max_retries, max_response_bytes, || {
        let mut builder = client.get(url);
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

    {
        let span = tracing::Span::current();
        span.record("http.status_code", resp.status().as_u16());
        span.record("http.retry_count", retry_count.saturating_sub(1));
    }

    read_response_body(resp, max_response_bytes).await
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener};

    use serial_test::serial;

    use super::*;
    use crate::provider::{OutboundPolicy, set_outbound_policy};

    /// Bind an ephemeral loopback port, then immediately release it. Connections to
    /// the now-closed port are refused instantly (no live network, no timeout wait),
    /// giving a deterministic, genuine `reqwest::Error` transport failure for tests.
    fn closed_port_url() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().expect("local_addr").port();
        drop(listener);
        format!("http://127.0.0.1:{port}/")
    }

    fn one_shot_json_server() -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind JSON server");
        let address = listener.local_addr().expect("JSON server address");
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept JSON request");
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request).expect("read JSON request");
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}")
                .expect("write JSON response");
        });
        (format!("http://{address}/v1/chat/completions"), handle)
    }

    pub(super) fn one_shot_server(response: String) -> (SocketAddr, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind HTTP server");
        let address = listener.local_addr().expect("HTTP server address");
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept HTTP request");
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request).expect("read HTTP request");
            stream.write_all(response.as_bytes()).expect("write HTTP response");
        });
        (address, handle)
    }

    #[derive(Clone, Default)]
    struct RetryWarnings(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

    impl tracing::Subscriber for RetryWarnings {
        fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
            *metadata.level() == tracing::Level::WARN
        }

        fn new_span(&self, _attributes: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }

        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
        fn enter(&self, _span: &tracing::span::Id) {}
        fn exit(&self, _span: &tracing::span::Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            if event.metadata().target() != module_path!().trim_end_matches("::tests") {
                return;
            }
            let mut fields = WarningFields::default();
            event.record(&mut fields);
            self.0.lock().expect("warning capture lock").push(fields.0);
        }
    }

    #[derive(Default)]
    struct WarningFields(String);

    impl tracing::field::Visit for WarningFields {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            use std::fmt::Write as _;
            write!(&mut self.0, "{}={value:?} ", field.name()).expect("write warning fields");
        }
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn with_retry_warning_omits_endpoint_credentials() {
        use tracing::instrument::WithSubscriber as _;

        let client = reqwest::Client::builder().no_proxy().build().expect("HTTP client");
        let url = format!("{}path-secret-marker?key=query-secret-marker", closed_port_url());
        let warnings = RetryWarnings::default();
        let mut attempts = 0;
        let result = with_retry(&url, 1, || {
            attempts += 1;
            client.get(&url).send()
        })
        .with_subscriber(warnings.clone())
        .await;

        assert!(result.is_err(), "closed port must fail after exhausting retries");
        assert_eq!(attempts, 2, "one retry must perform two actual requests");
        let events = warnings.0.lock().expect("warning capture lock");
        assert_eq!(events.len(), 1, "must capture the real transport retry warning");
        let warning = &events[0];
        assert!(warning.contains("transport-level error sending request; retrying"));
        assert!(
            warning.contains("error=error sending request"),
            "retain transport error category"
        );
        assert!(warning.contains("attempt=1"));
        assert!(warning.contains("max_retries=1"));
        assert!(
            !warning.contains("path-secret-marker"),
            "must not log endpoint path credentials"
        );
        assert!(
            !warning.contains("query-secret-marker"),
            "must not log endpoint query credentials"
        );
        assert!(!warning.contains("127.0.0.1"), "must remove the complete URL");
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn with_retry_retries_transport_errors_before_giving_up() {
        // ~keep Regression test for #44: before the fix, `send().await?` propagated a
        // transport-level error (e.g. connection refused) immediately via `?`,
        // bypassing the retry loop entirely regardless of `max_retries`.
        let client = reqwest::Client::new();
        let url = closed_port_url();
        let mut attempts = 0u32;

        let result = with_retry(&url, 2, || {
            attempts += 1;
            client.get(url.as_str()).send()
        })
        .await;

        assert!(
            result.is_err(),
            "every attempt fails at the transport layer, so the final result must be Err"
        );
        assert_eq!(
            attempts, 3,
            "must attempt once plus 2 retries (matching max_retries) before giving up"
        );
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn with_retry_does_not_retry_transport_errors_when_max_retries_is_zero() {
        let client = reqwest::Client::new();
        let url = closed_port_url();
        let mut attempts = 0u32;

        let result = with_retry(&url, 0, || {
            attempts += 1;
            client.get(url.as_str()).send()
        })
        .await;

        assert!(result.is_err());
        assert_eq!(
            attempts, 1,
            "max_retries = 0 must still make exactly one attempt and no retries"
        );
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn post_json_raw_rejects_direct_private_url_under_deny_private() {
        set_outbound_policy(OutboundPolicy::DenyPrivate);
        let result = post_json_raw(
            &reqwest::Client::new(),
            &closed_port_url(),
            None,
            &[],
            Bytes::from_static(b"{}"),
            0,
        )
        .await;
        set_outbound_policy(OutboundPolicy::Off);

        assert!(
            matches!(result, Err(LiterLlmError::OutboundForbidden { .. })),
            "private request URL must be rejected by the policy before reqwest connects: {result:?}"
        );
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn post_json_raw_allows_local_mock_when_policy_is_off() {
        set_outbound_policy(OutboundPolicy::Off);
        let (url, server) = one_shot_json_server();

        let result = post_json_raw(&reqwest::Client::new(), &url, None, &[], Bytes::from_static(b"{}"), 0).await;

        server.join().expect("JSON server thread");
        assert_eq!(
            result.expect("Off policy must preserve local mock access"),
            serde_json::json!({})
        );
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn post_multipart_rejects_direct_private_url_under_deny_private() {
        set_outbound_policy(OutboundPolicy::DenyPrivate);
        let result = post_multipart(
            &reqwest::Client::new(),
            &closed_port_url(),
            None,
            &[],
            reqwest::multipart::Form::new(),
        )
        .await;
        set_outbound_policy(OutboundPolicy::Off);

        assert!(
            matches!(result, Err(LiterLlmError::OutboundForbidden { .. })),
            "private multipart URL must be rejected before reqwest connects: {result:?}"
        );
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn post_multipart_preserves_redirect_policy_error_classification() {
        set_outbound_policy(OutboundPolicy::Off);
        let (source_address, source) = one_shot_server(
            "HTTP/1.1 302 Found\r\nLocation: http://169.254.169.254/token\r\nContent-Length: 0\r\n\r\n".to_owned(),
        );
        let source_url = format!("http://multipart.test:{}/upload", source_address.port());
        let client = crate::provider::configure_outbound_client_builder(
            reqwest::Client::builder().resolve("multipart.test", source_address),
            None,
        )
        .build()
        .expect("multipart redirect client");
        set_outbound_policy(OutboundPolicy::Allowlist(vec![
            url::Url::parse(&source_url).expect("source allowlist URL"),
        ]));

        let result = post_multipart(&client, &source_url, None, &[], reqwest::multipart::Form::new()).await;
        set_outbound_policy(OutboundPolicy::Off);

        source.join().expect("multipart source server");
        assert!(
            matches!(result, Err(LiterLlmError::OutboundForbidden { .. })),
            "multipart redirect policy errors must remain non-transient: {result:?}"
        );
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "request_body_tests.rs"]
mod body_tests;
