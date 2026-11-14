# iStash — handoff (14 Nov 2026)

Read this first. Then [`AGENTS.md`](AGENTS.md) for how to work in this repository, [`CHANGELOG.md`](CHANGELOG.md) for
what each release contains, and [`tasks/plan.md`](tasks/plan.md) / [`tasks/todo.md`](tasks/todo.md) for the roadmap.

iStash was called **Stowaway** until 14 Nov 2026 (release 0.1.11). Code, crates, data folders and the bundle
identifier still use the old name on purpose; see "Names that must not change" below.

## Where things stand

| Area | State |
|---|---|
| Rust engine (`crates/stowaway-core`, `crates/stowaway-cli`) | Done for v0.1: scan, rules, mover with journal and crash recovery, drives, eject guard, metrics, forecast. Hardened with fault-injection tests (T16). |
| App (`app/`, Tauri 2) | Shipping. Overview, Offload, Drives (cable map and per-drive pages), Memory (with Focus), Internet (speed test), Clean, Settings, menu-bar panel, edge widget. Runs the engine itself; there is no separate login agent yet. |
| Releases | Published automatically from `main`. Latest: **0.1.12** (menu-bar control centre). Installed copies update themselves. |
| Repository | [`wajedhossainbd/iStash`](https://github.com/wajedhossainbd/iStash), public, GPL 3.0. `main` is protected against force-pushes and deletion (owner included); normal pushes are allowed. |
| Owner's Mac | iStash 0.1.12 in `/Applications/iStash.app`, Open at login on. 28 folders live on "Mac 1TB SSD" and "Mac 500 SSD": 5 moved by iStash, the rest linked by hand before it existed. |

Work branch: `claude/determined-cerf-4bfe99` (worktree `.claude/worktrees/core-engine-96be1d`) is level with `main`.
Pull request 2 (menu-bar panel) is merged.

## Build, run, release

```bash
scripts/check.sh                          # engine: fmt, clippy -D warnings, all tests
cd app/ui && npm install
npm test                                  # UI tests
(cd ../src-tauri && cargo test)           # shell tests
npm run dev                               # UI alone on sample data: http://127.0.0.1:1420
npm run app:build                         # app/src-tauri/target/release/bundle/macos/iStash.app (+ DMG)
```

A local build that should also produce the signed update package needs the signing key:
`TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/stowaway-updater.key)" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" npm run app:build`.

**Release:** add lines under `## Unreleased` in `CHANGELOG.md`, then push to `main`.
`.github/workflows/release.yml` tests, builds version `0.1.<run number>`, and publishes `iStash.dmg`,
`iStash.app.tar.gz` (+ signature), `latest.json` and `install.sh`. Release notes come from the changelog section for
that version, or from `Unreleased`. Pushes that only change `*.md`, `docs/` or `LICENSE` don't release. After a
release, rename `Unreleased` to the version that was published.

**Signing key:** private key at `~/.tauri/stowaway-updater.key` on the owner's Mac and in the repository secret
`TAURI_SIGNING_PRIVATE_KEY`; public key in `app/src-tauri/tauri.conf.json`. If the key is lost, installed copies can
never accept another update. Back it up.

**Install for anyone:** `curl -fsSL https://github.com/wajedhossainbd/iStash/releases/latest/download/install.sh | sh`.

## Names that must not change

Changing any of these breaks existing installs or moved data:
- Bundle identifier `bd.wajed.stowaway`: settings (WebKit storage), the login item, update checks and macOS privacy
  grants are tied to it.
- `~/Library/Application Support/Stowaway/journal.jsonl`: the move journal used for recovery and Bring back.
- `/Volumes/<drive>/Stowaway/<path under home>`: where moved folders live; every link points into it.
- `localStorage` keys `stowaway.*`, crate names `stowaway-core` / `stowaway-cli`, binary `stowaway-app`.

## Rules that must not be broken

- Tests use temporary disk images (`hdiutil`) or throwaway folders only. Never move, delete or modify the owner's
  real folders or drives unless the owner asks for that folder.
- The original is never removed before the copy is verified; every step is journaled.
- Never read `~/Library/Containers` or `~/Library/Group Containers`, and never touch ~/Documents, ~/Desktop, Photos.
  The scan also skips macOS's own `Library/Application Support/com.apple.*` and `Library/Caches/com.apple.*` (reading
  them triggers privacy prompts, such as Apple Music access).
- No root helper. exFAT targets are refused.
- Don't push to `main` without the owner's go-ahead: every push publishes a release to everyone.

## Lessons (paid for already)

- A bare background binary is refused access to external drives; a bundle with its own id gets one prompt.
  Re-signing makes macOS ask again, which is why ad-hoc signed updates can re-prompt.
- An app updated in place keeps its old file name: copies first installed as `Stowaway.app` keep that name. The
  install script removes an old `Stowaway.app` only if its bundle id is this app's.
- A local build reports version 0.1.0, so its Settings offers the latest release as an "update" and installing it
  replaces local, unreleased work. Release first, then update.
- Web blur (`backdrop-filter`) inside a transparent window flickers on every redraw, and macOS re-traces the shadow.
  Use the window's own vibrancy (`windowEffects`) and force `theme: Dark` for floating panels so light text stays
  readable.
- Quit from the Dock or ⌘Q is answered in `stay.rs` (`applicationShouldTerminate:`, added at launch because the
  window library doesn't implement it): iStash hides to the menu bar. A real quit is the menu-bar "Quit iStash".
  Scripts must use `pkill`, not an AppleScript quit.
- A window that isn't in front gets no hover events; the edge widget follows the pointer from Rust instead.
- `pmset displaysleepnow` doesn't lock unless "require password immediately" is set; Lock uses
  `SACLockScreenImmediate` from `login.framework`, as the Apple menu does.
- Freeing memory without a password works by applying memory pressure (`memory_pressure`), which pushes idle memory
  to swap. `purge` needs an administrator.
- macOS can't cap another app's memory; Focus lowers other apps' priority (`taskpolicy -b`) instead.
- A page that redraws on every poll replaces controls under the mouse; morphdom keeps unchanged nodes.

## Known gaps

- No login agent (T18): everything runs inside the app, so drive warnings and schedules need iStash running
  (Open at login covers this).
- Not notarised: needs an Apple Developer ID. Downloads need the quarantine flag removed (the installer does it).
- Apple Silicon only.
- No state database or per-file manifests yet (T17), so there's no integrity re-check of moved data.
- Recipes for specific apps aren't verified one by one (T24).
- Folders linked by hand before iStash can't be brought back by it.
- The Speedtest server list and servers belong to Ookla; iStash uses their public endpoints, as speedtest-cli does.

## Open decisions for the owner

1. Apple Developer ID ($99/yr), for signed, notarised builds.
2. Free vs paid features and price (see `tasks/plan.md`), now that the code is GPL 3.0.
3. Intel Mac builds.
4. Whether to rename the internal identifiers (would need a migration for existing users).

## Related, outside this repo

- **NetWatch** (owner's tool, http://192.168.0.122:8787): the reference for the speed test UI.
- **Prototype "System Dashboard"** (Python + Swift): `Development/Projects/Other-Projects/system-dashboard`; iStash
  replaces it.
- **Owner's drives:** "Mac 500 SSD" (projects, dev kits), "Mac 1TB SSD" (`Development/`, Docker, Gradle, Rust,
  Xcode device support), "Jetdrive" (media; slow), "Storejet" (exFAT, files only).
- **Launch agents on this Mac:** `bd.wajed.storage-guard-check` / `-clean` (`~/bin/storage-guard`),
  `bd.wajed.downloads-mover`.
