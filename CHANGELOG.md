# Changelog

What changed in each release of iStash. The newest is at the top. Releases are published from `main`; each one
updates installed copies through the app itself.

## Unreleased

### Changed
- **Releases are signed with iStash's own certificate**, the same one every time, so macOS recognises each update as
  the same app and keeps its permissions (camera, external drives, Finder). Before, each update looked like a new app:
  the camera could stay refused without a prompt, and other permissions were asked again. After this update macOS may
  ask once more; from then on, permissions carry over.

## 0.1.18 — 15 Nov 2026

### Added
- **Notch island.** iStash's notifications (drive disconnected or back, memory running out, update ready) drop from
  the top of the screen as a black pill with a neon edge, then shrink back. Choose its glow colour and how long it
  stays in Settings, or turn it off to get macOS banners again.
- **Menu-bar panel:** shortcuts to System Settings, Disk Utility, Activity Monitor and System Information; a Camera
  button that opens a small window with the camera's picture; a **Screen on** toggle that keeps the display awake
  (Awake now keeps the Mac awake but lets the display sleep).
- **Fan control** in the menu-bar panel: System (macOS decides), Max, or Manual with a speed slider, plus every
  temperature sensor. The first change asks for your password once, to install a small helper that can only set fan
  speed. A manual speed lasts until you pick System or restart.
- The edge widget shows the fan speed and CPU temperature.
- **Permissions** in Settings and during setup: removable volumes, notifications, camera, Finder (for Empty Trash) and
  fan control, each with Allow, or Allow all at once, so iStash doesn't stop to ask later.
- An RGB glow for the island.

## 0.1.17 — 14 Nov 2026

### Changed
- **New cleaner design.** Memory and storage each show soft, translucent petals drifting round a solid centre with
  the amount, airy in the light theme and glowing in the dark. While cleaning, the petals speed up, particles drift
  in, and a thin ring fills once round the centre; when done, a tick appears with what was freed.
  In the dark theme it looks like a black hole: a dark centre brightening to a glowing coloured edge.
- While cleaning, the ring fills once from empty, quickly at first and slowing near the end, instead of a sweep
  that went round and round.
- After cleaning, the ring shows what was freed for a few seconds, then goes back to the live figure.
- **Measure again** is now **Refresh sizes**: it measures how much each kind of file takes up right now.

### Fixed
- The cleaner rings use the theme's colours, so they read well in the light theme.

## 0.1.16 — 14 Nov 2026

### Fixed
- The tick in the cleaner ring was turned sideways; it now sits upright.
- The cleaner animation no longer repeats one pattern: particles come from varied distances, at their own speed,
  with a swirl, and the sweep speeds up and slows down.
- An empty storage ring shows no stray dot, and Clean says how much it keeps back because it is newer than your
  "Only files untouched for" rule, instead of just showing 0.0 GB.

## 0.1.15 — 14 Nov 2026

### Changed
- **Clean looks like a phone cleaner.** Memory and storage each get a ring with the amount in the middle. While
  cleaning, the ring sweeps round and particles are pulled into it; rows being cleaned slide aside. When it's done,
  a tick pops in with what was freed.
- The memory card no longer carries a line of help text.

## 0.1.14 — 14 Nov 2026

### Changed
- **Quit from the Dock or ⌘Q now keeps iStash in the menu bar.** The window and Dock icon go away; the menu-bar
  panel, edge widget, drive warnings and schedules keep running. To quit completely, choose **Quit iStash** from the
  menu-bar icon's right-click menu. Logging out, restarting and shutting down are never held up.
- The install script quits a running iStash for real before replacing it, and refuses while a folder is being moved.

## 0.1.12 — 14 Nov 2026

### Added
- **Control-centre menu-bar panel.** Live tiles for CPU (user, system, idle), memory (with Free up memory), the
  internal disk (read and write speed) and internet, each drive's free space, quick toggles (Dark, Keep awake, Lock,
  Empty Trash, desktop icons, edge widget, Focus) and uptime.

### Fixed
- The panel's glass no longer changes transparency when its numbers refresh; it now uses macOS's own window glass.
- Text in the panel and the edge widget is readable over any wallpaper and in either app theme.
- **Lock** locks the screen, the same as Apple menu › Lock Screen. It used to turn the display off only.
- An active quick toggle stays lit when you hover over it.

## 0.1.11 — 14 Nov 2026

### Changed
- **Stowaway is now iStash**, with a new icon and menu-bar icon. Settings, moved folders and the login item carry
  over. The repository moved to `wajedhossainbd/iStash`; the old address still redirects.
- Release files are `iStash.dmg` and `iStash.app.tar.gz`. The install command removes an old `Stowaway.app`.

## 0.1.10 — 14 Nov 2026

### Added
- **Internet page.** A speed gauge whose needle follows the test live; Full, download-only or upload-only tests;
  public IP, provider, location, local network and router; a table of every past test.
- **Server picker.** One box that searches the Speedtest server list by city, country or provider, with the nearest
  servers, recently used ones with their last result, regional download files, or a link to any large file.

### Changed
- The Overview keeps a one-line internet summary and links to the Internet page.

## 0.1.8 — 14 Nov 2026

### Added
- Choose the speed-test server: Washington DC, Oregon, Montréal, Singapore, Germany, Finland, France, or a custom
  file.

### Fixed
- The edge widget is readable over light wallpapers, its rows no longer overlap, and it fits its height.

## 0.1.7 — 14 Nov 2026

### Added
- **Edge widget.** A thin glass column on the left or right screen edge with CPU, memory, network, disk and each
  drive; off, show on hover, or always.
- **Speed test** with download, upload, ping and jitter, public IP and provider, and a history of past tests.
- **Drive activity.** Read and write speed for each drive, its average, and when it was last active.
- **Livelier drive map.** Data flows along each cable, hovering lifts one, a missing drive's port pulses, and
  clicking a drive opens it.

### Changed
- The swap chart scales to the swap line and shows the disk limit only when swap is heading for it.
- Scrollbars are thin, and the layout reflows smoothly when the window is resized.

### Fixed
- The scan no longer asks for Apple Music access on every launch; it skips macOS's own Library folders.

## 0.1.6 — 14 Nov 2026

### Changed
- New project page: features, screenshots, install and update steps, and a GPL 3.0 licence.

## 0.1.5 — 14 Nov 2026

### Added
- **Open at login**, through macOS's Login Items.

### Changed
- **Free up memory** no longer asks for your password. It makes macOS compress and move idle apps' memory to disk.
- Help text across the app is shorter.

## 0.1.4 — 14 Nov 2026

The first release built and published automatically from `main`.

### Added
- **In-app updates.** iStash checks for new versions at launch and every six hours, and installs signed updates
  from Settings.
- **Missing-drive warnings.** A notification when a drive with moved folders disconnects or returns, and a prompt
  to connect it when an app that needs it is opened.
- **Focus.** Give one app priority over the others.
- **Clean.** Remove caches, logs, Xcode build files and old temporary files by hand, on a schedule, or once they pass
  a size; free up memory.
- Drive pages listing every folder linked to a drive, and a drive picker on Offload.
- One-command install script.

## 0.1.0 — 13 Nov 2026

The first release.

### Added
- Scan the home folder for big app folders that can live on an external drive.
- Move a folder to a drive and bring it back: copy, check every file, link, then remove the original. Stop at any
  time to put everything back; interrupted moves are finished or undone at the next launch.
- Eject guard: macOS asks iStash before any eject and it refuses while an app still needs the drive.
- Live CPU, memory, disk and network, and a forecast of when memory will run out.
- Dark and light themes, with glass surfaces.
