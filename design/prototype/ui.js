// Components. Each returns an HTML string; names match design/system.md so the app can port them one to one.

export const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
// ['a','b','c'] -> 'a, b and c'
export const list = (xs) => (xs.length < 2 ? xs.join('') : `${xs.slice(0, -1).join(', ')} and ${xs.at(-1)}`);
export const gb = (n) => `${n >= 100 ? Math.round(n) : n.toFixed(1)} GB`;

const P = {
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
  bell: '<path d="M4 11V7.5a4 4 0 0 1 8 0V11l1 1.2H3z"/><path d="M6.8 13.5a1.3 1.3 0 0 0 2.4 0"/>',
  app: '<rect x="3" y="3" width="10" height="10" rx="2.5"/>',
  folder: '<path d="M2 4.5h4l1.2 1.3H14V12.5H2z"/>',
  trash: '<path d="M3 4.5h10M6 4.5V3h4v1.5M4.2 4.5l.8 9h6l.8-9"/>',
  power: '<path d="M8 2v5"/><path d="M4.6 4.4a5 5 0 1 0 6.8 0"/>',
  cable: '<circle cx="4" cy="4.5" r="1.8"/><circle cx="12" cy="11.5" r="1.8"/><path d="M5.6 5.4c4 .5 1.8 5.6 4.8 5.9"/>',
};
export const icon = (name, cls = '') => `<svg class="${cls}" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${P[name]}</svg>`;

// Real app icon (fetched locally by fetch-icons.sh, read from the app bundle at runtime); neutral tile otherwise.
export const appIcon = (appId, label) =>
  `<span class="app-icon placeholder" role="img" aria-label="${esc(label)}">${icon('app')}<img class="app-icon" src="icons/${appId}.png" alt="" hidden onload="this.hidden=false;this.parentNode.replaceWith(this)"></span>`;

export const button = (label, action, { kind = '', id = '', disabled = false, title = '', large = false } = {}) =>
  `<button class="btn ${kind ? 'btn-' + kind : ''} ${large ? 'btn-large' : ''}" data-action="${action}" ${id ? `data-id="${id}"` : ''} ${disabled ? 'disabled' : ''} ${title ? `title="${esc(title)}"` : ''}>${label}</button>`;

export const pill = (kind, text) => `<span class="pill pill-${kind}">${esc(text)}</span>`;

export const seg = (action, options, current) =>
  `<div class="seg" role="group">${options.map(([v, label, count]) =>
    `<button data-action="${action}" data-id="${v}" aria-pressed="${v === current}">${label}${count != null ? `<span class="count">${count}</span>` : ''}</button>`).join('')}</div>`;

// parts: [{ value, color }] in the same unit as total.
export const capacity = (parts, total, large = false) =>
  `<div class="capacity ${large ? 'large' : ''}" role="img" aria-label="${parts.map((p) => p.label).filter(Boolean).join(', ')}">${parts.map((p) =>
    `<i style="width:${(p.value / total) * 100}%;background:${p.color}"></i>`).join('')}</div>`;

export const sparkline = (values, color, max = Math.max(...values)) => {
  const pts = values.map((v, i) => `${(i / (values.length - 1)) * 200},${26 - (v / max) * 22}`).join(' ');
  return `<svg viewBox="0 0 200 28" preserveAspectRatio="none" aria-hidden="true"><polyline fill="none" stroke="${color}" stroke-width="1.5" vector-effect="non-scaling-stroke" points="${pts}"/></svg>`;
};

export const banner = ({ kind, ic = 'warning', title, body, actions = '', reason = '' }) =>
  `<div class="banner banner-${kind}" role="${kind === 'danger' ? 'alert' : 'status'}">${icon(ic)}<div><strong>${title}</strong>${body ? `<p>${body}</p>` : ''}</div>
   <div class="actions"><div>${actions}</div>${reason ? `<span class="reason">${reason}</span>` : ''}</div></div>`;

export const empty = ({ illus = '', title, body, actions = '' }) =>
  `<div class="empty">${illus}<h2>${title}</h2><p>${body}</p>${actions ? `<div class="actions">${actions}</div>` : ''}</div>`;

export const skeletonRows = (n) => Array.from({ length: n }, () =>
  `<div class="table-row"><span class="skeleton" style="width:28px;height:28px"></span><span class="skeleton" style="height:12px;width:70%"></span><span class="skeleton" style="height:10px;width:90%"></span><span class="skeleton" style="height:12px"></span><span class="skeleton" style="height:12px;width:60%"></span><span></span></div>`).join('');

export const dialog = ({ title, body, buttons }) =>
  `<div class="backdrop" data-action="dismiss"><div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dlg-title" data-stop>
     <h2 id="dlg-title">${title}</h2>${body}<div class="buttons">${buttons}</div></div></div>`;

export const progress = (pct, indeterminate = false) =>
  `<div class="progress ${indeterminate ? 'indeterminate' : ''}" role="progressbar" aria-valuenow="${Math.round(pct)}" aria-valuemin="0" aria-valuemax="100"><i style="width:${pct}%"></i></div>`;

export const STEPS = { move: ['Copy', 'Check every file', 'Link', 'Remove original'], back: ['Copy back', 'Check every file', 'Remove link', 'Remove drive copy'] };
export const steps = (kind, at) => `<div class="steps">${STEPS[kind].map((s, i) =>
  `<span class="${i < at ? 'done' : i === at ? 'now' : ''}">${i < at ? '✓ ' : ''}${s}</span>`).join('')}</div>`;

export const toggle = (key, on, label) => `<input type="checkbox" class="switch" data-action="setting" data-id="${key}" ${on ? 'checked' : ''} aria-label="${esc(label)}">`;

// Swap chart: history solid, forecast dashed with a band; limit line where macOS runs out of room.
export function forecastChart(history, { calm = false, height = 150 } = {}) {
  const W = 880, H = height, top = 18, bottom = H - 22, max = 24, limit = 22;
  const y = (g) => bottom - (g / max) * (bottom - top);
  const xNow = 40 + 55 * 8.2;
  const x = (m) => xNow + m * 8.2;
  const hist = history.map((g, i) => `${x(i - 55).toFixed(1)},${y(g).toFixed(1)}`).join(' ');
  const now = history.at(-1);
  const fc = calm
    ? `M${xNow} ${y(now)} C ${x(15)} ${y(now - 1.2)} ${x(30)} ${y(now - 2.4)} ${x(45)} ${y(now - 2.8)}`
    : `M${xNow} ${y(now)} C ${x(14)} ${y(now + 3.5)} ${x(28)} ${y(16)} ${x(38)} ${y(limit)}`;
  const band = calm ? '' : `<path d="M${xNow} ${y(now)} C ${x(10)} ${y(now + 5)} ${x(22)} ${y(17)} ${x(30)} ${y(limit)} L${x(45)} ${y(limit)} L${x(45)} ${y(15)} C ${x(30)} ${y(13)} ${x(15)} ${y(11)} ${xNow} ${y(now)}Z" fill="var(--warn-tint)"/>`;
  const ticks = [[-54, '13:20'], [-34, '13:40'], [-14, '14:00'], [16, '14:30'], [36, '14:50']];
  return `<svg viewBox="0 0 ${W} ${H}" role="img" aria-label="${calm ? 'Swap is falling; no shortage expected' : 'Swap rose from 3.1 to 9.4 GB in 55 minutes and is forecast to reach the 22 GB limit around 14:52'}">
    <g stroke="var(--line-hairline)"><line x1="40" x2="${W - 20}" y1="${y(0)}" y2="${y(0)}" stroke="var(--line-strong)"/><line x1="40" x2="${W - 20}" y1="${y(10)}" y2="${y(10)}"/><line x1="40" x2="${W - 20}" y1="${y(20)}" y2="${y(20)}"/></g>
    <g font-size="10" fill="var(--text-tertiary)" font-family="var(--font-mono)" text-anchor="end"><text x="32" y="${y(0) + 3}">0</text><text x="32" y="${y(10) + 3}">10</text><text x="32" y="${y(20) + 3}">20 GB</text></g>
    <line x1="40" x2="${W - 20}" y1="${y(limit)}" y2="${y(limit)}" stroke="var(--danger-fill)" stroke-dasharray="3 3"/>
    <text x="${W - 20}" y="${y(limit) - 5}" text-anchor="end" font-size="10" font-family="var(--font-mono)" fill="var(--danger-text)">No room left for swap (22 GB)</text>
    ${band}
    <polyline points="${hist}" fill="none" stroke="var(--text-primary)" stroke-width="2"/>
    <path d="${fc}" fill="none" stroke="${calm ? 'var(--ok-fill)' : 'var(--warn-fill)'}" stroke-width="2" stroke-dasharray="5 4"/>
    <line x1="${xNow}" x2="${xNow}" y1="${top}" y2="${bottom}" stroke="var(--line-strong)"/>
    <circle cx="${xNow}" cy="${y(now)}" r="4" fill="var(--bg-surface)" stroke="var(--text-primary)" stroke-width="2"/>
    ${calm ? '' : `<line x1="${x(38)}" x2="${x(38)}" y1="${y(limit)}" y2="${bottom}" stroke="var(--danger-fill)"/><circle cx="${x(38)}" cy="${y(limit)}" r="4.5" fill="var(--danger-fill)"/>
      <text x="${x(38) - 8}" y="${bottom - 6}" text-anchor="end" font-size="11" font-family="var(--font-mono)" fill="var(--danger-text)">about 14:52</text>`}
    <g font-size="10" fill="var(--text-tertiary)" font-family="var(--font-mono)" text-anchor="middle">${ticks.map(([m, t]) => `<text x="${x(m)}" y="${H - 6}">${t}</text>`).join('')}
      <text x="${xNow}" y="${H - 6}" fill="var(--text-primary)" font-weight="600">now</text></g>
  </svg>`;
}
