use std::sync::Arc;

use crate::auth::KeyStore;
use crate::config::ProxyConfig;
use crate::file_store::FileStore;
use crate::service_pool::ServicePool;
use crate::shutdown::ShutdownHandle;

/// Shared application state passed to all axum handlers via `State`.
#[derive(Clone)]
pub struct AppState {
    pub key_store: Arc<KeyStore>,
    pub service_pool: Arc<ServicePool>,
    pub file_store: Arc<FileStore>,
    pub config: Arc<ProxyConfig>,
    /// Optional shutdown handle; present when the server was started via
    /// [`crate::ProxyServer::serve_with_shutdown`].  Health routes read this
    /// to differentiate between `/health/liveness` (200 during drain) and
    /// `/health/readiness` (503 during drain).
    pub shutdown: Option<ShutdownHandle>,
}
