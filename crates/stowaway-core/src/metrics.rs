//! Live CPU, memory, swap, disk and network figures, read straight from the kernel (no helper
//! processes), so sampling every two seconds stays well under 1% CPU.

use crate::forecast::Sample;
use crate::fsx;
use crate::procs::{self, app_of};
use serde::Serialize;
use std::collections::HashMap;
use std::ffi::{c_void, CStr};
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Pressure {
    Normal,
    Warn,
    Critical,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppUse {
    pub name: String,
    /// Share of the whole Mac (all cores = 100).
    pub cpu_pct: f64,
    pub rss: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    /// Seconds since 1970.
    pub t: f64,
    /// Whole-Mac CPU use since the previous sample (0 on the first).
    pub cpu_pct: f64,
    pub load1: f64,
    pub mem_total: u64,
    /// macOS's own "free memory" figure (what `memory_pressure` prints).
    pub mem_free_pct: f64,
    pub pressure: Pressure,
    pub swap_used: u64,
    pub swap_total: u64,
    /// Free space where swap files go (the startup disk's container).
    pub disk_free: u64,
    pub net_in_bps: f64,
    pub net_out_bps: f64,
    /// Your own apps, biggest CPU first. Other users' processes are not readable without root.
    pub apps: Vec<AppUse>,
}

impl Snapshot {
    pub fn forecast_sample(&self) -> Sample {
        Sample {
            t: self.t,
            free_pct: self.mem_free_pct,
            swap_used: self.swap_used as f64,
            disk_free: self.disk_free as f64,
        }
    }
}

fn sysctl<T: Copy>(name: &CStr) -> Option<T> {
    let mut v = std::mem::MaybeUninit::<T>::uninit();
    let mut len = std::mem::size_of::<T>();
    // SAFETY: the kernel writes at most len bytes into v; we only read it back if the size matches.
    let r =
        unsafe { libc::sysctlbyname(name.as_ptr(), v.as_mut_ptr() as *mut c_void, &mut len, std::ptr::null_mut(), 0) };
    (r == 0 && len == std::mem::size_of::<T>()).then(|| unsafe { v.assume_init() })
}

fn cpu_ticks(host: libc::mach_port_t) -> Option<[u64; 4]> {
    // SAFETY: host_statistics fills a host_cpu_load_info of HOST_CPU_LOAD_INFO_COUNT integers.
    let mut info: libc::host_cpu_load_info = unsafe { std::mem::zeroed() };
    let mut count = libc::HOST_CPU_LOAD_INFO_COUNT;
    let r =
        unsafe { libc::host_statistics(host, libc::HOST_CPU_LOAD_INFO, &mut info as *mut _ as *mut i32, &mut count) };
    (r == 0).then(|| info.cpu_ticks.map(u64::from))
}

/// Bytes received and sent on Ethernet/Wi-Fi interfaces (`en*`).
/// shortcut: sums every en* interface instead of following the default route; fine for one active link.
fn net_bytes() -> (u64, u64) {
    let (mut rx, mut tx) = (0u64, 0u64);
    let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs allocates a list we walk read-only and free; AF_LINK entries carry if_data.
    unsafe {
        if libc::getifaddrs(&mut ifap) != 0 {
            return (0, 0);
        }
        let mut p = ifap;
        while !p.is_null() {
            let a = &*p;
            let name = CStr::from_ptr(a.ifa_name).to_bytes();
            if !a.ifa_addr.is_null()
                && (*a.ifa_addr).sa_family as i32 == libc::AF_LINK
                && name.starts_with(b"en")
                && !a.ifa_data.is_null()
            {
                let d = &*(a.ifa_data as *const libc::if_data);
                rx += u64::from(d.ifi_ibytes);
                tx += u64::from(d.ifi_obytes);
            }
            p = a.ifa_next;
        }
        libc::freeifaddrs(ifap);
    }
    (rx, tx)
}

/// pid → (app name, CPU time in ns, resident bytes) for every process we may read.
type ProcUsage = HashMap<i32, (String, u64, u64)>;

fn proc_usage(ns_per_tick: f64) -> ProcUsage {
    let mut pids = vec![0i32; 4096];
    // SAFETY: the buffer holds pids.len() pids; the call returns how many it wrote.
    let n = unsafe { libc::proc_listallpids(pids.as_mut_ptr() as *mut c_void, (pids.len() * 4) as i32) };
    let mut out = HashMap::new();
    let mut path = vec![0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    for &pid in &pids[..n.max(0) as usize] {
        let mut ti: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<libc::proc_taskinfo>() as i32;
        // SAFETY: proc_pidinfo writes at most `size` bytes; it fails for processes we may not read.
        if unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDTASKINFO, 0, &mut ti as *mut _ as *mut c_void, size) } != size
        {
            continue;
        }
        // SAFETY: path buffer of PROC_PIDPATHINFO_MAXSIZE bytes.
        let len = unsafe { libc::proc_pidpath(pid, path.as_mut_ptr() as *mut c_void, path.len() as u32) };
        let name = if len > 0 { app_of(&String::from_utf8_lossy(&path[..len as usize])) } else { continue };
        let cpu = ((ti.pti_total_user + ti.pti_total_system) as f64 * ns_per_tick) as u64;
        out.insert(pid, (name, cpu, ti.pti_resident_size));
    }
    out
}

/// The previous sample's counters.
struct Prev {
    at: Instant,
    ticks: [u64; 4],
    net: (u64, u64),
    procs: ProcUsage,
}

pub struct Sampler {
    host: libc::mach_port_t,
    ncpu: f64,
    ns_per_tick: f64,
    prev: Option<Prev>,
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

impl Sampler {
    #[allow(deprecated)] // libc points to the mach2 crate for these; not worth a dependency for two calls
    pub fn new() -> Sampler {
        let mut tb = libc::mach_timebase_info { numer: 1, denom: 1 };
        // SAFETY: fills the struct; mach_host_self returns this task's host port (kept for the sampler's life).
        unsafe { libc::mach_timebase_info(&mut tb) };
        let host = unsafe { libc::mach_host_self() };
        Sampler {
            host,
            ncpu: std::thread::available_parallelism().map_or(1, |n| n.get()) as f64,
            ns_per_tick: tb.numer as f64 / tb.denom as f64,
            prev: None,
        }
    }

    /// One reading. Rates (CPU, network, per-app CPU) cover the time since the previous call.
    pub fn sample(&mut self) -> Snapshot {
        let now = Instant::now();
        let ticks = cpu_ticks(self.host).unwrap_or_default();
        let net = net_bytes();
        let procs = proc_usage(self.ns_per_tick);
        let swap: libc::xsw_usage = sysctl(c"vm.swapusage").unwrap_or(unsafe { std::mem::zeroed() });
        let mut load = [0f64; 3];
        // SAFETY: fills up to 3 doubles.
        unsafe { libc::getloadavg(load.as_mut_ptr(), 3) };
        let swap_dir = if Path::new("/System/Volumes/VM").exists() { "/System/Volumes/VM" } else { "/" };

        let (mut cpu_pct, mut net_in, mut net_out) = (0.0, 0.0, 0.0);
        let mut apps: HashMap<String, AppUse> = HashMap::new();
        let dt = self.prev.as_ref().map_or(0.0, |p| now.duration_since(p.at).as_secs_f64());
        for (pid, (name, cpu, rss)) in &procs {
            let a = apps.entry(name.clone()).or_insert_with(|| AppUse { name: name.clone(), cpu_pct: 0.0, rss: 0 });
            a.rss += rss;
            if let Some(prev) = &self.prev {
                if let Some((_, before, _)) = prev.procs.get(pid) {
                    a.cpu_pct += cpu.saturating_sub(*before) as f64 / (dt * 1e9) / self.ncpu * 100.0;
                }
            }
        }
        if let Some(Prev { ticks: pt, net: pn, .. }) = &self.prev {
            let d: Vec<u64> = (0..4).map(|i| ticks[i].saturating_sub(pt[i])).collect();
            let busy = d[libc::CPU_STATE_USER as usize]
                + d[libc::CPU_STATE_SYSTEM as usize]
                + d[libc::CPU_STATE_NICE as usize];
            let all = busy + d[libc::CPU_STATE_IDLE as usize];
            cpu_pct = if all > 0 { busy as f64 * 100.0 / all as f64 } else { 0.0 };
            if dt > 0.0 {
                // if_data counters are 32-bit and wrap.
                net_in = (net.0 as u32).wrapping_sub(pn.0 as u32) as f64 / dt;
                net_out = (net.1 as u32).wrapping_sub(pn.1 as u32) as f64 / dt;
            }
        }
        let mut apps: Vec<AppUse> = apps.into_values().collect();
        apps.sort_by(|a, b| b.cpu_pct.total_cmp(&a.cpu_pct).then(b.rss.cmp(&a.rss)));
        self.prev = Some(Prev { at: now, ticks, net, procs });
        Snapshot {
            t: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64(),
            cpu_pct,
            load1: load[0],
            mem_total: sysctl::<u64>(c"hw.memsize").unwrap_or(0),
            mem_free_pct: sysctl::<i32>(c"kern.memorystatus_level").unwrap_or(0) as f64,
            pressure: match sysctl::<i32>(c"kern.memorystatus_vm_pressure_level") {
                Some(4) => Pressure::Critical,
                Some(2) => Pressure::Warn,
                _ => Pressure::Normal,
            },
            swap_used: swap.xsu_used,
            swap_total: swap.xsu_total,
            disk_free: fsx::volume(Path::new(swap_dir)).map(|v| v.free).unwrap_or(0),
            net_in_bps: net_in,
            net_out_bps: net_out,
            apps,
        }
    }
}

/// Per-app bytes/sec from `nettop -L 2 -d` (the second block holds the deltas), biggest first, top 6.
/// nettop cuts names to 15 characters ("Google Chrome H"), so `names` ({pid: app}) gives the full name.
pub fn parse_nettop(text: &str, names: &HashMap<i32, String>) -> Vec<(String, u64, u64)> {
    let last = text.split(",bytes_in,bytes_out,").last().unwrap_or("");
    let mut rates: HashMap<String, (u64, u64)> = HashMap::new();
    for line in last.trim().lines() {
        let parts: Vec<&str> = line.split(',').collect();
        let (Some(id), Some(Ok(i)), Some(Ok(o))) =
            (parts.first(), parts.get(1).map(|s| s.parse::<u64>()), parts.get(2).map(|s| s.parse::<u64>()))
        else {
            continue;
        };
        let (name, pid) = id.rsplit_once('.').unwrap_or(("", id));
        let name = pid.parse().ok().and_then(|p| names.get(&p)).map_or(name, String::as_str);
        let r = rates.entry(name.to_string()).or_default();
        r.0 += i;
        r.1 += o;
    }
    let mut v: Vec<(String, u64, u64)> =
        rates.into_iter().filter(|(_, (i, o))| i + o > 0).map(|(n, (i, o))| (n, i, o)).collect();
    v.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then(a.0.cmp(&b.0)));
    v.truncate(6);
    v
}

/// Per-app network rates over one second (runs `nettop`, so call it only while someone is looking).
pub fn app_network() -> Vec<(String, u64, u64)> {
    let apps = procs::open_apps();
    let names = procs::process_table().into_iter().map(|p| (p.pid, procs::app_or_program(&p.cmd, &apps).0)).collect();
    let out = procs::run("nettop", &["-P", "-x", "-L", "2", "-s", "1", "-d", "-J", "bytes_in,bytes_out"]);
    parse_nettop(&out, &names)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ported from the prototype's self-test.
    #[test]
    fn nettop_parsing() {
        let sample = ",bytes_in,bytes_out,\nChrome.1,99,99,\n,bytes_in,bytes_out,\nGoogle Chrome H.12,300,20,\nGoogle Chrome H.13,100,0,\nidle.5,0,0,\n";
        assert_eq!(parse_nettop(sample, &HashMap::new()), [("Google Chrome H".to_string(), 400, 20)]);
        let names = HashMap::from([(12, "Google Chrome".to_string()), (13, "Google Chrome".to_string())]);
        assert_eq!(parse_nettop(sample, &names), [("Google Chrome".to_string(), 400, 20)]);
        assert!(parse_nettop("", &HashMap::new()).is_empty());
    }

    #[test]
    fn live_sample_is_sane() {
        let mut s = Sampler::new();
        let a = s.sample();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let b = s.sample();
        assert!(a.mem_total > 1 << 30 && b.mem_free_pct > 0.0 && b.mem_free_pct <= 100.0);
        assert!(b.cpu_pct >= 0.0 && b.cpu_pct <= 100.0);
        assert!(b.disk_free > 0 && b.swap_total >= b.swap_used);
        assert!(b.apps.iter().any(|a| a.rss > 0), "own processes readable");
    }

    fn thread_cpu_ns() -> u64 {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        // SAFETY: fills ts.
        unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) };
        ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
    }

    /// The agent samples every 2 s; under 1% CPU means each sample costs < 20 ms of CPU.
    #[test]
    fn sampling_costs_under_one_percent() {
        let mut s = Sampler::new();
        s.sample();
        let t0 = thread_cpu_ns();
        for _ in 0..10 {
            s.sample();
        }
        let per_sample_ms = (thread_cpu_ns() - t0) as f64 / 10.0 / 1e6;
        assert!(per_sample_ms < 20.0, "{per_sample_ms:.1} ms CPU per sample");
        eprintln!("{per_sample_ms:.2} ms CPU per sample");
    }
}
