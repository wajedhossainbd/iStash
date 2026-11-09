//! Find big folders under home and judge each one.

use crate::rules::{self, Catalog, Verdict};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub path: PathBuf,
    pub show: String,
    pub app: String,
    pub verdict: Verdict,
    pub note: String,
    /// Bytes on disk; None when deliberately not measured.
    pub size: Option<u64>,
    /// A link to a folder on an external drive.
    pub moved: bool,
    pub target: Option<PathBuf>,
    pub drive: Option<String>,
    pub connected: bool,
    /// An information line, not a folder that can be acted on.
    pub info: bool,
}

/// Expand one pattern relative to `base` (or absolute). Wildcards may appear in any component;
/// like a shell glob, `*` does not match names starting with a dot unless the pattern does.
pub fn expand(base: &Path, pattern: &str) -> Vec<PathBuf> {
    let (mut found, rest) = if let Some(abs) = pattern.strip_prefix('/') {
        (vec![PathBuf::from("/")], abs)
    } else {
        (vec![base.to_path_buf()], pattern)
    };
    for comp in rest.split('/') {
        let mut next = Vec::new();
        for dir in &found {
            if !comp.contains(['*', '?']) {
                let p = dir.join(comp);
                if p.symlink_metadata().is_ok() {
                    next.push(p);
                }
                continue;
            }
            let Ok(entries) = std::fs::read_dir(dir) else { continue };
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if (name.starts_with('.') && !comp.starts_with('.')) || !rules::fnmatch(comp, &name) {
                    continue;
                }
                next.push(e.path());
            }
        }
        found = next;
    }
    found
}

fn is_dir_or_link(p: &Path) -> bool {
    p.symlink_metadata().map(|m| m.is_dir() || m.file_type().is_symlink()).unwrap_or(false)
}

/// Folders worth measuring: scan patterns under home (minus skipped and never-read ones), plus links
/// outside home that point to a drive. Reads directory listings only, no sizes.
pub fn candidates(cat: &Catalog) -> Vec<PathBuf> {
    let mut paths: HashSet<PathBuf> = HashSet::new();
    for pattern in cat.scan_patterns() {
        // Check the pattern itself first, so a never-read folder is not even listed.
        if rules::never_read(&pattern) {
            continue;
        }
        for p in expand(&cat.home, &pattern) {
            let rel = cat.rel_of(&p);
            if is_dir_or_link(&p) && !rules::skipped(&rel) && !rules::never_read(&rel) {
                paths.insert(p);
            }
        }
    }
    for pattern in rules::EXTRA_LINKS {
        paths.extend(expand(&cat.home, pattern).into_iter().filter(|p| p.is_symlink()));
    }
    let mut v: Vec<PathBuf> = paths.into_iter().collect();
    v.sort();
    v
}

/// Bytes on disk under a folder (like `du -sk`), following the folder itself if it is a moved folder's
/// link. Hard links are counted once; unreadable entries are skipped.
pub fn disk_usage(path: &Path) -> u64 {
    let root = if path.is_symlink() { path.join("") } else { path.to_path_buf() };
    let mut seen = HashSet::new();
    let mut total = 0u64;
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let Ok(m) = dir.metadata() else { continue };
        total += m.blocks() * 512;
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let Ok(m) = e.metadata() else { continue }; // DirEntry::metadata does not follow links
            if m.is_dir() {
                stack.push(e.path());
            } else if m.nlink() < 2 || seen.insert((m.dev(), m.ino())) {
                total += m.blocks() * 512;
            }
        }
    }
    total
}

/// Measure many folders in parallel.
pub fn sizes(paths: &[PathBuf], threads: usize) -> HashMap<PathBuf, u64> {
    let next = AtomicUsize::new(0);
    let out = Mutex::new(HashMap::new());
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(p) = paths.get(i) else { break };
                let size = disk_usage(p);
                out.lock().unwrap().insert(p.clone(), size);
            });
        }
    });
    out.into_inner().unwrap()
}

/// The drive a moved folder's link points to: `/Volumes/<drive>/…`.
pub fn drive_of(target: &Path) -> Option<String> {
    let mut c = target.strip_prefix("/Volumes").ok()?.components();
    Some(c.next()?.as_os_str().to_string_lossy().into_owned())
}

pub fn make_item(cat: &Catalog, path: &Path, size: u64) -> Item {
    let rel = cat.rel_of(path);
    let (verdict, note) = cat.classify(&rel);
    let target = std::fs::read_link(path).ok();
    Item {
        path: path.to_path_buf(),
        show: cat.show(path),
        app: cat.owner(&rel).0,
        verdict,
        note,
        size: Some(size),
        moved: target.is_some(),
        drive: target.as_deref().and_then(drive_of),
        connected: path.exists(),
        target,
        info: false,
    }
}

/// One line for all sandboxed apps' data: never moved, and not measured (see `rules::SCAN`).
pub fn sandboxed_item(cat: &Catalog) -> Item {
    Item {
        path: cat.home.join("Library/Containers"),
        show: "~/Library/Containers and ~/Library/Group Containers".into(),
        app: "Sandboxed apps (WhatsApp, Office, Telegram…)".into(),
        verdict: Verdict::Keep,
        note: "macOS stops these apps from following a link to another drive, so their data stays here. \
               Not measured: reading it makes macOS ask for permission. Clear space inside each app instead."
            .into(),
        size: None,
        moved: false,
        target: None,
        drive: None,
        connected: true,
        info: true,
    }
}

/// Folders of at least `min_bytes` (and every moved folder), largest first, plus info lines.
pub fn scan(cat: &Catalog, min_bytes: u64) -> Vec<Item> {
    let paths = candidates(cat);
    let mut sizes = sizes(&paths, 8);
    subtract_nested(&paths, &mut sizes);
    let mut items: Vec<Item> = paths
        .iter()
        .filter_map(|p| {
            let size = sizes[p];
            match std::fs::read_link(p) {
                Ok(t) if !t.starts_with("/Volumes/") => None, // an ordinary link, not something moved to a drive
                Ok(_) => Some(make_item(cat, p, size)),
                Err(_) if size < min_bytes => None,
                Err(_) => Some(make_item(cat, p, size)),
            }
        })
        .collect();
    items.sort_by(|a, b| b.size.cmp(&a.size).then(a.path.cmp(&b.path)));
    items.push(sandboxed_item(cat));
    items
}

/// A listed folder inside another listed folder is counted once, in the inner one.
pub fn subtract_nested(paths: &[PathBuf], sizes: &mut HashMap<PathBuf, u64>) {
    for p in paths {
        for q in paths {
            if q != p && q.starts_with(p) && !q.is_symlink() {
                let inner = sizes[q];
                let outer = sizes.get_mut(p).unwrap();
                *outer = outer.saturating_sub(inner);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("stowaway-scan-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn candidates_skip_protected_and_containers() {
        let home = tmp("cand");
        for d in [
            ".gradle",
            ".Trash",
            "Library/Application Support/Notion",
            "Library/Application Support/AddressBook",
            "Library/Application Support/Claude/vm_bundles",
            "Library/Containers/com.docker.docker",
            "Library/Caches/x",
            "Visible",
        ] {
            fs::create_dir_all(home.join(d)).unwrap();
        }
        fs::write(home.join(".zshrc"), "").unwrap(); // a file, not a folder
        let cat = Catalog::new(&home);
        let got: Vec<String> = candidates(&cat).iter().map(|p| cat.rel_of(p)).collect();
        for want in [
            ".gradle",
            "Library/Application Support/Notion",
            "Library/Application Support/Claude/vm_bundles",
            "Library/Caches/x",
        ] {
            assert!(got.contains(&want.to_string()), "missing {want}: {got:?}");
        }
        for not in [".Trash", ".zshrc", "Library/Application Support/AddressBook", "Visible"] {
            assert!(!got.contains(&not.to_string()), "{not} listed");
        }
        assert!(!got.iter().any(|g| g.contains("Containers")), "{got:?}");
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn sizes_and_nesting() {
        let home = tmp("size");
        fs::create_dir_all(home.join("outer/inner")).unwrap();
        fs::write(home.join("outer/a"), vec![1u8; 100_000]).unwrap();
        fs::write(home.join("outer/inner/b"), vec![1u8; 300_000]).unwrap();
        fs::hard_link(home.join("outer/inner/b"), home.join("outer/inner/b2")).unwrap();
        let paths = vec![home.join("outer"), home.join("outer/inner")];
        let mut s = sizes(&paths, 2);
        let (outer, inner) = (s[&paths[0]], s[&paths[1]]);
        assert!((300_000..400_000).contains(&inner), "hard link counted once: {inner}");
        assert!(outer >= 400_000);
        subtract_nested(&paths, &mut s);
        assert_eq!(s[&paths[0]], outer - inner);
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn moved_links_are_listed_and_ordinary_links_are_not() {
        let home = tmp("links");
        fs::create_dir_all(home.join("real")).unwrap();
        std::os::unix::fs::symlink(home.join("real"), home.join(".ordinary")).unwrap();
        std::os::unix::fs::symlink("/Volumes/NoSuchDrive/Stowaway/.hermes", home.join(".hermes")).unwrap();
        let items = scan(&Catalog::new(&home), 1);
        let hermes = items.iter().find(|i| i.show == "~/.hermes").expect("moved folder listed");
        assert!(hermes.moved && !hermes.connected);
        assert_eq!(hermes.drive.as_deref(), Some("NoSuchDrive"));
        assert!(!items.iter().any(|i| i.show == "~/.ordinary"));
        assert!(items.last().unwrap().info, "sandboxed line last");
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn expand_globs() {
        let home = tmp("glob");
        fs::create_dir_all(home.join("A/x1")).unwrap();
        fs::create_dir_all(home.join("A/.hidden")).unwrap();
        fs::create_dir_all(home.join("A/y")).unwrap();
        let mut got: Vec<_> = expand(&home, "A/*").iter().map(|p| p.file_name().unwrap().to_owned()).collect();
        got.sort();
        assert_eq!(got, ["x1", "y"]);
        assert_eq!(expand(&home, "A/x?").len(), 1);
        assert_eq!(expand(&home, "A/.*").len(), 1);
        assert_eq!(expand(&home, "B/*").len(), 0);
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn drive_names() {
        assert_eq!(drive_of(Path::new("/Volumes/My SSD/Stowaway/.x")).as_deref(), Some("My SSD"));
        assert_eq!(drive_of(Path::new("/opt/x")), None);
    }
}
