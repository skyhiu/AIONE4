import { describe, expect, it } from 'vitest';
// @ts-expect-error: a plain .mjs module without types (the CI script's rules).
import { checkEmoji, checkFonts, checkGlyphsAndImages, checkIcons, checkSounds, stripComments } from '../../scripts/asset-rules.mjs';

type Finding = { rule: string; detail: string };
const NAMES = new Set(['sports_esports', 'battery_android_frame_1', 'battery_android_frame_alert', 'power_settings_new', 'wifi']);
const yes = () => true;

describe('A16: fonts', () => {
  it('passes the design system (tokens, the bundled families, a generic fallback)', () => {
    const css = `:root { --font-ui: 'M PLUS Rounded 1c', sans-serif; --font-icons: 'Material Symbols Rounded'; }
      .a { font-family: var(--font-ui); } .b { font: 500 calc(32 * var(--px, 1px)) / 1.2 var(--font-ui); }
      .c { font: 400 var(--text-sm) var(--font-ui); }`;
    expect(checkFonts('x.css', css)).toEqual([]);
    expect(checkFonts('x.ts', 'ctx.font = `400 ${n}px "M PLUS Rounded 1c"`;')).toEqual([]);
  });
  it('fails on a system font, another family, a generic alone, a remote font', () => {
    const bad = (t: string) => (checkFonts('x.css', t) as Finding[]).length;
    expect(bad(`.a { font-family: 'Segoe UI', sans-serif; }`)).toBeGreaterThan(0);
    expect(bad(`.a { font-family: 'Nunito'; }`)).toBeGreaterThan(0);
    expect(bad(`.a { font-family: 'M PLUS Rounded 1c', system-ui, sans-serif; }`)).toBeGreaterThan(0);
    expect(bad(`.a { font: 400 1rem sans-serif; }`)).toBeGreaterThan(0);
    expect(bad(`@import url('https://fonts.googleapis.com/css2?family=Inter');`)).toBeGreaterThan(0);
    expect((checkFonts('x.ts', "g.font = 'bold 72px sans-serif';") as Finding[]).length).toBeGreaterThan(0);
  });
  it('ignores comments', () => {
    expect(checkFonts('x.css', `/* font-family: Arial; */ .a { font-family: var(--font-ui); }`)).toEqual([]);
    expect(stripComments('a // b\nc')).toBe('a     \nc');
  });
});

describe('A16: emoji', () => {
  it('fails on an emoji, passes plain symbols and comments', () => {
    expect((checkEmoji('x.svelte', '<p>Done ✅</p>') as Finding[]).length).toBe(1);
    expect((checkEmoji('x.svelte', '<p>Heart ❤️</p>') as Finding[]).length).toBeGreaterThan(0);
    expect(checkEmoji('x.ts', "const a = '→ ✕ · ↔'; // 🎮 in a comment")).toEqual([]);
  });
});

describe('A16: icons', () => {
  it('passes Material names in markup, properties and icon functions', () => {
    const svelte = `<span class="icon" data-x>sports_esports</span>
      <script>const DRAWER = [{ icon: 'power_settings_new' }];
      function batteryIcon(b) { if (b) return 'battery_android_frame_alert'; return \`battery_android_frame_\${n}\`; }
      const networkIcon = s === 'wired' ? 'wifi' : null;</script>`;
    expect(checkIcons('src/x.svelte', svelte, NAMES)).toEqual([]);
  });
  it('fails on a name the font lacks, a drawn SVG, another icon set', () => {
    expect((checkIcons('src/x.svelte', '<span class="icon">battery_android_frame_0</span>', NAMES) as Finding[]).length).toBe(1);
    expect((checkIcons('src/x.ts', "const ICONS = { game: 'not_a_symbol' };", NAMES) as Finding[]).length).toBe(1);
    expect((checkIcons('src/x.svelte', '<svg class="icon" viewBox="0 0 24 24"><path d="M1 1"/></svg>', NAMES) as Finding[]).length).toBe(1);
    expect((checkIcons('src/x.svelte', `<span class="icon">{on ? 'wifi' : 'volume_nope'}</span>`, NAMES) as Finding[]).length).toBe(1);
    expect((checkIcons('src/x.ts', "import { Play } from 'lucide-svelte';", NAMES) as Finding[]).length).toBe(1);
  });
  it('allows the Window B logo files their SVG', () => {
    expect(checkIcons('src/lib/frame/Footer.svelte', '<svg class="mark"></svg>', NAMES)).toEqual([]);
  });
});

describe('A16: glyphs and sounds', () => {
  it('passes Kenney glyphs, brand files, the FilmCow family and the boot sounds', () => {
    expect(checkGlyphsAndImages('src/a.ts', "import a from '../../assets/glyphs/kenney-input-prompts/xbox-series/xbox_button_a.svg?url'; const f = { file: 'xbox_button_a.svg' };", yes)).toEqual([]);
    expect(checkSounds('src/a.ts', "import s from '../../assets/sounds/filmcow-menu/confirm.wav?url'; import b from '../assets/sounds/boot/aione_boot_up.wav?url';", yes)).toEqual([]);
  });
  it('fails on other images, other sounds, generated sounds, missing files', () => {
    expect((checkGlyphsAndImages('src/a.ts', "import a from '../assets/icons/pad.png';", yes) as Finding[]).length).toBe(1);
    expect((checkGlyphsAndImages('src/a.ts', "import a from '../assets/glyphs/kenney-input-prompts/x/none.svg';", () => false) as Finding[]).length).toBe(1);
    expect((checkSounds('src/a.ts', "import s from '../assets/sounds/other/ding.wav?url';", yes) as Finding[]).length).toBe(1);
    expect((checkSounds('src/a.ts', 'const o = ctx.createOscillator();', yes) as Finding[]).length).toBe(1);
    expect((checkSounds('src/a.ts', 'const b = ctx.createBuffer(1, 100, 48000);', yes) as Finding[]).length).toBe(1);
  });
});
