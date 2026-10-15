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
    // ... and on a slow machine, until the title's own move has really finished (its scroll loop,
    // "marquee", runs forever and doesn't count). alpha.11: measured mid-move at 2560 px, 150 %.
    await page.waitForFunction(() =>
      [...(document.querySelector('[data-testid="selected-title-row"]')?.getAnimations({ subtree: true }) ?? [])].every(
        (a) => a.id === 'marquee' || a.playState !== 'running' || (a as CSSAnimation).animationName === 'title-in',
      ),
    );
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
          // B-8 (rev. v): the title in the ring's colour, #FF46A2 (the user's choice, like Switch;
          // 2.65:1 on the light background, 4.36:1 on dark: logged, not asserted).
          expect(await page.getByTestId('selected-title').evaluate((el) => getComputedStyle(el).color)).toBe('rgb(255, 70, 162)');
          console.warn(`title contrast ${vp.w}x${vp.h} ${scale} ${preset}: ${(await contrastOf(page, '[data-testid="selected-title"]')).toFixed(2)}`);
          expect(await contrastOf(page, '[data-testid="clock"]')).toBeGreaterThanOrEqual(4.5);
          expect(await contrastOf(page, '[data-testid="prompt-confirm"] .label')).toBeGreaterThanOrEqual(4.5);
          expect(await contrastOf(page, '[data-testid="system-name"]')).toBeGreaterThanOrEqual(4.5);
          expect(await contrastOf(page, '[data-testid="status-network"]')).toBeGreaterThanOrEqual(3);
          // L-10.
          const m = await headerMetrics(page);
          expect(new Set(m.icons.map((i) => i.vs)).size).toBe(1);
          expect(m.clockWeight).toBe('400');
          expect(m.ampmSize / m.clockSize).toBeCloseTo(0.65, 1); // rev. w: AM about 20 px of ink
          expect(await page.locator('.tile.focused').count()).toBe(1);
          // Sizes (QA-6 of alpha.1): footer with the scale, header at half rate, tiles fixed.
          const label = await page.getByTestId('prompt-confirm').locator('.label').evaluate((el) => parseFloat(getComputedStyle(el).fontSize));
          // Rev. v (side by side with Switch): footer text 2.23 rem (about 1.35x), the clock 47 px
          // (digits about 34 px tall) at half rate.
          // rev. w (visible ink): footer text 2.4 rem (capitals about 28 px); clock 42.5 px (digits
          // about 31 px of ink), at half rate.
          expect(label).toBeCloseTo(2.23 * 16 * px * (scale / 100), 0); // rev. y: capitals about 27 px
          expect(m.clockSize).toBeCloseTo(42.5 * px * (1 + (scale / 100 - 1) / 2), 0);
          // B-14 (rev. v): tiles 398 square, the first 155 from the left, their top at 291 (the
          // ring's top at about 278, 13 px outside the art).
          const firstTile = await page.getByTestId('tile').first().boundingBox();
          expect(firstTile?.width ?? 0).toBeCloseTo(386 * px, 0); // rev. x
          expect(firstTile?.x ?? 0).toBeCloseTo(155 * px, 0);
          expect(firstTile?.y ?? 0).toBeCloseTo(291 * px, 0);

          // B-8 (rev. o): just like Switch, the title alone (no ▼, no icon), in the accent text colour.
          await expect(page.getByTestId('title-arrow')).toHaveCount(0);
          expect(await page.locator('[data-testid="selected-title-row"] .icon').count()).toBe(0);

          // D-023 (rev. y): the footer glyph's VISIBLE circle about 37 px (ink, not its box),
          // measured at 50 % coverage (halfway between the background and the glyph's darkest
          // tone), so the edge smoothing of each platform doesn't change the result (CI run #21:
          // a fixed "dark < 110" threshold read 3 px off on Windows).
          {
            const gb = await page.locator('[data-testid="prompt-confirm"] .glyph').boundingBox();
            const shot0 = decodePng(await page.screenshot());
            const x0 = Math.floor(gb?.x ?? 0), y0 = Math.floor(gb?.y ?? 0);
            const w = Math.ceil(gb?.width ?? 0), h = Math.ceil(gb?.height ?? 0);
            const lumAt = (x: number, y: number) => {
              const p = pixel(shot0, x, y);
              return (p[0] + p[1] + p[2]) / 3;
            };
            const bgL = lumAt(x0 - 4, y0 + h / 2);
            let far = bgL;
            for (let y = y0; y < y0 + h; y++) for (let x = x0; x < x0 + w; x++) far = Math.abs(lumAt(x, y) - bgL) > Math.abs(far - bgL) ? lumAt(x, y) : far;
            const half = (bgL + far) / 2;
            const covered = (x: number, y: number) => (far < bgL ? lumAt(x, y) <= half : lumAt(x, y) >= half);
            const rows: number[] = [], cols: number[] = [];
            for (let y = y0; y < y0 + h; y++) for (let x = x0; x < x0 + w; x++) if (covered(x, y)) { rows.push(y); cols.push(x); }
            const dh = rows.length ? Math.max(...rows) - Math.min(...rows) + 1 : 0;
            const dw = cols.length ? Math.max(...cols) - Math.min(...cols) + 1 : 0;
            const want = 37 * px * (scale / 100);
            const msg = `footer glyph at 50 % coverage: ${dw} x ${dh} px (want about ${want.toFixed(1)}; bg ${bgL.toFixed(0)}, glyph ${far.toFixed(0)})`;
            console.warn(`D-023 ${vp.w}x${vp.h} ${scale} ${preset}: ${msg}`);
            expect(Math.abs(dw - want), msg).toBeLessThanOrEqual(2);
            expect(Math.abs(dh - want), msg).toBeLessThanOrEqual(2);
          }
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
          // B-5 (rev. r): about 46 px at 1080 p / 100 %, in one colour (the footer's text colour).
          expect(mark.h).toBeCloseTo(46 * px * (scale / 100), 0);
          expect(mark.half).toBe(mark.text);
          const hex = (c: string) => c.replace(/^#?(\w)(\w)(\w)$/, '#$1$1$2$2$3$3').toLowerCase();
          const rgbOf = (h: string) => `rgb(${[1, 3, 5].map((i) => parseInt(hex(h).slice(i, i + 2), 16)).join(', ')})`;
          expect(mark.diamond).toBe(mark.text);
          void rgbOf;
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
          expect(await lab.evaluate((el) => getComputedStyle(el).color)).toBe('rgb(255, 70, 162)'); // rev. v
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
          // ... and the focused button at about 108 % (B-14 rev. r).
          // rev. z: discs 108 px; the focused one about 117 px (108 %)
          expect(geo.b?.width ?? 0).toBeCloseTo(108 * 1.08 * px * (1 + (scale / 100 - 1) / 2), 0);
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
    // B-14 (rev. w): every fresh press at an end wraps, however fast: to the last slot, and back.
    await pad(page, 'DPadLeft');
    expect(await focusIndex(page)).toBe(11);
    await pad(page, 'DPadRight');
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
    // B-14 (rev. r): 12 slots: the 10 games, then 2 empty slots (index 11); holding stops there.
    expect(await focusIndex(page)).toBe(11);
    await pad(page, 'DPadLeft');
    await pad(page, 'DPadLeft');
    await expect(page.getByTestId('selected-title')).toHaveText('Zephyr Line Express');
    await page.waitForTimeout(400);
    await shot(page, 't1-home-focus-last');
    const l = await logs(page);
    expect(l.some((x) => x.startsWith('info: input down DPadLeft (controller 1 seq 6) -> action left, wrap to the last slot: empty slot 2 (after 10 games), sound wrap'))).toBe(true);
    expect(l.some((x) => x.startsWith('info: input down DPadRight (controller 1 seq 7) -> action right, wrap to the first game: focus 1/10, sound wrap'))).toBe(true);
    expect(l.filter((x) => x.includes('-> action right, focus 10/10')).length).toBe(1);
    // QA-6 (alpha.9): the empty slots are named as such.
    expect(l.filter((x) => x.includes('-> action right, empty slot 1 (after 10 games)')).length).toBe(1);
    expect(l.filter((x) => x.includes('-> action right, empty slot 2 (after 10 games)')).length).toBe(1);
    expect(l.filter((x) => x.includes('end of row, bump right, sound edgeBump')).length).toBe(1); // once per hold
    // No line says a move happened twice for one event: every acting line names one focus.
    const moves = l.filter((x) => /action (left|right), focus \d+\/10( \(row scrolls\))?, sound/.test(x));
    expect(moves.length).toBe(3 + 3 + 9 + 1); // right, left, the held run to game 10, back to game 10
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
    expect(track).toMatch(/^matrix\(1, 0, 0, 1, -808(\.\d+)?, 0\)$/);
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
    expect(await page.evaluate(() => window.__aioneMock?.launches)).toEqual([]); // nothing started
    // Another tile: still only a selection.
    await tile(3).click();
    expect(await page.evaluate(() => window.__aioneMock?.launches)).toEqual([]); // nothing started
    // The selected tile again: start.
    await tile(3).click();
    l = await logs(page);
    // T1.3: the second click on the selected game starts it (L-06), with the launch sound.
    expect(await page.evaluate(() => window.__aioneMock?.launches)).toEqual(['Moss Kingdom']);
    expect(l.some((x) => x.includes('start "Moss Kingdom" (second click), sound launch'))).toBe(true);
    // The game ends; Home again (B-11: Moss Kingdom is now first in the row).
    await page.waitForTimeout(900);
    await page.evaluate(() => window.__aioneMock?.gameExit('Moss Kingdom'));
    await page.waitForTimeout(300);
    // After starting nothing stays selected; leaving the tile leaves nothing highlighted.
    await page.mouse.move(960, 900);
    await expect(page.locator('.tile.hovered')).toHaveCount(0);
    await expect(page.locator('.tile.focused')).toHaveCount(0);
    await shot(page, 't1-home-mouse-after-start');
    // B-10: picking up the pad only brings the ring back on the remembered tile.
    await pad(page, 'DPadRight');
    await expect(page.getByTestId('screen')).toHaveAttribute('data-input', 'pad');
    // B-10: the ring comes back on the remembered tile: the last one hovered. (Moss Kingdom moved
    // to the front of the row under the resting pointer, so that is another game now.)
    await expect(page.locator('.tile.focused')).toHaveCount(1);
  });

  test('A / Enter start the focused game once; one sound per action; the row move on every repeat', async ({ page }) => {
    await open(page);
    await pad(page, 'A');
    await page.keyboard.press('Enter');
    let l = await logs(page);
    // T1.3: A starts the game once, with the launch sound; Enter right after falls in the launch
    // transition and is discarded (never a second start).
    expect(l.filter((x) => x.includes('-> action confirm, start "Amber Lighthouse" (press), sound launch'))).toHaveLength(1);
    expect(l.filter((x) => x.includes('discarded (launching a game: input lock)'))).toHaveLength(1);
    expect(await page.evaluate(() => window.__aioneMock?.launches)).toEqual(['Amber Lighthouse']);
    // Back to Home (the game ends) before the row's own checks.
    await page.waitForTimeout(900);
    await page.evaluate(() => window.__aioneMock?.gameExit('Amber Lighthouse'));
    await page.waitForTimeout(300);
    // B-1 (rev. p): holding a direction, the row move sounds on every move, every repeat included.
    await pad(page, 'DPadRight');
    for (let i = 0; i < 6; i++) {
      await page.waitForTimeout(90);
      await pad(page, 'DPadRight', 'repeat');
    }
    l = await logs(page);
    const moveLines = l.filter((x) => /input (down|repeat) DPadRight/.test(x));
    // rev. u: a move that scrolls the row plays rowScroll (the click, then the scroll tick).
    const played = moveLines.filter((x) => / sound row(Move|Scroll)/.test(x)).length;
    const quiet = moveLines.filter((x) => / sound row(Move|Scroll) \(-4 dB\)/.test(x)).length;
    console.warn(`row move sounds: ${played} of ${moveLines.length} moves, ${quiet} of them 4 dB quieter`);
    expect(moveLines).toHaveLength(7);
    expect(played).toBe(7);
    // B-1 (rev. s): during a hold the repeats get quieter after the first three.
    expect(quiet).toBe(3);
    // Every acting line names at most one sound.
    expect(l.every((x) => (x.match(/sound (rowMove|rowScroll|drawerMove|edgeBump|wrap|confirm|back|error|optionsOpen|optionsClose)/g) ?? []).length <= 1)).toBe(true);
    expect(l.some((x) => x.startsWith('info: menu sounds ready: 11 sounds'))).toBe(true);
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
      // Hover outline, accent shapes: ≥ 3:1. Text: ≥ 4.5:1.
      // Rev. v exception (the user's choice, like Switch): the ring and the title are the brand
      // pink #FF46A2 in both presets; the ring relies on its saturation and the white band, not on
      // 3:1 against the background (logged above, not asserted).
      expect(ratios.hover).toBeGreaterThanOrEqual(3);
      expect(ratios.accentOnSurface).toBeGreaterThanOrEqual(3);
      const v = (n: string) => page.evaluate((x) => getComputedStyle(document.documentElement).getPropertyValue(x).trim(), n);
      expect(await v('--color-ring')).toBe('#d9006c'); // rev. y, B3 (the base)
      expect(await v('--color-title')).toBe('#ff46a2');
      expect(ratios.soft).toBeGreaterThanOrEqual(4.5);
      expect(ratios.pressed).toBeGreaterThanOrEqual(4.5);
    }
    const ring = await page.locator('.tile.focused .ring-line:not(.ring-alt)').evaluate((el) => getComputedStyle(el).borderTopColor);
    expect(ring).toBe('rgb(217, 0, 108)'); // rev. y, B3 (the base)
  });

  test('B-9 (rev. y, B3): #D9006C → #FF58AB, crisp; a thin glow at the peak only; edge then 4 px white', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    const lumOf = (c: number[]) => {
      const f = (v: number) => ((v /= 255) <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
      return 0.2126 * f(c[0] ?? 0) + 0.7152 * f(c[1] ?? 0) + 0.0722 * f(c[2] ?? 0);
    };
    const dist = (a: number[], b: number[]) => Math.hypot((a[0] ?? 0) - (b[0] ?? 0), (a[1] ?? 0) - (b[1] ?? 0), (a[2] ?? 0) - (b[2] ?? 0));
    for (const preset of ['light', 'dark'] as const) {
      await open(page, `preset=${preset}`);
      const bg = preset === 'dark' ? [45, 45, 45] : [235, 235, 235];
      const sample: Record<string, { ring: number[]; near: number[]; far: number[] }> = {};
      for (const [end, t] of [['peak', 0], ['base', 760]] as const) {
        await page.evaluate((ms) => {
          document.querySelectorAll('.tile.focused .ring-alt, .tile.focused .ring-halo').forEach((e) =>
            e.getAnimations().forEach((a) => {
              a.pause();
              a.currentTime = ms;
            }),
          );
        }, t);
        await page.waitForTimeout(100);
        const g = await page.evaluate(() => {
          const r = (document.querySelector('.tile.focused .ring') as HTMLElement).getBoundingClientRect();
          const art = (document.querySelector('.tile.focused') as HTMLElement).getBoundingClientRect();
          return { x: r.x + r.width / 2, ringTop: r.y, artTop: art.y };
        });
        const img = decodePng(await page.screenshot());
        // The glow: 2 px outside the ring (in it at the peak), 7 px outside (never: thin).
        sample[end] = { ring: pixel(img, g.x, g.ringTop + 3.5), near: pixel(img, g.x, g.ringTop - 2), far: pixel(img, g.x, g.ringTop - 7) };
        if (end === 'base') {
          const col = Array.from({ length: Math.round(g.artTop - g.ringTop) }, (_, k) => pixel(img, g.x, Math.round(g.ringTop) + k));
          const firstWhite = col.findIndex((p) => p[0] === 255 && p[1] === 255 && p[2] === 255);
          expect(col.filter((p) => p[0] === 255 && p[1] === 255 && p[2] === 255).length).toBe(4); // the band
          expect(firstWhite).toBeGreaterThanOrEqual(8); // after the ring and the edge (D-020)
        }
      }
      const { ring: base, near: nearBase, far: farBase } = sample['base'] ?? { ring: [], near: [], far: [] };
      const { ring: peak, near: nearPeak, far: farPeak } = sample['peak'] ?? { ring: [], near: [], far: [] };
      console.warn(`B-9 rev. x ${preset}: ring ${JSON.stringify(base)} → ${JSON.stringify(peak)}; glow 2 px out ${JSON.stringify(nearBase)} → ${JSON.stringify(nearPeak)}, 7 px out ${JSON.stringify(farBase)} → ${JSON.stringify(farPeak)}`);
      expect(dist(base, [217, 0, 108])).toBeLessThanOrEqual(4); // B3: #D9006C at the base
      expect(dist(peak, [255, 88, 171])).toBeLessThanOrEqual(4); // B3: #FF58AB at the peak
      expect(lumOf(peak) / lumOf(base)).toBeGreaterThan(1.2);
      expect(dist(nearBase, bg)).toBeLessThanOrEqual(6); // no glow at the base
      expect(dist(nearPeak, bg)).toBeGreaterThan(25); // a glow at the peak, right outside the ring
      expect(dist(farPeak, bg)).toBeLessThanOrEqual(10); // thin: nothing 7 px out (no wide haze)
      await page.evaluate(() => document.querySelectorAll('.ring-alt, .ring-halo').forEach((e) => e.getAnimations().forEach((a) => (a.currentTime = 0))));
      await shot(page, `t18-ring-peak-${preset}`);
      await page.evaluate(() => document.querySelectorAll('.ring-alt, .ring-halo').forEach((e) => e.getAnimations().forEach((a) => (a.currentTime = 760))));
      await shot(page, `t18-ring-base-${preset}`);
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
    // D-017: clipped sideways only (the descenders are never cut).
    expect(await page.getByTestId('selected-title').evaluate((el) => getComputedStyle(el).overflowX)).toBe('clip');
    expect(await page.getByTestId('selected-title').evaluate((el) => getComputedStyle(el).overflowY)).toBe('visible');
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
    // B-1 (rev. r): entering a section plays its own move sound: down = the drawer's tick, up =
    // the row's click; no separate row <-> drawer sound.
    expect(lines.filter((x) => x.includes('to the drawer: Search, sound drawerMove'))).toHaveLength(1);
    expect(lines.filter((x) => x.includes('back to the row: focus 3/10, sound rowMove'))).toHaveLength(1);
    expect(lines.filter((x) => x.includes('sound drawerMove'))).toHaveLength(5); // in, right, right, left, left
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
    expect(await page.evaluate(() => window.__aioneMock?.launches)).toEqual([]); // nothing started
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

  test('B-14 (rev. w): only held repeats bump; every fresh press wraps; landing click; D-019; frames mid-motion', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await open(page);
    const isTranslate = (k: Keyframe) => String(k['transform'] ?? k['translate'] ?? '').includes('translate') || 'translate' in k;
    /** Pauses the newest bump animation on `sel` at `ms` and returns the element's box then. */
    const at = (sel: string, ms: number) =>
      page.evaluate(
        ([q, t]) => {
          const el = document.querySelector(q as string) as HTMLElement | null;
          const a = el?.getAnimations().filter((x) => (x.effect as KeyframeEffect).getKeyframes().some((k) => 'translate' in k || String(k['transform'] ?? '').includes('translate'))).pop();
          if (!el || !a) return null;
          a.pause();
          a.currentTime = t as number;
          const r = el.getBoundingClientRect();
          return { x: r.x, y: r.y, dur: Number((a.effect as KeyframeEffect).getTiming().duration) };
        },
        [sel, ms] as const,
      );
    const finish = (sel: string) => page.evaluate((q) => document.querySelector(q)?.getAnimations().forEach((a) => a.finish()), sel);
    void isTranslate;
    // 1. Up from the row (nothing above): only the ring, snappy.
    const tileBefore = await page.locator('.tile.focused').boundingBox();
    const rest = await page.evaluate(() => (document.querySelector('.tile.focused .ring') as HTMLElement).getBoundingClientRect().y);
    await pad(page, 'DPadUp');
    const f1 = await at('.tile.focused .ring', 16);
    expect(rest - (f1?.y ?? rest)).toBeGreaterThan(9); // the full push (about 12 px) within one frame
    const f66 = await at('.tile.focused .ring', 66);
    expect((f66?.y ?? 0) - rest).toBeGreaterThan(1); // back past the centre by about 66 ms
    expect(f1?.dur ?? 999).toBeLessThanOrEqual(200); // settled by about 180 ms
    await at('.tile.focused .ring', 16);
    await shot(page, 't14-bump-row-up-mid');
    expect(await page.locator('.tile.focused').boundingBox()).toEqual(tileBefore); // the tile stays still
    await finish('.tile.focused .ring');
    // 2. The drawer's right end and down: ring only.
    await pad(page, 'DPadDown');
    for (let i = 0; i < 5; i++) await pad(page, 'DPadRight');
    await page.waitForTimeout(400);
    const btnBefore = await page.locator('.dbtn.focused').boundingBox();
    const dRest = await page.evaluate(() => (document.querySelector('.dbtn.focused .dring') as HTMLElement).getBoundingClientRect().x);
    await pad(page, 'DPadRight');
    const d1 = await at('.dbtn.focused .dring', 16);
    expect((d1?.x ?? 0) - dRest).toBeGreaterThan(10); // about 15 px within one frame
    expect(d1?.dur ?? 999).toBeLessThanOrEqual(200);
    await shot(page, 't14-bump-drawer-right-mid');
    expect(await page.locator('.dbtn.focused').boundingBox()).toEqual(btnBefore);
    await finish('.dbtn.focused .dring');
    await pad(page, 'DPadDown');
    const dn = await at('.dbtn.focused .dring', 16);
    expect(dn).not.toBeNull();
    await finish('.dbtn.focused .dring');
    // 3. B in the drawer: back to the game, with the row's click.
    await pad(page, 'B');
    await expect(page.getByTestId('drawer')).toHaveAttribute('data-zone', 'row');
    expect(await focusIndex(page)).toBe(0);
    // 4. B-14 (rev. u): holding into the row's start bumps only the ring, once per hold.
    const t0 = await page.locator('.tile.focused').boundingBox();
    const trackX = () => page.evaluate(() => (document.querySelector('.track') as HTMLElement).getBoundingClientRect().x);
    const tx0 = await trackX();
    const ringX0 = await page.evaluate(() => (document.querySelector('.tile.focused .ring') as HTMLElement).getBoundingClientRect().x);
    await pad(page, 'DPadLeft', 'repeat');
    const rb = await at('.tile.focused .ring', 16);
    expect(ringX0 - (rb?.x ?? ringX0)).toBeGreaterThan(9); // about 14 px toward the press
    expect(rb?.dur ?? 999).toBeLessThanOrEqual(150);
    expect(await trackX()).toBe(tx0); // the row stays
    expect(await page.locator('.tile.focused').boundingBox()).toEqual(t0);
    await shot(page, 't18-row-end-bump-mid');
    await finish('.tile.focused .ring');
    for (let i = 0; i < 3; i++) await pad(page, 'DPadLeft', 'repeat');
    let l = await logs(page);
    expect(l.filter((x) => x.includes('end of row, held: no bump'))).toHaveLength(3);
    // 5. B-14 (rev. w): every fresh press at an end wraps, however fast (no threshold); the row
    //    whips across in 200 ms with the wrap sound, no bump first; and back the same way.
    await pad(page, 'DPadRight');
    await pad(page, 'DPadLeft'); // back on the first game
    const before = (await logs(page)).length;
    await pad(page, 'DPadLeft'); // a fast fresh press at the end: the wrap
    expect(await focusIndex(page)).toBe(11);
    const whip = await page.evaluate(() => {
      const a = document.querySelector('.track')?.getAnimations().find((x) => (x as CSSTransition).transitionProperty === 'transform');
      if (!a) return null;
      a.pause();
      a.currentTime = 70;
      return Number((a.effect as KeyframeEffect).getTiming().duration);
    });
    expect(whip).toBe(200);
    await shot(page, 't18-wrap-mid');
    await page.evaluate(() => document.querySelector('.track')?.getAnimations().forEach((a) => a.finish()));
    l = (await logs(page)).slice(before);
    expect(l.some((x) => x.includes('wrap to the last slot: empty slot 2 (after 10 games), sound wrap'))).toBe(true);
    expect(l.some((x) => x.includes('bump'))).toBe(false); // no bump before the wrap
    await pad(page, 'DPadRight');
    expect(await focusIndex(page)).toBe(0); // and back at once
    // 5a. The move landing on the first game plays the click alone, though the row scrolls.
    for (let i = 0; i < 6; i++) await pad(page, 'DPadRight');
    for (let i = 0; i < 6; i++) await pad(page, 'DPadLeft');
    l = await logs(page);
    const landing = l.filter((x) => x.includes('-> action left, focus 1/10')).pop() ?? '';
    console.warn(`landing: ${landing}`);
    expect(landing).toMatch(/focus 1\/10 \(row scrolls\), sound rowMove/);
    // 5b. D-019: Up and Left repeating together at the row's start (as the pad did in alpha.11):
    //     at most one bump per direction, then only "held: no bump".
    await page.waitForTimeout(100);
    const mark = (await logs(page)).length;
    await pad(page, 'DPadUp');
    await pad(page, 'DPadLeft');
    for (let i = 0; i < 30; i++) {
      await pad(page, i % 2 ? 'DPadUp' : 'DPadLeft', 'repeat');
    }
    l = (await logs(page)).slice(mark);
    const bumps = l.filter((x) => / bump (up|left), sound edgeBump/.test(x));
    console.warn(`D-019: ${bumps.length} bumps in 32 events (${l.filter((x) => x.includes('held: no bump')).length} held: no bump)`);
    expect(l.filter((x) => x.includes('bump up, sound edgeBump')).length).toBeLessThanOrEqual(1);
    expect(l.filter((x) => x.includes('bump left, sound edgeBump')).length).toBeLessThanOrEqual(1);
    await page.evaluate(() => {
      window.__aioneMock?.pad('DPadUp', 'up');
      window.__aioneMock?.pad('DPadLeft', 'up');
    });
    await page.evaluate(() => document.querySelectorAll('.ring, .dring').forEach((e) => e.getAnimations().forEach((a) => a.finish())));
    // 6. Regression: a normal move after a bump replays nothing.
    await page.waitForTimeout(300);
    await pad(page, 'DPadRight');
    const replays = await page.evaluate(() =>
      [...document.querySelectorAll('.ring, .dring, .track')].flatMap((e) => e.getAnimations()).filter((a) => (a.effect as KeyframeEffect).getKeyframes().some((k) => 'translate' in k || String(k['transform'] ?? '').includes('translate'))).length,
    );
    expect(replays).toBe(0);
  });

  test('B-14: empty slots after the last game; no title there; A does nothing; "A OK" dimmed', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await open(page);
    expect(await page.getByTestId('empty-slot').count()).toBe(2);
    for (let i = 0; i < 11; i++) await page.keyboard.press('ArrowRight');
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
