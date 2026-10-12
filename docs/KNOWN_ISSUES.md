# Known issues

Mirrors Master Prompt Part C with status (A12): Fixed and Verified · Implemented, Unverified ·
Diagnostics added · Open · Blocked. Only runtime evidence checked by QA or the maintainer makes
an item Fixed and Verified.

## D-defects

| ID | Severity | Description | Status (4.0.0-alpha.2) |
|---|---|---|---|
| D-001 | Low | Copy counts incomplete (L-17 / KD-101): library folders not counted; hidden and favorites read from guessed fields. | **Implemented, Unverified; Diagnostics added.** One rule (`library::scan_library`) counts games, hidden, favorites and library folders separately, in the dialog and the log, with where each count came from and every skipped entry with its reason. Unit tests cover data with hidden games, favorites and folders, and the user's 10 / 0 / 0 / 0. **The WPF storage format of hidden, favorites and folders is a hypothesis** (not in the reference); the log now lists the WPF data folder's top level so the next log can confirm it. |
| D-002 | Low | A pad already connected at start shows as "not connected". | **Implemented, Unverified.** The UI asks the core for the connected slots at start (`pad_slots`) and merges the answer with events by sequence number. The mock now connects before presses and can be connected before the UI listens; UI test passes. |
| D-003 | Low | Copy progress bar disagrees with its text. | **Implemented, Unverified.** Cause **confirmed by measurement**: the text changed at once while the bar slid over 200 ms, with updates every 100–150 ms, so the bar always trailed (text 25 % with the bar at 6–12 %). Now one value drives both, and the slide is 80 ms (transform: scaleX). UI test: fill ÷ track = shown ratio ±2 % at rest. |

## L-lessons touched

| Lesson | Status | Notes |
|---|---|---|
| L-17 (import counts match) | Implemented, Unverified | See D-001. "What loads equals the counts" can be shown fully in T1. |
| L-20 (foreground at start) | Implemented, Unverified | Start and second start passed on the PC (alpha.1 QA); sign-in still to test. alpha.2 adds the keyboard-focus log line. |

## Limitations

- No Boot Up, Home, Settings or game launching yet (T1+).
- A library kept outside the WPF data folder (its "Games Folder" setting) is not copied.
- The copy cannot be cancelled once started.
- Steam Input or other tools that add a virtual XInput pad can show one press on two slots.
- Under the copy dialog, the wordmark is half-visible (alpha.1 QA-2): carried into T1 (frame).
