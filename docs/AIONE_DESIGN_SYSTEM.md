# AIONE 4 — Design system

One current statement per topic (A8). Skeleton in T0: neutral tokens only; the themes (Switch-style first, T1) replace them. Tokens live in `src/app.css` as CSS custom properties.

## Colour (T0 neutral dark)
| Token | Value | Use | Contrast on `--color-bg` |
|---|---|---|---|
| `--color-bg` | `#0f1013` | page background, flat (no gradient, L-09) | — |
| `--color-surface` | `#1b1d22` | panels (readout) | — |
| `--color-surface-raised` | `#24272d` | dialogs | — |
| `--color-text` | `#eceef1` | text | ≈ 16.4 : 1 |
| `--color-text-muted` | `#a4aab4` | secondary text | ≈ 8.1 : 1 (7.2 on surface, 6.4 on dialogs) |
| `--color-ring` / `--color-accent` | `#4fc3e8` | focus ring, progress | ≈ 9.3 : 1 |

## Type
System UI font (Segoe UI Variable / Segoe UI). Sizes: xs 14, sm 16, md 20, lg 28 px; display `clamp(3.5rem, 7vw, 7.5rem)` for the T0 wordmark. Numbers use tabular figures.

## Spacing and radii
4 px grid: 4, 8, 12, 16, 24, 32, 48. Radii: 8 (buttons), 12 (panels), 20 (dialogs). Focus ring 3 px, offset 3 px.

## Motion
Only `transform` and `opacity` animate (A7). Durations: fast 120 ms, medium 200 ms, slow 320 ms; ease-out `cubic-bezier(0.2, 0.8, 0.2, 1)`. Dialog: fade + scale from 0.96 (200 ms). Press: scale 0.97. Progress bar: `scaleX`.

## Input modes (A7)
`data-input="pad"` shows the focus ring; `data-input="mouse"` hides it and uses hover only.

## Frame
Not yet (T1: the shared frame with header, footer and prompts, reference A4).
