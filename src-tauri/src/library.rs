//! Reading the game library in the WPF format (`Games\<folder>\game.json` plus media; A9).
//!
//! **`scan_library` is the one rule for what the library contains.** The copy report uses it
//! now and the T1 loader must use it, so the counts the copy reports are the counts that load
//! (L-17, the KD-101 lesson: `_Removed` was once counted as a game).
//!
//! **Games:** a sub-folder of `Games` with a readable `game.json` (a JSON object). Folders whose
//! name starts with `_` or `.` are skipped by design. Every other entry that is not a game is
//! reported with its reason.
//!
//! **Hidden games, favorites and library folders: where they are read from.** The WPF format
//! for these is *not* recorded in the reference, and no WPF sample with such data was
//! available, so the readers below are a **hypothesis** that covers the likely layouts (D-001):
//! - in each `game.json`: true/false fields `Hidden` / `IsHidden`, `Favorite` / `IsFavorite`
//!   (also `Favourite`), and a folder name in `Folder` / `Folders` / `FolderName`
//!   (text or a list of texts);
//! - in any `.json` file at the top of the data folder: a list under a key `Folders` /
//!   `LibraryFolders` / `GameFolders` (each item a name, or an object with `Name` / `Title`), and
//!   lists under `Favorites` / `FavoriteGames` / `Hidden` / `HiddenGames` that name games by
//!   folder name, `Id` / `Guid` / `GameId`, or title; also files named `folders.json`,
//!   `favorites.json` or `hidden.json` that hold such a list directly.
//!
//! Every count says where it came from (`sources`), and list entries that match no game are
//! reported, so the real format can be confirmed from the logs and fixed in one place.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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
    /// Library folders (the user's groups of games), not file-system directories.
    pub folders: u32,
    /// Entries in `Games` that are not games, each with its reason.
    pub ignored: Vec<IgnoredEntry>,
    /// Where the hidden / favorites / folders counts were read from, and list entries that
    /// matched no game. Plain sentences for the log.
    #[serde(default)]
    pub sources: Vec<String>,
}

pub const GAMES_DIR: &str = "Games";
pub const GAME_FILE: &str = "game.json";

const HIDDEN_FIELDS: [&str; 2] = ["hidden", "ishidden"];
const FAVORITE_FIELDS: [&str; 4] = ["favorite", "isfavorite", "favourite", "isfavourite"];
const FOLDER_FIELDS: [&str; 3] = ["folder", "folders", "foldername"];
const FOLDER_LIST_KEYS: [&str; 3] = ["folders", "libraryfolders", "gamefolders"];
const FAVORITE_LIST_KEYS: [&str; 4] =
    ["favorites", "favoritegames", "favourites", "favouritegames"];
const HIDDEN_LIST_KEYS: [&str; 2] = ["hidden", "hiddengames"];

type Obj = serde_json::Map<String, serde_json::Value>;

/// A game as the loader sees it: its folder name and the ways other files may refer to it.
struct Game {
    folder: String,
    refs: BTreeSet<String>,
}

/// Counts what the library in `root` (a data folder that contains `Games`) holds. Read only.
pub fn scan_library(root: &Path) -> LibraryCounts {
    let mut counts = LibraryCounts::default();
    let mut games: Vec<Game> = Vec::new();
    // Per category: game folder names, and where the information came from.
    let mut hidden: BTreeSet<String> = BTreeSet::new();
    let mut favorites: BTreeSet<String> = BTreeSet::new();
    let mut folders: BTreeMap<String, String> = BTreeMap::new(); // lowercase -> shown name
    let mut found: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();

    // 1. Games and their own fields.
    if let Ok(read) = fs::read_dir(root.join(GAMES_DIR)) {
        let mut entries: Vec<_> = read.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let mut ignore = |reason: &str| {
                counts.ignored.push(IgnoredEntry {
                    name: name.clone(),
                    reason: reason.to_string(),
                })
            };
            let Ok(ft) = entry.file_type() else {
                ignore("could not be read");
                continue;
            };
            if !ft.is_dir() {
                ignore("not a folder");
                continue;
            }
            if name.starts_with('_') {
                ignore("skipped by name (starts with \"_\")");
                continue;
            }
            if name.starts_with('.') {
                ignore("skipped by name (starts with \".\")");
                continue;
            }
            let file = entry.path().join(GAME_FILE);
            if !file.is_file() {
                ignore("no game.json");
                continue;
            }
            let obj = match fs::read(&file)
                .map_err(|e| e.to_string())
                .and_then(|b| parse_object(&b))
            {
                Ok(o) => o,
                Err(e) => {
                    ignore(&format!("game.json could not be read: {e}"));
                    continue;
                }
            };
            let mut refs = BTreeSet::from([name.to_lowercase()]);
            for (k, v) in &obj {
                let key = norm(k);
                if ["id", "guid", "gameid", "title", "name"].contains(&key.as_str()) {
                    if let Some(s) = v.as_str().filter(|s| !s.trim().is_empty()) {
                        refs.insert(s.trim().to_lowercase());
                    }
                    if let Some(n) = v.as_i64() {
                        refs.insert(n.to_string());
                    }
                }
                if HIDDEN_FIELDS.contains(&key.as_str()) && v.as_bool() == Some(true) {
                    hidden.insert(name.clone());
                    found
                        .entry("hidden")
                        .or_default()
                        .insert(format!("game.json field \"{k}\""));
                }
                if FAVORITE_FIELDS.contains(&key.as_str()) && v.as_bool() == Some(true) {
                    favorites.insert(name.clone());
                    found
                        .entry("favorites")
                        .or_default()
                        .insert(format!("game.json field \"{k}\""));
                }
                if FOLDER_FIELDS.contains(&key.as_str()) {
                    for f in strings(v) {
                        folders.entry(f.to_lowercase()).or_insert(f);
                        found
                            .entry("folders")
                            .or_default()
                            .insert(format!("game.json field \"{k}\""));
                    }
                }
            }
            games.push(Game { folder: name, refs });
        }
    }
    counts.games = games.len() as u32;

    // 2. Lists in top-level .json files of the data folder.
    let mut unmatched: Vec<String> = Vec::new();
    let mut files: Vec<_> = fs::read_dir(root)
        .map(|r| r.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    files.sort();
    for path in files {
        let is_json = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("json"));
        if !is_json || !path.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let stem = norm(&path.file_stem().unwrap_or_default().to_string_lossy());
        let Ok(value) = fs::read(&path).map_err(|e| e.to_string()).and_then(|b| {
            serde_json::from_slice::<serde_json::Value>(strip_bom(&b)).map_err(|e| e.to_string())
        }) else {
            continue;
        };
        // (category, where, list)
        let mut lists: Vec<(&'static str, String, &serde_json::Value)> = Vec::new();
        match &value {
            serde_json::Value::Array(_) => {
                for (cat, keys) in [
                    ("folders", &FOLDER_LIST_KEYS[..]),
                    ("favorites", &FAVORITE_LIST_KEYS[..]),
                    ("hidden", &HIDDEN_LIST_KEYS[..]),
                ] {
                    if keys.contains(&stem.as_str()) {
                        lists.push((cat, file_name.clone(), &value));
                    }
                }
            }
            serde_json::Value::Object(obj) => {
                for (k, v) in obj {
                    if !v.is_array() {
                        continue;
                    }
                    let key = norm(k);
                    let at = format!("{file_name} key \"{k}\"");
                    if FOLDER_LIST_KEYS.contains(&key.as_str()) {
                        lists.push(("folders", at, v));
                    } else if FAVORITE_LIST_KEYS.contains(&key.as_str()) {
                        lists.push(("favorites", at, v));
                    } else if HIDDEN_LIST_KEYS.contains(&key.as_str()) {
                        lists.push(("hidden", at, v));
                    }
                }
            }
            _ => {}
        }
        for (cat, at, list) in lists {
            let items = list.as_array().map(|a| a.as_slice()).unwrap_or_default();
            if items.is_empty() {
                found
                    .entry(cat)
                    .or_default()
                    .insert(format!("{at} (empty)"));
                continue;
            }
            found.entry(cat).or_default().insert(at.clone());
            for item in items {
                if cat == "folders" {
                    if let Some(n) = item_name(item) {
                        folders.entry(n.to_lowercase()).or_insert(n);
                    }
                    continue;
                }
                let Some(r) = item_ref(item) else { continue };
                match games.iter().find(|g| g.refs.contains(&r.to_lowercase())) {
                    Some(g) => {
                        let set = if cat == "hidden" {
                            &mut hidden
                        } else {
                            &mut favorites
                        };
                        set.insert(g.folder.clone());
                    }
                    None => unmatched.push(format!("{at} lists \"{r}\", which matches no game")),
                }
            }
        }
    }

    counts.hidden = hidden.len() as u32;
    counts.favorites = favorites.len() as u32;
    counts.folders = folders.len() as u32;
    for cat in ["hidden", "favorites", "folders"] {
        let n = match cat {
            "hidden" => counts.hidden,
            "favorites" => counts.favorites,
            _ => counts.folders,
        };
        match found.get(cat) {
            Some(places) => counts.sources.push(format!(
                "{cat}: {n}, read from {}",
                places.iter().cloned().collect::<Vec<_>>().join(", ")
            )),
            None => counts.sources.push(format!(
                "{cat}: 0, no {cat} data found in game.json fields or top-level .json lists"
            )),
        }
    }
    counts.sources.extend(unmatched);
    counts
}

fn norm(k: &str) -> String {
    k.to_lowercase().replace(['_', ' ', '-'], "")
}

fn strip_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes)
}

fn parse_object(bytes: &[u8]) -> Result<Obj, String> {
    match serde_json::from_slice::<serde_json::Value>(strip_bom(bytes)) {
        Ok(serde_json::Value::Object(map)) => Ok(map),
        Ok(_) => Err("not a JSON object".into()),
        Err(e) => Err(e.to_string()),
    }
}

fn strings(v: &serde_json::Value) -> Vec<String> {
    match v {
        serde_json::Value::String(s) if !s.trim().is_empty() => vec![s.trim().to_string()],
        serde_json::Value::Array(a) => a
            .iter()
            .filter_map(|x| x.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect(),
        _ => vec![],
    }
}

/// A folder's name: a text item, or an object's Name / Title.
fn item_name(item: &serde_json::Value) -> Option<String> {
    match item {
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        serde_json::Value::Object(o) => o
            .iter()
            .find(|(k, v)| ["name", "title"].contains(&norm(k).as_str()) && v.is_string())
            .and_then(|(_, v)| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        _ => None,
    }
}

/// How a list item refers to a game: a text or number, or an object's Id / Guid / GameId /
/// Folder / Title / Name.
fn item_ref(item: &serde_json::Value) -> Option<String> {
    match item {
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Object(o) => ["id", "guid", "gameid", "folder", "title", "name"]
            .iter()
            .find_map(|want| {
                o.iter().find(|(k, _)| norm(k) == *want).and_then(|(_, v)| {
                    v.as_str()
                        .map(|s| s.trim().to_string())
                        .or_else(|| v.as_i64().map(|n| n.to_string()))
                })
            }),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::path::PathBuf;

    fn game(games: &Path, name: &str, json: &str) {
        let d = games.join(name);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join(GAME_FILE), json).unwrap();
        fs::write(d.join("cover.png"), [1u8, 2, 3]).unwrap();
    }

    /// Like the user's data: 10 games, `_Removed`, no hidden games, favorites or folders.
    pub fn make_user_like(root: &Path) -> PathBuf {
        let games = root.join(GAMES_DIR);
        for i in 1..=10 {
            game(
                &games,
                &format!("Game {i:02}"),
                &format!(r#"{{"Title":"Game {i}","Id":"g{i}"}}"#),
            );
        }
        game(&games, "_Removed", r#"{"Title":"Old"}"#);
        fs::write(
            root.join("settings.json"),
            r#"{"Theme":"Switch","Volume":40}"#,
        )
        .unwrap();
        games
    }

    /// A library with hidden games, favorites and folders in both kinds of places, plus every
    /// kind of skipped entry.
    pub fn make_library(root: &Path) -> PathBuf {
        let games = root.join(GAMES_DIR);
        game(
            &games,
            "Celeste",
            r#"{"Title":"Celeste","Id":"c1","IsFavorite":true,"Folder":"Platformers"}"#,
        );
        game(&games, "Hades", "\u{feff}{\"Title\":\"Hades\",\"Id\":\"h1\",\"Hidden\":true,\"Favorite\":true,\"Folders\":[\"Roguelikes\",\"platformers\"]}");
        game(&games, "Tunic", r#"{"Title":"Tunic","Id":"t1"}"#);
        game(
            &games,
            "Outer Wilds",
            r#"{"Title":"Outer Wilds","Id":"o1"}"#,
        );
        game(&games, "_Removed", r#"{"Title":"Old"}"#);
        game(&games, ".cache", r#"{}"#);
        game(&games, "Broken", "{not json");
        fs::create_dir_all(games.join("NoJson")).unwrap();
        fs::write(games.join("notes.txt"), "x").unwrap();
        // Top-level lists: folders as objects, favorites by title, hidden by id; one stray.
        fs::write(
            root.join("library.json"),
            r#"{"Folders":[{"Name":"Story"},{"Name":"Platformers"}],"FavoriteGames":["Tunic","Nope"],"HiddenGames":["o1"]}"#,
        )
        .unwrap();
        fs::write(root.join("settings.json"), r#"{"Theme":"Switch"}"#).unwrap();
        games
    }

    #[test]
    fn users_data_reads_10_0_0_0() {
        let dir = tempfile::tempdir().unwrap();
        make_user_like(dir.path());
        let c = scan_library(dir.path());
        assert_eq!((c.games, c.hidden, c.favorites, c.folders), (10, 0, 0, 0));
        assert_eq!(c.ignored.len(), 1);
        assert_eq!(c.ignored[0].name, "_Removed");
        assert!(c
            .sources
            .iter()
            .any(|s| s.starts_with("folders: 0, no folders data found")));
    }

    #[test]
    fn counts_hidden_favorites_and_folders_separately() {
        let dir = tempfile::tempdir().unwrap();
        make_library(dir.path());
        let c = scan_library(dir.path());
        assert_eq!(c.games, 4);
        // Hidden: Hades (field) + Outer Wilds (list, by id).
        assert_eq!(c.hidden, 2);
        // Favorites: Celeste + Hades (fields) + Tunic (list, by title); "Nope" matches nothing.
        assert_eq!(c.favorites, 3);
        // Folders: Platformers (field and list, counted once), Roguelikes, Story.
        assert_eq!(c.folders, 3);
        assert!(c
            .sources
            .iter()
            .any(|s| s.contains("\"Nope\", which matches no game")));
        assert!(c
            .sources
            .iter()
            .any(|s| s.starts_with("hidden: 2, read from") && s.contains("HiddenGames")));
    }

    #[test]
    fn every_skipped_entry_has_a_reason() {
        let dir = tempfile::tempdir().unwrap();
        make_library(dir.path());
        let c = scan_library(dir.path());
        let got: Vec<_> = c
            .ignored
            .iter()
            .map(|i| (i.name.as_str(), i.reason.as_str()))
            .collect();
        assert_eq!(got[0], (".cache", "skipped by name (starts with \".\")"));
        assert_eq!(got[3], ("_Removed", "skipped by name (starts with \"_\")"));
        assert_eq!(got[2], ("NoJson", "no game.json"));
        assert_eq!(got[4], ("notes.txt", "not a folder"));
        assert!(got[1].1.starts_with("game.json could not be read"));
    }

    #[test]
    fn list_files_named_after_their_content() {
        let dir = tempfile::tempdir().unwrap();
        let games = dir.path().join(GAMES_DIR);
        game(&games, "A", r#"{"Title":"Alpha"}"#);
        game(&games, "B", r#"{"Title":"Beta"}"#);
        fs::write(
            dir.path().join("favorites.json"),
            r#"["a", {"Title":"beta"}]"#,
        )
        .unwrap();
        fs::write(dir.path().join("folders.json"), r#"["One","Two","one"]"#).unwrap();
        fs::write(dir.path().join("hidden.json"), r#"[]"#).unwrap();
        let c = scan_library(dir.path());
        assert_eq!((c.games, c.hidden, c.favorites, c.folders), (2, 0, 2, 2));
        assert!(c.sources.iter().any(|s| s.contains("hidden.json (empty)")));
    }

    #[test]
    fn missing_library_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let c = scan_library(&dir.path().join("nothing"));
        assert_eq!((c.games, c.hidden, c.favorites, c.folders), (0, 0, 0, 0));
        assert!(c.ignored.is_empty());
    }
}
