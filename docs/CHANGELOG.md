# Changelog

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
