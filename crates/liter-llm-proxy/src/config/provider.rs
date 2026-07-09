//! `ConfigProvider` trait and built-in implementations.
//!
//! The trait decouples the proxy from its configuration source. Two
//! implementations are provided by default:
//!
//! - [`StaticFileConfigProvider`] — loads a TOML file once at startup; the
//!   `watch` method returns a receiver that never yields (no live reload).
//! - [`FileWatchConfigProvider`] — uses the `notify` crate to watch a TOML
//!   file for changes; emits a [`ConfigEvent::Put`] on every save.
//!
//! A third implementation is available under the `etcd-watch` feature:
//!
//! - [`EtcdConfigProvider`] — watches an etcd key prefix; emits
//!   [`ConfigEvent::Put`], [`ConfigEvent::Delete`], and
//!   [`ConfigEvent::Resync`] per etcd watch semantics. Requires `protoc` at
//!   build time.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
#[cfg(feature = "etcd-watch")]
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use tokio::sync::mpsc;

use super::{ProxyConfig, interpolate_env_vars};

/// Errors that a [`ConfigProvider`] can produce.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The requested configuration key was not found.
    #[error("configuration not found")]
    NotFound,

    /// The caller does not have permission to access the configuration.
    #[error("permission denied")]
    PermissionDenied,

    /// A transport or backend error (I/O, network, …).
    #[error("backend error: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// The configuration data could not be parsed.
    #[error("parse error: {0}")]
    Parse(String),
}

/// Events emitted by the MPSC receiver returned from
/// [`ConfigProvider::watch`].
#[derive(Clone)]
pub enum ConfigEvent {
    /// A configuration key was created or updated.
    Put {
        /// Monotonic revision counter (etcd revision or file mtime seconds).
        revision: u64,
        /// The new configuration value.
        config: ProxyConfig,
    },
    /// A configuration key was deleted from the backend.
    Delete {
        /// Monotonic revision counter at the time of deletion.
        revision: u64,
        /// The key path that was deleted.
        path: String,
    },
    /// The watch stream was interrupted and a full reload is required.
    ///
    /// Consumers should treat this like a `Put` — the embedded `config` is
    /// the latest snapshot fetched after reconnecting.
    Resync {
        /// Revision after the resync.
        revision: u64,
        /// Latest full configuration snapshot.
        config: ProxyConfig,
    },
}

impl std::fmt::Debug for ConfigEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Put { revision, .. } => formatter
                .debug_struct("Put")
                .field("revision", revision)
                .finish_non_exhaustive(),
            Self::Delete { revision, path } => formatter
                .debug_struct("Delete")
                .field("revision", revision)
                .field("path", path)
                .finish(),
            Self::Resync { revision, .. } => formatter
                .debug_struct("Resync")
                .field("revision", revision)
                .finish_non_exhaustive(),
        }
    }
}

/// Source of proxy configuration.
///
/// Implementors must be `Send + Sync + 'static` so they can be held inside
/// `Arc` and shared across Tokio tasks.
///
/// # Examples
///
/// Implement a static configuration provider that loads a TOML file once:
///
/// ```no_run
/// use liter_llm_proxy::config::{ConfigProvider, ConfigError, ProxyConfig, ConfigEvent};
/// use std::pin::Pin;
/// use std::future::Future;
/// use std::path::PathBuf;
/// use tokio::sync::mpsc;
///
/// struct MyStaticConfigProvider {
///     path: PathBuf,
/// }
///
/// impl ConfigProvider for MyStaticConfigProvider {
///     fn load<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<ProxyConfig, ConfigError>> + Send + 'a>> {
///         // Load from file...
///         Box::pin(async { Err(ConfigError::NotFound) })
///     }
///
///     fn watch<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<mpsc::Receiver<ConfigEvent>, ConfigError>> + Send + 'a>> {
///         // Return a receiver that never yields (no hot-reload)
///         Box::pin(async {
///             let (_tx, rx) = mpsc::channel(1);
///             Ok(rx)
///         })
///     }
/// }
/// ```
pub trait ConfigProvider: Send + Sync + 'static {
    /// Fetch the full configuration once.
    ///
    /// Returns the current [`ProxyConfig`], or a [`ConfigError`] when the
    /// backend is unavailable or the data is malformed.
    fn load<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<ProxyConfig, ConfigError>> + Send + 'a>>;

    /// Subscribe to configuration changes.
    ///
    /// Returns an MPSC receiver that emits [`ConfigEvent`] values on every
    /// put, delete, or resync until the provider is dropped or the receiver
    /// is closed.
    ///
    /// The first event sent after subscription is implementation-defined.
    /// Callers MUST call [`load`][ConfigProvider::load] independently to
    /// obtain the current snapshot before waiting on the receiver.
    fn watch<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<mpsc::Receiver<ConfigEvent>, ConfigError>> + Send + 'a>>;
}

/// Loads a TOML configuration file once at startup. The `watch` method
/// returns a receiver that never yields events — suitable for simple
/// single-instance deployments that restart the process to pick up changes.
pub struct StaticFileConfigProvider {
    path: PathBuf,
}

impl StaticFileConfigProvider {
    /// Create a provider that reads `path` on each [`load`][ConfigProvider::load] call.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl ConfigProvider for StaticFileConfigProvider {
    fn load<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<ProxyConfig, ConfigError>> + Send + 'a>> {
        let path = self.path.clone();
        Box::pin(async move { load_toml_file(&path) })
    }

    fn watch<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<mpsc::Receiver<ConfigEvent>, ConfigError>> + Send + 'a>> {
        Box::pin(async move {
            let (_tx, rx) = mpsc::channel::<ConfigEvent>(1);
            Ok(rx)
        })
    }
}

/// Watches a TOML file for changes using the `notify` crate (OS-level file
/// system events). Emits a [`ConfigEvent::Put`] whenever the file is modified
/// or created.
///
/// This is the default for non-distributed deployments when `--watch` is
/// passed to the CLI.
pub struct FileWatchConfigProvider {
    path: PathBuf,
}

impl FileWatchConfigProvider {
    /// Create a provider backed by the file at `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl ConfigProvider for FileWatchConfigProvider {
    fn load<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<ProxyConfig, ConfigError>> + Send + 'a>> {
        let path = self.path.clone();
        Box::pin(async move { load_toml_file(&path) })
    }

    fn watch<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<mpsc::Receiver<ConfigEvent>, ConfigError>> + Send + 'a>> {
        let path = self.path.clone();
        Box::pin(async move {
            use notify::{EventKind, RecursiveMode, Watcher, event::ModifyKind};

            let (event_tx, mut event_rx) = mpsc::channel::<notify::Result<notify::Event>>(32);
            let (config_tx, config_rx) = mpsc::channel::<ConfigEvent>(8);

            let watch_path = path.clone();
            let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                let _ = event_tx.blocking_send(res);
            })
            .map_err(|e| ConfigError::Backend(Box::new(e)))?;

            watcher
                .watch(&watch_path, RecursiveMode::NonRecursive)
                .map_err(|e| ConfigError::Backend(Box::new(e)))?;

            // ~keep The notify watcher must stay owned by the task to keep OS watching active.
            tokio::spawn(async move {
                // ~keep `_watcher` must stay alive for the OS watch to remain active.
                let _watcher = watcher;

                while let Some(event) = event_rx.recv().await {
                    let event = match event {
                        Ok(e) => e,
                        Err(err) => {
                            tracing::warn!("file watch error: {err}");
                            continue;
                        }
                    };

                    let is_write = matches!(
                        event.kind,
                        EventKind::Modify(ModifyKind::Data(_))
                            | EventKind::Modify(ModifyKind::Any)
                            | EventKind::Create(_)
                    );

                    if !is_write {
                        continue;
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

                    let revision = file_mtime_secs(&path).unwrap_or(0);

                    match load_toml_file(&path) {
                        Ok(config) => {
                            if config_tx.send(ConfigEvent::Put { revision, config }).await.is_err() {
                                break;
                            }
                        }
                        Err(err) => {
                            tracing::warn!("failed to reload config from {}: {err}", path.display());
                        }
                    }
                }
            });

            Ok(config_rx)
        })
    }
}

#[cfg(feature = "etcd-watch")]
/// Watches an etcd key prefix for configuration changes.
///
/// On startup, `load` fetches the value at `key`. The `watch` method opens
/// an etcd watch stream and emits [`ConfigEvent::Put`], [`ConfigEvent::Delete`],
/// and [`ConfigEvent::Resync`] events.
///
/// A [`ConfigEvent::Resync`] is emitted when the watch stream is interrupted
/// (e.g. etcd compaction) and a fresh snapshot has been fetched to replace it.
pub struct EtcdConfigProvider {
    client: Arc<tokio::sync::Mutex<etcd_client::Client>>,
    key: String,
}

#[cfg(feature = "etcd-watch")]
impl EtcdConfigProvider {
    /// Connect to an etcd cluster and return a provider that watches `key`.
    ///
    /// `endpoints` is a list of etcd URLs, e.g. `["http://127.0.0.1:2379"]`.
    pub async fn connect(endpoints: impl Into<Vec<String>>, key: impl Into<String>) -> Result<Self, ConfigError> {
        let client = etcd_client::Client::connect(endpoints.into(), None)
            .await
            .map_err(|e| ConfigError::Backend(Box::new(e)))?;
        Ok(Self {
            client: Arc::new(tokio::sync::Mutex::new(client)),
            key: key.into(),
        })
    }
}

#[cfg(feature = "etcd-watch")]
impl ConfigProvider for EtcdConfigProvider {
    fn load<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<ProxyConfig, ConfigError>> + Send + 'a>> {
        let client = Arc::clone(&self.client);
        let key = self.key.clone();
        Box::pin(async move {
            let mut guard = client.lock().await;
            let response = guard
                .get(key.as_str(), None)
                .await
                .map_err(|e| ConfigError::Backend(Box::new(e)))?;
            let kv = response.kvs().first().ok_or(ConfigError::NotFound)?;
            let raw = std::str::from_utf8(kv.value()).map_err(|e| ConfigError::Parse(e.to_string()))?;
            let expanded = interpolate_env_vars(raw);
            toml::from_str(&expanded).map_err(|e| ConfigError::Parse(e.to_string()))
        })
    }

    fn watch<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<mpsc::Receiver<ConfigEvent>, ConfigError>> + Send + 'a>> {
        let client = Arc::clone(&self.client);
        let key = self.key.clone();
        Box::pin(async move {
            let (config_tx, config_rx) = mpsc::channel::<ConfigEvent>(32);

            tokio::spawn(async move {
                etcd_watch_loop(client, key, config_tx).await;
            });

            Ok(config_rx)
        })
    }
}

#[cfg(feature = "etcd-watch")]
/// Internal: drive the etcd watch stream, reconnecting on interruption.
async fn etcd_watch_loop(
    client: Arc<tokio::sync::Mutex<etcd_client::Client>>,
    key: String,
    tx: mpsc::Sender<ConfigEvent>,
) {
    use etcd_client::WatchOptions;

    loop {
        // ~keep Recreate the etcd watch stream on every reconnect attempt.
        let mut stream = {
            let mut guard = client.lock().await;
            match guard.watch(key.as_str(), Some(WatchOptions::new().with_prefix())).await {
                Ok(stream) => stream,
                Err(err) => {
                    tracing::warn!("etcd watch connect failed: {err}; retrying in 5s");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            }
        };

        loop {
            match stream.message().await {
                Ok(Some(resp)) => {
                    for event in resp.events() {
                        use etcd_client::EventType;
                        let revision = event.kv().map(|kv| kv.mod_revision() as u64).unwrap_or(0);

                        match event.event_type() {
                            EventType::Put => {
                                if let Some(kv) = event.kv() {
                                    match std::str::from_utf8(kv.value()) {
                                        Ok(raw) => {
                                            let expanded = interpolate_env_vars(raw);
                                            match toml::from_str::<ProxyConfig>(&expanded) {
                                                Ok(config) => {
                                                    if tx.send(ConfigEvent::Put { revision, config }).await.is_err() {
                                                        let _ = stream.cancel(resp.watch_id()).await;
                                                        return;
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::warn!("etcd config parse error: {e}");
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!("etcd value is not valid UTF-8: {e}");
                                        }
                                    }
                                }
                            }
                            EventType::Delete => {
                                let path = event
                                    .kv()
                                    .map(|kv| String::from_utf8_lossy(kv.key()).into_owned())
                                    .unwrap_or_default();
                                if tx.send(ConfigEvent::Delete { revision, path }).await.is_err() {
                                    let _ = stream.cancel(resp.watch_id()).await;
                                    return;
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    // ~keep Clean watch termination still triggers reconnect and resync.
                    tracing::warn!("etcd watch stream ended; reconnecting");
                    break;
                }
                Err(err) => {
                    tracing::warn!("etcd watch stream error: {err}; reconnecting");
                    break;
                }
            }
        }

        // ~keep Interrupted etcd watches emit a fresh snapshot before reconnecting.
        let resync_config = {
            let mut guard = client.lock().await;
            guard
                .get(key.as_str(), None)
                .await
                .ok()
                .and_then(|resp| resp.kvs().first().cloned())
                .and_then(|kv| {
                    let raw = std::str::from_utf8(kv.value()).ok()?.to_owned();
                    let revision = kv.mod_revision() as u64;
                    let expanded = interpolate_env_vars(&raw);
                    let config = toml::from_str::<ProxyConfig>(&expanded).ok()?;
                    Some((revision, config))
                })
        };

        if let Some((revision, config)) = resync_config
            && tx.send(ConfigEvent::Resync { revision, config }).await.is_err()
        {
            return;
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

/// Load and parse a TOML file with env-var interpolation.
fn load_toml_file(path: &Path) -> Result<ProxyConfig, ConfigError> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ConfigError::NotFound
        } else {
            ConfigError::Backend(Box::new(e))
        }
    })?;
    let expanded = interpolate_env_vars(&raw);
    toml::from_str(&expanded).map_err(|e| ConfigError::Parse(e.to_string()))
}

/// Return the file's mtime as seconds since UNIX epoch, or `None` on error.
fn file_mtime_secs(path: &Path) -> Option<u64> {
    let metadata = std::fs::metadata(path).ok()?;
    let mtime = metadata.modified().ok()?;
    mtime.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    const MINIMAL_TOML: &str = r#"
[server]
host = "127.0.0.1"
port = 9000
"#;

    #[tokio::test]
    async fn config_provider_static_file_loads_once() {
        let mut file = NamedTempFile::new().expect("temp file");
        file.write_all(MINIMAL_TOML.as_bytes()).expect("write");
        file.flush().expect("flush");

        let provider = StaticFileConfigProvider::new(file.path());
        let config = provider.load().await.expect("load should succeed");

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9000);
    }

    #[tokio::test]
    async fn config_provider_static_file_watch_never_yields() {
        let mut file = NamedTempFile::new().expect("temp file");
        file.write_all(MINIMAL_TOML.as_bytes()).expect("write");

        let provider = StaticFileConfigProvider::new(file.path());
        let mut rx = provider.watch().await.expect("watch should succeed");

        let result = tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv()).await;
        assert!(
            result.is_err() || result.unwrap().is_none(),
            "static provider watch should never yield events"
        );
    }

    #[tokio::test]
    async fn config_provider_static_file_missing_returns_not_found() {
        let provider = StaticFileConfigProvider::new("/nonexistent/path/config.toml");
        let result = provider.load().await;
        assert!(result.is_err(), "missing file should error");
        let err = result.err().expect("is_err was asserted");
        assert!(matches!(err, ConfigError::NotFound), "expected NotFound, got: {err:?}");
    }

    #[tokio::test]
    async fn config_provider_static_file_invalid_toml_returns_parse_error() {
        let mut file = NamedTempFile::new().expect("temp file");
        file.write_all(b"not valid toml !!!").expect("write");

        let provider = StaticFileConfigProvider::new(file.path());
        let result = provider.load().await;
        assert!(result.is_err(), "invalid TOML should error");
        let err = result.err().expect("is_err was asserted");
        assert!(matches!(err, ConfigError::Parse(_)), "expected Parse, got: {err:?}");
    }

    #[tokio::test]
    async fn config_provider_file_watch_emits_event_on_save() {
        let mut file = NamedTempFile::new().expect("temp file");
        file.write_all(MINIMAL_TOML.as_bytes()).expect("write initial");
        file.flush().expect("flush initial");

        let provider = FileWatchConfigProvider::new(file.path().to_path_buf());

        let initial = provider.load().await.expect("initial load");
        assert_eq!(initial.server.port, 9000);

        let mut rx = provider.watch().await.expect("watch");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let updated_toml = r#"
[server]
host = "127.0.0.1"
port = 9001
"#;
        {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(file.path())
                .expect("open for write");
            f.write_all(updated_toml.as_bytes()).expect("write update");
            f.flush().expect("flush update");
        }

        let event = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv())
            .await
            .expect("event should arrive within 500ms")
            .expect("channel should not be closed");

        match event {
            ConfigEvent::Put { config, .. } => {
                assert_eq!(config.server.port, 9001, "reloaded config should reflect the new port");
            }
            other => panic!("expected Put event, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn config_provider_file_watch_loads_once() {
        let mut file = NamedTempFile::new().expect("temp file");
        file.write_all(MINIMAL_TOML.as_bytes()).expect("write");
        file.flush().expect("flush");

        let provider = FileWatchConfigProvider::new(file.path().to_path_buf());
        let config = provider.load().await.expect("load");
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9000);
    }

    #[cfg(feature = "etcd-watch")]
    #[test]
    fn config_provider_etcd_type_is_send_sync() {
        fn assert_send_sync<T: Send + Sync + 'static>() {}
        assert_send_sync::<EtcdConfigProvider>();
    }

    #[test]
    fn config_error_display() {
        assert_eq!(ConfigError::NotFound.to_string(), "configuration not found");
        assert_eq!(ConfigError::PermissionDenied.to_string(), "permission denied");
        assert_eq!(
            ConfigError::Parse("bad toml".into()).to_string(),
            "parse error: bad toml"
        );
    }
}
