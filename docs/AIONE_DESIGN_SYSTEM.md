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
- **Footer branding (B-5 rev. o):** the "Window B" mark, **4 rem** (64 px at 1080 p and 100 %,
  about twice the prompt text; scaled with Interface Scale), two-tone: the halves in the footer's
  text colour, the diamond in the accent (light #FD007E, dark #FF46A2; each ≥ 3:1), then "AIONE"
  in M PLUS Rounded 1c, Medium, letter spacing 0.08 em, 0.75 rem apart.

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

## The selected game's title (B-8, rev. o)
- **Just like Switch:** the game's name alone, in the accent text colour, above the selected
  tile, starting at its left edge. No ▼ and no icon (rev. o removed alpha.6–7's ▼).
- It moves with the focus by transform (260 ms, the row's motion), never by layout; near the right
  margin (95 % of the width) it shifts left just enough to fit; a longer title scrolls within its
  own space (the shared marquee, by transform), clipped at both ends.
- Hidden under a dialog, and while the drawer has the focus (like Switch).

## The drawer (T1.2; Switch-style, B-9 rev. o)
- **Buttons:** Search (`search`), Activity (`leaderboard`), Surprise Me (`casino`), Theme Settings
  (`palette`), Settings (`settings`), Power (`power_settings_new`): Material Symbols Rounded,
  outlined, 48 design px; each icon's weight balanced so all carry the same ink (Search 480,
  Settings 250, Power 380, others 300; measured within ±10 % of the median, KD-81).
- **Geometry** (design px at 1080 p, scaled with Interface Scale): discs 108 across, 28 apart,
  centred (Switch's proportions: disc / tile 0.28, measured on the reference); 86 px under the
  row, moved up only as much as needed to keep the label 12 px clear of the footer at large
  Interface Scales.
- **Colours:** the disc in the surface colour (white on light, #3A3A3A on dark), the icon in the
  text colour. **Focused:** the tiles' ring (6 px, 5 px gap, pulsing between the two accent
  shades), and the label under the button in the accent text colour (light #B5005A, dark
  #FF69B4), 1.75 rem, Medium, centred under it, never clipped. **Hover (mouse):** a 4 px outline
  in the hover colour and the label. **Press:** the icon's squash-and-zoom (260 ms).
- **Focus:** Down from the row enters the drawer on its remembered button; Up returns to the
  last focused tile; left / right stop at the ends.
- **The toast:** "Coming in a later stage", 2.4 s, in the footer band's empty centre (never over
  the drawer's label).

## The tile picture (B-13, rev. k)
- The game's **`Icon`** (the largest size inside an `.ico`), **filling** the square tile.
- Without an icon picture: `Thumbnail`, `Portrait` or `Hero`, **fitted whole** (never cropped)
  over a blurred, darkened (45 %) copy of the same picture.
- Without any picture: a plain tile with the name.
- The core's 640 × 640 copy is shown once made (D-007 / D-010); until then the original (an icon
  fills; another picture is shown whole on #161616).
- **Small pictures (B-13 rev. o):** enlarged with Lanczos3 (never blocky); scaling down uses
  CatmullRom.

## States and focus (reference A5, A7)
- **Pad / keyboard mode:** one focus ring on the focused tile: 6 px ring with a 5 px gap, 11 px
  outside the tile. The new ring fades in and settles from 102.5 % while the old one fades out
  (240 ms). **B-9 (rev. m):** like Switch's selection frame, the ring itself pulses between two
  shades of the accent, with no glow: light **#FD007E ↔ #D9006C** (3.22 and 4.23:1), dark
  **#FF46A2 ↔ #FF69B4** (4.36 and 5.20:1); 0.7 s each way (1.4 s per cycle), ease-in-out,
  starting after the focus-in. Made with two solid ring layers and the top one's opacity
  (opacity only, A7), so every moment is a blend of two passing colours (D-011). The mouse
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

## Boot Up and Boot Down (B-5 final, A9)
- Full screen on black (#000000) from the first frame. The Window B animation keeps the reference
  page's proportions: its 680 × 460 view is drawn 69 % of the screen height tall, centred
  (the reference page draws it 744 px tall in a 1080 px window); the shake, ring and shards are
  never clipped (the layer shows its overflow).
- Colours: off-white #F5F3EE, the diamond #FF46A2 (one brand pink), black. Flat.
- Letters "A I O N E" (B-5 rev. o): M PLUS Rounded 1c 500, **44 units** (twice the reference's
  22), 50 apart; the mark at y −33 and the baseline at 133, so the mark and the name are centred
  together; their rise 28 and drop 32 units (doubled with the size); timing unchanged.
- The custom video is shown whole (`object-fit: contain`) on black.
- Home appears when the Boot Up ends (its last 0.35 s fade to black).

## Effects
None in the app: no blur, shadow or filter on Home. The dialog keeps its one shadow from T0. The
Boot Up previews use a blur for the final glow; in the app it becomes a pre-rendered image (A7).

## The measured Switch Home (B-14, alpha.9) — current values
These supersede earlier entries above where they differ (design px at 1080 p; Interface Scale
scales text; tiles follow the window only).
- **Row:** tiles 398, gap 12, first at 155, top at 309; the ring 7 px directly around the art (no
  gap), jumping instantly; pulse 1.0 s from the bright end (light #FD007E → #D9006C, dark #FF69B4 →
  #FF46A2); the row slides 170 ms, cubic-bezier(0.16, 1, 0.3, 1).
- **Title:** 3.17 rem (capitals about 37 px), baseline at 281 (21 above the ring), centred on the
  tile; within 3 % screen margins. Long titles: a window 1.8 × (398 + 14) px centred on the tile; the
  text holds 1.0 s, scrolls left at 90 px/s (scaled with the title size) and repeats after a gap of
  4 cap heights; clipped at the window. Measured without layout reads (canvas text width).
- **Empty slots:** after the last game, #F4F4F4 (light) / #363636 (dark) with a hairline; focusable
  but the last (cut by the screen edge); no title; "A OK" dimmed (muted colour, ≥ 4.5:1).
- **Drawer:** discs 110 px, 133 centre to centre, growing at half rate with Interface Scale (so the
  drawer fits at 150 %); the ring 10 px outside (7 px); icons in palette colours (light: #B5005A,
  #D9006C, #900048, #FD007E, #6C0036, #AF5087; dark: tints); labels 2.14 rem (capitals about 25 px).
- **Edge bumps:** only the ring moves (WAAPI, transform): horizontal 15 px, back 6 px, 200 ms;
  vertical 12 px, back 4 px, 300 ms; segments eased, overall linear; once per held press.
- **Footer prompts:** a game "+ Options" (pad Menu) and "A Start"; the drawer "A OK"; an empty
  slot "A OK" dimmed. The keyboard has no Options key yet.
- **Sounds:** see `docs/sound/SOUND_MAPPING.md` (B-1 rev. q).
- **B-15 extras (off by default):** `press` (the ring squeezes to 97 % on A, 120 ms), `titlefade`
  (the title fades in, 120 ms), `labelfade` (the drawer label fades in, 120 ms).

## Rev. r (alpha.10) — current values, superseding the B-14 section above where they differ
- **Ring:** 7 px, outside a **2 px white separator** around the art (D-018); `--ring=plain` = no
  separator. Pulse unchanged.
- **Row:** 12 slots (games, then empty slots up to 12, all focusable). **Wrap:** a fresh press
  resting on an end (the last move ≥ 300 ms ago) goes to the other end; the row whips across in its
  170 ms slide. **Row bump:** holding, or a press within 300 ms of the last move, at an end: the
  whole row (`translate` on the track) +170 px toward the press, −14, +4, back, in 150 ms; once per
  hold; the edge-bump sound.
- **Ring bumps** (up / down, the drawer's ends): full push at 16 ms (15 px sideways, 12 px up /
  down), past the centre by 66 ms (6 / 4 px), a 3 / 2 px wobble, settled at 200 / 180 ms.
- **Drawer:** the focused button `scale(1.08)`; B returns to the game.
- **Title:** clipped sideways only; the baseline 21 × the UI unit above the ring's top (302 px).
- **Footer mark:** 2.875 rem (46 px), one colour (`currentColor`).
- **Sounds:** six wired (row move, drawer move, edge bump, confirm, back, error); entering a section
  plays its own move sound.
- **B-15:** `titlefade` on by default; `press`, `labelfade` behind `--extras`.
- **Options:** pad Menu (≡), keyboard Tab and the Menu key; never repeats.

## Rev. s / t (alpha.11) — current values, superseding the rev. r section above where they differ
- **Ring (B-9 rev. s):** outside in: a 4 px halo (`--color-ring-halo`: the accent at 18–22 %), the
  7 px ring (`--color-ring`), a 1.5 px bright edge (`--color-ring-peak`), a 4 px white band, the
  picture (12.5 px in all). **Flash:** the peak layer (`--color-ring-peak`: light #FFB0D7, dark
  #FFC2E0) fully shown at 0 % (every move restarts it), fading to 0 by 75 % (ease-out), resting to
  80 %, rising to 100 % (1.0 s, `aione-ring-flash`). The drawer's ring flashes the same way.
- **Row's ends (B-14 rev. s):** only the ring bumps (14 px, back 4, settled at 150 ms); the second
  fresh press at the same end wraps (the track's slide 200 ms instead of 170).
- **Sounds:** nine (row move, drawer move, edge bump, wrap, confirm, back, error, Options open /
  close); repeats −4 dB after the third in a hold.
- **Extras (B-15 rev. t):** `press` (tile / drawer button: 1 → 0.94 at 35 % → 1.015 at 75 % → 1,
  180 ms), `labelfade` (opacity 0 and +4 px → rest, 150 ms), `titlefade` (120 ms); all on;
  `--extras=none` off.

## Rev. u / v (alpha.12) — current values, superseding the sections above where they differ
- **Ring:** outside in: a halo (an 8 px blur, 3 px spread, in #FF46A2; its opacity follows the
  flash), the ring (7 px, #FF46A2 always; the peak layer #FF69B4), a 2 px bright edge (#FF8CC6),
  a 4 px white band, the picture (13 px in all). Same in both presets.
- **Title and drawer label:** #FF46A2 (the user's choice, like Switch; 2.65:1 on #EBEBEB).
- **Row:** tiles' top at 291 (the ring's top at 278). Ends: a resting press (> 400 ms after the
  last move) wraps (200 ms); otherwise a ring bump (14 px, overshoot 6, 150 ms), once per hold
  and direction.
- **Drawer:** the discs centred at 818 px (1080 p, 100 %); icons in palette harmonies (light:
  #1F7FE0, #0F9A55, #8F7C00, #E0287F, #4646FF, #5A9410; dark: the light originals), weight + 200,
  58 design px.
- **Header:** clock 47 px (digits about 34 px), icons 38 px, the battery percentage before its
  icon, centred 105 px from the top, all at half rate with Interface Scale.
- **Footer:** text 2.23 rem (capitals about 26 px), glyphs 2.25 rem.
- **Sounds:** the click C8 (4186 Hz); `rowScroll` = the click, then the scroll tick 65 ms later
  (G#6 about 1.7 kHz, −32 LUFS); the wrap = the click (+2 dB) and a soft sweep 60 ms later.

## Rev. w / x (alpha.13) — current values, superseding the sections above where they differ
- **Ring (option B):** 7 px, #FD007E at the base, #FF46A2 at the peak (crisp); a thin glow at the
  peak only (`0 0 4px` in #FF46A2, its opacity following the flash); 2 px edge #FF8CC6, 4 px white
  band; the drawer's focused button the same, with the same flash.
- **Row:** tiles 386 px, 18 px apart (pitch 404), the first at 155, top 291. Ends: a held repeat
  bumps the ring once; every fresh press wraps (200 ms, the wrap sound); the landing move on the
  first game or the last slot plays the click alone.
- **Header:** clock 42.5 px (digits about 31 px of ink), "AM" 0.65 em, icons 48 px (about 31 px of
  ink). **Footer:** text 2.4 rem (about 28 px), glyphs 38 px.

## Rev. y / z (alpha.14) — current values
- **Ring (B3):** #D9006C at the base, #FF58AB at the peak, row and drawer, both presets; the thin
  4 px glow at the peak only.
- **Drawer:** discs 108 px (soft shadow `--disc-shadow`), 27 px apart (pitch 135); focused 108 %
  (117 px); its ring 7.4 px before the scale (8 px after), directly on the disc, no band or edge;
  the label 2.55 rem (capitals about 31 px), regular, about 12 px below the ring.
- **Footer:** glyph box 3.2 rem (a 37 px visible circle; Kenney's art fills about 72 % of its box);
  gaps 7 / 53 px (16 / 63 px ink to ink, measured); text 2.23 rem (capitals about 27 px), regular;
  divider 2 px, `--color-footer-divider`.


## Notifications (alpha.36, T2.3) — current

- **Place:** top-left, in the Home header's empty spot (left 60 px, top 36 px at 1080 p; the header's
  half-rate scale for the top), above Home, below every dialog (z 60; dialogs hold notices anyway).
- **Card:** the surface colour, radius 20, soft shadow (0 6 22, 18 % black), padding 16 / 28 / 16 / 18;
  at most 760 wide (text ellipsized). All sizes follow Interface Scale (`--ui`).
- **Badge:** a 60 px circle in the soft accent fill (`--color-soft`) with a Material Symbols Rounded
  icon (38 px) in the hover accent (`--color-hover`): light #B5005A on #FFC2E0 (about 4.3:1), dark
  #FF69B4 on #480024 (about 6:1). Icons: `sports_esports` (a game ended), `videogame_asset` /
  `videogame_asset_off` (a controller), `battery_alert` (a battery).
- **Text:** the title at the footer's size (`--text-label`), the detail a step smaller (`--text-md`) in
  the muted text colour.
- **Motion:** in over 200 ms (fade and 16 px drop, ease-out, transform and opacity only); gone after
  4 s with a 200 ms fade. One at a time.
