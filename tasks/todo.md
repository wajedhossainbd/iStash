# Stowaway — task list (v2, 10 Nov 2026)

Sizes: S = 1–2 files, M = 3–5 files. Every task must meet the quality bar in `tasks/plan.md`
(tests + `scripts/check.sh` green, acceptance shown). Checkpoints need the owner's review.

## Phase 0 — Design ✅
- [x] **D1** Three visual directions (owner chose Patch Bay)
- [x] **D2** Design system (`design/tokens.json`, `design/system.md`)
- [x] **D3** Clickable prototype (`design/prototype/`)

## Phase 1 — Core engine ✅
- [x] **T1** Workspace scaffold · [x] **T2** Rules + scanner · [x] **T3** In-use detection
- [x] **T4** Move + bring back with journal · [x] **T5** Crash recovery · [x] **T6** Drive map + eject guard
- [x] **T7** Metrics + memory forecast · [x] **T8** App recipes

### Checkpoint A ✅ passed 10 Nov 2026
- [x] `scripts/check.sh` on merged main: fmt, clippy, 51/51 tests
- [x] Real drive: 301 MB test folder (203 files + nested link) moved to "Mac 1TB SSD" in 4 s, identical through the
  link (SHA-256 of every file), listed by `drives`, brought back identical, drive tidied
- [x] SIGKILL during a real move at 0.4 s, 1.5 s and 2.8 s: `recover` undid each, data identical
- [x] Design reviewed and chosen by the owner
- Found: an undone move leaves empty `Stowaway/Library/Caches` folders on the drive (cosmetic) → fixed in T16

## Phase 2 — v0.1 "Offload" (free core as an app)

Status 14 Nov 2026: shipped as iStash 0.1.12. T16 and T19–T23 done, T25 mostly done (DMG, signed updates, install
script; not notarised). T17, T18 and T24 are open: the app hosts the engine itself instead of a login agent. Details:
`HANDOFF.md`, `CHANGELOG.md`.

- [x] **T16: Mover hardening** — fault-injection tests: drive detached mid-copy, target full, permission denied, source
  changed during copy, sparse files stay sparse (Docker.raw-style), hard links, xattrs/ACLs, sockets/FIFOs skipped with
  a report, long/Unicode paths; remove empty parent folders after undo. *Accept:* each case has a test on a temporary
  disk image and leaves data in exactly one valid place. *Size:* M. *Deps:* none.
  - Done 11 Nov: `crates/stowaway-core/tests/faults.rs` (9 tests). Defects found and fixed: sparse files were copied
    at full size (1 GB → 1 GB, now 4 KB); one socket or FIFO failed the whole move (now left out and listed in
    `Plan::skipped`); hard links were split; an unreadable file was reported as "macOS blocked the drive"; a drive
    detached mid-copy left a partial copy that was journaled as undone (now kept open until recovery removes it);
    a deny-delete ACL or `uchg` lock stopped the original from being deleted; one stuck op aborted all recovery.
  - Known limit: folders with paths longer than 1,024 bytes are refused (copyfile stops at PATH_MAX), cleanly.
  - Known gap: a write between the verify and the switch is caught only by the second "in use" check.
- [ ] **T17: State store + manifests** (manifests, re-check and backfill done: Record now / `stowaway-cli record`, 0.1.27; `state.db` deferred to T26, nothing reads it before then) — SQLite `state.db`; write a manifest (path, size, mtime, BLAKE3) for every
  stowed folder at move time; backfill for folders already linked. *Accept:* manifest matches a fresh hash in tests;
  migration test. *Size:* M. *Deps:* T16.
- [ ] **T18: Agent** — `crates/stowaway-agent`: launchd login agent with its own bundle id, Unix-socket API with token,
  owns journal + state, runs recover on login, forecast notifications, eject guard. *Accept:* app/CLI talk only through
  the socket; forced-threshold notification test; < 25 MB RAM, < 1% CPU idle measured. *Size:* M. *Deps:* T17.
- [x] **T19: Tauri shell** — window per `design/prototype` (vibrancy sidebar, merged draggable title bar), routing,
  tokens, light/dark, onboarding screen that asks for drive access once. *Accept:* opens, navigates, VoiceOver reads
  the sidebar. *Size:* M. *Deps:* T18.
- [x] **T20: Overview screen** — live gauges, charts, memory forecast card. *Accept:* matches prototype; app < 150 MB.
  *Deps:* T19.
- [x] **T21: Offload screen** — list with verdicts and real app icons, Move / Quit / Bring back, progress, refusals,
  dry-run preview dialog. *Accept:* a real folder moved end to end from the UI. *Deps:* T19.
- [x] **T22: Drives screen (cable map)** — apps patched to drives, missing-drive state, eject warning. *Accept:* reacts
  to a disk image attach/detach within 2 s. *Deps:* T19.
- [x] **T23: Menu-bar popover + Settings.** *Deps:* T20.
- [ ] **T24: Recipe verification, first set** — Docker, Xcode (DerivedData, Device Support), Ollama, LM Studio, Claude
  Cowork VM, Android SDK/emulator, npm/pnpm/Gradle/Cargo caches: move, launch, use, bring back on the owner's Mac; mark
  verified or unsupported with the reason. *Accept:* `docs/recipes.md` table with evidence. *Deps:* T21.
- [ ] **T25: Packaging** — DMG, Tauri updater, signing/notarization hooks (ad-hoc until a Developer ID exists),
  migration from the Python prototype's links (`Mac Offload/…`). *Accept:* installs on a clean user account; updates
  0.1.0 → 0.1.1; prototype links adopted without moving data. *Deps:* T19–T23.

### Checkpoint B — v0.1 on the owner's Mac for a week
- [ ] Real folders moved with the app (e.g. ~/.hermes, Claude Cowork VM); eject warning seen; forecast notification
  seen in a forced test; no data incidents; budgets measured; owner approves the UI.

## Phase 3 — v0.2 "Autopilot" (first Pro features)

- [ ] **D4: Design for Pro screens** — Autopilot (target, rules, plan preview, activity log), Travel mode (pack list,
  progress), Timeline, Memory relief, licence/trial states. *Deps:* Checkpoint B.
- [ ] **T26: Timeline** — daily size snapshots per folder/app in `state.db`; "what grew" over 1/7/30 days.
  *Accept:* snapshot + diff tests; screen per D4. *Size:* M.
- [ ] **T27: Planner** — pure function: (state, rules, drives, running apps) → ordered plan with reasons; used for
  dry-run and execution. *Accept:* table-driven tests incl. "never plan a folder in use / unverified recipe / unfit
  drive / low battery". *Size:* M. *Deps:* T17.
- [ ] **T28: Autopilot** — "keep N GB free" + per-folder rules + schedule; agent executes planner output, logs every
  action, notifies, one-click undo; global pause. *Accept:* simulated low-space test stows the planned folders and
  stops at the target; nothing runs while its app is open. *Size:* M. *Deps:* T27, T18.
- [ ] **T29: Travel mode** — pack list of apps; "Pack" brings their data back if it fits (plan shown first) and holds
  the rest; restow on reconnect. *Accept:* disk-image test: pack → detach → apps' folders are local → attach → restowed
  identical. *Size:* M. *Deps:* T27.
- [ ] **T30: Launch guard** — when an app launches and its drive is missing: notification with Connect / Pack /
  Quit choices. *Accept:* NSWorkspace launch event test with a detached image. *Size:* S. *Deps:* T18.
- [ ] **T31: Memory relief** — per-app growth trend (leak detection), forecast with confidence, "Restart app" action
  (quit + relaunch), optional rule. *Accept:* forecast tests on recorded traces; restart tested on a sample app. *Size:* M.

### Checkpoint C — two weeks with Autopilot on
- [ ] Zero data incidents, zero "disk full" warnings, activity log reviewed with the owner.

## Phase 4 — v0.3 "Integrity"
- [ ] **T32: Drive fitness** — write/read/small-file speed test, format, encryption, free space, S.M.A.R.T. where
  available → verdict per use ("too slow for Docker"). *Accept:* slow-image test yields "unfit"; planner respects it.
- [ ] **T33: Scrub** — re-hash stowed files against manifests, incremental, on power only, pausable; report missing or
  changed files. *Accept:* detects an injected bit-flip and a deleted file on a test image.
- [ ] **T34: Mirror** — second copy of chosen stowed folders on another drive; restore from mirror. *Accept:* restore
  after Scrub failure returns identical data.

### Checkpoint D — integrity proven on test drives

## Phase 5 — v0.4 "Analyze & Clean" (outline; detail after Checkpoint C)
Storage map, developer-aware cleanup to Trash with dry-run + undo log, cold storage for stale projects, uninstaller
with leftovers, CLI polish + Shortcuts actions.

## Phase 6 — v1.0 launch (outline)
Offline licence + 14-day trial, merchant of record, Developer ID + notarization, website with install help, docs,
privacy policy, private beta (10–20 testers), Homebrew cask, public launch.
