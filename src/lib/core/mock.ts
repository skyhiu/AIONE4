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
//   &rename=0:gjpqy    renames game 0 (tests of titles with descenders, D-017)
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
  Settings,
} from './types';

type Handler<T> = (value: T) => void;

export interface MockControl {
  /** A press / release / repeat; connects the pad first if it is not connected (like XInput). */
  pad(control: string, kind?: PadEventKind, pad?: number): void;
  connect(pad?: number): void;
  disconnect(pad?: number): void;
  /** T2.3: a controller's battery word from the core ("low", "empty", "full", "wired", …). */
  battery(word: string, pad?: number): void;
  /** T2.3: a new PC status from the core (the header and the battery notice). */
  status(s: PcStatus): void;
  /** B-12: the right stick past its dead zone (-1000..1000, up positive); 0, 0 = let go. */
  pointer(x: number, y: number, pad?: number): void;
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
  /** T1.3: the games started (ids), in order. */
  launches: string[];
  /** T1.3: the running game ends (the core would bring AIONE back to the front). */
  gameExit(id: string): void;
  /** B-18: the close requests (ids), in order. */
  closes: string[];
  /** D-040: the returns to the running game (ids), in order. */
  returns: string[];
  /** B-22: what the core's held set holds now (for tests). */
  held(): string[];
  /** D-043: foreground attempts in flight now, and the most ever at once. */
  inFlight: number;
  maxInFlight: number;
  /** D-026: the running game's window appears (?window=never: only when this is called). */
  gameWindow(id: string): void;
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
  lastPlayed: null,
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
  g.font = 'bold 72px "M PLUS Rounded 1c"';
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
  // Tests: ?rename=0:gjpqy renames a game (D-017: titles with descenders).
  const rename = typeof location === 'undefined' ? null : new URLSearchParams(location.search).get('rename');
  if (rename) {
    const [i, ...name] = rename.split(':');
    const games = SAMPLE_GAMES.map((g, k) => (k === Number(i) ? { ...g, name: name.join(':') } : g));
    return { ...base, state: 'loaded', games, expected: 10 };
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

  const emit = (kind: PadEventKind, control: string, pad: number, pointer?: [number, number]) => {
    seq += 1;
    if (kind === 'connected' || kind === 'disconnected') {
      if (kind === 'connected') connected.add(pad);
      else connected.delete(pad);
      lastConnectionSeq = seq;
    }
    const ev: PadEvent = { kind, control, pad, seq, atMs: Date.now(), ...(pointer ? { pointer } : {}) };
    padHandlers.forEach((h) => h(ev));
  };

  // D-002 race: the pad was connected before anyone listened; the event goes nowhere.
  if (params.get('pad') === 'connected') emit('connected', '', 0);

  let running: string | null = null;
  /** T2.3: who hears the wake. */
  const resumedHandlers: Array<(e: { asleepS: number; fromAione: boolean }) => void> = [];
  /** D-047: the hint shown once. */
  let mediaHinted = false;
  /** B-26 rev. ax: the mock's display brightness. */
  let mockBrightness: number | null = null;
  /** B-26: the mock's Guide hold (as transit::GuideHold). */
  let guideAt = 0;
  let guideFired = false;
  /** B-24: the mock's settings store. */
  const settings: Settings = {
    schema: 1,
    volume: Number(params.get('volume') ?? 100),
    muted: params.get('muted') === '1',
    musicVolume: 100,
    menuSoundVolume: Number(params.get('menusound') ?? 100),
    menuSounds: true,
    sortOrder: 'recent',
    favoritesFirst: true,
    showHiddenGames: false,
    clock24h: false,
    startWithWindows: false,
    swapConfirmBack: false,
    bootUp: true,
    bootUpAudio: true,
    theme: 'switch',
    colorPreset: 'light',
    importedFromWpf: null,
  };
  /** D-046 rev. ar: when the current start began (the mock's windows are timed from it). */
  let launchAt = 0;
  /** B-22: the mirrored state machine, its held set and its limits (as transit.rs). */
  let lock: { kind: string; game: string | null; since: number } = { kind: 'none', game: null, since: Date.now() };
  const held = new Set<string>();
  const LIMITS: Record<string, number> = { 'boot-up': 10000, 'boot-down': 10000, launching: 46000, 'launcher-wait': 6000, returning: 3000, closing: 7000 };
  const expiredHandlers: ((e: { kind: string }) => void)[] = [];
  setInterval(() => {
    const lim = LIMITS[lock.kind];
    if (lim !== undefined && Date.now() - lock.since > lim) {
      const k = lock.kind;
      if (k === 'returning' && held.size) control.logs.push(`info: lock: ${k}: still held: ${[...held].join(', ')}; resuming anyway`);
      control.logs.push(`info: lock: ${k} ended: timed out after ${Date.now() - lock.since} ms`);
      lock = { kind: 'none', game: null, since: Date.now() };
      // As transit.rs's Kind::name (what the core sends).
      const NAMES: Record<string, string> = { 'boot-up': 'Boot Up', 'boot-down': 'Boot Down', launching: 'launching a game', 'launcher-wait': 'the launcher wait', returning: 'going back to the game', closing: 'closing the game' };
      expiredHandlers.forEach((h) => h({ kind: NAMES[k] ?? k }));
    }
  }, 200);
  /** D-037: whether the real game's window has appeared (after a launcher). */
  let gameShown = false;
  /** B-18 (rev. ah): the launcher wait was cancelled (the mock's later events are dropped). */
  let cancelled = false;
  /** D-046: the start B cancelled (a window appearing anyway is closed). */
  let startCancelled: string | null = null;
  const exitHandlers: ((e: { id: string; name: string; minutes: number }) => void)[] = [];
  const windowHandlers: ((e: { id: string; name: string; ms: number }) => void)[] = [];
  const hintHandlers: ((e: { id: string; text: string }) => void)[] = [];
  const followHandlers: ((e: { id: string; name: string }) => void)[] = [];
  const control: MockControl = {
    pad(name, kind = 'down', pad = 0) {
      if (!connected.has(pad)) emit('connected', '', pad);
      // B-22: the core's one held set sees every press and release (Guide included).
      if (kind === 'up') held.delete(`pad:${name}`);
      else held.add(`pad:${name}`);
      // B-26, as the core: Guide waits for its release (short) or 0.5 s (held: the Quick Menu).
      if (name === 'Guide') {
        if (kind === 'down') {
          guideFired = false;
          const at = Date.now();
          guideAt = at;
          setTimeout(() => {
            if (guideAt !== at || !held.has('pad:Guide')) return;
            guideFired = true;
            const blocked = ['launching', 'launcher-wait', 'returning', 'closing', 'boot-up', 'boot-down'].includes(lock.kind);
            control.logs.push(`info: Guide held 500 ms: ${blocked ? `ignored (${lock.kind})` : 'Quick Menu'}`);
            if (!blocked) emit('held', 'Guide', pad);
          }, 500);
        } else if (kind === 'up') {
          guideAt = 0;
          if (!guideFired) {
            emit('down', 'Guide', pad);
            emit('up', 'Guide', pad);
          }
        }
        return;
      }
      emit(kind, name, pad);
    },
    connect(pad = 0) {
      if (!connected.has(pad)) emit('connected', '', pad);
    },
    disconnect(pad = 0) {
      if (connected.has(pad)) emit('disconnected', '', pad);
    },
    battery(word, pad = 0) {
      if (!connected.has(pad)) emit('connected', '', pad);
      emit('battery', word, pad);
    },
    status(st) {
      statusHandlers.forEach((f) => f(st));
    },
    pointer(x, y, pad = 0) {
      if (!connected.has(pad)) emit('connected', '', pad);
      emit('pointer', 'RStick', pad, [x, y]);
    },

    launches: [],
    closes: [],
    returns: [],
    inFlight: 0,
    maxInFlight: 0,
    held: () => [...held],
    gameWindow(id) {
      // D-046: a cancelled start: the game appeared anyway: closed (as the core does).
      if (startCancelled === id) {
        startCancelled = null;
        running = null;
        control.logs.push(`info: start "${id}": cancelled (B); the game appeared anyway: closed`);
        return;
      }
      const g = SAMPLE_GAMES.find((x) => x.id === id);
      windowHandlers.forEach((h) => h({ id, name: g?.name ?? id, ms: 1200 }));
    },
    gameExit(id) {
      const g = SAMPLE_GAMES.find((x) => x.id === id);
      // As the core (take_if checks the generation): an old game's late end never clears the new one.
      if (running === id) running = null;
      exitHandlers.forEach((h) => h({ id, name: g?.name ?? id, minutes: 1 }));
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
        showCopyQuestion: params.get('copyquestion') === '1',
        extras: (params.get('extras') ?? '').split(',').filter(Boolean),
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
    // T1.3: ?launch=fail makes every start fail (a missing shortcut).
    launchGame(id) {
      launchAt = Date.now();
      // As the core: a cancellation belongs to one start (a new launch replaces it).
      startCancelled = null;
      const g = SAMPLE_GAMES.find((x) => x.id === id);
      control.launches.push(id);
      if (params.get('launch') === 'fail') return Promise.reject(new Error('its shortcut game.lnk is missing'));
      if (running === id) return Promise.resolve({ id, name: g?.name ?? id, how: 'already running', pid: 4242, alreadyRunning: true, foreground: 'OK via direct' });
      running = id;
      // D-026: the game's window appears shortly after (?window=<ms>), or never (?window=never).
      const w = params.get('window');
      // D-032: ?launcher=play: the launcher's window, then it ends ("following"), then the game's
      // window; ?launcher=noplay: the launcher ends and no game follows (then the end).
      const l = params.get('launcher');
      if (l) {
        setTimeout(() => control.gameWindow(id), 300);
        cancelled = false;
        gameShown = false;
        // ?followat=<ms>: when the launcher ends (default 1.3 s); the game 0.8 s later.
        const followAt = Number(params.get('followat') ?? 1300);
        setTimeout(() => followHandlers.forEach((h) => h({ id, name: g?.name ?? id })), followAt);
        if (l === 'play')
          setTimeout(() => {
            if (cancelled) return;
            gameShown = true;
            control.gameWindow(id);
          }, followAt + 800);
        else if (l === 'steam') {
          // D-048: Steam's window during the launcher wait (?showing=steam): the wait extends to the
          // start's limit; Continue → the game (?steamgame=1), or no game: the start ends at its
          // limit (?startlimit=ms) without AIONE taking the front from Steam's window.
          // As the core: Steam seen during the wait → the launch's limit replaces the 6 s wait's.
          setTimeout(() => {
            control.logs.push(`info: start "${g?.name ?? id}": Steam is showing a window during the start: waiting (up to the start's 45 s)`);
            if (lock.kind !== 'none') control.logs.push(`info: lock: ${lock.kind} ended: replaced`);
            lock = { kind: 'launching', game: id, since: Date.now() };
          }, followAt + 50);
          if (params.get('steamgame') === '1')
            setTimeout(() => {
              if (cancelled) return;
              gameShown = true;
              control.gameWindow(id);
            }, Number(params.get('showingms') ?? 10000) + 500);
          else
            setTimeout(() => {
              if (cancelled) return;
              control.logs.push(`info: "${g?.name ?? id}": the start ended while Steam is showing a window: AIONE stays behind (Home ready)`);
              control.gameExit(id);
            }, Number(params.get('startlimit') ?? 45000));
        } else setTimeout(() => !cancelled && control.gameExit(id), followAt + 5000);
      } else if (w !== 'never') setTimeout(() => control.gameWindow(id), Number(w ?? 300));
      return Promise.resolve({ id, name: g?.name ?? id, how: 'shortcut game.lnk', pid: 4242, alreadyRunning: false, foreground: null });
    },
    returnToGame(id) {
      if (running !== id) return Promise.reject(new Error("that game isn't running"));
      control.returns.push(id);
      // D-043: count foreground attempts in flight (the mashing test asserts never two at once).
      control.inFlight += 1;
      control.maxInFlight = Math.max(control.maxInFlight, control.inFlight);
      return new Promise((res) => setTimeout(() => {
        control.inFlight -= 1;
        res('OK via direct (before: aione4.exe)');
      }, 60));
    },
    runningGame() {
      return Promise.resolve(running);
    },
    // B-18: ?closewait=stuck: the game doesn't close (it may be asking to save).
    closeGame(id) {
      control.closes.push(id);
      if (running !== id) return Promise.reject(new Error("that game isn't running"));
      const g = SAMPLE_GAMES.find((x) => x.id === id)?.name ?? id;
      // B-27: ?closeadmin=1: Windows refuses (the game runs as administrator).
      if (params.has('closeadmin')) return new Promise((_, rej) => setTimeout(() => rej(new Error(`${g} runs as administrator: AIONE can't close it`)), 3200));
      // B-18 (rev. am): ?closewait=stuck: the game doesn't close itself; after about 3 s AIONE ends it.
      if (params.get('closewait') === 'stuck') {
        control.logs.push(`info: close "${g}": still running after 3 s: ended by AIONE (1 process(es))`);
        return new Promise((res) => setTimeout(() => {
          running = null;
          res();
        }, 3200));
      }
      // D-031: ?exitlate: "done" first, the game's end event 150 ms later (alpha.17 QA-1's order).
      if (params.has('exitlate')) {
        return new Promise((res) => setTimeout(() => {
          running = null;
          res();
          setTimeout(() => exitHandlers.forEach((h) => h({ id, name: g, minutes: 0.5 })), 150);
        }, 200));
      }
      return new Promise((res) => setTimeout(() => {
        control.gameExit(id);
        res();
      }, 200));
    },
    onGameFollowing(h) {
      followHandlers.push(h);
      return Promise.resolve(() => {});
    },
    // D-037 (rev. ak): a launcher (before the game appeared) is never handed the front.
    handBack(id) {
      if (running !== id) return Promise.reject(new Error("that game isn't running"));
      if (params.get('launcher') && !gameShown) return Promise.reject(new Error('not the game yet (a launcher): no hand-back'));
      return Promise.resolve(`window swap: "${id}": OK via direct`);
    },
    // B-22: the core's state machine, mirrored (the same limits as transit.rs).
    lockEnter(kind, game, why) {
      if (lock.kind !== 'none' && lock.kind !== kind) control.logs.push(`info: lock: ${lock.kind} ended: replaced`);
      if (lock.kind === kind && lock.game === game) return Promise.resolve();
      lock = { kind, game, since: Date.now() };
      control.logs.push(`info: lock: ${kind} started (${why})`);
      return Promise.resolve();
    },
    lockLeave(kind, how) {
      if (lock.kind !== kind || kind === 'none') return Promise.resolve(false);
      control.logs.push(`info: lock: ${kind} ended: ${how} after ${Date.now() - lock.since} ms`);
      lock = { kind: 'none', game: null, since: Date.now() };
      return Promise.resolve(true);
    },
    heldNow() {
      return Promise.resolve([...held]);
    },
    // B-24: the settings store (defaults; ?volume= / ?muted=1 / ?menusound= for tests).
    // T2.3: Sleep Mode: the mock "wakes" after ?sleepms (default 1500 ms).
    sleepPc() {
      control.logs.push('info: sleep: the Boot Down ended; asking Windows to sleep (AIONE stays)');
      setTimeout(() => {
        control.logs.push('info: Windows woke (about 2 s asleep); AIONE comes back (it slept from Sleep Mode)');
        resumedHandlers.forEach((h) => h({ asleepS: 2, fromAione: true }));
      }, Number(params.get('sleepms') ?? 1500));
      return Promise.resolve();
    },
    onResumed(handler) {
      resumedHandlers.push(handler);
      return Promise.resolve(() => {});
    },
    // D-047 (rev. bb), as the core: the hint once, when a media key comes while Guide is held.
    mediaKey(name) {
      control.logs.push(`info: media / volume key ${name}: it reached AIONE's page (not from AIONE)`);
      if (held.has('pad:Guide') && !mediaHinted) {
        mediaHinted = true;
        return Promise.resolve("Another program turns Guide + the stick into media keys (for example Steam Input's Guide Button Chord), which opens Windows' volume pop-up. Turn that chord off in its settings.");
      }
      return Promise.resolve(null);
    },
    // B-26 rev. ax: ?brightness=none: no display control; otherwise a built-in screen at N %.
    brightnessGet() {
      const b = params.get('brightness');
      if (b === 'none') return Promise.resolve({ method: 'none', value: null, reason: "Brightness can't be changed for this display" });
      return Promise.resolve({ method: 'builtin', value: mockBrightness ?? Number(b ?? 70), reason: null });
    },
    brightnessSet(value) {
      mockBrightness = value;
      control.logs.push(`info: brightness: ${value} % (builtin)`);
      return Promise.resolve();
    },
    powerPc(action) {
      control.logs.push(`info: power: ${action} the PC (confirmed by the user)`);
      return Promise.resolve();
    },
    getSettings() {
      return Promise.resolve({ ...settings });
    },
    changeSettings(patch) {
      Object.assign(settings, patch);
      control.logs.push(`info: settings changed: ${JSON.stringify(patch)}`);
      return Promise.resolve({ ...settings });
    },
    // D-046 rev. ar: ?showing=launcher|steam&showingms=N: that window is up for N ms from the start.
    onNotice() {
      return Promise.resolve(() => {});
    },
    startShowing(id) {
      // B-27: ?consent=N: Windows' permission prompt for N ms from the start.
      if (params.has('consent') && Date.now() - launchAt < Number(params.get('consent'))) return Promise.resolve('Windows is asking for permission: answer it with the mouse or keyboard');
      const what = params.get('showing');
      const ms = Number(params.get('showingms') ?? 10000);
      if (running !== id || !what || Date.now() - launchAt > ms) return Promise.resolve(null);
      return Promise.resolve(what === 'steam' ? 'Steam is showing a window' : 'the launcher is showing a window');
    },
    clearKeys() {
      for (const k of [...held]) if (k.startsWith('key:')) held.delete(k);
      return Promise.resolve();
    },
    noteKey(key, down) {
      if (down) held.add(`key:${key}`);
      else held.delete(`key:${key}`);
      return Promise.resolve();
    },
    onLockExpired(h) {
      expiredHandlers.push(h);
      return Promise.resolve(() => {});
    },
    msSinceGuide() {
      // The tests' focus events stand for Guide (just pressed); ?guidems=999999: no Guide (a swap).
      return Promise.resolve(Number(params.get('guidems') ?? 0));
    },
    cancelStart(id) {
      if (running !== id) return Promise.reject(new Error("that game isn't starting"));
      startCancelled = id;
      running = null;
      cancelled = true;
      // As the core does: its own line, logged once.
      const line = `start "${id}": the wait was cancelled (B): ended, nothing counted`;
      control.logs.push(`info: ${line}`);
      return Promise.resolve(line);
    },
    forceCloseGame(id) {
      control.closes.push(`force:${id}`);
      return new Promise((res) => setTimeout(() => {
        control.gameExit(id);
        res();
      }, 200));
    },
    onCloseHint(h) {
      hintHandlers.push(h);
      return Promise.resolve(() => {});
    },
    onGameWindow(h) {
      windowHandlers.push(h);
      return Promise.resolve(() => {});
    },
    onGameExited(h) {
      exitHandlers.push(h);
      return Promise.resolve(() => {});
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
