# Implementation Plan: iStash (formerly Stowaway)

> 14 Nov 2026: v0.1 "Offload" is shipping as iStash; see `HANDOFF.md` for what is done and what is open.

Revised 10 Nov 2026 (v2): rebuilt around features worth paying for, and a stricter quality bar.
Research: `research/market.md`, `research/tidymac-teardown.md`, `research/naming-and-emroze.md`.
Design: `design/system.md` (direction "Patch Bay", signature element: the cable map).

## Overview
Stowaway is a native macOS utility for people whose Mac is too small for their work. It **moves heavy app data to
external drives while the apps keep working**, and then **runs that for you**: it keeps a set amount of space free,
packs what you need before you unplug, warns before memory runs out, and checks that data on external drives is still
intact.

**Promise:** "Never see 'Your disk is almost full' again. Keep every app working."

## Why people would pay (strategy)
Cleaning is a commodity: TidyMac and Mole are free, CleanMyMac's subscription is its most-hated feature
(`research/market.md`). Stowaway does not sell cleaning. It sells three outcomes no product delivers:

1. **Space that manages itself** — Autopilot keeps N GB free by stowing the right folders at the right time.
2. **A laptop that still works away from the desk** — Travel mode packs the apps you choose before you unplug; a launch
   guard tells you the moment an app needs a missing drive.
3. **Data you can trust on cheap external drives** — every stowed file has a checksum; Scrub re-checks them, drive
   fitness tests catch slow or failing drives, Mirror keeps a second copy. (The owner's own JetDrive silently lost 424
   files and wrote at 4 MB/s; no tool warned.)

Plus the memory forecast (time until the "out of application memory" freeze, which app is leaking, one-click restart).

### Free vs Pro (recommendation; owner decides, see Open Questions)
| | Free | Pro (one-time purchase) |
|---|---|---|
| Live monitor, cable map, drive map, eject guard | ✓ | ✓ |
| Manual move / bring back, app recipes, crash-safe journal | ✓ unlimited | ✓ |
| Memory forecast notification | ✓ | ✓ |
| Storage map, cleanup preview, uninstaller (Phase 5) | ✓ | ✓ |
| **Autopilot** (keep N GB free, rules, schedule) | — | ✓ |
| **Travel mode** + launch guard actions | — | ✓ |
| **Timeline** ("what grew"), history beyond 7 days | 7 days | ✓ |
| **Scrub**, **drive fitness**, **Mirror** | fitness only | ✓ |
| **Memory relief** (leak detection, restart app, rules) | — | ✓ |
| **Cold storage** for stale projects, CLI + Shortcuts | — | ✓ |

Manual offload stays free and unlimited: trust and word of mouth come from the free core; money comes from automation
and protection. Pricing recommendation: **$29 once** (launch $19), 3 Macs, all 1.x updates, 14-day full trial,
no account (licence = signed key checked offline), no subscription, no upsell nags.

## Architecture Decisions
- **Rust core + Tauri 2 app + launchd agent**, as built in Phase 1 (`crates/stowaway-core`, `crates/stowaway-cli`).
  The agent has its own bundle identity so macOS asks once for Removable Volumes access. No root helper. No Python.
- **The agent is the product.** Autopilot, Travel mode, Scrub, forecast and guards all run in the agent; the app is a
  window onto it. Agent ↔ app over a local Unix socket with a per-user token; one writer (the agent) owns the journal.
- **State store:** SQLite (via `rusqlite`, bundled) at `~/Library/Application Support/Stowaway/state.db` for snapshots,
  manifests (path, size, mtime, BLAKE3), rules and history. The move journal stays an append-only JSONL file.
- **Every stowed folder has a manifest** written at move time. Scrub, Mirror and Travel mode all build on it.
- **Policies are data** (TOML/SQLite rows), evaluated by one planner that outputs a dry-run plan; the same plan is
  shown in the UI and executed by the agent. Nothing automatic runs without a plan the user could have previewed.
- **Automatic actions are conservative:** only folders whose recipe is "verified", only when their apps are closed,
  only to drives that pass the fitness test, never when on battery below 20%, always reversible, always logged.
- **Never** read other apps' sandbox containers or protected Apple data; never touch ~/Documents, ~/Desktop, Photos.
- **Licensing:** ed25519-signed licence file verified offline; trial clock in the agent; no network needed.
- **Distribution:** Developer ID signed + notarized DMG, Tauri updater, Homebrew cask. Merchant of record
  (Paddle or Lemon Squeezy) handles tax and refunds.

## Quality bar ("solid build") — applies to every task
- **Data safety invariants** (tested, not assumed): the original is never removed before the copy is verified by
  content hash; every step is journaled and recoverable; a failed or interrupted action leaves data in exactly one
  valid place.
- **Fault injection in CI:** SIGKILL at every journal step (done in Phase 1), drive detached mid-copy, target disk
  full, permission denied, source changed during copy, case-insensitive/sensitive targets, sparse files (Docker.raw),
  hard links, xattrs/ACLs/resource forks, sockets and FIFOs, very long paths, Unicode names.
- **Recipe verification:** a recipe is "verified" only after the real app was moved, launched, used and brought back
  on a test Mac; unverified recipes are manual-only and say so.
- **Budgets:** agent < 25 MB RAM and < 1% CPU idle; app < 150 MB; scan of home < 60 s; UI never blocks on disk.
- **Compatibility:** macOS 14, 15, 26 and 27; Apple silicon and Intel; APFS and HFS+ targets (exFAT refused).
- **Accessibility:** VoiceOver labels, full keyboard use, reduced motion, WCAG AA (already in the design tokens).
- **Honest numbers:** every figure in the UI is measured; forecasts show their confidence.
- **Definition of done per task:** tests + clippy + fmt green, acceptance criteria demonstrated, docs updated.

## Phases
- **Phase 0 — Design** ✅ done (D1–D3, Patch Bay).
- **Phase 1 — Core engine** ✅ done (T1–T8, 51 tests).
- **Checkpoint A** ✅ passed 10 Nov 2026 (see `tasks/todo.md`).
- **Phase 2 — v0.1 "Offload"** (free core as a real app): hardening from the quality bar, agent, Tauri app
  implementing the prototype, menu bar, DMG + updater. *Checkpoint B:* owner uses it daily on real folders for a week.
- **Phase 3 — v0.2 "Autopilot"** (first Pro features): state store + manifests, Timeline, Autopilot planner and rules,
  Travel mode + launch guard, Memory relief. *Checkpoint C:* two weeks on the owner's Mac with Autopilot on, zero data
  incidents, zero "disk full" warnings.
- **Phase 4 — v0.3 "Integrity":** drive fitness test, Scrub, Mirror. *Checkpoint D:* Scrub detects injected corruption
  and a deleted file on a test drive; Mirror restores them.
- **Phase 5 — v0.4 "Analyze & Clean":** storage map, developer-aware cleanup to Trash with undo, cold storage for
  projects, uninstaller. Kept deliberately last: it is table stakes, not the reason to buy.
- **Phase 6 — v1.0 launch:** licensing + trial, payments, Developer ID + notarization, website, docs, private beta
  (10–20 testers), public launch.

## Risks and Mitigations
| Risk | Impact | Mitigation |
|---|---|---|
| Automatic move breaks an app or loses data | Critical | Autopilot only on verified recipes, apps closed, fit drives; hash verify; journal; dry-run plan shown; kill switch; Checkpoint C before any public build |
| Drive unplugged while in use | High | Eject guard; launch guard; Travel mode; recipes prefer app-native relocation for databases |
| Scrub/Mirror cost (I/O, battery) | Med | Incremental (changed files first), only on power, pausable, budgeted MB/s |
| macOS permission prompts confuse | Med | Agent bundle identity; one onboarding screen that asks once; never read containers |
| Free rivals copy offloading | Med | Depth: recipes verified per app, Autopilot, Travel mode, integrity — hard to copy quickly; ship fast |
| Solo-developer support load after launch | Med | In-app diagnostics export, docs per recipe, refund policy via merchant of record |
| Nobody pays | High | Free core builds audience first; trial of Pro; price test at beta; decide Pro split with real usage data |
| No Developer ID yet | High for launch | Needed before beta ($99/yr); ad-hoc builds only for the owner until then |

## Open Questions (owner decisions)
1. Free vs Pro split and price above: accept, or change?
2. Apple Developer ID: buy before Phase 2's DMG (recommended) or before beta?
3. Merchant of record: Paddle or Lemon Squeezy (both handle tax; availability for Bangladesh payouts must be checked).
4. Licence for the public repo: closed source for the app with an open core, or fully closed? (Affects GitHub visibility.)
5. Which apps matter most for the first verified recipes? Proposed: Docker, Xcode, Ollama/LM Studio, Claude Cowork VM,
   Android SDK/emulator, npm/pnpm/Gradle/Cargo caches.

## Task List
`tasks/todo.md`.
