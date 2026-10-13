# AIONE logo proposals (B-5)

## 4.0.0-alpha.4: three "all in one" marks, each with a beat-matched Boot Up

Not in the app yet (T1.2 builds the one the user chooses). Built by
`tools/brand/make_t11_bootup.py` (sounds, measured beats, SVGs, the preview page) and
`tools/brand/shoot_t11.mjs` (PNG previews, frames, the sync check). Delivered separately as
`AIONE4_4.0.0-alpha.4_logo-bootup-preview.zip`: open `preview.html`, pick a logo, press Play.

| Logo | Idea | Boot Up (5 s): each sound hit is a visual event | Risks to judge |
|---|---|---|---|
| **Gather** | five pieces (your games and programs) gather into one ring; one piece in the accent | 5 rising notes, one per piece snapping in with a pop; a rising sparkle as the letters rise; a full chord as the ring closes, pulses and glows | a ring with one accent arc can read as a loading spinner |
| **Mosaic** | three tiles of different sizes join into one tile (not a 2×2 grid, which is Microsoft's mark) | 3 tile snaps; a low thump with a ting as the gaps close; a reverse swell peaking as the letters land; a chord as the tile pulses | faint seams stay visible after the join |
| **Path** | one line runs through every point and ends in the launcher's ring | the first point with a zip; 3 plucks as the line reaches each point; a ting as the ring pops; a sparkle for the name; a chord with the glow | the zig-zag can read like a chart |

**Name:** lowercase **"aione"** in M PLUS Rounded 1c Medium, evenly spaced, so it reads as one
word with no emphasis on "AI".

**Colours:** white with one teal accent on black (Boot Up); dark `#2D2D2D` with teal `#008C9E`
on the Switch-style light grey (footer branding).

**Beat matching:** every hit is measured in the finished sound (`beats.json`): the beat is the
moment the hit reaches half its peak (1 ms steps); for a swell with no sharp start, its peak.
Each visual event's key moment (a piece's pop, a letter landing, the ring closing, the glow) is
placed on that beat, and the preview drives the animation from the audio clock on every frame.
Cross-check: every hit with a sharp start is within 14 ms of an independently detected onset in
the mix (under one frame at 60 fps).

## Earlier proposals (not chosen)
alpha.3: Orbit, Focus, Tile (rev. e: none chosen).
