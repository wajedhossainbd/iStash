//! Temporary disk images (names stay under 15 characters, the exFAT label limit) for integration tests: created with hdiutil, detached and deleted on drop.
//! Tests only ever touch folders inside these images, never the user's real folders or drives.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static N: AtomicUsize = AtomicUsize::new(0);

pub struct Image {
    pub mount: PathBuf,
    file: PathBuf,
    attached: bool,
}

impl Image {
    /// A 20 GB sparse image (takes only the space written) with the given file system, e.g. "APFS", "ExFAT".
    pub fn new(fs: &str) -> Image {
        Image::sized(fs, "20g")
    }

    /// Like [`Image::new`] with a size such as "2g".
    pub fn sized(fs: &str, size: &str) -> Image {
        let name = format!("Stw{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed));
        let file = std::env::temp_dir().join(format!("{name}.sparseimage"));
        let _ = std::fs::remove_file(&file);
        let base = file.with_extension("");
        run(
            "hdiutil",
            &[
                "create",
                "-quiet",
                "-size",
                size,
                "-type",
                "SPARSE",
                "-fs",
                fs,
                "-volname",
                &name,
                base.to_str().unwrap(),
            ],
        );
        let mut img = Image { mount: PathBuf::from("/Volumes").join(&name), file, attached: false };
        img.attach();
        img
    }

    pub fn attach(&mut self) {
        run("hdiutil", &["attach", "-quiet", "-nobrowse", self.file.to_str().unwrap()]);
        assert!(self.mount.is_dir(), "{} not mounted", self.mount.display());
        self.attached = true;
    }

    pub fn detach(&mut self) {
        run("hdiutil", &["detach", "-quiet", "-force", self.mount.to_str().unwrap()]);
        self.attached = false;
    }

    /// Detach without force; returns hdiutil's success (false when something dissents).
    pub fn try_detach(&mut self) -> bool {
        let ok =
            Command::new("hdiutil").args(["detach", self.mount.to_str().unwrap()]).output().unwrap().status.success();
        if ok {
            self.attached = false;
        }
        ok
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.mount.join(rel)
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        if self.attached {
            self.detach();
        }
        let _ = std::fs::remove_file(&self.file);
    }
}

pub fn run(cmd: &str, args: &[&str]) {
    let out = Command::new(cmd).args(args).output().unwrap();
    assert!(out.status.success(), "{cmd} {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

/// A small app-data-like folder: nested folders, a big file, relative and absolute links, an xattr.
pub fn make_app_folder(dir: &Path) {
    std::fs::create_dir_all(dir.join("models/blobs")).unwrap();
    std::fs::create_dir_all(dir.join("empty")).unwrap();
    std::fs::write(dir.join("config.json"), r#"{"a":1}"#).unwrap();
    std::fs::write(dir.join("models/blobs/sha-1"), vec![42u8; 3_000_000]).unwrap();
    std::fs::write(dir.join(".hidden"), "h").unwrap();
    std::os::unix::fs::symlink("blobs/sha-1", dir.join("models/latest")).unwrap();
    std::os::unix::fs::symlink(dir.join("config.json"), dir.join("abs-link")).unwrap();
    std::os::unix::fs::symlink("/Volumes/NoSuchDrive/x", dir.join("dangling")).unwrap();
    run("xattr", &["-w", "dev.stowaway.test", "kept", dir.join("config.json").to_str().unwrap()]);
}
