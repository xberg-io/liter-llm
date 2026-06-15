// retry logic is pure (no reqwest/tokio) and is used by the streaming module
// even in WASM builds, so it is always compiled.
pub(crate) mod retry;

// request and streaming use reqwest + tokio and are only available when the
// native-http feature is enabled.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub(crate) mod eventstream;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub(crate) mod request;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub(crate) mod streaming;

// Transport configuration for HTTP client pooling and protocol selection.
pub mod transport;
