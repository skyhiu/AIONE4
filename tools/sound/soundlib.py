"""Shared helpers for AIONE's sound tools: read/write audio and measure it.

Requires ffmpeg (decoding), numpy, scipy, pyloudnorm (pip install pyloudnorm==0.1.1).
"""
import json
import subprocess

import numpy as np
import pyloudnorm as pyln

SR = 44100
NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]


def read(path, sr=SR):
    """Decode any audio file to float32 stereo (n, 2) at `sr` Hz."""
    raw = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", str(path), "-f", "f32le", "-ac", "2", "-ar", str(sr), "-"],
        check=True, capture_output=True,
    ).stdout
    return np.frombuffer(raw, dtype=np.float32).reshape(-1, 2).copy()


def write_wav(path, x, sr=SR):
    """Write float stereo as 24-bit PCM WAV."""
    x = np.clip(x, -1.0, 1.0).astype(np.float32)
    subprocess.run(
        ["ffmpeg", "-v", "error", "-y", "-f", "f32le", "-ac", "2", "-ar", str(sr), "-i", "-",
         "-c:a", "pcm_s24le", str(path)],
        input=x.tobytes(), check=True,
    )


def db(v):
    return 20 * np.log10(max(float(v), 1e-12))


def note(freq):
    if not freq or freq <= 0:
        return "-"
    n = int(round(12 * np.log2(freq / 440.0) + 69))
    return f"{NOTE_NAMES[n % 12]}{n // 12 - 1}"


def kweight(x, sr=SR):
    """BS.1770 K-weighting (the two pre-filters pyloudnorm uses), per channel."""
    meter = pyln.Meter(sr)
    y = x.astype(np.float64).copy()
    for f in meter._filters.values():  # high-shelf then high-pass
        for c in range(y.shape[1]):
            y[:, c] = f.apply_filter(y[:, c])
    return y


def loudness(x, sr=SR):
    """K-weighted loudness (LUFS) of the active part (above -40 dB of peak): the mean square
    over that part, without BS.1770's 400 ms gating blocks, which short UI sounds do not fill.
    Comparable between sounds of any length; for sounds over 0.4 s it is close to integrated
    loudness."""
    env = np.abs(x.mean(axis=1))
    peak = env.max() if len(env) else 0
    if peak <= 0:
        return -70.0
    idx = np.where(env > peak * 10 ** (-40 / 20))[0]
    y = kweight(x, sr)[idx[0]: idx[-1] + 1]
    ms = float((y ** 2).mean(axis=0).sum())
    return -0.691 + 10 * np.log10(max(ms, 1e-12))


def loudest_100ms(x, sr=SR):
    """K-weighted loudness (LUFS) of the loudest 100 ms: how loud a short sound feels at its
    strongest, comparable between a 0.1 s click and a 2 s start-up sound."""
    y = kweight(x, sr)
    w = int(0.1 * sr)
    if len(y) < w:
        y = np.concatenate([y, np.zeros((w - len(y), y.shape[1]))])
    power = (y ** 2).sum(axis=1)
    c = np.concatenate([[0.0], np.cumsum(power)])
    ms = (c[w:] - c[:-w]) / w
    return -0.691 + 10 * np.log10(max(float(ms.max()), 1e-12))


def measure(x, sr=SR):
    """Length, peak, loudness, envelope, brightness and pitch contour of a sound."""
    mono = x.mean(axis=1).astype(np.float64)
    n = len(mono)
    peak = np.abs(x).max()
    # Envelope: RMS in 5 ms windows.
    w = max(1, int(0.005 * sr))
    frames = n // w
    env = np.sqrt((mono[: frames * w].reshape(frames, w) ** 2).mean(axis=1)) if frames else np.zeros(1)
    env_db = 20 * np.log10(np.maximum(env, 1e-12))
    top = env_db.max()
    loud = np.where(env_db > top - 40)[0]
    start, end = (loud[0], loud[-1]) if len(loud) else (0, 0)
    ipk = int(env_db.argmax())
    attack_ms = (ipk - start) * 5
    after = np.where(env_db[ipk:] < top - 20)[0]
    decay20_ms = int(after[0] * 5) if len(after) else None
    tail_db = float(env_db[-4:].mean() - top) if frames >= 4 else 0.0
    # Spectrum: brightness (centroid) and pitch per 40 ms frame of the active part.
    hop, size = int(0.02 * sr), 2048
    win = np.hanning(size)
    freqs = np.fft.rfftfreq(size, 1 / sr)
    pitches, cents, weights = [], [], []
    for s0 in range(start * w, max(start * w + 1, end * w - size), hop):
        seg = mono[s0: s0 + size]
        if len(seg) < size:
            break
        mag = np.abs(np.fft.rfft(seg * win))
        e = float((seg ** 2).mean())
        if e <= 0:
            continue
        cents.append(float((freqs * mag).sum() / max(mag.sum(), 1e-12)))
        weights.append(e)
        band = (freqs > 80) & (freqs < 5000)
        # Harmonic product spectrum: the fundamental of tonal sounds.
        hps = mag.copy()
        for h in (2, 3):
            d = mag[::h]
            hps[: len(d)] *= d
        k = np.argmax(np.where(band, hps, 0))
        tonal = mag[k] / (mag[band].mean() + 1e-12)
        pitches.append(freqs[k] if tonal > 8 else None)
    centroid = float(np.average(cents, weights=weights)) if cents else 0.0
    tonal_pitches = [p for p in pitches if p]
    contour = "none (noisy)"
    p_start = p_end = None
    if len(tonal_pitches) >= 2:
        third = max(1, len(tonal_pitches) // 3)
        p_start = float(np.median(tonal_pitches[:third]))
        p_end = float(np.median(tonal_pitches[-third:]))
        semis = 12 * np.log2(p_end / p_start)
        contour = "rising" if semis > 1.5 else "falling" if semis < -1.5 else "steady"
    elif len(tonal_pitches) == 1:
        p_start = p_end = float(tonal_pitches[0])
        contour = "steady"
    return {
        "length_s": round(n / sr, 3),
        "active_s": round((end - start + 1) * 5 / 1000, 3),
        "peak_dbfs": round(db(peak), 1),
        "lufs": round(loudness(x, sr), 1),
        "loudest_100ms_lufs": round(loudest_100ms(x, sr), 1),
        "attack_ms": int(attack_ms),
        "decay20_ms": decay20_ms,
        "tail_db": round(tail_db, 1),
        "centroid_hz": int(centroid),
        "pitch_start": note(p_start),
        "pitch_end": note(p_end),
        "pitch_start_hz": round(p_start, 1) if p_start else None,
        "pitch_end_hz": round(p_end, 1) if p_end else None,
        "contour": contour,
        "tonal_share": round(len(tonal_pitches) / max(1, len(pitches)), 2),
    }


if __name__ == "__main__":
    import sys
    print(json.dumps(measure(read(sys.argv[1])), indent=2))
