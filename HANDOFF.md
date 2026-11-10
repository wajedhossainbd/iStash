# Stowaway — handoff (11 Nov 2026)

Read this first, then `tasks/plan.md` (roadmap v2, quality bar, decisions) and `tasks/todo.md` (tasks with checks).

## Where things stand
| Phase | State |
|---|---|
| 0 Design (D1–D3, "Patch Bay") | ✅ merged — `design/system.md`, `design/tokens.json`, `design/prototype/` |
| 1 Rust core engine (T1–T8) | ✅ merged — `crates/stowaway-core`, `crates/stowaway-cli`, 51 tests |
| Checkpoint A | ✅ passed 10 Nov on a real drive (details in `tasks/todo.md`) |
| 2 v0.1 "Offload" (T16–T25) | ⏳ **approved and briefed, not started** — no code written yet |
| 3–6 | planned (`tasks/plan.md`) |

`main` is pushed to the private repo `wajedhossainbd/stowaway`. Two local work branches exist and are fully merged
into `main` (safe to delete): `claude/core-engine-96be1d`, `claude/design-phase0-b1b6b6`, with worktrees under
`.claude/worktrees/`. Two Claude Desktop sessions ("Stowaway Rust core engine", "Stowaway UI/UX design") received the
Phase 2 briefs below and stopped on a usage limit before doing any work.

## Run and check
```bash
scripts/check.sh                                   # fmt + clippy -D warnings + all tests
cargo build --release
target/release/stowaway-cli --help                 # scan, who-uses, quit, move, bring-back, recover, drives, status, guard
target/release/stowaway-cli move <folder> "<Drive name>"        # dry run; add --yes to act
open design/prototype/index.html                   # clickable UI prototype
```
Moved folders go to `/Volumes/<drive>/Stowaway/<path under home>`; the journal is
`~/Library/Application Support/Stowaway/journal.jsonl`.

## Next: Phase 2, two parallel work streams
They touch different files, so they can run at the same time. Start each from an up-to-date `main`.

**Stream A — engine (owns `crates/**`, `docs/agent-api.md`, `scripts/`, root `Cargo.toml`)**
1. **T16** mover hardening: fault-injection tests (drive detached mid-copy, target full, permission denied, source
   changed during copy, sparse files stay sparse, hard links, xattrs/ACLs, sockets/FIFOs skipped with a report,
   long/Unicode paths) and fix: an undone move leaves empty `Stowaway/Library/Caches` folders on the drive.
2. **T17** SQLite `state.db` + a manifest (path, size, mtime, BLAKE3) per stowed folder; backfill for linked folders.
3. **T18** `crates/stowaway-agent`: launchd login agent with its own bundle identity, Unix-socket API with a per-user
   token, owns journal + state, recover on login, forecast notifications, eject guard. Document the API in
   `docs/agent-api.md`, shaped to serve the screens in `design/prototype/data.js` / `screens.js`.
   Budget: < 25 MB RAM, < 1% CPU idle (measure it).

**Stream B — app UI (owns `app/**`, `design/**`)**
1. **T19** Tauri 2 shell (`app/src-tauri` + `app/ui`, TypeScript): window per the prototype (vibrancy sidebar, merged
   title bar that still drags, min size, remembers frame), routing, tokens, light/dark, onboarding. Give
   `app/src-tauri/Cargo.toml` its own empty `[workspace]` table until integration.
2. **T20–T23** Overview, Offload (with dry-run preview dialog), Drives (cable map), menu-bar popover + Settings, with
   the designed empty/loading/error states.
3. All data behind one typed interface `app/ui/src/api.ts` with a mock built from `design/prototype/data.js`; commit it
   early. Budget: app < 150 MB.

**Then (one stream):** wire `api.ts` to the agent socket, **T24** verify recipes with the real apps, **T25** packaging
(DMG, updater, adopt the prototype's existing links), then **Checkpoint B** (owner uses v0.1 for a week).

## Rules that must not be broken
- Tests use temporary disk images (`hdiutil`) only. Never move, delete or modify the owner's real folders or drives,
  and never install an agent into the real `~/Library/LaunchAgents`, without the owner asking.
- The original is never removed before the copy is verified by content hash; every step is journaled.
- Never read `~/Library/Containers` or `~/Library/Group Containers` (each read triggers a macOS permission prompt,
  and sandboxed apps cannot follow a link anyway). Never touch ~/Documents, ~/Desktop, Photos.
- No root helper. exFAT targets are refused.
- Design: follow the de-AI rules in `design/system.md` (no decorative mono labels, no glow, real icons, real numbers).

## Lessons from the prototype (already paid for)
- A bare background binary is silently refused access to external drives; a helper with its own bundle id and stable
  signature gets one prompt. Rebuilding/re-signing the helper makes macOS ask again.
- A tool that lives inside an app bundle is not that app (python inside Xcode.app ≠ Xcode): only apps that are open
  as GUI apps count as apps; everything else is a program.
- Check "in use" again right before switching the link; quit apps normally, SIGTERM programs.
- Copying a sparse file (Docker.raw) naively can expand it to its full apparent size.
- Slow/cheap drives fail silently: the owner's old card wrote at 4 MB/s and lost 424 files (reason for Phase 4).
- A page that redraws on every poll replaces buttons under the mouse: redraw only on change.

## Open decisions for the owner (defaults in use until changed)
1. Free vs Pro split and price ($29 once, launch $19, 14-day trial, no account).
2. Apple Developer ID ($99/yr): needed before anyone else can install cleanly.
3. Payment provider (Paddle or Lemon Squeezy; check payouts to Bangladesh).
4. Closed source or open core (affects when the repo can go public; review `research/` before any public release).
5. First recipes to verify: Docker, Xcode, Ollama/LM Studio, Claude Cowork VM, Android SDK, npm/pnpm/Gradle/Cargo.

## Related, outside this repo
- **Prototype "System Dashboard"** (Python + Swift/WebKit): `Development/Projects/Other-Projects/system-dashboard`,
  local git repo, no remote. Installed in `/Applications`, with a login agent (`local.system-dashboard`) that serves
  http://127.0.0.1:8765 and sends memory warnings. Stowaway replaces it; T25 adopts its links.
- **Owner's Mac layout:** internal SSD for apps; "Mac 500 SSD" for projects and dev kits (Android SDK, Flutter, nvm,
  caches, linked from home); "Mac 1TB SSD" for `Development/` (this repo, Docker disk, Gradle, Rust, Xcode device
  support); "Jetdrive" for media only; "Storejet" is exFAT (files only).
- **Launch agents on this Mac:** `bd.wajed.storage-guard-check` (hourly) and `-clean` (weekly) run
  `~/bin/storage-guard`; `bd.wajed.downloads-mover` (daily) runs `~/bin/downloads-mover`; `~/bin/offload` moves a
  folder and links it. Logs: `~/Library/Logs/storage-guard.log`.
- **TidyMac check (not done):** if the owner drops its DMG in `research/competitors/TidyMac/`, inspect it without
  running it (signature, notarization, entitlements, contents) before they install it. `*.dmg` there is git-ignored.
