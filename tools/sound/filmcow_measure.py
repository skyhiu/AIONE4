"""Measures the FilmCow menu-sound candidates (B-1 rev. h / j) at their own sample rate.

Usage: python3 tools/sound/filmcow_measure.py <candidates dir> <out.json>
<candidates dir> holds one folder per group (rec_1_move_taps, mech_2_select, ...), as unpacked
from the ZIPs the maintainer chose. The raw files are never committed (licence and size).

Per file: sample rate, channels, length; peak; room-noise floor (quietest 20 ms); the onset
(first 1 ms window within 30 dB of the peak); attack (onset to peak); decay (peak to −30 dB);
how many separate hits it holds (peaks ≥ 40 ms apart within 12 dB of the loudest); brightness
(spectral centroid of the first 50 ms); pitch of the first 80 ms (harmonic product spectrum, only
when the sound is clearly tonal); loudness of the active part (K-weighted).
"""
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
from soundlib import loudness, note  # noqa: E402


def probe(path):
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "a:0", "-show_entries",
         "stream=sample_rate,channels", "-of", "csv=p=0", str(path)],
        check=True, capture_output=True, text=True).stdout.strip().split(",")
    return int(out[0]), int(out[1])


def read_native(path):
    sr, ch = probe(path)
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", str(path), "-f", "f32le", "-ac", "2", "-"],
                         check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).reshape(-1, 2).astype(np.float64), sr, ch


def env_1ms(mono, sr):
    w = max(1, sr // 1000)
    n = len(mono) // w
    return np.sqrt((mono[: n * w].reshape(n, w) ** 2).mean(axis=1) + 1e-20), w


def measure(path):
    x, sr, ch = read_native(path)
    mono = x.mean(axis=1)
    env, w = env_1ms(mono, sr)
    edb = 20 * np.log10(env)
    top = edb.max()
    i_on = int(np.where(edb > top - 30)[0][0])
    i_pk = int(edb.argmax())
    after = np.where(edb[i_pk:] < top - 30)[0]
    decay = int(after[0]) if len(after) else len(edb) - i_pk
    # Room noise: the quietest 20 ms window anywhere.
    k = 20
    if len(env) > k:
        c = np.concatenate([[0.0], np.cumsum(env ** 2)])
        rms = np.sqrt((c[k:] - c[:-k]) / k)
        floor = 20 * np.log10(rms.min() + 1e-20)
    else:
        floor = -120.0
    # Separate hits.
    hits = []
    for i in range(1, len(edb) - 1):
        if edb[i] >= top - 12 and edb[i] >= edb[i - 1] and edb[i] > edb[i + 1]:
            if not hits or i - hits[-1] >= 40:
                hits.append(i)
    # Brightness and pitch near the onset.
    a = i_on * w
    seg = mono[a: a + int(0.05 * sr)]
    spec = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) if len(seg) > 64 else np.zeros(2)
    freqs = np.fft.rfftfreq(len(seg), 1 / sr) if len(seg) > 64 else np.zeros(2)
    centroid = float((freqs * spec).sum() / max(spec.sum(), 1e-12))
    seg2 = mono[a: a + int(0.08 * sr)]
    pitch = None
    if len(seg2) > 1024:
        size = 1 << int(np.log2(len(seg2)))
        mag = np.abs(np.fft.rfft(seg2[:size] * np.hanning(size)))
        f = np.fft.rfftfreq(size, 1 / sr)
        band = (f > 150) & (f < 4000)
        hps = mag.copy()
        for h in (2, 3):
            d = mag[::h]
            hps[: len(d)] *= d
        cand = np.where(band[: len(hps)])[0]
        if len(cand):
            j = cand[hps[cand].argmax()]
            # Tonal only if the peak clearly stands out of its neighbourhood.
            nb = mag[max(0, j - 40): j + 40]
            if mag[j] > 6 * np.median(nb):
                pitch = float(f[j])
    return {
        "file": path.name, "group": path.parent.name if path.parent.name.startswith(("rec_", "mech_")) else path.parts[-2],
        "sr": sr, "channels": ch, "length_ms": round(len(mono) / sr * 1000),
        "peak_dbfs": round(20 * np.log10(np.abs(x).max() + 1e-20), 1),
        "noise_floor_db": round(floor, 1), "onset_ms": i_on, "attack_ms": i_pk - i_on,
        "decay30_ms": decay, "hits": len(hits), "hit_ms": [h - i_on for h in hits[:6]],
        "centroid_hz": round(centroid), "pitch_hz": round(pitch, 1) if pitch else None,
        "note": note(pitch) if pitch else "-",
        "loudness_lufs": round(loudness(x.astype(np.float32), sr), 1),
    }


def main(src, out):
    rows = []
    for g in sorted(Path(src).iterdir()):
        if not g.is_dir():
            continue
        for f in sorted(g.rglob("*.wav")):
            try:
                r = measure(f)
            except Exception as e:  # noqa: BLE001 - reported, not hidden
                print(f"UNREADABLE {g.name}/{f.name}: {e}")
                continue
            r["group"] = g.name
            rows.append(r)
    Path(out).write_text(json.dumps(rows, indent=1))
    for r in rows:
        print(f'{r["group"]:20} {r["file"][:34]:34} {r["sr"]//1000:>3}k {r["length_ms"]:>5}ms on{r["onset_ms"]:>4} '
              f'att{r["attack_ms"]:>3} dec{r["decay30_ms"]:>4} hits{r["hits"]} floor{r["noise_floor_db"]:>6} '
              f'cen{r["centroid_hz"]:>6} {r["note"]:>4} {r["loudness_lufs"]:>6}')


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
