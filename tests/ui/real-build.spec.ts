import { expect, test } from '@playwright/test';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';

// D-009: the REAL production build (dist/, from `npm run build`), served with the app's own
// content security policy (src-tauri/tauri.conf.json), as WebView2 gets it. The mocked UI tests
// use another build without that policy, which is why alpha.5's missing sounds passed CI.
const DIST = 'dist';
const ORIGIN = 'http://aione-real-build.test';
const CSP = (JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')) as { app: { security: { csp: string } } }).app
  .security.csp;

test.describe('D-009 real build', () => {
  test.skip(!existsSync(path.join(DIST, 'index.html')), 'run `npm run build` first');

  test('every menu sound is a file the app may fetch and decode under its CSP', async ({ page }) => {
    await page.route(`${ORIGIN}/**`, async (route) => {
      const rel = decodeURIComponent(new URL(route.request().url()).pathname).replace(/^\//, '') || 'index.html';
      const file = path.join(DIST, rel);
      if (!existsSync(file)) return route.fulfill({ status: 404, body: 'not found' });
      const type = rel.endsWith('.html')
        ? 'text/html'
        : rel.endsWith('.js')
          ? 'text/javascript'
          : rel.endsWith('.css')
            ? 'text/css'
            : rel.endsWith('.wav')
              ? 'audio/wav'
              : 'application/octet-stream';
      return route.fulfill({ status: 200, body: readFileSync(file), headers: { 'content-type': type, 'content-security-policy': CSP } });
    });
    await page.goto(`${ORIGIN}/index.html`);
    const js = readdirSync(path.join(DIST, 'assets'))
      .filter((f) => f.endsWith('.js'))
      .map((f) => readFileSync(path.join(DIST, 'assets', f), 'utf8'))
      .join('\n');
    const refs = [...new Set(js.match(/(?:row_move|drawer_move|edge_bump|confirm|back|error|wrap|options_open|options_close|scroll_tick|launch)-[\w-]+\.wav/g) ?? [])];
    expect(refs.map((r) => r.split('-')[0]).sort()).toEqual(['back', 'confirm', 'drawer_move', 'edge_bump', 'error', 'launch', 'options_close', 'options_open', 'row_move', 'scroll_tick', 'wrap']);
    const results = await page.evaluate(async (files) => {
      const out: Record<string, string> = {};
      const ctx = new OfflineAudioContext(1, 48000, 48000);
      for (const f of files) {
        try {
          const res = await fetch(`./assets/${f}`);
          const buf = await ctx.decodeAudioData(await res.arrayBuffer());
          out[f] = `ok ${Math.round(buf.duration * 1000)} ms`;
        } catch (e) {
          out[f] = `failed: ${String(e)}`;
        }
      }
      // Control: an inlined data: sound (what alpha.5 shipped) must be refused by this CSP.
      try {
        await fetch('data:audio/wav;base64,UklGRiQAAABXQVZFZm10IBAAAAABAAEAgLsAAAB3AQACABAAZGF0YQAAAAA=');
        out['data: control'] = 'allowed';
      } catch {
        out['data: control'] = 'blocked';
      }
      return out;
    }, refs);
    console.warn(`D-009 real build: ${JSON.stringify(results)}`);
    for (const r of refs) expect(results[r], r).toMatch(/^ok \d+ ms$/);
    expect(results['data: control']).toBe('blocked');
  });
});
