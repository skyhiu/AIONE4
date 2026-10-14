import { describe, expect, it } from 'vitest';
import { describeInput, describeKeyRelease, fromKeyboard, fromMouseButton, fromPadEvent, fromWheel, keyName, toAction } from '../../src/lib/input/events';
import { applyPadConnection, applyPadSlots, initialPads } from '../../src/lib/input/pads';
import { isBlockedBrowserKey, isRestartShortcut } from '../../src/lib/input/shortcuts';

describe('controller slots (D-002)', () => {
  it('tracks connected controller slots', () => {
    let s = initialPads();
    s = applyPadConnection(s, 2, true, 1);
    s = applyPadConnection(s, 1, true, 2);
    s = applyPadConnection(s, 2, false, 3);
    expect(s.pads).toEqual([1]);
  });

  it('the slots query shows a pad connected before the UI listened', () => {
    const s = applyPadSlots(initialPads(), [1], 7);
    expect(s).toEqual({ pads: [1], seq: 7 });
  });

  it('an older query answer never overwrites a newer event', () => {
    let s = applyPadSlots(initialPads(), [1], 7);
    s = applyPadConnection(s, 1, false, 9);
    s = applyPadSlots(s, [1], 7);
    expect(s.pads).toEqual([]);
    s = applyPadConnection(s, 1, true, 8);
    expect(s.pads).toEqual([]);
  });
});

describe('input events', () => {
  it('converts pad events and ignores connection events', () => {
    expect(fromPadEvent({ kind: 'down', control: 'B', pad: 0, seq: 1, atMs: 1000 })).toEqual({
      source: 'controller',
      kind: 'down',
      control: 'B',
      pad: 1,
      seq: 1,
      atMs: 1000,
    });
    expect(fromPadEvent({ kind: 'connected', control: '', pad: 0, seq: 2, atMs: 0 })).toBeNull();
  });

  it('names keys', () => {
    expect(keyName({ key: 'ArrowUp', code: 'ArrowUp' })).toBe('Up');
    expect(keyName({ key: 'a', code: 'KeyA' })).toBe('A');
    expect(keyName({ key: ' ', code: 'Space' })).toBe('Space');
    expect(keyName({ key: 'Enter', code: 'Enter' })).toBe('Enter');
    expect(keyName({ key: 'Unidentified', code: 'IntlRo' })).toBe('IntlRo');
  });

  it('marks keyboard auto-repeat as repeat', () => {
    expect(fromKeyboard({ key: 'ArrowLeft', code: 'ArrowLeft', repeat: true }, 'keydown').kind).toBe('repeat');
    expect(fromKeyboard({ key: 'ArrowLeft', code: 'ArrowLeft', repeat: false }, 'keyup').kind).toBe('up');
  });

  it('names mouse buttons and wheel', () => {
    expect(fromMouseButton(0, 'mousedown').control).toBe('Left click');
    expect(fromMouseButton(2, 'mouseup')).toEqual({ source: 'mouse', kind: 'up', control: 'Right click' });
    expect(fromWheel(-120)?.control).toBe('Wheel up');
    expect(fromWheel(0)).toBeNull();
  });

  it('maps controller and keyboard to the same actions', () => {
    expect(toAction({ source: 'controller', kind: 'down', control: 'A', pad: 1 })).toBe('confirm');
    expect(toAction({ source: 'keyboard', kind: 'down', control: 'Enter' })).toBe('confirm');
    expect(toAction({ source: 'controller', kind: 'down', control: 'LStickLeft', pad: 1 })).toBe('left');
    expect(toAction({ source: 'keyboard', kind: 'repeat', control: 'Right' })).toBe('right');
  });

  it('never acts on a release, and confirm / back never repeat', () => {
    expect(toAction({ source: 'controller', kind: 'up', control: 'A', pad: 1 })).toBeNull();
    expect(toAction({ source: 'controller', kind: 'repeat', control: 'A', pad: 1 })).toBeNull();
    expect(toAction({ source: 'keyboard', kind: 'repeat', control: 'Esc' })).toBeNull();
  });
});

describe('input log line', () => {
  it('carries the core sequence number, the action and what it did', () => {
    expect(
      describeInput({ source: 'controller', kind: 'down', control: 'DPadRight', pad: 1, seq: 57 }, 'right', 'focus 2/10, sound move'),
    ).toBe('input down DPadRight (controller 1 seq 57) -> action right, focus 2/10, sound move');
    expect(describeInput({ source: 'keyboard', kind: 'repeat', control: 'Down' }, null)).toBe(
      'input repeat Down (keyboard) -> no action',
    );
  });
  it('logs key releases like the pad (D-006)', () => {
    expect(describeKeyRelease('Enter', 132, 0)).toBe('input up Enter (keyboard), held 132 ms, 0 repeats');
  });
});

describe('shortcuts', () => {
  const k = (key: string, mods: Partial<{ ctrlKey: boolean; shiftKey: boolean; altKey: boolean }> = {}) => ({
    key,
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    ...mods,
  });
  it('blocks browser keys', () => {
    expect(isBlockedBrowserKey(k('F5'))).toBe(true);
    expect(isBlockedBrowserKey(k('r', { ctrlKey: true }))).toBe(true);
    expect(isBlockedBrowserKey(k('ArrowLeft', { altKey: true }))).toBe(true);
    expect(isBlockedBrowserKey(k('ArrowLeft'))).toBe(false);
    expect(isBlockedBrowserKey(k('Enter'))).toBe(false);
  });
  it('recognises the T0 restart shortcut', () => {
    expect(isRestartShortcut(k('R', { ctrlKey: true, shiftKey: true }))).toBe(true);
    expect(isRestartShortcut(k('r', { ctrlKey: true }))).toBe(false);
  });
});

describe('B-10 (rev. k): what counts as the first press after the mouse', () => {
  it('pad buttons and directions, and the Home keys count; modifiers and other keys do not', async () => {
    const { countsAsHomeInput, inputWhere } = await import('../../src/lib/input/events');
    for (const control of ['A', 'B', 'DPadRight', 'LStickUp', 'Menu'])
      expect(countsAsHomeInput({ source: 'controller', kind: 'down', control, pad: 1, seq: 1 })).toBe(true);
    for (const control of ['Up', 'Down', 'Left', 'Right', 'Enter', 'Space', 'Esc', 'Backspace'])
      expect(countsAsHomeInput({ source: 'keyboard', kind: 'down', control })).toBe(true);
    for (const control of ['Alt', 'Control', 'Shift', 'Meta', 'Tab', 'a', 'F4'])
      expect(countsAsHomeInput({ source: 'keyboard', kind: 'down', control }), control).toBe(false);
    expect(inputWhere({ source: 'controller', kind: 'down', control: 'A', pad: 1, seq: 7 })).toBe('controller 1 seq 7');
    expect(inputWhere({ source: 'keyboard', kind: 'down', control: 'Enter' })).toBe('keyboard');
  });
});
