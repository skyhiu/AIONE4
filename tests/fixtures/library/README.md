# Library fixtures (B-6)

| File | What |
|---|---|
| `wpf-game1.game.json`, `wpf-game2.game.json` | The **structure** of two of the user's real WPF `game.json` files (same fields, order, types and CRLF line ends) with **invented titles and values** (B-11, alpha.5). The user's own files are never committed. |
| `alpha4.game.json` | A `game.json` as 4.0.0-alpha.4 writes it after the conversion (`AioneSchema: 1` added, every other field kept), with the invented values of `wpf-game2`. |
| `alpha4.library.json` | `library.json` as 4.0.0-alpha.4 writes it (personal paths: none; the source is relative). |

Every later version must read these (`src-tauri/src/games.rs` tests, `tests/unit/library.test.ts`).
