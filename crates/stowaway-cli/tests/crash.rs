//! Kill the mover at every step (SIGKILL, like a crash or power loss), then recover: the data must be
//! identical and in exactly one place. Runs the real CLI binary with HOME inside a temporary disk image.

#[path = "../../stowaway-core/tests/common/image.rs"]
mod image;

use image::{make_app_folder, Image};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, Output};
use stowaway_core::fsx::{lexists, tree};
use stowaway_core::journal::Journal;

fn cli(home: &Path, crash_at: Option<&str>, args: &[&str]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_stowaway-cli"));
    c.args(args).env("HOME", home).env_remove("STOWAWAY_CRASH_AT");
    if let Some(p) = crash_at {
        c.env("STOWAWAY_CRASH_AT", p);
    }
    c.output().unwrap()
}

fn ok(o: &Output) -> String {
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success(), "{text}");
    text
}

fn killed(o: &Output, point: &str) {
    assert_eq!(o.status.signal(), Some(9), "{point}: not killed: {}", String::from_utf8_lossy(&o.stderr));
}

const MOVE_POINTS: &[(&str, bool)] = &[
    // (crash point, folder ends up moved)
    ("move:begin", false),
    ("move:copied-unverified", false),
    ("move:copied", false),
    ("move:aside-unjournaled", false),
    ("move:aside", false),
    ("move:linked-unjournaled", true),
    ("move:linked", true),
    ("move:deleted-unjournaled", true),
];

const BACK_POINTS: &[(&str, bool)] = &[
    // (crash point, folder ends up back on this Mac)
    ("back:begin", false),
    ("back:copied-unverified", false),
    ("back:copied", false),
    ("back:unlinked-unjournaled", true),
    ("back:unlinked", true),
    ("back:placed-unjournaled", true),
    ("back:placed", true),
    ("back:deleted-unjournaled", true),
];

fn assert_moved(folder: &Path, dest: &Path, moved: bool, point: &str) {
    if moved {
        assert_eq!(std::fs::read_link(folder).ok().as_deref(), Some(dest), "{point}: should be linked");
    } else {
        assert!(folder.is_dir() && !folder.is_symlink(), "{point}: should be a folder here");
        assert!(!lexists(dest), "{point}: drive copy should be gone");
    }
}

#[test]
fn kill_move_at_every_step() {
    let (home_img, drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let drive_s = drive.mount.to_str().unwrap();
    for &(point, moved) in MOVE_POINTS {
        let folder = home.join(format!(".app-{}", point.replace(':', "-")));
        let dest = drive.mount.join("Stowaway").join(folder.file_name().unwrap());
        make_app_folder(&folder);
        let before = tree(&folder).unwrap();
        killed(&cli(&home, Some(point), &["move", folder.to_str().unwrap(), drive_s, "--yes"]), point);
        let out = ok(&cli(&home, None, &["recover"]));
        assert!(out.contains("recovered:") && !out.contains("needs attention"), "{point}: {out}");
        assert_moved(&folder, &dest, moved, point);
        assert_eq!(tree(&folder.join("")).unwrap(), before, "{point}: data changed");
        assert!(!lexists(&home.join(format!("{}.stowaway-moving", folder.file_name().unwrap().to_str().unwrap()))));
        // Nothing is left half-done: the operation can be done (again) from here.
        let next = if moved { "bring-back" } else { "move" };
        let mut args = vec![next, folder.to_str().unwrap()];
        if !moved {
            args.push(drive_s);
        }
        args.push("--yes");
        ok(&cli(&home, None, &args));
        assert_moved(&folder, &dest, !moved, point);
        assert_eq!(tree(&folder.join("")).unwrap(), before);
    }
    assert!(Journal::for_home(&home).open_ops().unwrap().is_empty());
}

#[test]
fn kill_bring_back_at_every_step() {
    let (home_img, drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    for &(point, back) in BACK_POINTS {
        let folder = home.join(format!(".app-{}", point.replace(':', "-")));
        let dest = drive.mount.join("Stowaway").join(folder.file_name().unwrap());
        make_app_folder(&folder);
        let before = tree(&folder).unwrap();
        ok(&cli(&home, None, &["move", folder.to_str().unwrap(), drive.mount.to_str().unwrap(), "--yes"]));
        killed(&cli(&home, Some(point), &["bring-back", folder.to_str().unwrap(), "--yes"]), point);
        let out = ok(&cli(&home, None, &["recover"]));
        assert!(out.contains("recovered:") && !out.contains("needs attention"), "{point}: {out}");
        assert_moved(&folder, &dest, !back, point);
        assert_eq!(tree(&folder.join("")).unwrap(), before, "{point}: data changed");
    }
    assert!(Journal::for_home(&home).open_ops().unwrap().is_empty());
}

#[test]
fn recreated_folder_after_a_crash_is_left_for_the_user() {
    let (home_img, drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let folder = home.join(".app");
    make_app_folder(&folder);
    let before = tree(&folder).unwrap();
    killed(
        &cli(&home, Some("move:aside"), &["move", folder.to_str().unwrap(), drive.mount.to_str().unwrap(), "--yes"]),
        "aside",
    );
    std::fs::create_dir(&folder).unwrap(); // the app started again and made a fresh, empty folder
    let out = ok(&cli(&home, None, &["recover"]));
    assert!(out.contains("needs attention"), "{out}");
    assert_eq!(tree(&home.join(".app.stowaway-moving")).unwrap(), before, "original kept");
    assert!(lexists(&drive.mount.join("Stowaway/.app")), "checked copy kept");
    let o = cli(&home, None, &["move", folder.to_str().unwrap(), drive.mount.to_str().unwrap(), "--yes"]);
    assert!(!o.status.success(), "blocked until the user sorts it out");
}

#[test]
fn recovery_waits_for_a_missing_drive() {
    let (home_img, mut drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let folder = home.join(".app");
    make_app_folder(&folder);
    let before = tree(&folder).unwrap();
    killed(
        &cli(&home, Some("move:copied"), &["move", folder.to_str().unwrap(), drive.mount.to_str().unwrap(), "--yes"]),
        "copied",
    );
    drive.detach();
    let out = ok(&cli(&home, None, &["recover"]));
    assert!(out.contains("when its drive is connected"), "{out}");
    assert_eq!(tree(&folder).unwrap(), before);
    drive.attach();
    let out = ok(&cli(&home, None, &["recover"]));
    assert!(out.contains("move undone"), "{out}");
    assert!(!lexists(&drive.mount.join("Stowaway/.app")));
    assert!(Journal::for_home(&home).open_ops().unwrap().is_empty());
}
