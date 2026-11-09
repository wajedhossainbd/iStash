//! Eject guard: a DiskArbitration unmount-approval callback. macOS asks it before any volume is
//! unmounted (Finder eject, `diskutil`, `hdiutil detach`); returning a reason refuses the unmount.
//! A forced unmount cannot be refused.

use std::ffi::{c_char, c_void, CStr};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;

type CFTypeRef = *const c_void;
type CFStringRef = *const c_void;
type CFRunLoopRef = *mut c_void;
type DASessionRef = *mut c_void;
type DADiskRef = *mut c_void;
type DADissenterRef = *mut c_void;
type Callback = extern "C" fn(DADiskRef, *mut c_void) -> DADissenterRef;

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFRunLoopDefaultMode: CFStringRef;
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopRunInMode(mode: CFStringRef, seconds: f64, return_after_source: u8) -> i32;
    fn CFRunLoopStop(rl: CFRunLoopRef);
    fn CFRelease(cf: CFTypeRef);
    fn CFDictionaryGetValue(dict: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
    fn CFURLGetFileSystemRepresentation(url: CFTypeRef, resolve: u8, buf: *mut u8, len: isize) -> u8;
    fn CFStringCreateWithBytes(alloc: CFTypeRef, bytes: *const u8, len: isize, enc: u32, ext: u8) -> CFStringRef;
}

#[link(name = "DiskArbitration", kind = "framework")]
extern "C" {
    static kDADiskDescriptionVolumePathKey: CFStringRef;
    fn DASessionCreate(alloc: CFTypeRef) -> DASessionRef;
    fn DASessionScheduleWithRunLoop(s: DASessionRef, rl: CFRunLoopRef, mode: CFStringRef);
    fn DASessionUnscheduleFromRunLoop(s: DASessionRef, rl: CFRunLoopRef, mode: CFStringRef);
    fn DARegisterDiskUnmountApprovalCallback(s: DASessionRef, m: CFTypeRef, cb: Callback, ctx: *mut c_void);
    fn DAUnregisterApprovalCallback(s: DASessionRef, cb: *mut c_void, ctx: *mut c_void);
    fn DADiskCopyDescription(d: DADiskRef) -> CFTypeRef;
    fn DADissenterCreate(alloc: CFTypeRef, status: i32, reason: CFStringRef) -> DADissenterRef;
}

const UTF8: u32 = 0x0800_0100;
/// kDAReturnBusy = err_local | err_local_diskarbitration | 0x02.
const DA_RETURN_BUSY: i32 = 0xF8DA_0002_u32 as i32;

/// Given a volume's mount point, the reason to refuse its unmount, or None to allow it.
pub type Policy = dyn Fn(&Path) -> Option<String> + Send + Sync;

/// Volume path of a disk, if it is mounted.
unsafe fn volume_path(disk: DADiskRef) -> Option<PathBuf> {
    let desc = DADiskCopyDescription(disk);
    if desc.is_null() {
        return None;
    }
    let url = CFDictionaryGetValue(desc, kDADiskDescriptionVolumePathKey);
    let mut buf = [0u8; 1024];
    let ok = !url.is_null() && CFURLGetFileSystemRepresentation(url, 1, buf.as_mut_ptr(), buf.len() as isize) != 0;
    CFRelease(desc);
    ok.then(|| PathBuf::from(CStr::from_ptr(buf.as_ptr() as *const c_char).to_string_lossy().into_owned()))
}

extern "C" fn on_unmount(disk: DADiskRef, ctx: *mut c_void) -> DADissenterRef {
    // SAFETY: ctx is the boxed policy, freed by the guard thread only after it unregisters.
    let policy = unsafe { &*(ctx as *const Box<Policy>) };
    let Some(path) = (unsafe { volume_path(disk) }) else { return std::ptr::null_mut() };
    // A panic must not unwind into DiskArbitration; allow the unmount instead.
    let Ok(Some(reason)) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| policy(&path))) else {
        return std::ptr::null_mut();
    };
    // SAFETY: creates a CFString (released here; the dissenter retains it) and a dissenter that
    // DiskArbitration takes ownership of.
    unsafe {
        let s = CFStringCreateWithBytes(std::ptr::null(), reason.as_ptr(), reason.len() as isize, UTF8, 0);
        let d = DADissenterCreate(std::ptr::null(), DA_RETURN_BUSY, s);
        if !s.is_null() {
            CFRelease(s);
        }
        d
    }
}

/// Runs the policy for every unmount until dropped.
pub struct EjectGuard {
    stop: Arc<AtomicBool>,
    runloop: usize,
    thread: Option<JoinHandle<()>>,
}

impl EjectGuard {
    pub fn start(policy: Box<Policy>) -> EjectGuard {
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let flag = stop.clone();
        let thread = std::thread::spawn(move || {
            let ctx = Box::into_raw(Box::new(policy)) as *mut c_void;
            // SAFETY: standard DiskArbitration session on this thread's run loop; torn down below in
            // reverse order before ctx is freed.
            unsafe {
                let session = DASessionCreate(std::ptr::null());
                let rl = CFRunLoopGetCurrent();
                DARegisterDiskUnmountApprovalCallback(session, std::ptr::null(), on_unmount, ctx);
                DASessionScheduleWithRunLoop(session, rl, kCFRunLoopDefaultMode);
                let _ = tx.send(rl as usize);
                while !flag.load(Ordering::Acquire) {
                    CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.25, 0);
                }
                DASessionUnscheduleFromRunLoop(session, rl, kCFRunLoopDefaultMode);
                DAUnregisterApprovalCallback(session, on_unmount as *mut c_void, ctx);
                CFRelease(session);
                drop(Box::from_raw(ctx as *mut Box<Policy>));
            }
        });
        let runloop = rx.recv().expect("eject guard thread started");
        EjectGuard { stop, runloop, thread: Some(thread) }
    }
}

impl Drop for EjectGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        // SAFETY: CFRunLoopStop may be called from any thread; the run loop lives until the thread ends.
        unsafe { CFRunLoopStop(self.runloop as CFRunLoopRef) };
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
