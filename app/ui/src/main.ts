// App shell: routing, rendering, actions. Ported from design/prototype/app.js; all data goes through api.ts.
import morphdom from 'morphdom';
import '@fontsource-variable/geist/wght.css';
import '@fontsource-variable/geist-mono/wght.css';
import '../../../design/prototype/tokens.css';
import './app.css';
import { api, ApiError, type Metrics, type Plan, type State, type Theme } from './api';
import * as screens from './screens';
import type { Route, Ui, View } from './screens';

const native = '__TAURI_INTERNALS__' in window;
const ROUTES: Route[] = ['overview', 'offload', 'drives', 'memory', 'internet', 'clean', 'settings', 'onboarding', 'popover'];
const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
const root = document.getElementById('root')!;
const tauri = () => import('./native');

let s: State | null = null;
let m: Metrics | null = null;
let down = false;
const ui: Ui = { route: 'overview', dialog: null, offloadTab: 'suggested', onboardStep: 0, onboardDrive: null, busy: false, drive: null };
const isPopover = location.hash === '#/popover';
const isWidget = location.hash === '#/widget';
document.documentElement.classList.toggle('native', native);
document.documentElement.classList.toggle('popover-window', isPopover);
document.documentElement.classList.toggle('widget-window', isWidget);

// ---------- App icons, cached per bundle path ----------
const icons = new Map<string, string | null>();
function iconFor(path?: string) {
  if (!path) return null;
  if (!icons.has(path)) {
    icons.set(path, null);
    api.appIcon(path).then((u) => { if (u) { icons.set(path, u); render(); } });
  }
  return icons.get(path)!;
}

// ---------- Rendering: morph the DOM so focus, hover and VoiceOver position survive updates ----------
let animating = false;
let hot: string | null = null;
let lastTrigger: string | null = null;
let hadDialog = false;

function render() {
  const v: View = { s, m, ui, down, icon: iconFor };
  let html: string;
  if (ui.route === 'popover') html = screens.popover(v);
  else if (ui.route === 'widget') html = screens.widget(v);
  else if (ui.route === 'onboarding') html = `<div class="window no-sidebar">${screens.onboarding(v)}</div>`;
  else html = `<div class="window">${screens.sidebar(v)}<main class="content">${screens[ui.route](v)}</main>${screens.dialogFor(v)}</div>`;
  morphdom(root, `<div id="root">${html}</div>`, {
    onBeforeElUpdated: (from, to) => !(from.isEqualNode(to) || (animating && from.classList.contains('bay'))),
  });
  applyHot();
  const dlg = root.querySelector<HTMLElement>('.dialog');
  if (dlg && !hadDialog) (dlg.querySelector<HTMLElement>('.btn-primary') ?? dlg.querySelector<HTMLElement>('.btn'))?.focus();
  if (!dlg && hadDialog && lastTrigger) root.querySelector<HTMLElement>(lastTrigger)?.focus();
  hadDialog = !!dlg;
  if (isPopover && native) tauri().then((n) => n.fitPopover(root.firstElementChild as HTMLElement));
  if (isWidget) {
    const e = root.querySelector<HTMLElement>('.edge');
    // Shrink the whole column to fit the fixed window: measured unzoomed, applied to the page.
    if (e) { const r = document.documentElement; r.style.zoom = ''; const k = Math.min(1, innerHeight / e.scrollHeight); r.style.zoom = k < 1 ? String(Math.max(0.55, k - 0.01)) : ''; }
  }
}

// ---------- Data ----------
let theme: Theme | null = null;
function onState(next: State) {
  const prev = s;
  s = next;
  down = false;
  if (!s.settings.onboarded && !isPopover && !isWidget) ui.route = 'onboarding';
  ui.onboardDrive ??= s.settings.defaultDrive ?? s.drives.find((d) => d.connected && !d.refused && !d.problem)?.id ?? null;
  if (s.settings.theme !== theme) {
    theme = s.settings.theme;
    if (theme === 'system') delete document.documentElement.dataset.theme; else document.documentElement.dataset.theme = theme;
    if (native) tauri().then((n) => n.setTheme(theme!));
  }
  if (native && !isPopover && !isWidget) tauri().then((n) => n.setTrayTitle(s!.forecast && s!.forecast.condition !== 'fine' ? `${s!.forecast.minutesLeft}m` : ''));
  render();
  if (prev) for (const d of next.drives) {
    const was = prev.drives.find((x) => x.id === d.id)?.connected;
    if (was !== undefined && was !== d.connected) animateCable(d.id, d.connected);
  }
}

async function load() {
  try { onState(await api.state()); } catch (e) {
    if (!(e instanceof ApiError)) throw e;
    down = true;
    render();
  }
}
api.subscribe(onState);

async function poll() {
  if (document.hidden) return;
  try { m = await api.metrics(); render(); } catch { /* the helper-down state already says so */ }
}
setInterval(poll, 2000);

// ---------- Cable animation (unplug drops with a bounce, plug-in settles) ----------
const easeOutBack = (t: number) => 1 + 2.70158 * (t - 1) ** 3 + 1.70158 * (t - 1) ** 2;
const easeOutBounce = (t: number) => {
  const n = 7.5625, d = 2.75;
  if (t < 1 / d) return n * t * t;
  if (t < 2 / d) return n * (t -= 1.5 / d) * t + 0.75;
  if (t < 2.5 / d) return n * (t -= 2.25 / d) * t + 0.9375;
  return n * (t -= 2.625 / d) * t + 0.984375;
};
function animateCable(id: string, on: boolean) {
  if (ui.route !== 'drives' || reduced || !s) return;
  const L = screens.cableLayout(s);
  const t0 = performance.now(), dur = 520;
  animating = true;
  const frame = (now: number) => {
    const k = Math.min(1, (now - t0) / dur);
    const t = on ? easeOutBack(k) : 1 - easeOutBounce(k);
    L.cables.forEach((c, i) => {
      if (c.d.id !== id) return;
      const d = screens.cablePath(c, t, L.jackX, L.portX);
      root.querySelectorAll(`[data-cable="${i}"]`).forEach((p) => p.setAttribute('d', d));
      const plug = root.querySelector(`[data-plug="${i}"]`);
      plug?.setAttribute('x', String(L.jackX + 180 + (L.portX - L.jackX - 180) * t - 8));
      plug?.setAttribute('y', String(c.floor + (c.yp - c.floor) * t - 5));
    });
    if (k < 1) requestAnimationFrame(frame); else animating = false;
  };
  requestAnimationFrame(frame);
}

// Hover on the cable map: light the cables of one app or one drive.
function applyHot() {
  const bay = root.querySelector('.bay');
  if (!bay) return;
  bay.classList.toggle('has-hot', !!hot);
  root.querySelectorAll<SVGGElement>('.cable-group').forEach((g) => {
    const on = hot && (hot === `app:${g.dataset.app}` || hot === `drive:${g.dataset.drive}`);
    g.classList.toggle('is-hot', !!on);
    g.querySelector('.cable')!.classList.toggle('is-hot', !!on);
  });
}
document.addEventListener('mouseover', (e) => {
  const next = (e.target as Element).closest<HTMLElement>('[data-hot]')?.dataset.hot ?? null;
  if (next !== hot) { hot = next; applyHot(); }
});

// ---------- Actions ----------
const target = () => {
  const ok = s!.drives.filter((d) => d.connected && !d.refused && !d.problem);
  return (ok.find((d) => d.id === s!.settings.defaultDrive) ?? ok[0])?.id ?? '';
};
const failed = (e: unknown): Plan => ({ ok: false, title: 'That didn’t work', reason: e instanceof Error ? e.message : String(e) });

const actions: Record<string, (id: string) => unknown> = {
  go: (id) => { location.hash = `#/${id}`; },
  tab: (id) => { ui.offloadTab = id as Ui['offloadTab']; },
  theme: (id) => api.set('theme', id as Theme),
  'widget-mode': (id) => api.set('widget', id as 'off'),
  'widget-side': (id) => api.set('widgetSide', id as 'right'),
  scan: () => api.scan(),
  close: () => { ui.dialog = null; },
  'start-helper': async () => { await api.startHelper(); await load(); },
  open: (id) => native && tauri().then((n) => n.openTarget(id)),
  move: async (id) => {
    ui.dialog = null;
    if (!s!.settings.dryRun) return api.move(id, target());
    const drive = target();
    ui.dialog = { type: 'move', id, drive, plan: await api.planMove(id, drive) };
  },
  back: async (id) => { ui.dialog = { type: 'back', id, plan: await api.planBack(id) }; },
  eject: (id) => {
    if (s!.folders.some((i) => i.verdict === 'stowed' && i.drive === id)) ui.dialog = { type: 'eject', id };
    else return api.eject(id);
  },
  'confirm-move': (id) => { const p = ui.dialog?.plan; ui.dialog = null; return p?.ok && api.move(id, p.drive.id); },
  'confirm-back': (id) => { ui.dialog = null; return api.bringBack(id); },
  'confirm-eject': (id) => { ui.dialog = null; return api.eject(id); },
  'confirm-quit': (id) => { ui.dialog = null; return api.quitApp(id); },
  'quit-now': (id) => api.quitApp(id),
  stop: (id) => api.stop(id),
  focus: (id) => api.focus(id || null),
  'clean-scan': () => api.cleanScan(),
  'free-memory': () => api.freeMemory(),
  'speed-test': () => api.speedTest(),
  'speed-mode': (id) => api.setSpeedMode(id as 'full'),
  'pick-server': (id) => pickServer(+id),
  'check-update': () => api.checkUpdate(),
  'install-update': () => api.installUpdate(),
  'clean-ask': () => { ui.dialog = { type: 'clean' }; },
  'confirm-clean': () => { ui.dialog = null; return api.clean(s!.settings.cleanKinds); },
  grant: (id) => api.requestPermission(id as 'volumes'),
  'grant-all': () => api.requestAll(),
  'open-main': () => tauri().then((n) => n.showMain()),
  // Popover quick toggles. Each one flips back with a second click.
  'pop-dark': () => api.set('theme', root.querySelector('[data-action="pop-dark"]')?.getAttribute('aria-pressed') === 'true' ? 'light' : 'dark'),
  'pop-awake': () => api.keepAwake(!s!.sys?.awake),
  'pop-lock': () => api.lockScreen(),
  'pop-screen': () => api.keepScreen(!s!.sys?.screenOn),
  'island-test': () => api.testIsland(),
  'island-glow': (id) => api.set('islandGlow', id as 'amber'),
  'island-secs': (id) => api.set('islandSecs', Number(id)),
  'pop-camera': () => api.openCamera(),
  'pop-temps': () => { ui.temps = !ui.temps; render(); },
  'fan-mode': (id) => {
    if (id === 'manual') { ui.fanManual = true; render(); return; }
    return setFan(id as 'auto', undefined);
  },
  'open-app': (id) => native && tauri().then((n) => n.openTarget(id)),
  'pop-trash': () => { ui.dialog = { type: 'trash' }; },
  'confirm-trash': () => { ui.dialog = null; return api.emptyTrash(); },
  'pop-desktop': () => { ui.dialog = { type: 'desktop' }; },
  'confirm-desktop': () => { ui.dialog = null; return api.desktopIcons(s!.sys?.desktopIcons === false); },
  'pop-widget': () => api.set('widget', s!.settings.widget === 'off' ? 'auto' : 'off'),
  'onboard-next': () => { ui.onboardStep++; },
  'onboard-back': () => { ui.onboardStep--; },
  'onboard-done': async () => {
    if (ui.onboardDrive) await api.set('defaultDrive', ui.onboardDrive);
    await api.set('onboarded', true);
    ui.onboardStep = 0;
    location.hash = '#/offload';
    route();
  },
};
for (const t of ['how', 'why', 'apfs', 'quit-app'] as const) actions[t] = (id) => { ui.dialog = { type: t, id }; };

async function run(name: string, id: string) {
  try { await actions[name]?.(id); } catch (e) { ui.dialog = { type: 'move', id, plan: failed(e) }; }
  render();
}

document.addEventListener('click', (e) => {
  const el = (e.target as Element).closest<HTMLElement>('[data-action]');
  if (!el || el.matches('input, select')) return;
  if (el.dataset.action === 'dismiss') { if (e.target === el) { ui.dialog = null; render(); } return; }
  if ((el as HTMLButtonElement).disabled) return;
  if (!el.closest('.dialog')) lastTrigger = `[data-action="${el.dataset.action}"]${el.dataset.id ? `[data-id="${CSS.escape(el.dataset.id)}"]` : ''}`;
  run(el.dataset.action!, el.dataset.id ?? '');
});
document.addEventListener('change', async (e) => {
  const el = e.target as HTMLInputElement, a = el.dataset.action, id = el.dataset.id!;
  if (a === 'setting') await api.set(id as 'login', el.checked);
  else if (a === 'setting-select') await api.set('warnAt', +el.value);
  else if (a === 'pick-drive') ui.onboardDrive = id;
  else if (a === 'speed-server') await api.setSpeedServer(el.value);

  else if (a === 'fan-rpm') await setFan('rpm', +el.value).catch((err) => { ui.dialog = { type: 'move', id: '', plan: failed(err) }; });
  else if (a === 'clean-every') await api.set('cleanEvery', el.value as 'off');
  else if (a === 'clean-age') await api.set('cleanAge', +el.value);
  else if (a === 'clean-kind') await api.set('cleanKinds', el.checked ? [...s!.settings.cleanKinds, id] : s!.settings.cleanKinds.filter((k) => k !== id));
  else if (a === 'default-drive') await api.set('defaultDrive', el.value);
  else if (a === 'move-drive' && ui.dialog) ui.dialog = { ...ui.dialog, drive: el.value, plan: await api.planMove(ui.dialog.id!, el.value) };
  else return;
  render();
});
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && ui.dialog) { ui.dialog = null; render(); return; }
  // Keep Tab inside an open sheet.
  const dlg = root.querySelector('.dialog');
  if (e.key !== 'Tab' || !dlg) return;
  const f = [...dlg.querySelectorAll<HTMLElement>('button:not(:disabled), select, input')];
  const i = f.indexOf(document.activeElement as HTMLElement);
  if (e.shiftKey && i <= 0) { f.at(-1)?.focus(); e.preventDefault(); } else if (!e.shiftKey && i === f.length - 1) { f[0]?.focus(); e.preventDefault(); }
});

function route() {
  const [r, sub] = location.hash.replace('#/', '').split('/') as [Route, string?];
  ui.drive = r === 'drives' && sub ? decodeURIComponent(sub) : null;
  if (r === 'clean') api.cleanScan().catch(() => {});
  ui.route = isPopover ? 'popover' : isWidget ? 'widget' : s && !s.settings.onboarded ? 'onboarding' : ROUTES.includes(r) && r !== 'popover' && r !== 'onboarding' ? r : 'overview';
  ui.dialog = null;
  const show = () => { render(); root.querySelector<HTMLElement>('.page-header h1')?.focus({ preventScroll: true }); };
  // A short cross-fade between pages where the webview supports it.
  const vt = (document as any).startViewTransition;
  if (vt && !reduced && root.firstChild) vt.call(document, show); else show();
}
window.addEventListener('hashchange', route);
if (native) tauri().then((n) => n.onAppearance((t) => api.set('theme', t)));
route();
load().then(poll);


// Fans: the first change asks for the password once (to install the helper); errors show like any other.
async function setFan(mode: 'auto' | 'max' | 'rpm', rpm?: number) {
  ui.fanBusy = true; render();
  try { await api.setFan(mode, rpm); ui.fanManual = mode === 'rpm'; if (mode !== 'rpm') ui.fanRpm = undefined; } finally { ui.fanBusy = false; render(); }
}

// Server picker: typing filters at once and searches the Speedtest list after a pause; arrows move, Enter picks.
let searchTimer: ReturnType<typeof setTimeout> | undefined;
async function pickServer(i: number) {
  const o = screens.serverOptions[i];
  ui.serverOpen = false; ui.serverQuery = '';
  if (o) await api.setSpeedServer(o.value, o.value === '' ? 'Cloudflare' : `${o.title}${o.sub && !o.value.startsWith('http') ? ` — ${o.sub}` : ''}`);
}
const openPicker = () => { if (!ui.serverOpen) { ui.serverOpen = true; ui.serverQuery = ''; ui.serverIndex = 0; render(); } };
document.addEventListener('focusin', (e) => { if ((e.target as HTMLElement).dataset?.action === 'server-box') openPicker(); });
document.addEventListener('input', (e) => {
  const el = e.target as HTMLInputElement;
  if (el.dataset.action === 'fan-rpm') { ui.fanRpm = +el.value; const o = el.parentElement?.querySelector('.pop-fan-rpm'); if (o) o.textContent = (+el.value).toLocaleString('en-GB'); return; }
  if (el.dataset.action !== 'server-box') return;
  ui.serverOpen = true; ui.serverQuery = el.value; ui.serverIndex = 0; render();
  clearTimeout(searchTimer);
  const q = el.value.trim();
  if (/^https?:\/\//i.test(q)) return;
  searchTimer = setTimeout(async () => {
    ui.searching = true; render();
    try { await api.findServers(q); } catch { /* keep the last list */ }
    ui.searching = false; render();
  }, 350);
});
document.addEventListener('keydown', (e) => {
  if ((e.target as HTMLElement).dataset?.action !== 'server-box' || !ui.serverOpen) return;
  const n = screens.serverOptions.length;
  if (e.key === 'ArrowDown') ui.serverIndex = Math.min(n - 1, (ui.serverIndex ?? 0) + 1);
  else if (e.key === 'ArrowUp') ui.serverIndex = Math.max(0, (ui.serverIndex ?? 0) - 1);
  else if (e.key === 'Enter') { pickServer(ui.serverIndex ?? 0).then(render); (e.target as HTMLElement).blur(); }
  else if (e.key === 'Escape') { ui.serverOpen = false; (e.target as HTMLElement).blur(); }
  else return;
  e.preventDefault(); render();
  root.querySelector('.pick-row.is-hi')?.scrollIntoView({ block: 'nearest' });
});
// A click anywhere else closes the list.
document.addEventListener('mousedown', (e) => {
  if (ui.serverOpen && !(e.target as Element).closest('.pick')) { ui.serverOpen = false; render(); }
});
