import { expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import path from 'node:path';

export const SCREENSHOT_DIR = path.resolve('ui-screenshots');

/** Saves a full-viewport screenshot to ui-screenshots/<name>.png (uploaded by CI). */
export async function shot(page: Page, name: string): Promise<void> {
  mkdirSync(SCREENSHOT_DIR, { recursive: true });
  // Let transitions settle so the screenshot shows the resting state.
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(SCREENSHOT_DIR, `${name}.png`), fullPage: false });
}

/** The page never scrolls or overflows the viewport (fullscreen app). */
export async function expectNoOverflow(page: Page): Promise<void> {
  const o = await page.evaluate(() => ({
    sw: document.documentElement.scrollWidth,
    sh: document.documentElement.scrollHeight,
    w: window.innerWidth,
    h: window.innerHeight,
  }));
  expect(o.sw, 'horizontal overflow').toBeLessThanOrEqual(o.w);
  expect(o.sh, 'vertical overflow').toBeLessThanOrEqual(o.h);
}

/** Every element matching the selector is fully inside the viewport (not clipped). */
export async function expectInViewport(page: Page, selector: string): Promise<void> {
  const boxes = await page.locator(selector).evaluateAll((els) =>
    els.map((el) => {
      const r = el.getBoundingClientRect();
      return { l: r.left, t: r.top, r: r.right, b: r.bottom, w: window.innerWidth, h: window.innerHeight };
    }),
  );
  for (const b of boxes) {
    expect(b.l).toBeGreaterThanOrEqual(0);
    expect(b.t).toBeGreaterThanOrEqual(0);
    expect(b.r).toBeLessThanOrEqual(b.w);
    expect(b.b).toBeLessThanOrEqual(b.h);
  }
}

/** WCAG contrast ratio of an element's text colour over the page background colour. */
export async function contrastOf(page: Page, selector: string): Promise<number> {
  return page.locator(selector).first().evaluate((el) => {
    const parse = (c: string): [number, number, number] => {
      const m = c.match(/[\d.]+/g) ?? ['0', '0', '0'];
      return [Number(m[0]), Number(m[1]), Number(m[2])];
    };
    const lum = ([r, g, b]: [number, number, number]) => {
      const f = (v: number) => {
        const s = v / 255;
        return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
      };
      return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
    };
    // Walk up to the first ancestor with an opaque background.
    let bgEl: Element | null = el;
    let bg = 'rgb(0, 0, 0)';
    while (bgEl) {
      const c = getComputedStyle(bgEl).backgroundColor;
      if (c && !c.startsWith('rgba(0, 0, 0, 0)') && c !== 'transparent') {
        bg = c;
        break;
      }
      bgEl = bgEl.parentElement;
    }
    const a = lum(parse(getComputedStyle(el).color));
    const b = lum(parse(bg));
    return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
  });
}
