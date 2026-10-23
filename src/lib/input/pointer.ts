// B-12 (reference A7): the right-stick pointer. The core sends the stick past its dead zone
// (each axis -1000..1000, up positive, in steps of 25) only when it changes; the UI moves the
// pointer by transform on animation frames while the stick is tilted (no loop when it rests),
// and the item under it becomes the focused item. Pure helpers here (tested in
// tests/unit/pointer.test.ts); the view is lib/components/Pointer.svelte, the wiring App.svelte.

/** Full tilt: design px per second at 1080 p (about 1.3 s across the screen). */
export const POINTER_MAX_SPEED = 1500;
/** Smooth acceleration (reference A7): speed grows with the tilt squared, so a small tilt is precise. */
export const POINTER_CURVE = 2;
/** The pointer hides after about 2 s without right-stick movement (reference A7). */
export const POINTER_HIDE_MS = 2000;
/** An item only partly in view is focused after the pointer rests on it this long, so a list
 *  scrolls step by step while the pointer rests near its edge (reference A7). */
export const POINTER_EDGE_DWELL_MS = 400;

/** Velocity in screen px per second (y down) for a stick vector (y up), `px` = one design pixel. */
export function pointerVelocity(stick: readonly [number, number], px: number): [number, number] {
  const [x, y] = stick;
  const mag = Math.min(1000, Math.hypot(x, y));
  if (mag === 0) return [0, 0];
  const speed = POINTER_MAX_SPEED * px * (mag / 1000) ** POINTER_CURVE;
  return [(x / Math.hypot(x, y)) * speed, (-y / Math.hypot(x, y)) * speed];
}

/** One frame's move, kept inside the window. */
export function pointerStep(
  pos: { x: number; y: number },
  velocity: readonly [number, number],
  dtMs: number,
  bounds: { width: number; height: number },
): { x: number; y: number } {
  const dt = Math.min(dtMs, 50) / 1000; // a stalled frame never jumps the pointer far
  return {
    x: Math.min(bounds.width - 1, Math.max(0, pos.x + velocity[0] * dt)),
    y: Math.min(bounds.height - 1, Math.max(0, pos.y + velocity[1] * dt)),
  };
}

/** What a `data-pointer` mark names: the place and its index or answer. */
export type PointerTarget =
  | { kind: 'tile'; index: number }
  | { kind: 'drawer'; index: number }
  | { kind: 'qm'; index: number }
  | { kind: 'po'; index: number }
  | { kind: 'dialog'; answer: 'yes' | 'no' };

export function parseTarget(mark: string | null | undefined): PointerTarget | null {
  if (!mark) return null;
  const [kind, value] = mark.split(':');
  if (kind === 'dialog') return value === 'yes' || value === 'no' ? { kind, answer: value } : null;
  const index = Number(value);
  if (!Number.isInteger(index) || index < 0) return null;
  if (kind === 'tile' || kind === 'drawer' || kind === 'qm' || kind === 'po') return { kind, index };
  return null;
}

/** Whether a rectangle is fully inside the window (a partly shown item waits for the dwell). */
export function fullyInView(r: { left: number; right: number; top: number; bottom: number }, w: number, h: number): boolean {
  return r.left >= -0.5 && r.top >= -0.5 && r.right <= w + 0.5 && r.bottom <= h + 0.5;
}
