# AIONE 4 — Architecture

Current as of **4.0.0-alpha.8 (T1.2, part 2)**. Rules: Master Prompt Part A (A3 technology, A4 input, A5 window and lifecycle, A9 data).

## Overview

```
┌──────────────────────────── AIONE4.exe (Tauri 2) ─────────────────────────────┐
│ Rust core (src-tauri/)                        │ TypeScript UI (src/, Svelte 5)  │
│                                               │                                 │
│ lifecycle.rs  args, restart, autostart marker │ App.svelte   input owner, shell │
│ paths.rs      %LOCALAPPDATA%\AIONE4, \AIONE   │ lib/core     Core (tauri, mock) │
│ logging.rs    aione.log, lifecycle.log        │ lib/home     Home + pure model  │
│ foreground.rs foreground, guard, closing      │ lib/frame    header, footer,    │
│ input/        XInput thread, B-7 gate (gate)  │              glyphs, marquee    │
│ library.rs    what counts as a game (scan)    │ lib/input    events, keys, pads │
│ games.rs      conversion, loading, media      │
│ tiles.rs      tile and icon copies (D-007)    │                                 │ lib/audio    menu sounds        │
│ status.rs     battery and network             │ lib/perf     perf log           │
│ migration.rs  one-time copy of WPF data       │ components/  copy dialog        │
│ lib.rs        commands, media protocol, setup │ app.css      design tokens      │
└───────────────────────────────────────────────┴─────────────────────────────────┘
          commands (UI → core)  ·  events (core → UI)  ·  aione-media (images)
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
| `library_load` | convert the WPF copy once if needed, load `Games\`, log the L-17 lines; returns the games, skipped entries, notes and the copy's count |
| `status_now` | battery and network right now (then `aione://status` events) |

**Media protocol:** `http://aione-media.localhost/<game id>/<slot>` serves one image slot
(`Thumbnail`, `Background`, `Hero`, `Icon`, `Portrait`) from that game's own folder in `Games\`.
Anything else (other slots, `..`, absolute paths, unknown games) is a 404. CSP `img-src` allows it.

## Events (core → UI)

| Event | Payload |
|---|---|
| `aione://pad` | `{ kind: down/up/repeat/connected/disconnected, control, pad (0–3), seq }` |
| `aione://copy-progress` | `{ filesDone, filesTotal, bytesDone, bytesTotal }` (about every 100 ms) |
| `aione://copy-done` | the copy result (outcome, counts, source unchanged, notes); the UI reloads the library after it |
| `aione://status` | `{ network: wired/wifi/offline/unknown, battery: { percent, charging } or null }`, only on change |

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

## T1.1 additions

**Input path (A4, A6, B-7).** The XInput thread asks `foreground::aione_is_foreground()` for
every pad event; `input/gate.rs` decides: in front, everything goes to the UI; in the background,
only Guide (which also brings AIONE to the front), and everything else is logged "(AIONE in the
background: logged, not acted on)". A hold that began in the background never acts. In the UI,
`App.svelte` is the one owner of input: each event becomes one action for the active context
(the copy dialog when open, else Home). Keyboard repeats come from `KeyRepeater` (only the
arrows repeat); key releases are logged with their hold time.

**Home.** `lib/home/model.ts` is the pure state (focus, the first tile in view, the mouse's
selected and hovered tiles); `Home.svelte` draws it. The row moves by a CSS transform from the
first-in-view index (no measuring). The title's marquee measures overflow once per title.

**Library (B-6).** `games.rs` converts `Imported\AIONE-WPF\Games\` once into `Games\`
(staged in `Games.converting\`, renamed at the end), adds `AioneSchema: 1` to each `game.json`
and keeps every other field, and writes `library.json` (`schema`, `origin`, `convertedFrom`,
`at`, `appVersion`, `games`, `skipped`; all optional). If `Games\` exists, it never converts
again. Unreadable files are backed up once (`*.unreadable-<time>`) and reported. A newer schema is
read, never rewritten. Fixtures: `tests/fixtures/library/`.

**Sounds.** `lib/audio/sounds.ts`: one AudioContext, three decoded buffers, gain stages
Volume × Mute × menu × play. WebView2 runs with `--autoplay-policy=no-user-gesture-required`, so a
pad-only user hears sounds (pad input is not a browser gesture).

**Timers and loops (A7).** UI: the header clock (one timeout to the next minute), the key
repeater (only while an arrow is held), the press feedback (90 ms), the perf log (10 s, with 5 s
of frame sampling). Core: XInput polling (8 ms with a pad, 250 ms without), the status check
(30 s). Animations: the ring blink (focused tile), the marquee (long titles only).

**Lifecycle.** `--from-autostart` marks sign-in starts. Exit (close requested or exit requested)
calls `foreground::closing`, which stops the guard first (T0 QA-5). `--interface-scale=N` sets
`--scale` for one run.

## 4.0.0-alpha.5 additions

**Pictures (D-007, B-8).** `tiles.rs` makes, once, a 640 × 640 JPEG of each tile picture and a
≤ 128 px PNG of each icon in `<data>\Cache\Tiles\`, keyed by the source's size and time (a
changed picture is made again; old copies removed). `library_load` makes them (logged
"tile pictures: N made, M reused, F failed, in X ms; made from: … W×H"). Each game carries `tile`
and `icon` keys; the media protocol serves `/<id>/tile` and `/<id>/icon` (with `?v=<key>`).
Picture slots holding videos are skipped. The UI decodes every tile and icon before Home takes
input (logged "tile pictures ready: N decoded ahead in X ms").

**Timing evidence (D-007, D-008).** Pad events carry `atMs` (when the core read them); each pad
input line logs "core → UI N ms"; the perf line adds the longest frame, frames over 33 ms,
"JANKY" and the longest input delay. Each long task gets a line with the last action and the
pictures loaded or decoded around it.

**Input (B-10).** The first pad or key press after mouse use only switches back to pad mode and
shows the ring ("ring back on tile N (after mouse)").

## 4.0.0-alpha.6 changes

**Pictures (B-13, D-010, D-012).** `games::load` no longer makes copies: it marks a game's `tile`
key only when a copy already exists. `library_load` returns at once and starts the thread
`aione-tiles`, which runs `games::make_tiles` (4 workers) over `games::tile_jobs`; each finished
copy is sent as `aione://tile { id, tile }` and logged
(`tile copy: <game> from <file> (W×H, <what was read>; fill|fit) in N ms (decode, scale, encode,
write)`), then a summary line. `tiles.rs` reads ICO files itself, fills (icon) or fits (other
pictures over a blurred, darkened copy), and records failures in `Cache\Tiles\failed.json`.

**Sounds (D-009).** `vite.config.ts` never inlines audio (`assetsInlineLimit`); `sounds.ts` loads
each sound separately. CI checks the real build: `npm run check:built` and
`tests/ui/real-build.spec.ts` (the real `dist/` served with the app's CSP).

**Window (D-010).** `backgroundColor` and the page background are black until Home is drawn;
App renders Home only once the library is loaded.

**Input (B-10 rev. k).** `countsAsHomeInput` decides which inputs end mouse mode.

## 4.0.0-alpha.7: the lifecycle (A9)

**Phases (App.svelte):** `black` → `boot` → `home`, and `bootdown` from any phase.
- **black:** the window is shown on its first (black) frame. The black hold runs the work that
  would make the Boot Up stutter: `boot_media`, `library_load` and the tile pictures' decoding,
  the menu sounds and the fonts (icons stay hidden until Material Symbols is loaded, never
  shown as their ligature names), bounded at 4 s (logged `boot: black hold N ms`).
- **boot:** `BootPresenter` (src/lib/boot/presenter.ts) plays one presentation, decided once
  and logged: the custom video (`http://aione-media.localhost/_boot/up`, AIONE 4's copy) with
  its own sound, or Window B (`windowB.ts`, the reference's `fu`/`fd` ported) with
  `aione_boot_up.wav` decoded into its own AudioContext. Window B frames are drawn at the heard
  position (sound clock minus output latency); the first frame at or after 2550 ms is logged.
  A custom video that doesn't start within 4 s (counted only while in front) falls back once to
  Window B; each presentation's timers act only while it is current. Window blur / focus pause
  and resume it (A9, L-02).
- **Input lock:** while the phase isn't `home`, every input is discarded and logged, except the
  Boot Up's skip (A, B, Enter, Esc, a click).
- **bootdown:** the core turns the window's close (Alt+F4) into `aione://close-requested`
  (`prevent_close`, the foreground guard stopped first: T0 QA-5); the UI plays the custom
  `suspend.mp4` or Window B's Boot Down, holds black until its sound ends (2.6 s), then calls
  `close_now`. An 8 s safety exit in the core.

**Boot media (boot.rs):** `Imported\AIONE-WPF\Assets\Splash\splash.*` and
`Assets\Suspend\suspend.*` copied once into `<data>\Boot\boot-up-<key>.<ext>` /
`boot-down-<key>.<ext>` (key: size and date; a changed source is copied again and the old copy
removed); `/_boot/up|down` served with HTTP ranges (4 MB at most per answer). Only `ShowSplash`,
`SplashAudio`, `SplashVideoPlayToEnd` and `SplashDurationMs` are read from the WPF settings.

**Preview:** `npm run build:boot-preview` builds `dist-boot-preview/` (the same `windowB.ts`, opens
from the file system) for comparison with the reference page.

## 4.0.0-alpha.8

**Home's focus (one active context):** `HomeState.zone` is `row` or `drawer`; `moveZone`,
`moveDrawer`, `hoverDrawer` (model.ts, unit-tested). App routes up / down / left / right / A by
zone; the drawer's buttons call `openDrawer` (toast, sound, log, press feedback).

**Boot timing:** `BootPresenter` draws Window B for the predicted next refresh from the audio
output timestamp (D-015), records each slow frame with its time (D-013), cancels its timers at
its end (D-016), and calls `onFade` at 4.65 s so App mounts Home under the black layer (D-014);
`<main data-phase>` says when Home is live.

**Tiles:** a copy's key includes the recipe (2: Lanczos3 for enlarging), so copies are remade
once after a recipe change.

## 4.0.0-alpha.10
**The input lock's exceptions (A9, alpha.9 QA-4):** during the black hold, Boot Up and Boot Down,
everything is discarded except: the Boot Up's skip (A, B, Enter, Esc, a click), and the lifecycle
commands Ctrl+Shift+R (restart; logged "restart (allowed during the input lock)") and Alt+F4 /
the window's close (the Boot Down).
**D-014:** Home is mounted, decoded and drawn under the black layer before the Boot Up starts.
**Releases (A11, alpha.9 QA-5):** the release job compares the whole push (before..after); with no
file outside `.github/`, `docs/`, `tests/`, `tools/`, `scripts/ci-run.mjs` or top-level Markdown
changed, it publishes nothing.

## Launching games (T1.3, `src-tauri/src/launch.rs`)
- **What is started:** `Shortcut` (`game.lnk` / `game.url`) through `ShellExecuteExW` (like a
  double-click), else `SteamAppId` as `steam://rungameid/<id>`.
- **Tracking:** one running game; a watcher thread finds its window (the started process's first
  visible top-level window, or the first new program in front for launchers and Steam), logs the
  time to it, and reports the exit (`aione://game-exited`; the core brings AIONE to the front).
- **Commands:** `launch_game`, `return_to_game`, `running_game`.
- **Logs:** `aione.log` rotates at 5 MB; one old file (`aione.1.log`) is kept; the new file's first
  line says so.

## Handing the front to a game (alpha.17, D-027)
`launch::hand_over` (every return, and once when a launched game's window appears): the game's main
window is found again by process (largest visible top-level window, else a minimized one); its state
is logged; its active pop-up is targeted; `foreground::bring_to_front` tries alpha.15's methods
(direct, attach-thread-input, an input event, a topmost pulse; a minimized window is restored). If
Windows refuses, AIONE steps aside: the game's window is put on top (not activated) and AIONE's
window minimized, so Windows activates the game. The next Guide restores AIONE (`show_and_focus`
unminimizes it). A background Guide press is consumed in the input gate (D-028).

## Pausing the game while AIONE is in front (alpha.20, B-20; `src-tauri/src/pause.rs`)
- **Mechanism:** every thread of the game's process and of the processes it started is suspended
  with `SuspendThread` (found with `CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD)`) and resumed with
  `ResumeThread`. The undocumented `NtSuspendProcess` isn't used.
- **When (rev. al, D-039):** only **after** the foreground step that Guide started has succeeded
  (AIONE visible and active): Guide requests the pause, `show_and_focus` performs it; a 1 s watchdog
  drops the request (and resumes) if AIONE isn't confirmed in front; the foreground step never
  attaches its input to a paused process's thread. (Rev. ah's pause-at-once is superseded.) The game is
  only ever paused **while AIONE itself is in front** (rev. aj, D-035).
- **Resumed (rev. aj):** on A Continue, Guide back, before any close, when AIONE closes, and **the
  moment the user starts leaving AIONE** (a paused program can't answer Windows, so it can never
  "get the front" itself; alpha.22's Alt-Tab froze everything). Two WinEvent hooks on their own
  thread for AIONE's lifetime (out of context, skipping AIONE's own windows):
  - `EVENT_SYSTEM_FOREGROUND`: any other window took the front (a program, the taskbar, Start, the
    Game Bar, Steam) → `resume (AIONE lost the front: <program>)`;
  - `EVENT_SYSTEM_SWITCHSTART`: Alt-Tab / the task switcher started, before a window is chosen →
    `resume (Alt-Tab started)`.
- **Never:** Steam's client / helper, Explorer, the Game Bar, AIONE; games with an anti-cheat in
  their folder or running (Easy Anti-Cheat, BattlEye, Vanguard); `"NoPause": true` in game.json.
- **Safety:** the paused processes are recorded in `paused.json` in the data folder; the next start
  resumes any left paused.
- **Known risks:** a brief audio repeat or click at the pause or resume; a game measuring real time
  may jump forward on resume; online games may time out (mark them `"NoPause": true`).

## Input-locking states: one owner (alpha.27, B-22; `src-tauri/src/transit.rs`)

One state machine in the core owns the current input-locking state and **the set of held buttons
and keys** (every pad press and release, Guide included, recorded where the core reads the pad; the
UI reports keyboard keys, the only place they arrive). The UI mirrors the state it shows into the
core from one `$effect` (`lock_enter` / `lock_leave`) and asks the core what is held (`held_now`);
it keeps no copy. A core thread checks every 200 ms and ends any state past its limit
(`timed out`), telling the UI (`aione://lock-expired`). Every start and end is logged
(`lock: <state> started (…)` / `ended: <how> after N ms`).

| State | Limit (as built) | Escape | Ends | Guide in the core |
|---|---|---|---|---|
| Boot Up / Boot Down | 10 s (the animations' own skip and caps first) | skip (Boot Up) | done | ignored |
| Launching a game | 46 s (the UI says "Still starting <game>…" at 45 s) | **B Cancel start** after 3 s, **only while nothing is showing** (rev. ar: no window of the game, its launcher or Steam; otherwise `B ignored (…)`): the start is cancelled for real; the core keeps watching to the limit and **closes** a game appearing anyway (rev. ao, D-046) | the game's window; the start failed; B; timed out | ignored |
| Launcher wait | 6 s (the core's own wait is 5 s); **with Steam's window in front, the launch's 46 s** (rev. aq, D-048: "Steam is asking something") | **none** (rev. ao, D-046) | the game followed; none (AIONE never takes the front from Steam's window) | ignored |
| **The game still loading** (rev. ap) | from the game's window appearing until it has stayed put for 3 s (the same window, size and position, in front; each change restarts the 3 s), capped at 20 s (21 s in the machine) | none needed | settled; the 20 s cap | ignored (`Guide ignored (the game is still loading)`) |
| Going back to the game (D-040) | 3 s waiting for the release, then **resumed anyway** (`still held: …`) | B: cancel, stay on Home (the game stays paused) | resumed; B | ignored |
| Closing the game | 7 s (3 s for the game, then AIONE ends it, then up to 3 s) | none needed | ended | ignored |
| A dialog / toast | none (answered) | B / Cancel | answered | acts |
| **The Quick Menu** (T2.2) | none (answered) | B (a level back), a short Guide (closes it) | closed | a short Guide closes it; a held Guide does nothing more |

**Steam asking something during a start (rev. aq, D-048):** during the launcher wait, Steam's own
window (`steamwebhelper.exe` / `steam.exe`, any user's Steam) in front means Steam is asking something
(e.g. its "Launch game with custom arguments" dialog): the start isn't ended, the wait extends to the
start's 45 s limit (`Steam is showing a window during the start: waiting`), a game appearing after
it is followed, and a start that ends anyway leaves AIONE **behind** Steam's window (`AIONE stays
behind (Home ready)`). B Cancel start is offered only while nothing is showing (`start_showing`).

**The held set counts what matters (rev. ao, D-045):** the pad's buttons, and only the keys that
act in AIONE (arrows, Enter, Space, Esc, Backspace, Tab), each press **and release** reported; never
media keys, Alt or other modifiers. The keyboard part is cleared whenever AIONE gains or loses the
front (a release can't be seen from another window).

**AIONE's only synthetic input (rev. ap, D-047):** the `input-event` foreground method sends one
zero-distance mouse move (`SendInput`, `INPUT_MOUSE`, `MOUSEEVENTF_MOVE`), logged
`foreground: synthetic input sent (…)`; never a key. A read-only diagnostic (`keywatch.rs`, a
low-level keyboard hook that only observes) logs every media / volume key press with whether Windows
marks it as synthesized by a program, and which program was in front.

**Cleared** (logged `cleared (…)`) when the game ends, a new launch starts, or AIONE pauses a game
again. **Never two foreground attempts at once** (D-043): `show_and_focus` drops an attempt while
one runs (logged).

**Tests:** `transit::tests::button_mashing_never_leaves_a_lock_behind` (8 seeds, about 1,600
random presses / releases each, over every state) and `tests/ui/mash.spec.ts` (3 seeds, the UI with
the mock's mirror of the machine: a launch, a return, A / Guide spam, a close, the launcher wait,
and the game's end during a lock).

## The settings store (alpha.30, B-24; `src-tauri/src/settings.rs`)

AIONE 4's **own** schema (version 1, B-6), never a mirror of WPF's `Config`: `<data>\settings.json`,
camelCase keys. Every key has a default; an unreadable file or an invalid value falls back to it.
Changes are saved at once (`change_settings`, written to a temporary file and moved into place) and
survive restarts. The screens are built in T3 (B-25: **Settings** and **Theme Settings** are two
separate screens).

| Setting | Type | Default | Applies | Screen (T3) |
|---|---|---|---|---|
| `volume` | 0–100 % | 100 | live (AIONE's own gain; never Windows') | Settings › Audio |
| `muted` | bool | false | live | Settings › Audio |
| `musicVolume` | 0–100 % (relative) | 100 | live (music, when built) | Settings › Audio |
| `menuSoundVolume` | 0–100 % (relative) | 100 | live | Settings › Audio |
| `menuSounds` | bool | true | live | Settings › Audio |
| `sortOrder` | "recent" | "recent" | live | Settings › Library |
| `favoritesFirst` | bool | true | live (when favorites are shown) | Settings › Library |
| `showHiddenGames` | bool | false | live (when hiding is built) | Settings › Library |
| `clock24h` | bool | false | live | Settings › System |
| `startWithWindows` | bool | false | at the next sign-in | Settings › System |
| `swapConfirmBack` | bool | false | live (when built) | Settings › Controls |
| `bootUp` | bool | true | the next start | Settings › System |
| `bootUpAudio` | bool | true | the next start | Settings › System |
| `theme` | "switch" | "switch" | live | **Theme Settings** |
| `colorPreset` | "light" / "dark" | "light" | live | **Theme Settings** |
| `importedFromWpf` | record | none | (bookkeeping: the one-time import) | — |

**The one-time import from WPF AIONE:** read from the copy T0 made
(`<data>\Imported\AIONE-WPF\Config\settings.json`), never WPF's own folder; it runs once (after the
copy exists; recorded in `importedFromWpf`) and later changes in AIONE 4 always win. Only values that
clearly map and are valid are taken; everything else is left out and AIONE 4's default used.
`<data>\settings-import.md` lists every value taken (from what, to what) and left out (and why).
**WPF's `Config` format, as found:** `settings.json` (UTF-8, `SettingsVersion` 5): one flat object of
global keys (audio as 0–1 fractions, display, header, controller, library, launch, power) plus
`Theme` and `ThemeProfiles.<Theme>.*` (palette, accent, tiles, dimming, fonts, logo); beside it
WPF's own bookkeeping (`migration.json`, `data-folder.json`, `session-journal.json`, `typing.json`),
none of which maps to an AIONE 4 setting.

## The audio model (alpha.30; reference A10, decided after 3.9.13.2)

AIONE's Volume, Mute and menu-sound level are **its own**: they set AIONE's gain stages
(`MenuSounds.setLevels`) and **never** read or write Windows' volume or mute, which apply on top as
for any app. **The live audio environment (L-07):** a change of Windows' output devices
(`devicechange`) or of the output context's state (sleep, a device switch) is logged and the context
resumed, so every player keeps working.

## Games that run as administrator (alpha.30, B-27)

Measured, never assumed. At every launch, and again for the game after a launcher: `admin: "X":
AIONE elevated …; the game's process (…) elevated …; it asks for administrator rights: manifest …;
compatibility setting …` (token elevation via `OpenProcessToken` / `GetTokenInformation`; the
manifest's `requestedExecutionLevel`; `AppCompatFlags\Layers` `RUNASADMIN`, this user and all users),
with the guide line to untick "Run as administrator" when it asks. **Windows' permission prompt**
(`consent.exe`) during a start: said on the black screen, the start not ended, the wait extended,
no B. **Refusals:** a pause or a close Windows refuses is logged with its error and said plainly
("<game> runs as administrator: AIONE can't pause it" / "… close it"); a refused close offers to
bring the game to the front. Nothing hangs or retries in a loop. The administrator helper is T2.3.

## The held Guide and the Quick Menu (alpha.31, B-26; `transit::GuideHold`, `QuickMenu.svelte`)

Guide is decided **in the core** (it sees every press and release): its press waits for its release
or 0.5 s. **Released before 0.5 s:** today's behaviour, at the release (from the background: AIONE
comes forward; in front: the UI gets Guide's down and up). **Held 0.5 s:** `Guide held N ms: Quick
Menu`, from the background after bringing AIONE forward (the game pauses, B-20, D-039). Ignored
during transitions and while a game is still loading (B-22). The Quick Menu slides in from the
right, flush with the header and footer (measured), the whole screen (header included) dimmed (no
band, L-13), Home suspended under it (no ring, L-18). It owns input while open (one interaction
context, above Home and below a dialog). Rows: **Volume** (AIONE's own: left / right 1 %, faster when
held; LT / RT 10 %; A mutes; applied live to the menu sounds and Boot Up / Down, saved to the store),
**Theme** ("Coming in a later stage": only the Switch theme so far), **Power** (a submenu). Its own
footer prompts. Home's footer shows **"Menu"** (the Guide glyph) first.

**The Power menu** (the reference's order): Restart AIONE (Boot Down, then restart), Close AIONE
(Boot Down, then exit), Sleep Mode ("Coming in a later stage": Windows' sleep and resume are T2.3),
Restart PC and Turn Off PC (each confirmed, Cancel focused; then Windows' own `shutdown /r` / `/s`).

## Dialogs done once (alpha.31, L-13, L-18; `ConfirmDialog.svelte`)

One component for every question and message: two buttons (a question; Cancel focused for
destructive choices) or one (a message: OK). B is Cancel / OK; Left / Right move; A answers; the
buttons are clickable. Its scrim covers the whole screen (no band at the top, L-13), and Home is
suspended under it (no focus visuals, L-18). Every question and message uses it: the close and
switch questions, the Power confirmations, start failures, the administrator messages. *Not moved:*
T0's copy dialog, a multi-step screen (offer, progress, result) with its own verified flow.

## The Quick Menu, rev. ax (alpha.32; the user's Switch Quick Settings photo)

**Look:** a full-height panel from the right edge, 40 % wide, in the preset's colours (light panel
on light, dark on dark); the same status header (time, Wi-Fi, battery, controller) on its top and
the same footer ("B Back · A OK", with the divider) at its bottom, at Home's positions, so both flow
into it; the rest of the screen dimmed. Rows separated by thin lines, sliders with their icon, a
short explanation line under Volume, the B3 ring (Home's markup and flash) on the focused row.
**Rows, in order:** Power (opens Power options), **Brightness**, Volume (AIONE's own) with Mute,
Theme (its value; switching later).

**Brightness (`brightness.rs`, universal):** a built-in screen through Windows (WMI
`WmiMonitorBrightness` / `WmiSetBrightness`, via PowerShell); an external monitor through DDC/CI
(`Dxva2`); otherwise "Brightness can't be changed for this display". Probed once at start-up in the
background (logged); changes live, applied in the background (the last one wins), logged.

**Power options:** a short centered list, Switch's order (Sleep Mode, Restart PC, Turn Off PC,
Restart AIONE, Close AIONE), from the Quick Menu's Power row and the drawer's Power button.
Destructive choices ask first, Cancel focused. A B-22 dialog state while open.

## The keyboard as a full alternative (alpha.32, B-28)

**Win + Esc, everywhere** (a low-level keyboard hook, `keywatch.rs`): Esc with a Windows key held is
taken (never passed on) and decided by the same `transit::GuideHold` as the pad: short = a short
Guide (from the background: AIONE comes forward, the game pauses; in front: Guide), held 0.5 s = the
Quick Menu; ignored during transitions and while a game loads (B-22). One unassigned key (0xE8) is
sent so the Windows key's release doesn't open the Start menu (AIONE's synthetic input, logged,
D-047). **While Windows' Magnifier runs** (it closes with Win + Esc), the keys are left to Windows.
**Inside AIONE**, Home does the same (decided in the UI, through the same input path and locks). The
footer's "Menu" shows the Guide button or the Home key, by mode.

## Starts and closes, rev. ax (alpha.32)

- **D-051:** at 45 s with no window, the start becomes a **quiet watch** (up to about 3 min in the
  core): the tile shows "Starting…", A on it says "<game> is still starting", a late window is
  followed, another start ends it; it isn't the running game, so Continue / Close never act on it.
- **D-052:** the close sends WM_CLOSE to every visible top-level window of the game (a minimized one
  is visible to Windows); it counts the windows found and the messages Windows let through, and logs
  a refusal (UIPI: the game runs as administrator, AIONE doesn't), which is what "asked 0 windows"
  was on the user's PC.
- **The verified end:** after ending a process, `verified: the process has exited after N ms`; if it
  survives, B-27's message and "bring it to the front".

