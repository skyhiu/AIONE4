import type { Core } from './types';

/** Loads the core: the mocked one in `--mode mock` builds (UI tests), Tauri otherwise.
 *  The mock is a separate chunk and is not part of the production build. */
export async function loadCore(): Promise<Core> {
  if (import.meta.env.MODE === 'mock') {
    const { createMockCore } = await import('./mock');
    return createMockCore();
  }
  const { createTauriCore } = await import('./tauri');
  return createTauriCore();
}

export type * from './types';
