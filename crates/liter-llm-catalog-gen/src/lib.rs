//! Dev tool: regenerates liter-llm's `schemas/catalog.json` (and its crate
//! mirror) from the [models.dev](https://models.dev) unified model catalog.
//!
//! This crate is not part of liter-llm's public API surface — it is a
//! standalone data generator, invoked via `task generate:catalog` /
//! `task generate:catalog:check`. `catalog.json` is a nested
//! `provider -> model` document: every model in the upstream catalog is
//! represented, and unpriced models simply omit their `pricing` sub-object.
//! See `crates/liter-llm-catalog-gen/src/render.rs` for the exact wire shape
//! this crate emits.
//!
//! Pipeline: [`fetch_catalog_text`] (or a local `--input` file) ->
//! [`parse_and_validate`] -> [`transform::transform_catalog`] ->
//! [`render::render_document`].

pub mod error;
pub mod render;
pub mod schema;
pub mod transform;

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

pub use error::CatalogGenError;
pub use schema::Catalog;
pub use transform::CatalogDocument;

/// Default upstream source URL.
pub const DEFAULT_MODELS_DEV_URL: &str = "https://models.dev/api.json";

/// Hosts the generator is permitted to fetch from. Restricting the network
/// fetch to the canonical upstream means a misconfigured or hostile `--url`
/// (e.g. if this task is ever wired into CI with an externally-influenced
/// argument) cannot redirect the generator at an arbitrary or internal host.
/// Local/alternate catalogs are supplied via `--input <file>` instead, which
/// bypasses the network entirely.
pub const ALLOWED_FETCH_HOSTS: [&str; 1] = ["models.dev"];

/// User-Agent sent on the outbound fetch request.
pub const USER_AGENT: &str = "liter-llm-catalog-gen/1.0";

/// Install the `ring` crypto provider as the rustls process default,
/// idempotently. Non-Windows builds pull in `reqwest`'s `rustls-no-provider`
/// feature (matching the rest of the workspace's TLS convention — see
/// `crates/liter-llm/src/lib.rs::ensure_crypto_provider`), which requires an
/// explicit provider install before the first TLS handshake. Windows uses
/// `native-tls` (SChannel) instead, so this is a no-op there.
#[cfg(not(target_os = "windows"))]
fn ensure_crypto_provider() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // ~keep Ignore `install_default` errors; a prior install (e.g. from a
        // ~keep dependency's own init) is a harmless race, not a failure.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[cfg(target_os = "windows")]
fn ensure_crypto_provider() {}

/// Fetch the upstream catalog as raw JSON text over HTTPS.
///
/// Refuses any URL that does not start with `https://` — this generator is
/// never meant to fetch plaintext HTTP, even for local testing (use
/// `--input` with a local file for that instead).
pub async fn fetch_catalog_text(url: &str) -> Result<String, CatalogGenError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| CatalogGenError::InsecureUrl(url.to_string()))?;
    if parsed.scheme() != "https" {
        return Err(CatalogGenError::InsecureUrl(url.to_string()));
    }
    let host = parsed.host_str().unwrap_or_default();
    if !ALLOWED_FETCH_HOSTS.contains(&host) {
        return Err(CatalogGenError::DisallowedHost {
            host: host.to_string(),
            allowed: ALLOWED_FETCH_HOSTS.join(", "),
        });
    }
    ensure_crypto_provider();
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|source| CatalogGenError::Fetch {
            url: url.to_string(),
            source,
        })?;
    let response = client.get(url).send().await.map_err(|source| CatalogGenError::Fetch {
        url: url.to_string(),
        source,
    })?;
    if !response.status().is_success() {
        return Err(CatalogGenError::FetchStatus {
            url: url.to_string(),
            status: response.status().as_u16(),
        });
    }
    response.text().await.map_err(|source| CatalogGenError::Fetch {
        url: url.to_string(),
        source,
    })
}

/// Parse raw catalog JSON text and run structural validation
/// ([`schema::validate_catalog`]).
///
/// Deserialization failures (including `#[serde(deny_unknown_fields)]`
/// hits, which signal upstream schema drift) and validation failures both
/// surface as [`CatalogGenError`] with enough context to name the offending
/// provider/model/field.
pub fn parse_and_validate(text: &str) -> Result<Catalog, CatalogGenError> {
    let catalog: Catalog = serde_json::from_str(text)?;
    schema::validate_catalog(&catalog)?;
    Ok(catalog)
}

/// The `$provenance` block embedded at the top of `catalog.json`: the
/// SHA-256 of the exact upstream bytes that were fetched/loaded, the fetch
/// date, and this crate's own package version (which tracks the workspace's
/// single-source-of-truth `Cargo.toml` version).
///
/// Excluded from the [`stale_paths`] drift comparison — it is per-run data
/// (the fetch date changes daily even when the underlying catalog does not),
/// not part of the catalog's substantive content. The publish workflow
/// separately injects `generated_by_commit`/`release_tag` into the released
/// artifact copy; this generator never writes those fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Provenance {
    /// The upstream source identifier, always `"models.dev/api.json"`.
    pub source: String,
    /// Hex-encoded SHA-256 digest of the exact fetched/loaded catalog bytes.
    pub source_sha256: String,
    /// The fetch date, `YYYY-MM-DD`.
    pub fetched: String,
    /// This crate's package version (`CARGO_PKG_VERSION`), which mirrors the
    /// workspace `Cargo.toml` version.
    pub library_version: String,
}

/// The canonical upstream source identifier recorded in `$provenance.source`.
pub const PROVENANCE_SOURCE: &str = "models.dev/api.json";

/// Build the `$provenance` block, hashing `raw_text` (the exact
/// fetched/loaded bytes) and recording `fetched_on` as the fetch date.
pub fn build_provenance(raw_text: &str, fetched_on: &str) -> Provenance {
    let mut hasher = Sha256::new();
    hasher.update(raw_text.as_bytes());
    let digest = hasher.finalize();
    Provenance {
        source: PROVENANCE_SOURCE.to_string(),
        source_sha256: hex::encode(digest),
        fetched: fetched_on.to_string(),
        library_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Run the full transform + render pipeline over an already-validated
/// catalog, producing the exact `catalog.json` document text.
pub fn build_catalog_document(catalog: &Catalog, provenance: &Provenance) -> Result<String, CatalogGenError> {
    let document: CatalogDocument = transform::transform_catalog(catalog);
    render::render_document(&document, provenance)
}

/// The two dual-write output locations for `catalog.json`, relative to a
/// repository root: the canonical `schemas/` copy and its mirror under the
/// `liter-llm` crate (so the crate can `include_str!` it without a build
/// script reaching outside `CARGO_MANIFEST_DIR`).
pub fn default_output_paths(root: &Path) -> [PathBuf; 2] {
    [
        root.join("schemas/catalog.json"),
        root.join("crates/liter-llm/schemas/catalog.json"),
    ]
}

/// Write `content` to `path`, creating parent directories as needed.
pub fn write_document(path: &Path, content: &str) -> Result<(), CatalogGenError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| CatalogGenError::WriteFile {
            path: path.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(path, content).map_err(|source| CatalogGenError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

/// Compare each of `paths` against `fresh_document`, returning the subset
/// that are stale (missing, unparsable, or structurally different).
///
/// Only the `providers` payload and `$schema_version` are compared — never
/// `$provenance`, which embeds the fetch date and would otherwise make every
/// CI run report drift even when the underlying catalog data has not
/// changed. An empty result means every path is up to date.
pub fn stale_paths(paths: &[PathBuf], fresh_document: &str) -> Result<Vec<PathBuf>, CatalogGenError> {
    let fresh_value: serde_json::Value = serde_json::from_str(fresh_document).map_err(CatalogGenError::RoundTrip)?;

    let stale = paths
        .iter()
        .filter(|path| {
            let is_current = std::fs::read_to_string(path)
                .ok()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
                .is_some_and(|existing| {
                    existing.get("providers") == fresh_value.get("providers")
                        && existing.get("$schema_version") == fresh_value.get("$schema_version")
                });
            !is_current
        })
        .cloned()
        .collect();
    Ok(stale)
}
