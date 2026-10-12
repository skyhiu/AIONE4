// One input event shape for every source, so controller, keyboard and mouse reach the same
// readout (T0) and, from T1, the same actions (Master Prompt A4).
import type { PadEvent } from '../core/types';

export type InputSource = 'controller' | 'keyboard' | 'mouse';
export type InputKind = 'down' | 'up' | 'repeat';

export interface InputEvent {
  source: InputSource;
  kind: InputKind;
  /** Human-readable control name, e.g. "A", "DPadUp", "Enter", "Left click". */
  control: string;
  /** Controller slot (1–4) for controller events; undefined otherwise. */
  pad?: number;
}

/** Converts a core pad event to an input event; connect / disconnect are not input. */
export function fromPadEvent(ev: PadEvent): InputEvent | null {
  if (ev.kind === 'connected' || ev.kind === 'disconnected') return null;
  return { source: 'controller', kind: ev.kind, control: ev.control, pad: ev.pad + 1 };
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

/** The few actions the T0 dialog needs. The full action map comes in T1. */
export type UiAction = 'confirm' | 'back' | 'left' | 'right' | 'up' | 'down';

const ACTIONS: Record<InputSource, Record<string, UiAction>> = {
  controller: {
    A: 'confirm',
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
  const action = ACTIONS[ev.source][ev.control];
  if (!action) return null;
  // Confirm and back never repeat while held (one press, one action).
  if (ev.kind === 'repeat' && (action === 'confirm' || action === 'back')) return null;
  return action;
}
