//! Where AIONE 4 keeps its data (Master Prompt A9).
//!
//! - AIONE 4 data folder: `%LOCALAPPDATA%\AIONE4\` (logs in `Logs\`).
//! - The WPF AIONE data folder: `%LOCALAPPDATA%\AIONE\`. **Read only, never written.**
//!
//! `AIONE4_DATA_DIR` and `AIONE4_WPF_DIR` override both (development and tests only).

use std::path::{Path, PathBuf};

pub const DATA_DIR_NAME: &str = "AIONE4";
pub const WPF_DIR_NAME: &str = "AIONE";
pub const LOGS_DIR_NAME: &str = "Logs";

fn local_app_data() -> PathBuf {
    if let Some(p) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(p);
    }
    // Non-Windows development fallback (the app itself targets Windows only).
    if let Some(p) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    home.join(".local").join("share")
}

pub fn data_dir() -> PathBuf {
    std::env::var_os("AIONE4_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| local_app_data().join(DATA_DIR_NAME))
}

pub fn wpf_data_dir() -> PathBuf {
    std::env::var_os("AIONE4_WPF_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| local_app_data().join(WPF_DIR_NAME))
}

pub fn logs_dir(data: &Path) -> PathBuf {
    data.join(LOGS_DIR_NAME)
}

/// Creates the data folder and `Logs\`. Returns the logs folder.
pub fn ensure_data_dir(data: &Path) -> std::io::Result<PathBuf> {
    let logs = logs_dir(data);
    std::fs::create_dir_all(&logs)?;
    Ok(logs)
}

/// True when `inner` is `outer` or inside it (both made absolute without touching the disk).
pub fn is_within(inner: &Path, outer: &Path) -> bool {
    let norm = |p: &Path| -> Vec<String> {
        p.components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => Some(s.to_string_lossy().to_lowercase()),
                std::path::Component::Prefix(p) => {
                    Some(p.as_os_str().to_string_lossy().to_lowercase())
                }
                _ => None,
            })
            .collect()
    };
    let (i, o) = (norm(inner), norm(outer));
    i.len() >= o.len() && i[..o.len()] == o[..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_logs_folder() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("AIONE4");
        let logs = ensure_data_dir(&data).unwrap();
        assert!(logs.is_dir());
        assert_eq!(logs, data.join("Logs"));
    }

    #[test]
    fn data_and_wpf_folders_differ() {
        // The default names must never collide: AIONE 4 never writes to the WPF folder.
        assert_ne!(DATA_DIR_NAME, WPF_DIR_NAME);
        let base = Path::new("/x/Local");
        assert!(!is_within(
            &base.join(DATA_DIR_NAME),
            &base.join(WPF_DIR_NAME)
        ));
    }

    #[test]
    fn within_is_case_insensitive_and_component_based() {
        assert!(is_within(Path::new("/a/B/c"), Path::new("/a/b")));
        assert!(!is_within(Path::new("/a/bc"), Path::new("/a/b")));
        assert!(!is_within(Path::new("/a"), Path::new("/a/b")));
    }
}
