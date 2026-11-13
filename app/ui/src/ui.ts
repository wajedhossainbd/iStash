// Components, ported from design/prototype/ui.js. Each returns an HTML string; names match design/system.md.
import type { Forecast } from './api';

export const esc = (s: unknown) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!);
// ['a','b','c'] -> 'a, b and c'
export const list = (xs: string[]) => (xs.length < 2 ? xs.join('') : `${xs.slice(0, -1).join(', ')} and ${xs.at(-1)}`);
const GB = 1e9;
/** Bytes as decimal GB, like Finder. */
export const gb = (n: number) => `${Math.abs(n) >= 100 * GB ? Math.round(n / GB) : (n / GB).toFixed(1)} GB`;
export const tb = (n: number) => (n >= 1e12 ? `${+(n / 1e12).toFixed(1)} TB` : gb(n));
export const day = (ms: number) => new Date(ms).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
export const clock = (ms: number) => new Date(ms).toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' });
export function ago(ms: number, now = Date.now()) {
  const min = (now - ms) / 60e3;
  if (min < 2) return 'now';
  if (min < 60) return `${Math.round(min)} minutes ago`;
  if (min < 24 * 60) return `${Math.round(min / 60)} hours ago`;
  const d = Math.round(min / 1440);
  return d === 1 ? 'yesterday' : `${d} days ago`;
}

const P: Record<string, string> = {
  overview: '<path d="M2.5 11.5a5.5 5.5 0 0 1 11 0"/><path d="M8 11.5l2.6-3.4"/>',
  offload: '<path d="M8 2.5v7M5 6.8 8 9.8l3-3M3 13h10"/>',
  drives: '<rect x="2" y="4.5" width="12" height="7" rx="2"/><circle cx="11" cy="8" r=".9" fill="currentColor"/>',
  memory: '<rect x="2.5" y="4" width="11" height="8" rx="1.5"/><path d="M5 4V2.5M8 4V2.5M11 4V2.5M5 13.5V12M8 13.5V12M11 13.5V12"/>',
  settings: '<circle cx="8" cy="8" r="2.2"/><path d="M8 1.8v2M8 12.2v2M1.8 8h2M12.2 8h2M3.6 3.6l1.4 1.4M11 11l1.4 1.4M3.6 12.4 5 11M11 5l1.4-1.4"/>',
  warning: '<path d="M8 2.2 14 13H2z"/><path d="M8 6.5v3"/><circle cx="8" cy="11.3" r=".5" fill="currentColor"/>',
  check: '<path d="M3.5 8.5 6.5 11.5 12.5 4.5"/>',
  info: '<circle cx="8" cy="8" r="6"/><path d="M8 7.2V11"/><circle cx="8" cy="5" r=".5" fill="currentColor"/>',
  eject: '<path d="M8 3 13 9H3z"/><path d="M3 12h10"/>',
  plug: '<path d="M6 2v3M10 2v3M4.5 5h7v2.5a3.5 3.5 0 0 1-7 0z"/><path d="M8 11v3"/>',
  lock: '<rect x="3.5" y="7" width="9" height="6.5" rx="1.5"/><path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2"/>',
  app: '<rect x="3" y="3" width="10" height="10" rx="2.5"/>',
  power: '<path d="M8 2v5"/><path d="M4.6 4.4a5 5 0 1 0 6.8 0"/>',
  cable: '<circle cx="4" cy="4.5" r="1.8"/><circle cx="12" cy="11.5" r="1.8"/><path d="M5.6 5.4c4 .5 1.8 5.6 4.8 5.9"/>',
};
export const icon = (name: string, cls = '') =>
  `<svg class="${cls}" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${P[name]}</svg>`;

/** Real app icon from its bundle (resolved through api.appIcon); neutral tile when there is none. */
export const appIcon = (src: string | null | undefined, label: string) =>
  src ? `<img class="app-icon" src="${esc(src)}" alt="${esc(label)}">` : `<span class="app-icon placeholder" role="img" aria-label="${esc(label)}">${icon('app')}</span>`;

type BtnOpts = { kind?: string; id?: string; disabled?: boolean; title?: string; large?: boolean; label?: string };
export const button = (label: string, action: string, { kind = '', id = '', disabled = false, title = '', large = false, label: aria = '' }: BtnOpts = {}) =>
  `<button class="btn ${kind ? 'btn-' + kind : ''} ${large ? 'btn-large' : ''}" data-action="${action}" ${id ? `data-id="${esc(id)}"` : ''} ${disabled ? 'disabled' : ''} ${title ? `title="${esc(title)}"` : ''} ${aria ? `aria-label="${esc(aria)}"` : ''}>${label}</button>`;

export const pill = (kind: string, text: string) => `<span class="pill pill-${kind}">${esc(text)}</span>`;

export const seg = (action: string, options: [string, string, number?][], current: string) =>
  `<div class="seg" role="group">${options.map(([v, label, count]) =>
    `<button data-action="${action}" data-id="${v}" aria-pressed="${v === current}">${label}${count != null ? `<span class="count">${count}</span>` : ''}</button>`).join('')}</div>`;

type Part = { value: number; color: string; label?: string };
export const capacity = (parts: Part[], total: number, large = false) =>
  `<div class="capacity ${large ? 'large' : ''}" role="img" aria-label="${esc(parts.map((p) => p.label).filter(Boolean).join(', '))}">${parts.map((p) =>
    `<i style="width:${Math.max(0, (p.value / total) * 100)}%;background:${p.color}"></i>`).join('')}</div>`;

export const sparkline = (values: number[], color: string, max = Math.max(...values)) => {
  const pts = values.map((v, i) => `${(i / (values.length - 1)) * 200},${26 - (v / max) * 22}`).join(' ');
  return `<svg viewBox="0 0 200 28" preserveAspectRatio="none" aria-hidden="true"><polyline fill="none" stroke="${color}" stroke-width="1.5" vector-effect="non-scaling-stroke" points="${pts}"/></svg>`;
};

export const banner = ({ kind, ic = 'warning', title, body = '', actions = '', reason = '' }: { kind: string; ic?: string; title: string; body?: string; actions?: string; reason?: string }) =>
  `<div class="banner banner-${kind}" role="${kind === 'danger' ? 'alert' : 'status'}">${icon(ic)}<div><strong>${title}</strong>${body ? `<p>${body}</p>` : ''}</div>
   <div class="actions"><div>${actions}</div>${reason ? `<span class="reason">${reason}</span>` : ''}</div></div>`;

export const empty = ({ illus = '', title, body, actions = '' }: { illus?: string; title: string; body: string; actions?: string }) =>
  `<div class="empty">${illus}<h2>${title}</h2><p>${body}</p>${actions ? `<div class="actions">${actions}</div>` : ''}</div>`;

export const skeletonRows = (n: number) => Array.from({ length: n }, () =>
  `<div class="table-row" aria-hidden="true"><span class="skeleton" style="width:28px;height:28px"></span><span class="skeleton" style="height:12px;width:70%"></span><span class="skeleton" style="height:10px;width:90%"></span><span class="skeleton" style="height:12px"></span><span class="skeleton" style="height:12px;width:60%"></span><span></span></div>`).join('');

export const dialog = ({ title, body, buttons }: { title: string; body: string; buttons: string }) =>
  `<div class="backdrop" data-action="dismiss"><div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dlg-title">
     <h2 id="dlg-title">${title}</h2>${body}<div class="buttons">${buttons}</div></div></div>`;

export const progress = (pct: number, indeterminate = false, label = 'Progress') =>
  `<div class="progress ${indeterminate ? 'indeterminate' : ''}" role="progressbar" aria-label="${esc(label)}" ${indeterminate ? '' : `aria-valuenow="${Math.round(pct)}"`} aria-valuemin="0" aria-valuemax="100"><i style="${indeterminate ? '' : `width:${pct}%`}"></i></div>`;

export const STEPS = { move: ['Copy', 'Check every file', 'Link', 'Remove original'], back: ['Copy back', 'Check every file', 'Remove link', 'Remove drive copy'] };
export const steps = (kind: 'move' | 'back', at: number) => `<div class="steps">${STEPS[kind].map((s, i) =>
  `<span class="${i < at ? 'done' : i === at ? 'now' : ''}">${i < at ? '✓ ' : ''}${s}</span>`).join('')}</div>`;

export const toggle = (key: string, on: boolean, label: string) =>
  `<input type="checkbox" class="switch" role="switch" data-action="setting" data-id="${key}" ${on ? 'checked' : ''} aria-label="${esc(label)}">`;

// Swap chart: history solid, forecast dashed with a band; a limit line where swap has no more room.
export function forecastChart(f: Forecast, { height = 150, now = Date.now() } = {}) {
  const calm = f.condition === 'fine';
  const W = 880, H = height, top = 18, bottom = H - 22;
  const limit = f.swapLimit / GB, max = Math.ceil((limit * 1.1) / 10) * 10;
  const y = (g: number) => bottom - (g / max) * (bottom - top);
  const xNow = 40 + 55 * 8.2;
  const x = (m: number) => xNow + m * 8.2;
  const h = f.history.map((b) => b / GB);
  const hist = h.map((g, i) => `${x(i - h.length + 1).toFixed(1)},${y(g).toFixed(1)}`).join(' ');
  const cur = h.at(-1)!;
  const m = Math.min(45, f.minutesLeft ?? 45);
  const fc = calm
    ? `M${xNow} ${y(cur)} L ${x(45)} ${y(cur)}` // no shortage: the forecast is "about where it is", not an invented curve
    : `M${xNow} ${y(cur)} C ${x(m * 0.37)} ${y(cur + (limit - cur) * 0.28)} ${x(m * 0.74)} ${y(cur + (limit - cur) * 0.53)} ${x(m)} ${y(limit)}`;
  const band = calm ? '' : `<path d="M${xNow} ${y(cur)} C ${x(m * 0.26)} ${y(cur + (limit - cur) * 0.4)} ${x(m * 0.58)} ${y(cur + (limit - cur) * 0.6)} ${x(m * 0.79)} ${y(limit)} L${x(45)} ${y(limit)} L${x(45)} ${y(cur + (limit - cur) * 0.45)} C ${x(30)} ${y(cur + (limit - cur) * 0.29)} ${x(15)} ${y(cur + (limit - cur) * 0.13)} ${xNow} ${y(cur)}Z" fill="var(--warn-tint)"/>`;
  const grid = Array.from({ length: max / 10 + 1 }, (_, i) => i * 10).filter((g) => g <= max);
  const ticks = [-54, -34, -14, 16, 36].map((mm) => [mm, clock(now + mm * 60e3)] as const);
  const label = calm ? `Swap is ${gb(f.swap)}; no shortage expected`
    : `Swap rose from ${gb(f.history[0])} to ${gb(f.swap)} in the last hour and is forecast to reach the ${gb(f.swapLimit)} limit around ${clock(f.at!)}`;
  return `<svg viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(label)}">
    <g stroke="var(--line-hairline)">${grid.map((g) => `<line x1="40" x2="${W - 20}" y1="${y(g)}" y2="${y(g)}" ${g === 0 ? 'stroke="var(--line-strong)"' : ''}/>`).join('')}</g>
    <g font-size="10" fill="var(--text-tertiary)" font-family="var(--font-mono)" text-anchor="end">${grid.map((g, i) => `<text x="32" y="${y(g) + 3}">${g}${i === grid.length - 1 ? ' GB' : ''}</text>`).join('')}</g>
    <line x1="40" x2="${W - 20}" y1="${y(limit)}" y2="${y(limit)}" stroke="var(--danger-fill)" stroke-dasharray="3 3"/>
    <text x="${W - 20}" y="${y(limit) - 5}" text-anchor="end" font-size="10" fill="var(--danger-text)">No room left for swap (<tspan font-family="var(--font-mono)">${gb(f.swapLimit)}</tspan>)</text>
    ${band}
    <polyline points="${hist}" fill="none" stroke="var(--text-primary)" stroke-width="2"/>
    <path d="${fc}" fill="none" stroke="${calm ? 'var(--ok-fill)' : 'var(--warn-fill)'}" stroke-width="2" stroke-dasharray="5 4"/>
    <line x1="${xNow}" x2="${xNow}" y1="${top}" y2="${bottom}" stroke="var(--line-strong)"/>
    <circle cx="${xNow}" cy="${y(cur)}" r="4" fill="var(--bg-surface)" stroke="var(--text-primary)" stroke-width="2"/>
    ${calm || f.at == null ? '' : `<line x1="${x(m)}" x2="${x(m)}" y1="${y(limit)}" y2="${bottom}" stroke="var(--danger-fill)"/><circle cx="${x(m)}" cy="${y(limit)}" r="4.5" fill="var(--danger-fill)"/>
      <text x="${x(m) - 8}" y="${bottom - 6}" text-anchor="end" font-size="11" fill="var(--danger-text)">about <tspan font-family="var(--font-mono)">${clock(f.at)}</tspan></text>`}
    <g font-size="10" fill="var(--text-tertiary)" font-family="var(--font-mono)" text-anchor="middle">${ticks.map(([mm, t]) => `<text x="${x(mm)}" y="${H - 6}">${t}</text>`).join('')}
      <text x="${xNow}" y="${H - 6}" fill="var(--text-primary)" font-weight="600">now</text></g>
  </svg>`;
}
