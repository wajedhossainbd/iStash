//! What may move, what stays, and which app owns a folder. Paths are matched relative to home.
//!
//! Order of precedence: the user's own "keep" list, then app recipes (`recipes.toml`), then the
//! generic safety rules below, then "move". The mover re-checks [`never_move`] whatever a recipe says.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// Safe to move with a link left behind (quit the app first).
    Move,
    /// Move it with the app's own setting.
    App,
    /// Leave it on this Mac.
    Keep,
    /// Known to break when moved; Stowaway will not do it.
    Unsupported,
}

/// Generic rules, first match wins (after recipes).
pub const RULES: &[(&str, Verdict, &str)] = &[
    ("Documents", Verdict::Keep, "Your documents stay on this Mac."),
    ("Documents/*", Verdict::Keep, "Your documents stay on this Mac."),
    (
        "Library/Containers/*",
        Verdict::Keep,
        "Sandboxed app: macOS stops it from following a link to another drive, so moving it breaks the app.",
    ),
    (
        "Library/Group Containers/*",
        Verdict::Keep,
        "Shared data of sandboxed apps (WhatsApp, Office, Telegram…): macOS stops them following a link.",
    ),
    ("Library/Application Support/com.apple.*", Verdict::Keep, "macOS data."),
    ("Library/Caches/com.apple.*", Verdict::Keep, "macOS cache, managed by macOS."),
    ("Library/Mobile Documents", Verdict::Keep, "iCloud Drive."),
    ("Library/CloudStorage", Verdict::Keep, "Google Drive / Dropbox / OneDrive sync folders."),
    (".Trash", Verdict::Keep, "The Trash: empty it instead."),
];

/// Where to look for big folders (relative to home). Recipe folders are added to this.
/// Other apps' private folders (Library/Containers, Library/Group Containers) are deliberately not read:
/// macOS asks permission every time, and that data can never be moved anyway.
pub const SCAN: &[&str] = &[
    ".*",
    "Library/Application Support/*",
    "Library/Caches/*",
    "Library/Developer/Xcode/*",
    "Library/Developer/CoreSimulator",
    "Library/Android",
];

/// Privacy-protected Apple folders: reading them makes macOS ask permission, so they are not measured.
pub const SKIP: &[&str] = &[
    "Library/Application Support/AddressBook",
    "Library/Application Support/CallHistory*",
    "Library/Application Support/com.apple.TCC",
    "Library/Application Support/Knowledge",
    "Library/Application Support/FileProvider",
    "Library/Application Support/MobileSync",
    "Library/Application Support/CloudDocs",
    // macOS's own data, never moved; some of it (Music, TV, Photos, media services) is behind a privacy prompt.
    "Library/Application Support/com.apple.*",
    "Library/Caches/com.apple.*",
    "Library/Application Support/iLifeMediaBrowser",
    ".Trash",
];

/// Never read, whatever a recipe or scan pattern says.
pub const NEVER_READ: &[&str] =
    &["Library/Containers", "Library/Containers/*", "Library/Group Containers", "Library/Group Containers/*"];

/// Never moved by Stowaway, whatever a recipe says.
pub const NEVER_MOVE: &[&str] = &[
    "Documents",
    "Documents/*",
    "Library/Containers",
    "Library/Containers/*",
    "Library/Group Containers",
    "Library/Group Containers/*",
    "Library/Mobile Documents",
    "Library/Mobile Documents/*",
    "Library/CloudStorage",
    "Library/CloudStorage/*",
];

/// Links outside home that may point to a moved folder on a drive.
pub const EXTRA_LINKS: &[&str] = &["/opt/homebrew/share/*", "/usr/local/share/*"];

/// Readable names for bare programs.
pub const PROCESS_NAMES: &[(&str, &str)] = &[
    ("claude", "Claude Code (CLI)"),
    ("node", "Node.js"),
    ("python3", "Python"),
    ("language_server", "Antigravity"),
    ("com.apple.Virtualization.VirtualMachine", "Virtual machines (Claude Cowork, Docker)"),
];

const RECIPES_TOML: &str = include_str!("../recipes.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub app: String,
    /// Regex on running command lines that means this app is open.
    #[serde(default)]
    pub running: Option<String>,
    #[serde(default)]
    pub folder: Vec<Folder>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Folder {
    /// Relative to home; `*` matches anything (including `/`).
    pub path: String,
    /// Overrides the recipe's app name for this folder.
    #[serde(default)]
    pub app: Option<String>,
    /// None: only names the owner; the generic rules decide.
    #[serde(default)]
    pub verdict: Option<Verdict>,
    #[serde(default)]
    pub note: String,
}

#[derive(Deserialize)]
struct RecipeFile {
    recipe: Vec<Recipe>,
}

pub fn builtin_recipes() -> Vec<Recipe> {
    // The file ships inside the binary and is covered by tests, so a parse error is a build bug.
    toml::from_str::<RecipeFile>(RECIPES_TOML).expect("recipes.toml is valid").recipe
}

/// Everything needed to judge a folder: home, recipes and the user's own keep list.
#[derive(Debug, Clone)]
pub struct Catalog {
    pub home: PathBuf,
    pub recipes: Vec<Recipe>,
    /// The user's own "leave on this Mac" folders: (pattern under home, why).
    pub keep: Vec<(String, String)>,
}

impl Catalog {
    pub fn new(home: impl Into<PathBuf>) -> Self {
        Catalog { home: home.into(), recipes: builtin_recipes(), keep: Vec::new() }
    }

    /// Catalog for the current user (`$HOME`).
    pub fn for_user() -> Self {
        Self::new(std::env::var_os("HOME").expect("HOME is set"))
    }

    /// Path under home without the home prefix; paths outside home stay absolute.
    pub fn rel_of(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home) {
            Ok(rel) if !rel.as_os_str().is_empty() => rel.to_string_lossy().into_owned(),
            _ => path.to_string_lossy().into_owned(),
        }
    }

    /// `~/x` for paths under home.
    pub fn show(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home) {
            Ok(rel) if !rel.as_os_str().is_empty() => format!("~/{}", rel.to_string_lossy()),
            _ => path.to_string_lossy().into_owned(),
        }
    }

    pub fn classify(&self, rel: &str) -> (Verdict, String) {
        if let Some((_, why)) = self.keep.iter().find(|(p, _)| fnmatch(p, rel)) {
            return (Verdict::Keep, why.clone());
        }
        for f in self.recipes.iter().flat_map(|r| &r.folder) {
            if let Some(v) = f.verdict {
                if fnmatch(&f.path, rel) {
                    return (v, f.note.clone());
                }
            }
        }
        for (pattern, verdict, why) in RULES {
            if fnmatch(pattern, rel) {
                return (*verdict, why.to_string());
            }
        }
        (Verdict::Move, "Quit the app that uses it first.".into())
    }

    /// (app that uses the folder, regex on command lines that means that app is open).
    pub fn owner(&self, rel: &str) -> (String, Option<String>) {
        for r in &self.recipes {
            if let Some(f) = r.folder.iter().find(|f| fnmatch(&f.path, rel)) {
                return (f.app.clone().unwrap_or_else(|| r.app.clone()), r.running.clone());
            }
        }
        let name = rel.rsplit('/').next().unwrap_or(rel).trim_start_matches('.');
        if rel.starts_with("Library/Application Support/") {
            return (name.to_string(), Some(format!("/{}\\.app/", regex::escape(name))));
        }
        (name.to_string(), None)
    }

    /// Scan patterns: the generic list plus every recipe folder.
    pub fn scan_patterns(&self) -> Vec<String> {
        let mut out: Vec<String> = SCAN.iter().map(|s| s.to_string()).collect();
        out.extend(self.recipes.iter().flat_map(|r| r.folder.iter().map(|f| f.path.clone())));
        out
    }
}

pub fn never_read(rel: &str) -> bool {
    NEVER_READ.iter().any(|p| fnmatch(p, rel))
}

pub fn never_move(rel: &str) -> bool {
    NEVER_MOVE.iter().any(|p| fnmatch(p, rel))
}

pub fn skipped(rel: &str) -> bool {
    SKIP.iter().any(|p| fnmatch(p, rel))
}

/// Shell-style match like Python's fnmatch: `*` matches anything (including `/`), `?` one character.
pub fn fnmatch(pattern: &str, s: &str) -> bool {
    let (p, s): (Vec<char>, Vec<char>) = (pattern.chars().collect(), s.chars().collect());
    let (mut pi, mut si) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while si < s.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some((pi, si));
            pi += 1;
        } else if let Some((sp, ss)) = star {
            pi = sp + 1;
            si = ss + 1;
            star = Some((sp, ss + 1));
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat() -> Catalog {
        Catalog::new("/Users/x")
    }

    #[test]
    fn fnmatch_basics() {
        assert!(fnmatch("Library/Containers/*", "Library/Containers/a/b"));
        assert!(fnmatch(".*", ".gradle"));
        assert!(!fnmatch(".*", "Library"));
        assert!(fnmatch("CallHistory*", "CallHistoryDB"));
        assert!(fnmatch("a?c", "abc") && !fnmatch("a?c", "ac"));
        assert!(fnmatch("*", "") && !fnmatch("a", ""));
        assert!(fnmatch("a*b*c", "aXXbYYc") && !fnmatch("a*b*c", "aXXbYY"));
    }

    // Ported from the prototype's self-test.
    #[test]
    fn prototype_rules() {
        let c = cat();
        assert_eq!(c.classify("Library/Group Containers/group.net.whatsapp.WhatsApp.shared").0, Verdict::Keep);
        assert_eq!(c.classify("Library/Containers/net.whatsapp.WhatsApp").0, Verdict::Keep);
        assert_eq!(c.classify("Library/Containers/com.docker.docker").0, Verdict::App);
        assert_eq!(c.classify("Library/Application Support/Claude/vm_bundles").0, Verdict::Move);
        assert_eq!(c.classify("Library/Application Support/Claude").0, Verdict::Keep);
        assert_eq!(c.classify(".gradle").0, Verdict::Move);
        assert_eq!(c.classify(".claude").0, Verdict::Keep);
        assert_eq!(c.classify("Library/Application Support/Google").0, Verdict::Keep);
        assert_eq!(c.classify("Library/Application Support/com.apple.sharedfilelist").0, Verdict::Keep);
        assert_eq!(c.classify("Library/Caches/com.apple.Safari").0, Verdict::Keep);
        assert_eq!(c.classify("Library/Developer/CoreSimulator").0, Verdict::Keep);
        assert_eq!(c.classify("Library/Mobile Documents").0, Verdict::Keep);
        assert_eq!(c.classify("Library/CloudStorage").0, Verdict::Keep);
        assert_eq!(c.classify(".Trash").0, Verdict::Keep);
        assert_eq!(c.classify("Documents").0, Verdict::Keep);
        assert_eq!(c.classify(".hermes").0, Verdict::Move);
        let mut c = cat();
        c.keep.push(("ServerBackups".into(), "keys".into()));
        assert_eq!(c.classify("ServerBackups"), (Verdict::Keep, "keys".into()));
    }

    #[test]
    fn owners() {
        let c = cat();
        assert!(c.owner(".gradle").0.starts_with("Android"));
        assert_eq!(c.owner("Library/Application Support/Notion"), ("Notion".into(), Some(r"/Notion\.app/".into())));
        assert_eq!(c.owner("Library/Application Support/Claude/vm_bundles").0, "Claude (Cowork)");
        assert_eq!(c.owner("Library/Developer/Xcode/Archives").0, "Xcode");
        assert_eq!(c.owner(".hermes"), ("hermes".into(), None));
    }

    #[test]
    fn rel_and_show() {
        let c = cat();
        assert_eq!(c.rel_of(Path::new("/Users/x/.gradle")), ".gradle");
        assert_eq!(c.rel_of(Path::new("/opt/x")), "/opt/x");
        assert_eq!(c.rel_of(Path::new("/Users/xy")), "/Users/xy");
        assert_eq!(c.show(Path::new("/Users/x/.gradle")), "~/.gradle");
    }

    // Never read other apps' private folders or protected Apple data (each read makes macOS ask).
    #[test]
    fn never_read_containers() {
        assert!(!SCAN.iter().any(|g| g.contains("Containers")));
        assert!(skipped("Library/Application Support/AddressBook"));
        assert!(skipped("Library/Application Support/CallHistoryTransactions"));
        assert!(never_read("Library/Containers/com.docker.docker"));
        assert!(never_read("Library/Group Containers/x"));
        assert!(!never_read("Library/Caches/x"));
        // Reading these makes macOS ask for Apple Music and media library access.
        assert!(skipped("Library/Application Support/com.apple.AMPLibraryAgent") && skipped("Library/Caches/com.apple.Music"));
        assert!(!skipped("Library/Caches/Google"));
    }

    #[test]
    fn no_recipe_moves_protected_data() {
        for f in cat().recipes.iter().flat_map(|r| &r.folder) {
            if f.verdict == Some(Verdict::Move) {
                assert!(!never_move(&f.path), "{} is protected", f.path);
            }
        }
        assert!(never_move("Documents/x") && never_move("Library/Containers/a"));
        assert!(!never_move(".gradle"));
    }

    #[test]
    fn recipes_parse_and_regexes_compile() {
        let rs = builtin_recipes();
        assert!(!rs.is_empty());
        let mut ids: Vec<_> = rs.iter().map(|r| r.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), rs.len(), "duplicate recipe ids");
        for r in &rs {
            if let Some(p) = &r.running {
                regex::Regex::new(p).unwrap_or_else(|e| panic!("{}: {e}", r.id));
            }
            assert!(!r.folder.is_empty(), "{} has no folders", r.id);
        }
    }
}
