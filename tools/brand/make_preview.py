"""Builds AIONE4_bootup_preview.html: a rough preview of the B-5 Boot Up timeline.

It animates the chosen logo proposal (mark, then name, settle, fade) on black, silent by
default. "Play with sound" plays AIONE4_bootup_<A|B|C>.wav from the same folder as the HTML.
Only transform and opacity are animated (A7). The real animation is built in T1.
Usage: python3 tools/brand/make_preview.py <logos dir> <out html>
"""
import re
import sys
from pathlib import Path


def inner(svg_text):
    """The SVG's drawing (without the outer <svg> and <title>), and its viewBox."""
    vb = re.search(r'viewBox="([^"]+)"', svg_text).group(1)
    body = re.sub(r"^.*?<title>.*?</title>\n", "", svg_text, flags=re.S)
    body = body.rsplit("</svg>", 1)[0]
    return vb, body


def build(logos, out):
    marks, names = {}, {}
    for key, name in (("A", "orbit"), ("B", "focus"), ("C", "tile")):
        mvb, mbody = inner((logos / f"AIONE_mark_{key}_{name}_on-dark.svg").read_text())
        lvb, lbody = inner((logos / f"AIONE_logo_{key}_{name}_on-dark.svg").read_text())
        # The name is the <path transform="translate(... 164)"> of the full logo.
        name_path = re.search(r'<path transform="translate\(([0-9.]+) 164\)" d="([^"]+)"', lbody)
        width = float(lvb.split()[2])
        marks[key] = (mvb, mbody)
        names[key] = (width, name_path.group(1), name_path.group(2))
    options = "".join(
        f'<option value="{k}">{k} – {n}</option>' for k, n in (("A", "Orbit"), ("B", "Focus"), ("C", "Tile"))
    )
    logos_js = ",\n".join(
        f'{k}: {{ mark: `<svg viewBox="{marks[k][0]}" width="100%" height="100%">{marks[k][1]}</svg>`, '
        f'name: `<svg viewBox="0 0 {names[k][0]:.1f} 40" width="100%" height="100%"><path transform="translate({names[k][1]} 0)" d="{names[k][2]}" fill="#ECEEF1"/></svg>` }}'
        for k in "ABC"
    )
    html = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>AIONE 4 Boot Up preview (rough)</title>
<style>
  html, body {{ margin: 0; height: 100%; background: #000; color: #a4aab4; font: 14px system-ui, sans-serif; }}
  #stage {{ position: fixed; inset: 0; display: grid; place-items: center; }}
  #logo {{ display: grid; justify-items: center; gap: 4vh; }}
  #mark {{ width: 22vh; height: 22vh; opacity: 0; }}
  #name {{ width: 30vh; height: 8vh; opacity: 0; }}
  #bar {{ position: fixed; left: 0; right: 0; bottom: 0; padding: 10px 16px; display: flex; gap: 12px;
         align-items: center; background: rgba(20,22,26,.85); }}
  button, select {{ font: inherit; background: #24272d; color: #eceef1; border: 0; border-radius: 6px; padding: 6px 10px; }}
  #clock {{ font-variant-numeric: tabular-nums; min-width: 5.5em; color: #eceef1; }}
  #events {{ color: #a4aab4; }}
  #bar.hidden {{ opacity: 0; }}
</style></head><body>
<div id="stage"><div id="logo"><div id="mark"></div><div id="name"></div></div></div>
<div id="bar">
  <label>Logo <select id="pick">{options}</select></label>
  <label>Sound <select id="snd"><option value="A">A – Bloom</option><option value="B">B – Steps</option><option value="C">C – Sweep</option></select></label>
  <button id="silent">Play (silent)</button>
  <button id="sound">Play with sound</button>
  <span id="clock">0.00 s</span>
  <span id="events">0.0 black · 0.4 mark · 1.4 name · 2.4 settle · 4.2 fade · 5.0 Home</span>
</div>
<script>
const LOGOS = {{
{logos_js}
}};
const mark = document.getElementById('mark');
const name_ = document.getElementById('name');
const clock = document.getElementById('clock');
let anims = [];
let audio = null;
function setLogo() {{
  const l = LOGOS[document.getElementById('pick').value];
  mark.innerHTML = l.mark; name_.innerHTML = l.name;
}}
function build() {{
  anims.forEach((a) => a.cancel());
  const ease = 'cubic-bezier(0.2, 0.8, 0.2, 1)';
  // The timeline (B-5 proposal); only transform and opacity animate (A7).
  anims = [
    mark.animate([{{ opacity: 0, transform: 'scale(0.92)' }}, {{ opacity: 1, transform: 'scale(1)' }}],
      {{ delay: 400, duration: 600, easing: ease, fill: 'both' }}),
    name_.animate([{{ opacity: 0, transform: 'translateY(8px)' }}, {{ opacity: 1, transform: 'translateY(0)' }}],
      {{ delay: 1400, duration: 500, easing: ease, fill: 'both' }}),
    mark.animate([{{ transform: 'scale(1)' }}, {{ transform: 'scale(1.04)' }}, {{ transform: 'scale(1)' }}],
      {{ delay: 2400, duration: 500, easing: 'ease-in-out', composite: 'add' }}),
    document.getElementById('logo').animate([{{ opacity: 1 }}, {{ opacity: 0 }}],
      {{ delay: 4200, duration: 800, easing: 'ease-in', fill: 'both' }}),
  ];
}}
function tick() {{
  const t = anims.length ? (anims[0].currentTime ?? 0) / 1000 : 0;
  clock.textContent = Math.min(t, 5).toFixed(2) + ' s';
  if (t < 5.2 && anims.some((a) => a.playState === 'running')) requestAnimationFrame(tick);
}}
function play(withSound) {{
  if (audio) {{ audio.pause(); audio = null; }}
  setLogo(); build();
  if (withSound) {{
    audio = new Audio('AIONE4_bootup_' + document.getElementById('snd').value + '.wav');
    audio.play().catch(() => {{ clock.textContent = 'sound not found (put the WAVs next to this file)'; }});
  }}
  requestAnimationFrame(tick);
}}
/** For still frames: show the state at `ms` (pauses everything). */
window.seek = (ms) => {{ setLogo(); build(); anims.forEach((a) => {{ a.pause(); a.currentTime = ms; }});
  clock.textContent = (ms / 1000).toFixed(2) + ' s'; }};
document.getElementById('silent').onclick = () => play(false);
document.getElementById('sound').onclick = () => play(true);
document.getElementById('pick').onchange = setLogo;
setLogo();
</script></body></html>
"""
    out.write_text(html)


if __name__ == "__main__":
    build(Path(sys.argv[1]), Path(sys.argv[2]))
