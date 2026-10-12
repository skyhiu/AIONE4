//! Reading the game library (`Games\<folder>\game.json` plus media, the WPF format; A9).
//!
//! T0 only counts. **`scan_games_dir` is the one rule for what is a game**, used by the copy
//! report now and by the library loader from T1, so the counts the copy reports are the
//! counts that load (L-17, the KD-101 lesson: `_Removed` was counted as a game).
//!
//! Rules: a game is a sub-folder of `Games` with a readable `game.json` (a JSON object).
//! Folders whose name starts with `_` or `.` are ignored by design. Every other entry that is
//! not a game is reported with its reason.
//!
//! Hidden / favourite flags: T0 recognises boolean fields named `Hidden`, `IsHidden`,
//! `Favorite`, `IsFavorite` or `Favourite` (any case) in game.json. **Hypothesis:** this is
//! where WPF stores them; if WPF keeps them elsewhere these counts read 0 and T1 maps the
//! real format.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IgnoredEntry {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryCounts {
    pub games: u32,
    pub hidden: u32,
    pub favorites: u32,
    pub ignored: Vec<IgnoredEntry>,
}

pub const GAMES_DIR: &str = "Games";
pub const GAME_FILE: &str = "game.json";

/// Counts the games in a `Games` folder (read only).
pub fn scan_games_dir(games: &Path) -> LibraryCounts {
    let mut counts = LibraryCounts::default();
    let Ok(read) = fs::read_dir(games) else {
        return counts;
    };
    let mut entries: Vec<_> = read.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let ignore = |reason: &str| IgnoredEntry {
            name: name.clone(),
            reason: reason.to_string(),
        };
        let Ok(ft) = entry.file_type() else {
            counts.ignored.push(ignore("could not be read"));
            continue;
        };
        if !ft.is_dir() {
            counts.ignored.push(ignore("not a folder"));
            continue;
        }
        if name.starts_with('_') {
            counts
                .ignored
                .push(ignore("ignored by name (starts with \"_\")"));
            continue;
        }
        if name.starts_with('.') {
            counts
                .ignored
                .push(ignore("ignored by name (starts with \".\")"));
            continue;
        }
        let file = entry.path().join(GAME_FILE);
        if !file.is_file() {
            counts.ignored.push(ignore("no game.json"));
            continue;
        }
        let parsed = fs::read(&file)
            .map_err(|e| e.to_string())
            .and_then(|bytes| parse_game_json(&bytes));
        match parsed {
            Ok(obj) => {
                counts.games += 1;
                if flag(&obj, &["hidden", "ishidden"]) {
                    counts.hidden += 1;
                }
                if flag(
                    &obj,
                    &["favorite", "isfavorite", "favourite", "isfavourite"],
                ) {
                    counts.favorites += 1;
                }
            }
            Err(e) => counts
                .ignored
                .push(ignore(&format!("game.json could not be read: {e}"))),
        }
    }
    counts
}

fn parse_game_json(bytes: &[u8]) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    // Files written by .NET may start with a UTF-8 byte order mark.
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match serde_json::from_slice::<serde_json::Value>(bytes) {
        Ok(serde_json::Value::Object(map)) => Ok(map),
        Ok(_) => Err("not a JSON object".into()),
        Err(e) => Err(e.to_string()),
    }
}

fn flag(obj: &serde_json::Map<String, serde_json::Value>, names: &[&str]) -> bool {
    obj.iter()
        .any(|(k, v)| names.contains(&k.to_lowercase().as_str()) && v.as_bool() == Some(true))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Builds a WPF-like library: 3 games (1 hidden, 2 favourites), `_Removed`, `.cache`,
    /// a folder without game.json, a broken game.json and a stray file.
    pub fn make_library(root: &Path) -> PathBuf {
        let games = root.join(GAMES_DIR);
        let game = |name: &str, json: &str| {
            let d = games.join(name);
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join(GAME_FILE), json).unwrap();
            fs::write(d.join("cover.png"), [1u8, 2, 3]).unwrap();
        };
        game("Celeste", r#"{"Title":"Celeste","IsFavorite":true}"#);
        game(
            "Hades",
            "\u{feff}{\"Title\":\"Hades\",\"Hidden\":true,\"Favorite\":true}",
        );
        game("Tunic", r#"{"Title":"Tunic"}"#);
        game("_Removed", r#"{"Title":"Old"}"#);
        game(".cache", r#"{}"#);
        game("Broken", "{not json");
        fs::create_dir_all(games.join("NoJson")).unwrap();
        fs::write(games.join("notes.txt"), "x").unwrap();
        games
    }

    #[test]
    fn counts_games_and_reports_every_ignored_entry() {
        let dir = tempfile::tempdir().unwrap();
        let games = make_library(dir.path());
        let c = scan_games_dir(&games);
        assert_eq!(c.games, 3);
        assert_eq!(c.hidden, 1);
        assert_eq!(c.favorites, 2);
        let names: Vec<_> = c.ignored.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            names,
            vec![".cache", "Broken", "NoJson", "_Removed", "notes.txt"]
        );
        assert!(c
            .ignored
            .iter()
            .any(|i| i.name == "_Removed" && i.reason.contains("by name")));
    }

    #[test]
    fn missing_games_folder_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            scan_games_dir(&dir.path().join("Games")),
            LibraryCounts::default()
        );
    }
}
