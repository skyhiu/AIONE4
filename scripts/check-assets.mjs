// Checks the bundled assets (A3): every asset folder has its licence or credit file, and the
// Material Symbols font is byte-identical to the pinned npm package.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, readdirSync } from 'node:fs';

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

if (errors.length) {
  console.error(errors.join('\n'));
  process.exit(1);
}
console.warn('assets OK');
