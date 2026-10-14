# AIONE Logo Decision Summary

From the logo design chat, 2026-10-13. For the maintainer chat to record in the Master Prompt (B-5) and in task T1.2.

## 1. Chosen logo: "Window B"

A solid circle (the "all") split by one slanted cut, with a diamond at its heart (the "one"). The mark is tilted 25° clockwise. Meaning: **all in one**.

| File | Use |
|---|---|
| `aione_mark_B_color.svg` / `.png` | **Main colour mark**: off-white halves, hot pink diamond (Boot Up, app icon, marketing) |
| `aione_mark_B.svg` / `.png` | **White mark**, all off-white (footer branding in every theme, 16–24 px) |
| `aione_app_icon.svg` / `.png` | App icon preview: colour mark on a black rounded square |
| `aione_mark_B_4part.svg` | **Animation parts** (4 quarters + centre) for the Boot Up |
| `aione_boot_animation_reference.html` | **Final motion + sound reference**: open in a browser and press Play |
| `aione_boot_up.wav`, `aione_boot_down.wav` | **Final sounds**, timed to the animation |

**Colours:** off-white `#F5F3EE` (halves), hot pink `#FF3D7F` (diamond), background black `#000000`. Flat colours only.

## 2. Geometry (keep exactly)

Defined before rotation, in units:

- Circle radius **100**.
- Diamond window (cut out of the circle, centred): half-diagonal **65**.
- Diamond: half-diagonal **48**, centred, about half the circle's width.
- Vertical cut: **12** wide, the same width as the gap around the diamond.
- Whole mark rotated **25° clockwise**, with the diamond aligned to the cut.

## 3. Parts

- **At rest (static logo):** 3 parts: left half, right half, diamond.
- **For the animation:** 5 parts: 4 quarters (top-left, top-right, bottom-right, bottom-left) + the centre. The quarters are split by **hidden seams** on the horizontal axis. They have **no gap and must never be visible** once closed.

## 4. Boot Up (5.0 s, matches the brief)

| Time (s) | Motion |
|---|---|
| 0.40–1.00 | Centre springs in as a **hot pink circle** (r 44), with a slight stretch |
| 1.00–1.96 | 4 quarters **pop in one by one** clockwise (TL, TR, BR, BL; 120 ms apart), upright, hovering at distance 35 |
| 1.85–2.37 | **Charge:** quarters pull back +50 with growing trembling; pink circle **swells to 160%** |
| 2.37–2.43 | **Freeze:** everything still, slight extra pull-back (anticipation) |
| 2.43–2.55 | **Dash:** quarters strike in 0.12 s (strong ease-in, stretched along their direction); pink circle crushed from 160% to 76% |
| **2.55** | **Clamp (impact):** quarters squash and overshoot inward then bounce back; screen shake (about 10 px, decays over 0.36 s); flat ring bursts out and fades; pink circle rebounds and **morphs into the diamond** (2.57–2.75) |
| 2.58–2.65 | **Crack:** a crack shoots from the diamond window to the top and bottom edges along the cut |
| 2.65–3.17 | **Split:** halves thrown apart past the final gap with a slight opposite twist, then settle at gap 12; 10 small shards burst from the crack ends and arc away, fading |
| 3.00–3.35 | Hold, then the whole mark **tilts 0° → 25°** (ease in-out, no overshoot) |
| 3.45–4.31 | Letters **A I O N E** rise in one by one (90 ms apart) |
| 4.31–4.65 | Still hold |
| 4.65–5.00 | **Fade to black** (hand-off to the launcher) |

**Final landing moment:** the clamp at **2.55 s**.

## 5. Boot down (1.8 s)

| Time (s) | Motion |
|---|---|
| 0.00–0.50 | Letters drop away, E first, back to A |
| 0.20–0.70 | Mark leans slightly further, then turns back **upright** (halves **stay separate**) |
| 0.85–1.56 | Quarters **spring out one by one**, counter-clockwise (BL, BR, TR, TL): slight swell, then shrink away |
| 1.22–1.40 | Only after the last quarter starts to go: diamond **softens into a circle** |
| 1.40–1.72 | Pink circle swells slightly, then shrinks to nothing |

## 6. Sound

A console-style **sound logo in A major** (synthesized FM bells, pad and sub boom) with real foley underneath for weight. The **"A… ONE!"** two-note motif (E5 → A5 at 3.0 / 3.18 s) is AIONE's audio signature.

| Time (s) | Sound |
|---|---|
| 0.00 | Soft pad + faint CRT power-on crackle |
| 0.42 | Bell note A (pink circle) |
| 1.03–1.39 | Rising arpeggio C♯ E A B (quarters), panned to their corners |
| 1.85–2.37 | Rising tone, accelerating ticks, creaking rope (charge) |
| 2.37–2.43 | **Silence** (freeze) |
| 2.43–2.55 | Reversed chord echo "suck-in" + rising synth zip (dash) |
| **2.55** | **Big A major add9 chord** + sub boom + anvil + metal latch |
| 2.60–2.95 | Ceramic crack + shards as sparkling high bell notes |
| 3.00 / 3.18 | **"A… ONE!"** motif (tilt) |
| 3.47–3.83 | Five rising sparkle notes (letters) |
| Boot down | Falling sparkles, descending arpeggio, final low A bell + soft boom, CRT switch-off |

Foley comes from the **FilmCow Recorded SFX** library: tube tv turn on 1, fudge pop 1, rope tighten 2, anvil hit 2, metal latch 2, ceramic statue breaking 1, swoosh 2, swoosh 1, tube tv turn off 1.

## 7. Lookalike checks

Google Lens, TinEye, WIPO Global Brand Database and TMview were all run on the white mark. **No lookalikes found.**

## 8. Needs maintainer approval

1. **Sound source (plan change):** the brief planned the startup sound from the 50 Menu Interface SFX pack. The final sound is a custom synthesized sound logo plus FilmCow foley. Please approve, and **confirm the FilmCow licence allows use and editing in the app**.
2. Boot Up length is **5.0 s**, which matches the original brief, so no change is needed there.

## 9. Notes for the implementer (T1.2)

- Rebuild the mark as clean vectors from the SVG files; keep the geometry in section 2 exactly.
- Use `aione_mark_B_4part.svg` for the animation. The hidden seams must be invisible at rest.
- Match the motion in `aione_boot_animation_reference.html`. All timings and easing curves are in its script, and its **"Clamp in slow motion"** button shows the impact frame by frame.
- The Boot Up is **full-screen on black**: the screen shake, ring and shards extend beyond the logo, so the canvas must not clip them.
- Play `aione_boot_up.wav` from the first frame; the clamp hit lands at exactly 2.55 s.
- The footer uses the static **white** mark at 16–24 px; the colour mark is for the Boot Up and the app icon.
