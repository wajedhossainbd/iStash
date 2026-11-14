//! Focus (one app first, the rest in the background) and the temp-file cleaner.
//! macOS has no supported way to cap another app's memory, so Focus uses what it does allow: the other apps are
//! marked background, which gives them less processor and disk time until Focus is turned off.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use stowaway_core::{procs, scan};

static SLOWED: Mutex<Vec<i32>> = Mutex::new(Vec::new());

fn policy(flag: &str, pid: i32) -> bool {
    Command::new("/usr/sbin/taskpolicy").args([flag, "-p", &pid.to_string()]).status().is_ok_and(|s| s.success())
}

/// Puts every slowed process back to normal. Also called when iStash quits.
pub fn restore() {
    if let Ok(mut s) = SLOWED.lock() {
        for pid in s.drain(..) {
            policy("-B", pid);
        }
    }
}

/// Which processes Focus slows: apps from an Applications folder, except the focused one, macOS's own and iStash.
pub fn others(table: &[procs::Proc], focus: &str, me: i32) -> Vec<i32> {
    let focus = format!("{}/", focus.trim_end_matches('/'));
    table
        .iter()
        .filter(|p| p.pid != me && p.cmd.contains(".app/") && !p.cmd.starts_with("/System/") && !p.cmd.starts_with("/usr/") && !p.cmd.starts_with(&focus) && !p.cmd.contains("/iStash.app/") && !p.cmd.contains("/Stowaway.app/"))
        .map(|p| p.pid)
        .collect()
}

/// Focus on the app at `path`, or turn Focus off with null. Returns how many processes were slowed.
/// shortcut: processes an app starts after this call are slowed only if they inherit it; pick the app again to refresh.
#[tauri::command(async)]
pub fn focus(path: Option<String>) -> Result<usize, String> {
    restore();
    let Some(path) = path else { return Ok(0) };
    if !path.ends_with(".app") || !Path::new(&path).is_dir() {
        return Err("not an app bundle".into());
    }
    let pids: Vec<i32> = others(&procs::process_table(), &path, std::process::id() as i32).into_iter().filter(|pid| policy("-b", *pid)).collect();
    let n = pids.len();
    *SLOWED.lock().map_err(|e| e.to_string())? = pids;
    Ok(n)
}

/// Frees memory without a password: macOS's own memory_pressure tool asks for memory until 30% is free, which makes
/// macOS compress and move to disk what idle apps hold. Returns the bytes that became free.
/// (Clearing the file cache with purge needs an administrator, so it is not used.)
#[tauri::command(async)]
pub fn free_memory() -> Result<u64, String> {
    // Idle memory goes to swap on the internal disk, so leave it alone when the disk is nearly full.
    let disk = stowaway_core::fsx::volume(Path::new("/")).map_or(0, |v| v.free);
    if disk < 20_000_000_000 {
        return Err("This Mac has less than 20 GB free, and freeing memory needs room on the disk. Clean or move some folders first. Nothing was changed.".into());
    }
    let before = crate::mem::free_bytes();
    let mut child = Command::new("/usr/bin/memory_pressure").args(["-p", "30", "-y", "1", "-Q"]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map_err(|e| e.to_string())?;
    // It stops by itself once 30% is free; never let it hold memory longer than 30 seconds.
    for _ in 0..60 {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    let _ = child.kill();
    let _ = child.wait();
    std::thread::sleep(Duration::from_secs(2)); // let macOS hand the pages back
    Ok(crate::mem::free_bytes().saturating_sub(before))
}

pub const KINDS: [(&str, &str, &str); 4] = [
    ("caches", "App caches", "Apps rebuild them."),
    ("logs", "Logs", "Diagnostic logs."),
    ("derived", "Xcode build files", "Xcode rebuilds them."),
    ("temp", "Old temporary files", "Left behind by apps."),
];

/// When anything inside `p` was last changed. Links are not followed.
fn newest(p: &Path) -> SystemTime {
    let Ok(m) = p.symlink_metadata() else { return SystemTime::now() };
    let mut t = m.modified().unwrap_or_else(|_| SystemTime::now());
    if m.is_dir() {
        for e in std::fs::read_dir(p).into_iter().flatten().flatten() {
            t = t.max(newest(&e.path()));
        }
    }
    t
}

/// What one kind of cleaning would remove: only items with nothing changed inside for `days` days (temporary files
/// always wait at least three). Links are never followed or removed, so folders moved to a drive are safe.
pub fn targets(home: &Path, tmp: &Path, kind: &str, now: SystemTime, days: u64) -> Vec<PathBuf> {
    let days = if kind == "temp" { days.max(3) } else { days };
    let fresh = move |p: &Path| days > 0 && now.duration_since(newest(p)).unwrap_or_default() < Duration::from_secs(days * 86400);
    let (dir, keep): (PathBuf, Box<dyn Fn(&std::fs::DirEntry) -> bool>) = match kind {
        "caches" => (home.join("Library/Caches"), Box::new(|e| { let n = e.file_name().to_string_lossy().to_lowercase(); n.starts_with("com.apple.") || n.contains("stowaway") })),
        "logs" => (home.join("Library/Logs"), Box::new(|_| false)),
        "derived" => (home.join("Library/Developer/Xcode/DerivedData"), Box::new(|_| false)),
        "temp" => (tmp.to_path_buf(), Box::new(|_| false)),
        _ => return vec![],
    };
    // A category folder that is itself a link lives on a drive; leave it alone.
    if dir.symlink_metadata().map_or(true, |m| m.file_type().is_symlink()) {
        return vec![];
    }
    let Ok(rd) = std::fs::read_dir(&dir) else { return vec![] };
    rd.flatten().filter(|e| e.file_type().is_ok_and(|t| !t.is_symlink()) && !keep(e)).map(|e| e.path()).filter(|p| !fresh(p)).collect()
}

fn places() -> (PathBuf, PathBuf) {
    (PathBuf::from(std::env::var("HOME").unwrap_or_default()), std::env::temp_dir())
}

/// Sizes of everything the cleaner could remove. Changes nothing.
#[tauri::command(async)]
pub fn clean_scan(days: u64) -> Value {
    let (home, tmp) = places();
    let now = SystemTime::now();
    Value::Array(
        KINDS
            .iter()
            .map(|(id, name, note)| {
                let t = targets(&home, &tmp, id, now, days);
                json!({ "id": id, "name": name, "note": note, "count": t.len(), "size": t.iter().map(|p| scan::disk_usage(p)).sum::<u64>() })
            })
            .collect(),
    )
}

pub fn remove(paths: &[PathBuf]) -> u64 {
    let mut freed = 0;
    for p in paths {
        let size = scan::disk_usage(p);
        let gone = if p.is_dir() { std::fs::remove_dir_all(p) } else { std::fs::remove_file(p) };
        // A file an app holds open may refuse; whatever is left is simply not counted.
        freed += if gone.is_ok() { size } else { size.saturating_sub(scan::disk_usage(p)) };
    }
    freed
}

/// Removes the chosen kinds. Returns the bytes freed.
#[tauri::command(async)]
pub fn clean_run(kinds: Vec<String>, days: u64) -> u64 {
    let (home, tmp) = places();
    if home.as_os_str().is_empty() {
        return 0;
    }
    let now = SystemTime::now();
    kinds.iter().filter(|k| KINDS.iter().any(|x| x.0 == k.as_str())).map(|k| remove(&targets(&home, &tmp, k, now, days))).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn cleaner_spares_links_apple_caches_and_fresh_temp() {
        let root = std::env::temp_dir().join(format!("stowaway-clean-test-{}", std::process::id()));
        let (home, tmp, drive) = (root.join("home"), root.join("tmp"), root.join("drive"));
        for d in ["Library/Caches/pip", "Library/Caches/com.apple.Safari", "Library/Logs/App"] {
            fs::create_dir_all(home.join(d)).unwrap();
            fs::write(home.join(d).join("f"), vec![0u8; 4096]).unwrap();
        }
        fs::create_dir_all(drive.join("Google")).unwrap();
        fs::write(drive.join("Google/keep"), b"x").unwrap();
        std::os::unix::fs::symlink(drive.join("Google"), home.join("Library/Caches/Google")).unwrap();
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("fresh"), b"x").unwrap();
        let later = SystemTime::now() + Duration::from_secs(4 * 86400);

        let names = |k: &str, now| { let mut v: Vec<String> = targets(&home, &tmp, k, now, 0).iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect(); v.sort(); v };
        assert_eq!(names("caches", SystemTime::now()), ["pip"]);
        assert_eq!(names("temp", SystemTime::now()), Vec::<String>::new());
        assert_eq!(names("temp", later), ["fresh"]);
        assert_eq!(names("derived", SystemTime::now()), Vec::<String>::new());
        // With a 7-day rule nothing made just now goes; a week and a day later it does.
        assert!(targets(&home, &tmp, "caches", SystemTime::now(), 7).is_empty());
        assert_eq!(targets(&home, &tmp, "caches", SystemTime::now() + Duration::from_secs(8 * 86400), 7).len(), 1);
        assert!(remove(&targets(&home, &tmp, "caches", SystemTime::now(), 0)) > 0);
        assert!(!home.join("Library/Caches/pip").exists());
        assert!(home.join("Library/Caches/com.apple.Safari/f").exists() && drive.join("Google/keep").exists() && home.join("Library/Caches/Google").is_symlink());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn focus_slows_other_apps_only() {
        let p = |pid, cmd: &str| procs::Proc { pid, cmd: cmd.into() };
        let table = [p(1, "/Applications/Xcode.app/Contents/MacOS/Xcode"), p(2, "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"), p(3, "/Applications/Google Chrome.app/Contents/Frameworks/Helper.app/Contents/MacOS/Helper"),
            p(4, "/System/Applications/Mail.app/Contents/MacOS/Mail"), p(5, "/usr/bin/python3 x.py"), p(6, "/Applications/iStash.app/Contents/MacOS/stowaway-app")];
        assert_eq!(others(&table, "/Applications/Xcode.app", 6), [2, 3]);
    }
}
