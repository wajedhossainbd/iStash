// Screens, ported from design/prototype/screens.js. Each takes the view and returns HTML; data comes only from api.ts.
import type { Drive, Folder, Forecast, Metrics, Plan, State, Check } from './api';
import { esc, gb, tb, day, clock, ago, list, icon, appIcon, button, pill, seg, capacity, sparkline, banner, empty, skeletonRows, progress, steps, toggle, forecastChart, dialog } from './ui';

export type Route = 'overview' | 'offload' | 'drives' | 'memory' | 'internet' | 'clean' | 'settings' | 'onboarding' | 'popover' | 'widget';
export type Dialog = { type: 'move' | 'back' | 'check' | 'how' | 'why' | 'apfs' | 'eject' | 'quit-app' | 'clean' | 'trash' | 'desktop'; id?: string; drive?: string; plan?: Plan; check?: Check };
export interface Ui { route: Route; dialog: Dialog | null; offloadTab: 'suggested' | 'stowed' | 'all'; onboardStep: number; onboardDrive: string | null; busy: boolean; drive: string | null; serverOpen?: boolean; serverQuery?: string; serverIndex?: number; searching?: boolean; fanManual?: boolean; fanRpm?: number; fanBusy?: boolean; temps?: boolean }
/** s is null while the first state is loading; down when the helper can't be reached. */
export interface View { s: State | null; m: Metrics | null; ui: Ui; down: boolean; icon: (path?: string) => string | null }
type V = View & { s: State };

const GB = 1e9;
const usable = (s: State) => s.drives.filter((d) => !d.refused);
const drive = (s: State, id?: string) => s.drives.find((d) => d.id === id)!;
const isHeld = (s: State, it: Folder) => it.verdict === 'stowed' && !drive(s, it.drive).connected;
const stowedOn = (s: State, id: string) => s.folders.filter((i) => i.verdict === 'stowed' && i.drive === id);
const suggested = (s: State) => s.folders.filter((i) => i.verdict === 'move');
const strained = (s: State) => !!s.forecast && s.forecast.condition !== 'fine';
const sum = (xs: { size: number }[]) => xs.reduce((a, i) => a + i.size, 0);
const cableVar = (d: Drive) => `var(--cable-${d.cable})`;
const appsOn = (xs: Folder[]) => [...new Set(xs.map((i) => i.app))];

const ILLUS_PORT = `<svg class="illus" viewBox="0 0 120 72" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><rect x="62" y="16" width="52" height="40" rx="6"/><circle cx="76" cy="36" r="6"/><circle cx="76" cy="36" r="2" fill="currentColor"/><path d="M8 54c14 0 18-18 34-18h22" stroke-dasharray="3 4"/><rect x="2" y="48" width="12" height="10" rx="2"/></svg>`;

// The merged title bar: these elements drag the window (Tauri needs the attribute on each element, not just the parent).
const drag = 'data-tauri-drag-region';
const header = (title: string, subtitle: string, right = '') =>
  `<header class="page-header" ${drag}><h1 tabindex="-1" ${drag}>${title}</h1><span class="subtitle" ${drag}>${subtitle}</span><span class="spacer" ${drag}></span>${right}</header>`;

const helperDown = () => empty({ illus: icon('power', 'illus'), title: 'The background helper isn’t running',
  body: 'Live numbers and warnings need it running.',
  actions: button('Start helper', 'start-helper', { kind: 'primary' }) + button('Open Login Items…', 'open', { id: 'login-items' }) });

// ---------- Sidebar ----------
export function sidebar(v: View) {
  const { s, m, ui } = v;
  const missing = s ? usable(s).filter((d) => !d.connected && stowedOn(s, d.id).length).length : 0;
  // [route, label, visible badge, what VoiceOver says after the label]
  const nav: [Route, string, string, string][] = [
    ['overview', 'Overview', s && strained(s) ? '<span class="dot warn" style="margin-left:auto"></span>' : '', s && strained(s) ? ', memory strained' : ''],
    ['offload', 'Offload', s ? `<span class="badge">${suggested(s).length}</span>` : '', s ? `, ${suggested(s).length} folders can move` : ''],
    ['drives', 'Drives', missing ? `<span class="badge is-danger">${missing} missing</span>` : '', missing ? `, ${missing} missing` : ''],
    ['memory', 'Memory', '', ''],
    ['internet', 'Internet', '', ''],
    ['clean', 'Clean', '', ''],
  ];
  const item = ([r, label, extra, spoken]: [Route, string, string, string]) =>
    { const on = ui.route === r && !(r === 'drives' && ui.drive); return `<a class="nav-item ${on ? 'is-active' : ''}" href="#/${r}" aria-label="${label}${spoken}" ${on ? 'aria-current="page"' : ''}>${icon(r)}${label}${extra}</a>`; };
  const cpu = m?.cpu.at(-1) ?? 0;
  const foot = s && m ? `<div class="sidebar-foot" aria-label="This Mac now">
      <div class="mini"><span>CPU</span>${capacity([{ value: cpu, color: 'var(--text-secondary)', label: `CPU ${cpu}%` }], 100)}<span class="num">${cpu}%</span></div>
      <div class="mini"><span>Memory</span>${capacity([{ value: m.pressure, color: strained(s) ? 'var(--warn-fill)' : 'var(--ok-fill)', label: `Memory pressure ${m.pressure}%` }], 100)}<span class="num">${m.pressure}%</span></div>
      <div class="mini"><span>Disk</span>${capacity([{ value: s.mac.total - s.mac.free, color: s.mac.free < 40 * GB ? 'var(--danger-fill)' : 'var(--text-secondary)', label: `${gb(s.mac.free)} free` }], s.mac.total)}<span class="num">${Math.round(s.mac.free / GB)} GB</span></div>
    </div>` : '';
  return `<aside class="sidebar"><div class="traffic" ${drag}></div>
    <nav aria-label="Sections">${nav.map(item).join('')}
      ${s && usable(s).length ? `<div class="nav-group" id="nav-drives">Drives</div><div role="group" aria-labelledby="nav-drives">
      ${usable(s).map((d) => { const on = ui.route === 'drives' && ui.drive === d.id, n = stowedOn(s, d.id).length; return `<a class="nav-item nav-drive ${on ? 'is-active' : ''} ${d.connected ? '' : 'is-missing'}" href="#/drives/${encodeURIComponent(d.id)}" ${on ? 'aria-current="page"' : ''} aria-label="${esc(d.name)}, ${d.connected ? `${gb(d.free)} free` : 'missing'}, ${n} folder${n === 1 ? '' : 's'}">${icon('drives')}<span class="name">${esc(d.name)}</span>${d.connected ? `<span class="badge">${gb(d.free)}</span>` : '<span class="badge is-danger">missing</span>'}</a>`; }).join('')}</div>` : ''}
      <div class="nav-group"></div>${item(['settings', 'Settings', s?.update.available ? '<span class="badge" style="color:var(--accent-text)">Update</span>' : '', s?.update.available ? ', update available' : ''])}
    </nav>${foot}</aside>`;
}

// ---------- Overview ----------
const runsOut = (f: Forecast) => f.minutesLeft == null ? 'Memory is running short.' : f.minutesLeft < 1 ? 'Memory is about to run out.' : `Memory runs out in about ${f.minutesLeft} minute${f.minutesLeft === 1 ? '' : 's'}.`;

function forecastCard(v: V, { big = false } = {}) {
  const { s, m, ui } = v, f = s.forecast;
  if (!f) return `<section class="card" style="display:grid;gap:10px" aria-busy="true"><span class="skeleton" style="height:14px;width:30%"></span><span class="skeleton" style="height:26px;width:60%"></span><span class="skeleton" style="height:${big ? 200 : 120}px"></span>
    <p class="secondary">Collecting samples for the forecast…</p></section>`;
  const ok = f.condition === 'fine';
  const back = Math.min(20, f.history.length - 1), grew = f.history.at(-1)! - f.history.at(-1 - back)!;
  return `<section class="card${ok ? '' : ' is-alert'}" aria-labelledby="fc-title">
    <div class="forecast">
      <div style="display:grid;gap:6px">
        <div style="display:flex;gap:8px;align-items:center"><h2 class="secondary" style="font:var(--type-group)">Memory forecast</h2>${ok ? pill('ok', 'Fine') : pill(f.condition === 'critical' ? 'danger' : 'warn', f.condition === 'critical' ? 'Critical' : 'Strained')}</div>
        <p class="sentence" id="fc-title">${ok ? 'Memory looks fine for the next few hours.' : runsOut(f)}</p>
        <p class="why">${ok ? `Swap (memory macOS moved to disk): ${gb(f.swap)}${f.swapRate < 0 ? ', shrinking' : ''}.`
          : `Swap grew ${gb(grew)} in the last ${back} minute${back === 1 ? '' : 's'}. It has room for ${gb(f.swapLimit - f.swap)} more. When swap can’t grow, macOS shows “Your system has run out of application memory” and pauses apps.`}</p>
        ${ok || !f.suggest ? '' : `<div class="actions">${button(`Quit ${esc(f.suggest.app)}…`, 'quit-app', { kind: 'primary', id: f.suggest.appId })}${ui.route === 'memory' ? '' : button('What’s using memory', 'go', { id: 'memory' })}<span class="tertiary" style="font:var(--type-caption)">Frees about ${gb(f.suggest.frees)}</span></div>`}
      </div>
      <dl style="margin:0;display:grid;gap:2px;text-align:right">
        <dt class="secondary" style="font:var(--type-caption)">Memory pressure</dt><dd class="num" style="margin:0;font:var(--type-figure)">${m ? `${m.pressure}%` : '<span class="skeleton" style="display:inline-block;width:56px;height:24px"></span>'}</dd>
        <dt class="secondary" style="font:var(--type-caption);margin-top:6px">Swap</dt><dd class="num" style="margin:0">${gb(f.swap)}, ${f.swapRate < 0 ? 'falling' : `+${gb(f.swapRate)}/h`}</dd>
      </dl>
    </div>
    <div class="forecast-chart" style="margin-top:12px">
      <div class="chart-key" aria-hidden="true"><span><i style="background:var(--text-primary)"></i>Swap, last hour</span><span><i style="background:repeating-linear-gradient(90deg,${ok ? 'var(--ok-fill)' : 'var(--warn-fill)'} 0 4px,transparent 4px 7px)"></i>Forecast</span></div>
      ${forecastChart(f, { height: big ? 220 : 150 })}
    </div></section>`;
}

function metrics(v: V) {
  const { s, m } = v;
  if (!m) return `<div class="metrics" aria-busy="true">${'<div class="metric"><span class="skeleton" style="height:12px;width:40%"></span><span class="skeleton" style="height:26px;width:60%;margin-top:6px"></span></div>'.repeat(4)}</div>`;
  const cpu = m.cpu, net = m.netIn, low = s.mac.free < 40 * GB;
  return `<div class="metrics">
    <div class="metric"><span class="label">CPU</span><span class="figure num">${cpu.at(-1)}<small>%</small></span><span class="detail">${esc(m.cores)}</span>${sparkline(cpu, 'var(--accent-text)', 100)}</div>
    <div class="metric"><span class="label">Memory</span><span class="figure num">${(m.memUsed / GB).toFixed(1)}<small>of ${Math.round(m.memTotal / GB)} GB</small></span><span class="detail">Swap ${gb(m.swap)} · compressed ${gb(m.compressed)}</span>${capacity([{ value: m.memUsed, color: strained(s) ? 'var(--warn-fill)' : 'var(--ok-fill)', label: `${gb(m.memUsed)} used` }], m.memTotal)}</div>
    <div class="metric ${low ? 'is-danger' : ''}"><span class="label">Disk</span><span class="figure num">${(s.mac.free / GB).toFixed(1)}<small>GB free</small></span><span class="detail">of ${Math.round(s.mac.total / GB)} GB on Macintosh HD</span>${capacity([{ value: s.mac.total - s.mac.free, color: 'var(--text-secondary)', label: `${gb(s.mac.total - s.mac.free)} used` }], s.mac.total)}</div>
    <div class="metric"><span class="label">Network</span><span class="figure num">${(net.at(-1)! / 1e6).toFixed(1)}<small>MB/s in</small></span><span class="detail">${Math.round(m.netOut / 1e3)} KB/s out${s.net?.info ? ` · ${esc(s.net.info.kind)}` : ''}</span>${sparkline(net, 'var(--ok-fill)', Math.max(...net, 1e6))}</div>
  </div>${internetSummary(s)}`;
}

// Speed test. Servers: Cloudflare (nearest), Speedtest servers near you or found by search, a few regional
// download files (download and ping only), or any file the user links to.
export const FILES: [string, string][] = [
  ['https://ash-speed.hetzner.com/100MB.bin', 'Washington DC, USA (file)'],
  ['https://hil-speed.hetzner.com/100MB.bin', 'Oregon, USA (file)'],
  ['https://sin-speed.hetzner.com/100MB.bin', 'Singapore (file)'],
  ['https://fsn1-speed.hetzner.com/100MB.bin', 'Falkenstein, Germany (file)'],
  ['https://proof.ovh.net/files/100Mb.dat', 'Roubaix, France (file)'],
];
const GAUGE = [0, 5, 10, 50, 100, 250, 500, 750, 1000];
// Equal angle per scale step, linear inside a step, across a 240° arc.
const gaugeAngle = (v: number) => {
  const k = GAUGE.findIndex((g) => v < g);
  const i = k < 0 ? GAUGE.length - 1 : Math.max(1, k);
  const f = k < 0 ? 1 : (v - GAUGE[i - 1]) / (GAUGE[i] - GAUGE[i - 1]);
  return -120 + ((i - 1 + f) / (GAUGE.length - 1)) * 240;
};
function gauge(n: NonNullable<State['net']>) {
  const l = n.live, t = n.test;
  const phase = l ? l.phase : t ? 'done' : 'idle';
  const val = l ? (l.phase === 'ping' ? l.ping ?? 0 : l.mbps) : t ? t.down ?? t.up ?? 0 : 0;
  const color = phase === 'up' ? '#B57BFF' : phase === 'ping' ? '#FFB547' : '#3FC8FF';
  const label = { idle: 'READY', ping: 'PING', down: 'DOWNLOAD', up: 'UPLOAD', done: 'DONE' }[phase];
  const r = 92, cx = 120, cy = 116, pt = (deg: number, rr = r) => [cx + rr * Math.sin((deg * Math.PI) / 180), cy - rr * Math.cos((deg * Math.PI) / 180)];
  const arc = (a: number, b: number) => { const [x1, y1] = pt(a), [x2, y2] = pt(b); return `M${x1.toFixed(1)} ${y1.toFixed(1)} A${r} ${r} 0 ${b - a > 180 ? 1 : 0} 1 ${x2.toFixed(1)} ${y2.toFixed(1)}`; };
  const ang = phase === 'ping' ? -120 + Math.min(val / 200, 1) * 240 : gaugeAngle(val);
  return `<div class="gauge" style="--g:${color}"><svg viewBox="0 0 240 200" aria-hidden="true">
    <defs><linearGradient id="gauge-grad" x1="0" x2="1"><stop offset="0" stop-color="${color}" stop-opacity="0.35"/><stop offset="1" stop-color="${color}"/></linearGradient></defs>
    <path d="${arc(-120, 120)}" class="gauge-track"/>
    ${phase === 'idle' ? '' : `<path d="${arc(-120, Math.max(-119.5, ang))}" class="gauge-fill" stroke="url(#gauge-grad)"/>`}
    ${GAUGE.map((g, i) => { const [x, y] = pt(-120 + (i / (GAUGE.length - 1)) * 240, r - 22); return `<text x="${x.toFixed(1)}" y="${(y + 4).toFixed(1)}" class="gauge-tick">${g}</text>`; }).join('')}
    <g class="gauge-needle" style="transform:rotate(${ang.toFixed(1)}deg)"><path d="M${cx - 4} ${cy} L${cx} ${cy - r + 14} L${cx + 4} ${cy} Z"/><circle cx="${cx}" cy="${cy}" r="7"/></g>
  </svg><div class="gauge-read"><span class="gauge-phase">${label}</span><span class="gauge-val num">${phase === 'ping' ? Math.round(val) : val.toFixed(val < 10 ? 1 : 0)}</span><span class="gauge-unit">${phase === 'ping' ? 'ms' : phase === 'up' ? '↑ Mbps' : '↓ Mbps'}</span></div></div>`;
}

// Overview: the last test in one line of figures, and where to go for more.
function internetSummary(s: State) {
  const n = s.net, t = n?.test, i = n?.info;
  if (!n) return '';
  const mbps = (v: number | null | undefined) => v == null ? '–' : v.toFixed(v < 10 ? 1 : 0);
  return `<section class="card internet-mini">
    <div class="card-head"><h2>Internet</h2><span class="secondary" style="font:var(--type-caption)">${t ? `${ago(t.at)} · ${esc(t.server)}` : 'Not tested yet'}</span><span class="spacer"></span>${button('Open', 'go', { id: 'internet' })}</div>
    <div class="mini-figs"><span>${icon('down')}<b class="num">${mbps(t?.down)}</b> Mbps</span><span>${icon('up')}<b class="num">${mbps(t?.up)}</b> Mbps</span><span>${icon('pulse')}<b class="num">${t ? Math.round(t.ping) : '–'}</b> ms</span>
      <span class="tertiary">${[t?.isp, t?.ip, i ? `${i.kind} ${i.ip}` : ''].filter(Boolean).map((v) => esc(v!)).join(' · ')}</span></div>
  </section>`;
}

// The server picker: one box that shows the current server; typing searches. Rows are built here and kept in
// `serverOptions` so a click or Enter picks by index.
export type ServerOption = { value: string; title: string; sub: string; meta: string; group: string };
export let serverOptions: ServerOption[] = [];
function serverPicker(n: NonNullable<State['net']>, ui: Ui) {
  const q = (ui.serverQuery ?? '').trim(), ql = q.toLowerCase();
  const match = (o: ServerOption) => !ql || `${o.title} ${o.sub}`.toLowerCase().includes(ql);
  const seen = new Set<string>();
  const recent: ServerOption[] = n.history.filter((h) => h.key != null && !seen.has(h.key) && seen.add(h.key)).slice(0, 4)
    .map((h) => ({ value: h.key!, title: h.server, sub: `last ${ago(h.at)}`, meta: `${h.down != null ? Math.round(h.down) : '–'}↓ ${h.up != null ? Math.round(h.up) : '–'}↑`, group: 'Recently used' }));
  const ookla: ServerOption[] = n.servers.map((v) => ({ value: `ookla:${v.host}`, title: v.sponsor, sub: `${v.city}, ${v.cc}`, meta: v.km != null ? `${Math.round(v.km).toLocaleString('en-GB')} km` : '', group: q ? `Speedtest servers matching “${q}”` : 'Speedtest servers near you' }));
  const files: ServerOption[] = FILES.map(([u, l]) => ({ value: u, title: l.replace(' (file)', ''), sub: 'download file', meta: '↓ only', group: 'Download files' }));
  const url = /^https?:\/\/\S+$/i.test(q) ? [{ value: q, title: 'Use this file', sub: q, meta: '↓ only', group: 'Custom file' }] : [];
  serverOptions = [...url, { value: '', title: 'Nearest server', sub: 'Cloudflare picks the closest of its sites', meta: 'Auto', group: '' }, ...recent.filter(match), ...ookla, ...files.filter(match)];
  const cur = n.server === '' ? 'Nearest server — Cloudflare' : n.serverLabel || n.server.replace(/^ookla:/, '');
  const hi = Math.min(ui.serverIndex ?? 0, serverOptions.length - 1);
  let last = '';
  const rows = serverOptions.map((o, i) => {
    const head = o.group && o.group !== last ? `<div class="pick-group">${esc(o.group)}</div>` : '';
    last = o.group;
    return `${head}<div class="pick-row ${i === hi ? 'is-hi' : ''} ${o.value === n.server ? 'is-cur' : ''}" data-action="pick-server" data-id="${i}" role="option" aria-selected="${o.value === n.server}">
      <span class="pick-ic">${icon(o.value === '' ? 'pulse' : o.value.startsWith('ookla:') ? 'router' : o.group === 'Recently used' ? 'history' : 'down')}</span>
      <span class="pick-text"><strong>${esc(o.title)}</strong><small>${esc(o.sub)}</small></span><span class="pick-meta num">${esc(o.meta)}</span>${o.value === n.server ? icon('check', 'pick-check') : ''}</div>`;
  }).join('');
  return `<div class="pick ${ui.serverOpen ? 'is-open' : ''}">
    <div class="pick-box">${icon('globe')}<input class="pick-input" data-action="server-box" role="combobox" aria-expanded="${!!ui.serverOpen}" aria-label="Test server" spellcheck="false" autocomplete="off"
      placeholder="${esc(cur)}" value="${esc(ui.serverOpen ? ui.serverQuery ?? '' : cur)}" ${n.testing ? 'disabled' : ''}><span class="pick-caret">${ui.searching ? '<i class="pick-spin"></i>' : '⌄'}</span></div>
    ${ui.serverOpen ? `<div class="pick-panel" role="listbox">${rows || '<div class="pick-empty">No servers found. Try a city or country, or paste a link to a large file.</div>'}</div>` : ''}
  </div>`;
}

// The Internet page: speed test with gauge, servers, the connection, and every past result.
export function internet(v: View) {
  const h = header('Internet', 'Speed, connection and past tests.');
  if (!v.s?.net) return h + loadingPage();
  return h + `<div class="page-body">${internetCard(v.s, v.ui)}</div>`;
}

function internetCard(s: State, ui: Ui) {
  const n = s.net;
  if (!n) return '';
  const t = n.test, i = n.info, mbps = (v: number | null | undefined) => v == null ? '–' : v.toFixed(v < 10 ? 1 : 0);
  const fig = (ic: string, label: string, value: string, unit: string) => `<div><span class="label">${icon(ic)}${label}</span><span class="figure num">${value}<small>${unit}</small></span></div>`;
  const fact = (ic: string, label: string, value: string) => value ? `<div class="fact">${icon(ic)}<span>${label}</span><strong class="num" title="${esc(value)}">${esc(value)}</strong></div>` : '';
  const rows = n.history;
  return `<section class="card internet">
    <div class="card-head"><h2>Speed test</h2><span class="secondary" style="font:var(--type-caption)">${t ? `Last ${ago(t.at)} · ${esc(t.server)}` : 'Not tested yet'}</span><span class="spacer"></span>
      ${seg('speed-mode', [['full', 'Full'], ['down', '↓ only'], ['up', '↑ only']], n.mode)}</div>
    <div class="speed-grid">
      ${gauge(n)}
      <div class="speed-side">
        ${serverPicker(n, ui)}
        ${button(n.testing ? 'Testing…' : t ? 'Test again' : 'Start test', 'speed-test', { kind: 'primary', large: true, disabled: n.testing || !i })}
        <div class="internet-figs">${fig('down', 'Download', mbps(t?.down), 'Mbps')}${fig('up', 'Upload', mbps(t?.up), 'Mbps')}${fig('pulse', 'Ping', t ? Math.round(t.ping).toString() : '–', 'ms')}${fig('pulse', 'Jitter', t ? Math.round(t.jitter).toString() : '–', 'ms')}</div>
      </div>
    </div>
    <div class="facts">${fact('globe', 'Public IP', t?.ip ?? '')}${fact('building', 'Provider', t?.isp ?? '')}${fact('pin', 'Location', t?.city ? `${t.city}, ${t.country}` : '')}${fact('wifi', i?.kind ?? 'Network', i ? i.ip : 'Not connected')}${fact('router', 'Router', i?.gateway ?? '')}</div>
    ${rows.length ? `<div class="results"><div class="results-head"><span>${icon('history')}Recent results</span><span class="tertiary" style="font-weight:400">${n.history.length} test${n.history.length === 1 ? '' : 's'}</span></div>
      <div class="results-row results-cols"><span>When</span><span>↓ Mbps</span><span>↑ Mbps</span><span>Ping</span><span>Server</span></div>
      ${rows.map((h) => `<div class="results-row"><span>${day(h.at)}, ${clock(h.at)}</span><span class="num dl">${mbps(h.down)}</span><span class="num ul">${mbps(h.up)}</span><span class="num">${Math.round(h.ping)} ms</span><span class="tertiary" title="${esc(h.server)}">${esc(h.server)}</span></div>`).join('')}</div>` : ''}
  </section>`;
}

const loadingPage = () => `<div class="page-body" aria-busy="true"><div class="card" style="display:grid;gap:10px"><span class="skeleton" style="height:14px;width:30%"></span><span class="skeleton" style="height:26px;width:60%"></span><span class="skeleton" style="height:120px"></span></div>
  <div class="metrics">${'<div class="metric"><span class="skeleton" style="height:12px;width:40%"></span><span class="skeleton" style="height:26px;width:60%;margin-top:6px"></span></div>'.repeat(4)}</div></div>`;

export function overview(v: View) {
  const h = header('Overview', v.s ? esc(v.s.mac.model) : '');
  if (v.down) return h + `<div class="page-body">${helperDown()}</div>`;
  if (!v.s) return h + loadingPage();
  const s = v.s, stowable = sum(suggested(s));
  const other = s.mac.total - s.mac.system - s.mac.apps - stowable - s.mac.free;
  const drives = usable(s);
  return h + `<div class="page-body">${forecastCard(v as V)}${metrics(v as V)}
    <div class="split">
      <section class="card"><div class="card-head"><h2>This Mac</h2><span class="secondary"><span class="num">${gb(s.mac.free)}</span> free of <span class="num">${Math.round(s.mac.total / GB)} GB</span></span><span class="spacer"></span>${suggested(s).length ? button(`Review ${suggested(s).length} folders`, 'go', { id: 'offload' }) : ''}</div>
        ${s.mac.system + s.mac.apps ? `${capacity([{ value: s.mac.system, color: 'var(--storage-system)', label: `macOS ${gb(s.mac.system)}` }, { value: s.mac.apps, color: 'var(--storage-apps)', label: `Apps ${gb(s.mac.apps)}` }, { value: stowable, color: 'var(--accent-fill)', label: `Can move ${gb(stowable)}` }, { value: other, color: 'var(--storage-other)', label: 'Other' }], s.mac.total, true)} <div class="legend" aria-hidden="true"><span><i style="background:var(--storage-system)"></i>macOS ${gb(s.mac.system)}</span><span><i style="background:var(--storage-apps)"></i>Apps ${gb(s.mac.apps)}</span><span><i style="background:var(--accent-fill)"></i>Can move to a drive ${gb(stowable)}</span><span><i style="background:var(--storage-other)"></i>Other</span></div>` : `${capacity([{ value: s.mac.total - s.mac.free - stowable, color: 'var(--storage-system)', label: 'Used' }, { value: stowable, color: 'var(--accent-fill)', label: `Can move to a drive ${gb(stowable)}` }], s.mac.total, true)}
        <div class="legend" aria-hidden="true"><span><i style="background:var(--storage-system)"></i>Used ${gb(s.mac.total - s.mac.free - stowable)}</span><span><i style="background:var(--accent-fill)"></i>Can move to a drive ${gb(stowable)}</span></div>`}</section>
      <section class="card"><div class="card-head"><h2>Drives</h2></div>${drives.length ? `<div style="display:grid;gap:8px">${drives.map((d) => {
        const on = stowedOn(s, d.id);
        return `<div style="display:flex;gap:8px;align-items:center"><span class="dot ${d.connected ? 'ok' : 'hollow'}"></span><strong style="font:var(--type-emphasis)">${esc(d.name)}</strong>
          <span class="secondary" style="margin-left:auto;font:var(--type-callout)">${d.connected ? (on.length ? `${on.length} folders · ${gb(sum(on))}` : `${gb(d.free)} free`) : `<span style="color:var(--danger-text)">Missing</span> · ${esc(list(appsOn(on)))} held`}</span></div>`;
      }).join('')}</div>` : '<p class="secondary">Connect an APFS drive to start moving app data.</p>'}</section>
    </div></div>`;
}

// ---------- Offload ----------
function itemRow(v: V, it: Folder) {
  const s = v.s, job = s.jobs.find((j) => j.folder === it.id);
  const head = `${appIcon(v.icon(it.appPath), it.app)}<div class="name"><strong>${esc(it.name)}</strong><span>${esc(it.app)}</span></div>`;
  if (job) {
    const d = drive(s, job.drive), pct = (job.done / job.total) * 100;
    return `<div class="table-row is-busy" role="row">${head}<div class="progress-cell">${steps(job.kind, job.step)}${progress(pct, false, `${job.kind === 'move' ? 'Moving' : 'Bringing back'} ${it.app} ${it.name}`)}
      <span class="tertiary num" style="font:var(--type-caption)">${gb(job.done)} of ${gb(job.total)} ${job.kind === 'move' ? `to ${esc(d.name)}` : 'to this Mac'}. The original is kept until the check passes.</span></div>
      <div class="act">${button('Stop', 'stop', { kind: 'plain', id: it.id, label: `Stop ${it.app} ${it.name}` })}</div></div>`;
  }
  const preview = s.settings.dryRun;
  let verdict: string, act: string;
  if (it.verdict === 'move' && it.how?.length) {
    verdict = pill('accent', 'Can move') + `<span>${esc(it.app)}’s own setting can do it</span>`;
    act = button('Show how', 'how', { id: it.id, label: `Show how ${it.app} can keep ${it.name} on a drive` });
  } else if (it.verdict === 'move') {
    verdict = pill('accent', 'Can move') + `<span>${esc(it.running ? `${it.app} is open and will be asked to quit` : it.reason)}</span>`;
    act = it.running ? button(preview ? 'Quit & move…' : 'Quit & move', 'move', { kind: 'primary', id: it.id, label: `Quit ${it.app} and move ${it.name}` }) : button(preview ? 'Move…' : 'Move', 'move', { id: it.id, label: `Move ${it.app} ${it.name}` });
  } else if (it.verdict === 'setting') {
    verdict = pill('neutral', `Use ${it.app}’s setting`) + `<span>${esc(it.reason)}</span>`;
    act = button('Show how', 'how', { id: it.id, label: `Show how to move ${it.app} ${it.name}` });
  } else if (it.verdict === 'refused') {
    verdict = pill('danger', 'Can’t move') + `<span>${esc(it.reason)}</span>`;
    act = button('Why?', 'why', { kind: 'plain', id: it.id, label: `Why ${it.name} can’t move` });
  } else {
    const d = drive(s, it.drive), held = isHeld(s, it);
    verdict = held ? pill('danger', `${d.name} is missing`) + `<span>${esc(it.app)} is held until it’s back</span>`
      : pill('ok', `On ${d.name}`) + `<span>${it.since ? `Since ${day(it.since)}, checked file by file` : 'Linked by hand, before iStash'}</span>`;
    // The engine only undoes moves it made itself (its journal has them), so a hand-made link gets no button.
    act = !it.since ? '' : button('Check', 'check', { kind: 'plain', id: it.id, disabled: held, title: held ? `Connect ${d.name} first` : 'Read every file on the drive and compare it with what was moved', label: `Check ${it.app} ${it.name}` })
      + button('Bring back…', 'back', { id: it.id, disabled: held, title: held ? `Connect ${d.name} first` : '', label: `Bring back ${it.app} ${it.name}` });
  }
  return `<div class="table-row" role="row">${head}<div class="path" title="${esc(it.path)}">${esc(it.path)}</div><div class="size">${gb(it.size)}</div><div class="verdict">${verdict}</div><div class="act">${act}</div></div>`;
}

export function offload(v: View) {
  const s = v.s;
  const tabs = s && { suggested: suggested(s).concat(s.folders.filter((i) => ['setting', 'refused'].includes(i.verdict))), stowed: s.folders.filter((i) => i.verdict === 'stowed'), all: s.folders };
  const ready = s && !s.scan.running && s.permissions.volumes && usable(s).length;
  const right = tabs && ready ? seg('tab', [['suggested', 'Suggested', tabs.suggested.length], ['stowed', 'On drives', tabs.stowed.length], ['all', 'All']], v.ui.offloadTab) + button('Scan again', 'scan') : '';
  const h = header('Offload', 'Move heavy app data to a drive. The apps keep working.', right);
  const head = '<div class="table-head" role="row"><span></span><span>Folder</span><span>Where it lives</span><span style="text-align:right">Size</span><span>Verdict</span><span></span></div>';
  if (v.down) return h + `<div class="page-body">${helperDown()}</div>`;
  if (!s || s.scan.running) return h + `<div class="page-body"><section class="card" style="display:grid;gap:8px"><div style="display:flex;justify-content:space-between"><h2>Looking for big app folders…</h2>${s ? `<span class="num secondary">${s.scan.checked.toLocaleString('en-GB')} checked</span>` : ''}</div>${progress(0, true, 'Scanning')}
    <p class="secondary" style="font:var(--type-caption)">Read only. If macOS asks for access, choose Allow.</p></section><div class="table" aria-hidden="true">${head}${skeletonRows(5)}</div></div>`;
  if (!s.permissions.volumes) return h + `<div class="page-body">${empty({ illus: icon('lock', 'illus'), title: 'iStash can’t see your drives',
    body: 'macOS hasn’t allowed access to removable volumes, so nothing can be moved. Nothing was changed.', actions: button('Allow access…', 'grant', { kind: 'primary', id: 'volumes' }) + button('Open System Settings…', 'open', { id: 'privacy' }) })}</div>`;
  if (!usable(s).length) return h + `<div class="page-body">${empty({ illus: ILLUS_PORT, title: 'Connect a drive to start',
    body: 'Plug in an APFS or Mac OS Extended drive.', actions: button('Which drives work?', 'apfs', { kind: 'plain' }) })}</div>`;
  const busy = s.jobs.map((j) => s.folders.find((i) => i.id === j.folder)!);
  const rows = [...new Set([...busy, ...tabs![v.ui.offloadTab]])];
  const stowable = sum(suggested(s));
  // The same choice the move sheet opens with (main.ts target()).
  const targets = usable(s).filter((d) => d.connected && !d.problem), to = targets.find((d) => d.id === s.settings.defaultDrive) ?? targets[0];
  return h + `<div class="page-body">
    <section class="card" style="display:grid;grid-template-columns:1fr auto auto;gap:8px 32px;align-items:end">
      <div><h2 class="secondary" style="font:var(--type-group)">This Mac</h2><span class="num" style="font:var(--type-figure)">${gb(s.mac.free)}</span> <span class="secondary">free of ${Math.round(s.mac.total / GB)} GB</span></div>
      <div class="move-to"><label class="secondary" for="default-drive" style="font:var(--type-group)">Move to</label>${targets.length
        ? `<select class="select" id="default-drive" data-action="default-drive">${targets.map((d) => `<option value="${esc(d.id)}" ${d.id === to?.id ? 'selected' : ''}>${esc(d.name)} · ${gb(d.free)} free</option>`).join('')}</select>`
        : '<span style="color:var(--danger-text)">No drive that can take folders is connected</span>'}</div>
      <div style="text-align:right"><h2 class="secondary" style="font:var(--type-group)">After moving the ${suggested(s).length} suggested folders</h2><span class="num" style="font:var(--type-figure);color:var(--ok-text)">${gb(s.mac.free + stowable)}</span> <span class="secondary">free</span></div>
      <div style="grid-column:1/-1">${capacity([{ value: s.mac.total - s.mac.free - stowable, color: 'var(--storage-system)', label: 'Used' }, { value: stowable, color: 'var(--accent-fill)', label: `Can move ${gb(stowable)}` }], s.mac.total, true)}</div>
    </section>
    <div class="table" role="table" aria-label="App folders">${head}${rows.length ? rows.map((it) => itemRow(v as V, it)).join('') : '<div class="empty" style="padding:32px"><h2>Nothing here yet</h2><p>Folders you move to a drive show up in this list.</p></div>'}</div>
    <p class="tertiary" style="font:var(--type-caption)">Documents, Desktop and app sandboxes are never touched. Interrupted moves are undone safely.</p>
  </div>`;
}

// ---------- Drives ----------
export type Cable = { app: Folder; d: Drive; ya: number; yp: number; floor: number };
export function cableLayout(s: State) {
  const linked = s.folders.filter((i) => i.verdict === 'stowed');
  // Apps are listed drive by drive, biggest first, so cables run in bands instead of crossing.
  const order = usable(s).map((d) => d.id);
  const rank = (app: string) => Math.min(...linked.filter((i) => i.app === app).map((i) => order.indexOf(i.drive!)).map((r) => (r < 0 ? 99 : r)));
  const weight = (app: string) => linked.filter((i) => i.app === app).reduce((n, i) => n + i.size, 0);
  const apps = [...new Map(linked.map((i) => [i.app, i])).values()].sort((a, b) => rank(a.app) - rank(b.app) || weight(b.app) - weight(a.app));
  const rowY = (i: number) => 34 + i * 54;
  const jackX = 300, portX = 640;
  const blocks: { d: Drive; y: number; h: number }[] = []; let y = 10;
  for (const d of usable(s)) {
    const n = Math.max(1, apps.filter((a) => linked.some((i) => i.app === a.app && i.drive === d.id)).length);
    blocks.push({ d, y, h: n * 40 + 30 }); y += n * 40 + 46;
  }
  const H = Math.max(rowY(apps.length - 1) + 40, y) + 10;
  const cables: Cable[] = []; const used: Record<string, number> = {};
  apps.forEach((a, i) => {
    for (const { d, y: by } of blocks) {
      if (!linked.some((x) => x.app === a.app && x.drive === d.id)) continue;
      used[d.id] = (used[d.id] || 0) + 1;
      cables.push({ app: a, d, ya: rowY(i), yp: by + 24 + (used[d.id] - 1) * 40, floor: H - 12 });
    }
  });
  return { apps, blocks, cables, H, rowY, jackX, portX, linked };
}

// t = 1 plugged in, 0 hanging loose. Both shapes are one cubic so they interpolate.
export function cablePath(c: Pick<Cable, 'ya' | 'yp' | 'floor'>, t: number, jackX = 300, portX = 640) {
  const sag = 46 + Math.abs(c.yp - c.ya) * 0.15;
  const on = [jackX, c.ya, jackX + 130, c.ya + sag, portX - 140, c.yp + sag, portX, c.yp];
  const off = [jackX, c.ya, jackX + 40, c.ya + 50, jackX + 110, c.floor, jackX + 180, c.floor];
  const p = on.map((v, i) => (off[i] + (v - off[i]) * t).toFixed(1));
  return `M${p[0]} ${p[1]} C${p[2]} ${p[3]} ${p[4]} ${p[5]} ${p[6]} ${p[7]}`;
}

function cableMap(v: V) {
  const s = v.s, L = cableLayout(s);
  const sizeOf = (app: string) => sum(L.linked.filter((i) => i.app === app));
  const appRows = L.apps.map((a, i) => {
    const held = L.linked.some((x) => x.app === a.app && isHeld(s, x));
    const y = L.rowY(i), src = v.icon(a.appPath);
    return `<g data-hot="app:${esc(a.app)}"><rect class="hit" x="0" y="${y - 24}" width="${L.jackX + 12}" height="48"/>
      <rect x="0" y="${y - 14}" width="28" height="28" rx="7" fill="var(--bg-inset)"/>${src ? `<image href="${esc(src)}" x="0" y="${y - 14}" width="28" height="28"/>` : ''}
      <text x="40" y="${y - 2}">${esc(a.app)}</text><text x="40" y="${y + 13}" class="sub ${held ? 'bad' : ''}">${held ? 'held, drive missing' : gb(sizeOf(a.app))}</text>
      <circle cx="${L.jackX}" cy="${y}" r="8" fill="var(--jack-hole)" stroke="${held ? 'var(--danger-fill)' : 'var(--jack-ring)'}" stroke-width="2"/></g>`;
  }).join('');
  const blocks = L.blocks.map(({ d, y, h }) => {
    const on = d.connected, ports = L.cables.filter((c) => c.d.id === d.id);
    return `<g data-hot="drive:${d.id}" class="drive-block ${on ? '' : 'is-missing'}" data-action="go" data-id="drives/${encodeURIComponent(d.id)}" role="link" aria-label="Open ${esc(d.name)}"><rect x="${L.portX - 22}" y="${y}" width="${880 - L.portX + 22}" height="${h}" rx="8" fill="${on ? 'var(--bg-inset)' : 'none'}" stroke="${on ? 'var(--line-strong)' : 'var(--danger-fill)'}" ${on ? '' : 'stroke-dasharray="4 4"'}/>
      <rect x="866" y="${y + 14}" width="8" height="8" rx="2" fill="${cableVar(d)}"/>
      <text x="858" y="${y + 22}" text-anchor="end" style="font-weight:600">${esc(d.name)}</text>
      <text x="858" y="${y + 38}" text-anchor="end" class="sub ${on ? '' : 'bad'}">${on ? `${gb(d.free)} free` : `not connected · seen ${ago(d.lastSeen)}`}</text>
      ${ports.map((c) => `${on ? '' : `<circle class="port-pulse" cx="${L.portX}" cy="${c.yp}" r="8"/>`}<circle class="port" cx="${L.portX}" cy="${c.yp}" r="8" fill="var(--jack-hole)" stroke="${on ? 'var(--jack-ring)' : 'var(--danger-fill)'}" stroke-width="2"/>`).join('')}</g>`;
  }).join('');
  const cables = L.cables.map((c, i) => {
    const on = c.d.connected;
    const d = cablePath(c, on ? 1 : 0, L.jackX, L.portX), busy = s.jobs.some((j) => j.drive === c.d.id && L.linked.some((f) => f.id === j.folder && f.app === c.app.app));
    // Shadow, body, sheen and moving data: four strokes on one path, so the cable reads as a tube with something in it.
    return `<g class="cable-group ${on ? 'is-on' : 'is-loose'} ${busy ? 'is-busy' : ''}" data-app="${esc(c.app.app)}" data-drive="${c.d.id}" style="--c:${cableVar(c.d)};--i:${i};transform-origin:${L.jackX}px ${c.ya}px" data-hot="app:${esc(c.app.app)}">
      <path class="cable-shadow" data-cable="${i}" d="${d}"/>
      <path class="cable" data-cable="${i}" d="${d}" stroke="${cableVar(c.d)}" stroke-width="4.5"/>
      <path class="cable-sheen" data-cable="${i}" d="${d}"/>
      ${on ? `<path class="cable-flow" data-cable="${i}" d="${d}"/>` : ''}
      <rect class="plug" data-plug="${i}" x="${(on ? L.portX : L.jackX + 180) - 8}" y="${(on ? c.yp : c.floor) - 5}" width="16" height="10" rx="3" fill="${cableVar(c.d)}"/>
      <rect x="${L.jackX - 8}" y="${c.ya - 5}" width="16" height="10" rx="3" fill="${cableVar(c.d)}"/></g>`;
  }).join('');
  return `<section class="bay" aria-label="Which app needs which drive">
    <svg viewBox="0 0 880 ${L.H}" role="img" aria-label="${esc(L.cables.map((c) => `${c.app.app} uses ${c.d.name}${c.d.connected ? '' : ' (missing)'}`).join('; '))}">${appRows}${blocks}${cables}</svg></section>`;
}

function driveModule(s: State, d: Drive) {
  const on = d.connected, st = stowedOn(s, d.id), stowed = sum(st), busy = s.jobs.some((j) => j.drive === d.id);
  return `<article class="module ${on ? '' : 'is-missing'}" style="--c:${cableVar(d)}" data-hot="drive:${d.id}" aria-label="${esc(d.name)}">
    <header><span class="swatch"></span><strong>${esc(d.name)}</strong><span class="kind">${esc(d.format)} · ${esc(d.bus)}</span><span class="spacer"></span>
      ${d.refused ? pill('neutral', 'Can’t hold links') : !st.length ? '' : on ? button(icon('eject'), 'eject', { id: d.id, disabled: busy, title: busy ? `A folder is being copied. Stop it or wait before ejecting ${d.name}.` : `Eject ${d.name}`, label: `Eject ${d.name}` }) : pill('danger', 'Missing')}</header>
    ${capacity(on ? [{ value: d.total - d.free - stowed, color: 'var(--storage-apps)', label: 'Other files' }, { value: stowed, color: cableVar(d), label: `App data ${gb(stowed)}` }] : [], d.total)}
    <div class="stats"><span>${on ? `<span class="num">${gb(d.free)}</span> free of ${tb(d.total)}` : `Last seen ${ago(d.lastSeen)}`}</span><span>${st.length ? `<span class="num">${st.length}</span> folder${st.length === 1 ? '' : 's'} · <span class="num">${gb(stowed)}</span>` : 'Nothing moved here'}</span></div>
    ${on && s.io?.[d.id] ? (() => { const io = s.io![d.id], r = (b: number) => b >= 1e6 ? `${(b / 1e6).toFixed(1)} MB/s` : `${Math.round(b / 1e3)} KB/s`;
      return `<div class="stats"><span>Read <span class="num">${r(io.read)}</span> · write <span class="num">${r(io.write)}</span></span><span>avg ${r(io.avgRead + io.avgWrite)} · ${io.last ? `active ${ago(io.last)}` : 'idle'}</span></div>`; })() : ''}
    ${d.refused && d.problem ? `<p class="secondary" style="font:var(--type-caption)">${esc(d.problem)}</p>` : ''}
  </article>`;
}

export function drives(v: View) {
  const s = v.s;
  const holders = s ? usable(s).filter((d) => d.connected && stowedOn(s, d.id).length) : [];
  const h = header('Drives', 'Which app needs which drive.', holders.length === 1 && !v.down && !s!.jobs.some((j) => j.drive === holders[0].id) ? button(`${icon('eject')} Eject ${esc(holders[0].name)}…`, 'eject', { id: holders[0].id }) : '');
  if (v.down) return h + `<div class="page-body">${helperDown()}</div>`;
  if (!s) return h + `<div class="page-body" aria-busy="true"><div class="modules">${'<div class="module"><span class="skeleton" style="height:14px;width:50%"></span><span class="skeleton" style="height:8px"></span><span class="skeleton" style="height:10px;width:70%"></span></div>'.repeat(3)}</div><div class="bay"><span class="skeleton" style="display:block;height:260px"></span></div></div>`;
  const none = !s.folders.some((i) => i.verdict === 'stowed');
  const one = s.drives.find((d) => d.id === v.ui.drive);
  if (one) {
    const st = stowedOn(s, one.id);
    return header(esc(one.name), `${st.length ? `${st.length} folder${st.length === 1 ? '' : 's'} linked here · ${gb(sum(st))}` : 'No folders linked here yet'}`, button('All drives', 'go', { id: 'drives' })) + `<div class="page-body">
      <div class="modules" style="grid-template-columns:minmax(0,1fr)">${driveModule(s, one)}</div>
      ${st.length ? `<div class="table" role="table" aria-label="Folders on ${esc(one.name)}"><div class="table-head" role="row"><span></span><span>Folder</span><span>Where the link is</span><span style="text-align:right">Size</span><span>State</span><span></span></div>${st.map((it) => itemRow(v as V, it)).join('')}</div>`
        : empty({ illus: ILLUS_PORT, title: 'Nothing is linked to this drive', body: one.refused ? esc(one.problem ?? '') : `Pick it under Move to on Offload.`, actions: one.refused ? '' : button('Open Offload', 'go', { id: 'offload', kind: 'primary' }) })}
    </div>`;
  }
  const shown = s.drives.filter((d) => d.connected || stowedOn(s, d.id).length);
  if (none && !shown.length) return h + `<div class="page-body">${empty({ illus: ILLUS_PORT, title: 'No drives connected', body: 'Plug in an external drive. It shows up here with its free space, and later with the apps that keep data on it.' })}</div>`;
  const low = s.mac.free < 40 * GB;
  const mac = `<article class="module" style="--c:var(--text-secondary)" aria-label="Macintosh HD"><header><span class="swatch"></span><strong>Macintosh HD</strong><span class="kind">Internal</span></header>
    ${capacity([{ value: s.mac.total - s.mac.free, color: low ? 'var(--danger-fill)' : 'var(--storage-system)', label: 'Used' }], s.mac.total)}
    <div class="stats"><span><span class="num" style="color:${low ? 'var(--danger-text)' : ''}">${gb(s.mac.free)}</span> free of ${Math.round(s.mac.total / GB)} GB</span><span>${low ? 'Low' : ''}</span></div></article>`;
  const broken = usable(s).filter((d) => d.connected && d.problem).map((d) => {
    const apps = appsOn(stowedOn(s, d.id));
    return banner({ kind: 'danger', title: `${esc(d.name)} can’t be read`, body: `${esc(d.problem!)} iStash won’t write to it and keeps ${esc(list(apps))} held until it’s repaired.`, actions: button('Open Disk Utility…', 'open', { id: 'disk-utility', kind: 'primary' }) });
  });
  const missing = usable(s).filter((d) => !d.connected && stowedOn(s, d.id).length).map((d) => {
    const st = stowedOn(s, d.id), appList = appsOn(st), apps = esc(list(appList)), many = appList.length > 1;
    return banner({ kind: 'danger', ic: 'plug', title: `${esc(d.name)} is missing, so ${apps} ${many ? 'are' : 'is'} held.`,
      body: `The data is safe on the drive, and the link iStash left stops ${apps} from starting over with an empty folder. Plug ${esc(d.name)} in and ${many ? 'they open' : 'it opens'} normally. To keep the data (${gb(sum(st))}) on this Mac instead, plug it in and choose Bring back.` });
  });
  return h + `<div class="page-body">${broken.join('')}
    <div class="modules">${shown.map((d) => driveModule(s, d)).join('')}${mac}</div>
    ${none ? empty({ illus: ILLUS_PORT, title: 'No app data on drives yet', body: 'Moved folders show up here with the app that needs them.', actions: button('Go to Offload', 'go', { id: 'offload', kind: 'primary' }) }) : cableMap(v as V)}${missing.join('')}
  </div>`;
}

// ---------- Memory ----------
export function memory(v: View) {
  const h = header('Memory', 'Pressure, swap, and which apps use memory and CPU.');
  if (v.down) return h + `<div class="page-body">${helperDown()}</div>`;
  if (!v.s) return h + loadingPage();
  const s = v.s, max = Math.max(...s.memApps.map((a) => a.mem));
  const cols = 'style="grid-template-columns:32px minmax(0,1.3fr) minmax(0,1.4fr) 84px 190px"';
  return h + `<div class="page-body">${forecastCard(v as V, { big: true })}
    <section class="table" role="table" aria-label="Apps using memory"><div class="table-head" role="row" ${cols}><span></span><span>App</span><span>Memory</span><span style="text-align:right">Size</span><span></span></div>
    ${s.memApps.map((a) => `<div class="table-row" role="row" ${cols}>${appIcon(v.icon(a.appPath), a.app)}<div class="name"><strong>${esc(a.app)}${s.focus && s.focus === a.appPath ? ' ' + pill('accent', 'In focus') : ''}</strong><span>${esc(a.detail)}</span></div>
      ${capacity([{ value: a.mem, color: 'var(--accent-fill)', label: gb(a.mem) }], max)}<div class="size">${gb(a.mem)}</div><div class="act">${a.appPath ? (s.focus === a.appPath ? button('Stop focus', 'focus', { id: '', kind: 'primary', label: `Stop focusing on ${a.app}` }) : button('Focus', 'focus', { id: a.appPath, label: `Focus on ${a.app}` })) + button('Quit…', 'quit-app', { id: a.id, label: `Quit ${a.app}` }) : ''}</div></div>`).join('')}</section>
    ${cpuTable(v)}
    <p class="tertiary" style="font:var(--type-caption)">${s.focus ? `Focus is on. Other apps run in the background until you stop it.` : `Focus gives one app priority. macOS can’t cap an app’s memory.`}</p></div>`;
}

// Apps using the most CPU, busiest first. 100% is one full core, as in Activity Monitor.
function cpuTable(v: View) {
  const list = v.s?.cpuApps ?? [];
  if (!list.length) return '';
  const cores = navigator.hardwareConcurrency || 8, max = Math.max(100, ...list.map((a) => a.cpu ?? 0));
  const cols = 'style="grid-template-columns:32px minmax(0,1.3fr) minmax(0,1.4fr) 84px 190px"';
  return `<section class="table" role="table" aria-label="Apps using CPU"><div class="table-head" role="row" ${cols}><span></span><span>App</span><span>CPU</span><span style="text-align:right">Use</span><span></span></div>
    ${list.map((a) => { const c = Math.round(a.cpu ?? 0); return `<div class="table-row" role="row" ${cols}>${appIcon(v.icon(a.appPath), a.app)}<div class="name"><strong>${esc(a.app)}</strong><span>${c >= 100 ? `${(c / 100).toFixed(1)} of ${cores} cores` : `${gb(a.mem)} memory`}</span></div>
      ${capacity([{ value: c, color: c >= 150 ? 'var(--danger-fill)' : 'var(--accent-fill)', label: `${c}%` }], max)}<div class="size">${c}%</div><div class="act">${a.appPath ? button('Quit…', 'quit-app', { id: a.id, label: `Quit ${a.app}` }) : ''}</div></div>`; }).join('')}</section>`;
}

// ---------- Clean ----------
// A cleaner's ring, mobile style: the number in the middle, an arc for how full it is; while working the arc
// sweeps round and particles are pulled into the centre; when done a tick pops in with what was freed.
function orb(o: { pct: number; big: string; unit: string; busy: boolean; done: string; tone: 'mem' | 'disk'; small?: boolean; eta?: number }) {
  // Soft translucent petals drift round a solid centre, each on its own period so the motion never visibly repeats.
  // While working they speed up and a thin arc fills once round the centre; when done a tick pops in.
  const rnd = (i: number, k: number) => { const v = Math.sin(i * 12.9898 + k * 78.233) * 43758.5453; return v - Math.floor(v); };
  const petals = [0, 1, 2, 3, 4].map((i) => `<i style="--rot:${i * 72 + rnd(i, 1) * 30}deg;--dx:${(4 + rnd(i, 2) * 7).toFixed(1)}%;--p:${(11 + i * 4.3).toFixed(1)}s;--sc:${(0.9 + rnd(i, 3) * 0.14).toFixed(2)}"></i>`).join('');
  const r = 36, len = 2 * Math.PI * r;
  const motes = o.busy ? Array.from({ length: 16 }, (_, i) => {
    const dur = 1.2 + rnd(i, 1) * 1.4;
    return `<b style="--a:${((i * 137.5) % 360).toFixed(1)}deg;--r:${(0.8 + rnd(i, 2) * 0.5).toFixed(2)};--s:${(3 + rnd(i, 3) * 4).toFixed(1)}px;--sw:${((rnd(i, 4) - 0.5) * 120).toFixed(0)}deg;--t:${dur.toFixed(2)}s;--d:-${(rnd(i, 5) * dur).toFixed(2)}s"></b>`;
  }).join('') : '';
  return `<div class="bloom ${o.tone} ${o.small ? 'is-small' : ''} ${o.busy ? 'is-busy' : ''} ${o.done ? 'is-done' : ''}" aria-hidden="true">
    <div class="bloom-petals">${petals}</div>
    <div class="bloom-motes">${motes}</div>
    <div class="bloom-core">
      ${o.busy ? `<svg viewBox="0 0 80 80"><circle cx="40" cy="40" r="${r}" class="bloom-track"/><circle cx="40" cy="40" r="${r}" class="bloom-progress" stroke-dasharray="${len.toFixed(1)}" style="--len:${len.toFixed(1)}px;--eta:${o.eta ?? 20}s"/></svg>` : ''}
      ${o.done && !o.busy ? `<span class="bloom-tick">${icon('check')}</span><b>${esc(o.done)}</b>` : `<b class="num">${esc(o.big)}<small>GB</small></b><span>${esc(o.busy ? (o.tone === 'mem' ? 'Freeing memory' : 'Cleaning') : o.unit.replace(/^GB /, ''))}</span>`}
    </div>
  </div>`;
}

export function clean(v: View) {
  const s = v.s;
  const h = header('Clean', 'Free up memory and storage.', s && !v.down ? button('Refresh sizes', 'clean-scan', { disabled: s.clean.busy, label: 'Measure the storage kinds again' }) : '');
  if (v.down) return h + `<div class="page-body">${helperDown()}</div>`;
  if (!s) return h + loadingPage();
  const c = s.clean, st = s.settings, cols = 'style="grid-template-columns:32px minmax(0,1.2fr) minmax(0,2fr) 100px"';
  const picked = (c.items ?? []).filter((i) => st.cleanKinds.includes(i.id)), total = sum(picked), kept = picked.reduce((n, i) => n + (i.kept ?? 0), 0);
  const when = { off: 'Only when I choose', daily: 'Every day', weekly: 'Every week', monthly: 'Every month', over2: 'When it passes 2 GB', over5: 'When it passes 5 GB', over10: 'When it passes 10 GB' };
  const ages: [number, string][] = [[0, 'Any age'], [3, '3 days'], [7, '7 days'], [30, '30 days']];
  return h + `<div class="page-body">
    <section class="card boost-card">
      ${orb({ pct: v.m ? Math.round((v.m.memUsed / v.m.memTotal) * 100) : 0, big: v.m ? gb(v.m.memUsed).replace(' GB', '') : '–', unit: 'GB in use', busy: c.memBusy, done: c.memFreed != null && !c.memBusy && Date.now() - (c.memAt ?? 0) < 8000 ? (c.memFreed > 0 ? `${gb(c.memFreed)} freed` : 'Already lean') : '', tone: 'mem', eta: 25 })}
      <div class="boost-side"><h2>Memory</h2>
        <p class="secondary">${v.m ? `${Math.round(v.m.memTotal / 2 ** 30)} GB total${v.m.swap ? ` · swap ${gb(v.m.swap)}` : ''}` : ''}</p>
        ${button(c.memBusy ? 'Freeing…' : 'Free up memory', 'free-memory', { kind: 'primary', large: true, disabled: c.memBusy })}</div>
    </section>
    <h2 style="margin-top:4px">Storage</h2>
    <section class="card" style="display:grid;grid-template-columns:auto 1fr auto auto auto;gap:8px 24px;align-items:center">
      <div class="boost-inline">${orb({ pct: Math.min(100, (total / (5 * GB)) * 100), big: c.items ? gb(total).replace(' GB', '') : '…', unit: 'GB to clean', busy: c.busy, done: c.freed != null && c.last && !c.busy && Date.now() - c.last < 8000 ? `${gb(c.freed)} freed` : '', tone: 'disk', small: true, eta: 15 })}</div>
      <span class="secondary" style="font:var(--type-callout)">${c.items && kept > 0.05 * GB && !c.busy ? `${gb(kept)} is newer than ${st.cleanAge} days, so it's kept.` : ''}</span>
      <div class="move-to"><label class="secondary" for="clean-every" style="font:var(--type-group)">Clean automatically</label>
        <select class="select" id="clean-every" data-action="clean-every">${Object.entries(when).map(([k, label]) => `<option value="${k}" ${st.cleanEvery === k ? 'selected' : ''}>${label}</option>`).join('')}</select></div>
      <div class="move-to"><label class="secondary" for="clean-age" style="font:var(--type-group)">Only files untouched for</label>
        <select class="select" id="clean-age" data-action="clean-age">${ages.map(([d, label]) => `<option value="${d}" ${st.cleanAge === d ? 'selected' : ''}>${label}</option>`).join('')}</select></div>
      <div>${button(c.busy ? 'Cleaning…' : 'Clean now…', 'clean-ask', { kind: 'primary', large: true, disabled: c.busy || !total })}</div>
    </section>
    ${c.freed != null && c.last ? banner({ kind: 'info', ic: 'check', title: `Freed ${gb(c.freed)}`, body: `Last cleaned ${ago(c.last)}.` }) : ''}
    <div class="table" role="table" aria-label="What can be cleaned"><div class="table-head" role="row" ${cols}><span></span><span>Kind</span><span>What it is</span><span style="text-align:right">Size</span></div>
      ${c.items ? c.items.map((i, k) => `<label class="table-row ${c.busy && st.cleanKinds.includes(i.id) && i.size ? 'is-cleaning' : ''}" role="row" ${cols} style="--k:${k}"><input type="checkbox" data-action="clean-kind" data-id="${i.id}" ${st.cleanKinds.includes(i.id) ? 'checked' : ''} aria-label="${esc(i.name)}" style="accent-color:var(--accent-fill)">
        <div class="name"><strong>${esc(i.name)}</strong><span>${i.count} item${i.count === 1 ? '' : 's'}</span></div><div class="secondary" style="font:var(--type-callout)">${esc(i.note)}</div><div class="size">${gb(i.size)}</div></label>`).join('') : skeletonRows(4)}</div>
    <p class="tertiary" style="font:var(--type-caption)">Moved folders, macOS caches and your documents are never cleaned.</p>
  </div>`;
}

// ---------- Settings ----------
export function settings(v: View) {
  const h = header('Settings', '');
  if (v.down) return h + `<div class="page-body">${helperDown()}</div>`;
  if (!v.s) return h + `<div class="page-body" aria-busy="true"><span class="skeleton" style="height:200px;max-width:680px"></span></div>`;
  const s = v.s, st = s.settings;
  const sw = (key: 'login' | 'updates', label: string) => s.unsupported?.includes(key) ? '<span class="secondary">Not in this build</span>' : toggle(key, st[key], label);
  const row = (title: string, note: string, control: string) => `<div class="row"><div>${title}${note ? `<small>${note}</small>` : ''}</div>${control}</div>`;
  return h + `<div class="page-body"><div class="form">
    <section><h2>General</h2><div class="group">
      ${row('Appearance', 'System follows the light or dark setting of this Mac.', seg('theme', [['light', 'Light'], ['dark', 'Dark'], ['system', 'System']], st.theme))}
      ${row('Open at login', s.loginNeedsApproval ? 'Allow it in System Settings > Login Items.' : '', sw('login', 'Open at login'))}
      ${row('Show in menu bar', '', toggle('menubar', st.menubar, 'Show in menu bar'))}
      ${row('Edge widget', 'CPU, memory, network and drives on the edge of the screen.', seg('widget-mode', [['off', 'Off'], ['auto', 'Show on hover'], ['pinned', 'Always']], st.widget))}
      ${st.widget === 'off' ? '' : row('Widget edge', '', seg('widget-side', [['left', 'Left'], ['right', 'Right']], st.widgetSide))}
      ${row('Notch island', 'iStash notifications drop from the top of the screen.', toggle('island', st.island, 'Notch island'))}
      ${st.island ? row('Island glow', '', seg('island-glow', [['amber', 'Amber'], ['blue', 'Blue'], ['pink', 'Pink'], ['green', 'Green'], ['white', 'White'], ['rgb', 'RGB']], st.islandGlow)) : ''}
      ${st.island ? row('Island stays for', '', `${seg('island-secs', [['3', '3 s'], ['5', '5 s'], ['8', '8 s']], String(st.islandSecs))} ${button('Show it', 'island-test')}`) : ''}
      ${row('Check for updates automatically', '', sw('updates', 'Check for updates automatically'))}
      ${row(`Version ${esc(s.version)}`, s.update.available ? `Version ${esc(s.update.available)} is available. iStash restarts to finish.` : s.update.checked ? `This is the latest version. Checked ${ago(s.update.checked)}.` : '',
        s.update.available ? button(s.update.busy ? 'Updating…' : `Update to ${esc(s.update.available)}`, 'install-update', { kind: 'primary', disabled: s.update.busy }) : button(s.update.busy ? 'Checking…' : 'Check now', 'check-update', { disabled: s.update.busy }))}
    </div></section>
    <section><h2>Permissions</h2>${permRows(s)}</section>
    <section><h2>Moving</h2><div class="group">
      ${row('Show what will happen before every move', 'Lists each step and the space it needs. Recommended.', toggle('dryRun', st.dryRun, 'Show what will happen before every move'))}
      ${row('Keep the original until every file is checked', '', '<span class="secondary">Always on</span>')}
      ${row('Never touched', 'Documents, Desktop, iCloud Drive, app sandboxes, Photos and Mail data.', '')}
    </div></section>
    <section><h2>Memory</h2><div class="group">
      ${row('<label for="warnAt">Warn me when memory may run out within</label>', '', `<select id="warnAt" data-action="setting-select" data-id="warnAt">${[15, 30, 60].map((m) => `<option value="${m}" ${st.warnAt === m ? 'selected' : ''}>${m} minutes</option>`).join('')}</select>`)}
      ${row('Notifications', s.permissions.notify ? 'On. Used only for memory warnings.' : 'Used only for memory warnings.', s.permissions.notify ? pill('ok', 'On') : button('Allow…', 'grant', { id: 'notify' }))}
    </div></section>
    <section><h2>Drives</h2><div class="group">
      ${row('Warn before I eject a drive that apps still need', 'Names the apps and offers to quit them first.', toggle('ejectGuard', st.ejectGuard, 'Warn before I eject a drive that apps still need'))}
      ${row('Journal', `<span class="mono">${esc(s.journalPath)}</span>`, button('Show in Finder', 'open', { id: 'journal' }))}
    </div></section>
    <section><h2>Privacy</h2><div class="group">${row('No account, no analytics', '', '')}</div></section>
  </div></div>`;
}

// ---------- Onboarding ----------
// Every permission iStash uses, with an Allow button each and Allow all; shared by onboarding and Settings.
export function permRows(s: State) {
  const p = s.permissions;
  const perm = (key: keyof State['permissions'], title: string, why: string) => `<div class="perm"><strong>${title}</strong>${p[key] ? pill('ok', 'Allowed') : button('Allow…', 'grant', { id: key, label: `Allow ${title.toLowerCase()}` })}<p>${why}</p></div>`;
  const all = (['volumes', 'notify', 'camera', 'finder', 'fans'] as const).every((k) => p[k]);
  return perm('volumes', 'Removable volumes', 'Needed to copy app data to your external drive and check it.')
    + perm('notify', 'Notifications', 'Memory warnings, drive warnings and updates, on the notch island or as banners.')
    + perm('camera', 'Camera', 'Only for the Camera button in the menu bar.')
    + perm('finder', 'Finder', 'Lets Empty Trash ask Finder to empty it.')
    + perm('fans', 'Fan control', 'Asks for your password once to install a helper that only sets fan speed.')
    + (all ? '' : `<div style="margin-top:12px">${button('Allow all', 'grant-all', { kind: 'primary' })}</div>`);
}

export function onboarding(v: View) {
  const { s, ui } = v;
  const names = ['Welcome', 'Permissions', 'Choose a drive'];
  const aside = `<aside ${drag}><div class="traffic" ${drag}></div>
    <div style="display:flex;gap:10px;align-items:center;margin-top:20px"><img class="brand-mark" src="/logo.png" alt=""><strong style="font:var(--type-headline)">iStash</strong></div>
    <ol aria-label="Setup steps">${names.map((t, i) => `<li class="${i === ui.onboardStep ? 'is-current' : i < ui.onboardStep ? 'is-done' : ''}" ${i === ui.onboardStep ? 'aria-current="step"' : ''}><span class="n" aria-hidden="true">${i < ui.onboardStep ? '✓' : i + 1}</span>${t}</li>`).join('')}</ol>
    <p class="tertiary" style="margin-top:auto;font:var(--type-caption)">Nothing is moved until you choose.</p></aside>`;
  let main;
  if (v.down) main = helperDown();
  else if (!s) main = '<span class="skeleton" style="height:200px"></span>';
  else if (ui.onboardStep === 0) {
    const promise = (t: string, d: string) => `${icon('check')}<div><strong style="font:var(--type-emphasis)">${t}</strong><p>${d}</p></div>`;
    main = `<h1>Make room on your Mac.<br>Keep every app working.</h1>
      <p class="lead">iStash moves heavy app data, like AI models, build caches and virtual machines, to an external drive and leaves a link behind, so the apps never notice. It also tells you which app needs which drive and warns before memory runs out.</p>
      <div class="promise">${promise('Nothing is removed until the copy is checked', 'File count, size and content are compared first.')}${promise('Your documents are off limits', 'Documents, Desktop, iCloud Drive and app sandboxes are never touched.')}${promise('Every move can be undone', 'Bring anything back with one click, even after a crash.')}${promise('No account, no tracking, free', 'It only goes online to check for updates.')}</div>
      <div class="footer">${button('Continue', 'onboard-next', { kind: 'primary', large: true })}</div>`;
  } else if (ui.onboardStep === 1) {
    main = `<h1>Permissions</h1><p class="lead">Give them now, so iStash doesn’t stop to ask later. macOS asks once for each, in iStash’s name; fan control asks for your password once.</p>
      ${permRows(s)}
      <p class="secondary" style="font:var(--type-callout);max-width:600px">iStash doesn’t ask for Full Disk Access and never reads other apps’ sandboxes, so you won’t see a stream of “would like to access” prompts.</p>
      <div class="footer">${s.permissions.volumes ? '' : '<span class="tertiary" style="font:var(--type-caption);align-self:center">Allow removable volumes first</span>'}${button('Back', 'onboard-back', { large: true })}${button('Continue', 'onboard-next', { kind: 'primary', large: true, disabled: !s.permissions.volumes, title: s.permissions.volumes ? '' : 'Allow removable volumes first' })}</div>`;
  } else {
    const choice = (d: Drive) => {
      const off = !d.connected, bad = d.refused || !!d.problem, sel = ui.onboardDrive === d.id;
      const detail = bad ? d.problem! : off ? `${d.format} · last seen ${ago(d.lastSeen)}` : `${d.format} · ${gb(d.free)} free · ${d.bus}`;
      return `<label class="choice ${sel ? 'is-selected' : ''} ${bad ? 'is-refused' : ''}">
      <input type="radio" name="drive" data-action="pick-drive" data-id="${d.id}" ${sel ? 'checked' : ''} ${bad || off ? 'disabled' : ''}>
      <div><strong>${esc(d.name)}</strong><div class="${bad ? '' : 'secondary'}" style="font:var(--type-callout);${bad ? 'color:var(--danger-text)' : ''}">${esc(detail)}</div></div>${bad ? pill('danger', 'Can’t use') : off ? pill('neutral', 'Not connected') : pill('ok', 'Ready')}</label>`;
    };
    main = `<h1>Pick a drive for app data</h1><p class="lead">Use a drive that stays plugged in while you work. You can add more drives later.</p>
      ${s.drives.length ? `<div role="radiogroup" aria-label="Drives" style="display:grid;gap:8px">${s.drives.map(choice).join('')}</div>` : empty({ illus: ILLUS_PORT, title: 'No drive connected', body: 'Plug in an external drive formatted as APFS or Mac OS Extended. You can also skip this and choose later.' })}
      <div class="footer">${button('Back', 'onboard-back', { large: true })}${button('Start using iStash', 'onboard-done', { kind: 'primary', large: true })}</div>`;
  }
  return `<div class="onboard">${aside}<main>${main}</main></div>`;
}

// ---------- Menu-bar popover ----------
// ---------- Edge widget ----------
// A thin, tall glass column: each block is a label, a big number and a live trace or bar.
const rate = (b: number) => b >= 1e6 ? `${(b / 1e6).toFixed(1)} MB/s` : `${Math.round(b / 1e3)} KB/s`;
export function widget(v: View) {
  const { s, m } = v;
  if (!s || !m) return '<div class="edge" aria-busy="true"><span class="skeleton" style="height:320px"></span></div>';
  const cpu = m.cpu.at(-1) ?? 0, mem = m.pressure, disk = Math.round(((s.mac.total - s.mac.free) / s.mac.total) * 100);
  const spark = (vals: number[], max: number, color: string) => {
    const pts = vals.slice(-24), n = Math.max(1, pts.length - 1);
    const d = pts.map((y, i) => `${((i / n) * 100).toFixed(1)},${(26 - (Math.min(y, max) / max) * 24).toFixed(1)}`).join(' ');
    return `<svg class="edge-spark" viewBox="0 0 100 28" preserveAspectRatio="none" aria-hidden="true"><polyline points="0,28 ${d} 100,28" fill="${color}" fill-opacity="0.18" stroke="none"/><polyline points="${d}" fill="none" stroke="${color}" stroke-width="1.6" vector-effect="non-scaling-stroke"/></svg>`;
  };
  const bar = (pct: number, color: string) => `<div class="edge-bar"><i style="width:${Math.min(100, Math.max(3, pct))}%;background:${color}"></i></div>`;
  const tone = (pct: number, warn: number, bad: number) => pct >= bad ? '#FF6B6B' : pct >= warn ? '#FFB547' : '#C8FF3D';
  const netMax = Math.max(...m.netIn, 250e3);
  const drives = usable(s).map((d) => {
    const io = s.io?.[d.id], used = d.connected ? Math.round(((d.total - d.free) / d.total) * 100) : 0;
    return `<div class="edge-block edge-drive ${d.connected ? '' : 'is-missing'}" title="${esc(d.name)}">
      <span class="edge-label">${icon('drives')}${esc(d.name)}</span>
      <span class="edge-num">${d.connected ? gb(d.free) : 'Missing'}</span>
      ${d.connected ? bar(used, cableVar(d)) + (io && io.read + io.write > 4096 ? `<span class="edge-sub">↓${rate(io.read)} ↑${rate(io.write)}</span>` : '<span class="edge-sub">idle</span>') : '<span class="edge-sub">Connect it</span>'}
    </div>`;
  }).join('');
  const sn = s.sensors, fan = sn?.fans[0];
  const fanBlock = fan || sn?.cpu ? `<div class="edge-block"><span class="edge-label">${icon('fan')}${fan ? 'Fan' : 'Temp'}</span><span class="edge-num edge-small">${fan ? `${Math.round(fan.rpm).toLocaleString('en-GB')}<small> rpm</small>` : `${Math.round(sn!.cpu!.avg)}°`}</span>`
    + (fan?.max ? bar(((fan.rpm - (fan.min ?? 0)) / (fan.max - (fan.min ?? 0))) * 100, '#5AC8FA') : '')
    + (sn?.cpu ? `<span class="edge-sub">CPU ${Math.round(sn.cpu.avg)}° · peak ${Math.round(sn.cpu.max)}°</span>` : '') + '</div>' : '';
  return `<div class="edge" data-action="open-main" title="Open iStash">
    <div class="edge-head"><i class="edge-live"></i><strong>iStash</strong></div>
    <div class="edge-block"><span class="edge-label">${icon('pulse')}CPU</span><span class="edge-num">${cpu}<small>%</small></span>${spark(m.cpu, 100, tone(cpu, 70, 90))}</div>
    <div class="edge-block"><span class="edge-label">${icon('memory')}Memory</span><span class="edge-num">${mem}<small>%</small></span>${bar(mem, tone(mem, 60, 80))}${m.swap ? `<span class="edge-sub">swap ${gb(m.swap)}</span>` : ''}</div>
    <div class="edge-block"><span class="edge-label">${icon('wifi')}Network</span><span class="edge-num edge-small">↓ ${rate(m.netIn.at(-1) ?? 0)}</span><span class="edge-sub">↑ ${rate(m.netOut)}</span>${spark(m.netIn, netMax, '#5AC8FA')}</div>
    <div class="edge-block"><span class="edge-label">${icon('drives')}Disk</span><span class="edge-num edge-small">${gb(s.mac.free)}</span><span class="edge-sub">free</span>${bar(disk, tone(disk, 80, 90))}</div>
    ${fanBlock}
    ${drives ? `<div class="edge-rule"></div>${drives}` : ''}
  </div>`;
}

const uptime = (sec: number) => {
  const d = Math.floor(sec / 86400), h = Math.floor((sec % 86400) / 3600), mi = Math.floor((sec % 3600) / 60);
  return d ? `Up ${d} d ${h} h` : h ? `Up ${h} h ${mi} min` : `Up ${mi} min`;
};
const popSpark = (vals: number[], max: number, color: string) => {
  const pts = vals.slice(-30), n = Math.max(1, pts.length - 1);
  const d = pts.map((y, i) => `${((i / n) * 100).toFixed(1)},${(30 - (Math.min(y, max) / max) * 28).toFixed(1)}`).join(' ');
  return `<svg class="pop-spark" viewBox="0 0 100 32" preserveAspectRatio="none" aria-hidden="true"><polyline points="0,32 ${d} 100,32" fill="${color}" fill-opacity="0.16" stroke="none"/><polyline points="${d}" fill="none" stroke="${color}" stroke-width="1.5" vector-effect="non-scaling-stroke"/></svg>`;
};
const popBar = (pct: number, color: string, label: string) => `<div class="pop-bar" role="img" aria-label="${esc(label)}"><i style="width:${Math.min(100, Math.max(2, pct))}%;background:${color}"></i></div>`;
const tone = (pct: number, warn: number, bad: number) => pct >= bad ? 'var(--pop-bad)' : pct >= warn ? 'var(--pop-warn)' : 'var(--pop-ok)';

/** The menu-bar panel: live tiles, quick toggles, and a way into the app. Always dark glass, whatever sits behind it. */
export function popover(v: View) {
  const { s, m, ui } = v;
  if (v.down) return `<div class="popover pop">${helperDown()}</div>`;
  if (!s) return `<div class="popover pop" aria-busy="true"><span class="skeleton" style="height:240px"></span></div>`;
  const sys = s.sys, cpu = m?.cpu.at(-1) ?? 0, pressure = m?.pressure ?? 0;
  const tile = (label: string, value: string, body: string) => `<section class="pop-tile"><header><span>${label}</span><strong>${value}</strong></header>${body}</section>`;
  const line = (...xs: string[]) => `<p class="pop-line">${xs.join('')}</p>`;
  const kv = (k: string, val: string, dot = '') => `<span>${dot ? `<i style="background:${dot}"></i>` : ''}${k} <b>${val}</b></span>`;

  const cpuTile = tile('CPU', `${cpu}%`, popSpark(m?.cpu ?? [], 100, tone(cpu, 70, 90)) +
    line(kv('User', `${sys?.user ?? 0}%`), kv('Sys', `${sys?.system ?? 0}%`), kv('Idle', `${sys?.idle ?? 0}%`)) +
    (s.cpuApps ?? []).slice(0, 2).map((a) => `<div class="pop-line pop-top"><span class="pop-dim">${esc(a.app)}</span><b>${Math.round(a.cpu ?? 0)}%</b></div>`).join(''));
  const freed = s.clean.memFreed;
  const memTile = tile('Memory', m ? gb(m.memUsed) : '–', line(kv('of', m ? gb(m.memTotal) : '–'), m?.swap ? kv('swap', gb(m.swap)) : '') +
    popBar(pressure, tone(pressure, 60, 80), `Memory pressure ${pressure}%`) +
    `<button class="pop-btn" data-action="free-memory" ${s.clean.memBusy ? 'disabled' : ''}>${s.clean.memBusy ? 'Freeing…' : freed ? `Freed ${gb(freed)}` : 'Free up memory'}</button>`);
  const used = s.mac.total - s.mac.free, macIo = s.io?.mac, usedPct = Math.round((used / s.mac.total) * 100);
  const diskTile = tile('Macintosh HD', gb(used), line(kv('of', gb(s.mac.total)), kv('free', gb(s.mac.free))) +
    popBar(usedPct, tone(usedPct, 80, 92), `${usedPct}% used`) +
    line(kv('R', macIo ? rate(macIo.read) : '–'), kv('W', macIo ? rate(macIo.write) : '–')));
  const t = s.net?.test, netIn = m?.netIn ?? [];
  const netTile = tile('Internet', `↓ ${rate(netIn.at(-1) ?? 0)}`, line(kv('↑', rate(m?.netOut ?? 0))) + popSpark(netIn, Math.max(...netIn, 250e3), 'var(--pop-blue)') +
    line(kv('IP', esc(s.net?.info?.ip || 'offline'))) +
    line(t ? kv('Test', `${Math.round(t.down ?? 0)} Mbps`) + `<span class="pop-dim">${ago(t.at)}</span>` : '<span class="pop-dim">No speed test yet</span>'));

  // Fan and temperatures. macOS sets the fan speed itself; iStash only reads it.
  const sn = s.sensors, fan = sn?.fans[0], hot = sn?.cpu?.max ?? 0;
  // Fan modes: System (macOS decides), Max, Manual with a speed slider. Manual shows its slider before it's applied.
  const fanControls = (f: typeof fan & {}) => {
    const lo = f.min ?? 0, hi = f.max ?? 6000;
    const mode = ui.fanManual ? 'manual' : !f.manual ? 'auto' : f.target != null && f.target >= hi - 50 ? 'max' : 'manual';
    const rpm = ui.fanRpm ?? f.target ?? f.rpm;
    const b = (id: string, label: string) => `<button data-action="fan-mode" data-id="${id}" aria-pressed="${mode === id}" ${ui.fanBusy ? 'disabled' : ''}>${label}</button>`;
    return `<div class="pop-seg" role="group" aria-label="Fan mode">${b('auto', 'System')}${b('max', 'Max')}${b('manual', 'Manual')}</div>` +
      (mode === 'manual' ? `<div class="pop-fan"><input type="range" data-action="fan-rpm" min="${lo}" max="${hi}" step="100" value="${Math.round(rpm)}" aria-label="Fan speed" ${ui.fanBusy ? 'disabled' : ''}><span class="pop-fan-rpm">${Math.round(rpm).toLocaleString('en-GB')}</span></div>` : '') +
      line(`<span class="pop-dim">${ui.fanBusy ? 'Setting the fan…' : mode === 'auto' ? `${f.min && f.rpm <= f.min + 50 ? 'Idle speed' : 'Cooling'} · set by macOS` : 'Until you pick System or restart'}</span>`);
  };
  const fanTile = sn && (fan || sn.cpu) ? tile(fan ? 'Fan' : 'Temperature', fan ? `${Math.round(fan.rpm).toLocaleString('en-GB')} rpm` : `${Math.round(sn.cpu!.avg)}°C`,
    (fan && fan.max ? popBar(Math.round(((fan.rpm - (fan.min ?? 0)) / (fan.max - (fan.min ?? 0))) * 100), 'var(--pop-blue)', `Fan at ${Math.round(fan.rpm)} rpm`) : '') +
    line(sn.cpu ? kv('CPU', `${Math.round(sn.cpu.avg)}°`, tone(hot, 80, 95)) + kv('peak', `${Math.round(sn.cpu.max)}°`) : '', sn.gpu ? kv('GPU', `${Math.round(sn.gpu.avg)}°`) : '') +
    (fan ? fanControls(fan) : line('<span class="pop-dim">No fan in this Mac</span>')) +
    (sn.sensors.length ? `<button class="pop-more" data-action="pop-temps" aria-expanded="${!!ui.temps}">Temperatures <span>${ui.temps ? '−' : '+'}</span></button>` +
      (ui.temps ? `<div class="pop-temps">${[...sn.sensors].sort((a, b) => b.c - a.c).map((t) => `<div><span>${esc(t.name || 'Sensor')}</span><b>${t.c.toFixed(1)}°</b></div>`).join('')}</div>` : '') : '')) : '';
  const appsRow = `<div class="pop-apps">${([['system-settings', '/System/Applications/System Settings.app', 'System Settings'], ['disk-utility', '/System/Applications/Utilities/Disk Utility.app', 'Disk Utility'],
    ['activity-monitor', '/System/Applications/Utilities/Activity Monitor.app', 'Activity Monitor'], ['system-information', '/System/Applications/Utilities/System Information.app', 'System Information']] as const)
    .map(([id, path, name]) => `<button class="pop-app" data-action="open-app" data-id="${id}" title="Open ${name}">${appIcon(v.icon(path), name)}<span>${name.replace('System ', '')}</span></button>`).join('')}<button class="pop-app" data-action="pop-camera" title="See the camera's picture"><span class="pop-cam">${icon('camera')}</span><span>Camera</span></button></div>`;
  const drives = usable(s).map((d) => `<div class="pop-drive ${d.connected ? '' : 'is-missing'}"><i style="background:${d.connected ? cableVar(d) : 'transparent'}"></i><span class="pop-name">${esc(d.name)}</span>
    <span class="pop-dim">${d.connected ? `${gb(d.free)} free` : 'Missing'}</span></div>`).join('');

  const dark = s.settings.theme === 'dark' || (s.settings.theme === 'system' && globalThis.matchMedia?.('(prefers-color-scheme: dark)').matches);
  const q = (action: string, ic: string, label: string, title: string, on: boolean | null, disabled = false) =>
    `<button class="pop-q" data-action="${action}" title="${esc(title)}" ${on === null ? '' : `aria-pressed="${on}"`} ${disabled ? 'disabled' : ''}>${icon(ic)}<span>${label}</span></button>`;
  const d = ui.dialog, err = d?.type === 'move' && d.plan && !d.plan.ok ? d.plan : null;
  const confirm = (text: string, label: string, action: string) => `<div class="pop-confirm" role="alertdialog" aria-label="${esc(text)}"><span>${esc(text)}</span>${button('Cancel', 'close')}${button(label, action, { kind: 'primary' })}</div>`;
  const quick = err ? `<div class="pop-confirm" role="alert"><span title="${esc(err.reason)}">${esc(err.reason)}</span>${button('OK', 'close', { kind: 'primary' })}</div>`
    : d?.type === 'trash' ? confirm('Empty the Trash for good?', 'Empty', 'confirm-trash')
    : d?.type === 'desktop' ? confirm(`Finder restarts to ${sys?.desktopIcons === false ? 'show' : 'hide'} icons.`, sys?.desktopIcons === false ? 'Show' : 'Hide', 'confirm-desktop')
    : `<div class="pop-quick">${[
      q('pop-dark', 'moon', 'Dark', 'Dark mode for iStash', dark),
      q('pop-awake', 'cup', 'Awake', 'Keep this Mac awake', !!sys?.awake, !sys),
      q('pop-screen', 'desktop', 'Screen on', 'Keep the screen on', !!sys?.screenOn, !sys),
      q('pop-lock', 'lock', 'Lock', 'Lock screen', null),
      q('pop-trash', 'trash', 'Trash', 'Empty Trash', null),
      q('pop-desktop', 'desktop', 'Icons', sys?.desktopIcons === false ? 'Show desktop icons' : 'Hide desktop icons', sys?.desktopIcons === false, !sys),
      q('pop-widget', 'sidebar', 'Widget', 'Edge widget', s.settings.widget !== 'off'),
      q('focus', 'target', 'Focus', s.focus ? 'Turn Focus off' : 'Focus is off', !!s.focus, !s.focus),
    ].join('')}</div>`;

  return `<div class="popover pop" role="dialog" aria-label="iStash">
    <div class="pop-grid">${cpuTile}${memTile}${diskTile}${netTile}${fanTile}</div>
    ${appsRow}
    ${drives ? `<div class="pop-drives">${drives}</div>` : ''}
    ${quick}
    <footer class="pop-foot"><span class="pop-dim">${sys ? uptime(sys.uptime) : ''}</span>${button('Open iStash', 'open-main', { kind: 'primary' })}</footer>
  </div>`;
}

// ---------- Dialogs (sheets). Move and bring back show the engine's dry run. ----------
export function dialogFor(v: View) {
  const d = v.ui.dialog, s = v.s;
  if (!d || !s) return '';
  const close = (label = 'Cancel') => button(label, 'close');
  const okOnly = (title: string, body: string) => dialog({ title, body: `<p>${body}</p>`, buttons: button('OK', 'close', { kind: 'primary' }) });
  const it = s.folders.find((x) => x.id === d.id)!;
  const p = d.plan;
  switch (d.type) {
    case 'move': {
      if (!p) return '';
      if (!p.ok) return okOnly(esc(p.title), esc(p.reason));
      const targets = usable(s).filter((x) => x.connected && !x.problem);
      return dialog({ title: `Move ${esc(it.app)} ${esc(it.name.toLowerCase())} to ${esc(p.drive.name)}?`,
        buttons: close() + button(p.quits ? `Quit ${esc(p.quits)} and move` : 'Move', 'confirm-move', { kind: 'primary', id: it.id }),
        body: `<div class="field"><label class="secondary" for="move-drive">Drive</label><select id="move-drive" data-action="move-drive">${targets.map((x) => `<option value="${x.id}" ${x.id === p.drive.id ? 'selected' : ''}>${esc(x.name)} (${gb(x.free)} free)</option>`).join('')}</select></div>
          <ol>${p.quits ? `<li>Ask ${esc(p.quits)} to quit, so nothing is writing to the folder.</li>` : ''}<li>Copy ${gb(p.size)} to ${esc(p.drive.name)}.</li><li>Check every file: count, size and content.</li><li>Put a link where the folder was. ${esc(it.app)} follows it without noticing.</li><li>Remove the original only after the check passes.</li></ol>
          <p>This Mac will have <strong class="num">${gb(p.macFreeAfter)}</strong> free. You can bring it back at any time.</p>` });
    }
    case 'back': {
      if (!p) return '';
      if (!p.ok) return okOnly(esc(p.title), esc(p.reason));
      return dialog({ title: `Bring ${esc(it.app)} ${esc(it.name.toLowerCase())} back to this Mac?`,
        buttons: close() + button(p.quits ? `Quit ${esc(p.quits)} and bring back` : 'Bring back', 'confirm-back', { kind: 'primary', id: it.id }),
        body: `<ol>${p.quits ? `<li>Ask ${esc(p.quits)} to quit.</li>` : ''}<li>Copy ${gb(p.size)} from ${esc(p.drive.name)} to this Mac.</li><li>Check every file.</li><li>Replace the link with the folder.</li><li>Remove the copy on ${esc(p.drive.name)} last.</li></ol><p>This Mac will have <strong class="num">${gb(p.macFreeAfter)}</strong> free.</p>` });
    }
    case 'check': {
      const c = d.check;
      if (!c) return '';
      const rows = ([['Missing', c.missing], ['Damaged', c.damaged], ['Changed since the move', c.changed], ['New since the move', c.extra]] as const).filter(([, l]) => l.length);
      if (!rows.length) return okOnly(`${esc(it.app)} ${esc(it.name.toLowerCase())} is intact`, `All <span class="num">${c.checked}</span> files match what was moved.`);
      return dialog({ title: `${esc(it.app)} ${esc(it.name.toLowerCase())} differs from what was moved`, buttons: button('OK', 'close', { kind: 'primary' }),
        body: `<ul>${rows.map(([k, l]) => `<li><strong>${k}</strong> (<span class="num">${l.length}</span>): <span class="path">${esc(l.slice(0, 3).join(', '))}${l.length > 3 ? ', …' : ''}</span></li>`).join('')}</ul><p>${c.missing.length || c.damaged.length ? 'Missing or damaged files point to a drive problem. Bring the folder back while the rest is readable.' : `Changes are normal: ${esc(it.app)} has written to its data since the move.`}</p>` });
    }
    case 'how': {
      const link = it.verdict === 'move', to = usable(s).filter((x) => x.connected && !x.problem), dr = to.find((x) => x.id === s.settings.defaultDrive) ?? to[0];
      const steps = (it.how || []).map((x) => `<li>${esc(x.replaceAll('<drive>', dr?.name ?? 'your drive'))}</li>`).join('');
      return dialog({ title: `Move ${esc(it.app)}’s ${esc(it.name.toLowerCase())} with its own setting`,
        buttons: link ? button(s.settings.dryRun ? 'Move with a link instead…' : 'Move with a link instead', 'move', { id: it.id }) + button('Done', 'close', { kind: 'primary' }) : button('Done', 'close', { kind: 'primary' }),
        body: link ? `<p>${esc(it.app)} can keep this on a drive itself. Its own setting survives app updates and needs no link. iStash changes nothing here.</p><ol>${steps}</ol><p>Or let iStash move it with a link; you can bring it back at any time.</p>`
          : `<p>${esc(it.app)} keeps a live database here. A link can corrupt it if the drive is unplugged while it runs, so let ${esc(it.app)} move it.</p><ol>${steps}</ol>` });
    }
    case 'why':
      return okOnly(`Why ${esc(it.name.toLowerCase())} can’t move`, esc(it.why || it.reason));
    case 'apfs':
      return okOnly('Which drives work', 'APFS or Mac OS Extended drives work. exFAT and FAT drives (common on USB sticks) can’t hold the links apps need, so iStash won’t use them. Disk Utility can reformat a drive as APFS; that erases it.');
    case 'eject': {
      const dr = drive(s, d.id), on = stowedOn(s, dr.id), open = on.filter((i) => i.running), apps = appsOn(on);
      return dialog({ title: `${apps.length} app${apps.length === 1 ? '' : 's'} keep data on ${esc(dr.name)}`,
        buttons: close() + button(open.length ? `Quit ${esc(list(appsOn(open)))} and eject` : 'Eject', 'confirm-eject', { kind: 'primary', id: dr.id }),
        body: `<ul>${on.map((i) => `<li><strong>${esc(i.app)}</strong>: ${esc(i.name)}, ${gb(i.size)}${i.running ? ', open now' : ''}</li>`).join('')}</ul><p>After ejecting, these apps are held until ${esc(dr.name)} is back, so nothing writes to an empty folder.</p>` });
    }
    case 'clean': {
      const picked = (s.clean.items ?? []).filter((i) => s.settings.cleanKinds.includes(i.id) && i.size);
      return dialog({ title: `Remove ${gb(sum(picked))} of temporary files?`, buttons: close() + button('Remove', 'confirm-clean', { kind: 'primary' }),
        body: `<ul>${picked.map((i) => `<li>${esc(i.name)}: ${gb(i.size)}</li>`).join('')}</ul><p>They are deleted for good, not moved to the Trash. Apps rebuild what they need, so the first launch afterwards can be slower.</p>` });
    }
    case 'quit-app': {
      const a = [...s.memApps, ...(s.cpuApps ?? [])].find((x) => x.id === d.id);
      if (!a) return '';
      return dialog({ title: `Quit ${esc(a.app)}?`, buttons: close() + button('Quit', 'confirm-quit', { kind: 'primary', id: a.id }),
        body: `<p>iStash asks ${esc(a.app)} to quit normally, so it can save its work. ${esc(a.detail)}. This frees about ${gb(a.mem)}.</p>` });
    }
  }
  return '';
}

