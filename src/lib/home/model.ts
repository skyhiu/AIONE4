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
  // B-14 (rev. x, measured on the pictures' edges): tiles 386 square, 18 apart (pitch 404, as
  // before), the first 155 from
  // the left; rev. v: the ring's top at about 278 (the screen area corrected), the ring and its
  // white band 12.5 px outside the art, so the tiles' top at 290.5.
  tile: 386,
  gap: 18,
  left: 155,
  top: 291,
} as const;

/** B-14 (rev. r): the row always has 12 slots: the games, then plain empty slots up to 12 (all
 *  focusable). More than 12 games: the rest via "All games" (T4). */
export const ROW_SLOTS = 12;
export function emptySlots(games: number): number {
  return games === 0 ? 0 : Math.max(0, ROW_SLOTS - games);
}

export interface HomeState {
  /** Games in the row. */
  count: number;
  /** Focusable slots: the games, then the empty slots up to 12 (B-14 rev. r). */
  slots: number;
  focus: number;
  /** Index of the first tile in view. */
  first: number;
  /** Tiles fully in view. */
  visible: number;
  /** Mouse: the tile selected by a first click, or null. */
  armed: number | null;
  /** Mouse: the tile under the pointer, or null. */
  hover: number | null;
  /** T1.2: which part of Home has the focus (one active context): the game row or the drawer. */
  zone: 'row' | 'drawer';
  /** The drawer's focused button (remembered while the row has the focus). */
  drawer: number;
  /** Mouse: the drawer button under the pointer, or null. */
  drawerHover: number | null;
}

/** The System Drawer (reference Part B; Switch-style, B-9 rev. o). Icons from Material Symbols
 *  Rounded (outlined, light, like the header). Their screens come in later stages (T2–T4). */
// `weight`: each icon's Material Symbols weight, balanced so every icon carries about the same
// ink (reference KD-81: within about ±25 % of the median at the same box size; measured).
export const DRAWER = [
  { id: 'search', label: 'Search', icon: 'search', stage: 'T4', weight: 480 },
  { id: 'activity', label: 'Activity', icon: 'leaderboard', stage: 'T4', weight: 300 },
  { id: 'surprise', label: 'Surprise Me', icon: 'casino', stage: 'T4', weight: 300 },
  { id: 'theme', label: 'Theme Settings', icon: 'palette', stage: 'T3', weight: 300 },
  { id: 'settings', label: 'Settings', icon: 'settings', stage: 'T3', weight: 250 },
  { id: 'power', label: 'Power', icon: 'power_settings_new', stage: 'T2', weight: 380 },
] as const;
export type DrawerItem = (typeof DRAWER)[number];

/** Down from the row enters the drawer (on its remembered button); Up from the drawer returns
 *  to the last focused tile (reference Part B). Returns whether the focus moved. */
export function moveZone(s: HomeState, dir: 'up' | 'down'): [HomeState, boolean] {
  if (dir === 'down' && s.zone === 'row') return [{ ...s, zone: 'drawer' }, true];
  if (dir === 'up' && s.zone === 'drawer') return [{ ...s, zone: 'row' }, true];
  return [s, false];
}

/** Left / right in the drawer; stops at its ends. */
export function moveDrawer(s: HomeState, dir: -1 | 1): [HomeState, boolean] {
  const next = Math.max(0, Math.min(DRAWER.length - 1, s.drawer + dir));
  return next === s.drawer ? [s, false] : [{ ...s, drawer: next }, true];
}

/** Mouse over a drawer button (null when it leaves). */
export function hoverDrawer(s: HomeState, index: number | null): HomeState {
  // The last hovered button is remembered for the pad (reference A7).
  return index === null ? { ...s, drawerHover: null } : { ...s, drawerHover: index, drawer: index, zone: 'drawer' };
}

export function initialHome(count: number, visible: number): HomeState {
  const v = Math.max(1, visible);
  return { count, slots: count + emptySlots(count), focus: 0, first: 0, visible: v, armed: null, hover: null, zone: 'row', drawer: 0, drawerHover: null };
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
  const slots = count + emptySlots(count);
  const focus = Math.min(s.focus, Math.max(0, slots - 1));
  return { ...s, count, slots, focus, first: keepInView(s.first, focus, s.visible, slots), armed: null, hover: null };
}

export function withVisible(s: HomeState, visible: number): HomeState {
  const v = Math.max(1, visible);
  const slots = s.count + emptySlots(s.count);
  const focus = Math.min(s.focus, Math.max(0, slots - 1));
  return { ...s, visible: v, slots, focus, first: keepInView(s.first, focus, v, slots) };
}

/** B-14 (rev. r): the wrap at the row's ends: from the last slot to the first game, and from the
 *  first game to the last slot; the row whips across (its own 170 ms slide). */
export function wrapRow(s: HomeState, dir: -1 | 1): HomeState {
  const focus = dir > 0 ? 0 : s.slots - 1;
  return { ...s, focus, first: keepInView(s.first, focus, s.visible, s.slots), armed: null };
}

/** Whether a slot is an empty one (after the last game). */
export const isEmptySlot = (s: HomeState, index: number): boolean => index >= s.count;

/** One press or repeat left (-1) or right (+1). Returns the new state and whether focus moved. */
export function move(s: HomeState, dir: -1 | 1): [HomeState, boolean] {
  const focus = s.focus + dir;
  if (s.count === 0 || focus < 0 || focus >= s.slots) return [s, false];
  return [{ ...s, focus, first: keepInView(s.first, focus, s.visible, s.slots), armed: null }, true];
}

/** Back to pad / keyboard mode: the focus ring appears on the remembered item, in view. */
export function toPadMode(s: HomeState): HomeState {
  return { ...s, hover: null, armed: null, first: keepInView(s.first, s.focus, s.visible, s.slots) };
}

export function hoverTile(s: HomeState, index: number | null): HomeState {
  if (index === null) return { ...s, hover: null };
  if (index < 0 || index >= s.slots) return s;
  // A7: the hovered item is remembered for the pad.
  return { ...s, hover: index, focus: index, zone: 'row' };
}

/** B-12 (reference A7): the right-stick pointer rests on a slot: it becomes the focused item, in
 *  the row (the row scrolls to keep it fully in view, as a move would). Returns whether it moved. */
export function pointRow(s: HomeState, index: number): [HomeState, boolean] {
  if (index < 0 || index >= s.slots || (s.zone === 'row' && s.focus === index)) return [s, false];
  return [{ ...s, zone: 'row', focus: index, first: keepInView(s.first, index, s.visible, s.slots), armed: null, hover: null }, true];
}

/** B-12: the pointer rests on a drawer button: it becomes the focused one. */
export function pointDrawer(s: HomeState, index: number): [HomeState, boolean] {
  if (index < 0 || index >= DRAWER.length || (s.zone === 'drawer' && s.drawer === index)) return [s, false];
  return [{ ...s, zone: 'drawer', drawer: index, drawerHover: null }, true];
}

export type ClickResult = 'select' | 'start';

/** L-06: first click selects, second click on the selected tile starts. */
export function clickTile(s: HomeState, index: number): [HomeState, ClickResult] {
  if (s.armed === index) return [{ ...s, armed: null, focus: index }, 'start'];
  return [{ ...s, armed: index, focus: index }, 'select'];
}

/** Mouse wheel: scroll the view by one tile. Returns whether it moved. */
export function wheel(s: HomeState, dir: -1 | 1): [HomeState, boolean] {
  const max = Math.max(0, s.slots - s.visible);
  const first = Math.min(Math.max(0, s.first + dir), max);
  return [{ ...s, first }, first !== s.first];
}

/** The tile whose title is shown: pad mode the focus; mouse mode the hovered, else the
 *  selected tile (none when nothing is hovered or selected). */
export function titleIndex(s: HomeState, mode: 'pad' | 'mouse'): number | null {
  if (s.count === 0) return null;
  // Like Switch: no title while the drawer has the focus, nor on an empty slot.
  const i = mode === 'pad' ? (s.zone === 'drawer' ? null : s.focus) : (s.hover ?? s.armed);
  return i === null || i >= s.count ? null : i;
}
