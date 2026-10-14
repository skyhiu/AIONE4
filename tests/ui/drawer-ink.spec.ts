import { expect, test } from '@playwright/test';
// Reference KD-81 (drawer icons): every icon within about ±25 % of the median ink, same box size.
import { decodePng } from './png';
test('drawer icons: one ink weight (KD-81: within ±25 % of the median)', async ({ page }) => {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto('./?time=12:24&h12=1&pad=connected');
  await page.locator('main[data-phase="home"]').waitFor();
  await page.waitForTimeout(800);
  const out: [string, number][] = [];
  for (const b of await page.locator('.dicon').all()) {
    // Coverage, independent of the icon's colour (each icon has its own palette colour, B-14):
    // each pixel counts by how far it is from the disc towards the icon's own colour.
    const img = decodePng(await b.screenshot());
    const disc = [img.data[0] ?? 0, img.data[1] ?? 0, img.data[2] ?? 0];
    const dist = (i: number) => Math.hypot((img.data[i] ?? 0) - (disc[0] ?? 0), (img.data[i + 1] ?? 0) - (disc[1] ?? 0), (img.data[i + 2] ?? 0) - (disc[2] ?? 0));
    let far = 1;
    for (let i = 0; i < img.data.length; i += 4) far = Math.max(far, dist(i));
    let ink = 0;
    for (let i = 0; i < img.data.length; i += 4) ink += dist(i) / far;
    out.push([(await b.textContent()) ?? '', ink]);
  }
  const sorted = out.map((x) => x[1]).sort((a, b) => a - b);
  const med = ((sorted[2] ?? 0) + (sorted[3] ?? 0)) / 2;
  console.warn(out.map(([n, v]) => `${n} ${v.toFixed(0)} (${(((v - med) / med) * 100).toFixed(0)} %)`).join(' | ') + ` | median ${med.toFixed(0)}`);
  for (const [n, v] of out) expect(Math.abs(v - med) / med, n).toBeLessThanOrEqual(0.25);
  await page.locator('[data-testid="drawer"]').screenshot({ path: '/tmp/drawer-icons.png' });
});
