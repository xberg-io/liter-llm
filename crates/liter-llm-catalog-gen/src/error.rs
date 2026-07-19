//! Error types for the `liter-llm-catalog-gen` dev tool.

use std::path::PathBuf;

/// Errors produced while fetching, validating, transforming, or rendering
/// the models.dev catalog into liter-llm's `pricing.json` shape.
#[derive(Debug, thiserror::Error)]
pub enum CatalogGenError {
    /// The configured source URL does not use HTTPS.
    #[error("refusing to fetch non-HTTPS url: {0}")]
    InsecureUrl(String),

    /// The configured source URL's host is not in the allowlist. Fetching is
    /// restricted to the canonical upstream so a misconfigured or hostile
    /// `--url` (e.g. wired into CI with an externally-influenced argument)
    /// cannot redirect the generator at an arbitrary or internal host. Use
    /// `--input <file>` to transform a locally-supplied catalog instead.
    #[error("refusing to fetch from disallowed host `{host}` (allowed: {allowed}); use --input for local files")]
    DisallowedHost {
        /// The rejected host.
        host: String,
        /// Comma-separated allowlist, for the error message.
        allowed: String,
    },

    /// The HTTP request to fetch the catalog failed.
    #[error("failed to fetch catalog from {url}: {source}")]
    Fetch {
        /// The URL that was being fetched.
        url: String,
        #[source]
        source: reqwest::Error,
    },

    /// The upstream server returned a non-success HTTP status.
    #[error("fetching {url} returned HTTP {status}")]
    FetchStatus {
        /// The URL that was being fetched.
        url: String,
        /// The HTTP status code returned.
        status: u16,
    },

    /// Reading a local `--input` file failed.
    #[error("failed to read input file {path}: {source}")]
    ReadFile {
        /// The path that failed to read.
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Writing a rendered output file failed.
    #[error("failed to write output file {path}: {source}")]
    WriteFile {
        /// The path that failed to write.
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The fetched/loaded text is not valid JSON, or does not match the
    /// models.dev schema. Unknown top-level fields on strict record types are
    /// the intended drift signal — a genuinely new upstream field surfaces
    /// here rather than being silently ignored.
    #[error("failed to parse catalog JSON: {0}")]
    Parse(#[from] serde_json::Error),

    /// A structural validation rule (beyond what serde's type system
    /// enforces) failed, e.g. a negative cost or a non-positive context
    /// window.
    #[error("validation failed for provider `{provider}` model `{model}` field `{field}`: {message}")]
    Validation {
        /// The upstream provider id (e.g. `openai`).
        provider: String,
        /// The upstream model id, or the provider id again when the failing
        /// field is provider-scoped rather than model-scoped.
        model: String,
        /// The offending field path (e.g. `cost.input`).
        field: String,
        /// A human-readable description of the violated constraint.
        message: String,
    },

    /// The rendered document failed to round-trip through `serde_json`,
    /// meaning the hand-rolled renderer produced invalid or non-equivalent
    /// JSON. This should never happen; it indicates a bug in the renderer.
    #[error("rendered pricing document failed the JSON self-check: {0}")]
    RoundTrip(#[source] serde_json::Error),

    /// `--validate` detected drift between the freshly generated catalog and
    /// a committed output file.
    #[error("{} is out of date — run `task generate:catalog`", path.display())]
    Drift {
        /// The committed output file that no longer matches the freshly
        /// generated catalog.
        path: PathBuf,
    },
}
