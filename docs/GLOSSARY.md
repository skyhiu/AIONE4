# AIONE 4 — Glossary (A14.17)

One definition per term. Code, logs, handoffs and docs use these exact words. Each entry names the
Part B rule (Master Prompt) or reference section it comes from; when a new term is introduced, it is
added here in the same build.

| Term | Meaning | Rule |
|---|---|---|
| **Guide** | The pad's Xbox / Home button, and its keyboard equivalents: **Alt + `** everywhere, **Home** inside AIONE. All of them count as a Guide press for every Guide rule. | B-7, B-26, B-28 |
| **short Guide** | A Guide released before about 0.5 s: comes to AIONE (pausing the game), or, with AIONE in front, goes back to the running game. | B-26 |
| **held Guide** | A Guide held about 0.5 s: the Quick Menu (from Home or from inside a game). Decided in the core. | B-26 |
| **Alt + `** | The keyboard's global Guide: the key under Esc (by position, any layout) with Alt; a short press only; works both ways. | B-28 |
| **in front / the foreground** | The window Windows gives the keyboard and controller focus to. "AIONE in front" means AIONE's main window is the foreground window. | A5, B-20 |
| **bringing AIONE to the front** | The core's foreground sequence (`foreground::bring_to_front`): restore if minimized, then the methods *direct*, *attach-thread-input*, *input-event*, *topmost-pulse*, stopping at the first that works; each step time-limited (300 ms) and run off the main thread (D-057). | A5, B-20, D-057 |
| **step time limit** | 300 ms per step of bringing a window to the front; a step that doesn't return in time is **abandoned** (logged; the next method is tried; no new attach while one is still stuck). | D-057 |
| **the answer check** | Before *attach-thread-input*, the window in front must answer a harmless message within 100 ms; a **busy** window (a game at work) doesn't, and the attach is skipped. | D-057 |
| **hand over / back to the game** | Giving the running game the front (A Continue, short Guide in AIONE, Alt + `). Logged `back to "<game>": …`. | B-18, B-20, D-027 |
| **the game's main window** | Found again at each use, across the **game's process tree**: the largest visible top-level window, the one higher in Windows' order on a tie; else a minimized one. Logged with its program and process. | D-027, D-058 |
| **the game's process tree** | The counted process and every process it started (children, grandchildren), never Steam, Explorer or the Game Bar. The pause, the close, the window search and the game's end all use it. | B-20, B-18, D-058 |
| **counted** | The real game appeared (its window): the start is recorded, playtime runs. A launcher alone is never counted. | B-11 (rev. ag) |
| **the game's end** | For a counted game: its process has ended **and** no process it started still shows a window. A leftover helper with no window doesn't keep it running. | B-18, D-036, D-058 |
| **step aside** | When Windows refuses to give the game the front, AIONE puts the game's window on top and minimizes itself; never left minimized afterwards. | B-20, D-038 |
| **pause / resume** | Suspending the game's process tree while AIONE is in front, and resuming it the moment AIONE starts being left. Only after AIONE is confirmed in front (watchdog 1 s). | B-20, D-035, D-039 |
| **launcher** | A program that starts the real game (by name, or one that ends before any game appeared). | B-18 (rev. af / ai) |
| **the launcher wait** | About 5 s of black after a launcher ends, while AIONE looks for the real game; extended while any process of the launcher's chain is alive. No B. | B-22, D-055 |
| **the launcher's chain** | The launcher and every process it starts, followed while any of it is alive, until one shows a game window. | B-22 (rev. bb), D-055 |
| **Starting… / the quiet watch** | A start with no window after 45 s keeps being watched for up to about 3 min; the tile shows "Starting…". | B-22 (rev. ax), D-051 |
| **the game still loading** | From the game's window appearing until it stays put for 3 s (at most 20 s); Guide is ignored meanwhile. | B-22 (rev. ap) |
| **the held set** | Every pad button and key currently down, kept by the core; "Release the buttons" waits only for the controller and the keys that act in AIONE. | B-22 (rev. ao), D-045 |
| **input-locking state** | A state that locks input (Boot Up / Down, a launch, the launcher wait, a return, a close, a dialog): each has a time limit, an escape, and logs its start and end. One owner: the core's state machine. | B-22 |
| **Playing** | The running game's tile badge. | B-18 |
| **Close and start** | Closing the running game (one question) and starting another. | B-18 |
| **Close Software** | X on the running game: one question, then the polite close (about 3 s, every window of the game's tree), then ended by AIONE. | B-18 (rev. am), D-052, D-058 |
| **the edge bump** | The focus ring (or a slider's knob) springing about 14 px toward a press that can't move further, with the edge-bump sound; in every focusable place. | B-14, B-29 |
| **wrap** | Home's row jumping to its other end on a fresh press at an end (the only place that wraps). | B-14 (rev. w) |
| **notification** | AIONE's own passive notice (a game ended, a controller connected / disconnected, a battery low): one design, top-left, one at a time for 4 s; never focusable, never over a dialog; controller and battery notices that waited over 30 s are dropped. | T2.3, reference Part B "low battery warning" |
| **Boot Up / Boot Down** | The start presentation; the presentation before Sleep Mode and Close AIONE. | A9, B-5 |
| **Sleep Mode** | Boot Down, then Windows sleeps; AIONE comes back on waking. | A9 |
| **the Quick Menu** | The panel from the right edge on a held Guide: Power, Brightness, Volume, Theme. | B-26 (rev. ax) |
| **Power options** | The short centred list: Sleep Mode, Restart PC, Turn Off PC, Restart AIONE, Close AIONE. | B-26 (rev. ax) |
| **the right-stick pointer** | AIONE's own pointer (never the Windows cursor), moved by the right stick (a dead zone, then smooth acceleration); the item it rests on becomes the **focused item** of the context that owns input, and A activates it. An item only partly in view is focused after a 0.4 s rest, so a list scrolls step by step at its edge. Hidden after about 2 s at rest or by any other input; the focus stays. | B-12, reference A7 |
| **foreground loss / regain** | Anything taking the front from AIONE (Alt-Tab, another program, a game). On loss AIONE releases held keys, hides the pointer, clears the mouse hover and ends a drag; on regain it resumes its sound output; open menus stay open; both logged with what was open. | A9, reference A9 |
| **the user's assets** | The only fonts, icons, glyphs, sounds, logo and colours AIONE uses (`THIRD_PARTY_ASSETS.md`); `check:assets` fails CI on anything else. | A16 |
