"""Builds AIONE's console-like menu-sound family from FilmCow recordings (B-1 rev. h / j).

Usage: python3 tools/sound/make_menu_family.py <candidates dir> <out dir> [<repo sounds dir>]
<candidates dir>: the unpacked candidate groups (rec_…, mech_…). Raw files are never committed.
<out dir>: every alternative (WAV), family.json (sources, processing, measurements) and the
listening page. <repo sounds dir>: if given, the wired set (alternative A of move, select, back)
is copied there as move.wav, select.wav, back.wav (the alternatives named in WIRED).

Processing of every layer (in this order), at the source's own rate (192 kHz for "mech", 48 kHz
for "rec"):
1. **Cut one hit:** the chosen hit (the first, the loudest, or hit N) from 1.5 ms before it rises
   above −24 dB of its peak, to the layer's length or 3 ms before the next hit, whichever is
   first; 0.3 ms fade-in.
2. **Clean:** 2nd-order high-pass (room rumble and handling), optional low-pass; a tight
   exponential decay after a short hold, and a gentle gate that fades the tail out where it
   falls within 10 dB of the file's own noise floor.
3. **Tune:** the strongest resonance between 400 Hz and 7 kHz is moved (tape-style resampling,
   so a higher note is also shorter) to the nearest octave of the layer's target note in
   A major, by at most ±6 semitones.
4. **Layer:** layers are summed at their gains and offsets.
Then for the sound: resampled to 48 kHz (polyphase, anti-aliased), mono, a 3 ms fade-out,
loudness matched on the loudest 30 ms (K-weighted) to the role's target, true peak (4×
oversampled) ≤ −1 dBFS by gain only (no limiter, no clipping). Written as 24-bit mono WAV.
"""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np
from scipy.signal import butter, lfilter, resample_poly

sys.path.insert(0, str(Path(__file__).parent))
from soundlib import kweight, note  # noqa: E402

OUT_SR = 48000
NOTE_HZ = {n: 440 * 2 ** ((i - 9) / 12) for i, n in enumerate(["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"])}


def read_native(path):
    sr = int(subprocess.run(["ffprobe", "-v", "error", "-select_streams", "a:0", "-show_entries", "stream=sample_rate",
                             "-of", "csv=p=0", str(path)], check=True, capture_output=True, text=True).stdout.strip())
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", str(path), "-f", "f64le", "-ac", "1", "-"],
                         check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float64).copy(), sr


def write_wav(path, x, sr=OUT_SR):
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "f64le", "-ac", "1", "-ar", str(sr), "-i", "-",
                    "-c:a", "pcm_s24le", str(path)], input=np.clip(x, -1, 1).astype(np.float64).tobytes(), check=True)


def envelope(x, sr, ms=0.5):
    w = max(1, int(sr * ms / 1000))
    n = len(x) // w
    return 20 * np.log10(np.sqrt((x[: n * w].reshape(n, w) ** 2).mean(axis=1)) + 1e-20), w


def noise_floor(x, sr):
    e, w = envelope(x, sr, 1)
    k = 20
    if len(e) <= k:
        return -120.0
    p = 10 ** (e / 10)
    c = np.concatenate([[0.0], np.cumsum(p)])
    return float(10 * np.log10(((c[k:] - c[:-k]) / k).min() + 1e-20))


def hits_of(x, sr):
    """Local envelope maxima within 18 dB of the loudest, at least 25 ms apart."""
    e, w = envelope(x, sr, 0.5)
    top = e.max()
    out = []
    for i in range(1, len(e) - 1):
        if e[i] >= top - 18 and e[i] >= e[i - 1] and e[i] > e[i + 1]:
            if out and i - out[-1][0] < 50:  # 25 ms in 0.5 ms steps
                if e[i] > out[-1][1]:
                    out[-1] = (i, e[i])
                continue
            out.append((i, e[i]))
    return [(i * w, v) for i, v in out], e, w


def cut(x, sr, pick, length_ms, ring=False):
    hits, e, w = hits_of(x, sr)
    if pick == "loudest":
        k = int(np.argmax([v for _, v in hits]))
    elif pick == "first":
        k = 0
    else:
        k = min(int(pick.split(":")[1]), len(hits) - 1)
    pk, pv = hits[k]
    # Start: where the hit rises above −24 dB of its own peak (searching back 15 ms), minus 1.5 ms.
    i = pk // w
    lim = max(0, i - int(15 / 0.5))
    while i > lim and e[i] > pv - 24:
        i -= 1
    start = max(0, i * w - int(0.0015 * sr))
    end = start + int(length_ms / 1000 * sr)
    # Normally the cut ends before the next hit; a ringing layer (`ring`) keeps its full length.
    if k + 1 < len(hits) and not ring:
        end = min(end, hits[k + 1][0] - int(0.003 * sr) - int(0.002 * sr))
    seg = x[start:end].copy()
    fi = max(1, int(0.0003 * sr))
    seg[:fi] *= np.linspace(0, 1, fi)
    # Every cut ends with its own 4 ms fade, so a cut before the next hit never clicks.
    fo = min(len(seg) // 2, int(0.004 * sr))
    seg[-fo:] *= np.linspace(1, 0, fo) ** 2
    return seg, {"hit": k + 1, "of_hits": len(hits), "start_ms": round(start / sr * 1000, 1),
                 "length_ms": round(len(seg) / sr * 1000, 1)}


def tighten(seg, sr, hold_ms, tau_ms, floor_db, gate=True):
    e, w = envelope(seg, sr, 0.5)
    pk = int(e.argmax()) * w
    t = np.arange(len(seg))
    g = np.ones(len(seg))
    h = pk + int(hold_ms / 1000 * sr)
    g[h:] = np.exp(-(t[h:] - h) / (tau_ms / 1000 * sr))
    # Gentle gate: where the (decayed) signal nears the noise floor, fade it out over 3 ms.
    y = seg * g
    if not gate:
        return y
    e2, w2 = envelope(y, sr, 1)
    quiet = np.where((e2 < floor_db + 10) & (np.arange(len(e2)) * w2 > pk))[0]
    if len(quiet):
        q = quiet[0] * w2
        f = int(0.003 * sr)
        y = y[: q + f]
        y[q:] *= np.linspace(1, 0, len(y) - q)
    return y


def dominant(seg, sr, lo=400, hi=7000):
    n = 1 << int(np.ceil(np.log2(max(len(seg), 256))))
    mag = np.abs(np.fft.rfft(seg * np.hanning(len(seg)), n))
    f = np.fft.rfftfreq(n, 1 / sr)
    band = (f > lo) & (f < hi)
    return float(f[band][mag[band].argmax()])


def tune_semitones(freq, target):
    """Semitones to move `freq` to the nearest octave of note `target` (within ±6). `target` may
    list several notes ("A|C#|E": a chord tone); the one needing the smallest move wins."""
    best = None
    for t in target.split("|"):
        s = 12 * np.log2(freq / NOTE_HZ[t])
        s -= 12 * np.round(s / 12)
        if best is None or abs(s) < abs(best):
            best = s
    return float(-best)


def shift(seg, sr, semis):
    if abs(semis) < 0.01:
        return seg
    ratio = 2 ** (semis / 12)
    up, down = 1000, int(round(1000 * ratio))
    return resample_poly(seg, up, down)


def hp(x, sr, hz):
    b, a = butter(2, hz / (sr / 2), "high")
    return lfilter(b, a, x)


def lp(x, sr, hz):
    b, a = butter(2, min(hz / (sr / 2), 0.99), "low")
    return lfilter(b, a, x)


def to48k(x, sr):
    if sr == OUT_SR:
        return x
    from math import gcd
    g = gcd(OUT_SR, sr)
    return resample_poly(x, OUT_SR // g, sr // g)


def loudest_30ms(x, sr=OUT_SR):
    y = kweight(np.stack([x, x], axis=1), sr).mean(axis=1)
    w = int(0.03 * sr)
    if len(y) < w:
        y = np.concatenate([y, np.zeros(w - len(y))])
    c = np.concatenate([[0.0], np.cumsum(y ** 2)])
    return float(-0.691 + 10 * np.log10(((c[w:] - c[:-w]) / w).max() + 1e-20) + 3.01)


def true_peak(x):
    return float(20 * np.log10(np.abs(resample_poly(x, 4, 1)).max() + 1e-20))


def build_layer(src, L):
    path = Path(src) / L["group"] / L["file"]
    x, sr = read_native(path)
    floor = noise_floor(x, sr)
    seg, info = cut(x, sr, L.get("pick", "loudest"), L["len"], L.get("ring", False))
    seg = hp(seg, sr, L.get("hp", 150))
    if L.get("lp"):
        seg = lp(seg, sr, L["lp"])
    # Long sounds (jingles, Home) keep their natural tail: no gate (the decay still applies).
    seg = tighten(seg, sr, L.get("hold", 4), L.get("tau", 18), floor, L.get("gate", True))
    # The band searched for the resonance to tune; low sounds (bumps, Home) search lower.
    lo, hi = L.get("band", (400, 7000))
    dom = dominant(seg, sr, lo, hi)
    semis = 0.0
    if L.get("note"):
        semis = tune_semitones(dom, L["note"])
    elif L.get("semis") is not None:
        semis = L["semis"]
    seg = shift(seg, sr, semis)
    seg = to48k(seg, sr)
    seg *= 10 ** (L.get("gain", 0) / 20)
    after = dom * 2 ** (semis / 12)
    return seg, {
        "source": f'{L["group"]}/{L["file"]}', "library": "FilmCow Recorded SFX" if L["group"].startswith("rec_") else
        "FilmCow SFX – Mechanisms & Movement (192 kHz)", "source_rate": sr, "noise_floor_db": round(floor, 1),
        **info, "resonance_hz": round(dom), "tuned_semitones": round(semis, 2),
        "tuned_to": f"{note(after)} ({round(after)} Hz)" if L.get("note") or semis else "not tuned",
        "high_pass_hz": L.get("hp", 150), "low_pass_hz": L.get("lp"), "decay_tau_ms": L.get("tau", 18),
        "hold_ms": L.get("hold", 4), "gain_db": L.get("gain", 0), "offset_ms": L.get("at", 0),
    }


def build(src, recipe):
    parts, info = [], []
    for L in recipe["layers"]:
        seg, i = build_layer(src, L)
        parts.append((int(L.get("at", 0) / 1000 * OUT_SR), seg))
        info.append(i)
    n = max(o + len(s) for o, s in parts)
    y = np.zeros(n)
    for o, s in parts:
        y[o: o + len(s)] += s
    f = int(0.003 * OUT_SR)
    y[-f:] *= np.linspace(1, 0, f)
    y *= 10 ** ((recipe["target"] - loudest_30ms(y)) / 20)
    tp = true_peak(y)
    if tp > -1.0:
        y *= 10 ** ((-1.0 - tp) / 20)
    return y, info


# AIONE's full sound set by function and place (B-1 rev. q), one family in A major:
# row move E (every repeat: tiny, bright, soft), drawer move A (a different tick), row <-> drawer
# C# (a change of place), edge bump low A (dull), confirm F# (a step above the move), back D (a
# step below), error F -> E, launch A, toggle on E / off C#, open a destination A-C#-E, Home low A.
# Targets: loudness of the loudest 30 ms (LUFS); the frequent sounds are the softest.
TARGET = {"wrap": -22.0, "options_open": -15.0, "options_close": -19.0, "scroll_tick": -32.0, "row_move": -24.0, "drawer_move": -23.0, "row_drawer": -21.0, "edge_bump": -34.0, "confirm": -18.0,
          "back": -19.0, "error": -19.0, "launch": -16.0, "toggle_on": -20.0, "toggle_off": -20.0,
          "open": -18.0, "home": -18.0}

R1, R2, R3, R4 = "rec_1_move_taps", "rec_2_select_confirm", "rec_3_back_close", "rec_4_soft_bumps"
M1, M2, M3, M4, M5, M6 = "mech_1_move", "mech_2_select", "mech_3_snap_launch", "mech_4_back_toggle", "mech_5_drawer", "mech_6_bumps"


def L(group, file, length, tau, note=None, **kw):
    d = {"group": group, "file": f"{file}.wav", "pick": kw.pop("pick", "loudest"), "len": length, "tau": tau,
         "hold": kw.pop("hold", 1), "hp": kw.pop("hp", 200)}
    if note:
        d["note"] = note
    d.update(kw)
    return d


RECIPES = {
    # rev. v: the click around 4.2 kHz like Switch's (C8, 4186 Hz; alpha.11's sat at 5.27 kHz).
    "row_move": [
        {"id": "A", "label": "Clicky button, tiny, 4.2 kHz", "layers": [L(R1, "clicky button 7", 9, 2.0, "C", pick="first", hp=1200, band=(3300, 5000))]},
        {"id": "B", "label": "Button pressed, tiny, 4.2 kHz", "layers": [L(R1, "button pressed 3", 9, 2.0, "C", hp=1200, band=(3300, 5000))]},
        {"id": "C", "label": "Ratchet tick, tiny, 4.2 kHz", "layers": [L(M1, "clicky squeaky knob 17", 8, 1.8, "C", hp=1800, band=(3300, 5000))]},
    ],
    "drawer_move": [
        {"id": "A", "label": "Mouse click", "layers": [L(R1, "mouse click 2", 14, 3.0, "A", hp=600)]},
        {"id": "B", "label": "Plastic switch", "layers": [L(R1, "plastic switch 2", 14, 3.0, "A", hp=600)]},
        {"id": "C", "label": "Tape recorder button", "layers": [L(M2, "tape recorder button 3", 16, 3.5, "A", hp=500)]},
    ],
    "row_drawer": [
        {"id": "A", "label": "Small latch, a change of place", "layers": [L(R3, "metal latches 2", 45, 10, "C#", hold=3, hp=250)]},
        {"id": "B", "label": "Sliding door, short", "layers": [L(R3, "small sliding door opens 2", 60, 16, "C#", pick="first", hold=6, hp=250)]},
        {"id": "C", "label": "Metal plate slide, short", "layers": [L(M5, "small metal plate slide 1", 60, 16, "C#", pick="first", hold=6, hp=250)]},
    ],
    # rev. s (measured): about 30 ms, about -10 dB below the move click, main about 2.65 kHz (E7).
    "edge_bump": [
        {"id": "A", "label": "Short soft tick, E7 (2.6 kHz)", "layers": [L(R1, "mouse click 2", 30, 7, "E", hold=1, hp=600, lp=6000, band=(1800, 3500))]},
        {"id": "B", "label": "Plastic switch, low tick", "layers": [L(R1, "plastic switch 2", 30, 7, "E", hold=1, hp=600, lp=6000, band=(1800, 3500))]},
        {"id": "C", "label": "Button pressed, soft", "layers": [L(R1, "button pressed 3", 30, 7, "E", hold=1, hp=600, lp=6000, band=(1800, 3500))]},
    ],
    # rev. u / v: the click (a little louder, about +2 dB) plus a soft low sweep (about 35 ms, about
    # 12 dB below the click) about 60 ms later. Alpha.11's (183 ms, as loud as the click) is gone.
    "wrap": [
        {"id": "A", "label": "Click + soft metal slide, about 1.3 kHz (60 ms later)", "layers": [
            L(R1, "clicky button 7", 9, 2.0, "C", pick="first", hp=1200, band=(3300, 5000)),
            L(M5, "small metal plate slide 1", 35, 10, "E", pick="first", hold=4, hp=500, lp=2400, band=(900, 1700), gain=-2, at=60, ring=True)]},
        {"id": "B", "label": "Click + soft sliding door (60 ms later)", "layers": [
            L(R1, "clicky button 7", 9, 2.0, "C", pick="first", hp=1200, band=(3300, 5000)),
            L(R3, "small sliding door opens 2", 35, 10, "E", pick="first", hold=4, hp=500, lp=2400, band=(900, 1700), gain=-2, at=60, ring=True)]},
    ],
    # rev. s: open about 50 ms, bright (about 3 kHz, F#7), the loudest; close about 50 ms, lower
    # (about 2.1 kHz, C#7) with a low tail.
    "options_open": [
        {"id": "A", "label": "Switch press, bright (F#7)", "layers": [L(R2, "switch press 3", 50, 13, "F#", hold=2, hp=800, band=(2000, 4500))]},
        {"id": "B", "label": "Tape button, bright (F#7)", "layers": [L(M2, "tape recorder button 3", 50, 13, "F#", hold=2, hp=800, band=(2000, 4500))]},
    ],
    "options_close": [
        {"id": "A", "label": "Metal switch (C#7) + low tail", "layers": [
            L(R3, "metal switch 1", 50, 12, "C#", hold=2, hp=400, band=(1500, 3000)),
            L(R4, "wood drop 1", 70, 22, "A", hold=3, hp=50, lp=800, band=(60, 500), gain=-8, at=3, ring=True)]},
        {"id": "B", "label": "Switch (C#7) + low tail", "layers": [
            L(R3, "switch 3", 50, 12, "C#", hold=2, hp=400, band=(1500, 3000)),
            L(M6, "spring medium hit 2", 70, 22, "A", hold=3, hp=50, lp=800, band=(60, 500), gain=-8, at=3, ring=True)]},
    ],
    # rev. u: the row-scroll tick (also the drag tick, B-16): soft and low (about 1.3-1.9 kHz, A6),
    # about 5-6 dB below the click; played 60-90 ms after the click when the row scrolls.
    "scroll_tick": [
        # The overtones are filtered out (lp 2.2 kHz), so the main energy really sits at about
        # 1.6 kHz like Switch's (alpha.12's first build measured 3.46 kHz: a second high click).
        # (The tool filters before tuning; a source pitched up moves its cut-off up with it, so A is
        # the one needing almost no shift.)
        {"id": "A", "label": "Soft low button tick, about 1.6 kHz", "layers": [L(R1, "button pressed 3", 16, 4, "G#", hp=600, lp=1700, band=(1200, 2200))]},
        {"id": "B", "label": "Soft low click, about 1.6 kHz", "layers": [L(R1, "mouse click 2", 16, 4, "G#", hp=600, lp=1400, band=(1200, 2200))]},
    ],
    "confirm": [
        {"id": "A", "label": "Button + glass (your select A)", "layers": [
            L(M2, "mechanical button 8", 50, 12, "F#", hold=3, hp=150),
            L(R2, "glass ding 4", 110, 35, "F#", pick="first", hold=4, hp=600, gain=-13, at=2)]},
        {"id": "B", "label": "Switch press + clink", "layers": [
            L(R2, "switch press 3", 45, 11, "F#", hold=3, hp=150),
            L(R2, "glass clink 2", 90, 30, "F#", pick="first", hold=3, hp=700, gain=-14, at=2)]},
    ],
    "back": [
        {"id": "A", "label": "Metal switch (your back B)", "layers": [L(R3, "metal switch 1", 48, 11, "D", hold=3, hp=150, lp=9000)]},
        {"id": "B", "label": "Switch, lower", "layers": [L(R3, "switch 3", 48, 11, "D", hold=3, hp=150, lp=8000)]},
    ],
    "error": [
        {"id": "A", "label": "Two wood bumps, falling", "layers": [
            L(R4, "wood hit 4", 60, 16, "F", hold=2, hp=120, lp=3000),
            L(R4, "wood hit 4", 70, 20, "E", hold=2, hp=120, lp=2800, at=95)]},
        {"id": "B", "label": "Two tin pops, falling", "layers": [
            L(M6, "tin pop 2", 50, 12, "F", hold=2, hp=150, lp=3500),
            L(M6, "tin pop 2", 60, 16, "E", hold=2, hp=150, lp=3300, at=95)]},
    ],
    # rev. ab: a bright pop of about 150-190 ms (Switch's measured about 190 ms), at once on A.
    # rev. ab: a bright pop of about 150-190 ms (Switch's measured about 190 ms), at once on A.
    # The glass ring carries the length (ring=True keeps its natural tail).
    "launch": [
        {"id": "A", "label": "Snap + glass ring (about 170 ms)", "layers": [
            L(M3, "mechanism snap together 6", 60, 14, "A", hold=2, hp=300, gain=-2),
            L(R2, "glass ding 4", 200, 85, "A|C#|E", pick="first", hold=10, hp=700, gain=-2, at=3, ring=True)]},
        {"id": "B", "label": "Switch press + glass (about 160 ms)", "layers": [
            L(R2, "switch press 3", 50, 12, "A", hold=2, hp=500, gain=-2),
            L(R2, "glass ding 4", 190, 75, "A|C#|E", pick="first", hold=8, hp=700, gain=-1, at=3, ring=True)]},
        {"id": "C", "label": "Snap + soft bell tail (about 190 ms)", "layers": [
            L(M3, "mechanism snap together 6", 60, 14, "A", hold=2, hp=300, gain=-3),
            L(R2, "glass ding 4", 230, 110, "A|C#|E", pick="first", hold=12, hp=500, lp=6000, gain=-3, at=5, ring=True)]},
    ],
    "toggle_on": [
        {"id": "A", "label": "Plastic toy snap (on)", "layers": [L(M4, "plastic toy snapping shut 2", 45, 10, "E", hold=2, hp=200)]},
        {"id": "B", "label": "Metal switch (on)", "layers": [L(M4, "metal switch 7", 40, 9, "E", hold=2, hp=200)]},
    ],
    "toggle_off": [
        {"id": "A", "label": "Plastic toy snap (off, lower)", "layers": [L(M4, "plastic toy snapping shut 2", 45, 10, "C#", hold=2, hp=200, lp=7000)]},
        {"id": "B", "label": "Metal switch (off, lower)", "layers": [L(M4, "metal switch 7", 40, 9, "C#", hold=2, hp=200, lp=7000)]},
    ],
    "open": [
        {"id": "A", "label": "Glass arpeggio A-C#-E (about 0.8 s)", "layers": [
            L(R2, "glass ding 4", 420, 150, "A", pick="first", hold=6, hp=400, gate=False, ring=True),
            L(R2, "glass ding 4", 420, 150, "C#", pick="first", hold=6, hp=400, at=120, gate=False, ring=True),
            L(R2, "glass ding 4", 560, 200, "E", pick="first", hold=6, hp=400, at=240, gate=False, ring=True)]},
        {"id": "B", "label": "Clink + ding (about 0.6 s)", "layers": [
            L(R2, "glass clink 2", 160, 60, "E", pick="first", hold=3, hp=500, gate=False, ring=True),
            L(R2, "ding 2", 520, 190, "A", pick="first", hold=6, hp=300, at=120, gate=False, ring=True)]},
    ],
    "home": [
        {"id": "A", "label": "Low thump + low glass A (about 0.5 s)", "layers": [
            L(R4, "wood drop 1", 160, 50, "A", hold=4, hp=50, lp=1200, band=(60, 500), ring=True),
            L(R2, "ding 2", 520, 180, "A", pick="first", hold=8, hp=120, lp=2500, gain=-5, at=10, band=(150, 1200), gate=False, ring=True)]},
        {"id": "B", "label": "Low spring + glass (about 0.5 s)", "layers": [
            L(M6, "spring medium hit 2", 160, 50, "A", hold=4, hp=50, lp=1200, band=(60, 500), ring=True),
            L(R2, "glass ding 4", 500, 170, "A", pick="first", hold=8, hp=150, lp=2500, gain=-6, at=10, band=(150, 1200), gate=False, ring=True)]},
    ],
}
# Wired in AIONE now (the ones Home uses); the others are for their stages.
WIRED = {"row_move": "A", "drawer_move": "A", "edge_bump": "A", "confirm": "A", "back": "A", "error": "A", "wrap": "A", "options_open": "A", "options_close": "A", "scroll_tick": "A", "launch": "A"}
# rev. r: no separate row <-> drawer sound (entering a section plays its own move sound).
PROPOSED_ONLY = ["row_drawer", "toggle_on", "toggle_off", "open", "home"]


def main(src, out, repo=None):
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    family = {"key": "A major", "note": __doc__.split("Usage")[0].strip(), "roles": {}}
    for role, alts in RECIPES.items():
        family["roles"][role] = []
        for alt in alts:
            y, layers = build(src, {"layers": alt["layers"], "target": TARGET[role]})
            name = f"{role}_{alt['id']}.wav"
            write_wav(out / name, y)
            m = {"file": name, "id": alt["id"], "label": alt["label"], "layers": layers,
                 "length_ms": round(len(y) / OUT_SR * 1000, 1), "loudest_30ms_lufs": round(loudest_30ms(y), 1),
                 "true_peak_dbfs": round(true_peak(y), 2), "sample_peak_dbfs": round(20 * np.log10(np.abs(y).max()), 2),
                 "wired": WIRED.get(role) == alt["id"], "proposal_only": role in PROPOSED_ONLY}
            family["roles"][role].append(m)
            print(f'{role:13} {alt["id"]} {m["length_ms"]:6.1f} ms  L30 {m["loudest_30ms_lufs"]:6.1f}  TP {m["true_peak_dbfs"]:6.2f}  '
                  + "; ".join(f'{l["source"].split("/")[1]} hit {l["hit"]}/{l["of_hits"]} {l["tuned_to"]} ({l["tuned_semitones"]:+.2f})' for l in layers))
    (out / "family.json").write_text(json.dumps(family, indent=1))
    if repo:
        repo = Path(repo)
        repo.mkdir(parents=True, exist_ok=True)
        for role, alt in WIRED.items():
            shutil.copy(out / f"{role}_{alt}.wav", repo / f"{role}.wav")
        (repo / "family.json").write_text(json.dumps(
            {r: [a for a in family["roles"][r] if a["wired"]] for r in WIRED}, indent=1))
    page(out, family)
    return family


ROLE_TEXT = {
    "row_move": "Each move in the game row, every hold repeat included (tiny, bright, soft).",
    "drawer_move": "Each move in the drawer (a different tick from the row's).",
    "row_drawer": "Moving between the game row and the drawer (a change of place).",
    "edge_bump": "Every blocked move: the ring bumps (rev. s: about 30 ms, about -10 dB below the click, lower).",
    "wrap": "The row whipping to its other end (rev. s: louder than the click, about 90 ms, a low tail).",
    "options_open": "Opening a game's options with the pad's Menu or Tab (rev. s: bright, the loudest).",
    "options_close": "Closing the options (rev. s: lower, with a low tail).",
    "scroll_tick": "Added 60-90 ms after the click when the row itself scrolls; also the drag tick (B-16).",
    "confirm": "A on a drawer button or in a dialog.", "back": "B (a step lower than confirm).",
    "error": "Not available, e.g. the \"Coming in a later stage\" toast.",
    "launch": "Starting a game (T1.3).", "toggle_on": "A toggle turned on (T2+).", "toggle_off": "A toggle turned off (T2+).",
    "open": "Opening a destination such as Settings (from its stage).",
    "home": "Guide brings AIONE back, or returning to Home.",
}


def page(out, family):
    import html
    rows = []
    for role, alts in family["roles"].items():
        tag = "proposal for later, not wired" if alts[0]["proposal_only"] else "wired in alpha.5: A"
        rows.append(f'<section><h2>{role.replace("_", " ")} <small>({tag})</small></h2><p class="note">{ROLE_TEXT[role]}</p>')
        for a in alts:
            layers = "<br>".join(
                html.escape(f'{l["library"]}: "{l["source"].split("/")[1]}" (group {l["source"].split("/")[0]}), '
                            f'hit {l["hit"]} of {l["of_hits"]}, {l["length_ms"]} ms cut; high-pass {l["high_pass_hz"]} Hz'
                            + (f', low-pass {l["low_pass_hz"]} Hz' if l["low_pass_hz"] else "")
                            + f'; decay {l["decay_tau_ms"]} ms after {l["hold_ms"]} ms; '
                            + (f'tuned {l["tuned_semitones"]:+.2f} semitones to {l["tuned_to"]}' if l["tuned_semitones"] else "not tuned")
                            + (f'; {l["gain_db"]} dB' if l["gain_db"] else "") + (f', at +{l["offset_ms"]} ms' if l["offset_ms"] else ""))
                for l in a["layers"])
            star = " ★ wired" if a["wired"] else ""
            rows.append(
                f'<div class="alt"><button data-src="{a["file"]}">▶ {a["id"]}</button><div><b>{html.escape(a["label"])}{star}</b>'
                f' <span class="m">{a["length_ms"]} ms · loudest 30 ms {a["loudest_30ms_lufs"]} LUFS · true peak {a["true_peak_dbfs"]} dBFS</span>'
                f'<div class="src">{layers}</div></div></div>')
        rows.append("</section>")
    tpl = (Path(__file__).parent / "menu_family_page.html").read_text()
    (out / "index.html").write_text(tpl.replace("<!--ROWS-->", "\n".join(rows)))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], sys.argv[3] if len(sys.argv) > 3 else None)
