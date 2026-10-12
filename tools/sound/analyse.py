"""Measures every sound of the 50 Menu Interface SFX pack (D: alpha.2 item 7).

Usage: python3 tools/sound/analyse.py  (from the repository root)
Writes docs/sound/pack-measurements.json and prints a table.
"""
import json
from pathlib import Path

from soundlib import measure, read

PACK = Path("assets/sounds/50-menu-interface-sfx")
OUT = Path("docs/sound/pack-measurements.json")

rows = {}
for f in sorted(PACK.glob("*.wav")):
    rows[f.name] = measure(read(f))
OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_text(json.dumps(rows, indent=1))
print(f"{'file':14} {'len':>5} {'act':>5} {'peak':>5} {'LUFS':>6} {'att':>4} {'dec20':>5} {'tail':>6} {'bright':>6} pitch        contour tonal")
for k, m in rows.items():
    print(f"{k[:-4]:14} {m['length_s']:5.2f} {m['active_s']:5.2f} {m['peak_dbfs']:5.1f} {m['lufs']:6.1f} {m['attack_ms']:4} {str(m['decay20_ms']):>5} {m['tail_db']:6.1f} {m['centroid_hz']:6} {m['pitch_start']:>4}->{m['pitch_end']:<5} {m['contour']:8} {m['tonal_share']}")
