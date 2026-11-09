//! File-system primitives for the mover: volume facts, clone-aware copy, content verification.

use crate::{Fail, Result};
use std::ffi::{CStr, CString};
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
/// links as links). On the same APFS volume files are cloned, so the copy takes no extra space.
/// `dst` must not exist.
pub fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    // copyfile would copy into an existing folder (as dst/<name>) rather than fail, so check first.
    if lexists(dst) {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{} exists", dst.display())));
    }
    let (s, d) = (cpath(src).map_err(io::Error::other)?, cpath(dst).map_err(io::Error::other)?);
    let flags = libc::COPYFILE_METADATA
        | libc::COPYFILE_DATA
        | libc::COPYFILE_RECURSIVE
        | libc::COPYFILE_CLONE
        | libc::COPYFILE_NOFOLLOW_SRC
        | libc::COPYFILE_EXCL;
    // SAFETY: valid C strings, no state object.
    if unsafe { libc::copyfile(s.as_ptr(), d.as_ptr(), std::ptr::null_mut(), flags) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// File count, total bytes and a BLAKE3 hash over names, kinds, link targets and contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    pub files: u64,
    pub bytes: u64,
    pub hash: [u8; 32],
}

/// Summarize a folder for verification. Files are read past the page cache, so a copy on a drive
/// is checked as the drive stores it. Any unreadable entry fails the whole check.
pub fn tree(root: &Path) -> io::Result<Tree> {
    let mut t = Tree { files: 0, bytes: 0, hash: [0; 32] };
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
        h.update(rel.as_os_str().as_bytes());
        h.update(&[0]);
        if ft.is_dir() {
            h.update(b"d");
            walk(root, &rel, t, h)?;
            continue;
        }
        t.files += 1;
        t.bytes += m.size();
        if ft.is_file() {
            h.update(b"f");
            h.update(&m.size().to_le_bytes());
            h.update(hash_file(&root.join(&rel))?.as_bytes());
        } else if ft.is_symlink() {
            h.update(b"l");
            h.update(std::fs::read_link(root.join(&rel))?.as_os_str().as_bytes());
        } else {
            h.update(if ft.is_fifo() {
                b"p"
            } else if ft.is_socket() {
                b"s"
            } else {
                b"?"
            });
        }
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

/// Check that `copy` holds exactly what `orig` holds; say what differs if not.
pub fn verify(orig: &Path, copy: &Path) -> Result<()> {
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
        copy_tree(&src, &d.join("dst")).unwrap();
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
