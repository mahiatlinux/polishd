use crate::{clipboard, editable, force_focus, keystroke, polisher, AppState, PendingTransform, STORE_FILE};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_store::StoreExt;

const TRANSFORM_WINDOW_TITLE: &str = "polishd — transform";

const MODAL_LOGICAL_WIDTH:  f64 = 560.0;
const MODAL_LOGICAL_HEIGHT: f64 = 95.0;
const CURSOR_GAP_LOGICAL:   f64 = 20.0;

pub async fn handle_hotkey(app: AppHandle) {
    if !claim_processing_lock(&app) {
        return;
    }
    let Some((text, original)) = acquire_selection(&app).await else {
        return;
    };
    polish_and_paste(&app, text, original).await;
}

pub async fn handle_transform_hotkey(app: AppHandle) {
    if !claim_processing_lock(&app) {
        return;
    }
    let Some((text, original)) = acquire_selection(&app).await else {
        return;
    };

    if app.get_webview_window("transform").is_some() {
        if let Some(w) = app.get_webview_window("transform") {
            let _ = w.destroy();
        }
        for _ in 0..100 {
            if app.get_webview_window("transform").is_none() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }
    }

    let (anchor_x, anchor_top, scale) = compute_anchor(&app);

    {
        let state = app.state::<AppState>();
        *state.pending_transform.lock().unwrap() = Some(PendingTransform {
            text: text.clone(),
            original_clipboard: original,
            anchor_x,
            anchor_top,
        });
    }

    let theme = app
        .store(STORE_FILE)
        .ok()
        .and_then(|s| s.get("theme"))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .filter(|t| t == "dark" || t == "light")
        .unwrap_or_else(|| "dark".to_string());

    let init_script = format!(
        r#"window.__POLISHD_ANCHOR__={{x:{ax},y:{ay}}};window.__POLISHD_THEME__="{theme}";document.documentElement.setAttribute("data-theme","{theme}");"#,
        ax = anchor_x,
        ay = anchor_top,
        theme = theme,
    );

    let build_result = WebviewWindowBuilder::new(
        &app,
        "transform",
        WebviewUrl::App("index.html".into()),
    )
    .title(TRANSFORM_WINDOW_TITLE)
    .inner_size(MODAL_LOGICAL_WIDTH, MODAL_LOGICAL_HEIGHT)
    .min_inner_size(MODAL_LOGICAL_WIDTH, MODAL_LOGICAL_HEIGHT)
    .position(anchor_x * scale, anchor_top * scale)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .visible(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(true)
    .initialization_script(&init_script)
    .build();

    let w = match build_result {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[polishd] build transform window: {e}");
            let fallback_orig = {
                let state = app.state::<AppState>();
                let taken = state.pending_transform.lock().unwrap().take();
                taken.and_then(|p| p.original_clipboard)
            };
            polish_and_paste(&app, text, fallback_orig).await;
            return;
        }
    };

    let _ = w.set_focus();
    tauri::async_runtime::spawn(async move {
        for delay_ms in [0u64, 20, 60, 140, 280] {
            if delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }
            force_focus::activate_by_title(TRANSFORM_WINDOW_TITLE);
        }
    });

    let _ = app.emit("status-change", "ready");
}

fn claim_processing_lock(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    let mut processing = state.is_processing.lock().unwrap();
    if *processing {
        return false;
    }
    *processing = true;
    true
}

async fn acquire_selection(app: &AppHandle) -> Option<(String, Option<String>)> {
    let _ = app.emit("status-change", "processing");

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    if let Some(false) = editable::is_focused_editable().await {
        finish(app, "no-editable");
        return None;
    }

    let original = tokio::task::spawn_blocking(clipboard::save)
        .await
        .unwrap_or(None);

    let copy_ok = tokio::task::spawn_blocking(keystroke::copy)
        .await
        .unwrap_or(Err("spawn error".into()));

    if let Err(e) = copy_ok {
        eprintln!("[polishd] copy error: {e}");
        finish(app, "ready");
        return None;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(120)).await;

    let selected = tokio::task::spawn_blocking(clipboard::read)
        .await
        .unwrap_or(None);

    match selected {
        Some(t) if !t.is_empty() && Some(&t) != original.as_ref() => Some((t, original)),
        _ => {
            restore_and_finish(app, original, "no-selection").await;
            None
        }
    }
}

fn compute_anchor(app: &AppHandle) -> (f64, f64, f64) {
    let cursor = match app.cursor_position() {
        Ok(p) => p,
        Err(_) => return (200.0, 200.0, 1.0),
    };

    let monitor = app
        .available_monitors()
        .ok()
        .and_then(|mons| {
            mons.into_iter().find(|m| {
                let pos = m.position();
                let size = m.size();
                let cx = cursor.x as i32;
                let cy = cursor.y as i32;
                cx >= pos.x
                    && cx < pos.x + size.width as i32
                    && cy >= pos.y
                    && cy < pos.y + size.height as i32
            })
        })
        .or_else(|| app.primary_monitor().ok().flatten());

    let scale = monitor.as_ref().map(|m| m.scale_factor()).unwrap_or(1.0).max(0.1);
    let cx_logical = cursor.x / scale;
    let cy_logical = cursor.y / scale;

    let (mon_x, mon_y, mon_w, mon_h) = monitor
        .map(|m| {
            let mscale = m.scale_factor().max(0.1);
            let size = m.size();
            let pos = m.position();
            (
                pos.x as f64 / mscale,
                pos.y as f64 / mscale,
                size.width as f64 / mscale,
                size.height as f64 / mscale,
            )
        })
        .unwrap_or((0.0, 0.0, 1920.0, 1080.0));

    let above_top = cy_logical - CURSOR_GAP_LOGICAL - MODAL_LOGICAL_HEIGHT;
    let below_top = cy_logical + CURSOR_GAP_LOGICAL;
    let mut y = if above_top >= mon_y {
        above_top
    } else if below_top + MODAL_LOGICAL_HEIGHT <= mon_y + mon_h {
        below_top
    } else {
        above_top
    };

    let mut x = cx_logical - MODAL_LOGICAL_WIDTH / 2.0;

    let margin = 8.0;
    let max_x = mon_x + mon_w - MODAL_LOGICAL_WIDTH - margin;
    let max_y = mon_y + mon_h - MODAL_LOGICAL_HEIGHT - margin;
    if x < mon_x + margin { x = mon_x + margin; }
    if x > max_x          { x = max_x; }
    if y < mon_y + margin { y = mon_y + margin; }
    if y > max_y          { y = max_y; }

    (x, y, scale)
}

async fn polish_and_paste(app: &AppHandle, text: String, original: Option<String>) {
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
        restore_and_finish(app, original, "error").await;
        return;
    }

    match polisher::polish_text(&text, &api_key).await {
        Ok(polished) => {
            let polished_clone = polished.clone();
            let write_ok = tokio::task::spawn_blocking(move || clipboard::write(polished_clone))
                .await
                .unwrap_or(false);

            if write_ok {
                tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;
                let paste_result = tokio::task::spawn_blocking(keystroke::paste).await;
                if paste_result.unwrap_or(Err("spawn".into())).is_err() {
                    restore_and_finish(app, original, "error").await;
                    return;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(220)).await;
                if let Some(orig) = original {
                    let _ = tokio::task::spawn_blocking(move || clipboard::write(orig)).await;
                }
            }
            finish(app, "ready");
        }
        Err(e) => {
            eprintln!("[polishd] API error: {e}");
            let msg = if e.contains("401") || e.contains("403") {
                "Invalid API key. Check your OpenRouter key in Settings."
            } else if e.contains("429") {
                "Rate limited — try again in a moment."
            } else {
                "Polish failed. Check your internet connection."
            };
            let _ = app
                .notification()
                .builder()
                .title("polishd")
                .body(msg)
                .show();
            restore_and_finish(app, original, "error").await;
        }
    }
}

async fn restore_and_finish(app: &AppHandle, original: Option<String>, status: &str) {
    if let Some(orig) = original {
        let _ = tokio::task::spawn_blocking(move || clipboard::write(orig)).await;
    }
    finish(app, status);
}

fn finish(app: &AppHandle, status: &str) {
    let state = app.state::<AppState>();
    *state.is_processing.lock().unwrap() = false;
    let _ = app.emit("status-change", status);
}
