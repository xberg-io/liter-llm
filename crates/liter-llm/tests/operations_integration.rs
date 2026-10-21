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
                let path = full_path.split('?').next().unwrap_or("").to_string();

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

                captured_writer.lock().unwrap().push(CapturedRequest {
                    method: method.clone(),
                    path: full_path.clone(),
                    headers,
                    body,
                });

                let response_body = routes
                    .iter()
                    .find(|r| r.method == method && path.starts_with(&r.path_prefix))
                    .map(|r| (r.status, r.body.clone()))
                    .unwrap_or((200, r#"{"ok": true}"#.to_string()));

                let (status, resp_body) = response_body;
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
