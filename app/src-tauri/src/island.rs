//! The notch island: iStash's notifications shown as a black pill at the top centre of the screen that grows out of
//! the notch (or the menu bar on Macs without one), then shrinks back. Only iStash's own notifications: macOS gives no
//! app access to other apps' notifications. When the island is off, the same messages go to macOS's banners.

use objc2::msg_send;
use objc2::runtime::AnyObject;
use serde_json::json;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

const W: f64 = 380.0; // window, in points: room for the open island and its shadow
static ON: AtomicBool = AtomicBool::new(true);
static APP: OnceLock<AppHandle> = OnceLock::new();
static LOOK: Mutex<(String, u32)> = Mutex::new((String::new(), 5)); // glow colour, seconds on screen

pub fn init(app: &AppHandle) {
    let _ = APP.set(app.clone());
    let Some(w) = app.get_webview_window("island") else { return };
    // Above the menu bar, on every Space and over full-screen apps.
    if let Ok(ns) = w.ns_window() {
        // SAFETY: plain NSWindow setters on the main thread during setup.
        unsafe {
            let ns = ns as *mut AnyObject;
            let _: () = msg_send![ns, setLevel: 26isize]; // just above NSStatusWindowLevel (25)
            let _: () = msg_send![ns, setCollectionBehavior: (1usize | 16 | 256)]; // all Spaces, stationary, full-screen aux
            let _: () = msg_send![ns, setIgnoresMouseEvents: false];
        }
    }
}

fn place(app: &AppHandle) {
    let Some(w) = app.get_webview_window("island") else { return };
    let m = w.primary_monitor().ok().flatten();
    if let Some(m) = m {
        let (pos, size, k) = (m.position(), m.size(), m.scale_factor());
        let x = pos.x as f64 + (size.width as f64 - W * k) / 2.0;
        let _ = w.set_position(PhysicalPosition::new(x, pos.y as f64));
    }
}

/// Shows a notification: on the island if it is on, otherwise as a macOS banner. `kind` picks the island's icon.
pub fn notify(title: &str, body: &str, kind: &str) {
    let island = APP.get().filter(|_| ON.load(Ordering::Relaxed)).and_then(|a| a.get_webview_window("island").map(|w| (a.clone(), w)));
    match island {
        Some((app, w)) => {
            let a = app.clone();
            let _ = app.run_on_main_thread(move || place(&a));
            let _ = w.show();
            let (glow, secs) = LOOK.lock().map(|l| l.clone()).unwrap_or_default();
            let _ = w.emit("island", json!({ "title": title, "body": body, "kind": kind, "glow": glow, "secs": secs }));
        }
        None => {
            let _ = Command::new("/usr/bin/osascript")
                .args(["-e", "on run argv", "-e", "display notification (item 2 of argv) with title (item 1 of argv)", "-e", "end run", title, body])
                .status();
        }
    }
}

/// The island's page calls this once it has shrunk back.
#[tauri::command]
pub fn island_hide(app: AppHandle) {
    if let Some(w) = app.get_webview_window("island") {
        let _ = w.hide();
    }
}

#[tauri::command]
pub fn set_island(on: bool, glow: String, secs: u32) {
    ON.store(on, Ordering::Relaxed);
    let glow = if ["amber", "blue", "pink", "green", "white"].contains(&glow.as_str()) { glow } else { "amber".into() };
    if let Ok(mut l) = LOOK.lock() {
        *l = (glow, secs.clamp(2, 15));
    }
}

#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}

/// A small window with the camera's picture, to check the camera works. Opening it again brings it to the front.
/// macOS is asked for the camera first, so its permission prompt names iStash; if access was refused before, the
/// Camera privacy settings open next to the window.
#[tauri::command]
pub fn open_camera(app: AppHandle) -> Result<(), String> {
    use objc2::runtime::{AnyClass, Bool};
    use objc2_foundation::NSString;
    let cls = AnyClass::get(c"AVCaptureDevice").ok_or("This Mac has no camera support.")?;
    let video = NSString::from_str("vide"); // AVMediaTypeVideo
    // SAFETY: class methods of AVCaptureDevice with an NSString media type; 0 = not asked yet, 3 = allowed.
    let status: isize = unsafe { msg_send![cls, authorizationStatusForMediaType: &*video] };
    if status == 0 {
        let a = app.clone();
        let done = block2::RcBlock::new(move |_ok: Bool| {
            let b = a.clone();
            let _ = a.run_on_main_thread(move || {
                let _ = camera_window(&b);
            });
        });
        let _: () = unsafe { msg_send![cls, requestAccessForMediaType: &*video, completionHandler: &*done] };
        return Ok(());
    }
    if status != 3 {
        let _ = Command::new("/usr/bin/open").arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Camera").status();
    }
    camera_window(&app)
}

fn camera_window(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("camera") {
        let _ = w.show();
        return w.set_focus().map_err(|e| e.to_string());
    }
    tauri::WebviewWindowBuilder::new(app, "camera", tauri::WebviewUrl::App("camera.html".into()))
        .title("Camera")
        .inner_size(400.0, 300.0)
        .min_inner_size(240.0, 180.0)
        .always_on_top(true)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
