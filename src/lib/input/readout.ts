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
  /** Core sequence number of the connection state shown (D-002: query and events merge in
   *  order, so an older answer never overwrites a newer event). */
  padSeq: number;
}

export function initialReadout(): ReadoutState {
  return {
    last: null,
    presses: 0,
    repeats: 0,
    releases: 0,
    bySource: { controller: 0, keyboard: 0, mouse: 0 },
    pads: [],
    padSeq: 0,
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

/** A connect / disconnect event (slot 1–4) with the core's sequence number. */
export function applyPadConnection(state: ReadoutState, slot: number, connected: boolean, seq: number): ReadoutState {
  if (seq <= state.padSeq) return state;
  const set = new Set(state.pads);
  if (connected) set.add(slot);
  else set.delete(slot);
  return { ...state, pads: [...set].sort((a, b) => a - b), padSeq: seq };
}

/** The core's answer to "which slots are connected?" (slots 1–4), as of sequence `seq`.
 *  Ignored if an event newer than that answer was already applied. */
export function applyPadSlots(state: ReadoutState, slots: number[], seq: number): ReadoutState {
  if (seq < state.padSeq) return state;
  return { ...state, pads: [...new Set(slots)].sort((a, b) => a - b), padSeq: seq };
}

export function describeLast(state: ReadoutState): string {
  if (!state.last) return 'No input yet';
  const where =
    state.last.source === 'controller' ? `controller ${state.last.pad ?? '?'}` : state.last.source;
  return `${state.last.control} (${where})`;
}
