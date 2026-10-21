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
    await expect(page.getByTestId('qm-volume-muted')).toHaveAttribute('data-muted', 'true');
    await expect(page.getByTestId('qm-volume-value')).toHaveAttribute('data-value', '0'); // the slider shows mute
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
    // Rev. ay: its own footer hints.
    await expect(page.getByTestId('po-footer').getByTestId('prompt-back')).toHaveCount(1);
    await expect(page.getByTestId('po-footer').getByTestId('prompt-confirm')).toContainText('OK');
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

  test('rev. ay: a held direction moves a slider smoothly (steady, no jumps) and applies live', async ({ page }) => {
    await open(page, 'volume=20');
    await holdGuide(page);
    await pad(page, 'DPadDown');
    await pad(page, 'DPadDown'); // Volume
    await hold(page, 'DPadRight');
    // The core repeats a held direction (XInput); the test sends those repeats, sampling the knob.
    const seen: number[] = [];
    const at: number[] = [];
    for (let i = 0; i < 14; i++) {
      await page.waitForTimeout(70);
      await page.evaluate(() => window.__aioneMock?.pad('DPadRight', 'repeat'));
      const v = await page.evaluate(() => ({ v: Number(document.querySelector('[data-testid="qm-volume-value"]')?.getAttribute('data-value')), t: performance.now() }));
      seen.push(v.v);
      at.push(v.t);
    }
    await release(page, 'DPadRight');
    // Steady: it rises, never by more than a few % between samples (no visible jumps).
    expect(seen[seen.length - 1] ?? 0).toBeGreaterThan(seen[0] ?? 0);
    // The movement is time-based (50 % per second): no step larger than the steady speed allows for
    // the real time between two samples.
    const val = (i: number) => seen[i] ?? 0;
    const time = (i: number) => at[i] ?? 0;
    for (let i = 1; i < seen.length; i++) {
      // The sample's clock and the animation frames drift by a frame or so on a busy machine.
      const allowed = ((time(i) - time(i - 1)) / 1000) * 50 + 6;
      expect(val(i) - val(i - 1)).toBeLessThanOrEqual(allowed);
    }
    // The steady speed over the whole hold: about 50 % per second.
    const rate = (val(seen.length - 1) - val(0)) / ((time(seen.length - 1) - time(0)) / 1000);
    expect(rate).toBeGreaterThan(30);
    expect(rate).toBeLessThan(70);
    const l = await logs(page);
    expect(l.some((x) => x.includes('Quick Menu: Volume held'))).toBe(true);
  });

  test("B-29: the edge bump everywhere (one walk through every focusable screen's edges)", async ({ page }) => {
    await open(page, 'volume=97&brightness=3');
    const bumped = async (sel: string, dir: string) => {
      const el = page.locator(sel).first();
      await expect(el).toHaveAttribute('data-bump-dir', dir);
    };
    const lastLog = async (zone: string, dir: string) => {
      const l = await logs(page);
      expect(l.some((x) => x.includes(`bump ${dir} (${zone})`) && x.includes('sound edgeBump'))).toBe(true);
    };
    await holdGuide(page);
    // The Quick Menu: top and bottom.
    await pad(page, 'DPadUp');
    await bumped('[data-testid="quick-menu"] .item.focused .ring', 'up');
    await lastLog('qm', 'up');
    for (let i = 0; i < 3; i++) await pad(page, 'DPadDown');
    await pad(page, 'DPadDown');
    await bumped('[data-testid="quick-menu"] .item.focused .ring', 'down');
    // The sliders: Brightness at 0 %, Volume at 100 % (the knob bumps toward the press).
    await pad(page, 'DPadUp');
    await pad(page, 'DPadUp'); // Brightness (3 %)
    for (let i = 0; i < 4; i++) await pad(page, 'DPadLeft'); // 3 → 0, then the edge
    await bumped('[data-testid="qm-brightness-value"] .knob', 'left');
    await lastLog('slider-brightness', 'left');
    await pad(page, 'DPadDown'); // Volume (97 %)
    for (let i = 0; i < 4; i++) await pad(page, 'DPadRight');
    await bumped('[data-testid="qm-volume-value"] .knob', 'right');
    // D-019: a repeat held at the same edge: no second bump.
    await page.evaluate(() => window.__aioneMock?.pad('DPadRight', 'down'));
    await page.evaluate(() => window.__aioneMock?.pad('DPadRight', 'repeat'));
    await page.evaluate(() => window.__aioneMock?.pad('DPadRight', 'repeat'));
    await page.evaluate(() => window.__aioneMock?.pad('DPadRight', 'up'));
    const l = await logs(page);
    expect(l.some((x) => x.includes('held: no bump'))).toBe(true);
    // Power options: first and last.
    await pad(page, 'DPadUp');
    await pad(page, 'DPadUp'); // Power
    await pad(page, 'A');
    await pad(page, 'DPadUp');
    await bumped('[data-testid="power-options"] .item.focused .ring', 'up');
    for (let i = 0; i < 4; i++) await pad(page, 'DPadDown');
    await pad(page, 'DPadDown');
    await bumped('[data-testid="power-options"] .item.focused .ring', 'down');
    await lastLog('po', 'down');
    // A question's buttons: Cancel is focused (the right end); right, then left past Restart; up.
    for (let i = 0; i < 3; i++) await pad(page, 'DPadUp'); // Restart PC
    await pad(page, 'A');
    await expect(page.getByTestId('confirm-dialog')).toContainText('Restart the PC?');
    await pad(page, 'DPadRight');
    await bumped('[data-testid="confirm-dialog"] button.focused .ring', 'right');
    await pad(page, 'DPadLeft'); // to Restart
    await pad(page, 'DPadLeft');
    await bumped('[data-testid="confirm-dialog"] button.focused .ring', 'left');
    await pad(page, 'DPadUp');
    await bumped('[data-testid="confirm-dialog"] button.focused .ring', 'up');
    await lastLog('dialog', 'up');
    await pad(page, 'B'); // Cancel: nothing done
  });

  test("rev. ba: T0's copy question on the shared dialog; --show-copy-question previews it (nothing copied)", async ({ page }) => {
    await open(page, 'copyquestion=1');
    const d = page.getByTestId('copy-dialog');
    await expect(d).toBeVisible({ timeout: 4000 });
    await expect(d).toHaveAttribute('role', 'alertdialog'); // the shared dialog
    await expect(d).toContainText('Copy your AIONE data?');
    await expect(page.getByTestId('copy-accept')).toHaveClass(/focused/);
    await pad(page, 'DPadLeft'); // already the first button: B-29's bump
    await expect(page.locator('[role="alertdialog"] button.focused .ring')).toHaveAttribute('data-bump-dir', 'left');
    await pad(page, 'B'); // Not now: in the preview, nothing is recorded
    await expect(d).toHaveCount(0);
    const l = await logs(page);
    expect(l.some((x) => x.includes('copy question (preview, --show-copy-question): Not now: nothing copied or recorded'))).toBe(true);
    expect(await page.evaluate(() => window.__aioneMock?.decisions ?? [])).toEqual([]);
  });
});

