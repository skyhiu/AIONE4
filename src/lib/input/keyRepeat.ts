// D-006 / B-3: AIONE times key repeats itself, like the controller (400 ms, then every 90 ms),
// and ignores the browser's / Windows' own repeated keydown events, so Windows' keyboard
// settings never change the speed. Modifier keys never repeat. Only the most recently pressed
// key repeats (as on a keyboard); releasing it stops its repeats.
//
// Timers: one setTimeout chain while a key is held (owner: KeyRepeater); none when idle.

export const KEY_REPEAT_DELAY_MS = 400;
export const KEY_REPEAT_INTERVAL_MS = 90;

/** KeyboardEvent.key values that never repeat. */
const MODIFIERS = new Set(['Alt', 'AltGraph', 'Control', 'Shift', 'Meta', 'OS', 'CapsLock', 'NumLock', 'ScrollLock', 'Fn']);

export function isModifier(key: string): boolean {
  return MODIFIERS.has(key);
}

export type KeyPhase = 'down' | 'repeat' | 'up';

export interface KeyLike {
  key: string;
  code: string;
}

type Emit = (e: KeyLike, phase: KeyPhase) => void;

export class KeyRepeater {
  private held = new Map<string, KeyLike>();
  private repeatingCode: string | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;

  constructor(private readonly emit: Emit) {}

  /** A keydown from the browser. `repeat` events are ignored: AIONE makes its own. */
  keydown(e: KeyLike & { repeat: boolean }): void {
    if (e.repeat || this.held.has(e.code)) return;
    const key = { key: e.key, code: e.code };
    this.held.set(e.code, key);
    this.emit(key, 'down');
    if (isModifier(e.key)) return;
    this.stopTimer();
    this.repeatingCode = e.code;
    this.schedule(key, KEY_REPEAT_DELAY_MS);
  }

  keyup(e: KeyLike): void {
    const key = this.held.get(e.code);
    if (!key) return;
    this.held.delete(e.code);
    if (this.repeatingCode === e.code) {
      this.stopTimer();
      this.repeatingCode = null;
    }
    this.emit(key, 'up');
  }

  /** The window lost focus: keyups will not arrive, so release everything now. */
  releaseAll(): void {
    this.stopTimer();
    this.repeatingCode = null;
    const keys = [...this.held.values()];
    this.held.clear();
    keys.forEach((k) => this.emit(k, 'up'));
  }

  /** Whether a timer is running (for tests and the idle check). */
  get active(): boolean {
    return this.timer !== null;
  }

  private schedule(key: KeyLike, ms: number): void {
    this.timer = setTimeout(() => {
      if (this.repeatingCode !== key.code) return;
      this.emit(key, 'repeat');
      this.schedule(key, KEY_REPEAT_INTERVAL_MS);
    }, ms);
  }

  private stopTimer(): void {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
  }
}
