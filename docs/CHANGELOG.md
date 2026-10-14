# Changelog

## 4.0.0-alpha.7 — T1.2 (part 1): the Boot Up and Boot Down, the ring's pulse

- **Boot Up (A9, B-5 final):** black from the first frame; a black hold (at most 4 s) in which
  the library loads, the tile pictures are decoded and the boot media are prepared (alpha.6
  QA-2: nothing heavy runs during the animation); then one presentation: the user's custom
  video (copied once into `Boot\`) with its own sound, or the default **Window B** animation
  with its sound; one controlled fallback; then Home. The input lock discards input during the
  black hold, Boot Up and Boot Down; A / B / Enter / Esc / a click skip the Boot Up. Alt-Tab
  pauses and resumes the boot in sync (L-02).
- **Window B:** a line-for-line port of the design chat's reference animation, driven by the
  boot sound's own clock (the clamp's frame logged against 2550 ms); the diamond in #FF46A2.
- **Boot Down (Close AIONE):** Alt+F4 plays the custom `suspend.mp4` or Window B's Boot Down; the
  screen stays black until its sound ends; then AIONE closes (an 8 s safety exit in the core).
- **Custom boot media:** `Assets\Splash\splash.mp4` and `Assets\Suspend\suspend.mp4` copied
  once from the WPF copy into AIONE 4's `Boot` folder, remade when the source changes; only the
  four boot entries of the WPF settings are read. The media protocol serves video ranges.
- **`--boot=default`:** this run shows Window B even with a custom video.
- **The ring (B-9 rev. m):** pulses between two shades of the accent, no glow.
- **Tile copies (B-13 rev. m):** a changed source is remade and logged "remade".
- **Branding:** the footer shows the Window B mark + "AIONE"; the app icon is Window B; the
  delivered logo files, sounds and reference page are in `docs/brand/window-b/`.
- **Removed:** the 50 Menu Interface SFX pack and its credit (B-1 rev. i).
- **Moved to the next build:** the drawer (T1.2 item 5), as allowed.
- **Tests:** Rust 83; Vitest 54; Playwright 62 with 84 screenshots (boot key frames included).

## 4.0.0-alpha.6 — T1.1 fixes (D-009 to D-012, B-8 corrected, B-13)

- **D-009 (no menu sounds in the release):** cause confirmed: Vite inlined `move.wav` (3.6 KB)
  as a `data:` URL, which the app's CSP does not let `fetch()` reach; one failure stopped all
  three. Sounds are never inlined now; each loads on its own and a failure logs its address and
  reason; lines say "no sound (sounds unavailable)". New checks on the **real build**:
  `npm run check:built` (CI step) and `tests/ui/real-build.spec.ts` (fetch + decode under the
  real CSP; a `data:` sound must be blocked).
- **D-010 (slow first start, empty window):** Home is shown at once with the original pictures;
  missing tile copies are made in the background (4 threads) and swapped in; each copy logs its
  decode / scale / encode / write times; the window and page are black until Home is drawn. The
  picture libraries are built at full optimisation; the scaling filter is CatmullRom.
- **B-8 (corrected, rev. k / l):** the title sits above the selected tile, left-aligned with it,
  after a ▼ (Material Symbols Rounded `arrow_drop_down`) whose visible triangle is the capitals'
  height; it moves by transform, shifts left near the right margin, and long titles scroll.
  alpha.5's fixed-left title row and letter tile are removed.
- **B-13:** the tile picture is the game's icon (largest ICO size), filling the tile; otherwise
  another picture fitted whole over a blurred, darkened copy; otherwise a plain tile.
- **D-011:** the focus ring is solid and never fades; a soft glow pulses outside it.
- **D-012:** AIONE reads ICO files itself (an embedded PNG in any colour type, or the next size);
  a failed source is not retried until it changes.
- **B-10 (rev. k):** modifiers alone keep mouse mode. **QA-6:** the slot number in ring-back lines.
- **Tests:** Rust 79; Vitest 54; Playwright 43 with 71 screenshots.

## 4.0.0-alpha.5 — T1.1 fixes and changes

- **D-007 (Home freezes):** the core makes one 640 × 640 copy of each tile picture and one ≤ 128 px
  copy of each icon (`Cache\Tiles\`, remade only when the source changes); Home decodes all of
  them before it takes input, so scrolling never decodes. Every long task is logged with the last
  action and the pictures around it. UI test with 3000 × 3000 pictures.
- **D-008 (perf line):** adds the longest frame, frames over 33 ms, "JANKY" when any frame is over
  50 ms, and the longest input delay (core → UI). Each pad input line shows its own delay.
- **B-10:** after mouse use, the first pad or key press only brings the ring back (no move, no
  sound); logged "ring back on tile N (after mouse)".
- **B-8:** the selected game's title is always at the row's start, with the game's icon (rounded,
  about the cap height) or a pink letter tile; long titles scroll up to the right margin.
- **B-9:** hot pink accent (light: ring #FD007E, text #B5005A; dark: ring #FF46A2, text #FF69B4),
  hover, pressed and soft fills from the palette; every pair checked against A8 in the UI tests.
- **Menu sounds (B-1 rev. h / j):** a new family made from FilmCow recordings (move, select,
  back wired; launch, drawer, end of row, toggle, error proposed), tuned in A major; listening page
  delivered separately. The 50 Menu Interface SFX pack stays in the repository until T1.2.
- **B-11:** test fixtures use invented titles and values.
- Picture slots holding a video (`portrait.mp4`) are never used as tiles.
- **Tests:** Rust 77; Vitest 53; Playwright 39 with 43 screenshots.

## 4.0.0-alpha.4 — T1.1: the library and a still Home

- **Home (Switch-style, still):** status header (clock with a smaller AM / PM, controller,
  network, battery), the game row from the library with the focus ring and its blink, the
  selected game's title with a marquee for long titles, the footer with Kenney prompts and the
  "AIONE" branding. Controller, keyboard and mouse (hover only; first click selects, second click
  starts: L-06). Light (default) and dark presets. Empty and error states. Starting a game is
  logged only (launching comes in T1.3). No drawer or background video yet (T1.2).
- **Sizes follow the window and Interface Scale** (alpha.1 QA-6); the header at half rate; the
  tiles follow Tile Size. `--interface-scale=90..150` sets it for one run until Settings exist.
- **Library loader (B-6, L-17):** the copied WPF games are converted once into AIONE 4's own
  `Games\` (the WPF layout, plus `AioneSchema`), with a `library.json` record; the copy stays
  untouched. Logs `library loaded: N games …` and the L-17 check against the copy's count. Game art
  is served only from a game's own folder (`aione-media` protocol).
- **Menu sounds:** move #24, select #09, back #15 through AIONE's own gain stages; one sound per
  action; held directions paced to at most every 180 ms.
- **D-006 (B-3 rev. e):** only the arrow keys repeat; Enter, Space, Esc and modifiers never do;
  key releases are logged `input up <key> (keyboard), held N ms, R repeats`.
- **B-7:** in the background only Guide acts (it brings AIONE to the front); every other pad
  input is logged "(AIONE in the background: logged, not acted on)". The T0 readout is gone.
- **T0 QA-4:** the Start with Windows entry runs `"<exe>" --from-autostart`, logged at start.
- **T0 QA-5:** the foreground guard stops when exit begins.
- **Header status:** battery (laptops) and network (wired / Wi-Fi / offline) from the core,
  pushed on change (one 30 s check in the core).
- **Perf log (A7):** a `perf` line every 10 s and on each screen change.
- **WebView2 autoplay:** `--autoplay-policy=no-user-gesture-required`, so menu sounds play for
  pad-only use.
- **Docs:** `AIONE_DESIGN_SYSTEM.md` started in full; logo and Boot Up proposals
  (`docs/brand/LOGO_PROPOSALS.md`, tool `tools/brand/make_t11_bootup.py`).
- **Tests:** Rust 71 (was 50); Vitest 52; Playwright 31 with 24 screenshots.

## 4.0.0-alpha.3 — T0 fixes, logo and Boot Up sound proposals

- **D-004 (B-6):** the copy record reads from every earlier version. Fields are optional with
  defaults; an unreadable record is backed up and reported (log and a notice with "A OK"); a
  finished copy means the question never returns. alpha.1's and alpha.2's records are kept as
  test fixtures (`tests/fixtures/migration/`).
- **Restart order:** the start steps (`--autostart`, the restart wait, the "already running"
  check) are one tested function, `lifecycle::pre_start`. The wait comes before the check, as it
  already did in alpha.2 (confirmed from the code).
- **D-005:** the keyboard-focus line is judged by window ownership: "AIONE 4's web view: keys
  reach AIONE".
- **D-006:** AIONE times key repeats itself (400 / 90 ms) and ignores Windows' repeats;
  modifier keys never repeat.
- **D-001:** the "library hidden / favorites / folders" lines are always logged, including "none
  found, looked in: …", with `Config\` named as not yet read.
- **Copy dialogs (B-2 rev. d):** "Skipped: _Removed (not a game)"; result dialogs show "A OK",
  and B closes them; clicks are logged with the action they activated.
- **Tests:**
  - a log-capture test checks that every promised copy log line is written;
  - a timing race in three UI tests was fixed (they now wait for the dialog before acting).
- **Boot Up proposals (B-5, not used by the app):**
  - three logo designs (`tools/brand/make_logos.py`);
  - three 5 s Boot Up sounds from the pack (`tools/sound/make_bootup.py`, `docs/sound/BOOTUP_SOUNDS.md`);
  - an HTML preview of the timeline (`tools/brand/make_preview.py`).
- Version 4.0.0-alpha.3.

## 4.0.0-alpha.2 — T0 fixes and assets

- **D-002:** the UI asks the core which controller slots are connected at start; a pad
  connected before AIONE started shows at once. The mocked core now behaves like the real
  one (connect before presses; connected before the UI listens).
- **D-001:** the copy counts games, hidden games, favorites and library folders separately
  with one rule (`library::scan_library`); the log says where each count came from and lists
  the WPF data folder's top level.
- **D-003:** the copy progress bar and its numbers come from one value; the bar's slide is
  80 ms (was 200 ms, which made it trail the text).
- **Copy question (B-2):** B = Not now (asked again next start); "Don't copy" final with A or a
  click; hints "A Select · B Not now"; plain wording ("1 folder skipped: _Removed"); "favorites"
  spelled one way.
- **Logging:** "held N ms, R repeats" on each release; the UI logs each input it acts on with
  the core's sequence number; a second instance logs the hand-over; the keyboard-focus window
  after the foreground step (and 500 ms later) and the first key event time.
- **Assets** added to `assets/` (not used yet): Kenney Input Prompts (SVG), 50 Menu Interface
  SFX (WAV), Material Symbols Rounded (npm 0.47.5), fonts Nunito, Inter, Figtree, Quicksand,
  M PLUS Rounded 1c; licences next to them; `npm run check:assets` in CI.
- **Sound tools** (`tools/sound/`): measurements of the 50 sounds, a proposed mapping, and three
  startup variants made from the pack (`docs/sound/`).
- Version 4.0.0-alpha.2.


## 4.0.0-alpha.1 — T0: repository, CI, first window (2026-10-12)

- New project: Tauri 2.12 (Rust 1.91.1) + Svelte 5 + strict TypeScript + Vite 8.
- Fullscreen borderless window showing "AIONE 4" and the version on a plain dark background; shown after the first frame (no white flash).
- Single instance (a second start focuses the running one); restart waits for the old instance to exit (`--wait-for-pid`).
- Foreground at every start with logged fallbacks (direct, attach-thread-input, input-event, topmost-pulse); an 8 s start-up guard; never left topmost.
- `%LOCALAPPDATA%\AIONE4\` with `Logs\aione.log` and `Logs\lifecycle.log`.
- One-time offer to copy the WPF data folder (read only); counts with the loader's rule; source checked unchanged.
- Controller input (XInput, Guide included) read in the core; T0-only input readout for controller, keyboard and mouse.
- `--autostart=on|off` (temporary until the Settings toggle in T3).
- CI on Windows with pinned actions and tools; pre-release with the portable zip and UI screenshots.
