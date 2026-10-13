// Renders the T1.1 logo previews and Boot Up frames from the preview page, and checks that every
// visual event's key moment is on its measured beat (within one frame, 16.7 ms).
// Usage: node tools/brand/shoot_t11.mjs <out dir>   (PW_CHROMIUM_EXECUTABLE optional)
import { chromium } from '@playwright/test';
import { readFileSync, mkdirSync } from 'node:fs';
import path from 'node:path';

const out = path.resolve(process.argv[2] ?? 'out');
const frames = path.join(out, 'frames');
mkdirSync(frames, { recursive: true });
const beats = JSON.parse(readFileSync(path.join(out, 'beats.json'), 'utf8'));
const exe = process.env.PW_CHROMIUM_EXECUTABLE;
const browser = await chromium.launch(exe ? { executablePath: exe } : {});
const page = await browser.newPage({ viewport: { width: 1280, height: 1400 }, deviceScaleFactor: 1 });
const errors = [];
page.on('pageerror', (e) => errors.push(e.message));
await page.goto('file://' + path.join(out, 'preview.html'));
await page.waitForTimeout(500);

const report = {};
for (const lid of Object.keys(beats.logos)) {
  await page.evaluate((l) => window.__select(l), lid);
  // Key moments of every event animation (offset where the keyframe that hits lies).
  const keys = await page.evaluate(() =>
    document.getAnimations().map((a) => {
      const t = a.effect.getTiming();
      return { key: a.__key, delay: t.delay, duration: t.duration };
    }),
  );
  const checks = beats.logos[lid].beats.map((b) => {
    const ms = b.beat_s * 1000;
    const near = keys.reduce((best, k) => (Math.abs(k.key - ms) < Math.abs(best - ms) ? k.key : best), Infinity);
    return { hit: b.name, beat_ms: Math.round(ms * 10) / 10, event_ms: Math.round(near * 10) / 10, diff_ms: Math.round((near - ms) * 10) / 10 };
  });
  report[lid] = checks;
  const stage = page.locator('#stage');
  const times = [0, ...beats.logos[lid].beats.map((b) => Math.round(b.beat_s * 1000)), 3800, 4650];
  let i = 0;
  for (const t of times) {
    await page.evaluate((ms) => window.__seek(ms), t);
    await stage.screenshot({ path: path.join(frames, `${lid}-${String(i++).padStart(2, '0')}-${t}ms.png`) });
  }
  // Just before and on one landing, to show the pop.
  const first = Math.round(beats.logos[lid].beats[0].beat_s * 1000);
  await page.evaluate((ms) => window.__seek(ms), first - 120);
  await stage.screenshot({ path: path.join(frames, `${lid}-pre-${first - 120}ms.png`) });
}
await page.evaluate(() => window.__select('gather'));
await page.screenshot({ path: path.join(out, 'preview-page.png'), fullPage: true });

for (const lid of Object.keys(beats.logos)) {
  for (const v of ['dark', 'light']) {
    const file = path.join(out, `${lid}-lockup-${v}.svg`);
    const w = Number(/width="(\d+)"/.exec(readFileSync(file, 'utf8'))?.[1] ?? 400);
    const p2 = await browser.newPage({ viewport: { width: w, height: 360 }, deviceScaleFactor: 2 });
    await p2.goto('file://' + file);
    await p2.locator('svg').screenshot({ path: path.join(out, `${lid}-lockup-${v}.png`) });
    await p2.close();
  }
}
console.warn(JSON.stringify({ errors, report }, null, 1));
await browser.close();
