//! Native OS tray (Windows / macOS) using tray-icon + tao.

use crate::contracts::{AlertSnapshot, GuardSeverity, TrayActionId};
use crate::ui_shell::{
    build_menu, is_muted, mute_until_one_hour_from, open_audit, TrayCopy,
};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

enum UserEvent {
    TrayIconEvent(#[allow(dead_code)] TrayIconEvent),
    MenuEvent(MenuEvent),
}

/// Hooks invoked on the tray event-loop thread (keep work short or spawn).
pub struct NativeTrayHooks {
    pub scan_now: Box<dyn FnMut() -> Result<()> + Send>,
    /// Called when the user chooses Quit (stop background agent, etc.).
    pub on_quit: Box<dyn FnMut() + Send>,
}

pub struct NativeTrayConfig {
    pub audit_path: PathBuf,
    pub copy: TrayCopy,
    pub status: Box<dyn FnMut() -> Result<AlertSnapshot> + Send>,
    pub refresh_secs: u64,
    pub hooks: NativeTrayHooks,
}

pub fn run_native_tray(mut cfg: NativeTrayConfig) -> Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::TrayIconEvent(event));
    }));
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::MenuEvent(event));
    }));

    let mute_until: Arc<Mutex<Option<SystemTime>>> = Arc::new(Mutex::new(None));
    let mut tray_icon: Option<TrayIcon> = None;
    let mut menu_items: Option<MenuItems> = None;
    let mut next_refresh = Instant::now();

    let audit_path = cfg.audit_path.clone();
    let copy = cfg.copy.clone();
    let mute_flag = Arc::clone(&mute_until);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(next_refresh);

        match event {
            Event::NewEvents(StartCause::Init) | Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                let muted = {
                    let guard = mute_flag.lock().ok();
                    guard
                        .map(|g| is_muted(SystemTime::now(), *g))
                        .unwrap_or(false)
                };
                let snap = match (cfg.status)() {
                    Ok(s) => s,
                    Err(err) => {
                        tracing::warn!(error = %err, "tray status refresh failed");
                        AlertSnapshot::default()
                    }
                };
                let model = build_menu(&snap, &audit_path, &copy, muted);
                let items = MenuItems::build(&model.header_label, &model.items);
                let icon = icon_rgba(model.severity);

                match &mut tray_icon {
                    Some(tray) => {
                        let _ = tray.set_tooltip(Some(model.header_label.as_str()));
                        let _ = tray.set_icon(Some(icon));
                        let _ = tray.set_menu(Some(Box::new(items.menu.clone())));
                        menu_items = Some(items);
                    }
                    None => {
                        match TrayIconBuilder::new()
                            .with_menu(Box::new(items.menu.clone()))
                            .with_tooltip(model.header_label.clone())
                            .with_icon(icon)
                            .build()
                        {
                            Ok(t) => {
                                tray_icon = Some(t);
                                menu_items = Some(items);
                            }
                            Err(err) => {
                                tracing::error!(error = %err, "failed to create tray icon");
                                *control_flow = ControlFlow::Exit;
                            }
                        }
                    }
                }
                next_refresh = Instant::now() + Duration::from_secs(cfg.refresh_secs.max(5));
            }
            Event::UserEvent(UserEvent::MenuEvent(event)) => {
                let action = menu_items.as_ref().and_then(|m| m.action_for(&event.id));
                match action {
                    Some(TrayActionId::OpenAudit) => {
                        if let Err(err) = open_audit(&audit_path) {
                            tracing::warn!(error = %err, "open audit failed");
                        }
                    }
                    Some(TrayActionId::ScanNow) => {
                        if let Err(err) = (cfg.hooks.scan_now)() {
                            tracing::warn!(error = %err, "tray scan failed");
                        }
                        next_refresh = Instant::now();
                        *control_flow = ControlFlow::Poll;
                    }
                    Some(TrayActionId::Mute) => {
                        if let Ok(mut g) = mute_flag.lock() {
                            *g = Some(mute_until_one_hour_from(SystemTime::now()));
                        }
                        next_refresh = Instant::now();
                        *control_flow = ControlFlow::Poll;
                    }
                    Some(TrayActionId::Quit) => {
                        (cfg.hooks.on_quit)();
                        tray_icon.take();
                        *control_flow = ControlFlow::Exit;
                    }
                    None => {}
                }
            }
            Event::UserEvent(UserEvent::TrayIconEvent(_event)) => {}
            Event::LoopDestroyed => {}
            _ => {}
        }
    });
}

struct MenuItems {
    menu: Menu,
    open: MenuItem,
    scan: MenuItem,
    mute: MenuItem,
    quit: MenuItem,
}

impl MenuItems {
    fn build(header: &str, items: &[crate::contracts::TrayMenuItem]) -> Self {
        let menu = Menu::new();
        let header_item = MenuItem::new(header, false, None);
        let open = MenuItem::new(
            items
                .iter()
                .find(|i| i.action == TrayActionId::OpenAudit)
                .map(|i| i.label.as_str())
                .unwrap_or("Open audit log"),
            true,
            None,
        );
        let scan = MenuItem::new(
            items
                .iter()
                .find(|i| i.action == TrayActionId::ScanNow)
                .map(|i| i.label.as_str())
                .unwrap_or("Scan now"),
            true,
            None,
        );
        let mute = MenuItem::new(
            items
                .iter()
                .find(|i| i.action == TrayActionId::Mute)
                .map(|i| i.label.as_str())
                .unwrap_or("Mute alerts (1h)"),
            true,
            None,
        );
        let quit = MenuItem::new(
            items
                .iter()
                .find(|i| i.action == TrayActionId::Quit)
                .map(|i| i.label.as_str())
                .unwrap_or("Quit"),
            true,
            None,
        );
        let _ = menu.append_items(&[
            &header_item,
            &PredefinedMenuItem::separator(),
            &open,
            &scan,
            &mute,
            &PredefinedMenuItem::separator(),
            &quit,
        ]);
        Self {
            menu,
            open,
            scan,
            mute,
            quit,
        }
    }

    fn action_for(&self, id: &tray_icon::menu::MenuId) -> Option<TrayActionId> {
        if id == &self.open.id() {
            Some(TrayActionId::OpenAudit)
        } else if id == &self.scan.id() {
            Some(TrayActionId::ScanNow)
        } else if id == &self.mute.id() {
            Some(TrayActionId::Mute)
        } else if id == &self.quit.id() {
            Some(TrayActionId::Quit)
        } else {
            None
        }
    }
}

fn icon_rgba(severity: GuardSeverity) -> Icon {
    let (r, g, b) = match severity {
        GuardSeverity::Ok => (0x3d_u8, 0xd6, 0x8c),
        GuardSeverity::Warn => (0xf5, 0xa5, 0x24),
        GuardSeverity::Danger => (0xf3, 0x12, 0x60),
    };
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let cx = (size as i32) / 2;
    let cy = cx;
    let rad2 = 12i32 * 12i32;
    for y in 0..size as i32 {
        for x in 0..size as i32 {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= rad2 {
                let i = ((y as u32 * size + x as u32) * 4) as usize;
                rgba[i] = r;
                rgba[i + 1] = g;
                rgba[i + 2] = b;
                rgba[i + 3] = 255;
            }
        }
    }
    Icon::from_rgba(rgba, size, size).expect("icon rgba")
}
