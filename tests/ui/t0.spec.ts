import { expect, test } from '@playwright/test';
import { mkdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { SCREENSHOT_DIR, contrastOf, expectInViewport, expectNoOverflow, shot } from './helpers';

const pkg = JSON.parse(readFileSync('package.json', 'utf8')) as { version: string };

test.describe('T0 first window (mocked core)', () => {
  for (const vp of [
    { w: 1920, h: 1080 },
    { w: 1280, h: 720 },
    { w: 2560, h: 1440 },
  ]) {
    test(`shows AIONE 4 and the version at ${vp.w}x${vp.h}`, async ({ page }) => {
      await page.setViewportSize({ width: vp.w, height: vp.h });
      await page.goto('./');
      await expect(page.getByTestId('wordmark')).toHaveText('AIONE 4');
      await expect(page.getByTestId('version')).toHaveText(`Version ${pkg.version}`);
      await expect(page.getByTestId('copy-dialog')).toHaveCount(0);
      await expectNoOverflow(page);
      await expectInViewport(page, '[data-testid="wordmark"], [data-testid="version"], [data-testid="readout"]');
      expect(await contrastOf(page, '[data-testid="wordmark"]')).toBeGreaterThanOrEqual(4.5);
      expect(await contrastOf(page, '[data-testid="version"]')).toBeGreaterThanOrEqual(4.5);
      // The background is one flat colour (L-09).
      const bg = await page.evaluate(() => getComputedStyle(document.querySelector('main') ?? document.body).backgroundImage);
      expect(bg).toBe('none');
      await shot(page, `t0-home-${vp.w}x${vp.h}`);
    });
  }

  test('tells the core once that the first frame is drawn', async ({ page }) => {
    await page.goto('./');
    await expect(page.getByTestId('version')).toContainText(pkg.version);
    await expect.poll(() => page.evaluate(() => window.__aioneMock?.uiReadyCalls)).toBe(1);
  });

  test('readout: one controller press shows once; keyboard and mouse reach the same readout', async ({ page }) => {
    await page.goto('./');
    await expect(page.getByTestId('version')).toContainText(pkg.version);

    await page.evaluate(() => {
      window.__aioneMock?.pad('', 'connected', 0);
      window.__aioneMock?.pad('A', 'down');
      window.__aioneMock?.pad('A', 'up');
    });
    await expect(page.getByTestId('readout-last')).toHaveText('A (controller 1)');
    await expect(page.getByTestId('readout-presses')).toHaveText('1');
    await expect(page.getByTestId('readout-releases')).toHaveText('1');
    await expect(page.getByTestId('readout-pads')).toHaveText('Controller connected: slot 1');

    // Held direction: one press plus repeats.
    await page.evaluate(() => {
      window.__aioneMock?.pad('DPadDown', 'down');
      window.__aioneMock?.pad('DPadDown', 'repeat');
      window.__aioneMock?.pad('DPadDown', 'repeat');
      window.__aioneMock?.pad('DPadDown', 'up');
    });
    await expect(page.getByTestId('readout-presses')).toHaveText('2');
    await expect(page.getByTestId('readout-repeats')).toHaveText('2');

    await page.keyboard.press('ArrowRight');
    await expect(page.getByTestId('readout-last')).toHaveText('Right (keyboard)');
    await expect(page.getByTestId('readout-presses')).toHaveText('3');

    await page.mouse.click(400, 300);
    await expect(page.getByTestId('readout-last')).toHaveText('Left click (mouse)');
    await expect(page.getByTestId('readout-presses')).toHaveText('4');
    await expect(page.getByTestId('readout-sources')).toHaveText('Controller 2 · Keyboard 1 · Mouse 1');
    await shot(page, 't0-readout-after-inputs');
  });

  test('blocked browser keys do not reload the page; restart shortcut reaches the core', async ({ page }) => {
    await page.goto('./');
    await expect(page.getByTestId('version')).toContainText(pkg.version);
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('F5');
    await page.keyboard.press('Control+r');
    // A reload would reset the readout.
    await expect(page.getByTestId('readout-presses')).not.toHaveText('0');
    await page.keyboard.press('Control+Shift+R');
    await expect.poll(() => page.evaluate(() => window.__aioneMock?.restarts)).toBe(1);
  });
});

test.describe('T0 copy offer (mocked core)', () => {
  test('asks once with Copy focused; controller copies; result shows counts', async ({ page }) => {
    await page.goto('./?migration=offer');
    const dialog = page.getByTestId('copy-dialog');
    await expect(dialog).toHaveAttribute('data-phase', 'ask');
    await expect(page.getByTestId('copy-accept')).toHaveClass(/focused/);
    await expect(page.getByTestId('copy-decline')).not.toHaveClass(/focused/);
    await expect(dialog).toContainText('10 games');
    await expectNoOverflow(page);
    await expectInViewport(page, '[data-testid="copy-dialog"]');
    await shot(page, 't0-copy-offer');

    // B only moves focus; it never declines.
    await page.evaluate(() => window.__aioneMock?.pad('B'));
    await expect(page.getByTestId('copy-decline')).toHaveClass(/focused/);
    await expect(dialog).toHaveAttribute('data-phase', 'ask');
    await shot(page, 't0-copy-offer-focus-dont-copy');

    await page.evaluate(() => window.__aioneMock?.pad('DPadLeft'));
    await expect(page.getByTestId('copy-accept')).toHaveClass(/focused/);
    await page.evaluate(() => window.__aioneMock?.pad('A'));
    await expect(dialog).toHaveAttribute('data-phase', /copying|done/);
    await expect(dialog).toHaveAttribute('data-phase', 'done');
    await expect(page.getByTestId('copy-result-games')).toContainText('10 games (1 hidden, 3 favorites)');
    await expect(page.getByTestId('copy-result-ignored')).toContainText('_Removed');
    await expect(page.getByTestId('copy-result-source')).toHaveText('Your previous AIONE folder was not changed.');
    expect(await page.evaluate(() => window.__aioneMock?.decisions)).toEqual([true]);
    await expectInViewport(page, '[data-testid="copy-dialog"]');
    await shot(page, 't0-copy-done');

    await page.evaluate(() => window.__aioneMock?.pad('A'));
    await expect(dialog).toHaveCount(0);
  });

  test('progress is shown while copying', async ({ page }) => {
    await page.goto('./?migration=offer');
    await page.keyboard.press('Enter');
    await expect(page.getByTestId('copy-progress-text')).toContainText('of 1,234 files');
    // Taken at once: the mocked copy finishes within about 0.6 s.
    mkdirSync(SCREENSHOT_DIR, { recursive: true });
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 't0-copy-progress.png') });
  });

  test('mouse: Don\'t copy declines once; no focus ring in mouse mode', async ({ page }) => {
    await page.goto('./?migration=offer');
    await page.mouse.move(10, 10);
    await expect(page.getByTestId('screen')).toHaveAttribute('data-input', 'mouse');
    const outline = await page.getByTestId('copy-accept').evaluate((el) => getComputedStyle(el).outlineColor);
    expect(outline).toBe('rgba(0, 0, 0, 0)');
    await shot(page, 't0-copy-offer-mouse-mode');
    await page.getByTestId('copy-decline').click();
    await expect(page.getByTestId('copy-dialog')).toHaveCount(0);
    expect(await page.evaluate(() => window.__aioneMock?.decisions)).toEqual([false]);
  });

  test('a partial copy says so', async ({ page }) => {
    await page.goto('./?migration=copy-fails');
    await page.getByTestId('copy-accept').click();
    await expect(page.getByTestId('copy-dialog')).toHaveAttribute('data-phase', 'done');
    await expect(page.getByTestId('copy-dialog')).toContainText('The copy is incomplete');
    await expect(page.getByTestId('copy-dialog')).toContainText('2 files could not be copied');
    await shot(page, 't0-copy-partial');
  });

  test('no dialog when the data was handled before', async ({ page }) => {
    await page.goto('./?migration=handled');
    await expect(page.getByTestId('version')).toContainText(pkg.version);
    await expect(page.getByTestId('copy-dialog')).toHaveCount(0);
  });
});
