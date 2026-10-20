import { expect, test, type Page } from '@playwright/test';

// B-22 (rev. an): a seeded, reproducible button-mashing test, run on every build. During each
// transition (a launch, the launcher wait, a return, a close), random Guide / A / B / D-pad
// presses, releases and holds; then everything is let go. AIONE must reach a sane state (Home, or
// the game in front) within the transition's limit + 1 s; never two foreground attempts at once;
// no lock surviving the game's end. Boot Up is mashed in the core's Rust test (the UI tests open
// on Home).

const SEEDS = [1, 42, 2026];
const CONTROLS = ['Guide', 'A', 'B', 'DPadUp', 'DPadDown', 'DPadLeft', 'DPadRight'];

/** The same sequence for the same seed. */
function lcg(seed: number) {
  let x = seed >>> 0;
  return () => {
    x = (Math.imul(x, 1664525) + 1013904223) >>> 0;
    return x / 2 ** 32;
  };
}

async function open(page: Page, extra = '') {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto(`./?time=1:10&h12=1&pad=connected${extra ? `&${extra}` : ''}`);
  await page.locator('main[data-phase="home"]').waitFor();
  await page.waitForTimeout(300);
}
const send = (page: Page, c: string, kind: 'down' | 'up') => page.evaluate(([x, k]) => window.__aioneMock?.pad(x as string, k as 'down' | 'up'), [c, kind]);
const press = async (page: Page, c: string) => {
  await send(page, c, 'down');
  await send(page, c, 'up');
};
const state = (page: Page) => page.getByTestId('screen').evaluate((e) => ({ launching: e.getAttribute('data-launching'), lock: e.getAttribute('data-lock') }));

/** Mash for `ms`: presses, releases, and holds that are let go only at the end. */
async function mash(page: Page, rnd: () => number, ms: number, controls = CONTROLS) {
  const heldNow = new Set<string>();
  const end = Date.now() + ms;
  let n = 0;
  while (Date.now() < end) {
    const c = controls[Math.floor(rnd() * controls.length)] as string;
    const r = rnd();
    if (r < 0.6) await press(page, c);
    else if (r < 0.85) {
      await send(page, c, 'down');
      heldNow.add(c);
    } else if (heldNow.has(c)) {
      await send(page, c, 'up');
      heldNow.delete(c);
    }
    n += 1;
    await page.waitForTimeout(10 + Math.floor(rnd() * 60));
  }
  for (const c of heldNow) await send(page, c, 'up');
  return n;
}

/** Sane: Home ('none') or the game in front ('ingame'), and no lock. */
async function expectSane(page: Page, withinMs: number) {
  await expect.poll(async () => {
    const s = await state(page);
    return (s.launching === 'none' || s.launching === 'ingame') && (s.lock === 'none' || s.lock === 'dialog' || s.lock === 'quick-menu');
  }, { timeout: withinMs }).toBe(true);
  // T2.2: a held Guide in the mashing may have opened the Quick Menu (or a message): close it
  // with B, as the user would, before the next step.
  for (let i = 0; i < 4; i++) {
    const s = await state(page);
    if (s.lock !== 'quick-menu' && s.lock !== 'dialog') break;
    await press(page, 'B');
    await page.waitForTimeout(250);
  }
}

async function startGame(page: Page) {
  await press(page, 'A');
  await page.waitForTimeout(1100);
}
async function homeAgain(page: Page) {
  await page.evaluate(() => window.dispatchEvent(new Event('focus')));
  await page.waitForTimeout(400);
}

for (const seed of SEEDS) {
  test(`B-22 button-mashing, seed ${seed}: every transition ends sane; one foreground attempt at a time; no lock after the game's end`, async ({ page }) => {
    test.setTimeout(120_000);
    const rnd = lcg(seed);
    const counts: string[] = [];

    // 1. A launch (the game's window comes; mashing from the press until it is up).
    await open(page);
    await press(page, 'A');
    counts.push(`launch ${await mash(page, rnd, 1500)}`);
    await expectSane(page, 46_000 + 1000);

    // 2. A return (Continue), mashed through the fade and the wait (limit 3 s).
    await homeAgain(page);
    if ((await state(page)).launching === 'none') {
      await send(page, 'A', 'down');
      counts.push(`return ${await mash(page, rnd, 1200)}`);
      await send(page, 'A', 'up');
      await expectSane(page, 3000 + 1000);
    }

    // 2b. A and Guide spammed at the running game from Home: returns happen, never two foreground
    // attempts at once (D-043).
    await homeAgain(page);
    counts.push(`A/Guide spam ${await mash(page, rnd, 1500, ['A', 'Guide'])}`);
    await expectSane(page, 3000 + 1000);
    expect((await page.evaluate(() => window.__aioneMock?.returns ?? [])).length).toBeGreaterThan(0);

    // 2c. D-045: a media key and Alt (never counted as held), then a return: fast.
    await homeAgain(page);
    await page.keyboard.press('MediaTrackNext');
    await page.keyboard.press('Alt');
    {
      const before = (await page.evaluate(() => window.__aioneMock?.returns ?? [])).length;
      const t0 = Date.now();
      await press(page, 'A');
      await expect.poll(async () => (await page.evaluate(() => window.__aioneMock?.returns ?? [])).length, { timeout: 2000 }).toBe(before + 1);
      expect(Date.now() - t0).toBeLessThan(1500);
      counts.push('media key + Alt, then a fast return');
    }

    // 3. A close (X, Close), mashed until the game has ended (limit 7 s).
    await homeAgain(page);
    await press(page, 'X');
    await press(page, 'DPadLeft');
    await press(page, 'A');
    counts.push(`close ${await mash(page, rnd, 1500)}`);
    await expectSane(page, 7000 + 1000);

    // The foreground attempts so far (the page reloads for the launcher next).
    const mostBefore = await page.evaluate(() => window.__aioneMock?.maxInFlight ?? 0);

    // 4. The launcher wait (a launcher, then the game), mashed throughout (limit 6 s).
    await page.goto('./?time=1:10&h12=1&pad=connected&launcher=play&followat=1500');
    await page.locator('main[data-phase="home"]').waitFor();
    await page.waitForTimeout(300);
    await press(page, 'A');
    counts.push(`launcher wait ${await mash(page, rnd, 3000)}`);
    await expectSane(page, 6000 + 1000);

    // 4b. D-046: a slow start mashed (B included): it ends sane, and no game is left running.
    await page.goto('./?time=1:10&h12=1&pad=connected&window=5000');
    await page.locator('main[data-phase="home"]').waitFor();
    await page.waitForTimeout(300);
    await press(page, 'A');
    await page.waitForTimeout(3200); // "B Cancel start" is offered
    counts.push(`slow start ${await mash(page, rnd, 2500)}`);
    await expectSane(page, 46_000 + 1000);
    await page.waitForTimeout(800);
    // D-046: a cancelled start never leaves a game running untracked. A game running now must come
    // from a later, explicit start (the mashing's A on Home), never from the cancelled one.
    if ((await state(page)).launching === 'none' && (await page.getByTestId('playing-badge').count()) > 0) {
      const all = await page.evaluate(() => window.__aioneMock?.logs ?? []);
      const cancelAt = all.map((x) => x.includes('the start was cancelled (B)')).lastIndexOf(true);
      const startedAfter = all.slice(cancelAt + 1).some((x) => /-> action confirm, start "/.test(x));
      expect(cancelAt < 0 || startedAfter).toBe(true);
    }

    // 5. A lock, then the game ends: nothing survives.
    await startGame(page);
    await homeAgain(page);
    await send(page, 'A', 'down'); // a return, held
    await page.waitForTimeout(400);
    await page.evaluate(() => window.__aioneMock?.gameExit('Amber Lighthouse'));
    await send(page, 'A', 'up');
    await expectSane(page, 1000);

    const most = Math.max(mostBefore, await page.evaluate(() => window.__aioneMock?.maxInFlight ?? 0));
    expect(most).toBe(1); // attempts happened, and never two at once
    console.warn(`button-mashing seed ${seed}: ${counts.join(', ')} inputs; every transition ended sane; at most ${most} foreground attempt at once; no lock after the game's end`);
  });
}

// D-046 rev. ar and D-048 (B-22 rev. aq): a window of the launcher or Steam during a start.
async function openWith(page: Page, q: string) {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto(`./?time=1:10&h12=1&pad=connected&${q}`);
  await page.locator('main[data-phase="home"]').waitFor();
  await page.waitForTimeout(300);
}
const logs = (page: Page) => page.evaluate(() => window.__aioneMock?.logs ?? []);

test("D-046 rev. ar: the launcher's window up 10 s, B mashed: nothing cancelled; the game is followed", async ({ page }) => {
  test.setTimeout(60_000);
  await openWith(page, 'showing=launcher&showingms=10000&window=11000');
  await press(page, 'A');
  await page.waitForTimeout(3500);
  await mash(page, lcg(7), 4000, ['B', 'B', 'A', 'DPadLeft', 'Guide']);
  await expect(page.getByTestId('veil-text')).not.toContainText('B Cancel start');
  let l = await logs(page);
  expect(l.some((x) => x.includes('B ignored (the launcher is showing a window)'))).toBe(true);
  expect(l.some((x) => x.includes('the start was cancelled (B)'))).toBe(false);
  await expect.poll(async () => (await state(page)).launching, { timeout: 8000 }).toBe('ingame'); // followed
  l = await logs(page);
  expect(l.some((x) => x.includes('its window appeared'))).toBe(true);
});

test("D-048: Steam's window in front for 10 s, then the game: the wait extends and the game is followed", async ({ page }) => {
  test.setTimeout(60_000);
  await openWith(page, 'launcher=steam&showing=steam&showingms=10000&steamgame=1&followat=1500');
  await press(page, 'A');
  await page.waitForTimeout(2000);
  await mash(page, lcg(7), 4000, ['B', 'A', 'DPadRight', 'Guide']);
  let l = await logs(page);
  expect(l.some((x) => x.includes('Steam is showing a window during the start: waiting'))).toBe(true);
  expect((await state(page)).launching).toBe('starting'); // still waiting at about 6.5 s (no 5 s give-up)
  await expect.poll(async () => (await state(page)).launching, { timeout: 8000 }).toBe('ingame');
  l = await logs(page);
  expect(l.some((x) => x.includes('the start was cancelled (B)'))).toBe(false);
});

test("D-048: Steam's window and no game: the start ends at its limit; AIONE stays behind Steam's window", async ({ page }) => {
  test.setTimeout(60_000);
  await openWith(page, 'launcher=steam&showing=steam&showingms=60000&startlimit=8000&followat=1500');
  await press(page, 'A');
  await page.waitForTimeout(2000);
  await mash(page, lcg(7), 3000, ['B', 'A', 'Guide']);
  await expectSane(page, 8000 + 1000);
  const l = await logs(page);
  expect(l.some((x) => x.includes('the start ended while Steam is showing a window: AIONE stays behind'))).toBe(true);
});

test('D-046: nothing showing for 3 s, then B: the start is cancelled (as before)', async ({ page }) => {
  test.setTimeout(60_000);
  await openWith(page, 'window=8000');
  await press(page, 'A');
  await page.waitForTimeout(3600);
  await expect(page.getByTestId('veil-text')).toContainText('B Cancel start');
  await press(page, 'B');
  await expectSane(page, 1500);
  const l = await logs(page);
  expect(l.some((x) => x.includes('the start was cancelled (B)'))).toBe(true);
});

test('alpha.29 QA-1: a launcher hiding its window on Play: input locked at once; A mashed never acts on Home', async ({ page }) => {
  test.setTimeout(60_000);
  await openWith(page, 'launcher=play&guidems=999999&followat=2500');
  await press(page, 'A');
  await page.waitForTimeout(1500); // the launcher's window appeared (not the game)
  const before = (await logs(page)).length;
  await page.evaluate(() => window.dispatchEvent(new Event('focus'))); // the launcher hides its window on Play
  await mash(page, lcg(29), 800, ['A', 'A', 'DPadRight', 'X']); // mashed right then
  await expect.poll(async () => (await state(page)).launching, { timeout: 6000 }).toBe('ingame'); // the game followed
  const l = (await logs(page)).slice(before);
  expect(l.some((x) => /-> action confirm, start "/.test(x))).toBe(false); // nothing started from Home
  expect(l.some((x) => x.includes('asked: '))).toBe(false); // no question
  expect(l.some((x) => x.includes('Home again'))).toBe(false); // Home never live
});

test("B-27: Windows' permission prompt during a start: said on the black screen; B mashed does nothing", async ({ page }) => {
  test.setTimeout(60_000);
  await openWith(page, 'consent=6000&window=7000');
  await press(page, 'A');
  await page.waitForTimeout(1200);
  await expect(page.getByTestId('veil-text')).toHaveText('Windows is asking for permission: answer it with the mouse or keyboard');
  await mash(page, lcg(27), 3000, ['B', 'B', 'A', 'Guide']);
  await expect(page.getByTestId('veil-text')).not.toContainText('B Cancel start');
  const l = await logs(page);
  expect(l.some((x) => x.includes('the start was cancelled (B)'))).toBe(false);
  await expect.poll(async () => (await state(page)).launching, { timeout: 6000 }).toBe('ingame'); // the game, once answered
});

test('B-27: a close Windows refuses (the game runs as administrator): said plainly; bring it to the front to close it there', async ({ page }) => {
  test.setTimeout(60_000);
  await openWith(page, 'closeadmin=1');
  await press(page, 'A');
  await page.waitForTimeout(1100);
  await page.evaluate(() => window.dispatchEvent(new Event('focus')));
  await page.waitForTimeout(400);
  await press(page, 'X');
  await press(page, 'DPadLeft');
  await press(page, 'A'); // Close
  await expect(page.getByTestId('confirm-dialog')).toContainText('Bring Amber Lighthouse to the front to close it there?', { timeout: 6000 });
  await expect(page.getByTestId('confirm-text')).toHaveText("Amber Lighthouse runs as administrator: AIONE can't close it");
  await expect(page.getByTestId('confirm-no')).toHaveClass(/focused/);
  await press(page, 'DPadLeft');
  await press(page, 'A'); // Bring to front
  await expect.poll(async () => (await page.evaluate(() => window.__aioneMock?.returns ?? [])).length, { timeout: 3000 }).toBe(1);
});
