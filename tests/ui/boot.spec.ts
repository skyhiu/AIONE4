import { expect, test, type Page } from '@playwright/test';
import { decodePng, pixel } from './png';
import { shot } from './helpers';

// T1.2: the Boot Up and Boot Down (A9, B-5 final), in the mocked UI (no ?boot = no presentation).
const BASE = 'time=12:24&h12=1&pad=connected';
const logs = (page: Page) => page.evaluate(() => window.__aioneMock?.logs ?? []);
const pad = (page: Page, control: string) => page.evaluate((c) => window.__aioneMock?.pad(c, 'down'), control);
const waitLog = (page: Page, start: string, timeout = 12_000) =>
  expect.poll(async () => (await logs(page)).find((l) => l.includes(start)) ?? '', { timeout }).not.toBe('');

test.describe('T1.2 Boot Up and Boot Down (mocked core)', () => {
  test.setTimeout(60_000);

  test('the default Window B: black first, the clamp on the sound, input locked, then Home', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto(`./?${BASE}&boot=default`);
    // Black from the first frame (never grey or white).
    expect(await page.evaluate(() => getComputedStyle(document.body).backgroundColor)).toBe('rgb(0, 0, 0)');
    await expect(page.getByTestId('boot-layer')).toBeVisible();
    await waitLog(page, 'Boot Up: default Window B (no custom video)');
    // Input during the Boot Up is discarded, never queued.
    await page.waitForTimeout(600);
    await pad(page, 'DPadRight');
    await page.keyboard.press('ArrowRight');
    await waitLog(page, 'Boot Up clamp:');
    const clamp = (await logs(page)).find((l) => l.includes('Boot Up clamp:')) ?? '';
    const after = Number(/target 2550 ms, (-?[\d.]+) ms after/.exec(clamp)?.[1]);
    const prevT = Number(/previous frame at ([\d.]+) ms/.exec(clamp)?.[1]);
    const interval = Number(/frame interval ([\d.]+) ms/.exec(clamp)?.[1]);
    console.warn(`boot clamp: ${clamp}`);
    // The clamp is drawn on the first frame at or after the hit (the previous one was before it),
    // so it is never more than one frame late. On the PC at 60 fps one frame is 16.7 ms; the
    // sandbox's frame interval varies with load, so the test compares against the measured one.
    expect(after).toBeGreaterThanOrEqual(0);
    expect(prevT).toBeLessThan(2550);
    expect(after).toBeLessThanOrEqual(interval);
    await waitLog(page, 'Boot Up outcome: Completed');
    await expect(page.getByTestId('home')).toBeVisible();
    const l = await logs(page);
    expect(l.filter((x) => x.includes('discarded (Boot Up: input lock)'))).toHaveLength(2);
    expect(await page.getByTestId('game-row').getAttribute('data-focus')).toBe('0');
    const stats = l.find((x) => x.startsWith('info: Boot Up: Completed;')) ?? '';
    console.warn(`boot frames: ${stats}`);
    expect(l.some((x) => x.startsWith('info: boot: black hold'))).toBe(true);
    // QA-2: the pictures were decoded during the black hold, before the animation.
    const iReady = l.findIndex((x) => x.includes('tile pictures ready'));
    const iBoot = l.findIndex((x) => x.includes('Boot Up: default Window B'));
    expect(iReady).toBeGreaterThanOrEqual(0);
    expect(iReady).toBeLessThan(iBoot);
    const longDuring = l.filter((x) => x.includes('long task') && x.includes('last action: Boot Up'));
    console.warn(`long tasks during the Boot Up: ${longDuring.length}${longDuring[0] ? ` — ${longDuring[0]}` : ''}`);
  });

  test('a skip ends the Boot Up at once, with no fallback sound', async ({ page }) => {
    await page.goto(`./?${BASE}&boot=default`);
    await waitLog(page, 'Boot Up: default Window B');
    await page.waitForTimeout(800);
    await pad(page, 'A');
    await waitLog(page, 'Boot Up outcome: Skipped');
    await expect(page.getByTestId('home')).toBeVisible({ timeout: 1000 });
    const l = await logs(page);
    expect(l.some((x) => x.includes('-> action confirm, Boot Up skipped'))).toBe(true);
    expect(l.filter((x) => x.includes('Window B'))).toHaveLength(2); // the decision and the sound line only
  });

  test('the custom Boot Up video plays with its sound; a broken one falls back once to Window B', async ({ page }) => {
    await page.goto(`./?${BASE}&boot=custom`);
    await waitLog(page, "Boot Up: custom video (splash.mp4, AIONE 4's copy), sound on, to the end");
    await waitLog(page, 'custom Boot Up: after the first frames: volume 1.00, muted false');
    const audio = (await logs(page)).find((x) => x.includes('after the first frames')) ?? '';
    console.warn(`custom boot: ${audio}`);
    expect(audio).toContain('(sound yes)');
    await waitLog(page, 'Boot Up outcome: Completed');
    expect((await logs(page)).some((x) => x.includes('Window B'))).toBe(false); // never both
    await page.goto(`./?${BASE}&boot=custom-broken`);
    await waitLog(page, 'the default Window B plays instead (the one fallback)');
    await waitLog(page, 'Boot Up outcome: Completed');
  });

  test('--boot=default shows Window B although a custom video exists', async ({ page }) => {
    await page.goto(`./?${BASE}&boot=default-switch`);
    await waitLog(page, 'Boot Up: default Window B (--boot=default for this run)');
  });

  test('Alt-Tab during the boot video pauses and resumes it (A9, L-02)', async ({ page }) => {
    await page.goto(`./?${BASE}&boot=custom`);
    await waitLog(page, 'custom Boot Up: playing');
    await page.evaluate(() => window.dispatchEvent(new Event('blur')));
    await waitLog(page, 'boot paused (AIONE in the background) at video');
    await page.waitForTimeout(1500);
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
    await waitLog(page, 'boot resumed (AIONE back in front) at video');
    const l = await logs(page);
    const p = Number(/paused .* at video ([\d.]+) s/.exec(l.find((x) => x.includes('boot paused')) ?? '')?.[1]);
    const r = Number(/resumed .* at video ([\d.]+) s/.exec(l.find((x) => x.includes('boot resumed')) ?? '')?.[1]);
    console.warn(`L-02: paused at ${p} s, resumed at ${r} s`);
    expect(Math.abs(r - p)).toBeLessThan(0.15); // nothing advanced while in the background
    await waitLog(page, 'Boot Up outcome: Completed');
  });

  test('Close AIONE: the Boot Down plays, the screen stays black until its sound ends, then AIONE closes', async ({ page }) => {
    await page.goto(`./?${BASE}&boot=default`);
    await waitLog(page, 'Boot Up outcome: Completed');
    await expect(page.getByTestId('home')).toBeVisible();
    const t0 = Date.now();
    await page.evaluate(() => window.__aioneMock?.requestClose());
    await waitLog(page, 'Boot Down: default Window B (no custom video)');
    await pad(page, 'A');
    await expect.poll(() => page.evaluate(() => window.__aioneMock?.closed.length ?? 0), { timeout: 8000 }).toBe(1);
    const ms = Date.now() - t0;
    console.warn(`Boot Down until close: ${ms} ms`);
    expect(ms).toBeGreaterThanOrEqual(2550);
    expect(await page.evaluate(() => window.__aioneMock?.closed[0])).toBe('Boot Down Completed');
    expect((await logs(page)).some((x) => x.includes('discarded (Boot Down: input lock)'))).toBe(true);
    // A custom Boot Down video too.
    await page.goto(`./?${BASE}&boot=custom`);
    await waitLog(page, 'Boot Up outcome: Completed');
    await page.evaluate(() => window.__aioneMock?.requestClose());
    await waitLog(page, "Boot Down: custom video (suspend.mp4, AIONE 4's copy)");
    await expect.poll(() => page.evaluate(() => window.__aioneMock?.closed[0] ?? ''), { timeout: 8000 }).toBe('Boot Down Completed');
  });

  for (const t of [400, 1000, 1960, 2550, 2750, 3170, 3350, 4310, 5000]) {
    test(`key frame at ${t} ms, full screen (nothing clipped, black around it)`, async ({ page }) => {
      await page.setViewportSize({ width: 1920, height: 1080 });
      await page.goto(`./?${BASE}&bootframe=up:${t}`);
      await expect(page.getByTestId('boot-up-default')).toBeVisible();
      const img = decodePng(await page.screenshot());
      // The corners and the edges are pure black (nothing drawn there, nothing grey).
      for (const [x, y] of [[2, 2], [1917, 2], [2, 1077], [1917, 1077], [960, 2], [960, 1077]] as const)
        expect(pixel(img, x, y)).toEqual([0, 0, 0]);
      await shot(page, `t12-boot-up-${String(t).padStart(4, '0')}ms`);
    });
  }
  for (const t of [300, 1100, 1500]) {
    test(`Boot Down frame at ${t} ms`, async ({ page }) => {
      await page.setViewportSize({ width: 1920, height: 1080 });
      await page.goto(`./?${BASE}&bootframe=down:${t}`);
      await expect(page.getByTestId('boot-down-default')).toBeVisible();
      await shot(page, `t12-boot-down-${String(t).padStart(4, '0')}ms`);
    });
  }
});
