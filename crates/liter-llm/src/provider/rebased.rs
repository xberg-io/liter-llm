use std::borrow::Cow;

use crate::error::Result;
use crate::provider::{Provider, StreamFormat, join_url};

/// A provider sent to another host.
///
/// Wraps a provider and answers `base_url` with the new host. Every other method asks the
/// wrapped provider, so the request body, the paths, the headers and the reply handling stay
/// those of that provider. [`Provider::build_url`] and [`Provider::build_stream_url`] take the
/// URL the wrapped provider builds and move it to the new host, see [`Self::rebase`].
/// [`Provider::env_var`] is not forwarded.
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

    use super::BaseUrlOverride;
    use crate::provider::{Provider, on_base_url, route_hint};

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
}
