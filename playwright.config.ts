import { defineConfig, devices } from '@playwright/test';

// UI tests run against the UI built with the mocked core (`vite build --mode mock`).
// Screenshots are written to ./ui-screenshots and uploaded by CI as `ui-screenshots`.
// PW_CHROMIUM_EXECUTABLE (optional) points to an already installed Chromium, for machines
// that cannot download Playwright's browser. CI does not set it.
const chromiumPath = process.env['PW_CHROMIUM_EXECUTABLE'];

export default defineConfig({
  testDir: 'tests/ui',
  outputDir: 'test-results',
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env['CI'],
  retries: 0,
  // In CI also the "github" reporter: each failing test becomes a GitHub annotation (test name,
  // file, line, the error with expected / received), readable through api.github.com.
  reporter: process.env['CI']
    ? [['github'], ['list'], ['html', { open: 'never', outputFolder: 'playwright-report' }]]
    : [['list'], ['html', { open: 'never', outputFolder: 'playwright-report' }]],
  use: {
    baseURL: 'http://127.0.0.1:4173',
    trace: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1920, height: 1080 },
        deviceScaleFactor: 1,
        ...(chromiumPath ? { launchOptions: { executablePath: chromiumPath } } : {}),
      },
    },
  ],
  webServer: {
    command: 'npm run build:mock && npm run preview:mock',
    url: 'http://127.0.0.1:4173',
    reuseExistingServer: !process.env['CI'],
    timeout: 120_000,
  },
});
