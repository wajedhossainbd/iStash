//! The popover's quick toggles and the few readings only it shows. Everything here is something macOS lets an
//! ordinary app do, with no admin password. One exception: Lock calls `SACLockScreenImmediate` in Apple's private
//! login.framework, as Apple menu > Lock Screen does, because no public call locks the screen.

use serde_json::{json, Value};
use std::process::{Child, Command};
use std::sync::{Mutex, OnceLock};

static AWAKE: Mutex<Option<Child>> = Mutex::new(None);
static SCREEN: Mutex<Option<Child>> = Mutex::new(None);

fn ok(cmd: &str, args: &[&str]) -> Result<(), String> {
    let out = Command::new(cmd).args(args).output().map_err(|e| e.to_string())?;
    out.status.success().then_some(()).ok_or_else(|| String::from_utf8_lossy(&out.stderr).trim().to_string())
}

/// CPU ticks since boot (user, system, idle, nice); the UI turns two readings into a split.
#[allow(deprecated)] // libc points to the mach2 crate for these
fn cpu_ticks() -> [u32; 4] {
    let mut info: libc::host_cpu_load_info = unsafe { std::mem::zeroed() };
    let mut count = (std::mem::size_of::<libc::host_cpu_load_info>() / std::mem::size_of::<i32>()) as u32;
    // SAFETY: host_statistics fills a host_cpu_load_info of `count` integers.
    let r = unsafe {
        libc::host_statistics(
            libc::mach_host_self(),
            libc::HOST_CPU_LOAD_INFO,
            &mut info as *mut _ as *mut i32,
            &mut count,
        )
    };
    if r != 0 {
        return [0; 4];
    }
    let t = info.cpu_ticks;
    [
        t[libc::CPU_STATE_USER as usize],
        t[libc::CPU_STATE_SYSTEM as usize],
        t[libc::CPU_STATE_IDLE as usize],
        t[libc::CPU_STATE_NICE as usize],
    ]
}

/// Seconds since boot.
fn uptime() -> u64 {
    let mut tv: libc::timeval = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::timeval>();
    // SAFETY: kern.boottime is a timeval; the kernel writes at most len bytes.
    let r = unsafe {
        libc::sysctlbyname(c"kern.boottime".as_ptr(), &mut tv as *mut _ as *mut _, &mut len, std::ptr::null_mut(), 0)
    };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if r == 0 {
        now.saturating_sub(tv.tv_sec as u64)
    } else {
        0
    }
}

/// The whole physical disk the startup volume lives on ("disk0"), whose counters `disk_io` reports.
fn boot_disk() -> &'static str {
    static DISK: OnceLock<String> = OnceLock::new();
    DISK.get_or_init(|| {
        let out = Command::new("/usr/sbin/diskutil")
            .args(["info", "-plist", "/"])
            .output()
            .map(|o| o.stdout)
            .unwrap_or_default();
        let d = plist::Value::from_reader_xml(&out[..]).ok().and_then(|v| v.into_dictionary());
        let store = d.as_ref().and_then(|d| {
            d.get("APFSPhysicalStores")?
                .as_array()?
                .first()?
                .as_dictionary()?
                .get("APFSPhysicalStore")?
                .as_string()
                .map(str::to_string)
        });
        whole_disk(store.as_deref().unwrap_or("disk0")).to_string()
    })
}

fn whole_disk(part: &str) -> &str {
    // "disk0s2" -> "disk0"
    match part.get(4..).and_then(|r| r.find('s')) {
        Some(i) => &part[..4 + i],
        None => part,
    }
}

fn awake() -> bool {
    running(&AWAKE)
}

fn running(slot: &Mutex<Option<Child>>) -> bool {
    let mut g = slot.lock().unwrap();
    if let Some(c) = g.as_mut() {
        if matches!(c.try_wait(), Ok(Some(_))) {
            *g = None;
        }
    }
    g.is_some()
}

fn desktop_icons() -> bool {
    let out = Command::new("/usr/bin/defaults").args(["read", "com.apple.finder", "CreateDesktop"]).output();
    // Unset means macOS's default: icons shown.
    out.map(|o| !matches!(String::from_utf8_lossy(&o.stdout).trim(), "0" | "false" | "NO")).unwrap_or(true)
}

#[tauri::command(async)]
pub fn sys_status() -> Value {
    json!({ "cpu": cpu_ticks(), "uptime": uptime(), "bootDisk": boot_disk(), "awake": awake(), "screenOn": running(&SCREEN), "desktopIcons": desktop_icons() })
}

/// Keeps the Mac awake (no idle or system sleep) with `caffeinate`, which ends on its own if iStash quits. The display
/// may still turn off; Keep screen on is the separate toggle for that.
#[tauri::command]
pub fn keep_awake(on: bool) -> Result<bool, String> {
    caffeinate(&AWAKE, "-is", on)
}

/// Keeps the display on.
#[tauri::command]
pub fn keep_screen(on: bool) -> Result<bool, String> {
    caffeinate(&SCREEN, "-d", on)
}

fn caffeinate(slot: &Mutex<Option<Child>>, flags: &str, on: bool) -> Result<bool, String> {
    let mut g = slot.lock().unwrap();
    if let Some(mut c) = g.take() {
        let _ = c.kill();
        let _ = c.wait();
    }
    if on {
        let pid = std::process::id().to_string();
        *g = Some(Command::new("/usr/bin/caffeinate").args([flags, "-w", &pid]).spawn().map_err(|e| e.to_string())?);
    }
    Ok(on)
}

pub fn stop_awake() {
    let _ = keep_awake(false);
    let _ = keep_screen(false);
}

/// Sleeps the display; macOS locks it when "Require password after screen saver or display is off" is on.
#[tauri::command(async)]
pub fn lock_screen() -> Result<(), String> {
    // The same call as the Lock Screen item in the Apple menu (Control-Command-Q). Turning the display off
    // only locks when "require password immediately" is set, so it is not used.
    const LOGIN: &std::ffi::CStr = c"/System/Library/PrivateFrameworks/login.framework/Versions/Current/login";
    // SAFETY: dlopen/dlsym on a system framework; the symbol is a C function taking nothing and returning int.
    unsafe {
        let h = libc::dlopen(LOGIN.as_ptr(), libc::RTLD_LAZY);
        if h.is_null() {
            return Err("macOS didn’t let iStash lock the screen. Use Control-Command-Q instead.".into());
        }
        let f = libc::dlsym(h, c"SACLockScreenImmediate".as_ptr());
        if f.is_null() {
            return Err("macOS didn’t let iStash lock the screen. Use Control-Command-Q instead.".into());
        }
        let lock: extern "C" fn() -> i32 = std::mem::transmute(f);
        lock();
    }
    Ok(())
}

/// Empties the Trash through Finder (macOS asks once to let iStash control Finder).
#[tauri::command(async)]
pub fn empty_trash() -> Result<(), String> {
    ok("/usr/bin/osascript", &["-e", "tell application \"Finder\" to empty trash"])
}

/// Shows or hides the icons on the desktop. Finder restarts to apply it.
#[tauri::command(async)]
pub fn set_desktop_icons(show: bool) -> Result<bool, String> {
    ok(
        "/usr/bin/defaults",
        &["write", "com.apple.finder", "CreateDesktop", "-bool", if show { "true" } else { "false" }],
    )?;
    ok("/usr/bin/killall", &["Finder"])?;
    Ok(show)
}

#[cfg(test)]
mod tests {
    #[test]
    fn whole_disk_of_partition() {
        assert_eq!(super::whole_disk("disk0s2"), "disk0");
        assert_eq!(super::whole_disk("disk12s1"), "disk12");
        assert_eq!(super::whole_disk("disk3"), "disk3");
    }

    #[test]
    fn reads_this_mac() {
        let v = super::sys_status();
        assert!(v["cpu"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap()).sum::<u64>() > 0);
        assert!(v["uptime"].as_u64().unwrap() > 0);
        assert!(v["bootDisk"].as_str().unwrap().starts_with("disk"));
    }

    #[test]
    fn keep_awake_starts_and_stops_caffeinate() {
        assert!(super::keep_awake(true).unwrap() && super::awake());
        super::stop_awake();
        assert!(!super::awake());
    }
}

/// Asks for the permissions iStash uses later, all at once: "camera", "finder" (Apple Events, for Empty Trash) or
/// "fans" (the one-time password for fan control). Returns whether it's allowed now.
#[tauri::command(async)]
pub fn ask_permission(kind: String) -> Result<bool, String> {
    match kind.as_str() {
        "camera" => {
            let st = crate::island::ask_camera();
            if st == 1 || st == 2 {
                // Refused before: macOS won't ask again, so open the switch for it.
                let _ = Command::new("/usr/bin/open")
                    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Camera")
                    .status();
            }
            Ok(st == 3)
        }
        "finder" => Ok(Command::new("/usr/bin/osascript")
            .args(["-e", "tell application \"Finder\" to get name"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)),
        "fans" => crate::sensors::set_fan("auto".into(), None).map(|_| true),
        _ => Err("Unknown permission.".into()),
    }
}

/// What's already allowed, without asking: camera and fan control.
#[tauri::command]
pub fn permissions_status() -> serde_json::Value {
    serde_json::json!({ "camera": crate::island::camera_status() == 3, "fans": crate::sensors::fan_ready() })
}
