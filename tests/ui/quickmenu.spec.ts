import { expect, test, type Page } from '@playwright/test';

// T2.2: the Quick Menu (B-26), the Power menu and its confirmations, and dialogs done once.
async function open(page: Page, extra = '') {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto(`./?time=1:10&h12=1&pad=connected${extra ? `&${extra}` : ''}`);
  await page.locator('main[data-phase="home"]').waitFor();
  await page.waitForTimeout(300);
}
const pad = (page: Page, c: string) =>
  page.evaluate((x) => {
    window.__aioneMock?.pad(x, 'down');
    window.__aioneMock?.pad(x, 'up');
  }, c);
const hold = (page: Page, c: string) => page.evaluate((x) => window.__aioneMock?.pad(x, 'down'), c);
const release = (page: Page, c: string) => page.evaluate((x) => window.__aioneMock?.pad(x, 'up'), c);
const logs = (page: Page) => page.evaluate(() => window.__aioneMock?.logs ?? []);
async function holdGuide(page: Page) {
  await hold(page, 'Guide');
  await page.waitForTimeout(650);
  await release(page, 'Guide');
}

test.describe('T2.2 the Quick Menu, Power and dialogs (mocked core)', () => {
  test('a held Guide opens the Quick Menu; rows stop at the ends; volume, mute, Theme; B closes it', async ({ page }) => {
    await open(page, 'volume=50');
    await expect(page.getByTestId('prompt-menu')).toHaveCount(1); // "Menu" in the footer
    await holdGuide(page);
    await expect(page.getByTestId('quick-menu')).toBeVisible();
    await page.screenshot({ path: 'ui-screenshots/t23-quick-menu.png' });
    let l = await logs(page);
    expect(l.some((x) => x.includes('Guide held 500 ms: Quick Menu'))).toBe(true);
    // Rev. ax: Power, Brightness, Volume, Theme; Power first.
    await expect(page.getByTestId('qm-power')).toHaveClass(/focused/);
    await pad(page, 'DPadUp'); // the first row: stays
    await expect(page.getByTestId('qm-power')).toHaveClass(/focused/);
    await pad(page, 'DPadDown'); // Brightness
    await pad(page, 'DPadRight');
    await expect(page.getByTestId('qm-brightness-value')).toHaveAttribute('data-value', '71');
    await pad(page, 'DPadDown'); // Volume
    await pad(page, 'DPadRight');
    await pad(page, 'DPadRight');
    await expect(page.getByTestId('qm-volume-value')).toHaveAttribute('data-value', '52');
    await pad(page, 'RT');
    await expect(page.getByTestId('qm-volume-value')).toHaveAttribute('data-value', '62');
    await pad(page, 'A'); // mute
    await expect(page.getByTestId('qm-volume-value')).toHaveAttribute('data-muted', 'true');
    await page.waitForTimeout(500);
    l = await logs(page);
    expect(l.some((x) => x.includes('settings changed: {"volume":62,"muted":true}'))).toBe(true);
    await pad(page, 'DPadDown'); // Theme
    await pad(page, 'A');
    await expect(page.getByTestId('toast')).toHaveText('Coming in a later stage');
    await pad(page, 'DPadDown'); // the last row: stays
    await expect(page.getByTestId('qm-theme')).toHaveClass(/focused/);
    await pad(page, 'B');
    await expect(page.getByTestId('quick-menu')).toHaveCount(0);
  });

  test('a short Guide keeps today\'s behaviour (no Quick Menu)', async ({ page }) => {
    await open(page);
    await pad(page, 'Guide');
    await page.waitForTimeout(700);
    await expect(page.getByTestId('quick-menu')).toHaveCount(0);
    const l = await logs(page);
    expect(l.some((x) => x.includes('Guide held'))).toBe(false);
  });

  test('Power options (a centered list): Sleep Mode later; Restart PC asks (Cancel focused, B cancels); Turn Off PC confirmed; Close AIONE plays the Boot Down', async ({ page }) => {
    await open(page);
    await holdGuide(page);
    await pad(page, 'A'); // Power (the first row) → Power options, a short centered list
    await expect(page.getByTestId('power-options')).toBeVisible();
    await page.screenshot({ path: 'ui-screenshots/t23-power-options.png' });
    const items = page.getByTestId('po-item');
    await expect(items).toHaveCount(5);
    await expect(items.nth(0)).toHaveText('Sleep Mode');
    await pad(page, 'A'); // Sleep Mode
    await expect(page.getByTestId('toast')).toHaveText('Coming in a later stage');
    await pad(page, 'DPadDown'); // Restart PC
    await pad(page, 'A');
    await expect(page.getByTestId('confirm-dialog')).toContainText('Restart the PC?');
    await expect(page.getByTestId('confirm-no')).toHaveClass(/focused/);
    await page.screenshot({ path: 'ui-screenshots/t23-power-confirm.png' });
    await pad(page, 'B'); // Cancel
    await expect(page.getByTestId('confirm-dialog')).toHaveCount(0);
    let l = await logs(page);
    expect(l.some((x) => x.includes('power: restart the PC'))).toBe(false);
    // Turn Off PC, confirmed (from the drawer's Power button this time).
    await holdGuide(page);
    await pad(page, 'A');
    await pad(page, 'DPadDown');
    await pad(page, 'DPadDown'); // Turn Off PC
    await pad(page, 'A');
    await expect(page.getByTestId('confirm-dialog')).toContainText('Turn off the PC?');
    await pad(page, 'DPadLeft');
    await pad(page, 'A');
    l = await logs(page);
    expect(l.some((x) => x.includes('power: shutdown the PC (confirmed by the user)'))).toBe(true);
    // Close AIONE: the Boot Down.
    await holdGuide(page);
    await pad(page, 'A');
    for (let i = 0; i < 4; i++) await pad(page, 'DPadDown'); // Close AIONE (the last)
    await pad(page, 'DPadDown'); // stays on the last
    await pad(page, 'A');
    await expect(page.getByTestId('screen')).toHaveAttribute('data-phase', 'bootdown');
  });

  test('dialogs done once: a failed start is a one-button message; A or B closes it', async ({ page }) => {
    await open(page, 'launch=fail');
    await pad(page, 'A');
    await expect(page.getByTestId('confirm-dialog')).toContainText("Couldn't start", { timeout: 4000 });
    await expect(page.getByTestId('confirm-ok')).toHaveCount(1);
    await expect(page.getByTestId('confirm-yes')).toHaveCount(0);
    await pad(page, 'B');
    await expect(page.getByTestId('confirm-dialog')).toHaveCount(0);
  });

  test("rev. ax: a display AIONE can't control says so on the Brightness row", async ({ page }) => {
    await open(page, 'brightness=none');
    await holdGuide(page);
    await expect(page.getByTestId('qm-brightness-note')).toHaveText("Brightness can't be changed for this display");
  });

  test('B-28: the keyboard: Home held opens the Quick Menu, a short Home is Guide; "Menu" shows the Home key', async ({ page }) => {
    await open(page);
    await page.keyboard.press('ArrowRight'); // keyboard mode
    await expect(page.getByTestId('footer-prompts').first()).toHaveAttribute('data-device', 'keyboard');
    await expect(page.getByTestId('prompt-menu').locator('.glyph')).toHaveAttribute('data-glyph', 'keyboard_home.svg');
    await page.keyboard.down('Home');
    await page.waitForTimeout(650);
    await page.keyboard.up('Home');
    await expect(page.getByTestId('quick-menu')).toBeVisible();
    await page.keyboard.press('Escape'); // B
    await expect(page.getByTestId('quick-menu')).toHaveCount(0);
    await page.keyboard.press('Home'); // short: Guide (no game: nothing)
    await page.waitForTimeout(700);
    await expect(page.getByTestId('quick-menu')).toHaveCount(0);
    const l = await logs(page);
    expect(l.some((x) => x.includes('Home held 500 ms: Quick Menu'))).toBe(true);
  });
});

