//! Which moved folders (and so which apps) need which drive; whether an eject is safe.

use crate::fsx;
use crate::journal::Journal;
use crate::procs::Proc;
use crate::rules::Catalog;
use crate::scan::{self, drive_of};
use regex::Regex;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// A folder that lives on a drive, reached through a link (or an app's own setting, for Docker).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Moved {
    pub path: PathBuf,
    pub show: String,
    pub target: PathBuf,
    pub drive: String,
    pub app: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Drive {
    pub name: String,
    pub mount: PathBuf,
    pub connected: bool,
    pub fstype: Option<String>,
    pub free: Option<u64>,
    pub total: Option<u64>,
    pub can_hold: bool,
    /// Apps that need this drive, sorted.
    pub needed_by: Vec<String>,
    /// The moved folders on it.
    pub folders: Vec<String>,
}

fn moved_from_link(cat: &Catalog, path: &Path) -> Option<Moved> {
    let target = std::fs::read_link(path).ok()?;
    Some(Moved {
        drive: drive_of(&target)?,
        show: cat.show(path),
        app: cat.owner(&cat.rel_of(path)).0,
        path: path.to_path_buf(),
        target,
    })
}

/// Moved folders: Stowaway's journal plus any link found where big folders live (moved by hand or by
/// the prototype). Reads directory listings only, so it is quick.
pub fn moved_items(cat: &Catalog, journal: &Journal) -> Vec<Moved> {
    let mut paths: Vec<PathBuf> = journal.moved().unwrap_or_default().into_iter().map(|e| e.path).collect();
    paths.extend(scan::candidates(cat));
    paths.sort();
    paths.dedup();
    paths.iter().filter_map(|p| moved_from_link(cat, p)).collect()
}

/// Docker's disk image when Docker's own setting put it on a drive. Read from Docker's running disk
/// process (its settings sit in its sandbox container, which Stowaway never reads); remembered while
/// Docker is off in `state_file`.
pub fn docker_moved(table: &[Proc], state_file: &Path) -> Option<Moved> {
    static DISK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"--disk (/\S.*?/Docker\.raw)").unwrap());
    let cmds: Vec<&str> = table.iter().filter(|p| p.cmd.contains("com.docker")).map(|p| p.cmd.as_str()).collect();
    let seen = DISK.captures(&cmds.join(" ")).map(|c| c[1].to_string());
    let disk = match seen {
        Some(d) => {
            if std::fs::read_to_string(state_file).ok().as_deref() != Some(&d) {
                let _ = std::fs::create_dir_all(state_file.parent()?);
                let _ = std::fs::write(state_file, &d);
            }
            d
        }
        None => std::fs::read_to_string(state_file).ok()?,
    };
    let disk = PathBuf::from(disk.trim());
    Some(Moved {
        drive: drive_of(&disk)?,
        path: disk.parent()?.to_path_buf(),
        show: "Docker disk image".into(),
        target: disk,
        app: "Docker".into(),
    })
}

/// Where `docker_moved` remembers Docker's disk.
pub fn docker_state_file(home: &Path) -> PathBuf {
    home.join("Library/Application Support/Stowaway/docker-disk")
}

/// Connected drives, plus drives that moved folders need but that are not connected.
pub fn drives(moved: &[Moved]) -> Vec<Drive> {
    let mut out: BTreeMap<String, Drive> = BTreeMap::new();
    for e in std::fs::read_dir("/Volumes").into_iter().flatten().flatten() {
        let mount = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        let Ok(v) = fsx::volume(&mount) else { continue };
        if v.mount != mount || mount.canonicalize().is_ok_and(|r| r == Path::new("/")) {
            continue; // not mounted there, or the startup disk's /Volumes alias
        }
        out.insert(
            name.clone(),
            Drive {
                name,
                mount,
                connected: true,
                can_hold: v.can_hold(),
                fstype: Some(v.fstype),
                free: Some(v.free),
                total: Some(v.total),
                needed_by: Vec::new(),
                folders: Vec::new(),
            },
        );
    }
    for m in moved {
        let d = out.entry(m.drive.clone()).or_insert_with(|| Drive {
            name: m.drive.clone(),
            mount: Path::new("/Volumes").join(&m.drive),
            connected: false,
            fstype: None,
            free: None,
            total: None,
            can_hold: false,
            needed_by: Vec::new(),
            folders: Vec::new(),
        });
        d.needed_by.push(m.app.clone());
        d.folders.push(m.show.clone());
    }
    for d in out.values_mut() {
        d.needed_by.sort();
        d.needed_by.dedup();
        d.folders.sort();
    }
    out.into_values().collect()
}

/// Why ejecting the drive mounted at `mount` is unsafe right now, naming the apps; None when it is safe.
/// Only apps that are running and use a folder on that drive block it (`running` gives their names).
pub fn eject_objection(mount: &Path, moved: &[Moved], running: &dyn Fn(&Moved) -> Vec<String>) -> Option<String> {
    let name = mount.strip_prefix("/Volumes").ok()?.to_string_lossy().trim_matches('/').to_string();
    let mut apps: Vec<String> = moved.iter().filter(|m| m.drive == name).flat_map(running).collect();
    apps.sort();
    apps.dedup();
    if apps.is_empty() {
        return None;
    }
    let list = apps.join(", ");
    Some(format!(
        "{name} holds data that {list} {} using. Quit {list} first, then eject.",
        if apps.len() == 1 { "is" } else { "are" }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moved(drive: &str, app: &str) -> Moved {
        Moved {
            path: format!("/Users/x/.{app}").into(),
            show: format!("~/.{app}"),
            target: format!("/Volumes/{drive}/Stowaway/.{app}").into(),
            drive: drive.into(),
            app: app.into(),
        }
    }

    #[test]
    fn objection_names_running_apps_on_that_drive_only() {
        let items = [moved("SSD", "Docker"), moved("SSD", "Ollama"), moved("Other", "Xcode")];
        let all_running = |m: &Moved| vec![m.app.clone()];
        let msg = eject_objection(Path::new("/Volumes/SSD"), &items, &all_running).unwrap();
        assert!(msg.contains("Docker, Ollama are using") && !msg.contains("Xcode"), "{msg}");
        let only_ollama = |m: &Moved| if m.app == "Ollama" { vec!["Ollama".into()] } else { vec![] };
        assert!(eject_objection(Path::new("/Volumes/SSD/"), &items, &only_ollama).unwrap().contains("Ollama is using"));
        assert_eq!(eject_objection(Path::new("/Volumes/SSD"), &items, &|_| vec![]), None, "nothing running: safe");
        assert_eq!(eject_objection(Path::new("/Volumes/Empty"), &items, &all_running), None);
    }

    #[test]
    fn missing_drives_are_listed_with_their_apps() {
        let d = drives(&[moved("NoSuchDrive-xyz", "hermes"), moved("NoSuchDrive-xyz", "Ollama")]);
        let missing = d.iter().find(|d| d.name == "NoSuchDrive-xyz").unwrap();
        assert!(!missing.connected);
        assert_eq!(missing.needed_by, ["Ollama", "hermes"]);
        assert!(d.iter().all(|d| d.mount != Path::new("/Volumes/Macintosh HD") || !d.connected));
    }

    #[test]
    fn docker_disk_from_its_process_and_remembered() {
        let state = std::env::temp_dir().join(format!("stowaway-docker-{}", std::process::id()));
        let _ = std::fs::remove_file(&state);
        let table = [Proc {
            pid: 5,
            cmd: "/Applications/Docker.app/Contents/MacOS/com.docker.krun --disk /Volumes/My SSD/Docker/DockerDesktop/Docker.raw --cpus 4".into(),
        }];
        let m = docker_moved(&table, &state).unwrap();
        assert_eq!((m.drive.as_str(), m.app.as_str()), ("My SSD", "Docker"));
        assert_eq!(m.target, Path::new("/Volumes/My SSD/Docker/DockerDesktop/Docker.raw"));
        assert_eq!(docker_moved(&[], &state), Some(m), "remembered while Docker is off");
        std::fs::write(&state, "/Users/x/Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw").unwrap();
        assert_eq!(docker_moved(&[], &state), None, "on this Mac: not on a drive");
        std::fs::remove_file(state).unwrap();
    }
}
