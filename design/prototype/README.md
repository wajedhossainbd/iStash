# Stowaway clickable prototype (D3)

Plain HTML, CSS and ES modules, no build step. The Tauri app should port the same components (`ui.js`), screens
(`screens.js`) and token CSS (`tokens.css`, generated from `../tokens.json`) to TypeScript.

```sh
cd design/prototype && python3 -m http.server 8791 --bind 127.0.0.1   # then open http://127.0.0.1:8791
./fetch-icons.sh   # optional: copies real app icons from /Applications into icons/ (git-ignored)
./shoot.sh         # screenshots of every screen into screens/ (needs the server and Google Chrome)
```

The bar above the window is for review only: theme, screen state (normal / loading / empty / error) and which drives
are plugged in. Flows to try: Offload → Move… (dialog, progress, then "On drives"); Quit & move…; Why? on iPhone
backups; Show how on Docker; Bring back… on LM Studio with Archive plugged in (refused: not enough room); Drives →
Eject Samsung T7… (cables fall) and tick it back (cables plug in); Memory → Quit Docker Desktop (forecast clears);
Onboarding through to Offload; Menu-bar popover. URL parameters `?theme=dark&view=empty&archive=1` preset the controls.
