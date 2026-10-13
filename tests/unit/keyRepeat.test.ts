import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { KEY_REPEAT_DELAY_MS, KEY_REPEAT_INTERVAL_MS, KeyRepeater, REPEATING_KEYS, repeats } from '../../src/lib/input/keyRepeat';

// D-006 / B-3: AIONE times key repeats (400 ms, then every 90 ms) and ignores Windows' repeats.
describe('key repeat', () => {
  let events: string[];
  let r: KeyRepeater;
  const right = { key: 'ArrowRight', code: 'ArrowRight' };
  const alt = { key: 'Alt', code: 'AltLeft' };

  beforeEach(() => {
    vi.useFakeTimers();
    events = [];
    r = new KeyRepeater(
      (e, phase, rel) =>
        events.push(`${phase} ${e.key} @${Date.now() - start}${rel ? ` held ${rel.heldMs} ms, ${rel.repeats} repeats` : ''}`),
      () => Date.now(),
    );
    start = Date.now();
  });
  afterEach(() => vi.useRealTimers());
  let start = 0;

  it('repeats an arrow key after 400 ms, then every 90 ms, ignoring Windows repeats', () => {
    expect(KEY_REPEAT_DELAY_MS).toBe(400);
    expect(KEY_REPEAT_INTERVAL_MS).toBe(90);
    r.keydown({ ...right, repeat: false });
    // Windows sends its own repeats about every 33 ms after ~250 ms: all ignored.
    for (let t = 250; t <= 700; t += 33) {
      vi.advanceTimersByTime(t === 250 ? 250 : 33);
      r.keydown({ ...right, repeat: true });
    }
    vi.advanceTimersByTime(1000 - (250 + 33 * 13));
    r.keyup(right);
    expect(events).toEqual([
      'down ArrowRight @0',
      'repeat ArrowRight @400',
      'repeat ArrowRight @490',
      'repeat ArrowRight @580',
      'repeat ArrowRight @670',
      'repeat ArrowRight @760',
      'repeat ArrowRight @850',
      'repeat ArrowRight @940',
      'up ArrowRight @1000 held 1000 ms, 7 repeats',
    ]);
    expect(r.active).toBe(false);
  });

  it('modifier keys never repeat', () => {
    r.keydown({ ...alt, repeat: false });
    for (let i = 0; i < 40; i++) {
      vi.advanceTimersByTime(50);
      r.keydown({ ...alt, repeat: true });
    }
    r.keyup(alt);
    expect(events).toEqual(['down Alt @0', 'up Alt @2000 held 2000 ms, 0 repeats']);
    expect(r.active).toBe(false);
  });

  it('only the newest key repeats; releasing it stops the repeats', () => {
    r.keydown({ ...right, repeat: false });
    vi.advanceTimersByTime(100);
    r.keydown({ key: 'ArrowDown', code: 'ArrowDown', repeat: false });
    vi.advanceTimersByTime(400);
    r.keyup({ key: 'ArrowDown', code: 'ArrowDown' });
    vi.advanceTimersByTime(500);
    r.keyup(right);
    expect(events).toEqual(['down ArrowRight @0', 'down ArrowDown @100', 'repeat ArrowDown @500', 'up ArrowDown @500 held 400 ms, 1 repeats', 'up ArrowRight @1000 held 1000 ms, 0 repeats']);
  });

  it('losing focus releases held keys and stops repeating', () => {
    r.keydown({ ...right, repeat: false });
    r.releaseAll();
    vi.advanceTimersByTime(1000);
    expect(events).toEqual(['down ArrowRight @0', 'up ArrowRight @0 held 0 ms, 0 repeats']);
  });

  // B-3 rev. e: only navigation keys repeat; Enter, Space and Esc never do, like A and B.
  it('only the arrows repeat; Enter, Space, Esc, Tab, letters and modifiers never do', () => {
    expect([...REPEATING_KEYS].sort()).toEqual(['ArrowDown', 'ArrowLeft', 'ArrowRight', 'ArrowUp']);
    for (const k of ['Enter', ' ', 'Escape', 'Tab', 'PageDown', 'a', 'Backspace', 'Alt', 'Control', 'Shift', 'Meta'])
      expect(repeats(k), k).toBe(false);
    const keys: [string, string][] = [
      ['Enter', 'Enter'],
      [' ', 'Space'],
      ['Escape', 'Escape'],
    ];
    for (const [key, code] of keys) {
      events = [];
      start = Date.now();
      r.keydown({ key, code, repeat: false });
      for (let i = 0; i < 20; i++) {
        vi.advanceTimersByTime(50);
        r.keydown({ key, code, repeat: true });
      }
      r.keyup({ key, code });
      expect(events).toEqual([`down ${key} @0`, `up ${key} @1000 held 1000 ms, 0 repeats`]);
    }
    expect(r.active).toBe(false);
  });
});
