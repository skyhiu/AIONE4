# AIONE 4 — Architecture

Current as of **4.0.0-alpha.3 (T0)**. Rules: Master Prompt Part A (A3 technology, A4 input, A5 window and lifecycle, A9 data).

## Overview

```
┌──────────────────────────── AIONE4.exe (Tauri 2) ────────────────────────────┐
│ Rust core (src-tauri/)                       │ TypeScript UI (src/, Svelte 5) │
│                                              │                                │
│ lifecycle.rs  args, restart wait, autostart  │ App.svelte   first window     │
│ paths.rs      %LOCALAPPDATA%\AIONE4, \AIONE  │ lib/core     Core interface   │
│ logging.rs    aione.log, lifecycle.log       │   tauri.ts   invoke + listen  │
│ foreground.rs take / hold the foreground     │   mock.ts    UI tests only    │
│ input/        XInput thread → aione://pad    │ lib/input    events, readout  │
│ library.rs    what counts as a game          │ lib/migrationFlow copy dialog │
│ migration.rs  one-time copy of WPF data      │ components/  readout, dialog  │
│ lib.rs        commands, window, app setup    │ app.css      design tokens    │
└──────────────────────────────────────────────┴────────────────────────────────┘
          commands (UI → core)  ·  events (core → UI)
```

The core owns everything that touches the system; the UI owns everything the user sees (A3). The UI never reads files or the controller itself.

## Commands (UI → core)

| Command | Purpose |
|---|---|
| `app_info` | version, data and logs folders, pid |
| `ui_ready` | the first frame is drawn: show the window and take the foreground |
| `log_ui(level, message)` | UI messages into `aione.log` (area `ui`) |
| `migration_status` | whether to offer the one-time copy (with counts) |
| `migration_decide(accept)` | copy (background thread) or record "declined" (final). "Not now" (B) calls nothing: the question returns at the next start (B-2). |
| `pad_slots` | the controller slots connected right now and the sequence number of the last connect / disconnect (D-002) |
| `restart_app` | start a new instance that waits for this one, then exit |

## Events (core → UI)

| Event | Payload |
|---|---|
| `aione://pad` | `{ kind: down/up/repeat/connected/disconnected, control, pad (0–3), seq }` |
| `aione://copy-progress` | `{ filesDone, filesTotal, bytesDone, bytesTotal }` (about every 100 ms) |
| `aione://copy-done` | the copy result (outcome, counts, source unchanged, notes) |

## Start-up sequence

1. Parse arguments. Create `%LOCALAPPDATA%\AIONE4\Logs\`; open the logs; log `start`.
2. **`lifecycle::pre_start`**: the steps before Tauri, in this order (unit-tested with a fake
   that records the order):
   1. `--autostart=on|off`: write or remove `HKCU\…\Run\AIONE 4`, show a message, exit;
   2. `--wait-for-pid N` (restart): wait for the old instance to exit (≤ 15 s, logged);
   3. only then, the named-mutex "already running" check (a second start logs the hand-over).
3. Tauri starts. The **single-instance plugin** is registered only after `pre_start` returns, as
   the first plugin. A second start exits there, and the running instance logs it and takes
   the foreground. A restarted instance has already waited, so it is never taken for a second
   start.
4. The window is created **hidden** (fullscreen, borderless, background `#0f1013`). The XInput
   thread starts.
5. The UI mounts, subscribes to pad events, asks `pad_slots` (D-002), draws two frames, and
   calls `ui_ready`.
6. The core shows the window and runs the foreground chain (`foreground.rs`), logging the
   result. It then logs which window has keyboard focus, judged by **window ownership** (D-005):
   - "AIONE 4's web view: keys reach AIONE" when the focus window's top-level ancestor is
     AIONE's main window (WebView2 draws the page in its own `msedgewebview2.exe` process);
   - otherwise "another program has the keys: …".

   It logs this again 500 ms later. If the UI never reports ready, the window is shown after
   4 s anyway.
7. The **start-up foreground guard** listens to foreground changes for 8 s and takes the
   foreground back from other programs (at most 3 times) unless the user is switching.
8. The UI asks `migration_status`: the copy question, a notice, or nothing (see "Stored files").

## Input path (A4)

XInput (`xinput1_4.dll`, `XInputGetStateEx` for the Guide button) is polled on its own thread in
the core:
- every 8 ms while a pad is connected;
- empty slots are probed every 1 s;
- the thread sleeps 250 ms between checks when no pad is connected.

`input/mapping.rs` turns raw state into button events:
- **Sticks** become directions, using the stronger axis. A direction presses at about 49 % and
  releases at about 35 % (hysteresis).
- **Triggers** become buttons: press at 64, release at 32.
- Events reach the UI also while AIONE is not focused.

**Repeat rules (A4, B-3):**
- The **D-pad and the left stick** repeat after 400 ms, then every 90 ms (arrow keys repeat by
  the keyboard's own auto-repeat in the UI).
- **Buttons never repeat**, except the on-screen keyboard's hold features (X, LB / RB) in T6,
  which the T6 keyboard will handle itself.
- **The right stick never repeats:** it will drive the mouse-like pointer (reference A7), so it
  needs continuous movement, not steps. A unit test checks this.

**Connected pads at start (D-002):**
1. The core keeps a snapshot of the connected slots with the sequence number of the last
   connect / disconnect (`input::pad_slots`).
2. The UI subscribes to `aione://pad` first, then asks `pad_slots`.
3. It applies whichever is newer by sequence number (`applyPadSlots` / `applyPadConnection`).
   This way a pad connected before the UI listened shows from the first frame, and an older
   answer never overwrites a newer event.

Keyboard and mouse are handled in the UI (`lib/input/events.ts`) and mapped to the same actions
(`toAction`). The browser Gamepad API is not used (one source per press).

**Key repeat (D-006, B-3):** `lib/input/keyRepeat.ts`.
- AIONE times key repeats itself: 400 ms, then every 90 ms, like the pad.
- It ignores the browser's / Windows' own repeated keydown events, so Windows' keyboard settings never change the speed.
- Modifier keys (Alt, Ctrl, Shift, Windows) never repeat.
- Only the most recently pressed key repeats.
- Losing window focus releases all held keys.
- One `setTimeout` chain runs only while a key is held (owner: `KeyRepeater`).

## Logging (evidence lines)

| Line | Where | Shows |
|---|---|---|
| `pad 1 down A (seq N)` / `pad 1 up A (seq N), held N ms, R repeats` | `aione.log` [input] | each press and release in the core; the hold length and repeats on release |
| `input down A (controller 1 seq N) -> presses P, action confirm` | `aione.log` [ui] | each press or repeat the UI acted on, with the core's sequence number (one press shows once) |
| `controller slots at start: [1] (as of seq N)` | `aione.log` [ui] | D-002 state at start |
| `second instance: AIONE 4 is already running; handing over …` | `lifecycle.log` | the second start's own line before it exits |
| `foreground (start, … ms after start): OK via …` | `lifecycle.log` | L-20 result and fallback |
| `keyboard focus after the foreground step: AIONE 4's web view: keys reach AIONE (…)` (or `another program has the keys: …`) and `… 500 ms later` | `lifecycle.log` | which window gets the keys (D-005) |
| `input repeat Right (keyboard) -> …` | `aione.log` [ui] | AIONE-timed key repeats (first after 400 ms, then every 90 ms; never for modifiers) |
| `input click "Copy" (mouse) -> action copy` | `aione.log` [ui] | a click and the action it activated |
| `copy record read: decision "copied" (written by AIONE 4 … at …); not asking again` | `aione.log` [copy] | B-6: the record from an earlier version was read |
| `first keyboard event N ms after the page started (page focused: …)` | `aione.log` [ui] | when the first key reached the UI |
| `copy …; 10 games, 0 hidden, 0 favorites, 0 folders; 1 skipped`, `skipped in Games: …`, `library hidden: …`, `library favorites: …`, `library folders: …` (always; "none found, looked in: …; not yet read: Config\"), `WPF data folder top level: …` | `aione.log` [copy] | L-17 counts with their sources (a log-capture test checks these are written) |

## Data folder (A9)

```
%LOCALAPPDATA%\AIONE4\
  Logs\aione.log, lifecycle.log (+ .1.log after 5 MB)
  migration.json                the copy decision and result (asked once)
  Imported\AIONE-WPF\           the copy of %LOCALAPPDATA%\AIONE\ (T0; converted in T1/T2)
```

The WPF folder `%LOCALAPPDATA%\AIONE\` is **only read**. `library::scan_library` is the one rule for
games, hidden games, favorites and library folders (the hidden / favorites / folders format is
a hypothesis until confirmed from the user's data; see the module's comment). The copy fingerprints it before and after (paths, sizes, times) and reports whether it stayed unchanged. `library::scan_games_dir` is the single rule for what counts as a game; the loader in T1 must use it so copy counts equal loaded counts (L-17).

## Stored files (B-6: data from earlier versions is always read)

Every file AIONE 4 writes must stay readable by every later version.

**1. Fields.** New fields are **optional with a default** (`#[serde(default)]`), never required.
A change that can't be expressed that way raises the file's `schema` number, and the reader
migrates older schemas step by step. A file from a **newer** version is read for what this
version understands and never overwritten (A9).

**2. Unreadable files.** An unreadable file is **backed up next to itself**
(`<name>.unreadable-<time>`, once per distinct content) and **reported**: a WARN line in the
log, and a plain notice to the user in the app. It is **never** treated as "nothing stored".

**3. Fixtures.** Each released version's files are kept as **fixtures** in `tests/fixtures/`, with
personal paths replaced, and unit tests read them. Fixtures are never edited to make a test pass.

**Today: the copy record `migration.json`.** Only `decision` is required. Its fixtures are
`tests/fixtures/migration/alpha1.migration.json` and `alpha2.migration.json`. If
`Imported\AIONE-WPF\` holds a finished copy, the question is never asked again; when the
record is missing or unreadable, a new one is written.

**Coming:**
- **The library (T1):** `game.json` stays the WPF format (A9). AIONE 4's own additions go
  into new optional fields (or a separate file per game) with defaults. The loader skips and
  logs an unreadable `game.json`, and never rewrites it. The first released library files
  become fixtures.
- **Settings and profiles (T2):** one file with a `schema` number. Every setting has a default,
  so a missing setting is filled and a removed one is ignored. Migrations run per schema step
  after a backup (A9: the last 5). An unreadable settings file is backed up and reported, and
  the defaults are used without overwriting the bad file until the user is told. The first
  released settings file becomes a fixture.

## Assets and tools

- `assets/`: bundled third-party assets with their licences (A3; `assets/README.md`); not used
  by the UI before T1.
- `tools/sound/`: Python scripts that measure the menu-sound pack (`analyse.py`) and build the
  sound proposals: the startup variants of alpha.2 (`make_startup.py`) and the 5 s Boot Up variants
  (`make_bootup.py`; `docs/sound/BOOTUP_SOUNDS.md`).
- `tools/brand/`: the logo proposals (`make_logos.py`) and the Boot Up preview
  (`make_preview.py`; `docs/brand/LOGO_PROPOSALS.md`).
- All of these are developer tools only, not part of the app or CI. They need Python with numpy, scipy,
  pyloudnorm 0.1.1, fonttools 4.59.0 and matplotlib, plus ffmpeg.

## Build and test

- `npm run check:version | check:assets | typecheck | lint | test | build`, `npm run test:ui` (Playwright against `vite build --mode mock`).
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` in `src-tauri/`.
- CI (`.github/workflows/ci.yml`, Windows): all of the above, `tauri build --no-bundle`, the portable zip, the pre-release.
