// A16 (rev. bk): the assets the user provided are the only assets. Pure rules over one source
// file's text, used by scripts/check-assets.mjs (CI) and tested in tests/unit/asset-rules.test.ts.
// Each rule returns a list of findings: { rule, detail }.
//
// - fonts: only the design system's families (M PLUS Rounded 1c for text, Material Symbols
//   Rounded for icons) or their tokens (--font-ui / --font-display / --font-icons); a generic
//   family (sans-serif, …) only as a fallback after one of them; no named system font.
// - emoji: no character that shows as an emoji (Emoji_Presentation, or a variation selector 16).
// - icons: no hand-drawn <svg> outside the Window B logo files; every Material Symbols name the
//   code spells out exists in the pinned font's list.
// - glyphs: images only from Kenney Input Prompts (assets/glyphs/kenney-input-prompts) or the
//   brand files (assets/brand).
// - sounds: audio files only from the FilmCow family (assets/sounds/filmcow-menu) and the boot
//   sounds (assets/sounds/boot); no generated sound (oscillators, hand-filled buffers, speech).

export const UI_FONTS = ['M PLUS Rounded 1c', 'Material Symbols Rounded'];
export const FONT_TOKENS = ['--font-ui', '--font-display', '--font-icons'];
const GENERIC = new Set(['sans-serif', 'serif', 'monospace', 'cursive', 'fantasy', 'emoji', 'math', 'fangsong']);
const KEYWORDS = new Set(['inherit', 'initial', 'unset', 'revert', 'normal']);
const SYSTEM_FONTS = /\b(system-ui|ui-sans-serif|ui-serif|ui-monospace|ui-rounded|-apple-system|BlinkMacSystemFont|Segoe UI|Segoe UI Emoji|Arial|Helvetica|Roboto|Ubuntu|Tahoma|Verdana|Calibri|Consolas|Courier New|Times New Roman|Noto Sans)\b/;

/** Files whose inline SVG is the Window B logo (B-5), not an icon. */
export const LOGO_FILES = ['src/lib/frame/Footer.svelte', 'src/lib/boot/windowB.ts'];

/** Removes comments (CSS / JS block and line comments, HTML comments), keeping line count. */
export function stripComments(text) {
  const blank = (m) => m.replace(/[^\n]/g, ' ');
  return text
    .replace(/<!--[\s\S]*?-->/g, blank)
    .replace(/\/\*[\s\S]*?\*\//g, blank)
    .replace(/(^|[\s;{}(,])\/\/[^\n]*/g, (m, lead) => lead + blank(m.slice(lead.length)));
}

const lineOf = (text, index) => text.slice(0, index).split('\n').length;

/** The family list of a CSS `font` shorthand or `font-family` value, or a canvas font string. */
function families(value, shorthand) {
  let v = value.trim();
  if (shorthand) {
    // The families follow the size (and an optional "/ line-height"): take what follows the last
    // size-like token at the top level.
    const m = v.match(/^(.*?(?:[\d}](?:px|rem|em|%|pt|vh|vw)|\))\s*(?:\/\s*(?:[\d.]+\w*|var\([^)]*\)|calc\([^)]*\)))?)\s+(.+)$/);
    if (!m) return null;
    v = m[2];
  }
  return v.split(',').map((s) => s.trim()).filter(Boolean);
}

function checkFamilies(list, where, out) {
  if (!list || !list.length) return;
  const first = list[0];
  const allowed = (f) => {
    const name = f.replace(/^['"`]|['"`]$/g, '');
    if (UI_FONTS.includes(name)) return true;
    const tok = f.match(/^var\((--font-[a-z]+)\)$/);
    return !!tok && FONT_TOKENS.includes(tok[1]);
  };
  if (KEYWORDS.has(first)) return;
  if (!allowed(first)) {
    out.push({ rule: 'fonts', detail: `${where}: "${list.join(', ')}" starts with a family outside the design system` });
    return;
  }
  for (const f of list.slice(1)) {
    if (allowed(f) || GENERIC.has(f)) continue;
    out.push({ rule: 'fonts', detail: `${where}: "${f}" in "${list.join(', ')}" is outside the design system` });
  }
}

export function checkFonts(path, raw) {
  const text = stripComments(raw);
  const out = [];
  const css = [...text.matchAll(/(?<![\w-])(font-family|font|--font-[a-z]+)\s*:\s*([^;{}]+)/g)];
  for (const m of css) {
    const [, prop, value] = m;
    if (prop === 'font' && /^\s*(?:var\(--font-[a-z]+\)|inherit|initial|unset)\s*$/.test(value)) continue;
    if (prop === 'font' && !/\d|var\(--text|calc/.test(value)) continue; // e.g. a JS object key
    const list = families(value, prop === 'font');
    checkFamilies(list, `${path}:${lineOf(text, m.index)} ${prop}`, out);
  }
  // Script: canvas fonts and style.fontFamily, as string literals.
  for (const m of text.matchAll(/(?:\.font|fontFamily)\s*=\s*(['"])([^'"]*?)\1/g)) {
    checkFamilies(families(m[2], /\d/.test(m[2])), `${path}:${lineOf(text, m.index)} script font`, out);
  }
  for (const m of text.matchAll(/(?:\.font|fontFamily)\s*=\s*`([^`]*)`/g)) {
    checkFamilies(families(m[1], /[\d}]px/.test(m[1])), `${path}:${lineOf(text, m.index)} script font`, out);
  }
  for (const m of text.matchAll(/fontFamily\s*=\s*"((?:[^"\\]|\\.)*)"/g)) {
    checkFamilies(families(m[1], false), `${path}:${lineOf(text, m.index)} script font`, out);
  }
  for (const m of text.matchAll(new RegExp(SYSTEM_FONTS.source, 'g'))) {
    out.push({ rule: 'fonts', detail: `${path}:${lineOf(text, m.index)}: names the system font "${m[0]}"` });
  }
  // Font files only from the bundled folders.
  for (const m of text.matchAll(/url\(\s*['"]?([^'")]+\.(?:woff2?|ttf|otf))['"]?\s*\)/g)) {
    if (!/assets\/(fonts|icons\/material-symbols-rounded)\//.test(m[1]))
      out.push({ rule: 'fonts', detail: `${path}:${lineOf(text, m.index)}: font file "${m[1]}" outside assets/fonts` });
  }
  if (/fonts\.googleapis|fonts\.gstatic|use\.typekit/.test(text)) out.push({ rule: 'fonts', detail: `${path}: a remote web font` });
  return dedupe(out);
}

export function checkEmoji(path, raw) {
  const text = stripComments(raw);
  const out = [];
  for (const m of text.matchAll(/\p{Emoji_Presentation}|\uFE0F/gu)) {
    out.push({ rule: 'emoji', detail: `${path}:${lineOf(text, m.index)}: emoji U+${m[0].codePointAt(0).toString(16).toUpperCase()}` });
  }
  return out;
}

/** Material Symbols names the code spells out (literal icons; dynamic ones are checked at runtime). */
export function iconNames(raw) {
  const text = stripComments(raw);
  const names = [];
  const add = (n, index) => names.push({ name: n, line: lineOf(text, index) });
  // Markup: an element with class "icon" whose text is a literal name.
  for (const m of text.matchAll(/class="[^"]*\bicon\b[^"]*"[^<]{0,400}?>\s*([a-z0-9_]+)\s*<\//g)) add(m[1], m.index);
  // …or an expression: its string results (`{muted ? 'volume_off' : 'volume_up'}`).
  for (const m of text.matchAll(/class="[^"]*\bicon\b[^"]*"[^<]{0,400}?>\s*\{([^}<]*)\}\s*<\//g))
    for (const r of m[1].matchAll(/(?:\?|:|^)\s*['"]([a-z0-9_]+)['"]/g)) add(r[1], m.index);
  // `icon: 'name'` properties.
  for (const m of text.matchAll(/\bicon\s*:\s*['"]([a-z0-9_]+)['"]/g)) add(m[1], m.index);
  // Inside a declaration whose name says icon (a function, a const, a map): results only
  // (after return, ?, : or as an object value), never the compared values.
  for (const m of text.matchAll(/(?:function\s+\w*icon\w*|const\s+\w*icon\w*)[\s\S]{0,600}?(?=\n\s*\n|\n\s*(?:const|function|let|export)\b|$)/gi)) {
    for (const r of m[0].matchAll(/(?:return|\?|:)\s*['"]([a-z0-9_]+)['"]/g)) add(r[1], m.index + r.index);
    for (const r of m[0].matchAll(/`([a-z0-9_]+_)\$\{/g)) add(`${r[1]}*`, m.index + r.index);
  }
  // A whitespace-separated sample string named …ICON… (the font readiness check).
  for (const m of text.matchAll(/const\s+\w*ICON\w*\s*=\s*(['"])([a-z][a-z0-9_]*(?: [a-z][a-z0-9_]*)+)\1/g)) for (const n of m[2].split(' ')) add(n, m.index);
  return names;
}

export function checkIcons(path, raw, materialNames) {
  const text = stripComments(raw);
  const out = [];
  if (!LOGO_FILES.includes(path)) {
    for (const m of text.matchAll(/<svg\b|createElementNS\([^)]*svg/g)) {
      out.push({ rule: 'icons', detail: `${path}:${lineOf(text, m.index)}: a drawn SVG (icons come from Material Symbols Rounded)` });
    }
  }
  if (/from\s+['"](?:lucide|@fortawesome|@mdi|heroicons|react-icons|@tabler|bootstrap-icons|material-icons|@material-design-icons)/.test(text))
    out.push({ rule: 'icons', detail: `${path}: imports another icon set` });
  for (const { name, line } of iconNames(raw)) {
    const ok = name.endsWith('*') ? [...materialNames].some((n) => n.startsWith(name.slice(0, -1))) : materialNames.has(name);
    if (!ok) out.push({ rule: 'icons', detail: `${path}:${line}: "${name}" is not a Material Symbols Rounded name` });
  }
  return dedupe(out);
}

export function checkGlyphsAndImages(path, raw, exists) {
  const text = stripComments(raw);
  const out = [];
  for (const m of text.matchAll(/['"(]([^'"()\s]*\.(?:svg|png|jpe?g|gif|webp|ico))(?:\?url)?['")]/g)) {
    const p = m[1];
    if (p.startsWith('data:') || p.startsWith('http') || !p.includes('/')) continue; // a bare name is a label
    const ok = /assets\/glyphs\/kenney-input-prompts\//.test(p) || /assets\/brand\//.test(p);
    if (!ok) out.push({ rule: 'glyphs', detail: `${path}:${lineOf(text, m.index)}: image "${p}" outside Kenney's glyphs and the brand files` });
    else if (exists && !exists(path, p)) out.push({ rule: 'glyphs', detail: `${path}:${lineOf(text, m.index)}: "${p}" doesn't exist` });
  }
  return out;
}

export function checkSounds(path, raw, exists) {
  const text = stripComments(raw);
  const out = [];
  for (const m of text.matchAll(/['"(]([^'"()\s]*\.(?:wav|mp3|ogg|oga|flac|m4a|aac|opus))(?:\?url)?['")]/g)) {
    const p = m[1];
    if (!p.includes('/')) continue; // a bare name is a label, not a file the UI loads
    if (!/assets\/sounds\/(filmcow-menu|boot)\//.test(p)) out.push({ rule: 'sounds', detail: `${path}:${lineOf(text, m.index)}: sound "${p}" outside the FilmCow family and the boot sounds` });
    else if (exists && !exists(path, p)) out.push({ rule: 'sounds', detail: `${path}:${lineOf(text, m.index)}: "${p}" doesn't exist` });
  }
  for (const m of text.matchAll(/createOscillator|OscillatorNode|createBuffer\s*\(|new\s+AudioBuffer|getChannelData\([^)]*\)\s*\[[^\]]*\]\s*=|speechSynthesis/g)) {
    out.push({ rule: 'sounds', detail: `${path}:${lineOf(text, m.index)}: a generated sound (${m[0].trim()})` });
  }
  return out;
}

function dedupe(list) {
  const seen = new Set();
  return list.filter((f) => (seen.has(f.detail) ? false : (seen.add(f.detail), true)));
}
