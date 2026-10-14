import { expect, test, type Page } from '@playwright/test';
import { decodePng, pixel } from './png';
import { shot, waitForIcons } from './helpers';

// Rev. r (alpha.10): D-017 descenders, D-018 the ring's separator, D-014, Tab for Options.
const BASE = 'time=12:24&h12=1&pad=connected';
const logs = (page: Page) => page.evaluate(() => window.__aioneMock?.logs ?? []);
const lum = (c: number[]) => {
  const f = (v: number) => ((v /= 255) <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
  return 0.2126 * f(c[0] ?? 0) + 0.7152 * f(c[1] ?? 0) + 0.0722 * f(c[2] ?? 0);
};
const contrast = (a: number[], b: number[]) => {
  const [x, y] = [lum(a), lum(b)].sort((m, n) => n - m);
  return ((x ?? 0) + 0.05) / ((y ?? 0) + 0.05);
};
async function open(page: Page, query: string) {
  await page.goto(`./?${BASE}&${query}`);
  await expect(page.locator('main')).toHaveAttribute('data-phase', 'home');
  await waitForIcons(page);
  await page.evaluate(() => document.fonts.load('400 50px "M PLUS Rounded 1c"', 'Hgjpqy'));
}

test.describe('rev. r (alpha.10)', () => {
  for (const scale of [100, 150])
    for (const preset of ['light', 'dark'])
      for (const kind of ['short', 'scrolling']) {
        test(`D-017: descenders reach the font's descent, tapered (${scale} %, ${preset}, ${kind})`, async ({ page }) => {
          await page.setViewportSize({ width: 1920, height: 1080 });
          const name = kind === 'short' ? 'Hgjpqy' : Array(10).fill('Hgjpqy').join(' ');
          await open(page, `scale=${scale}&preset=${preset}&rename=0:${encodeURIComponent(name)}`);
          if (kind === 'scrolling') await expect(page.getByTestId('selected-title-text')).toHaveAttribute('data-moving', 'true');
          // The font's own metrics at the title's size (no layout reads in the app; here for the test).
          const m = await page.evaluate(() => {
            const el = document.querySelector('.title') as HTMLElement;
            const size = parseFloat(getComputedStyle(el).fontSize);
            const ctx = document.createElement('canvas').getContext('2d') as CanvasRenderingContext2D;
            ctx.font = `400 ${size}px "M PLUS Rounded 1c"`;
            const r = el.getBoundingClientRect();
            const ring = (document.querySelector('.tile.focused .ring') as HTMLElement).getBoundingClientRect();
            return { size, cap: ctx.measureText('H').actualBoundingBoxAscent, descent: ctx.measureText('gjpqy').actualBoundingBoxDescent, left: r.left, right: r.right, top: r.top, ringTop: ring.top };
          });
          const img = decodePng(await page.screenshot());
          const bg = pixel(img, 1900, 160);
          const rows: number[] = [];
          for (let y = Math.floor(m.top); y < Math.floor(m.ringTop) - 1; y++) {
            let n = 0;
            for (let x = Math.ceil(m.left); x < Math.min(img.width, Math.floor(m.right)); x++) {
              const p = pixel(img, x, y);
              if (Math.abs(p[0] - bg[0]) + Math.abs(p[1] - bg[1]) + Math.abs(p[2] - bg[2]) > 90) n++;
            }
            rows.push(n);
          }
          const first = rows.findIndex((n) => n > 0);
          const last = rows.length - 1 - [...rows].reverse().findIndex((n) => n > 0);
          const baseline = first + m.cap; // the capitals' top + the cap height
          const depth = last - baseline;
          console.warn(`D-017 ${scale} % ${preset} ${kind}: ink ${last - first + 1} px, below the baseline ${depth.toFixed(1)} px of the font's ${m.descent.toFixed(1)}; last rows ${rows.slice(last - 3, last + 1).join(',')}; ring top ${(m.ringTop - m.top - last).toFixed(1)} px below`);
          expect(depth).toBeGreaterThanOrEqual(0.85 * m.descent);
          expect(rows[last] ?? 0).toBeLessThan(rows[last - 3] ?? 0); // tapered, not cut flat
          expect(m.top + last).toBeLessThan(m.ringTop); // clear of the ring
          if (scale === 100 && kind === 'short') await shot(page, `t15-descenders-${preset}`);
        });
      }

  for (const preset of ['light', 'dark'])
    for (const ring of ['separator', 'plain']) {
      test(`D-018: the ring against the background and a dark / light picture (${preset}, ${ring})`, async ({ page }) => {
        await page.setViewportSize({ width: 1920, height: 1080 });
        await open(page, `preset=${preset}${ring === 'plain' ? '&ring=plain' : ''}`);
        // Two test pictures: a dark navy and a light grey, on the first two tiles.
        await page.evaluate(() => {
          const solid = (c: string) => {
            const cv = document.createElement('canvas');
            cv.width = cv.height = 64;
            const g = cv.getContext('2d') as CanvasRenderingContext2D;
            g.fillStyle = c;
            g.fillRect(0, 0, 64, 64);
            return cv.toDataURL();
          };
          const imgs = document.querySelectorAll<HTMLImageElement>('[data-testid="tile"] img');
          if (imgs[0]) imgs[0].src = solid('#203A60');
          if (imgs[1]) imgs[1].src = solid('#F0F0F0');
        });
        const out: string[] = [];
        for (const [tile, art] of [[0, 'dark'], [1, 'light']] as const) {
          if (tile === 1) await page.keyboard.press('ArrowRight');
          for (const end of ['start', 'end'] as const) {
            await page.evaluate((e) => {
              for (const a of document.getAnimations()) {
                if (!(a as CSSAnimation).animationName?.includes('ring-pulse')) continue;
                a.pause();
                a.currentTime = e === 'start' ? 0 : 499;
              }
            }, end);
            await page.waitForTimeout(120);
            const g = await page.evaluate(() => {
              const r = (document.querySelector('.tile.focused .ring') as HTMLElement).getBoundingClientRect();
              const t = (document.querySelector('.tile.focused') as HTMLElement).getBoundingClientRect();
              return { rx: r.x + r.width / 2, ry: r.y + 3, sepY: r.y + 7.9, artY: t.y + 12, x: t.x + t.width / 2 };
            });
            const img = decodePng(await page.screenshot());
            const ringC = pixel(img, g.rx, g.ry);
            const sepC = pixel(img, g.rx, g.sepY);
            const artC = pixel(img, g.x, g.artY);
            const bg = pixel(img, 1900, 160);
            const vsBg = contrast(ringC, bg);
            // The ring's inner neighbour: the separator (or, plain, the picture itself).
            const inner = ring === 'separator' ? sepC : artC;
            const vsInner = contrast(ringC, inner);
            // What separates the indicator from the picture: the better of the ring and the separator.
            const vsArt = Math.max(contrast(ringC, artC), ring === 'separator' ? contrast(sepC, artC) : 0);
            out.push(`${art} art, ${end}: ring ${JSON.stringify(ringC)} vs bg ${vsBg.toFixed(2)}, vs inner ${vsInner.toFixed(2)}, indicator vs picture ${vsArt.toFixed(2)}`);
            expect(vsBg).toBeGreaterThanOrEqual(3);
            if (ring === 'separator') expect(sepC).toEqual([255, 255, 255]);
            if (ring === 'separator' && preset === 'light') expect(vsArt).toBeGreaterThanOrEqual(3);
          }
          await page.evaluate(() => document.getAnimations().forEach((a) => a.play()));
          await shot(page, `t15-ring-${ring}-${preset}-on-${art}`);
        }
        console.warn(`D-018 ${preset} ${ring}: ${out.join(' | ')}`);
      });
    }

  test('D-014: Home ready under black before the Boot Up; the skip says so', async ({ page }) => {
    await page.goto(`./?${BASE}&boot=custom`);
    await expect.poll(async () => (await logs(page)).some((x) => x.includes('custom Boot Up: playing')), { timeout: 10_000 }).toBe(true);
    await page.keyboard.press('Enter');
    await expect(page.locator('main')).toHaveAttribute('data-phase', 'home');
    await page.waitForTimeout(1200);
    const l = await logs(page);
    const ready = l.findIndex((x) => x.includes('Home ready under the black screen in'));
    const boot = l.findIndex((x) => x.includes('Boot Up: custom video'));
    expect(ready).toBeGreaterThanOrEqual(0);
    expect(ready).toBeLessThan(boot);
    const skip = l.find((x) => x.includes('skip at')) ?? '';
    console.warn(`D-014: ${l[ready]} | ${skip}`);
    expect(skip).toMatch(/Home had been ready for [\d.]+ s/);
    const after = l.slice(l.findIndex((x) => x.includes('Boot Up outcome: Skipped'))).filter((x) => x.startsWith('info: long task'));
    console.warn(`D-014 long tasks within 1.2 s after Home: ${after.length}${after[0] ? ` — ${after[0]}` : ''}`);
  });

  test('B-3 (rev. r): Tab and the Menu key are Options; never repeat; "Tab Options" in keyboard mode', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await open(page, '');
    await page.keyboard.press('ArrowRight');
    await expect(page.getByTestId('footer-prompts')).toHaveAttribute('data-device', 'keyboard');
    await expect(page.locator('[data-testid="prompt-options"] .glyph')).toHaveAttribute('data-glyph', 'keyboard_tab.svg');
    await page.keyboard.down('Tab');
    await page.waitForTimeout(900); // held: never repeats
    await page.keyboard.up('Tab');
    await expect(page.getByTestId('toast')).toHaveText('Coming in a later stage');
    let l = await logs(page);
    expect(l.filter((x) => x.includes('input down Tab (keyboard) -> action options, options: toast, sound error'))).toHaveLength(1);
    expect(l.some((x) => x.includes('input repeat Tab'))).toBe(false);
    await page.keyboard.press('ContextMenu');
    l = await logs(page);
    expect(l.some((x) => x.includes('input down ContextMenu (keyboard) -> action options'))).toBe(true);
    await shot(page, 't15-tab-options');
  });
});
