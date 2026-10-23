import { expect, test, type Page } from '@playwright/test';

// A9: whatever takes the front from AIONE (Alt-Tab, another program, a game), AIONE comes back in
// a correct state: the pointer hidden, no hover left behind, held keys released, open menus kept,
// and each change logged. The window's own blur / focus events, as Windows sends them.
async function open(page: Page) {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto('./?time=1:10&h12=1&pad=connected');
  await page.locator('main[data-phase="home"]').waitFor();
  await page.waitForTimeout(300);
}
const front = (page: Page, has: boolean) => page.evaluate((h) => window.dispatchEvent(new Event(h ? 'focus' : 'blur')), has);
const logs = (page: Page) => page.evaluate(() => window.__aioneMock?.logs ?? []);

test.describe('A9: foreground loss and regain (mocked core)', () => {
  test('the pointer is hidden when AIONE loses the front', async ({ page }) => {
    await open(page);
    await page.evaluate(() => window.__aioneMock?.pointer(500, 0));
    await expect(page.getByTestId('pointer')).toHaveAttribute('data-shown', 'true');
    await front(page, false);
    await expect(page.getByTestId('pointer')).toHaveAttribute('data-shown', 'false');
    await page.evaluate(() => window.__aioneMock?.pointer(0, 0));
    await front(page, true);
    const l = await logs(page);
    expect(l.some((x) => x.includes('AIONE lost the front (Home, focus 1/'))).toBe(true);
    expect(l.some((x) => x.includes('AIONE back in front (Home, focus 1/'))).toBe(true);
  });

  test('no mouse hover is left behind', async ({ page }) => {
    await open(page);
    const tile = page.locator('[data-testid="tile"]').nth(1);
    const box = await tile.boundingBox();
    if (!box) throw new Error('no tile');
    await page.mouse.move(box.x + 20, box.y + 20);
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2, { steps: 5 });
    await expect(tile).toHaveClass(/hovered/);
    await front(page, false);
    await expect(tile).not.toHaveClass(/hovered/);
    await front(page, true);
  });

  test('an open Quick Menu stays open across a loss and regain', async ({ page }) => {
    await open(page);
    await page.evaluate(() => window.__aioneMock?.pad('Guide', 'down'));
    await page.waitForTimeout(650);
    await page.evaluate(() => window.__aioneMock?.pad('Guide', 'up'));
    await expect(page.getByTestId('quick-menu')).toBeVisible();
    await front(page, false);
    await front(page, true);
    await expect(page.getByTestId('quick-menu')).toBeVisible();
    expect((await logs(page)).some((x) => x.includes('AIONE back in front (the Quick Menu (Power))'))).toBe(true);
  });
});
