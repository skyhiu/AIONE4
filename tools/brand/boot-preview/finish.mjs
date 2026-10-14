// Makes the built preview open by double-click (file://): Chromium refuses ES-module scripts from
// the file system, so the bundle (no imports or exports) becomes a classic script, and
// `import.meta.url` (used for the sound files) becomes the script's own address.
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const out = process.argv[2];
const assets = path.join(out, 'assets');
const js = readdirSync(assets).find((f) => f.endsWith('.js'));
if (!js) throw new Error('no script in the preview build');
let code = readFileSync(path.join(assets, js), 'utf8');
if (/^\s*(import|export)\s/m.test(code)) throw new Error('the bundle has imports or exports; cannot run as a classic script');
code = `(function(){var __base=document.currentScript.src;\n${code.replaceAll('import.meta.url', '__base')}\n})();`;
writeFileSync(path.join(assets, js), code);
let html = readFileSync(path.join(out, 'index.html'), 'utf8');
html = html.replace(/<script type="module" crossorigin src="([^"]+)"><\/script>/, '');
html = html.replace('</body>', `<script src="./assets/${js}"></script>\n</body>`);
writeFileSync(path.join(out, 'index.html'), html);
console.warn(`boot preview: ${out}/index.html (classic script, opens from the file system)`);
