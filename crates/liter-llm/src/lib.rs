//! Universal LLM API client with provider-agnostic chat, embeddings, files,
//! batches, responses, image generation, transcription, moderation, OCR,
//! rerank, and web-search across 165 providers.
//!
//! See [`LlmClient`] for the high-level streaming client, [`DefaultClient`]
//! (native-http only) for the canonical reqwest-backed implementation, and
//! [`client::ClientConfig`] for builder-style configuration.

// ~keep wasm/no-native-http builds expose a type-only surface, so dead code is expected.
#![cfg_attr(
    not(any(feature = "native-http", feature = "wasm-http")),
    allow(dead_code, unused_imports)
)]
// ~keep rustdoc cannot resolve some short names visible in impl scope; rendered docs are still correct.
#![allow(rustdoc::broken_intra_doc_links)]

/// Per-provider authentication strategies (API keys, AWS SigV4, OAuth tokens).
pub mod auth;
/// FFI-friendly client constructors used by the polyglot bindings.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub mod bindings;
/// Pluggable cache key derivation strategies ([`CacheKeyStrategy`], built-in impls).
#[cfg(feature = "tower")]
pub mod cache_key;
/// High-level LLM client traits and the reqwest-backed [`client::DefaultClient`].
pub mod client;
/// Token-cost tracking helpers.
pub mod cost;
/// Embedding provider abstraction ([`EmbeddingProvider`], [`SelfHostedEmbeddingProvider`]).
#[cfg(feature = "tower")]
pub mod embedding;
/// Public error types and the crate-wide [`Result`] alias.
pub mod error;
/// Vendor-neutral guardrail plugin system (trait, stage enum, registry, built-in primitives).
pub mod guardrail;
pub(crate) mod http;
/// Base64 data URL encoding and decoding for inline image payloads.
///
/// Provides [`image::encode_data_url`] and [`image::decode_data_url`] for
/// building `data:<mime>;base64,<b64>` strings used in
/// [`ContentPart::ImageUrl`].
pub mod image;
/// Canonical per-request usage events and pluggable sinks.
pub mod observability;
/// Provider catalog (built-in providers plus runtime registration of custom providers).
pub mod provider;
/// Unified Realtime API event schema and per-provider translator trait.
pub mod realtime;
/// Ingress/egress streaming pipeline with zero-copy passthrough optimisation.
///
/// Exposes [`streaming::IngressStream`], [`streaming::StreamPipeline`], and
/// [`streaming::EgressStream`] for composing streaming request pipelines with
/// optional per-chunk middleware and end-to-end cancellation.
pub mod streaming;
/// Generic multi-tenant primitives: [`tenant::TenantId`], [`tenant::TenantContext`],
/// [`tenant::KeyResolver`], and [`tenant::InMemoryKeyResolver`].
pub mod tenant;
#[cfg(test)]
mod tests;
#[cfg(feature = "tokenizer")]
/// Tokenizer helpers for measuring prompt and completion lengths.
pub mod tokenizer;
#[cfg(feature = "tower")]
/// `tower` middleware layers (rate limiting, retries, observability).
pub mod tower;
/// Request/response DTOs shared across providers and bindings.
pub mod types;
/// Shared utility helpers (memory-bound guards, etc.).
pub mod util;
/// Vector store abstraction for the semantic cache tier ([`VectorStore`], [`InMemoryVectorStore`]).
#[cfg(feature = "tower")]
pub mod vectorstore;

pub use client::{
    BatchClient, BoxFuture, BoxStream, ClientBuilder, ClientConfig, ClientConfigBuilder, FileClient, FileConfig,
    LlmClient, LlmClientRaw, ResponseClient,
};
// ~keep Batch polling helpers are binding-public and used by every language binding.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub use client::{BatchWaitError, WaitForBatchConfig};
pub use http::transport::TransportConfig;
// ~keep DefaultClient requires an HTTP stack: reqwest on native or browser fetch on WASM.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub use client::DefaultClient;
// ~keep Binding-friendly constructors require an HTTP stack.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub use bindings::{create_client, create_client_from_json};
// ~keep ManagedClient requires both native HTTP and Tower middleware.
#[cfg(all(feature = "native-http", feature = "tower"))]
pub use client::managed::ManagedClient;
pub use error::{LiterLlmError, Result};
// ~keep Export only Tower config DTOs at root; Layer/Service composition is not FFI-friendly.
#[cfg(feature = "tower")]
pub use tower::{BudgetConfig, CacheBackend, CacheConfig, Enforcement, RateLimitConfig};
// ~keep Root-export Tower DTOs and traits that bindings need without exposing Layer/Service types.
#[cfg(feature = "tower")]
pub use tower::{
    CacheKeyStrategy, EmbeddingProvider, ExactHashStrategy, Guardrail, GuardrailContext, GuardrailDecision,
    GuardrailStage, NoOpEmbeddingProvider, SystemPromptAwareStrategy, TenantScopedStrategy, VectorMatch, VectorStore,
};
// ~keep Re-export provider helpers that are public API while the module stays pub(crate).
pub use cost::{ModelInfo, ModelTier, completion_cost, completion_cost_with_cache, model_info};
// ~keep Runtime catalog refresh surface. Always compiled so it reaches every
// ~keep binding; refresh is a runtime toggle (off by default), and the network
// ~keep fetch degrades to a clean error on builds without `native-http`.
pub use cost::refresh::{
    CatalogRefreshConfig, CatalogRefreshError, DEFAULT_CATALOG_URL, RefreshOutcome, clear_catalog_overlay,
    install_catalog_overlay_from_str, refresh_catalog,
};
pub use provider::custom::{
    AuthHeaderFormat, CustomProviderConfig, register_custom_provider, unregister_custom_provider,
};
pub use provider::{
    AuthConfig, AuthType, ProviderCapabilities, ProviderConfig, StreamFormat, all_providers, capabilities,
    complex_provider_names,
};
#[cfg(feature = "tokenizer")]
pub use tokenizer::{count_request_tokens, count_tokens};
pub use types::*;

// ~keep Do not re-export `realtime::ContentPart`; it would shadow `types::ContentPart`.
// ~keep Downstream callers rely on `liter_llm::ContentPart::ImageUrl`.
pub use realtime::{OpenAiRealtimeTranslator, RealtimeEnvelope, RealtimeEvent, RealtimeTranslator, ResponseStatus};
/// Tenant primitives re-exported at the crate root.
///
/// Importers can write `liter_llm::TenantId` without spelling out the
/// `tenant::` path.
///
/// # Example
///
/// ```
/// let id = liter_llm::TenantId::from("acme-corp");
/// assert_eq!(id.as_ref(), "acme-corp");
/// ```
pub use tenant::{InMemoryKeyResolver, KeyResolver, KeyResolverError, ResolvedKey, TenantContext, TenantId};

/// Install the `ring` crypto provider as the rustls process default, idempotently.
///
/// rustls 0.23+ removed the implicit default provider. This function installs
/// `ring` once per process. Subsequent calls are no-ops. Calling it after
/// another rustls crypto provider has already been installed is safe: the
/// `Err` from `install_default()` is silently ignored.
///
/// Called automatically by every internal `reqwest::Client` constructor
/// (auth providers, default HTTP client). Bindings and downstream consumers
/// reach those constructors transitively, so no manual init is required.
///
/// WASM builds are exempt — the WASM target uses the browser/Node.js fetch
/// API instead of rustls, so no crypto provider is needed.
///
/// Windows builds use native-tls (SChannel) via reqwest, so rustls is not
/// present and no crypto provider installation is needed.
#[cfg(all(feature = "native-http", not(target_os = "windows")))]
pub fn ensure_crypto_provider() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // ~keep Ignore `install_default` errors; callers may deliberately install another rustls provider.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// No-op on Windows: reqwest uses native-tls (SChannel), so no rustls provider
/// installation is needed. All callers use the same call site regardless of
/// platform.
#[cfg(all(feature = "native-http", target_os = "windows"))]
pub fn ensure_crypto_provider() {}
