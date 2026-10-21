import { expect, test } from '@playwright/test';
import { mkdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { SCREENSHOT_DIR, expectInViewport, expectNoOverflow, shot } from './helpers';

const pkg = JSON.parse(readFileSync('package.json', 'utf8')) as { version: string };

test.describe('T0 copy offer (mocked core)', () => {
  test('asks with Copy focused, plain counts and hints; controller copies; result shows counts', async ({ page }) => {
    await page.goto('./?migration=offer');
    const dialog = page.getByTestId('copy-dialog');
    await expect(dialog).toHaveAttribute('data-phase', 'ask');
    await expect(page.getByTestId('copy-accept')).toHaveClass(/focused/);
    await expect(page.getByTestId('copy-decline')).not.toHaveClass(/focused/);
    await expect(page.getByTestId('copy-offer-counts')).toHaveText('10 games · 0 hidden · 0 favorites · 0 folders');
    await expect(dialog).toContainText('Skipped: _Removed (not a game)');
    await expect(page.getByTestId('copy-hints')).toHaveText('A Select · B Not now');
    // B-2: the question is no longer "asked only once".
    await expect(dialog).not.toContainText('only once');
    await expect(dialog).toContainText('Not now (B) asks again at the next start');
    await expectNoOverflow(page);
    await expectInViewport(page, '[data-testid="copy-dialog"]');
    await shot(page, 't0-copy-offer');

    await page.evaluate(() => window.__aioneMock?.pad('DPadRight'));
    await expect(page.getByTestId('copy-decline')).toHaveClass(/focused/);
    await shot(page, 't0-copy-offer-focus-dont-copy');
    await page.evaluate(() => window.__aioneMock?.pad('DPadLeft'));
    await expect(page.getByTestId('copy-accept')).toHaveClass(/focused/);
    await page.evaluate(() => window.__aioneMock?.pad('A'));
    await expect(dialog).toHaveAttribute('data-phase', 'done');
    await expect(page.getByTestId('copy-result-games')).toHaveText('10 games · 0 hidden · 0 favorites · 0 folders');
    await expect(page.getByTestId('copy-result-ignored')).toHaveText('Skipped: _Removed (not a game)');
    await expect(page.getByTestId('copy-result-hints')).toHaveText('A OK');
    await expect(page.getByTestId('copy-result-source')).toHaveText('Your previous AIONE folder was not changed.');
    expect(await page.evaluate(() => window.__aioneMock?.decisions)).toEqual([true]);
    await expectInViewport(page, '[data-testid="copy-dialog"]');
    await shot(page, 't0-copy-done');

    await page.evaluate(() => window.__aioneMock?.pad('A'));
    await expect(dialog).toHaveCount(0);
  });

  test('B = Not now: closes without deciding (B-2)', async ({ page }) => {
    await page.goto('./?migration=offer');
    await expect(page.getByTestId('copy-dialog')).toHaveAttribute('data-phase', 'ask');
    await page.evaluate(() => window.__aioneMock?.pad('B'));
    await expect(page.getByTestId('copy-dialog')).toHaveCount(0);
    expect(await page.evaluate(() => window.__aioneMock?.decisions)).toEqual([]);
    const logs = (await page.evaluate(() => window.__aioneMock?.logs)) ?? [];
    expect(logs.some((l) => l.includes('user chose Not now'))).toBe(true);
    await shot(page, 't0-copy-not-now-closed');
  });

  test('B on "Don\'t copy" is still Not now; A on "Don\'t copy" declines', async ({ page }) => {
    await page.goto('./?migration=offer');
    await expect(page.getByTestId('copy-dialog')).toHaveAttribute('data-phase', 'ask');
    await page.evaluate(() => window.__aioneMock?.pad('DPadRight'));
    await page.evaluate(() => window.__aioneMock?.pad('B'));
    await expect(page.getByTestId('copy-dialog')).toHaveCount(0);
    expect(await page.evaluate(() => window.__aioneMock?.decisions)).toEqual([]);

    await page.goto('./?migration=offer');
    await expect(page.getByTestId('copy-dialog')).toHaveAttribute('data-phase', 'ask');
    await page.evaluate(() => window.__aioneMock?.pad('DPadRight'));
    await expect(page.getByTestId('copy-decline')).toHaveClass(/focused/);
    await page.evaluate(() => window.__aioneMock?.pad('A'));
    await expect(page.getByTestId('copy-dialog')).toHaveCount(0);
    expect(await page.evaluate(() => window.__aioneMock?.decisions)).toEqual([false]);
  });

  test('D-003: the bar matches the numbers under it after animations settle', async ({ page }) => {
    // Slow steps (3 s each) so the 25 % state can be measured at rest.
    await page.goto('./?migration=offer&step=3000');
    await expect(page.getByTestId('copy-dialog')).toHaveAttribute('data-phase', 'ask');
    await page.keyboard.press('Enter');
    const percent = page.getByTestId('copy-progress-percent');
    await expect(percent).toHaveText('25 %');
    const ratio = async () =>
      page.evaluate(() => {
        const fill = document.querySelector('[data-testid="copy-progress-fill"]');
        const track = document.querySelector('[data-testid="copy-progress-track"]');
        if (!fill || !track) return -1;
        return fill.getBoundingClientRect().width / track.getBoundingClientRect().width;
      });
    // Wait until the bar is at rest (two equal readings), then compare with the text.
    await expect
      .poll(async () => {
        const a = await ratio();
        await page.waitForTimeout(100);
        return Math.abs((await ratio()) - a);
      })
      .toBeLessThan(0.001);
    const shown = Number(await percent.getAttribute('data-fraction'));
    expect(shown).toBeCloseTo(0.25, 5);
    expect(Math.abs((await ratio()) - shown)).toBeLessThanOrEqual(0.02);
    // The bar animates by transform only (A7).
    const prop = await page
      .getByTestId('copy-progress-fill')
      .evaluate((el) => getComputedStyle(el).transitionProperty);
    expect(prop).toBe('transform');
    mkdirSync(SCREENSHOT_DIR, { recursive: true });
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 't0-copy-progress.png') });
  });

  test('mouse: Don\'t copy declines once; no focus ring in mouse mode', async ({ page }) => {
    await page.goto('./?migration=offer');
    await expect(page.getByTestId('copy-dialog')).toHaveAttribute('data-phase', 'ask');
    await page.mouse.move(10, 10);
    await page.mouse.move(40, 40, { steps: 4 }); // deliberate movement (A7)
    await expect(page.getByTestId('screen')).toHaveAttribute('data-input', 'mouse');
    // Rev. ba (the shared dialog): the focus is the button's ring; in mouse mode it isn't drawn.
    const ring = await page.getByTestId('copy-accept').locator('.ring').evaluate((el) => getComputedStyle(el).borderTopColor);
    expect(ring).toBe('rgba(0, 0, 0, 0)');
    await shot(page, 't0-copy-offer-mouse-mode');
    await page.getByTestId('copy-decline').click();
    await expect(page.getByTestId('copy-dialog')).toHaveCount(0);
    expect(await page.evaluate(() => window.__aioneMock?.decisions)).toEqual([false]);
    // QA-5: the click line names the action it activated.
    const logs = (await page.evaluate(() => window.__aioneMock?.logs)) ?? [];
    expect(logs.some((l) => l.startsWith('info: input click "Don\'t copy" (mouse) -> action dont-copy'))).toBe(true);
  });

  test('a partial copy says so', async ({ page }) => {
    await page.goto('./?migration=copy-fails');
    await expect(page.getByTestId('copy-dialog')).toHaveAttribute('data-phase', 'ask');
    await page.getByTestId('copy-accept').click();
    await expect(page.getByTestId('copy-dialog')).toHaveAttribute('data-phase', 'done');
    await expect(page.getByTestId('copy-dialog')).toContainText('The copy is incomplete');
    await expect(page.getByTestId('copy-dialog')).toContainText('2 files could not be copied');
    await expect(page.getByTestId('copy-result-hints')).toHaveText('A OK');
    await shot(page, 't0-copy-partial');
    // B closes a result dialog too (nothing is left to decide).
    await page.keyboard.press('Escape');
    await expect(page.getByTestId('copy-dialog')).toHaveCount(0);
    const logs = (await page.evaluate(() => window.__aioneMock?.logs)) ?? [];
    expect(logs.some((l) => l.startsWith('info: input click "Copy" (mouse) -> action copy'))).toBe(true);
  });

  for (const record of ['alpha1', 'alpha2']) {
    test(`B-6: started over the ${record} record, no question appears`, async ({ page }) => {
      await page.goto(`./?migration=offer&record=${record}`);
      await expect(page.getByTestId('screen')).toHaveAttribute('data-version', pkg.version);
      // Give the startup query time to answer, then make sure nothing appeared.
      await page.waitForTimeout(500);
      await expect(page.getByTestId('copy-dialog')).toHaveCount(0);
    });
  }

  test('B-6: an unreadable record with a finished copy is reported, not asked', async ({ page }) => {
    await page.goto('./?migration=offer&record=unreadable&copy=finished');
    const dialog = page.getByTestId('copy-dialog');
    await expect(dialog).toHaveAttribute('data-phase', 'notice');
    await expect(page.getByTestId('copy-notice')).toContainText('could not read its note');
    await expect(page.getByTestId('copy-result-hints')).toHaveText('A OK');
    await expect(page.getByTestId('copy-accept')).toHaveCount(0);
    await expectInViewport(page, '[data-testid="copy-dialog"]');
    await shot(page, 't0-copy-record-unreadable-notice');
    await page.evaluate(() => window.__aioneMock?.pad('A'));
    await expect(dialog).toHaveCount(0);
  });

  test('no dialog when the data was handled before', async ({ page }) => {
    await page.goto('./?migration=handled');
    await expect(page.getByTestId('screen')).toHaveAttribute('data-version', pkg.version);
    await expect(page.getByTestId('copy-dialog')).toHaveCount(0);
  });
});
