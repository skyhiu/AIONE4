// Checks the bundled assets (A3): every asset folder has its licence or credit file, and the
// Material Symbols font is byte-identical to the pinned npm package.
// A16 (rev. bk): the UI uses only the user's assets: the design system's fonts, no emoji,
// Material Symbols Rounded icons (every spelled-out name exists), Kenney glyphs, the FilmCow
// family and the boot sounds (rules in scripts/asset-rules.mjs, tested in
// tests/unit/asset-rules.test.ts). Each finding is a GitHub annotation in CI.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { checkEmoji, checkFonts, checkGlyphsAndImages, checkIcons, checkSounds } from './asset-rules.mjs';

const errors = [];
const need = (path) => {
  if (!existsSync(path)) errors.push(`missing ${path}`);
};
need('assets/glyphs/kenney-input-prompts/License.txt');
// FilmCow menu sounds (B-1 rev. h) and the Window B boot sounds (B-5 final; the 50 Menu
// Interface SFX pack was removed in T1.2, B-1 rev. i). Credits in docs/THIRD_PARTY_ASSETS.md.
for (const f of ['row_move.wav', 'drawer_move.wav', 'edge_bump.wav', 'confirm.wav', 'back.wav', 'error.wav', 'wrap.wav', 'options_open.wav', 'options_close.wav', 'scroll_tick.wav', 'launch.wav', 'family.json'])
  need(`assets/sounds/filmcow-menu/${f}`);
for (const f of ['aione_boot_up.wav', 'aione_boot_down.wav']) need(`assets/sounds/boot/${f}`);
for (const f of ['aione_mark_white.svg', 'aione_mark_color.svg', 'aione_app_icon.svg']) need(`assets/brand/${f}`);
need('assets/icons/material-symbols-rounded/LICENSE');
for (const family of readdirSync('assets/fonts')) need(`assets/fonts/${family}/OFL.txt`);

const sha = (p) => createHash('sha256').update(readFileSync(p)).digest('hex');
const ours = 'assets/icons/material-symbols-rounded/material-symbols-rounded.woff2';
const pkg = 'node_modules/material-symbols/material-symbols-rounded.woff2';
if (existsSync(pkg) && existsSync(ours) && sha(ours) !== sha(pkg)) {
  errors.push(`${ours} differs from the pinned npm package material-symbols`);
}
if (existsSync('assets/sounds/50-menu-interface-sfx')) errors.push('the 50 Menu Interface SFX pack is back (removed in T1.2)');

// ---- A16: what the UI uses ----
const walk = (dir) =>
  readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    return statSync(p).isDirectory() ? walk(p) : [p];
  });
const sources = [...walk('src').filter((p) => /\.(svelte|ts|css|js)$/.test(p)), 'index.html'].map((p) => p.split('\\').join('/'));
const material = new Set(
  [...readFileSync('node_modules/material-symbols/index.d.ts', 'utf8').matchAll(/^\s*"([a-z0-9_]+)",?$/gm)].map((m) => m[1]),
);
if (material.size < 1000) errors.push(`the Material Symbols name list couldn't be read (${material.size} names)`);
const exists = (from, p) => existsSync(resolve(dirname(from), p.replace(/\?url$/, '')));
const findings = [];
for (const path of sources) {
  const text = readFileSync(path, 'utf8');
  findings.push(
    ...checkFonts(path, text),
    ...checkEmoji(path, text),
    ...checkIcons(path, text, material),
    ...checkGlyphsAndImages(path, text, exists),
    ...checkSounds(path, text, exists),
  );
}
// Asset folders hold only the user's sets.
const only = (dir, allowed) => {
  for (const n of readdirSync(dir)) if (!allowed.includes(n)) findings.push({ rule: 'assets', detail: `${dir}/${n} isn't one of the user's asset sets` });
};
only('assets/glyphs', ['kenney-input-prompts']);
only('assets/sounds', ['filmcow-menu', 'boot']);
for (const f of findings) {
  const [file, line] = f.detail.split(':');
  const where = /^\d+$/.test(line ?? '') ? `file=${file},line=${line}` : `file=${file}`;
  if (process.env.GITHUB_ACTIONS) process.stdout.write(`::error ${where},title=A16 assets (${f.rule})::${f.detail}\n`);
  errors.push(`A16 ${f.rule}: ${f.detail}`);
}

if (errors.length) {
  console.error(errors.join('\n'));
  process.exit(1);
}
console.warn('assets OK');
