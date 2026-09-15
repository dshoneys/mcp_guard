//! macOS tray/dashboard process helpers (Dock policy + focus).

use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationOptions, NSApplicationActivationPolicy,
    NSRunningApplication,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::time::Duration;

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

static SHOW_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigusr1(_: libc::c_int) {
    // Async-signal-safe: only an atomic store.
    SHOW_REQUESTED.store(true, Ordering::SeqCst);
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
