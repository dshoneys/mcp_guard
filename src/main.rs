//! MCP Guard — local agent for MCP / agent tool-call surfaces.
//!
//! MVP commands:
//! - `scan`  — probe loopback ports for unauthenticated / CORS-open MCP-like HTTP
//! - `watch` — attribute listeners/clients on those ports to processes (soft gate)
//! - `serve` — resident loop: scan + watch + JSONL audit
//! - `version`

use anyhow::Result;
use clap::{Parser, Subcommand};
use mcp_guard::audit::JsonlSink;
use mcp_guard::scan::LoopbackScanner;
use mcp_guard::watch::SoftWatcher;
use mcp_guard::{config, scan, serve, watch};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "mcp-guard", version, about = "MCP Guard — agent-era local MCP sentinel")]
struct Cli {
    /// Config file (TOML). Defaults to ./mcp-guard.toml if present.
    #[arg(short, long, global = true)]
    config: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Scan loopback for MCP-like HTTP exposure (auth / CORS / open ports)
    Scan {
        /// Extra ports to probe (comma-separated). Merged with config defaults.
        #[arg(short, long, value_delimiter = ',')]
        ports: Vec<u16>,
    },
    /// Show which processes listen on / connect to watched ports
    Watch,
    /// Run the resident agent (scan + soft watch + audit)
    Serve {
        /// Print findings once then exit (no long-running loop)
        #[arg(long)]
        once: bool,
    },
    /// Print version
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let cfg = config::load(cli.config.as_deref())?;

    match cli.command {
        Commands::Scan { ports } => {
            let report = scan::run(&cfg, &ports).await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Commands::Watch => {
            let report = watch::run(&cfg)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Commands::Serve { once } => {
            serve::run_with(&cfg, once, &LoopbackScanner, &SoftWatcher, &JsonlSink).await?;
        }
        Commands::Version => {
            println!("mcp-guard {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
