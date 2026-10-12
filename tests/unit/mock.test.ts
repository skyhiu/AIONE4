import { describe, expect, it } from 'vitest';
import { createMock, createMockCore } from '../../src/lib/core/mock';
import type { PadEvent } from '../../src/lib/core/types';

// The mocked core must behave like the real one (A10, D-002).
describe('mocked core', () => {
  it('a pad connected before the UI subscribes is only visible through padSlots', async () => {
    const core = createMockCore('?pad=connected');
    const seen: PadEvent[] = [];
    await core.onPad((e) => seen.push(e));
    expect(seen).toEqual([]);
    expect(await core.padSlots()).toEqual({ slots: [0], seq: 1 });
  });

  it('a pad connects before it sends presses', async () => {
    const { core, control } = createMock('');
    const seen: string[] = [];
    await core.onPad((e) => seen.push(`${e.kind}:${e.control}:${e.seq}`));
    expect(await core.padSlots()).toEqual({ slots: [], seq: 0 });
    control.pad('A');
    control.pad('A', 'up');
    expect(seen).toEqual(['connected::1', 'down:A:2', 'up:A:3']);
    expect(await core.padSlots()).toEqual({ slots: [0], seq: 1 });
    control.disconnect();
    expect(await core.padSlots()).toEqual({ slots: [], seq: 4 });
  });
});

describe('mocked core: stored records (B-6 fixtures)', () => {
  it.each(['alpha1', 'alpha2'])('the %s record reads as copied: no question', async (record) => {
    const status = await createMockCore(`?migration=offer&record=${record}`).migrationStatus();
    expect(status).toEqual({ state: 'alreadyHandled', decision: 'copied', notice: null });
  });

  it('an unreadable record with a finished copy: no question, a notice', async () => {
    const status = await createMockCore('?migration=offer&record=unreadable&copy=finished').migrationStatus();
    expect(status.state).toBe('alreadyHandled');
    expect(status.state === 'alreadyHandled' && status.notice).toContain('could not read');
  });

  it('an unreadable record without a copy: the question with a notice', async () => {
    const status = await createMockCore('?record=unreadable').migrationStatus();
    expect(status.state === 'offer' && status.notice).toContain('asks again');
  });
});
