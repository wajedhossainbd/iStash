// The notch island: a black pill that grows out of the top of the screen with a neon edge, shows one of iStash's
// notifications, then shrinks back and hides its window. Messages that arrive meanwhile wait their turn.
import '@fontsource-variable/geist/wght.css';
import './island.css';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Window } from '@tauri-apps/api/window';
import { esc, icon } from './ui';

type Note = { title: string; body: string; kind: string; glow: string; secs: number };
const ICONS: Record<string, string> = { unplug: 'eject', plug: 'plug', memory: 'memory', update: 'down', clean: 'clean', info: 'info' };
const root = document.getElementById('root')!;
root.innerHTML = '<button class="island" aria-live="polite"><span class="island-icon"></span><span class="island-text"><b></b><small></small></span></button>';
const pill = root.firstElementChild as HTMLButtonElement;
const queue: Note[] = [];
let busy = false;
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

async function play(n: Note) {
  busy = true;
  pill.dataset.glow = n.glow || 'amber';
  pill.querySelector('.island-icon')!.innerHTML = icon(ICONS[n.kind] ?? 'info');
  pill.querySelector('b')!.innerHTML = esc(n.title);
  pill.querySelector('small')!.innerHTML = esc(n.body);
  pill.title = `${n.title}\n${n.body}`;
  pill.classList.add('shown');
  await wait(60);
  pill.classList.add('open');
  await wait(Math.max(2, n.secs || 5) * 1000);
  pill.classList.remove('open');
  await wait(520);
  pill.classList.remove('shown');
  await wait(260);
  const next = queue.shift();
  if (next) return play(next);
  busy = false;
  invoke('island_hide').catch(() => {});
}

listen<Note>('island', (e) => { if (busy) queue.push(e.payload); else play(e.payload); });
pill.addEventListener('click', async () => {
  const w = await Window.getByLabel('main');
  await w?.show();
  await w?.setFocus();
});
