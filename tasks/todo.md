# Stowaway — task list

Sizes: S = 1–2 files, M = 3–5 files. Build order follows the dependency graph; checkpoints need the owner's review.
Prior art to port behaviour and tests from: `Development/Projects/Other-Projects/system-dashboard/system_dashboard.py`.

## Phase 0 — Design (UI/UX session) — output in `design/`

- [ ] **D1: Three visual directions** — 2–3 distinct directions (name, mood, palette, type, motion idea), each with one
  hero screen. Must not copy TidyMac's water metaphor or layout. *Accept:* owner picks one. *Size:* M. *Deps:* none.
- [ ] **D2: Design system** — tokens (colour light+dark, type scale with mono for numbers, spacing, radius, elevation,
  motion durations), components (sidebar/tab nav, gauge, chart, table row, segmented control, buttons, pills, dialog,
  banner, empty state). *Accept:* `design/tokens.json` + `design/system.md`; WCAG AA contrast. *Deps:* D1.
- [ ] **D3: Information architecture + key screens** — Overview, Offload (move/quit/bring back), Drives (dependency map),
  Memory forecast, Settings, onboarding (permissions explainer), menu-bar popover; empty/loading/error states.
  *Accept:* clickable HTML prototype in `design/prototype/` the app can mirror. *Deps:* D2.

## Phase 1 — Core engine (app-build session) — `crates/stowaway-core`, `crates/stowaway-cli`

- [ ] **T1: Workspace scaffold** — Cargo workspace, core + cli crates, CI script (`cargo test`, `clippy -D warnings`,
  `fmt --check`). *Accept:* `cargo test` green on an empty test. *Size:* S. *Deps:* none.
- [ ] **T2: Rules + scanner** — classify folders (move / app-setting / keep + reason), skip sandbox containers and
  protected Apple data, measure sizes in parallel. *Accept:* unit tests for every rule from the prototype's self-test;
  `stowaway-cli scan` lists folders ≥ 200 MB with verdicts in < 60 s on this Mac. *Size:* M. *Deps:* T1.
- [ ] **T3: In-use detection** — process table, open apps (LaunchServices), "who uses this folder", readable names,
  quit (graceful app quit; SIGTERM for programs). *Accept:* tests ported from prototype (python-in-Xcode is not Xcode,
  hermes launcher named "hermes"); CLI `who-uses <path>`. *Size:* M. *Deps:* T1.
- [ ] **T4: Move + bring back with journal** — copy (clonefile when same volume), verify count+bytes+BLAKE3 hash,
  rename aside, symlink, delete; reverse for bring back; refuse non-APFS/HFS targets, ~/Documents, sandbox paths.
  *Accept:* integration tests on a temp APFS disk image (hdiutil) incl. nested links; CLI `move`/`bring-back`.
  *Size:* M. *Deps:* T2, T3.
- [ ] **T5: Crash recovery** — replay the journal on start: finish or undo each half-done step. *Accept:* tests that kill
  the mover at every step and recover with identical data. *Size:* S. *Deps:* T4.
- [ ] **T6: Drive map** — moved folders → drive → apps; connected/missing; DiskArbitration unmount-approval hook that
  names the apps. *Accept:* CLI `drives` output; test with a disk image detached. *Size:* M. *Deps:* T4.
- [ ] **T7: Metrics + memory forecast** — CPU, memory pressure, swap, network rate, per-app CPU/RAM; forecast minutes
  until out-of-memory from pressure + swap trend + free disk. *Accept:* CLI `status`; forecast unit tests on recorded
  samples; agent-side cost < 1% CPU. *Size:* M. *Deps:* T1.
- [ ] **T8: App recipes** — TOML recipes (Docker via setting, Xcode DerivedData/Device Support, Ollama/LM Studio/Hugging
  Face models, npm/pnpm/Gradle/Cargo/Homebrew caches, Claude Cowork VM; MobileSync marked unsupported). *Accept:*
  recipes loaded and applied by scanner; tests per recipe. *Size:* M. *Deps:* T2, T3.

### Checkpoint A — see plan.md (engine proven + design reviewed)

## Phase 2 — App v0.1 "Offload" — `app/` (after Checkpoint A)

- [ ] **T9: Tauri shell** — window per design (vibrancy sidebar, merged title bar, draggable), routing, design tokens.
  *Accept:* app opens, navigates between empty screens, light/dark. *Deps:* D2, T1.
- [ ] **T10: Overview** — live gauges and charts from T7. *Accept:* matches prototype screen; < 150 MB RAM. *Deps:* T9, T7.
- [ ] **T11: Offload screen** — list with verdicts, Move / Quit / Bring back, progress, dialogs. *Accept:* moves a real
  folder end to end from the UI; refusals shown inline. *Deps:* T9, T4, T3, T8.
- [ ] **T12: Drives screen** — dependency map, missing-drive banners. *Accept:* matches design; reacts to unplug. *Deps:* T9, T6.
- [ ] **T13: Background agent** — launchd login agent with its own bundle id: memory forecast notifications, eject
  guard, recovery on login. *Accept:* forced-threshold test fires a notification; eject of a needed drive warns. *Deps:* T5–T7.
- [ ] **T14: Menu-bar popover** — memory, disk, drives at a glance. *Deps:* T10, T13.
- [ ] **T15: Packaging** — DMG, updater, signing hooks, install-help page. *Accept:* DMG installs on a clean user
  account; update from 0.1.0 → 0.1.1 works. *Deps:* T9–T14.

### Checkpoint B — see plan.md (v0.1 usable on the owner's Mac)

## Phase 3 — v0.2 "Analyze & Clean" (outline; detail after Checkpoint B)
Storage map, large/old files, rebuildable caches to Trash with dry-run + undo, developer-aware categories.

## Phase 4 — v0.3 "Apps" + launch (outline)
Uninstaller with leftovers to Trash, login items, landing page, notarized release, Homebrew cask, public repo.
