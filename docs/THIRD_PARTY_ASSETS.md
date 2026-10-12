# Third-party assets and licences

AIONE 4 ships no third-party images, fonts, sounds or videos in 4.0.0-alpha.1.

| Asset | Source | Licence | Used for |
|---|---|---|---|
| App icon (ring and "4") | Original, drawn for AIONE 4 | Project's own | Window and exe icon |
| UI font | System font (Segoe UI on Windows), not bundled | Part of Windows | All text |

Planned reuse from the WPF version (same licences, recorded here when added): Kenney Input Prompts (CC0), Material Symbols Rounded (Apache 2.0), the user's sound pack, the SCOWL word list.

Code dependencies (npm and Cargo) are pinned in `package-lock.json` and `src-tauri/Cargo.lock`; their licences are those of the packages (Tauri: MIT / Apache-2.0; Svelte: MIT).
