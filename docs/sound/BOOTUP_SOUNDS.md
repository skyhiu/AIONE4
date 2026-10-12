# Default Boot Up: three 5-second sound variants and their timeline (B-5 proposal)

Status: **proposal** (alpha.3). The user listens and chooses. Not used by the app; the animation
is built in T1 with the chosen logo and sound. Made only from the 50 Menu Interface SFX pack
(B-1) by `tools/sound/make_bootup.py`; numbers are in `bootup-variants.json`. The earlier
1.5–2.2 s startup variants (alpha.2) were not chosen (B-1 rev. d) and are superseded.

## The timeline (shared by all three)
| Time | Animation (proposal) |
|---|---|
| 0.00 s | black screen (A9: black from the first frame, until animation and sound start together) |
| 0.40 s | the mark appears: fades in and grows from 92 % to 100 % over 0.6 s |
| 1.40 s | the name AIONE follows: fades in and rises 8 px over 0.5 s |
| 2.40 s | both settle: the mark swells by 4 % and back (0.5 s) |
| 4.20 s | everything fades out (0.8 s) |
| 5.00 s | Home |

Only `transform` and `opacity` animate (A7). A skip ends Boot Up at once, with no fallback
sound (A9, L-03). A custom Boot Up video replaces all of this.

## The variants

| Variant | Pack files | Length | Loudest 100 ms | True peak | Limiting | Measured peaks (s, LUFS) | In one line |
|---|---|---|---|---|---|---|---|
| **A – Bloom** | #48, #44, #43, #09 | 5.0 s | −10.5 LUFS | −2.4 dBFS | none | 0.41 (−10.5), 1.45 (−15.3), 2.41 (−22.2), 2.82 (−21.3) | A soft swell blooms into a sparkle with the logo, a bright chime greets the name, and a gentle do-mi-sol settles it. |
| **B – Steps** | #09, #43, #16 | 5.0 s | −11.2 LUFS | −1.0 dBFS | 1.4 dB | 0.45 (−13.5), 0.81 (−14.2), 1.45 (−11.2), 2.45 (−12.1) | Four clear rising notes draw the logo in, a bright chime names it, and a warm chord settles everything: the most melodic. |
| **C – Sweep** | #31, #44, #43, #16, #09 | 5.0 s | −11.4 LUFS | −1.0 dBFS | 3.5 dB | 0.45 (−12.4), 1.45 (−11.5), 2.45 (−13.2), plus echoes at 1.73, 2.70 | A quick upward sweep lifts the logo in with a sparkle, a bright double chime names it, and a full chord rings out: the grandest. |

How the peaks line up:
- **The mark (0.40 s):** every variant peaks at 0.41–0.45 s, as it appears. Peaks are 100 ms windows, so they are reported about 0.05 s after the onset.
- **The name (1.40 s):** every variant has its chime at 1.45 s.
- **The settle (2.40 s):** every variant has its arpeggio or chord at 2.41–2.45 s.
- **The fade (4.2 s):** the echo tails die away during the fade, before 5.0 s.

**A's settle is the gentlest:** about 12 dB under its first peak, so A sounds front-loaded.
**B and C** keep their three moments at similar loudness.

**All three:**
- 24-bit, 44.1 kHz, stereo, no clipped samples, true peaks at or below −1 dBFS.
- Loudness matched like the startup variants: the loudest 100 ms is set to the pack median
  (−8.5 LUFS) minus 2 dB, with gentle limiting. B and C end 0.7 and 0.9 dB under that target.

**Changes (CC BY 4.0 adaptations), A – Bloom:**
- #48 (swell) 5 dB down, starting 0.12 s before the mark.
- #44 at 0.40 s.
- #43 (C7 chime) 2 dB down at 1.40 s.
- #09 three times at 2.40 s (as is, +4 and +7 semitones, 120 ms apart, 8 dB down), closed by #43 an octave lower (C6, 9 dB down).
- Stereo echo 230 / 310 ms, feedback 0.50, wet 0.30, darkened above 4 kHz.
- Cut to 5.0 s with a 0.6 s fade-out; 60 Hz high-pass; loudness matched with gentle limiting.

**Changes, B – Steps:**
- #09 as C5-E5-G5-C6 (0, +4, +7, +12 semitones), 180 ms apart from 0.40 s, 2–3 dB down.
- #43 (C7) at 1.40 s.
- At 2.40 s, a C major chord: #16 moved down 4 semitones to C4, softened above 3 kHz, 6 dB down; #43 at C6, 6 dB down; #09 at G5, 10 dB down.
- Echo 200 / 280 ms, feedback 0.45, wet 0.28, darkened above 4.5 kHz.
- The same cut, fade, high-pass and loudness as A.

**Changes, C – Sweep:**
- #31 (rising zip) 6 dB down, from 0.30 s, into #44 at 0.40 s.
- #43 at 1.40 s, with a copy 5 semitones lower (G6) 90 ms later, 8 dB down.
- At 2.40 s, a layered C major chord: #16 at C4 (softened, 4 dB down); #09 at C5, E5 and G5 (7–9 dB down); #44 again, 9 dB down.
- Echo 250 / 340 ms, feedback 0.55, wet 0.33, darkened above 3.8 kHz.
- The same cut, fade, high-pass and loudness as A.

The pitch changes are tape-style, so a higher note is slightly shorter. Credit: "50 Menu Interface SFX" by ColorAlpha,
https://coloralpha.itch.io/50-menu-interface-sfx, CC BY 4.0, **modified** as listed.

## The preview
`AIONE4_bootup_preview.html` (delivered, not in the repository; made by `tools/brand/make_preview.py`):
- **What it is:** a silent, rough animation of this timeline on black, with any of the three logos.
- **"Play with sound":** plays the chosen WAV if the three WAV files are in the same folder as the HTML.
- **What it is not:** the T1 animation. There are no final easings, no glow, no Interface Scale, and no
  sync guarantee beyond starting the sound and the animation together.
