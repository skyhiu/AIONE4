# Library fixtures (B-6)

| File | What |
|---|---|
| `wpf-game1.game.json`, `wpf-game2.game.json` | Two of the user's real WPF `game.json` files (attached for alpha.4). They contain **no paths** (only game names, relative file names and numbers), so nothing had to be replaced. |
| `alpha4.game.json` | A `game.json` as 4.0.0-alpha.4 writes it after the conversion (`AioneSchema: 1` added, every other field kept). |
| `alpha4.library.json` | `library.json` as 4.0.0-alpha.4 writes it (personal paths: none; the source is relative). |

Every later version must read these (`src-tauri/src/games.rs` tests, `tests/unit/library.test.ts`).
