# AGENTS.md — working on iStash

Instructions for coding agents (Claude Code, Codex and others). Read [`HANDOFF.md`](HANDOFF.md) for the current state
and the lessons behind these rules.

## What iStash is

A macOS app (Apple Silicon, macOS 14+) that moves heavy app data to external drives and leaves a link, so apps keep
working. It also watches memory, disks and network, runs speed tests, cleans temporary files, and lives in the menu
bar and on a screen edge. Formerly "Stowaway"; internal names still say stowaway (see "Do not rename").

## Map

| Path | What |
|---|---|
| `crates/stowaway-core/` | Rust engine: `scan`, `rules` (what may move), `mover` (copy, verify, link, journal, recover), `drives`, `diskarb` (eject guard), `procs`, `metrics`, `forecast`. |
| `crates/stowaway-cli/` | Command line front end to the engine. |
| `app/src-tauri/src/` | Tauri 2 shell. `engine.rs` (scan, move, bring back, eject), `mem.rs` (live metrics), `away.rs` (missing-drive watcher), `net.rs` (connection, speed test, Speedtest server list), `tune.rs` (Focus, cleaner, free memory), `quick.rs` (menu-bar toggles), `widget.rs` (edge widget), `login.rs` (Open at login), `main.rs` (windows, tray, commands). |
| `app/ui/src/` | UI, TypeScript, no framework: HTML strings rendered with morphdom. `api.ts` (types + sample data), `real.ts` (calls the shell), `screens.ts` (every screen), `ui.ts` (components, icons), `main.ts` (routing, actions), `app.css`. |
| `design/` | `tokens.json` → `node design/tokens.mjs` → `prototype/tokens.css` (checks WCAG AA). `system.md` has the design rules. |
| `docs/screenshots/` | README pictures, taken on sample data. |
| `.github/workflows/release.yml` | Test, build, sign and publish a release on every push to `main`. |

## Commands

```bash
scripts/check.sh                              # engine: fmt, clippy -D warnings, tests
cd app/ui && npx tsc --noEmit && npm test     # UI types and tests
cd app/src-tauri && cargo test                # shell tests
cd app/ui && npm run dev                      # UI on sample data at http://127.0.0.1:1420 (add ?mock=loading|empty|error|noaccess|unreadable)
cd app/ui && npm run app:build                # iStash.app and DMG
```

Run all three test suites before committing.

## Conventions

- **Smallest complete change.** Reuse what exists (helpers in `ui.ts`, patterns in `screens.ts`, engine functions)
  before adding code. No new dependency for a few lines.
- **Every new command or screen** goes through `Api` in `api.ts`: add the method there, a sample version in the mock
  (`createMock`), and the real one in `real.ts`. Screens never call the shell directly.
- **Popover and edge widget** windows run `createReal` in a light mode (`light` in `real.ts`) that skips scanning,
  jobs and schedules. Keep heavy work in the main window.
- **Validate at the boundary.** Rust commands check every path, URL and host they receive before using it
  (see `net.rs`, `tune.rs`).
- **Shortcuts are marked** with `// shortcut: <limit>, <when to upgrade>`.
- **Commit messages** say what changed and why, in plain words. Changelog lines go under `## Unreleased`.

## Design rules

- Tokens only; no hard-coded colours outside the floating panels' fixed dark palette (`.pop`, `.edge`).
- Fonts: Geist for text and numbers, Geist Mono for paths. Numbers are tabular.
- Glass surfaces, readable in light and dark. Floating windows (menu-bar panel, edge widget) use macOS vibrancy
  (`windowEffects`) and `theme: Dark`, never `backdrop-filter` in a transparent window.
- Text never wraps in tiles, rows or the widget; long values end with an ellipsis.
- Help text is one short line, or none if the control says enough. Detail belongs in the sheet shown before an action.
- No decorative mono labels, glow, emoji or invented numbers (full list in `design/system.md`).
- Motion: transform, opacity of overlays and stroke offsets only; respect reduced motion; nothing may start invisible
  (WebKit pauses animations in background windows).

## Safety rules (non-negotiable)

- Never move, delete or modify the owner's real folders or drives in tests. Use `hdiutil` disk images or throwaway
  folders, and clean them up.
- The engine never removes an original before the copy is verified, and journals every step.
- Never read `~/Library/Containers`, `~/Library/Group Containers`, macOS's own `com.apple.*` Library folders,
  ~/Documents, ~/Desktop or Photos.
- No root helper and no admin prompts in normal use.
- Anything that deletes for good (Clean, Empty Trash) shows what it will remove and asks first.
- The only network calls are update checks and speed tests the user starts. Don't add others without the owner.

## Releasing

- A push to `main` publishes a release to every user. Only push to `main` when the owner says so; otherwise commit on
  a branch.
- `main` is protected: no force-pushes, no deletion.
- Before releasing: add the changes under `## Unreleased` in `CHANGELOG.md`. After the release is published, rename
  that heading to the version (`## 0.1.N — date`) and update the release notes if needed.
- Local builds report version 0.1.0; don't use their "Update" button to test unreleased work.

## Do not rename

`bd.wajed.stowaway` (bundle id), `~/Library/Application Support/Stowaway/`, the `Stowaway/` folder on each drive,
`localStorage` keys `stowaway.*`, crate and binary names. They hold settings, the login item, privacy grants and
users' moved data. A rename needs a migration.
