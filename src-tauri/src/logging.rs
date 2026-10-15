//! Plain-text logs in `<data>\Logs\`:
//! - `aione.log`: everything (startup, input connections, the copy, UI messages).
//! - `lifecycle.log`: start, single instance, foreground, restart, exit (also copied to aione.log).
//!
//! Each line: `2026-10-12 14:03:21.123 [pid 1234] [LEVEL] [area] message`.
//! Files over 5 MB are moved to `*.1.log` at start (one generation kept).

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const MAX_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

pub struct Logger {
    dir: PathBuf,
    main: Mutex<Option<File>>,
    lifecycle: Mutex<Option<File>>,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

impl Logger {
    pub fn open(dir: &Path) -> Logger {
        let _ = fs::create_dir_all(dir);
        Logger {
            dir: dir.to_path_buf(),
            main: Mutex::new(open_log(&dir.join("aione.log"))),
            lifecycle: Mutex::new(open_log(&dir.join("lifecycle.log"))),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn write(&self, level: Level, area: &str, message: &str) {
        let line = format_line(level, area, message);
        write_line(&self.main, &line);
    }

    pub fn write_lifecycle(&self, level: Level, message: &str) {
        let line = format_line(level, "lifecycle", message);
        write_line(&self.lifecycle, &line);
        write_line(&self.main, &line);
    }
}

/// Over `MAX_BYTES` (5 MB) a log is rotated: the old file is kept as `<name>.1.log` (one old
/// file; an older `.1.log` is replaced), and the new file's first line says so (alpha.14 QA-2).
fn open_log(path: &Path) -> Option<File> {
    let mut rotated_note = None;
    if let Ok(meta) = fs::metadata(path) {
        if meta.len() > MAX_BYTES {
            let rotated = path.with_extension("1.log");
            let _ = fs::remove_file(&rotated);
            // Fails harmlessly if another instance has the file open; we keep appending.
            if fs::rename(path, &rotated).is_ok() {
                rotated_note = Some(rotation_line(&rotated, meta.len()));
            }
        }
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()?;
    if let Some(note) = rotated_note {
        use std::io::Write;
        let _ = f.write_all(format_line(Level::Info, "lifecycle", &note).as_bytes());
    }
    Some(f)
}

/// "log rotated: previous file aione.1.log (5.4 MB); one old file is kept".
pub fn rotation_line(rotated: &Path, bytes: u64) -> String {
    format!(
        "log rotated: previous file {} ({:.1} MB); one old file is kept",
        rotated
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        bytes as f64 / (1024.0 * 1024.0)
    )
}

fn write_line(file: &Mutex<Option<File>>, line: &str) {
    if let Ok(mut guard) = file.lock() {
        if let Some(f) = guard.as_mut() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }
}

pub fn format_line(level: Level, area: &str, message: &str) -> String {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    // One message is one line: embedded newlines are escaped so the log stays parseable.
    let message = message.replace('\r', "").replace('\n', " | ");
    format!(
        "{now} [pid {}] [{}] [{area}] {message}\n",
        std::process::id(),
        level.as_str()
    )
}

/// Installs the global logger (first call wins).
pub fn init(dir: &Path) -> &'static Logger {
    LOGGER.get_or_init(|| Logger::open(dir))
}

pub fn get() -> Option<&'static Logger> {
    LOGGER.get()
}

/// Tests: every line written through the functions below is also kept here, so a test can
/// prove that a promised log line is really written (alpha.2 QA-3).
#[cfg(test)]
static CAPTURE: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[cfg(test)]
pub fn captured() -> Vec<String> {
    CAPTURE.lock().map(|c| c.clone()).unwrap_or_default()
}

fn capture(_level: Level, _area: &str, _message: &str) {
    #[cfg(test)]
    if let Ok(mut c) = CAPTURE.lock() {
        c.push(format!("[{}] [{_area}] {_message}", _level.as_str()));
    }
}

pub fn info(area: &str, message: &str) {
    capture(Level::Info, area, message);
    if let Some(l) = get() {
        l.write(Level::Info, area, message);
    }
}

pub fn warn(area: &str, message: &str) {
    capture(Level::Warn, area, message);
    if let Some(l) = get() {
        l.write(Level::Warn, area, message);
    }
}

pub fn error(area: &str, message: &str) {
    capture(Level::Error, area, message);
    if let Some(l) = get() {
        l.write(Level::Error, area, message);
    }
}

pub fn lifecycle(level: Level, message: &str) {
    capture(level, "lifecycle", message);
    if let Some(l) = get() {
        l.write_lifecycle(level, message);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_rotation_is_logged_in_the_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("aione.log");
        std::fs::write(&p, vec![b'x'; (super::MAX_BYTES + 10) as usize]).unwrap();
        drop(super::open_log(&p));
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(
            text.contains("log rotated: previous file aione.1.log (5.0 MB); one old file is kept"),
            "{text}"
        );
        assert!(dir.path().join("aione.1.log").exists());
    }

    use super::*;

    #[test]
    fn lines_have_level_area_and_no_newlines() {
        let line = format_line(Level::Warn, "copy", "a\nb");
        assert!(line.contains("[WARN] [copy] a | b"));
        assert_eq!(line.matches('\n').count(), 1);
    }

    #[test]
    fn lifecycle_lines_go_to_both_files() {
        let dir = tempfile::tempdir().unwrap();
        let logger = Logger::open(dir.path());
        logger.write_lifecycle(Level::Info, "start");
        logger.write(Level::Info, "x", "only main");
        let main = fs::read_to_string(dir.path().join("aione.log")).unwrap();
        let life = fs::read_to_string(dir.path().join("lifecycle.log")).unwrap();
        assert!(main.contains("start") && main.contains("only main"));
        assert!(life.contains("start") && !life.contains("only main"));
    }
}
