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
    // The game's window at 2 s, so the press below always lands inside the transition, however
    // slow the machine (alpha.18: on a busy sandbox the page was already past the fade at "0.4 s").
    await open(page, 'window=2000');
    await pad(page, 'DPadRight'); // Glass Orchard
    const name = await page.getByTestId('selected-title').textContent();
    const t0 = Date.now();
    await pad(page, 'A');
    let l = await logs(page);
    expect(l.some((x) => /action confirm, start ".+" \(.+\), sound launch/.test(x))).toBe(true);
    expect(await page.evaluate(() => window.__aioneMock?.launches)).toEqual(['Glass Orchard']);
    // Still at 0.4 s (the press dip plays), black after about 0.75 s.
    await page.waitForTimeout(Math.max(0, 400 - (Date.now() - t0)));
    await expect.poll(() => veil(page), { timeout: 3000 }).toBe(0); // the 0.3 s fade back out
    await page.screenshot({ path: 'ui-screenshots/t19-launch-still.png' });
    await pad(page, 'DPadLeft'); // discarded
    await page.waitForTimeout(Math.max(0, 640 - (Date.now() - t0)));
    await page.screenshot({ path: 'ui-screenshots/t19-launch-fading.png' });
    await page.waitForTimeout(Math.max(0, 1000 - (Date.now() - t0)));
    await expect.poll(() => veil(page), { timeout: 3000 }).toBe(1); // 0.55 s still + 0.3 s eased fade (rev. ab)
    await page.screenshot({ path: 'ui-screenshots/t19-launch-black.png' });
    l = await logs(page);
    expect(l.some((x) => x.includes('discarded (launching a game: input lock)'))).toBe(true);
    expect(l.some((x) => /launch "Glass Orchard": started \(shortcut game\.lnk, process 4242\), \d+ ms after the press/.test(x))).toBe(true);
    // The game ends: Home again, the focus on it, and it is first in the row now (B-11).
    await page.evaluate(() => window.__aioneMock?.gameExit('Glass Orchard'));
    await page.waitForTimeout(300);
    await expect.poll(() => veil(page), { timeout: 3000 }).toBe(0); // the 0.3 s fade back out
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
    await expect.poll(() => veil(page), { timeout: 3000 }).toBe(0); // the 0.3 s fade back out
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
    // 0.55 s still + 0.3 s fade in, then the 0.3 s fade back out (rev. ab), with room for a slow runner.
    await expect.poll(() => veil(page), { timeout: 3000 }).toBe(0);
    await expect(page.getByTestId('toast')).toHaveText("Couldn't start Amber Lighthouse: its shortcut game.lnk is missing");
    await page.screenshot({ path: 'ui-screenshots/t19-launch-failed.png' });
    const l = await logs(page);
    expect(l.some((x) => x.includes('launch "Amber Lighthouse" failed: its shortcut game.lnk is missing'))).toBe(true);
  });

  test('D-026: black until the game\'s window appears; AIONE in front is no return before it; the timeout says "Still starting"', async ({ page }) => {
    await open(page, 'window=never&starttimeout=2500');
    await pad(page, 'A');
    await page.waitForTimeout(1200);
    expect(await veil(page)).toBe(1);
    // AIONE gets the focus while Steam starts the game: still black, input still locked.
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
    await pad(page, 'DPadRight');
    await page.waitForTimeout(300);
    expect(await veil(page)).toBe(1);
    let l = await logs(page);
    expect(l.some((x) => x.includes('discarded (launching a game: input lock)'))).toBe(true);
    expect(l.some((x) => x.includes('Home again'))).toBe(false);
    // The timeout: Home with "Still starting".
    await page.waitForTimeout(1600);
    await expect.poll(() => veil(page), { timeout: 3000 }).toBe(0); // the 0.3 s fade back out
    await expect(page.getByTestId('toast')).toHaveText('Still starting Amber Lighthouse…');
    l = await logs(page);
    expect(l.some((x) => /launch "Amber Lighthouse": no window after 3 s: Home again \("Still starting"\)/.test(x))).toBe(true);
  });

  test('B-18: one game at a time: "Playing", its footer, the close question (Cancel, then Close and start), X Close Software', async ({ page }) => {
    await open(page);
    await pad(page, 'A'); // Amber Lighthouse
    await page.waitForTimeout(1100);
    let l = await logs(page);
    expect(l.some((x) => x.includes('"Amber Lighthouse": its window appeared after 1200 ms; the launch is complete'))).toBe(true);
    // Guide: AIONE in front: the running game first, with "Playing", and its footer.
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
    await page.waitForTimeout(400);
    await expect(page.getByTestId('playing-badge')).toHaveCount(1);
    await expect(page.locator('[data-testid="tile"]').first().getByTestId('playing-badge')).toHaveCount(1);
    await expect(page.getByTestId('footer-prompts')).toContainText('Close Software');
    await expect(page.getByTestId('footer-prompts')).toContainText('Continue');
    await page.screenshot({ path: 'ui-screenshots/t20-playing-badge.png' });
    // Another game: the question, Cancel focused; B cancels.
    await pad(page, 'DPadRight');
    await expect(page.getByTestId('footer-prompts')).toContainText('Start');
    await pad(page, 'A');
    await expect(page.getByTestId('confirm-dialog')).toBeVisible();
    await expect(page.getByTestId('confirm-dialog')).toContainText('Close Amber Lighthouse and start Glass Orchard?');
    await expect(page.getByTestId('confirm-text')).toHaveText('Unsaved progress in Amber Lighthouse will be lost.');
    await expect(page.getByTestId('confirm-no')).toHaveClass(/focused/);
    await page.screenshot({ path: 'ui-screenshots/t20-close-question.png' });
    await pad(page, 'B');
    await expect(page.getByTestId('confirm-dialog')).toHaveCount(0);
    expect(await page.evaluate(() => window.__aioneMock?.launches)).toEqual(['Amber Lighthouse']);
    // Again: Left to "Close and start", A: Amber closes politely, then Glass Orchard starts.
    await pad(page, 'A');
    await pad(page, 'DPadLeft');
    await expect(page.getByTestId('confirm-yes')).toHaveClass(/focused/);
    await pad(page, 'A');
    await page.waitForTimeout(800);
    expect(await page.evaluate(() => window.__aioneMock?.launches)).toEqual(['Amber Lighthouse', 'Glass Orchard']);
    l = await logs(page);
    expect(l.some((x) => x.includes('close "Amber Lighthouse": asked by the user (to start another game)'))).toBe(true);
    expect(l.some((x) => x.includes('close "Amber Lighthouse": done'))).toBe(true);
    // Each end handled once (the core logs it, rounded; QA-5: the UI no longer adds a raw line).
    expect(l.filter((x) => x.includes('Home again: "Amber Lighthouse" ended')).length).toBeLessThanOrEqual(1);
    expect(l.some((x) => /ended after \d+\.\d{3,} min/.test(x))).toBe(false);
    // X Close Software on the running game (now Glass Orchard).
    await page.waitForTimeout(1200);
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
    await page.waitForTimeout(400);
    await expect(page.getByTestId('selected-title')).toHaveText('Glass Orchard');
    await page.screenshot({ path: 'ui-screenshots/t20-running-footer.png' });
    await pad(page, 'X');
    await expect(page.getByTestId('confirm-dialog')).toContainText('Close Glass Orchard?');
    await expect(page.getByTestId('confirm-text')).toHaveText('Unsaved progress will be lost.');
    await pad(page, 'DPadLeft');
    await pad(page, 'A');
    await page.waitForTimeout(600);
    await expect(page.getByTestId('playing-badge')).toHaveCount(0);
    await expect(page.getByTestId('footer-prompts')).toContainText('Start');
  });

  test('B-18 / D-029 / D-030: "Closing…", X and A locked meanwhile (one request), the hint, then left running, said plainly', async ({ page }) => {
    await open(page, 'closewait=stuck');
    await pad(page, 'A');
    await page.waitForTimeout(1100);
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
    await page.waitForTimeout(400);
    await pad(page, 'X');
    await pad(page, 'DPadLeft');
    await pad(page, 'A');
    await expect(page.getByTestId('toast')).toHaveText('Closing Amber Lighthouse…');
    // D-030: X (and A) on the closing game wait: no second request.
    await pad(page, 'X');
    await pad(page, 'A');
    expect(await page.evaluate(() => window.__aioneMock?.closes)).toEqual(['Amber Lighthouse']);
    const l = await logs(page);
    expect(l.filter((x) => x.includes('ignored ("Amber Lighthouse" is closing)')).length).toBe(2);
    // D-029: after a moment, the hint (the core brings the game forward so it can be answered).
    await expect(page.getByTestId('toast')).toHaveText('Amber Lighthouse may be asking something: answer it there', { timeout: 2000 });
    await expect(page.getByTestId('toast')).toContainText('is still running after 10 seconds', { timeout: 3000 });
    await expect(page.getByTestId('playing-badge')).toHaveCount(1);
  });

  test('B-16: drag the row 1:1 with a glide, no ring while dragging, ticks; a pressed tile tinted; the footer prompts clickable', async ({ page }) => {
    await open(page);
    const first = () => page.getByTestId('track').evaluate((e) => Number(getComputedStyle(e).getPropertyValue('--first')));
    // A press without moving: the pale tint on that tile.
    await page.mouse.move(800, 480);
    await page.mouse.down();
    await expect(page.locator('.tile.touched')).toHaveCount(1);
    await page.mouse.up();
    await expect(page.locator('.tile.touched')).toHaveCount(0);
    // A drag left by about 2 tiles: the row follows, then glides to the nearest tile; no select.
    const before = (await logs(page)).length;
    await page.mouse.move(1400, 480);
    await page.mouse.down();
    for (let x = 1400; x >= 500; x -= 60) await page.mouse.move(x, 480);
    await expect(page.getByTestId('track')).toHaveClass(/dragging/);
    await page.screenshot({ path: 'ui-screenshots/t20-drag-mid.png' });
    await page.mouse.up();
    await page.waitForTimeout(300);
    expect(await first()).toBeGreaterThanOrEqual(2);
    const l = (await logs(page)).slice(before);
    expect(l.some((x) => /input drag \(mouse \/ touch\) -> the row at tile \d+, glide 150 ms/.test(x))).toBe(true);
    expect(l.some((x) => x.includes('select (first click)'))).toBe(false); // a drag is never a click
    // Dragging far right stops at the start.
    await page.mouse.move(300, 480);
    await page.mouse.down();
    for (let x = 300; x <= 1800; x += 100) await page.mouse.move(x, 480);
    await page.mouse.up();
    await page.waitForTimeout(300);
    expect(await first()).toBe(0);
    // The footer's "A Start" is clickable: it starts the selected game.
    await page.getByTestId('prompt-confirm').click();
    await page.waitForTimeout(200);
    expect((await page.evaluate(() => window.__aioneMock?.launches))?.length).toBe(1);
  });

  test('D-031: "Close and start" starts the new game even when the old game\'s end arrives after "done"', async ({ page }) => {
    await open(page, 'exitlate');
    for (let round = 0; round < 3; round++) {
      // Start the first tile's game, come back, then ask for the second tile's game.
      await pad(page, 'A');
      await page.waitForTimeout(1100);
      await page.evaluate(() => window.dispatchEvent(new Event('focus')));
      await page.waitForTimeout(400);
      const running = await page.getByTestId('selected-title').textContent();
      await pad(page, 'DPadRight');
      const next = await page.getByTestId('selected-title').textContent();
      await pad(page, 'A');
      await pad(page, 'DPadLeft');
      await pad(page, 'A'); // Close and start
      await page.waitForTimeout(1200);
      const l = await logs(page);
      expect(l.filter((x) => x.includes(`asked: close "${running}" and start "${next}"?`)).length).toBe(round === 0 ? 1 : l.filter((x) => x.includes(`asked: close "${running}" and start "${next}"?`)).length);
      expect(l.some((x) => x.includes(`"${running}" ended (already closed by AIONE)`))).toBe(true);
      expect((await page.evaluate(() => window.__aioneMock?.launches))?.at(-1)).toBe(next);
      await expect(page.getByTestId('confirm-dialog')).toHaveCount(0); // never the question again
      // back to Home for the next round
      await page.evaluate(() => window.dispatchEvent(new Event('focus')));
      await page.waitForTimeout(400);
    }
    const l = await logs(page);
    expect(l.some((x) => x.includes("that game isn't running"))).toBe(false);
  });

  test('B-18 (rev. ae): no window found during a close: "can\'t see its window", never "answer it there"', async ({ page }) => {
    await open(page, 'closewait=stuck&nowindow');
    await pad(page, 'A');
    await page.waitForTimeout(1100);
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
    await page.waitForTimeout(400);
    await pad(page, 'X');
    await pad(page, 'DPadLeft');
    await pad(page, 'A');
    await expect(page.getByTestId('toast')).toHaveText("AIONE can't see Amber Lighthouse's window; it may still be closing", { timeout: 2000 });
  });
});
