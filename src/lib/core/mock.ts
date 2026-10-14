// Mocked core for UI tests and `vite --mode mock`. It behaves like the real core (A10):
// a pad connects before it sends presses, and a pad can already be connected before the UI
// starts listening (D-002), in which case the UI only learns about it by asking (padSlots).
//
// Scenarios come from the URL:
//   ?migration=offer | none | handled | copy-fails
//   &pad=connected     a pad is in slot 1 before the UI subscribes (its connect event is lost)
//   &step=<ms>         time between copy progress steps (default 150)
//   &record=alpha1 | alpha2 | unreadable   a stored copy record (B-6 fixtures from
//                      tests/fixtures/migration/), read with the core's rule: a readable record
//                      with a decision other than "failed" means "already handled"
//   &copy=finished     Imported\\AIONE-WPF exists (used with record=unreadable)
//   &library=sample | empty | error | many   the games (sample = 10 like the user's copy)
//   &scale=90..150     Interface Scale for this run (like --interface-scale)
//   &network=wifi | wired | offline   &battery=none | <percent> | <percent>c (charging)
//   &status-later=1    the core pushes a status change 300 ms after start
//   &noart=1           no tile art (plain tiles)
//   &bigart=1          tile pictures of 3000 × 3000 px (D-007: as if the core served full-size files)
//   &tiles=pending     no tile copies yet: Home shows the originals; the copies arrive when the
//                      test calls releaseTiles()
//   &boot=default | custom | custom-broken | default-switch   the Boot Up / Boot Down (T1.2):
//                      without it the mock acts as ShowSplash = false (no presentation), so the
//                      other tests reach Home at once. custom = the WebM test clips;
//                      custom-broken = a custom video that cannot load (fallback to Window B);
//                      default-switch = custom videos exist but --boot=default is set.
// Tests drive input through `window.__aioneMock` (see MockControl below).
import pkg from '../../../package.json';
import alpha1Record from '../../../tests/fixtures/migration/alpha1.migration.json?raw';
import alpha2Record from '../../../tests/fixtures/migration/alpha2.migration.json?raw';
import type {
  GameView,
  LibraryLoad,
  PcStatus,
  CopyProgress,
  CopyResult,
  Core,
  LibraryCounts,
  MigrationStatus,
  PadEvent,
  PadEventKind,
  PadSlots,
} from './types';

type Handler<T> = (value: T) => void;

export interface MockControl {
  /** A press / release / repeat; connects the pad first if it is not connected (like XInput). */
  pad(control: string, kind?: PadEventKind, pad?: number): void;
  connect(pad?: number): void;
  disconnect(pad?: number): void;
  decisions: boolean[];
  logs: string[];
  /** performance.now() of each log line (same index as logs). */
  logTimes: number[];
  restarts: number;
  uiReadyCalls: number;
  padSlotsCalls: number;
  /** Like Alt+F4: the core asks the UI to close (A9: the Boot Down plays first). */
  requestClose(): void;
  /** tiles=pending: the background copies arrive now. */
  releaseTiles(): void;
  /** Reasons passed to `closeNow` (the app would exit). */
  closed: string[];
}

declare global {
  interface Window {
    __aioneMock?: MockControl;
  }
}

export const SAMPLE_LIBRARY: LibraryCounts = {
  games: 10,
  hidden: 0,
  favorites: 0,
  folders: 0,
  ignored: [{ name: '_Removed', reason: 'skipped by name (starts with "_")' }],
  sources: [
    'hidden: none found, looked in: game.json fields (Hidden, IsHidden) in 10 games; top-level .json lists (no .json files at the top level); not yet read: Config\\',
  ],
};

/** Original placeholder art (tests/fixtures/library/art), never real game art. */
const BOOT_CLIPS = import.meta.glob('../../../tests/fixtures/boot/*.webm', {
  eager: true,
  query: '?url',
  import: 'default',
}) as Record<string, string>;
const clip = (name: string) => Object.entries(BOOT_CLIPS).find(([k]) => k.endsWith(name))?.[1] ?? '';

const ART = import.meta.glob('../../../tests/fixtures/library/art/*.png', {
  eager: true,
  query: '?url',
  import: 'default',
}) as Record<string, string>;
const ART_URLS = Object.keys(ART)
  .sort()
  .map((k) => ART[k] ?? '');

/** Like the user's copy: 10 games, two with their real names (the attached game.json files),
 *  one without art, one with a long title (marquee). */
export const SAMPLE_GAMES: GameView[] = [
  'Amber Lighthouse',
  'Glass Orchard',
  'Lantern Valley',
  'Moss Kingdom',
  'Northbound',
  'Paper Comets',
  'Quiet Engines',
  'The Long Corridor Between Two Very Distant Stations: The Complete Night Edition with Every Bonus Chapter and Director Commentary',
  'Tide & Ember',
  'Zephyr Line Express',
].map((name) => ({
  id: name,
  name,
  // B-13: the icon fills the tile; Moss Kingdom has no icon file (a wide picture, fitted whole);
  // Quiet Engines has no picture at all (a plain tile).
  tileArt: name === 'Quiet Engines' ? null : name === 'Moss Kingdom' ? 'Thumbnail' : 'Icon',
  tileMode: name === 'Quiet Engines' ? null : name === 'Moss Kingdom' ? 'fit' : 'fill',
  tile: name === 'Quiet Engines' ? null : 'k1',
  favorite: false,
  hidden: false,
  category: name.startsWith('Zephyr') ? 'Steam' : null,
  playCount: 0,
  playTimeMinutes: 0,
}));

/** An original wide placeholder picture (920 × 430, like a game's Thumbnail). */
const wideCache = new Map<number, string>();
function widePicture(i: number): string {
  const hit = wideCache.get(i);
  if (hit) return hit;
  const c = document.createElement('canvas');
  c.width = 920;
  c.height = 430;
  const g = c.getContext('2d');
  if (!g) return '';
  g.fillStyle = '#2f6b4f';
  g.fillRect(0, 0, 920, 430);
  g.fillStyle = '#f2d16b';
  g.fillRect(0, 0, 40, 430);
  g.fillRect(880, 0, 40, 430);
  g.fillStyle = '#ffffff';
  g.font = 'bold 72px sans-serif';
  g.fillText('WIDE PICTURE', 200, 240);
  const url = c.toDataURL('image/png');
  wideCache.set(i, url);
  return url;
}

/** What the core makes for a wide picture (B-13): fitted whole over a blurred, darkened copy. */
const fittedCache = new Map<number, string>();
function fittedTile(i: number): string {
  const hit = fittedCache.get(i);
  if (hit) return hit;
  const c = document.createElement('canvas');
  c.width = c.height = 640;
  const g = c.getContext('2d');
  const img = new Image();
  img.src = widePicture(i);
  if (!g) return '';
  // Drawn synchronously from a data URL already decoded by the browser cache in tests;
  // if not ready yet, a dark tile with the bands only.
  g.fillStyle = '#16301f';
  g.fillRect(0, 0, 640, 640);
  if (img.complete && img.naturalWidth) {
    g.filter = 'blur(24px) brightness(0.45)';
    g.drawImage(img, -400, 0, 1440, 640);
    g.filter = 'none';
    const h = (640 * 430) / 920;
    g.drawImage(img, 0, (640 - h) / 2, 640, h);
  }
  const url = c.toDataURL('image/jpeg', 0.9);
  if (img.complete && img.naturalWidth) fittedCache.set(i, url);
  return url;
}

const bigCache = new Map<number, string>();
/** D-007 test: a 3000 × 3000 noisy JPEG (costly to decode), drawn once per picture. */
function bigArt(i: number): string {
  const k = i % 3; // variant 3 of the earlier set could not be fully decoded by Chromium
  const hit = bigCache.get(k);
  if (hit) return hit;
  const c = document.createElement('canvas');
  c.width = c.height = 3000;
  const g = c.getContext('2d');
  if (!g) return '';
  // Random 12 px blocks: still a full 3000 × 3000 decode, but a JPEG small enough to be a valid
  // data: URL (alpha.6: per-pixel noise made one picture too large for Chromium to decode).
  let seed = 1234 + k;
  for (let y = 0; y < 3000; y += 12)
    for (let x = 0; x < 3000; x += 12) {
      seed = (seed * 1103515245 + 12345) & 0x7fffffff;
      g.fillStyle = `rgb(${(seed >> 3) & 255},${(seed >> 11) & 255},${(k * 60 + (seed >> 19)) & 255})`;
      g.fillRect(x, y, 12, 12);
    }
  const url = c.toDataURL('image/jpeg', 0.92);
  bigCache.set(k, url);
  return url;
}

function mockLibrary(kind: string): LibraryLoad {
  const base = { source: 'C:\\Users\\Player\\AppData\\Local\\AIONE4\\Games', skipped: [], notes: [] };
  if (kind === 'empty') return { ...base, state: 'empty', games: [], expected: null };
  if (kind === 'error')
    return {
      ...base,
      state: 'error',
      games: [],
      notes: ['the conversion of your copied games failed: copying X: access denied'],
      expected: 10,
    };
  if (kind === 'many') {
    const games = Array.from({ length: 24 }, (_, i) => ({ ...(SAMPLE_GAMES[i % 10] as GameView), id: `g${i}`, name: `Game ${i + 1}` }));
    return { ...base, state: 'loaded', games, expected: 24 };
  }
  return { ...base, state: 'loaded', games: SAMPLE_GAMES, expected: 10 };
}

function mockStatus(params: URLSearchParams): PcStatus {
  const network = (params.get('network') ?? 'wifi') as PcStatus['network'];
  const b = params.get('battery') ?? 'none';
  if (b === 'none') return { network, battery: null };
  return { network, battery: { percent: Number.parseInt(b, 10), charging: b.endsWith('c') } };
}

export function createMockCore(search: string = globalThis.location?.search ?? ''): Core {
  return createMock(search).core;
}

export function createMock(search: string): { core: Core; control: MockControl } {
  const params = new URLSearchParams(search);
  const scenario = params.get('migration') ?? 'none';
  const stepMs = Number(params.get('step') ?? '150');
  const padHandlers = new Set<Handler<PadEvent>>();
  const statusHandlers = new Set<Handler<PcStatus>>();
  const noArt = params.get('noart') === '1';
  const tileHandlers = new Set<Handler<{ id: string; tile: string }>>();
  const closeHandlers = new Set<() => void>();
  const closed: string[] = [];
  const progressHandlers = new Set<Handler<CopyProgress>>();
  const doneHandlers = new Set<Handler<CopyResult>>();
  let seq = 0;
  let lastConnectionSeq = 0;
  const connected = new Set<number>();
  let handled = scenario === 'handled';
  const records: Record<string, string> = { alpha1: alpha1Record, alpha2: alpha2Record, unreadable: '{ not json' };
  const recordText = records[params.get('record') ?? ''];
  const finishedCopy = params.get('copy') === 'finished';
  let notice: string | null = null;
  let recordDecision: string | null = null;
  if (recordText !== undefined) {
    try {
      const parsed = JSON.parse(recordText) as { decision?: unknown };
      if (typeof parsed.decision !== 'string') throw new Error('missing field decision');
      recordDecision = parsed.decision;
    } catch {
      notice = finishedCopy
        ? `AIONE 4 could not read its note of your earlier copy answer. Your copied data is complete, so nothing changes and you are not asked again. The note was kept as C:\\Users\\Player\\AppData\\Local\\AIONE4\\migration.json.unreadable-20261013-120000 (details in aione.log).`
        : `AIONE 4 could not read its note of your earlier answer to this question, and no finished copy exists, so it asks again. The note was kept as C:\\Users\\Player\\AppData\\Local\\AIONE4\\migration.json.unreadable-20261013-120000 (details in aione.log).`;
      if (finishedCopy) recordDecision = 'copied';
    }
  }

  const emit = (kind: PadEventKind, control: string, pad: number) => {
    seq += 1;
    if (kind === 'connected' || kind === 'disconnected') {
      if (kind === 'connected') connected.add(pad);
      else connected.delete(pad);
      lastConnectionSeq = seq;
    }
    const ev: PadEvent = { kind, control, pad, seq, atMs: Date.now() };
    padHandlers.forEach((h) => h(ev));
  };

  // D-002 race: the pad was connected before anyone listened; the event goes nowhere.
  if (params.get('pad') === 'connected') emit('connected', '', 0);

  const control: MockControl = {
    pad(name, kind = 'down', pad = 0) {
      if (!connected.has(pad)) emit('connected', '', pad);
      emit(kind, name, pad);
    },
    connect(pad = 0) {
      if (!connected.has(pad)) emit('connected', '', pad);
    },
    disconnect(pad = 0) {
      if (connected.has(pad)) emit('disconnected', '', pad);
    },
    decisions: [],
    logs: [],
    logTimes: [],
    restarts: 0,
    uiReadyCalls: 0,
    padSlotsCalls: 0,
    requestClose: () => closeHandlers.forEach((f) => f()),
    releaseTiles: () =>
      SAMPLE_GAMES.filter((g) => g.tileArt).forEach((g) => tileHandlers.forEach((f) => f({ id: g.id, tile: 'k1' }))),
    closed,
  };
  if (typeof window !== 'undefined') window.__aioneMock = control;

  const sub = <T>(set: Set<Handler<T>>, h: Handler<T>) => {
    set.add(h);
    return Promise.resolve(() => {
      set.delete(h);
    });
  };

  const core: Core = {
    kind: 'mock',
    appInfo: () =>
      Promise.resolve({
        version: pkg.version,
        dataDir: 'C:\\Users\\Player\\AppData\\Local\\AIONE4',
        logsDir: 'C:\\Users\\Player\\AppData\\Local\\AIONE4\\Logs',
        platform: 'mock',
        pid: 4242,
        interfaceScale: Number(params.get('scale') ?? '100'),
        fromAutostart: false,
        bootDefault: params.get('boot') === 'default-switch',
      }),
    uiReady() {
      control.uiReadyCalls += 1;
      return Promise.resolve();
    },
    log(level, message) {
      control.logs.push(`${level}: ${message}`);
      control.logTimes.push(typeof performance === 'undefined' ? 0 : performance.now());
    },
    padSlots(): Promise<PadSlots> {
      control.padSlotsCalls += 1;
      return Promise.resolve({ slots: [...connected].sort((a, b) => a - b), seq: lastConnectionSeq });
    },
    migrationStatus(): Promise<MigrationStatus> {
      if (recordDecision !== null && recordDecision !== 'failed') {
        return Promise.resolve({ state: 'alreadyHandled', decision: recordDecision, notice });
      }
      if (handled) return Promise.resolve({ state: 'alreadyHandled', decision: 'copied', notice: null });
      if (scenario === 'offer' || scenario === 'copy-fails' || notice !== null) {
        return Promise.resolve({
          state: 'offer',
          source: 'C:\\Users\\Player\\AppData\\Local\\AIONE',
          files: 1234,
          bytes: 4_512_000_000,
          library: SAMPLE_LIBRARY,
          notice,
        });
      }
      return Promise.resolve({ state: 'noSource', source: 'C:\\Users\\Player\\AppData\\Local\\AIONE' });
    },
    migrationDecide(accept) {
      control.decisions.push(accept);
      handled = true;
      if (!accept) return Promise.resolve();
      const total = 1234;
      const bytesTotal = 4_512_000_000;
      // Progress in four steps (bytes and files move together here), then the result.
      const steps = [0.25, 0.5, 0.75, 1];
      steps.forEach((f, i) => {
        setTimeout(
          () => {
            progressHandlers.forEach((h) =>
              h({
                filesDone: Math.round(total * f),
                filesTotal: total,
                bytesDone: Math.round(bytesTotal * f),
                bytesTotal,
              }),
            );
            if (i === steps.length - 1) {
              const fails = scenario === 'copy-fails';
              const result: CopyResult = {
                ok: !fails,
                outcome: fails ? 'partial' : 'copied',
                destination: 'C:\\Users\\Player\\AppData\\Local\\AIONE4\\Imported\\AIONE-WPF',
                filesCopied: fails ? total - 2 : total,
                filesFailed: fails ? 2 : 0,
                bytesCopied: bytesTotal,
                library: SAMPLE_LIBRARY,
                sourceUnchanged: true,
                notes: fails ? ['2 files could not be read (in use by another program).'] : [],
              };
              setTimeout(() => doneHandlers.forEach((h) => h(result)), stepMs);
            }
          },
          stepMs * (i + 1),
        );
      });
      return Promise.resolve();
    },
    restart() {
      control.restarts += 1;
      return Promise.resolve();
    },
    libraryLoad: async () => {
      // The wide picture is decoded first, so the fitted tile can be drawn from it.
      await new Promise<void>((resolve) => {
        const im = new Image();
        im.onload = () => resolve();
        im.onerror = () => resolve();
        im.src = widePicture(3);
      });
      const l = mockLibrary(params.get('library') ?? 'sample');
      if (params.get('tiles') === 'pending') l.games = l.games.map((g) => ({ ...g, tile: null }));
      return l;
    },
    mediaUrl(gameId, slot) {
      const i0 = SAMPLE_GAMES.findIndex((g) => g.id === gameId);
      const i = i0 < 0 ? Number(gameId.replace(/\D/g, '')) : i0;
      if (noArt) return '';
      if (params.get('bigart') === '1' && (slot === 'tile' || slot === 'Icon')) return bigArt(i);
      const game = SAMPLE_GAMES[i0];
      // The fitted game: the original is a wide picture; the core's copy is composed (B-13).
      if (game?.tileMode === 'fit') return slot === 'tile' ? fittedTile(i) : widePicture(i);
      if (slot === 'tile' || slot === 'Icon') return ART_URLS[i % ART_URLS.length] ?? '';
      return '';
    },
    statusNow: () => Promise.resolve(mockStatus(params)),
    onStatus(h) {
      if (params.get('status-later') === '1')
        setTimeout(() => statusHandlers.forEach((f) => f({ network: 'offline', battery: { percent: 15, charging: false } })), 300);
      return sub(statusHandlers, h);
    },
    bootMedia() {
      const mode = params.get('boot');
      const custom = mode === 'custom' || mode === 'custom-broken' || mode === 'default-switch';
      const file = (source: string) => ({ key: 'm1', ext: 'webm', bytes: 1, source });
      return Promise.resolve({
        up: custom ? file('splash.mp4') : null,
        down: custom ? file('suspend.mp4') : null,
        settings: { showSplash: mode !== null, splashAudio: true, playToEnd: true, durationMs: 3200, from: 'WPF settings' },
        notes: [custom ? 'custom Boot Up: splash.mp4 (mock), AIONE 4\'s copy reused' : 'custom Boot Up: none (mock)'],
      });
    },
    bootUrl(which) {
      if (params.get('boot') === 'custom-broken') return `./missing-${which}.webm`;
      return clip(which === 'up' ? 'mock-boot-up.webm' : 'mock-boot-down.webm');
    },
    onCloseRequested(h) {
      closeHandlers.add(h);
      return Promise.resolve(() => closeHandlers.delete(h));
    },
    closeNow(reason) {
      closed.push(reason);
      return Promise.resolve();
    },
    onTile(h) {
      // tiles=pending: the copies arrive when the test calls releaseTiles() (deterministic).
      return sub(tileHandlers, h);
    },
    onPad: (h) => sub(padHandlers, h),
    onCopyProgress: (h) => sub(progressHandlers, h),
    onCopyDone: (h) => sub(doneHandlers, h),
  };
  return { core, control };
}
