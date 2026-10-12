// The one-time "copy your previous AIONE data" dialog as a small state machine (pure, tested).
import type { CopyProgress, CopyResult, MigrationStatus } from './core/types';
import type { UiAction } from './input/events';

export type OfferStatus = Extract<MigrationStatus, { state: 'offer' }>;

export type FlowState =
  | { phase: 'hidden' }
  | { phase: 'ask'; offer: OfferStatus; focus: 0 | 1 }
  | { phase: 'copying'; offer: OfferStatus; progress: CopyProgress | null }
  | { phase: 'done'; result: CopyResult };

/** What the UI must do after an action (the reducer itself stays pure). */
export type FlowEffect = { type: 'decide'; accept: boolean } | { type: 'none' };

export const BUTTON_COPY = 0;
export const BUTTON_SKIP = 1;

export function initialFlow(status: MigrationStatus): FlowState {
  return status.state === 'offer' ? { phase: 'ask', offer: status, focus: BUTTON_COPY } : { phase: 'hidden' };
}

export function flowAction(state: FlowState, action: UiAction): [FlowState, FlowEffect] {
  switch (state.phase) {
    case 'ask':
      switch (action) {
        // Buttons sit side by side; focus stops at the ends (no wrap).
        case 'left':
          return [{ ...state, focus: BUTTON_COPY }, { type: 'none' }];
        case 'right':
          return [{ ...state, focus: BUTTON_SKIP }, { type: 'none' }];
        // B only moves focus to "Don't copy": the question is asked once, so declining is
        // always an explicit choice, never an accidental B press.
        case 'back':
          return [{ ...state, focus: BUTTON_SKIP }, { type: 'none' }];
        case 'confirm':
          return choose(state, state.focus === BUTTON_COPY);
        default:
          return [state, { type: 'none' }];
      }
    case 'done':
      return action === 'confirm' || action === 'back' ? [{ phase: 'hidden' }, { type: 'none' }] : [state, { type: 'none' }];
    case 'copying':
    case 'hidden':
      return [state, { type: 'none' }];
  }
}

/** A direct choice (mouse click, or confirm on a focused button). */
export function choose(state: FlowState, accept: boolean): [FlowState, FlowEffect] {
  if (state.phase !== 'ask') return [state, { type: 'none' }];
  if (accept) return [{ phase: 'copying', offer: state.offer, progress: null }, { type: 'decide', accept: true }];
  return [{ phase: 'hidden' }, { type: 'decide', accept: false }];
}

export function flowProgress(state: FlowState, progress: CopyProgress): FlowState {
  return state.phase === 'copying' ? { ...state, progress } : state;
}

export function flowDone(state: FlowState, result: CopyResult): FlowState {
  return state.phase === 'copying' ? { phase: 'done', result } : state;
}
