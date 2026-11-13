//! Live CPU, memory, swap and network for the UI, read from the kernel by stowaway-core's Sampler, plus its
//! memory forecast. Read only.
//! shortcut: the shell samples on its own thread until the agent serves metrics over its socket; then this goes.

use serde_json::{json, Value};
use std::ffi::{c_void, CStr};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;
use stowaway_core::forecast::{forecast, Level, Sample, DISK_FLOOR};
use stowaway_core::metrics::{Sampler, Snapshot};

const EVERY: Duration = Duration::from_secs(2);
const HOUR: f64 = 3600.0;

#[derive(Default)]
struct Live {
    last: Option<Snapshot>,
    samples: Vec<Sample>, // every reading of the last hour, for the forecast and the swap chart
    cpu: Vec<f64>,
    net_in: Vec<f64>,
    used: (u64, u64), // memory in use, compressed
}

static LIVE: Mutex<Option<Live>> = Mutex::new(None);

pub fn start() {
    std::thread::spawn(|| {
        let mut sampler = Sampler::new();
        loop {
            let snap = sampler.sample();
            let used = vm_used();
            if let Ok(mut g) = LIVE.lock() {
                let l = g.get_or_insert_with(Live::default);
                l.samples.push(snap.forecast_sample());
                l.samples.retain(|s| s.t > snap.t - HOUR);
                for (v, x) in [(&mut l.cpu, snap.cpu_pct), (&mut l.net_in, snap.net_in_bps)] {
                    v.push(x);
                    if v.len() > 30 {
                        v.remove(0);
                    }
                }
                l.used = used;
                l.last = Some(snap);
            }
            std::thread::sleep(EVERY);
        }
    });
}

fn sysctl<T: Copy>(name: &CStr) -> Option<T> {
    let mut v = std::mem::MaybeUninit::<T>::uninit();
    let mut len = std::mem::size_of::<T>();
    // SAFETY: the kernel writes at most len bytes into v; it is read back only if the size matches.
    let r = unsafe { libc::sysctlbyname(name.as_ptr(), v.as_mut_ptr() as *mut c_void, &mut len, std::ptr::null_mut(), 0) };
    (r == 0 && len == std::mem::size_of::<T>()).then(|| unsafe { v.assume_init() })
}

/// (in use, compressed) the way Activity Monitor counts them: app memory + wired + compressed.
#[allow(deprecated)] // libc points to the mach2 crate for these
fn vm_used() -> (u64, u64) {
    let mut info: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
    let mut count = libc::HOST_VM_INFO64_COUNT;
    // SAFETY: host_statistics64 fills a vm_statistics64 of HOST_VM_INFO64_COUNT integers.
    let r = unsafe { libc::host_statistics64(libc::mach_host_self(), libc::HOST_VM_INFO64, &mut info as *mut _ as *mut i32, &mut count) };
    if r != 0 {
        return (0, 0);
    }
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    let app = (info.internal_page_count as u64).saturating_sub(info.purgeable_count as u64);
    let compressed = info.compressor_page_count as u64 * page;
    ((app + info.wire_count as u64) * page + compressed, compressed)
}

fn cores() -> String {
    match (sysctl::<i32>(c"hw.perflevel0.logicalcpu"), sysctl::<i32>(c"hw.perflevel1.logicalcpu")) {
        (Some(p), Some(e)) if e > 0 => format!("{p} performance, {e} efficiency cores"),
        _ => format!("{} cores", std::thread::available_parallelism().map_or(1, |n| n.get())),
    }
}

/// The app bundle for a process-group name, so the UI can show its real icon and offer Quit.
fn bundle(name: &str) -> Option<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    [format!("/Applications/{name}.app"), format!("/System/Applications/{name}.app"), format!("/System/Applications/Utilities/{name}.app"), format!("{home}/Applications/{name}.app")]
        .into_iter()
        .find(|p| Path::new(p).is_dir())
}

/// Everything the Overview, Memory screen and sidebar need, from the latest reading; null until the first one.
#[tauri::command]
pub fn memory() -> Value {
    let g = LIVE.lock().ok();
    let Some(l) = g.as_ref().and_then(|g| g.as_ref()) else { return Value::Null };
    let Some(s) = &l.last else { return Value::Null };
    let f = forecast(&l.samples);
    // One point per minute for the chart, newest last.
    let mut history: Vec<u64> = l.samples.iter().rev().step_by(30).map(|x| x.swap_used as u64).collect();
    history.reverse();
    let mut apps: Vec<_> = s.apps.iter().filter(|a| a.rss > 50_000_000).collect();
    apps.sort_by(|a, b| b.rss.cmp(&a.rss));
    json!({
        "t": s.t, "cpu": l.cpu, "cores": cores(), "netIn": l.net_in, "netOut": s.net_out_bps,
        "memTotal": s.mem_total, "memUsed": l.used.0, "compressed": l.used.1, "freePct": s.mem_free_pct,
        "swap": s.swap_used, "diskFree": s.disk_free, "swapLimit": s.swap_used + s.disk_free.saturating_sub(DISK_FLOOR as u64),
        "history": history,
        "level": match f.level { Level::Ok => "ok", Level::Watch => "watch", Level::Warn => "warn", Level::Critical => "critical" },
        "minutesLeft": f.minutes_left, "reason": f.reason,
        "apps": apps.iter().take(8).map(|a| json!({ "name": a.name, "mem": a.rss, "cpu": a.cpu_pct, "path": bundle(&a.name) })).collect::<Vec<_>>(),
    })
}

/// Asks an app to quit normally (it can save its work or refuse), the way the Dock's Quit does.
#[tauri::command]
pub fn quit_app(path: String) -> Result<(), String> {
    if !path.ends_with(".app") || !Path::new(&path).is_dir() {
        return Err("not an app bundle".into());
    }
    let ok = Command::new("/usr/bin/osascript")
        .args(["-e", "on run argv", "-e", "tell application (item 1 of argv) to quit", "-e", "end run", &path])
        .status()
        .map_err(|e| e.to_string())?
        .success();
    ok.then_some(()).ok_or_else(|| "The app didn’t quit. It may be waiting for you to save something.".into())
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_this_mac() {
        super::start();
        std::thread::sleep(std::time::Duration::from_millis(4500)); // two readings, so CPU has a delta
        let r = super::memory();
        let n = |k: &str| r[k].as_f64().unwrap_or(-1.0);
        eprintln!("{}", serde_json::to_string_pretty(&r).unwrap());
        assert!(n("memTotal") > 1e9 && n("memUsed") > 0.0 && n("memUsed") <= n("memTotal"), "memory figures");
        assert!((0.0..=100.0).contains(&n("freePct")), "free percentage");
        assert!(n("swapLimit") >= n("swap"), "swap limit");
        assert!(r["apps"].as_array().is_some_and(|a| !a.is_empty()), "apps");
    }
}
