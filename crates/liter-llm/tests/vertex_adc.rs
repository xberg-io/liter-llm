//! Integration tests for the Vertex AI ADC credential provider.
//!
//! Spins up a lightweight in-process HTTP server that mimics the GCE metadata
//! server, then points the provider at it via a custom `reqwest::Client`.
//! Tests are fully offline — no real GCP credentials required.

/// Install the ring crypto provider before any test in this process runs.
///
/// rustls 0.23+ requires an explicit provider to be installed before any
/// `reqwest::Client` is constructed.
#[cfg(feature = "native-http")]
#[ctor::ctor(unsafe)]
fn init_crypto() {
    liter_llm::ensure_crypto_provider();
}

#[cfg(feature = "native-http")]
mod adc_tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::thread;

    use liter_llm::auth::CredentialProvider;
    use liter_llm::auth::vertex_adc::VertexAdcCredentialProvider;

    // ── Minimal mock metadata server ─────────────────────────────────────────

    /// Response the mock server will return for each incoming request.
    #[derive(Clone)]
    struct MockResponse {
        status: u16,
        body: String,
    }

    /// A single-endpoint HTTP server that counts how many requests it receives.
    struct MockMetadataServer {
        base_url: String,
        request_count: Arc<AtomicUsize>,
        _handle: thread::JoinHandle<()>,
    }

    impl MockMetadataServer {
        /// Start the server.  Every request returns `response` (regardless of
        /// method or path).
        fn start(response: MockResponse) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock metadata server");
            let port = listener.local_addr().unwrap().port();
            let base_url = format!("http://127.0.0.1:{port}");
            let request_count = Arc::new(AtomicUsize::new(0));
            let counter = Arc::clone(&request_count);

            let handle = thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else { break };
                    let mut reader = BufReader::new(stream.try_clone().unwrap());

                    // Read and discard the request line + headers so the client
                    // does not stall waiting for the server to drain the socket.
                    let mut line = String::new();
                    let _ = reader.read_line(&mut line); // request line
                    loop {
                        let mut header = String::new();
                        if reader.read_line(&mut header).is_err() || header.trim().is_empty() {
                            break;
                        }
                    }

                    counter.fetch_add(1, Ordering::SeqCst);

                    let status_text = if response.status == 200 {
                        "OK"
                    } else {
                        "Internal Server Error"
                    };
                    let reply = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        response.status,
                        status_text,
                        response.body.len(),
                        response.body,
                    );
                    let _ = stream.write_all(reply.as_bytes());
                    let _ = stream.flush();
                }
            });

            MockMetadataServer {
                base_url,
                request_count,
                _handle: handle,
            }
        }

        fn request_count(&self) -> usize {
            self.request_count.load(Ordering::SeqCst)
        }
    }

    /// Build a `reqwest::Client` whose metadata-server base URL is rewritten to
    /// point at the mock server.  We achieve this by constructing the provider
    /// with a custom URL — since `METADATA_TOKEN_URL` is a compile-time constant
    /// we cannot override it directly, so instead we pass a client configured
    /// with a proxy that intercepts requests to the metadata IP.
    ///
    /// A simpler (and more portable) approach: build the provider with a
    /// custom client that has a very short connect timeout so it fails fast for
    /// the real metadata server, then in tests we set `METADATA_TOKEN_URL` via
    /// a build-time cfg or accept that we must mock at the HTTP layer.
    ///
    /// Since the implementation calls `http://169.254.169.254/...` which is
    /// unreachable outside GCP, we need a way to redirect that call.  The
    /// cleanest approach that does not require changing production code is to
    /// use a `reqwest::Client` with a custom resolver — but reqwest does not
    /// expose that.  Instead, we expose a `with_metadata_url` constructor
    /// specifically for testing.
    fn build_provider_with_mock(server: &MockMetadataServer) -> VertexAdcCredentialProvider {
        // We expose a test-only constructor that overrides the metadata URL.
        // The production `new()` path uses the real 169.254.169.254 address.
        VertexAdcCredentialProvider::with_metadata_url(server.base_url.clone())
    }

    fn token_json(token: &str, expires_in: u64) -> String {
        format!(r#"{{"access_token":"{token}","expires_in":{expires_in},"token_type":"Bearer"}}"#)
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn first_call_hits_metadata_server_and_returns_token() {
        let server = MockMetadataServer::start(MockResponse {
            status: 200,
            body: token_json("ya29.test-token", 3600),
        });
        let provider = build_provider_with_mock(&server);

        let credential = provider.resolve().await.expect("resolve should succeed");

        assert!(
            matches!(credential, liter_llm::auth::Credential::BearerToken(_)),
            "expected BearerToken, got {credential:?}"
        );
        assert_eq!(
            server.request_count(),
            1,
            "should have hit the metadata server exactly once"
        );
    }

    #[tokio::test]
    async fn second_call_within_cache_window_does_not_hit_server() {
        let server = MockMetadataServer::start(MockResponse {
            status: 200,
            body: token_json("ya29.cached-token", 3600),
        });
        let provider = build_provider_with_mock(&server);

        // First call — populates the cache.
        let _ = provider.resolve().await.expect("first resolve should succeed");
        // Second call — should be served from cache.
        let _ = provider.resolve().await.expect("second resolve should succeed");

        assert_eq!(
            server.request_count(),
            1,
            "metadata server should only be called once; second call should use cached token"
        );
    }

    #[tokio::test]
    async fn metadata_server_5xx_returns_error_when_no_fallback_configured() {
        let server = MockMetadataServer::start(MockResponse {
            status: 500,
            body: r#"{"error":"internal"}"#.to_owned(),
        });
        let provider = build_provider_with_mock(&server)
            // Disable the gcp_auth fallback so we can test the error path in isolation.
            .without_gcp_auth_fallback();

        let result = provider.resolve().await;

        assert!(
            result.is_err(),
            "should return an error when metadata server returns 5xx and no fallback is configured"
        );

        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("401") || msg.contains("Authentication") || msg.contains("ADC"),
            "error message should indicate authentication failure, got: {msg}"
        );
    }
}
