//! Opt-in, air-gap-safe runtime catalog refresh.
//!
//! Callers can optionally download an updated `catalog.json` (our published
//! catalog artifact — see `DEFAULT_CATALOG_URL` — the same shape the crate
//! already parses via [`super::registry_from_catalog_str`]) and overlay it on
//! top of the embedded, compile-time catalog. Refresh is a runtime opt-in,
//! off by default ([`CatalogRefreshConfig::default`] has `enabled: false`):
//! with `enabled: false`, [`refresh_catalog`] is a no-op that touches neither
//! the network, the filesystem, nor the overlay. Any refresh failure — an
//! unreachable host, a firewall, or a fully air-gapped network with no cache
//! — leaves the previously active registry (a prior successful overlay, or
//! the embedded catalog) untouched. Callers in offline environments therefore
//! always have working pricing/model-info data, whether or not they ever
//! attempt a refresh.
//!
//! The network fetch requires the `native-http` feature. Without it the public
//! API is unchanged, but an enabled refresh that misses the on-disk cache
//! returns [`CatalogRefreshError::Fetch`] instead of fetching — again leaving
//! the overlay untouched, so the air-gap contract still holds.
//!
//! [`super::completion_cost`], [`super::completion_cost_with_cache`], and
//! [`super::model_info`] consult the overlay installed here first, falling
//! back to the embedded catalog. [`super::model_pricing`] is embedded-only
//! by design (see its doc comment) and never reflects this overlay.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::SystemTime;

use arc_swap::ArcSwapOption;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{CatalogInfo, ModelPricing, catalog_info_from_str, registry_from_catalog_str};

/// Default source for [`refresh_catalog`]: the rolling `model-catalog`
/// release asset published by this repository.
pub const DEFAULT_CATALOG_URL: &str =
    "https://github.com/xberg-io/liter-llm/releases/download/model-catalog/catalog.json";

/// Directory (under `std::env::temp_dir()`) used for the default on-disk
/// cache path.
const CACHE_DIR_NAME: &str = "liter-llm";
/// File name used for the default on-disk cache path.
const CACHE_FILE_NAME: &str = "catalog.json";
/// Network timeout for a catalog fetch, so a stalled connection can never
/// hang the caller's `refresh_catalog` future indefinitely.
#[cfg(feature = "native-http")]
const FETCH_TIMEOUT_SECS: u64 = 30;

/// Process-global overlay registry installed by a successful
/// [`refresh_catalog`] or [`install_catalog_overlay_from_str`] call.
///
/// `None` until the first successful install — the embedded catalog is used
/// exclusively until then. Never mutated on a failed refresh, so a prior
/// successful overlay (or the embedded catalog, if none has ever been
/// installed) always remains available; this is what makes runtime refresh
/// air-gap-safe.
static OVERLAY: LazyLock<ArcSwapOption<HashMap<String, ModelPricing>>> = LazyLock::new(|| ArcSwapOption::from(None));
static OVERLAY_INFO: LazyLock<ArcSwapOption<CatalogInfo>> = LazyLock::new(|| ArcSwapOption::from(None));

/// Read the current overlay registry, if one has been installed.
///
/// Returns a cheaply-cloned [`Arc`] (an atomic refcount bump), not a borrow
/// tied to the static, so callers can read through it without holding a lock
/// across `.await` points or blocking a concurrent install.
pub(crate) fn overlay_registry() -> Option<Arc<HashMap<String, ModelPricing>>> {
    OVERLAY.load_full()
}

pub(crate) fn overlay_catalog_info() -> Option<CatalogInfo> {
    OVERLAY_INFO.load_full().map(|info| (*info).clone())
}

/// Plain-data configuration for [`refresh_catalog`].
///
/// Deliberately FFI/binding-friendly: no `Duration` or `PathBuf`, just
/// primitives that translate directly across language boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogRefreshConfig {
    /// Runtime catalog refresh is entirely opt-in: when `false`,
    /// [`refresh_catalog`] is a no-op that returns
    /// `Ok(`[`RefreshOutcome::Disabled`]`)` without touching the network,
    /// the filesystem, or the overlay registry.
    pub enabled: bool,
    /// Source URL to fetch `catalog.json` from. Must be `https`. Defaults to
    /// [`DEFAULT_CATALOG_URL`]; configurable so self-hosted mirrors work.
    pub source_url: String,
    /// How long a cached `catalog.json` remains valid before a network
    /// refetch is attempted, in seconds.
    pub ttl_seconds: u64,
    /// Filesystem path for the on-disk cache. `None` uses a default path
    /// under the current user's platform cache directory.
    pub cache_path: Option<String>,
}

impl Default for CatalogRefreshConfig {
    fn default() -> Self {
        CatalogRefreshConfig {
            enabled: false,
            source_url: DEFAULT_CATALOG_URL.to_string(),
            ttl_seconds: 86_400,
            cache_path: None,
        }
    }
}

/// Result of a [`refresh_catalog`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefreshOutcome {
    /// `config.enabled` was `false`; no network, filesystem, or overlay
    /// activity occurred.
    Disabled,
    /// The on-disk cache was fresh (age < `ttl_seconds`); the overlay was
    /// installed from the cached file without a network request.
    FromCache,
    /// The catalog was fetched over the network, the cache file was
    /// (best-effort) refreshed, and the overlay was installed from the
    /// fetched catalog.
    Fetched,
}

/// Errors from [`refresh_catalog`] and [`install_catalog_overlay_from_str`].
///
/// On every variant, the overlay registry is left untouched: a previously
/// installed overlay (or the embedded catalog, if none was ever installed)
/// remains active. This is the air-gap-safety contract — a failed refresh
/// never degrades pricing/model-info availability.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CatalogRefreshError {
    /// Runtime catalog refresh was not enabled. [`refresh_catalog`] itself
    /// never returns this — it returns `Ok(`[`RefreshOutcome::Disabled`]`)`
    /// instead — but the variant is part of the public error surface for
    /// callers that want to treat "disabled" as a hard error.
    #[error("catalog refresh is disabled")]
    Disabled,
    /// `source_url` did not use the `https` scheme, or failed to parse as a
    /// URL at all. There is no host allowlist: the URL is user-configurable
    /// for self-hosted catalog mirrors, so only the scheme is enforced.
    #[error("insecure catalog source URL {url:?}: only https is allowed")]
    InsecureUrl {
        /// The rejected URL.
        url: String,
    },
    /// The network fetch failed, timed out, or returned a non-success
    /// status.
    #[error("failed to fetch catalog from {url}: {message}")]
    Fetch {
        /// The URL that was fetched.
        url: String,
        /// Human-readable failure detail.
        message: String,
    },
    /// The fetched or cached catalog JSON failed to parse.
    #[error("failed to parse catalog JSON: {message}")]
    Parse {
        /// Human-readable failure detail (from `serde_json`, via
        /// [`registry_from_catalog_str`]).
        message: String,
    },
    /// A cache file **read** failed on an otherwise-fresh cache file. Cache
    /// **write** failures are best-effort and never surface as this error
    /// (see [`refresh_catalog`]).
    #[error("catalog cache I/O error at {path}: {message}")]
    Cache {
        /// The cache file path.
        path: String,
        /// Human-readable detail from the underlying I/O error. A `String`
        /// (rather than a `#[source] std::io::Error`) so the error surface
        /// stays losslessly representable across every language binding.
        message: String,
    },
    /// Catalog bytes did not match the published SHA-256 checksum.
    #[error("catalog integrity check failed: {message}")]
    Integrity {
        /// Human-readable checksum failure detail.
        message: String,
    },
}

/// Install the overlay registry from a raw catalog JSON string, bypassing
/// the network and disk cache entirely.
///
/// Parses and flattens `catalog_json` with the same
/// [`registry_from_catalog_str`] logic used for the embedded catalog and the
/// network refresh path, then atomically swaps it in as the active overlay.
/// A parse failure returns [`CatalogRefreshError::Parse`] and leaves any
/// existing overlay untouched.
///
/// This is primarily a testable seam: it lets tests exercise overlay
/// installation and the embedded/overlay fallback behavior in
/// [`super::completion_cost`] / [`super::model_info`] without a real network
/// call.
pub fn install_catalog_overlay_from_str(catalog_json: &str) -> Result<(), CatalogRefreshError> {
    let registry = registry_from_catalog_str(catalog_json).map_err(|message| CatalogRefreshError::Parse { message })?;
    let info =
        catalog_info_from_str(catalog_json, "overlay").map_err(|message| CatalogRefreshError::Parse { message })?;
    OVERLAY.store(Some(Arc::new(registry)));
    OVERLAY_INFO.store(Some(Arc::new(info)));
    Ok(())
}

/// Clear the overlay registry, reverting [`super::completion_cost`],
/// [`super::completion_cost_with_cache`], and [`super::model_info`] to the
/// embedded catalog.
///
/// Primarily a test seam (see [`install_catalog_overlay_from_str`]); also
/// usable by long-running processes that want to abandon a runtime refresh.
pub fn clear_catalog_overlay() {
    OVERLAY.store(None);
    OVERLAY_INFO.store(None);
}

/// Default on-disk cache path in the current user's platform cache directory.
fn default_cache_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    let root = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library").join("Caches"));
    #[cfg(target_os = "windows")]
    let root = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let root = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).or_else(|| {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(".cache"))
    });

    root.unwrap_or_else(|| PathBuf::from(".cache"))
        .join(CACHE_DIR_NAME)
        .join(CACHE_FILE_NAME)
}

fn checksum_path(cache_path: &Path) -> PathBuf {
    let mut path = cache_path.as_os_str().to_owned();
    path.push(".sha256");
    PathBuf::from(path)
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn parse_checksum(raw: &str) -> Result<&str, CatalogRefreshError> {
    let checksum = raw.split_whitespace().next().unwrap_or_default();
    if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CatalogRefreshError::Integrity {
            message: "published checksum is not a 64-character SHA-256 value".to_string(),
        });
    }
    Ok(checksum)
}

fn verify_checksum(bytes: &[u8], checksum_raw: &str) -> Result<(), CatalogRefreshError> {
    let expected = parse_checksum(checksum_raw)?;
    let actual = sha256_hex(bytes);
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(CatalogRefreshError::Integrity {
            message: format!("expected {expected}, computed {actual}"),
        })
    }
}

fn write_private_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
            let mut builder = std::fs::DirBuilder::new();
            builder.recursive(true).mode(0o700).create(parent)?;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
        #[cfg(not(unix))]
        std::fs::create_dir_all(parent)?;
    }

    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    std::fs::rename(temporary, path)
}

/// Resolve the effective cache path for `config`.
fn cache_path_for(config: &CatalogRefreshConfig) -> PathBuf {
    config
        .cache_path
        .as_ref()
        .map_or_else(default_cache_path, PathBuf::from)
}

/// `true` when `path` exists and its last-modified time is less than
/// `ttl_seconds` old. Any I/O error (missing file, unreadable metadata,
/// clock skew) is treated as "not fresh" so the caller falls through to a
/// network fetch rather than failing outright.
fn cache_is_fresh(path: &Path, ttl_seconds: u64) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    let Ok(age) = SystemTime::now().duration_since(modified) else {
        return false;
    };
    age.as_secs() < ttl_seconds
}

/// Refresh the runtime catalog overlay per `config`.
///
/// - `config.enabled == false`: returns `Ok(`[`RefreshOutcome::Disabled`]`)`
///   immediately. No network, filesystem, or overlay activity.
/// - A fresh on-disk cache (age < `config.ttl_seconds`) exists at the
///   resolved cache path (`config.cache_path`, or a default under
///   `std::env::temp_dir()`): read + flatten it and install the overlay,
///   returning `Ok(`[`RefreshOutcome::FromCache`]`)`. No network request is
///   made.
/// - Otherwise: validate `config.source_url` uses `https`
///   ([`CatalogRefreshError::InsecureUrl`] otherwise), fetch it, flatten it,
///   install the overlay, best-effort write the raw JSON to the cache path
///   (a cache write failure does not fail the refresh), and return
///   `Ok(`[`RefreshOutcome::Fetched`]`)`.
///
/// On any error return, the overlay is left untouched: the previously
/// active registry (a prior successful overlay, or the embedded catalog if
/// none was ever installed) remains in effect. This is what makes the
/// feature air-gap-safe — an unreachable or invalid `source_url` never
/// degrades `completion_cost` / `model_info` below embedded-catalog
/// availability.
#[tracing::instrument(level = "info", skip_all, fields(source_url = %config.source_url))]
pub async fn refresh_catalog(config: &CatalogRefreshConfig) -> Result<RefreshOutcome, CatalogRefreshError> {
    if !config.enabled {
        return Ok(RefreshOutcome::Disabled);
    }

    let cache_path = cache_path_for(config);

    if cache_is_fresh(&cache_path, config.ttl_seconds) {
        return refresh_from_cache(&cache_path);
    }

    refresh_from_network(config, &cache_path).await
}

/// [`refresh_catalog`]'s cache-hit path: read, parse, and install the
/// overlay from an already-fresh cache file.
fn refresh_from_cache(cache_path: &Path) -> Result<RefreshOutcome, CatalogRefreshError> {
    let raw = std::fs::read(cache_path).map_err(|source| CatalogRefreshError::Cache {
        path: cache_path.display().to_string(),
        message: source.to_string(),
    })?;
    let checksum_file = checksum_path(cache_path);
    let checksum = std::fs::read_to_string(&checksum_file).map_err(|source| CatalogRefreshError::Cache {
        path: checksum_file.display().to_string(),
        message: source.to_string(),
    })?;
    verify_checksum(&raw, &checksum)?;
    let raw = std::str::from_utf8(&raw).map_err(|error| CatalogRefreshError::Parse {
        message: error.to_string(),
    })?;
    let registry = registry_from_catalog_str(raw).map_err(|message| CatalogRefreshError::Parse { message })?;
    let info = catalog_info_from_str(raw, "overlay").map_err(|message| CatalogRefreshError::Parse { message })?;
    OVERLAY.store(Some(Arc::new(registry)));
    OVERLAY_INFO.store(Some(Arc::new(info)));
    Ok(RefreshOutcome::FromCache)
}

/// [`refresh_catalog`]'s network path: validate the URL, fetch, parse,
/// install the overlay, and best-effort refresh the on-disk cache.
///
/// Requires the `native-http` feature (for `reqwest`). See the
/// `#[cfg(not(feature = "native-http"))]` sibling below for the no-network
/// build.
#[cfg(feature = "native-http")]
async fn refresh_from_network(
    config: &CatalogRefreshConfig,
    cache_path: &Path,
) -> Result<RefreshOutcome, CatalogRefreshError> {
    let url = reqwest::Url::parse(&config.source_url).map_err(|_| CatalogRefreshError::InsecureUrl {
        url: config.source_url.clone(),
    })?;
    if url.scheme() != "https" {
        return Err(CatalogRefreshError::InsecureUrl {
            url: config.source_url.clone(),
        });
    }

    // ~keep Non-Windows: installs the `ring` rustls crypto provider once per
    // ~keep process (idempotent). Windows: no-op (native-tls/SChannel). Mirrors
    // ~keep every other internal `reqwest::Client` constructor in this crate.
    crate::ensure_crypto_provider();

    let fetch_err = |message: String| CatalogRefreshError::Fetch {
        url: config.source_url.clone(),
        message,
    };

    crate::provider::validate_outbound_url(url.as_str())
        .await
        .map_err(|error| fetch_err(error.to_string()))?;
    let builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(FETCH_TIMEOUT_SECS));
    let client = crate::provider::configure_credential_free_outbound_client_builder(builder)
        .build()
        .map_err(|e| fetch_err(e.to_string()))?;

    let checksum_url = format!("{}.sha256", url.as_str());
    crate::provider::validate_outbound_url(&checksum_url)
        .await
        .map_err(|error| fetch_err(error.to_string()))?;
    let response = client.get(url).send().await.map_err(|e| fetch_err(e.to_string()))?;
    let response = response.error_for_status().map_err(|e| fetch_err(e.to_string()))?;
    let raw = response.text().await.map_err(|e| fetch_err(e.to_string()))?;
    let checksum_response = client
        .get(&checksum_url)
        .send()
        .await
        .map_err(|e| fetch_err(e.to_string()))?;
    let checksum = checksum_response
        .error_for_status()
        .map_err(|e| fetch_err(e.to_string()))?
        .text()
        .await
        .map_err(|e| fetch_err(e.to_string()))?;
    verify_checksum(raw.as_bytes(), &checksum)?;

    let registry = registry_from_catalog_str(&raw).map_err(|message| CatalogRefreshError::Parse { message })?;
    let info = catalog_info_from_str(&raw, "overlay").map_err(|message| CatalogRefreshError::Parse { message })?;

    // ~keep Best-effort cache write: a failure here must not fail the refresh —
    // ~keep the overlay install below is the operation that matters, and it is
    // ~keep performed either way.
    if let Err(error) = write_private_file(cache_path, raw.as_bytes()) {
        tracing::warn!(%error, path = %cache_path.display(), "failed to write catalog cache file");
    }
    let checksum_path = checksum_path(cache_path);
    if let Err(error) = write_private_file(&checksum_path, checksum.as_bytes()) {
        tracing::warn!(%error, path = %checksum_path.display(), "failed to write catalog checksum file");
    }

    OVERLAY.store(Some(Arc::new(registry)));
    OVERLAY_INFO.store(Some(Arc::new(info)));
    Ok(RefreshOutcome::Fetched)
}

/// No-network build of [`refresh_catalog`]'s network path.
///
/// Without the `native-http` feature there is no HTTP client to fetch with, so
/// an enabled refresh that misses the on-disk cache fails cleanly with
/// [`CatalogRefreshError::Fetch`]. The overlay is left untouched, so the
/// embedded catalog (or a prior cache-installed overlay) stays in effect and
/// the air-gap contract holds.
#[cfg(not(feature = "native-http"))]
async fn refresh_from_network(
    config: &CatalogRefreshConfig,
    _cache_path: &Path,
) -> Result<RefreshOutcome, CatalogRefreshError> {
    Err(CatalogRefreshError::Fetch {
        url: config.source_url.clone(),
        message: "network catalog refresh requires the `native-http` feature".to_string(),
    })
}
