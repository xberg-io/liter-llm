//! A model for a registry provider that has no route is refused before a request is built.
//!
//! The spy is an HTTP proxy on a local port. The client reads `HTTPS_PROXY` and `HTTP_PROXY`
//! when it builds its HTTP client, so every connection that would leave the process lands on
//! the spy and is recorded. A refused call must record nothing. The control tests show the
//! spy records the calls that do leave the process.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use liter_llm::client::config::ClientConfigBuilder;
use liter_llm::client::{DefaultClient, LlmClient, LlmClientRaw, ResponseClient};
use liter_llm::error::LiterLlmError;
use liter_llm::{AuthHeaderFormat, CustomProviderConfig, register_custom_provider, unregister_custom_provider};
use serial_test::serial;

/// Records the request line of every connection and answers with a canned reply.
struct Spy {
    url: String,
    lines: Arc<Mutex<Vec<String>>>,
    _handle: thread::JoinHandle<()>,
}

impl Spy {
    fn start(reply: Option<&str>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind spy");
        let url = format!(
            "http://127.0.0.1:{}",
            listener.local_addr().expect("spy address").port()
        );
        let lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&lines);
        let reply = reply.map(str::to_owned);
        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
                let mut request_line = String::new();
                if reader.read_line(&mut request_line).is_err() {
                    continue;
                }
                let mut content_length = 0_usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
                        break;
                    }
                    if let Some((name, value)) = line.trim().split_once(':')
                        && name.trim().eq_ignore_ascii_case("content-length")
                    {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0_u8; content_length];
                let _ = std::io::Read::read_exact(&mut reader, &mut body);
                recorded.lock().expect("spy lock").push(request_line.trim().to_owned());
                let response = match &reply {
                    Some(body) => format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    ),
                    None => "HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
                };
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        Spy {
            url,
            lines,
            _handle: handle,
        }
    }

    fn seen(&self) -> Vec<String> {
        self.lines.lock().expect("spy lock").clone()
    }
}

const PROXY_VARS: [&str; 4] = ["HTTPS_PROXY", "HTTP_PROXY", "https_proxy", "http_proxy"];

/// Points the process proxy variables at a spy; restores them on drop.
struct ProxyGuard {
    spy: Spy,
    saved: Vec<(&'static str, Option<String>)>,
}

impl ProxyGuard {
    fn install() -> Self {
        let spy = Spy::start(None);
        let saved = PROXY_VARS.iter().map(|v| (*v, std::env::var(v).ok())).collect();
        for var in PROXY_VARS {
            // SAFETY: every test that reads the environment is `#[serial]`.
            unsafe { std::env::set_var(var, &spy.url) };
        }
        for var in ["NO_PROXY", "no_proxy", "ALL_PROXY", "all_proxy"] {
            // SAFETY: as above.
            unsafe { std::env::remove_var(var) };
        }
        ProxyGuard { spy, saved }
    }
}

impl Drop for ProxyGuard {
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

fn client(base_url: Option<&str>, hint: Option<&str>) -> liter_llm::Result<DefaultClient> {
    let mut builder = ClientConfigBuilder::new("key-for-another-vendor").max_retries(0);
    if let Some(url) = base_url {
        builder = builder.base_url(url);
    }
    DefaultClient::new(builder.build(), hint)
}

fn request<T: serde::de::DeserializeOwned>(json: serde_json::Value) -> T {
    serde_json::from_value(json).expect("request should deserialize")
}

type Outcome = (&'static str, liter_llm::Result<()>);

/// Calls every endpoint that takes a model, and reports each result.
async fn call_every_endpoint(client: &DefaultClient, model: &str) -> Vec<Outcome> {
    let messages = serde_json::json!([{"role": "user", "content": "hi"}]);
    vec![
        (
            "chat",
            client
                .chat(request(serde_json::json!({"model": model, "messages": messages})))
                .await
                .map(drop),
        ),
        (
            "chat_stream",
            client
                .chat_stream(request(serde_json::json!({"model": model, "messages": messages})))
                .await
                .map(drop),
        ),
        (
            "embed",
            client
                .embed(request(serde_json::json!({"model": model, "input": "hi"})))
                .await
                .map(drop),
        ),
        (
            "image_generate",
            client
                .image_generate(request(serde_json::json!({"model": model, "prompt": "a cat"})))
                .await
                .map(drop),
        ),
        (
            "speech",
            client
                .speech(request(
                    serde_json::json!({"model": model, "input": "hi", "voice": "alloy"}),
                ))
                .await
                .map(drop),
        ),
        (
            "transcribe",
            client
                .transcribe(request(serde_json::json!({"model": model, "file": "AAAA"})))
                .await
                .map(drop),
        ),
        (
            "moderate",
            client
                .moderate(request(serde_json::json!({"model": model, "input": "hi"})))
                .await
                .map(drop),
        ),
        (
            "rerank",
            client
                .rerank(request(
                    serde_json::json!({"model": model, "query": "q", "documents": ["a", "b"]}),
                ))
                .await
                .map(drop),
        ),
        (
            "search",
            client
                .search(request(serde_json::json!({"model": model, "query": "q"})))
                .await
                .map(drop),
        ),
        (
            "ocr",
            client
                .ocr(request(serde_json::json!({
                    "model": model,
                    "document": {"type": "document_url", "url": "https://example.com/a.pdf"}
                })))
                .await
                .map(drop),
        ),
        (
            "create_response",
            client
                .create_response(request(serde_json::json!({"model": model, "input": "hi"})))
                .await
                .map(drop),
        ),
        (
            "create_response_stream",
            client
                .create_response_stream(request(serde_json::json!({"model": model, "input": "hi"})))
                .await
                .map(drop),
        ),
    ]
}

fn assert_refused(outcomes: &[Outcome], provider: &str) {
    assert_eq!(outcomes.len(), 12, "every endpoint is called");
    for (endpoint, outcome) in outcomes {
        match outcome {
            Err(LiterLlmError::BadRequest { message, .. }) => {
                assert!(message.contains(&format!("`{provider}`")), "{endpoint}: {message}");
                assert!(message.contains("base_url"), "{endpoint}: {message}");
            }
            other => panic!("{endpoint} for {provider}: expected a refusal, got {other:?}"),
        }
    }
}

const CHAT_REPLY: &str = r#"{"id":"c1","object":"chat.completion","created":1,"model":"m","choices":[{"index":0,"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}]}"#;

#[tokio::test]
#[serial]
async fn control_openai_model_without_base_url_reaches_the_spy() {
    let proxy = ProxyGuard::install();
    let client = client(None, None).expect("client");
    let outcomes = call_every_endpoint(&client, "gpt-4o").await;
    assert!(outcomes.iter().any(|(e, _)| *e == "chat"));
    let seen = proxy.spy.seen();
    assert!(
        seen.iter().any(|l| l.contains("api.openai.com")),
        "the spy must see OpenAI traffic, saw {seen:?}"
    );
    assert!(
        outcomes
            .iter()
            .all(|(_, r)| !matches!(r, Err(LiterLlmError::BadRequest { message, .. }) if message.contains("no built-in route"))),
        "an OpenAI model is never refused"
    );
}

#[tokio::test]
#[serial]
async fn control_routed_providers_without_base_url_reach_their_own_hosts() {
    let proxy = ProxyGuard::install();
    let client = client(None, None).expect("client");
    for model in ["claude-3-5-sonnet-20241022", "ollama/llama3"] {
        let outcomes = call_every_endpoint(&client, model).await;
        assert!(
            outcomes
                .iter()
                .any(|(_, r)| !matches!(r, Err(LiterLlmError::BadRequest { message, .. }) if message.contains("no built-in route"))),
            "{model} is never refused"
        );
    }
    let seen = proxy.spy.seen();
    assert!(seen.iter().any(|l| l.contains("api.anthropic.com")), "saw {seen:?}");
    assert!(seen.iter().any(|l| l.contains("localhost:11434")), "saw {seen:?}");
}

#[tokio::test]
#[serial]
async fn provider_without_route_and_without_base_url_sends_nothing() {
    let proxy = ProxyGuard::install();
    let client = client(None, None).expect("client");
    for provider in [
        "snowflake",
        "watsonx",
        "gigachat",
        "oci",
        "databricks",
        "amazon_nova",
        "sagemaker",
    ] {
        let outcomes = call_every_endpoint(&client, &format!("{provider}/some-model")).await;
        assert_refused(&outcomes, provider);
    }
    assert_eq!(
        proxy.spy.seen(),
        Vec::<String>::new(),
        "no request may leave the process"
    );
}

#[tokio::test]
#[serial]
async fn gateway_without_base_url_is_refused() {
    let proxy = ProxyGuard::install();
    let client = client(None, None).expect("client");
    for provider in [
        "custom",
        "custom_openai",
        "hosted_vllm",
        "infinity",
        "litellm_proxy",
        "openai_like",
    ] {
        let outcomes = call_every_endpoint(&client, &format!("{provider}/some-model")).await;
        assert_refused(&outcomes, provider);
    }
    assert_eq!(
        proxy.spy.seen(),
        Vec::<String>::new(),
        "no request may leave the process"
    );
}

#[tokio::test]
#[serial]
async fn client_hinted_at_a_provider_without_route_is_refused_at_construction() {
    let proxy = ProxyGuard::install();
    match client(None, Some("snowflake/arctic")) {
        Err(LiterLlmError::BadRequest { message, .. }) => {
            assert!(
                message.contains("`snowflake`") && message.contains("base_url"),
                "{message}"
            );
        }
        Err(other) => panic!("expected BadRequest, got {other:?}"),
        Ok(_) => panic!("expected a refusal"),
    }
    assert!(
        client(None, Some("ollama/llama3")).is_ok(),
        "a routed hint still builds"
    );
    assert!(proxy.spy.seen().is_empty());
}

/// Audio, image and search providers whose native APIs are not OpenAI-shaped.
const NATIVE_API_PROVIDERS: [&str; 8] = [
    "elevenlabs",
    "deepgram",
    "stability",
    "tavily",
    "fal_ai",
    "runwayml",
    "assemblyai",
    "serper",
];

#[tokio::test]
#[serial]
async fn native_api_provider_without_base_url_sends_nothing_on_any_endpoint() {
    let proxy = ProxyGuard::install();
    let client = client(None, None).expect("client");
    for provider in NATIVE_API_PROVIDERS {
        let outcomes = call_every_endpoint(&client, &format!("{provider}/some-model")).await;
        assert_refused(&outcomes, provider);
    }
    assert_eq!(
        proxy.spy.seen(),
        Vec::<String>::new(),
        "no request may leave the process"
    );
}

#[tokio::test]
#[serial]
async fn an_empty_base_url_counts_as_unset() {
    let proxy = ProxyGuard::install();
    let empty = client(Some(""), None).expect("client");
    for provider in NATIVE_API_PROVIDERS {
        let outcomes = call_every_endpoint(&empty, &format!("{provider}/some-model")).await;
        assert_refused(&outcomes, provider);
    }
    assert!(proxy.spy.seen().is_empty());
    match client(Some(""), Some("deepgram/some-model")) {
        Err(LiterLlmError::BadRequest { message, .. }) => assert!(message.contains("`deepgram`"), "{message}"),
        other => panic!("expected a refusal, got {:?}", other.map(drop)),
    }
}

#[tokio::test]
#[serial]
async fn brave_chat_without_a_base_url_is_not_refused() {
    let proxy = ProxyGuard::install();
    let client = client(None, None).expect("client");
    let outcome = client
        .chat(request(serde_json::json!({
            "model": "brave/brave",
            "messages": [{"role": "user", "content": "hi"}]
        })))
        .await;
    if let Err(LiterLlmError::BadRequest { message, .. }) = &outcome {
        assert!(!message.contains("no built-in route"), "{message}");
    }
    assert!(!proxy.spy.seen().is_empty(), "the chat request reaches the network");
}

#[tokio::test]
#[serial]
async fn native_api_provider_with_a_base_url_still_reaches_it() {
    let target = Spy::start(Some(CHAT_REPLY));
    for provider in NATIVE_API_PROVIDERS {
        let before = target.seen().len();
        let client = client(Some(&target.url), None).expect("client");
        let chat = client
            .chat(request(serde_json::json!({
                "model": format!("{provider}/some-model"),
                "messages": [{"role": "user", "content": "hi"}]
            })))
            .await;
        assert!(chat.is_ok(), "{provider}: {chat:?}");
        assert_eq!(
            target.seen().len(),
            before + 1,
            "{provider}: one request to the base_url"
        );
    }
}

#[tokio::test]
#[serial]
async fn client_hinted_at_a_native_api_provider_is_refused_at_construction() {
    for provider in NATIVE_API_PROVIDERS {
        match client(None, Some(&format!("{provider}/some-model"))) {
            Err(LiterLlmError::BadRequest { message, .. }) => {
                assert!(message.contains(&format!("`{provider}`")), "{message}");
            }
            other => panic!("{provider}: expected a refusal, got {:?}", other.map(drop)),
        }
    }
}

#[tokio::test]
#[serial]
async fn provider_without_route_works_with_a_base_url() {
    let target = Spy::start(Some(CHAT_REPLY));
    for provider in ["snowflake", "watsonx", "hosted_vllm", "litellm_proxy"] {
        let before = target.seen().len();
        let model = format!("{provider}/some-model");
        let client = client(Some(&target.url), None).expect("client");
        let chat = client
            .chat(request(
                serde_json::json!({"model": model, "messages": [{"role": "user", "content": "hi"}]}),
            ))
            .await;
        assert!(chat.is_ok(), "{provider}: {chat:?}");
        assert_eq!(
            target.seen().len(),
            before + 1,
            "{provider}: one request to the base_url"
        );
    }
    assert!(
        target.seen().iter().all(|l| l.starts_with("POST ")),
        "saw {:?}",
        target.seen()
    );
    // A client hinted at the same provider builds too once a base_url is set.
    assert!(client(Some(&target.url), Some("snowflake/arctic")).is_ok());
}

#[tokio::test]
#[serial]
async fn client_with_a_bare_hint_for_a_provider_without_route_is_refused() {
    let proxy = ProxyGuard::install();
    for hint in ["snowflake", "watsonx", "hosted_vllm"] {
        match client(None, Some(hint)) {
            Err(LiterLlmError::BadRequest { message, status }) => {
                assert_eq!(status, 400, "{hint}");
                assert!(
                    message.contains(&format!("`{hint}`")) && message.contains("base_url"),
                    "{hint}: {message}"
                );
            }
            Err(other) => panic!("{hint}: expected BadRequest, got {other:?}"),
            Ok(_) => panic!("{hint}: expected a refusal"),
        }
    }
    assert!(
        proxy.spy.seen().is_empty(),
        "no request may leave the process, saw {:?}",
        proxy.spy.seen()
    );
}

#[tokio::test]
#[serial]
async fn client_with_a_bare_hint_that_has_a_route_or_a_base_url_still_builds() {
    let target = Spy::start(Some(CHAT_REPLY));
    for hint in ["anthropic", "groq", "ollama", "gpt-4o", "some-local-model"] {
        assert!(client(None, Some(hint)).is_ok(), "{hint} builds without a base_url");
    }
    let client = client(Some(&target.url), Some("snowflake")).expect("a base_url lifts the refusal");
    let chat = client
        .chat(request(
            serde_json::json!({"model": "m", "messages": [{"role": "user", "content": "hi"}]}),
        ))
        .await;
    assert!(chat.is_ok(), "{chat:?}");
    assert_eq!(target.seen().len(), 1, "one request to the base_url");
}

#[tokio::test]
#[serial]
async fn responses_for_a_provider_without_route_reach_the_base_url() {
    let target = Spy::start(None);
    let client = client(Some(&target.url), None).expect("client");
    let created = client
        .create_response(request(serde_json::json!({"model": "snowflake/m", "input": "hi"})))
        .await;
    assert!(
        !matches!(&created, Err(LiterLlmError::BadRequest { message, .. }) if message.contains("no built-in route")),
        "a base_url lifts the refusal: {created:?}"
    );
    let streamed = client
        .create_response_stream(request(serde_json::json!({"model": "snowflake/m", "input": "hi"})))
        .await
        .map(drop);
    assert!(
        !matches!(&streamed, Err(LiterLlmError::BadRequest { message, .. }) if message.contains("no built-in route")),
        "a base_url lifts the refusal: {streamed:?}"
    );
    let seen = target.seen();
    assert_eq!(seen.len(), 2, "create and stream each reach the base_url, saw {seen:?}");
    assert!(seen.iter().all(|l| l.starts_with("POST ")), "saw {seen:?}");
}

/// Providers whose chat endpoint is routed but whose search endpoint is a native API.
const NO_SEARCH_ROUTE: [&str; 8] = [
    "brave",
    "perplexity",
    "exa_ai",
    "firecrawl",
    "linkup",
    "parallel_ai",
    "google_pse",
    "duckduckgo",
];

fn assert_endpoint_refused(outcome: &liter_llm::Result<()>, provider: &str, endpoint: &str, call: &str) {
    match outcome {
        Err(LiterLlmError::BadRequest { message, status }) => {
            assert_eq!(*status, 400, "{call} for {provider}");
            assert_eq!(
                message,
                &format!("provider `{provider}` has no built-in route for {endpoint}; set base_url to its endpoint"),
                "{call} for {provider}"
            );
        }
        other => panic!("{call} for {provider}: expected a refusal, got {other:?}"),
    }
}

async fn search_calls(client: &DefaultClient, model: &str) -> Vec<(&'static str, liter_llm::Result<()>)> {
    let req = serde_json::json!({"model": model, "query": "q"});
    vec![
        ("search", client.search(request(req.clone())).await.map(drop)),
        ("search_raw", client.search_raw(request(req)).await.map(drop)),
    ]
}

async fn audio_calls(client: &DefaultClient, model: &str) -> Vec<(&'static str, liter_llm::Result<()>)> {
    let speech = serde_json::json!({"model": model, "input": "hi", "voice": "alloy"});
    let transcription = serde_json::json!({"model": model, "file": "AAAA"});
    vec![
        ("speech", client.speech(request(speech)).await.map(drop)),
        (
            "transcribe",
            client.transcribe(request(transcription.clone())).await.map(drop),
        ),
        (
            "transcribe_raw",
            client.transcribe_raw(request(transcription)).await.map(drop),
        ),
    ]
}

async fn image_calls(client: &DefaultClient, model: &str) -> Vec<(&'static str, liter_llm::Result<()>)> {
    let req = serde_json::json!({"model": model, "prompt": "a cat"});
    vec![
        (
            "image_generate",
            client.image_generate(request(req.clone())).await.map(drop),
        ),
        (
            "image_generate_raw",
            client.image_generate_raw(request(req)).await.map(drop),
        ),
    ]
}

#[tokio::test]
#[serial]
async fn search_without_a_route_sends_nothing() {
    let proxy = ProxyGuard::install();
    let client = client(None, None).expect("client");
    for provider in NO_SEARCH_ROUTE {
        for (call, outcome) in search_calls(&client, &format!("{provider}/some-model")).await {
            assert_endpoint_refused(&outcome, provider, "search", call);
        }
    }
    assert_eq!(
        proxy.spy.seen(),
        Vec::<String>::new(),
        "no request may leave the process"
    );
}

#[tokio::test]
#[serial]
async fn audio_without_a_route_sends_nothing() {
    let proxy = ProxyGuard::install();
    let client = client(None, None).expect("client");
    for (call, outcome) in audio_calls(&client, "edenai/some-model").await {
        assert_endpoint_refused(&outcome, "edenai", "audio", call);
    }
    assert_eq!(
        proxy.spy.seen(),
        Vec::<String>::new(),
        "no request may leave the process"
    );
}

#[tokio::test]
#[serial]
async fn image_without_a_route_sends_nothing() {
    let proxy = ProxyGuard::install();
    let client = client(None, None).expect("client");
    for (call, outcome) in image_calls(&client, "edenai/some-model").await {
        assert_endpoint_refused(&outcome, "edenai", "image", call);
    }
    assert_eq!(
        proxy.spy.seen(),
        Vec::<String>::new(),
        "no request may leave the process"
    );
}

#[tokio::test]
#[serial]
async fn a_client_pinned_to_the_provider_refuses_the_endpoint_for_a_bare_model() {
    let proxy = ProxyGuard::install();
    let client = client(None, Some("brave/brave")).expect("client");
    for (call, outcome) in search_calls(&client, "web-search").await {
        assert_endpoint_refused(&outcome, "brave", "search", call);
    }
    assert!(proxy.spy.seen().is_empty());
}

#[tokio::test]
#[serial]
async fn the_chat_endpoint_of_a_provider_with_a_refused_endpoint_still_sends() {
    let proxy = ProxyGuard::install();
    let client = client(None, None).expect("client");
    let mut expected = 0;
    for (provider, host) in [
        ("brave", "api.search.brave.com"),
        ("perplexity", "api.perplexity.ai"),
        ("edenai", "api.edenai.run"),
    ] {
        let chat = client
            .chat(request(serde_json::json!({
                "model": format!("{provider}/some-model"),
                "messages": [{"role": "user", "content": "hi"}]
            })))
            .await;
        assert!(
            !matches!(&chat, Err(LiterLlmError::BadRequest { message, .. }) if message.contains("no built-in route")),
            "{provider}: chat is not refused: {chat:?}"
        );
        expected += 1;
        let seen = proxy.spy.seen();
        assert_eq!(
            seen.len(),
            expected,
            "{provider}: the chat request reaches the network, saw {seen:?}"
        );
        assert!(seen[expected - 1].contains(host), "{provider}: saw {seen:?}");
    }
}

#[tokio::test]
#[serial]
async fn the_embedding_endpoint_of_edenai_still_sends() {
    let proxy = ProxyGuard::install();
    let client = client(None, None).expect("client");
    let embedded = client
        .embed(request(
            serde_json::json!({"model": "edenai/some-model", "input": "hi"}),
        ))
        .await;
    assert!(
        !matches!(&embedded, Err(LiterLlmError::BadRequest { message, .. }) if message.contains("no built-in route")),
        "{embedded:?}"
    );
    assert_eq!(proxy.spy.seen().len(), 1, "the embedding request reaches the network");
}

#[tokio::test]
#[serial]
async fn a_base_url_lifts_the_endpoint_refusal() {
    let target = Spy::start(Some("{}"));
    let client = client(Some(&target.url), None).expect("client");
    let mut outcomes = search_calls(&client, "brave/some-model").await;
    outcomes.extend(audio_calls(&client, "edenai/some-model").await);
    outcomes.extend(image_calls(&client, "edenai/some-model").await);
    for (call, outcome) in &outcomes {
        assert!(
            !matches!(outcome, Err(LiterLlmError::BadRequest { message, .. }) if message.contains("no built-in route")),
            "{call}: a base_url lifts the refusal: {outcome:?}"
        );
    }
    assert_eq!(target.seen().len(), outcomes.len(), "every call reaches the base_url");
}

/// Registers a custom provider and removes it on drop, so a failed assertion cannot leak it.
struct CustomGuard(&'static str);

impl CustomGuard {
    fn register(name: &'static str, base_url: &str) -> Self {
        register_custom_provider(CustomProviderConfig {
            name: name.into(),
            base_url: base_url.to_owned(),
            auth_header: AuthHeaderFormat::Bearer,
            model_prefixes: vec![format!("{name}/")],
        })
        .expect("registration should succeed");
        CustomGuard(name)
    }
}

impl Drop for CustomGuard {
    fn drop(&mut self) {
        let _ = unregister_custom_provider(self.0);
    }
}

/// A custom provider registered under the name of a registry provider sends the call to its
/// own `base_url`, and the registry provider of the same name is still refused.
async fn assert_custom_provider_not_refused(
    name: &'static str,
    endpoint: &str,
    calls: for<'a> fn(&'a DefaultClient, &'a str) -> Calls<'a>,
) {
    let target = Spy::start(Some("{}"));
    let model = format!("{name}/some-model");
    {
        let _custom = CustomGuard::register(name, &target.url);
        let client = client(None, None).expect("client");
        let outcomes = calls(&client, &model).await;
        for (call, outcome) in &outcomes {
            assert!(
                !matches!(outcome, Err(LiterLlmError::BadRequest { message, .. }) if message.contains("no built-in route")),
                "{name} {call}: a custom provider is never refused: {outcome:?}"
            );
        }
        assert_eq!(
            target.seen().len(),
            outcomes.len(),
            "{name}: every {endpoint} call reaches the custom base_url"
        );
    }
    let proxy = ProxyGuard::install();
    let client = client(None, None).expect("client");
    for (call, outcome) in calls(&client, &model).await {
        assert_endpoint_refused(&outcome, name, endpoint, call);
    }
    assert!(proxy.spy.seen().is_empty());
}

type Calls<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = Vec<(&'static str, liter_llm::Result<()>)>> + 'a>>;

fn search_of<'a>(client: &'a DefaultClient, model: &'a str) -> Calls<'a> {
    Box::pin(search_calls(client, model))
}

fn audio_of<'a>(client: &'a DefaultClient, model: &'a str) -> Calls<'a> {
    Box::pin(audio_calls(client, model))
}

fn image_of<'a>(client: &'a DefaultClient, model: &'a str) -> Calls<'a> {
    Box::pin(image_calls(client, model))
}

#[tokio::test]
#[serial]
async fn a_custom_provider_named_like_a_search_provider_sends_search_to_its_base_url() {
    for name in ["brave", "perplexity", "exa_ai"] {
        assert_custom_provider_not_refused(name, "search", search_of).await;
    }
}

#[tokio::test]
#[serial]
async fn a_custom_provider_named_edenai_sends_image_and_speech_to_its_base_url() {
    assert_custom_provider_not_refused("edenai", "image", image_of).await;
    assert_custom_provider_not_refused("edenai", "audio", audio_of).await;
}

fn assert_not_refused<T>(outcome: &liter_llm::Result<T>, provider: &str, call: &str) {
    let error = outcome.as_ref().err();
    assert!(
        !matches!(error, Some(LiterLlmError::BadRequest { message, .. }) if message.contains("no built-in route")),
        "{provider} {call}: not refused: {error:?}"
    );
}

/// Each chat and embedding entry point sends for a provider that refuses other endpoints, so a
/// call that passes the wrong endpoint kind to the check is caught on that exact entry point.
#[tokio::test]
#[serial]
async fn every_chat_and_embedding_entry_point_of_a_marked_provider_sends() {
    let proxy = ProxyGuard::install();
    let client = client(None, None).expect("client");
    let mut expected = 0;
    for provider in ["brave", "perplexity", "edenai"] {
        let model = format!("{provider}/some-model");
        let chat = serde_json::json!({"model": model, "messages": [{"role": "user", "content": "hi"}]});
        assert_not_refused(&client.chat(request(chat.clone())).await, provider, "chat");
        assert_not_refused(
            &client.chat_stream(request(chat.clone())).await,
            provider,
            "chat_stream",
        );
        assert_not_refused(&client.chat_raw(request(chat.clone())).await, provider, "chat_raw");
        assert_not_refused(
            &client.chat_stream_raw(request(chat)).await,
            provider,
            "chat_stream_raw",
        );
        expected += 4;
        let embedding = serde_json::json!({"model": model, "input": "hi"});
        assert_not_refused(&client.embed(request(embedding.clone())).await, provider, "embed");
        assert_not_refused(&client.embed_raw(request(embedding)).await, provider, "embed_raw");
        expected += 2;
        assert_eq!(
            proxy.spy.seen().len(),
            expected,
            "{provider}: every chat and embedding call reaches the network"
        );
    }
}
