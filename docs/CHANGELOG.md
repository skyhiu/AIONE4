# Changelog

## 4.0.0-alpha.25 — completing T1.3 part 1 (rev. al)

- **D-039:** the pause only after the foreground step succeeded (AIONE visible and active): Guide
  requests it, show_and_focus performs it; a 1 s watchdog drops the request and resumes; the
  attach-thread-input step is skipped when the thread in front belongs to a paused process.
- **B-20 rev. al:** when a game ends while another program was in front (recorded each check), AIONE
  stays behind, restored, Home ready (`the game ended while <program> was in front: AIONE stays
  behind`).
- Eden: unverified (eden_cli.txt not attached).

## 4.0.0-alpha.24 — completing T1.3 part 1 (rev. ak)

- **D-038:** bringing AIONE to the front restores it first if it's minimized; "already in front"
  never counts while minimized (logged `minimized: true → restored` / `false`); a failed step-aside
  restores AIONE at once; after each attempt AIONE is checked and restored if still minimized.
- **D-037 (reopened):** the window-swap hand-back (`hand_back`) only for the game (counted, never a
  launcher), only to its visible main window, never with a step-aside; a launcher hiding its window
  leaves AIONE black for the launcher wait.
- Eden: unverified (eden_cli.txt not attached).

## 4.0.0-alpha.23 — completing T1.3 part 1 (rev. aj)

- **D-035 (B-20 rev. aj):** the paused game resumes the moment the user starts leaving AIONE: any
  other window taking the front (EVENT_SYSTEM_FOREGROUND) or Alt-Tab starting
  (EVENT_SYSTEM_SWITCHSTART). Paused only while AIONE itself is in front.
- **D-037:** within 5 s of the game appearing, AIONE getting the front without Guide (a game
  swapping its first window) hands the front back to the game instead of showing Home.
- **QA-2:** the cancel line is logged once (by the core).
- Eden: unverified (eden_cli.txt not attached).

## 4.0.0-alpha.22 — completing T1.3 part 1 (rev. ai)

- **D-036:** once a process is the game (counted when its window appeared, or followed after a
  launcher) or AIONE is closing it, its end is the game's end: Home at once, playtime recorded,
  never the launcher wait. The wait only for a launcher by name, or a process ending before any
  game appeared (`end_is_launcher`).
- **The cancelled wait (QA-2):** B ends the watch in the core (one end, logged once); the message
  says what was counted (and records it).
- **QA-3:** `--check-emulators` is a known start-up argument.
- Eden: unverified (eden_cli.txt not attached). B-21 (capture, overlay) is for T7.

## 4.0.0-alpha.21 — the last build of T1.3 part 1 (rev. ah)

- **B-18 rev. ah:** the launcher wait is 5 s (was 30 s; the real game came within 0.60–0.84 s on
  the user's PC); the screen shows "Starting <game>…  B Back to Home"; B cancels the wait at once
  (ended, nothing counted); the follow delay is logged.
- **D-035 / B-20 rev. ah:** a paused game is resumed the moment it (or a process it started) gets the
  front by any route: a WinEvent foreground hook for AIONE's lifetime.
- **B-20 rev. ah:** paused at once when Guide brings AIONE forward (no 0.3 s delay).
- **B-11 rev. ah:** "not counted: the game never appeared (launcher only)" logged as INFO.
- Eden: unverified (eden_cli.txt not attached).

## 4.0.0-alpha.20 — the game paused while in AIONE; force close; launcher-only sessions

- **B-20:** the running game is paused (SuspendThread on its threads and its children's) while
  AIONE is in front after Guide; resumed on Continue, Guide back, before any close, and when AIONE
  closes; never for anti-cheat games or `"NoPause": true`; a left-over pause is resumed at start.
- **B-18 rev. ag:** a game still running 10 s after a polite close: "Force close <game>?" (Cancel
  focused); only on confirmation its processes are ended (TerminateProcess); the session recorded.
- **B-11 rev. ag:** a start and playtime count only once the real game appears; launcher-only
  sessions and the 30 s wait never count (earlier entries untouched).
- **B-19:** Dolphin's Background Input checked (read only) and guided. Eden: unverified
  (eden_cli.txt not attached).

## 4.0.0-alpha.19 — launcher games, playtime on close, Windows' shortcut reader

- **D-032:** a process that ends within 30 s of the launch (or a launcher) isn't the game's end:
  AIONE stays black ("Starting <game>…", input locked) and follows the real game for up to 30 s
  (a child of the launcher, or a process from the install folder; Steam: appmanifest → installdir
  in every library); then tracks it and hands it the front. Never brought to the front when a
  launcher exits; no game in 30 s → ended.
- **D-033:** a game AIONE closes has its session recorded when the close is done:
  `"<game>" ended (closed by AIONE)` (INFO) and `played … min (total …)`.
- **D-034:** shortcuts read with Windows' own interface (IShellLinkW + IPersistFile, read only);
  the check report says why a shortcut couldn't be read and what to do.
- **Slips:** no hand-over once the game's process has ended; no doubled "back to"; guide lines only
  when needed.
- **B-19:** Azahar's `-f` (verified: the game path is taken as the last argument). Eden still
  unverified (neither its source nor yuzu's is reachable from the sandbox). Cemu's settings check
  not yet (its keys weren't found).

## 4.0.0-alpha.18 — Close and start, the hints, emulator profiles part 1

- **D-031:** "Close and start" starts the new game when the close is done, whichever event comes
  first; a late end event for an already-closed game is only logged.
- **B-17 (rev. ae):** the hint checks the program holding the keys (GameBar.exe) too.
- **B-18 (rev. ae):** no window during a close: "AIONE can't see <game>'s window; it may still be
  closing" (the core decides the hint and sends it to the UI).
- **B-19 part 1:** emulator profiles (data: `src-tauri/emulators.json`; sources in
  `docs/EMULATORS.md`): Dolphin, Cemu, DuckStation, PCSX2 verified from source; Azahar read, nothing
  added yet; Eden and 8 others recognized, nothing added (not verified yet). At launch, a game's
  `game.lnk` is read (never changed) and only the missing options are added, logged. A read-only
  Dolphin settings check and the one-time check report (`emulator-check.md`).

## 4.0.0-alpha.17 — getting back into games (rev. ac / ad)

- **D-028:** a Guide press that brings AIONE to the front is consumed (logged, never passed to
  the UI); only a new Guide press in front returns to the running game.
- **D-027:** the handover to the game: its main window found again by process each time (the
  largest visible top-level window, else a minimized one), its state logged (visible, minimized,
  top-level, class, owner, fullscreen), its active pop-up targeted; alpha.15's methods; if Windows
  refuses, AIONE steps aside (the game's window to the top, AIONE minimized). Also done once when
  the game's window first appears after a launch (Steam included).
- **D-029:** after asking a game to close: if it is still running 0.6 s later, it is brought to
  the front so its question can be answered (the hint shown); no window found: said at once.
- **D-030:** "Closing <game>…" while a close is pending; X and A on that game wait.
- **Log slips (QA-5):** each end logged once (the UI's raw-minutes line removed); B-17's hint
  once per Guide press (a consumed background press no longer reaches the UI's hint path).
- **B-19 (emulator profiles) moved to alpha.18.**

## 4.0.0-alpha.16 — T1.3 part 1 continued (rev. ab)

- **D-024:** a launch ends the start-up foreground guard at once (`foreground guard off (a game was
  started)`), so it never takes the front from the game.
- **B-18 / D-025, one game at a time (like Switch):** another game while one runs asks "Close X
  and start Y? Unsaved progress in X will be lost." (Cancel focused); Yes closes X politely
  (WM_CLOSE to its windows, never killed), waits for it to end (10 s at most, else says so and
  leaves it), then starts Y. The running game: first in the row, a "Playing" badge, the footer
  "≡ Options · X Close Software · A Continue"; X asks "Close X? Unsaved progress will be lost.".
  One watcher per launch (a generation); each end reported once.
- **D-026:** after the fade, black with input locked until the game's window appears (an early
  window waits for the fade), the start fails, or 45 s pass ("Still starting X…").
- **B-11 rev. ab:** playtime in WPF's fields: each start `LastPlayed`, `PlayCount` + 1,
  `FirstPlayed` if missing; each end `LastSessionMinutes` and `PlayTimeMinutes` += it; dates in
  WPF's style (local time with the offset, microseconds); UTC dates still read.
- **The launch:** the fade 0.3 s, eased; three launch sounds of 152–175 ms (A wired).
- **B-16, mouse and touch:** the row drags 1:1, glides to a tile (about 150 ms), stops at the
  ends; no ring while dragging; the drag tick as tiles pass; a pale tint on a pressed tile; the
  footer prompts clickable.

## 4.0.0-alpha.15 — T1.3 part 1: launching games

- **Launching (T1.3):** A, or a click on the selected game, starts it from its `game.json`: the
  `Shortcut` (`game.lnk` / `game.url` in its folder) is opened through the Windows shell (a
  `.lnk`'s own target, arguments and working folder apply); without one, `SteamAppId` as
  `steam://rungameid/<id>`. The launch sound at once; Home still 0.55 s (the press dip), then
  0.2 s evenly to black; input discarded meanwhile. Every step logged (how, the process, the time
  to the game's window). An already-running game is brought to the front, never started twice.
  A failed start: a plain message, Home again, logged with the reason.
- **Returning (B-7):** Guide or A on the running game brings it back; when the game ends, AIONE
  comes to the front, focus on that game. A launcher or Steam: the first new program in front
  is followed as the game.
- **Last played (B-11):** each start writes `LastPlayed` and `PlayCount` + 1 into the game's own
  `game.json` (every other field kept; a newer AIONE 4's file never rewritten). The row: played
  games most recent first, then never-played A–Z.
- **B-9 rev. aa:** the blink is a trapezoid (bright 0.25 s, fall 0.4 s, dim 0.15 s, rise 0.2 s).
- **The footer** 20 px higher at 100 % (the divider at y 972 at 1080 p), alpha.14's height at
  150 % (the drawer needs the room).
- **B-17:** a hint in the log when Steam or the Game Bar takes the front after Guide.
- **Log rotation:** one line in the new log; one old file (`aione.1.log`) is kept.
- **The launch sound:** a bright pop (snap + short glass, 67 ms).
- **B-16 (mouse / touch dragging)** moved to the next build.

## 4.0.0-alpha.14 — rev. y / z: the blink B3, the drawer and footer measured 1:1

- **B-9 (rev. y, B3):** the ring #D9006C at the base, #FF58AB at the peak (row and drawer, both
  presets), the same flash, crisp, the thin glow at the peak only.
- **B-14 (rev. z), the drawer (measured at rest):** discs 108 px with a soft drop shadow, pitch
  135; the focused button 117 px (108 %) with an 8 px ring directly on it (no band or edge), outer
  132 px, 15 px to each neighbour; the label's capitals 31 px, regular, 11 px below the ring.
- **B-14 (rev. y), the footer:** the glyphs' visible circle 37 px (D-023: sized from the ink); 16 px
  glyph to text, 63 px between prompts (measured ink to ink); text capitals 27 px, regular; a 2 px
  darker divider (#9A9A9A light, #6E6E6E dark).
- **D-023:** the size tests measure the footer glyph's ink.
- **Tests:** Rust 85; Vitest 58; Playwright 87 with 114 screenshots.

## 4.0.0-alpha.13 — rev. w / x

- **B-9 (rev. x, option B):** the ring #FD007E at the base, #FF46A2 at the peak, crisp; only a
  thin glow (4 px soft edge) at the peak, nothing at the base (alpha.12's wide halo removed); the
  drawer's focused button gets the same ring, white band, glow and pulse.
- **B-14 (rev. x):** tiles 386 px with 18 px between pictures (pitch 404, as before).
- **B-14 (rev. w):** the header's digits, "%" and icons about 31 px of visible ink ("AM" about
  20 px); the footer's glyphs 38 px, its text about 28 px. At the row's ends only a held repeat
  bumps; every fresh press wraps (the 400 ms threshold removed); the move landing on the first
  game or the last slot plays the click alone.
- **D-021:** a hold after a wrap starts its repeats at full level (the wrap now resets the hold).
- **D-022:** not a title defect: the "…nes" at the screen's edge was the plain tile "Quiet Engines"
  cut by the edge (proven); a UI test now checks the scrolling title's window over a full loop.
- **Side by side (A10):** 28 of 28 presses of `sound.mp4` match in their sound pattern.
- **Tests:** Rust 85; Vitest 58; Playwright 87.

## 4.0.0-alpha.12 — rev. u / v: side by side with the user's Switch recordings

- **B-14 (rev. u):** at the row's ends, a resting press (> 0.4 s after the last move) wraps at
  once (200 ms whip, the wrap sound); fast scrolling or a held repeat bumps the ring (14 px,
  overshoot 6, 150 ms), once per hold.
- **D-019:** the pad repeats only the newest held direction (the other resumes after 400 ms
  when it's released); the bump memory per direction.
- **B-1 (rev. u / v):** the click at about 4.2 kHz (C8; alpha.11's 5.27 kHz); the row-scroll tick
  (about 1.7 kHz, about 5 dB below the click) 65 ms after the click whenever the row scrolls; the
  wrap as the click plus a soft sweep about 11 dB below, 60 ms later (99 ms; alpha.11's 183 ms).
- **B-9 (rev. v), D-020:** the ring always brand pink #FF46A2, only slightly lighter (#FF69B4)
  at the peak; a soft halo (8 px blur) that swells and fades; outside in: halo, ring, a 2 px
  bright edge, a 4 px white band, the picture (the edge was inside the band in alpha.11).
- **B-8 (rev. v):** the title and the drawer's focused label in #FF46A2.
- **B-14 (rev. v):** the header about 2x (clock 47 px, icons 38 px, the battery percentage),
  centred at 105 px; the footer text 1.35x; drawer icons in palette harmonies, heavier; the ring's
  top at about 278 px; the drawer's discs centred at 818 px (measured side by side).
- **The Options toast (rev. u):** the Options button, B or Esc close it with the close sound; the
  timeout closes it silently; a replacement is logged.
- **Side by side (A10):** `sound.mp4`'s 28 presses replayed on AIONE (frames and sound); the
  comparison images in the handoff.
- **Tests:** Rust 85; Vitest 57; Playwright 83 with 114 screenshots.

## 4.0.0-alpha.11 — the last build of T1.2 (rev. s / t)

- **Release guard (alpha.10 QA-1):** a release only when app code changed, named explicitly in
  `scripts/release-guard.cjs`; a test replays `aed4477` (CI only) and expects no release.
- **B-9 (rev. s), the ring as on Switch:** a faint halo, the 7 px ring, a 1.5 px bright edge and a
  4 px white band (both presets). The pulse is a flash: base → peak (light #FD007E → #FFB0D7, dark
  #FF46A2 → #FFC2E0), starting at the peak on every move, fading over 0.75 s, rising in 0.2 s, once
  per 1.0 s. Measured luminance × 2.56 (light) and × 2.31 (dark); the base meets 3:1, the peak may
  not (the user's exception). `--ring=plain` removed.
- **B-14 (rev. s):** the row's ends bump the ring only (14 px, settled in 150 ms); held: once per
  hold; the first fresh press: a bump; the next: the wrap (200 ms, the wrap sound). Rev. r's
  whole-row bump and 300 ms rule removed.
- **B-1 (rev. s):** wrap (+3 dB, 77 ms, low tail), edge bump (30 ms, E7, −10 dB), Options open
  (F#7, the loudest) and close (C#7, low tail) on the Options toast, hold repeats 4 dB quieter after
  the first three; the drag tick made for B-16. Nine sounds.
- **B-15 (rev. t):** press (the whole tile or drawer button dips to 94 % and springs back, 0.18 s)
  and labelfade (fade + 4 px rise, 0.15 s) on by default with titlefade; `--extras=none` turns all
  off for one run.
- **B-16 (mouse and touch drag) moved to T1.3**, as allowed.
- **Fixed:** a long title's last 2 px clipped at 2560 px / 150 % (the canvas measurement ran short).
- **Tests:** Rust 84; Vitest 57; Playwright 83 with 114 screenshots.

## 4.0.0-alpha.10 — completes T1.2 (rev. r)

- **D-017:** the title clips sideways only (`overflow-x: clip`); descenders reach the font's full
  descent (measured 12.0 of 12.0 px at 100 %, 19 of 18 at 150 %), tapered; the baseline's gap above
  the ring scales with Interface Scale, so they stay clear of it.
- **D-018 / B-9:** a 2 px white separator between the ring and the picture (light preset against
  dark navy art: 2.27 → 11.44:1); `--ring=plain` shows the alpha.9 ring for one run.
- **D-014:** Home prepared under the black screen before the Boot Up; the skip logs how long Home
  had been ready.
- **B-14 (rev. r):** 12 slots; the wrap (a fresh press resting on an end); the whole-row bump
  (about 170 px, 150 ms) when holding or fast scrolling into an end; snappy ring bumps (full push in
  16 ms, settled by 180–200 ms); the focused drawer button at 108 %; B in the drawer returns to the
  game.
- **B-1 (rev. r):** no row ↔ drawer sound; entering a section plays its own move sound (6 sounds).
- **B-5 (rev. r):** the footer mark 46 px, one colour. **B-3 (rev. r):** Tab and the Menu key =
  Options ("Tab Options"). **B-15 (rev. r):** titlefade on by default.
- **alpha.9 QA-4 to QA-6:** restart during the input lock logged as allowed (A9's exceptions listed);
  CI publishes no release for a push without app code; "empty slot N" in the log.
- **Tests:** Rust 84; Vitest 54; Playwright 84 with 115 screenshots.

## 4.0.0-alpha.9 — the measured Switch Home (B-14, rev. p / q), the full sound set (B-1), B-15

- **B-14 (measured from the user's Switch recording):** tiles 398 px, 12 apart, the first 155 from
  the left, their top at 309; the ring 7 px directly around the art, jumping instantly; the row
  slides one tile in 170 ms (strong ease-out); the title centred on the tile (capitals about 37 px,
  baseline 21 px above the ring), long titles in a centred window (1.8 × the ring), holding 1.0 s,
  scrolling at 90 px/s and repeating; empty slots after the last game (no title, A does nothing,
  "A OK" dimmed); the drawer 110 px, 133 apart, the ring 10 px outside, icons in palette colours,
  labels about 25 px caps; footer "+ Options" / "A Start" (a game), "A OK" (drawer).
- **Edge bumps (rev. q):** a blocked press makes only the ring spring toward it and back (about
  15 px at the drawer's ends in 200 ms; about 12 px up / down in 300 ms), with the edge-bump sound;
  once per held press.
- **B-9 (rev. p):** the ring's pulse one cycle per 1.0 s, restarting at the bright end on every move.
- **B-1 (rev. q):** AIONE's sound set by function and place, one family in A major from FilmCow:
  row move (every repeat, no pacing), drawer move, row ↔ drawer, edge bump, confirm, back, error
  wired; launch, toggle on / off, open a destination, Home proposed; listening page with 28 sounds.
- **B-15:** proposed extras behind `--extras=press,titlefade,labelfade` (off by default).
- **Options:** the pad's Menu (≡) on a game shows "Coming in a later stage" (options come in T4).
- **Fixed while building:** a bump replayed on later moves; a per-move layout read in the title
  (a 62–65 ms long task in 1 of 3 runs; 0 of 5 after).

## 4.0.0-alpha.8 — T1.2 part 2: the drawer; D-013 to D-016; title, footer mark and letters (rev. o)

- **The drawer** (Switch-style, in the palette): six round buttons under the game row: Search,
  Activity, Surprise Me, Theme Settings, Settings, Power. Down enters it (on its remembered
  button), Up returns to the last tile; left / right stop at its ends; while it has the focus,
  no game title or tile ring shows (one active context). The focused button's ring pulses like
  the tiles'; its label is under it in the accent text colour. A or a click shows a plain
  "Coming in a later stage" toast (their screens come in T2–T4); the icon's squash-and-zoom on
  press. Proportions measured on the Switch reference (disc / tile 0.28); icons balanced to one
  ink weight (KD-81, ±10 % measured).
- **D-013:** each slow Boot Up frame logged with its place in the timeline.
- **D-014:** Home prepared under the black screen before it shows.
- **D-015:** frames drawn for the predicted next refresh, from the audio output timestamp.
- **D-016:** a presentation's timers cancelled at its end.
- **B-8 (rev. o):** no ▼: the title alone, like Switch.
- **B-5 (rev. o):** the footer mark two-tone (diamond in the accent), 64 px at 1080 p / 100 %;
  "AIONE" and the Boot Up letters in M PLUS Rounded 1c; the letters twice as large, the
  composition re-centred (timing unchanged); the Boot Up preview updated.
- **B-13 (rev. o):** small icons enlarged with Lanczos3 (every copy remade once).
- **Tests:** Rust 84; Vitest 55; Playwright 68 with 99 screenshots.

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
- **CI fix (same version, second attempt):** CI failed "Home at 1280x720, 150 %, dark" twice with
  the ▼ measured at 0.136× the capitals. **Cause (confirmed): a test bug.** The test counted only
  pixels with green < 100, which rejects the dark preset's own title colour #FF69B4 (G = 105), so
  in dark it measured only some anti-aliased edge pixels; how many qualify depends on the
  platform's text smoothing (Windows: 3 px at that size). The test now counts any pixel clearly
  different from the background, and logs what the old rule would have read. The first attempt's
  explanation (the icon font not yet loaded) was wrong; its change is kept as a safeguard: the
  fonts load in the black hold and icons stay hidden until their font is there.
- **Tests:** Rust 83; Vitest 54; Playwright 63 with 84 screenshots (boot key frames included).

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
