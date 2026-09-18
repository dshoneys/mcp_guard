//! macOS tray/dashboard process helpers (Dock policy + focus + single-instance).

use anyhow::{bail, Context, Result};
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationOptions, NSApplicationActivationPolicy,
    NSRunningApplication,
};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::time::Duration;

use super::TraySingletonAcquire;

/// Menu-bar / agent style: keep windows, but do not show a Dock tile.
/// Raw `mcp-guard` binaries otherwise appear as a generic "exec" icon.
pub fn set_accessory_policy() {
    let Some(mtm) = MainThreadMarker::new() else {
        tracing::debug!("set_accessory_policy skipped (not on main thread)");
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    if !app.setActivationPolicy(NSApplicationActivationPolicy::Accessory) {
        tracing::warn!("NSApplication setActivationPolicy(Accessory) failed");
    }
}

/// Bring an existing dashboard process's windows forward.
pub fn activate_pid(pid: u32) -> bool {
    let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid as i32)
    else {
        return false;
    };
    if app.isTerminated() {
        return false;
    }
    app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows)
}

/// Ask a tray-owned dashboard child to unhide (see `install_show_signal_watcher`).
pub fn request_dashboard_show(pid: u32) {
    let rc = unsafe { libc::kill(pid as libc::pid_t, libc::SIGUSR1) };
    if rc != 0 {
        tracing::debug!(pid, "SIGUSR1 to dashboard failed");
    }
}

/// Ask the primary tray instance to open/focus the dashboard (second-launch handoff).
pub fn notify_running_tray_show_dashboard() -> Result<()> {
    let path = tray_lock_path()?;
    let mut f = File::open(&path).with_context(|| format!("open tray lock {}", path.display()))?;
    let mut body = String::new();
    f.read_to_string(&mut body)
        .with_context(|| format!("read tray lock {}", path.display()))?;
    let pid: u32 = body
        .trim()
        .parse()
        .with_context(|| format!("parse tray pid from {}", path.display()))?;
    let rc = unsafe { libc::kill(pid as libc::pid_t, libc::SIGUSR2) };
    if rc != 0 {
        bail!("SIGUSR2 to tray pid {pid} failed");
    }
    tracing::info!(pid, "notified running tray to show dashboard");
    Ok(())
}

static SHOW_REQUESTED: AtomicBool = AtomicBool::new(false);
static ACTIVATE_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigusr1(_: libc::c_int) {
    // Async-signal-safe: only an atomic store.
    SHOW_REQUESTED.store(true, Ordering::SeqCst);
}

extern "C" fn on_sigusr2(_: libc::c_int) {
    ACTIVATE_REQUESTED.store(true, Ordering::SeqCst);
}

/// Poll SIGUSR1 and invoke `on_show` on a worker thread (safe vs. signal handler).
pub fn install_show_signal_watcher(mut on_show: impl FnMut() + Send + 'static) {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| unsafe {
        libc::signal(libc::SIGUSR1, on_sigusr1 as *const () as libc::sighandler_t);
    });
    std::thread::Builder::new()
        .name("mcp-guard-dash-show".into())
        .spawn(move || loop {
            if SHOW_REQUESTED.swap(false, Ordering::SeqCst) {
                on_show();
            }
            std::thread::sleep(Duration::from_millis(50));
        })
        .ok();
}

/// Poll SIGUSR2 (second app launch) → open/focus dashboard from the primary tray.
pub fn install_tray_activate_watcher(mut on_activate: impl FnMut() + Send + 'static) {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| unsafe {
        libc::signal(libc::SIGUSR2, on_sigusr2 as *const () as libc::sighandler_t);
    });
    std::thread::Builder::new()
        .name("mcp-guard-tray-activate".into())
        .spawn(move || loop {
            if ACTIVATE_REQUESTED.swap(false, Ordering::SeqCst) {
                on_activate();
            }
            std::thread::sleep(Duration::from_millis(50));
        })
        .ok();
}

/// Exclusive flock on `~/Library/Application Support/mcp-guard/tray.lock` (+ PID).
pub struct TraySingleton {
    _file: File,
}

fn tray_lock_path() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME not set")?;
    let dir = PathBuf::from(home).join("Library/Application Support/mcp-guard");
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    Ok(dir.join("tray.lock"))
}

pub fn try_acquire_tray_singleton() -> Result<TraySingletonAcquire> {
    let path = tray_lock_path()?;
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .with_context(|| format!("open tray lock {}", path.display()))?;

    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        if err.kind() == std::io::ErrorKind::WouldBlock
            || err.raw_os_error() == Some(libc::EWOULDBLOCK)
            || err.raw_os_error() == Some(libc::EAGAIN)
        {
            return Ok(TraySingletonAcquire::Secondary);
        }
        return Err(err).with_context(|| format!("flock {}", path.display()));
    }

    file.set_len(0)
        .with_context(|| format!("truncate tray lock {}", path.display()))?;
    write!(file, "{}", std::process::id())
        .with_context(|| format!("write tray pid {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("sync tray lock {}", path.display()))?;

    Ok(TraySingletonAcquire::Primary(TraySingleton { _file: file }))
}
