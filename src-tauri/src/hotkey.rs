use crate::{clipboard, editable, force_focus, keystroke, polisher, AppState, PendingTransform, STORE_FILE};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_store::StoreExt;

fn force_gtk_size(window: &WebviewWindow, width: f64, height: f64, radius: i32) {
    #[cfg(target_os = "linux")]
    {
        let w = width.round() as i32;
        let h = height.round() as i32;
        let _ = window.with_webview(move |webview| {
            use gtk::prelude::{Cast, GtkWindowExt, WidgetExt};
            let wv = webview.inner();
            wv.set_size_request(w, h);
            if let Some(top) = wv.toplevel() {
                if let Ok(gtk_win) = top.downcast::<gtk::Window>() {
                    gtk_win.set_default_size(w, h);
                    gtk_win.resize(w, h);
                    apply_rounded_shape(&gtk_win, w, h, radius);
                }
            }
        });
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (window, width, height, radius);
    }
}

#[cfg(target_os = "linux")]
pub fn apply_rounded_shape(window: &gtk::Window, w: i32, h: i32, r: i32) {
    use gtk::cairo::{RectangleInt, Region};
    use gtk::prelude::WidgetExt;

    if r <= 0 || w < 2 * r || h < 2 * r {
        window.shape_combine_region(None);
        return;
    }

    let region = Region::create();

    for y in 0..r {
        let dy = (r - y) as f64;
        let dx = ((r * r) as f64 - dy * dy).max(0.0).sqrt().ceil() as i32;
        let x_start = r - dx;
        let strip_w = w - 2 * x_start;
        if strip_w > 0 {
            let _ = region.union_rectangle(&RectangleInt::new(x_start, y, strip_w, 1));
        }
    }

    if h - 2 * r > 0 {
        let _ = region.union_rectangle(&RectangleInt::new(0, r, w, h - 2 * r));
    }

    for y in 0..r {
        let dy = (y + 1) as f64;
        let dx = ((r * r) as f64 - dy * dy).max(0.0).sqrt().ceil() as i32;
        let x_start = r - dx;
        let strip_w = w - 2 * x_start;
        if strip_w > 0 {
            let _ = region.union_rectangle(&RectangleInt::new(x_start, h - r + y, strip_w, 1));
        }
    }

    window.shape_combine_region(Some(&region));
}

const TRANSFORM_WINDOW_TITLE: &str = "polishd — transform";
const POLISH_WINDOW_TITLE: &str = "polishd — polish";

pub const MODAL_LOGICAL_WIDTH:  f64 = 720.0;
pub const MODAL_LOGICAL_HEIGHT: f64 = 107.0;
pub const MODAL_TOP_OFFSET:     f64 = 96.0;
pub const MODAL_CORNER_RADIUS:  i32 = 0;

const POPUP_WIDTH:      f64 = 160.0;
const POPUP_HEIGHT:     f64 = 44.0;
const POPUP_TOP_OFFSET: f64 = 96.0;
const POPUP_CORNER_RADIUS: i32 = 0;

pub async fn handle_hotkey(app: AppHandle) {
    if !claim_processing_lock(&app) {
        return;
    }
    let Some((text, original)) = acquire_selection(&app).await else {
        return;
    };
    show_polish_popup(&app);
    polish_and_paste(&app, text, original).await;
    dismiss_polish_popup(&app);
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
        r#"window.__POLISHD_THEME__="{theme}";document.documentElement.setAttribute("data-theme","{theme}");"#,
        theme = theme,
    );

    #[cfg(target_os = "linux")]
    let resizable = true;
    #[cfg(not(target_os = "linux"))]
    let resizable = false;

    let build_result = WebviewWindowBuilder::new(
        &app,
        "transform",
        WebviewUrl::App("index.html".into()),
    )
    .title(TRANSFORM_WINDOW_TITLE)
    .inner_size(MODAL_LOGICAL_WIDTH, MODAL_LOGICAL_HEIGHT)
    .min_inner_size(MODAL_LOGICAL_WIDTH, MODAL_LOGICAL_HEIGHT)
    .position(anchor_x * scale, anchor_top * scale)
    .resizable(resizable)
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

    force_gtk_size(&w, MODAL_LOGICAL_WIDTH, MODAL_LOGICAL_HEIGHT, MODAL_CORNER_RADIUS);

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

fn show_polish_popup(app: &AppHandle) {
    let theme = app
        .store(STORE_FILE)
        .ok()
        .and_then(|s| s.get("theme"))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .filter(|t| t == "dark" || t == "light")
        .unwrap_or_else(|| "dark".to_string());

    let init_script = format!(
        r#"window.__POLISHD_THEME__="{theme}";document.documentElement.setAttribute("data-theme","{theme}");"#,
        theme = theme,
    );

    let (x, y, scale) = compute_popup_anchor(app);

    #[cfg(target_os = "linux")]
    let resizable = true;
    #[cfg(not(target_os = "linux"))]
    let resizable = false;

    let build = WebviewWindowBuilder::new(
        app,
        "polish",
        WebviewUrl::App("index.html".into()),
    )
    .title(POLISH_WINDOW_TITLE)
    .inner_size(POPUP_WIDTH, POPUP_HEIGHT)
    .min_inner_size(POPUP_WIDTH, POPUP_HEIGHT)
    .position(x * scale, y * scale)
    .resizable(resizable)
    .decorations(false)
    .transparent(true)
    .visible(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .initialization_script(&init_script)
    .build();

    if let Ok(w) = build {
        force_gtk_size(&w, POPUP_WIDTH, POPUP_HEIGHT, POPUP_CORNER_RADIUS);
    }
}

fn dismiss_polish_popup(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("polish") {
        let _ = w.destroy();
    }
}

fn compute_popup_anchor(app: &AppHandle) -> (f64, f64, f64) {
    let cursor = app.cursor_position().ok();
    let (mon_x, mon_y, mon_w, _mon_h, scale) = active_monitor_logical(app, cursor);

    let x = mon_x + (mon_w - POPUP_WIDTH) / 2.0;
    let y = mon_y + POPUP_TOP_OFFSET;
    (x, y, scale)
}

fn active_monitor_logical(
    app: &AppHandle,
    cursor: Option<tauri::PhysicalPosition<f64>>,
) -> (f64, f64, f64, f64, f64) {
    let monitor = cursor
        .and_then(|c| {
            app.available_monitors().ok().and_then(|mons| {
                mons.into_iter().find(|m| {
                    let pos = m.position();
                    let size = m.size();
                    let cx = c.x as i32;
                    let cy = c.y as i32;
                    cx >= pos.x
                        && cx < pos.x + size.width as i32
                        && cy >= pos.y
                        && cy < pos.y + size.height as i32
                })
            })
        })
        .or_else(|| app.primary_monitor().ok().flatten());

    let scale = monitor.as_ref().map(|m| m.scale_factor()).unwrap_or(1.0).max(0.1);

    monitor
        .map(|m| {
            let mscale = m.scale_factor().max(0.1);
            let size = m.size();
            let pos = m.position();
            (
                pos.x as f64 / mscale,
                pos.y as f64 / mscale,
                size.width as f64 / mscale,
                size.height as f64 / mscale,
                scale,
            )
        })
        .unwrap_or((0.0, 0.0, 1920.0, 1080.0, scale))
}

pub async fn run_keystroke(
    app: &AppHandle,
    f: fn() -> Result<(), String>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            let _ = tx.send(f());
        })
        .map_err(|e| e.to_string())?;
        rx.await.unwrap_or(Err("main-thread channel closed".into()))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        tokio::task::spawn_blocking(f)
            .await
            .unwrap_or(Err("spawn error".into()))
    }
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

    if let Err(e) = run_keystroke(app, keystroke::copy).await {
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
    let cursor = app.cursor_position().ok();
    let (mon_x, mon_y, mon_w, _mon_h, scale) = active_monitor_logical(app, cursor);

    let x = mon_x + (mon_w - MODAL_LOGICAL_WIDTH) / 2.0;
    let y = mon_y + MODAL_TOP_OFFSET;
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
                if run_keystroke(app, keystroke::paste).await.is_err() {
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
