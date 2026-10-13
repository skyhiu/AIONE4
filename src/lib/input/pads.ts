// Which controller slots are connected, for the header's controller icon. D-002: the UI asks
// the core at start and merges the answer with connect / disconnect events by sequence number,
// so an older answer never overwrites a newer event.

export interface PadState {
  /** Connected slots, 1–4. */
  pads: number[];
  /** Core sequence number of the state shown. */
  seq: number;
}

export function initialPads(): PadState {
  return { pads: [], seq: 0 };
}

/** A connect / disconnect event (slot 1–4) with the core's sequence number. */
export function applyPadConnection(state: PadState, slot: number, connected: boolean, seq: number): PadState {
  if (seq <= state.seq) return state;
  const set = new Set(state.pads);
  if (connected) set.add(slot);
  else set.delete(slot);
  return { pads: [...set].sort((a, b) => a - b), seq };
}

/** The core's answer to "which slots are connected?" (slots 1–4), as of sequence `seq`. */
export function applyPadSlots(state: PadState, slots: number[], seq: number): PadState {
  if (seq < state.seq) return state;
  return { pads: [...new Set(slots)].sort((a, b) => a - b), seq };
}
