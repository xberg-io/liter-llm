//! Embedding provider abstraction.
//!
//! [`EmbeddingProvider`] converts text into a dense float vector suitable for
//! similarity lookup in a [`VectorStore`][crate::vectorstore::VectorStore].
//!
//! # Built-in implementations
//!
//! | Type | Description |
//! |---|---|
//! | [`SelfHostedEmbeddingProvider`] | Calls back into the liter-llm [`LlmClient::embed`] API so the user can use any provider they have configured. |
//! | [`NoOpEmbeddingProvider`] | Returns zero vectors. For tests and the `lite` feature where the semantic cache tier is disabled. |

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::client::LlmClient;
use crate::error::Result;
use crate::types::EmbeddingRequest;

// ── EmbeddingProvider trait ───────────────────────────────────────────────────

/// Pluggable embedding provider.
///
/// Implement this trait to convert arbitrary text into a dense vector for
/// semantic cache lookup.
///
/// # Object safety
///
/// The trait is object-safe; implementations can be stored behind
/// `Arc<dyn EmbeddingProvider>`.
#[cfg_attr(alef, alef(skip))]
pub trait EmbeddingProvider: Send + Sync + 'static {
    /// Embed `text` and return a dense float vector.
    ///
    /// The returned vector must have length equal to [`dim`][Self::dim].
    fn embed<'a>(&'a self, text: &'a str) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>>;

    /// Expected output dimensionality.
    ///
    /// This value must match the [`VectorStore::dim`][crate::vectorstore::VectorStore::dim]
    /// of any store used alongside this provider.
    fn dim(&self) -> usize;
}

// ── SelfHostedEmbeddingProvider ───────────────────────────────────────────────

/// Embedding provider that calls back into the liter-llm [`LlmClient::embed`]
/// API.
///
/// This lets callers use any of the 143 providers already configured in their
/// `LlmClient` instance as the embedding backend without any additional setup.
///
/// # Example
///
/// ```rust,ignore
/// use liter_llm::embedding::SelfHostedEmbeddingProvider;
/// use std::sync::Arc;
///
/// let embedding_provider = Arc::new(
///     SelfHostedEmbeddingProvider::new(
///         Arc::new(my_client),
///         "openai/text-embedding-3-small",
///         1536,
///     ),
/// );
/// ```
#[cfg_attr(alef, alef(skip))]
pub struct SelfHostedEmbeddingProvider {
    client: Arc<dyn LlmClient>,
    model: String,
    dim: usize,
}

impl SelfHostedEmbeddingProvider {
    /// Create a new self-hosted embedding provider.
    ///
    /// `client` will be called to produce embeddings.
    /// `model` is the model identifier forwarded to `LlmClient::embed`.
    /// `dim` is the expected output dimensionality (must match the model).
    #[must_use]
    pub fn new(client: Arc<dyn LlmClient>, model: impl Into<String>, dim: usize) -> Self {
        Self {
            client,
            model: model.into(),
            dim,
        }
    }
}

impl EmbeddingProvider for SelfHostedEmbeddingProvider {
    fn embed<'a>(&'a self, text: &'a str) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>> {
        let req = EmbeddingRequest {
            model: self.model.clone(),
            input: crate::types::EmbeddingInput::Single(text.to_owned()),
            encoding_format: None,
            dimensions: Some(self.dim as u32),
            user: None,
        };
        let client = Arc::clone(&self.client);
        Box::pin(async move {
            let resp = client.embed(req).await?;
            // Extract the first embedding vector.
            // The LLM API returns f64 embeddings; VectorStore operates on f32.
            // Truncate precision at the boundary — f32 is sufficient for cosine
            // similarity lookups and halves memory usage.
            let vec: Vec<f32> = resp
                .data
                .into_iter()
                .next()
                .map(|obj| obj.embedding.into_iter().map(|x| x as f32).collect())
                .unwrap_or_default();
            Ok(vec)
        })
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

// ── NoOpEmbeddingProvider ─────────────────────────────────────────────────────

/// A no-op embedding provider that returns a zero vector of the configured
/// dimensionality.
///
/// Useful in tests and in the `lite` configuration where the semantic cache
/// tier is disabled and no embedding provider is wired up.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct NoOpEmbeddingProvider {
    /// Dimensionality of the zero vector returned by [`embed`][Self::embed].
    pub dim: usize,
}

impl EmbeddingProvider for NoOpEmbeddingProvider {
    fn embed<'a>(&'a self, _text: &'a str) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>> {
        Box::pin(std::future::ready(Ok(vec![0.0_f32; self.dim])))
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::client::LlmClient;
    use crate::client::{BoxFuture, BoxStream};
    use crate::error::LiterLlmError;
    use crate::types::{
        ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, EmbeddingObject, EmbeddingRequest,
        EmbeddingResponse, ModelsListResponse, Usage,
        audio::{CreateSpeechRequest, CreateTranscriptionRequest, TranscriptionResponse},
        image::{CreateImageRequest, ImagesResponse},
        moderation::{ModerationRequest, ModerationResponse},
        ocr::{OcrRequest, OcrResponse},
        rerank::{RerankRequest, RerankResponse},
        search::{SearchRequest, SearchResponse},
    };

    // ── NoOpEmbeddingProvider tests ───────────────────────────────────────────

    #[tokio::test]
    async fn no_op_embedding_provider_returns_zero_vector() {
        let provider = NoOpEmbeddingProvider { dim: 4 };
        let vec = provider.embed("hello world").await.unwrap();
        assert_eq!(vec.len(), 4);
        assert!(vec.iter().all(|&x| x == 0.0));
    }

    #[tokio::test]
    async fn no_op_embedding_provider_dim_is_consistent() {
        let provider = NoOpEmbeddingProvider { dim: 128 };
        assert_eq!(provider.dim(), 128);
        let vec = provider.embed("test").await.unwrap();
        assert_eq!(vec.len(), provider.dim());
    }

    // ── SelfHostedEmbeddingProvider tests ─────────────────────────────────────

    /// A mock client that returns a fixed embedding vector.
    #[derive(Clone)]
    struct MockEmbedClient {
        embedding: Vec<f32>,
    }

    impl MockEmbedClient {
        fn new(embedding: Vec<f32>) -> Self {
            Self { embedding }
        }
    }

    impl LlmClient for MockEmbedClient {
        fn chat(&self, _req: ChatCompletionRequest) -> BoxFuture<'_, crate::error::Result<ChatCompletionResponse>> {
            Box::pin(async {
                Err(LiterLlmError::EndpointNotSupported {
                    endpoint: "chat".into(),
                    provider: "mock".into(),
                })
            })
        }

        fn chat_stream(
            &self,
            _req: ChatCompletionRequest,
        ) -> BoxFuture<'_, crate::error::Result<BoxStream<'static, crate::error::Result<ChatCompletionChunk>>>>
        {
            Box::pin(async {
                Err(LiterLlmError::EndpointNotSupported {
                    endpoint: "chat_stream".into(),
                    provider: "mock".into(),
                })
            })
        }

        fn embed(&self, req: EmbeddingRequest) -> BoxFuture<'_, crate::error::Result<EmbeddingResponse>> {
            let embedding: Vec<f64> = self.embedding.iter().map(|&v| f64::from(v)).collect();
            Box::pin(async move {
                Ok(EmbeddingResponse {
                    object: "list".into(),
                    data: vec![EmbeddingObject {
                        object: "embedding".into(),
                        embedding,
                        index: 0,
                    }],
                    model: req.model,
                    usage: Some(Usage {
                        prompt_tokens: 4,
                        completion_tokens: 0,
                        total_tokens: 4,
                        prompt_tokens_details: None,
                    }),
                })
            })
        }

        fn list_models(&self) -> BoxFuture<'_, crate::error::Result<ModelsListResponse>> {
            Box::pin(async {
                Ok(ModelsListResponse {
                    object: "list".into(),
                    data: vec![],
                })
            })
        }

        fn image_generate(&self, _req: CreateImageRequest) -> BoxFuture<'_, crate::error::Result<ImagesResponse>> {
            Box::pin(async {
                Ok(ImagesResponse {
                    created: 0,
                    data: vec![],
                })
            })
        }

        fn speech(&self, _req: CreateSpeechRequest) -> BoxFuture<'_, crate::error::Result<bytes::Bytes>> {
            Box::pin(async { Ok(bytes::Bytes::new()) })
        }

        fn transcribe(
            &self,
            _req: CreateTranscriptionRequest,
        ) -> BoxFuture<'_, crate::error::Result<TranscriptionResponse>> {
            Box::pin(async {
                Ok(TranscriptionResponse {
                    text: String::new(),
                    language: None,
                    duration: None,
                    segments: None,
                })
            })
        }

        fn moderate(&self, _req: ModerationRequest) -> BoxFuture<'_, crate::error::Result<ModerationResponse>> {
            Box::pin(async {
                Ok(ModerationResponse {
                    id: String::new(),
                    model: String::new(),
                    results: vec![],
                })
            })
        }

        fn rerank(&self, _req: RerankRequest) -> BoxFuture<'_, crate::error::Result<RerankResponse>> {
            Box::pin(async {
                Ok(RerankResponse {
                    id: None,
                    results: vec![],
                    meta: None,
                })
            })
        }

        fn search(&self, _req: SearchRequest) -> BoxFuture<'_, crate::error::Result<SearchResponse>> {
            Box::pin(async {
                Err(LiterLlmError::EndpointNotSupported {
                    endpoint: "search".into(),
                    provider: "mock".into(),
                })
            })
        }

        fn ocr(&self, _req: OcrRequest) -> BoxFuture<'_, crate::error::Result<OcrResponse>> {
            Box::pin(async {
                Err(LiterLlmError::EndpointNotSupported {
                    endpoint: "ocr".into(),
                    provider: "mock".into(),
                })
            })
        }
    }

    #[tokio::test]
    async fn self_hosted_embedding_provider_round_trips_through_mock_client() {
        let expected_vec = vec![0.1_f32, 0.2, 0.3, 0.4];
        let client = Arc::new(MockEmbedClient::new(expected_vec.clone()));
        let provider = SelfHostedEmbeddingProvider::new(client, "openai/text-embedding-3-small", 4);

        let result = provider.embed("hello world").await.unwrap();
        assert_eq!(result, expected_vec, "should return the mock client's embedding vector");
    }

    #[tokio::test]
    async fn self_hosted_embedding_provider_dim_matches_constructor() {
        let client = Arc::new(MockEmbedClient::new(vec![0.0; 1536]));
        let provider = SelfHostedEmbeddingProvider::new(client, "openai/text-embedding-3-small", 1536);
        assert_eq!(provider.dim(), 1536);
    }
}
