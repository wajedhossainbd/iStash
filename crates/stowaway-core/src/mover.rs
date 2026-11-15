//! Move a folder to a drive and leave a link behind; bring it back; recover after a crash.
//!
//! Move: copy (cloned on the same volume) → verify count, bytes and BLAKE3 hash → check "in use" again →
//! rename the original aside → link → delete the original. Bring back is the mirror image. Every step is
//! journaled before the next one starts, so [`Mover::recover`] can finish or undo it.

use crate::fsx::{self, lexists, sibling};
use crate::journal::{Journal, Op, Step};
use crate::manifest::{self, Manifest};
use crate::rules::{self, Catalog, Verdict};
use crate::scan::{disk_usage, drive_of};
use crate::{fail, Fail, Result};
use std::path::{Component, Path, PathBuf};

/// Moved folders land in `/Volumes/<drive>/Stowaway/<path under home>`.
pub const OFFLOAD_DIR: &str = "Stowaway";
/// Folders the prototype moved, which Stowaway may bring back too.
const LEGACY_OFFLOAD_DIR: &str = "Mac Offload";
const ASIDE: &str = ".stowaway-moving";
const RESTORING: &str = ".stowaway-restoring";
const GB: u64 = 1 << 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub op: Op,
    /// The folder under home (for a bring-back: the link).
    pub path: PathBuf,
    /// The folder on the drive.
    pub dest: PathBuf,
    pub size: u64,
    /// Source and destination share a volume, so files are cloned.
    pub same_volume: bool,
    /// After the copy: sockets and FIFOs left out (relative paths), which programs make again.
    pub skipped: Vec<PathBuf>,
}

pub struct Mover<'a> {
    pub cat: &'a Catalog,
    pub journal: Journal,
    /// Names of whatever uses a folder right now (`procs::in_use` in the app; a stub in tests).
    pub in_use: &'a dyn Fn(&Path) -> Vec<String>,
}

impl<'a> Mover<'a> {
    pub fn new(cat: &'a Catalog, in_use: &'a dyn Fn(&Path) -> Vec<String>) -> Self {
        Mover { cat, journal: Journal::for_home(&cat.home), in_use }
    }

    fn rel_under_home(&self, path: &Path) -> Result<String> {
        let plain = path.components().all(|c| matches!(c, Component::RootDir | Component::Normal(_)));
        match path.strip_prefix(&self.cat.home) {
            Ok(r) if !r.as_os_str().is_empty() && path.is_absolute() && plain => Ok(r.to_string_lossy().into_owned()),
            _ => fail(format!("Only folders inside your home folder can be moved, not {}.", path.display())),
        }
    }

    fn no_open_op(&self, path: &Path) -> Result<()> {
        if self.journal.open_ops()?.iter().any(|e| e.path == path) {
            return fail("An earlier move of this folder was interrupted. Run recovery first.");
        }
        Ok(())
    }

    fn not_in_use(&self, path: &Path) -> Result<()> {
        let names = (self.in_use)(path);
        if !names.is_empty() {
            return fail(format!("In use by {}. Quit it first, so nothing changes during the copy.", names.join(", ")));
        }
        Ok(())
    }

    /// Check everything a move needs, change nothing (the dry run).
    pub fn plan_move(&self, path: &Path, drive: &Path) -> Result<Plan> {
        let rel = self.rel_under_home(path)?;
        let show = self.cat.show(path);
        if rules::never_move(&rel) || rules::never_read(&rel) {
            return fail(format!("Stowaway never moves {show}: {}", self.cat.classify(&rel).1));
        }
        let meta = path.symlink_metadata().map_err(|_| Fail(format!("{show} does not exist.")))?;
        if meta.file_type().is_symlink() {
            return fail(format!("{show} is a link already (moved before?)."));
        }
        if !meta.is_dir() {
            return fail(format!("{show} is not a folder."));
        }
        let (verdict, note) = self.cat.classify(&rel);
        if verdict != Verdict::Move {
            return fail(format!("{show} cannot be moved by Stowaway: {note}"));
        }
        self.no_open_op(path)?;
        if lexists(&sibling(path, ASIDE)) {
            return fail(format!("{} is in the way. Run recovery first.", sibling(path, ASIDE).display()));
        }
        self.not_in_use(path)?;

        let vol = fsx::volume(drive)?;
        if !drive.starts_with("/Volumes") || vol.mount != drive || drive.canonicalize()? == Path::new("/") {
            return fail(format!("{} is not a connected drive.", drive.display()));
        }
        if !vol.can_hold() {
            return fail(format!(
                "That drive cannot hold app data: it must be a writable APFS or Mac OS Extended drive (it is {}{}).",
                vol.fstype,
                if vol.readonly { ", read-only" } else { "" }
            ));
        }
        let size = disk_usage(path);
        let same_volume = fsx::volume(path)?.dev == vol.dev;
        if !same_volume && vol.free < size + size / 10 + GB {
            return fail("Not enough space on that drive.");
        }
        let dest = drive.join(OFFLOAD_DIR).join(&rel);
        if lexists(&dest) {
            return fail(format!("{} already exists on that drive.", dest.display()));
        }
        Ok(Plan { op: Op::Move, path: path.into(), dest, size, same_volume, skipped: Vec::new() })
    }

    /// Move a folder to a drive and leave a link in its place.
    pub fn move_folder(&self, path: &Path, drive: &Path) -> Result<Plan> {
        let mut plan = self.plan_move(path, drive)?;
        let (path, dest) = (plan.path.clone(), plan.dest.clone());
        let (path, dest) = (path.as_path(), dest.as_path());
        let id = Journal::new_id();
        let j = |step| self.journal.append(&id, Op::Move, step, path, dest);
        j(Step::Begin)?;
        crate::step("move:begin");
        let undo_copy = |e: Fail| {
            // Close the operation only once the drive copy is really gone (a detached drive keeps it).
            if drive_connected(dest) && delete_drive_copy(dest).is_ok() {
                let _ = std::fs::remove_file(manifest::path_for(&self.journal, &id));
                let _ = j(Step::Undone);
                return Err(e);
            }
            Err(Fail(format!("{e} The unfinished copy on the drive is removed by recovery once the drive is back.")))
        };
        match std::fs::create_dir_all(dest.parent().unwrap()).and_then(|()| fsx::copy_tree(path, dest)) {
            Ok(skipped) => plan.skipped = skipped,
            Err(e) => return undo_copy(copy_error(e)),
        }
        crate::step("move:copied-unverified");
        let items = match fsx::verify(path, dest) {
            Ok(t) => t.items,
            Err(e) => return undo_copy(e),
        };
        // Before "copied": a journaled move always has its manifest.
        if let Err(e) = manifest::write(&self.journal, &id, &Manifest { dest: dest.into(), items }) {
            return undo_copy(e);
        }
        j(Step::Copied)?;
        crate::step("move:copied");
        // Something may have opened the folder during the copy: check right before switching.
        if let Err(e) = self.not_in_use(path) {
            return undo_copy(e);
        }
        let aside = sibling(path, ASIDE);
        let dir = path.parent().unwrap();
        if let Err(e) = std::fs::rename(path, &aside) {
            return undo_copy(e.into());
        }
        fsx::sync_dir(dir);
        crate::step("move:aside-unjournaled");
        j(Step::Aside)?;
        crate::step("move:aside");
        if let Err(e) = std::os::unix::fs::symlink(dest, path) {
            let _ = std::fs::rename(&aside, path);
            return undo_copy(e.into());
        }
        fsx::sync_dir(dir);
        crate::step("move:linked-unjournaled");
        j(Step::Linked)?;
        crate::step("move:linked");
        if let Err(e) = fsx::remove_tree(&aside) {
            return fail(format!(
                "Moved and linked, but the original could not be fully deleted yet ({e}). Recovery will retry."
            ));
        }
        crate::step("move:deleted-unjournaled");
        j(Step::Done)?;
        Ok(plan)
    }

    /// Check everything a bring-back needs, change nothing.
    pub fn plan_bring_back(&self, path: &Path) -> Result<Plan> {
        self.rel_under_home(path)?;
        let show = self.cat.show(path);
        let target = std::fs::read_link(path).map_err(|_| Fail(format!("{show} is not a moved folder.")))?;
        let drive = drive_of(&target).ok_or_else(|| Fail(format!("{show} does not point to a drive.")))?;
        let root = Path::new("/Volumes").join(&drive);
        if !target.starts_with(root.join(OFFLOAD_DIR)) && !target.starts_with(root.join(LEGACY_OFFLOAD_DIR)) {
            return fail(format!("{show} was linked by hand, not by Stowaway; bring it back by hand."));
        }
        if !target.is_dir() {
            return fail(format!("Connect {drive} first."));
        }
        self.no_open_op(path)?;
        if lexists(&sibling(path, RESTORING)) {
            return fail(format!("{} is in the way. Run recovery first.", sibling(path, RESTORING).display()));
        }
        self.not_in_use(path)?;
        let size = disk_usage(&target);
        let here = fsx::volume(path.parent().unwrap())?;
        let same_volume = here.dev == fsx::volume(&target)?.dev;
        if !same_volume && here.free < size + 10 * GB {
            return fail("Not enough space on this Mac (Stowaway keeps 10 GB spare).");
        }
        Ok(Plan { op: Op::BringBack, path: path.into(), dest: target, size, same_volume, skipped: Vec::new() })
    }

    /// Copy a moved folder back to where its link is, then delete the drive copy.
    pub fn bring_back(&self, path: &Path) -> Result<Plan> {
        let mut plan = self.plan_bring_back(path)?;
        let (path, target) = (plan.path.clone(), plan.dest.clone());
        let (path, target) = (path.as_path(), target.as_path());
        let id = Journal::new_id();
        let j = |step| self.journal.append(&id, Op::BringBack, step, path, target);
        let tmp = sibling(path, RESTORING);
        j(Step::Begin)?;
        crate::step("back:begin");
        let undo_copy = |e: Fail| {
            let _ = fsx::remove_tree(&tmp);
            let _ = j(Step::Undone);
            Err(e)
        };
        match fsx::copy_tree(target, &tmp) {
            Ok(skipped) => plan.skipped = skipped,
            Err(e) => return undo_copy(copy_error(e)),
        }
        crate::step("back:copied-unverified");
        if let Err(e) = fsx::verify(target, &tmp) {
            return undo_copy(e);
        }
        j(Step::Copied)?;
        crate::step("back:copied");
        if let Err(e) = self.not_in_use(path) {
            return undo_copy(e);
        }
        let dir = path.parent().unwrap();
        if let Err(e) = std::fs::remove_file(path) {
            return undo_copy(e.into());
        }
        fsx::sync_dir(dir);
        crate::step("back:unlinked-unjournaled");
        j(Step::Unlinked)?;
        crate::step("back:unlinked");
        if let Err(e) = std::fs::rename(&tmp, path) {
            let _ = std::os::unix::fs::symlink(target, path);
            return undo_copy(e.into());
        }
        fsx::sync_dir(dir);
        crate::step("back:placed-unjournaled");
        j(Step::Placed)?;
        crate::step("back:placed");
        delete_drive_copy(target)
            .map_err(|e| Fail(format!("Brought back, but the drive copy could not be deleted yet ({e}).")))?;
        crate::step("back:deleted-unjournaled");
        j(Step::Done)?;
        self.drop_manifest(path);
        Ok(plan)
    }

    /// The move's manifest describes a drive copy that is gone once a bring-back finishes.
    fn drop_manifest(&self, path: &Path) {
        let moves = self.journal.latest().unwrap_or_default();
        for e in moves.iter().filter(|e| e.op == Op::Move && e.path == path) {
            let _ = std::fs::remove_file(manifest::path_for(&self.journal, &e.id));
        }
    }

    /// Finish or undo every operation a crash interrupted. Returns what was done, one line each.
    /// Anything it cannot decide safely is left in the journal and reported as needing attention.
    pub fn recover(&self) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for e in self.journal.open_ops()? {
            let show = self.cat.show(&e.path);
            let j = |step| self.journal.append(&e.id, e.op, step, &e.path, &e.dest);
            // One stuck operation must not stop the others from being recovered.
            let msg = match e.op {
                Op::Move => recover_move(&e.path, &e.dest, e.step, &j),
                Op::BringBack => recover_back(&e.path, &e.dest, e.step, &j),
            }
            .unwrap_or_else(|f| format!("needs attention: {f}"));
            out.push(format!("{show}: {msg}"));
        }
        Ok(out)
    }
}

fn copy_error(e: std::io::Error) -> Fail {
    // EPERM is the privacy control; EACCES (also PermissionDenied) is an unreadable file, named in `e`.
    if e.raw_os_error() == Some(libc::EPERM) {
        return Fail(
            "macOS blocked access to the drive, so nothing was changed. Allow Stowaway in System Settings → \
             Privacy & Security → Files and Folders (Removable Volumes), then try again."
                .into(),
        );
    }
    Fail(format!("Copy failed, so nothing was changed: {e}"))
}

/// Is the drive a path on `/Volumes/<drive>/…` lives on connected?
fn drive_connected(p: &Path) -> bool {
    drive_of(p).is_some_and(|d| {
        let root = Path::new("/Volumes").join(d);
        fsx::volume(&root).is_ok_and(|v| v.mount == root)
    })
}

/// Delete the drive copy and the folders above it that are left empty, up to the drive itself.
fn delete_drive_copy(target: &Path) -> std::io::Result<()> {
    if lexists(target) {
        fsx::remove_tree(target)?;
    }
    let mut dir = target.parent();
    while let Some(d) = dir {
        if d.parent() == Some(Path::new("/Volumes")) || std::fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
    Ok(())
}

fn recover_move(path: &Path, dest: &Path, step: Step, j: &dyn Fn(Step) -> Result<()>) -> Result<String> {
    let aside = sibling(path, ASIDE);
    let linked = std::fs::read_link(path).is_ok_and(|t| t == dest);
    if linked {
        if step < Step::Copied {
            return Ok(format!("needs attention: linked to {} before the copy was checked.", dest.display()));
        }
        if lexists(&aside) {
            fsx::remove_tree(&aside)?;
        }
        j(Step::Done)?;
        return Ok("finished moving.".into());
    }
    match (path.is_dir() && !path.is_symlink(), lexists(&aside)) {
        (false, true) if !lexists(path) => {
            std::fs::rename(&aside, path)?;
            fsx::sync_dir(path.parent().unwrap());
        }
        (true, false) => {}
        _ => {
            return Ok(format!(
                "needs attention: both {} and {} exist; nothing was changed.",
                path.display(),
                aside.display()
            ))
        }
    }
    // The original is in place again. Drop the drive copy: it may be unchecked or out of date.
    if !drive_connected(dest) {
        return Ok("put back; the unfinished copy is removed when its drive is connected.".into());
    }
    delete_drive_copy(dest)?;
    j(Step::Undone)?;
    Ok("put back (move undone).".into())
}

fn recover_back(path: &Path, target: &Path, step: Step, j: &dyn Fn(Step) -> Result<()>) -> Result<String> {
    let tmp = sibling(path, RESTORING);
    if std::fs::read_link(path).is_ok_and(|t| t == target) {
        // The link is still there and the drive still has everything; drop the unfinished copy.
        if lexists(&tmp) {
            fsx::remove_tree(&tmp)?;
        }
        j(Step::Undone)?;
        return Ok("still on the drive (bring-back undone).".into());
    }
    if step < Step::Copied {
        return Ok("needs attention: the link is gone but the copy back was not checked.".into());
    }
    let is_dir = path.is_dir() && !path.is_symlink();
    match (is_dir, lexists(&tmp)) {
        (false, true) if !lexists(path) => {
            std::fs::rename(&tmp, path)?;
            fsx::sync_dir(path.parent().unwrap());
        }
        (true, false) => {}
        _ => return Ok("needs attention: the folder and its copy are both missing or both present.".into()),
    }
    if drive_connected(target) {
        delete_drive_copy(target)?;
        j(Step::Done)?;
        return Ok("finished bringing back.".into());
    }
    Ok("brought back; the drive copy is deleted when its drive is connected.".into())
}
