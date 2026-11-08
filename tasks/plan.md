# Implementation Plan: Stowaway

## Overview
Stowaway is a native macOS utility that gives a full Mac room to breathe: it **moves heavy app data to external drives
while the apps keep working**, knows **which app needs which drive**, **predicts the "out of application memory"
freeze** before it happens, and shows live CPU, memory, disk and network. Later it adds a storage map, cleanup and an
uninstaller to compete head-on with CleanMyMac and TidyMac. Research: `research/market.md`,
`research/tidymac-teardown.md`, `research/naming-and-emroze.md`.

**Positioning:** "Make room on your Mac. Keep every app working." Free core, no account, no tracking, no nags;
trust is the product (dry-run first, Trash not delete, an undo journal, never touch ~/Documents).

**Prior art in this repo family:** `Development/Projects/Other-Projects/system-dashboard` (Python + WebKit prototype).
It proves the move/link/verify/recover engine, in-use detection, sandbox rules and the memory warning. Port its
behaviour and its tests; do not port its code style or its Python dependency.

## Architecture Decisions
- **Rust, Tauri 2.** One Rust core shared by the app and a background agent; UI in TypeScript on Tauri's WebKit view.
  Why: the user chose Rust; the playful, data-dense UI (gauges, charts, storage map) is far easier in web tech than in
  Rust-native GUI kits; Tauri gives a small signed bundle, DMG builder, updater, tray/menu-bar, and vibrancy.
  Rejected: egui/iced/gpui (non-native look, weak accessibility), SwiftUI + Rust FFI (two languages for a solo dev).
- **Cargo workspace:** `crates/stowaway-core` (all logic, no UI), `crates/stowaway-agent` (launchd login agent),
  `app/` (Tauri: `src-tauri` + `ui`). The UI never touches the filesystem directly; it calls Tauri commands that call core.
- **No Python, no Xcode tools needed by users.** Everything ships inside the .app.
- **Background agent with its own bundle identity** (`dev.stowaway.agent`, hidden, LSUIElement) so macOS asks once,
  in Stowaway's name, for Removable Volumes access. Lesson from the prototype: a bare background binary is silently refused.
- **No root helper in v1.** Everything runs as the user. (TidyMac's root helper and DB "repairs" are a trust risk we avoid.)
- **Never read other apps' sandbox containers** (`~/Library/Containers`, `Group Containers`) or privacy-protected Apple
  data: each read triggers a permission prompt and that data can never be moved anyway.
- **Moves are journaled** (`~/Library/Application Support/Stowaway/journal.jsonl`): copy → verify (file count, bytes,
  then content hash) → rename original aside → link → delete original. Recovery on start finishes or undoes any step.
- **App recipes** (data-driven TOML in core): per app, which folders move, whether by link or by the app's own setting,
  how to detect it running, how to quit it, and known breakages (e.g. Docker: use its setting; MobileSync: link breaks
  since Catalina → unsupported). Unknown folders fall back to the generic rules.
- **Distribution:** Developer ID signed + notarized DMG, Tauri updater (self-update — a TidyMac gap), Homebrew cask.
  Until a Developer ID exists: ad-hoc builds for the owner's Mac only.
- **Design** comes from the UI/UX session (`design/`): its tokens and screens are the spec the app implements.

## Phases

### Phase 0 — Design (UI/UX session, parallel with Phase 1)
Design system, app structure, key screens, motion, empty/error states, menu-bar popover. Output in `design/`.

### Phase 1 — Core engine (no UI; parallel with Phase 0)
Scanner + rules, process/"in use" detection, move/bring-back with journal and recovery, drive map, metrics, memory
forecast. CLI (`stowaway-cli`) to exercise everything. Port and extend the prototype's tests.

### Checkpoint A — engine proven
All core tests pass; CLI moves a real test folder to an external APFS drive and back with content-hash verification;
killing the process mid-move and restarting recovers cleanly; design reviewed by the owner.

### Phase 2 — App v0.1 "Offload" (vertical slices through the UI)
App shell per design → Overview (live gauges) → Offload screen (move / quit / bring back) → Drives (dependency map) →
background agent with memory forecast + drive-eject warnings → menu-bar popover → signed DMG.

### Checkpoint B — v0.1 usable on the owner's Mac
Owner moves real folders (e.g. ~/.hermes, Claude Cowork VM) with the app; eject warning fires; memory forecast
notification fires in a forced test; DMG installs and updates itself.

### Phase 3 — v0.2 "Analyze & Clean"
Storage map (sunburst or bubbles), large/old files, rebuildable-cache cleanup to Trash with dry-run and undo log,
developer awareness (Xcode, node_modules, Docker, Homebrew, LLM models).

### Phase 4 — v0.3 "Apps" and public launch
Uninstaller with leftovers (to Trash), login items, landing page, notarized release, Homebrew cask, GitHub repo public.

## Risks and Mitigations
| Risk | Impact | Mitigation |
|---|---|---|
| A move breaks an app (Plex, MobileSync, DerivedData cases in research) | High | App recipes with "unsupported" verdicts; post-move health check (app launches, folder readable); one-click bring back; journal |
| Drive unplugged while an app writes (Docker corruption case) | High | Eject guard via DiskArbitration unmount approval; agent warns and names the apps; recipes prefer app-native relocation for databases |
| macOS permission prompts confuse or loop (seen in prototype) | Med | Agent with stable bundle id; never read containers; ask for Removable Volumes up front with an explainer screen |
| Unsigned builds scare users | Med | Developer ID + notarization before public launch; install-help page meanwhile |
| Data loss bug in the mover | High | Content-hash verify, rename-aside before delete, crash recovery tests, refuse ~/Documents and sandbox paths, dry-run mode |
| Scope creep toward a CleanMyMac clone | Med | Ship v0.1 around the three unique features first; cleanup/uninstall only after |
| WebKit UI memory use (TidyMac shows ~340 MB) | Med | Budget: app < 150 MB, agent < 25 MB; measure every checkpoint |

## Open Questions
- Price: free forever, or free core + one-time "Pro" (scheduled offload rules, multiple drives)? Recommendation: free until v0.3.
- Apple Developer ID ($99/yr): when? Needed for notarization and the clean first-open experience.
- Visual identity: the UI/UX session proposes 2–3 directions; the owner picks one.
- Licence when the repo goes public (MIT vs source-available like Pearcleaner).

## Task List
Tasks, acceptance criteria and checks: `tasks/todo.md`.
