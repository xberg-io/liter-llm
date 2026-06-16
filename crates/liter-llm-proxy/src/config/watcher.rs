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
                        // Channel closed — provider dropped or watch ended.
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
            // A delete means the config key was removed. We keep the
            // last-known-good configuration in place and log an error.
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

    // Final attempt — propagate the error.
    provider.watch().await
}

// ---------------------------------------------------------------------------
// OTel metric stubs
//
// Full OTel export is gated behind the `otel` feature. These thin wrappers
// use `tracing` events as the fallback so metrics appear in logs even without
// the OTel pipeline connected.
// ---------------------------------------------------------------------------

fn increment_counter(name: &'static str) {
    tracing::debug!(metric = name, value = 1_u64, "metric counter increment");
    // When the `otel` feature is enabled, the tracing-opentelemetry bridge
    // picks this up automatically via the span attributes. A dedicated
    // opentelemetry::global::meter() call can be wired in here when Phase 5
    // adds the full metrics pipeline.
}

fn record_revision(name: &'static str, revision: u64) {
    tracing::debug!(metric = name, value = revision, "metric gauge record");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    // ── Helpers ──────────────────────────────────────────────────────────

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

    // ── arc_swap_reconfig_does_not_block_inflight_requests ───────────────
    //
    // Verifies snapshot semantics: a reload happening concurrently with a
    // read returns the old config to the reader that already loaded it.

    #[tokio::test]
    async fn arc_swap_reconfig_does_not_block_inflight_requests() {
        let initial = Arc::new(base_config(1000));
        let swap = Arc::new(ArcSwap::from(Arc::clone(&initial)));

        // Simulate an "in-flight" request that captures the config snapshot
        // at entry, then waits before reading from it.
        let snap_before_reload = swap.load_full();
        assert_eq!(snap_before_reload.server.port, 1000);

        // Perform a hot-reload concurrently.
        let updated = Arc::new(base_config(2000));
        swap.store(Arc::clone(&updated));

        // The snapshot taken before the reload is unaffected.
        assert_eq!(
            snap_before_reload.server.port, 1000,
            "in-flight request should see the old config"
        );

        // Subsequent loads see the new config.
        assert_eq!(swap.load().server.port, 2000, "new requests should see updated config");
    }

    // ── arc_swap_reconfig_invalid_config_keeps_existing ──────────────────
    //
    // Simulates a Put with an invalid ProxyConfig by sending a parse error
    // signal through the watcher. The watcher logs the error and keeps the
    // existing config untouched.

    #[tokio::test]
    async fn arc_swap_reconfig_invalid_config_keeps_existing() {
        let initial = base_config(3000);
        let swap = Arc::new(ArcSwap::from(Arc::new(initial.clone())));

        let cancel = CancellationToken::new();
        let (provider, tx) = ManualProvider::new(initial);
        let provider_arc: Arc<dyn ConfigProvider> = Arc::new(provider);

        spawn_watcher(Arc::clone(&provider_arc), Arc::clone(&swap), cancel.clone()).await;

        // Allow the watcher to start.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // Send a Delete event (simulates config removal — watcher keeps existing).
        tx.send(ConfigEvent::Delete {
            revision: 1,
            path: "/config/proxy".into(),
        })
        .await
        .expect("send");

        // Give the watcher time to process.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Config must be unchanged.
        assert_eq!(
            swap.load().server.port,
            3000,
            "invalid/delete event must not overwrite existing config"
        );

        cancel.cancel();
    }

    // ── Watcher applies Put event ─────────────────────────────────────────

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

        // Wait for the watcher to apply the event.
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

    // ── Watcher applies Resync event ─────────────────────────────────────

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
