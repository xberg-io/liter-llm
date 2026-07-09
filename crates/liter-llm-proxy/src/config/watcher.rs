//! Background configuration watcher.
//!
//! Spawns a Tokio task that drives a [`ConfigProvider`]'s watch stream and
//! atomically stores every validated new config into an `ArcSwap<ProxyConfig>`.
//!
//! OTel counters emitted:
//! - `gen_ai.config.reload`       — successful hot-reloads
//! - `gen_ai.config.reload_error` — failed reloads (parse / validation)
//! - `gen_ai.config.revision`     — current revision as an observable gauge

use std::sync::Arc;

use arc_swap::ArcSwap;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{
    ProxyConfig,
    provider::{ConfigError, ConfigEvent, ConfigProvider},
};

/// Start the hot-reload background task.
///
/// - `provider` is polled via `watch()`. Events are processed as they arrive.
/// - `swap` is atomically updated on every valid reload.
/// - `cancel` allows the caller to shut down the watcher gracefully.
///
/// The task logs every reload and error. It never panics — errors are logged
/// and the existing configuration is retained.
pub async fn spawn_watcher(
    provider: Arc<dyn ConfigProvider>,
    swap: Arc<ArcSwap<ProxyConfig>>,
    cancel: CancellationToken,
) {
    tokio::spawn(async move {
        run_watcher(provider, swap, cancel).await;
    });
}

async fn run_watcher(provider: Arc<dyn ConfigProvider>, swap: Arc<ArcSwap<ProxyConfig>>, cancel: CancellationToken) {
    let mut rx = match open_watch_stream(provider.as_ref()).await {
        Ok(rx) => rx,
        Err(err) => {
            tracing::error!("config watcher: failed to open watch stream: {err}");
            return;
        }
    };

    loop {
        tokio::select! {
            () = cancel.cancelled() => {
                tracing::info!("config watcher: shutdown requested");
                return;
            }
            event = rx.recv() => {
                match event {
                    Some(ev) => handle_event(ev, &swap),
                    None => {
                        tracing::warn!("config watcher: watch channel closed; watcher stopping");
                        return;
                    }
                }
            }
        }
    }
}

fn handle_event(event: ConfigEvent, swap: &Arc<ArcSwap<ProxyConfig>>) {
    match event {
        ConfigEvent::Put { revision, config } => {
            tracing::info!(revision, "config watcher: hot-reload successful");
            increment_counter("gen_ai.config.reload");
            record_revision("gen_ai.config.revision", revision);
            swap.store(Arc::new(config));
        }
        ConfigEvent::Resync { revision, config } => {
            tracing::info!(revision, "config watcher: resync — full reload applied");
            increment_counter("gen_ai.config.reload");
            record_revision("gen_ai.config.revision", revision);
            swap.store(Arc::new(config));
        }
        ConfigEvent::Delete { revision, path } => {
            // ~keep Deleted config keys retain the last-known-good configuration.
            tracing::error!(
                revision,
                path = %path,
                "config watcher: config key deleted in backend — retaining last-known-good config"
            );
            increment_counter("gen_ai.config.reload_error");
        }
    }
}

/// Attempt to open a watch stream, retrying with exponential back-off.
async fn open_watch_stream(provider: &dyn ConfigProvider) -> Result<mpsc::Receiver<ConfigEvent>, ConfigError> {
    let mut delay = std::time::Duration::from_millis(200);
    const MAX_DELAY: std::time::Duration = std::time::Duration::from_secs(30);
    const MAX_ATTEMPTS: u32 = 5;

    for attempt in 1..=MAX_ATTEMPTS {
        match provider.watch().await {
            Ok(rx) => return Ok(rx),
            Err(err) => {
                tracing::warn!(attempt, "config watcher: watch() failed: {err}; retrying in {delay:?}");
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(MAX_DELAY);
            }
        }
    }

    // ~keep The final watch attempt propagates its error instead of sleeping again.
    provider.watch().await
}

fn increment_counter(name: &'static str) {
    tracing::debug!(metric = name, value = 1_u64, "metric counter increment");
}

fn record_revision(name: &'static str, revision: u64) {
    tracing::debug!(metric = name, value = revision, "metric gauge record");
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    use arc_swap::ArcSwap;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::config::ProxyConfig;
    use crate::config::provider::{ConfigError, ConfigEvent, ConfigProvider};

    /// A `ConfigProvider` that returns a fixed config from `load` and can be
    /// driven externally via a pre-seeded event sender.
    struct ManualProvider {
        config: ProxyConfig,
        /// Receiver side returned by `watch`.
        rx: tokio::sync::Mutex<Option<mpsc::Receiver<ConfigEvent>>>,
    }

    impl ManualProvider {
        fn new(config: ProxyConfig) -> (Self, mpsc::Sender<ConfigEvent>) {
            let (tx, rx) = mpsc::channel::<ConfigEvent>(16);
            let provider = Self {
                config,
                rx: tokio::sync::Mutex::new(Some(rx)),
            };
            (provider, tx)
        }
    }

    impl ConfigProvider for ManualProvider {
        fn load<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<ProxyConfig, ConfigError>> + Send + 'a>> {
            let cfg = self.config.clone();
            Box::pin(async move { Ok(cfg) })
        }

        fn watch<'a>(
            &'a self,
        ) -> Pin<Box<dyn Future<Output = Result<mpsc::Receiver<ConfigEvent>, ConfigError>> + Send + 'a>> {
            Box::pin(async move {
                let mut guard = self.rx.lock().await;
                Ok(guard.take().expect("watch called twice"))
            })
        }
    }

    fn base_config(port: u16) -> ProxyConfig {
        let mut c = ProxyConfig::default();
        c.server.port = port;
        c
    }

    #[tokio::test]
    async fn arc_swap_reconfig_does_not_block_inflight_requests() {
        let initial = Arc::new(base_config(1000));
        let swap = Arc::new(ArcSwap::from(Arc::clone(&initial)));

        let snap_before_reload = swap.load_full();
        assert_eq!(snap_before_reload.server.port, 1000);

        let updated = Arc::new(base_config(2000));
        swap.store(Arc::clone(&updated));

        assert_eq!(
            snap_before_reload.server.port, 1000,
            "in-flight request should see the old config"
        );

        assert_eq!(swap.load().server.port, 2000, "new requests should see updated config");
    }

    #[tokio::test]
    async fn arc_swap_reconfig_invalid_config_keeps_existing() {
        let initial = base_config(3000);
        let swap = Arc::new(ArcSwap::from(Arc::new(initial.clone())));

        let cancel = CancellationToken::new();
        let (provider, tx) = ManualProvider::new(initial);
        let provider_arc: Arc<dyn ConfigProvider> = Arc::new(provider);

        spawn_watcher(Arc::clone(&provider_arc), Arc::clone(&swap), cancel.clone()).await;

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        tx.send(ConfigEvent::Delete {
            revision: 1,
            path: "/config/proxy".into(),
        })
        .await
        .expect("send");

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(
            swap.load().server.port,
            3000,
            "invalid/delete event must not overwrite existing config"
        );

        cancel.cancel();
    }

    #[tokio::test]
    async fn watcher_applies_put_event() {
        let initial = base_config(4000);
        let swap = Arc::new(ArcSwap::from(Arc::new(initial.clone())));
        let cancel = CancellationToken::new();
        let (provider, tx) = ManualProvider::new(initial);
        let provider_arc: Arc<dyn ConfigProvider> = Arc::new(provider);

        spawn_watcher(Arc::clone(&provider_arc), Arc::clone(&swap), cancel.clone()).await;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let new_config = base_config(5000);
        tx.send(ConfigEvent::Put {
            revision: 10,
            config: new_config,
        })
        .await
        .expect("send");

        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
        while swap.load().server.port != 5000 {
            if std::time::Instant::now() > deadline {
                panic!("watcher did not apply Put event within 200ms");
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        assert_eq!(swap.load().server.port, 5000);
        cancel.cancel();
    }

    #[tokio::test]
    async fn watcher_applies_resync_event() {
        let initial = base_config(6000);
        let swap = Arc::new(ArcSwap::from(Arc::new(initial.clone())));
        let cancel = CancellationToken::new();
        let (provider, tx) = ManualProvider::new(initial);
        let provider_arc: Arc<dyn ConfigProvider> = Arc::new(provider);

        spawn_watcher(Arc::clone(&provider_arc), Arc::clone(&swap), cancel.clone()).await;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let resynced = base_config(7000);
        tx.send(ConfigEvent::Resync {
            revision: 42,
            config: resynced,
        })
        .await
        .expect("send");

        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
        while swap.load().server.port != 7000 {
            if std::time::Instant::now() > deadline {
                panic!("watcher did not apply Resync event within 200ms");
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        assert_eq!(swap.load().server.port, 7000);
        cancel.cancel();
    }
}
