AIONE 4 {VERSION}
=================

Run AIONE4.exe. No installation is needed; keep the exe wherever you like.

- Data and logs: %LOCALAPPDATA%\AIONE4\ (logs in the Logs folder).
- Needs the Microsoft Edge WebView2 Runtime (included in Windows 10 and 11).
- On the first start AIONE 4 offers to copy your previous AIONE data
  (%LOCALAPPDATA%\AIONE\). That folder is only read and never changed.
  B = Not now (asked again next start); "Don't copy" is final. After a copy (or a
  "Don't copy") updates never ask again.

Test build (until Settings exist):
- Ctrl+Shift+R restarts AIONE 4. Alt+F4 closes it.
- Start with Windows:   AIONE4.exe --autostart=on    (off: --autostart=off)
  The entry starts AIONE with --from-autostart, so the log shows a sign-in start.
- Interface Scale for one run:   AIONE4.exe --interface-scale=150   (90 to 150, steps of 10)
- Show the default Window B Boot Up / Boot Down for one run (even with your own boot video):
                                 AIONE4.exe --boot=default
- A, B, Enter, Esc or a click skips the Boot Up. Alt+F4 plays the Boot Down, then closes.

Credits (details in THIRD_PARTY_ASSETS.md):
- Menu sounds made from FilmCow recordings: "FilmCow Recorded SFX" and "FilmCow SFX -
  Mechanisms & Movement" by FilmCow, royalty free. Files used: clicky button 7, mechanical
  button 8, glass ding 4, metal switch 1 (cut, filtered, shortened, pitch-shifted, layered,
  levelled, converted to 48 kHz).
- The AIONE logo "Window B", its Boot Up / Boot Down animation and their sound logo: AIONE's own.
  The boot sounds include foley from "FilmCow Recorded SFX" by FilmCow, royalty free (tube tv
  turn on 1, fudge pop 1, rope tighten 2, anvil hit 2, metal latch 2, ceramic statue breaking 1,
  swoosh 2, swoosh 1, tube tv turn off 1).
- Kenney Input Prompts by Kenney (www.kenney.nl), CC0.
- Material Symbols Rounded by Google, Apache-2.0.
- Fonts Nunito, Inter, Figtree, Quicksand, M PLUS Rounded 1c: SIL Open Font License 1.1.
