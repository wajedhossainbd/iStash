//! Fan speed and temperatures. Fans come from the SMC (the System Management Controller) through its IOKit user
//! client; temperatures on Apple silicon come from the HID temperature sensors, as tools such as Stats read them.
//! Setting fan speed needs root: a copy of this binary, installed once with the owner's password as a setuid helper,
//! runs `--fan auto|max|rpm N` and nothing else (see `fan_cli`, and `set_fan` for the install).
//! shortcut: sensor names differ by model; CPU and GPU are told apart by name ("tdie", "GPU"), good enough for
//! averages, not for a per-core list.

use serde_json::{json, Value};
use std::ffi::{c_char, c_void};

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Vers {
    major: u8,
    minor: u8,
    build: u8,
    reserved: u8,
    release: u16,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct PLimit {
    version: u16,
    length: u16,
    cpu: u32,
    gpu: u32,
    mem: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct KeyInfo {
    size: u32,
    kind: u32,
    attributes: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct KeyData {
    key: u32,
    vers: Vers,
    plimit: PLimit,
    info: KeyInfo,
    result: u8,
    status: u8,
    data8: u8,
    data32: u32,
    bytes: [u8; 32],
}

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    fn IOServiceGetMatchingService(main_port: u32, matching: *mut c_void) -> u32;
    fn IOServiceOpen(service: u32, task: u32, kind: u32, connect: *mut u32) -> i32;
    fn IOServiceClose(connect: u32) -> i32;
    fn IOObjectRelease(object: u32) -> i32;
    fn IOConnectCallStructMethod(
        connect: u32,
        selector: u32,
        input: *const c_void,
        in_size: usize,
        output: *mut c_void,
        out_size: *mut usize,
    ) -> i32;
    fn IOHIDEventSystemClientCreate(allocator: *const c_void) -> *mut c_void;
    fn IOHIDEventSystemClientSetMatching(client: *mut c_void, matching: *const c_void) -> i32;
    fn IOHIDEventSystemClientCopyServices(client: *mut c_void) -> *const c_void;
    fn IOHIDServiceClientCopyProperty(service: *const c_void, key: *const c_void) -> *const c_void;
    fn IOHIDServiceClientCopyEvent(service: *const c_void, kind: i64, options: i32, timestamp: i64) -> *const c_void;
    fn IOHIDEventGetFloatValue(event: *const c_void, field: i32) -> f64;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
    fn CFArrayGetCount(a: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(a: *const c_void, i: isize) -> *const c_void;
    fn CFStringCreateWithCString(alloc: *const c_void, s: *const c_char, encoding: u32) -> *const c_void;
    fn CFStringGetCString(s: *const c_void, buf: *mut c_char, size: isize, encoding: u32) -> bool;
    fn CFNumberCreate(alloc: *const c_void, kind: isize, value: *const c_void) -> *const c_void;
    fn CFDictionaryCreate(
        alloc: *const c_void,
        keys: *const *const c_void,
        values: *const *const c_void,
        n: isize,
        kcb: *const c_void,
        vcb: *const c_void,
    ) -> *const c_void;
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
}
const UTF8: u32 = 0x0800_0100;

fn fourcc(s: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*s)
}

struct Smc(u32);
impl Smc {
    fn open() -> Option<Smc> {
        // SAFETY: standard IOKit service lookup and open; the connection is closed in Drop.
        unsafe {
            let svc = IOServiceGetMatchingService(0, IOServiceMatching(c"AppleSMC".as_ptr()));
            if svc == 0 {
                return None;
            }
            let mut conn = 0;
            #[allow(deprecated)]
            let r = IOServiceOpen(svc, libc::mach_task_self(), 0, &mut conn);
            IOObjectRelease(svc);
            (r == 0).then_some(Smc(conn))
        }
    }
    fn call(&self, input: &KeyData) -> Option<KeyData> {
        let mut out = KeyData::default();
        let mut size = std::mem::size_of::<KeyData>();
        // SAFETY: selector 2 is the SMC's struct call; input and output are the 80-byte SMCKeyData_t.
        let r = unsafe {
            IOConnectCallStructMethod(
                self.0,
                2,
                input as *const _ as *const c_void,
                size,
                &mut out as *mut _ as *mut c_void,
                &mut size,
            )
        };
        (r == 0 && out.result == 0).then_some(out)
    }
    /// Writes a key, sized and typed as the SMC reports it. Needs root.
    fn write(&self, key: &[u8; 4], value: f64) -> bool {
        let Some(info) = self.call(&KeyData { key: fourcc(key), data8: 9, ..Default::default() }).map(|o| o.info)
        else {
            return false;
        };
        let mut bytes = [0u8; 32];
        match &info.kind.to_be_bytes() {
            b"flt " => bytes[..4].copy_from_slice(&(value as f32).to_le_bytes()),
            b"ui8 " => bytes[0] = value as u8,
            b"ui16" => bytes[..2].copy_from_slice(&(value as u16).to_be_bytes()),
            b"fpe2" => bytes[..2].copy_from_slice(&((value * 4.0) as u16).to_be_bytes()),
            _ => return false,
        }
        self.call(&KeyData { key: fourcc(key), info, data8: 6, bytes, ..Default::default() }).is_some()
    }
    /// A key's value as a number, for the types fans use.
    fn read(&self, key: &[u8; 4]) -> Option<f64> {
        let info = self.call(&KeyData { key: fourcc(key), data8: 9, ..Default::default() })?.info;
        let out = self.call(&KeyData { key: fourcc(key), info, data8: 5, ..Default::default() })?;
        let b = out.bytes;
        Some(match &info.kind.to_be_bytes() {
            b"flt " => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
            b"fpe2" => u16::from_be_bytes([b[0], b[1]]) as f64 / 4.0,
            b"ui8 " => b[0] as f64,
            b"ui16" => u16::from_be_bytes([b[0], b[1]]) as f64,
            _ => return None,
        })
    }
}
impl Drop for Smc {
    fn drop(&mut self) {
        // SAFETY: closes the connection opened in `open`.
        unsafe { IOServiceClose(self.0) };
    }
}

fn fans() -> Vec<Value> {
    let Some(smc) = Smc::open() else { return vec![] };
    let n = smc.read(b"FNum").unwrap_or(0.0) as u8;
    (0..n.min(4))
        .filter_map(|i| {
            let d = b'0' + i;
            let rpm = smc.read(&[b'F', d, b'A', b'c'])?;
            let manual = smc.read(&[b'F', d, b'M', b'd']).unwrap_or(0.0) >= 1.0;
            Some(json!({ "rpm": rpm.round(), "min": smc.read(&[b'F', d, b'M', b'n']).map(f64::round), "max": smc.read(&[b'F', d, b'M', b'x']).map(f64::round),
                "manual": manual, "target": smc.read(&[b'F', d, b'T', b'g']).map(f64::round) }))
        })
        .collect()
}

fn temps() -> Vec<(String, f64)> {
    let mut out = vec![];
    // SAFETY: the HID event system calls below follow their documented ownership: Create/Copy results are released.
    unsafe {
        let client = IOHIDEventSystemClientCreate(std::ptr::null());
        if client.is_null() {
            return out;
        }
        let (page, usage): (i32, i32) = (0xff00, 5); // Apple vendor page, temperature sensors
        let keys = [
            CFStringCreateWithCString(std::ptr::null(), c"PrimaryUsagePage".as_ptr(), UTF8),
            CFStringCreateWithCString(std::ptr::null(), c"PrimaryUsage".as_ptr(), UTF8),
        ];
        let vals = [
            CFNumberCreate(std::ptr::null(), 3, &page as *const _ as *const c_void),
            CFNumberCreate(std::ptr::null(), 3, &usage as *const _ as *const c_void),
        ];
        let dict = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            vals.as_ptr(),
            2,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        );
        IOHIDEventSystemClientSetMatching(client, dict);
        let services = IOHIDEventSystemClientCopyServices(client);
        let product = CFStringCreateWithCString(std::ptr::null(), c"Product".as_ptr(), UTF8);
        if !services.is_null() {
            for i in 0..CFArrayGetCount(services) {
                let svc = CFArrayGetValueAtIndex(services, i);
                let ev = IOHIDServiceClientCopyEvent(svc, 15, 0, 0); // kIOHIDEventTypeTemperature
                if ev.is_null() {
                    continue;
                }
                let c = IOHIDEventGetFloatValue(ev, 15 << 16);
                CFRelease(ev);
                let name_ref = IOHIDServiceClientCopyProperty(svc, product);
                let mut buf = [0 as c_char; 64];
                let name = if !name_ref.is_null() && CFStringGetCString(name_ref, buf.as_mut_ptr(), 64, UTF8) {
                    std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
                } else {
                    String::new()
                };
                if !name_ref.is_null() {
                    CFRelease(name_ref);
                }
                if c > 5.0 && c < 130.0 {
                    out.push((name, c));
                }
            }
            CFRelease(services);
        }
        for k in keys.iter().chain(vals.iter()) {
            CFRelease(*k);
        }
        CFRelease(product);
        CFRelease(dict);
        CFRelease(client);
    }
    out
}

/// Fans (rpm, min, max) and temperatures: CPU and GPU averages and peaks, plus every sensor by name.
#[tauri::command(async)]
pub fn sensors() -> Value {
    let t = temps();
    let group = |f: &dyn Fn(&str) -> bool| {
        let v: Vec<f64> = t.iter().filter(|(n, _)| f(n)).map(|(_, c)| *c).collect();
        (!v.is_empty()).then(|| json!({ "avg": (v.iter().sum::<f64>() / v.len() as f64 * 10.0).round() / 10.0, "max": (v.iter().cloned().fold(0.0, f64::max) * 10.0).round() / 10.0 }))
    };
    json!({
        "fans": fans(),
        "cpu": group(&|n: &str| n.contains("tdie") || n.contains("CPU")),
        "gpu": group(&|n: &str| n.contains("GPU")),
        "sensors": t.iter().map(|(n, c)| json!({ "name": n, "c": (c * 10.0).round() / 10.0 })).collect::<Vec<_>>(),
    })
}

/// The root helper's whole job: `auto` gives the fans back to macOS, `max` runs them at full speed, `rpm N` holds
/// them at N, clamped to each fan's own range. Returns the process exit code.
pub fn fan_cli(args: &[String]) -> i32 {
    let target = match args {
        [m] if m == "auto" => None,
        [m] if m == "max" => Some(f64::MAX),
        [m, n] if m == "rpm" => match n.parse::<f64>() {
            Ok(v) if v.is_finite() => Some(v),
            _ => return 2,
        },
        _ => return 2,
    };
    let Some(smc) = Smc::open() else { return 3 };
    let n = smc.read(b"FNum").unwrap_or(0.0) as u8;
    // Apple silicon only takes manual fan modes after this test-mode key is set; it can take a few seconds.
    smc.write(b"Ftst", if target.is_some() { 1.0 } else { 0.0 });
    let mut ok = true;
    for i in 0..n.min(4) {
        let d = b'0' + i;
        let (lo, hi) =
            (smc.read(&[b'F', d, b'M', b'n']).unwrap_or(0.0), smc.read(&[b'F', d, b'M', b'x']).unwrap_or(0.0));
        let done = (0..40).any(|_| {
            let w = match target {
                None => smc.write(&[b'F', d, b'M', b'd'], 0.0),
                Some(t) => {
                    smc.write(&[b'F', d, b'M', b'd'], 1.0) && smc.write(&[b'F', d, b'T', b'g'], t.clamp(lo, hi.max(lo)))
                }
            };
            if !w {
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            w
        });
        ok &= done;
    }
    if ok {
        0
    } else {
        4
    }
}

/// Whether the fan helper is installed: owned by root and setuid.
pub fn fan_ready() -> bool {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(HELPER).map(|m| m.uid() == 0 && m.mode() & 0o4000 != 0).unwrap_or(false)
}

const HELPER: &str = "/Library/PrivilegedHelperTools/bd.wajed.stowaway.fan";

/// Sets the fans: "auto", "max", or "rpm" with a speed. The first time, macOS asks for the password to install the
/// helper; after that it runs without asking.
#[tauri::command(async)]
pub fn set_fan(mode: String, rpm: Option<u32>) -> Result<(), String> {
    let args: Vec<String> = match (mode.as_str(), rpm) {
        ("auto", _) | ("max", _) => vec![mode.clone()],
        ("rpm", Some(n)) if n <= 20_000 => vec!["rpm".into(), n.to_string()],
        _ => return Err("Unknown fan setting.".into()),
    };
    if !fan_ready() {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe =
            exe.to_str().filter(|p| !p.contains('\'') && !p.contains('"')).ok_or("iStash's path can't be used.")?;
        let script = format!(
            "do shell script \"mkdir -p /Library/PrivilegedHelperTools && cp '{exe}' '{HELPER}' && chown root:wheel '{HELPER}' && chmod 4755 '{HELPER}'\" with prompt \"iStash needs your password once to control the fans.\" with administrator privileges"
        );
        let out = std::process::Command::new("/usr/bin/osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err("Fan control wasn't set up. Nothing was changed.".into());
        }
    }
    let st = std::process::Command::new(HELPER).arg("--fan").args(&args).status().map_err(|e| e.to_string())?;
    match st.code() {
        Some(0) => Ok(()),
        Some(4) => Err("The fan didn't accept the setting. macOS is still in charge of it.".into()),
        _ => Err("Fan control failed. macOS is still in charge of the fans.".into()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn layout_matches_the_smc() {
        assert_eq!(std::mem::size_of::<super::KeyData>(), 80);
    }

    #[test]
    fn fan_cli_refuses_anything_else() {
        let a = |v: &[&str]| super::fan_cli(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(a(&[]), 2);
        assert_eq!(a(&["rpm"]), 2);
        assert_eq!(a(&["rpm", "NaN"]), 2);
        assert_eq!(a(&["rm", "-rf"]), 2);
    }

    #[test]
    #[ignore = "reads this Mac's sensors; run by hand"]
    fn reads_this_mac() {
        eprintln!("{}", serde_json::to_string_pretty(&super::sensors()).unwrap());
    }
}
