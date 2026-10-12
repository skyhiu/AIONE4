import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import svelteConfig from './svelte.config.js';

export default tseslint.config(
  {
    ignores: [
      'dist/',
      'dist-mock/',
      'node_modules/',
      'src-tauri/target/',
      'src-tauri/gen/',
      'test-results/',
      'playwright-report/',
      'ui-screenshots/',
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.strict,
  ...svelte.configs.recommended,
  {
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
    },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: ['.svelte'],
        svelteConfig,
      },
    },
  },
  {
    rules: {
      // A3: no `any` in new code without a comment (use an eslint-disable line with the reason).
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/consistent-type-imports': 'error',
      'no-console': ['error', { allow: ['warn', 'error'] }],
    },
  },
);
