import { describe, expect, it } from 'vitest';
import { fromKeyboard, fromMouseButton, fromPadEvent, fromWheel, keyName, toAction } from '../../src/lib/input/events';
import { applyInput, applyPadConnection, describeLast, initialReadout } from '../../src/lib/input/readout';
import { isBlockedBrowserKey, isRestartShortcut } from '../../src/lib/input/shortcuts';

describe('input readout (T0)', () => {
  it('counts one press once, with its release separately', () => {
    let s = initialReadout();
    s = applyInput(s, { source: 'controller', kind: 'down', control: 'A', pad: 1 });
    s = applyInput(s, { source: 'controller', kind: 'up', control: 'A', pad: 1 });
    expect(s.presses).toBe(1);
    expect(s.releases).toBe(1);
    expect(s.repeats).toBe(0);
    expect(describeLast(s)).toBe('A (controller 1)');
  });

  it('counts repeats apart from presses', () => {
    let s = initialReadout();
    s = applyInput(s, { source: 'keyboard', kind: 'down', control: 'Down' });
    s = applyInput(s, { source: 'keyboard', kind: 'repeat', control: 'Down' });
    s = applyInput(s, { source: 'keyboard', kind: 'repeat', control: 'Down' });
    expect(s.presses).toBe(1);
    expect(s.repeats).toBe(2);
    expect(s.bySource).toEqual({ controller: 0, keyboard: 1, mouse: 0 });
  });

  it('tracks connected controller slots', () => {
    let s = initialReadout();
    s = applyPadConnection(s, 2, true);
    s = applyPadConnection(s, 1, true);
    s = applyPadConnection(s, 2, false);
    expect(s.pads).toEqual([1]);
  });

  it('starts empty', () => {
    expect(describeLast(initialReadout())).toBe('No input yet');
  });
});

describe('input events', () => {
  it('converts pad events and ignores connection events', () => {
    expect(fromPadEvent({ kind: 'down', control: 'B', pad: 0, seq: 1 })).toEqual({
      source: 'controller',
      kind: 'down',
      control: 'B',
      pad: 1,
    });
    expect(fromPadEvent({ kind: 'connected', control: '', pad: 0, seq: 2 })).toBeNull();
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
