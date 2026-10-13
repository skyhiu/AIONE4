import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  AppInfo,
  CopyProgress,
  CopyResult,
  Core,
  LibraryLoad,
  MigrationStatus,
  PadEvent,
  PadSlots,
  PcStatus,
} from './types';

/** Event names emitted by the Rust core (src-tauri/src/events.rs). */
export const EVENTS = {
  pad: 'aione://pad',
  copyProgress: 'aione://copy-progress',
  copyDone: 'aione://copy-done',
  status: 'aione://status',
} as const;

/** The core's media protocol (`aione-media`); WebView2 reaches custom protocols as
 *  http://<scheme>.localhost/. */
export const MEDIA_BASE = 'http://aione-media.localhost';

export function createTauriCore(): Core {
  return {
    kind: 'tauri',
    appInfo: () => invoke<AppInfo>('app_info'),
    uiReady: async () => {
      await invoke('ui_ready');
    },
    log(level, message) {
      invoke('log_ui', { level, message }).catch((err: unknown) => {
        console.error('log_ui failed', err);
      });
    },
    migrationStatus: () => invoke<MigrationStatus>('migration_status'),
    migrationDecide: async (accept) => {
      await invoke('migration_decide', { accept });
    },
    padSlots: () => invoke<PadSlots>('pad_slots'),
    restart: async () => {
      await invoke('restart_app');
    },
    libraryLoad: () => invoke<LibraryLoad>('library_load'),
    mediaUrl: (gameId, slot) => `${MEDIA_BASE}/${encodeURIComponent(gameId)}/${encodeURIComponent(slot)}`,
    statusNow: () => invoke<PcStatus>('status_now'),
    onStatus: (handler) => listen<PcStatus>(EVENTS.status, (e) => handler(e.payload)),
    onPad: (handler) => listen<PadEvent>(EVENTS.pad, (e) => handler(e.payload)),
    onCopyProgress: (handler) => listen<CopyProgress>(EVENTS.copyProgress, (e) => handler(e.payload)),
    onCopyDone: (handler) => listen<CopyResult>(EVENTS.copyDone, (e) => handler(e.payload)),
  };
}
