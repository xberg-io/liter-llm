use std::borrow::Cow;

use crate::error::Result;
use crate::provider::{Provider, StreamFormat};

/// A provider sent to another host.
///
/// Wraps a provider whose URL is `{base_url}{path}` and answers `base_url` with the new host.
/// Every other method asks the wrapped provider, so the request body, the paths, the headers
/// and the reply handling stay those of that provider. [`Provider::build_url`] and
/// [`Provider::build_stream_url`] are not forwarded: their default builds the URL from
/// [`Provider::base_url`], which is the new host. [`Provider::env_var`] is not forwarded
/// either.
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
