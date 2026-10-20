//! B-24 (rev. at): AIONE 4's own settings store, and the one-time import from WPF AIONE.
//!
//! **The store:** `<data>\settings.json`, AIONE 4's own clean schema (versioned, B-6), with its own
//! key names: never a mirror of WPF's `Config`. Every key has a default; an unreadable file or an
//! invalid value falls back to it. Changes are saved at once and survive restarts.
//!
//! **The import (once):** from the copy T0 made of WPF AIONE's data
//! (`<data>\Imported\AIONE-WPF\Config\settings.json`; WPF's folder itself is never read again),
//! only values that clearly map to an AIONE 4 setting and are valid are taken; everything doubtful,
//! contradictory, obsolete or without an equivalent is left out (the default is used). The import
//! is recorded in the store (`importedFromWpf`), so it never runs again, and later changes in AIONE
//! 4 always win. `<data>\settings-import.md` lists every value taken (from what, to what) and every
//! value left out (and why).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub schema: u32,
    // ----- Audio (Settings › Audio). AIONE's own; never Windows' volume or mute. -----
    /// AIONE's volume, 0–100 % (all of AIONE's sounds).
    pub volume: u8,
    /// AIONE's mute (all of AIONE's sounds).
    pub muted: bool,
    /// Music's level relative to AIONE's volume, 0–100 %.
    pub music_volume: u8,
    /// Menu sounds' level relative to AIONE's volume, 0–100 %.
    pub menu_sound_volume: u8,
    /// Menu sounds on or off.
    pub menu_sounds: bool,
    // ----- Home (Settings › Library) -----
    /// "recent" (recently played first; the only order AIONE 4 has so far).
    pub sort_order: String,
    pub favorites_first: bool,
    pub show_hidden_games: bool,
    // ----- System (Settings › System) -----
    pub clock_24h: bool,
    pub start_with_windows: bool,
    // ----- Controls (Settings › Controls) -----
    /// Swap A / B (confirm / back).
    pub swap_confirm_back: bool,
    // ----- Boot (Settings › System) -----
    /// Show the Boot Up presentation (the custom one if there is one, else the default).
    pub boot_up: bool,
    /// Boot Up's sound.
    pub boot_up_audio: bool,
    // ----- Theme Settings (a separate screen, B-25) -----
    /// The theme ("switch": the only one AIONE 4 has so far).
    pub theme: String,
    /// The colour preset ("light" / "dark").
    pub color_preset: String,
    // ----- Bookkeeping -----
    /// The one-time import (B-6 / B-24): when, and from which file. Never runs again once set.
    pub imported_from_wpf: Option<ImportRecord>,
    /// B-24 rev. aw: the one-time reset of alpha.30's imported volume to 100 % (done once).
    pub volume_reset_done: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRecord {
    pub at: String,
    pub source: String,
    pub taken: usize,
    pub left_out: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            schema: SCHEMA,
            volume: 100,
            muted: false,
            music_volume: 100,
            menu_sound_volume: 100,
            menu_sounds: true,
            sort_order: "recent".into(),
            favorites_first: true,
            show_hidden_games: false,
            clock_24h: false,
            start_with_windows: false,
            swap_confirm_back: false,
            boot_up: true,
            boot_up_audio: true,
            theme: "switch".into(),
            color_preset: "light".into(),
            imported_from_wpf: None,
            volume_reset_done: false,
        }
    }
}

impl Settings {
    /// Every value inside its range; anything else back to its default.
    pub fn sanitized(mut self) -> Settings {
        let d = Settings::default();
        self.schema = SCHEMA;
        self.volume = self.volume.min(100);
        self.music_volume = self.music_volume.min(100);
        self.menu_sound_volume = self.menu_sound_volume.min(100);
        if self.sort_order != "recent" {
            self.sort_order = d.sort_order;
        }
        if self.theme != "switch" {
            self.theme = d.theme;
        }
        if self.color_preset != "light" && self.color_preset != "dark" {
            self.color_preset = d.color_preset;
        }
        self
    }
}

pub fn path(data: &Path) -> PathBuf {
    data.join("settings.json")
}

/// Reads the store (defaults where it's missing or unreadable).
pub fn load(data: &Path) -> Settings {
    std::fs::read_to_string(path(data))
        .ok()
        .and_then(|t| serde_json::from_str::<Settings>(&t).ok())
        .unwrap_or_default()
        .sanitized()
}

/// Saves the store (written to a temporary file, then moved into place).
pub fn save(data: &Path, s: &Settings) -> Result<(), String> {
    let p = path(data);
    let tmp = p.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, text).map_err(|e| format!("couldn't write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("couldn't save {}: {e}", p.display()))
}

/// Applies a change from the UI (a partial object of AIONE 4's keys), sanitized, and saves it.
pub fn change(data: &Path, patch: &Value) -> Result<Settings, String> {
    let current = serde_json::to_value(load(data)).map_err(|e| e.to_string())?;
    let mut merged = current;
    if let (Some(m), Some(p)) = (merged.as_object_mut(), patch.as_object()) {
        for (k, v) in p {
            if k != "schema"
                && k != "importedFromWpf"
                && k != "volumeResetDone"
                && m.contains_key(k)
            {
                m.insert(k.clone(), v.clone());
            }
        }
    }
    let s = serde_json::from_value::<Settings>(merged)
        .map_err(|e| format!("invalid setting: {e}"))?
        .sanitized();
    save(data, &s)?;
    Ok(s)
}

// ---------- the one-time import from WPF AIONE ----------

/// The copy of WPF AIONE's settings that T0 made.
pub fn wpf_source(data: &Path) -> PathBuf {
    data.join("Imported")
        .join("AIONE-WPF")
        .join("Config")
        .join("settings.json")
}

pub struct Import {
    pub settings: Settings,
    /// (WPF key, its value, AIONE 4 setting, the new value)
    pub taken: Vec<(String, String, String, String)>,
    /// (WPF key, its value, why it was left out)
    pub left_out: Vec<(String, String, String)>,
}

fn fraction_to_percent(v: &Value) -> Option<u8> {
    let f = v.as_f64()?;
    (0.0..=1.0).contains(&f).then(|| (f * 100.0).round() as u8)
}

/// Maps WPF AIONE's `settings.json` (its `SettingsVersion` 5 format) onto AIONE 4's settings.
pub fn import_from(wpf: &Value, base: Settings) -> Import {
    let mut s = base;
    let mut taken = Vec::new();
    let mut left_out = Vec::new();
    let Some(obj) = wpf.as_object() else {
        return Import {
            settings: s,
            taken,
            left_out: vec![(
                "(the file)".into(),
                String::new(),
                "not a JSON object: nothing imported".into(),
            )],
        };
    };
    let shown = |v: &Value| v.to_string();
    for (key, v) in obj {
        let mut take =
            |to: &str, value: String| taken.push((key.clone(), shown(v), to.to_string(), value));
        let why: Option<&str> = match key.as_str() {
            // B-24 rev. aw: WPF's master volume belonged to its older audio model (tied to Windows'
            // volume), so its meaning changed: AIONE 4 starts at 100 %.
            "MasterVolume" => Some("WPF's master volume belonged to its older audio model (tied to Windows' volume), so its meaning changed: AIONE 4 starts at 100 %"),
            "Mute" => match v.as_bool() {
                Some(b) => {
                    s.muted = b;
                    take("muted", b.to_string());
                    None
                }
                None => Some("not true / false"),
            },
            "MusicVolume" => match fraction_to_percent(v) {
                Some(p) => {
                    s.music_volume = p;
                    take("musicVolume", format!("{p} %"));
                    None
                }
                None => Some("not a level between 0 and 1"),
            },
            "SfxVolume" => match fraction_to_percent(v) {
                Some(p) => {
                    s.menu_sound_volume = p;
                    take("menuSoundVolume", format!("{p} %"));
                    None
                }
                None => Some("not a level between 0 and 1"),
            },
            "MenuSounds" => match v.as_bool() {
                Some(b) => {
                    s.menu_sounds = b;
                    take("menuSounds", b.to_string());
                    None
                }
                None => Some("not true / false"),
            },
            "SortBy" => match v.as_str() {
                Some("RecentlyPlayed") => {
                    s.sort_order = "recent".into();
                    take("sortOrder", "recent".into());
                    None
                }
                _ => Some("no equivalent order in AIONE 4 yet (only recently played)"),
            },
            "FavoritesFirst" => match v.as_bool() {
                Some(b) => {
                    s.favorites_first = b;
                    take("favoritesFirst", b.to_string());
                    None
                }
                None => Some("not true / false"),
            },
            "ShowHiddenGames" => match v.as_bool() {
                Some(b) => {
                    s.show_hidden_games = b;
                    take("showHiddenGames", b.to_string());
                    None
                }
                None => Some("not true / false"),
            },
            "Use24HourClock" => match v.as_bool() {
                Some(b) => {
                    s.clock_24h = b;
                    take("clock24h", b.to_string());
                    None
                }
                None => Some("not true / false"),
            },
            "StartWithWindows" => match v.as_bool() {
                Some(b) => {
                    s.start_with_windows = b;
                    take("startWithWindows", b.to_string());
                    None
                }
                None => Some("not true / false"),
            },
            "SwapConfirmBack" => match v.as_bool() {
                Some(b) => {
                    s.swap_confirm_back = b;
                    take("swapConfirmBack", b.to_string());
                    None
                }
                None => Some("not true / false"),
            },
            "ShowSplash" => match v.as_bool() {
                Some(b) => {
                    s.boot_up = b;
                    take("bootUp", b.to_string());
                    None
                }
                None => Some("not true / false"),
            },
            "SplashAudio" => match v.as_bool() {
                Some(b) => {
                    s.boot_up_audio = b;
                    take("bootUpAudio", b.to_string());
                    None
                }
                None => Some("not true / false"),
            },
            "Theme" => match v.as_str() {
                Some("Switch") => {
                    s.theme = "switch".into();
                    take("theme", "switch".into());
                    None
                }
                _ => Some("AIONE 4 has only the Switch theme so far"),
            },
            "ThemeProfiles" => {
                // Only the active theme's colour preset maps (Light / Dark); the rest is WPF-only.
                let active = obj.get("Theme").and_then(Value::as_str).unwrap_or("");
                let palette = v.get(active).and_then(|p| p.get("Palette")).and_then(Value::as_str).unwrap_or("");
                match (active, palette) {
                    ("Switch", "Light") | ("Switch", "Dark") => {
                        s.color_preset = palette.to_ascii_lowercase();
                        taken.push((format!("ThemeProfiles.{active}.Palette"), format!("\"{palette}\""), "colorPreset".into(), s.color_preset.clone()));
                        Some("the other theme profiles and the tile, dimming, font and logo fields: WPF-only, no AIONE 4 equivalent")
                    }
                    _ => Some("no Light / Dark palette for the active theme: AIONE 4's default preset is used"),
                }
            }
            "SettingsVersion" => Some("WPF's own format number"),
            "LastSelectedGameId" | "GamesFolder" => Some("state or a path, not a setting (the library and data folder are handled by the copy)"),
            "ShowClock" | "ShowBattery" | "ShowNetwork" | "ShowControllerStatus" | "ShowVolume" | "ShowControllerBattery" | "ShowBatteryPercent" => {
                Some("AIONE 4's header is fixed by design (T1.2: clock, icons and battery %); no switch")
            }
            "EnableController" | "GuideButtonOpensQuickMenu" => Some("always on in AIONE 4 (the controller; B-26's held Guide)"),
            "SplashDurationMs" | "SplashVideoPlayToEnd" => Some("AIONE 4's Boot Up has its own fixed timing (T1.2)"),
            _ => Some("no AIONE 4 equivalent (not built, or WPF-only)"),
        };
        if let Some(why) = why {
            left_out.push((key.clone(), shown(v), why.to_string()));
        }
    }
    taken.sort();
    left_out.sort();
    Import {
        settings: s.sanitized(),
        taken,
        left_out,
    }
}

/// The import report, `settings-import.md`.
pub fn report(i: &Import, source: &Path, at: &str) -> String {
    let mut out = format!(
        "# AIONE 4 — settings imported from WPF AIONE\n\nWritten {at} from {} (the copy T0 made; WPF's own folder was not read). The import runs once; settings you change in AIONE 4 afterwards always win.\n\n## Taken ({})\n\n| WPF setting | WPF value | AIONE 4 setting | AIONE 4 value |\n|---|---|---|---|\n",
        source.display(),
        i.taken.len()
    );
    for (k, v, to, nv) in &i.taken {
        out.push_str(&format!("| {k} | {v} | {to} | {nv} |\n"));
    }
    out.push_str(&format!("\n## Left out ({}): AIONE 4's default is used\n\n| WPF setting | WPF value | Why |\n|---|---|---|\n", i.left_out.len()));
    for (k, v, why) in &i.left_out {
        let v = if v.len() > 60 {
            format!("{}…", &v[..60])
        } else {
            v.clone()
        };
        out.push_str(&format!("| {k} | {} | {why} |\n", v.replace('|', "\\|")));
    }
    out.push_str("\nHidden games, favorites and folders live in each game's own `game.json` (D-001), which the library reads directly; they are not part of WPF's `settings.json`.\n");
    out
}

/// Runs the import once (if the copy exists and it hasn't run yet). Returns a line for the log.
pub fn import_once(data: &Path, now: &str) -> Option<String> {
    let current = load(data);
    if current.imported_from_wpf.is_some() {
        return None;
    }
    let src = wpf_source(data);
    let text = std::fs::read_to_string(&src).ok()?;
    let json: Value = match serde_json::from_str(text.trim_start_matches('\u{feff}')) {
        Ok(j) => j,
        Err(e) => {
            return Some(format!(
                "settings import: {} couldn't be read ({e}); nothing imported, defaults kept",
                src.display()
            ))
        }
    };
    let mut i = import_from(&json, current);
    i.settings.imported_from_wpf = Some(ImportRecord {
        at: now.into(),
        source: src.display().to_string(),
        taken: i.taken.len(),
        left_out: i.left_out.len(),
    });
    if let Err(e) = save(data, &i.settings) {
        return Some(format!("settings import: {e}"));
    }
    let _ = std::fs::write(data.join("settings-import.md"), report(&i, &src, now));
    Some(format!(
        "settings import: {} taken, {} left out (settings-import.md)",
        i.taken.len(),
        i.left_out.len()
    ))
}

/// B-24 rev. aw: alpha.30 imported WPF's master volume; reset it ONCE to 100 % if it still equals
/// the imported value (the user hasn't changed it). Returns a line for the log.
pub fn reset_imported_volume_once(data: &Path) -> Option<String> {
    let mut s = load(data);
    if s.volume_reset_done || s.imported_from_wpf.is_none() {
        return None;
    }
    let imported = std::fs::read_to_string(wpf_source(data))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')).ok())
        .and_then(|j| j.get("MasterVolume").and_then(fraction_to_percent));
    s.volume_reset_done = true;
    let line = match imported {
        Some(v) if s.volume == v && v != 100 => {
            s.volume = 100;
            format!("settings: the volume imported by alpha.30 ({v} %) reset once to 100 % (B-24 rev. aw: the volume isn't imported)")
        }
        Some(v) => format!("settings: the volume ({} %) differs from alpha.30's import ({v} %): kept (changed in AIONE 4)", s.volume),
        None => "settings: no imported volume to reset".to_string(),
    };
    save(data, &s).ok()?;
    // Rev. ax: the import report says so too, so it matches settings.json.
    let report = data.join("settings-import.md");
    if let Ok(text) = std::fs::read_to_string(&report) {
        if !text.contains("rev. aw") {
            let note = format!("\n## Note (B-24 rev. aw)\n\nThe volume is no longer imported: WPF's master volume belonged to its older audio model (tied to Windows' volume), so its meaning changed. {line}.\n");
            let _ = std::fs::write(&report, text + &note);
        }
    }
    Some(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Invented values in WPF's format (never the user's file).
    fn wpf() -> Value {
        json!({
            "SettingsVersion": 5, "Theme": "Switch",
            "ThemeProfiles": { "Switch": { "Palette": "Dark", "TileSize": "Small" }, "Wii": { "Palette": "Classic white" } },
            "MasterVolume": 0.35, "Mute": true, "MusicVolume": 0.5, "SfxVolume": 0.8, "MenuSounds": false,
            "SortBy": "RecentlyPlayed", "FavoritesFirst": false, "ShowHiddenGames": true,
            "Use24HourClock": true, "StartWithWindows": false, "SwapConfirmBack": true,
            "ShowSplash": false, "SplashAudio": false, "SplashDurationMs": 999,
            "ShowBatteryPercent": false, "Animations": "Reduced", "LastSelectedGameId": "An Invented Game",
            "IdleDimMinutes": 12
        })
    }

    #[test]
    fn imports_only_what_maps_and_explains_the_rest() {
        let i = import_from(&wpf(), Settings::default());
        let s = &i.settings;
        assert_eq!(
            (
                s.volume,
                s.muted,
                s.music_volume,
                s.menu_sound_volume,
                s.menu_sounds
            ),
            (100, true, 50, 80, false)
        );
        // B-24 rev. aw: the volume isn't imported (AIONE 4 starts at 100 %).
        assert!(i
            .left_out
            .iter()
            .any(|(k, _, why)| k == "MasterVolume" && why.contains("older audio model")));
        assert_eq!(
            (
                s.favorites_first,
                s.show_hidden_games,
                s.clock_24h,
                s.swap_confirm_back
            ),
            (false, true, true, true)
        );
        assert_eq!((s.boot_up, s.boot_up_audio), (false, false));
        assert_eq!(
            (s.theme.as_str(), s.color_preset.as_str()),
            ("switch", "dark")
        );
        let left: Vec<&str> = i.left_out.iter().map(|(k, _, _)| k.as_str()).collect();
        for k in [
            "ShowBatteryPercent",
            "Animations",
            "LastSelectedGameId",
            "IdleDimMinutes",
            "SplashDurationMs",
            "SettingsVersion",
        ] {
            assert!(left.contains(&k), "{k} should be left out");
        }
        assert!(i.left_out.iter().all(|(_, _, why)| !why.is_empty()));
    }

    /// B-24 rev. aw: alpha.30's imported volume, unchanged since, is reset once to 100 %.
    #[test]
    fn alpha30s_imported_volume_is_reset_once() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let src = wpf_source(data);
        std::fs::create_dir_all(src.parent().unwrap()).unwrap();
        std::fs::write(&src, wpf().to_string()).unwrap();
        let mut s = Settings {
            volume: 35,
            ..Settings::default()
        }; // as alpha.30 left it (invented)
        s.imported_from_wpf = Some(ImportRecord {
            at: "x".into(),
            source: "x".into(),
            taken: 1,
            left_out: 1,
        });
        save(data, &s).unwrap();
        std::fs::write(data.join("settings-import.md"), "# report\n").unwrap();
        assert!(reset_imported_volume_once(data)
            .unwrap()
            .contains("reset once to 100 %"));
        assert_eq!(load(data).volume, 100);
        assert!(std::fs::read_to_string(data.join("settings-import.md"))
            .unwrap()
            .contains("## Note (B-24 rev. aw)"));
        assert_eq!(reset_imported_volume_once(data), None); // once only
    }

    #[test]
    fn doubtful_values_are_left_out_never_guessed() {
        let i = import_from(
            &json!({ "MusicVolume": 7, "Mute": "yes", "SortBy": "Alphabetical", "Theme": "Wii" }),
            Settings::default(),
        );
        assert_eq!(i.settings, Settings::default()); // nothing taken
        assert_eq!(i.left_out.len(), 4);
    }

    #[test]
    fn runs_once_and_later_changes_win() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let src = wpf_source(data);
        std::fs::create_dir_all(src.parent().unwrap()).unwrap();
        std::fs::write(&src, wpf().to_string()).unwrap();
        assert!(import_once(data, "2026-01-01 00:00")
            .unwrap()
            .contains("taken"));
        assert_eq!(load(data).volume, 100); // rev. aw: not imported
        assert!(std::fs::read_to_string(data.join("settings-import.md"))
            .unwrap()
            .contains(
                "| MasterVolume | 0.35 | WPF's master volume belonged to its older audio model"
            ));
        // A change in AIONE 4 wins; the import never runs again.
        change(data, &json!({ "volume": 80 })).unwrap();
        assert_eq!(import_once(data, "2026-01-02 00:00"), None);
        assert_eq!(load(data).volume, 80);
        assert!(reset_imported_volume_once(data).unwrap().contains("kept")); // 80 differs from the imported 35
                                                                             // Invalid values fall back; unknown keys are ignored.
        let s = change(
            data,
            &json!({ "colorPreset": "purple", "volume": 250, "nonsense": 1 }),
        );
        assert!(s.is_err() || s.as_ref().unwrap().color_preset == "light");
    }
}
