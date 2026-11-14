// The one door between the UI and the data. Screens never fetch, read files or call Tauri for data themselves;
// they go through `api`. In the app that is real.ts (the shell's commands, which run stowaway-core); in a browser
// and in tests it is the mock below (built from design/prototype/data.js). Both have the same interface.
// Units: bytes for sizes, bytes/s for rates, unix milliseconds for times.

import { createReal } from './real.ts';

export type Verdict = 'move' | 'setting' | 'refused' | 'stowed';
export type Cable = 'blue' | 'orange' | 'violet' | 'green' | 'pink' | 'teal';
export type Theme = 'system' | 'light' | 'dark';

export interface Drive {
  id: string;
  name: string;
  format: string; // 'APFS', 'Mac OS Extended', 'exFAT'
  bus: string; // 'USB-C', 'Thunderbolt'
  total: number;
  free: number;
  cable: Cable; // assigned on first use, never reused while the drive is known
  connected: boolean;
  lastSeen: number;
  /** Why Stowaway won't write to it (exFAT, needs First Aid); absent when usable. */
  problem?: string;
  /** Formats that can never hold links; such drives are listed only so onboarding can explain the refusal. */
  refused?: boolean;
}

export interface Folder {
  id: string;
  name: string; // 'Models'
  app: string; // who owns the data
  appPath?: string; // bundle path, for the real icon; absent for CLI tools
  path: string; // under home, '~/...'
  size: number;
  verdict: Verdict;
  reason: string; // one short line
  why?: string; // long explanation when refused
  how?: string[]; // steps when the app's own setting should move it
  drive?: string; // when stowed
  since?: number; // when stowed
  running: boolean; // the owning app is open
}

export interface Job {
  folder: string;
  kind: 'move' | 'back';
  step: 0 | 1 | 2 | 3; // copy, check every file, link, remove original (reversed for back)
  done: number;
  total: number;
  drive: string;
}

export interface Mac {
  model: string;
  total: number;
  free: number;
  system: number;
  apps: number;
  /** Stowaway keeps this much free on the Mac; a bring-back that would cut into it is refused. */
  reserve: number;
}

export interface Forecast {
  condition: 'fine' | 'strained' | 'critical';
  minutesLeft: number | null;
  at: number | null; // predicted time swap runs out of room
  swap: number;
  swapRate: number; // bytes per hour, negative when falling
  swapLimit: number; // swap can't grow past this (what macOS will give it from free disk)
  history: number[]; // swap, one sample per minute, oldest first, last hour
  suggest?: { appId: string; app: string; frees: number };
}

export interface Metrics {
  at: number;
  cpu: number[]; // percent, last samples, every 2 s
  cores: string;
  memUsed: number;
  memTotal: number;
  pressure: number; // percent
  swap: number;
  compressed: number;
  netIn: number[]; // bytes/s, last samples
  netOut: number;
  netName: string;
}

export interface MemApp {
  id: string;
  app: string;
  appPath?: string;
  mem: number;
  detail: string;
}

export interface Settings {
  onboarded: boolean;
  theme: Theme;
  login: boolean;
  menubar: boolean;
  dryRun: boolean;
  warnAt: number; // minutes
  ejectGuard: boolean;
  updates: boolean;
  defaultDrive: string | null;
  cleanEvery: 'off' | 'daily' | 'weekly' | 'monthly' | 'over2' | 'over5' | 'over10'; // overN: when the ticked kinds pass N GB
  cleanAge: number;
  widget: 'off' | 'auto' | 'pinned'; // the glass panel on a screen edge
  widgetSide: 'left' | 'right'; // days an item must be untouched before it may be removed; 0 = any age
  cleanKinds: string[]; // which kinds the Clean screen has ticked; the schedule cleans the same ones
}

export interface Permissions {
  volumes: boolean;
  notify: boolean;
}

export interface CleanItem { id: string; name: string; note: string; count: number; size: number }

export interface State {
  version: string;
  mac: Mac;
  drives: Drive[];
  folders: Folder[];
  jobs: Job[];
  /** null while the first five minutes of samples are collected. */
  forecast: Forecast | null;
  memApps: MemApp[];
  settings: Settings;
  permissions: Permissions;
  scan: { running: boolean; checked: number };
  journalPath: string;
  /** available: a newer version's number once a check found one. */
  update: { available: string | null; busy: boolean; checked: number | null };
  /** The active connection, and the last speed test (run only when asked). */
  net?: { info: NetInfo | null; test: SpeedTest | null; testing: boolean; history: SpeedTest[]; server: string; serverLabel?: string; mode: 'full' | 'down' | 'up'; servers: SpeedServer[]; live: SpeedLive | null };
  /** Read and write activity per drive id. */
  io?: Record<string, DriveIo>;
  /** App bundle path Focus is on, or null. */
  focus: string | null;
  clean: { items: CleanItem[] | null; busy: boolean; freed: number | null; last: number | null; memBusy: boolean; memFreed: number | null };
  loginNeedsApproval?: boolean; // macOS waits for the user to allow Stowaway in Login Items
  unsupported?: (keyof Settings)[]; // settings this build cannot act on; shown as such instead of a dead switch
}

/** A dry run: what an action would do, or why it is refused. Nothing is changed by asking. */
export type Plan =
  | { ok: true; size: number; drive: Drive; quits: string | null; macFreeAfter: number }
  | { ok: false; title: string; reason: string };

export type ApiErrorCode = 'helper-down';
export class ApiError extends Error {
  code: ApiErrorCode;
  constructor(code: ApiErrorCode, message: string) {
    super(message);
    this.code = code;
  }
}

export interface Api {
  /** Full state. Rejects with ApiError('helper-down') when the agent can't be reached. */
  state(): Promise<State>;
  /** Called with the new state whenever it changes (not on every poll). Returns unsubscribe. */
  subscribe(fn: (s: State) => void): () => void;
  /** Live numbers, every 2 s. */
  metrics(): Promise<Metrics>;
  planMove(folder: string, drive: string): Promise<Plan>;
  move(folder: string, drive: string): Promise<void>;
  planBack(folder: string): Promise<Plan>;
  bringBack(folder: string): Promise<void>;
  /** Stop a running job; the engine rolls back to the original. */
  stop(folder: string): Promise<void>;
  eject(drive: string): Promise<void>;
  quitApp(appId: string): Promise<void>;
  scan(): Promise<void>;
  set<K extends keyof Settings>(key: K, value: Settings[K]): Promise<void>;
  requestPermission(p: keyof Permissions): Promise<void>;
  startHelper(): Promise<void>;
  /** Measures what the cleaner could remove. Changes nothing. */
  cleanScan(): Promise<void>;
  /** Removes the given kinds of temporary files for good. */
  clean(kinds: string[]): Promise<void>;
  /** Measures internet speed against Cloudflare and looks up the public address and provider. */
  speedTest(): Promise<void>;
  /** '' for Cloudflare, or a download URL on a server of the user's choice. */
  setSpeedServer(url: string, label?: string): Promise<void>;
  setSpeedMode(mode: 'full' | 'down' | 'up'): Promise<void>;
  /** Nearest Speedtest servers, or those matching a search. */
  findServers(query: string): Promise<void>;
  /** Looks for a newer release. */
  checkUpdate(): Promise<void>;
  /** Downloads the release found by checkUpdate, installs it and restarts the app. */
  installUpdate(): Promise<void>;
  /** Makes macOS compress and page out idle apps' memory (no password). */
  freeMemory(): Promise<void>;
  /** Runs every other app in the background so this one gets the processor and disk first; null turns it off. */
  focus(appPath: string | null): Promise<void>;
  /** Data URL of an app's real icon, or null (CLI tools, missing apps, not running in the app). */
  appIcon(appPath: string): Promise<string | null>;
}

/** One mounted external volume, as the app shell's read-only `volumes` command reports it. */
export interface Volume { id: string; name: string; format: string; bus: string; total: number; free: number; writable: boolean; disk?: string }
/** Bytes per second now and averaged since Stowaway started, and when the drive last read or wrote. */
export interface NetInfo { iface: string; kind: string; ip: string; gateway: string }
export interface SpeedServer { host: string; sponsor: string; city: string; cc: string; km?: number }
export interface SpeedLive { phase: 'ping' | 'down' | 'up' | 'done'; mbps: number; ping: number | null; pct: number }
export interface SpeedTest { key?: string; mode?: 'full' | 'down' | 'up'; down: number | null; up: number | null; ping: number; jitter: number; ip: string; isp: string; city: string; country: string; at: number; server: string }
export interface DriveIo { read: number; write: number; avgRead: number; avgWrite: number; last: number | null }

const CABLES: Cable[] = ['blue', 'orange', 'violet', 'green', 'pink', 'teal'];
const HOLDS_LINKS = /^(APFS|Mac OS Extended)/;
/** Real volumes as drives. A drive that still holds folders stays listed (missing) when unplugged; others drop out. */
export function toDrives(vols: Volume[], prev: Drive[], holds: (id: string) => boolean, now = Date.now()): Drive[] {
  const used = new Set(prev.map((d) => d.cable));
  const here = [...vols].sort((a, b) => a.name.localeCompare(b.name)).map((v): Drive => {
    const old = prev.find((d) => d.id === v.id);
    const cable = old?.cable ?? CABLES.find((c) => !used.has(c)) ?? CABLES[used.size % CABLES.length];
    used.add(cable);
    const refused = !HOLDS_LINKS.test(v.format);
    const problem = refused ? `${v.format} can’t hold links safely. Stowaway can use it only after it is erased and reformatted as APFS in Disk Utility.`
      : v.writable ? undefined : 'macOS mounted it read-only, so Stowaway can’t write to it.';
    return { id: v.id, name: v.name, format: v.format, bus: v.bus, total: v.total, free: v.free, cable, connected: true,
      lastSeen: old?.connected ? old.lastSeen : now, ...(refused && { refused }), ...(problem && { problem }) };
  });
  const gone = prev.filter((d) => !vols.some((v) => v.id === d.id) && holds(d.id)).map((d) => ({ ...d, connected: false, lastSeen: d.connected ? now : d.lastSeen }));
  return [...here, ...gone];
}

/** What the app shell's read-only `memory` command returns (stowaway-core's Sampler and forecast). */
export interface Reading {
  t: number; cpu: number[]; cores: string; netIn: number[]; netOut: number;
  memTotal: number; memUsed: number; compressed: number; freePct: number; swap: number; diskFree: number; diskTotal?: number; swapLimit: number;
  history: number[]; level: 'ok' | 'watch' | 'warn' | 'critical'; minutesLeft: number | null; reason: string;
  apps: { name: string; mem: number; cpu: number; path: string | null }[];
}

/** The core's forecast in the UI's shape; null until there is at least a minute of swap history to draw. */
export function toForecast(r: Reading, now = Date.now()): Forecast | null {
  const h = r.history;
  if (h.length < 2) return null;
  const k = Math.min(11, h.length);
  const swapRate = ((h.at(-1)! - h.at(-k)!) / (k - 1)) * 60;
  const minutes = r.minutesLeft == null ? null : Math.round(r.minutesLeft);
  // The core says "watch" both for a slow trend and for low free memory with no trend; only a trend is a forecast.
  const condition = r.level === 'critical' ? 'critical' : r.level === 'warn' || (r.level === 'watch' && minutes != null) ? 'strained' : 'fine';
  const top = r.apps.find((a) => a.path);
  return {
    condition, minutesLeft: condition === 'fine' ? null : minutes, at: condition === 'fine' || minutes == null ? null : now + minutes * 60e3,
    swap: r.swap, swapRate, swapLimit: r.swapLimit, history: h,
    ...(top && condition !== 'fine' && { suggest: { appId: top.name, app: top.name, frees: top.mem } }),
  };
}

export const toMemApps = (r: Reading): MemApp[] => r.apps.map((a) => ({
  id: a.name, app: a.name, ...(a.path && { appPath: a.path }), mem: a.mem, detail: `${Math.round(a.cpu)}% CPU`,
}));

// ---------------------------------------------------------------------------------------------------------------
// Mock, from design/prototype/data.js. `?mock=loading|empty|error|noaccess|unreadable` picks a designed state.

const GB = 1e9;
const DAY = 864e5;
const at = (s: string) => new Date(`${s} 2026 12:00`).getTime();

export type Scenario = 'normal' | 'loading' | 'empty' | 'error' | 'noaccess' | 'unreadable';

export function mockState(now = Date.now()): State {
  const drives: Drive[] = [
    { id: 't7', name: 'Samsung T7', format: 'APFS', bus: 'USB-C', total: 1000 * GB, free: 412 * GB, cable: 'blue', connected: true, lastSeen: now },
    { id: 'archive', name: 'Archive', format: 'APFS', bus: 'USB', total: 2000 * GB, free: 1210 * GB, cable: 'orange', connected: false, lastSeen: now - 2 * DAY },
    { id: 'stick', name: 'USB Stick', format: 'exFAT', bus: 'USB', total: 64 * GB, free: 50 * GB, cable: 'teal', connected: true, lastSeen: now, refused: true,
      problem: 'exFAT can’t hold links safely. Reformat it as APFS in Disk Utility to use it.' },
  ];
  const f = (o: Omit<Folder, 'size' | 'running' | 'reason'> & { size: number; running?: boolean; reason?: string }): Folder =>
    ({ running: false, reason: '', ...o, size: o.size * GB });
  const folders: Folder[] = [
    f({ id: 'ollama', name: 'Models', app: 'Ollama', appPath: '/Applications/Ollama.app', path: '~/.ollama/models', size: 48.2, verdict: 'move', running: true, reason: 'Ollama is open' }),
    f({ id: 'derived', name: 'Derived data', app: 'Xcode', appPath: '/Applications/Xcode.app', path: '~/Library/Developer/Xcode/DerivedData', size: 21.7, verdict: 'move', reason: 'Rebuilt if the drive is away' }),
    f({ id: 'gradle', name: 'Gradle caches', app: 'Gradle', path: '~/.gradle/caches', size: 7.3, verdict: 'move', reason: 'Cache, safe to move' }),
    f({ id: 'docker', name: 'Disk image', app: 'Docker Desktop', appPath: '/Applications/Docker.app', path: '~/Library/Containers/com.docker.docker/Data', size: 59.6, verdict: 'setting',
      reason: 'A link here can corrupt data',
      how: ['Quit nothing yet. Open Docker Desktop.', 'Go to Settings, then Resources, then Advanced.', 'Set "Disk image location" to a folder on your drive and press Apply.', 'Docker moves the image itself and keeps working.'] }),
    f({ id: 'mobilesync', name: 'iPhone backups', app: 'Finder', appPath: '/System/Library/CoreServices/Finder.app', path: '~/Library/Application Support/MobileSync/Backup', size: 28.4, verdict: 'refused',
      reason: 'macOS ignores links here',
      why: 'Since macOS 10.15, Finder no longer follows a link for iPhone backups. It would quietly start a new backup on this Mac. Move old backups by hand from Finder (Manage Backups) instead.' }),
    f({ id: 'hf', name: 'Cache', app: 'Hugging Face', path: '~/.cache/huggingface', size: 33.1, verdict: 'stowed', drive: 't7', since: at('3 Nov') }),
    f({ id: 'devsupport', name: 'iOS Device Support', app: 'Xcode', appPath: '/Applications/Xcode.app', path: '~/Library/Developer/Xcode/iOS DeviceSupport', size: 18.6, verdict: 'stowed', drive: 't7', since: at('28 Oct') }),
    f({ id: 'cowork', name: 'Cowork VM', app: 'Claude', appPath: '/Applications/Claude.app', path: '~/Library/Application Support/Claude/vm', size: 12.4, verdict: 'stowed', drive: 't7', since: at('3 Nov'), running: true }),
    f({ id: 'hermes', name: 'Data', app: 'hermes', path: '~/.hermes', size: 3.8, verdict: 'stowed', drive: 't7', since: at('1 Nov') }),
    f({ id: 'lmstudio', name: 'Models', app: 'LM Studio', appPath: '/Applications/LM Studio.app', path: '~/.lmstudio/models', size: 61.0, verdict: 'stowed', drive: 'archive', since: at('19 Oct') }),
  ];
  const memApps: MemApp[] = [
    { id: 'docker', app: 'Docker Desktop', appPath: '/Applications/Docker.app', mem: 6.2 * GB, detail: '3 containers running' },
    { id: 'chrome', app: 'Google Chrome', appPath: '/Applications/Google Chrome.app', mem: 3.1 * GB, detail: '41 tabs' },
    { id: 'xcode', app: 'Xcode', appPath: '/Applications/Xcode.app', mem: 2.4 * GB, detail: 'Indexing' },
    { id: 'claude', app: 'Claude', appPath: '/Applications/Claude.app', mem: 1.6 * GB, detail: 'Cowork VM running' },
    { id: 'ollama', app: 'Ollama', appPath: '/Applications/Ollama.app', mem: 0.9 * GB, detail: 'No model loaded' },
  ];
  return {
    version: '0.1.0',
    mac: { model: 'MacBook Pro', total: 494 * GB, free: 31.2 * GB, system: 104 * GB, apps: 153 * GB, reserve: 10 * GB },
    drives, folders, jobs: [], memApps,
    forecast: strained(now),
    settings: { onboarded: false, theme: 'dark', login: true, menubar: true, dryRun: true, warnAt: 30, ejectGuard: true, updates: true, defaultDrive: null, cleanEvery: 'off', cleanAge: 7, widget: 'off', widgetSide: 'right', cleanKinds: ['caches', 'logs', 'temp'] },
    permissions: { volumes: false, notify: false },
    scan: { running: false, checked: 0 },
    journalPath: '~/Library/Application Support/Stowaway/journal.jsonl',
    update: { available: null, busy: false, checked: null },
    net: { info: { iface: 'en0', kind: 'Wi-Fi', ip: '192.168.1.24', gateway: '192.168.1.1' }, test: null, testing: false, history: [], server: '', mode: 'full', live: null,
      servers: [{ host: 'speedtest.example.net:8080', sponsor: 'Example Fibre', city: 'Dhaka', cc: 'BD', km: 4 }, { host: 'st2.example.net:8080', sponsor: 'Metro Net', city: 'Narayanganj', cc: 'BD', km: 12 }] },
    focus: null, clean: { items: null, busy: false, freed: null, last: null, memBusy: false, memFreed: null },
  };
}

const history = (from: number, to: number) => Array.from({ length: 56 }, (_, i) => (from + (to - from) * (i / 55) ** 2.2) * GB);
// The mock forecast is derived from its own swap curve, so the sentence, the rate and the chart always agree.
const LIMIT = 22, FROM = 3.1, TO = 9.4, EXP = 2.2;
const strained = (now: number): Forecast => {
  const h = history(FROM, TO);
  // Same curve, continued until it meets the limit.
  const minutesLeft = Math.round(55 * ((LIMIT - FROM) / (TO - FROM)) ** (1 / EXP) - 55);
  return {
    condition: 'strained', minutesLeft, at: now + minutesLeft * 60e3, swap: h.at(-1)!, swapRate: (h.at(-1)! - h.at(-21)!) * 3, swapLimit: LIMIT * GB,
    history: h, suggest: { appId: 'docker', app: 'Docker Desktop', frees: 6.2 * GB },
  };
};
// After the suggested app quits: swap falls over the last five samples.
const fine = (prev: Forecast): Forecast => {
  const h = [...prev.history.slice(5), ...[9.1, 8.8, 8.5, 8.3, 8.1].map((x) => x * GB)];
  return { ...prev, condition: 'fine', minutesLeft: null, at: null, history: h, swap: h.at(-1)!, swapRate: (h.at(-1)! - h.at(-6)!) * 12, suggest: undefined };
};

export function createMock(scenario: Scenario = 'normal', onboarded = scenario !== 'normal'): Api & { setConnected(id: string, on: boolean): void; state0: State } {
  const s = mockState();
  s.settings.onboarded = onboarded;
  s.permissions.volumes = onboarded;
  // The mock keeps settings across launches the way the agent will; the designed-state scenarios start clean.
  const KEY = 'stowaway.mock';
  try { if (scenario === 'normal') Object.assign(s, JSON.parse(localStorage.getItem(KEY) || '{}')); } catch { /* fresh start */ }
  const save = () => { try { localStorage.setItem(KEY, JSON.stringify({ settings: s.settings, permissions: s.permissions })); } catch { /* not kept */ } };
  if (scenario === 'empty') { s.drives = s.drives.filter((d) => d.refused); s.folders = s.folders.filter((x) => x.verdict !== 'stowed'); }
  if (scenario === 'noaccess') s.permissions.volumes = false;
  if (scenario === 'unreadable') s.drives[0].problem = 'macOS reports the disk needs First Aid.';
  if (scenario === 'loading') { s.scan = { running: true, checked: 1284 }; s.forecast = null; }
  let helperUp = scenario !== 'error';
  const subs = new Set<(s: State) => void>();
  const emit = () => subs.forEach((fn) => fn(structuredClone(s)));
  const folder = (id: string) => s.folders.find((x) => x.id === id)!;
  const drive = (id: string) => s.drives.find((x) => x.id === id)!;
  const live = { cpu: [22, 25, 31, 28, 34, 30, 36, 33, 29, 34], net: [1.2, 2.4, 0.8, 3.9, 4.2, 2.2, 1.1, 5.0, 3.3, 4.2].map((x) => x * 1e6) };
  const ok = async () => { if (!helperUp) throw new ApiError('helper-down', 'Stowaway’s helper isn’t running.'); };

  // Jobs advance in the steps the engine reports, about once a second.
  const ticker = setInterval(() => {
    if (!s.jobs.length) return;
    for (const j of [...s.jobs]) {
      j.done = Math.min(j.total, j.done + 6 * GB);
      const pct = j.done / j.total;
      j.step = pct < 0.6 ? 0 : pct < 0.9 ? 1 : pct < 0.97 ? 2 : 3;
      if (j.done < j.total) continue;
      const it = folder(j.folder), d = drive(j.drive);
      if (j.kind === 'move') { Object.assign(it, { verdict: 'stowed', drive: d.id, since: Date.now() }); s.mac.free += it.size; d.free -= it.size; }
      else { Object.assign(it, { verdict: 'move', reason: 'Brought back today', drive: undefined, since: undefined }); s.mac.free -= it.size; d.free += it.size; }
      s.jobs = s.jobs.filter((x) => x !== j);
    }
    emit();
  }, 1000) as any;
  ticker.unref?.(); // lets the node tests exit

  const planMove = (id: string, driveId: string): Plan => {
    const it = folder(id), d = drive(driveId);
    if (!d || !d.connected || d.refused || d.problem) return { ok: false, title: 'No drive connected', reason: 'Plug in an APFS drive to move this folder. Nothing was changed.' };
    if (it.size > d.free) return { ok: false, title: `Not enough room on ${d.name}`, reason: `${d.name} has ${fmt(d.free)} free and this folder needs ${fmt(it.size)}. Nothing was changed.` };
    return { ok: true, size: it.size, drive: d, quits: it.running ? it.app : null, macFreeAfter: s.mac.free + it.size };
  };
  const planBack = (id: string): Plan => {
    const it = folder(id), d = drive(it.drive!);
    if (!d.connected) return { ok: false, title: `${d.name} is missing`, reason: `Connect ${d.name} first. Nothing was changed.` };
    if (it.size > s.mac.free - s.mac.reserve) return { ok: false, title: 'Not enough room on this Mac',
      reason: `${it.app} ${it.name.toLowerCase()} needs ${fmt(it.size)}. This Mac has ${fmt(s.mac.free)} free, and Stowaway keeps ${fmt(s.mac.reserve)} spare so macOS can keep working. Nothing was changed.` };
    return { ok: true, size: it.size, drive: d, quits: it.running ? it.app : null, macFreeAfter: s.mac.free - it.size };
  };
  const start = (id: string, kind: Job['kind'], driveId: string) => {
    const p = kind === 'move' ? planMove(id, driveId) : planBack(id);
    if (!p.ok) throw new Error(p.reason);
    folder(id).running = false;
    s.jobs.push({ folder: id, kind, step: 0, done: 0, total: folder(id).size, drive: driveId });
    emit();
  };

  return {
    state0: s,
    async state() { await ok(); return structuredClone(s); },
    subscribe(fn) { subs.add(fn); return () => subs.delete(fn); },
    async metrics() {
      await ok();
      const walk = (a: number[], lo: number, hi: number, step: number) => { a.push(Math.max(lo, Math.min(hi, a.at(-1)! + (Math.random() - 0.5) * step))); a.shift(); };
      walk(live.cpu, 8, 92, 12); live.cpu[live.cpu.length - 1] = Math.round(live.cpu.at(-1)!);
      walk(live.net, 0.2e6, 6e6, 2.4e6);
      const calm = s.forecast?.condition === 'fine';
      return { at: Date.now(), cpu: [...live.cpu], cores: '8 performance, 4 efficiency cores', memUsed: (calm ? 7.6 : 13.8) * GB, memTotal: 16 * GB,
        pressure: calm ? 54 : 72, swap: s.forecast?.swap ?? 9.4 * GB, compressed: 3.2 * GB, netIn: [...live.net], netOut: 310e3, netName: 'Wi-Fi' };
    },
    async planMove(id, d) { await ok(); return planMove(id, d); },
    async move(id, d) { await ok(); start(id, 'move', d); },
    async planBack(id) { await ok(); return planBack(id); },
    async bringBack(id) { await ok(); start(id, 'back', folder(id).drive!); },
    async stop(id) { await ok(); s.jobs = s.jobs.filter((j) => j.folder !== id); emit(); },
    async eject(id) {
      await ok();
      const j = s.jobs.find((x) => x.drive === id);
      if (j) throw new Error(`${drive(id).name} is in use: ${folder(j.folder).app} ${folder(j.folder).name.toLowerCase()} is being ${j.kind === 'move' ? 'moved to' : 'brought back from'} it. Nothing was changed.`);
      this.setConnected(id, false);
    },
    async quitApp(appId) {
      await ok();
      s.memApps = s.memApps.filter((a) => a.id !== appId);
      s.folders.forEach((x) => { if (x.appPath && s.memApps.every((a) => a.appPath !== x.appPath)) x.running = false; });
      if (s.forecast?.suggest?.appId === appId) s.forecast = fine(s.forecast);
      emit();
    },
    async scan() { await ok(); s.scan = { running: true, checked: 0 }; emit(); setTimeout(() => { s.scan = { running: false, checked: 1284 }; emit(); }, 1600); },
    async set(key, value) { await ok(); s.settings[key] = value; save(); emit(); },
    async requestPermission(p) { await ok(); s.permissions[p] = true; save(); emit(); },
    async startHelper() { helperUp = true; emit(); },
    async cleanScan() {
      await ok();
      s.clean.items = [['caches', 'App caches', 'Apps rebuild them.', 41, 6.4], ['logs', 'Logs', 'Diagnostic logs.', 18, 0.3],
        ['derived', 'Xcode build files', 'Xcode rebuilds them.', 7, 11.2], ['temp', 'Old temporary files', 'Left behind by apps.', 96, 0.8]]
        .map(([id, name, note, count, size]) => ({ id, name, note, count, size: (size as number) * GB }) as CleanItem);
      emit();
    },
    async clean(kinds) {
      await ok();
      const hit = (s.clean.items ?? []).filter((i) => kinds.includes(i.id));
      s.clean.freed = hit.reduce((n, i) => n + i.size, 0); s.clean.last = Date.now(); s.mac.free += s.clean.freed;
      hit.forEach((i) => { i.size = 0; i.count = 0; });
      emit();
    },
    async focus(appPath) { await ok(); s.focus = appPath; emit(); },
    async freeMemory() { await ok(); s.clean.memFreed = 1.8 * GB; emit(); },
    async speedTest() { await ok(); const t: SpeedTest = { mode: 'full', down: 182.4, up: 41.7, ping: 12, jitter: 2, ip: '203.0.113.7', isp: 'Example Fibre', city: 'Dhaka', country: 'BD', at: Date.now(), server: 'Cloudflare' }; s.net = { ...s.net!, testing: false, test: t, history: [t, ...s.net!.history].slice(0, 20) }; emit(); },
    async setSpeedServer(url, label) { await ok(); s.net!.server = url; s.net!.serverLabel = label; emit(); },
    async setSpeedMode(mode) { await ok(); s.net!.mode = mode; emit(); },
    async findServers() { await ok(); },
    async checkUpdate() { await ok(); s.update = { available: '0.2.0', busy: false, checked: Date.now() }; emit(); },
    async installUpdate() { await ok(); s.version = s.update.available ?? s.version; s.update.available = null; emit(); },
    async appIcon(path) {
      // Icons are a UI concern, read from the bundle by the app shell (never from app sandboxes).
      const tauri = (window as any).__TAURI_INTERNALS__;
      if (!tauri) return null;
      return tauri.invoke('app_icon', { path }).then((png: ArrayBuffer) => URL.createObjectURL(new Blob([png], { type: 'image/png' })), () => null);
    },
    setConnected(id, on) {
      const d = drive(id);
      d.connected = on;
      if (on) d.lastSeen = Date.now();
      else s.folders.forEach((x) => { if (x.drive === id) x.running = false; });
      emit();
    },
  };
}

const fmt = (n: number) => `${n >= 100 * GB ? Math.round(n / GB) : (n / GB).toFixed(1)} GB`;

// shortcut: each window (main, menu-bar popover) runs its own mock, so their states can drift apart in a demo;
// goes away when both read the agent.
const scenario = (new URLSearchParams(globalThis.location?.search).get('mock') || 'normal') as Scenario;
const shell = (globalThis as any).__TAURI_INTERNALS__;
// In the app: the real engine. In a browser, in tests and for the designed states (?mock=…): the mock.
export const api: Api = shell && scenario === 'normal' ? createReal(shell.invoke) : createMock(scenario);
// Dev only: lets a tester plug drives in and out from the web inspector (`stowawayMock.setConnected('archive', true)`).
(globalThis as any).stowawayMock = api;
