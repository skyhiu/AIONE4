# Third-party assets and licences

Assets live in `assets/` (A3), each licence or credit file next to its files. In 4.0.0-alpha.2
they are in the repository only: **the UI does not use them yet**, and the portable zip carries
only this document and the README credits. `npm run check:assets` (CI) checks that each licence
file is present and that Material Symbols matches its pinned npm package.

| Asset | Source | Licence | Files in the repository | Changes |
|---|---|---|---|---|
| **Kenney Input Prompts 1.5** (Kenney, www.kenney.nl) | the user's download `kenney_input-prompts_1.5.zip` | CC0 1.0 (`License.txt`) | `assets/glyphs/kenney-input-prompts/`: the `Vector` (SVG) folders of Xbox Series (99), PlayStation Series (136), Generic (36), Keyboard & Mouse (257) | none; set folders renamed only |
| **50 Menu Interface SFX** by **ColorAlpha** | https://coloralpha.itch.io/50-menu-interface-sfx (user's download) | **CC BY 4.0**. The download has no licence file; the credit is the itch.io address with "CC BY 4.0" (the user's decision, B-1 rev. d; `CREDIT.txt`). | `assets/sounds/50-menu-interface-sfx/`: the 50 WAV files | none (FLAC, MP3 and Ableton `.asd` files not added) |
| **Startup-sound variants A, B, C** (adaptations of 50 Menu Interface SFX) | made by `tools/sound/make_startup.py` from the pack | CC BY 4.0, **modified** | not in the repository; **not chosen** (alpha.2), superseded by the Boot Up variants (B-5) | See below and `docs/sound/SOUND_MAPPING.md` |
| **Boot Up sound variants A (Bloom), B (Steps), C (Sweep)** (adaptations of 50 Menu Interface SFX), 5 s each | made by `tools/sound/make_bootup.py` from the pack | CC BY 4.0, **modified** | not in the repository (delivered for listening; the chosen one is added in T1) | See below and `docs/sound/BOOTUP_SOUNDS.md` |
| **AIONE logo proposals** (the name as outlines) | `tools/brand/make_logos.py`, glyph outlines of Quicksand, Figtree and Nunito from `assets/fonts` | marks: the project's own; name outlines: SIL OFL 1.1 fonts, used as artwork (allowed by the OFL) | not in the repository (proposals delivered with the handoff) | `docs/brand/LOGO_PROPOSALS.md` |
| **Material Symbols Rounded** (Google) | npm `material-symbols` **0.47.5** (published 2026-09-22, pinned in `package.json`) | Apache-2.0 (`LICENSE`) | `assets/icons/material-symbols-rounded/`: `material-symbols-rounded.woff2`, `rounded.css` | none |
| **Nunito** | Google Fonts download attached by the user | SIL OFL 1.1 (`OFL.txt`) | `assets/fonts/nunito/`: variable upright and italic | none |
| **Inter** | same | SIL OFL 1.1 | `assets/fonts/inter/`: variable upright and italic | none |
| **Figtree** | same | SIL OFL 1.1 | `assets/fonts/figtree/`: variable upright and italic | none |
| **Quicksand** | same | SIL OFL 1.1 | `assets/fonts/quicksand/`: variable | none |
| **M PLUS Rounded 1c** | same | SIL OFL 1.1 | `assets/fonts/m-plus-rounded-1c/`: Regular, Medium, Bold | none (Thin, Light, ExtraBold and Black, about 3.4 MB each, not added) |
| App icon (ring and "4") | original, drawn for AIONE 4 | the project's own | `src-tauri/icons/` | — |

Not used: "Cute, Cozy & Magical UI SFX" (B-1, may return in T8); the WPF startup `e.wav`.

## Boot Up sound variants: credit and changes
"AIONE 4 Boot Up sound (variant A / B / C)" is adapted from **"50 Menu Interface SFX" by
ColorAlpha** (https://coloralpha.itch.io/50-menu-interface-sfx), licensed **CC BY 4.0**
(https://creativecommons.org/licenses/by/4.0/). **Modified:**
- **A, Bloom:** from `MI_SFX 48`, `44`, `43`, `09`. Sequenced to a 5 s timeline; #09 pitched
  +4 / +7 semitones; #43 pitched −12; stereo echo; cut to 5 s with a fade; high-pass; loudness
  matched.
- **B, Steps:** from `MI_SFX 09`, `43`, `16`. #09 pitched to C5-E5-G5-C6; #16 pitched −4 and
  softened; #43 pitched −12; sequenced and layered; stereo echo; cut, fade, high-pass, loudness.
- **C, Sweep:** from `MI_SFX 31`, `44`, `43`, `16`, `09`. Sequenced and layered; #43 pitched −5;
  #16 −4 and softened; #09 to C5, E5, G5; stereo echo; cut, fade, high-pass, loudness, limiting.

Details: `docs/sound/BOOTUP_SOUNDS.md`.

## Startup-sound variants (alpha.2, not chosen): credit and changes
"AIONE 4 startup sound (variant A / B / C)" is adapted from **"50 Menu Interface SFX" by
ColorAlpha** (https://coloralpha.itch.io/50-menu-interface-sfx), licensed **CC BY 4.0**
(https://creativecommons.org/licenses/by/4.0/). **Modified:**
- **A:** from `MI_SFX 44.wav`. Padding trimmed, light stereo echo added, faded out, high-passed
  at 60 Hz, loudness matched with gentle peak limiting.
- **B:** from `MI_SFX 09.wav` (played three times, two of them pitched +4 and +7 semitones) and
  `MI_SFX 43.wav`. Sequenced, light stereo echo, fade, high-pass, loudness matched.
- **C:** from `MI_SFX 48.wav`, `MI_SFX 44.wav`, `MI_SFX 43.wav` and `MI_SFX 16.wav` (pitched
  −4 semitones and softened). Layered, stereo echo, fade, high-pass, loudness matched.

## Used by the app from 4.0.0-alpha.4

- **50 Menu Interface SFX** (ColorAlpha, CC BY 4.0, coloralpha.itch.io/50-menu-interface-sfx):
  `MI_SFX 24.wav` (move), `MI_SFX 09.wav` (select), `MI_SFX 15.wav` (back), unchanged.
- **Kenney Input Prompts 1.5** (CC0): `xbox_button_a.svg`, `xbox_button_b.svg`,
  `keyboard_enter.svg`, `keyboard_escape.svg`, unchanged (tinted by the app as a mask).
- **Material Symbols Rounded** (Apache-2.0): the header icons.
- **M PLUS Rounded 1c** (SIL OFL 1.1): Regular, Medium, Bold, the theme font.

## Logo and Boot Up proposals (4.0.0-alpha.4; delivered separately, not in the app)

Sounds `gather.wav`, `mosaic.wav`, `path.wav`: **adaptations** of "50 Menu Interface SFX" by
ColorAlpha (CC BY 4.0), made by `tools/brand/make_t11_bootup.py`:
- **Gather:** #09 at 0, +4, +7, +12, +16 semitones (−2 dB); #44 (−3 dB); #43 (−1 dB), #16 at
  −4 semitones low-passed at 3 kHz (−6 dB), #09 at −12 (−8 dB); stereo echo 210 / 290 ms.
- **Mosaic:** #13 at 0, +5, +7 with #24 (−6 dB); #49 (−5 dB) with #28 (−4 dB); #48 (−5 dB); #43
  (−2 dB) with #16 at −4, low-passed (−7 dB); stereo echo 190 / 260 ms.
- **Path:** #21 at 0, +4, +7, +12 with #31 (−9 dB); #28 (−3 dB); #44 (−3 dB); #43 with #16 at −4,
  low-passed; stereo echo 200 / 280 ms.
- All: 60 Hz high-pass, loudest 100 ms matched to the pack median − 2 dB, gentle limiting (at
  most 0.1 dB used), true peaks ≤ −1 dBFS, exactly 5.0 s with a 400 ms fade.

Lettering in the proposals: M PLUS Rounded 1c Medium (SIL OFL 1.1), converted to outlines.
The marks are original (designed in this build); no console maker's shapes.

## FilmCow (menu sounds from 4.0.0-alpha.5)

- **FilmCow Recorded SFX** and **FilmCow SFX – Mechanisms & Movement (192 kHz)** by **FilmCow**,
  the user's own libraries, **royalty free** (as the user reports; B-1 rev. h / j). Credit:
  "FilmCow". The raw recordings are **not** in the repository.
- **In the app** (`assets/sounds/filmcow-menu/`, made by `tools/sound/make_menu_family.py`):
  - `move.wav` from "clicky button 7" (Recorded SFX);
  - `select.wav` from "mechanical button 8" (Mechanisms & Movement) layered with "glass ding 4"
    (Recorded SFX) at −13 dB;
  - `back.wav` from "metal switch 1" (Recorded SFX).
- **Changes:** one hit cut out; high-pass and low-pass filtered; gated and shortened (a tight
  decay); pitch-shifted to the family's notes (move −0 … +2.3 semitones, details in
  `assets/sounds/filmcow-menu/family.json`); layered; loudness matched; converted to 48 kHz mono
  24-bit.
- **Alternatives and proposals** (listening page only, not in the repository): made the same way
  from "clicky squeaky knob 17", "mechanical button 7", "plastic toy mechanism snap 6", "mechanism snap together 6", "tape recorder eject 5", "metal switch 7", "old metal switch 1", "camera lens contract 4", "camera lens extend 2", "plastic case slide off 1", "plastic case slide on 9", "tin pop 2", "tin pop 5", "light switch on", "metal button pop up 7", "metal latch 4", "wood hit 4".

The 50 Menu Interface SFX pack (above) stays in the repository until T1.2, as instructed; the app
no longer plays it.

