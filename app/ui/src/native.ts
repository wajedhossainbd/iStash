// Window and tray glue for the Tauri shell. Loaded only inside the app; holds no data (that is api.ts).
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { setTheme as setAppTheme } from '@tauri-apps/api/app';
import { TrayIcon } from '@tauri-apps/api/tray';
import { Window, getCurrentWindow } from '@tauri-apps/api/window';
import { LogicalSize } from '@tauri-apps/api/dpi';
import type { Theme } from './api';

export const setTheme = (t: Theme) => Promise.all([setAppTheme(t === 'system' ? null : t), invoke('appearance_checked', { theme: t })]);
export const openTarget = (target: string) => invoke('open_target', { target });
export const onAppearance = (fn: (t: Theme) => void) => listen<Theme>('appearance', (e) => fn(e.payload));

let title: string | null = null;
export async function setTrayTitle(t: string) {
  if (t === title) return;
  title = t;
  await (await TrayIcon.getById('main'))?.setTitle(t || null);
}

export async function showMain() {
  const w = await Window.getByLabel('main');
  await w?.show();
  await w?.setFocus();
  await getCurrentWindow().hide();
}

let height = 0;
export async function fitPopover(el: HTMLElement | null) {
  const h = el?.offsetHeight ?? 0;
  if (!h || h === height) return;
  height = h;
  await getCurrentWindow().setSize(new LogicalSize(340, h));
}
