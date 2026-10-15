import { expect, test, type Page } from '@playwright/test';

// T1.3 part 1: launching a game, the transition, returning, the game's exit, a failed start.
const logs = (page: Page): Promise<string[]> => page.evaluate(() => window.__aioneMock?.logs ?? []);
const pad = (page: Page, c: string) => page.evaluate((x) => window.__aioneMock?.pad(x, 'down'), c);
async function open(page: Page, extra = '') {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto(`./?time=1:10&h12=1&pad=connected${extra ? `&${extra}` : ''}`);
  await page.locator('main[data-phase="home"]').waitFor();
  await page.waitForTimeout(300);
}
const veil = (page: Page) => page.getByTestId('launch-veil').evaluate((e) => Number(getComputedStyle(e).opacity));

test.describe('T1.3 launching (mocked core)', () => {
  test('A starts the game: the launch sound at once; still 0.55 s, then 0.2 s to black; input discarded; the game first in the row', async ({ page }) => {
    await open(page);
    await pad(page, 'DPadRight'); // Glass Orchard
    const name = await page.getByTestId('selected-title').textContent();
    const t0 = Date.now();
    await pad(page, 'A');
    let l = await logs(page);
    expect(l.some((x) => /action confirm, start ".+" \(.+\), sound launch/.test(x))).toBe(true);
    expect(await page.evaluate(() => window.__aioneMock?.launches)).toEqual(['Glass Orchard']);
    // Still at 0.4 s (the press dip plays), black after about 0.75 s.
    await page.waitForTimeout(Math.max(0, 400 - (Date.now() - t0)));
    expect(await veil(page)).toBe(0);
    await page.screenshot({ path: 'ui-screenshots/t19-launch-still.png' });
    await pad(page, 'DPadLeft'); // discarded
    await page.waitForTimeout(Math.max(0, 640 - (Date.now() - t0)));
    await page.screenshot({ path: 'ui-screenshots/t19-launch-fading.png' });
    await page.waitForTimeout(Math.max(0, 900 - (Date.now() - t0)));
    expect(await veil(page)).toBe(1);
    await page.screenshot({ path: 'ui-screenshots/t19-launch-black.png' });
    l = await logs(page);
    expect(l.some((x) => x.includes('discarded (launching a game: input lock)'))).toBe(true);
    expect(l.some((x) => /launch "Glass Orchard": started \(shortcut game\.lnk, process 4242\), \d+ ms after the press/.test(x))).toBe(true);
    // The game ends: Home again, the focus on it, and it is first in the row now (B-11).
    await page.evaluate(() => window.__aioneMock?.gameExit('Glass Orchard'));
    await page.waitForTimeout(300);
    expect(await veil(page)).toBe(0);
    await expect(page.getByTestId('selected-title')).toHaveText(name ?? '');
    await expect(page.getByTestId('tile').first()).toHaveAttribute('aria-label', /Glass Orchard/);
    l = await logs(page);
    expect(l.some((x) => x.includes('Home again: "Glass Orchard" ended'))).toBe(true);
  });

  test('A on the running game returns to it (never started twice); Guide in front does too', async ({ page }) => {
    await open(page);
    await pad(page, 'A');
    await page.waitForTimeout(900);
    // AIONE comes back to the front (Guide from the background): Home, the focus on the game.
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
    await page.waitForTimeout(300);
    expect(await veil(page)).toBe(0);
    await pad(page, 'A');
    await page.waitForTimeout(300);
    let l = await logs(page);
    expect(l.some((x) => /action confirm, back to the running game "Amber Lighthouse" \(press\)/.test(x))).toBe(true);
    expect(await page.evaluate(() => window.__aioneMock?.launches)).toEqual(['Amber Lighthouse']); // once
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
    await page.waitForTimeout(300);
    await pad(page, 'Guide');
    await page.waitForTimeout(300);
    l = await logs(page);
    expect(l.some((x) => /action guide, back to the running game "Amber Lighthouse" \(Guide\)/.test(x))).toBe(true);
  });

  test('a failed start: a plain message, Home again, logged with the reason', async ({ page }) => {
    await open(page, 'launch=fail');
    await pad(page, 'A');
    await page.waitForTimeout(1100);
    expect(await veil(page)).toBe(0);
    await expect(page.getByTestId('toast')).toHaveText("Couldn't start Amber Lighthouse: its shortcut game.lnk is missing");
    await page.screenshot({ path: 'ui-screenshots/t19-launch-failed.png' });
    const l = await logs(page);
    expect(l.some((x) => x.includes('launch "Amber Lighthouse" failed: its shortcut game.lnk is missing'))).toBe(true);
  });
});
