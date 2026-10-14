// Types shared between the UI and the Rust core (src-tauri). Field names match the
// core's serde output (camelCase). Keep both sides in step when changing a type.

export interface AppInfo {
  version: string;
  dataDir: string;
  logsDir: string;
  platform: string;
  pid: number;
  /** Interface Scale in percent for this run (100 until Settings exist; `--interface-scale`). */
  interfaceScale: number;
  fromAutostart: boolean;
  /** `--boot=default`: this run shows Window B even when a custom boot video exists. */
  bootDefault: boolean;
  /** B-15: `--extras=press,titlefade,labelfade`: proposed extras for this run (off by default). */
  extras: string[];
}

/** A custom boot video, copied once from the WPF copy into AIONE 4's `Boot` folder. */
export interface BootFile {
  key: string;
  ext: string;
  bytes: number;
  source: string;
}

export interface BootMedia {
  up: BootFile | null;
  down: BootFile | null;
  settings: { showSplash: boolean; splashAudio: boolean; playToEnd: boolean; durationMs: number; from: string };
  notes: string[];
}

/** A game as the core reads it from AIONE 4's library (`Games\\<folder>\\game.json`). */
export interface GameView {
  /** The game's folder name (unique). */
  id: string;
  name: string;
  /** The media slot shown on the tile ("Thumbnail", …), or null for a plain tile. */
  tileArt: string | null;
  /** B-13: 'fill' (the icon fills the tile) or 'fit' (another picture shown whole), or null. */
  tileMode: 'fill' | 'fit' | null;
  /** D-007 / D-010: cache key of the tile-sized copy (640 × 640), or null until the core has
   *  made it in the background (`onTile`); meanwhile the original picture is shown. */
  tile: string | null;
  favorite: boolean;
  hidden: boolean;
  category: string | null;
  playCount: number;
  playTimeMinutes: number;
}

export interface LibraryLoad {
  state: 'loaded' | 'empty' | 'error';
  /** Every game (hidden ones included), sorted by name. */
  games: GameView[];
  source: string;
  skipped: IgnoredEntry[];
  notes: string[];
  /** How many games the copy counted (L-17), if known. */
  expected: number | null;
}

/** The PC's status for the header (the core pushes changes). */
export interface PcStatus {
  network: 'wired' | 'wifi' | 'offline' | 'unknown';
  /** null: no battery in this PC. */
  battery: { percent: number | null; charging: boolean } | null;
}

/** A normalized controller event from the Rust core (XInput). */
export type PadEventKind = 'down' | 'up' | 'repeat' | 'connected' | 'disconnected';

export interface PadEvent {
  kind: PadEventKind;
  /** Control name, e.g. "A", "DPadUp", "LStickLeft", "LT". Empty for connect / disconnect. */
  control: string;
  /** XInput slot, 0–3. */
  pad: number;
  /** Monotonic sequence number from the core (detects lost or duplicated events). */
  seq: number;
  /** D-008: when the core read it (Unix ms, same clock as Date.now()). */
  atMs: number;
}

export interface LibraryCounts {
  games: number;
  hidden: number;
  favorites: number;
  /** Library folders (groups of games), not file-system directories. */
  folders: number;
  /** Entries in Games that are not games, each with its reason. */
  ignored: IgnoredEntry[];
  /** Where the hidden / favorites / folders counts were read from (for the log). */
  sources: string[];
}

/** D-002: the connected controller slots (0–3) and the sequence number of the last
 *  connect / disconnect event, so a query and the event stream can be merged in order. */
export interface PadSlots {
  slots: number[];
  seq: number;
}

export interface IgnoredEntry {
  name: string;
  reason: string;
}

export type MigrationStatus =
  | { state: 'noSource'; source: string }
  /** `notice`: a plain message for the user (B-6: e.g. the record could not be read), or null. */
  | { state: 'alreadyHandled'; decision: string; notice: string | null }
  | {
      state: 'offer';
      source: string;
      files: number;
      bytes: number;
      library: LibraryCounts;
      notice: string | null;
    };

export interface CopyProgress {
  filesDone: number;
  filesTotal: number;
  bytesDone: number;
  bytesTotal: number;
}

export interface CopyResult {
  ok: boolean;
  /** "copied" | "partial" | "failed" */
  outcome: string;
  destination: string;
  filesCopied: number;
  filesFailed: number;
  bytesCopied: number;
  library: LibraryCounts;
  sourceUnchanged: boolean;
  notes: string[];
}

export type Unlisten = () => void;

/** Everything the UI may ask of the core. The real implementation talks to Tauri; the
 *  mock (tests, `vite --mode mock`) simulates it. */
export interface Core {
  readonly kind: 'tauri' | 'mock';
  appInfo(): Promise<AppInfo>;
  /** Tells the core the first frame is drawn: it shows the window and takes the foreground. */
  uiReady(): Promise<void>;
  log(level: 'info' | 'warn' | 'error', message: string): void;
  migrationStatus(): Promise<MigrationStatus>;
  /** accept = true copies the WPF data folder; false records "declined" (final, B-2).
   *  "Not now" never calls this: nothing is recorded, so the question returns next start. */
  migrationDecide(accept: boolean): Promise<void>;
  /** D-002: the controller slots connected right now. */
  padSlots(): Promise<PadSlots>;
  restart(): Promise<void>;
  libraryLoad(): Promise<LibraryLoad>;
  /** URL of a game's media slot (served only from the game's own folder), or of its cached
   *  'tile' / 'icon' copy; `key` busts the web view's cache when the copy changes. */
  mediaUrl(gameId: string, slot: string, key?: string): string;
  statusNow(): Promise<PcStatus>;
  onStatus(handler: (status: PcStatus) => void): Promise<Unlisten>;
  /** T1.2: the custom boot videos and the boot settings. */
  bootMedia(): Promise<BootMedia>;
  /** URL of a custom boot video ('up' or 'down'). */
  bootUrl(which: 'up' | 'down', key: string): string;
  /** A9: Close AIONE was requested (Alt+F4): play the Boot Down, then `closeNow`. */
  onCloseRequested(handler: () => void): Promise<Unlisten>;
  closeNow(reason: string): Promise<void>;
  /** D-010: a tile copy made in the background. */
  onTile(handler: (t: { id: string; tile: string }) => void): Promise<Unlisten>;
  onPad(handler: (event: PadEvent) => void): Promise<Unlisten>;
  onCopyProgress(handler: (progress: CopyProgress) => void): Promise<Unlisten>;
  onCopyDone(handler: (result: CopyResult) => void): Promise<Unlisten>;
}
