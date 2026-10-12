# Known issues

Mirrors Master Prompt Part C (D-defects) with status. Statuses per A12: Fixed and Verified, Implemented Unverified, Diagnostics added, Open, Blocked.

## D-defects

*(none yet)*

## L-lessons touched in T0

| Lesson | Status | Notes |
|---|---|---|
| L-17 (import counts match) | Implemented, Unverified | One rule for what is a game (`library::scan_games_dir`); copy result and log list every ignored entry. Hidden / favourite counts read `Hidden` / `IsFavorite`-style fields in game.json — **hypothesis** about the WPF format, to confirm with the real data. |
| L-20 (foreground at start) | Implemented, Unverified | Fallback chain and start-up guard, all logged in `lifecycle.log`. Needs runtime evidence on the PC, including at Windows sign-in. |

## Limitations in T0

- No Boot Up, Home, Settings or game launching yet (T1+).
- The WPF library at a custom "Games Folder" outside the WPF data folder is not copied (the copy result says so if it finds the setting).
- The copy cannot be cancelled once started.
- Steam Input or other tools that add a virtual XInput pad can show one press on two slots (the readout shows which slots are connected).
