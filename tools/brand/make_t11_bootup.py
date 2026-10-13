"""T1.1 logo and Boot Up proposals (B-5 rev. e): three "all in one" marks, each with a 5 s Boot
Up whose every sound hit is a visual event.

Method (beat for beat):
1. Each sound is composed from the 50 Menu Interface SFX pack (B-1): every hit is one pack sound
   (pitched, layered or echoed) placed at a planned time.
2. Every hit is then MEASURED in the rendered sound: the hit's own stem goes through the same
   chain, and its beat is the 5 ms-RMS envelope peak (where the ear hears the hit). A second,
   independent measure (spectral-flux onsets on the finished mix) cross-checks it.
3. The animation's events are built from the measured beat times, not the planned ones.

Usage (repository root): python3 tools/brand/make_t11_bootup.py <out dir>
Writes <id>.wav, <id>-mark.svg, <id>-lockup-dark.svg, <id>-lockup-light.svg, beats.json,
preview.html (+ the font and menu sounds it uses). PNG previews come from Chromium
(tools/brand/shoot_t11.mjs).
"""
import json
import shutil
import sys
from pathlib import Path

import numpy as np
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "sound"))
import make_startup as ms  # noqa: E402
from make_startup import echo, gain, lowpass, pitch, sfx  # noqa: E402
from soundlib import SR, loudest_100ms, write_wav  # noqa: E402

LENGTH_S = 5.0
WHITE, TEAL, INK = "#FFFFFF", "#1BB8CC", "#2D2D2D"
FONT = ROOT / "assets/fonts/m-plus-rounded-1c/MPLUSRounded1c-Medium.ttf"


# ---------------------------------------------------------------------------------------
# Sound

def fit(x, length_s=LENGTH_S):
    n = int(length_s * SR)
    if len(x) < n:
        x = np.concatenate([x, np.zeros((n - len(x), 2))])
    return x[:n]


def render(hits, tail):
    """hits: list of (planned_s, name, sound). Returns the finished mix and each hit's stem
    rendered alone through the same chain (for measuring)."""
    def chain(parts):
        out = np.zeros((int(LENGTH_S * SR) + SR, 2))
        for t, s in parts:
            i = int(round(t * SR))
            n = min(len(s), len(out) - i)
            out[i: i + n] += s[:n]
        return tail(out)

    mix = chain([(t, s) for t, _, s in hits])
    stems = [chain([(t, s)]) for t, _, s in hits]
    return mix, stems


def finish(mix, stems):
    """Loudness like the pack (loudest 100 ms at the pack median − 2 dB), gentle limiting,
    true peaks ≤ −1 dBFS, exactly 5.0 s with a 400 ms fade at the end. The stems get the same
    overall gain (the limiter changes level, never timing)."""
    before = loudest_100ms(fit(mix))
    y = fit(ms.finish(mix.copy()))
    after = loudest_100ms(y)
    g = after - before
    n = int(0.4 * SR)
    y[-n:] *= np.linspace(1, 0, n)[:, None] ** 2
    return y, [fit(gain(s, g)) for s in stems], ms.finish.last_reduction_db


def beat_of(stem, planned):
    """The hit's beat (perceptual attack): the first moment its 5 ms RMS envelope, sampled every
    1 ms, reaches half (−6 dB) of its peak within 400 ms after its planned start. For a click
    this is within a few ms of its peak; for a chime or a swell it sits between its start and
    its peak, so the visual event is within one frame of both. Returns (beat, peak) in s."""
    mono = stem.mean(axis=1)
    a = int(planned * SR)
    seg = mono[a: a + int(0.4 * SR)] ** 2
    w, hop = int(0.005 * SR), int(0.001 * SR)
    c = np.concatenate([[0.0], np.cumsum(seg)])
    starts = np.arange(0, len(seg) - w, hop)
    env = np.sqrt((c[starts + w] - c[starts]) / w)
    i_peak = int(env.argmax())
    i_beat = int(np.where(env >= 0.5 * env[i_peak])[0][0])
    centre = lambda i: planned + (starts[i] + w / 2) / SR  # noqa: E731
    return centre(i_beat), centre(i_peak)


def onsets(x, thresh=0.25):
    """Spectral-flux onsets (s) of the finished mix: an independent cross-check."""
    mono = x.mean(axis=1)
    size, hop = 1024, 220  # 5 ms hop
    win = np.hanning(size)
    prev = None
    flux = []
    for i in range(0, len(mono) - size, hop):
        mag = np.abs(np.fft.rfft(mono[i: i + size] * win))
        if prev is not None:
            flux.append(np.maximum(mag - prev, 0).sum())
        prev = mag
    flux = np.array(flux)
    flux /= flux.max()
    out = []
    for i in range(1, len(flux) - 1):
        if flux[i] > thresh and flux[i] >= flux[i - 1] and flux[i] > flux[i + 1]:
            t = (i + 1) * hop / SR + size / 2 / SR
            if not out or t - out[-1] > 0.06:
                out.append(round(t, 4))
    return out


def tail_echo(left, right, fb, wet, tone, tail_s=1.2):
    return lambda x: echo(x, left, right, fb, wet, tone, tail_s)


def gather_sound():
    """Five rising notes (#09 as C5 E5 G5 C6 E6) as the pieces land; a rising sparkle (#44)
    as the name rises; a full chord (#43 chime, #16 an octave and a fifth down, #09 as C) as
    the ring closes."""
    n9 = sfx(9)
    hits = [(0.50 + i * 0.28, f"piece {i + 1}", gain(pitch(n9, st), -2)) for i, st in enumerate([0, 4, 7, 12, 16])]
    hits.append((2.06, "name", gain(sfx(44), -3)))
    chord = gain(sfx(43), -1) + 0
    low = gain(lowpass(pitch(sfx(16), -4), 3000), -6)
    c5 = gain(pitch(n9, -12), -8)
    hits.append((2.90, "ring closes", chord))
    hits.append((2.90, "ring closes (low)", low))
    hits.append((2.90, "ring closes (body)", c5))
    return hits, tail_echo(210, 290, 0.40, 0.30, 4200)


def mosaic_sound():
    """Three tile snaps (#13 click-tone at +0, +5, +7 with #24's click), the join (#49 thump with
    #28's bright ting), the name (#48's reverse swell peaking on it), the final pulse (#43 + #16)."""
    hits = []
    for i, (t, st) in enumerate([(0.55, 0), (0.95, 5), (1.35, 7)]):
        hits.append((t, f"tile {i + 1}", gain(pitch(sfx(13), st), -2)))
        hits.append((t, f"tile {i + 1} (click)", gain(sfx(24), -6)))
    hits.append((1.95, "join", gain(sfx(49), -5)))
    hits.append((1.95, "join (ting)", gain(sfx(28), -4)))
    # #48 peaks 0.12 s after its start: started early so the peak meets the name.
    hits.append((2.43, "name", gain(sfx(48), -5)))
    hits.append((3.15, "pulse", gain(sfx(43), -2)))
    hits.append((3.15, "pulse (low)", gain(lowpass(pitch(sfx(16), -4), 3000), -7)))
    return hits, tail_echo(190, 260, 0.36, 0.28, 4500)


def path_sound():
    """The first point and the line start (#31's rising zip with a #21 pluck), three more points
    (#21 plucks as C#5 E5 A5), the ring (#28 click into a ting), the name (#44 sparkle), the
    glow (#43 + #16)."""
    hits = [(0.40, "line starts", gain(pitch(sfx(21), 0), -1)), (0.40, "line starts (zip)", gain(sfx(31), -9))]
    for i, (t, st) in enumerate([(0.75, 4), (1.10, 7), (1.45, 12)]):
        hits.append((t, f"point {i + 2}", gain(pitch(sfx(21), st), 0)))
    hits.append((1.85, "ring", gain(sfx(28), -3)))
    hits.append((2.45, "name", gain(sfx(44), -3)))
    hits.append((3.05, "glow", gain(sfx(43), -2)))
    hits.append((3.05, "glow (low)", gain(lowpass(pitch(sfx(16), -4), 3000), -7)))
    return hits, tail_echo(200, 280, 0.40, 0.30, 4200)


# ---------------------------------------------------------------------------------------
# Graphics (all coordinates in a 240 × 240 mark box)

def arc_path(cx, cy, r, a0, a1):
    p0 = (cx + r * np.cos(np.radians(a0)), cy + r * np.sin(np.radians(a0)))
    p1 = (cx + r * np.cos(np.radians(a1)), cy + r * np.sin(np.radians(a1)))
    large = 1 if (a1 - a0) % 360 > 180 else 0
    return f"M{p0[0]:.2f} {p0[1]:.2f}A{r} {r} 0 {large} 1 {p1[0]:.2f} {p1[1]:.2f}"


def rrect(x0, y0, x1, y1, r):
    """Path of a rectangle with per-corner radii r = (tl, tr, br, bl)."""
    tl, tr, br, bl = r
    return (f"M{x0 + tl} {y0}H{x1 - tr}A{tr} {tr} 0 0 1 {x1} {y0 + tr}V{y1 - br}"
            f"A{br} {br} 0 0 1 {x1 - br} {y1}H{x0 + bl}A{bl} {bl} 0 0 1 {x0} {y1 - bl}"
            f"V{y0 + tl}A{tl} {tl} 0 0 1 {x0 + tl} {y0}Z")


def gather_parts(fg, accent):
    cx = cy = 120
    r, sw = 74, 24
    parts = []
    for k in range(5):
        mid = -90 + k * 72
        half = 22.5
        colour = accent if k == 4 else fg
        d = arc_path(cx, cy, r, mid - half, mid + half)
        # Where the piece comes from: outward along its own direction.
        dx, dy = 70 * np.cos(np.radians(mid)), 70 * np.sin(np.radians(mid))
        parts.append({"id": f"p{k + 1}", "svg": f'<path d="{d}" fill="none" stroke="{colour}" stroke-width="{sw}" stroke-linecap="round"/>',
                      "from": [round(dx, 1), round(dy, 1)], "origin": [round(cx + r * np.cos(np.radians(mid)), 1), round(cy + r * np.sin(np.radians(mid)), 1)]})
    whole = f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="{fg}" stroke-width="{sw}"/>'
    over = f'<path d="{arc_path(cx, cy, r, 198, 234)}" fill="none" stroke="{accent}" stroke-width="{sw}" stroke-linecap="round"/>'
    return parts, whole, over


def mosaic_parts(fg, accent):
    a, b, g = 36, 204, 10
    m = (a + b) / 2
    left = rrect(a, a, m - g / 2, b, (30, 10, 10, 30))
    tr = rrect(m + g / 2, a, b, m - g / 2, (10, 30, 10, 10))
    br = rrect(m + g / 2, m + g / 2, b, b, (10, 10, 30, 10))
    return [
        {"id": "t1", "svg": f'<path d="{left}" fill="{fg}"/>', "from": [-60, 0], "join": [g / 2, 0], "origin": [a + 40, 120]},
        {"id": "t2", "svg": f'<path d="{tr}" fill="{fg}"/>', "from": [0, -60], "join": [-g / 2, g / 2], "origin": [164, 76]},
        {"id": "t3", "svg": f'<path d="{br}" fill="{accent}"/>', "from": [60, 40], "join": [-g / 2, -g / 2], "origin": [164, 164]},
    ]


PATH_POINTS = [(36, 176), (80, 122), (122, 150), (160, 96)]
PATH_RING = (198, 60, 24)


def catmull(points):
    pts = [points[0]] + points + [points[-1]]
    d = f"M{pts[1][0]} {pts[1][1]}"
    for i in range(1, len(pts) - 2):
        p0, p1, p2, p3 = pts[i - 1], pts[i], pts[i + 1], pts[i + 2]
        c1 = (p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6)
        c2 = (p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6)
        d += f"C{c1[0]:.1f} {c1[1]:.1f} {c2[0]:.1f} {c2[1]:.1f} {p2[0]:.1f} {p2[1]:.1f}"
    return d


def path_parts(fg, accent):
    cx, cy, r = PATH_RING
    # The line ends on the ring's edge, pointing at its centre.
    last = PATH_POINTS[-1]
    v = np.array([cx - last[0], cy - last[1]], float)
    v /= np.linalg.norm(v)
    end = (round(cx - v[0] * r, 1), round(cy - v[1] * r, 1))
    line = f'<path d="{catmull(PATH_POINTS + [end])}" fill="none" stroke="{fg}" stroke-width="9" stroke-linecap="round" stroke-linejoin="round"/>'
    dots = [{"id": f"d{i + 1}", "svg": f'<circle cx="{x}" cy="{y}" r="12" fill="{fg}"/>', "origin": [x, y]} for i, (x, y) in enumerate(PATH_POINTS)]
    ring = f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="{accent}" stroke-width="11"/>'
    return line, dots, ring


def wordmark(text="aione", height=64):
    """The name as outlined paths (M PLUS Rounded 1c Medium, SIL OFL), evenly spaced: one
    word, no letter emphasised. Returns per-letter paths and the total width."""
    font = TTFont(FONT)
    gs = font.getGlyphSet()
    cmap = font.getBestCmap()
    upm = font["head"].unitsPerEm
    from fontTools.pens.boundsPen import BoundsPen

    def top_of(ch):
        bp = BoundsPen(gs)
        gs[cmap[ord(ch)]].draw(bp)
        return bp.bounds[3]

    # This font's OS/2 table has no cap / x-height fields: measured from H and x.
    xh = getattr(font["OS/2"], "sxHeight", 0) or top_of("x")
    asc = getattr(font["OS/2"], "sCapHeight", 0) or top_of("H")
    scale = height / asc
    track = 0.06 * upm
    x = 0.0
    letters = []
    for ch in text:
        g = cmap[ord(ch)]
        pen = SVGPathPen(gs)
        gs[g].draw(TransformPen(pen, (scale, 0, 0, -scale, x * scale, asc * scale)))
        letters.append({"ch": ch, "d": pen.getCommands(), "x": round(x * scale, 1)})
        x += gs[g].width + track
    return letters, round((x - track) * scale, 1), round(height, 1), round(xh * scale, 1)


LOGOS = {
    "gather": {"title": "Gather", "line": "Five pieces (your games and programs) fly in and snap into one ring.", "sound": gather_sound},
    "mosaic": {"title": "Mosaic", "line": "Three tiles of different sizes join into one tile.", "sound": mosaic_sound},
    "path": {"title": "Path", "line": "One line runs through every point and ends in the launcher's ring.", "sound": path_sound},
}


def mark_svg(lid, fg, accent):
    if lid == "gather":
        parts, whole, _ = gather_parts(fg, accent)
        body = whole + "".join(p["svg"] for p in parts)
    elif lid == "mosaic":
        parts = mosaic_parts(fg, accent)
        body = "".join(f'<g transform="translate({p["join"][0]} {p["join"][1]})">{p["svg"]}</g>' for p in parts)
    else:
        line, dots, ring = path_parts(fg, accent)
        body = line + "".join(d["svg"] for d in dots) + ring
    return body


def lockup_svg(lid, fg, accent, bg=None):
    letters, w, h, _ = wordmark()
    body = mark_svg(lid, fg, accent)
    W = max(240, w) + 80
    ox = (W - 240) / 2
    wx = (W - w) / 2
    name = "".join(f'<path d="{L["d"]}" fill="{fg}"/>' for L in letters)
    rect = f'<rect width="100%" height="100%" fill="{bg}"/>' if bg else ""
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W:.0f} 360" width="{W:.0f}" height="360">{rect}'
            f'<g transform="translate({ox:.1f} 0)">{body}</g><g transform="translate({wx:.1f} 270)">{name}</g></svg>')


def main(out):
    from soundlib import read
    ms.PACK_MEDIAN_LOUDEST_100MS = float(np.median([loudest_100ms(read(f)) for f in sorted(ms.PACK.glob("*.wav"))]))
    out.mkdir(parents=True, exist_ok=True)
    data = {"method": __doc__.split("Usage")[0].strip(), "logos": {}}
    for lid, spec in LOGOS.items():
        hits, tail = spec["sound"]()
        mix, stems = render(hits, tail)
        y, stems, limiting = finish(mix, stems)
        write_wav(out / f"{lid}.wav", y)
        found = onsets(y)
        beats = []
        for (planned, name, _), stem in zip(hits, stems):
            if "(" in name:  # layers of a hit share its beat
                continue
            b, pk = beat_of(stem, planned)
            near = min(found, key=lambda o: abs(o - b)) if found else None
            rule = "half peak (perceptual attack)"
            if near is not None and abs(near - b) > 0.06:
                near = None
            if near is None:
                # A swell rises without a sharp start: its moment is where it culminates.
                b, rule = pk, "peak (a swell: no sharp onset)"
            beats.append({"name": name, "planned_s": planned, "beat_s": round(b, 4), "peak_s": round(pk, 4),
                          "beat_to_peak_ms": round((pk - b) * 1000, 1), "rule": rule,
                          "mix_onset_s": near, "onset_to_beat_ms": round((b - near) * 1000, 1) if near else None})
        data["logos"][lid] = {
            "title": spec["title"], "line": spec["line"], "beats": beats,
            "loudest_100ms_lufs": round(loudest_100ms(y), 1),
            "true_peak_dbfs": round(20 * np.log10(np.abs(y).max()), 2),
            "limiting_db": limiting, "length_s": len(y) / SR,
        }
        (out / f"{lid}-mark.svg").write_text(
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 240" width="240" height="240">{mark_svg(lid, WHITE, TEAL)}</svg>')
        (out / f"{lid}-lockup-dark.svg").write_text(lockup_svg(lid, WHITE, TEAL, "#000000"))
        (out / f"{lid}-lockup-light.svg").write_text(lockup_svg(lid, INK, "#008C9E", "#EBEBEB"))
    letters, w, h, xh = wordmark()
    data["wordmark"] = {"letters": letters, "width": w, "height": h}
    geo = {
        "gather": {"parts": gather_parts(WHITE, TEAL)[0], "whole": gather_parts(WHITE, TEAL)[1], "over": gather_parts(WHITE, TEAL)[2]},
        "mosaic": {"parts": mosaic_parts(WHITE, TEAL)},
        "path": dict(zip(["line", "dots", "ringSvg"], path_parts(WHITE, TEAL)), points=PATH_POINTS, ring=PATH_RING),
    }
    data["geometry"] = geo
    (out / "beats.json").write_text(json.dumps({k: v for k, v in data.items() if k != "geometry"}, indent=2))
    template = (Path(__file__).parent / "t11_preview.html").read_text()
    (out / "preview.html").write_text(template.replace("/*DATA*/null", json.dumps(data)))
    for n, name in [(24, "move"), (9, "select"), (15, "back")]:
        shutil.copy(ROOT / f"assets/sounds/50-menu-interface-sfx/MI_SFX {n:02d}.wav", out / f"menu-{name}.wav")
    for lid, d in data["logos"].items():
        print(lid, d["loudest_100ms_lufs"], d["true_peak_dbfs"], d["limiting_db"])
        for b in d["beats"]:
            print(f'   {b["name"]:12} beat {b["beat_s"]:.3f}  peak +{b["beat_to_peak_ms"]:5.1f} ms  mix onset {b["mix_onset_s"]}  onset->beat {b["onset_to_beat_ms"]} ms')


if __name__ == "__main__":
    main(Path(sys.argv[1] if len(sys.argv) > 1 else "out"))
