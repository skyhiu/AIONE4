// One input event shape for every source, so controller, keyboard and mouse reach the same
// actions (Master Prompt A4).
import type { PadEvent } from '../core/types';

export type InputSource = 'controller' | 'keyboard' | 'mouse';
export type InputKind = 'down' | 'up' | 'repeat' | 'held';

export interface InputEvent {
  source: InputSource;
  kind: InputKind;
  /** Human-readable control name, e.g. "A", "DPadUp", "Enter", "Left click". */
  control: string;
  /** Controller slot (1–4) for controller events; undefined otherwise. */
  pad?: number;
  /** The core's sequence number (controller events), to prove one press shows once. */
  seq?: number;
  /** D-008: when the core read it (Unix ms), for the core → UI delay. */
  atMs?: number;
}

/** Converts a core pad event to an input event; connect / disconnect and battery are not input. */
export function fromPadEvent(ev: PadEvent): InputEvent | null {
  if (ev.kind === 'connected' || ev.kind === 'disconnected' || ev.kind === 'battery') return null;
  return { source: 'controller', kind: ev.kind, control: ev.control, pad: ev.pad + 1, seq: ev.seq, atMs: ev.atMs };
}

const KEY_NAMES: Record<string, string> = {
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  Escape: 'Esc',
  ' ': 'Space',
};

/** Names a keyboard key from a KeyboardEvent-like object. */
export function keyName(e: { key: string; code: string }): string {
  const named = KEY_NAMES[e.key];
  if (named) return named;
  if (e.key.length === 1) return e.key.toUpperCase();
  if (e.key === 'Unidentified' || e.key === '') return e.code || 'Unknown';
  return e.key;
}

export function fromKeyboard(e: { key: string; code: string; repeat: boolean }, type: 'keydown' | 'keyup'): InputEvent {
  const kind: InputKind = type === 'keyup' ? 'up' : e.repeat ? 'repeat' : 'down';
  return { source: 'keyboard', kind, control: keyName(e) };
}

/** A key event from AIONE's own key repeater (D-006): down, AIONE-timed repeat, or up. */
export function fromKey(e: { key: string; code: string }, phase: InputKind): InputEvent {
  return { source: 'keyboard', kind: phase, control: keyName(e) };
}

const MOUSE_BUTTONS = ['Left click', 'Middle click', 'Right click', 'Back button', 'Forward button'];

export function fromMouseButton(button: number, type: 'mousedown' | 'mouseup'): InputEvent {
  return {
    source: 'mouse',
    kind: type === 'mouseup' ? 'up' : 'down',
    control: MOUSE_BUTTONS[button] ?? `Mouse button ${button}`,
  };
}

/** A wheel notch counts as one press (there is no release). */
export function fromWheel(deltaY: number): InputEvent | null {
  if (deltaY === 0) return null;
  return { source: 'mouse', kind: 'down', control: deltaY < 0 ? 'Wheel up' : 'Wheel down' };
}

/** The actions Home and the dialogs use (T1.1). More come with the Quick Menu (T2). */
export type UiAction = 'confirm' | 'back' | 'left' | 'right' | 'up' | 'down' | 'options' | 'guide' | 'close' | 'menu' | 'bigLeft' | 'bigRight';

const ACTIONS: Record<InputSource, Record<string, UiAction>> = {
  controller: {
    A: 'confirm',
    // B-14: Switch's "+ Options" is the pad's Menu button (≡).
    Menu: 'options',
    // B-18: X closes the running game ("X Close Software").
    X: 'close',
    // B-7 (T1.3): Guide in front returns to the running game, if one is running.
    Guide: 'guide',
    // A8: LT / RT change a focused slider by 10 %.
    LT: 'bigLeft',
    RT: 'bigRight',
    B: 'back',
    DPadLeft: 'left',
    DPadRight: 'right',
    DPadUp: 'up',
    DPadDown: 'down',
    LStickLeft: 'left',
    LStickRight: 'right',
    LStickUp: 'up',
    LStickDown: 'down',
  },
  keyboard: {
    Enter: 'confirm',
    Space: 'confirm',
    Esc: 'back',
    Backspace: 'back',
    // B-28: inside AIONE, Home is the keyboard's Guide (short; a hold is the Quick Menu).
    Home: 'guide',
    // D-056: Alt + ` caught by the page (when the core's hook didn't take it): a short Guide.
    'Alt+`': 'guide',
    // B-3 (rev. r): Tab is the keyboard's Options key (like the pad's ≡ / +); the Menu key too.
    Tab: 'options',
    ContextMenu: 'options',
    // B-18: on the keyboard, Delete closes the running game.
    Delete: 'close',
    Left: 'left',
    Right: 'right',
    Up: 'up',
    Down: 'down',
  },
  mouse: {},
};

/** Maps an input event to a UI action. Only presses and repeats act; releases never do. */
export function toAction(ev: InputEvent): UiAction | null {
  if (ev.kind === 'up') return null;
  // B-26: a held Guide (decided in the core) opens the Quick Menu.
  if (ev.kind === 'held') return ev.control === 'Guide' || ev.control === 'Home' ? 'menu' : null;
  const action = ACTIONS[ev.source][ev.control];
  if (!action) return null;
  // Confirm and back never repeat while held (one press, one action).
  if (ev.kind === 'repeat' && (action === 'confirm' || action === 'back' || action === 'options' || action === 'guide' || action === 'close')) return null;
  return action;
}

/** One log line for a press or repeat the UI received: source, kind, control, slot, the
 *  core's sequence number, the action, and what it did (focus move, sound), so "one press, one
 *  action, one sound" can be proven from the log. */
export function describeInput(ev: InputEvent, action: UiAction | null, outcome?: string): string {
  const where = ev.pad === undefined ? ev.source : `${ev.source} ${ev.pad}`;
  const seq = ev.seq === undefined ? '' : ` seq ${ev.seq}`;
  const what = action ? `action ${action}` : 'no action';
  return `input ${ev.kind} ${ev.control} (${where}${seq}) -> ${what}${outcome ? `, ${outcome}` : ''}`;
}

/** D-006 / B-3: a key release, like the pad's: `input up <key> (keyboard), held N ms, R repeats`. */
export function describeKeyRelease(control: string, heldMs: number, repeats: number): string {
  return `input up ${control} (keyboard), held ${heldMs} ms, ${repeats} repeats`;
}

/** B-10 (rev. k): the inputs that count as the first press after mouse use, i.e. those that
 *  act on Home: any pad button or direction, and the keys arrows, Enter, Space, Esc and
 *  Backspace. Modifiers alone (Alt, Ctrl, Shift, Windows) and other keys do nothing, so Alt-Tab
 *  or Alt+F4 never bring the ring back. */
const HOME_KEYS = new Set(['Up', 'Down', 'Left', 'Right', 'Enter', 'Space', 'Esc', 'Backspace']);
export function countsAsHomeInput(ev: InputEvent): boolean {
  if (ev.source === 'controller') return true;
  if (ev.source === 'keyboard') return HOME_KEYS.has(ev.control);
  return false;
}

/** The "(controller 1 seq 2)" / "(keyboard)" part of an input line (QA-6: always the slot). */
export function inputWhere(ev: InputEvent): string {
  const where = ev.pad === undefined ? ev.source : `${ev.source} ${ev.pad}`;
  return `${where}${ev.seq === undefined ? '' : ` seq ${ev.seq}`}`;
}

