mod clipboard;
mod editable;
mod force_focus;
mod hotkey;
mod keystroke;
mod polisher;
mod prompt;

use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutEvent, ShortcutState};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as AutostartManagerExt};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_store::StoreExt;

pub const TRANSFORM_HOTKEY: &str = "CmdOrCtrl+Shift+D";

pub const STORE_FILE: &str = "polishd.json";

const TRAY_ICON_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82,
    0, 0, 0, 32, 0, 0, 0, 32, 8, 6, 0, 0, 0, 115, 122, 122,
    244, 0, 0, 0, 222, 73, 68, 65, 84, 120, 156, 237, 148, 191, 14, 129,
    49, 20, 71, 207, 39, 38, 49, 84, 226, 190, 129, 213, 100, 178, 137, 68,
    152, 77, 222, 192, 43, 121, 4, 22, 196, 38, 17, 17, 49, 218, 236, 102,
    201, 53, 116, 192, 202, 64, 252, 203, 23, 95, 67, 74, 36, 61, 227, 189,
    191, 182, 167, 105, 110, 33, 16, 8, 252, 152, 200, 37, 164, 106, 151, 64,
    233, 69, 100, 7, 108, 128, 37, 208, 5, 70, 34, 230, 232, 178, 119, 202,
    37, 228, 64, 22, 40, 0, 45, 96, 8, 76, 85, 109, 238, 155, 2, 207,
    84, 128, 190, 79, 129, 177, 136, 137, 68, 76, 196, 249, 246, 53, 96, 253,
    44, 161, 106, 171, 190, 4, 174, 136, 152, 189, 136, 153, 0, 237, 152, 118,
    197, 187, 192, 29, 171, 152, 90, 254, 155, 2, 197, 152, 218, 54, 105, 81,
    250, 211, 83, 85, 109, 6, 40, 3, 157, 152, 246, 204, 151, 64, 93, 213,
    38, 205, 249, 92, 196, 76, 147, 54, 242, 53, 134, 11, 160, 233, 18, 252,
    248, 9, 46, 28, 184, 253, 132, 61, 96, 224, 250, 19, 190, 43, 48, 22,
    49, 141, 55, 215, 62, 224, 235, 9, 130, 64, 16, 248, 31, 129, 64, 32,
    240, 115, 78, 19, 19, 43, 12, 167, 80, 245, 195, 0, 0, 0, 0, 73,
    69, 78, 68, 174, 66, 96, 130,
];

pub struct PendingTransform {
    pub text: String,
    pub original_clipboard: Option<String>,
    pub anchor_x: f64,
    pub anchor_top: f64,
}

pub struct AppState {
    pub is_processing:     Mutex<bool>,
    pub hotkey:            Mutex<String>,
    pub pending_transform: Mutex<Option<PendingTransform>>,
}

#[tauri::command]
fn get_api_key(app: AppHandle) -> String {
    app.store(STORE_FILE)
        .ok()
        .and_then(|s| s.get("api_key"))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

#[tauri::command]
fn save_api_key(app: AppHandle, key: String) -> bool {
    let Ok(store) = app.store(STORE_FILE) else { return false; };
    store.set("api_key", key);
    store.save().is_ok()
}

#[tauri::command]
fn get_theme(app: AppHandle) -> String {
    app.store(STORE_FILE)
        .ok()
        .and_then(|s| s.get("theme"))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .filter(|t| t == "dark" || t == "light")
        .unwrap_or_else(|| "dark".to_string())
}

#[tauri::command]
fn save_theme(app: AppHandle, theme: String) -> bool {
    if theme != "dark" && theme != "light" {
        return false;
    }
    let Ok(store) = app.store(STORE_FILE) else { return false; };
    store.set("theme", theme);
    store.save().is_ok()
}

#[tauri::command]
fn get_hotkey(app: AppHandle) -> String {
    app.state::<AppState>().hotkey.lock().unwrap().clone()
}

#[tauri::command]
fn get_transform_hotkey() -> String {
    #[cfg(target_os = "macos")]
    { "Cmd+Shift+D".to_string() }
    #[cfg(not(target_os = "macos"))]
    { "Ctrl+Shift+D".to_string() }
}

#[tauri::command]
fn set_hotkey(app: AppHandle, shortcut: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let old = state.hotkey.lock().unwrap().clone();

    let _ = app.global_shortcut().unregister(old.as_str());

    let ah = app.clone();
    app.global_shortcut()
        .on_shortcut(shortcut.as_str(), move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                let handle = ah.clone();
                tauri::async_runtime::spawn(async move {
                    hotkey::handle_hotkey(handle).await;
                });
            }
        })
        .map_err(|e| e.to_string())?;

    *state.hotkey.lock().unwrap() = shortcut.clone();
    if let Ok(store) = app.store(STORE_FILE) {
        store.set("hotkey", shortcut);
        let _ = store.save();
    }

    Ok(())
}

#[tauri::command]
async fn submit_transform(app: AppHandle, instruction: String, mode: String) -> Result<(), String> {
    let instruction = instruction.trim().to_string();
    if mode == "transform" && instruction.is_empty() {
        return Err("Empty instruction".to_string());
    }

    let pending = {
        let state = app.state::<AppState>();
        let taken = state.pending_transform.lock().unwrap().take();
        taken
    };
    let Some(pending) = pending else {
        return Err("No pending text".to_string());
    };

    let api_key = app
        .store(STORE_FILE)
        .ok()
        .and_then(|s| s.get("api_key"))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    if api_key.is_empty() {
        let _ = app
            .notification()
            .builder()
            .title("polishd")
            .body("No API key — open Settings and paste your OpenRouter key.")
            .show();
        finish_transform(&app, pending.original_clipboard, "error").await;
        return Err("No API key".to_string());
    }

    let _ = app.emit("status-change", "processing");

    let result = if mode == "prompt" {
        polisher::optimize_prompt(&pending.text, &instruction, &api_key).await
    } else {
        polisher::transform_text(&pending.text, &instruction, &api_key).await
    };

    match result {
        Ok(transformed) => {
            if let Some(w) = app.get_webview_window("transform") {
                let _ = w.destroy();
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

            let t_clone = transformed.clone();
            let write_ok = tokio::task::spawn_blocking(move || clipboard::write(t_clone))
                .await
                .unwrap_or(false);

            if write_ok {
                tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;
                let _ = tokio::task::spawn_blocking(keystroke::paste).await;

                tokio::time::sleep(tokio::time::Duration::from_millis(220)).await;
                if let Some(orig) = pending.original_clipboard {
                    let _ = tokio::task::spawn_blocking(move || clipboard::write(orig)).await;
                }
            }

            finish_transform(&app, None, "ready").await;
            Ok(())
        }
        Err(e) => {
            eprintln!("[polishd] transform error: {e}");
            let msg = if e.contains("401") || e.contains("403") {
                "Invalid API key. Check your OpenRouter key in Settings."
            } else if e.contains("429") {
                "Rate limited — try again in a moment."
            } else {
                "Transform failed. Check your internet connection."
            };
            let _ = app
                .notification()
                .builder()
                .title("polishd")
                .body(msg)
                .show();

            if let Some(w) = app.get_webview_window("transform") {
                let _ = w.destroy();
            }
            finish_transform(&app, pending.original_clipboard, "error").await;
            Err(e)
        }
    }
}

#[tauri::command]
async fn cancel_transform(app: AppHandle) {
    let pending = {
        let state = app.state::<AppState>();
        let taken = state.pending_transform.lock().unwrap().take();
        taken
    };

    if let Some(w) = app.get_webview_window("transform") {
        let _ = w.destroy();
    }

    let orig = pending.and_then(|p| p.original_clipboard);
    finish_transform(&app, orig, "ready").await;
}

async fn finish_transform(app: &AppHandle, restore_clipboard: Option<String>, status: &str) {
    if let Some(orig) = restore_clipboard {
        let _ = tokio::task::spawn_blocking(move || clipboard::write(orig)).await;
    }
    let state = app.state::<AppState>();
    *state.is_processing.lock().unwrap() = false;
    let _ = app.emit("status-change", status);
}

#[tauri::command]
async fn test_polish(app: AppHandle, text: String) -> Result<String, String> {
    let api_key = app
        .store(STORE_FILE)
        .ok()
        .and_then(|s| s.get("api_key"))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    if api_key.is_empty() {
        return Err("No API key configured.".to_string());
    }

    let _ = app.emit("status-change", "processing");
    let result = polisher::polish_text(&text, &api_key).await;
    let _ = app.emit("status-change", "ready");
    result
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .setup(|app| {
            let stored_hotkey = app
                .store(STORE_FILE)
                .ok()
                .and_then(|s| s.get("hotkey"))
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "Ctrl+Shift+E".to_string());

            let settings_i =
                MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_i =
                MenuItem::with_id(app, "quit", "Quit polishd", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&settings_i, &sep, &quit_i])?;

            let tray_icon =
                tauri::image::Image::from_bytes(TRAY_ICON_PNG).expect("failed to build tray icon");

            TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .menu(&menu)
                .tooltip("polishd — ready")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "settings" => {
                        if let Some(w) = app.get_webview_window("settings") {
                            let _ = w.show();
                            let _ = w.set_focus();
                            let _ = w.center();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            app.manage(AppState {
                is_processing:     Mutex::new(false),
                hotkey:            Mutex::new(stored_hotkey.clone()),
                pending_transform: Mutex::new(None),
            });

            let _ = app.autolaunch().enable();

            editable::init();

            let ah_polish = app.handle().clone();
            app.global_shortcut().on_shortcut(
                stored_hotkey.as_str(),
                move |_app: &AppHandle, _shortcut: &_, event: ShortcutEvent| {
                    if event.state() == ShortcutState::Pressed {
                        let handle = ah_polish.clone();
                        tauri::async_runtime::spawn(async move {
                            hotkey::handle_hotkey(handle).await;
                        });
                    }
                },
            )?;

            let ah_transform = app.handle().clone();
            app.global_shortcut().on_shortcut(
                TRANSFORM_HOTKEY,
                move |_app: &AppHandle, _shortcut: &_, event: ShortcutEvent| {
                    if event.state() == ShortcutState::Pressed {
                        let handle = ah_transform.clone();
                        tauri::async_runtime::spawn(async move {
                            hotkey::handle_transform_hotkey(handle).await;
                        });
                    }
                },
            )?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_api_key,
            save_api_key,
            get_theme,
            save_theme,
            get_hotkey,
            get_transform_hotkey,
            set_hotkey,
            test_polish,
            submit_transform,
            cancel_transform,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
