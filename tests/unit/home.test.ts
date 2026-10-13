import { describe, expect, it } from 'vitest';
import {
  clickTile,
  hoverTile,
  initialHome,
  keepInView,
  move,
  titleAlign,
  titleIndex,
  toPadMode,
  visibleTiles,
  wheel,
  withCount,
} from '../../src/lib/home/model';
import { RepeatPacer, REPEAT_MIN_GAP_MS, SOUND_FILES } from '../../src/lib/audio/sounds';
import { formatClock, msToNextMinute } from '../../src/lib/clock';
import { frameStats } from '../../src/lib/perf/perf';
import { deviceAfter } from '../../src/lib/frame/glyphs';

describe('Home row model', () => {
  it('fits 4 full tiles at 16:9 at every size', () => {
    expect(visibleTiles(1280, 720)).toBe(4);
    expect(visibleTiles(1920, 1080)).toBe(4);
    expect(visibleTiles(2560, 1440)).toBe(4);
  });

  it('moves one tile per press and stops at both ends', () => {
    let s = withCount(initialHome(0, 4), 10);
    let moved: boolean;
    [s, moved] = move(s, -1);
    expect(moved).toBe(false);
    for (let i = 0; i < 12; i++) [s] = move(s, 1);
    expect(s.focus).toBe(9);
    [s, moved] = move(s, 1);
    expect(moved).toBe(false);
    expect(s.first).toBe(6);
  });

  it('keeps the focus in view, moving the view as little as possible', () => {
    expect(keepInView(0, 3, 4, 10)).toBe(0);
    expect(keepInView(0, 4, 4, 10)).toBe(1);
    expect(keepInView(5, 2, 4, 10)).toBe(2);
    expect(keepInView(0, 2, 4, 3)).toBe(0);
  });

  it('L-06: first click selects, second click on the same tile starts', () => {
    let s = withCount(initialHome(0, 4), 10);
    let r;
    [s, r] = clickTile(s, 2);
    expect(r).toBe('select');
    [s, r] = clickTile(s, 3);
    expect(r).toBe('select');
    [s, r] = clickTile(s, 3);
    expect(r).toBe('start');
    expect(s.armed).toBeNull();
    expect(s.focus).toBe(3);
  });

  it('hover is remembered for the pad; pad mode clears mouse visuals', () => {
    let s = withCount(initialHome(0, 4), 10);
    s = hoverTile(s, 5);
    expect(s.focus).toBe(5);
    expect(titleIndex(s, 'mouse')).toBe(5);
    s = toPadMode(s);
    expect(s.hover).toBeNull();
    expect(s.first).toBe(2);
    expect(titleIndex(s, 'pad')).toBe(5);
    expect(titleIndex(hoverTile(s, null), 'mouse')).toBeNull();
  });

  it('the wheel scrolls the view only', () => {
    let s = withCount(initialHome(0, 4), 10);
    const [first, moved] = wheel(s, 1);
    s = first;
    expect(moved).toBe(true);
    expect(s.focus).toBe(0);
    for (let i = 0; i < 10; i++) [s] = wheel(s, 1);
    expect(s.first).toBe(6);
  });

  it('the title aligns with the row ends', () => {
    const s = withCount(initialHome(0, 4), 10);
    expect(titleAlign(s, 0)).toBe('start');
    expect(titleAlign(s, 1)).toBe('center');
    expect(titleAlign(s, 3)).toBe('end');
  });
});

describe('menu sounds', () => {
  it('maps move, select and back to the proposed pack files', () => {
    expect(SOUND_FILES.move.pack).toBe(24);
    expect(SOUND_FILES.select.pack).toBe(9);
    expect(SOUND_FILES.back.pack).toBe(15);
  });
  it('a press always plays; repeats every 90 ms play every second time', () => {
    const p = new RepeatPacer();
    expect(REPEAT_MIN_GAP_MS).toBe(180);
    const played = [p.shouldPlay('press', 0)];
    for (let t = 400; t <= 400 + 90 * 9; t += 90) played.push(p.shouldPlay('repeat', t));
    expect(played).toEqual([true, true, false, true, false, true, false, true, false, true, false]);
    expect(p.shouldPlay('press', 1211)).toBe(true);
  });
});

describe('clock', () => {
  it('12-hour with a separate AM / PM; 24-hour without', () => {
    expect(formatClock(new Date(2026, 9, 13, 12, 24), true)).toEqual({ time: '12:24', ampm: 'PM' });
    expect(formatClock(new Date(2026, 9, 13, 0, 5), true)).toEqual({ time: '12:05', ampm: 'AM' });
    expect(formatClock(new Date(2026, 9, 13, 9, 5), false)).toEqual({ time: '09:05', ampm: null });
    expect(msToNextMinute(new Date(2026, 9, 13, 9, 5, 59, 500))).toBe(500);
  });
});

describe('perf and glyphs', () => {
  it('frame stats', () => {
    const s = frameStats([16, 17, 16, 17, 16, 17, 16, 17, 16, 50]);
    expect(s.n).toBe(10);
    expect(s.avg).toBeCloseTo(19.8, 1);
    expect(s.p95).toBe(50);
  });
  it('the glyph set follows the last controller or keyboard; the mouse keeps it', () => {
    expect(deviceAfter('xbox', 'keyboard')).toBe('keyboard');
    expect(deviceAfter('keyboard', 'mouse')).toBe('keyboard');
    expect(deviceAfter('keyboard', 'controller')).toBe('xbox');
  });
});
