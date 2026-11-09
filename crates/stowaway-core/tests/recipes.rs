//! One check per app recipe: verdict, owner, "is it open" detection, and that the scanner applies it.

use std::collections::HashSet;
use std::path::Path;
use stowaway_core::procs::{parse_ps, users_of};
use stowaway_core::rules::{Catalog, Verdict};
use stowaway_core::scan;

const HOME: &str = "/Users/x";

fn apps(names: &[&str]) -> HashSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

#[test]
fn verdicts_and_owners() {
    use Verdict::*;
    let c = Catalog::new(HOME);
    let cases: &[(&str, Verdict, &str)] = &[
        ("Library/Containers/com.docker.docker", App, "Docker"),
        ("Library/Developer/Xcode/DerivedData", Move, "Xcode"),
        ("Library/Developer/Xcode/iOS DeviceSupport", Move, "Xcode"),
        ("Library/Developer/Xcode/watchOS DeviceSupport", Move, "Xcode"),
        ("Library/Developer/CoreSimulator", Keep, "Xcode"),
        (".ollama/models", Move, "Ollama"),
        (".lmstudio/models", Move, "LM Studio"),
        (".cache/lm-studio/models", Move, "LM Studio"),
        (".cache/huggingface", Move, "Hugging Face models"),
        (".npm", Move, "npm"),
        ("Library/pnpm/store", App, "pnpm"),
        ("Library/Caches/pnpm", Move, "pnpm"),
        (".gradle", Move, "Android builds (Gradle)"),
        (".cargo/registry", Move, "Rust"),
        (".cargo/git", Move, "Rust"),
        (".cargo", Move, "Rust"),
        ("Library/Caches/Homebrew", Move, "Homebrew"),
        ("Library/Application Support/Claude/vm_bundles", Move, "Claude (Cowork)"),
        ("Library/Application Support/Claude", Keep, "Claude"),
        ("Library/Application Support/MobileSync", Unsupported, "iPhone and iPad backups"),
    ];
    for &(rel, verdict, app) in cases {
        let (v, note) = c.classify(rel);
        assert_eq!(v, verdict, "{rel}");
        assert!(!note.is_empty(), "{rel} explains itself");
        assert_eq!(c.owner(rel).0, app, "{rel}");
    }
    assert!(c.classify("Library/pnpm/store").1.contains("store-dir"));
    assert!(c.classify("Library/Containers/com.docker.docker").1.contains("Disk image location"));
    assert!(c.classify("Library/Application Support/MobileSync").1.contains("Catalina"));
}

#[test]
fn open_app_detection_per_recipe() {
    let c = Catalog::new(HOME);
    let cases: &[(&str, &str, &[&str], bool)] = &[
        // (folder, running command line, open GUI apps, counts as in use)
        (".ollama/models", "/Applications/Ollama.app/Contents/Resources/ollama serve", &["Ollama"], true),
        (".ollama/models", "/Applications/Ollama.app/Contents/Resources/ollama serve", &[], false),
        (".ollama/models", "/opt/homebrew/bin/ollama serve", &[], true),
        (".lmstudio/models", "/Applications/LM Studio.app/Contents/MacOS/LM Studio", &["LM Studio"], true),
        (".lmstudio/models", "/Users/x/.lmstudio/bin/lms server start", &[], true),
        ("Library/Caches/Homebrew", "/opt/homebrew/Library/Homebrew/vendor/portable-ruby/bin/ruby /opt/homebrew/Library/Homebrew/brew.rb install x", &[], true),
        (".npm", "node /opt/homebrew/lib/node_modules/npm/bin/npm-cli.js install", &[], true),
        ("Library/Caches/pnpm", "node /opt/homebrew/bin/pnpm install", &[], true),
        (".gradle", "/usr/bin/java -cp gradle-launcher.jar org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.10", &[], true),
        (".cargo/registry", "/Users/x/.rustup/toolchains/stable/bin/cargo build", &[], true),
        ("Library/Developer/Xcode/DerivedData", "/Applications/Xcode.app/Contents/MacOS/Xcode", &["Xcode"], true),
        (".cache/huggingface", "/usr/bin/python3 train.py --cache /Users/x/.cache/huggingface/hub", &[], true),
        (".cache/huggingface", "/usr/bin/python3 train.py", &[], false),
    ];
    for &(rel, cmd, open, expect) in cases {
        let table = parse_ps(&format!("42 {cmd}\n"), 1);
        let got = !users_of(&c, &Path::new(HOME).join(rel), &table, &apps(open)).is_empty();
        assert_eq!(got, expect, "{rel} with `{cmd}` (open: {open:?})");
    }
}

#[test]
fn scanner_applies_recipes() {
    let home = std::env::temp_dir().join(format!("stowaway-recipes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    for (rel, bytes) in [
        (".ollama/models/blobs", 3_000_000),
        (".ollama/logs", 1_000_000),
        (".cache/huggingface/hub", 2_000_000),
        (".cargo/registry/cache", 1_500_000),
    ] {
        std::fs::create_dir_all(home.join(rel)).unwrap();
        std::fs::write(home.join(rel).join("f"), vec![0u8; bytes]).unwrap();
    }
    std::fs::create_dir_all(home.join("Library/Application Support/MobileSync/Backup")).unwrap();
    let items = scan::scan(&Catalog::new(&home), 500_000);
    let find = |show: &str| items.iter().find(|i| i.show == show).unwrap_or_else(|| panic!("{show} not listed"));
    let models = find("~/.ollama/models");
    assert_eq!((models.verdict, models.app.as_str()), (Verdict::Move, "Ollama"));
    assert!(models.size.unwrap() >= 3_000_000);
    let ollama = find("~/.ollama");
    assert!(ollama.size.unwrap() < 2_000_000, "inner recipe folder counted once, in the inner one");
    assert_eq!(find("~/.cache/huggingface").app, "Hugging Face models");
    assert_eq!(find("~/.cargo/registry").verdict, Verdict::Move);
    assert!(!items.iter().any(|i| i.show.contains("MobileSync")), "protected Apple data is not read");
    std::fs::remove_dir_all(home).unwrap();
}
