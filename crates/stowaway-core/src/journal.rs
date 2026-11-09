//! Append-only record of every move and bring-back, one JSON object per line, synced after each
//! write. The last line of an operation says how far it got; recovery reads it after a crash.

use crate::Result;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Move,
    BringBack,
}

/// Steps in order. Move: begin → copied → aside → linked → done.
/// Bring back: begin → copied → unlinked → placed → done. Either may end in undone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Begin,
    /// The copy is complete and verified.
    Copied,
    /// Move: the original was renamed aside.
    Aside,
    /// Bring back: the link was removed.
    Unlinked,
    /// Move: the link to the drive copy is in place.
    Linked,
    /// Bring back: the local copy took the link's place.
    Placed,
    Done,
    Undone,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub op: Op,
    pub step: Step,
    /// The folder under home (for a bring-back: the link).
    pub path: PathBuf,
    /// The folder on the drive.
    pub dest: PathBuf,
    /// Seconds since 1970.
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct Journal {
    pub path: PathBuf,
}

impl Journal {
    /// `~/Library/Application Support/Stowaway/journal.jsonl`.
    pub fn for_home(home: &Path) -> Self {
        Journal { path: home.join("Library/Application Support/Stowaway/journal.jsonl") }
    }

    pub fn new_id() -> String {
        let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        format!("{}-{}", t.as_nanos(), std::process::id())
    }

    pub fn append(&self, id: &str, op: Op, step: Step, path: &Path, dest: &Path) -> Result<()> {
        let e = Entry {
            id: id.into(),
            op,
            step,
            path: path.into(),
            dest: dest.into(),
            ts: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        };
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut f = OpenOptions::new().create(true).read(true).append(true).open(&self.path)?;
        // A line torn by a crash has no newline; start a fresh line so this entry is not lost with it.
        let mut last = [0u8; 1];
        let torn = f.metadata()?.len() > 0 && f.read_at(&mut last, f.metadata()?.len() - 1)? == 1 && last[0] != b'\n';
        let mut line = if torn { "\n".to_string() } else { String::new() };
        line.push_str(&serde_json::to_string(&e).expect("entry serializes"));
        line.push('\n');
        f.write_all(line.as_bytes())?;
        f.sync_all()?;
        Ok(())
    }

    /// Every entry in order. A line torn by a crash mid-write is skipped.
    pub fn entries(&self) -> Result<Vec<Entry>> {
        match std::fs::read_to_string(&self.path) {
            Ok(s) => Ok(s.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    /// The latest entry of each operation, oldest operation first.
    pub fn latest(&self) -> Result<Vec<Entry>> {
        let mut out: Vec<Entry> = Vec::new();
        for e in self.entries()? {
            match out.iter_mut().find(|x| x.id == e.id) {
                Some(x) => *x = e,
                None => out.push(e),
            }
        }
        Ok(out)
    }

    /// Operations that neither finished nor were undone.
    pub fn open_ops(&self) -> Result<Vec<Entry>> {
        Ok(self.latest()?.into_iter().filter(|e| !matches!(e.step, Step::Done | Step::Undone)).collect())
    }

    /// Folders that are moved now: finished moves not followed by a finished bring-back.
    pub fn moved(&self) -> Result<Vec<Entry>> {
        let mut out: Vec<Entry> = Vec::new();
        for e in self.latest()?.into_iter().filter(|e| e.step == Step::Done) {
            out.retain(|x| x.path != e.path);
            if e.op == Op::Move {
                out.push(e);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_open_and_moved() {
        let dir = std::env::temp_dir().join(format!("stowaway-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let j = Journal { path: dir.join("sub/journal.jsonl") };
        assert!(j.entries().unwrap().is_empty());
        let (a, b) = (Path::new("/h/.a"), Path::new("/h/.b"));
        let d = Path::new("/Volumes/D/Stowaway/.a");
        j.append("1", Op::Move, Step::Begin, a, d).unwrap();
        j.append("1", Op::Move, Step::Done, a, d).unwrap();
        j.append("2", Op::Move, Step::Begin, b, d).unwrap();
        j.append("2", Op::Move, Step::Copied, b, d).unwrap();
        std::fs::OpenOptions::new().append(true).open(&j.path).unwrap().write_all(b"{\"id\":\"3\",\"op").unwrap();
        assert_eq!(j.entries().unwrap().len(), 4, "torn line skipped");
        j.append("2", Op::Move, Step::Copied, b, d).unwrap();
        assert_eq!(j.entries().unwrap().len(), 5, "entry after a torn line survives");
        let open = j.open_ops().unwrap();
        assert_eq!((open.len(), open[0].step), (1, Step::Copied));
        assert_eq!(j.moved().unwrap().iter().map(|e| e.path.as_path()).collect::<Vec<_>>(), [a]);
        j.append("4", Op::BringBack, Step::Done, a, d).unwrap();
        assert!(j.moved().unwrap().is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn steps_are_ordered() {
        assert!(Step::Begin < Step::Copied && Step::Copied < Step::Aside && Step::Aside < Step::Linked);
        assert!(Step::Copied < Step::Unlinked && Step::Unlinked < Step::Placed);
    }
}
