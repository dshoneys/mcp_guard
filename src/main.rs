//! MCP Guard — local agent for MCP / agent tool-call surfaces.

use anyhow::Result;
use clap::{Parser, Subcommand};
use mcp_guard::audit::{JsonlSink, JsonlStatusSource};
use mcp_guard::config::Config;
use mcp_guard::contracts::{StatusSource, TrayActionId};
use mcp_guard::scan::LoopbackScanner;
use mcp_guard::watch::SoftWatcher;
use mcp_guard::{config, scan, serve, ui_shell, vault, watch};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "mcp-guard", version, about = "MCP Guard — agent-era local MCP sentinel")]
struct Cli {
    /// Config file (TOML). Defaults to ./mcp-guard.toml if present.
    #[arg(short, long, global = true)]
    config: Option<std::path::PathBuf>,

    /// UI locale (`zh-CN` default for daily debug; also `en`). Overrides ui/default.toml.
    #[arg(long, global = true)]
    locale: Option<String>,

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
    /// OS tray + main window + background agent (default entry).
    Tray {
        #[arg(long)]
        ui: Option<std::path::PathBuf>,
        /// Force console menu instead of native tray
        #[arg(long)]
        console: bool,
        /// Do not start scan/watch loop (status from existing audit only)
        #[arg(long)]
        no_agent: bool,
        /// Tray icon only — do not open the main window on start
        #[arg(long)]
        no_dashboard: bool,
    },
    /// Main window only (no tray). Prefer `tray` for normal use.
    Dashboard {
        #[arg(long)]
        ui: Option<std::path::PathBuf>,
    },
    /// Encrypted secret vault (NoContext MCP companion)
    Vault {
        #[command(subcommand)]
        action: VaultCmd,
    },
    /// stdio MCP server: vault tools that never return plaintext
    VaultMcp,
    /// Print version
    Version,
}

#[derive(Debug, Subcommand)]
enum VaultCmd {
    /// List secret names (no values)
    List,
    /// Store a secret (value from --value or stdin)
    Put {
        name: String,
        #[arg(long)]
        value: Option<String>,
    },
    /// Delete a secret by name
    Delete { name: String },
    /// Issue opaque ref for a secret (prints ref only)
    IssueRef { name: String },
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
    let locale = cli.locale.as_deref();

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
                    run_tray_with_options(cfg, ui, /*agent*/ true, /*open_dashboard*/ true, locale)
                })?;
            } else {
                serve::run_with(&cfg, once, &LoopbackScanner, &SoftWatcher, &JsonlSink).await?;
            }
        }
        Commands::Status { ui } => {
            let ui_cfg = ui_shell::load_ui_bundle(ui.as_deref(), locale)?;
            let snap = JsonlStatusSource.snapshot(&cfg.audit.path)?;
            let model =
                ui_shell::build_menu(&snap, &cfg.audit.path, &ui_cfg.catalog, false);
            ui_shell::print_status_json(&model, &snap)?;
        }
        Commands::Tray {
            ui,
            console,
            no_agent,
            no_dashboard,
        } => {
            let use_console = console || !native_tray_supported();
            if !console && !native_tray_supported() {
                tracing::info!("native tray unsupported on this OS; using console");
            }
            if use_console {
                if !no_agent {
                    tracing::info!("console tray: start `serve` in another terminal for live agent, or omit --no-agent on native tray");
                }
                run_console_tray(&cfg, ui.as_deref(), locale).await?;
            } else {
                tokio::task::block_in_place(|| {
                    run_tray_with_options(cfg, ui, !no_agent, !no_dashboard, locale)
                })?;
            }
        }
        Commands::Dashboard { ui } => {
            tokio::task::block_in_place(|| run_dashboard_cli(cfg, ui, locale))?;
        }
        Commands::Vault { action } => {
            let v = vault::Vault::open(&cfg.vault)?;
            match action {
                VaultCmd::List => {
                    for s in v.list()? {
                        println!("{}\t{}", s.name, s.updated_at);
                    }
                }
                VaultCmd::Put { name, value } => {
                    let value = match value {
                        Some(v) => v,
                        None => {
                            eprint!("secret value (stdin): ");
                            let mut line = String::new();
                            io::stdin().read_line(&mut line)?;
                            line.trim_end_matches(['\r', '\n']).to_string()
                        }
                    };
                    v.put(&name, &value)?;
                    println!("stored '{name}' (plaintext not echoed)");
                }
                VaultCmd::Delete { name } => {
                    if v.delete(&name)? {
                        println!("deleted '{name}'");
                    } else {
                        println!("not found: {name}");
                    }
                }
                VaultCmd::IssueRef { name } => {
                    let r = v.issue_ref(&name)?;
                    println!("{}", serde_json::to_string_pretty(&r)?);
                }
            }
        }
        Commands::VaultMcp => {
            // Quiet logs on stdout — MCP uses stdout for JSON-RPC
            let v = vault::Vault::open(&cfg.vault)?;
            vault::run_stdio_mcp(&v)?;
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
    open_dashboard: bool,
    locale: Option<&str>,
) -> Result<()> {
    #[cfg(any(windows, target_os = "macos"))]
    {
        // Keep mutex alive for the whole tray session.
        #[cfg(windows)]
        let _singleton = ui_shell::acquire_tray_singleton()?;
        #[cfg(windows)]
        ui_shell::detach_console();

        let ui_cfg = ui_shell::load_ui_bundle(ui.as_deref(), locale)?;
        tracing::info!(locale = %ui_cfg.locale, "UI locale loaded");
        let catalog = Arc::new(ui_cfg.catalog);
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
        let mute_until = Arc::new(Mutex::new(None));
        let mute_for_dash = Arc::clone(&mute_until);
        let catalog_dash = Arc::clone(&catalog);
        let catalog_tray = Arc::clone(&catalog);
        let catalog_scan = Arc::clone(&catalog);
        let catalog_fail = Arc::clone(&catalog);
        let cfg_dash = cfg.clone();
        let scan_rt_dash = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;

        let dash_open = Arc::new(AtomicBool::new(false));
        let dash_show = Arc::new(Mutex::new(None::<ui_shell::DashboardShowHandle>));
        let open_dashboard_fn: Arc<dyn Fn() + Send + Sync> = {
            let dash_open = Arc::clone(&dash_open);
            let dash_show = Arc::clone(&dash_show);
            Arc::new(move || {
                if let Ok(g) = dash_show.lock() {
                    if let Some(handle) = g.as_ref() {
                        tracing::info!("restoring dashboard from tray");
                        handle.show();
                        return;
                    }
                }
                if dash_open.swap(true, Ordering::SeqCst) {
                    tracing::info!("dashboard already starting");
                    return;
                }
                match dashboard_hooks(
                    cfg_dash.clone(),
                    Arc::clone(&catalog_dash),
                    Arc::clone(&mute_for_dash),
                    &scan_rt_dash,
                    true,
                    Arc::clone(&dash_show),
                ) {
                    Ok(hooks) => {
                        let dash_open = Arc::clone(&dash_open);
                        std::thread::spawn(move || {
                            if let Err(err) = ui_shell::run_dashboard(hooks) {
                                tracing::error!(error = %err, "dashboard closed with error");
                            }
                            dash_open.store(false, Ordering::SeqCst);
                        });
                    }
                    Err(err) => {
                        dash_open.store(false, Ordering::SeqCst);
                        tracing::error!(error = %err, "open dashboard failed");
                        ui_shell::notify(
                            &catalog_fail.toast.dashboard_fail_title,
                            &err.to_string(),
                        );
                    }
                }
            })
        };

        if open_dashboard {
            tracing::info!("opening main dashboard alongside tray");
            open_dashboard_fn();
        }

        tracing::info!("mcp-guard native tray starting (right-click icon for menu)");
        ui_shell::run_native_tray(ui_shell::NativeTrayConfig {
            audit_path,
            catalog: catalog_tray,
            refresh_secs: cfg.serve.interval_secs.max(5),
            mute_until: Arc::clone(&mute_until),
            status: Box::new(move || JsonlStatusSource.snapshot(&audit_for_status)),
            hooks: ui_shell::NativeTrayHooks {
                open_dashboard: Box::new({
                    let f = Arc::clone(&open_dashboard_fn);
                    move || f()
                }),
                scan_now: Box::new(move || {
                    let summary = agent_rt.block_on(tray_scan_once(&cfg_scan))?;
                    ui_shell::notify_scan_finished(
                        &catalog_scan,
                        summary.open_services,
                        summary.exposures,
                        summary.activity_alerts,
                    );
                    Ok(())
                }),
                on_quit: Box::new({
                    let dash_show = Arc::clone(&dash_show);
                    move || {
                        if let Ok(g) = dash_show.lock() {
                            if let Some(handle) = g.as_ref() {
                                handle.request_exit();
                            }
                        }
                        cancel_quit.store(true, Ordering::SeqCst);
                        tracing::info!("quit requested — stopping agent");
                    }
                }),
            },
        })?;
        // Ensure agent stops if tray loop ends for any reason
        cancel.store(true, Ordering::SeqCst);
        return Ok(());
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (cfg, ui, agent, open_dashboard, locale);
        anyhow::bail!("native tray not built for this target; use --console");
    }
}

#[cfg(any(windows, target_os = "macos"))]
fn dashboard_hooks(
    cfg: Config,
    catalog: Arc<ui_shell::Catalog>,
    mute_until: Arc<Mutex<Option<SystemTime>>>,
    scan_rt: &tokio::runtime::Runtime,
    hide_to_tray: bool,
    show_handle: Arc<Mutex<Option<ui_shell::DashboardShowHandle>>>,
) -> Result<ui_shell::DashboardHooks> {
    let audit_path = cfg.audit.path.clone();
    let audit_status = audit_path.clone();
    let audit_risks = audit_path.clone();
    let cfg_scan = cfg.clone();
    let vault = Arc::new(vault::Vault::open(&cfg.vault)?);
    let handle = scan_rt.handle().clone();
    Ok(ui_shell::DashboardHooks {
        audit_path,
        catalog,
        mute_until,
        vault,
        hide_to_tray,
        show_handle,
        status: Arc::new(move || {
            let snap = JsonlStatusSource.snapshot(&audit_status)?;
            Ok((snap, false))
        }),
        risks: Arc::new(move || mcp_guard::audit::latest_risks_from_jsonl(&audit_risks)),
        scan: Arc::new(move || handle.block_on(tray_scan_once(&cfg_scan))),
    })
}

fn run_dashboard_cli(cfg: Config, ui: Option<PathBuf>, locale: Option<&str>) -> Result<()> {
    #[cfg(any(windows, target_os = "macos"))]
    {
        let ui_cfg = ui_shell::load_ui_bundle(ui.as_deref(), locale)?;
        tracing::info!(locale = %ui_cfg.locale, "UI locale loaded");
        let mute_until = Arc::new(Mutex::new(None));
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        let hooks = dashboard_hooks(
            cfg,
            Arc::new(ui_cfg.catalog),
            mute_until,
            &rt,
            false,
            Arc::new(Mutex::new(None)),
        )?;
        ui_shell::run_dashboard(hooks)?;
        return Ok(());
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (cfg, ui, locale);
        anyhow::bail!("dashboard not built for this target");
    }
}

async fn tray_scan_once(cfg: &Config) -> Result<mcp_guard::contracts::TickSummary> {
    serve::tick_once(cfg, &LoopbackScanner, &SoftWatcher, &JsonlSink).await
}

async fn run_console_tray(
    cfg: &Config,
    ui_path: Option<&std::path::Path>,
    locale: Option<&str>,
) -> Result<()> {
    let ui_cfg = ui_shell::load_ui_bundle(ui_path, locale)?;
    let source = JsonlStatusSource;
    let mut mute_until: Option<SystemTime> = None;

    println!(
        "mcp-guard tray (console, locale={}). Commands: status | open | scan | mute | quit",
        ui_cfg.locale
    );

    loop {
        let now = SystemTime::now();
        let muted = ui_shell::is_muted(now, mute_until);
        let snap = source.snapshot(&cfg.audit.path)?;
        let model = ui_shell::build_menu(&snap, &cfg.audit.path, &ui_cfg.catalog, muted);
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
            "1" | "dash" | "d" => Some(TrayActionId::OpenDashboard),
            "2" | "open" | "o" => Some(TrayActionId::OpenAudit),
            "3" | "scan" | "s" => Some(TrayActionId::ScanNow),
            "4" | "mute" | "m" => Some(TrayActionId::Mute),
            "5" | "quit" | "q" => Some(TrayActionId::Quit),
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
            Some(TrayActionId::OpenDashboard) => {
                println!("console mode: run `mcp-guard dashboard` for the main window");
            }
            Some(TrayActionId::OpenAudit) => {
                ui_shell::open_audit(&cfg.audit.path)?;
                println!("opened {}", cfg.audit.path.display());
            }
            Some(TrayActionId::ScanNow) => {
                let summary = tray_scan_once(cfg).await?;
                ui_shell::notify_scan_finished(
                    &ui_cfg.catalog,
                    summary.open_services,
                    summary.exposures,
                    summary.activity_alerts,
                );
                println!(
                    "scan+watch tick complete (open={}, exposures={}, activity={})",
                    summary.open_services, summary.exposures, summary.activity_alerts
                );
            }
            Some(TrayActionId::Mute) => {
                mute_until = Some(ui_shell::mute_until_one_hour_from(SystemTime::now()));
                println!("{}", ui_cfg.catalog.toast.mute_body);
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
