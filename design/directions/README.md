# D1 — Visual directions (pick one)

Three directions for Stowaway. Each has its own hero screen, so between them they cover the three things only
Stowaway does: **Offload** (A), **Drives / dependency map** (B), **Memory forecast** (C). Every file is standalone
HTML (open it in a browser; `?theme=light` or `?theme=dark` forces a theme). Screenshots are in `screens/`; re-shoot
with `./shoot.sh a-manifest b-patchbay c-barometer` (needs Google Chrome).

All three share what makes it feel like a Mac app and not TidyMac: a native **sidebar** (TidyMac uses a top pill bar),
SF Pro for UI text, real light **and** dark mode (TidyMac is dark only), no water/drops, no per-tab gradients, and a
monospace face for every number, size, path and hint line.

| | A · Manifest | B · Patch Bay | C · Barometer |
|---|---|---|---|
| Idea | Folders are crates on a ship's manifest: counted, stamped "VERIFIED", always listed so you can bring them home | Drives are rack modules; apps are patched to them with cables. Pull a drive and its cables fall loose | Your Mac's weather: plain-language forecasts ("memory runs out in about 38 minutes") read from instrument dials |
| Hero | Offload | Drives | Overview + forecast |
| Mood | calm logistics, warm paper + hull ink, one signal orange | tactile studio hardware, graphite, a cable colour per drive | editorial, paper and hairlines, colour = condition only |
| Primary mode | light | dark | light |
| Text / mono / display | SF Pro · IBM Plex Mono · Big Shoulders Stencil (wordmark, stamps) | SF Pro · JetBrains Mono (also tracked-caps faceplate labels) | SF Pro · Geist Mono · Instrument Serif (forecast headlines) |
| Motion | crates ride a rail while copying; a stamp "thump" when verified; free-space bar slides | spring-simulated cables sway, drop with gravity on unplug, snap home on reconnect; LEDs blink with real I/O | needle as a damped spring with a brass "20 min ago" hand; forecast cone draws in; page wash shifts with the condition |
| Strength | strongest link to the name and to the trust story (journal = manifest) | the dependency map is instantly understood and fun to touch | the forecast is the clearest, calmest way to say "act now" |
| Risk | stencil type and crate art can feel themed if overused — keep to wordmark, stamps, drive labels | hardware skeuomorphism can age; patch bay gets busy past ~12 links (needs a List view) | most "serious"; less playful than TidyMac; serif needs discipline |

## Recommendation

**A · Manifest** as the base identity: it is the one that only Stowaway could own (the name *is* cargo), it turns the
journal and verification into visible trust (stamps, counts, "the original stays until the copy is proven"), and it
is light-first, which sets it apart from TidyMac's dark gradients. Two ideas travel well into it whichever is picked:
the **cable map** from B for the Drives screen (recoloured to crate/signal tones) and the **sentence-with-a-time
forecast + brass set-hand** from C for the memory card. Those are additions, not a blend of three styles.

## Contrast

Text colours were checked against WCAG AA (4.5:1 for body text) on their backgrounds; the few that failed were
darkened. One known gap: B's light-mode cable-blue as small text on the panel is 4.4:1 — fixed in D2 tokens for
whichever direction wins.

## Not done yet (waits for the pick)
D2 tokens/system and D3 clickable prototype. Fonts load from Google Fonts here; the app will bundle the OFL files.
