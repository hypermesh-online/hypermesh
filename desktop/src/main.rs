// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.3 — HyperMesh desktop shell entry point.
//
// Tauri 2 application that:
//   1. Spawns a system tray (see tray.rs) on launch.
//   2. Manages the `hypermesh` daemon as a child process (see daemon.rs).
//   3. Hosts the existing React UI (ui/frontend/dist) in the main WebView.
//   4. Exposes a small set of `#[tauri::command]` functions the React
//      side calls via @tauri-apps/api/core invoke().
//
// On window-close the app stays alive in the tray (standard desktop
// pattern); the only paths to true exit are the tray's "Quit" item or
// a SIGTERM/SIGINT to the shell process.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod daemon;
mod tray;
mod wizard;

use std::time::Duration;

use serde_json::Value;
use tauri::{Emitter, Manager, RunEvent, WindowEvent};

use crate::daemon::{DaemonManager, DaemonStartArgs, DaemonStatus};
use crate::wizard::WizardState;

// ---------------------------------------------------------------------------
// Tauri commands — invoked from React via @tauri-apps/api/core.invoke()
// ---------------------------------------------------------------------------

#[tauri::command]
async fn daemon_start(
    args: Option<DaemonStartArgs>,
    daemon: tauri::State<'_, DaemonManager>,
) -> Result<DaemonStatus, String> {
    daemon
        .start(args.unwrap_or_default())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn daemon_stop(daemon: tauri::State<'_, DaemonManager>) -> Result<DaemonStatus, String> {
    daemon.stop().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn daemon_status(daemon: tauri::State<'_, DaemonManager>) -> Result<DaemonStatus, String> {
    Ok(daemon.status().await)
}

#[tauri::command]
async fn daemon_check_update(daemon: tauri::State<'_, DaemonManager>) -> Result<Option<Value>, String> {
    Ok(daemon.check_update().await)
}

#[tauri::command]
async fn wizard_should_show() -> Result<bool, String> {
    Ok(wizard::current_state().await.should_show)
}

#[tauri::command]
async fn wizard_state() -> Result<WizardState, String> {
    Ok(wizard::current_state().await)
}

#[tauri::command]
async fn wizard_set_privacy(mode: String) -> Result<(), String> {
    wizard::set_privacy(&mode).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn wizard_set_trustnet_test(opt_in: bool) -> Result<(), String> {
    wizard::set_trustnet_test(opt_in).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn wizard_set_foundation_grant(requested: bool) -> Result<(), String> {
    wizard::set_foundation_grant_requested(requested)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn wizard_complete() -> Result<WizardState, String> {
    wizard::complete().await.map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// App entry
// ---------------------------------------------------------------------------

fn main() {
    let daemon = DaemonManager::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(daemon.clone())
        .setup(move |app| {
            // Install the system tray.
            tray::install(&app.handle(), daemon.clone())?;

            // Background poll loop: refresh tray state every 5s.
            let app_handle = app.handle().clone();
            let daemon_for_loop = daemon.clone();
            tauri::async_runtime::spawn(async move {
                let mut last_state = daemon::DaemonState::Stopped;
                let mut last_update: Option<String> = None;
                loop {
                    let status = daemon_for_loop.status().await;

                    // Pull update info opportunistically. Only call when the
                    // daemon is up — otherwise the IPC ping will fail and
                    // pollute logs.
                    let update_version = if matches!(status.state, daemon::DaemonState::Running) {
                        extract_update_version(daemon_for_loop.check_update().await)
                    } else {
                        None
                    };

                    if status.state != last_state || update_version != last_update {
                        if let Err(e) =
                            tray::refresh(&app_handle, status.state, update_version.clone()).await
                        {
                            eprintln!("[tray] refresh failed: {e}");
                        }
                        // Notify the UI via Tauri events so React can react.
                        let _ = app_handle.emit("daemon-status", &status);
                        if update_version.is_some() && update_version != last_update {
                            let _ = app_handle.emit("update-available", &update_version);
                        }
                        last_state = status.state;
                        last_update = update_version;
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            daemon_start,
            daemon_stop,
            daemon_status,
            daemon_check_update,
            wizard_should_show,
            wizard_state,
            wizard_set_privacy,
            wizard_set_trustnet_test,
            wizard_set_foundation_grant,
            wizard_complete,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            // Hide-to-tray on close instead of exiting; the only true exit
            // path is the tray's Quit item.
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { api, .. },
                ..
            } if label == "main" => {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.hide();
                }
                api.prevent_close();
            }
            RunEvent::ExitRequested { .. } => {
                // Best-effort daemon stop on shell exit. We don't await
                // because RunEvent handlers run on the main loop.
                let daemon = app_handle.state::<DaemonManager>().inner().clone();
                tauri::async_runtime::block_on(async move {
                    let _ = daemon.stop().await;
                });
            }
            _ => {}
        });
}

fn extract_update_version(resp: Option<Value>) -> Option<String> {
    let v = resp?;
    if v.get("up_to_date").and_then(|x| x.as_bool()) == Some(true) {
        return None;
    }
    v.get("available_version")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
}
