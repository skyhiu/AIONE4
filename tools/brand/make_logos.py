"""Builds the AIONE logo proposals (B-5) as SVG: an original mark plus the name "AIONE".

The name is drawn as outlines from a font bundled in assets/fonts (SIL OFL 1.1; turning
glyphs into artwork is allowed by the OFL), so the SVGs need no installed font.
Usage: python3 tools/brand/make_logos.py <out dir>   (from the repository root)
"""
import sys
from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

INK_DARK_BG = "#ECEEF1"    # ink on black (the T0 text colour)
INK_LIGHT_BG = "#15171B"   # ink on white
ACCENT_DARK_BG = "#4FC3E8"  # accent on black
ACCENT_LIGHT_BG = "#127FA6"  # accent on white (darker, for >= 3:1 contrast)


def wordmark(font_path, weight, text, height, tracking_em):
    """Returns (svg path data, width) of `text` with cap height `height`, origin top-left."""
    font = TTFont(font_path)
    if "fvar" in font:
        font = instantiateVariableFont(font, {"wght": weight})
    glyphs = font.getGlyphSet()
    cmap = font.getBestCmap()
    cap = font["OS/2"].sCapHeight or font["head"].unitsPerEm * 0.7
    scale = height / cap
    pen = SVGPathPen(glyphs)
    x = 0.0
    names = [cmap[ord(c)] for c in text]
    for i, name in enumerate(names):
        g = glyphs[name]
        # Flip y (font units go up) and place the baseline at `height`.
        t = TransformPen(pen, (scale, 0, 0, -scale, x, height))
        g.draw(t)
        x += g.width * scale
        if i < len(names) - 1:
            x += tracking_em * font["head"].unitsPerEm * scale
    return pen.getCommands(), x


def svg(width, height, body, title):
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width:.1f} {height:.1f}" '
        f'width="{width:.0f}" height="{height:.0f}" role="img" aria-label="{title}">\n'
        f"<title>{title}</title>\n{body}\n</svg>\n"
    )


def mark_orbit(ink, accent):
    """A: a ring that is not quite closed, with one dot in the opening: all in one."""
    # Ring centre (60,60), radius 42, stroke 12. The opening is centred at 30 degrees (upper
    # right), 56 degrees wide, so the round line ends keep clear of the dot.
    return (
        f'<path d="M 82.3 24.4 A 42 42 0 1 0 102.0 58.5" fill="none" stroke="{ink}" '
        f'stroke-width="12" stroke-linecap="round"/>\n'
        f'<circle cx="96.4" cy="39.0" r="9" fill="{accent}"/>'
    )


def mark_focus(ink, accent):
    """B: two rounded corner brackets around a dot, like a selection frame on screen."""
    return (
        f'<path d="M 18 52 L 18 34 A 16 16 0 0 1 34 18 L 52 18" fill="none" stroke="{ink}" '
        f'stroke-width="12" stroke-linecap="round" stroke-linejoin="round"/>\n'
        f'<path d="M 102 68 L 102 86 A 16 16 0 0 1 86 102 L 68 102" fill="none" stroke="{ink}" '
        f'stroke-width="12" stroke-linecap="round" stroke-linejoin="round"/>\n'
        f'<circle cx="60" cy="60" r="17" fill="{accent}"/>'
    )


def mark_tile(ink, accent):
    """C: a rounded tile holding a smaller, highlighted tile: one place for every game."""
    return (
        f'<rect x="16" y="16" width="88" height="88" rx="26" fill="none" stroke="{ink}" stroke-width="11"/>\n'
        f'<rect x="38" y="38" width="44" height="44" rx="13" fill="{accent}"/>'
    )


CONCEPTS = {
    "A": ("Orbit", mark_orbit, "assets/fonts/quicksand/Quicksand-VariableFont_wght.ttf", 600, 0.10,
          "A ring that is almost closed, with one dot in the opening: everything in one place, and the one thing you pick."),
    "B": ("Focus", mark_focus, "assets/fonts/figtree/Figtree-VariableFont_wght.ttf", 600, 0.12,
          "Two rounded corners framing a dot, like the selection on a console screen: the game you have picked."),
    "C": ("Tile", mark_tile, "assets/fonts/nunito/Nunito-VariableFont_wght.ttf", 700, 0.10,
          "A rounded tile holding a smaller highlighted tile: the game you are about to start, inside your library."),
}


def build(out):
    out.mkdir(parents=True, exist_ok=True)
    notes = {}
    for key, (name, mark, font, weight, track, idea) in CONCEPTS.items():
        for bg, ink, accent in (("on-dark", INK_DARK_BG, ACCENT_DARK_BG), ("on-light", INK_LIGHT_BG, ACCENT_LIGHT_BG)):
            # Stacked lockup: the mark (120 x 120) above the name, centred.
            cap = 34
            d, w = wordmark(font, weight, "AIONE", cap, track)
            width = max(120, w) + 40
            mx = (width - 120) / 2
            nx = (width - w) / 2
            body = (
                f'<g transform="translate({mx:.1f} 20)">\n{mark(ink, accent)}\n</g>\n'
                f'<path transform="translate({nx:.1f} 164)" d="{d}" fill="{ink}"/>'
            )
            title = f"AIONE logo proposal {key} ({name})"
            (out / f"AIONE_logo_{key}_{name.lower()}_{bg}.svg").write_text(svg(width, 164 + cap + 20, body, title))
            # The mark alone (for the T1 animation, which shows the mark first).
            (out / f"AIONE_mark_{key}_{name.lower()}_{bg}.svg").write_text(
                svg(120, 120, mark(ink, accent), f"AIONE mark {key} ({name})")
            )
        notes[key] = (name, idea, Path(font).parent.name, weight)
    return notes


if __name__ == "__main__":
    for k, v in build(Path(sys.argv[1] if len(sys.argv) > 1 else "logos")).items():
        print(k, v)
