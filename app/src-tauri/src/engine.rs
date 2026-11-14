//! The bridge from the UI to stowaway-core: scan, dry runs, move, bring back, stop, eject and the eject guard.
//!
//! A move or bring-back runs in a child process (this same binary with `--engine`), because the engine's safety
//! model is crash recovery: Stop kills the child and `recover` puts everything back, exactly as after a power cut.
//! shortcut: the shell hosts the engine until the login agent exists; then these commands become socket calls.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use stowaway_core::drives::{self, Moved};
use stowaway_core::journal::{Journal, Op, Step};
use stowaway_core::mover::Mover;
use stowaway_core::rules::{Catalog, Verdict};
use stowaway_core::{diskarb, fsx, procs, scan};

const MIN_BYTES: u64 = 200_000_000;

struct Job {
    kind: &'static str, // "move" | "back"
    path: PathBuf,
    drive: String,
    /// Where the copy grows, measured for progress.
    growing: PathBuf,
    total: u64,
    done: u64,
    child: Child,
    stopped: bool,
}

static JOBS: Mutex<Vec<Job>> = Mutex::new(Vec::new());
/// Finished jobs the UI has not collected yet: path → error (None when it worked).
static FINISHED: Mutex<Vec<(PathBuf, Option<String>)>> = Mutex::new(Vec::new());
static GUARD_ON: AtomicBool = AtomicBool::new(true);

fn idle(_: &Path) -> Vec<String> {
    Vec::new()
}

/// `stowaway-app --engine move <path> <drive>` / `--engine back <path>`: one engine operation, then exit.
pub fn child(args: &[String]) -> Option<i32> {
    if args.get(1).map(String::as_str) != Some("--engine") {
        return None;
    }
    let cat = Catalog::for_user();
    let in_use = |p: &Path| procs::in_use(&cat, p);
    let m = Mover::new(&cat, &in_use);
    let res = match (args.get(2).map(String::as_str), args.get(3), args.get(4)) {
        (Some("move"), Some(p), Some(d)) => m.move_folder(Path::new(p), Path::new(d)),
        (Some("back"), Some(p), _) => m.bring_back(Path::new(p)),
        _ => return Some(2),
    };
    Some(match res {
        Ok(_) => 0,
        Err(e) => {
            eprint!("{e}");
            1
        }
    })
}

pub fn all_moved(cat: &Catalog, table: &[procs::Proc]) -> Vec<Moved> {
    let mut moved = drives::moved_items(cat, &Journal::for_home(&cat.home));
    moved.extend(drives::docker_moved(table, &drives::docker_state_file(&cat.home)));
    moved
}

fn objection(cat: &Catalog, mount: &Path) -> Option<String> {
    let (table, apps) = (procs::process_table(), procs::open_apps());
    let running = |m: &Moved| if procs::users_of(cat, &m.path, &table, &apps).is_empty() { vec![] } else { vec![m.app.clone()] };
    drives::eject_objection(mount, &all_moved(cat, &table), &running)
}

/// At launch: finish or undo whatever a crash interrupted, start refusing unsafe ejects, and watch jobs.
pub fn start() {
    let cat = Catalog::for_user();
    let _ = Mover::new(&cat, &idle).recover();
    // Lives as long as the app: macOS asks it before any unmount, from Finder or anywhere else.
    std::mem::forget(diskarb::EjectGuard::start(Box::new(move |mount: &Path| {
        if GUARD_ON.load(Ordering::Relaxed) { objection(&cat, mount) } else { None }
    })));
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_millis(1200));
        // Measure outside the lock: walking a big copy takes a moment.
        let growing: Vec<PathBuf> = JOBS.lock().map(|j| j.iter().map(|x| x.growing.clone()).collect()).unwrap_or_default();
        let sizes: Vec<u64> = growing.iter().map(|p| scan::disk_usage(p)).collect();
        let Ok(mut jobs) = JOBS.lock() else { continue };
        for (p, size) in growing.iter().zip(sizes) {
            if let Some(j) = jobs.iter_mut().find(|j| &j.growing == p) {
                j.done = size.min(j.total);
            }
        }
        let mut i = 0;
        while i < jobs.len() {
            match jobs[i].child.try_wait() {
                Ok(Some(status)) => {
                    let mut j = jobs.remove(i);
                    let mut err = String::new();
                    if let Some(mut e) = j.child.stderr.take() {
                        let _ = std::io::Read::read_to_string(&mut e, &mut err);
                    }
                    let error = (!status.success() && !j.stopped).then(|| if err.is_empty() { "The move stopped unexpectedly. Nothing was lost; Stowaway put things back.".into() } else { err });
                    if !status.success() {
                        let cat = Catalog::for_user();
                        let _ = Mover::new(&cat, &idle).recover();
                    }
                    if let Ok(mut f) = FINISHED.lock() {
                        f.push((j.path, error));
                    }
                }
                _ => i += 1,
            }
        }
    });
}

fn verdict(v: Verdict) -> &'static str {
    match v {
        Verdict::Move => "move",
        Verdict::App => "setting",
        Verdict::Unsupported => "refused",
        Verdict::Keep => "keep",
    }
}

/// Big app folders under home with the engine's verdicts, moved folders, and this Mac's disk. Read only.
#[tauri::command(async)]
pub fn scan_home() -> Value {
    let cat = Catalog::for_user();
    let items = scan::scan(&cat, MIN_BYTES);
    let (table, apps) = (procs::process_table(), procs::open_apps());
    let since: HashMap<PathBuf, u64> = Journal::for_home(&cat.home).moved().unwrap_or_default().into_iter().map(|e| (e.path, e.ts)).collect();
    let folders: Vec<Value> = items
        .iter()
        .filter(|i| !i.info && (i.moved || i.verdict != Verdict::Keep))
        .map(|i| {
            let users = procs::users_of(&cat, &i.path, &table, &apps);
            json!({
                "path": i.path, "show": i.show, "app": i.app, "appPath": crate::mem::bundle(&i.app),
                "verdict": if i.moved { "stowed" } else { verdict(i.verdict) }, "note": i.note, "size": i.size.unwrap_or(0),
                "drive": i.drive, "since": since.get(&i.path).map(|t| t * 1000),
                "running": !users.is_empty(), "users": procs::user_names(&cat, &i.path, &users, &apps),
            })
        })
        .collect();
    let mac = fsx::volume(Path::new("/")).ok();
    json!({
        "folders": folders, "checked": items.len(),
        "mac": { "total": mac.as_ref().map_or(0, |v| v.total), "free": mac.as_ref().map_or(0, |v| v.free) },
        "journal": Journal::for_home(&cat.home).path,
    })
}

/// Dry run. Checks everything the real operation checks and changes nothing. An open app is reported in `quits`
/// (the plan assumes it will be asked to quit first).
#[tauri::command(async)]
pub fn plan(kind: String, path: String, drive: Option<String>) -> Value {
    let cat = Catalog::for_user();
    let p = Path::new(&path);
    let users = procs::in_use(&cat, p);
    let m = Mover::new(&cat, &idle);
    let res = match kind.as_str() {
        "move" => m.plan_move(p, &Path::new("/Volumes").join(drive.unwrap_or_default())),
        _ => m.plan_bring_back(p),
    };
    match res {
        Ok(pl) => json!({ "ok": true, "size": pl.size, "dest": pl.dest, "quits": users, "sameVolume": pl.same_volume }),
        Err(e) => json!({ "ok": false, "reason": e.0 }),
    }
}

/// Start a move or bring-back. With `quit`, the apps using the folder are asked to quit first (normally, so they
/// can save). Every check runs again here; a refusal changes nothing.
#[tauri::command(async)]
pub fn start_job(kind: String, path: String, drive: Option<String>, quit: bool) -> Result<(), String> {
    let cat = Catalog::for_user();
    let p = PathBuf::from(&path);
    if JOBS.lock().map_err(|e| e.to_string())?.iter().any(|j| j.path == p) {
        return Err("This folder is already being moved.".into());
    }
    if quit {
        procs::quit_users(&cat, &p).map_err(|e| e.0)?;
    }
    let in_use = |q: &Path| procs::in_use(&cat, q);
    let m = Mover::new(&cat, &in_use);
    let back = kind != "move";
    let mount = Path::new("/Volumes").join(drive.clone().unwrap_or_default());
    let pl = if back { m.plan_bring_back(&p) } else { m.plan_move(&p, &mount) }.map_err(|e| e.0)?;
    let mut cmd = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    cmd.arg("--engine").arg(if back { "back" } else { "move" }).arg(&p);
    if !back {
        cmd.arg(&mount);
    }
    let child = cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
    let job = Job {
        kind: if back { "back" } else { "move" },
        growing: if back { fsx::sibling(&p, ".stowaway-restoring") } else { pl.dest.clone() },
        drive: scan::drive_of(&pl.dest).unwrap_or_default(),
        path: p,
        total: pl.size.max(1),
        done: 0,
        child,
        stopped: false,
    };
    JOBS.lock().map_err(|e| e.to_string())?.push(job);
    Ok(())
}

/// Running jobs with the step the journal has reached, plus jobs that ended since the last call.
#[tauri::command]
pub fn jobs() -> Value {
    let cat = Catalog::for_user();
    let open = Journal::for_home(&cat.home).open_ops().unwrap_or_default();
    let running: Vec<Value> = JOBS
        .lock()
        .map(|jobs| {
            jobs.iter()
                .map(|j| {
                    let reached = open.iter().rev().find(|e| e.path == j.path && (e.op == Op::Move) == (j.kind == "move")).map(|e| e.step);
                    // copy, check every file, link, remove the other copy
                    let step = match reached {
                        Some(Step::Linked | Step::Placed) => 3,
                        Some(Step::Copied | Step::Aside | Step::Unlinked) => 2,
                        _ if j.done >= j.total => 1,
                        _ => 0,
                    };
                    json!({ "path": j.path, "kind": j.kind, "drive": j.drive, "done": j.done, "total": j.total, "step": step })
                })
                .collect()
        })
        .unwrap_or_default();
    let finished: Vec<Value> = FINISHED.lock().map(|mut f| f.drain(..).map(|(p, e)| json!({ "path": p, "error": e })).collect()).unwrap_or_default();
    json!({ "running": running, "finished": finished })
}

/// Stop a job: end the engine process, then let recovery undo the half-done work. The original is untouched
/// until the copy has been verified, so stopping never loses data.
#[tauri::command(async)]
pub fn stop_job(path: String) -> Result<(), String> {
    let p = PathBuf::from(path);
    if let Some(j) = JOBS.lock().map_err(|e| e.to_string())?.iter_mut().find(|j| j.path == p) {
        j.stopped = true;
        let _ = j.child.kill();
    }
    Ok(()) // the watcher reaps the child and runs recovery
}

/// Eject a drive, unless apps that keep data on it are open (the same rule the eject guard applies to Finder).
#[tauri::command(async)]
pub fn eject(name: String) -> Result<(), String> {
    let mount = Path::new("/Volumes").join(&name);
    if JOBS.lock().map_err(|e| e.to_string())?.iter().any(|j| j.drive == name) {
        return Err(format!("{name} is in use: a folder is being copied. Stop it or wait. Nothing was changed."));
    }
    if let Some(why) = objection(&Catalog::for_user(), &mount) {
        return Err(why);
    }
    let out = Command::new("/usr/sbin/diskutil").arg("eject").arg(&mount).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("macOS couldn’t eject {name}: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// Ask the apps that use a moved folder on this drive to quit (before an eject the user confirmed).
#[tauri::command(async)]
pub fn quit_drive_users(name: String) -> Result<(), String> {
    let cat = Catalog::for_user();
    for m in all_moved(&cat, &procs::process_table()).iter().filter(|m| m.drive == name) {
        procs::quit_users(&cat, &m.path).map_err(|e| e.0)?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_guard(on: bool) {
    GUARD_ON.store(on, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "reads the real home folder; run by hand"]
    fn scans_this_mac() {
        let r = super::scan_home();
        eprintln!("{} folders, mac {}", r["folders"].as_array().map_or(0, Vec::len), r["mac"]);
        eprintln!("{}", serde_json::to_string(&r["folders"][0]).unwrap());
        assert!(r["mac"]["total"].as_u64().unwrap_or(0) > 0);
    }
}
