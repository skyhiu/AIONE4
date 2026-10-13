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
| D-006 | Low | Key repeats and releases. | Timing **Fixed and Verified** (alpha.3, PC). Key list and release lines: **Implemented, Unverified** (alpha.4: only the arrows repeat; Enter, Space, Esc, modifiers never; `input up <key> (keyboard), held N ms, R repeats`; unit and UI tests pass). |

## T0 QA findings carried into T1.1

| Finding | Status (4.0.0-alpha.4) |
|---|---|
| QA-4 (T0 test): an autostart start is not marked | **Implemented, Unverified**: the entry runs `"<exe>" --from-autostart`; logged "started by Windows at sign-in". Unit test on the command. |
| QA-5 (T0 retest): closing logged as "the user switched away" | **Implemented, Unverified**: exit stops the guard first ("AIONE is closing"); later foreground changes are ignored. Unit test on the guard's decision. Windows code compiles first in CI. |

## T1.1 changes (rev. h)

| Item | Status (4.0.0-alpha.5) |
|---|---|
| B-8 title at the row's start with the game's icon | Implemented, Unverified (UI tests at 3 sizes × 2 scales × 2 presets) |
| B-9 hot pink accent | Implemented, Unverified (every pair measured in UI tests) |
| B-10 first press after the mouse brings the ring back only | Implemented, Unverified (UI test, pad and keyboard) |
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
