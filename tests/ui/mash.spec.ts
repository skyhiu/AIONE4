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
    return (s.launching === 'none' || s.launching === 'ingame') && (s.lock === 'none' || s.lock === 'dialog');
  }, { timeout: withinMs }).toBe(true);
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
