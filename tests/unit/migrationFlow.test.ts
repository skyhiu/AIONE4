import { describe, expect, it } from 'vitest';
import type { CopyResult, MigrationStatus } from '../../src/lib/core/types';
import { BUTTON_COPY, BUTTON_SKIP, choose, flowAction, flowDone, flowProgress, initialFlow } from '../../src/lib/migrationFlow';

const offer: MigrationStatus = {
  state: 'offer',
  source: 'C:\\AIONE',
  files: 10,
  bytes: 1000,
  library: { games: 2, hidden: 0, favorites: 1, ignored: [] },
};

const result: CopyResult = {
  ok: true,
  outcome: 'copied',
  destination: 'D',
  filesCopied: 10,
  filesFailed: 0,
  bytesCopied: 1000,
  library: { games: 2, hidden: 0, favorites: 1, ignored: [] },
  sourceUnchanged: true,
  notes: [],
};

describe('copy offer flow', () => {
  it('shows the question only when the core offers it', () => {
    expect(initialFlow(offer).phase).toBe('ask');
    expect(initialFlow({ state: 'noSource', source: 'x' }).phase).toBe('hidden');
    expect(initialFlow({ state: 'alreadyHandled', decision: 'declined' }).phase).toBe('hidden');
  });

  it('starts on Copy and stops at the ends', () => {
    let s = initialFlow(offer);
    expect(s.phase === 'ask' && s.focus).toBe(BUTTON_COPY);
    [s] = flowAction(s, 'left');
    expect(s.phase === 'ask' && s.focus).toBe(BUTTON_COPY);
    [s] = flowAction(s, 'right');
    [s] = flowAction(s, 'right');
    expect(s.phase === 'ask' && s.focus).toBe(BUTTON_SKIP);
  });

  it('B never declines by itself; it only moves focus to "Don\'t copy"', () => {
    const [s, effect] = flowAction(initialFlow(offer), 'back');
    expect(effect.type).toBe('none');
    expect(s.phase === 'ask' && s.focus).toBe(BUTTON_SKIP);
  });

  it('confirm on Copy starts the copy; progress and result follow', () => {
    let [s, effect] = flowAction(initialFlow(offer), 'confirm');
    expect(effect).toEqual({ type: 'decide', accept: true });
    expect(s.phase).toBe('copying');
    s = flowProgress(s, { filesDone: 5, filesTotal: 10, bytesDone: 500, bytesTotal: 1000 });
    expect(s.phase === 'copying' && s.progress?.filesDone).toBe(5);
    // Input during the copy does nothing.
    [s, effect] = flowAction(s, 'confirm');
    expect(effect.type).toBe('none');
    s = flowDone(s, result);
    expect(s.phase).toBe('done');
    [s] = flowAction(s, 'confirm');
    expect(s.phase).toBe('hidden');
  });

  it('declining hides the dialog and reports the decision once', () => {
    const [s, effect] = choose(initialFlow(offer), false);
    expect(s.phase).toBe('hidden');
    expect(effect).toEqual({ type: 'decide', accept: false });
    const [, again] = choose(s, false);
    expect(again.type).toBe('none');
  });
});
