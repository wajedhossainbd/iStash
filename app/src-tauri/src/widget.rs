//! The screen-edge widget: a narrow glass panel on the left or right edge of the display showing CPU, memory,
//! network, disk and drives. "pinned" keeps it out; "auto" tucks it in, leaving a thin edge, and slides it out under the pointer.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

const WIDTH: f64 = 132.0; // logical points: tall and thin
const PEEK: f64 = 6.0; // what stays on screen while tucked in
static MODE: Mutex<String> = Mutex::new(String::new());
static OPEN: AtomicBool = AtomicBool::new(false);
static HEIGHT: AtomicU32 = AtomicU32::new(620); // fixed, so the panel never moves as numbers change
static LEFT: AtomicBool = AtomicBool::new(false);

fn win(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("widget")
}

/// (x when out, x when tucked, y), in physical pixels on the display the widget is on.
fn spots(w: &WebviewWindow) -> Option<(f64, f64, f64)> {
    let m = w.current_monitor().ok().flatten().or_else(|| w.primary_monitor().ok().flatten())?;
    let (pos, size, k) = (m.position(), m.size(), m.scale_factor());
    let y = pos.y as f64 + (size.height as f64 - HEIGHT.load(Ordering::Relaxed) as f64 * k) / 2.0;
    if LEFT.load(Ordering::Relaxed) {
        let left = pos.x as f64;
        return Some((left + 8.0 * k, left - (WIDTH + 8.0 - PEEK) * k, y));
    }
    let right = pos.x as f64 + size.width as f64;
    Some((right - (WIDTH + 8.0) * k, right - PEEK * k, y))
}

fn slide_to(w: &WebviewWindow, open: bool) {
    let Some((out, tucked, y)) = spots(w) else { return };
    let from = w.outer_position().map(|p| p.x as f64).unwrap_or(if open { tucked } else { out });
    let to = if open { out } else { tucked };
    let w = w.clone();
    std::thread::spawn(move || {
        for i in 1..=14 {
            let t = i as f64 / 14.0;
            let e = 1.0 - (1.0 - t).powi(3); // ease out
            let _ = w.set_position(PhysicalPosition::new(from + (to - from) * e, y));
            std::thread::sleep(Duration::from_millis(12));
        }
    });
}

/// "off", "auto" or "pinned". Called by the main window at launch and when the setting changes.
#[tauri::command]
pub fn widget_mode(app: AppHandle, mode: String, left: Option<bool>) {
    let Some(w) = win(&app) else { return };
    LEFT.store(left.unwrap_or(false), Ordering::Relaxed);
    let _ = w.set_size(tauri::LogicalSize::new(WIDTH + 8.0, HEIGHT.load(Ordering::Relaxed) as f64));
    *MODE.lock().unwrap() = mode.clone();
    if mode == "off" {
        let _ = w.hide();
        return;
    }
    let open = mode == "pinned";
    OPEN.store(open, Ordering::Relaxed);
    if let Some((out, tucked, y)) = spots(&w) {
        let _ = w.set_position(PhysicalPosition::new(if open { out } else { tucked }, y));
    }
    let _ = w.show();
    let _ = w.emit("widget-mode", &mode);
}

/// In auto mode, watches the pointer: touching the right edge where the widget is slides it out, moving away
/// slides it back. Polls the pointer because a window that is not in front gets no hover events.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut away = 0u32;
        loop {
            std::thread::sleep(Duration::from_millis(90));
            if *MODE.lock().unwrap() != "auto" {
                continue;
            }
            let (Some(w), Ok(p)) = (win(&app), app.cursor_position()) else { continue };
            let (Ok(pos), Ok(size)) = (w.outer_position(), w.outer_size()) else { continue };
            let open = OPEN.load(Ordering::Relaxed);
            let k = w.scale_factor().unwrap_or(1.0);
            let (top, bottom) = (pos.y as f64, pos.y as f64 + size.height as f64);
            let in_band = p.y >= top && p.y <= bottom;
            // Closed: the pointer must reach the screen edge strip. Open: anywhere over the panel keeps it out.
            let right_edge = pos.x as f64 + size.width as f64;
            let inside = in_band
                && if LEFT.load(Ordering::Relaxed) {
                    p.x <= right_edge + if open { 4.0 } else { 2.0 } * k
                } else {
                    p.x >= pos.x as f64 - if open { 4.0 } else { 2.0 } * k
                };
            if inside {
                away = 0;
                if !open {
                    OPEN.store(true, Ordering::Relaxed);
                    slide_to(&w, true);
                }
            } else if open {
                away += 1;
                if away >= 5 {
                    OPEN.store(false, Ordering::Relaxed);
                    slide_to(&w, false);
                }
            }
        }
    });
}

/// The pointer entered (true) or left (false) the widget. Only moves it in auto mode.
#[tauri::command]
pub fn widget_hover(app: AppHandle, inside: bool) {
    if *MODE.lock().unwrap() != "auto" || OPEN.swap(inside, Ordering::Relaxed) == inside {
        return;
    }
    if let Some(w) = win(&app) {
        slide_to(&w, inside);
    }
}
