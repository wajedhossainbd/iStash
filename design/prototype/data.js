// Mock data for the prototype. Shapes mirror what the Rust core should hand the UI (see design/system.md, "Data the UI needs").

export const DRIVES = [
  { id: 't7', name: 'Samsung T7', kind: 'APFS · USB-C', total: 1000, free: 412, cable: 'blue', lastSeen: 'now' },
  { id: 'archive', name: 'Archive', kind: 'APFS · USB', total: 2000, free: 1210, cable: 'orange', lastSeen: '2 days ago' },
];

// verdict: move | setting | refused | stowed. `app` is who owns the data; `appId` picks the icon file.
export const ITEMS = [
  { id: 'ollama', name: 'Models', app: 'Ollama', appId: 'ollama', path: '~/.ollama/models', size: 48.2, verdict: 'move', running: true,
    reason: 'Ollama is open' },
  { id: 'derived', name: 'Derived data', app: 'Xcode', appId: 'xcode', path: '~/Library/Developer/Xcode/DerivedData', size: 21.7, verdict: 'move',
    reason: 'Rebuilt if the drive is away' },
  { id: 'gradle', name: 'Gradle caches', app: 'Gradle', appId: 'gradle', path: '~/.gradle/caches', size: 7.3, verdict: 'move', reason: 'Cache, safe to move' },
  { id: 'docker', name: 'Disk image', app: 'Docker Desktop', appId: 'docker', path: '~/Library/Containers/com.docker.docker/Data', size: 59.6, verdict: 'setting',
    reason: 'A link here can corrupt data',
    how: ['Quit nothing yet. Open Docker Desktop.', 'Go to Settings, then Resources, then Advanced.', 'Set "Disk image location" to a folder on your drive and press Apply.', 'Docker moves the image itself and keeps working.'] },
  { id: 'mobilesync', name: 'iPhone backups', app: 'Finder', appId: 'finder', path: '~/Library/Application Support/MobileSync/Backup', size: 28.4, verdict: 'refused',
    reason: 'macOS ignores links here',
    why: 'Since macOS 10.15, Finder no longer follows a link for iPhone backups. It would quietly start a new backup on this Mac. Move old backups by hand from Finder (Manage Backups) instead.' },
  { id: 'hf', name: 'Cache', app: 'Hugging Face', appId: 'huggingface', path: '~/.cache/huggingface', size: 33.1, verdict: 'stowed', drive: 't7', since: '3 Nov' },
  { id: 'devsupport', name: 'iOS Device Support', app: 'Xcode', appId: 'xcode', path: '~/Library/Developer/Xcode/iOS DeviceSupport', size: 18.6, verdict: 'stowed', drive: 't7', since: '28 Oct' },
  { id: 'cowork', name: 'Cowork VM', app: 'Claude', appId: 'claude', path: '~/Library/Application Support/Claude/vm', size: 12.4, verdict: 'stowed', drive: 't7', since: '3 Nov', running: true },
  { id: 'hermes', name: 'Data', app: 'hermes', appId: 'hermes', path: '~/.hermes', size: 3.8, verdict: 'stowed', drive: 't7', since: '1 Nov' },
  { id: 'lmstudio', name: 'Models', app: 'LM Studio', appId: 'lmstudio', path: '~/.lmstudio/models', size: 61.0, verdict: 'stowed', drive: 'archive', since: '19 Oct' },
];

export const MEMORY_APPS = [
  { id: 'docker', app: 'Docker Desktop', appId: 'docker', mem: 6.2, detail: '3 containers running' },
  { id: 'chrome', app: 'Google Chrome', appId: 'chrome', mem: 3.1, detail: '41 tabs' },
  { id: 'xcode', app: 'Xcode', appId: 'xcode', mem: 2.4, detail: 'Indexing' },
  { id: 'claude', app: 'Claude', appId: 'claude', mem: 1.6, detail: 'Cowork VM running' },
  { id: 'ollama', app: 'Ollama', appId: 'ollama', mem: 0.9, detail: 'No model loaded' },
];

// Swap in GB, one sample per minute for the last 55 minutes.
export const SWAP_HISTORY = Array.from({ length: 56 }, (_, i) => 3.1 + 6.3 * (i / 55) ** 2.2);

export function initialState() {
  return {
    route: 'overview',
    view: 'normal',               // normal | empty | loading | error (prototype control)
    theme: 'auto',
    connected: { t7: true, archive: false },
    macTotal: 494, macFree: 31.2,
    drives: structuredClone(DRIVES),
    items: structuredClone(ITEMS),
    memApps: structuredClone(MEMORY_APPS),
    offloadTab: 'suggested',
    dialog: null,
    jobs: {},                     // itemId -> { kind: 'move' | 'back', step, pct, drive }
    onboardStep: 0, granted: { volumes: false, notify: false }, onboardDrive: 't7',
    settings: { login: true, menubar: true, dryRun: true, warnAt: 30, ejectGuard: true, updates: true },
    live: { cpu: [22, 25, 31, 28, 34, 30, 36, 33, 29, 34], net: [1.2, 2.4, 0.8, 3.9, 4.2, 2.2, 1.1, 5.0, 3.3, 4.2], pressure: 72 },
  };
}
