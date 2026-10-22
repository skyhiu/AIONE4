import { describe, expect, it } from 'vitest';
import {
  emptyQueue,
  gameEnded,
  nextDue,
  padBattery,
  padConnection,
  pcBattery,
  push,
  SHOW_MS,
  STALE_MS,
  MAX_WAITING,
  step,
} from '../../src/lib/notify/notices';

describe('T2.3 notifications: the queue', () => {
  it('shows one at a time, each for SHOW_MS, in order', () => {
    let q = push(emptyQueue(), padConnection(1, true, 0)).q;
    q = push(q, gameEnded('Game A', 12, 0)).q;
    let r = step(q, 0, false);
    expect(r.shown?.title).toBe('Controller connected');
    expect(nextDue(r.q, 0)).toBe(SHOW_MS);
    r = step(r.q, SHOW_MS - 1, false);
    expect(r.ended).toBeNull();
    r = step(r.q, SHOW_MS, false);
    expect(r.ended?.title).toBe('Controller connected');
    expect(r.shown?.title).toBe('Game A ended');
    expect(r.shown?.detail).toBe('Played 12 min');
  });

  it('never shows while a dialog is open (or AIONE is busy); shows when it closes', () => {
    const q = push(emptyQueue(), gameEnded('Game A', 3, 0)).q;
    const held = step(q, 100, true);
    expect(held.shown).toBeNull();
    expect(held.q.waiting).toHaveLength(1);
    expect(step(held.q, 200, false).shown?.title).toBe('Game A ended');
  });

  it('a newer notice about the same controller replaces the waiting one', () => {
    let q = push(emptyQueue(), padConnection(1, false, 0)).q;
    const r = push(q, padConnection(1, true, 50));
    q = r.q;
    expect(r.dropped.map((d) => d.title)).toEqual(['Controller disconnected']);
    expect(q.waiting.map((w) => w.title)).toEqual(['Controller connected']);
  });

  it('keeps at most MAX_WAITING, dropping the oldest', () => {
    let q = emptyQueue();
    const dropped: string[] = [];
    for (let i = 1; i <= MAX_WAITING + 1; i++) {
      const r = push(q, gameEnded(`G${i}`, 1, i));
      q = r.q;
      dropped.push(...r.dropped.map((d) => d.title));
    }
    expect(q.waiting).toHaveLength(MAX_WAITING);
    expect(dropped).toEqual(['G1 ended']);
  });

  it('drops controller notices that waited too long, never a game end', () => {
    let q = push(emptyQueue(), padConnection(2, true, 0)).q;
    q = push(q, gameEnded('Game A', 0, 0)).q;
    const r = step(q, STALE_MS + 1, false);
    expect(r.stale.map((s) => s.title)).toEqual(['Controller connected']);
    expect(r.shown?.title).toBe('Game A ended');
    expect(r.shown?.detail).toBeUndefined(); // nothing played: no time
  });
});

describe('T2.3 notifications: batteries', () => {
  it('a controller: once at low, once at empty, again after a charge', () => {
    let last: string | null = null;
    const seen: string[] = [];
    for (const word of ['full', 'medium', 'low', 'low', 'empty', 'low', 'full', 'low']) {
      const r = padBattery(1, word, last, 0);
      last = r.last;
      if (r.notice) seen.push(r.notice.title);
    }
    expect(seen).toEqual(['Controller battery low', 'Controller battery empty', 'Controller battery low']);
    // Wired or unknown: no notice.
    expect(padBattery(1, 'wired', null, 0).notice).toBeNull();
    expect(padBattery(1, 'unknown', null, 0).notice).toBeNull();
  });

  it('the PC: at 20 % and at 10 % while not charging; charging starts over', () => {
    let last: number | null = null;
    const seen: string[] = [];
    const levels: Array<[number, boolean]> = [
      [50, false], [21, false], [20, false], [19, false], [10, false], [9, false], [9, true], [18, false],
    ];
    for (const [percent, charging] of levels) {
      const r = pcBattery({ percent, charging }, last, 0);
      last = r.last;
      if (r.notice) seen.push(r.notice.detail ?? '');
    }
    expect(seen).toEqual(['20 % left: connect the charger', '10 % left: connect the charger', '18 % left: connect the charger']);
    // A desktop (no battery): nothing.
    expect(pcBattery(null, null, 0).notice).toBeNull();
  });
});
