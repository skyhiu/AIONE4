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
    const img = decodePng(await b.screenshot());
    let ink = 0;
    for (let i = 0; i < img.data.length; i += 4) ink += (255 - (img.data[i] ?? 255)) / 255;
    out.push([(await b.textContent()) ?? '', ink]);
  }
  const sorted = out.map((x) => x[1]).sort((a, b) => a - b);
  const med = ((sorted[2] ?? 0) + (sorted[3] ?? 0)) / 2;
  console.warn(out.map(([n, v]) => `${n} ${v.toFixed(0)} (${(((v - med) / med) * 100).toFixed(0)} %)`).join(' | ') + ` | median ${med.toFixed(0)}`);
  for (const [n, v] of out) expect(Math.abs(v - med) / med, n).toBeLessThanOrEqual(0.25);
  await page.locator('[data-testid="drawer"]').screenshot({ path: '/tmp/drawer-icons.png' });
});
