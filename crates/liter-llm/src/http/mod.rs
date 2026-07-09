pub(crate) mod retry;

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub(crate) mod eventstream;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub(crate) mod request;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub(crate) mod streaming;

/// Transport configuration (`TransportConfig`) for HTTP client pooling,
/// connection limits, and HTTP-version negotiation.
pub mod transport;
