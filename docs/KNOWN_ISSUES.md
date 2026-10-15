# Known issues

Mirrors Master Prompt Part C with status (A12): Fixed and Verified · Implemented, Unverified ·
Diagnostics added · Open · Blocked. Only runtime evidence checked by QA or the maintainer makes
an item Fixed and Verified.

## D-defects

| ID | Severity | Description | Status (4.0.0-alpha.4) |
|---|---|---|---|
| D-001 | Low | Copy counts (L-17 / KD-101). | **Verified for T0** (alpha.3); `Config\` part open for T2. alpha.4: the real WPF files confirm the field names `Favorite` and `Hidden` (they were a hypothesis); the loader reads them. Folders: no field in the two files seen; still read as before. |
| D-002 | Low | A pad connected before start showed as not connected. | **Fixed and Verified** (alpha.3, PC). Home's controller icon uses the same query. |
| D-003 | Low | Copy bar against its text. | **Fixed and Verified** (alpha.3). |
| D-004 | Medium | An update asked the copy question again. | **Fixed and Verified** (alpha.3, PC). |
| D-005 | Low | Focus line wording. | **Fixed and Verified**, both wordings (alpha.3, PC). |
| D-007 | Medium | Home freezes for 0.1–0.5 s in quick pad use, then plays the waiting presses at once (alpha.4 QA-1). | **Implemented, Unverified.** Cause: **hypothesis** (not reproduced in the sandbox): large tile pictures (full-size `Thumbnail` files) decoded and scaled on the main thread as the row scrolls. Fix: tile-sized 640 × 640 copies made once by the core, and every tile and icon decoded before Home takes input. Diagnostics: each long task is logged with the last action, the focused tile and the pictures loaded or decoded around it (with pixel sizes). UI test with 3000 × 3000 pictures: 36 quick presses, longest core → UI 1 ms, no long task during navigation (sandbox, Chromium). Needs the user's quick pad session. |
| D-008 | Low | The perf line hides freezes (alpha.4 QA-2). | **Implemented, Unverified.** Each perf line adds the longest frame, frames over 33 ms, "JANKY" when a frame is over 50 ms, and the longest core → UI input delay; unit test shows a D-007-like freeze (p95 16.7 ms) as "longest 510 ms … JANKY". Note: frames are sampled 5 s of every 10 s; long tasks and input delays are recorded all the time. |
| D-017 | Medium | The title cut its descenders (alpha.9 QA-1). | **Implemented, Unverified.** Cause confirmed: the inner title box was 1 em tall and clipped, ending 0.14 em (7.1 px) below the baseline. Now clipped sideways only. UI test: "Hgjpqy" at 100 / 150 %, both presets, short and scrolling: the ink reaches the font's descent, tapered, clear of the ring. |
| D-018 | Low | The ring weak against dark art (alpha.9 QA-2). | **Implemented, Unverified** for the light preset (separator: 11.44:1 against dark navy, 3.36:1 against light grey, ring vs background 3.22 / 4.23). **Open question (Part B):** in the dark preset light art stays at 2.32:1 with or without the white separator (see the alpha.10 handoff §7). |
| D-013 | Low | Small hitches in the Window B Boot Up: 1–2 frames over 33 ms in 9 of 11 runs, once 69 ms before the clamp (alpha.7 QA-1). | **Diagnostics added (alpha.8).** Each slow frame is logged with its place in the timeline (`over 33 ms: N (at 2.48 s: 69.3 ms, …)`, up to 12 listed). The heavy work is out of the animation (QA-2 of alpha.6, D-014). Cause still unknown; the PC's lines will show where. A GPU-composited form only if the lines point there. |
| D-014 | Low | A 64–95 ms freeze as Home first appears, every start (alpha.7 QA-2); still after early skips (alpha.9 QA-3). | **Implemented, Unverified (alpha.10):** Home is now prepared before the Boot Up (the skip logs how long it had been ready). Earlier: Home is mounted under the black boot layer when Window B's fade to black begins (4.65 s), so its first frames (layout, the tile pictures) are drawn while black; after a custom video, two frames under black first. Logged `Home prepared under the black screen`. |
| D-015 | Low | The clamp once 19.3 ms late; the heard time moved in 8–11 ms steps (alpha.7 QA-3). | **Implemented, Unverified.** Each frame is drawn for the predicted next refresh; the sound position comes from the audio output timestamp, interpolated with the page clock (resolution about 0.01 ms; stated in the clamp line, with the fallback named when the browser lacks it). |
| D-016 | Low | After a skip, the skipped video's "after the first frames" line still appeared (alpha.7 QA-4). | **Implemented, Unverified.** Every timer of a presentation is cancelled at its end (skip, completion, fallback), and a per-presentation counter blocks any late callback. UI test: no line from the skipped presentation after its outcome. |
| D-009 | High | No menu sounds in the release (alpha.5 QA-1). | **Implemented, Unverified.** Cause **confirmed**: `move.wav` (3.6 KB, under Vite's 4 KB inline limit) was built into the JavaScript as a `data:` URL; the CSP's `connect-src` doesn't allow `data:`, so its fetch failed and `Promise.all` silenced all three. Fix: no inlined sounds; each loads separately; failures log the address and the reason; lines say "no sound (sounds unavailable)". Checks on the real build: `npm run check:built` (fails on alpha.5's config, passes now) and a Playwright test serving `dist/` with the real CSP (all 3 fetch and decode; a `data:` sound is blocked). |
| D-010 | Medium | First start after an update took 11.2 s with an empty grey / white window (alpha.5 QA-2). | **Implemented, Unverified.** Home no longer waits for copies (load 1 ms with 20 new pictures in a Rust test; Home in about 0.3 s in the UI test); copies in the background on 4 threads, swapped in. The window and page are black until Home is drawn. Time per copy: about 50 ms in a sandbox benchmark (release settings); the 0.5 s seen on the PC is about 10× that, cause still a **hypothesis** (Windows Defender scanning each new file is the leading candidate); each copy now logs decode / scale / encode / write. |
| D-011 | Low | The ring blinked below 3:1 (alpha.5 QA-3). | **Implemented, Unverified.** Solid ring, glow pulsing outside. UI test samples the ring at the glow's lowest and highest points: 3.22:1 light, 4.36:1 dark. |
| D-012 | Low | Zelda's `icon.ico` unreadable (alpha.5 QA-5). | **Implemented, Unverified.** AIONE's own ICO reader: largest entry first, embedded PNG in any colour type, BMP entries through the library, next size on failure. Failures recorded in `Cache\Tiles\failed.json` and skipped until the source changes. Rust tests: RGB and grey PNG entries, a damaged largest entry. |
| D-006 | Low | Key repeats and releases. | Timing **Fixed and Verified** (alpha.3, PC). Key list and release lines: **Implemented, Unverified** (alpha.4: only the arrows repeat; Enter, Space, Esc, modifiers never; `input up <key> (keyboard), held N ms, R repeats`; unit and UI tests pass). |

## T0 QA findings carried into T1.1

| Finding | Status (4.0.0-alpha.4) |
|---|---|
| QA-4 (T0 test): an autostart start is not marked | **Implemented, Unverified**: the entry runs `"<exe>" --from-autostart`; logged "started by Windows at sign-in". Unit test on the command. |
| QA-5 (T0 retest): closing logged as "the user switched away" | **Implemented, Unverified**: exit stops the guard first ("AIONE is closing"); later foreground changes are ignored. Unit test on the guard's decision. Windows code compiles first in CI. |

## T1.3 part 1, continued (4.0.0-alpha.16)

| Item | Status |
|---|---|
| D-024 (the guard vs the game) | Implemented, Unverified (the guard ends at the launch) |
| D-025 / B-18 (one game at a time, close question, badge, footer, polite close) | Implemented, Unverified (UI tests; the Windows close needs the PC) |
| D-026 (black until the window) | Implemented, Unverified (UI test with a window that never appears) |
| B-11 rev. ab (playtime in WPF's fields and date style) | Implemented, Unverified (Rust test) |
| B-16 (mouse and touch dragging, clickable prompts) | Implemented, Unverified (UI test with the mouse) |

## T1.3 part 1 (4.0.0-alpha.15)

| Item | Status |
|---|---|
| Launching, returning, the game's exit (Windows) | Implemented, Unverified: the Windows code compiles only in CI; the mocked flow is tested |
| Last played, the row's order | Implemented, Unverified (core test) |
| Blink trapezoid, footer, B-17 hint, log rotation line | Implemented, Unverified |
| B-16 mouse / touch dragging | Moved to the next build |

## Rev. y / z (4.0.0-alpha.14)

| Item | Status |
|---|---|
| B-9 rev. y (B3) | Implemented, Unverified |
| B-14 rev. z (the drawer at rest) | Implemented, Unverified (measured: outer 132, ring 8, 15 px to the neighbour, label 11 px below, capitals 31) |
| B-14 rev. y (the footer) | Implemented, Unverified (measured: circle 37, 16 / 63 px, capitals 27-28) |
| D-023 | Implemented, Unverified (the size tests measure the ink) |

## Rev. w / x (4.0.0-alpha.13)

| Item | Status |
|---|---|
| D-021 (a hold after a wrap starts quiet) | Implemented, Unverified: the cause confirmed (the wrap path didn't reset the hold); UI test |
| D-022 (a stray title piece) | Not a title defect (the plain tile "Quiet Engines" cut by the screen edge, proven); UI test over a full loop: no stray title pixels |
| B-9 rev. x (option B, thin glow; the drawer's ring pulses) | Implemented, Unverified |
| B-14 rev. w / x (header / footer ink, tiles 386 + 18, row ends, landing click) | Implemented, Unverified |

## Rev. u / v (4.0.0-alpha.12)

| Item | Status |
|---|---|
| D-019 (a held diagonal bumps on every repeat) | Implemented, Unverified: only the newest held direction repeats (core test); the memory per direction (UI test: 2 bumps in 32 interleaved events) |
| D-020 (the bright edge inside the white band) | Implemented, Unverified: the cause confirmed (the two inner shadows were stacked in the wrong order); UI test reads ring → edge → 4 px white |
| B-14 rev. u (resting press wraps; fast press bumps) | Implemented, Unverified |
| B-1 rev. u / v (click 4.2 kHz, scroll tick, wrap) | Implemented, Unverified (measured against `sound.mp4`) |
| B-9 / B-8 / B-14 rev. v (ring, title, header, footer, drawer) | Implemented, Unverified (side by side in the handoff) |
| Remaining differences vs Switch | Listed in the alpha.12 handoff §5 |

## Rev. s / t (4.0.0-alpha.11, the last build of T1.2)

| Item | Status |
|---|---|
| alpha.10 QA-1: the release guard (app code named) | Implemented, Unverified (unit test replays `aed4477`) |
| B-9 rev. s: the ring's halo, edge, 4 px white band, the flash | Implemented, Unverified (UI test: luminance × 2.56 / × 2.31; band 4 px; base ≥ 3:1) |
| B-14 rev. s: ring bumps at the row's ends; the second press wraps | Implemented, Unverified |
| B-1 rev. s: wrap, edge bump, Options open / close, quieter repeats | Implemented, Unverified (the user listens) |
| B-15 rev. t: press, labelfade, titlefade on; `--extras=none` | Implemented, Unverified |
| B-16: mouse and touch drag | **Moved to T1.3** |

## B-14 / B-1 / B-15 (4.0.0-alpha.9)

| Item | Status |
|---|---|
| B-14 measured Switch Home (row, ring, scroll, title, empty slots, drawer, footer) | Implemented, Unverified |
| B-14 edge bumps (rev. q) | Implemented, Unverified (UI test: ring moves, tile still, settled < 400 ms, once per hold) |
| B-9 rev. p: 1.0 s pulse from the bright end | Implemented, Unverified |
| B-1 rev. q: the sound set by function and place | Implemented, Unverified (7 wired; the user picks by ear) |
| B-15 extras (press, titlefade, labelfade) | Proposed, off by default (`--extras`) |

## T1.2 part 2 (4.0.0-alpha.8)

| Item | Status |
|---|---|
| The drawer (Switch-style, B-9 rev. o) | Implemented, Unverified (UI tests and 12 screenshots with the focus in the drawer) |
| B-8 rev. o: the title alone, no ▼ | Implemented, Unverified |
| B-5 rev. o: footer mark (two-tone, 64 px), M PLUS Rounded 1c, Boot Up letters ×2 | Implemented, Unverified |
| B-13 rev. o: small icons enlarged with Lanczos3 | Implemented, Unverified (Rust test; every copy remade once, logged) |
| A11 rev. o: annotations, screenshots on the release | Done in alpha.7 (`aed4477`; proof run #10 on a deleted throwaway branch) |

## T1.2 (4.0.0-alpha.7)

| Item | Status |
|---|---|
| B-9 rev. m: the ring pulses between two shades, no glow (D-011 stays met) | Implemented, Unverified (UI test: light 3.22 → 4.23:1, dark 4.36 → 5.20:1 at the two ends) |
| B-8 rev. m: the ▼ fixed; the title scrolls only to its right | Implemented, Unverified (UI test) |
| B-13 rev. m: a changed source remade and logged | Implemented, Unverified (Rust test) |
| alpha.6 QA-2: no heavy work during the Boot Up | Implemented, Unverified: the pictures are decoded in the black hold before the animation; in the UI test, 0 long tasks during the Boot Up (the first one comes when Home appears) |
| Window B Boot Up / Boot Down (B-5 final) | Implemented, Unverified: frames match the reference page (≤ 0.016 % of pixels differ at 12 moments); the clamp drawn on the first frame at or after 2550 ms of the heard sound (logged) |
| The custom Boot Up / Boot Down (A9, KD-01, KD-02, L-01) | Implemented, Unverified: copied once into `Boot\`; custom first, Window B as the one fallback; sound read back after the first frames |
| Input lock and skip (A9, L-03 partly) | Implemented, Unverified (UI tests) |
| L-02 (Alt-Tab during the boot video) | Implemented, Unverified (pause / resume at the same position in the UI test) |
| L-19 | Moved to the later stage that shows game background videos (user decision, rev. m answer) |
| The drawer | **Moved to the next build** (allowed) |

## T1.1 changes (rev. h)

| Item | Status (4.0.0-alpha.5) |
|---|---|
| B-8 (corrected, rev. k / l): title above the selected tile, ▼ before it | Implemented, Unverified (alpha.6; alpha.5's fixed-left row removed) |
| B-13 tile picture = the icon | Implemented, Unverified |
| B-9 hot pink accent | Implemented, Unverified (every pair measured in UI tests) |
| B-10 first press after the mouse brings the ring back only | **Verified (alpha.5, PC)**; rev. k (modifiers don't count): Implemented, Unverified |
| B-1 menu sounds from FilmCow | Implemented, Unverified (the user judges by ear) |
| B-11 invented fixtures | Done (no user titles in the repository) |

## L-lessons touched

| Lesson | Status | Notes |
|---|---|---|
| L-06 (click activates, no hover left, no single-click launch) | Implemented, Unverified | UI tests: first click selects, second starts; nothing highlighted after. Launching itself is T1.3. |
| L-09 (no unintended gradient / band) | Implemented, Unverified | UI tests: no background image anywhere on Home (art excepted), light and dark. |
| L-10 (header one optical size and weight) | Implemented, Unverified | Measured in the sandbox: every header item's ink 18 px at 1080 p (battery 19). UI test: one weight, AM / PM 60 %. |
| L-16 (no layout loops; marquee by transform) | Implemented, Unverified | alpha.5: the title marquee (B-8) also moves by transform only. |
| L-16 (alpha.4 note) | Implemented, Unverified | UI tests: the row and the marquee move by transform; every running animation changes only transform or opacity; overflow measured once per title. |
| L-17 (counts match what loads) | Implemented, Unverified | `library check (L-17): the copy counted 10 games; 10 loaded (match)`. Rust test on a copy shaped like the user's; UI test: Home shows 10. |
| L-20 | Verified for T0 | Sign-in starts are now marked (QA-4). |

## Limitations

- No drawer, background video, Boot Up or game launching yet (T1.2, T1.3); A on a game is logged only.
- No right-stick pointer yet (reference A7); the right stick does nothing on Home.
- Wi-Fi signal strength is not shown (always the full Wi-Fi icon).
- Interface Scale and the light / dark preset cannot be changed in the app before Settings (T3); `--interface-scale` exists for testing.
- The copy dialogs still show plain-text hints ("A Select · B Not now"); glyphs come with the dialog work in T2.
- A library kept outside the WPF data folder (its "Games Folder" setting) is not copied.
- The copy cannot be cancelled once started.
- Steam Input or other tools that add a virtual XInput pad can show one press on two slots.
- Under the copy dialog Home is suspended (no ring, title, hover or prompts) and dimmed (alpha.1 QA-2).
