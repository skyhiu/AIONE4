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
| Selected title (B-8) | above the selected tile, left-aligned with it (see below) |
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
Contrast per A8: text ≥ 4.5:1, large text and icons ≥ 3:1. Every accent pair is measured in the UI
test "B-9: every accent pair meets A8" (values below are that test's output).

**Accent (B-9, rev. h): hot pink**, brand colour **#FF46A2** (also the logo's diamond, B-5 final).
Hover, pressed and soft fills come from the user's #FF46A2 palette (shades #FF2290 … #480024,
tints #FF58AB … #FFC2E0).

| Role | Light | Dark |
|---|---|---|
| Background | `#EBEBEB` | `#2D2D2D` |
| Surface (plain tile) / dialog | `#FFFFFF` / `#F6F6F6` | `#3A3A3A` / `#444444` |
| Text | `#2D2D2D` (11.6:1) | `#FFFFFF` (13.8:1) |
| Muted text | `#5C5C5C` (5.6:1) | `#B4B4B4` (6.6:1) |
| Dimmed icon (no controller) | `#7A7A7A` | `#8C8C8C` |
| **Focus ring and accent shapes** | **`#FD007E`** (3.22:1; 3.83:1 against the white gap) | **`#FF46A2`** (4.36:1) |
| **Accent text** (the selected title) | **`#B5005A`** (5.66:1) | **`#FF69B4`** (5.20:1) |
| Hover outline (mouse) | `#B5005A` (5.66:1) | `#FF69B4` (5.20:1) |
| Soft fill (hover fill in dialogs) | `#FFC2E0`, text `#2D2D2D` (9.21:1) | `#480024`, text white (16.14:1) |
| Pressed fill | `#FF58AB`, text `#2D2D2D` (4.75:1) | `#FF46A2`, text `#1E1E1E` (5.28:1) |
| Accent on the dialog surface (progress bar) | 3.55:1 | 3.08:1 |
| Divider | `#9E9E9E` | `#6E6E6E` |
| Warning / OK text | `#8A5A00` / `#1C7A43` | `#F2C56B` / `#6FD39A` |
| Scrim under dialogs | black 42 % | black 55 % |

The light preset is the default (B-11).

## The selected game's title (B-8, rev. k / l)
- **Where:** above the **selected tile**, **left-aligned with that tile's left edge**, moving
  with the focus. It is one element moved by `transform: translateX` with the row's own motion
  (260 ms, ease-out), never by layout (L-16). Its width is measured once per title / size change.
- **Near the right edge:** when the title would run past 95 % of the width, it shifts left just
  enough to fit (never past the row's start); a title longer than the row's width scrolls (the
  shared marquee, by transform).
- **The ▼:** Material Symbols Rounded **`arrow_drop_down`**, FILL 1, in the title colour, pointing
  at the tile. Its visible triangle is 0.19 × 0.365 em of its own size (measured), so the glyph is
  set at **3.84 × the title size**, giving a triangle as tall as the capitals (0.73 em) and
  1.4 em wide, with 0.32 em before the text. Measured in the UI tests at every size: height
  0.85–1.25 × the capitals (measured 1.00–1.05), centred within 0.08 em (measured ≤ 0.025 em).
- Title: accent text colour, Regular, `--text-title`. Hidden under a dialog and in mouse mode
  when nothing is hovered or selected (then it follows the hovered or selected tile).

## The tile picture (B-13, rev. k)
- The game's **`Icon`** (the largest size inside an `.ico`), **filling** the square tile.
- Without an icon picture: `Thumbnail`, `Portrait` or `Hero`, **fitted whole** (never cropped)
  over a blurred, darkened (45 %) copy of the same picture.
- Without any picture: a plain tile with the name.
- The core's 640 × 640 copy is shown once made (D-007 / D-010); until then the original (an icon
  fills; another picture is shown whole on #161616).

## States and focus (reference A5, A7)
- **Pad / keyboard mode:** one focus ring on the focused tile: 6 px ring in the ring colour (hot
  pink) with a 5 px gap, 11 px outside the tile. The new ring fades in and settles from 102.5 %
  while the old one fades out (240 ms). **D-011 (B-9 rev. k):** the ring itself is solid at full
  colour and never fades; a soft glow (box-shadow 18 px blur, 5 px spread, ring colour) pulses
  **outside** it, opacity 0.15 → 0.75 → 0.15 over 1.4 s, starting after the focus-in. Measured
  ring contrast at the glow's lowest and highest points: 3.22:1 (light), 4.36:1 (dark). The mouse
  cursor is hidden.
- **From mouse to pad or keys (B-10):** the first pad or key press after mouse use only brings
  the ring back, on the tile last hovered or clicked; it does not move and plays no sound. The
  next press moves. Logged "ring back on tile N (after mouse)". **Rev. k:** only Home's inputs
  count (pad buttons and directions; arrows, Enter, Space, Esc, Backspace); modifiers alone and
  other keys keep mouse mode (logged "(mouse mode kept)").
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
AIONE's own family, made from FilmCow recordings (B-1 rev. h / j) by
`tools/sound/make_menu_family.py`, in **A major**: move on E, select a step above (F#), back a
step below (D); proposals: launch on A, error falling F → E. Cut to one hit, high-passed, gently
gated, short decays (move 25 ms, select 64 ms, back 28 ms), 48 kHz mono, levelled to the family
(loudest 30 ms: move −20, select −18, back −19 LUFS), true peaks under −1 dBFS. Played through
AIONE's gain stages at menu level 100 %. One sound per action; a sound never stacks on itself;
held directions: at most every 180 ms, 3 dB softer. Details: `docs/sound/SOUND_MAPPING.md`.

## The frame
One shared frame per screen (A3): `StatusHeader` (Home only), `Footer` (prompts right, branding
left on Home only). Full screens get the title row in T3 / T4.

## Pictures (D-007, D-010)
Tiles show the core's 640 × 640 JPEG copy (B-13 above), made in the background; Home decodes the
pictures it shows before it takes input (at most 1.5 s), and decodes each copy before swapping it
in. Before Home, the window and the page are black (#000000), never grey or white.

## Effects
None in the app: no blur, shadow or filter on Home. The dialog keeps its one shadow from T0. The
Boot Up previews use a blur for the final glow; in the app it becomes a pre-rendered image (A7).
