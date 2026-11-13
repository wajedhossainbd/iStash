// Run with `npm test` (node's own runner, no extra dependencies).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { gb, list, ago } from './ui.ts';
import { createMock, toDrives, type Volume } from './api.ts';

test('formatting', () => {
  assert.equal(gb(31.2e9), '31.2 GB');
  assert.equal(gb(412e9), '412 GB');
  assert.equal(list(['Claude', 'Xcode', 'hermes']), 'Claude, Xcode and hermes');
  assert.equal(ago(Date.now() - 2 * 864e5), '2 days ago');
});

test('mock dry runs refuse what the engine would refuse', async () => {
  const api = createMock('normal', true);
  const missing = await api.planBack('lmstudio');
  assert.equal(missing.ok, false);
  api.setConnected('archive', true);
  const full = await api.planBack('lmstudio'); // 61 GB back onto a Mac with 31.2 GB free
  assert.ok(!full.ok && full.title === 'Not enough room on this Mac');
  const move = await api.planMove('gradle', 't7');
  assert.ok(move.ok && Math.round(move.macFreeAfter / 1e8) === 385);
  assert.equal((await api.planMove('gradle', 'stick')).ok, false, 'exFAT is refused');
});

test('quitting the suggested app clears the forecast', async () => {
  const api = createMock('normal', true);
  await api.quitApp('docker');
  assert.equal((await api.state()).forecast?.condition, 'fine');
});

test('mock forecast agrees with its own swap curve', async () => {
  const api = createMock('normal', true);
  const f = (await api.state()).forecast!;
  assert.equal(f.swap, f.history.at(-1));
  assert.equal(f.minutesLeft, 36);
  assert.ok(f.swapRate > 0 && f.swap < f.swapLimit);
  await api.move('gradle', 't7');
  await assert.rejects(api.eject('t7'), /in use/);
});

test('real volumes become drives: exFAT refused, cables kept, unplugged drives drop unless they hold data', () => {
  const v = (id: string, name: string, format = 'APFS'): Volume => ({ id, name, format, bus: 'USB', total: 1e12, free: 5e11, writable: true });
  const a = toDrives([v('b', 'Mac 500 SSD'), v('a', 'Jetdrive'), v('c', 'Storejet', 'ExFAT')], [], () => false, 1);
  assert.deepEqual(a.map((d) => [d.name, d.cable, !!d.refused]), [['Jetdrive', 'blue', false], ['Mac 500 SSD', 'orange', false], ['Storejet', 'violet', true]]);
  const b = toDrives([v('b', 'Mac 500 SSD')], a, (id) => id === 'a', 2);
  assert.deepEqual(b.map((d) => [d.name, d.cable, d.connected, d.lastSeen]), [['Mac 500 SSD', 'orange', true, 1], ['Jetdrive', 'blue', false, 2]]);
});
