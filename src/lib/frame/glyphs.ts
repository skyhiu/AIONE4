// Footer prompt glyphs (docs/GLYPH_MAPPING.md): Kenney Input Prompts 1.5 (CC0), the pack's own
// SVG files, unchanged. Monochrome glyphs are drawn as a mask tinted in the theme's text colour.
//
// Device: the glyph set follows the last controller or keyboard input. XInput cannot tell the
// pad's make, so pads use the Xbox (plain) set. The mouse is not a glyph device: in mouse mode
// the prompts keep the last controller / keyboard set.
import xboxA from '../../../assets/glyphs/kenney-input-prompts/xbox-series/xbox_button_a.svg?url';
import xboxB from '../../../assets/glyphs/kenney-input-prompts/xbox-series/xbox_button_b.svg?url';
import keyEnter from '../../../assets/glyphs/kenney-input-prompts/keyboard-mouse/keyboard_enter.svg?url';
import keyEscape from '../../../assets/glyphs/kenney-input-prompts/keyboard-mouse/keyboard_escape.svg?url';

export type GlyphDevice = 'xbox' | 'keyboard';
export type PromptButton = 'confirm' | 'back';

export const GLYPHS: Record<GlyphDevice, Record<PromptButton, { url: string; file: string }>> = {
  xbox: {
    confirm: { url: xboxA, file: 'xbox_button_a.svg' },
    back: { url: xboxB, file: 'xbox_button_b.svg' },
  },
  keyboard: {
    confirm: { url: keyEnter, file: 'keyboard_enter.svg' },
    back: { url: keyEscape, file: 'keyboard_escape.svg' },
  },
};

export interface Prompt {
  button: PromptButton;
  label: string;
}

/** The glyph set after an input event (mouse keeps the previous set). */
export function deviceAfter(prev: GlyphDevice, source: 'controller' | 'keyboard' | 'mouse'): GlyphDevice {
  if (source === 'controller') return 'xbox';
  if (source === 'keyboard') return 'keyboard';
  return prev;
}
