"""Builds the three startup-sound variants from the 50 Menu Interface SFX pack (B-1, rev. c).

Usage: python3 tools/sound/make_startup.py <output folder>   (from the repository root)
Only files of the pack are used; every edit is listed in the printed recipe.
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.signal import butter, lfilter, resample_poly

from soundlib import SR, db, measure, read, write_wav

PACK = Path("assets/sounds/50-menu-interface-sfx")
# Loudness is matched on the loudest 100 ms (see soundlib.loudest_100ms): the target is the
# median of the pack's 50 sounds, measured by tools/sound/analyse.py, minus 2 dB so the start-up
# sound is never louder than the menu sounds around it.
PACK_MEDIAN_LOUDEST_100MS = None  # filled in at run time from the pack
PEAK_LIMIT = -1.0     # dBFS, after 4x oversampling (true-peak estimate)
MAX_LIMITING_DB = 4.0  # the limiter may take at most this much off the loudest peaks


def sfx(n):
    x = read(PACK / f"MI_SFX {n:02d}.wav")
    # Trim the silent padding after the sound (every file is 0.56 s long).
    env = np.abs(x).max(axis=1)
    last = np.where(env > env.max() * 10 ** (-70 / 20))[0][-1]
    return x[: last + 1]


def pitch(x, semitones):
    """Tape-style pitch change (length changes with it)."""
    if semitones == 0:
        return x
    ratio = 2 ** (semitones / 12)
    up, down = 1000, int(round(1000 * ratio))
    return np.stack([resample_poly(x[:, c], up, down) for c in range(2)], axis=1)


def gain(x, decibels):
    return x * 10 ** (decibels / 20)


def lowpass(x, hz):
    b, a = butter(2, hz / (SR / 2), "low")
    return lfilter(b, a, x, axis=0)


def highpass(x, hz):
    b, a = butter(2, hz / (SR / 2), "high")
    return lfilter(b, a, x, axis=0)


def place(parts, length_s):
    """Mixes (start seconds, sound) pairs into one buffer of length_s."""
    out = np.zeros((int(length_s * SR), 2))
    for start, s in parts:
        i = int(start * SR)
        n = min(len(s), len(out) - i)
        out[i: i + n] += s[:n]
    return out


def echo(x, left_ms, right_ms, feedback, wet, tone_hz, tail_s):
    """Light stereo echo: two feedback delays (one per side), darkened each repeat."""
    out = np.concatenate([x, np.zeros((int(tail_s * SR), 2))])
    dry = out.copy()
    for c, ms in enumerate((left_ms, right_ms)):
        d = int(ms * SR / 1000)
        buf = lowpass(dry[:, c: c + 1], tone_hz)[:, 0] * wet
        acc = np.zeros(len(out))
        tap = buf
        for _ in range(12):
            shifted = np.zeros(len(out))
            shifted[d:] = tap[:-d]
            acc += shifted
            tap = lowpass(shifted[:, None], tone_hz)[:, 0] * feedback
            if np.abs(tap).max() < 1e-5:
                break
        out[:, c] += acc
    return out


def fade_out(x, ms):
    n = int(ms * SR / 1000)
    x = x.copy()
    x[-n:] *= np.linspace(1, 0, n)[:, None] ** 2
    return x


def limit(x, ceiling_db, release_ms=80, lookahead_ms=5):
    """Look-ahead peak limiter: gain never above what keeps peaks under the ceiling; the gain
    falls within the look-ahead window and recovers over release_ms."""
    ceiling = 10 ** (ceiling_db / 20)
    peak = np.abs(x).max(axis=1)
    need = np.minimum(1.0, ceiling / np.maximum(peak, 1e-12))
    la = int(lookahead_ms * SR / 1000)
    # Minimum over the look-ahead window (so the gain is already down when the peak arrives).
    padded = np.concatenate([need, np.ones(la)])
    g = np.array([padded[i: i + la + 1].min() for i in range(len(need))])
    rel = np.exp(-1 / (release_ms * SR / 1000))
    out = np.empty_like(g)
    cur = 1.0
    for i, v in enumerate(g):
        cur = v if v < cur else v + (cur - v) * rel
        out[i] = cur
    return x * out[:, None], 20 * np.log10(max(out.min(), 1e-12))


def finish(x):
    """Trims the tail below -60 dB, fades the last 120 ms, matches the pack's loudness on the
    loudest 100 ms, limits peaks gently (at most MAX_LIMITING_DB), keeps true peaks safe."""
    from soundlib import loudest_100ms
    env = np.abs(x).max(axis=1)
    last = np.where(env > env.max() * 10 ** (-60 / 20))[0][-1]
    x = fade_out(x[: last + 1], 120)
    x = highpass(x, 60)  # no sub-bass rumble
    target = PACK_MEDIAN_LOUDEST_100MS - 2.0
    x = gain(x, target - loudest_100ms(x))
    # Peaks: limit up to MAX_LIMITING_DB; beyond that, turn the whole sound down instead.
    over = db(np.abs(x).max()) - PEAK_LIMIT
    if over > MAX_LIMITING_DB:
        x = gain(x, -(over - MAX_LIMITING_DB))
    x, reduction = limit(x, PEAK_LIMIT - 0.3)
    tp = db(np.abs(resample_poly(x, 4, 1, axis=0)).max())
    if tp > PEAK_LIMIT:
        x = gain(x, PEAK_LIMIT - tp)
    finish.last_reduction_db = round(-reduction, 1)
    return x


def variant_a():
    """A: #44 alone, lightly shaped (light echo, longer gentle tail)."""
    s = sfx(44)
    x = echo(s, 190, 260, feedback=0.38, wet=0.32, tone_hz=4500, tail_s=1.2)
    return finish(x), {
        "uses": ["MI_SFX 44.wav"],
        "changes": [
            "trimmed the silent padding",
            "light stereo echo (190 ms left, 260 ms right, feedback 0.38, wet 0.32, each repeat darkened above 4.5 kHz)",
            "120 ms fade-out at the end; 60 Hz high-pass",
            "loudness matched to the pack (loudest 100 ms: pack median minus 2 dB), gentle peak limiting, peaks below -1 dBFS",
        ],
        "description": "The pack's rising sparkle on its own, with a soft echo that lets it settle instead of stopping.",
    }


def variant_b():
    """B: a short rising C-major arpeggio from #09 (C5 blip) ending on #43 (C7 chime)."""
    blip = sfx(9)
    notes = [(0.00, pitch(blip, 0)), (0.11, pitch(blip, 4)), (0.22, pitch(blip, 7))]
    chime = sfx(43)
    parts = [(t, gain(s, -3)) for t, s in notes] + [(0.33, chime)]
    x = place(parts, 0.33 + len(chime) / SR + 0.05)
    x = echo(x, 160, 220, feedback=0.30, wet=0.25, tone_hz=5000, tail_s=0.9)
    return finish(x), {
        "uses": ["MI_SFX 09.wav", "MI_SFX 43.wav"],
        "changes": [
            "#09 (a bright blip measured at C5) played three times: as is, 4 semitones up and 7 semitones up "
            "(tape-style pitch change, so the higher notes are slightly shorter), 110 ms apart, each 3 dB quieter",
            "#43 (a chime whose main tone measures at C7, two octaves above) as the last note, 330 ms in",
            "light stereo echo (160 / 220 ms, feedback 0.30, wet 0.25, darkened above 5 kHz)",
            "120 ms fade-out; 60 Hz high-pass",
            "loudness matched to the pack (loudest 100 ms: pack median minus 2 dB), gentle peak limiting, peaks below -1 dBFS",
        ],
        "description": "Three quick rising notes (do-mi-sol) landing on a bright chime: the most 'console start-up' of the three.",
    }


def variant_c():
    """C: fuller layer: #48 swell into #44 sparkle and #43 chime, with a warm #16 body."""
    swell = sfx(48)
    sparkle = sfx(44)
    chime = sfx(43)
    body = lowpass(pitch(sfx(16), -4), 3000)  # E4 tone moved to C4, softened
    t_hit = 0.11  # the swell peaks at about 110-120 ms
    parts = [
        (0.0, gain(swell, -4)),
        (t_hit, sparkle),
        (t_hit + 0.06, gain(chime, -2)),
        (t_hit, gain(body, -8)),
    ]
    length = t_hit + max(len(sparkle), len(chime), len(body)) / SR + 0.1
    x = place(parts, length)
    x = echo(x, 210, 290, feedback=0.42, wet=0.30, tone_hz=4000, tail_s=1.6)
    return finish(x), {
        "uses": ["MI_SFX 48.wav", "MI_SFX 44.wav", "MI_SFX 43.wav", "MI_SFX 16.wav"],
        "changes": [
            "#48 (a swell that builds for about 0.1 s) as the lead-in, 4 dB quieter",
            "#44 (rising sparkle) starts where the swell peaks (110 ms); #43 (C7 chime) 60 ms after it, 2 dB quieter",
            "#16 (a warm tone measured at E4) moved down 4 semitones to C4, softened above 3 kHz, 8 dB quieter, as a low body",
            "stereo echo (210 / 290 ms, feedback 0.42, wet 0.30, darkened above 4 kHz)",
            "120 ms fade-out; 60 Hz high-pass",
            "loudness matched to the pack (loudest 100 ms: pack median minus 2 dB), gentle peak limiting, peaks below -1 dBFS",
        ],
        "description": "A short swell that blooms into the sparkle and chime over a soft low tone, with a longer echo tail: the fullest of the three.",
    }


if __name__ == "__main__":
    from soundlib import loudest_100ms
    PACK_MEDIAN_LOUDEST_100MS = float(np.median([loudest_100ms(read(f)) for f in sorted(PACK.glob("*.wav"))]))
    print("pack median loudest-100ms LUFS:", round(PACK_MEDIAN_LOUDEST_100MS, 1))
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "startup")
    out.mkdir(parents=True, exist_ok=True)
    recipes = {}
    for name, make in (("A", variant_a), ("B", variant_b), ("C", variant_c)):
        x, recipe = make()
        path = out / f"AIONE4_startup_{name}.wav"
        write_wav(path, x)
        recipe["measurements"] = measure(read(path))
        recipe["peak_4x_dbfs"] = round(db(np.abs(resample_poly(x, 4, 1, axis=0)).max()), 2)
        recipe["limiter_reduction_db"] = finish.last_reduction_db
        recipe["clipped_samples"] = int((np.abs(read(path)) >= 0.9999).sum())
        recipes[name] = recipe
    (out / "startup_recipes.json").write_text(json.dumps(recipes, indent=1))
    for k, r in recipes.items():
        m = r["measurements"]
        print(k, json.dumps({kk: m[kk] for kk in ("length_s", "active_s", "peak_dbfs", "lufs", "loudest_100ms_lufs", "attack_ms", "centroid_hz", "pitch_start", "pitch_end", "contour")}), "peak4x", r["peak_4x_dbfs"], "limit", r["limiter_reduction_db"], "clipped", r["clipped_samples"])
