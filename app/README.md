# iStash app (Tauri 2)

`ui/` is the web UI (TypeScript, Vite, no framework, rendered with morphdom). `src-tauri/` is the native shell: the
main window, the menu-bar panel, the edge widget, and the commands that run `stowaway-core`.

All data goes through `ui/src/api.ts`. Inside the app that is `real.ts`, which calls the shell; in a browser or in
tests it is the sample data in `api.ts`, so every screen can be designed and checked without a Mac's real data.

| Shell module | Does |
|---|---|
| `engine.rs` | Scan, dry runs, move and bring back (in a child process, so Stop can undo), eject and the eject guard |
| `mem.rs` | Live CPU, memory, swap, network and the memory forecast |
| `away.rs` | Notices a drive with moved folders going away, and an app used while its drive is missing |
| `net.rs` | Connection details, the Speedtest server list, and the speed test with live progress events |
| `tune.rs` | Focus, the cleaner, and freeing memory without a password |
| `quick.rs` | The menu-bar panel's toggles: keep awake, lock, empty Trash, desktop icons, uptime |
| `widget.rs` | The edge widget: side, show on hover (follows the pointer), always on |
| `login.rs` | Open at login through macOS's Login Items |

```sh
cd app/ui && npm install
npm run app:dev        # the app, with hot reload
npm run app:build      # app/src-tauri/target/release/bundle/macos/iStash.app and the DMG
npm run dev            # the UI alone in a browser at http://127.0.0.1:1420
npm test               # UI tests
```

Routes: `#/overview`, `#/offload`, `#/drives` (and `#/drives/<id>`), `#/memory`, `#/internet`, `#/clean`,
`#/settings`; `#/popover` and `#/widget` are the floating windows.

Designed states on the sample data: add `?mock=loading`, `empty`, `error` (helper not running), `noaccess` (no
removable volume access) or `unreadable` (drive needs First Aid) to the URL in `npm run dev`. In the inspector,
`stowawayMock.setConnected('archive', true)` plugs a drive in, so the cable animation can be checked.

Updates use `tauri-plugin-updater` against the GitHub releases; releasing is described in `HANDOFF.md`.
