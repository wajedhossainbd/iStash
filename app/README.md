# Stowaway app (Tauri 2)

`ui/` is the web UI (TypeScript, Vite, no framework), ported from `design/prototype/`. `src-tauri/` is the native
shell: the window, the menu-bar popover, and the commands that run `stowaway-core` (scan, move, bring back, eject
guard, memory readings, the missing-drive watcher, Focus and the cleaner). All data goes through `ui/src/api.ts`:
inside the app that is `real.ts`, and in a browser or in tests it is the sample data in `api.ts`.

Updates use `tauri-plugin-updater` against the GitHub releases; see "Releases" in the root README.

```sh
cd app/ui && npm install
npm run app:dev        # the app, with hot reload
npm run app:build      # app/src-tauri/target/release/bundle/macos/Stowaway.app
npm run dev            # the UI alone in a browser at http://127.0.0.1:1420
npm test               # formatting and mock dry-run checks
```

Designed states on the mock: add `?mock=loading`, `empty`, `error` (helper not running), `noaccess` (no removable
volume access) or `unreadable` (drive needs First Aid) to the URL in `npm run dev`. In the inspector,
`stowawayMock.setConnected('archive', true)` plugs a drive in, so the cable animation can be checked.
