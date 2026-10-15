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
/// B-13 (rev. k): the tile picture is the game's `Icon` (filling the tile); without one, the
/// first of the other picture slots (fitted whole over a blurred, darkened copy).
pub const TILE_ART_ORDER: [&str; 4] = ["Icon", "Thumbnail", "Portrait", "Hero"];

/// How the slot becomes a tile (B-13).
pub fn tile_mode(slot: &str) -> crate::tiles::Mode {
    if slot == "Icon" {
        crate::tiles::Mode::Fill
    } else {
        crate::tiles::Mode::Fit
    }
}
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
    /// B-13: "fill" (the icon fills the tile) or "fit" (another picture fitted whole), when
    /// there is a tile picture.
    pub tile_mode: Option<String>,
    /// D-007 / D-010: the cache key of the tile-sized copy (`/<id>/tile?v=<key>`) when it
    /// already exists; otherwise the UI shows the original picture until the background copy
    /// arrives (`aione://tile` event).
    pub tile: Option<String>,
    pub favorite: bool,
    pub hidden: bool,
    pub category: Option<String>,
    pub play_count: u64,
    pub play_time_minutes: u64,
    /// B-11 (T1.3): the game's last start (RFC 3339, UTC), from `LastPlayed` in its game.json.
    pub last_played: Option<String>,
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
        // Only pictures: a slot can hold a video (a `portrait.mp4`).
        if !crate::tiles::is_picture(&file) {
            return None;
        }
        media_path(folder, &file).filter(|p| p.is_file())?;
        Some(slot.to_string())
    });
    GameView {
        id: id.to_string(),
        name: text(obj, "Name").unwrap_or_else(|| id.to_string()),
        tile_mode: tile_art.as_deref().map(|s| tile_mode(s).name().to_string()),
        tile_art,
        tile: None,
        favorite: flag(obj, "Favorite"),
        hidden: flag(obj, "Hidden"),
        category: text(obj, "Category"),
        play_count: number(obj, "PlayCount"),
        play_time_minutes: number(obj, "PlayTimeMinutes"),
        last_played: text(obj, "LastPlayed"),
    }
}

/// B-11 (T1.3): the row's order: games that have been played, most recent first; then the
/// never-played ones A-Z.
pub fn sort_for_home(games: &mut [GameView]) {
    let when = |g: &GameView| {
        g.last_played
            .as_deref()
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .map(|t| t.timestamp())
    };
    games.sort_by(|a, b| match (when(a), when(b)) {
        (Some(x), Some(y)) => y.cmp(&x).then(a.id.cmp(&b.id)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a
            .name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.id.cmp(&b.id)),
    });
}

/// What `game.json` says about starting a game (T1.3): its `Shortcut` (a `game.lnk` or
/// `game.url` in its folder) and its `SteamAppId`.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchFields {
    pub name: String,
    pub folder: PathBuf,
    pub shortcut: Option<PathBuf>,
    pub steam_app_id: Option<String>,
    /// B-20: `"NoPause": true` in game.json: never paused while AIONE is in front.
    pub no_pause: bool,
}

pub fn launch_fields(data: &Path, id: &str) -> Result<LaunchFields, String> {
    let folder = games_dir(data).join(id);
    let obj = read_object(&folder.join(GAME_FILE))?;
    let shortcut = text(&obj, "Shortcut").and_then(|f| media_path(&folder, &f));
    Ok(LaunchFields {
        name: text(&obj, "Name").unwrap_or_else(|| id.to_string()),
        shortcut,
        steam_app_id: text(&obj, "SteamAppId").filter(|v| v.chars().all(|c| c.is_ascii_digit())),
        no_pause: field(&obj, "NoPause")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        folder,
    })
}

/// B-11 (rev. ab): dates as WPF writes them: local time with the offset and microseconds
/// (e.g. `2026-01-02T03:04:05.123456+07:00`, an invented example). Both that and UTC `Z` are read.
pub fn wpf_date(t: chrono::DateTime<chrono::Local>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Micros, false)
}

fn key_for(obj: &Obj, name: &str) -> String {
    obj.keys()
        .find(|k| norm(k) == norm(name))
        .cloned()
        .unwrap_or_else(|| name.to_string())
}

/// A `game.json` read for a change; refused if a newer AIONE 4 wrote it (B-6).
fn read_for_change(path: &Path) -> Result<Obj, String> {
    let obj = read_object(path)?;
    if let Some(v) = field(&obj, SCHEMA_FIELD).and_then(|v| v.as_u64()) {
        if v > LIBRARY_SCHEMA as u64 {
            return Err(format!(
                "game.json was written by a newer AIONE 4 (schema {v}); not changed"
            ));
        }
    }
    Ok(obj)
}

fn write_atomic(path: &Path, mut obj: Obj) -> Result<(), String> {
    let k = key_for(&obj, SCHEMA_FIELD);
    obj.insert(k, LIBRARY_SCHEMA.into());
    let bytes =
        serde_json::to_vec_pretty(&serde_json::Value::Object(obj)).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes).map_err(|e| format!("could not write: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("could not write: {e}"))
}

/// WPF's minutes are numbers with a fraction (`0.5`).
fn minutes(obj: &Obj, name: &str) -> f64 {
    match field(obj, name) {
        Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0).max(0.0),
        Some(serde_json::Value::String(s)) => s.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// B-11 (rev. ab), in WPF's own fields: a start sets `LastPlayed` (now), `PlayCount` + 1 and
/// `FirstPlayed` if it is missing. Every other field (and the names' spelling) is kept; a newer
/// AIONE 4's `game.json` is never rewritten (B-6). Written atomically.
pub fn record_start(
    data: &Path,
    id: &str,
    now: chrono::DateTime<chrono::Local>,
) -> Result<(String, u64), String> {
    let path = games_dir(data).join(id).join(GAME_FILE);
    let mut obj = read_for_change(&path)?;
    let when = wpf_date(now);
    let count = number(&obj, "PlayCount") + 1;
    let k = key_for(&obj, "LastPlayed");
    obj.insert(k, when.clone().into());
    let k = key_for(&obj, "PlayCount");
    obj.insert(k, count.into());
    if text(&obj, "FirstPlayed").is_none() {
        let k = key_for(&obj, "FirstPlayed");
        obj.insert(k, when.clone().into());
    }
    write_atomic(&path, obj)?;
    Ok((when, count))
}

/// B-11 (rev. ab): a session's end: `LastSessionMinutes` = its length, `PlayTimeMinutes` += it
/// (minutes with a fraction, rounded to 0.1, as WPF stores them). Returns the new total.
pub fn record_end(data: &Path, id: &str, session_minutes: f64) -> Result<f64, String> {
    let path = games_dir(data).join(id).join(GAME_FILE);
    let mut obj = read_for_change(&path)?;
    let session = (session_minutes.max(0.0) * 10.0).round() / 10.0;
    let total = ((minutes(&obj, "PlayTimeMinutes") + session) * 10.0).round() / 10.0;
    let k = key_for(&obj, "LastSessionMinutes");
    obj.insert(k, serde_json::json!(session));
    let k = key_for(&obj, "PlayTimeMinutes");
    obj.insert(k, serde_json::json!(total));
    write_atomic(&path, obj)?;
    Ok(total)
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
                    let mut v = view_of(&id, &obj, &entry.path());
                    // D-010: only an existing copy is used here; missing copies are made in
                    // the background after Home is shown (`tile_jobs`, `make_tiles`).
                    if let Some(src) = tile_source(&obj, &entry.path(), v.tile_art.as_deref()) {
                        let mode = tile_mode(v.tile_art.as_deref().unwrap_or("Icon"));
                        v.tile = crate::tiles::existing(data, &v.id, &src, mode);
                    }
                    games.push(v);
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
    sort_for_home(&mut games);
    LibraryLoad {
        state: if games.is_empty() { "empty" } else { "loaded" }.into(),
        games,
        source: games_root.display().to_string(),
        skipped,
        notes,
        expected,
    }
}

/// The source file of a game's tile picture (its slot's file, inside the game's folder).
fn tile_source(obj: &Obj, folder: &Path, slot: Option<&str>) -> Option<PathBuf> {
    let file = text(obj, slot?)?;
    media_path(folder, &file).filter(|p| p.is_file())
}

/// One missing tile copy to make in the background (D-010).
#[derive(Debug, Clone)]
pub struct TileJob {
    pub id: String,
    pub src: PathBuf,
    pub mode: crate::tiles::Mode,
}

/// The games whose tile copy doesn't exist yet.
pub fn tile_jobs(data: &Path, load: &LibraryLoad) -> Vec<TileJob> {
    load.games
        .iter()
        .filter(|g| g.tile.is_none())
        .filter_map(|g| {
            let folder = games_dir(data).join(&g.id);
            let obj = read_object(&folder.join(GAME_FILE)).ok()?;
            let src = tile_source(&obj, &folder, g.tile_art.as_deref())?;
            Some(TileJob {
                id: g.id.clone(),
                src,
                mode: tile_mode(g.tile_art.as_deref()?),
            })
        })
        .collect()
}

/// Makes the missing copies on up to `workers` threads, calling `done` for each (from the
/// worker threads), and returns the per-copy log lines and the summary line.
pub fn make_tiles(
    data: &Path,
    jobs: Vec<TileJob>,
    workers: usize,
    done: &(dyn Fn(&str, &crate::tiles::Made) + Sync),
) -> (Vec<String>, String) {
    use crate::tiles::TileError;
    let started = std::time::Instant::now();
    let queue = std::sync::Mutex::new(jobs.into_iter());
    let lines = std::sync::Mutex::new(Vec::new());
    let counts = std::sync::Mutex::new((0u32, 0u32, 0u32, 0u32)); // made, reused, failed, skipped
    std::thread::scope(|s| {
        for _ in 0..workers.max(1) {
            s.spawn(|| loop {
                let Some(job) = queue.lock().ok().and_then(|mut q| q.next()) else {
                    break;
                };
                let file = job
                    .src
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let line = match crate::tiles::ensure(data, &job.id, &job.src, job.mode) {
                    Ok(m) => {
                        done(&job.id, &m);
                        let mut c = counts.lock().unwrap_or_else(|e| e.into_inner());
                        if m.made {
                            c.0 += 1;
                            let t = m.timing.unwrap_or_default();
                            let (w, h) = m.source_px.unwrap_or((0, 0));
                            format!(
                                "tile copy: {} from {file} ({w}×{h}, {}; {}) in {:.0} ms (decode {:.0}, scale {:.0}, encode {:.0}, write {:.0}){}",
                                job.id,
                                m.source_note,
                                job.mode.name(),
                                t.total(),
                                t.decode_ms,
                                t.scale_ms,
                                t.encode_ms,
                                t.write_ms,
                                if m.replaced {
                                    "; remade: the source changed since the last copy"
                                } else {
                                    ""
                                }
                            )
                        } else {
                            c.1 += 1;
                            continue;
                        }
                    }
                    Err(TileError::Failed(e)) => {
                        counts.lock().unwrap_or_else(|e| e.into_inner()).2 += 1;
                        format!("tile copy: {} from {file} FAILED: {e}", job.id)
                    }
                    Err(TileError::Skipped(e)) => {
                        counts.lock().unwrap_or_else(|e| e.into_inner()).3 += 1;
                        format!(
                            "tile copy: {} from {file} skipped (failed before, unchanged since): {e}",
                            job.id
                        )
                    }
                };
                lines.lock().unwrap_or_else(|e| e.into_inner()).push(line);
            });
        }
    });
    let (made, reused, failed, skipped) = counts.into_inner().unwrap_or_else(|e| e.into_inner());
    let summary = format!(
        "tile pictures: {made} made, {reused} reused, {failed} failed, {skipped} skipped (failed before), in {} ms in the background on {} threads ({}×{} JPEG in Cache\\Tiles)",
        started.elapsed().as_millis(),
        workers.max(1),
        crate::tiles::TILE_PX,
        crate::tiles::TILE_PX
    );
    (
        lines.into_inner().unwrap_or_else(|e| e.into_inner()),
        summary,
    )
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
    // D-007 / B-13: the cached tile copy.
    if slot == "tile" {
        if id.is_empty() || id.contains(['/', '\\']) || id == ".." {
            return None;
        }
        let folder = games_dir(data).join(id);
        let obj = read_object(&folder.join(GAME_FILE)).ok()?;
        let v = view_of(id, &obj, &folder);
        let slot = v.tile_art?;
        let src = tile_source(&obj, &folder, Some(&slot))?;
        let key = crate::tiles::key_of(&src, tile_mode(&slot))?;
        let p = crate::tiles::path_for(data, id, &key);
        return p.is_file().then_some(p);
    }
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
    fn png(path: &Path, w: u32, h: u32) {
        image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 200) as u8, (y % 200) as u8, 40])
        })
        .save(path)
        .unwrap();
    }

    fn user_like(data: &Path) {
        let games = crate::migration::destination(data).join(GAMES_DIR);
        let real = [
            ("Amber Lighthouse", "wpf-game1.game.json"),
            ("Zephyr Line Express", "wpf-game2.game.json"),
        ];
        for (folder, fixture) in real {
            let d = games.join(folder);
            fs::create_dir_all(&d).unwrap();
            fs::copy(Path::new(FIXTURES).join(fixture), d.join(GAME_FILE)).unwrap();
            png(&d.join("thumbnail.png"), 1200, 900);
            for f in ["background.png", "hero.png", "portrait.jpg", "game.lnk"] {
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
            png(&d.join("thumbnail.png"), 300, 300);
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
        // A video in a picture slot is skipped (the user's Portrait is an .mp4).
        let mut vid = serde_json::Map::new();
        vid.insert("Name".into(), "V".into());
        vid.insert("Portrait".into(), "portrait.mp4".into());
        fs::write(folder.join("portrait.mp4"), [1u8]).unwrap();
        assert_eq!(view_of("V", &vid, folder).tile_art, None);
        let obj = read_object(&Path::new(FIXTURES).join("wpf-game2.game.json")).unwrap();
        let v = view_of("Zephyr Line Express", &obj, folder);
        assert_eq!(v.name, "Zephyr Line Express");
        assert_eq!(v.category.as_deref(), Some("Steam"));
        assert_eq!(v.tile_art.as_deref(), Some("Thumbnail"));
        assert!(!v.favorite && !v.hidden);
        let obj1 = read_object(&Path::new(FIXTURES).join("wpf-game1.game.json")).unwrap();
        let v1 = view_of("Amber Lighthouse", &obj1, folder);
        assert_eq!(v1.name, "Amber Lighthouse");
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
        assert_eq!(l.games[0].name, "Amber Lighthouse");
        assert_eq!(l.games[9].name, "Zephyr Line Express");
        // The copy in Imported\ is unchanged.
        let after = crate::migration::fingerprint(&crate::migration::destination(data));
        assert_eq!(before, after);
        // Converted files keep every field and get AioneSchema.
        let conv =
            read_object(&games_dir(data).join("Zephyr Line Express").join(GAME_FILE)).unwrap();
        assert_eq!(
            conv.get("SteamAppId").and_then(|v| v.as_str()),
            Some("123450")
        );
        assert_eq!(conv.get(SCHEMA_FIELD).and_then(|v| v.as_u64()), Some(1));
        let lines = log_lines(&l);
        assert!(lines
            .iter()
            .any(|x| x.starts_with("library loaded: 10 games (from ")));
        assert!(lines.iter().any(|x| x.ends_with("10 loaded (match)")));
        // D-010: the load makes no copies (Home is shown at once); the background job does.
        assert!(l.games.iter().all(|g| g.tile.is_none()));
        // B-13: these fixtures name an icon file that doesn't exist, so the thumbnail is fitted.
        assert_eq!(l.games[0].tile_art.as_deref(), Some("Thumbnail"));
        assert_eq!(l.games[0].tile_mode.as_deref(), Some("fit"));
        let jobs = tile_jobs(data, &l);
        assert_eq!(jobs.len(), 10);
        let seen = std::sync::Mutex::new(Vec::new());
        let (lines, summary) = make_tiles(data, jobs, 4, &|id, m| {
            assert!(m.made);
            seen.lock().unwrap().push(id.to_string());
        });
        assert_eq!(seen.lock().unwrap().len(), 10);
        assert!(
            summary.starts_with("tile pictures: 10 made, 0 reused, 0 failed, 0 skipped"),
            "{summary}"
        );
        assert_eq!(lines.len(), 10);
        let amber = lines
            .iter()
            .find(|x| x.contains("Amber Lighthouse"))
            .unwrap();
        assert!(
            amber.contains("from thumbnail.png (1200×900, picture; fit) in "),
            "{amber}"
        );
        assert!(amber.contains("(decode "), "{amber}");
        let tile = resolve_media(data, "Amber Lighthouse", "tile").unwrap();
        assert_eq!(image::open(tile).unwrap().width(), crate::tiles::TILE_PX);
        // A second load converts nothing and finds every copy.
        let again = load(data);
        assert_eq!(again.games.len(), 10);
        assert!(again.notes.is_empty(), "{:?}", again.notes);
        assert!(again.games.iter().all(|g| g.tile.is_some()));
        assert!(tile_jobs(data, &again).is_empty());
    }

    /// D-010: a first start with 20 new pictures: the library (and so Home) is ready without
    /// waiting for any copy; the copies follow in the background.
    #[test]
    fn a_first_start_with_20_new_pictures_does_not_wait_for_copies() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        for i in 0..20 {
            let g = games_dir(data).join(format!("Game {i:02}"));
            fs::create_dir_all(&g).unwrap();
            fs::write(
                g.join(GAME_FILE),
                format!(
                    r#"{{"Name":"Game {i:02}","Icon":"icon.png","Thumbnail":"thumbnail.png"}}"#
                ),
            )
            .unwrap();
            png(&g.join("icon.png"), 512, 512);
            png(&g.join("thumbnail.png"), 920, 430);
        }
        let t = std::time::Instant::now();
        let l = load(data);
        let load_ms = t.elapsed().as_millis();
        assert_eq!(l.games.len(), 20);
        assert!(
            l.games
                .iter()
                .all(|g| g.tile_mode.as_deref() == Some("fill")),
            "the icon fills"
        );
        assert!(load_ms < 1000, "load took {load_ms} ms");
        let t = std::time::Instant::now();
        let (_, summary) = make_tiles(data, tile_jobs(data, &l), 4, &|_, _| {});
        eprintln!(
            "first start: load {load_ms} ms; 20 copies {} ms (debug build) — {summary}",
            t.elapsed().as_millis()
        );
        assert!(summary.starts_with("tile pictures: 20 made"));
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
        let v = view_of("Zephyr Line Express", &obj, Path::new("/nonexistent"));
        assert_eq!(v.name, "Zephyr Line Express");
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
        // The tile copy is served only once made; a video is never a tile.
        assert!(resolve_media(data, "G", "tile").is_none());
        assert!(resolve_media(data, "../G", "tile").is_none());
        assert!(resolve_media(data, "G", "Thumbnail").is_some());
        assert!(resolve_media(data, "G", "Hero").is_none());
        assert!(resolve_media(data, "G", "Icon").is_none());
        assert!(resolve_media(data, "G", "Shortcut").is_none());
        assert!(resolve_media(data, "../G", "Thumbnail").is_none());
        assert!(resolve_media(data, "", "Thumbnail").is_none());
        assert_eq!(mime_of(Path::new("a.JPG")), "image/jpeg");
    }

    /// B-11 (rev. ab): starts and sessions in WPF's own fields and date style; the row puts played
    /// games first (most recent first), then the rest A-Z. Invented titles and values.
    #[test]
    fn starts_are_recorded_and_order_the_row() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        for (id, name) in [("a", "Zelda"), ("b", "Amber"), ("c", "Moss")] {
            let f = games_dir(data).join(id);
            fs::create_dir_all(&f).unwrap();
            fs::write(f.join(GAME_FILE), format!(r#"{{"Name":"{name}","Shortcut":"game.lnk","SteamAppId":"1234","Extra":{{"kept":true}},"playcount":2}}"#)).unwrap();
        }
        let at = |s: &str| {
            chrono::DateTime::parse_from_rfc3339(s)
                .unwrap()
                .with_timezone(&chrono::Local)
        };
        let (when, n) = record_start(data, "c", at("2026-10-16T10:00:00.000001+07:00")).unwrap();
        assert_eq!(n, 3);
        // WPF's style: local time with the offset and microseconds (never UTC "Z").
        assert!(
            chrono::DateTime::parse_from_rfc3339(&when).is_ok()
                && !when.ends_with('Z')
                && when.contains('.'),
            "{when}"
        );
        record_start(data, "a", at("2026-10-16T11:00:00.000001+07:00")).unwrap();
        let obj = read_object(&games_dir(data).join("a").join(GAME_FILE)).unwrap();
        assert_eq!(obj.get("Extra"), Some(&serde_json::json!({"kept": true})));
        assert_eq!(obj.get("playcount"), Some(&serde_json::json!(3))); // the original spelling kept
        assert_eq!(obj.get("LastPlayed"), obj.get("FirstPlayed")); // FirstPlayed set when missing
                                                                   // A second start keeps FirstPlayed.
        record_start(data, "a", at("2026-10-16T12:00:00.000001+07:00")).unwrap();
        let obj2 = read_object(&games_dir(data).join("a").join(GAME_FILE)).unwrap();
        assert_eq!(obj2.get("FirstPlayed"), obj.get("FirstPlayed"));
        assert_ne!(obj2.get("LastPlayed"), obj.get("LastPlayed"));
        // A session's end: LastSessionMinutes = its length, PlayTimeMinutes += it.
        fs::write(
            games_dir(data).join("b").join(GAME_FILE),
            r#"{"Name":"Amber","PlayTimeMinutes":0.5,"LastSessionMinutes":0.5}"#,
        )
        .unwrap();
        assert_eq!(record_end(data, "b", 12.34).unwrap(), 12.8);
        let o = read_object(&games_dir(data).join("b").join(GAME_FILE)).unwrap();
        assert_eq!(o.get("LastSessionMinutes"), Some(&serde_json::json!(12.3)));
        assert_eq!(o.get("PlayTimeMinutes"), Some(&serde_json::json!(12.8)));
        // An alpha.15 UTC "Z" date is still read for the order.
        fs::write(
            games_dir(data).join("b").join(GAME_FILE),
            r#"{"Name":"Amber","LastPlayed":"2026-10-16T03:30:00Z"}"#,
        )
        .unwrap();
        let mut games: Vec<GameView> = ["a", "b", "c"]
            .iter()
            .map(|id| {
                let f = games_dir(data).join(id);
                view_of(id, &read_object(&f.join(GAME_FILE)).unwrap(), &f)
            })
            .collect();
        sort_for_home(&mut games);
        let order: Vec<_> = games.iter().map(|g| g.name.as_str()).collect();
        // Zelda 12:00+07 (05:00Z), Moss 10:00+07 (03:00Z), Amber 03:30Z: most recent first.
        assert_eq!(order, ["Zelda", "Amber", "Moss"]);
        let lf = launch_fields(data, "c").unwrap();
        assert_eq!(
            lf.shortcut,
            Some(games_dir(data).join("c").join("game.lnk"))
        );
        assert_eq!(lf.steam_app_id.as_deref(), Some("1234"));
        // A newer AIONE 4's game.json is never rewritten.
        fs::write(
            games_dir(data).join("b").join(GAME_FILE),
            r#"{"Name":"Amber","AioneSchema":9}"#,
        )
        .unwrap();
        assert!(record_start(data, "b", at("2026-10-16T12:00:00+07:00"))
            .unwrap_err()
            .contains("newer AIONE 4"));
        assert!(record_end(data, "b", 1.0)
            .unwrap_err()
            .contains("newer AIONE 4"));
    }
}
