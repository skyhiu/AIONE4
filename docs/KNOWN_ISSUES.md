# Known issues

Mirrors Master Prompt Part C with status (A12): Fixed and Verified · Implemented, Unverified ·
Diagnostics added · Open · Blocked. Only runtime evidence checked by QA or the maintainer makes
an item Fixed and Verified.

## D-defects

| ID | Severity | Description | Status (4.0.0-alpha.3) |
|---|---|---|---|
| D-001 | Low | Copy counts: library folders, hidden and favorites (L-17 / KD-101). | **Implemented, Unverified (except `Config\`, moved to T2).** Four counts with one rule since alpha.2. alpha.3 fixes the missing source lines. Cause **confirmed**: an alpha.2 edit did not apply, so the lines were never written; a log-capture test now proves they are. The three "library … " lines are always written, also "none found, looked in: …", and name `Config\` as not yet read until the user's files arrive in T2. |
| D-002 | Low | A pad already connected at start shows as "not connected". | **Implemented, Unverified** (alpha.2; not yet tested on the PC with a pad on). |
| D-003 | Low | Copy progress bar disagrees with its text. | **Implemented, Unverified** (alpha.2; CI screenshot showed 25.0 % against "25 %"; QA to confirm). |
| D-004 | Medium | An update asked the copy question again and copied everything a second time. | **Implemented, Unverified.** Cause **confirmed** (QA-1, by the log): alpha.2 required a `folders` field that alpha.1 had not written, and treated "unreadable" as "never asked". Now, per B-6: every record field has a default except the decision; an unreadable record is backed up once next to itself and reported in the log and to the user; a finished copy in `Imported\AIONE-WPF\` means "never ask again", even without a record. Fixtures of alpha.1's and alpha.2's records are read as "copied" in Rust and UI tests. |
| D-005 | Low | The keyboard-focus line calls AIONE's own web view "another program". | **Implemented, Unverified.** Cause **confirmed** from the source: it compared processes. Now judged by window ownership: is the focus window's top-level ancestor AIONE's main window? Logged as "AIONE 4's web view: keys reach AIONE", or "another program has the keys: … (keys do NOT reach AIONE)". |
| D-006 | Low | Held keys repeat at Windows' speed, not 400 / 90 ms. | **Implemented, Unverified.** Cause **confirmed** from the source: the UI counted the browser's repeated keydowns. Now AIONE times key repeats itself (400 ms, then every 90 ms) and ignores Windows' repeats; modifier keys never repeat. Unit test with fake timers; UI test measured the first repeat at 400–401 ms, then a 90.3–92.3 ms mean gap (five runs). |

## L-lessons touched

| Lesson | Status | Notes |
|---|---|---|
| L-17 (import counts match) | Implemented, Unverified | See D-001 and D-004. "What loads equals the counts" can be shown fully in T1. |
| L-20 (foreground at start) | Implemented, Unverified | Start and second start passed on the PC (alpha.1 QA); sign-in still to test. alpha.3 fixes the keyboard-focus line (D-005). |

## Limitations

- No Boot Up, Home, Settings or game launching yet (T1+).
- A library kept outside the WPF data folder (its "Games Folder" setting) is not copied.
- The copy cannot be cancelled once started.
- Steam Input or other tools that add a virtual XInput pad can show one press on two slots.
- Under the copy dialog, the wordmark is half-visible (alpha.1 QA-2): carried into T1 (frame).
