//! Windows process helpers for native tray (single-instance + detach console).

use anyhow::{bail, Result};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;

type HANDLE = *mut std::ffi::c_void;

const ERROR_ALREADY_EXISTS: u32 = 183;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(
        lp_mutex_attributes: *mut std::ffi::c_void,
        b_initial_owner: i32,
        lp_name: *const u16,
    ) -> HANDLE;
    fn GetLastError() -> u32;
    fn CloseHandle(h: HANDLE) -> i32;
    fn FreeConsole() -> i32;
}

/// Holds the named mutex for the process lifetime so a second tray exits early.
pub struct TraySingleton(HANDLE);

impl Drop for TraySingleton {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

/// Acquire `Local\mcp-guard-tray`. Second instance returns an error (not panic).
pub fn acquire_tray_singleton() -> Result<TraySingleton> {
    let name: Vec<u16> = OsStr::new("Local\\mcp-guard-tray")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let handle = unsafe { CreateMutexW(ptr::null_mut(), 1, name.as_ptr()) };
    if handle.is_null() {
        bail!("CreateMutexW failed ({})", unsafe { GetLastError() });
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            CloseHandle(handle);
        }
        bail!("MCP Guard 托盘已在运行（请使用系统托盘图标，勿重复启动）");
    }
    Ok(TraySingleton(handle))
}

/// Drop the inherited console so `Start-Process` / `cargo run` does not leave a black CMD window.
pub fn detach_console() {
    unsafe {
        FreeConsole();
    }
}
