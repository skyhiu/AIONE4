import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  AppInfo,
  BootMedia,
  LaunchResult,
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
  tile: 'aione://tile',
  closeRequested: 'aione://close-requested',
  gameExited: 'aione://game-exited',
  gameWindow: 'aione://game-window',
  closeHint: 'aione://close-hint',
  gameFollowing: 'aione://game-following',
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
    mediaUrl: (gameId, slot, key) =>
      `${MEDIA_BASE}/${encodeURIComponent(gameId)}/${encodeURIComponent(slot)}${key ? `?v=${encodeURIComponent(key)}` : ''}`,
    statusNow: () => invoke<PcStatus>('status_now'),
    onStatus: (handler) => listen<PcStatus>(EVENTS.status, (e) => handler(e.payload)),
    launchGame: (id) => invoke<LaunchResult>('launch_game', { id }),
    returnToGame: (id) => invoke<string>('return_to_game', { id }),
    forceCloseGame: async (id) => {
      await invoke('force_close_game', { id });
    },
    closeGame: async (id) => {
      await invoke('close_game', { id });
    },
    onGameFollowing: (handler) => listen<{ id: string; name: string }>(EVENTS.gameFollowing, (e) => handler(e.payload)),
    onCloseHint: (handler) => listen<{ id: string; text: string }>(EVENTS.closeHint, (e) => handler(e.payload)),
    onGameWindow: (handler) => listen<{ id: string; name: string; ms: number }>(EVENTS.gameWindow, (e) => handler(e.payload)),
    runningGame: () => invoke<string | null>('running_game'),
    onGameExited: (handler) => listen<{ id: string; name: string; minutes: number }>(EVENTS.gameExited, (e) => handler(e.payload)),
    bootMedia: () => invoke<BootMedia>('boot_media'),
    bootUrl: (which, key) => `${MEDIA_BASE}/_boot/${which}?v=${encodeURIComponent(key)}`,
    onCloseRequested: (handler) => listen(EVENTS.closeRequested, () => handler()),
    closeNow: (reason) => invoke('close_now', { reason }).then(() => undefined),
    onTile: (handler) => listen<{ id: string; tile: string }>(EVENTS.tile, (e) => handler(e.payload)),
    onPad: (handler) => listen<PadEvent>(EVENTS.pad, (e) => handler(e.payload)),
    onCopyProgress: (handler) => listen<CopyProgress>(EVENTS.copyProgress, (e) => handler(e.payload)),
    onCopyDone: (handler) => listen<CopyResult>(EVENTS.copyDone, (e) => handler(e.payload)),
  };
}
