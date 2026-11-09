//! Which running programs use a folder, their readable names, and quitting them.

use crate::rules::{Catalog, PROCESS_NAMES};
use crate::{Fail, Result};
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proc {
    pub pid: i32,
    pub cmd: String,
}

static BUNDLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/([^/]+)\.app/").unwrap());

pub(crate) fn run(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd).args(args).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

/// Every running process except Stowaway itself.
pub fn process_table() -> Vec<Proc> {
    parse_ps(&run("ps", &["-axww", "-o", "pid=,command="]), std::process::id() as i32)
}

pub fn parse_ps(text: &str, self_pid: i32) -> Vec<Proc> {
    text.lines()
        .filter_map(|line| {
            let (pid, cmd) = line.trim_start().split_once(' ')?;
            let pid: i32 = pid.parse().ok()?;
            let cmd = cmd.trim().to_string();
            (pid != self_pid && !cmd.contains("/Stowaway.app/")).then_some(Proc { pid, cmd })
        })
        .collect()
}

/// Names of the GUI apps open right now, from LaunchServices (needs no permission, launches nothing).
/// Background-only processes are not apps: they cannot be asked to quit and hold no unsaved work.
pub fn open_apps() -> HashSet<String> {
    parse_lsappinfo(&run("lsappinfo", &["list"]))
}

pub fn parse_lsappinfo(text: &str) -> HashSet<String> {
    static HEAD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^\s*\d+\) "([^"]+)""#).unwrap());
    let mut out = HashSet::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if let Some(c) = HEAD.captures(line) {
            current = Some(c[1].to_string());
        } else if line.contains("type=\"") {
            if let Some(name) = current.take() {
                if line.contains("type=\"Foreground\"") || line.contains("type=\"UIElement\"") {
                    out.insert(name);
                }
            }
        }
    }
    out
}

/// App bundles named in a command line, outermost first.
fn bundles(cmd: &str) -> Vec<&str> {
    BUNDLE.captures_iter(cmd).map(|c| c.get(1).unwrap().as_str()).collect()
}

/// App name for a process path: the first `*.app` in it, else the program name.
pub fn app_of(command: &str) -> String {
    let name = match bundles(command).first() {
        Some(b) => b.to_string(),
        None => command.rsplit('/').next().unwrap_or(command).to_string(),
    };
    PROCESS_NAMES.iter().find(|(k, _)| *k == name).map(|(_, v)| v.to_string()).unwrap_or(name)
}

/// (name, is_app) for a process: its app if that app is open, else the program itself
/// (python that merely lives inside Xcode.app is "Python", not "Xcode").
pub fn app_or_program(cmd: &str, apps: &HashSet<String>) -> (String, bool) {
    let b = bundles(cmd);
    if let Some(first) = b.first() {
        if apps.contains(*first) {
            return (first.to_string(), true);
        }
    }
    let program = b.last().copied().unwrap_or_else(|| cmd.split_whitespace().next().unwrap_or(cmd));
    (app_of(program), false)
}

fn mentions(path: &Path) -> [String; 2] {
    let real = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    [format!("{}/", path.display()), format!("{}/", real.display())]
}

/// Running processes using a folder: any process whose command line mentions the folder, or its known
/// app, but only while that app is open (python shipped inside Xcode.app is not Xcode).
pub fn users_of<'a>(cat: &Catalog, path: &Path, table: &'a [Proc], apps: &HashSet<String>) -> Vec<&'a Proc> {
    let (_, pattern) = cat.owner(&cat.rel_of(path));
    let app_rx = pattern.and_then(|p| Regex::new(&p).ok());
    let m = mentions(path);
    table
        .iter()
        .filter(|p| {
            if m.iter().any(|s| p.cmd.contains(s.as_str())) {
                return true;
            }
            match &app_rx {
                Some(rx) if rx.is_match(&p.cmd) => bundles(&p.cmd).first().is_none_or(|b| apps.contains(*b)),
                _ => false,
            }
        })
        .collect()
}

/// Readable names for the processes using a folder ("Google Chrome", "hermes", "Python").
pub fn user_names(cat: &Catalog, path: &Path, users: &[&Proc], apps: &HashSet<String>) -> Vec<String> {
    let m = mentions(path);
    let mut names: Vec<String> = users
        .iter()
        .map(|p| {
            let (name, is_app) = app_or_program(&p.cmd, apps);
            if !is_app && m.iter().any(|s| p.cmd.contains(s.as_str())) {
                // A program running from or launching the folder: name it after the folder.
                return cat.owner(&cat.rel_of(path)).0;
            }
            name
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Names of whatever uses the folder right now (empty when nothing does).
pub fn in_use(cat: &Catalog, path: &Path) -> Vec<String> {
    let (table, apps) = (process_table(), open_apps());
    user_names(cat, path, &users_of(cat, path, &table, &apps), &apps)
}

fn applescript_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Quit the apps using a folder: apps are asked to quit normally (so they can offer to save);
/// other programs get SIGTERM. Fails naming whatever is still running after 15 s.
pub fn quit_users(cat: &Catalog, path: &Path) -> Result<()> {
    let (table, apps) = (process_table(), open_apps());
    let mut asked = HashSet::new();
    for p in users_of(cat, path, &table, &apps) {
        let (name, is_app) = app_or_program(&p.cmd, &apps);
        if is_app {
            if asked.insert(name.clone()) {
                run("osascript", &["-e", &format!("quit app {}", applescript_string(&name))]);
            }
        } else {
            // SAFETY: plain syscall; a pid that has exited already just returns ESRCH.
            unsafe { libc::kill(p.pid, libc::SIGTERM) };
        }
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let left = in_use(cat, path);
        if left.is_empty() {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(Fail(format!(
                "Still running: {}. It may have unsaved work open, or restart on its own; close it yourself.",
                left.join(", ")
            )));
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/Users/x";
    const XCODE_PYTHON: &str = "/Applications/Xcode.app/Contents/Developer/Library/Frameworks/Python3.framework/Resources/Python.app/Contents/MacOS/Python -c x";

    fn set(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn table() -> Vec<Proc> {
        parse_ps(
            &format!(
                "    1 {HOME}/.hermes/tools/python3 -I -c run gateway\n    2 /Applications/Google Chrome.app/Contents/MacOS/Google Chrome\n    3 /usr/bin/python3 /x/Stowaway.app/y --serve {HOME}/.hermes/\n   77 stowaway-cli who-uses {HOME}/.hermes/\nnot a line\n"
            ),
            77,
        )
    }

    // Ported from the prototype's self-test.
    #[test]
    fn app_names() {
        assert_eq!(
            app_of(
                "/Applications/Google Chrome.app/Contents/Frameworks/X.framework/Helpers/Helper.app/Contents/MacOS/H"
            ),
            "Google Chrome"
        );
        assert_eq!(app_of("/Users/x/.local/bin/claude"), "Claude Code (CLI)");
        assert_eq!(app_or_program(XCODE_PYTHON, &set(&["Finder"])), ("Python".into(), false));
        assert_eq!(app_or_program(XCODE_PYTHON, &set(&["Xcode"])), ("Xcode".into(), true));
        assert_eq!(
            app_or_program("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", &set(&["Google Chrome"])),
            ("Google Chrome".into(), true)
        );
        assert_eq!(
            app_or_program("/Users/x/.local/bin/claude --resume", &set(&[])),
            ("Claude Code (CLI)".into(), false)
        );
    }

    #[test]
    fn who_uses_a_folder() {
        let cat = Catalog::new(HOME);
        let t = table();
        assert_eq!(t.len(), 2, "own pid, Stowaway.app and junk dropped");
        let hermes = Path::new("/Users/x/.hermes");
        let users = users_of(&cat, hermes, &t, &set(&[]));
        assert_eq!(users.iter().map(|p| p.pid).collect::<Vec<_>>(), [1], "Stowaway itself never counts");
        assert_eq!(user_names(&cat, hermes, &users, &set(&[])), ["hermes"]);
        let chrome_cache = Path::new("/Users/x/Library/Caches/Google");
        let chrome = set(&["Google Chrome"]);
        assert_eq!(
            user_names(&cat, chrome_cache, &users_of(&cat, chrome_cache, &t, &chrome), &chrome),
            ["Google Chrome"]
        );
        assert!(users_of(&cat, chrome_cache, &t, &set(&[])).is_empty(), "Chrome closed: not in use");
        assert!(users_of(&cat, Path::new("/Users/x/.npm"), &t, &set(&[])).is_empty());
        let xc = [Proc { pid: 9, cmd: XCODE_PYTHON.into() }];
        let dd = Path::new("/Users/x/Library/Developer/Xcode/DerivedData");
        assert!(users_of(&cat, dd, &xc, &set(&["Finder"])).is_empty(), "a tool inside Xcode.app is not Xcode");
        assert_eq!(users_of(&cat, dd, &xc, &set(&["Xcode"])).len(), 1);
    }

    #[test]
    fn lsappinfo_counts_gui_apps_only() {
        let text = r#" 1) "loginwindow" ASN:0x0-0x2002:
    pid = 436 type="UIElement" flavor=3
 2) "universalaccessd" ASN:0x0-0x9009:
    pid = 700 !signalled type="BackgroundOnly" flavor=3
 3) "Google Chrome" ASN:0x0-0x1:
    bundleID="com.google.Chrome"
    pid = 900 type="Foreground" flavor=3
"#;
        assert_eq!(parse_lsappinfo(text), set(&["loginwindow", "Google Chrome"]));
    }

    #[test]
    fn applescript_quoting() {
        assert_eq!(applescript_string(r#"a "b" \c"#), r#""a \"b\" \\c""#);
    }

    #[test]
    fn live_tables_parse() {
        assert!(process_table().len() > 10);
        assert!(!open_apps().is_empty());
    }
}
