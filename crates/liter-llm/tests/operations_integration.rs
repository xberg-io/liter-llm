//! Integration tests for File, Batch, Response, and rerank operations.
//!
//! Each test starts a lightweight mock HTTP server, constructs a `DefaultClient`
//! pointing at it, calls the relevant trait method, and verifies:
//! - The correct HTTP method and URL path were used.
//! - Query parameters are passed correctly.
//! - Request bodies serialize as expected.
//! - Responses deserialize into the correct Rust types.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use liter_llm::client::config::ClientConfigBuilder;
use liter_llm::client::{BatchClient, DefaultClient, FileClient, LlmClient, LlmClientRaw, ResponseClient};
use liter_llm::types::EmbeddingRequest;
use liter_llm::types::batch::{BatchListQuery, CreateBatchRequest};
use liter_llm::types::files::FileListQuery;
use liter_llm::types::rerank::{RerankDocument, RerankRequest};
use liter_llm::types::responses::CreateResponseRequest;
use liter_llm::{LiterLlmError, Result};

/// A captured HTTP request.
#[derive(Debug, Clone)]
struct CapturedRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

/// Configurable mock server that returns different responses based on method+path.
struct MockServer {
    addr: String,
    captured: Arc<Mutex<Vec<CapturedRequest>>>,
    _handle: thread::JoinHandle<()>,
}

/// A route definition for the mock server.
#[derive(Clone)]
struct MockRoute {
    method: String,
    path_prefix: String,
    status: u16,
    body: String,
}

impl MockServer {
    /// Start a mock server with configurable routes.
    /// Falls back to a 200 OK with a generic JSON response for unmatched routes.
    fn start_with_routes(routes: Vec<MockRoute>) -> Self {
        Self::start_with_responder(move |method, full_path, _index| {
            let path = full_path.split('?').next().unwrap_or("");
            routes
                .iter()
                .find(|r| r.method == method && path.starts_with(&r.path_prefix))
                .map(|r| (r.status, r.body.clone()))
                .unwrap_or((200, r#"{"ok": true}"#.to_string()))
        })
    }

    /// Start a mock server that answers each request with `responder(method, path_and_query, index)`,
    /// where `index` counts the requests that the server has received before this one.
    fn start_with_responder(responder: impl Fn(&str, &str, usize) -> (u16, String) + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind mock server");
        let addr = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let captured: Arc<Mutex<Vec<CapturedRequest>>> = Arc::new(Mutex::new(Vec::new()));
        let captured_writer = Arc::clone(&captured);

        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut reader = BufReader::new(stream.try_clone().unwrap());

                let mut request_line = String::new();
                if reader.read_line(&mut request_line).is_err() {
                    continue;
                }
                let parts: Vec<&str> = request_line.trim().splitn(3, ' ').collect();
                let method = parts.first().unwrap_or(&"").to_string();
                let full_path = parts.get(1).unwrap_or(&"").to_string();

                let mut headers = Vec::new();
                let mut content_length: usize = 0;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
                        break;
                    }
                    if let Some((name, value)) = line.trim().split_once(':') {
                        let name = name.trim().to_lowercase();
                        let value = value.trim().to_string();
                        if name == "content-length" {
                            content_length = value.parse().unwrap_or(0);
                        }
                        headers.push((name, value));
                    }
                }

                let mut body_buf = vec![0u8; content_length];
                if content_length > 0 {
                    let _ = std::io::Read::read_exact(&mut reader, &mut body_buf);
                }
                let body = String::from_utf8_lossy(&body_buf).to_string();

                let index = {
                    let mut captured = captured_writer.lock().unwrap();
                    captured.push(CapturedRequest {
                        method: method.clone(),
                        path: full_path.clone(),
                        headers,
                        body,
                    });
                    captured.len() - 1
                };

                let (status, resp_body) = responder(&method, &full_path, index);
                let status_text = match status {
                    200 => "OK",
                    201 => "Created",
                    204 => "No Content",
                    _ => "OK",
                };
                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    status_text,
                    resp_body.len(),
                    resp_body
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });

        MockServer {
            addr,
            captured,
            _handle: handle,
        }
    }

    fn requests(&self) -> Vec<CapturedRequest> {
        self.captured.lock().unwrap().clone()
    }

    fn url(&self) -> &str {
        &self.addr
    }
}

fn build_client(mock: &MockServer) -> DefaultClient {
    let config = ClientConfigBuilder::new("test-api-key")
        .base_url(mock.url())
        .max_retries(0)
        .build();
    DefaultClient::new(config, None).expect("client creation should succeed")
}

fn file_object_json() -> String {
    serde_json::json!({
        "id": "file-abc123",
        "object": "file",
        "bytes": 1024,
        "created_at": 1_700_000_000_u64,
        "filename": "data.jsonl",
        "purpose": "batch"
    })
    .to_string()
}

fn file_list_json() -> String {
    serde_json::json!({
        "object": "list",
        "data": [{
            "id": "file-abc123",
            "object": "file",
            "bytes": 1024,
            "created_at": 1_700_000_000_u64,
            "filename": "data.jsonl",
            "purpose": "batch"
        }],
        "has_more": false
    })
    .to_string()
}

fn delete_response_json() -> String {
    serde_json::json!({
        "id": "file-abc123",
        "object": "file",
        "deleted": true
    })
    .to_string()
}

fn batch_object_json() -> String {
    serde_json::json!({
        "id": "batch-xyz789",
        "object": "batch",
        "endpoint": "/v1/chat/completions",
        "input_file_id": "file-abc123",
        "completion_window": "24h",
        "status": "validating",
        "created_at": 1_700_000_000_u64
    })
    .to_string()
}

fn batch_list_json() -> String {
    serde_json::json!({
        "object": "list",
        "data": [{
            "id": "batch-xyz789",
            "object": "batch",
            "endpoint": "/v1/chat/completions",
            "input_file_id": "file-abc123",
            "completion_window": "24h",
            "status": "completed",
            "created_at": 1_700_000_000_u64
        }],
        "has_more": false
    })
    .to_string()
}

fn response_object_json() -> String {
    serde_json::json!({
        "id": "resp-001",
        "object": "response",
        "created_at": 1_700_000_000_u64,
        "model": "gpt-4",
        "status": "completed",
        "output": [{
            "type": "message",
            "content": "Hello"
        }],
        "usage": {
            "input_tokens": 10,
            "output_tokens": 20,
            "total_tokens": 30
        }
    })
    .to_string()
}

#[tokio::test]
async fn retrieve_file_should_send_get_to_correct_path() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "GET".into(),
        path_prefix: "/files/".into(),
        status: 200,
        body: file_object_json(),
    }]);
    let client = build_client(&mock);

    let result = client.retrieve_file("file-abc123").await;
    assert!(result.is_ok(), "retrieve_file should succeed: {result:?}");

    let file = result.unwrap();
    assert_eq!(file.id, "file-abc123");
    assert_eq!(file.filename, "data.jsonl");
    assert_eq!(file.bytes, 1024);

    let requests = mock.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/files/file-abc123");
}

#[tokio::test]
async fn delete_file_should_send_delete_method() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "DELETE".into(),
        path_prefix: "/files/".into(),
        status: 200,
        body: delete_response_json(),
    }]);
    let client = build_client(&mock);

    let result = client.delete_file("file-abc123").await;
    assert!(result.is_ok(), "delete_file should succeed: {result:?}");

    let resp = result.unwrap();
    assert_eq!(resp.id, "file-abc123");
    assert!(resp.deleted, "deleted should be true");

    let requests = mock.requests();
    assert_eq!(requests[0].method, "DELETE");
    assert_eq!(requests[0].path, "/files/file-abc123");
}

#[tokio::test]
async fn list_files_should_send_get_without_query_when_none() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "GET".into(),
        path_prefix: "/files".into(),
        status: 200,
        body: file_list_json(),
    }]);
    let client = build_client(&mock);

    let result = client.list_files(None).await;
    assert!(result.is_ok(), "list_files should succeed: {result:?}");

    let list = result.unwrap();
    assert_eq!(list.data.len(), 1);

    let requests = mock.requests();
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/files", "no query string when query is None");
}

#[tokio::test]
async fn list_files_should_include_query_parameters() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "GET".into(),
        path_prefix: "/files".into(),
        status: 200,
        body: file_list_json(),
    }]);
    let client = build_client(&mock);

    let query = FileListQuery {
        purpose: Some("batch".into()),
        limit: Some(10),
        after: Some("file-cursor".into()),
    };
    let result = client.list_files(Some(query)).await;
    assert!(result.is_ok(), "list_files with query should succeed");

    let requests = mock.requests();
    let path = &requests[0].path;
    assert!(path.contains("purpose=batch"), "should include purpose param: {path}");
    assert!(path.contains("limit=10"), "should include limit param: {path}");
    assert!(path.contains("after=file-cursor"), "should include after param: {path}");
}

#[tokio::test]
async fn file_content_should_return_bytes() {
    let content_bytes = b"file content here";
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "GET".into(),
        path_prefix: "/files/".into(),
        status: 200,
        body: String::from_utf8_lossy(content_bytes).to_string(),
    }]);
    let client = build_client(&mock);

    let result = client.file_content("file-abc123").await;
    assert!(result.is_ok(), "file_content should succeed: {result:?}");

    let bytes = result.unwrap();
    assert_eq!(bytes.as_ref(), content_bytes);

    let requests = mock.requests();
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/files/file-abc123/content");
}

#[tokio::test]
async fn create_batch_should_send_post_with_json_body() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/batches".into(),
        status: 200,
        body: batch_object_json(),
    }]);
    let client = build_client(&mock);

    let req = CreateBatchRequest {
        input_file_id: "file-abc123".into(),
        endpoint: "/v1/chat/completions".into(),
        completion_window: "24h".into(),
        metadata: None,
    };
    let result = client.create_batch(req).await;
    assert!(result.is_ok(), "create_batch should succeed: {result:?}");

    let batch = result.unwrap();
    assert_eq!(batch.id, "batch-xyz789");
    assert_eq!(batch.endpoint, "/v1/chat/completions");
    assert_eq!(batch.input_file_id, "file-abc123");

    let requests = mock.requests();
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/batches");

    let body: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
    assert_eq!(body["input_file_id"], "file-abc123");
    assert_eq!(body["endpoint"], "/v1/chat/completions");
    assert_eq!(body["completion_window"], "24h");
}

#[tokio::test]
async fn retrieve_batch_should_send_get_to_correct_path() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "GET".into(),
        path_prefix: "/batches/".into(),
        status: 200,
        body: batch_object_json(),
    }]);
    let client = build_client(&mock);

    let result = client.retrieve_batch("batch-xyz789").await;
    assert!(result.is_ok(), "retrieve_batch should succeed: {result:?}");

    let batch = result.unwrap();
    assert_eq!(batch.id, "batch-xyz789");

    let requests = mock.requests();
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/batches/batch-xyz789");
}

#[tokio::test]
async fn list_batches_should_include_query_parameters() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "GET".into(),
        path_prefix: "/batches".into(),
        status: 200,
        body: batch_list_json(),
    }]);
    let client = build_client(&mock);

    let query = BatchListQuery {
        limit: Some(5),
        after: Some("batch-cursor".into()),
    };
    let result = client.list_batches(Some(query)).await;
    assert!(result.is_ok(), "list_batches should succeed: {result:?}");

    let list = result.unwrap();
    assert_eq!(list.data.len(), 1);

    let requests = mock.requests();
    let path = &requests[0].path;
    assert!(path.contains("limit=5"), "should include limit param: {path}");
    assert!(
        path.contains("after=batch-cursor"),
        "should include after param: {path}"
    );
}

#[tokio::test]
async fn list_batches_should_send_get_without_query_when_none() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "GET".into(),
        path_prefix: "/batches".into(),
        status: 200,
        body: batch_list_json(),
    }]);
    let client = build_client(&mock);

    let result = client.list_batches(None).await;
    assert!(result.is_ok());

    let requests = mock.requests();
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/batches", "no query string when query is None");
}

#[tokio::test]
async fn cancel_batch_should_post_to_cancel_endpoint() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/batches/".into(),
        status: 200,
        body: batch_object_json(),
    }]);
    let client = build_client(&mock);

    let result = client.cancel_batch("batch-xyz789").await;
    assert!(result.is_ok(), "cancel_batch should succeed: {result:?}");

    let requests = mock.requests();
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/batches/batch-xyz789/cancel");
}

#[tokio::test]
async fn create_response_should_send_post_with_json_body() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/responses".into(),
        status: 200,
        body: response_object_json(),
    }]);
    let client = build_client(&mock);

    let req = CreateResponseRequest {
        model: "gpt-4".into(),
        input: serde_json::json!("What is the capital of France?"),
        instructions: Some("Be concise.".into()),
        tools: None,
        temperature: None,
        max_output_tokens: None,
        metadata: None,
        extra_body: None,
        stream: None,
    };
    let result = client.create_response(req).await;
    assert!(result.is_ok(), "create_response should succeed: {result:?}");

    let resp = result.unwrap();
    assert_eq!(resp.id, "resp-001");
    assert_eq!(resp.model, "gpt-4");
    assert_eq!(resp.status, "completed");
    assert_eq!(resp.output.len(), 1);

    let usage = resp.usage.expect("usage should be present");
    assert_eq!(usage.input_tokens, 10);
    assert_eq!(usage.output_tokens, 20);
    assert_eq!(usage.total_tokens, 30);

    let requests = mock.requests();
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/responses");

    let body: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
    assert_eq!(body["model"], "gpt-4");
    assert!(body["input"].is_string(), "input should be serialized");
}

#[tokio::test]
async fn retrieve_response_should_send_get_to_correct_path() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "GET".into(),
        path_prefix: "/responses/".into(),
        status: 200,
        body: response_object_json(),
    }]);
    let client = build_client(&mock);

    let result = client.retrieve_response("resp-001").await;
    assert!(result.is_ok(), "retrieve_response should succeed: {result:?}");

    let resp = result.unwrap();
    assert_eq!(resp.id, "resp-001");

    let requests = mock.requests();
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/responses/resp-001");
}

#[tokio::test]
async fn cancel_response_should_post_to_cancel_endpoint() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/responses/".into(),
        status: 200,
        body: response_object_json(),
    }]);
    let client = build_client(&mock);

    let result = client.cancel_response("resp-001").await;
    assert!(result.is_ok(), "cancel_response should succeed: {result:?}");

    let requests = mock.requests();
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/responses/resp-001/cancel");
}

/// Builds an Azure-routed client whose `base_url` does not already embed a deployment
/// segment (`/openai/deployments/...`), which is the configuration under which
/// `AzureProvider::build_url` embeds the model into the path.
fn build_azure_client(mock: &MockServer) -> DefaultClient {
    let config = ClientConfigBuilder::new("test-api-key")
        .base_url(mock.url())
        .max_retries(0)
        .build();
    DefaultClient::new(config, Some("azure/gpt-4o")).expect("client creation should succeed")
}

fn assert_responses_endpoint_not_supported_by_azure<T>(result: &Result<T>) {
    match result {
        Err(LiterLlmError::EndpointNotSupported { endpoint, provider }) => {
            assert_eq!(endpoint, "responses");
            assert_eq!(provider, "azure");
        }
        Err(other) => panic!("expected EndpointNotSupported, got a different error: {other}"),
        Ok(_) => panic!("expected Err(EndpointNotSupported), got Ok"),
    }
}

/// Revert line: removing `reject_malformed_responses_url(&url, self.provider.name())?;` from
/// `DefaultClient::create_response` in `crates/liter-llm/src/client/mod.rs` makes this test fail
/// — the request would instead be sent to `.../openai/deployments//responses` and this mock
/// server would record it.
#[tokio::test]
async fn create_response_rejects_azure_empty_deployment_before_any_request() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/responses".into(),
        status: 200,
        body: response_object_json(),
    }]);
    let client = build_azure_client(&mock);

    let req = CreateResponseRequest {
        model: "gpt-4o".into(),
        input: serde_json::json!("What is the capital of France?"),
        instructions: None,
        tools: None,
        temperature: None,
        max_output_tokens: None,
        metadata: None,
        extra_body: None,
        stream: None,
    };
    let result = client.create_response(req).await;

    assert_responses_endpoint_not_supported_by_azure(&result);
    assert!(
        mock.requests().is_empty(),
        "the malformed URL must be rejected before any HTTP request is sent, got {:?}",
        mock.requests()
    );
}

/// Revert line: removing `reject_malformed_responses_url(&url, self.provider.name())?;` from
/// `DefaultClient::create_response_stream` in `crates/liter-llm/src/client/mod.rs` makes this
/// test fail — the stream would instead try to open against `.../openai/deployments//responses`.
#[tokio::test]
async fn create_response_stream_rejects_azure_empty_deployment_before_any_request() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/responses".into(),
        status: 200,
        body: response_object_json(),
    }]);
    let client = build_azure_client(&mock);

    let req = CreateResponseRequest {
        model: "gpt-4o".into(),
        input: serde_json::json!("What is the capital of France?"),
        instructions: None,
        tools: None,
        temperature: None,
        max_output_tokens: None,
        metadata: None,
        extra_body: None,
        stream: None,
    };
    let result = client.create_response_stream(req).await;

    assert_responses_endpoint_not_supported_by_azure(&result);
    assert!(
        mock.requests().is_empty(),
        "the malformed URL must be rejected before any HTTP request is sent, got {:?}",
        mock.requests()
    );
}

/// Revert line: removing `reject_malformed_responses_url(&base_url, self.provider.name())?;`
/// from `DefaultClient::retrieve_response` in `crates/liter-llm/src/client/mod.rs` makes this
/// test fail — the request would instead be sent to
/// `.../openai/deployments//responses/resp-001`.
#[tokio::test]
async fn retrieve_response_rejects_azure_empty_deployment_before_any_request() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "GET".into(),
        path_prefix: "/responses/".into(),
        status: 200,
        body: response_object_json(),
    }]);
    let client = build_azure_client(&mock);

    let result = client.retrieve_response("resp-001").await;

    assert_responses_endpoint_not_supported_by_azure(&result);
    assert!(
        mock.requests().is_empty(),
        "the malformed URL must be rejected before any HTTP request is sent, got {:?}",
        mock.requests()
    );
}

/// Revert line: removing `reject_malformed_responses_url(&base_url, self.provider.name())?;`
/// from `DefaultClient::cancel_response` in `crates/liter-llm/src/client/mod.rs` makes this test
/// fail — the request would instead be sent to
/// `.../openai/deployments//responses/resp-001/cancel`.
#[tokio::test]
async fn cancel_response_rejects_azure_empty_deployment_before_any_request() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/responses/".into(),
        status: 200,
        body: response_object_json(),
    }]);
    let client = build_azure_client(&mock);

    let result = client.cancel_response("resp-001").await;

    assert_responses_endpoint_not_supported_by_azure(&result);
    assert!(
        mock.requests().is_empty(),
        "the malformed URL must be rejected before any HTTP request is sent, got {:?}",
        mock.requests()
    );
}

#[tokio::test]
async fn all_operations_should_include_auth_header() {
    let mock = MockServer::start_with_routes(vec![
        MockRoute {
            method: "GET".into(),
            path_prefix: "/files".into(),
            status: 200,
            body: file_list_json(),
        },
        MockRoute {
            method: "GET".into(),
            path_prefix: "/batches".into(),
            status: 200,
            body: batch_list_json(),
        },
    ]);
    let client = build_client(&mock);

    let _ = client.list_files(None).await;
    let _ = client.list_batches(None).await;

    let requests = mock.requests();
    assert_eq!(requests.len(), 2);

    for (i, req) in requests.iter().enumerate() {
        let auth = req
            .headers
            .iter()
            .find(|(n, _)| n == "authorization")
            .map(|(_, v)| v.as_str());
        assert!(auth.is_some(), "request {i} should include Authorization header");
        assert!(
            auth.unwrap().contains("test-api-key"),
            "request {i} should contain the api key"
        );
    }
}

#[tokio::test]
async fn batch_create_with_metadata_should_serialize_metadata() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/batches".into(),
        status: 200,
        body: batch_object_json(),
    }]);
    let client = build_client(&mock);

    let req = CreateBatchRequest {
        input_file_id: "file-meta".into(),
        endpoint: "/v1/chat/completions".into(),
        completion_window: "24h".into(),
        metadata: Some(serde_json::json!({"run_id": "test-run-42"})),
    };
    let _ = client.create_batch(req).await;

    let requests = mock.requests();
    let body: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
    assert_eq!(body["metadata"]["run_id"], "test-run-42");
}

#[tokio::test]
async fn create_response_stream_should_yield_ordered_events_and_terminate_on_completed() {
    use futures_util::StreamExt;
    use liter_llm::types::responses::ResponseStreamEvent;

    let sse_body = concat!(
        "data: {\"type\":\"response.output_item.added\",\"output_index\":0,",
        "\"item\":{\"type\":\"message\",\"id\":\"msg_1\"}}\n\n",
        "data: {\"type\":\"response.output_text.delta\",\"item_id\":\"msg_1\",",
        "\"output_index\":0,\"content_index\":0,\"delta\":\"Hello\"}\n\n",
        "data: {\"type\":\"response.output_text.delta\",\"item_id\":\"msg_1\",",
        "\"output_index\":0,\"content_index\":0,\"delta\":\" world\"}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"output_index\":0,",
        "\"item\":{\"type\":\"message\",\"id\":\"msg_1\"}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-001\",",
        "\"object\":\"response\",\"created_at\":1700000000,\"model\":\"gpt-4\",",
        "\"status\":\"completed\",\"output\":[]}}\n\n",
    );

    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/responses".into(),
        status: 200,
        body: sse_body.to_string(),
    }]);
    let client = build_client(&mock);

    let req = CreateResponseRequest {
        model: "gpt-4".into(),
        input: serde_json::json!("What is the capital of France?"),
        instructions: None,
        tools: None,
        temperature: None,
        max_output_tokens: None,
        metadata: None,
        extra_body: None,
        stream: None,
    };

    let mut stream = client
        .create_response_stream(req)
        .await
        .expect("stream should open successfully");

    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event.expect("each SSE event should parse successfully"));
    }

    assert_eq!(events.len(), 5, "expected exactly 5 SSE events, got {events:?}");

    match &events[0] {
        ResponseStreamEvent::OutputItemAdded(added) => {
            assert_eq!(added.output_index, 0);
            assert_eq!(added.item.item_type, "message");
        }
        other => panic!("expected OutputItemAdded as event 0, got {other:?}"),
    }

    match &events[1] {
        ResponseStreamEvent::OutputTextDelta(delta) => {
            assert_eq!(delta.delta, "Hello");
            assert_eq!(delta.item_id, "msg_1");
            assert_eq!(delta.output_index, 0);
            assert_eq!(delta.content_index, 0);
        }
        other => panic!("expected OutputTextDelta as event 1, got {other:?}"),
    }

    match &events[2] {
        ResponseStreamEvent::OutputTextDelta(delta) => {
            assert_eq!(delta.delta, " world");
        }
        other => panic!("expected OutputTextDelta as event 2, got {other:?}"),
    }

    match &events[3] {
        ResponseStreamEvent::OutputItemDone(done) => {
            assert_eq!(done.output_index, 0);
            assert_eq!(done.item.item_type, "message");
        }
        other => panic!("expected OutputItemDone as event 3, got {other:?}"),
    }

    match &events[4] {
        ResponseStreamEvent::Completed(completed) => {
            assert_eq!(completed.response.id, "resp-001");
            assert_eq!(completed.response.status, "completed");
        }
        other => panic!("expected Completed as event 4, got {other:?}"),
    }

    // ~keep The mock server closes the connection after the terminal event; the
    // stream must end cleanly with no further items rather than hanging or erroring.
    assert!(
        stream.next().await.is_none(),
        "stream should terminate cleanly after response.completed"
    );

    let requests = mock.requests();
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/responses");
    let body: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
    assert_eq!(
        body["stream"],
        serde_json::Value::Bool(true),
        "stream must be forced to true on the wire"
    );
}

#[tokio::test]
async fn response_object_deserializes_output_items() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "GET".into(),
        path_prefix: "/responses/".into(),
        status: 200,
        body: response_object_json(),
    }]);
    let client = build_client(&mock);

    let resp = client.retrieve_response("resp-001").await.expect("should succeed");

    assert_eq!(resp.output.len(), 1);
    assert_eq!(resp.output[0].item_type, "message");
}

fn rerank_request(model: &str) -> RerankRequest {
    RerankRequest {
        model: model.to_owned(),
        query: "q".to_owned(),
        documents: ["a", "b", "c"].map(|d| RerankDocument::Text(d.to_owned())).to_vec(),
        ..Default::default()
    }
}

fn voyage_rerank_reply() -> String {
    serde_json::json!({
        "object": "list",
        "data": [
            {"index": 2, "relevance_score": 0.9},
            {"index": 0, "relevance_score": 0.5},
            {"index": 1, "relevance_score": 0.1}
        ],
        "model": "rerank-2",
        "usage": {"total_tokens": 3}
    })
    .to_string()
}

#[tokio::test]
async fn rerank_voyage_reply_with_data_array_parses_into_results() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/rerank".into(),
        status: 200,
        body: voyage_rerank_reply(),
    }]);
    let client = build_client(&mock);

    let resp = client
        .rerank(rerank_request("voyage/rerank-2"))
        .await
        .expect("a Voyage rerank reply should parse");

    let order: Vec<u32> = resp.results.iter().map(|r| r.index).collect();
    assert_eq!(order, vec![2, 0, 1]);
    assert_eq!(resp.results[0].relevance_score, 0.9);

    let requests = mock.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/rerank");
}

#[tokio::test]
async fn rerank_voyage_reply_parses_when_the_client_names_voyage_as_hint() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/rerank".into(),
        status: 200,
        body: voyage_rerank_reply(),
    }]);
    let config = ClientConfigBuilder::new("test-api-key")
        .base_url(mock.url())
        .max_retries(0)
        .build();
    let client = DefaultClient::new(config, Some("voyage")).expect("client creation should succeed");

    let resp = client
        .rerank(rerank_request("voyage/rerank-2"))
        .await
        .expect("a Voyage rerank reply should parse");

    assert_eq!(resp.results.len(), 3);
    let requests = mock.requests();
    let body: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
    assert_eq!(body["model"], "rerank-2");
}

#[tokio::test]
async fn rerank_raw_voyage_reply_with_data_array_parses_and_keeps_the_raw_reply() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/rerank".into(),
        status: 200,
        body: voyage_rerank_reply(),
    }]);
    let client = build_client(&mock);

    let exchange = client
        .rerank_raw(rerank_request("voyage/rerank-2"))
        .await
        .expect("a Voyage rerank reply should parse");

    assert_eq!(exchange.data.results.len(), 3);
    let raw = exchange.raw_response.expect("raw reply is kept");
    assert!(raw.get("data").is_some(), "the raw reply stays as the provider sent it");
    assert!(raw.get("results").is_none());
}

#[tokio::test]
async fn rerank_voyage_reply_with_document_strings_parses() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/rerank".into(),
        status: 200,
        body: serde_json::json!({
            "object": "list",
            "data": [{"index": 1, "relevance_score": 0.8, "document": "bravo"}],
            "model": "rerank-2",
            "usage": {"total_tokens": 3}
        })
        .to_string(),
    }]);
    let client = build_client(&mock);

    let resp = client
        .rerank(rerank_request("voyage/rerank-2"))
        .await
        .expect("a Voyage rerank reply with documents should parse");

    assert_eq!(
        resp.results[0].document.as_ref().map(|d| d.text.as_str()),
        Some("bravo")
    );
}

#[tokio::test]
async fn rerank_voyage_reply_with_results_array_still_parses() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/rerank".into(),
        status: 200,
        body: serde_json::json!({
            "id": "r1",
            "results": [{"index": 2, "relevance_score": 0.9}, {"index": 0, "relevance_score": 0.5}]
        })
        .to_string(),
    }]);
    let client = build_client(&mock);

    let resp = client
        .rerank(rerank_request("voyage/rerank-2"))
        .await
        .expect("a reply with a results array should parse");

    assert_eq!(resp.results.len(), 2);
}

#[tokio::test]
async fn rerank_cohere_reply_keeps_its_results_array() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/rerank".into(),
        status: 200,
        body: serde_json::json!({
            "id": "r1",
            "results": [{"index": 2, "relevance_score": 0.9}, {"index": 0, "relevance_score": 0.5}],
            "meta": {"billed_units": {"search_units": 1}}
        })
        .to_string(),
    }]);
    let client = build_client(&mock);

    let resp = client
        .rerank(rerank_request("cohere/rerank-v3.5"))
        .await
        .expect("a Cohere rerank reply should parse");

    let order: Vec<u32> = resp.results.iter().map(|r| r.index).collect();
    assert_eq!(order, vec![2, 0]);
}

fn rerank_mock(body: impl Into<String>) -> MockServer {
    MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/rerank".into(),
        status: 200,
        body: body.into(),
    }])
}

fn hinted_client(mock: &MockServer, hint: &str) -> DefaultClient {
    let config = ClientConfigBuilder::new("test-api-key")
        .base_url(mock.url())
        .max_retries(0)
        .build();
    DefaultClient::new(config, Some(hint)).expect("client creation should succeed")
}

#[tokio::test]
async fn rerank_voyage_reply_parses_for_a_bare_model_when_the_client_names_voyage_as_hint() {
    let mock = rerank_mock(voyage_rerank_reply());
    let client = hinted_client(&mock, "voyage");

    let resp = client
        .rerank(rerank_request("rerank-2"))
        .await
        .expect("a Voyage rerank reply should parse for a client that names Voyage");

    let order: Vec<u32> = resp.results.iter().map(|r| r.index).collect();
    assert_eq!(order, vec![2, 0, 1]);
}

#[tokio::test]
async fn rerank_voyage_reply_with_a_hint_that_names_a_model_parses_for_a_bare_model() {
    let mock = rerank_mock(voyage_rerank_reply());
    let client = hinted_client(&mock, "voyage/rerank-2");

    let resp = client
        .rerank_raw(rerank_request("rerank-2"))
        .await
        .expect("a Voyage rerank reply should parse for a client that names Voyage");

    assert_eq!(resp.data.results.len(), 3);
}

#[tokio::test]
async fn rerank_voyage_reply_stays_unparsed_for_a_bare_model_when_the_client_names_another_provider() {
    let mock = rerank_mock(voyage_rerank_reply());
    let client = hinted_client(&mock, "jina");

    let err = client
        .rerank(rerank_request("rerank-2"))
        .await
        .expect_err("a client that names another provider keeps that provider's reply shape");

    assert!(err.to_string().contains("results"), "unexpected error: {err}");
}

#[tokio::test]
#[serial_test::serial]
async fn rerank_voyage_reply_parses_when_a_custom_provider_claims_the_voyage_prefix() {
    let mock = rerank_mock(voyage_rerank_reply());
    liter_llm::register_custom_provider(liter_llm::CustomProviderConfig {
        name: "voyage-through-gateway".into(),
        base_url: mock.url().to_owned(),
        auth_header: liter_llm::AuthHeaderFormat::Bearer,
        model_prefixes: vec!["voyage/".into()],
    })
    .expect("registration should succeed");
    let config = ClientConfigBuilder::new("test-api-key").max_retries(0).build();
    let client = DefaultClient::new(config, None).expect("client creation should succeed");

    let result = client.rerank(rerank_request("voyage/rerank-2")).await;
    liter_llm::unregister_custom_provider("voyage-through-gateway").expect("unregister should succeed");

    let resp = result.expect("the Voyage reply shape is read although a custom provider routes the request");
    assert_eq!(
        mock.requests().len(),
        1,
        "the custom provider still decides where the request goes"
    );
    let order: Vec<u32> = resp.results.iter().map(|r| r.index).collect();
    assert_eq!(order, vec![2, 0, 1]);
}

#[tokio::test]
async fn rerank_reply_of_a_provider_with_a_chat_transform_keeps_its_results_array() {
    let mock = rerank_mock(
        serde_json::json!({
            "id": "r1",
            "results": [{"index": 2, "relevance_score": 0.9}, {"index": 0, "relevance_score": 0.5}]
        })
        .to_string(),
    );
    let client = hinted_client(&mock, "gemini/gemini-2.5-flash");

    let resp = client
        .rerank(rerank_request("cohere/rerank-v3.5"))
        .await
        .expect("a rerank reply must not pass through the chat transform of the client's provider");

    let order: Vec<u32> = resp.results.iter().map(|r| r.index).collect();
    assert_eq!(order, vec![2, 0]);
}

#[tokio::test]
async fn rerank_voyage_reply_with_null_results_beside_data_parses() {
    let mock = rerank_mock(
        serde_json::json!({
            "results": null,
            "data": [{"index": 1, "relevance_score": 0.7}]
        })
        .to_string(),
    );
    let client = build_client(&mock);

    let resp = client
        .rerank(rerank_request("voyage/rerank-2"))
        .await
        .expect("null results beside a valid data array should parse");

    assert_eq!(resp.results.len(), 1);
    assert_eq!(resp.results[0].index, 1);
}

#[tokio::test]
async fn rerank_voyage_reply_with_empty_data_parses_into_no_results() {
    let mock = rerank_mock(serde_json::json!({"object": "list", "data": []}).to_string());
    let client = build_client(&mock);

    let resp = client
        .rerank(rerank_request("voyage/rerank-2"))
        .await
        .expect("an empty data array should parse");

    assert!(resp.results.is_empty());
}

#[tokio::test]
async fn rerank_voyage_reply_item_without_index_is_a_clear_error() {
    let mock = rerank_mock(serde_json::json!({"data": [{"relevance_score": 0.9}]}).to_string());
    let client = build_client(&mock);

    let err = client
        .rerank(rerank_request("voyage/rerank-2"))
        .await
        .expect_err("an item without an index is an error");

    assert!(
        matches!(err, LiterLlmError::Serialization(_)),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("index"), "unexpected error: {err}");
}

#[tokio::test]
async fn rerank_voyage_reply_item_without_relevance_score_is_a_clear_error() {
    let mock = rerank_mock(serde_json::json!({"data": [{"index": 0}]}).to_string());
    let client = build_client(&mock);

    let err = client
        .rerank(rerank_request("voyage/rerank-2"))
        .await
        .expect_err("an item without a relevance score is an error");

    assert!(
        matches!(err, LiterLlmError::Serialization(_)),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("relevance_score"), "unexpected error: {err}");
}

#[tokio::test]
async fn rerank_reply_that_is_not_an_object_is_a_parse_error_not_a_panic() {
    for body in ["[1, 2]", "\"text\"", "7"] {
        let mock = rerank_mock(body);
        let client = build_client(&mock);

        let err = client
            .rerank(rerank_request("voyage/rerank-2"))
            .await
            .expect_err("a reply that is not an object is an error");

        assert!(
            matches!(err, LiterLlmError::Serialization(_)),
            "body {body}: unexpected error: {err:?}"
        );
    }
}

fn embedding_reply_without_a_model() -> String {
    serde_json::json!({
        "object": "list",
        "data": [{"object": "embedding", "embedding": [0.5], "index": 0}],
        "model": "",
    })
    .to_string()
}

fn embedding_request() -> EmbeddingRequest {
    EmbeddingRequest {
        model: "text-embedding-3-small".into(),
        input: liter_llm::types::EmbeddingInput::Single("alpha".into()),
        ..Default::default()
    }
}

const IMAGEN_MODEL: &str = "vertex_ai/imagen-3.0-generate-002";

#[tokio::test]
async fn vertex_image_generate_posts_to_the_imagen_predict_path() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/publishers/google/models/".into(),
        status: 200,
        body: serde_json::json!({
            "predictions": [
                {"bytesBase64Encoded": "aGk=", "mimeType": "image/png"},
                {"bytesBase64Encoded": "eW8=", "mimeType": "image/png", "prompt": "a tabby cat"}
            ]
        })
        .to_string(),
    }]);
    let client = hinted_client(&mock, IMAGEN_MODEL);

    let resp = client
        .image_generate(liter_llm::types::CreateImageRequest {
            prompt: "a cat".into(),
            model: Some(IMAGEN_MODEL.into()),
            n: Some(2),
            size: Some("1920x1080".into()),
            ..Default::default()
        })
        .await
        .expect("an Imagen reply should parse");

    let requests = mock.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(
        requests[0].path,
        "/publishers/google/models/imagen-3.0-generate-002:predict"
    );
    let sent: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
    assert_eq!(
        sent,
        serde_json::json!({
            "instances": [{"prompt": "a cat"}],
            "parameters": {"sampleCount": 2, "aspectRatio": "16:9"}
        })
    );
    assert_eq!(resp.data.len(), 2);
    assert_eq!(resp.data[0].b64_json.as_deref(), Some("aGk="));
    assert_eq!(resp.data[1].revised_prompt.as_deref(), Some("a tabby cat"));
}

#[tokio::test]
async fn vertex_image_generate_refuses_a_size_with_no_imagen_aspect_ratio() {
    let mock = MockServer::start_with_routes(vec![]);
    let client = hinted_client(&mock, IMAGEN_MODEL);

    let err = client
        .image_generate(liter_llm::types::CreateImageRequest {
            prompt: "a cat".into(),
            model: Some(IMAGEN_MODEL.into()),
            size: Some("1000x300".into()),
            ..Default::default()
        })
        .await
        .expect_err("an aspect ratio Imagen lacks is refused before any request");

    assert!(err.to_string().contains("1000x300"), "unexpected error: {err}");
    assert!(mock.requests().is_empty());
}

#[tokio::test]
async fn vertex_list_models_gets_the_publisher_models() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "GET".into(),
        path_prefix: "/v1beta1/publishers/google/models".into(),
        status: 200,
        body: serde_json::json!({
            "publisherModels": [{"name": "publishers/google/models/gemini-2.0-flash", "versionId": "001"}]
        })
        .to_string(),
    }]);
    let client = hinted_client(&mock, "vertex_ai/gemini-2.0-flash");

    let resp = client
        .list_models()
        .await
        .expect("a publisher model listing should parse");

    let requests = mock.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/v1beta1/publishers/google/models");
    assert_eq!(resp.data.len(), 1);
    assert_eq!(resp.data[0].id, "gemini-2.0-flash");
    assert_eq!(resp.data[0].owned_by, "google");
}

#[tokio::test]
async fn vertex_image_generate_returns_no_images_when_the_safety_filter_removed_them_all() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/publishers/google/models/".into(),
        status: 200,
        body: serde_json::json!({
            "predictions": [{"raiFilteredReason": "Image blocked by the safety filter"}]
        })
        .to_string(),
    }]);
    let client = hinted_client(&mock, IMAGEN_MODEL);

    let resp = client
        .image_generate(liter_llm::types::CreateImageRequest {
            prompt: "a cat".into(),
            model: Some(IMAGEN_MODEL.into()),
            ..Default::default()
        })
        .await
        .expect("a reply that the safety filter emptied is an image reply with no images");

    assert!(resp.data.is_empty());
}

#[tokio::test]
async fn vertex_list_models_follows_the_next_page_token_to_the_last_page() {
    let mock = MockServer::start_with_responder(|_method, path, _index| {
        let body = if path.contains("pageToken=t%2B2") {
            serde_json::json!({"publisherModels": [{"name": "publishers/google/models/gemini-b"}]})
        } else if path.contains("pageToken=t%2B1") {
            serde_json::json!({
                "publisherModels": [{"name": "publishers/meta/models/llama-a"}],
                "nextPageToken": "t+2"
            })
        } else {
            serde_json::json!({
                "publisherModels": [{"name": "publishers/google/models/gemini-a"}],
                "nextPageToken": "t+1"
            })
        };
        (200, body.to_string())
    });
    let client = hinted_client(&mock, "vertex_ai/gemini-2.0-flash");

    let resp = client.list_models().await.expect("a paged listing should parse");

    let paths: Vec<String> = mock.requests().into_iter().map(|r| r.path).collect();
    assert_eq!(
        paths,
        [
            "/v1beta1/publishers/google/models",
            "/v1beta1/publishers/google/models?pageToken=t%2B1",
            "/v1beta1/publishers/google/models?pageToken=t%2B2"
        ]
    );
    let ids: Vec<&str> = resp.data.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["gemini-a", "llama-a", "gemini-b"]);
}

#[tokio::test]
async fn list_models_that_never_ends_its_paging_is_an_error() {
    let mock = MockServer::start_with_responder(|_method, _path, _index| {
        (
            200,
            serde_json::json!({
                "publisherModels": [{"name": "publishers/google/models/m"}],
                "nextPageToken": "again"
            })
            .to_string(),
        )
    });
    let client = hinted_client(&mock, "vertex_ai/gemini-2.0-flash");

    let err = client
        .list_models()
        .await
        .expect_err("a listing that never ends is an error");

    assert!(err.to_string().contains("did not end"), "unexpected error: {err}");
}

fn bedrock_client(mock: &MockServer) -> DefaultClient {
    let config = ClientConfigBuilder::new("unused")
        .base_url(mock.url())
        .max_retries(0)
        .bedrock_credentials("AKIATESTDUMMY", "test-dummy-secret", None)
        .build();
    DefaultClient::new(config, Some("bedrock/amazon.rerank-v1:0")).expect("client creation should succeed")
}

#[tokio::test]
async fn bedrock_base_url_serves_rerank() {
    let mock = rerank_mock(
        serde_json::json!({"results": [{"index": 2, "relevanceScore": 0.9}, {"index": 0, "relevanceScore": 0.5}]})
            .to_string(),
    );
    let client = bedrock_client(&mock);

    let resp = client
        .rerank(rerank_request("bedrock/amazon.rerank-v1:0"))
        .await
        .expect("a Bedrock rerank reply should parse");

    let requests = mock.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/rerank");
    let sent: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
    assert_eq!(sent["queries"][0]["textQuery"]["text"], "q");
    let order: Vec<u32> = resp.results.iter().map(|r| r.index).collect();
    assert_eq!(order, vec![2, 0]);
}

#[tokio::test]
async fn bedrock_base_url_serves_list_models() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "GET".into(),
        path_prefix: "/models".into(),
        status: 200,
        body: serde_json::json!({
            "modelSummaries": [{"modelId": "amazon.rerank-v1:0", "providerName": "Amazon"}]
        })
        .to_string(),
    }]);
    let client = bedrock_client(&mock);

    let resp = client
        .list_models()
        .await
        .expect("a Bedrock model listing should parse");

    let requests = mock.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/models");
    assert_eq!(resp.data.len(), 1);
    assert_eq!(resp.data[0].id, "amazon.rerank-v1:0");
    assert_eq!(resp.data[0].owned_by, "Amazon");
}

fn rerank_json_mock(body: serde_json::Value) -> MockServer {
    rerank_mock(body.to_string())
}

/// The route a rerank request for `hint` reaches. NVIDIA NIM serves reranking at `/ranking`.
fn rerank_mock_for(hint: &str, body: serde_json::Value) -> MockServer {
    let path = if hint == "nvidia_nim" { "/ranking" } else { "/rerank" };
    MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: path.into(),
        status: 200,
        body: body.to_string(),
    }])
}

/// One reply for each provider whose rerank reply is not the `results` shape.
/// Every reply ranks document 2 first and document 0 second.
fn foreign_rerank_replies() -> Vec<(&'static str, &'static str, serde_json::Value)> {
    vec![
        (
            "fireworks_ai",
            "fireworks_ai/rerank-x",
            serde_json::json!({
                "object": "list",
                "data": [{"index": 2, "relevance_score": 0.9}, {"index": 0, "relevance_score": 0.5}],
                "model": "rerank-x",
                "usage": {"total_tokens": 3}
            }),
        ),
        (
            "nvidia_nim",
            "nvidia_nim/nv-rerankqa",
            serde_json::json!({"rankings": [{"index": 2, "logit": 0.2}, {"index": 0, "logit": -1.0}]}),
        ),
        (
            "huggingface",
            "huggingface/bge-reranker",
            serde_json::json!([{"index": 2, "score": 0.9}, {"index": 0, "score": 0.5}]),
        ),
    ]
}

#[tokio::test]
async fn rerank_reply_of_fireworks_nim_and_tei_parses_into_results() {
    for (hint, model, reply) in foreign_rerank_replies() {
        let mock = rerank_mock_for(hint, reply);
        let client = hinted_client(&mock, hint);

        let resp = client
            .rerank(rerank_request(model))
            .await
            .unwrap_or_else(|e| panic!("{model}: the reply should parse: {e}"));

        let order: Vec<u32> = resp.results.iter().map(|r| r.index).collect();
        assert_eq!(order, vec![2, 0], "{model}");
    }
}

#[tokio::test]
async fn embed_reply_with_an_empty_model_keeps_it_for_an_openai_compatible_provider() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/embeddings".into(),
        status: 200,
        body: embedding_reply_without_a_model(),
    }]);
    let client = build_client(&mock);

    let resp = client.embed(embedding_request()).await.expect("embed should succeed");

    assert_eq!(resp.model, "");
}

#[tokio::test]
async fn embed_raw_reply_with_an_empty_model_keeps_it_for_an_openai_compatible_provider() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/embeddings".into(),
        status: 200,
        body: embedding_reply_without_a_model(),
    }]);
    let client = build_client(&mock);

    let resp = client
        .embed_raw(embedding_request())
        .await
        .expect("embed_raw should succeed");

    assert_eq!(resp.data.model, "");
}

#[tokio::test]
async fn rerank_raw_reply_of_fireworks_nim_and_tei_parses_and_keeps_the_raw_reply() {
    for (hint, model, reply) in foreign_rerank_replies() {
        let mock = rerank_mock_for(hint, reply.clone());
        let client = hinted_client(&mock, hint);

        let exchange = client
            .rerank_raw(rerank_request(model))
            .await
            .unwrap_or_else(|e| panic!("{model}: the reply should parse: {e}"));

        assert_eq!(exchange.data.results.len(), 2, "{model}");
        assert_eq!(exchange.raw_response, Some(reply), "{model}");
    }
}

#[tokio::test]
async fn rerank_nim_logit_becomes_the_relevance_score() {
    let (hint, model, reply) = foreign_rerank_replies().remove(1);
    let mock = rerank_mock_for(hint, reply);
    let client = hinted_client(&mock, hint);

    let resp = client.rerank(rerank_request(model)).await.expect("NIM reply parses");

    assert_eq!(resp.results[0].relevance_score, 0.2);
    assert_eq!(resp.results[1].relevance_score, -1.0);
}

fn top_n_request(model: &str) -> RerankRequest {
    RerankRequest {
        top_n: Some(1),
        ..rerank_request(model)
    }
}

fn first_request_body(mock: &MockServer) -> serde_json::Value {
    let requests = mock.requests();
    serde_json::from_str(&requests[0].body).expect("request body is JSON")
}

#[tokio::test]
async fn rerank_voyage_request_sends_top_k_when_the_model_names_voyage() {
    let mock = rerank_json_mock(serde_json::json!({"data": [{"index": 1, "relevance_score": 0.9}]}));
    let client = build_client(&mock);

    client
        .rerank(top_n_request("voyage/rerank-2"))
        .await
        .expect("reply parses");

    let body = first_request_body(&mock);
    assert_eq!(body["top_k"], 1);
    assert!(body.get("top_n").is_none(), "Voyage does not document top_n: {body}");
    assert_eq!(body["query"], "q", "the other fields stay");
}

#[tokio::test]
async fn rerank_voyage_request_sends_top_k_when_the_client_names_voyage_as_hint() {
    let mock = rerank_json_mock(serde_json::json!({"data": [{"index": 1, "relevance_score": 0.9}]}));
    let client = hinted_client(&mock, "voyage");

    client.rerank(top_n_request("rerank-2")).await.expect("reply parses");

    let body = first_request_body(&mock);
    assert_eq!(body["top_k"], 1);
    assert!(body.get("top_n").is_none(), "Voyage does not document top_n: {body}");
}

#[tokio::test]
async fn rerank_raw_voyage_request_sends_top_k() {
    let mock = rerank_json_mock(serde_json::json!({"data": [{"index": 1, "relevance_score": 0.9}]}));
    let client = hinted_client(&mock, "voyage");

    let exchange = client
        .rerank_raw(top_n_request("voyage/rerank-2"))
        .await
        .expect("reply parses");

    assert_eq!(exchange.raw_request["top_k"], 1);
    assert!(exchange.raw_request.get("top_n").is_none());
    let body = first_request_body(&mock);
    assert_eq!(body["top_k"], 1);
    assert!(body.get("top_n").is_none());
}

#[tokio::test]
async fn rerank_request_to_cohere_keeps_top_n() {
    let mock = rerank_json_mock(serde_json::json!({"results": [{"index": 1, "relevance_score": 0.9}]}));
    let client = hinted_client(&mock, "cohere");

    client
        .rerank(top_n_request("cohere/rerank-v3.5"))
        .await
        .expect("reply parses");

    let body = first_request_body(&mock);
    assert_eq!(body["top_n"], 1);
    assert!(body.get("top_k").is_none());
}

fn object_document_request(model: &str) -> RerankRequest {
    RerankRequest {
        model: model.to_owned(),
        query: "q".to_owned(),
        documents: vec![
            RerankDocument::Text("a".to_owned()),
            RerankDocument::Object { text: "b".to_owned() },
        ],
        top_n: Some(1),
        return_documents: Some(true),
        extra_body: None,
    }
}

#[tokio::test]
async fn rerank_nim_request_goes_to_ranking_with_query_and_passages() {
    let mock = rerank_mock_for(
        "nvidia_nim",
        serde_json::json!({"rankings": [{"index": 1, "logit": 0.5}]}),
    );
    let client = hinted_client(&mock, "nvidia_nim");

    client
        .rerank(object_document_request("nvidia_nim/nv-rerankqa"))
        .await
        .expect("reply parses");

    let requests = mock.requests();
    assert_eq!(requests[0].path, "/ranking");
    assert_eq!(
        first_request_body(&mock),
        serde_json::json!({
            "model": "nv-rerankqa",
            "query": {"text": "q"},
            "passages": [{"text": "a"}, {"text": "b"}]
        })
    );
}

#[tokio::test]
async fn rerank_nim_request_follows_the_model_prefix_of_a_client_with_no_hint() {
    let mock = rerank_mock_for(
        "nvidia_nim",
        serde_json::json!({"rankings": [{"index": 1, "logit": 0.5}]}),
    );
    let client = build_client(&mock);

    client
        .rerank(object_document_request("nvidia_nim/nv-rerankqa"))
        .await
        .expect("reply parses");

    assert_eq!(mock.requests()[0].path, "/ranking");
    assert_eq!(
        first_request_body(&mock)["passages"][1],
        serde_json::json!({"text": "b"})
    );
}

#[tokio::test]
async fn rerank_nim_request_follows_a_client_that_names_nim_as_hint() {
    let mock = rerank_mock_for(
        "nvidia_nim",
        serde_json::json!({"rankings": [{"index": 1, "logit": 0.5}]}),
    );
    let client = hinted_client(&mock, "nvidia_nim");

    client
        .rerank(object_document_request("nv-rerankqa"))
        .await
        .expect("reply parses");

    assert_eq!(mock.requests()[0].path, "/ranking");
    assert_eq!(first_request_body(&mock)["query"], serde_json::json!({"text": "q"}));
}

#[tokio::test]
async fn rerank_tei_request_sends_texts_and_return_text() {
    let mock = rerank_json_mock(serde_json::json!([{"index": 1, "score": 0.9, "text": "b"}]));
    let client = hinted_client(&mock, "huggingface");

    let resp = client
        .rerank(object_document_request("huggingface/bge-reranker"))
        .await
        .expect("reply parses");

    assert_eq!(
        first_request_body(&mock),
        serde_json::json!({
            "model": "bge-reranker",
            "query": "q",
            "texts": ["a", "b"],
            "return_text": true
        })
    );
    assert_eq!(resp.results[0].document.as_ref().map(|d| d.text.as_str()), Some("b"));
}

// ── Hinted client at a base_url: key and chat reply ──────────────────────────

/// A key in the environment that no request may carry.
const KEY_FROM_ENV: &str = "key-from-the-environment";

/// Key variables of providers that a hint at a base_url wraps.
const WRAPPED_KEY_VARS: &[&str] = &["COHERE_API_KEY", "MISTRAL_API_KEY", "GROQ_API_KEY"];

/// Key variables of the providers that keep their own type at a base_url.
const NATIVE_KEY_VARS: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "AZURE_OPENAI_API_KEY",
    "GEMINI_API_KEY",
    "AWS_BEARER_TOKEN_BEDROCK",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
];

/// Sets or clears key variables; restores them on drop.
struct KeyVarGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl KeyVarGuard {
    fn set(vars: &[&'static str], value: Option<&str>) -> Self {
        let saved = vars.iter().map(|v| (*v, std::env::var(v).ok())).collect();
        for &var in vars {
            // SAFETY: every test that writes the environment is `#[serial]`.
            unsafe {
                match value {
                    Some(v) => std::env::set_var(var, v),
                    None => std::env::remove_var(var),
                }
            }
        }
        KeyVarGuard { saved }
    }
}

impl Drop for KeyVarGuard {
    fn drop(&mut self) {
        for (var, value) in &self.saved {
            // SAFETY: as above.
            unsafe {
                match value {
                    Some(v) => std::env::set_var(var, v),
                    None => std::env::remove_var(var),
                }
            }
        }
    }
}

fn keyless_hinted_client(mock: &MockServer, hint: &str) -> liter_llm::Result<DefaultClient> {
    let config = ClientConfigBuilder::new("").base_url(mock.url()).max_retries(0).build();
    DefaultClient::new(config, Some(hint))
}

fn chat_request(model: &str) -> liter_llm::types::ChatCompletionRequest {
    serde_json::from_value(serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "Hi"}]
    }))
    .expect("minimal chat request should deserialize")
}

/// Hints of providers that a base_url wraps, in the bare and the `provider/model` form.
const WRAPPED_HINTS: &[&str] = &[
    "cohere",
    "mistral",
    "groq",
    "cohere/command-r-plus",
    "mistral/mistral-large-latest",
    "groq/llama3-8b",
];

/// Revert line: forward `env_var` to the wrapped provider in `BaseUrlOverride`, and the
/// client reads `COHERE_API_KEY` for the `cohere/` hint and sends it to the `base_url` host.
#[tokio::test]
#[serial_test::serial]
async fn a_hint_at_a_base_url_sends_no_key_from_the_environment() {
    let _guard = KeyVarGuard::set(WRAPPED_KEY_VARS, Some(KEY_FROM_ENV));
    for hint in WRAPPED_HINTS {
        let mock = MockServer::start_with_routes(vec![]);
        let client = keyless_hinted_client(&mock, hint).expect("a client without a key builds");

        let _ = client.chat(chat_request("m")).await;

        let requests = mock.requests();
        assert_eq!(requests.len(), 1, "hint {hint}: the request reaches the base_url host");
        let carried: Vec<_> = requests[0]
            .headers
            .iter()
            .filter(|(_, value)| value.contains(KEY_FROM_ENV))
            .collect();
        assert!(
            carried.is_empty(),
            "hint {hint}: the key variable was sent: {carried:?}"
        );
    }
}

/// Revert line: as above. With the key variables unset, the client then fails to build
/// with "environment variable COHERE_API_KEY is not set".
#[test]
#[serial_test::serial]
fn a_hint_at_a_base_url_builds_without_a_key_variable() {
    let _guard = KeyVarGuard::set(WRAPPED_KEY_VARS, None);
    let mock = MockServer::start_with_routes(vec![]);
    for hint in WRAPPED_HINTS {
        if let Err(error) = keyless_hinted_client(&mock, hint) {
            panic!("hint {hint}: a client without a key fails to build: {error}");
        }
    }
}

/// Sends one chat request from a keyless client with `hint` at a base_url, with every
/// key variable of Anthropic, Azure, Gemini and Bedrock set. Returns the request.
async fn keyless_native_request(hint: &str) -> CapturedRequest {
    let _guard = KeyVarGuard::set(NATIVE_KEY_VARS, Some(KEY_FROM_ENV));
    let mock = MockServer::start_with_routes(vec![]);
    let client = keyless_hinted_client(&mock, hint).expect("a client without a key builds");

    let _ = client.chat(chat_request("m")).await;

    let mut requests = mock.requests();
    assert_eq!(requests.len(), 1, "hint {hint}: the request reaches the base_url host");
    requests.remove(0)
}

/// A bare or model-name hint at a base_url keeps its provider's path and sends no
/// key from the environment.
async fn assert_no_key_from_the_environment(hint: &str, path: &str) {
    let request = keyless_native_request(hint).await;

    let (route, _query) = request.path.split_once('?').unwrap_or((&request.path, ""));
    assert_eq!(route, path, "hint {hint}: the hinted provider builds the path");
    assert!(
        !request.path.contains(KEY_FROM_ENV),
        "hint {hint}: a key variable was sent in the URL"
    );
    let carried: Vec<_> = request
        .headers
        .iter()
        .filter(|(_, value)| value.contains(KEY_FROM_ENV))
        .collect();
    assert!(carried.is_empty(), "hint {hint}: a key variable was sent: {carried:?}");
    assert!(
        !request.body.contains(KEY_FROM_ENV),
        "hint {hint}: a key variable was sent in the body"
    );
}

/// Revert line: drop the base_url check in `reads_environment_keys`, and the client
/// reads `ANTHROPIC_API_KEY` and sends it to the base_url host.
#[tokio::test]
#[serial_test::serial]
async fn a_bare_anthropic_hint_at_a_base_url_sends_no_key_from_the_environment() {
    assert_no_key_from_the_environment("anthropic", "/messages").await;
}

/// Revert line: as above.
#[tokio::test]
#[serial_test::serial]
async fn a_claude_x_hint_at_a_base_url_sends_no_key_from_the_environment() {
    assert_no_key_from_the_environment("claude-x", "/messages").await;
}

/// Revert line: as above.
#[tokio::test]
#[serial_test::serial]
async fn a_claude_sonnet_4_hint_at_a_base_url_sends_no_key_from_the_environment() {
    assert_no_key_from_the_environment("claude-sonnet-4", "/messages").await;
}

/// Revert line: as above, and the client sends `AZURE_OPENAI_API_KEY`.
#[tokio::test]
#[serial_test::serial]
async fn a_bare_azure_hint_at_a_base_url_sends_no_key_from_the_environment() {
    assert_no_key_from_the_environment("azure", "/openai/deployments/m/chat/completions").await;
}

/// Revert line: as above, and the client sends `GEMINI_API_KEY`.
#[tokio::test]
#[serial_test::serial]
async fn a_bare_gemini_hint_at_a_base_url_sends_no_key_from_the_environment() {
    assert_no_key_from_the_environment("gemini", "/models/m:generateContent").await;
}

/// Revert line: as above, and the client sends `GEMINI_API_KEY`.
#[tokio::test]
#[serial_test::serial]
async fn a_bare_google_ai_hint_at_a_base_url_sends_no_key_from_the_environment() {
    assert_no_key_from_the_environment("google_ai", "/models/m:generateContent").await;
}

/// Revert line: drop the `with_environment_keys` call for Bedrock in `route_hint`,
/// and the client sends `AWS_BEARER_TOKEN_BEDROCK` to the base_url host.
#[tokio::test]
#[serial_test::serial]
async fn a_bare_bedrock_hint_at_a_base_url_sends_no_key_from_the_environment() {
    assert_no_key_from_the_environment("bedrock", "/model/m/converse").await;
}

/// Revert line: drop the base_url check in `reads_environment_keys`, and with the
/// variables unset the client fails to build with "environment variable ANTHROPIC_API_KEY
/// is not set".
#[test]
#[serial_test::serial]
fn a_bare_or_model_name_hint_at_a_base_url_builds_without_a_key_variable() {
    let _guard = KeyVarGuard::set(NATIVE_KEY_VARS, None);
    let mock = MockServer::start_with_routes(vec![]);
    for hint in [
        "anthropic",
        "claude-x",
        "claude-sonnet-4",
        "azure",
        "gemini",
        "google_ai",
        "bedrock",
    ] {
        if let Err(error) = keyless_hinted_client(&mock, hint) {
            panic!("hint {hint}: a client without a key fails to build: {error}");
        }
    }
}

/// Revert line: return `None` from `AnthropicProvider::at_base_url`, so that the wrapper
/// carries Anthropic to the base_url, and `ANTHROPIC_API_KEY` is no longer read.
#[tokio::test]
#[serial_test::serial]
async fn a_provider_prefix_hint_at_a_base_url_reads_its_key_variable() {
    let request = keyless_native_request("anthropic/claude-x").await;

    assert_eq!(request.path, "/messages");
    let key = request
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("x-api-key"))
        .map(|(_, value)| value.as_str());
    assert_eq!(key, Some(KEY_FROM_ENV), "the key variable is the client's key");
}

/// Revert line: return `None` from `GoogleAiProvider::at_base_url`, so that the wrapper
/// carries Google AI to the base_url, and `GEMINI_API_KEY` is no longer read.
#[tokio::test]
#[serial_test::serial]
async fn a_gemini_prefix_hint_at_a_base_url_reads_its_key_variable() {
    let request = keyless_native_request("gemini/gemini-2.0-flash").await;

    let (route, _query) = request.path.split_once('?').unwrap_or((&request.path, ""));
    assert_eq!(route, "/models/m:generateContent");
    let key = request
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("x-goog-api-key"))
        .map(|(_, value)| value.as_str());
    assert_eq!(key, Some(KEY_FROM_ENV), "the key variable is the client's key");
}

/// Revert line: drop the `transform_response` forward in `BaseUrlOverride`, and Cohere's
/// native chat reply is read as an OpenAI reply, which has no `choices`.
#[tokio::test]
async fn a_cohere_chat_reply_at_a_base_url_is_rebuilt() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/chat".into(),
        status: 200,
        body: serde_json::json!({
            "id": "c1",
            "finish_reason": "COMPLETE",
            "message": {"role": "assistant", "content": [{"type": "text", "text": "hello"}]},
            "usage": {"billed_units": {"input_tokens": 3, "output_tokens": 2}}
        })
        .to_string(),
    }]);
    let client = hinted_client(&mock, "cohere");

    let resp = client
        .chat(chat_request("command-r-plus"))
        .await
        .expect("a native Cohere reply parses");

    assert_eq!(mock.requests()[0].path, "/chat");
    let reply = serde_json::to_value(&resp).expect("reply serializes");
    assert_eq!(reply["id"], "c1");
    assert_eq!(reply["choices"][0]["message"]["content"], "hello");
    assert_eq!(reply["choices"][0]["finish_reason"], "stop");
    assert_eq!(reply["usage"]["total_tokens"], 5);
}

/// Revert line: drop the `build_url` of `BaseUrlOverride`, and the Cohere model list at a
/// base_url goes to `/models` with no page size.
#[tokio::test]
async fn a_cohere_model_list_at_a_base_url_keeps_its_path_and_page_size() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "GET".into(),
        path_prefix: "/v1/models".into(),
        status: 200,
        body: serde_json::json!({"models": []}).to_string(),
    }]);
    let client = hinted_client(&mock, "cohere");

    let _ = client.list_models().await;

    let requests = mock.requests();
    assert_eq!(requests.len(), 1, "one request reaches the base_url host");
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/v1/models?page_size=1000");
}

/// Revert line: answer `/embeddings` from `embeddings_path` in `BaseUrlOverride`, or drop
/// its `transform_embedding_request` forward, and Cohere's embed request at a base_url
/// loses its path or its body.
#[tokio::test]
async fn a_cohere_embed_at_a_base_url_keeps_its_path_and_body() {
    let mock = MockServer::start_with_routes(vec![MockRoute {
        method: "POST".into(),
        path_prefix: "/embed".into(),
        status: 200,
        body: serde_json::json!({
            "id": "e1",
            "embeddings": {"float": [[0.5, 0.25]]},
            "texts": ["alpha"],
            "meta": {"billed_units": {"input_tokens": 3}}
        })
        .to_string(),
    }]);
    let client = hinted_client(&mock, "cohere");

    let resp = client
        .embed(EmbeddingRequest {
            model: "embed-english-v3.0".into(),
            input: liter_llm::types::EmbeddingInput::Single("alpha".into()),
            ..Default::default()
        })
        .await
        .expect("a native Cohere embed reply parses");

    let requests = mock.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/embed");
    let sent: serde_json::Value = serde_json::from_str(&requests[0].body).expect("a JSON body");
    assert_eq!(sent["texts"], serde_json::json!(["alpha"]));
    assert!(sent.get("input").is_none(), "the OpenAI field is rebuilt: {sent}");
    assert_eq!(resp.model, "embed-english-v3.0");
    assert_eq!(resp.data[0].embedding, vec![0.5, 0.25]);
}
