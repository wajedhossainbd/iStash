<div align="center">

# iStash

**Make room on your Mac. Keep every app working.**

iStash moves heavy app data to an external drive and leaves a link in its place,
so your apps carry on as if nothing moved.

[![Latest release](https://img.shields.io/github/v/release/wajedhossainbd/iStash?label=release&color=C8FF3D&labelColor=15171B)](https://github.com/wajedhossainbd/iStash/releases/latest)
![macOS 14+](https://img.shields.io/badge/macOS-14%2B-F2F4F5?labelColor=15171B)
![Apple Silicon](https://img.shields.io/badge/Apple%20Silicon-M1%20and%20later-F2F4F5?labelColor=15171B)
![Download 3 MB](https://img.shields.io/badge/download-3%20MB-F2F4F5?labelColor=15171B)
[![License: GPL 3.0](https://img.shields.io/badge/license-GPL%203.0-F2F4F5?labelColor=15171B)](LICENSE)

[**Download for Mac**](https://github.com/wajedhossainbd/iStash/releases/latest/download/iStash.dmg) · [Install with one command](#install) · [Features](#features) · [Changelog](CHANGELOG.md)

[![Buy me a coffee](https://img.shields.io/badge/Buy%20me%20a%20coffee-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=000000)](https://supportkori.com/wajed)

<br>

![A short tour of iStash](docs/screenshots/tour.gif)

</div>

## Why iStash

A 256 GB Mac fills up fast, and most of the weight isn't your files. It's the data apps keep for themselves: AI models,
SDKs, build caches, virtual machines, browser caches. Moving those folders by hand works until the drive is unplugged,
an app starts over with an empty folder, or you forget which app needed which drive.

iStash does the move safely, keeps track of every link, and tells you when a drive an app needs isn't there.

## Features

| Feature | What it does |
| --- | --- |
| **Offload app data** | Finds the big app folders that can live on an external drive and moves them with one click. |
| **Safe moves** | Copies, checks every file, leaves a link, then removes the original. Stop at any time and everything is put back. |
| **Crash-proof** | Every step is written to a journal. An interrupted move is finished or undone the next time iStash starts. |
| **Which app needs which drive** | A drive map shows each app's data and where it lives. Open a drive to see every folder on it. |
| **Eject guard** | macOS asks iStash before any eject. If an app still needs the drive, the eject is refused and the app named. |
| **Missing-drive warnings** | A notification when a drive with app data disappears, and a prompt to connect it if you open an app that needs it. |
| **Notch island** | iStash's notifications drop from the top of the screen as a black pill with a neon glow (amber, blue, pink, green, white or RGB), then shrink back. |
| **Menu-bar panel** | A control-centre panel: live CPU, memory, disk and internet tiles, fan and temperatures, your drives, shortcuts to System Settings, Disk Utility, Activity Monitor, System Information and the camera, and quick toggles for dark mode, keep awake, keep the screen on, lock, empty Trash, desktop icons, the edge widget and Focus. |
| **Fan control** | Let macOS decide, run the fans at full speed, or set a speed yourself, with every temperature sensor listed. Your password is asked once. |
| **Edge widget** | A thin glass column on the left or right edge of the screen with CPU, memory, network, disk, fan and drives. Always on, or slides out when you touch the edge. |
| **Internet** | A live speed gauge with download, upload, ping and jitter; your public IP, provider and location; every past test in one table. |
| **Any test server** | Search Speedtest's servers by city, country or provider, pick a recently used one, or test against any file. |
| **Drive activity** | Read and write speed for each drive, its average, and when it was last busy. |
| **Memory forecast** | Live CPU, memory, disk and network, with a warning before macOS runs out of memory. |
| **What's using the CPU** | The apps and macOS processes using the most CPU, busiest first, counted like Activity Monitor (100% is one core). The two busiest also show in the menu-bar panel. |
| **Focus** | Give one app priority: the other apps run in the background until you stop it. |
| **Clean** | Free up memory with no password, and clear caches, logs, Xcode build files and old temporary files, shown as a glowing orb that pulls the clutter in and tells you what was freed. |
| **Scheduled cleaning** | Clean daily, weekly, monthly, or once the files pass a size such as 5 GB, and only files older than you choose. |
| **Open at login** | Starts with your Mac, so warnings and schedules keep working. |
| **Permissions up front** | Every permission iStash uses is listed in setup and Settings, with Allow all, so it doesn't stop to ask later. |
| **Updates built in** | New versions install from inside the app, each one checked against iStash's signature. |
| **Light and dark** | Follows your Mac's appearance, or pick one. |

## Screenshots

<p align="center"><img src="docs/screenshots/cables.gif" alt="The drive map: data flows along each cable, hovering lifts one, and a drive plugs in and out" width="760"><br><sub><b>The drive map</b> · data flows along each cable; hover to follow one, and watch a drive plug in and out</sub></p>

<table>
  <tr>
    <td width="50%"><img src="docs/screenshots/offload.png" alt="Offload: big app folders and where they can go"><br><sub><b>Offload</b> · big app folders, and the drive they move to</sub></td>
    <td width="50%"><img src="docs/screenshots/moving.png" alt="A move in progress"><br><sub><b>Moving</b> · copy, check every file, link, remove the original</sub></td>
  </tr>
  <tr>
    <td><img src="docs/screenshots/drives.png" alt="Drives: which app needs which drive"><br><sub><b>Drives</b> · which app needs which drive</sub></td>
    <td><img src="docs/screenshots/overview.png" alt="Overview with the memory forecast"><br><sub><b>Overview</b> · live numbers and the memory forecast</sub></td>
  </tr>
  <tr>
    <td><img src="docs/screenshots/internet.png" alt="Internet: live speed gauge and test history"><br><sub><b>Internet</b> · live speed gauge, connection details and every past test</sub></td>
    <td><img src="docs/screenshots/servers.png" alt="Searching test servers"><br><sub><b>Test servers</b> · search by city, country or provider</sub></td>
  </tr>
  <tr>
    <td><img src="docs/screenshots/menubar.png" alt="The menu-bar panel"><br><sub><b>Menu-bar panel</b> · live tiles, fan control, shortcuts and quick toggles</sub></td>
    <td align="center"><img src="docs/screenshots/widget.png" alt="The edge widget" width="170"><br><sub><b>Edge widget</b> · on the edge of your screen</sub></td>
  </tr>
  <tr>
    <td colspan="2"><img src="docs/screenshots/island.png" alt="The notch island with an amber and an RGB glow"><br><sub><b>Notch island</b> · notifications from the top of the screen, in the glow you pick</sub></td>
  </tr>
  <tr>
    <td><img src="docs/screenshots/memory.png" alt="Memory: the apps using the most memory and CPU, and Focus"><br><sub><b>Memory</b> · what's using memory and CPU, and Focus</sub></td>
    <td><img src="docs/screenshots/clean.png" alt="Clean: freeing memory"><br><sub><b>Clean</b> · freeing memory, by hand or on a schedule</sub></td>
  </tr>
</table>

### Light theme

<table>
  <tr>
    <td width="50%"><img src="docs/screenshots/overview-light.png" alt="The Overview screen in the light theme"></td>
    <td width="50%"><img src="docs/screenshots/clean-light.png" alt="The Clean screen in the light theme"></td>
  </tr>
</table>

## Install

**Requirements:** macOS 14 Sonoma or newer, on a Mac with Apple Silicon (M1 or later).
The download is about 3 MB and the installed app about 6 MB.

Step by step, with pictures, first launch, troubleshooting and uninstalling: **[the install guide](docs/INSTALL.md)**.

### One command (recommended)

```sh
curl -fsSL https://github.com/wajedhossainbd/iStash/releases/latest/download/install.sh | sh
```

It installs iStash in Applications and opens it, with no warnings.

### Download

Get [**iStash.dmg**](https://github.com/wajedhossainbd/iStash/releases/latest/download/iStash.dmg).

> [!NOTE]
> iStash isn't notarised by Apple yet, so macOS says **"iStash.dmg" Not Opened**. To open it anyway:
> 1. Click **Done**.
> 2. Open **System Settings › Privacy & Security**, scroll down to the message about iStash.dmg and click
>    **Open Anyway**, then confirm with your password.
> 3. Drag iStash into Applications. If macOS asks again when you open iStash, repeat step 2.
>
> Or clear the download flag in Terminal, then open the DMG as usual:
>
> ```sh
> xattr -d com.apple.quarantine ~/Downloads/iStash.dmg
> ```

### Updates

iStash checks for a new version at launch and every six hours. When one is ready, Settings shows an **Update**
button; iStash installs it and restarts.

## Privacy and safety

- **Never touched:** Documents, Desktop, iCloud Drive, Photos, Mail and app sandboxes.
- **No account, no analytics.** iStash only goes online to check for updates.
- **Drives:** APFS or Mac OS Extended. exFAT and FAT can't hold links safely, so iStash won't use them.
- **Cleaning** deletes for good, never touches folders moved to a drive or macOS's own caches, and always shows what
  it will remove first.

## Build from source

You need Node 22, Rust (stable) and Xcode's command line tools.

```sh
git clone https://github.com/wajedhossainbd/iStash.git
cd iStash/app/ui
npm install
npm run app:build    # → app/src-tauri/target/release/bundle/macos/iStash.app
```

`npm run app:dev` runs the app with hot reload; `npm run dev` shows the UI alone in a browser on sample data.

<details>
<summary><b>Releases and repository layout</b></summary>

<br>

Every push to `main` runs [`release.yml`](.github/workflows/release.yml): it tests the app, builds the next version,
and publishes a release with `iStash.dmg`, the signed update package and `install.sh`. Updates are signed with the
repository secret `TAURI_SIGNING_PRIVATE_KEY`; the matching public key is in `app/src-tauri/tauri.conf.json`.

| Folder | What's in it |
| --- | --- |
| `app/` | The app: Tauri shell (`src-tauri/`) and UI (`ui/`) |
| `crates/` | The Rust engine and command line tool |
| `design/` | Design tokens, system notes and the prototype |
| `docs/screenshots/` | The pictures on this page, taken on sample data |

</details>

## Changelog

What changed in each version is in [CHANGELOG.md](CHANGELOG.md), and on each [release](https://github.com/wajedhossainbd/iStash/releases).

## Support

iStash is free. If it saved you a drive's worth of space, you can [buy me a coffee](https://supportkori.com/wajed).

## License

iStash is free software under the [GNU General Public License v3.0](LICENSE). You may use, study, share and change
it; if you distribute a version based on it, it must also be under the GPL.
