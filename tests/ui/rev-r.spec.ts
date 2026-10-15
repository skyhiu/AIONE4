import { expect, test, type Page } from '@playwright/test';
import { decodePng, pixel } from './png';
import { shot, waitForIcons } from './helpers';

// Rev. r / s (alpha.10–11): D-017 descenders, the ring's white band, D-014, Tab for Options, the extras.
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
          // The ring's glow (rev. v halo) extends above the ring's box into the rows scanned here:
          // freeze the flash at its base (the halo invisible) so only the title's ink is counted.
          // (CI run #17 counted the halo at its peak as a 410 px "descender" row.)
          await page.evaluate(() => document.querySelectorAll('.ring-alt, .ring-halo').forEach((e) => e.getAnimations().forEach((a) => (a.pause(), (a.currentTime = 760)))));
          await page.waitForTimeout(80);
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
    test(`B-9 (rev. v): the 4 px white band separates the brand-pink ring from a dark and a light picture (${preset})`, async ({ page }) => {
      await page.setViewportSize({ width: 1920, height: 1080 });
      await open(page, `preset=${preset}`);
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
        await page.evaluate(() => document.querySelectorAll('.ring-alt').forEach((e) => e.getAnimations().forEach((a) => (a.pause(), (a.currentTime = 760)))));
        await page.waitForTimeout(120);
        const g = await page.evaluate(() => {
          const r = (document.querySelector('.tile.focused .ring') as HTMLElement).getBoundingClientRect();
          const t = (document.querySelector('.tile.focused') as HTMLElement).getBoundingClientRect();
          return { x: r.x + r.width / 2, ringY: r.y + 3.5, bandY: t.y - 2, artY: t.y + 12 };
        });
        const img = decodePng(await page.screenshot());
        const ringC = pixel(img, g.x, g.ringY);
        const band = pixel(img, g.x, g.bandY);
        const artC = pixel(img, g.x, g.artY);
        const bg = pixel(img, 1900, 160);
        out.push(`${art} art: ring vs bg ${contrast(ringC, bg).toFixed(2)}, band ${JSON.stringify(band)} vs picture ${contrast(band, artC).toFixed(2)}, ring vs band ${contrast(ringC, band).toFixed(2)}`);
        expect(band).toEqual([255, 255, 255]); // the band is pure white in both presets (rev. t)
        // Rev. v: the ring is the brand pink #FF46A2 and relies on its saturation and the white
        // band, not on 3:1 against the background (logged); the band separates it from any picture.
        expect(Math.abs(ringC[0] - 217) + Math.abs(ringC[1] - 0) + Math.abs(ringC[2] - 108)).toBeLessThanOrEqual(6); // rev. y B3 base #D9006C
        expect(contrast(band, [32, 58, 96])).toBeGreaterThanOrEqual(3);
        await shot(page, `t17-band-${preset}-on-${art}`);
      }
      console.warn(`B-9 band ${preset}: ${out.join(' | ')}`);
    });

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
    // B-1 (rev. s): the open sound with the toast; never repeats.
    expect(l.filter((x) => x.includes('input down Tab (keyboard) -> action options, options: toast, sound optionsOpen'))).toHaveLength(1);
    expect(l.some((x) => x.includes('input repeat Tab'))).toBe(false);
    // B (Esc) dismisses it, with the close sound.
    await page.keyboard.press('Escape');
    await expect(page.getByTestId('toast')).toHaveCount(0);
    l = await logs(page);
    expect(l.some((x) => x.includes('input down Esc (keyboard) -> action back, options: closed (B), sound optionsClose'))).toBe(true);
    // Rev. u: Tab (or ≡) while the toast shows closes it, with the close sound.
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    await expect(page.getByTestId('toast')).toHaveCount(0);
    l = await logs(page);
    expect(l.some((x) => x.includes('input down Tab (keyboard) -> action options, options: closed (the Options button), sound optionsClose'))).toBe(true);
    // The timeout closes it silently.
    await page.keyboard.press('ContextMenu');
    l = await logs(page);
    expect(l.some((x) => x.includes('input down ContextMenu (keyboard) -> action options, options: toast, sound optionsOpen'))).toBe(true);
    await page.waitForTimeout(2700);
    l = await logs(page);
    expect(l.some((x) => x.includes('options toast closed (timeout), no sound'))).toBe(true);
    await shot(page, 't15-tab-options');
  });

  test('B-15 (rev. t): press dips the whole tile to about 94 %; the label rises; --extras=none turns them off', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await open(page, '');
    expect(await page.getByTestId('home').getAttribute('data-extras')).toBe('titlefade press labelfade');
    // A on a game: the whole tile dips (to about 94 % at 35 % of 0.18 s) and springs back.
    await page.keyboard.press('Enter');
    const dip = await page.evaluate(() => {
      const a = document.querySelector('.tile.pressed')?.getAnimations().find((x) => (x as CSSAnimation).animationName?.includes('press-dip'));
      if (!a) return null;
      a.pause();
      a.currentTime = 63;
      const r = (document.querySelector('.tile.pressed') as HTMLElement).getBoundingClientRect();
      return { w: r.width, dur: Number((a.effect as KeyframeEffect).getTiming().duration) };
    });
    expect(dip?.dur).toBe(180);
    expect((dip?.w ?? 0) / 386).toBeGreaterThan(0.93); // rev. x: tiles 386 px
    expect((dip?.w ?? 0) / 386).toBeLessThan(0.95);
    await shot(page, 't16-press-dip');
    await page.evaluate(() => document.querySelectorAll('.tile').forEach((e) => e.getAnimations().forEach((a) => a.finish())));
    // The drawer label: fades in and rises about 4 px over about 0.15 s.
    await page.keyboard.press('ArrowDown');
    const rise = await page.evaluate(() => {
      const a = document.querySelector('[data-testid="drawer-label"]')?.getAnimations()[0] as CSSAnimation | undefined;
      if (!a) return null;
      const k: ComputedKeyframe = (a.effect as KeyframeEffect).getKeyframes()[0] ?? { offset: 0, computedOffset: 0, composite: 'auto', easing: 'linear' };
      return { name: a.animationName, dur: Number((a.effect as KeyframeEffect).getTiming().duration), from: String(k['translate'] ?? ''), opacity: k['opacity'] };
    });
    expect(rise?.name).toContain('label-rise');
    expect(rise?.dur).toBe(150);
    expect(rise?.from).toMatch(/^0px 4px$/);
    expect(String(rise?.opacity)).toBe('0');
    // --extras=none: all three off for one run.
    await open(page, 'extras=none');
    expect(await page.getByTestId('home').getAttribute('data-extras')).toBe('');
  });

  for (const [slot, label] of [[0, 'slot 1'], [1, 'slot 2'], [9, 'the last game']] as const)
    test(`D-022: a scrolling title never shows outside its window or the screen (${label})`, async ({ page }) => {
      await page.setViewportSize({ width: 1920, height: 1080 });
      const long = 'The Long Corridor Between Two Very Distant Stations Complete Night Edition';
      await open(page, `rename=${slot}:${encodeURIComponent(long)}`);
      for (let i = 0; i < slot; i++) await page.keyboard.press('ArrowRight');
      await expect(page.getByTestId('selected-title-text')).toHaveAttribute('data-moving', 'true');
      await page.waitForTimeout(400);
      const dur = await page.evaluate(() => {
        const a = document.querySelector('.ttrack')?.getAnimations().find((x) => x.id === 'marquee');
        return a ? Number((a.effect as KeyframeEffect).getTiming().duration) : 0;
      });
      expect(dur).toBeGreaterThan(1000);
      const stray: string[] = [];
      for (let k = 0; k < 8; k++) {
        const t = (dur * k) / 8;
        await page.evaluate((ms) => {
          const a = document.querySelector('.ttrack')?.getAnimations().find((x) => x.id === 'marquee');
          if (a) {
            a.pause();
            a.currentTime = ms;
          }
          document.querySelectorAll('.ring-alt, .ring-halo').forEach((e) => e.getAnimations().forEach((x) => (x.pause(), (x.currentTime = 760))));
        }, t);
        await page.waitForTimeout(60);
        const box = await page.evaluate(() => {
          const r = (document.querySelector('[data-testid="selected-title-row"]') as HTMLElement).getBoundingClientRect();
          return { left: r.left, right: r.right, top: r.top, bottom: r.bottom, vw: innerWidth };
        });
        // The window inside the screen's margins.
        expect(box.left).toBeGreaterThanOrEqual(box.vw * 0.03 - 1);
        expect(box.right).toBeLessThanOrEqual(box.vw * 0.97 + 1);
        // No title-coloured pixel (#FF46A2) in the title's rows outside the window.
        const img = decodePng(await page.screenshot());
        let outside = 0;
        for (let y = Math.max(0, Math.floor(box.top)); y < Math.min(img.height, Math.ceil(box.bottom)); y++)
          for (let x = 0; x < img.width; x++) {
            if (x >= box.left - 1 && x <= box.right + 1) continue;
            const p = pixel(img, x, y);
            if (Math.abs(p[0] - 255) + Math.abs(p[1] - 70) + Math.abs(p[2] - 162) < 90) outside++;
          }
        if (outside) stray.push(`${Math.round(t)} ms: ${outside} px`);
      }
      console.warn(`D-022 ${label}: loop ${Math.round(dur)} ms, 8 moments, stray title pixels: ${stray.length ? stray.join(', ') : 'none'}`);
      expect(stray).toEqual([]);
    });

  test('D-021: a hold after a wrap starts its repeats at full level (1-3 full, then -4 dB)', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await open(page, '');
    // A long hold first (so the old counter would be high), then back to the first game.
    for (let i = 0; i < 3; i++) await page.evaluate(() => window.__aioneMock?.pad('DPadRight', 'down'));
    for (let i = 0; i < 6; i++) await page.evaluate(() => window.__aioneMock?.pad('DPadRight', 'repeat'));
    await page.evaluate(() => window.__aioneMock?.pad('DPadRight', 'up'));
    // Back to the first game (never past it: a fresh press there would already wrap).
    while (Number(await page.getByTestId('game-row').getAttribute('data-focus')) > 0) await page.keyboard.press('ArrowLeft');
    await page.waitForTimeout(100);
    const mark = (await logs(page)).length;
    // The wrap (a fresh press at the start), then the same direction held.
    await page.evaluate(() => window.__aioneMock?.pad('DPadLeft', 'down'));
    for (let i = 0; i < 5; i++) await page.evaluate(() => window.__aioneMock?.pad('DPadLeft', 'repeat'));
    const l = (await logs(page)).slice(mark).filter((x) => /input (down|repeat) DPadLeft/.test(x));
    console.warn(`D-021:\n${l.map((x) => x.slice(x.indexOf('->') + 3, x.indexOf(', core'))).join('\n')}`);
    expect(l[0]).toContain('sound wrap');
    const reps = l.slice(1);
    expect(reps).toHaveLength(5);
    expect(reps.slice(0, 3).every((x) => !x.includes('(-4 dB)'))).toBe(true); // full level
    expect(reps.slice(3).every((x) => x.includes('(-4 dB)'))).toBe(true); // then quieter
  });
});
