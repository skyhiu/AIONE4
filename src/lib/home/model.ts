// The Home game row as pure state (no DOM): focus, which tiles are in view, and the mouse's
// two-click rule. Home.svelte draws it; tests/unit/homeModel.test.ts checks it.
//
// Rules (reference A6 / A7, L-06):
// - D-pad, left stick and arrows move the focus one tile per press or repeat; the row stops at
//   both ends (no wrap, as on the console) and a press at an end moves nothing.
// - The view keeps the focused tile fully visible, moving as little as possible.
// - Mouse: hover only shows hover and remembers the item for the pad; the **first click on a
//   tile selects it**, a **second click on the selected tile starts it**; a single click on an
//   unselected tile never starts a game. After starting, nothing stays selected.
// - The wheel scrolls the view (mouse mode) without moving the focus.

/** Layout in design pixels at 1080 p (the row scales with the window height only; Interface
 *  Scale does not change tiles, which follow Tile Size; T1.1 has the default, Small). */
export const ROW = {
  tile: 384,
  gap: 20,
  left: 160,
} as const;

export interface HomeState {
  count: number;
  focus: number;
  /** Index of the first tile in view. */
  first: number;
  /** Tiles fully in view. */
  visible: number;
  /** Mouse: the tile selected by a first click, or null. */
  armed: number | null;
  /** Mouse: the tile under the pointer, or null. */
  hover: number | null;
}

export function initialHome(count: number, visible: number): HomeState {
  return { count, focus: 0, first: 0, visible: Math.max(1, visible), armed: null, hover: null };
}

/** How many tiles fit fully, for a window of `width` × `height` CSS pixels. */
export function visibleTiles(width: number, height: number): number {
  const px = height / 1080;
  const fit = Math.floor((width - ROW.left * px + ROW.gap * px) / ((ROW.tile + ROW.gap) * px));
  return Math.max(1, fit);
}

/** The first tile in view so that `focus` is fully visible, moving the view minimally. */
export function keepInView(first: number, focus: number, visible: number, count: number): number {
  let f = first;
  if (focus < f) f = focus;
  if (focus > f + visible - 1) f = focus - visible + 1;
  const max = Math.max(0, count - visible);
  return Math.min(Math.max(0, f), max);
}

export function withCount(s: HomeState, count: number): HomeState {
  const focus = Math.min(s.focus, Math.max(0, count - 1));
  return { ...s, count, focus, first: keepInView(s.first, focus, s.visible, count), armed: null, hover: null };
}

export function withVisible(s: HomeState, visible: number): HomeState {
  const v = Math.max(1, visible);
  return { ...s, visible: v, first: keepInView(s.first, s.focus, v, s.count) };
}

/** One press or repeat left (-1) or right (+1). Returns the new state and whether focus moved. */
export function move(s: HomeState, dir: -1 | 1): [HomeState, boolean] {
  const focus = s.focus + dir;
  if (s.count === 0 || focus < 0 || focus >= s.count) return [s, false];
  return [{ ...s, focus, first: keepInView(s.first, focus, s.visible, s.count), armed: null }, true];
}

/** Back to pad / keyboard mode: the focus ring appears on the remembered item, in view. */
export function toPadMode(s: HomeState): HomeState {
  return { ...s, hover: null, armed: null, first: keepInView(s.first, s.focus, s.visible, s.count) };
}

export function hoverTile(s: HomeState, index: number | null): HomeState {
  if (index === null) return { ...s, hover: null };
  if (index < 0 || index >= s.count) return s;
  // A7: the hovered item is remembered for the pad.
  return { ...s, hover: index, focus: index };
}

export type ClickResult = 'select' | 'start';

/** L-06: first click selects, second click on the selected tile starts. */
export function clickTile(s: HomeState, index: number): [HomeState, ClickResult] {
  if (s.armed === index) return [{ ...s, armed: null, focus: index }, 'start'];
  return [{ ...s, armed: index, focus: index }, 'select'];
}

/** Mouse wheel: scroll the view by one tile. Returns whether it moved. */
export function wheel(s: HomeState, dir: -1 | 1): [HomeState, boolean] {
  const max = Math.max(0, s.count - s.visible);
  const first = Math.min(Math.max(0, s.first + dir), max);
  return [{ ...s, first }, first !== s.first];
}

/** Where the selected game's title sits over its tile: at the row's ends it aligns with the
 *  tile's outer edge so it never leaves the screen. */
export function titleAlign(s: HomeState, index: number): 'start' | 'center' | 'end' {
  const pos = index - s.first;
  if (pos <= 0) return 'start';
  if (pos >= s.visible - 1) return 'end';
  return 'center';
}

/** The tile whose title is shown: pad mode the focus; mouse mode the hovered, else the
 *  selected tile (none when nothing is hovered or selected). */
export function titleIndex(s: HomeState, mode: 'pad' | 'mouse'): number | null {
  if (s.count === 0) return null;
  if (mode === 'pad') return s.focus;
  return s.hover ?? s.armed;
}
