# Third-party assets and licences

Assets live in `assets/` (A3), each licence or credit file next to its files. In 4.0.0-alpha.2
they are in the repository only: **the UI does not use them yet**, and the portable zip carries
only this document and the README credits. `npm run check:assets` (CI) checks that each licence
file is present and that Material Symbols matches its pinned npm package.

| Asset | Source | Licence | Files in the repository | Changes |
|---|---|---|---|---|
| **Kenney Input Prompts 1.5** (Kenney, www.kenney.nl) | the user's download `kenney_input-prompts_1.5.zip` | CC0 1.0 (`License.txt`) | `assets/glyphs/kenney-input-prompts/`: the `Vector` (SVG) folders of Xbox Series (99), PlayStation Series (136), Generic (36), Keyboard & Mouse (257) | none; set folders renamed only |
| **50 Menu Interface SFX** by **ColorAlpha** | https://coloralpha.itch.io/50-menu-interface-sfx (user's download) | **CC BY 4.0** as reported by the user. The download has **no licence file**; the itch.io page is the authority (`CREDIT.txt`). | `assets/sounds/50-menu-interface-sfx/`: the 50 WAV files | none (FLAC, MP3 and Ableton `.asd` files not added) |
| **Startup-sound variants A, B, C** (adaptations of 50 Menu Interface SFX) | made by `tools/sound/make_startup.py` from the pack | CC BY 4.0, **modified** | not in the repository yet (delivered for listening; the chosen one is added in T1) | See below and `docs/sound/SOUND_MAPPING.md` |
| **Material Symbols Rounded** (Google) | npm `material-symbols` **0.47.5** (published 2026-09-22, pinned in `package.json`) | Apache-2.0 (`LICENSE`) | `assets/icons/material-symbols-rounded/`: `material-symbols-rounded.woff2`, `rounded.css` | none |
| **Nunito** | Google Fonts download attached by the user | SIL OFL 1.1 (`OFL.txt`) | `assets/fonts/nunito/`: variable upright and italic | none |
| **Inter** | same | SIL OFL 1.1 | `assets/fonts/inter/`: variable upright and italic | none |
| **Figtree** | same | SIL OFL 1.1 | `assets/fonts/figtree/`: variable upright and italic | none |
| **Quicksand** | same | SIL OFL 1.1 | `assets/fonts/quicksand/`: variable | none |
| **M PLUS Rounded 1c** | same | SIL OFL 1.1 | `assets/fonts/m-plus-rounded-1c/`: Regular, Medium, Bold | none (Thin, Light, ExtraBold and Black, about 3.4 MB each, not added) |
| App icon (ring and "4") | original, drawn for AIONE 4 | the project's own | `src-tauri/icons/` | — |

Not used: "Cute, Cozy & Magical UI SFX" (B-1, may return in T8); the WPF startup `e.wav`.

## Startup-sound variants: credit and changes
"AIONE 4 startup sound (variant A / B / C)" is adapted from **"50 Menu Interface SFX" by
ColorAlpha** (https://coloralpha.itch.io/50-menu-interface-sfx), licensed **CC BY 4.0**
(https://creativecommons.org/licenses/by/4.0/). **Modified:**
- **A:** from `MI_SFX 44.wav`. Padding trimmed, light stereo echo added, faded out, high-passed
  at 60 Hz, loudness matched with gentle peak limiting.
- **B:** from `MI_SFX 09.wav` (played three times, two of them pitched +4 and +7 semitones) and
  `MI_SFX 43.wav`. Sequenced, light stereo echo, fade, high-pass, loudness matched.
- **C:** from `MI_SFX 48.wav`, `MI_SFX 44.wav`, `MI_SFX 43.wav` and `MI_SFX 16.wav` (pitched
  −4 semitones and softened). Layered, stereo echo, fade, high-pass, loudness matched.
