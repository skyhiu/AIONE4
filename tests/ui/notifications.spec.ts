import { expect, test, type Page } from '@playwright/test';

// T2.3: AIONE's own notifications (a game ended, a controller connected / disconnected, a battery
// low): one design, one at a time, never over a dialog, never focusable.
async function open(page: Page, extra = '') {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto(`./?time=1:10&h12=1&pad=connected${extra ? `&${extra}` : ''}`);
  await page.locator('main[data-phase="home"]').waitFor();
  await page.waitForTimeout(300);
}
const logs = (page: Page) => page.evaluate(() => window.__aioneMock?.logs ?? []);
const hold = (page: Page, c: string) => page.evaluate((x) => window.__aioneMock?.pad(x, 'down'), c);
const release = (page: Page, c: string) => page.evaluate((x) => window.__aioneMock?.pad(x, 'up'), c);
const tap = async (page: Page, c: string) => {
  await hold(page, c);
  await release(page, c);
};

test.describe('T2.3 notifications (mocked core)', () => {
  for (const preset of ['light', 'dark']) {
    test(`a controller connects and disconnects: one notice at a time (${preset})`, async ({ page }) => {
      await open(page, preset === 'dark' ? 'preset=dark' : '');
      await page.evaluate(() => window.__aioneMock?.connect(1));
      const n = page.getByTestId('notification');
      await expect(n).toBeVisible();
      await expect(page.getByTestId('notification-title')).toHaveText('Controller connected');
      await expect(page.getByTestId('notification-detail')).toHaveText('Controller 2');
      await page.waitForTimeout(400);
      await page.screenshot({ path: `ui-screenshots/t23-notification-controller-${preset}.png` });
      // Never focusable, never takes a click.
      await expect(n).toHaveCSS('pointer-events', 'none');
      expect(await n.getAttribute('tabindex')).toBeNull();
      // The next one waits for the first to end (about 4 s).
      await page.evaluate(() => window.__aioneMock?.disconnect(1));
      await expect(page.getByTestId('notification-title')).toHaveText('Controller connected');
      await expect(page.getByTestId('notification-title')).toHaveText('Controller disconnected', { timeout: 6000 });
      const l = await logs(page);
      expect(l.some((x) => x.includes('notification shown: "Controller connected" (Controller 2)'))).toBe(true);
    });
  }

  test('a notice never covers a dialog: it waits until the dialog closes', async ({ page }) => {
    await open(page);
    // Power options (a dialog-like list) from the Quick Menu.
    await hold(page, 'Guide');
    await page.waitForTimeout(650);
    await release(page, 'Guide');
    await tap(page, 'A');
    await expect(page.getByTestId('power-options')).toBeVisible();
    await page.evaluate(() => window.__aioneMock?.battery('low', 0));
    await page.waitForTimeout(500);
    await expect(page.getByTestId('notification')).toHaveCount(0);
    await page.keyboard.press('Escape');
    await expect(page.getByTestId('power-options')).toHaveCount(0);
    await expect(page.getByTestId('notification-title')).toHaveText('Controller battery low');
    await page.waitForTimeout(400);
    await page.screenshot({ path: 'ui-screenshots/t23-notification-battery-after-dialog.png' });
    const l = await logs(page);
    expect(l.some((x) => x.includes('notification queued (controller 1 battery low)') && x.includes('(waits'))).toBe(true);
  });

  test('a game ended: its notice on Home, with the time played', async ({ page }) => {
    await open(page);
    await tap(page, 'A'); // start the focused game
    await page.waitForTimeout(400);
    const id = await page.evaluate(() => window.__aioneMock?.launches.at(-1) ?? '');
    expect(id).not.toBe('');
    await page.evaluate((x) => window.__aioneMock?.gameWindow(x), id);
    await page.waitForTimeout(800);
    await page.evaluate((x) => window.__aioneMock?.gameExit(x), id);
    await expect(page.getByTestId('notification-title')).toHaveText(/ ended$/, { timeout: 5000 });
    await expect(page.getByTestId('notification-detail')).toHaveText('Played 1 min');
    await page.waitForTimeout(400);
    await page.screenshot({ path: 'ui-screenshots/t23-notification-game-ended.png' });
  });

  test("the PC's battery: a notice at 20 %, not again at 19 %, again at 10 %", async ({ page }) => {
    await open(page);
    const status = (percent: number) =>
      page.evaluate((p) => window.__aioneMock?.status({ network: 'wifi', battery: { percent: p, charging: false } }), percent);
    await status(20);
    await expect(page.getByTestId('notification-detail')).toHaveText('20 % left: connect the charger');
    await status(19);
    await status(10);
    await expect(page.getByTestId('notification-detail')).toHaveText('10 % left: connect the charger', { timeout: 6000 });
    const l = await logs(page);
    expect(l.filter((x) => x.includes('notification queued') && x.includes("the PC's battery")).length).toBe(2);
  });
});
