import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// The Tauri CLI sets TAURI_ENV_* variables while it runs this config.
const isTauriBuild = process.env['TAURI_ENV_PLATFORM'] !== undefined;

export default defineConfig(({ mode }) => ({
  plugins: [svelte()],
  // Relative paths so the same build works inside Tauri and in `vite preview`.
  base: './',
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    // WebView2 is evergreen Chromium; the UI never runs in another browser.
    target: 'chrome120',
    sourcemap: mode === 'mock' || !isTauriBuild,
    outDir: mode === 'mock' ? 'dist-mock' : 'dist',
    emptyOutDir: true,
    // D-009: never inline sounds as data: URLs. The app's content security policy only lets
    // fetch() reach 'self', so an inlined sound (move.wav, 3.6 KB, was under Vite's 4 KB limit)
    // could not be loaded in the release. Other small assets keep Vite's default.
    assetsInlineLimit: (file: string) => (/\.(wav|mp3|ogg|flac|m4a)$/i.test(file) ? false : undefined),
  },
}));
