//! MCP Guard — local agent for MCP / agent tool-call surfaces.

use anyhow::Result;
use clap::{Parser, Subcommand};
use mcp_guard::audit::{JsonlSink, JsonlStatusSource};
use mcp_guard::config::Config;
use mcp_guard::contracts::{StatusSource, TrayActionId};
use mcp_guard::git_scan;
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
    /// Scan a local git repo for opaque LLM reasoning signatures (anti leak-to-git)
    GitScan {
        /// Repo root (default: cwd)
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Only scan staged files (pre-commit friendly)
        #[arg(long)]
        staged: bool,
        /// Exit 1 when findings are present (default: true)
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        fail: bool,
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
    /// Delete a secret by name (also drops aliases pointing at it)
    Delete { name: String },
    /// Rename a canonical secret
    Rename { from: String, to: String },
    /// Add/update alias → secret name
    Alias {
        alias: String,
        #[arg(long = "to")]
        name: String,
    },
    /// Remove an alias
    Unalias { alias: String },
    /// Issue opaque ref for a secret (prints ref only)
    IssueRef { name: String },
}

fn main() -> Result<()> {
    // macOS .app double-click: CFBundleExecutable is this binary with no args.
    let args = inject_macos_app_launch_args(std::env::args_os().collect());
    // Set Accessory *before* any AppKit/tao init so Dock never creates an "exec" tile.
    #[cfg(target_os = "macos")]
    early_macos_accessory_policy(&args);

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse_from(args);
    let cfg = config::load(cli.config.as_deref())?;
    let locale = cli.locale.as_deref();
    // Keep CLI config path so macOS can spawn a dashboard child with the same flags.
    let config_path = cli.config.clone();

    match cli.command {
        Commands::Scan { ports } => {
            let rt = tokio_rt()?;
            let report = rt.block_on(scan::run(&cfg, &ports))?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Commands::GitScan { path, staged, fail } => {
            let report = git_scan::scan_repo(&cfg, &path, staged)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            if fail && !report.findings.is_empty() {
                anyhow::bail!(
                    "git-scan: {} opaque reasoning signature(s) found — refuse commit / clean traces",
                    report.findings.len()
                );
            }
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
                // Must run on the OS main thread (macOS tao/wry EventLoop).
                run_tray_with_options(
                    cfg,
                    ui,
                    /*agent*/ true,
                    /*open_dashboard*/ true,
                    locale,
                    config_path.as_deref(),
                )?;
            } else {
                let rt = tokio_rt()?;
                rt.block_on(serve::run_with(
                    &cfg,
                    once,
                    &LoopbackScanner,
                    &SoftWatcher,
                    &JsonlSink,
                ))?;
            }
        }
        Commands::Status { ui } => {
            let ui_cfg = ui_shell::load_ui_bundle(ui.as_deref(), locale)?;
            let snap = JsonlStatusSource::from_audit_cfg(&cfg.audit).snapshot(&cfg.audit.path)?;
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
                let rt = tokio_rt()?;
                rt.block_on(run_console_tray(&cfg, ui.as_deref(), locale))?;
            } else {
                // Must run on the OS main thread (macOS tao/wry EventLoop).
                run_tray_with_options(
                    cfg,
                    ui,
                    !no_agent,
                    !no_dashboard,
                    locale,
                    config_path.as_deref(),
                )?;
            }
        }
        Commands::Dashboard { ui } => {
            // Must run on the OS main thread (macOS tao/wry EventLoop).
            run_dashboard_cli(cfg, ui, locale)?;
        }
        Commands::Vault { action } => {
            let v = vault::Vault::open(&cfg.vault)?;
            match action {
                VaultCmd::List => {
                    for s in v.list()? {
                        println!("{}\t{}", s.name, s.updated_at);
                    }
                    for a in v.list_aliases()? {
                        println!("alias:{}\t→\t{}", a.alias, a.name);
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
                VaultCmd::Rename { from, to } => {
                    v.rename(&from, &to)?;
                    println!("renamed '{from}' → '{to}'");
                }
                VaultCmd::Alias { alias, name } => {
                    v.set_alias(&alias, &name)?;
                    let canonical = v.canonical_name(&alias)?;
                    println!("alias '{alias}' → '{canonical}'");
                }
                VaultCmd::Unalias { alias } => {
                    if v.remove_alias(&alias)? {
                        println!("removed alias '{alias}'");
                    } else {
                        println!("alias not found: {alias}");
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

fn tokio_rt() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}

fn native_tray_supported() -> bool {
    cfg!(any(windows, target_os = "macos"))
}

/// When launched as `Something.app/Contents/MacOS/MCPGuard` with no CLI args,
/// default to `tray` and load `Contents/Resources/mcp-guard.toml` if present.
fn inject_macos_app_launch_args(
    mut args: Vec<std::ffi::OsString>,
) -> Vec<std::ffi::OsString> {
    if args.len() != 1 {
        return args;
    }
    let exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from(&args[0]));
    let exe_s = exe.to_string_lossy();
    if !exe_s.contains(".app/Contents/MacOS/") {
        return args;
    }
    if let Some(macos_dir) = exe.parent() {
        let cfg = macos_dir.join("../Resources/mcp-guard.toml");
        if let Ok(cfg) = cfg.canonicalize() {
            args.push("--config".into());
            args.push(cfg.into_os_string());
        }
    }
    args.push("tray".into());
    args
}

#[cfg(target_os = "macos")]
fn early_macos_accessory_policy(args: &[std::ffi::OsString]) {
    let ui = args.iter().any(|a| {
        matches!(
            a.to_string_lossy().as_ref(),
            "tray" | "dashboard" | "--tray"
        )
    });
    if ui {
        ui_shell::set_accessory_policy();
    }
}

fn run_tray_with_options(
    cfg: Config,
    ui: Option<PathBuf>,
    agent: bool,
    open_dashboard: bool,
    locale: Option<&str>,
    config_path: Option<&std::path::Path>,
) -> Result<()> {
    #[cfg(any(windows, target_os = "macos"))]
    {
        // Single tray agent for the user session.
        let _singleton = match ui_shell::try_acquire_tray_singleton()? {
            ui_shell::TraySingletonAcquire::Secondary => {
                #[cfg(target_os = "macos")]
                {
                    if let Err(err) = ui_shell::notify_running_tray_show_dashboard() {
                        tracing::warn!(
                            error = %err,
                            "tray already running; could not signal primary instance"
                        );
                    } else {
                        tracing::info!("tray already running; handed off to primary instance");
                    }
                }
                #[cfg(windows)]
                tracing::info!("tray already running; exiting second instance");
                return Ok(());
            }
            ui_shell::TraySingletonAcquire::Primary(guard) => guard,
        };
        #[cfg(windows)]
        ui_shell::detach_console();
        #[cfg(target_os = "macos")]
        ui_shell::set_accessory_policy();

        let ui_cfg = ui_shell::load_ui_bundle(ui.as_deref(), locale)?;
        tracing::info!(locale = %ui_cfg.locale, "UI locale loaded");
        let catalog = Arc::new(ui_cfg.catalog);
        let audit_path = cfg.audit.path.clone();
        let audit_for_status = audit_path.clone();
        let activity_alert_ttl_secs = cfg.audit.activity_alert_ttl_secs;
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
        let catalog_tray = Arc::clone(&catalog);
        let catalog_scan = Arc::clone(&catalog);
        let dash_show = Arc::new(Mutex::new(None::<ui_shell::DashboardShowHandle>));
        #[cfg(target_os = "macos")]
        let dash_child: Arc<Mutex<Option<std::process::Child>>> = Arc::new(Mutex::new(None));

        // macOS: only one tao EventLoop may live on the OS main thread. Spawning a
        // second EventLoop for the dashboard panics — open it as a child process.
        // Windows: keep in-process dashboard thread + show-handle restore.
        let open_dashboard_fn: Arc<dyn Fn() + Send + Sync> = {
            #[cfg(target_os = "macos")]
            {
                let locale = locale.map(|s| s.to_string());
                let config_path = config_path.map(|p| p.to_path_buf());
                let ui_path = ui.clone();
                let catalog_fail = Arc::clone(&catalog);
                let dash_child = Arc::clone(&dash_child);
                Arc::new(move || {
                    if let Err(err) = open_or_focus_dashboard(
                        &dash_child,
                        config_path.as_deref(),
                        ui_path.as_deref(),
                        locale.as_deref(),
                    ) {
                        tracing::error!(error = %err, "spawn dashboard failed");
                        ui_shell::notify(
                            &catalog_fail.toast.dashboard_fail_title,
                            &err.to_string(),
                        );
                    }
                })
            }
            #[cfg(windows)]
            {
                let mute_for_dash = Arc::clone(&mute_until);
                let catalog_dash = Arc::clone(&catalog);
                let catalog_fail = Arc::clone(&catalog);
                let cfg_dash = cfg.clone();
                let scan_rt_dash = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .expect("dashboard scan runtime");
                let dash_open = Arc::new(AtomicBool::new(false));
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
            }
        };

        #[cfg(target_os = "macos")]
        {
            let f = Arc::clone(&open_dashboard_fn);
            ui_shell::install_tray_activate_watcher(move || f());
        }

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
            status: Box::new(move || {
                JsonlStatusSource::new(activity_alert_ttl_secs).snapshot(&audit_for_status)
            }),
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
                    #[cfg(target_os = "macos")]
                    let dash_child = Arc::clone(&dash_child);
                    move || {
                        if let Ok(g) = dash_show.lock() {
                            if let Some(handle) = g.as_ref() {
                                handle.request_exit();
                            }
                        }
                        #[cfg(target_os = "macos")]
                        kill_dashboard_child(&dash_child);
                        cancel_quit.store(true, Ordering::SeqCst);
                        tracing::info!("quit requested — stopping agent");
                    }
                }),
            },
        })?;
        // Ensure agent stops if tray loop ends for any reason
        cancel.store(true, Ordering::SeqCst);
        #[cfg(target_os = "macos")]
        kill_dashboard_child(&dash_child);
        return Ok(());
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (cfg, ui, agent, open_dashboard, locale, config_path);
        anyhow::bail!("native tray not built for this target; use --console");
    }
}

#[cfg(target_os = "macos")]
fn open_or_focus_dashboard(
    dash_child: &Mutex<Option<std::process::Child>>,
    config_path: Option<&std::path::Path>,
    ui_path: Option<&std::path::Path>,
    locale: Option<&str>,
) -> Result<()> {
    let mut slot = dash_child
        .lock()
        .map_err(|_| anyhow::anyhow!("dashboard child lock poisoned"))?;
    if let Some(child) = slot.as_mut() {
        match child.try_wait() {
            Ok(None) => {
                let pid = child.id();
                // Unhide if minimized-to-tray, then bring forward.
                ui_shell::request_dashboard_show(pid);
                if ui_shell::activate_pid(pid) {
                    tracing::info!(pid, "focused existing dashboard process");
                } else {
                    tracing::warn!(pid, "dashboard still running but activate failed; left alone");
                }
                return Ok(());
            }
            Ok(Some(status)) => {
                tracing::debug!(?status, "previous dashboard exited");
                *slot = None;
            }
            Err(err) => {
                tracing::warn!(error = %err, "dashboard try_wait failed; respawning");
                *slot = None;
            }
        }
    }

    let exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("resolve mcp-guard executable: {e}"))?;
    let mut cmd = std::process::Command::new(&exe);
    if let Some(c) = config_path {
        cmd.arg("--config").arg(c);
    }
    if let Some(l) = locale {
        cmd.arg("--locale").arg(l);
    }
    cmd.arg("dashboard");
    if let Some(u) = ui_path {
        cmd.arg("--ui").arg(u);
    }
    // Hide-on-minimize/close; tray reopens via SIGUSR1 (not Dock minimize).
    cmd.env("MCP_GUARD_HIDE_TO_TRAY", "1");
    // Detach from tray's stdio so dashboard logs don't interleave awkwardly.
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("spawn mcp-guard dashboard: {e}"))?;
    tracing::info!(pid = child.id(), "spawned dashboard process (macOS main-thread EventLoop)");
    *slot = Some(child);
    Ok(())
}

#[cfg(target_os = "macos")]
fn kill_dashboard_child(dash_child: &Mutex<Option<std::process::Child>>) {
    if let Ok(mut slot) = dash_child.lock() {
        if let Some(mut child) = slot.take() {
            let pid = child.id();
            match child.try_wait() {
                Ok(None) => {
                    tracing::info!(pid, "stopping dashboard child on tray quit");
                    let _ = child.kill();
                    let _ = child.wait();
                }
                Ok(Some(_)) => {}
                Err(err) => tracing::debug!(error = %err, pid, "dashboard child wait failed"),
            }
        }
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
    let activity_alert_ttl_secs = cfg.audit.activity_alert_ttl_secs;
    let cfg_shared = Arc::new(Mutex::new(cfg));
    let cfg_allow = Arc::clone(&cfg_shared);
    let cfg_scan = Arc::clone(&cfg_shared);
    let vault = Arc::new(vault::Vault::open(&cfg_shared.lock().unwrap().vault)?);
    let handle = scan_rt.handle().clone();
    Ok(ui_shell::DashboardHooks {
        audit_path,
        catalog,
        mute_until,
        vault,
        hide_to_tray,
        show_handle,
        status: Arc::new(move || {
            let snap = JsonlStatusSource::new(activity_alert_ttl_secs).snapshot(&audit_status)?;
            Ok((snap, false))
        }),
        risks: Arc::new(move || {
            mcp_guard::audit::latest_risks_from_jsonl(&audit_risks, activity_alert_ttl_secs)
        }),
        allow_process: Arc::new(move |app| {
            let mut c = cfg_allow.lock().unwrap();
            config::add_manual_allow(&mut c, app)
        }),
        scan: Arc::new(move || {
            let cfg = cfg_scan.lock().unwrap().clone();
            handle.block_on(tray_scan_once(&cfg))
        }),
    })
}

fn run_dashboard_cli(cfg: Config, ui: Option<PathBuf>, locale: Option<&str>) -> Result<()> {
    #[cfg(any(windows, target_os = "macos"))]
    {
        #[cfg(target_os = "macos")]
        ui_shell::set_accessory_policy();
        let ui_cfg = ui_shell::load_ui_bundle(ui.as_deref(), locale)?;
        tracing::info!(locale = %ui_cfg.locale, "UI locale loaded");
        let mute_until = Arc::new(Mutex::new(None));
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        // Tray-spawned child sets MCP_GUARD_HIDE_TO_TRAY=1 so minimize/close
        // hide the window (no Dock miniature) instead of exiting.
        let hide_to_tray = std::env::var_os("MCP_GUARD_HIDE_TO_TRAY").is_some();
        let hooks = dashboard_hooks(
            cfg,
            Arc::new(ui_cfg.catalog),
            mute_until,
            &rt,
            hide_to_tray,
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
    let source = JsonlStatusSource::from_audit_cfg(&cfg.audit);
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
