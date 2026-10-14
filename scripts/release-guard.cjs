// A11 (alpha.10 QA-1): CI publishes a release only when APP CODE changed, named explicitly:
// src/, src-tauri/, assets/, index.html, package.json and its lockfile, the Vite and Svelte config
// (the Tauri config lives in src-tauri/). Test, CI, tool and docs files never publish.
// Used by the release job (.github/workflows/ci.yml) and by tests/unit/release-guard.test.ts.
'use strict';

const APP_CODE = /^(src\/|src-tauri\/|assets\/|index\.html$|package\.json$|package-lock\.json$|vite\.config\.[cm]?[jt]s$|svelte\.config\.[cm]?js$)/;

/** The changed files that are app code. */
function appFiles(files) {
  return files.filter((f) => APP_CODE.test(f));
}

/** Whether a push publishes a release: when app code changed, or when the current version has
 *  no release yet (alpha.12, CI run #18: the app-code push failed its tests, the follow-up test
 *  fix changed no app code, and the version was left without a release). */
function shouldRelease(files, releaseExists = true) {
  return !releaseExists || appFiles(files).length > 0;
}

module.exports = { APP_CODE, appFiles, shouldRelease };
