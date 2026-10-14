import { describe, expect, it } from 'vitest';
import {
  emptySlots,
  isEmptySlot,
  wrapRow,
  clickTile,
  hoverTile,
  initialHome,
  keepInView,
  move,
  titleIndex,
  toPadMode,
  visibleTiles,
  wheel,
  withCount,
} from '../../src/lib/home/model';
import { SOUND_FILES } from '../../src/lib/audio/sounds';
import { formatClock, msToNextMinute } from '../../src/lib/clock';
import { describeFrames, describeLongTask, frameStats } from '../../src/lib/perf/perf';
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
    // B-14 (rev. r): 12 slots: the 10 games, then 2 empty slots, all focusable.
    expect(s.slots).toBe(12);
    expect(s.focus).toBe(11);
    expect(isEmptySlot(s, 10) && isEmptySlot(s, 11)).toBe(true);
    expect(titleIndex(s, 'pad')).toBeNull(); // no title on an empty slot
    [s, moved] = move(s, 1);
    expect(moved).toBe(false);
    expect(s.first).toBe(8);
    // The wrap: from the last slot to the first game, and back.
    s = wrapRow(s, 1);
    expect([s.focus, s.first]).toEqual([0, 0]);
    s = wrapRow(s, -1);
    expect([s.focus, s.first]).toEqual([11, 8]);
    expect(emptySlots(2)).toBe(10);
    expect(emptySlots(15)).toBe(0);
    expect(withCount(initialHome(0, 4), 2).slots).toBe(12);
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
    expect(s.first).toBe(8); // B-14 (rev. r): up to the 12th slot
  });

});

describe('menu sounds', () => {
  it('maps the sound set by function and place (B-1 rev. q)', () => {
    expect(Object.keys(SOUND_FILES).sort()).toEqual(['back', 'confirm', 'drawerMove', 'edgeBump', 'error', 'rowMove']);
    expect(SOUND_FILES.rowMove.file).toBe('row_move.wav');
    expect(SOUND_FILES.drawerMove.file).not.toBe(SOUND_FILES.rowMove.file);
    expect(Object.values(SOUND_FILES).every((s) => s.source.startsWith('FilmCow'))).toBe(true);
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

describe('perf line (D-008)', () => {
  it('names the longest frame, frames over 33 ms, and marks a window janky over 50 ms', () => {
    const smooth = frameStats(Array(300).fill(16.7));
    expect(smooth.janky).toBe(false);
    expect(describeFrames(smooth, 4)).toBe(
      'frames 5.0 s: avg 16.7 ms, p95 16.7 ms, longest 16.7 ms, over 33 ms: 0 (300 frames); longest input delay (core → UI) 4 ms',
    );
    // A D-007-like freeze: 290 normal frames and one of 510 ms. p95 hides it; the new parts show it.
    const freeze = frameStats([...Array(290).fill(16.7), 40, 510]);
    expect(freeze.p95).toBeLessThan(17);
    expect(freeze.longest).toBe(510);
    expect(freeze.over33).toBe(2);
    expect(freeze.janky).toBe(true);
    expect(describeFrames(freeze, 498)).toContain('longest 510.0 ms, over 33 ms: 2 (292 frames), JANKY; longest input delay (core → UI) 498 ms');
    expect(describeFrames(frameStats([]), null)).toBe('frames: not sampled yet');
  });
  it('a long task line names the last action and the pictures around it (D-007)', () => {
    const line = describeLongTask(10_000, 180, 'down DPadRight → right, focus 6/10 "Moss Kingdom"', [
      { at: 9_500, what: 'tile "Moss Kingdom" shown', width: 2048, height: 2048 },
      { at: 2_000, what: 'tile "Old" shown', width: 640, height: 640 },
    ]);
    expect(line).toBe(
      'long task 180 ms at 10.00 s; last action: down DPadRight → right, focus 6/10 "Moss Kingdom"; pictures loaded or decoded around it: tile "Moss Kingdom" shown 2048×2048',
    );
    expect(describeLongTask(0, 60, 'none yet', [])).toContain('around it: none');
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

describe('the drawer (T1.2)', () => {
  it('Down enters the drawer on its remembered button; Up returns to the last tile; ends stop', async () => {
    const { DRAWER, initialHome, move, moveDrawer, moveZone, titleIndex, hoverDrawer } = await import('../../src/lib/home/model');
    let s = initialHome(10, 4);
    [s] = move(s, 1);
    [s] = move(s, 1);
    expect(s.focus).toBe(2);
    let moved: boolean;
    [s, moved] = moveZone(s, 'up');
    expect(moved).toBe(false); // nothing above the row
    [s, moved] = moveZone(s, 'down');
    expect(moved && s.zone === 'drawer' && s.drawer === 0).toBe(true);
    expect(titleIndex(s, 'pad')).toBeNull(); // no game title while in the drawer
    [s, moved] = moveDrawer(s, -1);
    expect(moved).toBe(false); // the first button stops
    for (let i = 0; i < 10; i++) [s] = moveDrawer(s, 1);
    expect(s.drawer).toBe(DRAWER.length - 1); // the last button stops
    [s, moved] = moveZone(s, 'down');
    expect(moved).toBe(false); // nothing below the drawer
    [s] = moveZone(s, 'up');
    expect(s.zone === 'row' && s.focus === 2).toBe(true); // back on the last focused tile
    [s] = moveZone(s, 'down');
    expect(s.drawer).toBe(DRAWER.length - 1); // the drawer remembers its button
    s = hoverDrawer(s, 1);
    expect(s.zone === 'drawer' && s.drawer === 1 && s.drawerHover === 1).toBe(true);
    expect(DRAWER.map((d) => d.label)).toEqual(['Search', 'Activity', 'Surprise Me', 'Theme Settings', 'Settings', 'Power']);
  });
});
