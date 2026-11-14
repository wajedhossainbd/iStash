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
  'toggle-results': () => { ui.allResults = !ui.allResults; },
  'check-update': () => api.checkUpdate(),
  'install-update': () => api.installUpdate(),
  'clean-ask': () => { ui.dialog = { type: 'clean' }; },
  'confirm-clean': () => { ui.dialog = null; return api.clean(s!.settings.cleanKinds); },
  grant: (id) => api.requestPermission(id as 'volumes' | 'notify'),
  'open-main': () => tauri().then((n) => n.showMain()),
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
  else if (a === 'speed-server-pick') {
    ui.customServer = el.value === 'custom';
    if (!ui.customServer) await api.setSpeedServer(el.value, (el as unknown as HTMLSelectElement).selectedOptions[0]?.textContent ?? '');
  }
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


// Server search: as you type (after a pause), the list fills with matching Speedtest servers.
let searchTimer: ReturnType<typeof setTimeout> | undefined;
document.addEventListener('input', (e) => {
  const el = e.target as HTMLInputElement;
  if (el.dataset.action !== 'speed-search') return;
  clearTimeout(searchTimer);
  searchTimer = setTimeout(() => api.findServers(el.value.trim()).then(render, () => {}), 400);
});
