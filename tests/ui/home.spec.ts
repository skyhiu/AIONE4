import { expect, test, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { contrastOf, expectInViewport, expectNoOverflow, shot, waitForIcons } from './helpers';
import { contrast, decodePng, pixel } from './png';

const pkg = JSON.parse(readFileSync('package.json', 'utf8')) as { version: string };
// A fixed clock, a pad on before start, Wi-Fi and a laptop battery: stable screenshots.
const BASE = 'time=12:24&h12=1&pad=connected&network=wifi&battery=72';

async function open(page: Page, query = ''): Promise<void> {
  await page.goto(`./?${BASE}${query ? `&${query}` : ''}`);
  await expect(page.getByTestId('screen')).toHaveAttribute('data-version', pkg.version);
  await expect(page.getByTestId('screen')).not.toHaveAttribute('data-library', 'loading');
  // Home takes input only once it is live (A9: the input lock until then; D-014 adds two frames).
  await expect(page.locator('main')).toHaveAttribute('data-phase', 'home');
  await page.evaluate(() => document.fonts.ready);
  await waitForIcons(page);
}

const logs = (page: Page) => page.evaluate(() => window.__aioneMock?.logs ?? []);
const focusIndex = async (page: Page) => Number(await page.getByTestId('game-row').getAttribute('data-focus'));
const pad = (page: Page, control: string, kind: 'down' | 'up' | 'repeat' = 'down') =>
  page.evaluate(([c, k]) => window.__aioneMock?.pad(c as string, k as 'down'), [control, kind]);

/** L-09: no gradient or image on any background (Home is flat). */
async function expectFlat(page: Page): Promise<void> {
  const bad = await page.evaluate(() =>
    [...document.querySelectorAll('*')]
      .filter((el) => !el.closest('.art'))
      .map((el) => getComputedStyle(el).backgroundImage)
      .filter((b) => b !== 'none'),
  );
  expect(bad).toEqual([]);
}

/** L-10: every header item has one optical size and weight. */
async function headerMetrics(page: Page) {
  return page.evaluate(() => {
    const icons = [...document.querySelectorAll<HTMLElement>('[data-testid="status-header"] .icon')].map((el) => {
      const r = el.getBoundingClientRect();
      const cs = getComputedStyle(el);
      return { h: r.height, size: cs.fontSize, vs: cs.fontVariationSettings };
    });
    const digits = document.querySelector<HTMLElement>('[data-testid="clock"] .digits');
    const ampm = document.querySelector<HTMLElement>('[data-testid="ampm"]');
    const cs = digits ? getComputedStyle(digits) : null;
    return {
      icons,
      clockWeight: cs?.fontWeight,
      clockSize: parseFloat(cs?.fontSize ?? '0'),
      ampmSize: ampm ? parseFloat(getComputedStyle(ampm).fontSize) : 0,
      letterSpacing: cs?.letterSpacing,
    };
  });
}

test.describe('T1.1 Home (mocked core)', () => {
  /** B-8: the title's left edge vs the focused tile's left edge, and whether it is shifted. */
  async function titleVsTile(page: Page) {
    await page.waitForTimeout(450); // the row and the title move together (260 ms)
    return page.evaluate(() => {
      const row = document.querySelector<HTMLElement>('[data-testid="selected-title-row"]');
      const i = Number(row?.dataset['index'] ?? -1);
      const tile = document.querySelector<HTMLElement>(`[data-testid="tile"][data-index="${i}"]`);
      const r = row?.getBoundingClientRect();
      return {
        index: i,
        titleLeft: r?.left ?? -1,
        titleCentre: r ? r.left + r.width / 2 : -1,
        tileCentre: (() => {
          const b = tile?.getBoundingClientRect();
          return b ? b.left + b.width / 2 : -1;
        })(),
        titleRight: r?.right ?? -1,
        tileLeft: tile?.getBoundingClientRect().left ?? -1,
        shifted: row?.dataset['shifted'] === 'true',
        vw: innerWidth,
      };
    });
  }

  for (const vp of [
    { w: 1280, h: 720 },
    { w: 1920, h: 1080 },
    { w: 2560, h: 1440 },
  ]) {
    for (const scale of [100, 150]) {
      for (const preset of ['light', 'dark'] as const) {
        test(`Home at ${vp.w}x${vp.h}, ${scale} %, ${preset}`, async ({ page }) => {
          await page.setViewportSize({ width: vp.w, height: vp.h });
          await open(page, `scale=${scale}&preset=${preset}`);
          const px = vp.h / 1080;
          await expect(page.getByTestId('tile')).toHaveCount(10);
          await expect(page.getByTestId('selected-title')).toHaveText('Amber Lighthouse');
          await expectNoOverflow(page);
          await expectInViewport(page, '[data-testid="status-header"], [data-testid="footer"], [data-testid="selected-title-row"]');
          await expectFlat(page);
          // Contrast (A8).
          expect(await contrastOf(page, '[data-testid="selected-title"]')).toBeGreaterThanOrEqual(4.5);
          expect(await contrastOf(page, '[data-testid="clock"]')).toBeGreaterThanOrEqual(4.5);
          expect(await contrastOf(page, '[data-testid="prompt-confirm"] .label')).toBeGreaterThanOrEqual(4.5);
          expect(await contrastOf(page, '[data-testid="system-name"]')).toBeGreaterThanOrEqual(4.5);
          expect(await contrastOf(page, '[data-testid="status-network"]')).toBeGreaterThanOrEqual(3);
          // L-10.
          const m = await headerMetrics(page);
          expect(new Set(m.icons.map((i) => i.vs)).size).toBe(1);
          expect(m.clockWeight).toBe('400');
          expect(m.ampmSize / m.clockSize).toBeCloseTo(0.6, 1);
          expect(await page.locator('.tile.focused').count()).toBe(1);
          // Sizes (QA-6 of alpha.1): footer with the scale, header at half rate, tiles fixed.
          const label = await page.getByTestId('prompt-confirm').locator('.label').evaluate((el) => parseFloat(getComputedStyle(el).fontSize));
          expect(label).toBeCloseTo(28 * px * (scale / 100), 0);
          expect(m.clockSize).toBeCloseTo(25 * px * (1 + (scale / 100 - 1) / 2), 0);
          // B-14: tiles 398 square, the first 155 from the left, their top at 309 (ring top 302).
          const firstTile = await page.getByTestId('tile').first().boundingBox();
          expect(firstTile?.width ?? 0).toBeCloseTo(398 * px, 0);
          expect(firstTile?.x ?? 0).toBeCloseTo(155 * px, 0);
          expect(firstTile?.y ?? 0).toBeCloseTo(309 * px, 0);

          // B-8 (rev. o): just like Switch, the title alone (no ▼, no icon), in the accent text colour.
          await expect(page.getByTestId('title-arrow')).toHaveCount(0);
          expect(await page.locator('[data-testid="selected-title-row"] .icon').count()).toBe(0);

          // B-5 (rev. o): the footer mark about 64 px at 1080 p / 100 % (twice the prompt text),
          // scaled with Interface Scale; two-tone: halves in the footer's text colour, the
          // diamond in the accent; each part ≥ 3:1 on the background.
          const mark = await page.getByTestId('footer-mark').evaluate((el) => {
            const r = el.getBoundingClientRect();
            const half = getComputedStyle(el.querySelector('.half') as Element).fill;
            const diamond = getComputedStyle(el.querySelector('.diamond') as Element).fill;
            const text = getComputedStyle(el.parentElement as Element).color;
            const accent = getComputedStyle(document.documentElement).getPropertyValue('--color-accent').trim();
            return { h: r.height, half, diamond, text, accent };
          });
          expect(mark.h).toBeCloseTo(64 * px * (scale / 100), 0);
          expect(mark.half).toBe(mark.text);
          const hex = (c: string) => c.replace(/^#?(\w)(\w)(\w)$/, '#$1$1$2$2$3$3').toLowerCase();
          const rgbOf = (h: string) => `rgb(${[1, 3, 5].map((i) => parseInt(hex(h).slice(i, i + 2), 16)).join(', ')})`;
          expect(mark.diamond).toBe(rgbOf(mark.accent));
          const bgRgb = preset === 'dark' ? [45, 45, 45] : [235, 235, 235];
          const num = (c: string) => (c.match(/\d+/g) ?? []).slice(0, 3).map(Number) as [number, number, number];
          expect(contrast(num(mark.half), bgRgb as [number, number, number])).toBeGreaterThanOrEqual(3);
          expect(contrast(num(mark.diamond), bgRgb as [number, number, number])).toBeGreaterThanOrEqual(3);

          // Focus on the first tile: the title's left edge = the tile's left edge.
          let t = await titleVsTile(page);
          expect(t.index).toBe(0);
          // B-8 (rev. p): centred on the tile, unless shifted to stay on screen.
          if (!t.shifted) expect(Math.abs(t.titleCentre - t.tileCentre)).toBeLessThan(1.5);
          else expect(t.titleLeft).toBeGreaterThanOrEqual(vp.w * 0.03 - 1);
          await shot(page, `t1-home-${vp.w}x${vp.h}-${scale}-${preset}-first`);

          // A middle tile, which is also the game without an icon file (B-13: its wide picture
          // fitted whole over a blurred copy).
          for (let i = 0; i < 3; i++) await page.keyboard.press('ArrowRight');
          await expect(page.getByTestId('selected-title')).toHaveText('Moss Kingdom');
          t = await titleVsTile(page);
          expect(t.index).toBe(3);
          expect(Math.abs(t.titleCentre - t.tileCentre)).toBeLessThan(1.5);
          expect(t.shifted).toBe(false);
          await shot(page, `t1-home-${vp.w}x${vp.h}-${scale}-${preset}-middle-no-icon`);

          // A long title: shifted left to fit, scrolling by transform (L-16), on screen.
          for (let i = 0; i < 4; i++) await page.keyboard.press('ArrowRight');
          await expect(page.getByTestId('selected-title')).toContainText('The Long Corridor');
          t = await titleVsTile(page);
          await expect(page.getByTestId('selected-title-text')).toHaveAttribute('data-moving', 'true');
          expect(t.titleRight).toBeLessThanOrEqual(t.vw * 0.97 + 1);
          expect(t.titleLeft).toBeGreaterThanOrEqual(vp.w * 0.03 - 1);
          await shot(page, `t1-home-${vp.w}x${vp.h}-${scale}-${preset}-long-title`);

          // The last tile: the title stays on screen (shifted left when it would run past the margin).
          for (let i = 0; i < 2; i++) await page.keyboard.press('ArrowRight');
          await expect(page.getByTestId('selected-title')).toHaveText('Zephyr Line Express');
          t = await titleVsTile(page);
          expect(t.index).toBe(9);
          expect(t.titleRight).toBeLessThanOrEqual(t.vw * 0.97 + 1);
          if (!t.shifted) expect(Math.abs(t.titleCentre - t.tileCentre)).toBeLessThan(1.5);
          else expect(t.titleCentre).toBeLessThan(t.tileCentre);
          await shot(page, `t1-home-${vp.w}x${vp.h}-${scale}-${preset}-last`);

          // T1.2: the drawer. Down: the focus in the drawer (one active context): no game title,
          // no tile ring; the focused button's ring and its label under it, in the accent.
          await page.keyboard.press('ArrowDown');
          for (let i = 0; i < 3; i++) await page.keyboard.press('ArrowRight');
          await expect(page.getByTestId('drawer')).toHaveAttribute('data-zone', 'drawer');
          await expect(page.getByTestId('selected-title-row')).toHaveCount(0);
          await expect(page.locator('.tile.focused')).toHaveCount(0);
          await expect(page.locator('.dbtn.focused')).toHaveAttribute('data-id', 'theme');
          const lab = page.getByTestId('drawer-label');
          await expect(lab).toHaveText('Theme Settings');
          expect(await contrastOf(page, '[data-testid="drawer-label"]')).toBeGreaterThanOrEqual(4.5);
          const geo = await page.evaluate(() => {
            const b = document.querySelector('.dbtn.focused')?.getBoundingClientRect();
            const l = document.querySelector('[data-testid="drawer-label"]')?.getBoundingClientRect();
            const f = document.querySelector('[data-testid="footer"]')?.getBoundingClientRect();
            const row = document.querySelector('[data-testid="tile"]')?.getBoundingClientRect();
            return { b, l, f, row, vw: innerWidth };
          });
          // Centred under the button, never clipped, above the footer, below the row.
          expect(Math.abs((geo.l?.left ?? 0) + (geo.l?.width ?? 0) / 2 - ((geo.b?.left ?? 0) + (geo.b?.width ?? 0) / 2))).toBeLessThan(2);
          expect(geo.l?.left ?? -1).toBeGreaterThanOrEqual(0);
          expect(geo.l?.right ?? 1e9).toBeLessThanOrEqual(geo.vw);
          expect(geo.l?.bottom ?? 1e9).toBeLessThanOrEqual(geo.f?.top ?? 0);
          expect(geo.b?.top ?? 0).toBeGreaterThan(geo.row?.bottom ?? 1e9);
          // B-14: 110 px at 100 %; the discs grow at half rate with Interface Scale (to fit).
          expect(geo.b?.width ?? 0).toBeCloseTo(110 * px * (1 + (scale / 100 - 1) / 2), 0);
          await shot(page, `t1-home-${vp.w}x${vp.h}-${scale}-${preset}-drawer`);
        });
      }
    }
  }

  test('status: no controller, offline, low battery charging; a later change arrives as an event', async ({ page }) => {
    await page.goto('./?time=09:05&h12=0&network=offline&battery=20c&status-later=1');
    await expect(page.getByTestId('status-controller')).toHaveAttribute('data-connected', 'false');
    await expect(page.getByTestId('clock')).toHaveText('09:05');
    await expect(page.getByTestId('ampm')).toHaveCount(0);
    expect(await contrastOf(page, '[data-testid="status-controller"]')).toBeGreaterThanOrEqual(3);
    await shot(page, 't1-home-status-offline-24h');
    await expect(page.getByTestId('status-battery')).toHaveText('battery_android_frame_1');
    // D-002: a pad connecting later turns the icon on.
    await page.evaluate(() => window.__aioneMock?.connect(0));
    await expect(page.getByTestId('status-controller')).toHaveAttribute('data-connected', 'true');
  });

  test('desktop PC: no battery icon; wired connection', async ({ page }) => {
    await page.goto('./?time=12:24&h12=1&network=wired&battery=none&pad=connected');
    await expect(page.getByTestId('status-battery')).toHaveCount(0);
    await expect(page.getByTestId('status-network')).toHaveText('lan');
  });

  test('focus moves with D-pad, stick and arrows; one press = one move; stops at the ends', async ({ page }) => {
    await open(page);
    expect(await focusIndex(page)).toBe(0);
    await pad(page, 'DPadRight');
    expect(await focusIndex(page)).toBe(1);
    await pad(page, 'LStickRight');
    expect(await focusIndex(page)).toBe(2);
    await page.keyboard.press('ArrowRight');
    expect(await focusIndex(page)).toBe(3);
    await pad(page, 'DPadLeft');
    await pad(page, 'LStickLeft');
    await page.keyboard.press('ArrowLeft');
    expect(await focusIndex(page)).toBe(0);
    // A press at the end moves nothing and makes no sound.
    await pad(page, 'DPadLeft');
    expect(await focusIndex(page)).toBe(0);
    // T1.2: Down enters the drawer and Up returns to the same tile (the row keeps its focus).
    await pad(page, 'DPadDown');
    await expect(page.getByTestId('drawer')).toHaveAttribute('data-zone', 'drawer');
    await pad(page, 'DPadUp');
    await expect(page.getByTestId('drawer')).toHaveAttribute('data-zone', 'row');
    expect(await focusIndex(page)).toBe(0);
    // Repeats move one tile each.
    await pad(page, 'DPadRight');
    for (let i = 0; i < 20; i++) await pad(page, 'DPadRight', 'repeat');
    // B-14: the row ends after its empty slot (index 10); the last game is index 9.
    expect(await focusIndex(page)).toBe(10);
    await pad(page, 'DPadLeft');
    await expect(page.getByTestId('selected-title')).toHaveText('Zephyr Line Express');
    await page.waitForTimeout(400);
    await shot(page, 't1-home-focus-last');
    const l = await logs(page);
    expect(l.some((x) => x.startsWith('info: input down DPadLeft (controller 1 seq 6) -> action left, end of row, bump left, sound edgeBump, core → UI '))).toBe(true);
    expect(l.filter((x) => x.includes('-> action right, focus 10/10')).length).toBe(1);
    expect(l.filter((x) => x.includes('-> action right, focus 11/10')).length).toBe(1); // the empty slot
    // No line says a move happened twice for one event: every acting line names one focus.
    const moves = l.filter((x) => /action (left|right), focus/.test(x));
    expect(moves.length).toBe(3 + 3 + 10 + 1);
  });

  test('the view keeps the focused tile in view; the row moves by transform only (L-16)', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await open(page);
    for (let i = 0; i < 5; i++) await page.keyboard.press('ArrowRight');
    await expect(page.getByTestId('game-row')).toHaveAttribute('data-first', '2');
    await page.waitForTimeout(400);
    const box = await page.locator('.tile.focused').boundingBox();
    expect((box?.x ?? 0) + (box?.width ?? 0)).toBeLessThanOrEqual(1920);
    const track = await page.locator('.track').evaluate((el) => getComputedStyle(el).transform);
    expect(track).toMatch(/^matrix\(1, 0, 0, 1, -820(\.\d+)?, 0\)$/);
    // Every running animation changes only transform or opacity (A7).
    const props = await page.evaluate(() =>
      document.getAnimations().flatMap((a) => {
        const kf = (a.effect as KeyframeEffect).getKeyframes();
        return kf.flatMap((k) => Object.keys(k).filter((p) => !['offset', 'easing', 'composite', 'computedOffset'].includes(p)));
      }),
    );
    expect(props.every((p) => p === 'transform' || p === 'opacity')).toBe(true);
  });

  test('a long title scrolls by transform (marquee); a short one stays still', async ({ page }) => {
    await open(page);
    await expect(page.getByTestId('selected-title-text')).toHaveAttribute('data-moving', 'false');
    for (let i = 0; i < 7; i++) await page.keyboard.press('ArrowRight');
    await expect(page.getByTestId('selected-title')).toContainText('The Long Corridor');
    await expect(page.getByTestId('selected-title-text')).toHaveAttribute('data-moving', 'true');
    const marquee = await page.evaluate(() =>
      document
        .getAnimations()
        .filter((a) => a.id === 'marquee')
        .map((a) => Object.keys((a.effect as KeyframeEffect).getKeyframes()[2] ?? {})),
    );
    expect(marquee).toHaveLength(1);
    expect(marquee[0]).toContain('transform');
    await page.waitForTimeout(2600);
    await shot(page, 't1-home-marquee-midway');
    // Moving on ends it (no orphaned animation).
    await page.keyboard.press('ArrowRight');
    expect(await page.evaluate(() => document.getAnimations().filter((a) => a.id === 'marquee').length)).toBe(0);
  });

  test('mouse (L-06): hover only, first click selects, second click starts, nothing left highlighted', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await open(page);
    const tile = (i: number) => page.locator(`[data-testid="tile"][data-index="${i}"]`);
    await tile(2).hover();
    await page.mouse.move(700, 450, { steps: 4 });
    await expect(page.getByTestId('screen')).toHaveAttribute('data-input', 'mouse');
    await expect(page.locator('.tile.focused')).toHaveCount(0);
    await expect(tile(1)).toHaveClass(/hovered/);
    await shot(page, 't1-home-mouse-hover');
    await tile(2).click();
    let l = await logs(page);
    expect(l.some((x) => x.startsWith('info: input click tile 3 "Lantern Valley" (mouse) -> select (first click)'))).toBe(true);
    expect(l.some((x) => x.startsWith('info: start requested'))).toBe(false);
    // Another tile: still only a selection.
    await tile(3).click();
    l = await logs(page);
    expect(l.filter((x) => x.startsWith('info: start requested'))).toHaveLength(0);
    // The selected tile again: start.
    await tile(3).click();
    l = await logs(page);
    expect(l).toContain('info: start requested: "Moss Kingdom" (second click); launching comes in T1.3');
    // After starting nothing stays selected; leaving the tile leaves nothing highlighted.
    await page.mouse.move(960, 900);
    await expect(page.locator('.tile.hovered')).toHaveCount(0);
    await expect(page.locator('.tile.focused')).toHaveCount(0);
    await shot(page, 't1-home-mouse-after-start');
    // B-10: picking up the pad only brings the ring back on the remembered tile.
    await pad(page, 'DPadRight');
    await expect(page.getByTestId('screen')).toHaveAttribute('data-input', 'pad');
    await expect(page.locator('.tile.focused')).toHaveAttribute('data-index', '3');
  });

  test('A / Enter start the focused game once; one sound per action; the row move on every repeat', async ({ page }) => {
    await open(page);
    await pad(page, 'A');
    await page.keyboard.press('Enter');
    let l = await logs(page);
    expect(l.filter((x) => x.startsWith('info: start requested: "Amber Lighthouse" (press)'))).toHaveLength(2);
    expect(l.filter((x) => x.includes('-> action confirm, start "Amber Lighthouse"'))).toHaveLength(2);
    expect(l.filter((x) => x.includes('-> action confirm, start "Amber Lighthouse"') && x.includes('sound confirm'))).toHaveLength(2);
    // B-1 (rev. p): holding a direction, the row move sounds on every move, every repeat included.
    await pad(page, 'DPadRight');
    for (let i = 0; i < 6; i++) {
      await page.waitForTimeout(90);
      await pad(page, 'DPadRight', 'repeat');
    }
    l = await logs(page);
    const moveLines = l.filter((x) => x.includes('DPadRight'));
    const played = moveLines.filter((x) => x.includes(', sound rowMove')).length;
    console.warn(`row move sounds: ${played} of ${moveLines.length} moves`);
    expect(moveLines).toHaveLength(7);
    expect(played).toBe(7);
    // Every acting line names at most one sound.
    expect(l.every((x) => (x.match(/sound (rowMove|drawerMove|rowDrawer|edgeBump|confirm|back|error)/g) ?? []).length <= 1)).toBe(true);
    expect(l.some((x) => x.startsWith('info: menu sounds ready: 7 sounds'))).toBe(true);
  });

  test('D-006: an arrow key repeats at 400 ms then every 90 ms; Enter and Alt never repeat; releases logged', async ({ page }) => {
    await open(page);
    const t0 = await page.evaluate(() => performance.now());
    await page.keyboard.down('ArrowRight');
    for (let i = 0; i < 40; i++) {
      await page.waitForTimeout(33);
      await page.keyboard.down('ArrowRight');
    }
    const held = await page.evaluate(() => performance.now());
    await page.waitForTimeout(Math.max(0, 1600 - (held - t0)));
    await page.keyboard.up('ArrowRight');
    const { lines, times } = await page.evaluate(() => ({
      lines: window.__aioneMock?.logs ?? [],
      times: window.__aioneMock?.logTimes ?? [],
    }));
    const at = (needle: string) => lines.flatMap((l, i) => (l.includes(needle) ? [times[i] ?? 0] : []));
    const down = at('input down Right (keyboard)');
    const reps = at('input repeat Right (keyboard)');
    expect(down).toHaveLength(1);
    const first = (reps[0] ?? 0) - (down[0] ?? 0);
    const gaps = reps.slice(1).map((t, i) => t - (reps[i] ?? 0));
    const mean = gaps.reduce((a, b) => a + b, 0) / Math.max(1, gaps.length);
    console.warn(`D-006 arrow: first repeat ${first.toFixed(0)} ms, ${reps.length} repeats, mean gap ${mean.toFixed(1)} ms`);
    expect(first).toBeGreaterThan(380);
    expect(first).toBeLessThan(480);
    expect(mean).toBeGreaterThan(84);
    expect(mean).toBeLessThan(110);
    expect(lines.some((l) => /^info: input up Right \(keyboard\), held \d+ ms, \d+ repeats$/.test(l))).toBe(true);

    for (const key of ['Enter', 'Alt', 'Escape']) {
      await page.keyboard.down(key);
      for (let i = 0; i < 25; i++) {
        await page.waitForTimeout(33);
        await page.keyboard.down(key);
      }
      await page.keyboard.up(key);
    }
    const after = await logs(page);
    for (const name of ['Enter', 'Alt', 'Esc']) {
      expect(after.filter((l) => l.startsWith(`info: input down ${name} (keyboard)`))).toHaveLength(1);
      expect(after.filter((l) => l.startsWith(`info: input repeat ${name}`))).toHaveLength(0);
      expect(after.filter((l) => new RegExp(`^info: input up ${name} \\(keyboard\\), held \\d+ ms, 0 repeats$`).test(l))).toHaveLength(1);
    }
  });

  test('blocked browser keys do not reload the page; restart shortcut reaches the core', async ({ page }) => {
    await open(page);
    await page.keyboard.press('ArrowRight');
    await page.keyboard.press('F5');
    await page.keyboard.press('Control+r');
    expect(await focusIndex(page)).toBe(1); // a reload would reset the focus
    await page.keyboard.press('Control+Shift+R');
    await expect.poll(() => page.evaluate(() => window.__aioneMock?.restarts)).toBe(1);
  });

  test('under the copy dialog Home is suspended: no ring, title or hover; input goes to the dialog', async ({ page }) => {
    await open(page, 'migration=offer');
    await expect(page.getByTestId('copy-dialog')).toHaveAttribute('data-phase', 'ask');
    await expect(page.getByTestId('home')).toHaveAttribute('data-suspended', 'true');
    await expect(page.locator('.tile.focused')).toHaveCount(0);
    await expect(page.getByTestId('selected-title')).toHaveCount(0);
    await pad(page, 'DPadRight');
    expect(await focusIndex(page)).toBe(0);
    const l = await logs(page);
    expect(l.some((x) => x.includes('input down DPadRight') && x.includes('dialog focus moved, sound rowMove'))).toBe(true);
    await shot(page, 't1-home-under-copy-dialog');
    await pad(page, 'B');
    await expect(page.getByTestId('copy-dialog')).toHaveCount(0);
    await expect(page.locator('.tile.focused')).toHaveCount(1);
    expect((await logs(page)).some((x) => x.includes('dialog not now, sound back'))).toBe(true);
  });

  test('empty library and error states', async ({ page }) => {
    await open(page, 'library=empty');
    await expect(page.getByTestId('home-empty')).toContainText('No games yet');
    await expect(page.getByTestId('prompt-confirm')).toHaveCount(0);
    expect(await contrastOf(page, '[data-testid="home-empty"] .headline')).toBeGreaterThanOrEqual(4.5);
    await shot(page, 't1-home-empty');
    await open(page, 'library=error');
    await expect(page.getByTestId('home-error')).toContainText('could not be loaded');
    await shot(page, 't1-home-error');
  });

  test('a plain tile shows the name when a game has no art', async ({ page }) => {
    await open(page, 'noart=1');
    await expect(page.getByTestId('tile-plain')).toHaveCount(10);
    await shot(page, 't1-home-no-art');
  });

  test('L-17: Home shows the games the library loaded (10)', async ({ page }) => {
    await open(page);
    await expect(page.getByTestId('tile')).toHaveCount(10);
    expect(await logs(page)).toContain('info: home shows 10 games (0 hidden)');
  });

  test('many games: the row still keeps the focus in view', async ({ page }) => {
    await open(page, 'library=many');
    for (let i = 0; i < 23; i++) await pad(page, 'DPadRight');
    expect(await focusIndex(page)).toBe(23);
    await page.waitForTimeout(400);
    const box = await page.locator('.tile.focused').boundingBox();
    const vw = page.viewportSize()?.width ?? 1920;
    expect((box?.x ?? 0) + (box?.width ?? 0)).toBeLessThanOrEqual(vw);
    await expectNoOverflow(page);
  });

  test('B-10: after the mouse, the first pad or key press only brings the ring back', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await open(page);
    const tile = (i: number) => page.locator(`[data-testid="tile"][data-index="${i}"]`);
    await tile(2).hover();
    await page.mouse.move(700, 450, { steps: 4 });
    await tile(2).hover();
    await expect(page.getByTestId('screen')).toHaveAttribute('data-input', 'mouse');
    await expect(page.locator('.tile.focused')).toHaveCount(0);
    // The pad: no move, no sound; the ring on the hovered tile.
    await pad(page, 'DPadRight');
    await expect(page.locator('.tile.focused')).toHaveAttribute('data-index', '2');
    let l = await logs(page);
    expect(l).toContain('info: ring back on tile 3 (after mouse)');
    const first = l.find((x) => x.includes('input down DPadRight'));
    expect(first).toContain('-> ring back on tile 3 (after mouse), no move, no sound');
    expect(l.filter((x) => x.includes('sound rowMove'))).toHaveLength(0);
    // The next press moves, with its sound.
    await pad(page, 'DPadRight');
    await expect(page.locator('.tile.focused')).toHaveAttribute('data-index', '3');
    l = await logs(page);
    expect(l.filter((x) => x.includes('action right, focus 4/10') && x.includes('sound rowMove'))).toHaveLength(1);
    // The same for the keyboard after a click.
    await tile(1).click();
    await page.mouse.move(800, 500, { steps: 4 });
    await tile(1).hover();
    await page.keyboard.press('ArrowLeft');
    await expect(page.locator('.tile.focused')).toHaveAttribute('data-index', '1');
    expect(await logs(page)).toContain('info: ring back on tile 2 (after mouse)');
    await shot(page, 't1-home-ring-back-after-mouse');
  });

  test('D-007: with 3000 px pictures, pictures are decoded ahead and quick navigation stays smooth', async ({ page }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto(`./?${BASE}&bigart=1`);
    await expect(page.getByTestId('screen')).toHaveAttribute('data-library', 'loaded', { timeout: 30_000 });
    await expect(page.getByTestId('tile')).toHaveCount(10);
    // Home doesn't wait more than 1.5 s for decoding (D-010); with 3000 px pictures the rest
    // finishes just after. Navigation starts once every tile picture is decoded.
    const ready = (await logs(page)).find((x) => /tile pictures( ready|: decoding ahead took over)/.test(x));
    expect(ready).toBeTruthy();
    await page.waitForFunction(
      () => [...document.querySelectorAll<HTMLImageElement>('[data-testid="tile"] img')].every((i) => i.complete && i.naturalWidth > 0),
      undefined,
      { timeout: 20_000 },
    );
    // Which picture (if any) is still not drawable, for the record.
    const pending = await page.evaluate(() =>
      Promise.all(
        [...document.querySelectorAll<HTMLImageElement>('[data-testid="tile"] img')].map((i) =>
          i.decode().then(
            () => null,
            (e: unknown) => `${i.closest<HTMLElement>('[data-testid="tile"]')?.dataset['index']}: ${String(e)} (${i.src.slice(0, 30)}…, ${i.naturalWidth}×${i.naturalHeight})`,
          ),
        ),
      ),
    );
    console.warn(`D-007 test: pictures not decodable by decode(): ${JSON.stringify(pending.filter(Boolean))}`);
    await page.waitForTimeout(1000);
    expect(await page.locator('[data-testid="tile"] img').first().evaluate((i) => (i as HTMLImageElement).naturalWidth)).toBe(3000);
    const marker = (await logs(page)).length;
    // Quick pad use over all 10 tiles and back, a press every 70 ms.
    for (let round = 0; round < 2; round++) {
      for (const dir of ['DPadRight', 'DPadLeft']) {
        for (let i = 0; i < 9; i++) {
          await pad(page, dir);
          await page.waitForTimeout(70);
        }
      }
    }
    await page.waitForTimeout(500);
    const l = (await logs(page)).slice(marker);
    const presses = l.filter((x) => /input down DPad(Right|Left)/.test(x));
    expect(presses).toHaveLength(36);
    const delays = presses.map((x) => Number(/core → UI (\d+) ms/.exec(x)?.[1] ?? '999'));
    console.warn(`D-007 big pictures: core → UI max ${Math.max(...delays)} ms over ${delays.length} presses`);
    expect(Math.max(...delays)).toBeLessThanOrEqual(50);
    const long = l.filter((x) => x.startsWith('info: long task'));
    console.warn(`D-007 long tasks during navigation: ${long.length}${long.length ? ` — ${long[0]}` : ''}`);
    expect(long).toHaveLength(0);
    await shot(page, 't1-home-big-pictures');
  });

  test('B-9: every accent pair meets A8, light and dark', async ({ page }) => {
    for (const preset of ['light', 'dark']) {
      await open(page, `preset=${preset}`);
      const ratios = await page.evaluate(() => {
        const v = (n: string) => getComputedStyle(document.documentElement).getPropertyValue(n).trim();
        // The build may shorten colours (#ffffff → #fff).
        const full = (h: string) => (h.length === 4 ? `#${[...h.slice(1)].map((c) => c + c).join('')}` : h);
        const hex = (h: string) => [1, 3, 5].map((i) => parseInt(full(h).slice(i, i + 2), 16));
        const lum = (h: string) => {
          const [r, g, b] = hex(h).map((c) => {
            const s = c / 255;
            return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
          });
          return 0.2126 * (r ?? 0) + 0.7152 * (g ?? 0) + 0.0722 * (b ?? 0);
        };
        const cr = (a: string, b: string) => {
          const [x, y] = [lum(v(a)), lum(v(b))];
          return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
        };
        return {
          ring: cr('--color-ring', '--color-bg'),
          ringGap: cr('--color-ring', '--color-ring-gap'),
          title: cr('--color-title', '--color-bg'),
          hover: cr('--color-hover', '--color-bg'),
          soft: cr('--color-on-soft', '--color-soft'),
          pressed: cr('--color-on-pressed', '--color-pressed'),
          accentOnSurface: cr('--color-accent', '--color-surface-raised'),
        };
      });
      expect(Object.values(ratios).every((x) => Number.isFinite(x))).toBe(true);
      console.warn(`B-9 ${preset}: ${JSON.stringify(Object.fromEntries(Object.entries(ratios).map(([k, x]) => [k, Number(x.toFixed(2))])))}`);
      // Focus ring, hover outline, accent shapes: ≥ 3:1. Text: ≥ 4.5:1.
      expect(ratios.ring).toBeGreaterThanOrEqual(3);
      expect(ratios.ringGap).toBeGreaterThanOrEqual(3);
      expect(ratios.hover).toBeGreaterThanOrEqual(3);
      expect(ratios.accentOnSurface).toBeGreaterThanOrEqual(3);
      expect(ratios.title).toBeGreaterThanOrEqual(4.5);
      expect(ratios.soft).toBeGreaterThanOrEqual(4.5);
      expect(ratios.pressed).toBeGreaterThanOrEqual(4.5);
    }
    const ring = await page.locator('.tile.focused .ring-line:not(.ring-alt)').evaluate((el) => getComputedStyle(el).borderTopColor);
    expect(ring).toBe('rgb(255, 70, 162)');
  });

  test('B-9 (rev. m) / D-011: the ring pulses between two shades, no glow; ≥ 3:1 at both ends', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    // B-9 (rev. p): the pulse starts at the bright end: light #FD007E then #D9006C; dark #FF69B4 then #FF46A2.
    const expected = { light: [[253, 0, 126], [217, 0, 108]], dark: [[255, 105, 180], [255, 70, 162]] } as const;
    for (const preset of ['light', 'dark'] as const) {
      await open(page, `preset=${preset}`);
      const anims = await page.evaluate(() =>
        document.getAnimations().map((a) => ({
          name: (a as CSSAnimation).animationName,
          duration: Number((a.effect as KeyframeEffect).getTiming().duration),
          dir: (a.effect as KeyframeEffect).getTiming().direction,
        })),
      );
      const pulse = anims.find((a) => a.name.includes('ring-pulse'));
      expect(pulse).toBeTruthy();
      expect(anims.some((a) => a.name.includes('glow'))).toBe(false);
      // 0.7 s each way, alternating: 1.4 s per cycle, eased.
      // B-9 (rev. p): one cycle every 1.0 s.
      expect((pulse?.duration ?? 0) * 2).toBe(1000);
      expect(pulse?.dir).toBe('alternate');
      // CSS applies the easing per keyframe: read it from the ring itself.
      expect(await page.evaluate(() => getComputedStyle(document.querySelector('.tile.focused .ring-alt') as Element).animationTimingFunction)).toBe('ease-in-out');
      expect(await page.evaluate(() => getComputedStyle(document.querySelector('.tile.focused .ring') as Element).boxShadow)).toBe('none');
      for (const [i, end] of (['start', 'end'] as const).entries()) {
        await page.evaluate((e) => {
          for (const a of document.getAnimations()) {
            if (!(a as CSSAnimation).animationName.includes('ring-pulse')) continue;
            const t = (a.effect as KeyframeEffect).getTiming();
            a.pause();
            a.currentTime = Number(t.delay ?? 0) + (e === 'start' ? 0 : Number(t.duration) - 0.01);
          }
        }, end);
        await page.waitForTimeout(100);
        const ring = await page.locator('.tile.focused .ring-line').first().boundingBox();
        const img = decodePng(await page.screenshot());
        const strokeY = (ring?.y ?? 0) + 3;
        const bg = pixel(img, 1900, 160); // plain background (the larger tiles now reach 1900, 700)
        const c = pixel(img, (ring?.x ?? 0) + (ring?.width ?? 0) / 2, strokeY);
        console.warn(`ring ${preset} ${end}: ${JSON.stringify(c)} vs bg ${JSON.stringify(bg)}: ${contrast(c, bg).toFixed(2)}`);
        expect(contrast(c, bg)).toBeGreaterThanOrEqual(3);
        const want = expected[preset][i] ?? [0, 0, 0];
        for (let k = 0; k < 3; k++) expect(Math.abs((c[k] ?? 0) - (want[k] ?? 0))).toBeLessThanOrEqual(3);
      }
      await shot(page, `t1-home-ring-pulse-${preset}`);
    }
  });

  test('B-8 (rev. o): no ▼; a long title scrolls within its own space, clipped at both ends', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await open(page);
    for (let i = 0; i < 7; i++) await page.keyboard.press('ArrowRight');
    await expect(page.getByTestId('selected-title-text')).toHaveAttribute('data-moving', 'true');
    await page.waitForTimeout(600); // the title's own move with the focus is over
    const box = () =>
      page.evaluate(() => {
        const row = document.querySelector('[data-testid="selected-title-row"]')?.getBoundingClientRect();
        const text = document.querySelector('[data-testid="selected-title"]')?.getBoundingClientRect();
        return { rowLeft: row?.left ?? 0, textLeft: text?.left ?? 0, textRight: text?.right ?? 0, vw: innerWidth };
      });
    const a = await box();
    await page.waitForTimeout(2500);
    const b = await box();
    // The clipping box stays put (only the text inside moves), starts at the title's own start
    // (no icon before it) and ends inside the right margin.
    expect(b.textLeft).toBeCloseTo(a.textLeft, 1);
    expect(Math.abs(b.textLeft - b.rowLeft)).toBeLessThan(1);
    expect(b.textRight).toBeLessThanOrEqual(b.vw * 0.97 + 1);
    expect(await page.getByTestId('selected-title').evaluate((el) => getComputedStyle(el).overflow)).toBe('hidden');
    await shot(page, 't1-home-marquee-no-arrow');
  });

  test('the drawer: one press = one move; sounds per action; Up back to the tile; the toast; mouse', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await open(page);
    await page.keyboard.press('ArrowRight');
    await page.keyboard.press('ArrowRight');
    const marker = (await logs(page)).length;
    await pad(page, 'DPadDown');
    await expect(page.locator('.dbtn.focused')).toHaveAttribute('data-index', '0');
    await pad(page, 'DPadRight');
    await expect(page.locator('.dbtn.focused')).toHaveAttribute('data-index', '1');
    await page.keyboard.press('ArrowRight');
    await expect(page.locator('.dbtn.focused')).toHaveAttribute('data-index', '2');
    await pad(page, 'DPadLeft');
    await pad(page, 'DPadLeft');
    await pad(page, 'DPadLeft'); // stops at the first button
    await expect(page.locator('.dbtn.focused')).toHaveAttribute('data-index', '0');
    await pad(page, 'DPadDown'); // nothing below
    await pad(page, 'A');
    await expect(page.getByTestId('toast')).toHaveText('Coming in a later stage');
    // The toast never covers the drawer's label, and stays inside the footer band.
    const ov = await page.evaluate(() => {
      const t = document.querySelector('[data-testid="toast"]')?.getBoundingClientRect();
      const l = document.querySelector('[data-testid="drawer-label"]')?.getBoundingClientRect();
      const f = document.querySelector('[data-testid="footer"]')?.getBoundingClientRect();
      return { tTop: t?.top ?? 0, tBottom: t?.bottom ?? 0, lBottom: l?.bottom ?? 1e9, fTop: f?.top ?? 0, fBottom: f?.bottom ?? 0 };
    });
    expect(ov.tTop).toBeGreaterThanOrEqual(ov.lBottom);
    expect(ov.tTop).toBeGreaterThanOrEqual(ov.fTop);
    expect(ov.tBottom).toBeLessThanOrEqual(ov.fBottom);
    await shot(page, 't12-drawer-toast');
    await pad(page, 'DPadUp');
    await expect(page.locator('.tile.focused')).toHaveAttribute('data-index', '2'); // the last focused tile
    let l = (await logs(page)).slice(marker);
    const lines = l.filter((x) => x.startsWith('info: input down'));
    expect(lines).toHaveLength(9); // one line per press
    // B-1 (rev. q): sounds by place: the change of place, the drawer's own tick, the edge bump.
    expect(lines.filter((x) => x.includes('sound rowDrawer'))).toHaveLength(2); // down, up
    expect(lines.filter((x) => x.includes('sound drawerMove'))).toHaveLength(4); // right, right, left, left
    expect(lines.some((x) => x.includes('end of drawer, bump left, sound edgeBump'))).toBe(true);
    expect(lines.some((x) => x.includes('nothing below the drawer, bump down, sound edgeBump'))).toBe(true);
    expect(lines.some((x) => x.includes('drawer Search: toast, sound error'))).toBe(true);
    expect(l.some((x) => x.includes('drawer: Search (press): its screen comes in T4'))).toBe(true);
    // Mouse: hover shows only an outline and the label; a click opens at once; nothing stays.
    const btn = (i: number) => page.locator(`[data-testid="drawer-button"][data-index="${i}"]`);
    await btn(4).hover();
    await page.mouse.move(900, 900, { steps: 4 });
    await btn(4).hover();
    await expect(page.getByTestId('screen')).toHaveAttribute('data-input', 'mouse');
    await expect(page.locator('.dbtn.focused')).toHaveCount(0);
    await expect(btn(4)).toHaveClass(/hovered/);
    await shot(page, 't12-drawer-mouse-hover');
    await btn(4).click();
    await expect(page.getByTestId('toast')).toBeVisible();
    l = await logs(page);
    expect(l.some((x) => x.includes('input click (mouse) -> drawer Settings: toast, sound error'))).toBe(true);
    // Away along the bottom (the path crosses no tile or button: hovering a tile would rightly
    // make the row the remembered place, A7).
    const b4 = (await btn(4).boundingBox()) ?? { x: 0, y: 0, width: 0, height: 0 };
    await page.mouse.move(b4.x + b4.width / 2, 1040, { steps: 4 }); // straight down
    await page.mouse.move(1750, 1040, { steps: 4 }); // then along the footer
    await expect(page.locator('.dbtn.hovered, .dbtn.focused')).toHaveCount(0); // nothing left highlighted
    // B-10: the first pad press after the mouse brings the ring back on the drawer button.
    await pad(page, 'DPadRight');
    await expect(page.locator('.dbtn.focused')).toHaveAttribute('data-index', '4');
    expect((await logs(page)).some((x) => x.includes('ring back on drawer button "Settings" (after mouse)'))).toBe(true);
  });

  test('D-010: Home appears at once with the original pictures; the copies are swapped in later', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    const started = Date.now();
    await page.goto(`./?${BASE}&tiles=pending`);
    // Black, never grey or white, before Home.
    const htmlBg = await page.evaluate(() => getComputedStyle(document.documentElement).backgroundColor);
    expect(htmlBg).toBe('rgb(0, 0, 0)');
    await expect(page.getByTestId('tile')).toHaveCount(10);
    const ms = Date.now() - started;
    console.warn(`D-010: Home with originals ${ms} ms after navigation`);
    expect(ms).toBeLessThan(2000);
    const sources = await page.locator('[data-testid="tile"] img').evaluateAll((els) => els.map((e) => e.getAttribute('data-source')));
    expect(sources.every((x) => x === 'original')).toBe(true);
    // B-13: before its copy, the wide picture is shown whole (never cropped).
    await expect(page.locator('[data-testid="tile"][data-index="3"] img')).toHaveClass(/fit/);
    await shot(page, 't1-home-originals-before-copies');
    await page.evaluate(() => window.__aioneMock?.releaseTiles());
    await expect.poll(async () => page.locator('[data-testid="tile"] img[data-source="copy"]').count()).toBe(9);
    await shot(page, 't1-home-copies-swapped-in');
  });

  test('B-10 (rev. k): modifiers alone keep mouse mode; Enter brings the ring back', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await open(page);
    const tile = (i: number) => page.locator(`[data-testid="tile"][data-index="${i}"]`);
    await tile(1).hover();
    await page.mouse.move(700, 450, { steps: 4 });
    await tile(1).hover();
    await expect(page.getByTestId('screen')).toHaveAttribute('data-input', 'mouse');
    for (const k of ['Alt', 'Control', 'Shift']) await page.keyboard.press(k);
    await expect(page.getByTestId('screen')).toHaveAttribute('data-input', 'mouse');
    await expect(page.locator('.tile.focused')).toHaveCount(0);
    let l = await logs(page);
    expect(l.filter((x) => x.includes('ring back'))).toHaveLength(0);
    expect(l.some((x) => x.startsWith('info: input down Alt (keyboard) -> no action (mouse mode kept)'))).toBe(true);
    await page.keyboard.press('Enter');
    await expect(page.locator('.tile.focused')).toHaveAttribute('data-index', '1');
    l = await logs(page);
    expect(l).toContain('info: ring back on tile 2 (after mouse)');
    expect(l.filter((x) => x.startsWith('info: start requested'))).toHaveLength(0);
    // QA-6: the pad's ring-back line carries the slot number.
    await tile(4).hover();
    await page.mouse.move(900, 500, { steps: 4 });
    await tile(4).hover();
    await pad(page, 'DPadRight');
    expect((await logs(page)).some((x) => /input down DPadRight \(controller 1 seq \d+\) -> ring back on tile 5/.test(x))).toBe(true);
  });

  test('alpha.7 CI: without the icon font, icons stay hidden (never their names as text)', async ({ page }) => {
    await page.route('**/material-symbols*.woff2', (route) => route.abort());
    await page.goto(`./?${BASE}`);
    await expect(page.getByTestId('tile')).toHaveCount(10);
    await expect(page.locator('main')).toHaveAttribute('data-icons-ready', 'false');
    const visible = await page.evaluate(() =>
      [...document.querySelectorAll<HTMLElement>('.icon')].filter((el) => getComputedStyle(el).visibility !== 'hidden').map((el) => el.textContent),
    );
    expect(visible).toEqual([]);
    expect((await logs(page)).some((x) => x.includes('icons stay hidden rather than shown as text'))).toBe(true);
    // The title text itself is still shown.
    await expect(page.getByTestId('selected-title')).toHaveText('Amber Lighthouse');
  });

  test('B-14 (rev. q) edge bumps: only the ring springs and settles; one bump per hold; the sound', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await open(page);
    // Watch one bump: the ring's offset at its furthest point, the tile before / during, and
    // whether everything has settled 400 ms after the press.
    const watch = async (ringSel: string, holderSel: string, press: () => Promise<unknown>, shotName: string) => {
      const before = await page.locator(holderSel).boundingBox();
      await press();
      const mid = await page.evaluate((sel) => {
        const ring = document.querySelector(sel) as HTMLElement | null;
        const a = ring?.getAnimations().find((x) => (x.effect as KeyframeEffect).getKeyframes().some((k) => String(k['transform'] ?? '').includes('translate')));
        if (!ring || !a) return null;
        a.pause();
        a.currentTime = Number((a.effect as KeyframeEffect).getTiming().duration) * 0.3; // the furthest point
        const r = ring.getBoundingClientRect();
        return { x: r.x, y: r.y, dur: Number((a.effect as KeyframeEffect).getTiming().duration) };
      }, ringSel);
      const during = await page.locator(holderSel).boundingBox();
      await shot(page, shotName);
      await page.evaluate((sel) => document.querySelector(sel)?.getAnimations().forEach((a) => a.play()), ringSel);
      await page.waitForTimeout(400);
      const settled = await page.evaluate((sel) => {
        const ring = document.querySelector(sel) as HTMLElement | null;
        return (ring?.getAnimations() ?? []).filter((x) => (x.effect as KeyframeEffect).getKeyframes().some((k) => String(k['transform'] ?? '').includes('translate'))).length;
      }, ringSel);
      const rest = await page.evaluate((sel) => (document.querySelector(sel) as HTMLElement).getBoundingClientRect(), ringSel);
      return { before, during, mid, settled, rest };
    };
    // Row start: Left on the first tile.
    const r = await watch('.tile.focused .ring', '.tile.focused', () => pad(page, 'DPadLeft'), 't14-bump-row-left-mid');
    expect(r.mid).not.toBeNull();
    expect((r.mid?.dur ?? 999)).toBeLessThanOrEqual(400);
    expect(r.during).toEqual(r.before); // the tile stays still
    expect((r.rest.x ?? 0) - (r.mid?.x ?? 0)).toBeGreaterThan(10); // the ring went left (about 15 px)
    expect(r.settled).toBe(0); // settled within 400 ms
    // Held at the edge: the repeats don't bump again.
    for (let i = 0; i < 4; i++) await pad(page, 'DPadLeft', 'repeat');
    let l = await logs(page);
    expect(l.filter((x) => x.includes('end of row, bump left, sound edgeBump'))).toHaveLength(1);
    expect(l.filter((x) => x.includes('end of row, held: no bump'))).toHaveLength(4);
    // Regression (found in this build): a normal move after a bump must not replay the bump.
    await pad(page, 'DPadRight');
    const replays = await page.evaluate(() =>
      [...document.querySelectorAll('.ring, .dring')].flatMap((e) => e.getAnimations()).filter((a) => (a.effect as KeyframeEffect).getKeyframes().some((k) => String(k['transform'] ?? '').includes('translate'))).length,
    );
    expect(replays).toBe(0);
    await pad(page, 'DPadLeft');
    // Up from the row: nothing above.
    const u = await watch('.tile.focused .ring', '.tile.focused', () => pad(page, 'DPadUp'), 't14-bump-row-up-mid');
    expect((u.rest.y ?? 0) - (u.mid?.y ?? 0)).toBeGreaterThan(8); // up about 12 px
    expect(u.during).toEqual(u.before);
    // The drawer's right end, then down where nothing is below.
    await pad(page, 'DPadDown');
    for (let i = 0; i < 5; i++) await pad(page, 'DPadRight');
    const d = await watch('.dbtn.focused .dring', '.dbtn.focused', () => pad(page, 'DPadRight'), 't14-bump-drawer-right-mid');
    expect((d.mid?.x ?? 0) - (d.rest.x ?? 0)).toBeGreaterThan(10);
    expect(d.during).toEqual(d.before); // the button stays still
    expect(d.settled).toBe(0);
    const dn = await watch('.dbtn.focused .dring', '.dbtn.focused', () => pad(page, 'DPadDown'), 't14-bump-drawer-down-mid');
    expect((dn.mid?.y ?? 0) - (dn.rest.y ?? 0)).toBeGreaterThan(8);
    l = await logs(page);
    expect(l.filter((x) => x.includes('bump') && x.includes('sound edgeBump'))).toHaveLength(4);
  });

  test('B-14: empty slots after the last game; no title there; A does nothing; "A OK" dimmed', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await open(page);
    expect(await page.getByTestId('empty-slot').count()).toBe(2);
    for (let i = 0; i < 12; i++) await page.keyboard.press('ArrowRight');
    await expect(page.locator('.tile.focused')).toHaveAttribute('data-testid', 'empty-slot');
    await expect(page.getByTestId('selected-title-row')).toHaveCount(0);
    await expect(page.getByTestId('prompt-confirm')).toHaveAttribute('data-dimmed', 'true');
    await expect(page.getByTestId('prompt-options')).toHaveCount(0);
    await page.keyboard.press('Enter');
    expect((await logs(page)).some((x) => x.includes('empty slot: A does nothing'))).toBe(true);
    expect(await contrastOf(page, '[data-testid="prompt-confirm"] .label')).toBeGreaterThanOrEqual(4.5);
    await shot(page, 't14-empty-slot-focused');
  });
});
