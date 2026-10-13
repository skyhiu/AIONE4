//! AIONE 4's own game library (T1.1; B-6, L-17, A9).
//!
//! **Layout (A9: compatible with WPF):** `<data>\Games\<folder>\game.json` plus the game's
//! media, exactly the WPF layout and field names. AIONE 4 adds one field, `AioneSchema`, to each
//! `game.json` it writes, and keeps every other field as it found it (also fields it does not
//! know). A record `<data>\library.json` says where the library came from.
//!
//! **Conversion (once):** when `<data>\Games\` does not exist yet and a finished copy of the WPF
//! data exists (`Imported\AIONE-WPF\`), every game of `Imported\AIONE-WPF\Games\` is copied into
//! `Games.converting\`, its `game.json` gets `AioneSchema`, and the folder is renamed to `Games\`
//! only at the end. `Imported\AIONE-WPF\` is only read (it stays the untouched archive). The
//! conversion never runs again once `Games\` exists, even without the record.
//!
//! **What is a game:** the same rule as the copy's count (`library::scan_library`), so the games
//! that load are the games the copy counted (L-17).
//!
//! **Fields of the real WPF `game.json`** (confirmed from two of the user's files): `Name`,
//! `Category` (optional), `Shortcut` (`game.lnk` / `game.url`), `SteamAppId` (optional),
//! `Thumbnail`, `Background`, `Hero`, `Icon`, `Portrait` (file names in the game's folder),
//! `Favorite`, `Hidden`, `PlayCount`, `PlayTimeMinutes`, `LastSessionMinutes`. Names are matched
//! without regard to case; values of an unexpected type fall back to the default and are noted.
//!
//! **B-6:** every field is optional with a default; a `game.json` written by a newer AIONE 4
//! (higher `AioneSchema`) is read and never rewritten; an unreadable `game.json` or record is
//! backed up next to itself and reported, never silently dropped.

use crate::library::{self, IgnoredEntry, GAMES_DIR, GAME_FILE};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const LIBRARY_SCHEMA: u32 = 1;
pub const SCHEMA_FIELD: &str = "AioneSchema";
pub const RECORD_FILE: &str = "library.json";
const CONVERTING: &str = "Games.converting";

/// The media slots of the WPF format, in the order the tile art is chosen from.
/// Hypothesis: `Thumbnail` is the tile picture; the others are fallbacks.
pub const TILE_ART_ORDER: [&str; 4] = ["Thumbnail", "Portrait", "Hero", "Icon"];
pub const MEDIA_SLOTS: [&str; 5] = ["Thumbnail", "Background", "Hero", "Icon", "Portrait"];

type Obj = serde_json::Map<String, serde_json::Value>;

/// A game as the UI sees it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameView {
    /// The folder name (unique in `Games\`).
    pub id: String,
    pub name: String,
    /// The media slot used for the tile (`Thumbnail`, …), or none.
    pub tile_art: Option<String>,
    pub favorite: bool,
    pub hidden: bool,
    pub category: Option<String>,
    pub play_count: u64,
    pub play_time_minutes: u64,
}

/// What `load` returns to the UI and the log.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryLoad {
    /// "loaded" | "empty" | "error"
    pub state: String,
    /// Every game, hidden ones included (Home leaves them out), sorted by name.
    pub games: Vec<GameView>,
    pub source: String,
    pub skipped: Vec<IgnoredEntry>,
    /// Plain notes (conversion, backups, newer data). Also logged.
    pub notes: Vec<String>,
    /// The number of games the copy counted (from `migration.json`), for L-17.
    pub expected: Option<u32>,
}

/// `<data>\library.json`. B-6: every field has a default.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct LibraryRecord {
    pub schema: u32,
    /// "converted" (from the WPF copy) | "found" (Games\ existed without a record) | "new"
    pub origin: String,
    pub converted_from: String,
    pub at: String,
    pub app_version: String,
    pub games: u32,
    pub skipped: Vec<IgnoredEntry>,
}

pub fn games_dir(data: &Path) -> PathBuf {
    data.join(GAMES_DIR)
}

fn wpf_games(data: &Path) -> PathBuf {
    crate::migration::destination(data).join(GAMES_DIR)
}

fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn stamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

fn norm(k: &str) -> String {
    k.to_lowercase().replace(['_', ' ', '-'], "")
}

fn strip_bom(b: &[u8]) -> &[u8] {
    b.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(b)
}

fn field<'a>(obj: &'a Obj, name: &str) -> Option<&'a serde_json::Value> {
    let want = norm(name);
    obj.iter().find(|(k, _)| norm(k) == want).map(|(_, v)| v)
}

fn text(obj: &Obj, name: &str) -> Option<String> {
    match field(obj, name)? {
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn flag(obj: &Obj, name: &str) -> bool {
    match field(obj, name) {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::String(s)) => s.eq_ignore_ascii_case("true"),
        Some(serde_json::Value::Number(n)) => n.as_i64() == Some(1),
        _ => false,
    }
}

fn number(obj: &Obj, name: &str) -> u64 {
    match field(obj, name) {
        Some(serde_json::Value::Number(n)) => n
            .as_u64()
            .or_else(|| n.as_f64().map(|f| f.max(0.0) as u64))
            .unwrap_or(0),
        Some(serde_json::Value::String(s)) => s.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

/// Reads one game's `game.json` object into the view. `folder` is the game's folder (to check
/// which media files exist).
pub fn view_of(id: &str, obj: &Obj, folder: &Path) -> GameView {
    let tile_art = TILE_ART_ORDER.iter().find_map(|slot| {
        let file = text(obj, slot)?;
        media_path(folder, &file).filter(|p| p.is_file())?;
        Some(slot.to_string())
    });
    GameView {
        id: id.to_string(),
        name: text(obj, "Name").unwrap_or_else(|| id.to_string()),
        tile_art,
        favorite: flag(obj, "Favorite"),
        hidden: flag(obj, "Hidden"),
        category: text(obj, "Category"),
        play_count: number(obj, "PlayCount"),
        play_time_minutes: number(obj, "PlayTimeMinutes"),
    }
}

/// A media file name from `game.json`, resolved inside the game's folder only (no absolute
/// paths, no `..`): anything else is refused.
pub fn media_path(folder: &Path, file: &str) -> Option<PathBuf> {
    let p = Path::new(file);
    let plain = p
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)));
    if !plain || file.trim().is_empty() {
        return None;
    }
    Some(folder.join(p))
}

fn read_object(path: &Path) -> Result<Obj, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    match serde_json::from_slice::<serde_json::Value>(strip_bom(&bytes)) {
        Ok(serde_json::Value::Object(o)) => Ok(o),
        Ok(_) => Err("not a JSON object".into()),
        Err(e) => Err(e.to_string()),
    }
}

/// Backs up an unreadable file next to itself (B-6). Returns the backup's name.
/// Only one backup per unreadable file: an existing one is reused (no new copy at every start).
fn back_up(path: &Path) -> Option<String> {
    let base = format!("{}.unreadable-", path.file_name()?.to_string_lossy());
    if let Ok(read) = fs::read_dir(path.parent()?) {
        if let Some(found) = read
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|n| n.starts_with(&base))
        {
            return Some(found);
        }
    }
    let name = format!(
        "{}.unreadable-{}",
        path.file_name()?.to_string_lossy(),
        stamp()
    );
    fs::copy(path, path.with_file_name(&name)).ok()?;
    Some(name)
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<u64> {
    fs::create_dir_all(to)?;
    let mut n = 0;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let target = to.join(entry.file_name());
        if ft.is_dir() {
            n += copy_tree(&entry.path(), &target)?;
        } else if ft.is_file() {
            fs::copy(entry.path(), &target)?;
            n += 1;
        }
        // Links are not followed (as in the copy).
    }
    Ok(n)
}

/// Converts the WPF copy's games into `<data>\Games\` (once). Returns the record, or an error
/// in plain words; on error nothing is left behind except the staging folder's removal.
pub fn convert(data: &Path) -> Result<LibraryRecord, String> {
    let source = wpf_games(data);
    let counts = library::scan_library(&crate::migration::destination(data));
    let staging = data.join(CONVERTING);
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|e| format!("could not clear {CONVERTING}: {e}"))?;
    }
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let skipped_names: Vec<&str> = counts.ignored.iter().map(|i| i.name.as_str()).collect();
    let mut converted = 0u32;
    let mut entries: Vec<_> = fs::read_dir(&source)
        .map_err(|e| format!("could not read {}: {e}", source.display()))?
        .flatten()
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if skipped_names.contains(&name.as_str()) || !entry.path().is_dir() {
            continue;
        }
        let target = staging.join(&name);
        copy_tree(&entry.path(), &target).map_err(|e| format!("copying {name}: {e}"))?;
        let file = target.join(GAME_FILE);
        let mut obj = read_object(&file).map_err(|e| format!("{name}\\game.json: {e}"))?;
        obj.insert(SCHEMA_FIELD.into(), LIBRARY_SCHEMA.into());
        let text = serde_json::to_vec_pretty(&serde_json::Value::Object(obj))
            .map_err(|e| e.to_string())?;
        fs::write(&file, text).map_err(|e| format!("{name}\\game.json: {e}"))?;
        converted += 1;
    }
    fs::rename(&staging, games_dir(data)).map_err(|e| format!("could not finish: {e}"))?;
    let record = LibraryRecord {
        schema: LIBRARY_SCHEMA,
        origin: "converted".into(),
        converted_from: format!("Imported\\{}\\{GAMES_DIR}", crate::migration::COPY_NAME),
        at: now(),
        app_version: crate::version::VERSION.into(),
        games: converted,
        skipped: counts.ignored,
    };
    write_record(data, &record).map_err(|e| format!("could not write {RECORD_FILE}: {e}"))?;
    Ok(record)
}

pub fn write_record(data: &Path, record: &LibraryRecord) -> std::io::Result<()> {
    let tmp = data.join(format!("{RECORD_FILE}.tmp"));
    fs::write(&tmp, serde_json::to_vec_pretty(record).unwrap_or_default())?;
    fs::rename(&tmp, data.join(RECORD_FILE))
}

/// Reads `library.json`. `Err` = it exists but could not be read (backed up and reported).
pub fn read_record(data: &Path) -> Result<Option<LibraryRecord>, String> {
    let path = data.join(RECORD_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    match serde_json::from_slice::<LibraryRecord>(strip_bom(&bytes)) {
        Ok(r) => Ok(Some(r)),
        Err(e) => {
            let kept = back_up(&path).unwrap_or_else(|| "no backup could be made".into());
            Err(format!(
                "AIONE 4 could not read its library record ({RECORD_FILE}: {e}); it was kept as {kept}. Your games are read from the Games folder as usual."
            ))
        }
    }
}

/// Converts if needed, then loads `<data>\Games\`. Never writes to `Imported\`.
pub fn load(data: &Path) -> LibraryLoad {
    let mut notes = Vec::new();
    let games_root = games_dir(data);
    let expected = crate::migration::read_state(data)
        .and_then(|s| s.result)
        .map(|r| r.library.games);

    if !games_root.exists() {
        if crate::migration::finished_copy_exists(data) && wpf_games(data).is_dir() {
            match convert(data) {
                Ok(r) => notes.push(format!(
                    "converted {} games from {} into Games (the copy stays unchanged){}",
                    r.games,
                    r.converted_from,
                    skipped_text(&r.skipped)
                )),
                Err(e) => {
                    let _ = fs::remove_dir_all(data.join(CONVERTING));
                    return LibraryLoad {
                        state: "error".into(),
                        games: vec![],
                        source: games_root.display().to_string(),
                        skipped: vec![],
                        notes: vec![format!("the conversion of your copied games failed: {e}")],
                        expected,
                    };
                }
            }
        }
    } else {
        match read_record(data) {
            Ok(Some(r)) if r.schema > LIBRARY_SCHEMA => notes.push(format!(
                "library.json was written by a newer AIONE 4 (schema {}); read only",
                r.schema
            )),
            Ok(Some(_)) => {}
            Ok(None) => {
                // B-6: Games\ exists without a record: never convert again; note where it came from.
                let r = LibraryRecord {
                    schema: LIBRARY_SCHEMA,
                    origin: "found".into(),
                    at: now(),
                    app_version: crate::version::VERSION.into(),
                    ..Default::default()
                };
                if write_record(data, &r).is_ok() {
                    notes.push("Games existed without library.json; a new record was written (nothing converted)".into());
                }
            }
            Err(e) => notes.push(e),
        }
    }

    let counts = library::scan_library(data);
    let mut skipped = counts.ignored.clone();
    let mut games = Vec::new();
    if let Ok(read) = fs::read_dir(&games_root) {
        let mut entries: Vec<_> = read.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let id = entry.file_name().to_string_lossy().into_owned();
            if counts.ignored.iter().any(|i| i.name == id) || !entry.path().is_dir() {
                continue;
            }
            let file = entry.path().join(GAME_FILE);
            match read_object(&file) {
                Ok(obj) => {
                    if let Some(v) = field(&obj, SCHEMA_FIELD).and_then(|v| v.as_u64()) {
                        if v > LIBRARY_SCHEMA as u64 {
                            notes.push(format!(
                                "{id}\\game.json is from a newer AIONE 4 (schema {v}); read only"
                            ));
                        }
                    }
                    games.push(view_of(&id, &obj, &entry.path()));
                }
                Err(e) => {
                    // scan_library already skips unreadable files; this covers a race.
                    let kept = back_up(&file).unwrap_or_default();
                    skipped.push(IgnoredEntry {
                        name: id,
                        reason: format!("game.json could not be read ({e}); kept as {kept}"),
                    });
                }
            }
        }
    }
    // Back up every game.json the scan could not read (B-6), once per load.
    for s in skipped.iter_mut() {
        if s.reason.starts_with("game.json could not be read:") {
            if let Some(kept) = back_up(&games_root.join(&s.name).join(GAME_FILE)) {
                s.reason.push_str(&format!("; kept as {kept}"));
            }
        }
    }
    games.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.id.cmp(&b.id))
    });
    LibraryLoad {
        state: if games.is_empty() { "empty" } else { "loaded" }.into(),
        games,
        source: games_root.display().to_string(),
        skipped,
        notes,
        expected,
    }
}

fn skipped_text(s: &[IgnoredEntry]) -> String {
    if s.is_empty() {
        String::new()
    } else {
        format!("; not converted: {}", describe_skipped(s))
    }
}

pub fn describe_skipped(s: &[IgnoredEntry]) -> String {
    if s.is_empty() {
        return "none".into();
    }
    s.iter()
        .map(|i| format!("{} ({})", i.name, i.reason))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The L-17 evidence lines for the log.
pub fn log_lines(l: &LibraryLoad) -> Vec<String> {
    let hidden = l.games.iter().filter(|g| g.hidden).count();
    let mut out: Vec<String> = l.notes.iter().map(|n| format!("library: {n}")).collect();
    out.push(format!(
        "library loaded: {} games (from {}), {} shown on Home, {} hidden; skipped: {}",
        l.games.len(),
        l.source,
        l.games.len() - hidden,
        hidden,
        describe_skipped(&l.skipped)
    ));
    out.push(match l.expected {
        Some(n) if n as usize == l.games.len() => {
            format!("library check (L-17): the copy counted {n} games; {n} loaded (match)")
        }
        Some(n) => format!(
            "library check (L-17): the copy counted {n} games; {} loaded (MISMATCH)",
            l.games.len()
        ),
        None => "library check (L-17): no copy count to compare with".into(),
    });
    out
}

/// The file behind `aione-media://<id>/<slot>`, or None. Only known slots of a game folder
/// inside `<data>\Games\` are ever served.
pub fn resolve_media(data: &Path, id: &str, slot: &str) -> Option<PathBuf> {
    if !MEDIA_SLOTS.contains(&slot)
        || id.is_empty()
        || !Path::new(id)
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
        || id.contains(['/', '\\'])
    {
        return None;
    }
    let folder = games_dir(data).join(id);
    let obj = read_object(&folder.join(GAME_FILE)).ok()?;
    let file = text(&obj, slot)?;
    media_path(&folder, &file).filter(|p| p.is_file())
}

pub fn mime_of(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("ico") => "image/x-icon",
        Some("gif") => "image/gif",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/library");

    /// Builds `<data>\Imported\AIONE-WPF\Games\` like the user's: the two real game.json files
    /// (fixtures), eight more of the same shape, `_Removed`, and a finished copy record.
    fn user_like(data: &Path) {
        let games = crate::migration::destination(data).join(GAMES_DIR);
        let real = [
            ("FATAL FRAME 4", "wpf-game1.game.json"),
            ("Zero Escape The Nonary Games", "wpf-game2.game.json"),
        ];
        for (folder, fixture) in real {
            let d = games.join(folder);
            fs::create_dir_all(&d).unwrap();
            fs::copy(Path::new(FIXTURES).join(fixture), d.join(GAME_FILE)).unwrap();
            for f in [
                "thumbnail.png",
                "background.png",
                "hero.png",
                "portrait.jpg",
                "game.lnk",
            ] {
                fs::write(d.join(f), [1u8]).unwrap();
            }
        }
        for i in 3..=10 {
            let d = games.join(format!("Game {i:02}"));
            fs::create_dir_all(&d).unwrap();
            fs::write(
                d.join(GAME_FILE),
                format!(r#"{{"Name":"Game {i:02}","Shortcut":"game.lnk","Thumbnail":"thumbnail.png","Favorite":false,"Hidden":false,"PlayCount":0}}"#),
            )
            .unwrap();
            fs::write(d.join("thumbnail.png"), [1u8]).unwrap();
        }
        fs::create_dir_all(games.join("_Removed")).unwrap();
        fs::write(games.join("_Removed").join(GAME_FILE), r#"{"Name":"Old"}"#).unwrap();
        fs::copy(
            Path::new(FIXTURES).join("../migration/alpha2.migration.json"),
            data.join("migration.json"),
        )
        .unwrap();
    }

    #[test]
    fn the_real_wpf_files_read_with_every_field() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        fs::write(folder.join("thumbnail.png"), [1u8]).unwrap();
        let obj = read_object(&Path::new(FIXTURES).join("wpf-game2.game.json")).unwrap();
        let v = view_of("Zero Escape The Nonary Games", &obj, folder);
        assert_eq!(v.name, "Zero Escape The Nonary Games");
        assert_eq!(v.category.as_deref(), Some("Steam"));
        assert_eq!(v.tile_art.as_deref(), Some("Thumbnail"));
        assert!(!v.favorite && !v.hidden);
        let obj1 = read_object(&Path::new(FIXTURES).join("wpf-game1.game.json")).unwrap();
        let v1 = view_of("FATAL FRAME 4", &obj1, folder);
        assert_eq!(v1.name, "FATAL FRAME 4");
        assert_eq!(v1.category, None);
    }

    #[test]
    fn converts_once_and_loads_the_copys_ten_games() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        user_like(data);
        let before = crate::migration::fingerprint(&crate::migration::destination(data));
        let l = load(data);
        assert_eq!(l.state, "loaded");
        assert_eq!(l.games.len(), 10);
        assert_eq!(l.expected, Some(10));
        assert!(l.notes[0].starts_with("converted 10 games from Imported\\AIONE-WPF\\Games"));
        assert!(l.notes[0].contains("not converted: _Removed"));
        // Sorted by name; the real names are used.
        assert_eq!(l.games[0].name, "FATAL FRAME 4");
        assert_eq!(l.games[9].name, "Zero Escape The Nonary Games");
        // The copy in Imported\ is unchanged.
        let after = crate::migration::fingerprint(&crate::migration::destination(data));
        assert_eq!(before, after);
        // Converted files keep every field and get AioneSchema.
        let conv = read_object(
            &games_dir(data)
                .join("Zero Escape The Nonary Games")
                .join(GAME_FILE),
        )
        .unwrap();
        assert_eq!(
            conv.get("SteamAppId").and_then(|v| v.as_str()),
            Some("477740")
        );
        assert_eq!(conv.get(SCHEMA_FIELD).and_then(|v| v.as_u64()), Some(1));
        let lines = log_lines(&l);
        assert!(lines
            .iter()
            .any(|x| x.starts_with("library loaded: 10 games (from ")));
        assert!(lines.iter().any(|x| x.ends_with("10 loaded (match)")));
        // A second load converts nothing.
        let again = load(data);
        assert_eq!(again.games.len(), 10);
        assert!(again.notes.is_empty(), "{:?}", again.notes);
    }

    #[test]
    fn hidden_games_load_and_are_counted() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let g = games_dir(data).join("Secret");
        fs::create_dir_all(&g).unwrap();
        fs::write(
            g.join(GAME_FILE),
            r#"{"Name":"Secret","Hidden":true,"Favorite":"true"}"#,
        )
        .unwrap();
        let l = load(data);
        assert_eq!(l.games.len(), 1);
        assert!(l.games[0].hidden && l.games[0].favorite);
        assert!(log_lines(&l)[1].contains("0 shown on Home, 1 hidden"));
    }

    #[test]
    fn games_without_a_record_are_never_converted_again() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        user_like(data);
        fs::create_dir_all(games_dir(data)).unwrap();
        let l = load(data);
        assert_eq!(l.state, "empty");
        assert!(l.notes[0].contains("without library.json"));
        assert_eq!(read_record(data).unwrap().unwrap().origin, "found");
    }

    #[test]
    fn unreadable_record_and_game_are_backed_up_and_reported() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let g = games_dir(data);
        fs::create_dir_all(g.join("Good")).unwrap();
        fs::write(g.join("Good").join(GAME_FILE), r#"{"Name":"Good"}"#).unwrap();
        fs::create_dir_all(g.join("Bad")).unwrap();
        fs::write(g.join("Bad").join(GAME_FILE), "{oops").unwrap();
        fs::write(data.join(RECORD_FILE), "{oops").unwrap();
        let l = load(data);
        assert_eq!(l.games.len(), 1);
        assert!(l
            .notes
            .iter()
            .any(|n| n.contains("could not read its library record")));
        let bad = l.skipped.iter().find(|s| s.name == "Bad").unwrap();
        assert!(
            bad.reason.contains("kept as game.json.unreadable-"),
            "{}",
            bad.reason
        );
        let names: Vec<_> = fs::read_dir(g.join("Bad"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(names.len(), 2, "original kept, backup added");
    }

    #[test]
    fn newer_schema_is_read_and_reported() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let g = games_dir(data).join("Future");
        fs::create_dir_all(&g).unwrap();
        fs::write(
            g.join(GAME_FILE),
            r#"{"Name":"Future","AioneSchema":7,"NewThing":[1]}"#,
        )
        .unwrap();
        let l = load(data);
        assert_eq!(l.games.len(), 1);
        assert!(l
            .notes
            .iter()
            .any(|n| n.contains("newer AIONE 4 (schema 7)")));
    }

    /// B-6: the files this version writes are kept as fixtures and read by every later version.
    #[test]
    fn alpha4_stored_files_read() {
        let r: LibraryRecord = serde_json::from_slice(
            &fs::read(Path::new(FIXTURES).join("alpha4.library.json")).unwrap(),
        )
        .unwrap();
        assert_eq!((r.schema, r.games, r.origin.as_str()), (1, 10, "converted"));
        let obj = read_object(&Path::new(FIXTURES).join("alpha4.game.json")).unwrap();
        let v = view_of(
            "Zero Escape The Nonary Games",
            &obj,
            Path::new("/nonexistent"),
        );
        assert_eq!(v.name, "Zero Escape The Nonary Games");
        assert_eq!(v.tile_art, None, "no files in a missing folder");
        // An empty record (all defaults) reads too.
        let empty: LibraryRecord = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.games, 0);
    }

    #[test]
    fn media_is_served_only_from_known_slots_inside_the_game_folder() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let g = games_dir(data).join("G");
        fs::create_dir_all(&g).unwrap();
        fs::write(
            g.join(GAME_FILE),
            r#"{"Name":"G","Thumbnail":"t.png","Hero":"../../secret.png","Icon":"C:\\x.png"}"#,
        )
        .unwrap();
        fs::write(g.join("t.png"), [1u8]).unwrap();
        fs::write(data.join("secret.png"), [1u8]).unwrap();
        assert!(resolve_media(data, "G", "Thumbnail").is_some());
        assert!(resolve_media(data, "G", "Hero").is_none());
        assert!(resolve_media(data, "G", "Icon").is_none());
        assert!(resolve_media(data, "G", "Shortcut").is_none());
        assert!(resolve_media(data, "../G", "Thumbnail").is_none());
        assert!(resolve_media(data, "", "Thumbnail").is_none());
        assert_eq!(mime_of(Path::new("a.JPG")), "image/jpeg");
    }
}
