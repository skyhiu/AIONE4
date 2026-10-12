# Sounds: the 50 Menu Interface SFX pack, a proposed mapping, and the startup variants

Status: **proposal** (alpha.2). The startup variants A–C at the end were not chosen (B-1 rev. d); the 5 s Boot Up variants replace them (`BOOTUP_SOUNDS.md`). Nothing plays these yet; I (the implementer) cannot hear
them, so every description below comes from measurements and spectrograms. The user listens
and adjusts in T1.

Pack: "50 Menu Interface SFX" by ColorAlpha, CC BY 4.0 (`assets/sounds/50-menu-interface-sfx/`,
credit in `docs/THIRD_PARTY_ASSETS.md`). Every file is 0.56 s long including silence, 24-bit
44.1 kHz stereo, peak-normalized to 0 dBFS. The sound itself lasts 0.04–0.36 s.

## How the numbers were measured
`tools/sound/analyse.py` (numbers in `pack-measurements.json`):
- **Sound (s):** the part above −40 dB of the peak.
- **Loudest 100 ms (LUFS):** K-weighted loudness (ITU-R BS.1770 filters) of the strongest
  100 ms. This is how loud a short sound feels at its peak. The pack's median is −8.5 LUFS.
- **Attack / decay:** ms from the start to the peak, and from the peak to −20 dB.
- **Brightness (Hz):** spectral centroid; higher is brighter.
- **Pitch:** the fundamental where the sound is tonal ("–" = noisy or too short to tell).

Descriptions come from reading each sound's spectrogram (a picture of pitch over time).

## The 50 sounds

| # | Sound (s) | Loudest 100 ms (LUFS) | Attack (ms) | Decay to −20 dB (ms) | Brightness (Hz) | Pitch | What it is like |
|---|---|---|---|---|---|---|---|
| 01 | 0.12 | -9.3 | 5 | 60 | 2,202 | – | Short bell-like 'ding': three clear partials (about 1, 5.3 and 7.2 kHz), gone in about 0.1 s. |
| 02 | 0.36 | -4.7 | 100 | 95 | 403 | E4→F4 | Upward whoosh: a swell that rises in pitch and level for 0.1 s, then a soft low tail. |
| 03 | 0.14 | -7.1 | 10 | 90 | 813 | F2 | Low soft pluck, quick decay. |
| 04 | 0.29 | -8.1 | 10 | 95 | 760 | – | Soft noisy tap with a low hum tail. |
| 05 | 0.27 | -5.9 | 5 | 90 | 1,834 | F#6 | Click followed by a steady tone near 1.5 kHz that rings on for the whole file. |
| 06 | 0.09 | -11.9 | 10 | 40 | 6,342 | – | Quick downward chirps (falling pitch), very short. |
| 07 | 0.14 | -6.8 | 10 | 85 | 766 | – | Two-step tick: a click and a smaller second blip about 0.1 s later. |
| 08 | 0.12 | -13.8 | 5 | 55 | 5,408 | A#5→D#6 | Bright harmonic blip with a faint echo at about 0.15 s. |
| 09 | 0.10 | -11.4 | 5 | 45 | 4,587 | C5 | Bright, clean harmonic blip (about C5), about 0.1 s, very tonal. |
| 10 | 0.20 | -4.9 | 105 | 50 | 561 | – | Reverse swell: grows for 0.1 s into a high ping, then stops. |
| 11 | 0.20 | -5.2 | 105 | 50 | 786 | G4 | Double swell: two small rises into a high ping. |
| 12 | 0.20 | -2.5 | 105 | 45 | 1,706 | – | Swell into a mid tone (about 1.8 kHz) at 0.1 s. |
| 13 | 0.10 | -8.2 | 5 | 45 | 1,420 | – | Click with a short 1 kHz tone, about 0.1 s. |
| 14 | 0.12 | -8.4 | 5 | 60 | 2,364 | – | Bell-like 'ting' (partials about 1.2, 5 and 7.5 kHz), about 0.12 s. |
| 15 | 0.12 | -11.0 | 5 | 60 | 1,152 | – | Short tonal blip that falls slightly in pitch. |
| 16 | 0.15 | -9.7 | 5 | 75 | 2,391 | E4 | Warm, rich harmonic tone (about E4), decays over 0.15 s. |
| 17 | 0.04 | -19.0 | 5 | 15 | 2,507 | – | Tiny dry click (about 40 ms). |
| 18 | 0.10 | -12.8 | 0 | 40 | 7,685 | – | Short airy noise burst ('shh'), very bright, about 0.1 s. |
| 19 | 0.10 | -12.7 | 5 | 45 | 7,605 | C#7 | Airy noise burst with a faint high tone, about 0.1 s. |
| 20 | 0.12 | -11.6 | 5 | 60 | 6,857 | – | Airy noise burst, slightly longer (about 0.15 s). |
| 21 | 0.07 | -14.0 | 5 | 35 | 1,607 | A4 | Very short harmonic pluck (about A4), 70 ms, clean. |
| 22 | 0.12 | -9.5 | 5 | 65 | 3,286 | A#5→F5 | Falling sweep: partials slide down over 0.15 s. |
| 23 | 0.14 | -14.4 | 5 | 70 | 1,872 | F2 | Dense, slightly noisy harmonic buzz, about 0.15 s, quiet. |
| 24 | 0.04 | -11.3 | 0 | 25 | 771 | – | Tiny dry click (about 40 ms), softer than #17. |
| 25 | 0.07 | -16.6 | 0 | 25 | 942 | F2 | Tiny click with a low thump, about 70 ms. |
| 26 | 0.07 | -8.2 | 35 | 25 | 523 | – | Short little swell-click, about 50 ms. |
| 27 | 0.17 | -6.4 | 5 | 85 | 880 | F#4 | Click with a short low tone (about F#4), about 0.15 s. |
| 28 | 0.17 | -5.7 | 5 | 85 | 2,173 | – | Click followed by a bright steady 'ting' near 2.8 kHz lasting about 0.25 s. |
| 29 | 0.17 | -8.1 | 5 | 80 | 348 | – | Click with a low tone tail, about 0.15 s. |
| 30 | 0.10 | -9.9 | 5 | 50 | 434 | – | Click with a short low tone, about 0.1 s. |
| 31 | 0.10 | -10.0 | 10 | 45 | 1,668 | – | Rising 'zip': a fast upward sweep across the whole range in 0.1 s. |
| 32 | 0.05 | -10.2 | 15 | 20 | 505 | – | Tiny blip that rises a little at the low end. |
| 33 | 0.14 | -4.7 | 95 | 15 | 1,170 | – | Flat buzzy block, 0.1 s, steady level then a hard stop (alarm-like). |
| 34 | 0.13 | -4.6 | 5 | 95 | 1,549 | C3 | Bright downward sweep over a low tone ('drop'). |
| 35 | 0.12 | -11.3 | 65 | 40 | 1,264 | G3 | Noisy rattle burst, about 0.1 s. |
| 36 | 0.30 | -13.7 | 5 | 20 | 1,933 | F2 | Stuttering noisy burst with a short low tail, quiet. |
| 37 | 0.10 | -5.9 | 30 | 25 | 3,203 | C#4 | Noisy burst with a short tone near 4 kHz. |
| 38 | 0.12 | -7.8 | 75 | 10 | 1,669 | D5 | Dense buzzy block, about 0.1 s (alarm-like). |
| 39 | 0.14 | -6.4 | 20 | 100 | 1,855 | E4 | Buzzy block with a soft start, about 0.12 s. |
| 40 | 0.17 | -4.5 | 105 | 55 | 2,832 | C5 | Smooth swell: rises for 0.1 s and falls for 0.1 s, broad and soft ('whoosh'). |
| 41 | 0.12 | -8.6 | 15 | 70 | 2,248 | – | Noisy burst, quickly gone. |
| 42 | 0.07 | -12.2 | 25 | 35 | 1,938 | G3 | Small swell-blip at 40 ms, mid pitch. |
| 43 | 0.29 | -10.4 | 35 | 115 | 5,455 | C7 | Bright chime: two steady tones (about 2.1 kHz = C7 and 6.4 kHz) for 0.2 s, then small echo pulses. |
| 44 | 0.26 | -11.6 | 35 | 115 | 5,059 | D5→G#6 | Rising sparkle: several partials glide upward over 0.25 s, then small echo pulses. |
| 45 | 0.23 | -10.2 | 25 | 80 | 5,600 | C3→F2 | Bright burst followed by a row of quick echo ticks (stutter). |
| 46 | 0.16 | -7.9 | 5 | 95 | 1,672 | G4 | Harmonic pluck with a 0.2 s decay, slightly falling. |
| 47 | 0.17 | -7.9 | 5 | 125 | 5,050 | – | Click with two bright steady tones (about 4 and 5 kHz) for 0.2 s. |
| 48 | 0.30 | -4.3 | 110 | 110 | 2,861 | – | Reverse swell into two bright high tones (about 4–5 kHz), peaking at 0.12 s, short tail. |
| 49 | 0.24 | -4.5 | 5 | 180 | 853 | – | Big low thump with a long smooth decay (about 0.27 s). |
| 50 | 0.27 | -5.0 | 15 | 215 | 299 | D3 | Low buzzy tone (about D3) with a rough decay over 0.28 s (error-like). |

## Proposed mapping to AIONE's sound names

AIONE 4's names follow the user's list. The reference names only the start-up (`e.wav`), launch
(`i.wav`) and error (`error.wav`) files; the rest are the events the reference and Part D describe.
| AIONE sound | Proposed | Why | Second choice |
|---|---|---|---|
| move (focus moves) | **#24** | tiny soft click (40 ms); must not tire with fast repeats (repeat every 90 ms) | #17 |
| select (A / confirm) | **#09** | bright, clean, tonal blip; reads as "yes" | #21 |
| back (B) | **#15** | short blip that falls slightly; the opposite of select | #22 |
| error (not allowed) | **#33** | flat buzzy block with a hard stop | #38, #50 |
| launch (a game starts) | **#31** | energetic upward "zip" | #48 |
| open (menu, overlay, drawer) | **#10** | reverse swell into a ping: something arrives | #02 |
| close | **#06** | quick downward chirps: something leaves | #34 |
| toggle (switch on / off) | **#07** | two-step tick, like a switch | #13 |
| slider tick (one step) | **#17** | the driest, shortest click | #25 |
| tab switch (LB / RB) | **#26** | small swell-click, a little more than "move" | #42 |
| end stop (cannot go further) | **#25** | tiny click with a low thump: "bump" | #03 |
| notification | **#14** | bell-like "ting", noticeable but short | #01 |
| controller connected (T8) | **#28** | click into a bright steady ting | #47 |
| controller disconnected (T8) | **#29** | click with a low falling tail | #30 |
| keyboard key (T6) | **#13** | short click with a light tone | #24 |
| power / confirm-danger dialogs | **#16** | warm, serious tone | #46 |

The startup sound uses #44, #43, #09, #48 and #16 (below), so it shares the pack's character
but differs from any single menu sound.

## Sounds that suit a console start-up
Bright, welcoming, with a clear ending:
- **#44** rising sparkle: partials glide upward over 0.25 s, then small echo pulses. Bright and
  hopeful, with a natural ending. The best single candidate.
- **#43** bright chime on C7 with an upper tone, holds for 0.2 s, then small echoes. Clean
  and "finished"; a good final note.
- **#48** reverse swell into bright high tones. Builds anticipation; good as a lead-in, too
  short alone.
- **#28** click into a bright steady ting. Simple and friendly, but small.
- **#09** bright clean blip (C5). Too short alone; tonal enough to be played as notes.
- **#05** click into a steady 1.5 kHz ring that does not fade in the file. Unclear ending.
- **#10 / #12** reverse swells into a ping. Welcoming but abrupt at the end.

## The three startup variants (delivered as WAV files, not in the repository)
Made by `tools/sound/make_startup.py` from pack files only (numbers in `startup-variants.json`).
All are 24-bit 44.1 kHz stereo, with no clipped samples and true peaks at or below −1 dBFS. Each
is matched to the pack's loudness: the loudest 100 ms is set to the pack median minus 2 dB
(−10.5 LUFS), with gentle peak limiting (at most 4 dB, only on the loudest moments).

| Variant | Pack files | Length | Loudest 100 ms | Limiting | In one line |
|---|---|---|---|---|---|
| **A** | #44 | 1.6 s | −12.1 LUFS | 2.2 dB | The rising sparkle alone, with a soft echo so it settles instead of stopping. |
| **B** | #09, #43 | 1.5 s | −11.1 LUFS | 1.4 dB | Three quick rising notes (do-mi-sol, C5-E5-G5) landing on the bright C7 chime. |
| **C** | #48, #44, #43, #16 | 2.2 s | −10.5 LUFS | 0 dB | A short swell blooming into sparkle and chime over a soft low tone, with a longer echo tail. |

A and B end up to 1.6 dB quieter than the target, because the limiter is capped at 4 dB.
None of the three is louder than the pack's menu sounds.

**Changes (CC BY 4.0 adaptations):**
- **A:** padding trimmed; light stereo echo (190 / 260 ms, feedback 0.38, wet 0.32, each
  repeat darkened above 4.5 kHz); 120 ms fade-out; 60 Hz high-pass; loudness and peaks as above.
- **B:** #09 played three times (as is, +4 and +7 semitones, tape-style so the higher notes
  are slightly shorter), 110 ms apart and 3 dB down, then #43 at 330 ms. Light stereo echo
  (160 / 220 ms, feedback 0.30, wet 0.25, darkened above 5 kHz); fade, high-pass, loudness and
  peaks as above.
- **C:** #48 (−4 dB) as the lead-in. #44 at 110 ms, where the swell peaks. #43 at 170 ms (−2 dB).
  #16 moved down 4 semitones (E4 → C4), softened above 3 kHz, −8 dB, as a low body. Stereo echo
  (210 / 290 ms, feedback 0.42, wet 0.30, darkened above 4 kHz); fade, high-pass, loudness and
  peaks as above.

**Pitch contour:** all three rise. In their spectrograms the glides and the echo repeats curve
upward, and B steps up C5 → E5 → G5 → C7. The automatic pitch tracker reports "falling" for A
and C because it compares the start with the darkened echo tail; the spectrograms are the
better evidence here.
