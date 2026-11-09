// Prototype shell: state, routing, actions, simulated jobs and live numbers. No framework; the Tauri app ports this to TS.
import { initialState } from './data.js';
import { esc, gb, list, button, dialog, sparkline, STEPS } from './ui.js';
import * as screens from './screens.js';

const params = new URLSearchParams(location.search);
let s = initialState();
s.theme = params.get('theme') || 'auto';
s.view = params.get('view') || 'normal';
if (params.get('archive') === '1') s.connected.archive = true;
const root = document.getElementById('root');
const ROUTES = ['overview', 'offload', 'drives', 'memory', 'settings', 'onboarding', 'popover'];
const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;

const item = (id) => s.items.find((i) => i.id === id);
const driveOf = (id) => s.drives.find((d) => d.id === id);

// ---------- Rendering ----------
function protoBar() {
  const link = (r, label) => `<a href="#/${r}" ${s.route === r || (r === 'overview' && !['onboarding', 'popover'].includes(s.route)) ? 'aria-current="page"' : ''}>${label}</a>`;
  const sel = (id, opts, cur) => `<select data-proto="${id}">${opts.map(([v, l]) => `<option value="${v}" ${v === cur ? 'selected' : ''}>${l}</option>`).join('')}</select>`;
  return `<div class="proto-bar" aria-label="Prototype controls"><strong>Prototype</strong>${link('onboarding', 'Onboarding')}${link('overview', 'App')}${link('popover', 'Menu-bar popover')}
    <label>Theme ${sel('theme', [['auto', 'System'], ['light', 'Light'], ['dark', 'Dark']], s.theme)}</label>
    <label>Screen state ${sel('view', [['normal', 'Normal'], ['loading', 'Loading'], ['empty', 'Empty'], ['error', 'Error']], s.view)}</label>
    ${s.drives.map((d) => `<label><input type="checkbox" data-proto="drive" data-id="${d.id}" ${s.connected[d.id] ? 'checked' : ''}> ${esc(d.name)} plugged in</label>`).join('')}
    <button class="btn" data-proto-reset>Reset</button></div>`;
}

function render() {
  const scroll = root.querySelector('.page-body')?.scrollTop || 0;
  if (s.theme === 'auto') delete document.documentElement.dataset.theme; else document.documentElement.dataset.theme = s.theme;
  let body;
  if (s.route === 'popover') body = screens.popover(s);
  else if (s.route === 'onboarding') body = `<div class="window no-sidebar">${screens.onboarding(s)}</div>`;
  else body = `<div class="window">${screens.sidebar(s)}<main class="content">${screens[s.route](s)}</main>${dialogFor(s.dialog)}</div>`;
  root.innerHTML = `<div class="stage">${protoBar()}${body}</div>`;
  const pb = root.querySelector('.page-body'); if (pb) pb.scrollTop = scroll;
  root.querySelector('.dialog .btn-primary, .dialog .btn-danger, .dialog .btn')?.focus();
}

function dialogFor(d) {
  if (!d) return '';
  const close = (label = 'Cancel') => button(label, 'close');
  const it = d.id && item(d.id);
  switch (d.type) {
    case 'move': {
      const targets = s.drives.filter((x) => s.connected[x.id]);
      const dr = driveOf(d.drive || targets[0]?.id);
      if (!dr) return dialog({ title: 'No drive connected', body: '<p>Plug in an APFS drive to move this folder.</p>', buttons: close('OK') });
      const q = it.running ? `Quit ${esc(it.app)} and move` : 'Move';
      return dialog({ title: `Move ${esc(it.app)} ${esc(it.name.toLowerCase())} to ${esc(dr.name)}?`, buttons: close() + button(q, 'confirm-move', { kind: 'primary', id: it.id }),
        body: `<div class="field"><span class="secondary">Drive</span><select data-action="move-drive">${targets.map((x) => `<option value="${x.id}" ${x.id === dr.id ? 'selected' : ''}>${esc(x.name)} (${x.free} GB free)</option>`).join('')}</select></div>
          <ol>${it.running ? `<li>Ask ${esc(it.app)} to quit, so nothing is writing to the folder.</li>` : ''}<li>Copy ${gb(it.size)} to ${esc(dr.name)}.</li><li>Check every file: count, size and content.</li><li>Put a link where the folder was. ${esc(it.app)} follows it without noticing.</li><li>Remove the original only after the check passes.</li></ol>
          <p>This Mac will have <strong class="num">${gb(s.macFree + it.size)}</strong> free. You can bring it back at any time.</p>` });
    }
    case 'how':
      return dialog({ title: `Move ${esc(it.app)}’s ${esc(it.name.toLowerCase())} with its own setting`, buttons: button('Done', 'close', { kind: 'primary' }),
        body: `<p>${esc(it.app)} keeps a live database here. A link can corrupt it if the drive is unplugged while it runs, so let ${esc(it.app)} move it.</p><ol>${it.how.map((x) => `<li>${esc(x)}</li>`).join('')}</ol>` });
    case 'why':
      return dialog({ title: `Why ${esc(it.name.toLowerCase())} can’t move`, body: `<p>${esc(it.why)}</p>`, buttons: button('OK', 'close', { kind: 'primary' }) });
    case 'apfs':
      return dialog({ title: 'Which drives work', buttons: button('OK', 'close', { kind: 'primary' }),
        body: '<p>APFS or Mac OS Extended drives work. exFAT and FAT drives (common on USB sticks) can’t hold the links apps need, so Stowaway won’t use them. Disk Utility can reformat a drive as APFS; that erases it.</p>' });
    case 'back': {
      const dr = driveOf(it.drive);
      if (it.size > s.macFree - 10) return dialog({ title: 'Not enough room on this Mac', buttons: button('OK', 'close', { kind: 'primary' }),
        body: `<p>${esc(it.app)} ${esc(it.name.toLowerCase())} needs ${gb(it.size)}. This Mac has ${gb(s.macFree)} free, and Stowaway keeps 10 GB spare so macOS can keep working. Nothing was changed.</p>` });
      return dialog({ title: `Bring ${esc(it.app)} ${esc(it.name.toLowerCase())} back to this Mac?`, buttons: close() + button(it.running ? `Quit ${esc(it.app)} and bring back` : 'Bring back', 'confirm-back', { kind: 'primary', id: it.id }),
        body: `<ol>${it.running ? `<li>Ask ${esc(it.app)} to quit.</li>` : ''}<li>Copy ${gb(it.size)} from ${esc(dr.name)} to this Mac.</li><li>Check every file.</li><li>Replace the link with the folder.</li><li>Remove the copy on ${esc(dr.name)} last.</li></ol><p>This Mac will have <strong class="num">${gb(s.macFree - it.size)}</strong> free.</p>` });
    }
    case 'eject': {
      const dr = driveOf(d.id), on = s.items.filter((i) => i.verdict === 'stowed' && i.drive === dr.id);
      const open = on.filter((i) => i.running);
      const apps = [...new Set(on.map((i) => i.app))];
      return dialog({ title: `${apps.length} apps keep data on ${esc(dr.name)}`, buttons: close() + button(open.length ? `Quit ${esc(list(open.map((i) => i.app)))} and eject` : 'Eject', 'confirm-eject', { kind: 'primary', id: dr.id }),
        body: `<ul>${on.map((i) => `<li><strong>${esc(i.app)}</strong>: ${esc(i.name)}, ${gb(i.size)}${i.running ? ' · open now' : ''}</li>`).join('')}</ul><p>After ejecting, these apps are held until ${esc(dr.name)} is back, so nothing writes to an empty folder.</p>` });
    }
    case 'quit-app': {
      const a = s.memApps.find((x) => x.id === d.id);
      return dialog({ title: `Quit ${esc(a.app)}?`, buttons: close() + button('Quit', 'confirm-quit', { kind: 'primary', id: a.id }),
        body: `<p>Stowaway asks ${esc(a.app)} to quit normally, so it can save its work. ${esc(a.detail)}. This frees about ${gb(a.mem)}.</p>` });
    }
  }
  return '';
}

// ---------- Cable animation (unplug drops with a bounce, plug-in settles) ----------
const easeOutBack = (t) => 1 + 2.70158 * (t - 1) ** 3 + 1.70158 * (t - 1) ** 2;
const easeOutBounce = (t) => {
  const n = 7.5625, d = 2.75;
  if (t < 1 / d) return n * t * t;
  if (t < 2 / d) return n * (t -= 1.5 / d) * t + 0.75;
  if (t < 2.5 / d) return n * (t -= 2.25 / d) * t + 0.9375;
  return n * (t -= 2.625 / d) * t + 0.984375;
};
function setConnected(id, on) {
  s.connected[id] = on;
  if (!on) s.items.forEach((i) => { if (i.drive === id) i.running = false; });
  render();
  if (s.route !== 'drives' || reduced) return;
  const L = screens.cableLayout(s);
  const t0 = performance.now(), dur = 520;
  const frame = (now) => {
    const k = Math.min(1, (now - t0) / dur);
    const t = on ? easeOutBack(k) : 1 - easeOutBounce(k);
    L.cables.forEach((c, i) => {
      if (c.d.id !== id) return;
      root.querySelector(`[data-cable="${i}"]`)?.setAttribute('d', screens.cablePath(c, t, L.jackX, L.portX));
      const plug = root.querySelector(`[data-plug="${i}"]`);
      if (plug) { plug.setAttribute('x', (L.jackX + 180 + (L.portX - L.jackX - 180) * t) - 8); plug.setAttribute('y', (c.floor + (c.yp - c.floor) * t) - 5); }
    });
    if (k < 1) requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

// ---------- Jobs ----------
function startJob(id, kind, drive) {
  const it = item(id);
  it.running = false;
  s.jobs[id] = { kind, step: 0, pct: 0, drive: drive || it.drive };
  s.dialog = null;
  render();
}
setInterval(() => {
  const ids = Object.keys(s.jobs);
  if (!ids.length) return;
  for (const id of ids) {
    const j = s.jobs[id], it = item(id);
    j.pct = Math.min(100, j.pct + 400 / Math.max(it.size, 8));
    j.step = j.pct < 60 ? 0 : j.pct < 90 ? 1 : j.pct < 97 ? 2 : 3;
    if (j.pct >= 100) {
      const d = driveOf(j.drive);
      if (j.kind === 'move') { it.verdict = 'stowed'; it.drive = d.id; it.since = 'today'; s.macFree += it.size; d.free = +(d.free - it.size).toFixed(1); }
      else { it.verdict = 'move'; it.reason = 'Brought back today'; delete it.drive; s.macFree -= it.size; d.free = +(d.free + it.size).toFixed(1); }
      delete s.jobs[id];
    }
  }
  if (['offload', 'drives', 'overview'].includes(s.route) && !s.dialog) render();
}, 120);

// ---------- Live numbers (updated in place so hover and focus survive) ----------
setInterval(() => {
  const walk = (arr, lo, hi, step) => { arr.push(Math.max(lo, Math.min(hi, +(arr.at(-1) + (Math.random() - 0.5) * step).toFixed(1)))); arr.shift(); };
  walk(s.live.cpu, 8, 92, 12); s.live.cpu[s.live.cpu.length - 1] = Math.round(s.live.cpu.at(-1));
  walk(s.live.net, 0.2, 6, 2.4);
  const set = (k, html) => root.querySelectorAll(`[data-live="${k}"]`).forEach((n) => { n.innerHTML = html; });
  set('cpu', `${s.live.cpu.at(-1)}<small>%</small>`); set('cpu-mini', `${s.live.cpu.at(-1)}%`);
  set('net', `${s.live.net.at(-1).toFixed(1)}<small>MB/s in</small>`);
  set('cpu-spark', sparkline(s.live.cpu, 'var(--accent-text)', 100)); set('net-spark', sparkline(s.live.net, 'var(--ok-fill)', 6));
}, 2000);

// ---------- Events ----------
const actions = {
  go: (id) => { location.hash = `#/${id}`; },
  tab: (id) => { s.offloadTab = id; },
  scan: () => { s.view = 'loading'; setTimeout(() => { s.view = 'normal'; render(); }, 1600); },
  close: () => { s.dialog = null; },
  fix: () => { s.view = 'normal'; },
  noop: () => {},
  'confirm-move': (id) => startJob(id, 'move', s.dialog.drive || s.drives.find((d) => s.connected[d.id]).id),
  'confirm-back': (id) => startJob(id, 'back'),
  'cancel-job': (id) => { delete s.jobs[id]; },
  'confirm-eject': (id) => { s.dialog = null; setConnected(id, false); return true; },
  'confirm-quit': (id) => {
    s.memApps = s.memApps.filter((a) => a.id !== id);
    s.items.forEach((i) => { if (i.appId === id) i.running = false; });
    if (id === 'docker') s.live.pressure = 54;
    s.dialog = null;
  },
  'onboard-next': () => { s.onboardStep++; },
  'onboard-back': () => { s.onboardStep--; },
  'onboard-done': () => { s.onboardStep = 0; location.hash = '#/offload'; },
  grant: (id) => { s.granted[id] = true; },
};
for (const t of ['move', 'how', 'why', 'back', 'eject', 'quit-app', 'apfs']) actions[t] = (id) => { s.dialog = { type: t, id }; };

document.addEventListener('click', (e) => {
  if (e.target.closest('[data-proto-reset]')) { const keep = { theme: s.theme, route: s.route }; s = { ...initialState(), ...keep }; return render(); }
  const el = e.target.closest('[data-action]');
  if (!el || el.matches('input, select')) return;
  if (el.dataset.action === 'dismiss') { if (e.target === el) { s.dialog = null; render(); } return; }
  if (el.disabled) return;
  const skip = actions[el.dataset.action]?.(el.dataset.id);
  if (!skip) render();
});
document.addEventListener('change', (e) => {
  const el = e.target, p = el.dataset.proto, a = el.dataset.action;
  if (p === 'theme') s.theme = el.value;
  else if (p === 'view') s.view = el.value;
  else if (p === 'drive') return setConnected(el.dataset.id, el.checked);
  else if (a === 'setting') s.settings[el.dataset.id] = el.checked;
  else if (a === 'setting-select') s.settings[el.dataset.id] = +el.value;
  else if (a === 'pick-drive') s.onboardDrive = el.dataset.id;
  else if (a === 'move-drive') s.dialog.drive = el.value;
  else return;
  render();
});
document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && s.dialog) { s.dialog = null; render(); } });

// Hover on the cable map: light the cables of one app or one drive.
document.addEventListener('mouseover', (e) => {
  const bay = root.querySelector('.bay'); if (!bay) return;
  const hot = e.target.closest('[data-hot]')?.dataset.hot;
  bay.classList.toggle('has-hot', !!hot);
  root.querySelectorAll('.cable-group').forEach((g) => {
    const on = hot && (hot === `app:${g.dataset.app}` || hot === `drive:${g.dataset.drive}`);
    g.querySelector('.cable').classList.toggle('is-hot', !!on);
  });
});

function route() {
  const r = location.hash.replace('#/', '');
  s.route = ROUTES.includes(r) ? r : 'overview';
  s.dialog = null;
  render();
}
window.addEventListener('hashchange', route);
route();
console.assert(STEPS.move.length === 4 && screens.cablePath({ ya: 0, yp: 0, floor: 0 }, 1).startsWith('M300'), 'prototype self-check');
