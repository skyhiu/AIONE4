# Emulator profiles (B-19 part 1, alpha.18)

AIONE recognizes the emulator behind a game's `game.lnk` (by the program it starts) and, at launch,
adds **only the options the shortcut lacks** (never twice). It **never changes a shortcut or an
emulator's files**. The profiles are data: `src-tauri/emulators.json`.

**Rule:** every option and setting below is taken from the emulator's **own source code** on GitHub,
at the commit given. Nothing is guessed. A point that can't be set from the command line says so and
gives guide text instead.

B-19's six points: **1** opens the game directly · **2** fullscreen · **3** stopping closes the
emulator · **4** no confirm on stop · **5** no start-up pop-ups · **6** the controller.

## Verified from source

### Dolphin (GameCube / Wii)
Source: `dolphin-emu/dolphin`, master **8d2ad1a66247**.

| Point | How | Source |
|---|---|---|
| 1 | `-e` / `--exec <file>` "Load the specified file" (must be in your shortcut) | [`Source/Core/UICommon/CommandLineParse.cpp`](https://github.com/dolphin-emu/dolphin/blob/master/Source/Core/UICommon/CommandLineParse.cpp) |
| 2 | **added:** `-C Dolphin.Display.Fullscreen=True` (`-C <System>.<Section>.<Key>=<Value>`; `Main` is named `Dolphin`) | `CommandLineParse.cpp`; [`Source/Core/Core/Config/MainSettings.cpp`](https://github.com/dolphin-emu/dolphin/blob/master/Source/Core/Core/Config/MainSettings.cpp) (`MAIN_FULLSCREEN = {Main, "Display", "Fullscreen"}`, default false); [`Source/Core/Common/Config/Config.cpp`](https://github.com/dolphin-emu/dolphin/blob/master/Source/Core/Common/Config/Config.cpp) (`{System::Main, "Dolphin"}`) |
| 3 | **added:** `-b` / `--batch` "Run Dolphin without the user interface (Requires --exec or --nand-title)": added only when `-e` / `-n` is in the shortcut | `CommandLineParse.cpp` |
| 4 | **added:** `-C Dolphin.Interface.ConfirmStop=False` (`MAIN_CONFIRM_ON_STOP = {Main, "Interface", "ConfirmStop"}`, default **true**) | `MainSettings.cpp` |
| 5 | Guide: the update check is a setting (`AutoUpdate` / `UpdateTrack`); not added | `MainSettings.cpp` |
| 6 | Guide: set the pad in Dolphin's Controllers | — |

**B-20 backup, "Background Input":** `MAIN_INPUT_BACKGROUND_INPUT = {Main, "Input", "BackgroundInput"}`, default **false** (`MainSettings.cpp`, master `8d2ad1a66247`); its checkbox is "Background Input" in Controllers (`DolphinQt/Config/CommonControllersWidget.cpp`). When it's on, Dolphin reacts to the pad while AIONE is open: the check report says so, with where to turn it off.

**Settings check (read only):** Dolphin's user folder, found as
[`Source/Core/UICommon/UICommon.cpp`](https://github.com/dolphin-emu/dolphin/blob/master/Source/Core/UICommon/UICommon.cpp)
does:
1. `portable.txt` next to the program → `<program folder>\User`;
2. else `Documents\Dolphin Emulator` if it exists;
3. else `%APPDATA%\Dolphin Emulator`.

The registry overrides aren't read. The file is `Config\Dolphin.ini`; AIONE reports `[Interface] ConfirmStop`, `[Display] Fullscreen` and `[Input] BackgroundInput`.

### Cemu (Wii U)
Source: `cemu-project/Cemu`, main **a53cf0ece7**, [`src/config/LaunchSettings.cpp`](https://github.com/cemu-project/Cemu/blob/main/src/config/LaunchSettings.cpp).

| Point | How |
|---|---|
| 1 | `-g` / `--game <path>` "Path of game to launch" (must be in your shortcut) |
| 2 | **added:** `-f` / `--fullscreen` "Launch games in fullscreen mode" (only when `-g` / `-t` is in the shortcut) |
| 3, 4, 5 | **No command-line option** in this file. Guide: close Cemu's window (or Close Software) to end the game; check Options → General Settings. |
| 6 | Guide: Options → Input Settings |

### DuckStation (PlayStation)
Source: `stenzek/duckstation`, master **66888ba0d4**, `src/duckstation-qt/qthost.cpp` (its help text).

| Point | How |
|---|---|
| 1 | the game's file as the last argument (in your shortcut) |
| 2 | **added:** `-fullscreen` "Enters fullscreen mode immediately after starting." |
| 3 | **added:** `-batch` "Enables batch mode (exits after powering off)." (`-nogui` also implies exit on shutdown; either counts as present) |
| 4, 5, 6 | Guide: DuckStation's Settings (Interface, Controllers) |

### PCSX2 (PlayStation 2)
Source: `PCSX2/pcsx2`, master **685637d30b**, `pcsx2-qt/QtHost.cpp` (its help text).

| Point | How |
|---|---|
| 1 | the game's file as the last argument (in your shortcut) |
| 2 | **added:** `-fullscreen` "Enters fullscreen mode immediately after starting." |
| 3 | **added:** `-batch` "Enables batch mode (exits after shutting down)." (`-nogui` "implies batch mode") |
| 4, 5, 6 | Guide: PCSX2's Settings (Interface, Controllers) |

### Azahar (3DS)
Source: `azahar-emu/azahar`, master **3c3e54989e**, [`src/citra_qt/citra_qt.cpp`](https://github.com/azahar-emu/azahar/blob/master/src/citra_qt/citra_qt.cpp).

| Point | How |
|---|---|
| 1 | the game's path: alone (`args.size() == 2`) **or as the last argument** ("Launch game at path": `i == args.size() - 1` and not starting with `-`) |
| 2 | **added (alpha.19):** `-f` / `--fullscreen`, in front of the shortcut's arguments (the game stays last); not added when `-f`, `--fullscreen` or `-w` / `--windowed` is already there |
| 3, 4, 5, 6 | Guide: Azahar's settings |

## Not verified yet (recognized; nothing added)

| Emulator | Why |
|---|---|
| **Eden** (Switch) | Its source (git.eden-emu.dev) and yuzu's (rev. af's reference: taken down in 2024; the GitHub mirrors tried, `yuzu-mirror/yuzu`, `yuzu-emu/yuzu-mainline`, `pineappleEA/pineapple-src`, need a login) couldn't be read from the sandbox. **Not "inherited from yuzu" yet:** nothing verified, nothing added. |
| RPCS3, PPSSPP, Vita3K, shadPS4, xemu, Xenia, Ryujinx, RetroArch | Their option parsing wasn't located in this build; alpha.19. |

For these AIONE recognizes the program (the check report names the emulator) but adds nothing and
checks nothing, so it never guesses.

## Settings checks
- **Dolphin:** read only (above).
- **Cemu:** not in alpha.19. Its `CemuConfig.cpp` (main `a53cf0ece7`) has no fullscreen or update-check key (only `FullscreenScaling`); the interface settings live elsewhere and weren't located, so nothing is checked rather than guessed.

## Reading shortcuts (D-034)
On Windows, AIONE reads `game.lnk` with Windows' own shortcut interface (`IShellLinkW` + `IPersistFile`, read only), which handles every form (ID list, network or mapped drives, `%…%` variables). The byte reader is only the fallback for tests on other systems.

## The check report
On the first start of 4.0.0-alpha.18, AIONE writes **`emulator-check.md`** in its data folder: each
game, its emulator (or not), the options added at launch (and those not added, with why), the
settings to fix (Dolphin), and the guide lines. `AIONE4.exe --check-emulators` writes it again.
