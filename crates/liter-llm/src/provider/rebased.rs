use std::borrow::Cow;

use crate::error::Result;
use crate::provider::{Provider, StreamFormat, join_url};

/// A provider sent to another host.
///
/// Wraps a provider and answers `base_url` with the new host. Every other method asks the
/// wrapped provider, so the request body, the paths, the headers and the reply handling stay
/// those of that provider. [`Provider::build_url`] and [`Provider::build_stream_url`] take the
/// URL the wrapped provider builds and move it to the new host, see [`Self::rebase`].
///
/// Two methods are not forwarded:
/// - [`Provider::env_var`] answers `None`. The variable belongs to the wrapped provider's own
///   host, so a client with no `api_key` reads no key from it for the `base_url` host.
/// - [`Provider::unrouted_endpoints`] keeps its empty default. The wrapped provider lists the
///   endpoints it has no route for at its own host. A `base_url` names the host of those
///   endpoints, so no call is refused.
pub(crate) struct BaseUrlOverride {
    inner: Box<dyn Provider>,
    base_url: String,
}

impl BaseUrlOverride {
    pub(crate) fn new(inner: Box<dyn Provider>, base_url: &str) -> Self {
        Self {
            inner,
            base_url: base_url.trim_end_matches('/').to_owned(),
        }
    }

    /// `url`, built by the wrapped provider for `endpoint_path`, moved to `base_url`.
    ///
    /// The path and the query stay those the wrapped provider built:
    /// - A URL under the wrapped provider's own base URL keeps the part after that base URL,
    ///   and `base_url` takes the place of the base URL (`{base_url}/chat` for Cohere).
    /// - A URL outside it (Cohere lists models at `/v1/models`, beside its `/v2` base) keeps
    ///   its whole path, and takes the scheme, the host and the port of `base_url`.
    ///
    /// A query of `base_url` stays, before the query of `url`. When a URL does not parse,
    /// the result is `{base_url}{endpoint_path}`.
    fn rebase(&self, url: &str, endpoint_path: &str) -> String {
        let parsed = (
            url::Url::parse(url),
            url::Url::parse(self.inner.base_url()),
            url::Url::parse(&self.base_url),
        );
        let (Ok(url), Ok(own), Ok(mut target)) = parsed else {
            return join_url(&self.base_url, endpoint_path);
        };
        let own_path = own.path().trim_end_matches('/');
        let below_own_base = url
            .path()
            .strip_prefix(own_path)
            .filter(|rest| url.origin() == own.origin() && (rest.is_empty() || rest.starts_with('/')));
        let path = match below_own_base {
            Some(rest) => format!(
                "{}/{}",
                target.path().trim_end_matches('/'),
                rest.trim_start_matches('/')
            ),
            None => url.path().to_owned(),
        };
        target.set_path(&path);
        if let Some(query) = url.query() {
            let query = match target.query() {
                Some(own_query) => format!("{own_query}&{query}"),
                None => query.to_owned(),
            };
            target.set_query(Some(&query));
        }
        target.into()
    }
}

impl Provider for BaseUrlOverride {
    fn validate(&self) -> Result<()> {
        self.inner.validate()
    }

    /// No key variable: the variable belongs to the provider's own host, so a client that
    /// has no `api_key` sends no key read from it to the `base_url` host.
    fn env_var(&self) -> Option<&str> {
        None
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, endpoint_path: &str, model: &str) -> String {
        self.rebase(&self.inner.build_url(endpoint_path, model), endpoint_path)
    }

    fn build_stream_url(&self, endpoint_path: &str, model: &str) -> String {
        self.rebase(&self.inner.build_stream_url(endpoint_path, model), endpoint_path)
    }

    fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
        // A provider that needs no key at its own host (Ollama, LM Studio) sends the key as a
        // bearer token at a `base_url`, where a gateway can ask for it.
        self.inner
            .auth_header(api_key)
            .or_else(|| Some((Cow::Borrowed("Authorization"), Cow::Owned(format!("Bearer {api_key}")))))
    }

    fn extra_headers(&self) -> &'static [(&'static str, &'static str)] {
        self.inner.extra_headers()
    }

    fn dynamic_headers(&self, body: &serde_json::Value) -> Vec<(String, String)> {
        self.inner.dynamic_headers(body)
    }

    fn matches_model(&self, model: &str) -> bool {
        self.inner.matches_model(model)
    }

    fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
        self.inner.strip_model_prefix(model)
    }

    fn chat_completions_path(&self) -> &str {
        self.inner.chat_completions_path()
    }

    fn embeddings_path(&self) -> &str {
        self.inner.embeddings_path()
    }

    fn models_path(&self) -> &str {
        self.inner.models_path()
    }

    fn next_models_page_url(&self, url: &str, page: &serde_json::Value) -> Option<String> {
        self.inner.next_models_page_url(url, page)
    }

    fn image_generations_path(&self) -> &str {
        self.inner.image_generations_path()
    }

    fn audio_speech_path(&self) -> &str {
        self.inner.audio_speech_path()
    }

    fn audio_transcriptions_path(&self) -> &str {
        self.inner.audio_transcriptions_path()
    }

    fn moderations_path(&self) -> &str {
        self.inner.moderations_path()
    }

    fn rerank_path(&self) -> &str {
        self.inner.rerank_path()
    }

    fn files_path(&self) -> &str {
        self.inner.files_path()
    }

    fn batches_path(&self) -> &str {
        self.inner.batches_path()
    }

    fn responses_path(&self) -> &str {
        self.inner.responses_path()
    }

    fn search_path(&self) -> &str {
        self.inner.search_path()
    }

    fn ocr_path(&self) -> &str {
        self.inner.ocr_path()
    }

    fn supports_streaming(&self) -> bool {
        self.inner.supports_streaming()
    }

    fn transform_request(&self, body: &mut serde_json::Value) -> Result<()> {
        self.inner.transform_request(body)
    }

    fn transform_response(&self, body: &mut serde_json::Value) -> Result<()> {
        self.inner.transform_response(body)
    }

    fn transform_embedding_request(&self, body: &mut serde_json::Value) -> Result<()> {
        self.inner.transform_embedding_request(body)
    }

    fn transform_embedding_response(&self, body: &mut serde_json::Value, request_model: &str) -> Result<()> {
        self.inner.transform_embedding_response(body, request_model)
    }

    fn transform_error(&self, error: crate::error::LiterLlmError) -> crate::error::LiterLlmError {
        self.inner.transform_error(error)
    }

    fn transform_rerank_response(&self, body: &mut serde_json::Value) -> Result<()> {
        self.inner.transform_rerank_response(body)
    }

    fn transform_rerank_request(&self, body: &mut serde_json::Value) -> Result<()> {
        self.inner.transform_rerank_request(body)
    }

    fn parse_stream_event(&self, event_data: &str) -> Result<Option<crate::types::ChatCompletionChunk>> {
        self.inner.parse_stream_event(event_data)
    }

    fn stream_format(&self) -> StreamFormat {
        self.inner.stream_format()
    }

    fn stream_has_final_event(&self) -> bool {
        self.inner.stream_has_final_event()
    }

    fn stream_tool_call_indexes_restart(&self) -> bool {
        self.inner.stream_tool_call_indexes_restart()
    }

    fn signing_headers(&self, method: &str, url: &str, body: &[u8]) -> Result<Vec<(String, String)>> {
        self.inner.signing_headers(method, url, body)
    }

    fn signing_headers_with_credential(
        &self,
        method: &str,
        url: &str,
        body: &[u8],
        credential: Option<&crate::auth::Credential>,
    ) -> Result<Vec<(String, String)>> {
        self.inner
            .signing_headers_with_credential(method, url, body, credential)
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use serde_json::json;
    use serial_test::serial;

    use super::BaseUrlOverride;
    use crate::auth::Credential;
    use crate::error::{LiterLlmError, Result};
    use crate::provider::{Provider, StreamFormat, all_providers, on_base_url, route_builtin_hint, route_hint};

    /// A provider whose every path differs from the default paths.
    struct OwnPaths;

    impl Provider for OwnPaths {
        fn name(&self) -> &str {
            "own-paths"
        }

        fn base_url(&self) -> &str {
            "https://own.test/api"
        }

        fn auth_header<'a>(&'a self, _api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
            None
        }

        fn matches_model(&self, _model: &str) -> bool {
            false
        }

        fn chat_completions_path(&self) -> &str {
            "/own-chat"
        }

        fn embeddings_path(&self) -> &str {
            "/own-embed"
        }

        fn models_path(&self) -> &str {
            "/own-models"
        }
    }

    fn hinted_at(hint: &str, base_url: &str) -> Box<dyn Provider> {
        let provider = route_hint(hint)
            .expect("no refusal")
            .expect("the hint names a provider");
        on_base_url(provider, base_url)
    }

    /// Revert line: answer `/embeddings` or `/models` from `embeddings_path` or `models_path`
    /// in `BaseUrlOverride`, and the wrapped provider's own path is lost.
    #[test]
    fn the_wrapper_keeps_each_path_of_the_wrapped_provider() {
        let rebased = BaseUrlOverride::new(Box::new(OwnPaths), "http://gateway.test/v1/");
        let url = |path: &str| rebased.build_url(path, "m");
        assert_eq!(url(rebased.chat_completions_path()), "http://gateway.test/v1/own-chat");
        assert_eq!(url(rebased.embeddings_path()), "http://gateway.test/v1/own-embed");
        assert_eq!(url(rebased.models_path()), "http://gateway.test/v1/own-models");
    }

    /// Revert line: drop the `build_url` of `BaseUrlOverride`, so that the default builds
    /// `{base_url}{path}`, and the Cohere model list goes to `/v1/models` with no page size.
    #[test]
    fn the_cohere_model_list_keeps_its_path_and_query_at_a_base_url() {
        for (base_url, expected) in [
            ("http://gateway.test/v1", "http://gateway.test/v1/models?page_size=1000"),
            ("http://127.0.0.1:9", "http://127.0.0.1:9/v1/models?page_size=1000"),
            (
                "http://gateway.test/v1?tenant=a",
                "http://gateway.test/v1/models?tenant=a&page_size=1000",
            ),
        ] {
            let cohere = hinted_at("cohere", base_url);
            assert_eq!(cohere.build_url(cohere.models_path(), ""), expected, "{base_url}");
        }
    }

    #[test]
    fn cohere_chat_and_embed_take_the_place_of_its_base_url() {
        let cohere = hinted_at("cohere", "http://gateway.test/v1");
        let url = |path: &str| cohere.build_url(path, "m");
        assert_eq!(url(cohere.chat_completions_path()), "http://gateway.test/v1/chat");
        assert_eq!(url(cohere.embeddings_path()), "http://gateway.test/v1/embed");
        assert_eq!(url(cohere.rerank_path()), "http://gateway.test/v1/rerank");
        assert_eq!(
            cohere.build_stream_url(cohere.chat_completions_path(), "m"),
            "http://gateway.test/v1/chat"
        );
        let tenant = hinted_at("cohere", "http://gateway.test/v1?tenant=a");
        assert_eq!(
            tenant.build_url(tenant.chat_completions_path(), "m"),
            "http://gateway.test/v1/chat?tenant=a"
        );
    }

    /// A rerank endpoint on another host of the provider keeps its path at the `base_url` host.
    #[test]
    fn a_rerank_endpoint_on_another_host_keeps_its_path_at_a_base_url() {
        let dashscope = hinted_at("dashscope", "http://gateway.test/v1");
        assert_eq!(
            dashscope.build_url(dashscope.rerank_path(), "m"),
            "http://gateway.test/compatible-api/v1/reranks"
        );
        assert_eq!(
            dashscope.build_url(dashscope.chat_completions_path(), "m"),
            "http://gateway.test/v1/chat/completions"
        );
    }

    /// Revert line: forward `env_var` to the wrapped provider in `BaseUrlOverride`.
    #[test]
    fn the_wrapper_names_no_key_variable() {
        for hint in [
            "cohere",
            "cohere/command-r-plus",
            "mistral/mistral-large-latest",
            "groq",
        ] {
            assert_eq!(hinted_at(hint, "http://gateway.test/v1").env_var(), None, "{hint}");
        }
    }

    #[test]
    fn a_base_url_that_does_not_parse_is_joined_with_the_path() {
        let cohere = hinted_at("cohere", "gateway");
        assert_eq!(cohere.build_url(cohere.chat_completions_path(), "m"), "gateway/chat");
    }

    /// A provider that answers every hook the wrapper forwards with a value other than the
    /// default, for the hooks that no shipped provider overrides.
    struct EveryHook;

    impl Provider for EveryHook {
        fn validate(&self) -> Result<()> {
            Err(LiterLlmError::BadRequest {
                message: "every-hook refuses".to_owned(),
                status: 400,
            })
        }

        fn name(&self) -> &str {
            "every-hook"
        }

        fn base_url(&self) -> &str {
            "https://own.test/v1"
        }

        fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
            Some((Cow::Borrowed("x-every-key"), Cow::Borrowed(api_key)))
        }

        fn extra_headers(&self) -> &'static [(&'static str, &'static str)] {
            &[("x-every-extra", "1")]
        }

        fn dynamic_headers(&self, body: &serde_json::Value) -> Vec<(String, String)> {
            vec![("x-every-dynamic".to_owned(), body.to_string())]
        }

        fn matches_model(&self, model: &str) -> bool {
            model.starts_with("every/")
        }

        fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
            model.strip_prefix("every/").unwrap_or(model)
        }

        fn chat_completions_path(&self) -> &str {
            "/every/chat"
        }

        fn embeddings_path(&self) -> &str {
            "/every/embed"
        }

        fn models_path(&self) -> &str {
            "/every/models"
        }

        fn next_models_page_url(&self, url: &str, _page: &serde_json::Value) -> Option<String> {
            Some(format!("{url}?page=2"))
        }

        fn image_generations_path(&self) -> &str {
            "/every/image"
        }

        fn audio_speech_path(&self) -> &str {
            "/every/speech"
        }

        fn audio_transcriptions_path(&self) -> &str {
            "/every/transcriptions"
        }

        fn moderations_path(&self) -> &str {
            "/every/moderations"
        }

        fn rerank_path(&self) -> &str {
            "/every/rerank"
        }

        fn files_path(&self) -> &str {
            "/every/files"
        }

        fn batches_path(&self) -> &str {
            "/every/batches"
        }

        fn responses_path(&self) -> &str {
            "/every/responses"
        }

        fn search_path(&self) -> &str {
            "/every/search"
        }

        fn ocr_path(&self) -> &str {
            "/every/ocr"
        }

        fn supports_streaming(&self) -> bool {
            false
        }

        fn transform_error(&self, error: LiterLlmError) -> LiterLlmError {
            LiterLlmError::ServerError {
                message: format!("every-hook: {error}"),
                status: 502,
            }
        }

        fn parse_stream_event(&self, event_data: &str) -> Result<Option<crate::types::ChatCompletionChunk>> {
            Err(LiterLlmError::Streaming {
                message: format!("every-hook: {event_data}"),
            })
        }

        fn stream_format(&self) -> StreamFormat {
            StreamFormat::Ndjson
        }

        fn stream_has_final_event(&self) -> bool {
            true
        }

        fn stream_tool_call_indexes_restart(&self) -> bool {
            true
        }

        fn signing_headers(&self, _method: &str, _url: &str, _body: &[u8]) -> Result<Vec<(String, String)>> {
            Ok(vec![("x-every-signed".to_owned(), "static".to_owned())])
        }

        fn signing_headers_with_credential(
            &self,
            _method: &str,
            _url: &str,
            _body: &[u8],
            credential: Option<&Credential>,
        ) -> Result<Vec<(String, String)>> {
            let header = if credential.is_some() {
                "x-every-signed-per-request"
            } else {
                "x-every-signed-without-key"
            };
            Ok(vec![(header.to_owned(), "1".to_owned())])
        }
    }

    /// What a provider answers to every hook the wrapper forwards, for fixed inputs.
    ///
    /// The URL is not here: a `base_url` changes its host, and the URL tests above pin it.
    /// Signing headers keep only their names, because a signature holds the time. A
    /// dynamic `x-request-id` keeps only its name, because it is random.
    #[derive(Debug, PartialEq)]
    struct Hooks {
        name: String,
        validate: String,
        auth_header: Option<(String, String)>,
        extra_headers: Vec<(String, String)>,
        dynamic_headers: Vec<Vec<(String, String)>>,
        signing_headers: Vec<String>,
        signing_headers_per_request: Vec<String>,
        models: Vec<(bool, String)>,
        paths: Vec<String>,
        next_models_page_url: Option<String>,
        transform_error: String,
        stream_events: Vec<String>,
        stream_format: StreamFormat,
        stream_has_final_event: bool,
        stream_tool_call_indexes_restart: bool,
        supports_streaming: bool,
    }

    const SIGNED_URL: &str = "https://own.test/v1/chat";

    /// One stream event in the wire format of each stream parser, an error event and the end.
    const STREAM_EVENTS: &[&str] = &[
        r#"{"id":"c","object":"chat.completion.chunk","created":1,"model":"m","choices":[{"index":0,"delta":{"content":"hi"},"finish_reason":null}]}"#,
        r#"{"model":"m","message":{"role":"assistant","content":"hi"},"done":false}"#,
        r#"{"model":"m","message":{"role":"assistant","content":""},"done":true,"done_reason":"stop"}"#,
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hi"}}"#,
        r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"hi"}]},"index":0}]}"#,
        r#"{"type":"content-delta","index":0,"delta":{"message":{"content":{"text":"hi"}}}}"#,
        r#"{"error":{"message":"boom","type":"server_error"}}"#,
        "[DONE]",
    ];

    const MODELS: &[&str] = &[
        "gpt-4o",
        "claude-3-5-sonnet",
        "command-r-plus",
        "gemini-1.5-pro",
        "mistral-large-latest",
        "llama3",
        "anthropic.claude-v2",
        "every/m",
    ];

    fn header_names(headers: Result<Vec<(String, String)>>) -> Vec<String> {
        match headers {
            Ok(headers) => headers.into_iter().map(|(name, _)| name).collect(),
            Err(error) => vec![format!("error: {error}")],
        }
    }

    fn hooks(provider: &dyn Provider) -> Hooks {
        let name = provider.name().to_owned();
        let models = MODELS
            .iter()
            .map(|model| (*model).to_owned())
            .chain([format!("{name}/m"), name.clone()])
            .map(|model| {
                (
                    provider.matches_model(&model),
                    provider.strip_model_prefix(&model).to_owned(),
                )
            })
            .collect();
        let bodies = [
            json!({}),
            json!({"thinking": {"type": "enabled", "budget_tokens": 1024}}),
            json!({"messages": [{"role": "tool", "content": "x"}]}),
        ];
        let dynamic_headers = bodies
            .iter()
            .map(|body| {
                provider
                    .dynamic_headers(body)
                    .into_iter()
                    .map(|(header, value)| {
                        let value = if header.eq_ignore_ascii_case("x-request-id") {
                            String::new()
                        } else {
                            value
                        };
                        (header, value)
                    })
                    .collect()
            })
            .collect();
        let aws_keys = Credential::AwsCredentials {
            access_key_id: "AKIDEXAMPLE".into(),
            secret_access_key: "secret".into(),
            session_token: None,
        };
        let bearer = Credential::BearerToken("token".into());
        let signing_headers_per_request = [None, Some(&aws_keys), Some(&bearer)]
            .into_iter()
            .flat_map(|key| header_names(provider.signing_headers_with_credential("POST", SIGNED_URL, b"{}", key)))
            .collect();
        let paths = [
            provider.chat_completions_path(),
            provider.embeddings_path(),
            provider.models_path(),
            provider.image_generations_path(),
            provider.audio_speech_path(),
            provider.audio_transcriptions_path(),
            provider.moderations_path(),
            provider.rerank_path(),
            provider.files_path(),
            provider.batches_path(),
            provider.responses_path(),
            provider.search_path(),
            provider.ocr_path(),
        ]
        .map(str::to_owned)
        .to_vec();
        let stream_events = STREAM_EVENTS
            .iter()
            .map(|event| {
                let parsed = provider.parse_stream_event(event).map(|chunk| {
                    chunk.map(|mut chunk| {
                        chunk.created = 0;
                        chunk
                    })
                });
                format!("{parsed:?}")
            })
            .collect();
        let error = LiterLlmError::ServerError {
            message: r#"{"error":"boom"}"#.to_owned(),
            status: 500,
        };
        Hooks {
            validate: format!("{:?}", provider.validate()),
            auth_header: provider
                .auth_header("key")
                .map(|(header, value)| (header.into_owned(), value.into_owned())),
            extra_headers: provider
                .extra_headers()
                .iter()
                .map(|(header, value)| ((*header).to_owned(), (*value).to_owned()))
                .collect(),
            dynamic_headers,
            signing_headers: header_names(provider.signing_headers("POST", SIGNED_URL, b"{}")),
            signing_headers_per_request,
            models,
            paths,
            next_models_page_url: provider.next_models_page_url(
                "https://own.test/v1/models",
                &json!({"next_page_token": "t", "has_more": true, "last_id": "m"}),
            ),
            transform_error: format!("{:?}", provider.transform_error(error)),
            stream_events,
            stream_format: provider.stream_format(),
            stream_has_final_event: provider.stream_has_final_event(),
            stream_tool_call_indexes_restart: provider.stream_tool_call_indexes_restart(),
            supports_streaming: provider.supports_streaming(),
            name,
        }
    }

    /// Every shipped provider a hint selects, and [`EveryHook`], answer each forwarded hook
    /// the same with and without the wrapper. The one intended difference: a provider that
    /// sends no key header at its own host sends the key as a bearer token at a `base_url`.
    ///
    /// Revert line: answer any forwarded hook in `BaseUrlOverride` with the trait default
    /// instead of asking the wrapped provider.
    #[test]
    #[serial]
    fn the_wrapper_answers_every_forwarded_hook_as_the_wrapped_provider() {
        let mut providers: Vec<Box<dyn Provider>> = all_providers()
            .expect("the shipped registry parses")
            .iter()
            .filter_map(|config| route_builtin_hint(&config.name).ok().flatten())
            .collect();
        providers.push(Box::new(EveryHook));
        let names: Vec<&str> = providers.iter().map(|provider| provider.name()).collect();
        for name in [
            "anthropic",
            "azure",
            "bedrock",
            "cohere",
            "github_copilot",
            "google_ai",
            "ollama",
            "vertex_ai",
            "every-hook",
        ] {
            assert!(names.contains(&name), "{name} is not in the table");
        }
        assert!(providers.len() > 50, "{} providers in the table", providers.len());
        for provider in providers {
            let mut expected = hooks(provider.as_ref());
            expected.auth_header = expected
                .auth_header
                .or_else(|| Some(("Authorization".to_owned(), "Bearer key".to_owned())));
            let rebased = BaseUrlOverride::new(provider, "http://gateway.test/v1");
            assert_eq!(hooks(&rebased), expected, "{}", expected.name);
        }
    }

    /// A provider whose `build_url` returns the endpoint path as the whole URL.
    struct UrlAsPath;

    impl Provider for UrlAsPath {
        fn name(&self) -> &str {
            "url-as-path"
        }

        fn base_url(&self) -> &str {
            "https://own.test/v1"
        }

        fn auth_header<'a>(&'a self, _api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
            None
        }

        fn matches_model(&self, _model: &str) -> bool {
            false
        }

        fn build_url(&self, endpoint_path: &str, _model: &str) -> String {
            endpoint_path.to_owned()
        }
    }

    /// Only a URL on the provider's own host, under its base path at a `/`, takes the
    /// `base_url` path in place of the base path. Any other URL keeps its whole path.
    ///
    /// Revert line: drop the origin check or the `/` boundary check from
    /// `BaseUrlOverride::rebase`.
    #[test]
    fn only_a_url_under_the_own_base_path_takes_the_base_url_path() {
        let rebased = BaseUrlOverride::new(Box::new(UrlAsPath), "http://gateway.test/proxy");
        for (url, expected) in [
            ("https://own.test/v1/chat", "http://gateway.test/proxy/chat"),
            ("https://own.test/v1", "http://gateway.test/proxy/"),
            ("https://other.test/v1/chat", "http://gateway.test/v1/chat"),
            ("https://own.test:8443/v1/chat", "http://gateway.test/v1/chat"),
            ("https://own.test/v1beta/chat", "http://gateway.test/v1beta/chat"),
        ] {
            assert_eq!(rebased.build_url(url, "m"), expected, "{url}");
        }
    }
}
