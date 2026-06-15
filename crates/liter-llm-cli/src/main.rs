mod commands;

use clap::Parser;
use commands::Command;
use std::num::NonZeroUsize;

#[derive(Parser)]
#[command(name = "liter-llm", version, about = "LiterLLM proxy server and MCP tool server")]
struct Cli {
    /// Number of Tokio worker threads (default: physical CPU count).
    #[arg(long, global = true)]
    tokio_worker_threads: Option<NonZeroUsize>,

    /// Maximum number of Tokio blocking threads (default: 512).
    #[arg(long, global = true)]
    tokio_max_blocking_threads: Option<NonZeroUsize>,

    #[command(subcommand)]
    command: Command,
}

fn main() {
    liter_llm::ensure_crypto_provider();
    let cli = Cli::parse();

    // Build the Tokio runtime with optional worker thread overrides.
    let runtime = build_tokio_runtime(&cli);

    if let Err(e) = runtime.block_on(run(cli)) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn build_tokio_runtime(cli: &Cli) -> tokio::runtime::Runtime {
    let mut builder = tokio::runtime::Builder::new_multi_thread();

    // Configure worker threads if specified.
    if let Some(worker_threads) = cli.tokio_worker_threads {
        builder.worker_threads(worker_threads.get());
    }

    // Configure max blocking threads if specified.
    if let Some(max_blocking) = cli.tokio_max_blocking_threads {
        builder.max_blocking_threads(max_blocking.get());
    }

    builder
        .thread_name("liter-llm-tokio")
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
}

async fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Api(args) => commands::api::run(args).await,
        Command::Mcp(args) => commands::mcp::run(args).await,
    }
}
