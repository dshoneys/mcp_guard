//! MCP Guard — local agent for MCP / agent tool-call surfaces.

use anyhow::Result;
use clap::{Parser, Subcommand};
use mcp_guard::audit::{JsonlSink, JsonlStatusSource};
use mcp_guard::config::Config;
use mcp_guard::contracts::{StatusSource, TrayActionId};
use mcp_guard::scan::LoopbackScanner;
use mcp_guard::watch::SoftWatcher;
use mcp_guard::{config, scan, serve, ui_shell, watch};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
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
        #[arg(short, long, value_delimiter = ',')]
        ports: Vec<u16>,
    },
    /// Show which processes listen on / connect to watched ports
    Watch,
    /// Run the resident agent (scan + soft watch + audit)
    Serve {
        #[arg(long)]
        once: bool,
        /// Also show OS tray (Windows/macOS); agent keeps running until Quit
        #[arg(long)]
        tray: bool,
        #[arg(long)]
        ui: Option<std::path::PathBuf>,
    },
    /// Print tray menu model + alert snapshot as JSON
    Status {
        #[arg(long)]
        ui: Option<std::path::PathBuf>,
    },
    /// OS tray + background agent (default). `--no-agent` = tray only.
    Tray {
        #[arg(long)]
        ui: Option<std::path::PathBuf>,
        /// Force console menu instead of native tray
        #[arg(long)]
        console: bool,
        /// Do not start scan/watch loop (status from existing audit only)
        #[arg(long)]
        no_agent: bool,
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
        Commands::Serve { once, tray, ui } => {
            if tray {
                if once {
                    tracing::warn!("--once is ignored with --tray (use Quit to stop)");
                }
                tokio::task::block_in_place(|| {
                    run_tray_with_options(cfg, ui, /*agent*/ true, /*console*/ false)
                })?;
            } else {
                serve::run_with(&cfg, once, &LoopbackScanner, &SoftWatcher, &JsonlSink).await?;
            }
        }
        Commands::Status { ui } => {
            let ui_cfg = ui_shell::load_ui_config(ui.as_deref())?;
            let snap = JsonlStatusSource.snapshot(&cfg.audit.path)?;
            let model =
                ui_shell::build_menu(&snap, &cfg.audit.path, &ui_cfg.tray.copy, false);
            ui_shell::print_status_json(&model, &snap)?;
        }
        Commands::Tray {
            ui,
            console,
            no_agent,
        } => {
            let use_console = console || !native_tray_supported();
            if !console && !native_tray_supported() {
                tracing::info!("native tray unsupported on this OS; using console");
            }
            if use_console {
                if !no_agent {
                    tracing::info!("console tray: start `serve` in another terminal for live agent, or omit --no-agent on native tray");
                }
                run_console_tray(&cfg, ui.as_deref()).await?;
            } else {
                tokio::task::block_in_place(|| {
                    run_tray_with_options(cfg, ui, !no_agent, false)
                })?;
            }
        }
        Commands::Version => {
            println!("mcp-guard {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}

fn native_tray_supported() -> bool {
    cfg!(any(windows, target_os = "macos"))
}

fn run_tray_with_options(
    cfg: Config,
    ui: Option<PathBuf>,
    agent: bool,
    _console: bool,
) -> Result<()> {
    #[cfg(any(windows, target_os = "macos"))]
    {
        let ui_cfg = ui_shell::load_ui_config(ui.as_deref())?;
        let audit_path = cfg.audit.path.clone();
        let audit_for_status = audit_path.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_quit = Arc::clone(&cancel);

        let agent_rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;

        if agent {
            let cfg_agent = cfg.clone();
            let cancel_agent = Arc::clone(&cancel);
            std::thread::Builder::new()
                .name("mcp-guard-agent".into())
                .spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build()
                        .expect("agent runtime");
                    if let Err(err) = rt.block_on(serve::run_with_cancel(
                        &cfg_agent,
                        false,
                        &LoopbackScanner,
                        &SoftWatcher,
                        &JsonlSink,
                        Some(cancel_agent),
                    )) {
                        tracing::error!(error = %err, "background agent exited with error");
                    }
                })
                .expect("spawn agent thread");
            tracing::info!("background agent started (scan+watch)");
        }

        let cfg_scan = cfg.clone();
        tracing::info!("mcp-guard native tray starting (right-click icon for menu)");
        ui_shell::run_native_tray(ui_shell::NativeTrayConfig {
            audit_path,
            copy: ui_cfg.tray.copy,
            refresh_secs: cfg.serve.interval_secs.max(5),
            status: Box::new(move || JsonlStatusSource.snapshot(&audit_for_status)),
            hooks: ui_shell::NativeTrayHooks {
                scan_now: Box::new(move || {
                    agent_rt.block_on(tray_scan_once(&cfg_scan))?;
                    Ok(())
                }),
                on_quit: Box::new(move || {
                    cancel_quit.store(true, Ordering::SeqCst);
                    tracing::info!("quit requested — stopping agent");
                }),
            },
        })?;
        // Ensure agent stops if tray loop ends for any reason
        cancel.store(true, Ordering::SeqCst);
        return Ok(());
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (cfg, ui, agent, _console);
        anyhow::bail!("native tray not built for this target; use --console");
    }
}

async fn tray_scan_once(cfg: &Config) -> Result<()> {
    serve::tick_once(cfg, &LoopbackScanner, &SoftWatcher, &JsonlSink).await
}

async fn run_console_tray(cfg: &Config, ui_path: Option<&std::path::Path>) -> Result<()> {
    let ui_cfg = ui_shell::load_ui_config(ui_path)?;
    let source = JsonlStatusSource;
    let mut mute_until: Option<SystemTime> = None;

    println!("mcp-guard tray (console). Commands: status | open | scan | mute | quit");

    loop {
        let now = SystemTime::now();
        let muted = ui_shell::is_muted(now, mute_until);
        let snap = source.snapshot(&cfg.audit.path)?;
        let model = ui_shell::build_menu(&snap, &cfg.audit.path, &ui_cfg.tray.copy, muted);
        println!();
        println!("[{}] {}", model.state_id, model.header_label);
        for (i, item) in model.items.iter().enumerate() {
            match &item.subtitle {
                Some(sub) => println!("  {}. {} ({})", i + 1, item.label, sub),
                None => println!("  {}. {}", i + 1, item.label),
            }
        }
        print!("> ");
        io::stdout().flush()?;

        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            break;
        }
        let cmd = line.trim().to_ascii_lowercase();
        let action = match cmd.as_str() {
            "1" | "open" | "o" => Some(TrayActionId::OpenAudit),
            "2" | "scan" | "s" => Some(TrayActionId::ScanNow),
            "3" | "mute" | "m" => Some(TrayActionId::Mute),
            "4" | "quit" | "q" => Some(TrayActionId::Quit),
            "status" | "" => {
                ui_shell::print_status_json(&model, &snap)?;
                None
            }
            _ => {
                println!("unknown: {cmd}");
                None
            }
        };

        match action {
            Some(TrayActionId::OpenAudit) => {
                ui_shell::open_audit(&cfg.audit.path)?;
                println!("opened {}", cfg.audit.path.display());
            }
            Some(TrayActionId::ScanNow) => {
                tray_scan_once(cfg).await?;
                println!("scan+watch tick complete");
            }
            Some(TrayActionId::Mute) => {
                mute_until = Some(ui_shell::mute_until_one_hour_from(SystemTime::now()));
                println!("alerts muted for 1h (audit continues)");
            }
            Some(TrayActionId::Quit) => {
                println!("bye");
                break;
            }
            None => {}
        }
    }

    Ok(())
}
