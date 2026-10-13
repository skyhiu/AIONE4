# AIONE 4 — Design system

One current statement per topic (Master Prompt A8). Updated with every visual change. Source of
truth for values: `src/app.css` (tokens); this document says what they are and why.
Theme in this build: **Switch-style** (T1.1), presets **light** (default) and **dark**. The
other four themes come in T5.

## The look
Inspired by the real Switch Home (reference A4, A15; `hac-screenshot-homemenu-userpage-nso-highlighted_1.png`),
never copied: no Nintendo logos, fonts, icons or art. Flat backgrounds (L-09), one accent colour,
hierarchy through size and weight, short console wording (reference A12, A13).

## Size units (alpha.1 QA-6: no fixed pixel sizes)
| Unit | Is | Used by |
|---|---|---|
| `--px` | one design pixel at 1080 p: `100vh / 1080` | tiles, the focus ring, the row (they follow Tile Size, not Interface Scale) |
| `--ui` | `--px × Interface Scale`; `1rem = 16 --ui` | all text, the footer, dialogs, spacing |
| `--hdr` | `--px × (1 + (Interface Scale − 1) / 2)` | the Home status header (half rate: 150 % → 125 %; reference A4, decided after 3.9.12) |

Interface Scale: 90–150 % in steps of 10 (reference A8). Until Settings exist (T3) it is set per
run with `AIONE4.exe --interface-scale=150` (never stored). Default 100 %.

## Layout of Switch-style Home (design px at 1080 p, measured from the reference image)
| Element | Value |
|---|---|
| Status cluster | centre 8.5 % of the height from the top; right edge 5 % of the width in |
| Game row | top of the tiles at 292 (27 %); first tile 160 from the left (8.3 % of 1920) |
| Tile (Tile Size "Small", the Switch-style default) | 384 × 384, gap 20, radius 4; 4 full tiles in view at 16:9 |
| Selected title | above its tile, 0.75 rem gap; box 150 % of the tile; aligned to the tile's start at the left end of the view, its end at the right end, centred between |
| Drawer area | kept free below the row (the drawer comes in T1.2) |
| Footer | divider 2.4 % in from each side; height 5.5 rem; branding left (Home only), prompts right |

## Type
- **Family:** M PLUS Rounded 1c (SIL OFL), Regular 400, Medium 500, Bold 700, bundled. One family
  everywhere in the theme (reference A4).
- **Scale (rem at 1080 p, 100 %):** caption 1.25 (20 px) · small 1.5 (24) · body 1.625 (26) ·
  footer label 1.75 (28, Medium: bolder and larger than body) · selected title 1.875 (30) ·
  heading 2.5 (40) · display 4 (64).
- **Header:** clock 25 `--hdr` px, Regular, letter spacing 0.06 em, tabular figures; AM / PM at
  60 % on the same baseline. Clock cap height 18 px at 1080 p / 100 %.
- **Footer branding:** the System Name "AIONE", Medium, letter spacing 0.08 em. The logo joins it
  once chosen (B-5).

## Icons
- **Pack:** Material Symbols Rounded (variable font, Apache-2.0), weight 300, FILL 0, opsz 24:
  light and outlined (reference A4).
- **Header icons:** `sports_esports` (controller; dimmed when none is connected), `wifi` /
  `wifi_off` / `lan` (network), `battery_android_frame_0…6`, `_full`, `_bolt` (charging),
  `_question` (unknown). No battery icon on a PC without one. Order: clock, controller, network,
  battery.
- **One optical size (L-10, KD-81):** each glyph has an optical factor so its ink is as tall as
  the clock's capitals: controller 1.2, battery 1.45, `lan` 0.8, `wifi_off` 0.92, `wifi` 0.96.
  Measured in the sandbox at 1080 p / 100 %: clock 18 px, controller 18, network 18 (all three
  kinds), battery 19 (anti-aliasing).
- **Button prompts:** Kenney Input Prompts 1.5 (CC0), the pack's SVG files, drawn as a mask in
  the text colour: Xbox plain set for pads (XInput cannot tell a pad's make), keyboard key caps for
  the keyboard; the mouse keeps the last set. Mapping: `docs/GLYPH_MAPPING.md`. Glyph 2.25 rem.

## Colour (Switch-style)
Contrast per A8: text ≥ 4.5:1, large text and icons ≥ 3:1 (checked by the UI tests).

| Role | Light | Dark | Notes |
|---|---|---|---|
| Background | `#EBEBEB` | `#2D2D2D` | flat; no gradient or band (L-09) |
| Surface (plain tile, dialog) | `#FFFFFF` / dialog `#F6F6F6` | `#3A3A3A` / `#444444` | |
| Text | `#2D2D2D` (11.6:1) | `#FFFFFF` (13.8:1) | |
| Muted text | `#5C5C5C` (5.6:1) | `#B4B4B4` (6.6:1) | |
| Dimmed icon | `#7A7A7A` (≈3.6:1) | `#8C8C8C` (≈4.0:1) | controller with no pad |
| Focus ring | `#008C9E` (3.4:1) | `#1BB8CC` (5.8:1) | with a gap in the background / white |
| Selected title | `#006F7D` (≥ 4.5:1) | `#3FD3E6` (7.7:1) | in the ring's hue (reference A4). The real Switch's bright teal would fail A8 on light grey, so the light preset uses a darker teal |
| Hover (mouse) | `#5C5C5C` outline | `#B4B4B4` outline | never the ring colour |
| Divider | `#9E9E9E` | `#6E6E6E` | decorative |
| Warning / OK text | `#8A5A00` / `#1C7A43` | `#F2C56B` / `#6FD39A` | copy dialog |
| Scrim under dialogs | black 42 % | black 55 % | the header dims with the rest (KD-90) |

The preset is chosen in Settings (T3); this build starts in light (an assumption; see the handoff).

## States and focus (reference A5, A7)
- **Pad / keyboard mode:** one focus ring on the focused tile: 6 px ring in the ring colour with a
  5 px gap, 11 px outside the tile. The new ring fades in and settles from 102.5 % while the old
  one fades out at the same time (240 ms, ease-in-out). After that, the ring alone blinks
  (opacity 1 → 0.45 → 1, 1.4 s, ease-in-out) while it stays focused; the blink restarts on every
  move, so it never runs during movement. The mouse cursor is hidden.
- **Mouse mode:** hover only, a 4 px outline in the hover colour; the tile selected by a first
  click keeps the same hover look; no ring. A click starts only the selected tile (L-06).
  Movement of more than 6 px switches to mouse mode; a pad or key press switches back at once,
  and the ring appears on the remembered tile.
- **Pressed:** the tile scales to 97 % in 90 ms and returns in 180 ms.
- **Suspended (a dialog over Home):** no ring, blink, title, marquee, hover or prompts.

## Motion tokens (only transform and opacity animate; A7)
| Token | Value | Use |
|---|---|---|
| `--motion-press` / `--motion-release` | 90 / 180 ms | press feedback |
| `--motion-focus` | 240 ms | ring in / out, hover |
| `--motion-row` | 260 ms, `--ease-out` | the row scrolling (transform) |
| `--motion-blink` | 1.4 s | focus ring blink |
| `--motion-fast` / `--motion-medium` / `--motion-slow` | 120 / 200 / 320 ms | dialogs, fades |
| Marquee | pause 1.2 s, 40 design px/s, pause, return; Web Animations on transform; runs only on the shown title | long titles (L-16) |

Eases: `--ease-out` `cubic-bezier(0.2, 0.8, 0.2, 1)`, `--ease-in-out` `cubic-bezier(0.4, 0, 0.2, 1)`.

## Sound (menu)
Move #24, select #09, back #15 of "50 Menu Interface SFX" (CC BY 4.0), unchanged, through
AIONE's own gain stages: Volume 100 % × menu level 60 % (−4.4 dB). One sound per action; a sound
never stacks on itself. Held directions: the move sound plays at most every 180 ms, 3 dB softer.
Details: `docs/sound/SOUND_MAPPING.md`.

## The frame
One shared frame per screen (A3): `StatusHeader` (Home only), `Footer` (prompts right, branding
left on Home only). Full screens get the title row in T3 / T4.

## Effects
None in the app: no blur, shadow or filter on Home. The dialog keeps its one shadow from T0. The
Boot Up previews use a blur for the final glow; in the app it becomes a pre-rendered image (A7).
