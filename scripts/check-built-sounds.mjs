// D-009: checks the REAL production build (dist/, made by `npm run build`), not the mock.
// Fails when a menu sound is inlined as a data: URL (the app's CSP blocks fetching those),
// when a referenced sound file is missing, or when the CSP would not let fetch() reach it.
// The browser half (fetch + decode under the real CSP) is tests/ui/real-build.spec.ts.
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';

const dist = 'dist';
const fail = (msg) => {
  console.error(`check-built-sounds: FAIL: ${msg}`);
  process.exit(1);
};
if (!existsSync(path.join(dist, 'index.html'))) fail('dist/index.html missing: run `npm run build` first');
const assets = readdirSync(path.join(dist, 'assets'));
const js = assets.filter((f) => f.endsWith('.js')).map((f) => readFileSync(path.join(dist, 'assets', f), 'utf8')).join('\n');
if (/data:audio\//.test(js)) fail('a sound is inlined as a data: URL (blocked by the CSP in the app)');
const wanted = ['move', 'select', 'back'];
const refs = [...new Set(js.match(/[\w./-]*?(?:move|select|back)-[\w-]+\.wav/g) ?? [])];
for (const name of wanted) {
  const ref = refs.find((r) => path.basename(r).startsWith(`${name}-`));
  if (!ref) fail(`no reference to ${name}.wav in the built JavaScript`);
  const file = path.join(dist, 'assets', path.basename(ref));
  if (!existsSync(file)) fail(`${ref} is referenced but not in dist/assets`);
  if (readFileSync(file).subarray(0, 4).toString('latin1') !== 'RIFF') fail(`${file} is not a WAV file`);
}
const csp = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).app.security.csp;
const connect = /connect-src ([^;]+)/.exec(csp)?.[1] ?? '';
if (!connect.split(/\s+/).includes("'self'")) fail(`the CSP's connect-src (${connect}) does not allow 'self'`);
console.warn(`check-built-sounds: OK (${wanted.length} sounds as files in dist/assets, none inlined; connect-src allows 'self')`);
