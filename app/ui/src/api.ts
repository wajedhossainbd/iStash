// The one door between the UI and the data. Screens never fetch, read files or call Tauri for data themselves;
// they go through `api`. Today `api` is the mock below (built from design/prototype/data.js); integration swaps it
// for a client of the agent socket (docs/agent-api.md) with the same interface.
// Units: bytes for sizes, bytes/s for rates, unix milliseconds for times.

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
  swapLimit: number; // swap can't grow past this (free disk)
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
}

export interface Permissions {
  volumes: boolean;
  notify: boolean;
}

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
  /** Data URL of an app's real icon, or null (CLI tools, missing apps, not running in the app). */
  appIcon(appPath: string): Promise<string | null>;
}

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
    settings: { onboarded: false, theme: 'system', login: true, menubar: true, dryRun: true, warnAt: 30, ejectGuard: true, updates: true, defaultDrive: null },
    permissions: { volumes: false, notify: false },
    scan: { running: false, checked: 0 },
    journalPath: '~/Library/Application Support/Stowaway/journal.jsonl',
  };
}

const history = (from: number, to: number) => Array.from({ length: 56 }, (_, i) => (from + (to - from) * (i / 55) ** 2.2) * GB);
const strained = (now: number): Forecast => ({
  condition: 'strained', minutesLeft: 38, at: now + 38 * 60e3, swap: 9.4 * GB, swapRate: 6.3 * GB, swapLimit: 22 * GB,
  history: history(3.1, 9.4), suggest: { appId: 'docker', app: 'Docker Desktop', frees: 6.2 * GB },
});
const fine = (prev: Forecast): Forecast => ({ ...prev, condition: 'fine', minutesLeft: null, at: null, swap: 8.1 * GB, swapRate: -1.3 * GB, suggest: undefined });

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
    async eject(id) { await ok(); this.setConnected(id, false); },
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
const mock = createMock(scenario);
// Dev only: lets a tester plug drives in and out from the web inspector (`stowawayMock.setConnected('archive', true)`).
(globalThis as any).stowawayMock = mock;
export const api: Api = mock;
