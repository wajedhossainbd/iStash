//! stowaway-cli: exercise the Stowaway core from a terminal.

use std::process::ExitCode;
use stowaway_core::rules::Catalog;
use stowaway_core::{scan, Result};

const USAGE: &str = "usage: stowaway-cli <command>
  scan [--min-mb N]           big folders under home with verdicts (default 200 MB)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    let cat = Catalog::for_user();
    let res = match a.as_slice() {
        ["scan", rest @ ..] => cmd_scan(&cat, rest),
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
