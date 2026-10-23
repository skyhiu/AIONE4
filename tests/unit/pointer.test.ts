import { describe, expect, it } from 'vitest';
import { initialHome, pointDrawer, pointRow, withCount } from '../../src/lib/home/model';
import { fullyInView, parseTarget, pointerStep, pointerVelocity, POINTER_MAX_SPEED } from '../../src/lib/input/pointer';

describe('B-12: the right-stick pointer', () => {
  it('rests at no tilt; full tilt is full speed; up on the stick is up on screen', () => {
    expect(pointerVelocity([0, 0], 1)).toEqual([0, 0]);
    const [vx, vy] = pointerVelocity([1000, 0], 1);
    expect(vx).toBeCloseTo(POINTER_MAX_SPEED);
    expect(vy).toBeCloseTo(0);
    expect(pointerVelocity([0, 1000], 1)[1]).toBeCloseTo(-POINTER_MAX_SPEED);
  });
  it('accelerates smoothly: half tilt is a quarter of the speed (precise small moves)', () => {
    expect(pointerVelocity([500, 0], 1)[0]).toBeCloseTo(POINTER_MAX_SPEED / 4);
  });
  it('follows the window size (design pixels)', () => {
    expect(pointerVelocity([1000, 0], 2)[0]).toBeCloseTo(POINTER_MAX_SPEED * 2);
  });
  it('stays inside the window and never jumps on a stalled frame', () => {
    const b = { width: 1920, height: 1080 };
    expect(pointerStep({ x: 1910, y: 5 }, [1500, -1500], 16, b)).toEqual({ x: 1919, y: 0 });
    const far = pointerStep({ x: 100, y: 100 }, [1500, 0], 1000, b);
    expect(far.x).toBeCloseTo(175); // capped at 50 ms
  });
  it('reads the marks', () => {
    expect(parseTarget('tile:3')).toEqual({ kind: 'tile', index: 3 });
    expect(parseTarget('drawer:0')).toEqual({ kind: 'drawer', index: 0 });
    expect(parseTarget('dialog:yes')).toEqual({ kind: 'dialog', answer: 'yes' });
    expect(parseTarget('tile:x')).toBeNull();
    expect(parseTarget('nope:1')).toBeNull();
    expect(parseTarget(null)).toBeNull();
  });
  it('tells a partly shown item', () => {
    expect(fullyInView({ left: 10, right: 400, top: 300, bottom: 700 }, 1920, 1080)).toBe(true);
    expect(fullyInView({ left: 1700, right: 2100, top: 300, bottom: 700 }, 1920, 1080)).toBe(false);
  });
});

describe('B-12: the pointer on Home', () => {
  const home = withCount(initialHome(10, 4), 10);
  it('focuses the tile it rests on, scrolling the row to keep it in view', () => {
    const [a, movedA] = pointRow(home, 2);
    expect(movedA).toBe(true);
    expect([a.zone, a.focus, a.first]).toEqual(['row', 2, 0]);
    const [b] = pointRow(a, 4); // the partly shown fifth tile: the row scrolls by one
    expect([b.focus, b.first]).toEqual([4, 1]);
    expect(pointRow(b, 4)[1]).toBe(false); // resting on the focused tile: nothing new
    expect(pointRow(b, 99)[1]).toBe(false);
  });
  it('focuses the drawer button it rests on, and back to a tile', () => {
    const [d, moved] = pointDrawer(home, 5);
    expect(moved).toBe(true);
    expect([d.zone, d.drawer]).toEqual(['drawer', 5]);
    const [r] = pointRow(d, 0);
    expect(r.zone).toBe('row');
    expect(pointDrawer(home, 6)[1]).toBe(false);
  });
});
