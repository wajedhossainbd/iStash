//! Fault injection on temporary disk images. Every case must leave the data in exactly one valid place:
//! either the original untouched (and nothing left on the drive), or moved and linked.

mod common;
use common::image::{make_app_folder, run, Image};
use std::cell::Cell;
use std::io::Write;
use std::os::unix::fs::{FileExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use stowaway_core::fsx::{self, lexists, tree};
use stowaway_core::journal::Step;
use stowaway_core::mover::Mover;
use stowaway_core::rules::Catalog;
use stowaway_core::set_step_hook;

fn idle(_: &Path) -> Vec<String> {
    Vec::new()
}

/// Run `f` the first time the mover reaches `step` on this thread.
fn at(step: &'static str, f: impl FnOnce() + 'static) {
    let f = Cell::new(Some(f));
    set_step_hook(Some(Box::new(move |s| {
        if s == step {
            if let Some(f) = f.take() {
                f()
            }
        }
    })));
}

fn force_detach(mount: &Path) {
    run("hdiutil", &["detach", "-quiet", "-force", mount.to_str().unwrap()]);
}

/// The original is a real folder with the same data, and the move left nothing on the drive.
fn untouched(folder: &Path, before: &fsx::Tree, drive: &Image) {
    assert!(folder.is_dir() && !folder.is_symlink(), "original still a folder");
    assert_eq!(&tree(folder).unwrap(), before, "original data unchanged");
    assert!(!lexists(&drive.path("Stowaway")), "nothing left on the drive, not even empty folders");
}

#[test]
fn drive_detached_mid_copy() {
    let (home_img, mut drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let folder = home.join("Library/Caches/com.example.big");
    make_app_folder(&folder);
    let before = tree(&folder).unwrap();
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &idle);

    let mount = drive.mount.clone();
    at("copy:file", move || force_detach(&mount));
    let e = m.move_folder(&folder, &drive.mount).unwrap_err();
    set_step_hook(None);
    assert!(e.0.contains("removed by recovery"), "{e}");
    assert!(folder.is_dir() && !folder.is_symlink());
    assert_eq!(tree(&folder).unwrap(), before);
    assert_eq!(m.journal.open_ops().unwrap().len(), 1, "kept open: the drive still holds a partial copy");
    assert!(m.move_folder(&folder, &drive.mount).unwrap_err().0.contains("Run recovery"));

    drive.attach();
    let out = m.recover().unwrap();
    assert!(out[0].contains("move undone"), "{out:?}");
    untouched(&folder, &before, &drive);
    m.move_folder(&folder, &drive.mount).unwrap();
    assert_eq!(tree(&folder.join("")).unwrap(), before, "moves fine once recovered");
}

#[test]
fn drive_detached_before_verify() {
    let (home_img, mut drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let folder = home.join(".hermes");
    make_app_folder(&folder);
    let before = tree(&folder).unwrap();
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &idle);

    let mount = drive.mount.clone();
    at("move:copied-unverified", move || force_detach(&mount));
    let e = m.move_folder(&folder, &drive.mount).unwrap_err();
    set_step_hook(None);
    assert!(e.0.contains("Could not read") && e.0.contains("removed by recovery"), "{e}");
    assert_eq!(tree(&folder).unwrap(), before);

    drive.attach();
    assert!(lexists(&drive.path("Stowaway/.hermes")), "the full but unchecked copy is still there");
    m.recover().unwrap();
    untouched(&folder, &before, &drive);
}

#[test]
fn drive_fills_up_during_copy() {
    let (home_img, drive) = (Image::new("APFS"), Image::sized("APFS", "2g"));
    let home = home_img.path("home");
    let folder = home.join("Library/Caches/com.example.big");
    make_app_folder(&folder);
    std::fs::write(folder.join("big"), vec![9u8; 300_000_000]).unwrap();
    let before = tree(&folder).unwrap();
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &idle);

    // The space check passes; then something else fills the drive before the copy starts.
    let mount = drive.mount.clone();
    at("move:begin", move || {
        let left = fsx::volume(&mount).unwrap().free.saturating_sub(50_000_000);
        let mut f = std::fs::File::create(mount.join("filler")).unwrap();
        let chunk = vec![1u8; 8 << 20];
        for _ in 0..left / chunk.len() as u64 {
            f.write_all(&chunk).unwrap();
        }
    });
    let e = m.move_folder(&folder, &drive.mount).unwrap_err();
    set_step_hook(None);
    assert!(e.0.contains("Copy failed, so nothing was changed") && e.0.contains("No space left"), "{e}");
    untouched(&folder, &before, &drive);
    assert_eq!(m.journal.latest().unwrap()[0].step, Step::Undone);
}

#[test]
fn unreadable_file_is_named_not_blamed_on_the_drive() {
    let (home_img, drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let folder = home.join(".hermes");
    make_app_folder(&folder);
    let secret = folder.join("models/secret.key");
    std::fs::write(&secret, "s").unwrap();
    let before = tree(&folder).unwrap();
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000)).unwrap();
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &idle);

    let e = m.move_folder(&folder, &drive.mount).unwrap_err();
    assert!(e.0.contains("secret.key") && e.0.contains("Permission denied"), "{e}");
    assert!(!e.0.contains("System Settings"), "not the drive's fault: {e}");
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o644)).unwrap();
    untouched(&folder, &before, &drive);
    assert_eq!(m.journal.latest().unwrap()[0].step, Step::Undone);
}

#[test]
fn source_changed_during_copy_is_caught() {
    let (home_img, drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let folder = home.join(".hermes");
    make_app_folder(&folder);
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &idle);

    let file = folder.join("models/blobs/sha-1");
    at("move:copied-unverified", move || {
        std::fs::OpenOptions::new().write(true).open(&file).unwrap().write_at(b"new", 1000).unwrap();
    });
    let e = m.move_folder(&folder, &drive.mount).unwrap_err();
    set_step_hook(None);
    assert!(e.0.contains("did not match"), "{e}");
    let after = tree(&folder).unwrap();
    untouched(&folder, &after, &drive);
    assert_eq!(&std::fs::read(folder.join("models/blobs/sha-1")).unwrap()[1000..1003], b"new", "the new write kept");
}

/// Sparse file, hard links, a FIFO, a socket, an ACL and non-ASCII names.
fn special_folder(dir: &Path) {
    make_app_folder(dir);
    let f = std::fs::File::create(dir.join("Docker.raw")).unwrap();
    f.set_len(1 << 30).unwrap();
    f.write_at(b"data", 1 << 29).unwrap();
    std::fs::write(dir.join("a"), "linked").unwrap();
    std::fs::hard_link(dir.join("a"), dir.join("models/b")).unwrap();
    run("mkfifo", &[dir.join("fifo").to_str().unwrap()]);
    drop(std::os::unix::net::UnixListener::bind(dir.join("sock")).unwrap());
    run("chmod", &["+a", "everyone deny delete", dir.join("config.json").to_str().unwrap()]);
    for name in ["cafe\u{301}", "日本語", "😀 file.txt", "আমার ফাইল"] {
        std::fs::write(dir.join(name), name).unwrap();
    }
}

fn check_special(dir: &Path, before: &fsx::Tree) {
    assert_eq!(&tree(dir).unwrap(), before, "names (byte for byte), links and contents");
    let raw = std::fs::metadata(dir.join("Docker.raw")).unwrap();
    assert_eq!(raw.len(), 1 << 30);
    assert!(raw.blocks() * 512 < 10_000_000, "sparse file stayed sparse: {} bytes on disk", raw.blocks() * 512);
    let (a, b) = (std::fs::metadata(dir.join("a")).unwrap(), std::fs::metadata(dir.join("models/b")).unwrap());
    assert_eq!((a.ino(), a.nlink()), (b.ino(), 2), "hard link kept");
    assert!(!lexists(&dir.join("fifo")) && !lexists(&dir.join("sock")), "sockets and FIFOs are not copied");
    let ls = std::process::Command::new("ls").arg("-le").arg(dir.join("config.json")).output().unwrap();
    let ls = String::from_utf8_lossy(&ls.stdout);
    assert!(ls.contains("everyone deny delete"), "ACL kept in {}: {ls}", dir.display());
}

fn special_round_trip(same_volume: bool) {
    let (home_img, other) = (Image::new("APFS"), Image::new("APFS"));
    let drive = if same_volume { &home_img } else { &other };
    let home = home_img.path("home");
    let folder = home.join(".docker-data");
    special_folder(&folder);
    let before = tree(&folder).unwrap();
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &idle);

    let mut plan = m.move_folder(&folder, &drive.mount).unwrap();
    assert_eq!(plan.same_volume, same_volume);
    plan.skipped.sort();
    assert_eq!(plan.skipped, [PathBuf::from("fifo"), PathBuf::from("sock")], "reported as left out");
    assert!(folder.is_symlink());
    check_special(&drive.path("Stowaway/.docker-data"), &before);

    let back = m.bring_back(&folder).unwrap();
    assert!(back.skipped.is_empty());
    assert!(folder.is_dir() && !folder.is_symlink());
    check_special(&folder, &before);
    assert!(!lexists(&drive.path("Stowaway")));
}

#[test]
fn special_files_across_volumes() {
    special_round_trip(false);
}

#[test]
fn special_files_on_the_same_volume() {
    special_round_trip(true);
}

#[test]
fn paths_longer_than_path_max() {
    let (home_img, drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let folder = home.join(".deep");
    std::fs::create_dir_all(&folder).unwrap();
    let seg = "ঘ".repeat(80); // 240 bytes per name, about 1,250 bytes in all: past PATH_MAX (1,024)
    let deep = format!("cd '{}' && for i in 1 2 3 4 5; do mkdir {seg} && cd {seg}; done", folder.display());
    run("sh", &["-c", &format!("{deep} && echo deep > f")]);
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &idle);

    // copyfile itself stops at PATH_MAX, so such a folder is refused, cleanly.
    let e = m.move_folder(&folder, &drive.mount).unwrap_err();
    assert!(e.0.contains("Copy failed, so nothing was changed") && e.0.contains("File name too long"), "{e}");
    assert!(folder.is_dir() && !folder.is_symlink());
    let cd = format!("cd '{}' && for i in 1 2 3 4 5; do cd {seg}; done && cat f", folder.display());
    let out = std::process::Command::new("sh").args(["-c", &cd]).output().unwrap();
    assert_eq!(out.stdout, b"deep\n", "the deep file is still in place: {}", String::from_utf8_lossy(&out.stderr));
    assert!(!lexists(&drive.path("Stowaway")));
    assert_eq!(m.journal.latest().unwrap()[0].step, Step::Undone);
}

#[test]
fn unicode_and_long_names_round_trip() {
    let (home_img, drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let folder = home.join(".ünïcødé");
    // Names at the 255-byte limit, nested to about 1,000 bytes: long, but inside PATH_MAX.
    let long = folder.join("ঘ".repeat(85)).join("a".repeat(255)).join("😀".repeat(63));
    std::fs::create_dir_all(&long).unwrap();
    std::fs::write(long.join("ক"), "deep").unwrap();
    let before = tree(&folder).unwrap();
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &idle);

    m.move_folder(&folder, &drive.mount).unwrap();
    assert_eq!(tree(&drive.path("Stowaway/.ünïcødé")).unwrap(), before);
    m.bring_back(&folder).unwrap();
    assert_eq!(tree(&folder).unwrap(), before);
    assert!(!lexists(&drive.path("Stowaway")));
}
