// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.3 — System tray menu builder + event handler.
//
// The tray reflects three pieces of state:
//   1. Daemon health (Running / Starting / Stopped / Error)
//   2. Whether an update is available (system.check_update IPC)
//   3. Application identity (version label)
//
// Tray menu layout:
//
//   HyperMesh v{VERSION}                  (header, disabled)
//   Status: <state>                       (disabled, dynamic label)
//   ------------------------------------
//   Open Dashboard
//   Start daemon | Stop daemon            (mutually exclusive)
//   Update available: vX.Y.Z              (only when present)
//   ------------------------------------
//   Quit
//
// Tray icon color hint is platform-dependent: on Linux/Windows we ship
// three icon variants under `icons/tray-{green,yellow,red}.png` and
// swap them via `set_icon`. macOS template icons render monochrome, so
// state is communicated through the menu label only.

use std::sync::Arc;

use tauri::{
    image::Image,
    menu::{Menu, MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

use crate::daemon::{DaemonManager, DaemonState};

/// Cached tray state for the background poll loop. Currently the loop
/// in `main.rs` keeps its own copies; this struct is kept as a managed
/// resource so future Tauri commands (e.g. "force tray refresh") can
/// reach the cached values without re-querying the daemon.
#[allow(dead_code)]
pub struct TrayState {
    pub last_state: std::sync::Mutex<DaemonState>,
    pub last_update_version: std::sync::Mutex<Option<String>>,
}

impl TrayState {
    pub fn new() -> Self {
        Self {
            last_state: std::sync::Mutex::new(DaemonState::Stopped),
            last_update_version: std::sync::Mutex::new(None),
        }
    }
}

/// Build the initial tray + register click/menu handlers.
pub fn install<R: Runtime>(app: &AppHandle<R>, daemon: DaemonManager) -> tauri::Result<()> {
    let menu = build_menu(app, DaemonState::Stopped, None)?;

    let tray_state = Arc::new(TrayState::new());
    app.manage(tray_state.clone());

    let app_handle_for_menu = app.clone();
    let daemon_for_menu = daemon.clone();

    let _tray = TrayIconBuilder::with_id("hypermesh-tray")
        .tooltip("HyperMesh")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            let app = app.clone();
            let daemon = daemon_for_menu.clone();
            tauri::async_runtime::spawn(async move {
                handle_menu_event(app, daemon, event.id().0.as_str()).await;
            });
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(&app_handle_for_menu)?;

    Ok(())
}

/// Refresh the tray menu in response to state changes (called from a poll loop).
pub async fn refresh<R: Runtime>(
    app: &AppHandle<R>,
    daemon_state: DaemonState,
    available_update: Option<String>,
) -> tauri::Result<()> {
    let menu = build_menu(app, daemon_state, available_update.as_deref())?;
    if let Some(tray) = app.tray_by_id("hypermesh-tray") {
        tray.set_menu(Some(menu))?;
        tray.set_tooltip(Some(format!("HyperMesh — {}", state_label(daemon_state))))?;
        // Best-effort icon swap (silently skipped if the asset isn't present).
        if let Some(icon) = state_icon(app, daemon_state) {
            let _ = tray.set_icon(Some(icon));
        }
    }
    Ok(())
}

fn build_menu<R: Runtime>(
    app: &AppHandle<R>,
    state: DaemonState,
    available_update: Option<&str>,
) -> tauri::Result<Menu<R>> {
    let version = app.package_info().version.to_string();
    let header = MenuItemBuilder::with_id("header", format!("HyperMesh v{version}"))
        .enabled(false)
        .build(app)?;
    let status = MenuItemBuilder::with_id("status", format!("Status: {}", state_label(state)))
        .enabled(false)
        .build(app)?;
    let open = MenuItemBuilder::with_id("open_dashboard", "Open Dashboard").build(app)?;

    let toggle = match state {
        DaemonState::Running | DaemonState::Starting => {
            MenuItemBuilder::with_id("stop_daemon", "Stop daemon").build(app)?
        }
        DaemonState::Stopped | DaemonState::Error => {
            MenuItemBuilder::with_id("start_daemon", "Start daemon").build(app)?
        }
    };

    let mut builder = MenuBuilder::new(app)
        .item(&header)
        .item(&status)
        .separator()
        .item(&open)
        .item(&toggle);

    if let Some(v) = available_update {
        let update_item = MenuItemBuilder::with_id(
            "update_available",
            format!("Update available: v{}", v),
        )
        .build(app)?;
        builder = builder.item(&update_item);
    }

    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = builder
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&quit)
        .build()?;

    Ok(menu)
}

async fn handle_menu_event<R: Runtime>(app: AppHandle<R>, daemon: DaemonManager, id: &str) {
    match id {
        "open_dashboard" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                let _ = window.unminimize();
            }
        }
        "start_daemon" => {
            let _ = daemon.start(Default::default()).await;
        }
        "stop_daemon" => {
            let _ = daemon.stop().await;
        }
        "update_available" => {
            // Surface the update banner in the dashboard. The React side
            // already polls /api/v1/system/check_update via UpdateBanner.tsx
            // and shows a clickable link; we just bring the window forward.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                // Emit an event the UI can listen on to open release notes.
                let _ = app.emit("update-available", true);
            }
        }
        "quit" => {
            // Best-effort daemon stop before exit.
            let _ = daemon.stop().await;
            app.exit(0);
        }
        _ => {}
    }
}

fn state_label(s: DaemonState) -> &'static str {
    match s {
        DaemonState::Stopped => "Stopped",
        DaemonState::Starting => "Starting",
        DaemonState::Running => "Running",
        DaemonState::Error => "Error",
    }
}

fn state_icon<R: Runtime>(app: &AppHandle<R>, s: DaemonState) -> Option<Image<'static>> {
    let name = match s {
        DaemonState::Running => "tray-green.png",
        DaemonState::Starting => "tray-yellow.png",
        DaemonState::Stopped | DaemonState::Error => "tray-red.png",
    };
    let resource = app
        .path()
        .resolve(format!("icons/{name}"), tauri::path::BaseDirectory::Resource)
        .ok()?;
    // Read bytes into a Vec<u8>, then construct an owned Image<'static>.
    // Image::from_path returns Image<'_> bound to a temporary buffer; we
    // need 'static so the icon outlives the call site.
    let bytes = std::fs::read(resource).ok()?;
    Image::from_bytes(&bytes).ok().map(|img| {
        // from_bytes returns Image<'_> borrowing the input slice. Convert
        // to an owned image by reading the rgba data into a Vec.
        Image::new_owned(img.rgba().to_vec(), img.width(), img.height())
    })
}
