//! Drive map and eject guard against a temporary disk image that is detached and reattached.

mod common;
use common::image::{make_app_folder, Image};
use std::path::Path;
use stowaway_core::diskarb::EjectGuard;
use stowaway_core::drives::{self, eject_objection};
use stowaway_core::mover::Mover;
use stowaway_core::rules::Catalog;

#[test]
fn drive_map_and_eject_guard() {
    let (home_img, mut drive) = (Image::new("APFS"), Image::new("APFS"));
    let home = home_img.path("home");
    let cat = Catalog::new(&home);
    let idle = |_: &Path| Vec::new();
    let m = Mover::new(&cat, &idle);
    let name = drive.mount.file_name().unwrap().to_string_lossy().into_owned();
    make_app_folder(&home.join(".hermes"));
    m.move_folder(&home.join(".hermes"), &drive.mount).unwrap();

    // This Mac's own links outside home (/opt/homebrew/share/*) are listed too; keep the test's drive only.
    let moved: Vec<_> = drives::moved_items(&cat, &m.journal).into_iter().filter(|m| m.drive == name).collect();
    assert_eq!(moved.len(), 1, "{moved:?}");
    assert_eq!((moved[0].drive.as_str(), moved[0].app.as_str()), (name.as_str(), "hermes"));
    let map = drives::drives(&moved);
    let d = map.iter().find(|d| d.name == name).unwrap();
    assert!(d.connected && d.can_hold && d.free.is_some());
    assert_eq!((d.needed_by.clone(), d.folders.clone()), (vec!["hermes".to_string()], vec!["~/.hermes".to_string()]));

    // While hermes "runs", the guard refuses to let the drive go; other volumes are not affected.
    let for_policy = moved.clone();
    let guard = EjectGuard::start(Box::new(move |mount: &Path| {
        eject_objection(mount, &for_policy, &|_| vec!["hermes".into()])
    }));
    assert!(!drive.try_detach(), "eject of a needed drive refused");
    assert!(drive.mount.is_dir());
    let mut other = Image::new("APFS");
    assert!(other.try_detach(), "an unrelated drive ejects normally");
    drop(guard);
    assert!(drive.try_detach(), "allowed once the guard stops");

    let map = drives::drives(&moved);
    let d = map.iter().find(|d| d.name == name).unwrap();
    assert!(!d.connected && !d.can_hold);
    assert_eq!(d.needed_by, ["hermes"]);
    assert!(!home.join(".hermes").exists() && home.join(".hermes").is_symlink(), "link dangles while unplugged");
    drive.attach();
    assert!(home.join(".hermes/config.json").is_file(), "works again when reconnected");
}
