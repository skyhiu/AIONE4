// Checks the bundled assets (A3): every asset folder has its licence or credit file, and the
// Material Symbols font is byte-identical to the pinned npm package.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, readdirSync } from 'node:fs';

const errors = [];
const need = (path) => {
  if (!existsSync(path)) errors.push(`missing ${path}`);
};
need('assets/glyphs/kenney-input-prompts/License.txt');
need('assets/sounds/50-menu-interface-sfx/CREDIT.txt');
need('assets/icons/material-symbols-rounded/LICENSE');
for (const family of readdirSync('assets/fonts')) need(`assets/fonts/${family}/OFL.txt`);

const sha = (p) => createHash('sha256').update(readFileSync(p)).digest('hex');
const ours = 'assets/icons/material-symbols-rounded/material-symbols-rounded.woff2';
const pkg = 'node_modules/material-symbols/material-symbols-rounded.woff2';
if (existsSync(pkg) && existsSync(ours) && sha(ours) !== sha(pkg)) {
  errors.push(`${ours} differs from the pinned npm package material-symbols`);
}
const wavs = readdirSync('assets/sounds/50-menu-interface-sfx').filter((f) => f.endsWith('.wav'));
if (wavs.length !== 50) errors.push(`expected 50 WAV files, found ${wavs.length}`);

if (errors.length) {
  console.error(errors.join('\n'));
  process.exit(1);
}
console.warn('assets OK');
