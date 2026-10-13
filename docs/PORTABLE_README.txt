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

Credits (details in THIRD_PARTY_ASSETS.md):
- Menu sounds made from FilmCow recordings: "FilmCow Recorded SFX" and "FilmCow SFX -
  Mechanisms & Movement" by FilmCow, royalty free. Files used: clicky button 7, mechanical
  button 8, glass ding 4, metal switch 1 (cut, filtered, shortened, pitch-shifted, layered,
  levelled, converted to 48 kHz).
- "50 Menu Interface SFX" by ColorAlpha, https://coloralpha.itch.io/50-menu-interface-sfx,
  CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/). The Boot Up sound will be an
  adaptation of this pack (modified: sounds combined, pitched, echoed, faded, loudness
  matched). No longer played from 4.0.0-alpha.5; removed in a later build.
- Kenney Input Prompts by Kenney (www.kenney.nl), CC0.
- Material Symbols Rounded by Google, Apache-2.0.
- Fonts Nunito, Inter, Figtree, Quicksand, M PLUS Rounded 1c: SIL Open Font License 1.1.
