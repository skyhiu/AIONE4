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
//   &noicons=1         no game has an icon (B-8 fallback everywhere)
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
  tileArt: name === 'Quiet Engines' ? null : 'Thumbnail',
  tile: name === 'Quiet Engines' ? null : 'k1',
  // Two games without an icon show the B-8 letter tile.
  icon: name === 'Quiet Engines' || name === 'Moss Kingdom' ? null : 'i1',
  favorite: false,
  hidden: false,
  category: name.startsWith('Zephyr') ? 'Steam' : null,
  playCount: 0,
  playTimeMinutes: 0,
}));

const ICON_COLOURS = ['#3b6ea8', '#c0392b', '#2e8b57', '#8e44ad', '#d35400', '#16a085', '#7f8c8d', '#b8860b', '#2c3e50', '#c71585'];
const iconCache = new Map<number, string>();
/** An original placeholder icon (a coloured square with a white shape), drawn once. */
function mockIcon(i: number): string {
  const hit = iconCache.get(i);
  if (hit) return hit;
  const c = document.createElement('canvas');
  c.width = c.height = 128;
  const g = c.getContext('2d');
  if (!g) return '';
  g.fillStyle = ICON_COLOURS[i % ICON_COLOURS.length] ?? '#555';
  g.fillRect(0, 0, 128, 128);
  g.fillStyle = '#fff';
  g.beginPath();
  if (i % 3 === 0) g.arc(64, 64, 34, 0, Math.PI * 2);
  else if (i % 3 === 1) g.rect(32, 32, 64, 64);
  else {
    g.moveTo(64, 26);
    g.lineTo(104, 98);
    g.lineTo(24, 98);
  }
  g.fill();
  const url = c.toDataURL('image/png');
  iconCache.set(i, url);
  return url;
}

const bigCache = new Map<number, string>();
/** D-007 test: a 3000 × 3000 noisy JPEG (costly to decode), drawn once per picture. */
function bigArt(i: number): string {
  const k = i % 4;
  const hit = bigCache.get(k);
  if (hit) return hit;
  const c = document.createElement('canvas');
  c.width = c.height = 3000;
  const g = c.getContext('2d');
  if (!g) return '';
  const img = g.createImageData(3000, 3000);
  let seed = 1234 + k;
  for (let p = 0; p < img.data.length; p += 4) {
    seed = (seed * 1103515245 + 12345) & 0x7fffffff;
    img.data[p] = (seed >> 3) & 255;
    img.data[p + 1] = (seed >> 11) & 255;
    img.data[p + 2] = (k * 60 + (seed >> 19)) & 255;
    img.data[p + 3] = 255;
  }
  g.putImageData(img, 0, 0);
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
    libraryLoad: () => Promise.resolve(mockLibrary(params.get('library') ?? 'sample')),
    mediaUrl(gameId, slot) {
      const i0 = SAMPLE_GAMES.findIndex((g) => g.id === gameId);
      const i = i0 < 0 ? Number(gameId.replace(/\D/g, '')) : i0;
      if (slot === 'icon') return params.get('noicons') === '1' ? '' : mockIcon(i);
      if (noArt || (slot !== 'Thumbnail' && slot !== 'tile')) return '';
      if (params.get('bigart') === '1') return bigArt(i);
      return ART_URLS[i % ART_URLS.length] ?? '';
    },
    statusNow: () => Promise.resolve(mockStatus(params)),
    onStatus(h) {
      if (params.get('status-later') === '1')
        setTimeout(() => statusHandlers.forEach((f) => f({ network: 'offline', battery: { percent: 15, charging: false } })), 300);
      return sub(statusHandlers, h);
    },
    onPad: (h) => sub(padHandlers, h),
    onCopyProgress: (h) => sub(progressHandlers, h),
    onCopyDone: (h) => sub(doneHandlers, h),
  };
  return { core, control };
}
