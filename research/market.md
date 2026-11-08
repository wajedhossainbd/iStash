# Mac utility market, November 2026

Prices conflict between sources for CleanMyMac and Sensei; each figure names its source in the original report.

| App | Core | Price | Distribution | Weak spots |
|---|---|---|---|---|
| **CleanMyMac 5.7** (MacPaw) | Smart Care one-click scan, junk, optimization, uninstaller/updater, malware, cloud cleanup, duplicates, Space Lens bubbles, menu-bar Menu (RAM/pressure/swap, "free up RAM"), Assistant, Eney AI | subscription from $3.33/mo; ~$47.50/yr Basic, $71.40/yr Plus; one-time ~$120–196 | direct + reduced App Store build | cancellation/refund complaints, renewal pushes, lifetime owners asked to pay again, weak malware detection, upsell nags |
| **DaisyDisk** | sunburst map, accurate sizes, System Data, snapshots, external/cloud scan | $9.99 once | direct + App Store | deletes permanently, find-only |
| **Pearcleaner** | uninstaller, orphans, Sentinel, Homebrew/PKG managers, lipo, CLI | free (Apache + Commons Clause) | Homebrew | **on hold since end of 2025**; deleted a user's ~/Documents/OpenSCAD; lookalike malware site |
| **AppCleaner** | drag-to-uninstall | free | direct zip | basic |
| **OnyX** | maintenance scripts, hidden settings | free | per-macOS builds | expert-only |
| **iStat Menus 7** | sensors, per-app network, S.M.A.R.T., fan, **Rules** (threshold alerts) | $11.99 once | direct, App Store, Setapp | paid upgrades |
| **Stats** (exelban) | menu-bar monitors | free, MIT | Homebrew | sensors double CPU/power, fan control unmaintained, no memory-pressure alerts |
| **Sensei** (Cindori) | monitor, cleaner, uninstaller, S.M.A.R.T., battery, benchmarks | $29/yr or $59 once | direct | no RAM view listed |
| **Mole** (tw93) | CLI: clean, uninstall, optimize, analyze, status; dry-run, Trash, op log, whitelist | free GPL-3 + paid app | Homebrew (~74k stars) | CLI-first |
| **DiskPort** | moves **Xcode** folders to external with symlinks, verify, rollback, local fallback | ? | direct | Xcode only |
| **AppPorts** | symlinks app **bundles** to external | free MIT | GitHub | bundles only, not data |
| **TidyMac** | see tidymac-teardown.md | free | direct DMG | 0.x risks, no self-update |

## The three questions
1. **Move app data to an external drive, app keeps working?** No major product does. Closest: DiskPort (Xcode), AppPorts
   (bundles). People do it by hand and break things: Plex opens empty, MobileSync symlink stopped working after
   Catalina, DerivedData on external breaks tests, Docker data corrupts when the drive is unplugged while running.
2. **Which apps need which drive?** Nobody. DiskPort's unplug fallback is nearest.
3. **Warn before out-of-memory?** iStat Rules and Memory Diag alert on thresholds; CleanMyMac Menu only shows pressure.
   Nothing predicts the "out of application memory" dialog, which depends on swap growth **and free disk**.

## Gaps Stowaway can own
1. Safe offloading with a per-app compatibility list, verification and one-click restore.
2. A drive dependency map: warn before eject, hold apps when a drive is missing, repair on reconnect.
3. Memory-exhaustion forecast (pressure + swap trend + free disk → minutes left, which app to quit).
4. Trust as the product: dry-run by default, Trash not delete, undo journal, never ~/Documents, fair one-time price.
5. One light, maintained native app combining monitoring, cleanup and storage, developer-aware (Homebrew, Xcode,
   Docker, node_modules, LLM models).
