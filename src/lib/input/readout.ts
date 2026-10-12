// The T0 input readout (this stage only): shows the last press and counts presses, repeats
// and releases separately, so one physical press can be seen to count exactly once.
import type { InputEvent, InputSource } from './events';

export interface ReadoutState {
  last: { control: string; source: InputSource; pad?: number } | null;
  presses: number;
  repeats: number;
  releases: number;
  /** Presses per source, to spot a second source mirroring the same press. */
  bySource: Record<InputSource, number>;
  /** Controller slots currently connected (1–4). */
  pads: number[];
}

export function initialReadout(): ReadoutState {
  return {
    last: null,
    presses: 0,
    repeats: 0,
    releases: 0,
    bySource: { controller: 0, keyboard: 0, mouse: 0 },
    pads: [],
  };
}

export function applyInput(state: ReadoutState, ev: InputEvent): ReadoutState {
  switch (ev.kind) {
    case 'down': {
      const last: ReadoutState['last'] =
        ev.pad === undefined
          ? { control: ev.control, source: ev.source }
          : { control: ev.control, source: ev.source, pad: ev.pad };
      return {
        ...state,
        last,
        presses: state.presses + 1,
        bySource: { ...state.bySource, [ev.source]: state.bySource[ev.source] + 1 },
      };
    }
    case 'repeat':
      return { ...state, repeats: state.repeats + 1 };
    case 'up':
      return { ...state, releases: state.releases + 1 };
  }
}

export function applyPadConnection(state: ReadoutState, slot: number, connected: boolean): ReadoutState {
  const set = new Set(state.pads);
  if (connected) set.add(slot);
  else set.delete(slot);
  return { ...state, pads: [...set].sort((a, b) => a - b) };
}

export function describeLast(state: ReadoutState): string {
  if (!state.last) return 'No input yet';
  const where =
    state.last.source === 'controller' ? `controller ${state.last.pad ?? '?'}` : state.last.source;
  return `${state.last.control} (${where})`;
}
