---
description: "Opt-in runtime refresh of liter-llm's model catalog from a downloadable catalog.json."
title: "Catalog Refresh"
---

Runtime catalog refresh lets a running process download an updated `catalog.json` and overlay it on top of the embedded catalog described in [Cost Estimation](/concepts/cost-estimation/), without a rebuild. It is a runtime option — no Cargo feature to enable — and off by default: nothing happens until you pass a config with `enabled: true`.

:::warning
Runtime refresh is opt-in and disabled by default (`enabled: false`). When enabled and the on-disk cache is stale, the process always attempts an outbound HTTPS request — it does not detect or skip air-gapped environments. A failed or unreachable request falls back to the embedded/prior catalog, so a refresh failure never degrades availability — see [Air-gap guarantee](#air-gap-guarantee) below.
:::

## Availability

The refresh API is part of the default surface across every language binding — there is no Cargo feature or extra dependency to turn on. The network fetch uses the crate's native HTTP client; on builds without it (the WebAssembly binding), an enabled refresh that misses the on-disk cache returns a fetch error and the embedded catalog stays active.

## Usage

```rust
use liter_llm::cost::{refresh_catalog, CatalogRefreshConfig, RefreshOutcome};

let config = CatalogRefreshConfig {
    enabled: true,
    ..Default::default()
};

match refresh_catalog(&config).await {
    Ok(RefreshOutcome::Fetched) => println!("catalog fetched over the network"),
    Ok(RefreshOutcome::FromCache) => println!("catalog loaded from a fresh on-disk cache"),
    Ok(RefreshOutcome::Disabled) => println!("refresh is disabled; using the embedded catalog"),
    Err(error) => eprintln!("refresh failed, embedded/prior catalog still active: {error}"),
}

// completion_cost and model_info now consult the overlay, falling back to
// the embedded catalog for anything the overlay doesn't cover.
let usd = liter_llm::cost::completion_cost("gpt-4o", 1_000, 500);
```

## `CatalogRefreshConfig`

```rust
pub struct CatalogRefreshConfig {
    pub enabled: bool,           // default: false
    pub source_url: String,      // default: DEFAULT_CATALOG_URL
    pub ttl_seconds: u64,        // default: 86_400 (24h)
    pub cache_path: Option<String>, // default: None (<temp_dir>/liter-llm/catalog.json)
}
```

| Field         | Default                                                                                 | Notes                                                                 |
| ------------- | ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `enabled`     | `false`                                                                                   | When `false`, `refresh_catalog` is a no-op: no network, filesystem, or overlay activity. |
| `source_url`  | `https://github.com/xberg-io/liter-llm/releases/download/model-catalog/catalog.json`     | Must use `https`; the rolling `model-catalog` release artifact by default. |
| `ttl_seconds` | `86400`                                                                                   | Cache age threshold before a network refetch is attempted.             |
| `cache_path`  | `None` (resolves to `<temp_dir>/liter-llm/catalog.json`)                                 | Override to control where the fetched catalog is cached on disk.       |

`CatalogRefreshConfig` is plain data — no `Duration` or `PathBuf` fields — so it translates directly across FFI and language bindings.

:::tip
Set `source_url` to a self-hosted mirror if your deployment can't reach `github.com`, e.g. an internal artifact proxy serving the same `catalog.json` shape.
:::

## `refresh_catalog`

`refresh_catalog(config: &CatalogRefreshConfig) -> Result<RefreshOutcome, CatalogRefreshError>` resolves in one of three ways:

1. **Disabled** — `config.enabled == false`. Returns `Ok(RefreshOutcome::Disabled)` immediately.
2. **From cache** — the on-disk cache at `cache_path` exists and is younger than `ttl_seconds`. Reads and installs it without a network request. Returns `Ok(RefreshOutcome::FromCache)`.
3. **Fetched** — otherwise, validates `source_url` is `https` (`CatalogRefreshError::InsecureUrl` if not), fetches it, installs the overlay, and best-effort writes it to the cache path. A cache *write* failure never fails the refresh. Returns `Ok(RefreshOutcome::Fetched)`.

## Air-gap guarantee

On every error path — an unreachable host, a firewall, a fully air-gapped network with no cache, or a cache read failure — the overlay registry is left **untouched**. The previously active data (a prior successful overlay, or the embedded catalog if no overlay has ever installed successfully) remains in effect. A failed refresh never clears active pricing/model-info data and never degrades availability below the embedded catalog.

This means callers in offline or restricted environments can call `refresh_catalog` unconditionally — with `enabled: true` or `enabled: false` — and always get working `completion_cost` / `model_info` results.

## What reflects the refresh

- [`cost::completion_cost`](/concepts/cost-estimation/) and `cost::completion_cost_with_cache` consult the installed overlay first, falling back to the embedded catalog for models the overlay doesn't cover.
- `cost::model_info` behaves the same way.
- `cost::model_pricing` is **embedded-only by design** and never reflects the overlay, even after a successful refresh.

## Testing without the network

`install_catalog_overlay_from_str(catalog_json: &str)` installs an overlay directly from a JSON string, bypassing the network and disk cache — useful for tests that exercise the overlay/embedded fallback behavior deterministically. `clear_catalog_overlay()` reverts to the embedded catalog.
