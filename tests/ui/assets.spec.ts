import { readFileSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';

// A16 (rev. bk): what is on screen uses only the user's assets. The static check (`check:assets`)
// reads the source; this reads the rendered page, so names computed at runtime are covered too:
// every Material Symbols icon on screen is a name the pinned font has (a missing one shows as its
// raw text), and every text element's font is the design system's.
const MATERIAL = new Set(
  [...readFileSync('node_modules/material-symbols/index.d.ts', 'utf8').matchAll(/^\s*"([a-z0-9_]+)",?$/gm)].map((m) => m[1]),
);
const ALLOWED_FONTS = ['M PLUS Rounded 1c', 'Material Symbols Rounded'];

async function audit(page: Page, where: string) {
  const found = await page.evaluate(() => {
    const icons = [...document.querySelectorAll('span.icon')].map((e) => (e.textContent ?? '').trim()).filter(Boolean);
    const fonts = new Set<string>();
    for (const e of document.querySelectorAll('body *')) {
      if (!(e instanceof HTMLElement) || !e.offsetParent) continue;
      if (![...e.childNodes].some((n) => n.nodeType === 3 && (n.textContent ?? '').trim())) continue;
      fonts.add(getComputedStyle(e).fontFamily.split(',')[0]?.trim().replace(/^["']|["']$/g, '') ?? '');
    }
    return { icons, fonts: [...fonts] };
  });
  const badIcons = found.icons.filter((n) => !MATERIAL.has(n));
  const badFonts = found.fonts.filter((f) => !ALLOWED_FONTS.includes(f));
  expect(badIcons, `${where}: icons not in Material Symbols Rounded`).toEqual([]);
  expect(badFonts, `${where}: fonts outside the design system`).toEqual([]);
  return found.icons;
}
const hold = (page: Page, c: string) => page.evaluate((x) => window.__aioneMock?.pad(x, 'down'), c);
const release = (page: Page, c: string) => page.evaluate((x) => window.__aioneMock?.pad(x, 'up'), c);

test.describe('A16: the rendered UI uses only the user assets', () => {
  for (const battery of ['3', '50', '100', '40c']) {
    test(`Home, the header with battery ${battery}`, async ({ page }) => {
      await page.setViewportSize({ width: 1920, height: 1080 });
      await page.goto(`./?time=1:10&h12=1&pad=connected&network=wifi&battery=${battery}`);
      await page.locator('main[data-phase="home"]').waitFor();
      await page.waitForTimeout(300);
      const icons = await audit(page, `Home (battery ${battery})`);
      expect(icons.some((n) => n.startsWith('battery_android'))).toBe(true);
    });
  }

  for (const muted of ['0', '1']) {
    test(`the Quick Menu and Power Options (muted ${muted}), and a notification`, async ({ page }) => {
      await page.setViewportSize({ width: 1920, height: 1080 });
      await page.goto(`./?time=1:10&h12=1&pad=connected&muted=${muted}`);
      await page.locator('main[data-phase="home"]').waitFor();
      await page.waitForTimeout(300);
      await hold(page, 'Guide');
      await page.waitForTimeout(650);
      await release(page, 'Guide');
      await expect(page.getByTestId('quick-menu')).toBeVisible();
      await page.waitForTimeout(300);
      const icons = await audit(page, 'Quick Menu');
      expect(icons).toEqual(expect.arrayContaining(['power_settings_new', 'light_mode', muted === '1' ? 'volume_off' : 'volume_up']));
      await page.screenshot({ path: `ui-screenshots/a16-quick-menu-icons-muted${muted}.png` });
      await page.keyboard.press('Escape');
      await page.waitForTimeout(400);
      await page.evaluate(() => window.__aioneMock?.disconnect(0));
      await expect(page.getByTestId('notification')).toBeVisible();
      await audit(page, 'notification');
    });
  }
});
