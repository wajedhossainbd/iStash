//! stowaway-cli: exercise the Stowaway core from a terminal.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use stowaway_core::drives::{self, Moved};
use stowaway_core::journal::Journal;
use stowaway_core::mover::{Mover, Plan};
use stowaway_core::rules::Catalog;
use stowaway_core::{diskarb, procs, scan, Fail, Result};

const USAGE: &str = "usage: stowaway-cli <command>
  scan [--min-mb N]           big folders under home with verdicts (default 200 MB)
  who-uses <path>             apps and programs using a folder right now
  quit <path>                 quit them (apps normally, programs with SIGTERM)
  move <path> <drive> [--yes] move a folder to a drive and link it (dry run without --yes)
  bring-back <path> [--yes]   copy a moved folder back to this Mac (dry run without --yes)
  recover                     finish or undo moves a crash interrupted
  drives                      which drive holds which moved folders, and which apps need it
  guard                       refuse ejecting a drive while apps that need it are open (Ctrl-C stops)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    let cat = Catalog::for_user();
    let res = match a.as_slice() {
        ["scan", rest @ ..] => cmd_scan(&cat, rest),
        ["who-uses", p] => who_uses(&cat, &abs(p)),
        ["move", p, drive, rest @ ..] => cmd_move(&cat, &abs(p), &drive_path(drive), rest.contains(&"--yes")),
        ["bring-back", p, rest @ ..] => cmd_bring_back(&cat, &abs(p), rest.contains(&"--yes")),
        ["recover"] => recover(&cat),
        ["drives"] => cmd_drives(&cat),
        ["guard"] => cmd_guard(cat.clone()),
        ["quit", p] => procs::quit_users(&cat, &abs(p)).map(|()| println!("Nothing uses it now.")),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match res {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("stowaway: {e}");
            ExitCode::FAILURE
        }
    }
}

fn flag<'a>(rest: &[&'a str], name: &str) -> Option<&'a str> {
    rest.iter().position(|a| *a == name).and_then(|i| rest.get(i + 1).copied())
}

fn gb(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1e9)
}

fn cmd_scan(cat: &Catalog, rest: &[&str]) -> Result<()> {
    let min_mb: u64 = flag(rest, "--min-mb").and_then(|v| v.parse().ok()).unwrap_or(200);
    let t = std::time::Instant::now();
    let items = scan::scan(cat, min_mb * 1_000_000);
    for i in &items {
        let size = i.size.map(gb).unwrap_or_else(|| "—".into());
        let moved = i.target.as_ref().map(|t| format!(" → {}", t.display())).unwrap_or_default();
        println!(
            "{size:>9}  {:<11} {}{moved}  [{}]\n           {}",
            format!("{:?}", i.verdict).to_lowercase(),
            i.show,
            i.app,
            i.note
        );
    }
    eprintln!("{} folders in {:.1}s", items.len(), t.elapsed().as_secs_f64());
    Ok(())
}

/// Absolute path without resolving links (a moved folder is the link itself).
fn abs(p: &str) -> PathBuf {
    let p = Path::new(p);
    let p = if p.is_absolute() { p.to_path_buf() } else { std::env::current_dir().unwrap_or_default().join(p) };
    PathBuf::from(p.to_string_lossy().trim_end_matches('/'))
}

fn who_uses(cat: &Catalog, path: &Path) -> Result<()> {
    if path.symlink_metadata().is_err() {
        return Err(Fail(format!("{} does not exist.", path.display())));
    }
    let (table, apps) = (procs::process_table(), procs::open_apps());
    let users = procs::users_of(cat, path, &table, &apps);
    if users.is_empty() {
        println!("Nothing uses {} right now.", cat.show(path));
    }
    for name in procs::user_names(cat, path, &users, &apps) {
        println!("{name}");
    }
    for p in users {
        eprintln!("  {:>6} {}", p.pid, p.cmd.chars().take(140).collect::<String>());
    }
    Ok(())
}

/// A drive by name ("My SSD") or mount point ("/Volumes/My SSD").
fn drive_path(d: &str) -> PathBuf {
    if d.starts_with('/') {
        abs(d)
    } else {
        Path::new("/Volumes").join(d)
    }
}

fn recover(cat: &Catalog) -> Result<()> {
    let idle = |_: &Path| Vec::new(); // recovery only finishes or undoes; it never starts a copy
    for line in Mover::new(cat, &idle).recover()? {
        println!("recovered: {line}");
    }
    Ok(())
}

fn show_plan(cat: &Catalog, plan: &Plan, verb: &str) {
    let how = if plan.same_volume { "cloned (same volume)" } else { "copied and checked file by file" };
    println!("{verb} {} ({}) ↔ {}: {how}.", cat.show(&plan.path), gb(plan.size), plan.dest.display());
}

fn cmd_move(cat: &Catalog, path: &Path, drive: &Path, yes: bool) -> Result<()> {
    recover(cat)?;
    let in_use = |p: &Path| procs::in_use(cat, p);
    let m = Mover::new(cat, &in_use);
    if !yes {
        show_plan(cat, &m.plan_move(path, drive)?, "Would move");
        println!("Dry run: nothing changed. Add --yes to move.");
        return Ok(());
    }
    show_plan(cat, &m.move_folder(path, drive)?, "Moved");
    Ok(())
}

fn cmd_bring_back(cat: &Catalog, path: &Path, yes: bool) -> Result<()> {
    recover(cat)?;
    let in_use = |p: &Path| procs::in_use(cat, p);
    let m = Mover::new(cat, &in_use);
    if !yes {
        show_plan(cat, &m.plan_bring_back(path)?, "Would bring back");
        println!("Dry run: nothing changed. Add --yes to bring it back.");
        return Ok(());
    }
    show_plan(cat, &m.bring_back(path)?, "Brought back");
    Ok(())
}

fn all_moved(cat: &Catalog, table: &[procs::Proc]) -> Vec<Moved> {
    let mut moved = drives::moved_items(cat, &Journal::for_home(&cat.home));
    moved.extend(drives::docker_moved(table, &drives::docker_state_file(&cat.home)));
    moved
}

fn cmd_drives(cat: &Catalog) -> Result<()> {
    for d in drives::drives(&all_moved(cat, &procs::process_table())) {
        let state = match (d.connected, d.free, d.total) {
            (true, Some(f), Some(t)) => format!("{} free of {}, {}", gb(f), gb(t), d.fstype.unwrap_or_default()),
            _ => "NOT CONNECTED".into(),
        };
        let hold = if d.connected && !d.can_hold { " (cannot hold app data)" } else { "" };
        println!("{}  [{state}]{hold}", d.name);
        if !d.needed_by.is_empty() {
            println!("  needed by: {}", d.needed_by.join(", "));
        }
        for f in &d.folders {
            println!("    {f}");
        }
    }
    Ok(())
}

fn cmd_guard(cat: Catalog) -> Result<()> {
    let _guard = diskarb::EjectGuard::start(Box::new(move |mount: &Path| {
        let (table, apps) = (procs::process_table(), procs::open_apps());
        let running = |m: &Moved| {
            let users = procs::users_of(&cat, &m.path, &table, &apps);
            if users.is_empty() {
                vec![]
            } else {
                vec![m.app.clone()]
            }
        };
        let why = drives::eject_objection(mount, &all_moved(&cat, &table), &running);
        eprintln!("eject {}: {}", mount.display(), why.as_deref().unwrap_or("allowed"));
        why
    }));
    eprintln!("Guarding ejects. Ctrl-C to stop.");
    loop {
        std::thread::park();
    }
}
