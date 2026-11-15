//! Per-folder manifests: what a moved folder held when its copy was verified (path, size, modified time,
//! BLAKE3), so the drive copy can be re-checked later. One JSON file per move, named after its journal id,
//! in `~/Library/Application Support/Stowaway/manifests/`. Checking only reads.

use crate::journal::Journal;
use crate::{fail, fsx, Fail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One file or link, relative to the moved folder. A link's hash is of its target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub path: String,
    pub size: u64,
    /// Seconds since 1970.
    pub mtime: i64,
    /// BLAKE3, hex.
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// The folder on the drive.
    pub dest: PathBuf,
    pub items: Vec<Item>,
}

/// What a re-check found, relative paths. `changed` files were written since the move (newer modified time),
/// which apps do; `damaged` ones differ but kept their modified time, which apps don't.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Report {
    /// False when the folder has no record yet (moved before iStash kept them); nothing was compared.
    pub recorded: bool,
    pub checked: usize,
    pub missing: Vec<String>,
    pub changed: Vec<String>,
    pub damaged: Vec<String>,
    pub extra: Vec<String>,
}

impl Report {
    pub fn intact(&self) -> bool {
        self.missing.is_empty() && self.changed.is_empty() && self.damaged.is_empty() && self.extra.is_empty()
    }
}

/// Where the manifest of the move journaled as `id` lives.
pub fn path_for(journal: &Journal, id: &str) -> PathBuf {
    journal.path.with_file_name("manifests").join(format!("{id}.json"))
}

/// Write a manifest durably (temporary file, sync, rename).
pub fn write(journal: &Journal, id: &str, m: &Manifest) -> Result<()> {
    use std::io::Write;
    let p = path_for(journal, id);
    let dir = p.parent().unwrap();
    std::fs::create_dir_all(dir)?;
    let tmp = p.with_extension("json.tmp");
    let mut f = std::fs::File::create(&tmp)?;
    f.write_all(&serde_json::to_vec(m).expect("manifest serializes"))?;
    f.sync_all()?;
    std::fs::rename(&tmp, &p)?;
    fsx::sync_dir(dir);
    Ok(())
}

/// Compare a folder with a manifest. Reads every file; changes nothing.
pub fn compare(m: &Manifest, dir: &Path) -> Result<Report> {
    let now = fsx::tree(dir).map_err(|e| Fail(format!("Could not read {} to check it: {e}", dir.display())))?;
    let mut was: HashMap<&str, &Item> = m.items.iter().map(|i| (i.path.as_str(), i)).collect();
    let mut r = Report { recorded: true, checked: m.items.len(), ..Report::default() };
    for i in &now.items {
        match was.remove(i.path.as_str()) {
            None => r.extra.push(i.path.clone()),
            Some(o) if (o.size, &o.hash) == (i.size, &i.hash) => {}
            Some(o) if o.mtime != i.mtime => r.changed.push(i.path.clone()),
            Some(_) => r.damaged.push(i.path.clone()),
        }
    }
    r.missing = was.into_keys().map(String::from).collect();
    r.missing.sort();
    Ok(r)
}

fn moved_entry(journal: &Journal, path: &Path) -> Result<crate::journal::Entry> {
    match journal.moved()?.into_iter().find(|e| e.path == path) {
        Some(e) => Ok(e),
        None => fail(format!("{} was not moved by iStash.", path.display())),
    }
}

/// Re-check a moved folder (the link under home) against the manifest written when it was moved. A folder moved
/// before manifests existed reports `recorded: false`.
pub fn check(journal: &Journal, path: &Path) -> Result<Report> {
    let e = moved_entry(journal, path)?;
    let m: Manifest = match std::fs::read(path_for(journal, &e.id)) {
        Ok(b) => {
            serde_json::from_slice(&b).map_err(|err| Fail(format!("The record of this move is unreadable: {err}")))?
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Report::default()),
        Err(err) => return Err(err.into()),
    };
    if !m.dest.is_dir() {
        return fail(format!("Connect the drive that holds {} first.", m.dest.display()));
    }
    compare(&m, &m.dest)
}

/// Make a record for a folder moved before manifests existed, from its drive copy as it is now, so later checks
/// have something to compare with. Reads the drive copy; writes only the record. Never replaces an existing record.
pub fn record(journal: &Journal, path: &Path) -> Result<Report> {
    let e = moved_entry(journal, path)?;
    let p = path_for(journal, &e.id);
    if p.exists() {
        return fail("This folder already has a record. Check compares with it.");
    }
    if !e.dest.is_dir() {
        return fail(format!("Connect the drive that holds {} first.", e.dest.display()));
    }
    let items = fsx::tree(&e.dest)
        .map_err(|err| Fail(format!("Could not read {} to record it: {err}", e.dest.display())))?
        .items;
    let r = Report { recorded: true, checked: items.len(), ..Report::default() };
    write(journal, &e.id, &Manifest { dest: e.dest, items })?;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn write_and_compare() {
        let d = std::env::temp_dir().join(format!("stowaway-manifest-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let dir = d.join("data");
        fs::create_dir_all(dir.join("sub")).unwrap();
        for f in ["a", "b", "sub/c", "sub/d"] {
            fs::write(dir.join(f), f).unwrap();
        }
        std::os::unix::fs::symlink("a", dir.join("link")).unwrap();
        let j = Journal { path: d.join("state/journal.jsonl") };
        let m = Manifest { dest: dir.clone(), items: fsx::tree(&dir).unwrap().items };
        assert_eq!(m.items.len(), 5);
        write(&j, "1", &m).unwrap();
        let back: Manifest = serde_json::from_slice(&fs::read(path_for(&j, "1")).unwrap()).unwrap();
        assert_eq!(back, m);
        assert!(compare(&m, &dir).unwrap().intact(), "a fresh hash matches");

        fs::remove_file(dir.join("b")).unwrap();
        fs::write(dir.join("new"), "x").unwrap();
        // Same size, other bytes, modified time put back: damage, not an app's write.
        let t = fs::metadata(dir.join("sub/c")).unwrap().modified().unwrap();
        fs::write(dir.join("sub/c"), "X").unwrap();
        fs::File::options().write(true).open(dir.join("sub/c")).unwrap().set_modified(t).unwrap();
        // Written later: a change.
        fs::write(dir.join("sub/d"), "longer").unwrap();
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        fs::File::options().write(true).open(dir.join("sub/d")).unwrap().set_modified(later).unwrap();
        let r = compare(&m, &dir).unwrap();
        assert_eq!(r.checked, 5);
        assert_eq!((r.missing, r.extra), (vec!["b".to_string()], vec!["new".to_string()]));
        assert_eq!((r.damaged, r.changed), (vec!["sub/c".to_string()], vec!["sub/d".to_string()]));
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn older_moves_have_no_record_until_recorded() {
        use crate::journal::{Op, Step};
        let d = std::env::temp_dir().join(format!("stowaway-record-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let (home, dest) = (d.join("home/data"), d.join("drive/data"));
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("a"), "a").unwrap();
        let j = Journal { path: d.join("state/journal.jsonl") };
        j.append("old", Op::Move, Step::Done, &home, &dest).unwrap();

        let r = check(&j, &home).unwrap();
        assert!(!r.recorded && r.checked == 0, "no record yet, nothing compared");
        assert_eq!(record(&j, &home).unwrap().checked, 1);
        assert!(record(&j, &home).is_err(), "an existing record is never replaced");
        let r = check(&j, &home).unwrap();
        assert!(r.recorded && r.intact());
        fs::remove_dir_all(d).unwrap();
    }
}
