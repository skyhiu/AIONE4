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


def cut(x, sr, pick, length_ms):
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
    if k + 1 < len(hits):
        end = min(end, hits[k + 1][0] - int(0.003 * sr) - int(0.002 * sr))
    seg = x[start:end].copy()
    fi = max(1, int(0.0003 * sr))
    seg[:fi] *= np.linspace(0, 1, fi)
    # Every cut ends with its own 4 ms fade, so a cut before the next hit never clicks.
    fo = min(len(seg) // 2, int(0.004 * sr))
    seg[-fo:] *= np.linspace(1, 0, fo) ** 2
    return seg, {"hit": k + 1, "of_hits": len(hits), "start_ms": round(start / sr * 1000, 1),
                 "length_ms": round(len(seg) / sr * 1000, 1)}


def tighten(seg, sr, hold_ms, tau_ms, floor_db):
    e, w = envelope(seg, sr, 0.5)
    pk = int(e.argmax()) * w
    t = np.arange(len(seg))
    g = np.ones(len(seg))
    h = pk + int(hold_ms / 1000 * sr)
    g[h:] = np.exp(-(t[h:] - h) / (tau_ms / 1000 * sr))
    # Gentle gate: where the (decayed) signal nears the noise floor, fade it out over 3 ms.
    y = seg * g
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
    seg, info = cut(x, sr, L.get("pick", "loudest"), L["len"])
    seg = hp(seg, sr, L.get("hp", 150))
    if L.get("lp"):
        seg = lp(seg, sr, L["lp"])
    seg = tighten(seg, sr, L.get("hold", 4), L.get("tau", 18), floor)
    dom = dominant(seg, sr)
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


# A major family: move E, select F# (a step above), back D (a step below), launch A, error F → E.
TARGET = {"move": -20.0, "select": -18.0, "back": -19.0, "launch": -16.0, "drawer_open": -21.0,
          "drawer_close": -21.0, "end_of_row": -22.0, "toggle": -19.0, "error": -18.0}

RECIPES = {
    "move": [
        {"id": "A", "label": "Ratchet tick", "layers": [
            {"group": "mech_1_move", "file": "clicky squeaky knob 17.wav", "pick": "loudest", "len": 28, "tau": 6, "hold": 1.5, "hp": 300, "note": "E"}]},
        {"id": "B", "label": "Clicky button", "layers": [
            {"group": "rec_1_move_taps", "file": "clicky button 7.wav", "pick": "first", "len": 30, "tau": 7, "hold": 1.5, "hp": 250, "note": "E"}]},
        {"id": "C", "label": "Toy snap tick", "layers": [
            {"group": "mech_2_select", "file": "plastic toy mechanism snap 6.wav", "pick": "loudest", "len": 25, "tau": 5, "hold": 1, "hp": 300, "note": "E"}]},
    ],
    "select": [
        {"id": "A", "label": "Button + glass", "layers": [
            {"group": "mech_2_select", "file": "mechanical button 8.wav", "pick": "loudest", "len": 60, "tau": 14, "hold": 3, "hp": 150, "note": "F#"},
            {"group": "rec_2_select_confirm", "file": "glass ding 4.wav", "pick": "first", "len": 140, "tau": 45, "hold": 4, "hp": 600, "note": "F#", "gain": -13, "at": 2}]},
        {"id": "B", "label": "Metal pop-up", "layers": [
            {"group": "rec_2_select_confirm", "file": "metal button pop up 7.wav", "pick": "loudest", "len": 70, "tau": 16, "hold": 3, "hp": 200, "note": "F#"}]},
        {"id": "C", "label": "Light switch", "layers": [
            {"group": "rec_2_select_confirm", "file": "light switch on.wav", "pick": "loudest", "len": 60, "tau": 14, "hold": 3, "hp": 150, "note": "F#"}]},
    ],
    "back": [
        {"id": "A", "label": "Old metal switch", "layers": [
            {"group": "mech_4_back_toggle", "file": "old metal switch 1.wav", "pick": "loudest", "len": 55, "tau": 12, "hold": 3, "hp": 150, "lp": 9000, "note": "D"}]},
        {"id": "B", "label": "Metal switch (rec)", "layers": [
            {"group": "rec_3_back_close", "file": "metal switch 1.wav", "pick": "loudest", "len": 50, "tau": 11, "hold": 3, "hp": 150, "lp": 9000, "note": "D"}]},
        {"id": "C", "label": "Soft button, lower", "layers": [
            {"group": "mech_2_select", "file": "mechanical button 7.wav", "pick": "loudest", "len": 55, "tau": 12, "hold": 3, "hp": 120, "lp": 6000, "note": "D"}]},
    ],
    "launch": [
        {"id": "A", "label": "Snap + glass on A", "layers": [
            {"group": "mech_3_snap_launch", "file": "mechanism snap together 6.wav", "pick": "loudest", "len": 120, "tau": 30, "hold": 4, "hp": 120, "note": "A"},
            {"group": "rec_2_select_confirm", "file": "glass ding 4.wav", "pick": "first", "len": 320, "tau": 110, "hold": 6, "hp": 500, "note": "A|C#|E", "gain": -9, "at": 4}]},
        {"id": "B", "label": "Tape eject", "layers": [
            {"group": "mech_3_snap_launch", "file": "tape recorder eject 5.wav", "pick": "loudest", "len": 160, "tau": 40, "hold": 6, "hp": 120, "note": "A"}]},
    ],
    "drawer_open": [
        {"id": "A", "label": "Case slide on", "layers": [
            {"group": "mech_5_drawer", "file": "plastic case slide on 9.wav", "pick": "first", "len": 200, "tau": 60, "hold": 20, "hp": 200}]},
        {"id": "B", "label": "Lens extend", "layers": [
            {"group": "mech_5_drawer", "file": "camera lens extend 2.wav", "pick": "first", "len": 220, "tau": 70, "hold": 30, "hp": 200}]},
    ],
    "drawer_close": [
        {"id": "A", "label": "Case slide off", "layers": [
            {"group": "mech_5_drawer", "file": "plastic case slide off 1.wav", "pick": "loudest", "len": 120, "tau": 30, "hold": 6, "hp": 200}]},
        {"id": "B", "label": "Lens contract", "layers": [
            {"group": "mech_5_drawer", "file": "camera lens contract 4.wav", "pick": "loudest", "len": 160, "tau": 40, "hold": 8, "hp": 200}]},
    ],
    "end_of_row": [
        {"id": "A", "label": "Soft wood bump", "layers": [
            {"group": "rec_4_soft_bumps", "file": "wood hit 4.wav", "pick": "loudest", "len": 60, "tau": 14, "hold": 2, "hp": 120, "lp": 3500, "note": "A|C#|E"}]},
        {"id": "B", "label": "Tin pop, muted", "layers": [
            {"group": "mech_6_bumps", "file": "tin pop 5.wav", "pick": "loudest", "len": 50, "tau": 12, "hold": 2, "hp": 150, "lp": 4000, "note": "A|C#|E"}]},
    ],
    "toggle": [
        {"id": "A", "label": "Metal switch", "layers": [
            {"group": "mech_4_back_toggle", "file": "metal switch 7.wav", "pick": "loudest", "len": 40, "tau": 9, "hold": 2, "hp": 200, "note": "E"}]},
        {"id": "B", "label": "Metal latch (rec)", "layers": [
            {"group": "rec_2_select_confirm", "file": "metal latch 4.wav", "pick": "loudest", "len": 45, "tau": 10, "hold": 2, "hp": 200, "note": "E"}]},
    ],
    "error": [
        {"id": "A", "label": "Two wood bumps, falling", "layers": [
            {"group": "rec_4_soft_bumps", "file": "wood hit 4.wav", "pick": "loudest", "len": 60, "tau": 16, "hold": 2, "hp": 120, "lp": 3000, "note": "F"},
            {"group": "rec_4_soft_bumps", "file": "wood hit 4.wav", "pick": "loudest", "len": 70, "tau": 20, "hold": 2, "hp": 120, "lp": 2800, "note": "E", "at": 95}]},
        {"id": "B", "label": "Two tin pops, falling", "layers": [
            {"group": "mech_6_bumps", "file": "tin pop 2.wav", "pick": "loudest", "len": 50, "tau": 12, "hold": 2, "hp": 150, "lp": 3500, "note": "F"},
            {"group": "mech_6_bumps", "file": "tin pop 2.wav", "pick": "loudest", "len": 60, "tau": 16, "hold": 2, "hp": 150, "lp": 3300, "note": "E", "at": 95}]},
    ],
}
# Chosen from the measurements and the spectrograms (crisp single transient, natural decay):
WIRED = {"move": "B", "select": "A", "back": "B"}
PROPOSED_ONLY = ["launch", "drawer_open", "drawer_close", "end_of_row", "toggle", "error"]


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
    "move": "Moving the focus (every press and paced repeat).", "select": "Confirming (A / Enter, a second click).",
    "back": "Going back or closing (B / Esc).", "launch": "Starting a game (T1.3).",
    "drawer_open": "Opening the drawer (T1.2).", "drawer_close": "Closing the drawer (T1.2).",
    "end_of_row": "A press at the end of a row (nothing to move to).", "toggle": "Flipping a toggle (Settings, T3).",
    "error": "Something cannot be done.",
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
