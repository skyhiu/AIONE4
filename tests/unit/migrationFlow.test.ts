import { describe, expect, it } from 'vitest';
import type { CopyResult, MigrationStatus } from '../../src/lib/core/types';
import {
  BUTTON_COPY,
  BUTTON_SKIP,
  choose,
  copyFraction,
  describeCounts,
  describeSkipped,
  flowAction,
  flowDone,
  flowProgress,
  initialFlow,
} from '../../src/lib/migrationFlow';

const offer: MigrationStatus = {
  state: 'offer',
  source: 'C:\\AIONE',
  files: 10,
  bytes: 1000,
  library: { games: 2, hidden: 0, favorites: 1, folders: 0, ignored: [], sources: [] },
  notice: null,
};

const result: CopyResult = {
  ok: true,
  outcome: 'copied',
  destination: 'D',
  filesCopied: 10,
  filesFailed: 0,
  bytesCopied: 1000,
  library: { games: 2, hidden: 0, favorites: 1, folders: 0, ignored: [], sources: [] },
  sourceUnchanged: true,
  notes: [],
};

describe('copy offer flow', () => {
  it('shows the question only when the core offers it', () => {
    expect(initialFlow(offer).phase).toBe('ask');
    expect(initialFlow({ state: 'noSource', source: 'x' }).phase).toBe('hidden');
    expect(initialFlow({ state: 'alreadyHandled', decision: 'declined', notice: null }).phase).toBe('hidden');
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

  it('B = Not now: closes without deciding (B-2)', () => {
    const [s, effect] = flowAction(initialFlow(offer), 'back');
    expect(effect.type).toBe('notNow');
    expect(s.phase).toBe('hidden');
    // From "Don't copy" too: B never declines.
    const [onSkip] = flowAction(initialFlow(offer), 'right');
    const [, effect2] = flowAction(onSkip, 'back');
    expect(effect2.type).toBe('notNow');
  });

  it('"Don\'t copy" needs A (confirm) on it', () => {
    const [onSkip] = flowAction(initialFlow(offer), 'right');
    expect(onSkip.phase === 'ask' && onSkip.focus).toBe(BUTTON_SKIP);
    const [s, effect] = flowAction(onSkip, 'confirm');
    expect(effect).toEqual({ type: 'decide', accept: false });
    expect(s.phase).toBe('hidden');
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

describe('copy progress and wording', () => {
  it('one value for bar and text: bytes done / bytes total (D-003)', () => {
    expect(copyFraction(null)).toBe(0);
    expect(copyFraction({ filesDone: 1, filesTotal: 4, bytesDone: 50, bytesTotal: 200 })).toBe(0.25);
    expect(copyFraction({ filesDone: 3, filesTotal: 4, bytesDone: 0, bytesTotal: 0 })).toBe(0.75);
    expect(copyFraction({ filesDone: 9, filesTotal: 4, bytesDone: 900, bytesTotal: 200 })).toBe(1);
  });

  it('plain wording for skipped entries (B-2 rev. d)', () => {
    expect(describeSkipped([{ name: '_Removed', reason: 'skipped by name (starts with "_")' }])).toEqual([
      'Skipped: _Removed (not a game)',
    ]);
    expect(
      describeSkipped([
        { name: '.b', reason: 'skipped by name (starts with ".")' },
        { name: 'X', reason: 'no game.json' },
        { name: 'notes.txt', reason: 'not a folder' },
        { name: 'Y', reason: 'game.json could not be read: EOF' },
        // alpha.1 records used "ignored by name".
        { name: '_Old', reason: 'ignored by name (starts with "_")' },
      ]),
    ).toEqual([
      'Skipped: .b (not a game)',
      'Skipped: X (no game.json)',
      'Skipped: notes.txt (not a game folder)',
      "Skipped: Y (its game.json could not be read)",
      'Skipped: _Old (not a game)',
    ]);
  });

  it('result dialogs: A or B closes (B-2 rev. d)', () => {
    for (const action of ['confirm', 'back'] as const) {
      const [s] = flowAction({ phase: 'done', result }, action);
      expect(s.phase).toBe('hidden');
    }
  });

  it('a notice from the core shows once and A or B closes it (B-6)', () => {
    const s = initialFlow({ state: 'alreadyHandled', decision: 'copied', notice: 'could not read' });
    expect(s).toEqual({ phase: 'notice', message: 'could not read' });
    expect(flowAction(s, 'back')[0].phase).toBe('hidden');
    expect(flowAction(s, 'left')[0]).toEqual(s);
  });

  it('counts read the same as the log, "favorites" spelled one way', () => {
    expect(describeCounts({ games: 10, hidden: 0, favorites: 0, folders: 0 })).toBe(
      '10 games · 0 hidden · 0 favorites · 0 folders',
    );
    expect(describeCounts({ games: 1, hidden: 1, favorites: 1, folders: 1 })).toBe('1 game · 1 hidden · 1 favorite · 1 folder');
  });
});
