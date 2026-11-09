//! Move and bring back on temporary APFS disk images. One image plays the home folder, another the drive.

mod common;
use common::image::{make_app_folder, run, Image};
use std::cell::Cell;
use std::path::Path;
use stowaway_core::fsx::{self, tree};
use stowaway_core::journal::{Op, Step};
use stowaway_core::mover::Mover;
use stowaway_core::rules::Catalog;

fn idle(_: &Path) -> Vec<String> {
    Vec::new()
}

fn xattr(p: &Path) -> String {
    let out = std::process::Command::new("xattr").args(["-p", "dev.stowaway.test"]).arg(p).output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn move_and_bring_back_across_volumes() {
    let (home_img, drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &idle);
    let folder = home.join(".ollama/models");
    make_app_folder(&folder);
    let before = tree(&folder).unwrap();

    let plan = m.plan_move(&folder, &drive.mount).unwrap();
    assert!(!plan.same_volume && plan.size > 3_000_000);
    assert!(folder.is_dir() && !folder.is_symlink(), "the dry run changes nothing");

    m.move_folder(&folder, &drive.mount).unwrap();
    let dest = drive.path("Stowaway/.ollama/models");
    assert_eq!(std::fs::read_link(&folder).unwrap(), dest);
    assert_eq!(tree(&folder.join("")).unwrap(), before, "same data through the link");
    assert_eq!(xattr(&dest.join("config.json")), "kept", "extended attributes kept");
    assert!(dest.join("models/latest").is_symlink(), "inner links stay links");
    assert!(!fsx::lexists(&home.join(".ollama/models.stowaway-moving")));
    let moved = m.journal.moved().unwrap();
    assert_eq!((moved.len(), moved[0].dest.as_path()), (1, dest.as_path()));

    let err = m.move_folder(&folder, &drive.mount).unwrap_err();
    assert!(err.0.contains("link already"), "{err}");

    m.bring_back(&folder).unwrap();
    assert!(folder.is_dir() && !folder.is_symlink());
    assert_eq!(tree(&folder).unwrap(), before);
    assert_eq!(xattr(&folder.join("config.json")), "kept");
    assert!(!fsx::lexists(&drive.path("Stowaway")), "empty offload folders tidied up");
    assert!(m.journal.moved().unwrap().is_empty() && m.journal.open_ops().unwrap().is_empty());
}

#[test]
fn same_volume_move_is_cloned() {
    let img = Image::new("APFS");
    let home = img.path("home");
    let folder = home.join(".big");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("blob"), vec![1u8; 400_000_000]).unwrap();
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &idle);
    let free_before = fsx::volume(&img.mount).unwrap().free;
    let plan = m.move_folder(&folder, &img.mount).unwrap();
    assert!(plan.same_volume);
    let free_after = fsx::volume(&img.mount).unwrap().free;
    assert!(free_before.abs_diff(free_after) < 100_000_000, "clone, not copy: {free_before} → {free_after}");
    assert_eq!(std::fs::metadata(folder.join("blob")).unwrap().len(), 400_000_000);
}

#[test]
fn refusals_change_nothing() {
    let (home_img, drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &idle);
    for rel in [
        "Documents",
        "Documents/Work",
        "Library/Containers/net.whatsapp.WhatsApp",
        "Library/Group Containers/g",
        ".claude",
    ] {
        std::fs::create_dir_all(home.join(rel)).unwrap();
        let e = m.move_folder(&home.join(rel), &drive.mount).unwrap_err();
        assert!(e.0.contains("never moves") || e.0.contains("cannot be moved"), "{rel}: {e}");
        assert!(home.join(rel).is_dir() && !home.join(rel).is_symlink());
    }
    assert!(m.move_folder(&home_img.path("outside"), &drive.mount).unwrap_err().0.contains("home folder"));
    assert!(m.move_folder(&home.join(".nope"), &drive.mount).unwrap_err().0.contains("does not exist"));
    std::fs::create_dir_all(home.join(".x")).unwrap();
    assert!(m.move_folder(&home.join("Documents/../.x"), &drive.mount).unwrap_err().0.contains("home folder"));
    assert!(m.move_folder(&home.join(".x"), Path::new("/")).unwrap_err().0.contains("not a connected drive"));
    assert!(m.move_folder(&home.join(".x"), &drive.path("sub")).is_err(), "must be the drive itself");

    let busy = |_: &Path| vec!["hermes".to_string()];
    let mb = Mover::new(&cat, &busy);
    assert!(mb.move_folder(&home.join(".x"), &drive.mount).unwrap_err().0.contains("In use by hermes"));

    std::fs::create_dir_all(drive.path("Stowaway/.x")).unwrap();
    assert!(m.move_folder(&home.join(".x"), &drive.mount).unwrap_err().0.contains("already exists"));
    assert!(m.journal.entries().unwrap().is_empty(), "refusals are not journaled");
}

#[test]
fn refuses_drives_that_cannot_hold_app_data() {
    let (home_img, exfat) = (Image::new("APFS"), Image::new("ExFAT"));
    let home = home_img.path("home");
    std::fs::create_dir_all(home.join(".x")).unwrap();
    let cat = Catalog::new(&home);
    let e = Mover::new(&cat, &idle).move_folder(&home.join(".x"), &exfat.mount).unwrap_err();
    assert!(e.0.contains("APFS or Mac OS Extended") && e.0.contains("exfat"), "{e}");
}

#[test]
fn opened_during_the_copy_undoes_the_move() {
    let (home_img, drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let folder = home.join(".hermes");
    make_app_folder(&folder);
    let before = tree(&folder).unwrap();
    let calls = Cell::new(0);
    let opens_later = |_: &Path| {
        calls.set(calls.get() + 1);
        if calls.get() > 1 {
            vec!["hermes".into()]
        } else {
            vec![]
        }
    };
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &opens_later);
    let e = m.move_folder(&folder, &drive.mount).unwrap_err();
    assert!(e.0.contains("In use by hermes"), "{e}");
    assert_eq!(tree(&folder).unwrap(), before);
    assert!(!fsx::lexists(&drive.path("Stowaway/.hermes")), "copy removed");
    assert_eq!(m.journal.latest().unwrap()[0].step, Step::Undone);
}

#[test]
fn bring_back_refusals() {
    let (home_img, mut drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(drive.path("Elsewhere/.y")).unwrap();
    std::os::unix::fs::symlink(drive.path("Elsewhere/.y"), home.join(".y")).unwrap();
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &idle);
    assert!(m.bring_back(&home.join(".y")).unwrap_err().0.contains("by hand"));
    make_app_folder(&home.join(".z"));
    m.move_folder(&home.join(".z"), &drive.mount).unwrap();
    drive.detach();
    let e = m.bring_back(&home.join(".z")).unwrap_err();
    assert!(e.0.starts_with("Connect Stw"), "{e}");
    drive.attach();
    m.bring_back(&home.join(".z")).unwrap();
    assert_eq!(m.journal.latest().unwrap().iter().filter(|e| e.op == Op::BringBack).count(), 1);
}

#[test]
fn a_moved_folder_inside_a_moved_folder_stays_a_link() {
    let (home_img, d1, d2) = (Image::new("APFS"), Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let cat = Catalog::new(&home);
    let m = Mover::new(&cat, &idle);
    make_app_folder(&home.join(".cache/huggingface"));
    std::fs::write(home.join(".cache/other"), "o").unwrap();
    m.move_folder(&home.join(".cache/huggingface"), &d1.mount).unwrap();
    m.move_folder(&home.join(".cache"), &d2.mount).unwrap();
    let inner = d2.path("Stowaway/.cache/huggingface");
    assert_eq!(std::fs::read_link(&inner).unwrap(), d1.path("Stowaway/.cache/huggingface"));
    assert_eq!(std::fs::read(home.join(".cache/huggingface/config.json")).unwrap(), br#"{"a":1}"#);
    run("test", &["-f", home.join(".cache/huggingface/models/latest").to_str().unwrap()]);
}
