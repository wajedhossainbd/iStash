//! File-system primitives for the mover: volume facts, clone-aware copy, content verification.

use crate::{Fail, Result};
use libc::{c_char, c_int, c_void};
use std::collections::hash_map::{Entry, HashMap};
use std::ffi::{CStr, CString, OsStr};
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    pub mount: PathBuf,
    /// "apfs", "hfs", "exfat", "msdos", "smbfs", …
    pub fstype: String,
    pub readonly: bool,
    pub free: u64,
    pub total: u64,
    /// Device id: equal for paths on the same volume.
    pub dev: u64,
}

impl Volume {
    /// Can hold app data behind a link: APFS or Mac OS Extended, writable.
    pub fn can_hold(&self) -> bool {
        matches!(self.fstype.as_str(), "apfs" | "hfs") && !self.readonly
    }
}

fn cpath(p: &Path) -> Result<CString> {
    CString::new(p.as_os_str().as_bytes()).map_err(|_| Fail(format!("Bad path: {}", p.display())))
}

/// The volume a path lives on.
pub fn volume(path: &Path) -> Result<Volume> {
    let c = cpath(path)?;
    // SAFETY: statfs fills the zeroed struct; c is a valid NUL-terminated path.
    let mut st: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(c.as_ptr(), &mut st) } != 0 {
        return Err(Fail(format!("{}: {}", path.display(), io::Error::last_os_error())));
    }
    let s = |b: &[libc::c_char]| unsafe { CStr::from_ptr(b.as_ptr()) }.to_string_lossy().into_owned();
    Ok(Volume {
        mount: PathBuf::from(s(&st.f_mntonname)),
        fstype: s(&st.f_fstypename),
        readonly: st.f_flags & libc::MNT_RDONLY as u32 != 0,
        free: st.f_bavail * st.f_bsize as u64,
        total: st.f_blocks * st.f_bsize as u64,
        dev: std::fs::metadata(path)?.dev(),
    })
}

/// Copy a folder with everything macOS keeps on files (permissions, ACLs, extended attributes,
/// links as links, hard links as hard links, sparse files sparse). On the same APFS volume files are
/// cloned, so the copy takes no extra space. `dst` must not exist.
///
/// Returns what was left out, relative to `src`: sockets and FIFOs, which cannot be copied (the
/// program that uses one makes it again). Errors name the file they happened on.
pub fn copy_tree(src: &Path, dst: &Path) -> io::Result<Vec<PathBuf>> {
    // copyfile would copy into an existing folder (as dst/<name>) rather than fail, so check first.
    if lexists(dst) {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{} exists", dst.display())));
    }
    let (s, d) = (cpath(src).map_err(io::Error::other)?, cpath(dst).map_err(io::Error::other)?);
    let flags = libc::COPYFILE_METADATA
        | libc::COPYFILE_DATA
        | libc::COPYFILE_DATA_SPARSE
        | libc::COPYFILE_RECURSIVE
        | libc::COPYFILE_CLONE
        | libc::COPYFILE_NOFOLLOW_SRC
        | libc::COPYFILE_EXCL;
    let mut run = CopyRun { root: src.into(), skipped: Vec::new(), links: HashMap::new(), err: None };
    let cb: extern "C" fn(_, _, _, _, _, _) -> _ = on_entry;
    // SAFETY: the state is freed below; copyfile calls on_entry only during this call, while `run` lives.
    let (r, os) = unsafe {
        let st = libc::copyfile_state_alloc();
        libc::copyfile_state_set(st, libc::COPYFILE_STATE_STATUS_CB as u32, cb as *const c_void);
        libc::copyfile_state_set(st, libc::COPYFILE_STATE_STATUS_CTX as u32, &mut run as *mut CopyRun as *const c_void);
        let r = libc::copyfile(s.as_ptr(), d.as_ptr(), st, flags);
        let os = io::Error::last_os_error();
        libc::copyfile_state_free(st);
        (r, os)
    };
    match (r, run.err) {
        (0, _) => Ok(run.skipped),
        (_, Some(e)) => Err(e),
        (_, None) => Err(os),
    }
}

struct CopyRun {
    root: PathBuf,
    skipped: Vec<PathBuf>,
    /// (device, inode) of a hard-linked file → its first copy.
    links: HashMap<(u64, u64), PathBuf>,
    err: Option<io::Error>,
}

extern "C" fn on_entry(
    what: c_int,
    stage: c_int,
    _: libc::copyfile_state_t,
    src: *const c_char,
    dst: *const c_char,
    ctx: *mut c_void,
) -> c_int {
    // SAFETY: ctx is the CopyRun copy_tree passed in; src and dst are NUL-terminated paths or null.
    let run = unsafe { &mut *(ctx as *mut CopyRun) };
    let path = |p: *const c_char| {
        (!p.is_null()).then(|| PathBuf::from(OsStr::from_bytes(unsafe { CStr::from_ptr(p) }.to_bytes())))
    };
    let (src, dst) = (path(src), path(dst));
    if stage == libc::COPYFILE_ERR {
        let e = io::Error::last_os_error();
        // EPERM is macOS privacy control refusing the drive; keep it raw so callers can tell.
        run.err = Some(match src {
            Some(p) if e.raw_os_error() != Some(libc::EPERM) => {
                io::Error::new(e.kind(), format!("{}: {e}", p.display()))
            }
            _ => e,
        });
        return libc::COPYFILE_QUIT;
    }
    if what != libc::COPYFILE_RECURSE_FILE || stage != libc::COPYFILE_START {
        return libc::COPYFILE_CONTINUE;
    }
    crate::step("copy:file");
    let (Some(src), Some(dst)) = (src, dst) else { return libc::COPYFILE_CONTINUE };
    let Ok(m) = src.symlink_metadata() else { return libc::COPYFILE_CONTINUE };
    let ft = m.file_type();
    if ft.is_fifo() || ft.is_socket() {
        run.skipped.push(src.strip_prefix(&run.root).unwrap_or(&src).into());
        return libc::COPYFILE_SKIP;
    }
    if ft.is_file() && m.nlink() > 1 {
        match run.links.entry((m.dev(), m.ino())) {
            Entry::Occupied(first) => {
                if let Err(e) = std::fs::hard_link(first.get(), &dst) {
                    run.err = Some(io::Error::new(e.kind(), format!("{}: {e}", src.display())));
                    return libc::COPYFILE_QUIT;
                }
                return libc::COPYFILE_SKIP;
            }
            Entry::Vacant(v) => {
                v.insert(dst);
            }
        }
    }
    libc::COPYFILE_CONTINUE
}

/// File count, total bytes and a BLAKE3 hash over names, kinds, link targets and contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    pub files: u64,
    pub bytes: u64,
    pub hash: [u8; 32],
    /// Every file and link, in walk order: what a manifest records.
    pub items: Vec<crate::manifest::Item>,
}

/// Summarize a folder for verification. Files are read past the page cache, so a copy on a drive
/// is checked as the drive stores it. Any unreadable entry fails the whole check.
pub fn tree(root: &Path) -> io::Result<Tree> {
    let mut t = Tree { files: 0, bytes: 0, hash: [0; 32], items: Vec::new() };
    let mut h = blake3::Hasher::new();
    walk(root, Path::new(""), &mut t, &mut h)?;
    t.hash = *h.finalize().as_bytes();
    Ok(t)
}

fn walk(root: &Path, rel: &Path, t: &mut Tree, h: &mut blake3::Hasher) -> io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(root.join(rel))?.collect::<io::Result<_>>()?;
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let rel = rel.join(e.file_name());
        let m = e.metadata()?; // does not follow links
        let ft = m.file_type();
        if ft.is_fifo() || ft.is_socket() {
            continue; // never copied (see copy_tree)
        }
        h.update(rel.as_os_str().as_bytes());
        h.update(&[0]);
        if ft.is_dir() {
            h.update(b"d");
            walk(root, &rel, t, h)?;
            continue;
        }
        t.files += 1;
        t.bytes += m.size();
        let content = if ft.is_file() {
            h.update(b"f");
            h.update(&m.size().to_le_bytes());
            let c = hash_file(&root.join(&rel))?;
            h.update(c.as_bytes());
            c
        } else if ft.is_symlink() {
            h.update(b"l");
            let target = std::fs::read_link(root.join(&rel))?;
            h.update(target.as_os_str().as_bytes());
            blake3::hash(target.as_os_str().as_bytes())
        } else {
            h.update(b"?");
            continue;
        };
        t.items.push(crate::manifest::Item {
            path: rel.to_string_lossy().into_owned(),
            size: m.size(),
            mtime: m.mtime(),
            hash: content.to_hex().to_string(),
        });
    }
    Ok(())
}

fn hash_file(p: &Path) -> io::Result<blake3::Hash> {
    let f = File::open(p)?;
    // SAFETY: plain fcntl on an open fd; failure only means the page cache may be used.
    unsafe { libc::fcntl(f.as_raw_fd(), libc::F_NOCACHE, 1) };
    let mut h = blake3::Hasher::new();
    h.update_reader(&f)?;
    Ok(h.finalize())
}

/// Check that `copy` holds exactly what `orig` holds; say what differs if not. Returns the copy's tree.
pub fn verify(orig: &Path, copy: &Path) -> Result<Tree> {
    let read = |p: &Path| tree(p).map_err(|e| Fail(format!("Could not read {} to check it: {e}", p.display())));
    let (a, b) = (read(orig)?, read(copy)?);
    if (a.files, a.bytes) != (b.files, b.bytes) {
        return Err(Fail(format!(
            "The copy did not match: {} files / {} bytes, the copy has {} / {}.",
            a.files, a.bytes, b.files, b.bytes
        )));
    }
    if a.hash != b.hash {
        return Err(Fail("The copy did not match: file contents differ.".into()));
    }
    Ok(b)
}

/// Delete a folder Stowaway holds a verified copy of. Files a deny-delete ACL or the user lock
/// (`uchg`) protects are unprotected and deleted too; the other copy keeps that protection.
pub fn remove_tree(p: &Path) -> io::Result<()> {
    match std::fs::remove_dir_all(p) {
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            unprotect(p)?;
            std::fs::remove_dir_all(p)
        }
        r => r,
    }
}

extern "C" {
    fn acl_init(count: c_int) -> *mut c_void;
    fn acl_set_link_np(path: *const c_char, kind: c_int, acl: *mut c_void) -> c_int;
    fn acl_free(obj: *mut c_void) -> c_int;
}
const ACL_TYPE_EXTENDED: c_int = 0x100;

/// Drop ACLs and the user lock from everything under `p`, never following links: `chmod -R -N`
/// follows links inside the tree and would strip the protection of files outside it.
fn unprotect(p: &Path) -> io::Result<()> {
    use std::os::macos::fs::MetadataExt as _;
    let m = p.symlink_metadata()?;
    if m.file_type().is_symlink() {
        return Ok(());
    }
    let c = cpath(p).map_err(io::Error::other)?;
    // SAFETY: valid C string; the empty ACL is freed after use.
    unsafe {
        if m.st_flags() & libc::UF_IMMUTABLE != 0 {
            libc::lchflags(c.as_ptr(), m.st_flags() & !libc::UF_IMMUTABLE);
        }
        let empty = acl_init(0);
        acl_set_link_np(c.as_ptr(), ACL_TYPE_EXTENDED, empty);
        acl_free(empty);
    }
    if m.is_dir() {
        for e in std::fs::read_dir(p)? {
            unprotect(&e?.path())?;
        }
    }
    Ok(())
}

/// Make a rename or link in `dir` durable before the journal says it happened.
pub fn sync_dir(dir: &Path) {
    if let Ok(f) = File::open(dir) {
        // SAFETY: fsync on an open fd.
        unsafe { libc::fsync(f.as_raw_fd()) };
    }
}

/// Whether something (even a dangling link) exists at `p`.
pub fn lexists(p: &Path) -> bool {
    p.symlink_metadata().is_ok()
}

/// `name.suffix` next to `path`.
pub fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("stowaway-fsx-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn copy_and_verify() {
        let d = tmp("copy");
        let src = d.join("src");
        fs::create_dir_all(src.join("sub/deeper")).unwrap();
        fs::write(src.join("a"), "hello").unwrap();
        fs::write(src.join("sub/deeper/b"), vec![7u8; 100_000]).unwrap();
        std::os::unix::fs::symlink("../a", src.join("sub/link")).unwrap();
        std::os::unix::fs::symlink("/nonexistent", src.join("dangling")).unwrap();
        assert!(copy_tree(&src, &d.join("dst")).unwrap().is_empty());
        verify(&src, &d.join("dst")).unwrap();
        assert!(d.join("dst/sub/link").is_symlink(), "links copied as links");
        let t = tree(&src).unwrap();
        assert_eq!((t.files, t.bytes), (4, 5 + 100_000 + 4 + 12));
        assert!(copy_tree(&src, &d.join("dst")).is_err(), "never overwrites");

        fs::write(d.join("dst/sub/deeper/b"), vec![8u8; 100_000]).unwrap(); // same size, other bytes
        assert!(verify(&src, &d.join("dst")).unwrap_err().0.contains("contents"));
        fs::remove_file(d.join("dst/a")).unwrap();
        assert!(verify(&src, &d.join("dst")).unwrap_err().0.contains("files"));
        fs::rename(d.join("dst/sub"), d.join("dst/sub2")).unwrap();
        fs::write(d.join("dst/a"), "hello").unwrap();
        assert!(verify(&src, &d.join("dst")).is_err(), "renames are caught");
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn volume_facts() {
        let v = volume(Path::new("/")).unwrap();
        assert_eq!(v.mount, Path::new("/"));
        assert_eq!(v.fstype, "apfs");
        assert!(v.readonly, "the system volume is sealed");
        let data = volume(&std::env::temp_dir()).unwrap();
        assert!(data.can_hold() && data.free > 0 && data.total >= data.free);
    }

    #[test]
    fn siblings() {
        assert_eq!(sibling(Path::new("/a/.b"), ".x"), Path::new("/a/.b.x"));
    }
}
