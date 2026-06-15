//! Universal LLM API client with provider-agnostic chat, embeddings, files,
//! batches, responses, image generation, transcription, moderation, OCR,
//! rerank, and web-search across 140+ providers.
//!
//! See [`LlmClient`] for the high-level streaming client, [`DefaultClient`]
//! (native-http only) for the canonical reqwest-backed implementation, and
//! [`client::ClientConfig`] for builder-style configuration.

// Provider, HTTP, and retry infrastructure are only active with native-http.
// Suppress dead_code lints on the wasm / no-native-http target so that the
// type-only surface compiles cleanly.
#![cfg_attr(
    not(any(feature = "native-http", feature = "wasm-http")),
    allow(dead_code, unused_imports)
)]
// Many doc comments reference types by short name (`Service`, `LlmRequest`,
// `LiterLlmError::ServiceUnavailable`) when they are in lexical scope inside
// the surrounding `impl` block but not in the rustdoc resolution context.
// These links render fine in the rendered docs (rustdoc treats them as
// plain text); the warnings are noise for our docs flow.
#![allow(rustdoc::broken_intra_doc_links)]

/// Per-provider authentication strategies (API keys, AWS SigV4, OAuth tokens).
pub mod auth;
/// FFI-friendly client constructors used by the polyglot bindings.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub mod bindings;
/// High-level LLM client traits and the reqwest-backed [`client::DefaultClient`].
pub mod client;
/// Token-cost tracking helpers.
pub mod cost;
/// Public error types and the crate-wide [`Result`] alias.
pub mod error;
/// Vendor-neutral guardrail plugin system (trait, stage enum, registry, built-in primitives).
pub mod guardrail;
pub(crate) mod http;
/// Provider catalog (built-in providers plus runtime registration of custom providers).
pub mod provider;
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

// Re-export key types at crate root.
pub use client::{
    BatchClient, BoxFuture, BoxStream, ClientBuilder, ClientConfig, ClientConfigBuilder, FileClient, FileConfig,
    LlmClient, LlmClientRaw, ResponseClient,
};
// DefaultClient requires the native HTTP stack (reqwest on native or WASM fetch API).
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub use client::DefaultClient;
// Binding-friendly constructors require the native HTTP stack.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub use bindings::{create_client, create_client_from_json};
// ManagedClient requires both the native HTTP stack and Tower middleware.
#[cfg(all(feature = "native-http", feature = "tower"))]
pub use client::managed::ManagedClient;
pub use error::{LiterLlmError, Result};
// Tower middleware public config DTOs (only the configs — Layer/Service/State
// types stay inside the `tower` module since middleware composition is a Rust
// pattern that does not cross FFI cleanly).
#[cfg(feature = "tower")]
pub use tower::{BudgetConfig, CacheBackend, CacheConfig, Enforcement, RateLimitConfig};
// Re-export the public provider helper functions that are part of the crate's
// public API even though the `provider` module itself is pub(crate).
pub use cost::{completion_cost, completion_cost_with_cache};
pub use provider::custom::{
    AuthHeaderFormat, CustomProviderConfig, register_custom_provider, unregister_custom_provider,
};
pub use provider::{
    AuthConfig, AuthType, ProviderCapabilities, ProviderConfig, all_providers, capabilities, complex_provider_names,
};
#[cfg(feature = "tokenizer")]
pub use tokenizer::{count_request_tokens, count_tokens};
pub use types::*;

/// Install the `ring` crypto provider as the rustls process default, idempotently.
///
/// rustls 0.23+ removed the implicit default provider. This function installs
/// `ring` once per process. Subsequent calls are no-ops. Calling it from a
/// downstream Rust app that has already installed `aws-lc-rs` is safe — the
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
        // `install_default` returns Err if another provider is already installed.
        // That is fine — the caller may have installed `aws-lc-rs` deliberately;
        // we do not want to override their choice.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// No-op on Windows: reqwest uses native-tls (SChannel), so no rustls provider
/// installation is needed. All callers use the same call site regardless of
/// platform.
#[cfg(all(feature = "native-http", target_os = "windows"))]
pub fn ensure_crypto_provider() {}
