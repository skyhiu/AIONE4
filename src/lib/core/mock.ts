// Mocked core for UI tests and `vite --mode mock`. Scenarios come from the URL:
//   ?migration=offer | none | handled | copy-fails
// Tests drive input through `window.__aioneMock` (see MockControl below).
import pkg from '../../../package.json';
import type {
  CopyProgress,
  CopyResult,
  Core,
  LibraryCounts,
  MigrationStatus,
  PadEvent,
  PadEventKind,
} from './types';

type Handler<T> = (value: T) => void;

export interface MockControl {
  pad(control: string, kind?: PadEventKind, pad?: number): void;
  decisions: boolean[];
  logs: string[];
  restarts: number;
  uiReadyCalls: number;
}

declare global {
  interface Window {
    __aioneMock?: MockControl;
  }
}

const SAMPLE_LIBRARY: LibraryCounts = {
  games: 10,
  hidden: 1,
  favorites: 3,
  ignored: [{ name: '_Removed', reason: 'ignored by name (starts with "_")' }],
};

export function createMockCore(search: string = globalThis.location?.search ?? ''): Core {
  const params = new URLSearchParams(search);
  const scenario = params.get('migration') ?? 'none';
  const padHandlers = new Set<Handler<PadEvent>>();
  const progressHandlers = new Set<Handler<CopyProgress>>();
  const doneHandlers = new Set<Handler<CopyResult>>();
  let seq = 0;
  let handled = scenario === 'handled';

  const control: MockControl = {
    pad(name, kind = 'down', pad = 0) {
      seq += 1;
      const ev: PadEvent = { kind, control: name, pad, seq };
      padHandlers.forEach((h) => h(ev));
    },
    decisions: [],
    logs: [],
    restarts: 0,
    uiReadyCalls: 0,
  };
  if (typeof window !== 'undefined') window.__aioneMock = control;

  const sub = <T>(set: Set<Handler<T>>, h: Handler<T>) => {
    set.add(h);
    return Promise.resolve(() => {
      set.delete(h);
    });
  };

  return {
    kind: 'mock',
    appInfo: () =>
      Promise.resolve({
        version: pkg.version,
        dataDir: 'C:\\Users\\Player\\AppData\\Local\\AIONE4',
        logsDir: 'C:\\Users\\Player\\AppData\\Local\\AIONE4\\Logs',
        platform: 'mock',
        pid: 4242,
      }),
    uiReady() {
      control.uiReadyCalls += 1;
      return Promise.resolve();
    },
    log(level, message) {
      control.logs.push(`${level}: ${message}`);
    },
    migrationStatus(): Promise<MigrationStatus> {
      if (handled) return Promise.resolve({ state: 'alreadyHandled', decision: 'copied' });
      if (scenario === 'offer' || scenario === 'copy-fails') {
        return Promise.resolve({
          state: 'offer',
          source: 'C:\\Users\\Player\\AppData\\Local\\AIONE',
          files: 1234,
          bytes: 4_512_000_000,
          library: SAMPLE_LIBRARY,
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
      // Simulated progress in four steps, then the result.
      const steps = [0.25, 0.5, 0.75, 1];
      steps.forEach((f, i) => {
        setTimeout(() => {
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
            doneHandlers.forEach((h) => h(result));
          }
        }, 150 * (i + 1));
      });
      return Promise.resolve();
    },
    restart() {
      control.restarts += 1;
      return Promise.resolve();
    },
    onPad: (h) => sub(padHandlers, h),
    onCopyProgress: (h) => sub(progressHandlers, h),
    onCopyDone: (h) => sub(doneHandlers, h),
  };
}
