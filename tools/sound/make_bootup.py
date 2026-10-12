"""Builds the three default Boot Up sound variants (B-5): about 5 s, timed to an animation of
the AIONE logo and name, made only from the 50 Menu Interface SFX pack (B-1 rules).

Usage: python3 tools/sound/make_bootup.py <out dir>   (from the repository root)
Writes AIONE4_bootup_A/B/C.wav and bootup_timelines.json (planned animation events, the
sound's measured peaks, the recipe, and the measurements).
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.signal import resample_poly

import make_startup as ms
from make_startup import echo, fade_out, gain, lowpass, pitch, place, sfx
from soundlib import SR, db, kweight, loudest_100ms, measure, read, write_wav

LENGTH_S = 5.0

# One animation timeline for all three (seconds). The real animation is built in T1.
TIMELINE = [
    (0.00, "black screen (A9: black from the first frame)"),
    (0.40, "the mark appears (fades in and grows from 92 % to 100 %, 0.6 s)"),
    (1.40, "the name AIONE follows (fades in and rises 8 px, 0.5 s)"),
    (2.40, "both settle: a soft brightening of the mark (0.5 s)"),
    (4.20, "everything fades out (0.8 s)"),
    (5.00, "Home"),
]
T_MARK, T_NAME, T_SETTLE = 0.40, 1.40, 2.40


def fit_length(x, length_s=LENGTH_S, fade_ms=600):
    """Cuts or pads to exactly length_s, with a fade-out ending at length_s."""
    n = int(length_s * SR)
    if len(x) < n:
        x = np.concatenate([x, np.zeros((n - len(x), 2))])
    x = x[:n].copy()
    return fade_out(x, fade_ms)


def finish_boot(x):
    """Like the startup finish (loudness matched to the pack, gentle limiting, safe peaks),
    keeping the exact 5 s length."""
    y = ms.finish(x)
    return fit_length(y, fade_ms=30)


def peaks(x, min_gap_s=0.25, within_db=12):
    """Times (s) of the loudness peaks (K-weighted, 100 ms windows, 10 ms steps)."""
    y = kweight(x)
    power = (y ** 2).sum(axis=1)
    w, hop = int(0.1 * SR), int(0.01 * SR)
    c = np.concatenate([[0.0], np.cumsum(power)])
    starts = np.arange(0, len(power) - w, hop)
    lufs = -0.691 + 10 * np.log10(np.maximum((c[starts + w] - c[starts]) / w, 1e-12))
    top = lufs.max()
    found = []
    for i in range(1, len(lufs) - 1):
        if lufs[i] >= lufs[i - 1] and lufs[i] > lufs[i + 1] and lufs[i] > top - within_db:
            t = (starts[i] + w / 2) / SR
            if not found or t - found[-1][0] >= min_gap_s:
                found.append((round(t, 2), round(float(lufs[i]), 1)))
            elif lufs[i] > found[-1][1]:
                found[-1] = (round(t, 2), round(float(lufs[i]), 1))
    return found


def variant_a():
    """A, "Bloom": a swell into the sparkle with the mark, the chime with the name, a soft
    rising arpeggio as they settle, then a long echo."""
    swell, sparkle, chime, blip = sfx(48), sfx(44), sfx(43), sfx(9)
    parts = [
        (T_MARK - 0.12, gain(swell, -5)),           # peaks as the mark appears
        (T_MARK, sparkle),
        (T_NAME, gain(chime, -2)),
        (T_SETTLE, gain(pitch(blip, 0), -8)),
        (T_SETTLE + 0.12, gain(pitch(blip, 4), -8)),
        (T_SETTLE + 0.24, gain(pitch(blip, 7), -8)),
        (T_SETTLE + 0.36, gain(pitch(chime, -12), -9)),  # the chime an octave lower (C6)
    ]
    x = place(parts, LENGTH_S)
    x = echo(x, 230, 310, feedback=0.50, wet=0.30, tone_hz=4000, tail_s=0)
    return finish_boot(fit_length(x)), {
        "uses": ["MI_SFX 48.wav", "MI_SFX 44.wav", "MI_SFX 43.wav", "MI_SFX 09.wav"],
        "changes": [
            "#48 (swell) 5 dB down, starting 0.12 s before the mark so it peaks with it",
            "#44 (rising sparkle) at the mark (0.40 s)",
            "#43 (C7 chime) 2 dB down at the name (1.40 s)",
            "#09 (C5 blip) three times at the settle (2.40 s): as is, +4 and +7 semitones, 120 ms apart, 8 dB down",
            "#43 an octave lower (-12 semitones, C6), 9 dB down, closing the arpeggio",
            "stereo echo (230 / 310 ms, feedback 0.50, wet 0.30, darkened above 4 kHz) for the long tail",
            "cut to exactly 5.0 s with a 0.6 s fade-out; 60 Hz high-pass; loudness matched to the pack with gentle limiting",
        ],
        "description": "A soft swell blooms into a sparkle with the logo, a bright chime greets the name, and a gentle do-mi-sol settles it.",
    }


def variant_b():
    """B, "Steps": clear rising notes as the mark draws in, the chime with the name, a warm
    chord as they settle."""
    blip, chime, body = sfx(9), sfx(43), sfx(16)
    parts = [
        (T_MARK, gain(pitch(blip, 0), -2)),
        (T_MARK + 0.18, gain(pitch(blip, 4), -2)),
        (T_MARK + 0.36, gain(pitch(blip, 7), -2)),
        (T_MARK + 0.54, gain(pitch(blip, 12), -3)),       # C6
        (T_NAME, chime),                                  # C7
        (T_SETTLE, gain(lowpass(pitch(body, -4), 3000), -6)),   # C4, warm
        (T_SETTLE, gain(pitch(chime, -12), -6)),          # C6
        (T_SETTLE + 0.02, gain(pitch(blip, 7), -10)),     # G5 on top: a C major chord
    ]
    x = place(parts, LENGTH_S)
    x = echo(x, 200, 280, feedback=0.45, wet=0.28, tone_hz=4500, tail_s=0)
    return finish_boot(fit_length(x)), {
        "uses": ["MI_SFX 09.wav", "MI_SFX 43.wav", "MI_SFX 16.wav"],
        "changes": [
            "#09 (C5 blip) as four rising notes C5-E5-G5-C6 (0, +4, +7, +12 semitones), 180 ms apart from the mark (0.40 s), 2–3 dB down",
            "#43 (C7 chime) at the name (1.40 s)",
            "at the settle (2.40 s), a soft C major chord: #16 moved down 4 semitones to C4 and softened above 3 kHz (6 dB down), #43 an octave lower (C6, 6 dB down), #09 at G5 (10 dB down)",
            "stereo echo (200 / 280 ms, feedback 0.45, wet 0.28, darkened above 4.5 kHz)",
            "cut to exactly 5.0 s with a 0.6 s fade-out; 60 Hz high-pass; loudness matched to the pack with gentle limiting",
        ],
        "description": "Four clear rising notes draw the logo in, a bright chime names it, and a warm chord settles everything: the most melodic of the three.",
    }


def variant_c():
    """C, "Sweep": an upward zip into the sparkle with the mark, a bright double chime with the
    name, a fuller layered chord as they settle, with the longest tail."""
    zip_, sparkle, chime, body, blip = sfx(31), sfx(44), sfx(43), sfx(16), sfx(9)
    parts = [
        (T_MARK - 0.10, gain(zip_, -6)),
        (T_MARK, sparkle),
        (T_NAME, gain(chime, -1)),
        (T_NAME + 0.09, gain(pitch(chime, -5), -8)),       # a fourth lower, a soft double
        (T_SETTLE, gain(lowpass(pitch(body, -4), 3000), -4)),   # C4
        (T_SETTLE, gain(pitch(blip, 0), -7)),              # C5
        (T_SETTLE, gain(pitch(blip, 4), -9)),              # E5
        (T_SETTLE, gain(pitch(blip, 7), -9)),              # G5
        (T_SETTLE + 0.05, gain(sparkle, -9)),              # a little sparkle on top
    ]
    x = place(parts, LENGTH_S)
    x = echo(x, 250, 340, feedback=0.55, wet=0.33, tone_hz=3800, tail_s=0)
    return finish_boot(fit_length(x)), {
        "uses": ["MI_SFX 31.wav", "MI_SFX 44.wav", "MI_SFX 43.wav", "MI_SFX 16.wav", "MI_SFX 09.wav"],
        "changes": [
            "#31 (rising zip) 6 dB down, 0.10 s before the mark, leading into #44 (rising sparkle) at the mark (0.40 s)",
            "#43 (C7 chime) at the name (1.40 s), with a copy 5 semitones lower (G6) 90 ms later, 8 dB down",
            "at the settle (2.40 s), a layered C major chord: #16 moved down 4 semitones to C4 (softened, 4 dB down), #09 at C5, E5 and G5 (7–9 dB down), and #44 again, 9 dB down",
            "stereo echo (250 / 340 ms, feedback 0.55, wet 0.33, darkened above 3.8 kHz): the longest tail",
            "cut to exactly 5.0 s with a 0.6 s fade-out; 60 Hz high-pass; loudness matched to the pack with gentle limiting",
        ],
        "description": "A quick upward sweep lifts the logo in with a sparkle, a bright double chime names it, and a full chord rings out: the grandest of the three.",
    }


if __name__ == "__main__":
    ms.PACK_MEDIAN_LOUDEST_100MS = float(
        np.median([loudest_100ms(read(f)) for f in sorted(ms.PACK.glob("*.wav"))])
    )
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "bootup")
    out.mkdir(parents=True, exist_ok=True)
    result = {"timeline": TIMELINE, "variants": {}}
    for name, make in (("A", variant_a), ("B", variant_b), ("C", variant_c)):
        x, recipe = make()
        path = out / f"AIONE4_bootup_{name}.wav"
        write_wav(path, x)
        back = read(path)
        recipe["measurements"] = measure(back)
        recipe["peak_4x_dbfs"] = round(db(np.abs(resample_poly(x, 4, 1, axis=0)).max()), 2)
        recipe["limiter_reduction_db"] = ms.finish.last_reduction_db
        recipe["clipped_samples"] = int((np.abs(back) >= 0.9999).sum())
        recipe["length_s"] = round(len(back) / SR, 3)
        recipe["peaks_s_lufs"] = peaks(back)
        result["variants"][name] = recipe
        m = recipe["measurements"]
        print(name, recipe["length_s"], "s; loudest100", m["loudest_100ms_lufs"], "; peak4x", recipe["peak_4x_dbfs"],
              "; limit", recipe["limiter_reduction_db"], "; clipped", recipe["clipped_samples"], "; peaks", recipe["peaks_s_lufs"])
    (out / "bootup_timelines.json").write_text(json.dumps(result, indent=1))
