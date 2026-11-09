# Stowaway design system

Direction **B · Patch Bay**, chosen by the owner on 9 Nov 2026, after a de-AI pass (below). Tokens live in
[`tokens.json`](tokens.json); `node design/tokens.mjs` regenerates `prototype/tokens.css` and fails if any listed
colour pair misses WCAG AA. The clickable prototype in [`prototype/`](prototype/) is the reference implementation:
every component here has a function of the same name in `prototype/ui.js` or `prototype/screens.js`.

## The idea in one line

Stowaway is a quiet, native Mac utility with **one** signature element: the **cable map**, where apps are patched to
the drives that hold their data. Everything else looks like a well-made Apple app, so the cables carry the identity.

## De-AI rules (what was removed from the first draft, and stays out)

The D1 draft of Patch Bay had the usual tells of generated UI. These rules keep them out:

1. **No decorative uppercase mono labels.** Group titles are SF Pro 11 semibold, sentence case. Mono is only for
   numbers, sizes, times and paths, never for labels or headings.
2. **No hint lines** ("▸ drag a row…"). If something is interactive it looks interactive; extra help goes in a tooltip
   or the empty state.
3. **No glow, no fake hardware.** No LED glow shadows, screws, brushed faceplates or segmented VU meters. Status is a
   flat 8 px dot; capacity is a plain bar.
4. **One accent.** Blue for actions (follows the macOS accent when its contrast passes, see Colour). Status colours
   mean status only. Drive colours appear only on that drive's cable, swatch and capacity bar.
5. **Real icons or nothing.** App rows use the app's real icon from its bundle (`NSWorkspace.icon(forFile:)`). No
   coloured letter tiles. If there is no icon (a CLI tool such as hermes), show a neutral grey tile.
6. **Write sentences, not stat strings.** "Archive is missing, so LM Studio is held." beats
   "ARCHIVE · MISSING · 1 APP". Middle dots are allowed only between two short facts in captions.
7. **Every number is real or absent.** No invented counters ("3 links"), health scores or fake precision.
8. **No controls that do nothing.** No "coming later" nav items and no tabs without a second view.
9. **Motion explains a state change** (a cable falling when a drive leaves) and is never ambient decoration.
10. **Copy is plain and specific:** say what will happen, to which folder, how big, and what is kept. No em dashes, no
    exclamation marks, no "magic", "smart", "seamless", "effortless".

## Foundations

### Colour
- Light and dark are equal citizens; the app follows the system unless the user picks one in the View menu.
- Roles: `bg.*` (content, sidebar vibrancy, surface, inset, raised, overlay), `line.*`, `text.*`
  (primary, secondary, tertiary, disabled, onFill), `accent.*`, `ok.*`, `warn.*`, `danger.*`, each with `fill`
  (bars, buttons, dots), `text` (words) and `tint` (backgrounds). Never put `*.fill` colours behind body text and never
  use `*.text` for large fills.
- **System accent:** read `NSColor.controlAccentColor`; use it for `accent.fill` only if white text on it is ≥ 4.5:1,
  otherwise keep the token. `accent.text` stays the token (the system colour is often too light as text).
- **Drive colours** (`cable.blue`, `orange`, `violet`, `green`, `pink`, `teal`): assigned in the order drives are
  first used, stored with the drive, never reused while that drive is known. `cable.loose` is the hanging cable.
- **Contrast:** every text pair in `tokens.json → contrast.text` is ≥ 4.5:1 and every graphic pair ≥ 3:1, in both
  themes; the script enforces it. Colour is never the only signal: missing drives also get a dashed outline, a
  "Missing" pill and a sentence.

### Type
| Token | Use |
|---|---|
| `title` 20/24 bold | page titles |
| `headline` 15/20 semibold | card titles, empty-state titles |
| `body` 13/18, `emphasis` 13/18 semibold | everything else; row names |
| `callout` 12/16 | buttons, secondary text |
| `caption` 11/14, `group` 11/14 semibold | captions, sidebar groups, column headers |
| `figure` mono 24/28 | the one big number of a card |
| `number` mono 13/18, `path` mono 11/14 | sizes, percentages, paths |

SF Pro comes from the system. JetBrains Mono (OFL) is bundled with the app; the prototype loads it from Google Fonts.
Numbers always use tabular figures. The forecast sentence is SF Pro 22/28 semibold, the only text above 20 pt
except onboarding headings (26/32).

### Space, radius, elevation
- Spacing on a 4 px grid (`space.1`..`space.10` = 2…40 px). Page padding 24, card padding 16, gaps 12–16.
- Radius: `xs 4` pills · `sm 6` buttons and nav rows · `md 8` drive modules · `lg 10` cards, tables, banners ·
  `xl 12` window, dialogs, popover.
- Elevation: `1` buttons and selected segments, `2` raised cards, `3` popover, `4` window and sheets. Cards on the
  content background use a hairline, not a shadow.

### Motion
- Durations: `fast 120` (hover, toggles), `base 200` (fades, highlights), `slow 320` (sheets, progress),
  `cable 520` (plug and unplug). Easing `standard` for most, `exit` for dismissals.
- Cables: unplug drops the cable to the floor with a bounce; plug-in travels to the port with a slight overshoot
  (`easeOutBack`). Both interpolate one cubic Bézier (`cablePath(c, t)` in `screens.js`).
- Progress bars move in steps the engine reports; never fake smooth progress.
- `prefers-reduced-motion`: all durations become 0; cables jump to their end state.

## Layout and information architecture

Window 1200 × 760 default, minimum 960 × 640. Merged title bar, traffic lights inset 18 px.

- **Sidebar** (220 px, vibrancy): Overview · Offload · Drives · Memory, then a "Drives" group listing each known
  drive with its dot, then Settings. Bottom: live CPU, Memory, Disk mini bars.
- **Overview:** memory forecast card (sentence, why, one action, swap chart) → four metric tiles (CPU, Memory,
  Disk, Network, live every 2 s) → This Mac capacity + Drives summary.
- **Offload:** free-space summary with "after moving" figure → table of folders with verdicts → trust caption.
- **Drives:** drive modules (each external drive + Macintosh HD) → cable map → one banner per missing drive.
- **Memory:** forecast card with a taller chart → apps using memory, each with Quit.
- **Settings:** grouped form (General, Moving, Memory, Drives, Privacy).
- **Onboarding** (first launch, no sidebar): Welcome (promises) → Permissions → Choose a drive.
- **Menu-bar popover:** forecast sentence, four mini meters, drives with status, primary "Open Stowaway".

## Components

Each entry: what it is, its states, and its rules. Names match the prototype functions.

### `sidebar` / nav item
Row 28 px, icon 16 px in `accent.text`, label `body`. Active: `line.strong` background, `aria-current="page"`.
Badge on the right: a count in mono `caption`, or a status (`1 missing` in `danger.text`, a warn dot for strained
memory). No badges for things that need no action.

### `button`
Variants: default (raised, `elevation.1`), `primary` (accent fill, white semibold), `danger` (only for actions that
lose data, which v1 has none of), `plain` (accent text, for "Why?" and "Stop"). Sizes: regular 24 px, `large` 30 px
for onboarding. One primary button per view or dialog. Labels are verbs; an ellipsis means a dialog follows
("Move…"). Disabled buttons keep a `title` saying why, and the reason is also shown as text next to them.

### `seg` (segmented control)
Inset track, raised selected segment, `aria-pressed`. Optional mono count per segment. Only when every segment
filters the same list.

### `pill`
State words only: `accent` "Can move", `neutral` "Use Docker Desktop's setting", `danger` "Can't move" / "Missing",
`ok` "On Samsung T7" / "Allowed", `warn` "Strained". Always followed by a caption with the reason.

### `capacity` (bar)
Plain bar, 8 px (14 px large), parts in a fixed order: system, apps, movable (accent) or app data (drive colour),
other, free (track). Always paired with numbers in text; has an `aria-label`.

### `metric` tile and `sparkline`
Label (`group`), one `figure`, one caption line, then either a sparkline (CPU, network) or a capacity bar (memory,
disk). Disk tile turns `danger.text` under 40 GB free. Sparklines are 1.5 px lines, no fill, no axes.

### Forecast card + `forecastChart`
Sentence first ("Memory runs out in about 38 minutes."), then why in one or two sentences, then one action with
its effect ("Frees about 6.2 GB"). Right column: pressure % and swap trend. Chart: swap history solid, forecast
dashed (`warn.fill`, or `ok.fill` when calm) with an uncertainty band, a dashed `danger` line where swap has no
more room, and the predicted time. Calm state says so plainly and offers no button.

### Table row (`itemRow`)
Grid: icon · folder + app · path (mono, ellipsis, full path in `title`) · size (mono, right) · verdict (pill +
reason) · actions. Rows by verdict:
- **Can move:** "Move…"; if the app is open, primary "Quit & move…".
- **Use the app's setting:** "Show how" opens steps (Docker).
- **Can't move:** red pill, reason, "Why?" (MobileSync).
- **On a drive:** green pill "On Samsung T7", "Since 3 Nov, checked file by file", "Bring back…".
- **Held** (drive missing): red pill "Archive is missing", "Bring back…" disabled with the reason.
- **Busy:** accent tint, the path/size/verdict cells become a progress cell: four steps (Copy, Check every file,
  Link, Remove original; reversed wording for bring back), a progress bar, bytes done, and "original kept until the
  check passes". Action: "Stop".

### Drive `module`
Swatch in the drive colour, name, format and bus, eject button (connected) or "Missing" pill. Capacity bar with
app data in the drive colour. Footer: free space, and folder count + size. Missing: dashed `danger` outline, last
seen time.

### Cable map (`cableMap`)
Left: apps that keep data on a drive (real icon, name, size, or "held, drive missing" in `danger.text`) with a jack.
Right: one block per drive with a port per app. Cables in the drive colour, 4.5 px, round caps, sagging by distance.
Hover an app or a drive: its cables go to 6 px, the rest fade to 25 %. Missing drive: dashed block, its cables hang
to the floor with the plug lying loose. The SVG has an `aria-label` listing every app → drive pair; the Drives list
in the sidebar and the banners carry the same facts as text.

### `banner`
Full-width, tinted (`danger`, `warn`, `info`), icon, bold sentence, one supporting sentence, actions on the right
with an optional reason under them. `role="alert"` for danger. One banner per problem, never stacked
for the same cause.

### `dialog` (sheet)
Drops from the title bar (macOS sheet), 440 px, `elevation.4`, overlay behind. Title is the question ("Move Xcode
derived data to Samsung T7?"). Body lists what will happen as numbered steps, then the outcome ("This Mac will have
53 GB free."). Buttons: Cancel, then the primary verb that repeats the action. Escape and clicking outside cancel.
Refusal dialogs ("Not enough room on this Mac") say what was checked, the numbers, and "Nothing was changed",
with a single OK.

### `empty` (empty, loading and error states)
Centered: illustration or 48 px icon, `headline` title, one or two sentences, at most two buttons.
- **Empty:** says what will appear here and how to get there ("Connect a drive to start").
- **Loading:** skeleton blocks in the final layout plus one real progress fact ("1,284 checked"); never a spinner alone.
- **Error:** names what failed, says nothing was changed, offers the fix ("Open System Settings…").

### Switch, select, range
Native look: 32 × 18 switch with accent when on; selects on a raised background. Settings rows: title, optional
caption, control on the right; 44 px minimum height. Safety guarantees show as text ("Always on"), not as a
disabled switch.

### Menu-bar popover
340 px, `elevation.3`. Menu-bar item: the cable glyph, plus minutes left when memory is strained ("38m").
Content: status dot + forecast sentence + one-line advice, four mini meters, drives list, then the advised action
and "Open Stowaway".

## Accessibility
- Full keyboard use: Tab order follows reading order; Escape closes sheets; focus goes to the primary button when a
  sheet opens and returns to the trigger after.
- Visible focus ring (2 px `accent.text`). Hit targets ≥ 24 px.
- Every chart and the cable map has a text alternative; every colour state is also a word.
- Respect Reduce Motion, Increase Contrast (swap `line.hairline` for `line.strong`) and Reduce Transparency
  (sidebar becomes `bg.surface`).

## Data the UI needs from the core
Shapes are in `prototype/data.js`. In short:
- **Folder:** id, name, owning app + bundle path (for the icon), path, size, verdict (`move | setting | refused |
  stowed`), reason (short), long explanation for refusals, setting steps for `setting`, drive id + date when stowed,
  whether the owning app is running.
- **Drive:** id, name, format + bus, total/free, colour, connected, last seen.
- **Job:** folder id, kind (`move | back`), current step (0–3), bytes done/total, target drive.
- **Forecast:** condition (`fine | strained | critical`), minutes left, predicted time, swap now + rate, free disk,
  swap history (1 per minute, last hour), the suggested app to quit and memory it frees.
- **Metrics:** CPU %, memory used/total, pressure %, swap, compressed, disk free/total, network in/out; every 2 s.
- **Apps using memory:** app, bundle path, memory, one-line detail.
