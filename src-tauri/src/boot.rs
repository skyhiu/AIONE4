//! The custom Boot Up / Boot Down media (reference A9; T1.2).
//!
//! WPF kept them in `Assets\Splash\` (Boot Up) and `Assets\Suspend\suspend.mp4` (Boot Down). Like
//! the games, AIONE 4 **copies them once** from the WPF copy (`Imported\AIONE-WPF\Assets\…`) into
//! its own folder, `<data>\Boot\boot-up-<key>.<ext>` and `boot-down-<key>.<ext>`, logs it, and
//! plays only its own copy; the key comes from the source's size and date, so a changed source is
//! copied again (and the old copy removed). `Imported\` is only read.
//!
//! From the user's WPF settings (`Imported\AIONE-WPF\Config\settings.json`) only the boot entries
//! are read here: `ShowSplash`, `SplashAudio`, `SplashVideoPlayToEnd`, `SplashDurationMs`. The full
//! settings conversion is T2.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};

pub const BOOT_DIR: &str = "Boot";
const VIDEO_EXT: [&str; 4] = ["mp4", "webm", "m4v", "mov"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    Up,
    Down,
}

impl Which {
    pub fn name(self) -> &'static str {
        match self {
            Which::Up => "up",
            Which::Down => "down",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BootFile {
    /// Cache key (also in the media URL).
    pub key: String,
    pub ext: String,
    pub bytes: u64,
    /// The file name in the WPF copy it came from.
    pub source: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BootSettings {
    pub show_splash: bool,
    pub splash_audio: bool,
    pub play_to_end: bool,
    pub duration_ms: u32,
    /// Where they came from: "WPF settings" or "defaults".
    pub from: String,
}

impl Default for BootSettings {
    fn default() -> Self {
        BootSettings {
            show_splash: true,
            splash_audio: true,
            play_to_end: true,
            duration_ms: 3200,
            from: "defaults".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BootMedia {
    pub up: Option<BootFile>,
    pub down: Option<BootFile>,
    pub settings: BootSettings,
    /// Plain log lines (what was found, copied, reused or failed).
    pub notes: Vec<String>,
}

fn wpf_assets(data: &Path) -> PathBuf {
    crate::migration::destination(data).join("Assets")
}

fn is_video(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| VIDEO_EXT.contains(&e.to_ascii_lowercase().as_str()))
}

/// The WPF source for Boot Up (`Assets\Splash\`, `splash.*` first) or Boot Down
/// (`Assets\Suspend\suspend.mp4` first).
pub fn source(data: &Path, which: Which) -> Option<PathBuf> {
    let (dir, prefer) = match which {
        Which::Up => (wpf_assets(data).join("Splash"), "splash"),
        Which::Down => (wpf_assets(data).join("Suspend"), "suspend"),
    };
    let mut videos: Vec<PathBuf> = fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_video(p))
        .collect();
    videos.sort();
    let preferred = videos.iter().find(|p| {
        p.file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case(prefer))
    });
    preferred.cloned().or_else(|| videos.into_iter().next())
}

fn key_of(src: &Path) -> Option<String> {
    let m = fs::metadata(src).ok()?;
    let t = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in m.len().to_le_bytes().iter().chain(t.to_le_bytes().iter()) {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Some(format!("{h:016x}"))
}

pub fn copy_path(data: &Path, which: Which, key: &str, ext: &str) -> PathBuf {
    data.join(BOOT_DIR)
        .join(format!("boot-{}-{key}.{ext}", which.name()))
}

/// The current copy for `which` (whatever key), if any.
pub fn current_copy(data: &Path, which: Which) -> Option<PathBuf> {
    let prefix = format!("boot-{}-", which.name());
    fs::read_dir(data.join(BOOT_DIR))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&prefix) && !n.ends_with(".tmp"))
        })
}

/// Copies the source once into `<data>\Boot\` (or reuses the copy). Returns the file and a note.
pub fn ensure(data: &Path, which: Which) -> (Option<BootFile>, String) {
    let label = if which == Which::Up {
        "Boot Up"
    } else {
        "Boot Down"
    };
    let Some(src) = source(data, which) else {
        // No WPF source: an earlier copy may still exist (the WPF copy is never required).
        if let Some(p) = current_copy(data, which) {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let (key, ext) = name
                .trim_start_matches(&format!("boot-{}-", which.name()))
                .split_once('.')
                .map(|(k, e)| (k.to_string(), e.to_string()))
                .unwrap_or_default();
            let bytes = fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            return (
                Some(BootFile {
                    key,
                    ext,
                    bytes,
                    source: String::new(),
                }),
                format!("custom {label}: no WPF source; using AIONE 4's copy {name}"),
            );
        }
        return (
            None,
            format!("custom {label}: none (no video in the WPF copy's Assets folder)"),
        );
    };
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4")
        .to_ascii_lowercase();
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let Some(key) = key_of(&src) else {
        return (None, format!("custom {label}: cannot read {name}"));
    };
    let out = copy_path(data, which, &key, &ext);
    let bytes = fs::metadata(&src).map(|m| m.len()).unwrap_or(0);
    let file = BootFile {
        key: key.clone(),
        ext: ext.clone(),
        bytes,
        source: name.clone(),
    };
    if out.is_file() {
        return (
            Some(file),
            format!("custom {label}: {name} ({bytes} bytes), AIONE 4's copy reused"),
        );
    }
    let started = Instant::now();
    let old = current_copy(data, which);
    let res = (|| -> std::io::Result<()> {
        fs::create_dir_all(data.join(BOOT_DIR))?;
        let tmp = out.with_extension(format!("{ext}.tmp"));
        fs::copy(&src, &tmp)?;
        fs::rename(&tmp, &out)
    })();
    match res {
        Ok(()) => {
            if let Some(o) = &old {
                let _ = fs::remove_file(o);
            }
            (
                Some(file),
                format!(
                    "custom {label}: {name} ({bytes} bytes) copied into Boot\\ in {} ms{}",
                    started.elapsed().as_millis(),
                    if old.is_some() {
                        "; remade: the source changed since the last copy"
                    } else {
                        ""
                    }
                ),
            )
        }
        Err(e) => {
            // Keep using an older copy if there is one.
            match old {
                Some(_) => (None, format!("custom {label}: copying {name} failed ({e}); the older copy is not used (it may be outdated)")),
                None => (None, format!("custom {label}: copying {name} failed ({e})")),
            }
        }
    }
}

/// Only the four boot entries of the WPF settings (lenient; defaults when missing).
pub fn read_settings(data: &Path) -> BootSettings {
    let p = crate::migration::destination(data)
        .join("Config")
        .join("settings.json");
    let Ok(bytes) = fs::read(&p) else {
        return BootSettings::default();
    };
    let text = String::from_utf8_lossy(bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes))
        .into_owned();
    let Ok(serde_json::Value::Object(o)) = serde_json::from_str::<serde_json::Value>(&text) else {
        return BootSettings::default();
    };
    let flag = |k: &str, d: bool| match o.get(k) {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::String(s)) => s.eq_ignore_ascii_case("true"),
        _ => d,
    };
    let num = |k: &str, d: u32| match o.get(k) {
        Some(serde_json::Value::Number(n)) => n.as_u64().map(|v| v as u32).unwrap_or(d),
        Some(serde_json::Value::String(s)) => s.trim().parse().unwrap_or(d),
        _ => d,
    };
    BootSettings {
        show_splash: flag("ShowSplash", true),
        splash_audio: flag("SplashAudio", true),
        play_to_end: flag("SplashVideoPlayToEnd", true),
        duration_ms: num("SplashDurationMs", 3200),
        from: "WPF settings".into(),
    }
}

pub fn prepare(data: &Path) -> BootMedia {
    let (up, n1) = ensure(data, Which::Up);
    let (down, n2) = ensure(data, Which::Down);
    BootMedia {
        up,
        down,
        settings: read_settings(data),
        notes: vec![n1, n2],
    }
}

/// The file behind `/_boot/up` or `/_boot/down`.
pub fn resolve(data: &Path, which: &str) -> Option<PathBuf> {
    let w = match which {
        "up" => Which::Up,
        "down" => Which::Down,
        _ => return None,
    };
    current_copy(data, w)
}

pub fn mime_of(p: &Path) -> &'static str {
    match p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",
        _ => "video/mp4",
    }
}

/// Parses `bytes=a-b` / `bytes=a-` against a file of `len` bytes. At most `max` bytes per answer.
pub fn parse_range(header: &str, len: u64, max: u64) -> Option<(u64, u64)> {
    let spec = header.trim().strip_prefix("bytes=")?;
    let (a, b) = spec.split_once('-')?;
    let (start, end) = if a.is_empty() {
        let n: u64 = b.parse().ok()?;
        (len.saturating_sub(n), len.checked_sub(1)?)
    } else {
        let s: u64 = a.parse().ok()?;
        let e: u64 = if b.is_empty() {
            len.checked_sub(1)?
        } else {
            b.parse::<u64>().ok()?.min(len.checked_sub(1)?)
        };
        (s, e)
    };
    if start > end || start >= len {
        return None;
    }
    Some((start, end.min(start + max - 1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wpf(data: &Path) -> PathBuf {
        crate::migration::destination(data).join("Assets")
    }

    #[test]
    fn copies_once_reuses_and_remakes_on_change() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        fs::create_dir_all(wpf(data).join("Splash")).unwrap();
        fs::create_dir_all(wpf(data).join("Suspend")).unwrap();
        fs::write(wpf(data).join("Splash").join("splash.mp4"), b"video one").unwrap();
        fs::write(wpf(data).join("Suspend").join("suspend.mp4"), b"down").unwrap();
        let m = prepare(data);
        assert!(
            m.notes[0].contains("splash.mp4 (9 bytes) copied into Boot\\"),
            "{:?}",
            m.notes
        );
        assert!(m.notes[1].contains("suspend.mp4 (4 bytes) copied"));
        let up = m.up.unwrap();
        assert_eq!(
            fs::read(copy_path(data, Which::Up, &up.key, "mp4")).unwrap(),
            b"video one"
        );
        let again = prepare(data);
        assert!(again.notes[0].ends_with("AIONE 4's copy reused"));
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(
            wpf(data).join("Splash").join("splash.mp4"),
            b"video two, longer",
        )
        .unwrap();
        let changed = prepare(data);
        assert!(
            changed.notes[0].contains("remade: the source changed"),
            "{:?}",
            changed.notes
        );
        let files: Vec<_> = fs::read_dir(data.join(BOOT_DIR))
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(files.len(), 2, "one Boot Up and one Boot Down copy");
        assert_eq!(
            fs::read(resolve(data, "up").unwrap()).unwrap(),
            b"video two, longer"
        );
    }

    #[test]
    fn without_wpf_media_there_is_no_custom_boot() {
        let dir = tempfile::tempdir().unwrap();
        let m = prepare(dir.path());
        assert!(m.up.is_none() && m.down.is_none());
        assert!(m.notes[0].starts_with("custom Boot Up: none"));
        assert_eq!(m.settings, BootSettings::default());
    }

    /// Fixture with invented values (the user's own settings are never committed).
    #[test]
    fn reads_only_the_four_boot_entries() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let cfg = crate::migration::destination(data).join("Config");
        fs::create_dir_all(&cfg).unwrap();
        fs::write(
            cfg.join("settings.json"),
            r#"{"ShowSplash":"True","SplashAudio":"False","SplashVideoPlayToEnd":"False","SplashDurationMs":"2500","LauncherName":"Invented"}"#,
        )
        .unwrap();
        let s = read_settings(data);
        assert!(s.show_splash && !s.splash_audio && !s.play_to_end);
        assert_eq!((s.duration_ms, s.from.as_str()), (2500, "WPF settings"));
    }

    #[test]
    fn video_ranges() {
        assert_eq!(parse_range("bytes=0-", 1000, 4096), Some((0, 999)));
        assert_eq!(parse_range("bytes=100-199", 1000, 4096), Some((100, 199)));
        assert_eq!(parse_range("bytes=0-", 10_000, 4096), Some((0, 4095)));
        assert_eq!(parse_range("bytes=-100", 1000, 4096), Some((900, 999)));
        assert_eq!(parse_range("bytes=2000-", 1000, 4096), None);
        assert_eq!(parse_range("items=0-1", 1000, 4096), None);
    }
}
