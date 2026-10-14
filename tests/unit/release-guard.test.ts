import { createRequire } from 'node:module';
import { describe, expect, it } from 'vitest';

const { shouldRelease, appFiles } = createRequire(import.meta.url)('../../scripts/release-guard.cjs') as {
  shouldRelease: (f: string[]) => boolean;
  appFiles: (f: string[]) => string[];
};

describe('the release guard (A11, alpha.10 QA-1)', () => {
  it("replays aed4477's file list (CI only): no release", () => {
    // aed4477 "CI reports failures as annotations": the push behind alpha.9 QA-5.
    expect(shouldRelease(['.github/workflows/ci.yml', 'scripts/ci-run.mjs', 'playwright.config.ts'])).toBe(false);
  });
  it('tests, docs, tools and top-level test configs never publish', () => {
    expect(shouldRelease(['tests/ui/home.spec.ts', 'docs/CHANGELOG.md', 'tools/sound/make_menu_family.py', 'README.md', 'vitest.config.ts', 'eslint.config.js', 'scripts/check-assets.mjs'])).toBe(false);
  });
  it('app code publishes', () => {
    for (const f of ['src/App.svelte', 'src-tauri/src/lib.rs', 'src-tauri/tauri.conf.json', 'assets/sounds/boot/aione_boot_up.wav', 'index.html', 'package.json', 'package-lock.json', 'vite.config.ts', 'svelte.config.js'])
      expect(shouldRelease([f]), f).toBe(true);
    expect(appFiles(['docs/x.md', 'src/main.ts'])).toEqual(['src/main.ts']);
  });
});
