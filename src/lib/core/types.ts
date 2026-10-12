// Types shared between the UI and the Rust core (src-tauri). Field names match the
// core's serde output (camelCase). Keep both sides in step when changing a type.

export interface AppInfo {
  version: string;
  dataDir: string;
  logsDir: string;
  platform: string;
  pid: number;
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
}

export interface LibraryCounts {
  games: number;
  hidden: number;
  favorites: number;
  ignored: IgnoredEntry[];
}

export interface IgnoredEntry {
  name: string;
  reason: string;
}

export type MigrationStatus =
  | { state: 'noSource'; source: string }
  | { state: 'alreadyHandled'; decision: string }
  | {
      state: 'offer';
      source: string;
      files: number;
      bytes: number;
      library: LibraryCounts;
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
  /** accept = true copies the WPF data folder; false records "declined" (asked only once). */
  migrationDecide(accept: boolean): Promise<void>;
  restart(): Promise<void>;
  onPad(handler: (event: PadEvent) => void): Promise<Unlisten>;
  onCopyProgress(handler: (progress: CopyProgress) => void): Promise<Unlisten>;
  onCopyDone(handler: (result: CopyResult) => void): Promise<Unlisten>;
}
