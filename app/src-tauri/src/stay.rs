//! Keeps iStash running in the menu bar and on the screen edge when it is quit from the Dock or with ⌘Q.
//! macOS asks the app delegate `applicationShouldTerminate:` before quitting; the window library doesn't implement
//! it, so this adds it at launch. It answers "not now" and hides iStash to the menu bar, except for a real quit
//! (menu-bar "Quit iStash") and for log out, restart or shut down, which must never be held up.

use objc2::runtime::{AnyObject, Imp, Sel};
use objc2::{class, msg_send, sel};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use tauri::AppHandle;

pub static REAL_QUIT: AtomicBool = AtomicBool::new(false);
static APP: OnceLock<AppHandle> = OnceLock::new();

/// True when the quit comes from the system (log out, restart, shut down): those Apple Events carry a reason.
fn system_quit() -> bool {
    // SAFETY: plain Foundation calls on the main thread, where AppKit calls this delegate method.
    unsafe {
        let mgr: *mut AnyObject = msg_send![class!(NSAppleEventManager), sharedAppleEventManager];
        let ev: *mut AnyObject = msg_send![mgr, currentAppleEvent];
        if ev.is_null() {
            return false;
        }
        let why: *mut AnyObject = msg_send![ev, attributeDescriptorForKeyword: u32::from_be_bytes(*b"why?")];
        !why.is_null()
    }
}

extern "C-unwind" fn should_terminate(_this: *mut AnyObject, _cmd: Sel, _sender: *mut AnyObject) -> usize {
    const NOW: usize = 1;
    const CANCEL: usize = 0;
    if REAL_QUIT.load(Ordering::Relaxed) || system_quit() {
        return NOW;
    }
    if let Some(app) = APP.get() {
        crate::to_menu_bar(app);
    }
    CANCEL
}

/// Call once from setup, after the app delegate exists.
pub fn install(app: &AppHandle) {
    let _ = APP.set(app.clone());
    // SAFETY: adds one method, with the signature AppKit expects (NSUInteger (id, SEL, id)), to the delegate's class.
    unsafe {
        let nsapp: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![nsapp, delegate];
        if delegate.is_null() {
            return;
        }
        let cls = (*delegate).class() as *const _ as *mut objc2::runtime::AnyClass;
        let imp: Imp =
            std::mem::transmute(should_terminate as extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject) -> usize);
        objc2::ffi::class_addMethod(cls, sel!(applicationShouldTerminate:), imp, c"Q@:@".as_ptr());
    }
}
