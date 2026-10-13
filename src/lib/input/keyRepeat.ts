// D-006 / B-3 (rev. e): AIONE times key repeats itself, like the controller (400 ms, then every
// 90 ms), and ignores the browser's / Windows' own repeated keydown events, so Windows' keyboard
// settings never change the speed.
//
// **Which keys repeat:** only navigation keys (`REPEATING_KEYS`): the four arrows. Tab and
// Page Up / Page Down join the list when AIONE gives them a use (none on Home in T1.1).
// Enter, Space (confirm), Esc (back), modifiers and every other key never repeat. Letter keys and
// Backspace are decided in T6 with the on-screen keyboard.
//
// **Releases** carry how long the key was held and how many repeats it gave, for the log line
// `input up <key> (keyboard), held N ms, R repeats` (alpha.3 QA-2).
//
// Only the most recently pressed navigation key repeats (as on a keyboard); releasing it stops
// its repeats. Timers: one setTimeout chain while a navigation key is held (owner: keyboard
// repeat); none when idle.

export const KEY_REPEAT_DELAY_MS = 400;
export const KEY_REPEAT_INTERVAL_MS = 90;

/** KeyboardEvent.key values that repeat while held. */
export const REPEATING_KEYS: ReadonlySet<string> = new Set(['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight']);

export function repeats(key: string): boolean {
  return REPEATING_KEYS.has(key);
}

export type KeyPhase = 'down' | 'repeat' | 'up';

export interface KeyLike {
  key: string;
  code: string;
}

/** For 'up': how long the key was held and how many repeats it gave. */
export interface ReleaseInfo {
  heldMs: number;
  repeats: number;
}

type Emit = (e: KeyLike, phase: KeyPhase, release?: ReleaseInfo) => void;

interface Held {
  key: KeyLike;
  downAt: number;
  repeats: number;
}

export class KeyRepeater {
  private held = new Map<string, Held>();
  private repeatingCode: string | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private readonly emit: Emit,
    private readonly now: () => number = () => performance.now(),
  ) {}

  /** A keydown from the browser. `repeat` events are ignored: AIONE makes its own. */
  keydown(e: KeyLike & { repeat: boolean }): void {
    if (e.repeat || this.held.has(e.code)) return;
    const key = { key: e.key, code: e.code };
    this.held.set(e.code, { key, downAt: this.now(), repeats: 0 });
    this.emit(key, 'down');
    if (!repeats(e.key)) return;
    this.stopTimer();
    this.repeatingCode = e.code;
    this.schedule(e.code, KEY_REPEAT_DELAY_MS);
  }

  keyup(e: KeyLike): void {
    const h = this.held.get(e.code);
    if (!h) return;
    this.held.delete(e.code);
    if (this.repeatingCode === e.code) {
      this.stopTimer();
      this.repeatingCode = null;
    }
    this.emit(h.key, 'up', { heldMs: Math.round(this.now() - h.downAt), repeats: h.repeats });
  }

  /** The window lost focus: keyups will not arrive, so release everything now. */
  releaseAll(): void {
    this.stopTimer();
    this.repeatingCode = null;
    const all = [...this.held.values()];
    this.held.clear();
    const t = this.now();
    all.forEach((h) => this.emit(h.key, 'up', { heldMs: Math.round(t - h.downAt), repeats: h.repeats }));
  }

  /** Whether a timer is running (for tests and the perf line). */
  get active(): boolean {
    return this.timer !== null;
  }

  private schedule(code: string, ms: number): void {
    this.timer = setTimeout(() => {
      const h = this.held.get(code);
      if (this.repeatingCode !== code || !h) return;
      h.repeats += 1;
      this.emit(h.key, 'repeat');
      this.schedule(code, KEY_REPEAT_INTERVAL_MS);
    }, ms);
  }

  private stopTimer(): void {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
  }
}
