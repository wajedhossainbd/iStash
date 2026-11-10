# Stowaway app (Tauri 2)

`ui/` is the web UI (TypeScript, Vite, no framework), ported from `design/prototype/`. `src-tauri/` is the native
shell: window with sidebar vibrancy and a merged title bar, menu-bar item with a popover, View > Appearance, and
two small commands (`app_icon` reads an app's real icon from its bundle; `open_target` opens a fixed set of places such
as Disk Utility). All data goes through `ui/src/api.ts`; today that is a mock built from `design/prototype/data.js`.

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
