// The one-time "copy your previous AIONE data" dialog as a small state machine (pure, tested).
import type { CopyProgress, CopyResult, MigrationStatus } from './core/types';
import type { UiAction } from './input/events';

export type OfferStatus = Extract<MigrationStatus, { state: 'offer' }>;

export type FlowState =
  | { phase: 'hidden' }
  /** B-6: a plain message (e.g. the copy record could not be read); A OK, B closes. */
  | { phase: 'notice'; message: string }
  | { phase: 'ask'; offer: OfferStatus; focus: 0 | 1 }
  | { phase: 'copying'; offer: OfferStatus; progress: CopyProgress | null }
  | { phase: 'done'; result: CopyResult };

/** What the UI must do after an action (the reducer itself stays pure).
 *  `notNow`: the question closes without a decision and returns at the next start (B-2). */
export type FlowEffect = { type: 'decide'; accept: boolean } | { type: 'notNow' } | { type: 'none' };

export const BUTTON_COPY = 0;
export const BUTTON_SKIP = 1;

export function initialFlow(status: MigrationStatus): FlowState {
  if (status.state === 'offer') return { phase: 'ask', offer: status, focus: BUTTON_COPY };
  if (status.state === 'alreadyHandled' && status.notice) return { phase: 'notice', message: status.notice };
  return { phase: 'hidden' };
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
        // B = "Not now" (B-2): closes without deciding; nothing is recorded, so the question
        // returns at the next start. "Don't copy" is final and needs A or a click.
        case 'back':
          return [{ phase: 'hidden' }, { type: 'notNow' }];
        case 'confirm':
          return choose(state, state.focus === BUTTON_COPY);
        default:
          return [state, { type: 'none' }];
      }
    case 'done':
    case 'notice':
      // Nothing is left to decide: A or B closes (B-2 rev. d).
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

/** The share of the copy done, 0–1: the one value that drives both the bar and the
 *  percentage text (D-003). Bytes, because file sizes differ widely. */
export function copyFraction(progress: CopyProgress | null): number {
  if (!progress) return 0;
  if (progress.bytesTotal > 0) return Math.min(1, Math.max(0, progress.bytesDone / progress.bytesTotal));
  if (progress.filesTotal > 0) return Math.min(1, Math.max(0, progress.filesDone / progress.filesTotal));
  return 0;
}

/** Plain wording for skipped entries (A13, B-2 rev. d): "Skipped: _Removed (not a game)". */
export function describeSkipped(ignored: { name: string; reason: string }[]): string[] {
  return ignored.map((i) => `Skipped: ${i.name} (${plainReason(i.reason)})`);
}

function plainReason(reason: string): string {
  if (reason.startsWith('skipped by name') || reason.startsWith('ignored by name')) return 'not a game';
  if (reason === 'not a folder') return 'not a game folder';
  if (reason.startsWith('game.json could not be read')) return 'its game.json could not be read';
  return reason;
}

/** "10 games · 0 hidden · 0 favorites · 0 folders" (same words as the log). */
export function describeCounts(c: { games: number; hidden: number; favorites: number; folders: number }): string {
  const n = (v: number, one: string, many: string) => `${v.toLocaleString('en-US')} ${v === 1 ? one : many}`;
  return [n(c.games, 'game', 'games'), `${c.hidden} hidden`, n(c.favorites, 'favorite', 'favorites'), n(c.folders, 'folder', 'folders')].join(' · ');
}

export function flowProgress(state: FlowState, progress: CopyProgress): FlowState {
  return state.phase === 'copying' ? { ...state, progress } : state;
}

export function flowDone(state: FlowState, result: CopyResult): FlowState {
  return state.phase === 'copying' ? { phase: 'done', result } : state;
}
