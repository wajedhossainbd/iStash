// Screens. Each takes the whole state and returns HTML. Shared pieces live in ui.js.
import { esc, gb, list, icon, appIcon, button, pill, seg, capacity, sparkline, banner, empty, skeletonRows, progress, steps, toggle, forecastChart } from './ui.js';
import { SWAP_HISTORY } from './data.js';

const drive = (s, id) => s.drives.find((d) => d.id === id);
const isHeld = (s, it) => it.verdict === 'stowed' && !s.connected[it.drive];
const stowedOn = (s, id) => s.items.filter((i) => i.verdict === 'stowed' && i.drive === id);
const suggested = (s) => s.items.filter((i) => i.verdict === 'move');
const calm = (s) => !s.memApps.some((a) => a.id === 'docker');
const cableVar = (d) => `var(--cable-${d.cable})`;

const ILLUS_PORT = `<svg class="illus" viewBox="0 0 120 72" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="62" y="16" width="52" height="40" rx="6"/><circle cx="76" cy="36" r="6"/><circle cx="76" cy="36" r="2" fill="currentColor"/><path d="M8 54c14 0 18-18 34-18h22" stroke-dasharray="3 4"/><rect x="2" y="48" width="12" height="10" rx="2"/></svg>`;

// ---------- Sidebar ----------
export function sidebar(s) {
  const missing = s.drives.filter((d) => !s.connected[d.id] && stowedOn(s, d.id).length).length;
  const nav = [
    ['overview', 'Overview', calm(s) ? '' : '<span class="dot warn" style="margin-left:auto" title="Memory strained"></span>'],
    ['offload', 'Offload', `<span class="badge">${suggested(s).length}</span>`],
    ['drives', 'Drives', missing ? `<span class="badge is-danger">${missing} missing</span>` : ''],
    ['memory', 'Memory', ''],
  ];
  const item = ([r, label, extra]) => `<a class="nav-item ${s.route === r ? 'is-active' : ''}" href="#/${r}" ${s.route === r ? 'aria-current="page"' : ''}>${icon(r)}${label}${extra}</a>`;
  const free = s.macFree, cpu = s.live.cpu.at(-1);
  return `<aside class="sidebar"><div class="traffic"><i></i><i></i><i></i></div>
    <nav aria-label="Sections">${nav.map(item).join('')}
      <div class="nav-group">Drives</div>
      ${s.drives.map((d) => `<a class="nav-item" href="#/drives"><span class="dot ${s.connected[d.id] ? 'ok' : 'hollow'}"></span>${esc(d.name)}${s.connected[d.id] ? '' : '<span class="badge is-danger">missing</span>'}</a>`).join('')}
      <div class="nav-group"></div>${item(['settings', 'Settings', ''])}
    </nav>
    <div class="sidebar-foot">
      <div class="mini"><span>CPU</span>${capacity([{ value: cpu, color: 'var(--text-secondary)' }], 100)}<span class="num" data-live="cpu-mini">${cpu}%</span></div>
      <div class="mini"><span>Memory</span>${capacity([{ value: s.live.pressure, color: calm(s) ? 'var(--ok-fill)' : 'var(--warn-fill)' }], 100)}<span class="num">${s.live.pressure}%</span></div>
      <div class="mini"><span>Disk</span>${capacity([{ value: s.macTotal - free, color: free < 40 ? 'var(--danger-fill)' : 'var(--text-secondary)' }], s.macTotal)}<span class="num">${Math.round(free)} GB</span></div>
    </div></aside>`;
}

const header = (title, subtitle, right = '') =>
  `<header class="page-header"><h1>${title}</h1><span class="subtitle">${subtitle}</span><span class="spacer"></span>${right}</header>`;

// ---------- Overview ----------
function forecastCard(s, { big = false } = {}) {
  const ok = calm(s);
  return `<section class="card" aria-labelledby="fc-title">
    <div class="forecast">
      <div style="display:grid;gap:6px">
        <div style="display:flex;gap:8px;align-items:center"><h2 class="secondary" style="font:var(--type-group)">Memory forecast</h2>${ok ? pill('ok', 'Fine') : pill('warn', 'Strained')}</div>
        <p class="sentence" id="fc-title">${ok ? 'Memory looks fine for the next few hours.' : 'Memory runs out in about 38 minutes.'}</p>
        <p class="why">${ok ? 'Swap is shrinking since Docker Desktop quit. Stowaway keeps watching and will warn you well before it matters.'
          : 'Swap grew 2.1 GB in the last 20 minutes and only 31.2 GB of disk is left for it. When swap can’t grow, macOS shows “Your system has run out of application memory” and pauses apps.'}</p>
        ${ok ? '' : `<div class="actions">${button('Quit Docker Desktop…', 'quit-app', { kind: 'primary', id: 'docker' })}${s.route === 'memory' ? '' : button('What’s using memory', 'go', { id: 'memory' })}<span class="tertiary" style="font:var(--type-caption)">Frees about 6.2 GB</span></div>`}
      </div>
      <dl style="margin:0;display:grid;gap:2px;text-align:right">
        <dt class="secondary" style="font:var(--type-caption)">Memory pressure</dt><dd class="num" style="margin:0;font:var(--type-figure)">${s.live.pressure}%</dd>
        <dt class="secondary" style="font:var(--type-caption);margin-top:6px">Swap</dt><dd class="num" style="margin:0">${ok ? '8.1 GB, falling' : '9.4 GB, +6.3 GB/h'}</dd>
      </dl>
    </div>
    <div class="forecast-chart" style="margin-top:12px">
      <div class="chart-key"><span><i style="background:var(--text-primary)"></i>Swap used</span><span><i style="background:repeating-linear-gradient(90deg,${ok ? 'var(--ok-fill)' : 'var(--warn-fill)'} 0 4px,transparent 4px 7px)"></i>Forecast</span></div>
      ${forecastChart(SWAP_HISTORY, { calm: ok, height: big ? 220 : 150 })}
    </div></section>`;
}

function metrics(s) {
  const cpu = s.live.cpu, net = s.live.net;
  return `<div class="metrics">
    <div class="metric"><span class="label">CPU</span><span class="figure num" data-live="cpu">${cpu.at(-1)}<small>%</small></span><span class="detail">8 performance, 4 efficiency cores</span><span data-live="cpu-spark">${sparkline(cpu, 'var(--accent-text)', 100)}</span></div>
    <div class="metric"><span class="label">Memory</span><span class="figure num">13.8<small>of 16 GB</small></span><span class="detail">Swap 9.4 GB · compressed 3.2 GB</span>${capacity([{ value: 13.8, color: calm(s) ? 'var(--ok-fill)' : 'var(--warn-fill)', label: '13.8 GB used' }], 16)}</div>
    <div class="metric ${s.macFree < 40 ? 'is-danger' : ''}"><span class="label">Disk</span><span class="figure num">${s.macFree.toFixed(1)}<small>GB free</small></span><span class="detail">of ${s.macTotal} GB on Macintosh HD</span>${capacity([{ value: s.macTotal - s.macFree, color: 'var(--text-secondary)', label: 'used' }], s.macTotal)}</div>
    <div class="metric"><span class="label">Network</span><span class="figure num" data-live="net">${net.at(-1).toFixed(1)}<small>MB/s in</small></span><span class="detail">310 KB/s out · Wi-Fi</span><span data-live="net-spark">${sparkline(net, 'var(--ok-fill)', 6)}</span></div>
  </div>`;
}

export function overview(s) {
  const h = header('Overview', 'MacBook Pro · updates every 2 seconds');
  if (s.view === 'loading') return h + `<div class="page-body"><div class="card" style="display:grid;gap:10px"><span class="skeleton" style="height:14px;width:30%"></span><span class="skeleton" style="height:26px;width:60%"></span><span class="skeleton" style="height:120px"></span><p class="secondary">Collecting the first minute of samples. The memory forecast starts after 5 minutes.</p></div>
    <div class="metrics">${'<div class="metric"><span class="skeleton" style="height:12px;width:40%"></span><span class="skeleton" style="height:26px;width:60%;margin-top:6px"></span></div>'.repeat(4)}</div></div>`;
  if (s.view === 'error') return h + `<div class="page-body">${empty({ illus: icon('power', 'illus'), title: 'The background helper isn’t running',
    body: 'Live numbers, memory forecasts and eject warnings come from Stowaway’s helper. It uses about 14 MB of memory and starts at login.',
    actions: button('Start helper', 'fix', { kind: 'primary' }) + button('Open Login Items…', 'noop') })}</div>`;
  const stowable = suggested(s).reduce((a, i) => a + i.size, 0);
  const other = s.macTotal - 104 - 153 - stowable - s.macFree;
  return h + `<div class="page-body">${forecastCard(s)}${metrics(s)}
    <div style="display:grid;grid-template-columns:1.4fr 1fr;gap:12px">
      <section class="card"><div class="card-head"><h2>This Mac</h2><span class="num secondary">${gb(s.macFree)} free of ${s.macTotal} GB</span><span class="spacer"></span>${button(`Review ${suggested(s).length} folders`, 'go', { id: 'offload' })}</div>
        ${capacity([{ value: 104, color: 'var(--storage-system)', label: 'macOS 104 GB' }, { value: 153, color: 'var(--storage-apps)', label: 'Apps 153 GB' }, { value: stowable, color: 'var(--accent-fill)', label: `Can move ${gb(stowable)}` }, { value: other, color: 'var(--storage-other)', label: 'Other' }], s.macTotal, true)}
        <div class="legend"><span><i style="background:var(--storage-system)"></i>macOS 104 GB</span><span><i style="background:var(--storage-apps)"></i>Apps 153 GB</span><span><i style="background:var(--accent-fill)"></i>Can move to a drive ${gb(stowable)}</span><span><i style="background:var(--storage-other)"></i>Other</span></div></section>
      <section class="card"><div class="card-head"><h2>Drives</h2></div><div style="display:grid;gap:8px">${s.drives.map((d) => {
        const on = stowedOn(s, d.id);
        return `<div style="display:flex;gap:8px;align-items:center"><span class="dot ${s.connected[d.id] ? 'ok' : 'hollow'}"></span><strong style="font:var(--type-emphasis)">${esc(d.name)}</strong>
          <span class="secondary" style="margin-left:auto;font:var(--type-callout)">${s.connected[d.id] ? `${on.length} folders · ${gb(on.reduce((a, i) => a + i.size, 0))}` : `<span style="color:var(--danger-text)">Missing</span> · ${on.map((i) => esc(i.app)).join(', ')} held`}</span></div>`;
      }).join('')}</div></section>
    </div></div>`;
}

// ---------- Offload ----------
function itemRow(s, it) {
  const job = s.jobs[it.id];
  const head = `${appIcon(it.appId, it.app)}<div class="name"><strong>${esc(it.name)}</strong><span>${esc(it.app)}</span></div>`;
  if (job) {
    const d = drive(s, job.drive);
    return `<div class="table-row is-busy">${head}<div class="progress-cell">${steps(job.kind, job.step)}${progress(job.pct)}
      <span class="tertiary num" style="font:var(--type-caption)">${gb(it.size * job.pct / 100)} of ${gb(it.size)} · ${job.kind === 'move' ? `to ${esc(d.name)}` : 'to this Mac'} · original kept until the check passes</span></div>
      <div class="act">${button('Stop', 'cancel-job', { kind: 'plain', id: it.id })}</div></div>`;
  }
  let verdict, act;
  if (it.verdict === 'move') {
    verdict = pill('accent', 'Can move') + `<span>${esc(it.running ? `${it.app} is open and will be asked to quit` : it.reason)}</span>`;
    act = it.running ? button(`Quit & move…`, 'move', { kind: 'primary', id: it.id }) : button('Move…', 'move', { id: it.id });
  } else if (it.verdict === 'setting') {
    verdict = pill('neutral', `Use ${it.app}’s setting`) + `<span>${esc(it.reason)}</span>`;
    act = button('Show how', 'how', { id: it.id });
  } else if (it.verdict === 'refused') {
    verdict = pill('danger', 'Can’t move') + `<span>${esc(it.reason)}</span>`;
    act = button('Why?', 'why', { kind: 'plain', id: it.id });
  } else {
    const d = drive(s, it.drive);
    verdict = isHeld(s, it) ? pill('danger', `${d.name} is missing`) + `<span>${esc(it.app)} is held until it’s back</span>`
      : pill('ok', `On ${d.name}`) + `<span>Since ${esc(it.since)}, checked file by file</span>`;
    act = button('Bring back…', 'back', { id: it.id, disabled: isHeld(s, it), title: isHeld(s, it) ? `Connect ${d.name} first` : '' });
  }
  return `<div class="table-row">${head}<div class="path" title="${esc(it.path)}">${esc(it.path)}</div><div class="size">${gb(it.size)}</div><div class="verdict">${verdict}</div><div class="act">${act}</div></div>`;
}

export function offload(s) {
  const tabs = { suggested: suggested(s).concat(s.items.filter((i) => ['setting', 'refused'].includes(i.verdict)) ), stowed: s.items.filter((i) => i.verdict === 'stowed'), all: s.items };
  const busy = Object.keys(s.jobs).map((id) => s.items.find((i) => i.id === id));
  const rows = [...new Set([...busy, ...tabs[s.offloadTab]])];
  const right = seg('tab', [['suggested', 'Suggested', tabs.suggested.length], ['stowed', 'On drives', tabs.stowed.length], ['all', 'All']], s.offloadTab) + button('Scan again', 'scan');
  const h = header('Offload', 'Move heavy app data to a drive. The apps keep working.', s.view === 'normal' ? right : '');
  const head = '<div class="table-head"><span></span><span>Folder</span><span>Where it lives</span><span style="text-align:right">Size</span><span>Verdict</span><span></span></div>';
  if (s.view === 'loading') return h + `<div class="page-body"><section class="card" style="display:grid;gap:8px"><div style="display:flex;justify-content:space-between"><h2>Looking for big app folders…</h2><span class="num secondary">1,284 checked</span></div>${progress(0, true)}
    <p class="secondary" style="font:var(--type-caption)">Skipping app sandboxes, Documents and Desktop. Nothing is changed while scanning.</p></section><div class="table">${head}${skeletonRows(5)}</div></div>`;
  if (s.view === 'empty') return h + `<div class="page-body">${empty({ illus: ILLUS_PORT, title: 'Connect a drive to start',
    body: 'Stowaway moves app data to an external drive formatted as APFS or Mac OS Extended. Plug one in and it appears here.', actions: button('Which drives work?', 'apfs', { kind: 'plain' }) })}</div>`;
  if (s.view === 'error') return h + `<div class="page-body">${empty({ illus: icon('lock', 'illus'), title: 'Stowaway can’t see your drives',
    body: 'macOS hasn’t allowed access to removable volumes, so nothing can be moved. Nothing was changed.', actions: button('Open System Settings…', 'fix', { kind: 'primary' }) })}</div>`;
  const stowable = suggested(s).reduce((a, i) => a + i.size, 0);
  return h + `<div class="page-body">
    <section class="card" style="display:grid;grid-template-columns:1fr auto;gap:8px 24px;align-items:end">
      <div><h2 class="secondary" style="font:var(--type-group)">This Mac</h2><span class="num" style="font:var(--type-figure)">${gb(s.macFree)}</span> <span class="secondary">free of ${s.macTotal} GB</span></div>
      <div style="text-align:right"><h2 class="secondary" style="font:var(--type-group)">After moving the ${suggested(s).length} suggested folders</h2><span class="num" style="font:var(--type-figure);color:var(--ok-text)">${gb(s.macFree + stowable)}</span> <span class="secondary">free</span></div>
      <div style="grid-column:1/-1">${capacity([{ value: s.macTotal - s.macFree - stowable, color: 'var(--storage-system)', label: 'Used' }, { value: stowable, color: 'var(--accent-fill)', label: `Can move ${gb(stowable)}` }], s.macTotal, true)}</div>
    </section>
    <div class="table" role="table">${head}${rows.length ? rows.map((it) => itemRow(s, it)).join('') : '<div class="empty" style="padding:32px"><h2>Nothing here yet</h2><p>Folders you move to a drive show up in this list.</p></div>'}</div>
    <p class="tertiary" style="font:var(--type-caption)">Stowaway never touches Documents, Desktop or app sandboxes. Every move is written to a journal, so an interrupted move finishes or rolls back the next time Stowaway starts.</p>
  </div>`;
}

// ---------- Drives ----------
export function cableLayout(s) {
  const linked = s.items.filter((i) => i.verdict === 'stowed');
  const apps = [...new Map(linked.map((i) => [i.app, i])).values()];
  const rowY = (i) => 34 + i * 54;
  const jackX = 300, portX = 640;
  const blocks = []; let y = 10;
  for (const d of s.drives) {
    const n = Math.max(1, apps.filter((a) => linked.some((i) => i.app === a.app && i.drive === d.id)).length);
    blocks.push({ d, y, h: n * 40 + 30 }); y += n * 40 + 46;
  }
  const H = Math.max(rowY(apps.length - 1) + 40, y) + 10;
  const cables = []; const used = {};
  apps.forEach((a, i) => {
    for (const d of s.drives) {
      if (!linked.some((x) => x.app === a.app && x.drive === d.id)) continue;
      const b = blocks.find((b) => b.d.id === d.id);
      used[d.id] = (used[d.id] || 0) + 1;
      cables.push({ app: a, d, ya: rowY(i), yp: b.y + 24 + (used[d.id] - 1) * 40, floor: H - 12 });
    }
  });
  return { apps, blocks, cables, H, rowY, jackX, portX, linked };
}

// t = 1 plugged in, 0 hanging loose. Both shapes are one cubic so they interpolate.
export function cablePath(c, t, jackX = 300, portX = 640) {
  const sag = 46 + Math.abs(c.yp - c.ya) * 0.15;
  const on = [jackX, c.ya, jackX + 130, c.ya + sag, portX - 140, c.yp + sag, portX, c.yp];
  const off = [jackX, c.ya, jackX + 40, c.ya + 50, jackX + 110, c.floor, jackX + 180, c.floor];
  const p = on.map((v, i) => (off[i] + (v - off[i]) * t).toFixed(1));
  return `M${p[0]} ${p[1]} C${p[2]} ${p[3]} ${p[4]} ${p[5]} ${p[6]} ${p[7]}`;
}

function cableMap(s) {
  const L = cableLayout(s);
  const sizeOf = (app) => L.linked.filter((i) => i.app === app).reduce((a, i) => a + i.size, 0);
  const appRows = L.apps.map((a, i) => {
    const held = L.linked.some((x) => x.app === a.app && !s.connected[x.drive]);
    const y = L.rowY(i);
    return `<g data-hot="app:${esc(a.app)}"><rect class="hit" x="0" y="${y - 24}" width="${L.jackX + 12}" height="48"/>
      <rect x="0" y="${y - 14}" width="28" height="28" rx="7" fill="var(--bg-inset)"/><image href="icons/${a.appId}.png" x="0" y="${y - 14}" width="28" height="28" onerror="this.remove()"/>
      <text x="40" y="${y - 2}">${esc(a.app)}</text><text x="40" y="${y + 13}" class="sub ${held ? 'bad' : ''}">${held ? 'held, drive missing' : gb(sizeOf(a.app))}</text>
      <circle cx="${L.jackX}" cy="${y}" r="8" fill="var(--jack-hole)" stroke="${held ? 'var(--danger-fill)' : 'var(--jack-ring)'}" stroke-width="2"/></g>`;
  }).join('');
  const blocks = L.blocks.map(({ d, y, h }) => {
    const on = s.connected[d.id], ports = L.cables.filter((c) => c.d.id === d.id);
    return `<g data-hot="drive:${d.id}"><rect x="${L.portX - 22}" y="${y}" width="${880 - L.portX + 22}" height="${h}" rx="8" fill="${on ? 'var(--bg-inset)' : 'none'}" stroke="${on ? 'var(--line-strong)' : 'var(--danger-fill)'}" ${on ? '' : 'stroke-dasharray="4 4"'}/>
      <rect x="866" y="${y + 14}" width="8" height="8" rx="2" fill="${cableVar(d)}"/>
      <text x="858" y="${y + 22}" text-anchor="end" style="font-weight:600">${esc(d.name)}</text>
      <text x="858" y="${y + 38}" text-anchor="end" class="sub ${on ? '' : 'bad'}">${on ? `${d.free} GB free` : `not connected · seen ${d.lastSeen}`}</text>
      ${ports.map((c) => `<circle cx="${L.portX}" cy="${c.yp}" r="8" fill="var(--jack-hole)" stroke="${on ? 'var(--jack-ring)' : 'var(--danger-fill)'}" stroke-width="2"/>`).join('')}</g>`;
  }).join('');
  const cables = L.cables.map((c, i) => {
    const on = s.connected[c.d.id];
    return `<g class="cable-group" data-app="${esc(c.app.app)}" data-drive="${c.d.id}">
      <path class="cable" data-cable="${i}" d="${cablePath(c, on ? 1 : 0, L.jackX, L.portX)}" stroke="${cableVar(c.d)}" stroke-width="4.5"/>
      <rect class="plug" data-plug="${i}" x="${(on ? L.portX : L.jackX + 180) - 8}" y="${(on ? c.yp : c.floor) - 5}" width="16" height="10" rx="3" fill="${cableVar(c.d)}"/>
      <rect x="${L.jackX - 8}" y="${c.ya - 5}" width="16" height="10" rx="3" fill="${cableVar(c.d)}"/></g>`;
  }).join('');
  return `<section class="bay" aria-label="Which app needs which drive">
    <svg viewBox="0 0 880 ${L.H}" role="img" aria-label="${L.cables.map((c) => `${c.app.app} uses ${c.d.name}${s.connected[c.d.id] ? '' : ' (missing)'}`).join('; ')}">${appRows}${blocks}${cables}</svg></section>`;
}

function driveModule(s, d) {
  const on = s.connected[d.id], st = stowedOn(s, d.id), stowed = st.reduce((a, i) => a + i.size, 0);
  const used = d.total - d.free;
  return `<article class="module ${on ? '' : 'is-missing'}" style="--c:${cableVar(d)}" data-hot="drive:${d.id}">
    <header><span class="swatch"></span><strong>${esc(d.name)}</strong><span class="kind">${esc(d.kind)}</span><span class="spacer"></span>
      ${on ? button(icon('eject'), 'eject', { id: d.id, title: `Eject ${d.name}` }) : pill('danger', 'Missing')}</header>
    ${capacity(on ? [{ value: used - stowed, color: 'var(--storage-apps)', label: 'Other files' }, { value: stowed, color: cableVar(d), label: `App data ${gb(stowed)}` }] : [], d.total)}
    <div class="stats"><span>${on ? `<span class="num">${d.free} GB</span> free of ${d.total >= 1000 ? d.total / 1000 + ' TB' : d.total + ' GB'}` : `Last seen ${d.lastSeen}`}</span><span><span class="num">${st.length}</span> folder${st.length === 1 ? '' : 's'} · <span class="num">${gb(stowed)}</span></span></div>
  </article>`;
}

export function drives(s) {
  const t7 = s.connected.t7;
  const h = header('Drives', 'Which app needs which drive.', s.view === 'normal' && t7 ? button(`${icon('eject')} Eject Samsung T7…`, 'eject', { id: 't7' }) : '');
  if (s.view === 'loading') return h + `<div class="page-body"><div class="modules">${'<div class="module"><span class="skeleton" style="height:14px;width:50%"></span><span class="skeleton" style="height:8px"></span><span class="skeleton" style="height:10px;width:70%"></span></div>'.repeat(3)}</div><div class="bay"><span class="skeleton" style="display:block;height:260px"></span></div></div>`;
  if (s.view === 'empty') return h + `<div class="page-body">${empty({ illus: ILLUS_PORT, title: 'No app data on drives yet', body: 'When you move a folder to a drive, it shows up here with the app that needs it, so you know what breaks before you unplug.', actions: button('Go to Offload', 'go', { id: 'offload', kind: 'primary' }) })}</div>`;
  if (s.view === 'error') return h + `<div class="page-body">${banner({ kind: 'danger', title: 'Samsung T7 can’t be read', body: 'macOS reports the disk needs First Aid. Stowaway won’t write to it and keeps its four apps held until it’s repaired.', actions: button('Open Disk Utility…', 'noop', { kind: 'primary' }) })}${cableMap(s)}</div>`;
  const missing = s.drives.filter((d) => !s.connected[d.id] && stowedOn(s, d.id).length);
  const mac = `<article class="module" style="--c:var(--text-secondary)"><header><span class="swatch"></span><strong>Macintosh HD</strong><span class="kind">Internal</span></header>
    ${capacity([{ value: s.macTotal - s.macFree, color: s.macFree < 40 ? 'var(--danger-fill)' : 'var(--storage-system)', label: 'Used' }], s.macTotal)}
    <div class="stats"><span><span class="num" style="color:${s.macFree < 40 ? 'var(--danger-text)' : ''}">${gb(s.macFree)}</span> free of ${s.macTotal} GB</span><span>${s.macFree < 40 ? 'Low' : ''}</span></div></article>`;
  return h + `<div class="page-body">
    <div class="modules">${s.drives.map((d) => driveModule(s, d)).join('')}${mac}</div>
    ${cableMap(s)}
    ${missing.map((d) => {
      const st = stowedOn(s, d.id), size = st.reduce((a, i) => a + i.size, 0), appList = [...new Set(st.map((i) => i.app))], apps = list(appList);
      return banner({ kind: 'danger', ic: 'plug', title: `${d.name} is missing, so ${apps} ${appList.length > 1 ? 'are' : 'is'} held.`, body: `${apps} would otherwise write to an empty folder. Plug ${d.name} in and ${appList.length > 1 ? 'they open' : 'it opens'} normally, or bring the data (${gb(size)}) back to this Mac.`,
        actions: button('Bring back to Mac', 'back', { id: st[0].id, disabled: size > s.macFree - 10 }), reason: size > s.macFree - 10 ? `Needs ${gb(size)}, this Mac has ${gb(s.macFree)} free` : '' });
    }).join('')}
  </div>`;
}

// ---------- Memory ----------
export function memory(s) {
  const h = header('Memory', 'Pressure, swap, and what’s using it.');
  if (s.view !== 'normal') return overview({ ...s, route: 'overview' }).replace(/^<header[\s\S]*?<\/header>/, h);
  const max = Math.max(...s.memApps.map((a) => a.mem));
  return h + `<div class="page-body">${forecastCard(s, { big: true })}
    <section class="table"><div class="table-head" style="grid-template-columns:32px minmax(0,1.3fr) minmax(0,1.6fr) 84px 140px"><span></span><span>App</span><span>Memory</span><span style="text-align:right">Size</span><span></span></div>
    ${s.memApps.map((a) => `<div class="table-row" style="grid-template-columns:32px minmax(0,1.3fr) minmax(0,1.6fr) 84px 140px">${appIcon(a.appId, a.app)}<div class="name"><strong>${esc(a.app)}</strong><span>${esc(a.detail)}</span></div>
      ${capacity([{ value: a.mem, color: 'var(--accent-fill)', label: gb(a.mem) }], max)}<div class="size">${gb(a.mem)}</div><div class="act">${button('Quit…', 'quit-app', { id: a.id })}</div></div>`).join('')}</section></div>`;
}

// ---------- Settings ----------
export function settings(s) {
  const st = s.settings;
  const row = (title, note, control) => `<div class="row"><div>${title}${note ? `<small>${note}</small>` : ''}</div>${control}</div>`;
  return header('Settings', '') + `<div class="page-body"><div class="form">
    <section><h2>General</h2><div class="group">
      ${row('Open at login', 'Starts the small helper that watches memory and drives.', toggle('login', st.login, 'Open at login'))}
      ${row('Show in menu bar', '', toggle('menubar', st.menubar, 'Show in menu bar'))}
      ${row('Check for updates automatically', 'Version 0.1.0', toggle('updates', st.updates, 'Check for updates') + button('Check now', 'noop'))}
    </div></section>
    <section><h2>Moving</h2><div class="group">
      ${row('Show what will happen before every move', 'Lists each step and the space it needs. Recommended.', toggle('dryRun', st.dryRun, 'Show a dry run first'))}
      ${row('Keep the original until every file is checked', 'Always on. Stowaway compares file count, size and content before removing anything.', '<span class="secondary">Always on</span>')}
      ${row('Never touched', 'Documents, Desktop, iCloud Drive, app sandboxes, Photos and Mail data.', '')}
    </div></section>
    <section><h2>Memory</h2><div class="group">
      ${row('Warn me when memory may run out within', '', `<select data-action="setting-select" data-id="warnAt">${[15, 30, 60].map((m) => `<option value="${m}" ${st.warnAt === m ? 'selected' : ''}>${m} minutes</option>`).join('')}</select>`)}
      ${row('Notifications', 'Allowed in System Settings.', pill('ok', 'Allowed'))}
    </div></section>
    <section><h2>Drives</h2><div class="group">
      ${row('Warn before I eject a drive that apps still need', 'Names the apps and offers to quit them first.', toggle('ejectGuard', st.ejectGuard, 'Eject warning'))}
      ${row('Journal', '~/Library/Application Support/Stowaway/journal.jsonl', button('Show in Finder', 'noop'))}
    </div></section>
    <section><h2>Privacy</h2><div class="group">${row('No account, no analytics', 'Stowaway only goes online to check for updates.', '')}</div></section>
  </div></div>`;
}

// ---------- Onboarding ----------
export function onboarding(s) {
  const steps = ['Welcome', 'Permissions', 'Choose a drive'];
  const aside = `<aside><div class="traffic" style="padding-left:0"><i></i><i></i><i></i></div>
    <div style="display:flex;gap:10px;align-items:center;margin-top:20px">${icon('cable', 'brand-mark')}<strong style="font:var(--type-headline)">Stowaway</strong></div>
    <ol>${steps.map((t, i) => `<li class="${i === s.onboardStep ? 'is-current' : i < s.onboardStep ? 'is-done' : ''}"><span class="n">${i < s.onboardStep ? '✓' : i + 1}</span>${t}</li>`).join('')}</ol>
    <p class="tertiary" style="margin-top:auto;font:var(--type-caption)">Takes about a minute. Nothing is moved until you choose.</p></aside>`;
  let main;
  if (s.onboardStep === 0) {
    const promise = (t, d) => `${icon('check')}<div><strong style="font:var(--type-emphasis)">${t}</strong><p>${d}</p></div>`;
    main = `<h1>Make room on your Mac.<br>Keep every app working.</h1>
      <p class="lead">Stowaway moves heavy app data, like AI models, build caches and virtual machines, to an external drive and leaves a link behind, so the apps never notice. It also tells you which app needs which drive and warns before memory runs out.</p>
      <div class="promise">${promise('Nothing is removed until the copy is checked', 'File count, size and content are compared first.')}${promise('Your documents are off limits', 'Documents, Desktop, iCloud Drive and app sandboxes are never touched.')}${promise('Every move can be undone', 'Bring anything back with one click, even after a crash.')}${promise('No account, no tracking, free', 'It only goes online to check for updates.')}</div>
      <div class="footer">${button('Continue', 'onboard-next', { kind: 'primary', large: true })}</div>`;
  } else if (s.onboardStep === 1) {
    const perm = (key, title, why) => `<div class="perm"><strong>${title}</strong>${s.granted[key] ? pill('ok', 'Allowed') : button('Allow…', 'grant', { id: key, kind: key === 'volumes' ? 'primary' : '' })}<p>${why}</p></div>`;
    main = `<h1>Two permissions</h1><p class="lead">macOS asks once for each, in Stowaway’s name. You can change them later in System Settings.</p>
      ${perm('volumes', 'Removable volumes', 'Needed to copy app data to your external drive and check it. Without it nothing can be moved.')}
      ${perm('notify', 'Notifications', 'Optional. Used only for memory warnings and when a drive an app needs is about to be ejected.')}
      <p class="secondary" style="font:var(--type-callout);max-width:600px">Stowaway doesn’t ask for Full Disk Access and never reads other apps’ sandboxes, so you won’t see a stream of “would like to access” prompts.</p>
      <div class="footer">${button('Back', 'onboard-back', { large: true })}${button('Continue', 'onboard-next', { kind: 'primary', large: true, disabled: !s.granted.volumes, title: s.granted.volumes ? '' : 'Allow removable volumes first' })}</div>`;
  } else {
    const choice = (id, name, detail, { refused = '', off = false } = {}) => `<label class="choice ${s.onboardDrive === id ? 'is-selected' : ''} ${refused ? 'is-refused' : ''}">
      <input type="radio" name="drive" data-action="pick-drive" data-id="${id}" ${s.onboardDrive === id ? 'checked' : ''} ${refused || off ? 'disabled' : ''}>
      <div><strong>${name}</strong><div class="${refused ? '' : 'secondary'}" style="font:var(--type-callout);${refused ? 'color:var(--danger-text)' : ''}">${refused || detail}</div></div>${refused ? pill('danger', 'Can’t use') : off ? pill('neutral', 'Not connected') : pill('ok', 'Ready')}</label>`;
    main = `<h1>Pick a drive for app data</h1><p class="lead">Use a drive that stays plugged in while you work. You can add more drives later.</p>
      ${choice('t7', 'Samsung T7', 'APFS · 412 GB free · USB-C')}
      ${choice('archive', 'Archive', 'APFS · last seen 2 days ago', { off: !s.connected.archive })}
      ${choice('stick', 'USB Stick', '', { refused: 'exFAT can’t hold links safely. Reformat it as APFS in Disk Utility to use it.' })}
      <div class="footer">${button('Back', 'onboard-back', { large: true })}${button('Start using Stowaway', 'onboard-done', { kind: 'primary', large: true })}</div>`;
  }
  return `<div class="onboard">${aside}<main>${main}</main></div>`;
}

// ---------- Menu-bar popover ----------
export function popover(s) {
  const ok = calm(s);
  const mini = (label, value, frac, color) => `<div><span>${label}</span><strong class="num">${value}</strong>${capacity([{ value: frac, color }], 1)}</div>`;
  return `<div class="menubar" role="menubar"><span class="item is-open" aria-label="Stowaway">${icon('cable')}<span class="num" style="font-size:12px">${ok ? '' : '38m'}</span></span><span class="item">Wi-Fi</span><span class="item">100%</span><span class="item">Mon 9 Nov 14:14</span></div>
  <div class="popover-wrap"><div class="popover" role="dialog" aria-label="Stowaway">
    <div style="display:flex;gap:8px;align-items:flex-start"><span class="dot ${ok ? 'ok' : 'warn'}" style="margin-top:5px"></span><div><p class="sentence">${ok ? 'Memory looks fine for the next few hours.' : 'Memory runs out in about 38 minutes.'}</p>
      <p class="secondary" style="font:var(--type-caption)">${ok ? 'Swap 8.1 GB and falling.' : 'Quitting Docker Desktop frees about 6.2 GB.'}</p></div></div>
    <div class="grid">${mini('CPU', s.live.cpu.at(-1) + '%', s.live.cpu.at(-1) / 100, 'var(--text-secondary)')}${mini('Memory', s.live.pressure + '%', s.live.pressure / 100, ok ? 'var(--ok-fill)' : 'var(--warn-fill)')}
      ${mini('Disk', gb(s.macFree) + ' free', 1 - s.macFree / s.macTotal, 'var(--danger-fill)')}${mini('Network', s.live.net.at(-1).toFixed(1) + ' MB/s', s.live.net.at(-1) / 6, 'var(--ok-fill)')}</div>
    <hr>
    ${s.drives.map((d) => `<div class="drive"><span class="dot ${s.connected[d.id] ? 'ok' : 'hollow'}"></span>${esc(d.name)}<span class="tertiary">${s.connected[d.id] ? `${stowedOn(s, d.id).length} apps rely on it` : `missing · ${stowedOn(s, d.id).map((i) => esc(i.app)).join(', ')} held`}</span></div>`).join('')}
    <hr>
    <div class="buttons">${ok ? '<span></span>' : button('Quit Docker Desktop', 'quit-app', { id: 'docker' })}${button('Open Stowaway', 'go', { id: 'overview', kind: 'primary' })}</div>
  </div></div>`;
}
