// Run with `npm test` (node's own runner, no extra dependencies).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { gb, list, ago } from './ui.ts';
import { createMock } from './api.ts';

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
