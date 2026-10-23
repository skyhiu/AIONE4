import { expect, test, type Page } from '@playwright/test';

// B-12 (reference A7): the right-stick pointer: shown by the right stick, the item under it
// becomes the focused item (Home's row and drawer, the Quick Menu, Power Options, a question),
// hidden by any other input or after about 2 s at rest; the focus stays.
async function open(page: Page, extra = '') {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto(`./?time=1:10&h12=1&pad=connected${extra ? `&${extra}` : ''}`);
  await page.locator('main[data-phase="home"]').waitFor();
  await page.waitForTimeout(300);
}
const stick = (page: Page, x: number, y: number) => page.evaluate(([a, b]) => window.__aioneMock?.pointer(a as number, b as number), [x, y]);
const tap = async (page: Page, c: string) => {
  await page.evaluate((x) => window.__aioneMock?.pad(x, 'down'), c);
  await page.evaluate((x) => window.__aioneMock?.pad(x, 'up'), c);
};
/** Steers the stick toward an element (as a person would) until the pointer is over it. */
async function steerTo(page: Page, selector: string) {
  for (let i = 0; i < 80; i++) {
    const at = await page.evaluate((sel) => {
      const m = /translate\(([-\d.]+)px, ([-\d.]+)px\)/.exec(document.querySelector<HTMLElement>('[data-testid="pointer"]')?.style.transform ?? '');
      const r = document.querySelector(sel)?.getBoundingClientRect();
      return m && r ? { x: Number(m[1]), y: Number(m[2]), cx: r.left + r.width / 2, cy: r.top + r.height / 2, w: r.width / 4, h: r.height / 4 } : null;
    }, selector);
    if (!at) throw new Error(`nothing to steer to: ${selector}`);
    const dx = at.cx - at.x;
    const dy = at.cy - at.y;
    if (Math.abs(dx) < at.w && Math.abs(dy) < at.h) break;
    const d = Math.hypot(dx, dy);
    const tilt = d > 200 ? 700 : 350;
    await stick(page, Math.round((dx / d) * tilt), Math.round((-dy / d) * tilt));
    await page.waitForTimeout(40);
  }
  await stick(page, 0, 0);
  await page.waitForTimeout(80);
}
const logs = (page: Page) => page.evaluate(() => window.__aioneMock?.logs ?? []);
const focusedTile = (page: Page) => page.locator('[data-testid="tile"].focused').getAttribute('data-index');

test.describe('B-12: the right-stick pointer (mocked core)', () => {
  for (const preset of ['light', 'dark']) {
    test(`on Home: the tile under it gets the focus (${preset})`, async ({ page }) => {
      await open(page, preset === 'dark' ? 'preset=dark' : '');
      expect(await focusedTile(page)).toBe('0');
      await stick(page, 1000, 0); // full tilt right
      await expect(page.getByTestId('pointer')).toHaveAttribute('data-shown', 'true');
      await page.waitForTimeout(350);
      await stick(page, 0, 0);
      const now = Number(await focusedTile(page));
      expect(now).toBeGreaterThan(0);
      await page.waitForTimeout(250);
      await page.screenshot({ path: `ui-screenshots/b12-pointer-home-${preset}.png` });
      const l = await logs(page);
      expect(l.some((x) => x.includes('input pointer shown'))).toBe(true);
      expect(l.some((x) => x.includes('input pointer -> focus'))).toBe(true);
    });
  }

  test('down onto the drawer; a D-pad press hides it and the focus stays', async ({ page }) => {
    await open(page);
    await stick(page, 0, -300); // shows it on the focused tile
    await page.waitForTimeout(60);
    await steerTo(page, '[data-testid="drawer-button"][data-index="2"]');
    await expect(page.getByTestId('drawer')).toHaveAttribute('data-zone', 'drawer');
    await expect(page.getByTestId('drawer')).toHaveAttribute('data-focus', '2');
    const where = '2';
    await tap(page, 'DPadRight');
    await expect(page.getByTestId('pointer')).toHaveAttribute('data-shown', 'false');
    // The D-pad moved on from where the pointer had left the focus.
    await expect(page.getByTestId('drawer')).toHaveAttribute('data-focus', String(Number(where) + 1));
  });

  test('hides after about 2 s at rest', async ({ page }) => {
    await open(page);
    await stick(page, 400, 0);
    await page.waitForTimeout(150);
    await stick(page, 0, 0);
    await expect(page.getByTestId('pointer')).toHaveAttribute('data-shown', 'true');
    await page.waitForTimeout(1500);
    await expect(page.getByTestId('pointer')).toHaveAttribute('data-shown', 'true');
    await expect(page.getByTestId('pointer')).toHaveAttribute('data-shown', 'false', { timeout: 1500 });
    expect((await logs(page)).some((x) => x.includes('pointer hidden (2 s without right-stick movement)'))).toBe(true);
  });

  test('resting at the row edge scrolls the row step by step', async ({ page }) => {
    await open(page);
    await stick(page, 1000, 0);
    await page.waitForTimeout(1300); // to the right edge, over the partly shown tile
    await stick(page, 0, 0);
    await page.waitForTimeout(1800); // rests: about one step per 0.6 s (the slide, then the dwell)
    const now = Number(await focusedTile(page));
    expect(now).toBeGreaterThan(4); // past the four fully shown tiles
    expect((await logs(page)).some((x) => x.includes('(row scrolls)'))).toBe(true);
  });

  test('in the Quick Menu and Power Options, and nothing under them', async ({ page }) => {
    await open(page);
    await page.evaluate(() => window.__aioneMock?.pad('Guide', 'down'));
    await page.waitForTimeout(650);
    await page.evaluate(() => window.__aioneMock?.pad('Guide', 'up'));
    await expect(page.getByTestId('quick-menu')).toBeVisible();
    await stick(page, 0, -300);
    await page.waitForTimeout(60);
    await steerTo(page, '[data-testid="qm-volume"]');
    await expect(page.getByTestId('qm-volume')).toHaveClass(/focused/);
    await page.screenshot({ path: 'ui-screenshots/b12-pointer-quick-menu.png' });
    // Home under the scrim is never hit: steering over it changes nothing there.
    await steerTo(page, '[data-testid="tile"][data-index="1"]');
    await expect(page.getByTestId('qm-volume')).toHaveClass(/focused/);
    // Power Options: its items take the focus.
    await steerTo(page, '[data-testid="qm-power"]');
    await tap(page, 'A');
    await expect(page.getByTestId('power-options')).toBeVisible();
    await stick(page, 0, -300);
    await page.waitForTimeout(60);
    await steerTo(page, '[data-testid="po-item"]:nth-of-type(3)');
    await expect(page.locator('[data-testid="po-item"]').nth(2)).toHaveClass(/focused/);
    await page.keyboard.press('Escape');
    await page.keyboard.press('Escape');
    await expect(page.getByTestId('quick-menu')).toHaveCount(0);
    expect(await focusedTile(page)).toBe('0');
  });
});
