# TidyMac teardown (competitor)

By Abdullah Emroze (emroze.dev). Read from public pages only (overview, privacy, support, screenshots, demo video);
the app itself was not downloaded or run. Sources: https://emroze.dev/tidymac/ · /privacy/ · /support/

## Facts
- Free, no account, no subscription. v0.1.0 (build 4), ~5 MB DMG, "under 9 MB installed", macOS 14+, Apple Silicon + Intel, native.
- Direct download, says Developer ID signed + notarized (not verified). Not on the App Store (needs access the sandbox forbids).
- Full Disk Access requested via an in-app banner; a privileged helper ("installed once with your password") for some tasks.
- No analytics. Network only for app update feeds, Homebrew, speed test. Does **not** update itself.

## Tabs (top segmented pill bar; search "⌘F"; history + settings buttons)
- **Clean**: water-gauge ring ("FREE SPACE"), category chips (Developer, Project builds, App caches, Browsers, System,
  Installers, Leftovers). Scan shows a live counter inside the ring, category rows with progress bars, a "latest finds"
  feed with reasons. Results: category sidebar + item list with reason pills ("Opt-in", "Quit & include",
  "Last built 69 months ago"). Safe items pre-selected, opt-in remembered, active projects left unselected.
  Everything goes to the **Trash** (reversible) and nothing is removed before the list is shown.
- **Apps**: "orbit radar" (each app is a dot; distance = last used), filters (unused 2+ months, Intel under Rosetta,
  over 1 GB), sub-tabs Uninstall / Updates (Sparkle feeds + Homebrew) / Startup. Uninstall is permanent (warned).
- **Optimize**: maintenance task cards with a one-line rationale, "last run", status pills (ready, needs helper,
  opt-in, "Safari is running"): flush DNS, rebuild QuickLook/LaunchServices, VACUUM Mail/Messages/Safari DBs,
  repair plists, thin TM snapshots, purge memory, Spotlight check, verify disk, trim knowledgeC, broken launch agents…
- **Analyze**: sunburst storage map around a drop core; dashed = cleanable, dotted = protected; sub-tabs Folders,
  Large files, Old files, Duplicates.
- **Status**: health score, CPU (P/E cores, temp), GPU, memory pressure, battery, disk (SMART, R/W), network, fan
  (with Auto/Cool/Max control), process table; menu-bar popover with top processes and a speed test.

## Visual language
Dark only. A different ambient gradient per tab (teal Clean, violet Apps, amber Optimize, rust Analyze, green Status).
Geometric sans for text, **monospace for numbers, sizes, paths and hint lines**. Playful physics: liquid gauge with
bubbles, finds "drop into the water", radar sweep, gyroscope that "settles when your Mac is tidy", spinnable sunburst.
Mono hint lines teach the interactions ("hover the ring to read it").

## Weaknesses / openings
- No self-update, no changelog, no repo or community: users stay on 0.1.0.
- Name collides with other "TidyMac" products.
- Risky one-click operations in a 0.x build (permanent uninstall, plist "repair", VACUUM of Mail/Messages DBs, TM
  snapshot thinning, root helper) with no backup/rollback story.
- Demo bugs: "used 46 years ago" (epoch-0 date); the app itself ~340 MB RAM.
- Missing vs CleanMyMac: malware scan, privacy cleaner, mail attachments, Photos junk, iOS backups, scheduler,
  shredder, localization, light mode, accessibility.
- **Does not move app data to external drives**, track which app needs which drive, or warn before memory runs out —
  Stowaway's core.
