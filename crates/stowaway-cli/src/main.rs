//! stowaway-cli: exercise the Stowaway core from a terminal.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use stowaway_core::rules::Catalog;
use stowaway_core::{procs, scan, Fail, Result};

const USAGE: &str = "usage: stowaway-cli <command>
  scan [--min-mb N]           big folders under home with verdicts (default 200 MB)
  who-uses <path>             apps and programs using a folder right now
  quit <path>                 quit them (apps normally, programs with SIGTERM)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    let cat = Catalog::for_user();
    let res = match a.as_slice() {
        ["scan", rest @ ..] => cmd_scan(&cat, rest),
        ["who-uses", p] => who_uses(&cat, &abs(p)),
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
