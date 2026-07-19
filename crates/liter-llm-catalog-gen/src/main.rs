//! CLI entry point for `liter-llm-catalog-gen`.
//!
//! ```text
//! liter-llm-catalog-gen             # fetch + write both catalog.json files
//! liter-llm-catalog-gen --dry-run   # fetch + print to stdout, write nothing
//! liter-llm-catalog-gen --validate  # CI: fail if committed files are stale
//! liter-llm-catalog-gen --input path/to/api.json   # skip the network fetch
//! ```

use std::path::{Path, PathBuf};

use clap::Parser;
use liter_llm_catalog_gen::{
    CatalogGenError, DEFAULT_MODELS_DEV_URL, build_catalog_document, build_provenance, default_output_paths,
    fetch_catalog_text, parse_and_validate, stale_paths, write_document,
};

/// Regenerate liter-llm's `catalog.json` from the models.dev catalog.
#[derive(Debug, Parser)]
#[command(name = "liter-llm-catalog-gen", version, about)]
struct Cli {
    /// Source URL to fetch the catalog from.
    #[arg(long, default_value = DEFAULT_MODELS_DEV_URL)]
    url: String,

    /// Read the catalog from a local JSON file instead of fetching `--url`.
    /// Required for deterministic/offline tests.
    #[arg(long)]
    input: Option<PathBuf>,

    /// Print the rendered document to stdout instead of writing files.
    #[arg(long, conflicts_with = "validate")]
    dry_run: bool,

    /// CI mode: regenerate in-memory and diff against the committed output
    /// files, exiting non-zero on drift. Writes nothing.
    #[arg(long, conflicts_with = "dry_run")]
    validate: bool,

    /// Repository root containing `schemas/` and `crates/liter-llm/schemas/`.
    /// Defaults to this crate's own checkout location.
    #[arg(long)]
    root: Option<PathBuf>,
}

fn default_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(error) = run(cli).await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), CatalogGenError> {
    let raw_text = load_catalog_text(&cli).await?;
    let catalog = parse_and_validate(&raw_text)?;

    let fetched_on = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let provenance = build_provenance(&raw_text, &fetched_on);
    let document = build_catalog_document(&catalog, &provenance)?;

    if cli.dry_run {
        print!("{document}");
        return Ok(());
    }

    let root = cli.root.unwrap_or_else(default_root);
    let paths = default_output_paths(&root);

    if cli.validate {
        return run_validate(&paths, &document);
    }

    for path in &paths {
        write_document(path, &document)?;
    }
    eprintln!("Wrote {} catalog file(s)", paths.len());
    Ok(())
}

async fn load_catalog_text(cli: &Cli) -> Result<String, CatalogGenError> {
    if let Some(input) = &cli.input {
        return std::fs::read_to_string(input).map_err(|source| CatalogGenError::ReadFile {
            path: input.clone(),
            source,
        });
    }
    fetch_catalog_text(&cli.url).await
}

fn run_validate(paths: &[PathBuf], document: &str) -> Result<(), CatalogGenError> {
    let stale = stale_paths(paths, document)?;
    if stale.is_empty() {
        eprintln!("All catalog files are up to date");
        return Ok(());
    }
    for path in &stale {
        eprintln!("stale: {}", path.display());
    }
    Err(CatalogGenError::Drift { path: stale[0].clone() })
}
