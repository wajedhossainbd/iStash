// The real data source, used inside the app: every call goes to the shell's commands, which run stowaway-core.
// Same `Api` as the mock, so screens don't know the difference.
// shortcut: polls the shell (drives 5 s, memory 2 s, jobs 1 s); becomes pushes from the agent socket later.
import { ApiError, toDrives, toForecast, toMemApps } from './api.ts';
import type { Api, CleanItem, Drive, DriveIo, NetInfo, SpeedLive, SpeedServer, SpeedTest, Folder, Job, Plan, Reading, Settings, State, Verdict, Volume } from './api.ts';

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
interface Scanned { path: string; show: string; app: string; appPath: string | null; verdict: Verdict; note: string; size: number; drive: string | null; since: number | null; running: boolean; users: string[] }
interface ScanResult { folders: Scanned[]; checked: number; mac: { total: number; free: number }; journal: string }
interface EnginePlan { ok: boolean; size: number; quits: string[]; reason: string }
interface EngineJobs { running: { path: string; kind: Job['kind']; drive: string; done: number; total: number; step: Job['step'] }[]; finished: { path: string; error: string | null }[] }

const GB = 1e9;
const KEY = 'stowaway.mock'; // same key the mock used, so settings chosen before carry over
const read = <T>(key: string, fallback: T): T => { try { return { ...fallback, ...JSON.parse(localStorage.getItem(key) || '{}') }; } catch { return fallback; } };
const write = (key: string, v: unknown) => { try { localStorage.setItem(key, JSON.stringify(v)); } catch { /* not kept */ } };

export function toFolder(f: Scanned): Folder {
  return {
    id: f.path, name: f.path.split('/').pop()!, app: f.app, ...(f.appPath && { appPath: f.appPath }), path: f.show, size: f.size, verdict: f.verdict,
    reason: f.verdict === 'stowed' ? '' : f.note, ...(f.verdict === 'refused' && { why: f.note }), ...(f.verdict === 'setting' && { how: [f.note] }),
    ...(f.drive && { drive: f.drive }), ...(f.since && { since: f.since }), running: f.running,
  };
}

export function createReal(invoke: Invoke): Api {
  // The popover and the edge widget only show numbers: they leave scanning, jobs and schedules to the main window.
  const light = /^#\/(popover|widget)/.test(location.hash);
  const isPopover = location.hash.startsWith('#/popover');
  let ticks: number[] | null = null, bootDisk = '';
  const saved = read(KEY, { settings: {} as Partial<Settings>, permissions: {} });
  const s: State = {
    version: '0.1.0',
    mac: { model: 'This Mac', total: 1, free: 0, system: 0, apps: 0, reserve: 10 * 2 ** 30 },
    drives: [], folders: [], jobs: [], forecast: null, memApps: [],
    settings: { onboarded: false, theme: 'dark', login: false, menubar: true, dryRun: true, warnAt: 30, ejectGuard: true, updates: false, defaultDrive: null, cleanEvery: 'off', cleanAge: 7, widget: 'off', widgetSide: 'right', cleanKinds: ['caches', 'logs', 'temp'], ...saved.settings },
    // macOS asks for these itself the first time they are needed; there is nothing to pre-grant.
    permissions: { volumes: true, notify: false, ...saved.permissions },
    scan: { running: true, checked: 0 },
    journalPath: '~/Library/Application Support/Stowaway/journal.jsonl',
    update: { available: null, busy: false, checked: null },
    net: { info: null, testing: false, servers: [], live: null, ...read<{ test: SpeedTest | null; history: SpeedTest[]; server: string; serverLabel?: string; mode: 'full' | 'down' | 'up' }>('stowaway.net', { test: null, history: [], server: '', mode: 'full' }) },
    focus: null, clean: { items: null, busy: false, freed: null, memBusy: false, memFreed: null, last: read<{ last: number | null }>('stowaway.clean', { last: null }).last },
  };
  const subs = new Set<(s: State) => void>();
  const emit = () => subs.forEach((fn) => fn(structuredClone(s)));
  const save = () => write(KEY, { settings: s.settings, permissions: s.permissions });
  // Another window (the popover's toggles) changed a setting: take it, so theme and widget agree everywhere.
  globalThis.addEventListener?.('storage', (e) => { if (e.key === KEY && e.newValue) { try { Object.assign(s.settings, JSON.parse(e.newValue).settings); emit(); } catch { /* ignore */ } } });
  const folder = (id: string) => s.folders.find((x) => x.id === id);
  let reading: Reading | null = null;
  let warned = false;
  let vols: Volume[] = [];
  const seen = read<Record<string, number>>('stowaway.seen', {});

  const refreshDrives = () => {
    const now = Date.now();
    // A drive that holds moved folders but is absent (even since before launch) is listed as missing.
    const known = [...s.drives];
    for (const f of s.folders) {
      if (f.drive && !known.some((d) => d.id === f.drive)) known.push({ id: f.drive, name: f.drive, format: '', bus: '', total: 1, free: 0, cable: 'blue', connected: false, lastSeen: seen[f.drive] ?? f.since ?? now });
    }
    const taken = new Set<string>();
    known.forEach((d) => { if (taken.has(d.cable)) d.cable = (['blue', 'orange', 'violet', 'green', 'pink', 'teal'] as const).find((c) => !taken.has(c)) ?? d.cable; taken.add(d.cable); });
    const next = toDrives(vols, known, (id) => s.folders.some((x) => x.drive === id), now);
    next.forEach((d) => { if (d.connected) seen[d.id] = now; });
    write('stowaway.seen', seen);
    const same = (a: Drive[], b: Drive[]) => JSON.stringify(a.map((d) => [d.id, d.connected, d.free, d.problem])) === JSON.stringify(b.map((d) => [d.id, d.connected, d.free, d.problem]));
    if (!same(next, s.drives)) { s.drives = next; emit(); }
  };
  const pollDrives = () => invoke<(Volume & { name: string })[]>('volumes').then((v) => { vols = v.map((x) => ({ ...x, id: x.name })); refreshDrives(); }, () => { /* keep the last list */ });

  let scanning: Promise<void> | null = null;
  const scan = (quiet = false) => scanning ??= (async () => {
    if (!quiet) { s.scan = { running: true, checked: s.scan.checked }; emit(); }
    try {
      const r = await invoke<ScanResult>('scan_home');
      s.folders = r.folders.map(toFolder);
      s.mac = { ...s.mac, ...r.mac };
      s.journalPath = r.journal.replace(/^\/Users\/[^/]+/, '~');
      s.scan = { running: false, checked: r.checked };
      write('stowaway.scan', r);
    } finally { s.scan.running = false; scanning = null; refreshDrives(); emit(); }
  })();

  // Jobs: each move()/bringBack() promise settles when its job ends, so a failure reaches the caller's error sheet.
  const waiting = new Map<string, { resolve(): void; reject(e: Error): void }>();
  let jobTimer: ReturnType<typeof setInterval> | undefined;
  const pollJobs = async () => {
    const r = await invoke<EngineJobs>('jobs');
    s.jobs = r.running.map((j) => ({ folder: j.path, kind: j.kind, step: j.step, done: j.done, total: j.total, drive: j.drive }));
    for (const f of r.finished) {
      const w = waiting.get(f.path);
      waiting.delete(f.path);
      if (f.error) w?.reject(new Error(f.error)); else w?.resolve();
    }
    if (r.finished.length) scan(true);
    if (!r.running.length) { clearInterval(jobTimer); jobTimer = undefined; }
    emit();
  };
  const start = async (kind: Job['kind'], id: string, drive?: string) => {
    await invoke('start_job', { kind, path: id, drive: drive ?? null, quit: !!folder(id)?.running });
    const done = new Promise<void>((resolve, reject) => waiting.set(id, { resolve, reject }));
    await pollJobs();
    jobTimer ??= setInterval(() => pollJobs().catch(() => { /* next tick */ }), 1000);
    return done;
  };
  const plan = async (kind: Job['kind'], id: string, driveId?: string): Promise<Plan> => {
    const p = await invoke<EnginePlan>('plan', { kind, path: id, drive: driveId ?? null });
    if (!p.ok) return { ok: false, title: kind === 'move' ? 'This folder can’t be moved right now' : 'This folder can’t be brought back right now', reason: `${p.reason} Nothing was changed.` };
    const drive = s.drives.find((d) => d.id === (driveId ?? folder(id)?.drive))!;
    return { ok: true, size: p.size, drive, quits: p.quits.join(', ') || null, macFreeAfter: s.mac.free + (kind === 'move' ? p.size : -p.size) };
  };

  // Start: show the last scan at once, then refresh it.
  const cached = read<Partial<ScanResult>>('stowaway.scan', {});
  if (cached.folders) { s.folders = cached.folders.map(toFolder); s.mac = { ...s.mac, ...cached.mac }; s.scan = { running: false, checked: cached.checked ?? 0 }; }
  if (!light) scan(!!cached.folders);
  pollDrives();
  setInterval(pollDrives, 5000);
  // Drive activity: two readings of the counters make a speed; the first reading is the baseline for averages.
  const first: Record<string, { t: number; b: [number, number] }> = {}, prev: Record<string, { t: number; b: [number, number] }> = {}, lastActive: Record<string, number> = {};
  const pollIo = () => invoke<Record<string, [number, number]>>('disk_io').then((io) => {
    const now = Date.now(), out: Record<string, DriveIo> = {};
    for (const v of vols) {
      const b = v.disk ? io[v.disk] : undefined;
      if (!b) continue;
      const p = prev[v.id], f = (first[v.id] ??= { t: now, b });
      prev[v.id] = { t: now, b };
      if (!p) continue;
      const dt = (now - p.t) / 1000, all = (now - f.t) / 1000;
      const read = Math.max(0, (b[0] - p.b[0]) / dt), write = Math.max(0, (b[1] - p.b[1]) / dt);
      if (read + write > 4096) lastActive[v.id] = now;
      out[v.id] = { read, write, avgRead: all ? (b[0] - f.b[0]) / all : 0, avgWrite: all ? (b[1] - f.b[1]) / all : 0, last: lastActive[v.id] ?? null };
    }
    // This Mac's own disk, keyed 'mac', so the popover can show its read and write speed.
    const b = io[bootDisk], p = prev.mac;
    if (b) { prev.mac = { t: now, b }; if (p) { const dt = (now - p.t) / 1000; out.mac = { read: Math.max(0, (b[0] - p.b[0]) / dt), write: Math.max(0, (b[1] - p.b[1]) / dt), avgRead: 0, avgWrite: 0, last: null }; } }
    s.io = out;
  }, () => {});
  const saveNet = () => write('stowaway.net', { test: s.net!.test, history: s.net!.history, server: s.net!.server, serverLabel: s.net!.serverLabel, mode: s.net!.mode });
  if (!light) invoke<SpeedServer[]>('speed_servers', { query: null }).then((v) => { s.net!.servers = v; emit(); }, () => {});
  type Sys = { cpu: number[]; uptime: number; bootDisk: string; awake: boolean; desktopIcons: boolean };
  const pollSys = () => invoke<Sys>('sys_status').then((r) => {
    bootDisk = r.bootDisk;
    // Two tick readings make the split; nice time counts as user, as Activity Monitor does.
    const d = ticks ? r.cpu.map((x, i) => x - ticks![i]) : null, all = d ? d.reduce((a, b) => a + b, 0) : 0;
    ticks = r.cpu;
    const pct = (x: number) => all > 0 ? Math.round((x / all) * 100) : 0;
    s.sys = { user: d ? pct(d[0] + d[3]) : 0, system: d ? pct(d[1]) : 0, idle: d ? pct(d[2]) : 100, uptime: r.uptime, awake: r.awake, desktopIcons: r.desktopIcons };
  }, () => {});
  const pollNet = () => invoke<NetInfo | null>('net_info').then((info) => { s.net!.info = info; emit(); }, () => {});
  pollNet();
  setInterval(pollNet, 30e3);
  const pollMem = () => invoke<Reading | null>('memory').then(async (r) => {
    if (isPopover && !document.hidden) await pollSys();
    await pollIo();
    Object.assign(s.net!, read<{ test: SpeedTest | null; history: SpeedTest[] }>('stowaway.net', { test: s.net!.test, history: s.net!.history })); // a test run in another window
    if (!r) return;
    reading = r;
    s.forecast = toForecast(r);
    // One notification per episode: when the forecast first drops inside the chosen window.
    const left = s.forecast?.minutesLeft;
    const due = left != null && left <= s.settings.warnAt;
    if (due && !warned && s.permissions.notify) invoke('notify', { title: `Memory may run out in about ${left} minutes`, body: s.forecast?.suggest ? `Quitting ${s.forecast.suggest.app} frees the most.` : 'Open iStash to see what is using memory.' }).catch(() => {});
    warned = due;
    s.memApps = toMemApps(r);
    s.mac.free = r.diskFree || s.mac.free;
    s.mac.total = r.diskTotal || s.mac.total;
    emit();
  }, () => { /* keep the last reading */ });
  pollMem();
  setInterval(pollMem, 2000);
  if (!light) pollJobs().then(() => { if (s.jobs.length) jobTimer ??= setInterval(() => pollJobs().catch(() => {}), 1000); }, () => {});
  if (!light) {
    invoke('set_guard', { on: s.settings.ejectGuard }).catch(() => {});
    invoke('widget_mode', { mode: s.settings.widget, left: s.settings.widgetSide === 'left' }).catch(() => {});
  }

  const cleanScan = async () => { s.clean.items = await invoke<CleanItem[]>('clean_scan', { days: s.settings.cleanAge }); emit(); };
  const clean = async (kinds: string[]) => {
    if (s.clean.busy || !kinds.length) return;
    s.clean.busy = true; emit();
    try {
      s.clean.freed = await invoke<number>('clean_run', { kinds, days: s.settings.cleanAge });
      s.clean.last = Date.now();
      write('stowaway.clean', { last: s.clean.last });
    } finally { s.clean.busy = false; emit(); }
    await cleanScan();
  };
  // Schedule: checked at launch and every hour while iStash is open; a missed run happens at the next check.
  const DAY = 864e5, EVERY: Record<string, number> = { off: Infinity, daily: DAY, weekly: 7 * DAY, monthly: 30 * DAY };
  const cleanIfDue = async () => {
    const every = s.settings.cleanEvery, kinds = s.settings.cleanKinds;
    if (!every.startsWith('over')) { if (Date.now() - (s.clean.last ?? 0) >= EVERY[every]) await clean(kinds); return; }
    // Size rule: measure, and clean only once the ticked kinds together pass the chosen size.
    await cleanScan();
    if ((s.clean.items ?? []).filter((i) => kinds.includes(i.id)).reduce((n, i) => n + i.size, 0) >= +every.slice(4) * GB) await clean(kinds);
  };
  if (!light) { setTimeout(() => cleanIfDue().catch(() => {}), 60e3); setInterval(() => cleanIfDue().catch(() => {}), 36e5); }

  // The switch shows what macOS has, not what was last saved: the user can also change it in System Settings.
  invoke<string>('login_status').then((now) => { s.settings.login = now !== 'off'; s.loginNeedsApproval = now === 'approve'; emit(); }, () => {});
  const native = () => import('./native.ts');
  native().then((n) => n.appVersion()).then((v) => { s.version = v; emit(); }, () => {});
  const checkUpdate = async () => {
    s.update.busy = true; emit();
    try {
      const was = s.update.available;
      try { s.update.available = await (await native()).checkUpdate(); }
      catch { throw new Error('iStash couldn’t reach its update page. Check your connection and try again. Nothing was changed.'); }
      s.update.checked = Date.now();
      if (s.update.available && s.update.available !== was && s.permissions.notify) invoke('notify', { title: `iStash ${s.update.available} is ready`, body: 'Open Settings to install it.' }).catch(() => {});
    } finally { s.update.busy = false; emit(); }
  };
  // Automatic checks: shortly after launch, then every six hours, when the setting is on. Offline is not an error.
  const autoCheck = () => { if (s.settings.updates) checkUpdate().catch(() => {}); };
  if (!light) { setTimeout(autoCheck, 20e3); setInterval(autoCheck, 6 * 36e5); }

  return {
    async state() { return structuredClone(s); },
    subscribe(fn) { subs.add(fn); return () => subs.delete(fn); },
    async metrics() {
      if (!reading) throw new ApiError('helper-down', 'No reading yet');
      const r = reading;
      return { at: r.t * 1000, cpu: r.cpu.map(Math.round), cores: r.cores, memUsed: r.memUsed, memTotal: r.memTotal, pressure: Math.round(100 - r.freePct),
        swap: r.swap, compressed: r.compressed, netIn: r.netIn, netOut: r.netOut, netName: '' };
    },
    planMove: (id, d) => plan('move', id, d),
    move: (id, d) => start('move', id, d),
    planBack: (id) => plan('back', id),
    bringBack: (id) => start('back', id),
    async stop(id) { await invoke('stop_job', { path: id }); },
    async eject(id) {
      // The sheet that leads here already named the open apps and said they will be asked to quit.
      if (s.folders.some((f) => f.drive === id && f.running)) await invoke('quit_drive_users', { name: id });
      await invoke('eject', { name: id });
      await pollDrives();
    },
    async quitApp(appId) {
      const a = s.memApps.find((x) => x.id === appId);
      if (!a?.appPath) throw new Error('iStash can only quit apps, not background programs. Nothing was changed.');
      await invoke('quit_app', { path: a.appPath });
    },
    async scan() { await scan(); },
    async set(key, value) {
      s.settings[key] = value;
      save();
      if (key === 'ejectGuard') await invoke('set_guard', { on: value });
      if (key === 'login') {
        const now = await invoke<string>('set_login', { on: value }).catch((e) => { s.settings.login = !value; emit(); throw new Error(String(e)); });
        s.settings.login = now !== 'off'; s.loginNeedsApproval = now === 'approve';
      }
      // Turning a schedule on starts the clock now, so the first cleaning is one interval away, not at once.
      if (key === 'cleanAge') cleanScan().catch(() => {});
      if (key === 'widget' || key === 'widgetSide') await invoke('widget_mode', { mode: s.settings.widget, left: s.settings.widgetSide === 'left' });
      if (key === 'cleanEvery' && value !== 'off' && s.clean.last == null) { s.clean.last = Date.now(); write('stowaway.clean', { last: s.clean.last }); }
      emit();
    },
    async requestPermission(p) { s.permissions[p] = true; save(); emit(); },
    async startHelper() { /* the engine runs inside the app in this build */ },
    cleanScan, clean, checkUpdate,
    async setSpeedServer(url, label) {
      const u = url.trim();
      if (u && !u.startsWith('ookla:') && !/^https?:\/\/[^\s']+$/.test(u)) throw new Error('Enter a full web address starting with https://, pointing to a large file to download.');
      s.net!.server = u; s.net!.serverLabel = label; saveNet(); emit();
    },
    async setSpeedMode(mode) { s.net!.mode = mode; saveNet(); emit(); },
    async findServers(query) { s.net!.servers = await invoke<SpeedServer[]>('speed_servers', { query: query || null }); },
    async speedTest() {
      s.net!.testing = true; s.net!.live = { phase: 'ping', mbps: 0, ping: null, pct: 0 }; emit();
      const stop = await (await native()).onSpeedProgress((p) => { s.net!.live = p as SpeedLive; emit(); });
      try {
        const t = await invoke<SpeedTest>('speed_test', { server: s.net!.server || null, mode: s.net!.mode });
        t.server = s.net!.serverLabel || t.server; t.key = s.net!.server;
        s.net!.test = t; s.net!.history = [t, ...s.net!.history].slice(0, 200); saveNet();
      }
      catch (e) { throw new Error(String(e)); }
      finally { stop(); s.net!.testing = false; s.net!.live = null; emit(); }
    },
    async installUpdate() {
      if (s.jobs.length) throw new Error('A folder is being moved. Wait for it to finish, then update. Nothing was changed.');
      s.update.busy = true; emit();
      try { await (await native()).installUpdate(); } finally { s.update.busy = false; emit(); }
    },
    async freeMemory() {
      s.clean.memBusy = true; emit();
      try { s.clean.memFreed = await invoke<number>('free_memory'); }
      catch (e) { throw new Error(String(e)); }
      finally { s.clean.memBusy = false; emit(); }
    },
    async keepAwake(on) { s.sys!.awake = await invoke<boolean>('keep_awake', { on }).catch((e) => { throw new Error(String(e)); }); emit(); },
    async lockScreen() { await invoke('lock_screen').catch((e) => { throw new Error(`macOS didn’t lock the screen: ${e}`); }); },
    async emptyTrash() { await invoke('empty_trash').catch((e) => { throw new Error(`Finder didn’t empty the Trash: ${e}`); }); },
    async desktopIcons(show) { s.sys!.desktopIcons = await invoke<boolean>('set_desktop_icons', { show }).catch((e) => { throw new Error(String(e)); }); emit(); },
    async focus(appPath) { await invoke('focus', { path: appPath }); s.focus = appPath; emit(); },
    appIcon: (path) => invoke<ArrayBuffer>('app_icon', { path }).then((png) => URL.createObjectURL(new Blob([png], { type: 'image/png' })), () => null),
  };
}
