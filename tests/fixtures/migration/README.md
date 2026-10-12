# Stored-file fixtures (B-6)

Every released AIONE 4 version's stored files, with personal paths replaced (`C:\Users\Player`).
Unit tests read them (Rust: `src-tauri/src/migration.rs`; the mocked UI: `tests/ui/t0.spec.ts`).
Never edit a fixture to make a test pass: a later version must read them as they are.

| File | Written by | How it was obtained |
|---|---|---|
| `migration/alpha1.migration.json` | 4.0.0-alpha.1 | Rebuilt from alpha.1's structs (git `4d4abad`, `serde_json::to_vec_pretty`) with the values of the user's alpha.1 copy (QA report alpha.1). Shape confirmed: alpha.2's parser fails on it at "line 24 column 5", exactly the error in the user's alpha.2 log. |
| `migration/alpha2.migration.json` | 4.0.0-alpha.2 | The user's file from their PC, user name replaced. |
