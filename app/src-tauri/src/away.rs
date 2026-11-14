//! Watches for a drive that holds moved folders going away, and for an app being used while its folder is away.
//! macOS gives no way to stop an app from opening, so this reacts within a few seconds instead: it says which
//! drive to connect and offers to quit the app before it runs for long without its data.
//! shortcut: polls every 3 s and covers only moves in iStash's journal; folders linked by hand are not watched.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use stowaway_core::procs;
use stowaway_core::rules::Catalog;

#[derive(Default)]
pub struct Seen {
    started: bool,
    missing: BTreeSet<String>,          // drives that were away on the last look
    warned: BTreeSet<(String, String)>, // (drive, app) already asked about
}

#[derive(Debug, PartialEq)]
pub enum Event {
    Gone { drive: String, apps: Vec<String> },
    Back { drive: String },
    Held { drive: String, apps: Vec<String> },
}

/// One look. `moved` is (drive, app, is the app running now); `connected` says whether a drive is plugged in.
pub fn step(seen: &mut Seen, moved: &[(String, String, bool)], connected: &dyn Fn(&str) -> bool) -> Vec<Event> {
    let drives: BTreeSet<&String> = moved.iter().map(|m| &m.0).collect();
    let missing: BTreeSet<String> = drives.into_iter().filter(|d| !connected(d)).cloned().collect();
    let mut out = vec![];
    // Nothing is announced on the first look: a drive that was already away at launch is not news.
    if seen.started {
        for d in missing.difference(&seen.missing) {
            let mut apps: Vec<String> = moved.iter().filter(|m| &m.0 == d).map(|m| m.1.clone()).collect();
            apps.dedup();
            out.push(Event::Gone { drive: d.clone(), apps });
        }
        for d in seen.missing.difference(&missing) {
            out.push(Event::Back { drive: d.clone() });
        }
    }
    seen.warned.retain(|(d, a)| missing.contains(d) && moved.iter().any(|m| &m.0 == d && &m.1 == a && m.2));
    for d in &missing {
        let apps: Vec<String> = moved
            .iter()
            .filter(|m| &m.0 == d && m.2 && seen.warned.insert((d.clone(), m.1.clone())))
            .map(|m| m.1.clone())
            .collect();
        if !apps.is_empty() {
            out.push(Event::Held { drive: d.clone(), apps });
        }
    }
    seen.started = true;
    seen.missing = missing;
    out
}

fn osa(lines: &[&str], args: &[&str]) -> String {
    let mut c = Command::new("/usr/bin/osascript");
    c.args(["-e", "on run argv"]);
    for l in lines {
        c.args(["-e", l]);
    }
    c.args(["-e", "end run"]).args(args);
    c.output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

fn list(apps: &[String]) -> String {
    match apps {
        [a] => a.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        [] => String::new(),
    }
}

pub fn start() {
    std::thread::spawn(|| {
        let cat = Catalog::for_user();
        let mut seen = Seen::default();
        loop {
            let (table, apps) = (procs::process_table(), procs::open_apps());
            let moved = crate::engine::all_moved(&cat, &table);
            let rows: Vec<(String, String, bool)> = moved
                .iter()
                .map(|m| (m.drive.clone(), m.app.clone(), !procs::users_of(&cat, &m.path, &table, &apps).is_empty()))
                .collect();
            for e in step(&mut seen, &rows, &|d| Path::new("/Volumes").join(d).is_dir()) {
                match e {
                    Event::Gone { drive, apps } => {
                        crate::island::notify(
                            &format!("{drive} was disconnected"),
                            &format!(
                                "{} keep{} data on it and may not work until it is back. Nothing is lost.",
                                list(&apps),
                                if apps.len() == 1 { "s" } else { "" }
                            ),
                            "unplug",
                        );
                    }
                    Event::Back { drive } => {
                        crate::island::notify(
                            &format!("{drive} is back"),
                            "The apps that keep data on it work again.",
                            "plug",
                        );
                    }
                    Event::Held { drive, apps } => {
                        let many = apps.len() > 1;
                        let quit = if many { "Quit Them".to_string() } else { format!("Quit {}", apps[0]) };
                        let said = osa(
                            &["activate", "display dialog (item 2 of argv) with title (item 1 of argv) buttons {\"Keep Open\", (item 3 of argv)} default button 2 with icon caution"],
                            &[
                                &format!("Connect {drive} first"),
                                &format!("{} {} open, but {} data is on {drive}, which is not connected.\n\nPlug {drive} in and {} again. Until then {} cannot read or save that data. Nothing is lost.", list(&apps), if many { "are" } else { "is" }, if many { "their" } else { "its" }, if many { "open them" } else { "open it" }, if many { "they" } else { "it" }),
                                &quit,
                            ],
                        );
                        if said.contains(&quit) {
                            for m in moved.iter().filter(|m| m.drive == drive && apps.contains(&m.app)) {
                                let _ = procs::quit_users(&cat, &m.path);
                            }
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_secs(3));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn m(d: &str, a: &str, r: bool) -> (String, String, bool) {
        (d.into(), a.into(), r)
    }

    #[test]
    fn drive_leaves_app_opens_drive_returns() {
        let mut s = Seen::default();
        let here = |_: &str| true;
        let away = |_: &str| false;
        // First look never announces, even if the drive is already away.
        assert_eq!(step(&mut Seen::default(), &[m("T7", "Chrome", false)], &away), vec![]);
        assert_eq!(step(&mut s, &[m("T7", "Chrome", false), m("T7", "venvs", false)], &here), vec![]);
        assert_eq!(
            step(&mut s, &[m("T7", "Chrome", false), m("T7", "venvs", false)], &away),
            vec![Event::Gone { drive: "T7".into(), apps: vec!["Chrome".into(), "venvs".into()] }]
        );
        // Chrome is opened while the drive is away: asked once, not on every look.
        assert_eq!(
            step(&mut s, &[m("T7", "Chrome", true)], &away),
            vec![Event::Held { drive: "T7".into(), apps: vec!["Chrome".into()] }]
        );
        assert_eq!(step(&mut s, &[m("T7", "Chrome", true)], &away), vec![]);
        // Quit and opened again: asked again.
        assert_eq!(step(&mut s, &[m("T7", "Chrome", false)], &away), vec![]);
        assert_eq!(
            step(&mut s, &[m("T7", "Chrome", true)], &away),
            vec![Event::Held { drive: "T7".into(), apps: vec!["Chrome".into()] }]
        );
        assert_eq!(step(&mut s, &[m("T7", "Chrome", true)], &here), vec![Event::Back { drive: "T7".into() }]);
    }
}
