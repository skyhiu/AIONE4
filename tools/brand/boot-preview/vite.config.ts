import { resolve } from 'node:path';
import { defineConfig } from 'vite';

// Builds the standalone Boot Up preview (opens from the file system: relative paths, sounds as files).
export default defineConfig({
  root: __dirname,
  base: './',
  build: {
    // Relative to the project root (the config's root is this folder).
    outDir: process.env['PREVIEW_OUT'] ?? resolve(__dirname, '../../../dist-boot-preview'),
    emptyOutDir: true,
    assetsInlineLimit: 0,
    target: 'chrome120',
  },
});
