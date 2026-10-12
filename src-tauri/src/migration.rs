//! The one-time copy of the WPF AIONE data folder (Master Prompt A9, L-17).
//!
//! - The source (`%LOCALAPPDATA%\AIONE\`) is **only read**: files are opened read-only by
//!   `std::fs::copy`; nothing is created, renamed or deleted there. A fingerprint (every
//!   path, size and modified time) is taken before and after; the result says whether the
//!   source stayed unchanged.
//! - The copy goes to `<data>\Imported\AIONE-WPF.partial\` and is renamed to
//!   `<data>\Imported\AIONE-WPF\` when complete. Converting it to AIONE 4's own format is
//!   T1 / T2; T0 copies it and keeps it safe.
//! - The question is asked once: the decision is stored in `<data>\migration.json`. A copy
//!   that failed completely is offered again at the next start.
//! - Counts: games, hidden games, favorites and library folders come from
//!   `library::scan_library`, the same rule the loader uses (L-17); every entry that is not a
//!   game is logged with its reason, and where each count was read from.

use crate::library::{self, LibraryCounts};
use crate::logging;
use crate::paths;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

pub const STATE_FILE: &str = "migration.json";
pub const IMPORT_DIR: &str = "Imported";
pub const COPY_NAME: &str = "AIONE-WPF";
const STATE_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum MigrationStatus {
    #[serde(rename_all = "camelCase")]
    NoSource { source: String },
    /// `notice`: a plain message for the user (e.g. the record could not be read), or null.
    #[serde(rename_all = "camelCase")]
    AlreadyHandled {
        decision: String,
        notice: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Offer {
        source: String,
        files: u64,
        bytes: u64,
        library: LibraryCounts,
        notice: Option<String>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CopyProgress {
    pub files_done: u64,
    pub files_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

/// B-6: every field has a default, so records written by earlier versions (with fewer
/// fields) always read. Never add a required field here; migrate by `schema` instead.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CopyResult {
    pub ok: bool,
    /// "copied" | "partial" | "failed"
    pub outcome: String,
    pub destination: String,
    pub files_copied: u64,
    pub files_failed: u64,
    pub bytes_copied: u64,
    pub library: LibraryCounts,
    pub source_unchanged: bool,
    pub notes: Vec<String>,
}

/// The copy record (`migration.json`). B-6: only `decision` is required; everything else has
/// a default so every earlier version's record reads (fixtures in `tests/fixtures/migration/`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationState {
    #[serde(default = "schema_one")]
    pub schema: u32,
    /// "copied" | "partial" | "failed" | "declined"
    pub decision: String,
    #[serde(default)]
    pub at: String,
    #[serde(default)]
    pub app_version: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub result: Option<CopyResult>,
}

fn schema_one() -> u32 {
    1
}

// ---------------------------------------------------------------------------------------
// Fingerprint (read only)

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub rel: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified_ns: u128,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fingerprint {
    pub entries: Vec<Entry>,
    /// Links / junctions are not followed (and not copied); listed for the log.
    pub skipped_links: Vec<PathBuf>,
    pub unreadable: Vec<(PathBuf, String)>,
}

impl Fingerprint {
    pub fn files(&self) -> u64 {
        self.entries.iter().filter(|e| !e.is_dir).count() as u64
    }
    pub fn bytes(&self) -> u64 {
        self.entries
            .iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.size)
            .sum()
    }
}

pub fn fingerprint(root: &Path) -> Fingerprint {
    let mut fp = Fingerprint::default();
    walk(root, Path::new(""), &mut fp);
    fp.entries.sort_by(|a, b| a.rel.cmp(&b.rel));
    fp
}

fn walk(root: &Path, rel: &Path, fp: &mut Fingerprint) {
    let dir = root.join(rel);
    let read = match fs::read_dir(&dir) {
        Ok(r) => r,
        Err(e) => {
            fp.unreadable.push((rel.to_path_buf(), e.to_string()));
            return;
        }
    };
    for entry in read.flatten() {
        let child_rel = rel.join(entry.file_name());
        let meta = match fs::symlink_metadata(entry.path()) {
            Ok(m) => m,
            Err(e) => {
                fp.unreadable.push((child_rel, e.to_string()));
                continue;
            }
        };
        if meta.file_type().is_symlink() {
            fp.skipped_links.push(child_rel);
            continue;
        }
        let modified_ns = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        if meta.is_dir() {
            fp.entries.push(Entry {
                rel: child_rel.clone(),
                is_dir: true,
                size: 0,
                modified_ns,
            });
            walk(root, &child_rel, fp);
        } else {
            fp.entries.push(Entry {
                rel: child_rel,
                is_dir: false,
                size: meta.len(),
                modified_ns,
            });
        }
    }
}

/// Differences between two fingerprints of the same folder, for the log (at most `limit`).
pub fn diff(before: &Fingerprint, after: &Fingerprint, limit: usize) -> Vec<String> {
    use std::collections::BTreeMap;
    let map = |fp: &Fingerprint| -> BTreeMap<PathBuf, (bool, u64, u128)> {
        fp.entries
            .iter()
            .map(|e| {
                (
                    e.rel.clone(),
                    (e.is_dir, e.size, if e.is_dir { 0 } else { e.modified_ns }),
                )
            })
            .collect()
    };
    let (a, b) = (map(before), map(after));
    let mut out = Vec::new();
    for (k, v) in &a {
        match b.get(k) {
            None => out.push(format!("removed: {}", k.display())),
            Some(w) if w != v => out.push(format!("changed: {}", k.display())),
            _ => {}
        }
    }
    for k in b.keys() {
        if !a.contains_key(k) {
            out.push(format!("added: {}", k.display()));
        }
    }
    out.truncate(limit);
    out
}

// ---------------------------------------------------------------------------------------
// State

pub fn state_path(data: &Path) -> PathBuf {
    data.join(STATE_FILE)
}

/// What reading the copy record found.
#[derive(Debug, Clone, PartialEq)]
pub enum RecordRead {
    Missing,
    Ok(Box<MigrationState>),
    /// The file exists but cannot be read; `backup` is where a copy of it was saved.
    Unreadable {
        error: String,
        backup: Option<PathBuf>,
    },
}

/// Reads `migration.json` (B-6). Never treats an unreadable record as "not handled": it is
/// backed up next to itself (once per distinct content) and reported.
pub fn read_record(data: &Path) -> RecordRead {
    let path = state_path(data);
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return RecordRead::Missing,
        Err(e) => {
            return RecordRead::Unreadable {
                error: e.to_string(),
                backup: None,
            }
        }
    };
    let bytes_no_bom = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    match serde_json::from_slice::<MigrationState>(bytes_no_bom) {
        Ok(s) => {
            if s.schema > STATE_SCHEMA {
                // A newer AIONE 4 wrote this: it is handled; never overwrite it (A9).
                logging::warn(
                    "copy",
                    &format!(
                        "migration.json was written by a newer AIONE 4 (schema {}, this build knows {STATE_SCHEMA}); reading only its decision",
                        s.schema
                    ),
                );
            }
            RecordRead::Ok(Box::new(s))
        }
        Err(e) => RecordRead::Unreadable {
            error: e.to_string(),
            backup: back_up_once(&path, &bytes),
        },
    }
}

/// Saves `migration.json.unreadable-<time>` next to the record, unless an identical backup
/// already exists (so a record that stays unreadable is not backed up at every start).
fn back_up_once(path: &Path, bytes: &[u8]) -> Option<PathBuf> {
    let dir = path.parent()?;
    let prefix = format!("{STATE_FILE}.unreadable-");
    if let Ok(read) = fs::read_dir(dir) {
        for e in read.flatten() {
            if e.file_name().to_string_lossy().starts_with(&prefix)
                && fs::read(e.path()).ok().as_deref() == Some(bytes)
            {
                return Some(e.path());
            }
        }
    }
    let backup = dir.join(format!(
        "{prefix}{}",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    ));
    fs::write(&backup, bytes).ok().map(|_| backup)
}

/// The decision, if the record says one (compatibility helper for callers and tests).
pub fn read_state(data: &Path) -> Option<MigrationState> {
    match read_record(data) {
        RecordRead::Ok(s) => Some(*s),
        _ => None,
    }
}

/// A finished copy exists: `Imported\AIONE-WPF\` is a non-empty folder. The copy is staged in
/// `AIONE-WPF.partial` and renamed only after every file was copied, so the final folder only
/// exists when a copy ran to its end.
pub fn finished_copy_exists(data: &Path) -> bool {
    let dest = destination(data);
    dest.is_dir()
        && fs::read_dir(&dest)
            .map(|mut r| r.next().is_some())
            .unwrap_or(false)
}

fn write_state(data: &Path, state: &MigrationState) -> std::io::Result<()> {
    let tmp = data.join(format!("{STATE_FILE}.tmp"));
    fs::write(&tmp, serde_json::to_vec_pretty(state).unwrap_or_default())?;
    fs::rename(&tmp, state_path(data))
}

fn now_string() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn record_decision(
    data: &Path,
    source: &Path,
    decision: &str,
    result: Option<CopyResult>,
) -> std::io::Result<()> {
    write_state(
        data,
        &MigrationState {
            schema: STATE_SCHEMA,
            decision: decision.to_string(),
            at: now_string(),
            app_version: crate::version::VERSION.to_string(),
            source: source.display().to_string(),
            result,
        },
    )
}

// ---------------------------------------------------------------------------------------
// Status

pub fn status(data: &Path, source: &Path) -> MigrationStatus {
    let finished = finished_copy_exists(data);
    let mut notice: Option<String> = None;
    match read_record(data) {
        RecordRead::Ok(s) if s.decision != "failed" => {
            logging::info(
                "copy",
                &format!(
                    "copy record read: decision \"{}\" (written by AIONE 4 {} at {}); not asking again",
                    s.decision,
                    if s.app_version.is_empty() { "?" } else { &s.app_version },
                    if s.at.is_empty() { "?" } else { &s.at }
                ),
            );
            return MigrationStatus::AlreadyHandled {
                decision: s.decision,
                notice: None,
            };
        }
        RecordRead::Ok(_) => {
            if finished {
                logging::info("copy", "copy record says the last copy failed, but a finished copy exists; not asking again");
                rewrite_record(
                    data,
                    source,
                    "copied (found a finished copy after a failed record)",
                );
                return MigrationStatus::AlreadyHandled {
                    decision: "copied".into(),
                    notice: None,
                };
            }
            logging::info(
                "copy",
                "the previous copy failed completely; offering it again",
            );
        }
        RecordRead::Missing => {
            if finished {
                logging::info(
                    "copy",
                    &format!(
                        "no copy record, but a finished copy exists in {}; not asking again (record written)",
                        destination(data).display()
                    ),
                );
                rewrite_record(
                    data,
                    source,
                    "copied (record was missing; a finished copy exists)",
                );
                return MigrationStatus::AlreadyHandled {
                    decision: "copied".into(),
                    notice: None,
                };
            }
        }
        RecordRead::Unreadable { error, backup } => {
            let saved = backup
                .as_ref()
                .map(|b| format!("a copy of it was saved as {}", b.display()))
                .unwrap_or_else(|| "it could not be backed up".into());
            logging::warn(
                "copy",
                &format!("the copy record migration.json could not be read ({error}); {saved}"),
            );
            if finished {
                rewrite_record(
                    data,
                    source,
                    "copied (record was unreadable; a finished copy exists)",
                );
                return MigrationStatus::AlreadyHandled {
                    decision: "copied".into(),
                    notice: Some(format!(
                        "AIONE 4 could not read its note of your earlier copy answer. Your copied data is complete, so nothing changes and you are not asked again. The note was kept as {} (details in aione.log).",
                        backup.map(|b| b.display().to_string()).unwrap_or_else(|| "(not saved)".into())
                    )),
                };
            }
            notice = Some(format!(
                "AIONE 4 could not read its note of your earlier answer to this question, and no finished copy exists, so it asks again. The note was kept as {} (details in aione.log).",
                backup.map(|b| b.display().to_string()).unwrap_or_else(|| "(not saved)".into())
            ));
        }
    }
    let has_content = fs::read_dir(source)
        .map(|mut r| r.next().is_some())
        .unwrap_or(false);
    if !has_content {
        logging::info(
            "copy",
            &format!(
                "no WPF AIONE data folder with content at {}; nothing to offer",
                source.display()
            ),
        );
        return MigrationStatus::NoSource {
            source: source.display().to_string(),
        };
    }
    let fp = fingerprint(source);
    let lib = library::scan_library(source);
    logging::info(
        "copy",
        &format!(
            "offering the copy of {}: {} files, {} bytes; {}",
            source.display(),
            fp.files(),
            fp.bytes(),
            counts_line(&lib)
        ),
    );
    log_library(&lib);
    log_top_level(source);
    MigrationStatus::Offer {
        source: source.display().to_string(),
        files: fp.files(),
        bytes: fp.bytes(),
        library: lib,
        notice,
    }
}

/// Writes a fresh record for a finished copy found without a readable record (B-6).
fn rewrite_record(data: &Path, source: &Path, why: &str) {
    match record_decision(data, source, "copied", None) {
        Ok(()) => logging::info("copy", &format!("copy record written: {why}")),
        Err(e) => logging::error("copy", &format!("could not write migration.json: {e}")),
    }
}

/// The L-17 evidence lines: every skipped entry, and where each count was read from. Always
/// written, also when nothing was found (D-001).
pub fn log_library(lib: &LibraryCounts) {
    for i in &lib.ignored {
        logging::info(
            "copy",
            &format!("skipped in Games: {} ({})", i.name, i.reason),
        );
    }
    for line in &lib.sources {
        logging::info("copy", &format!("library {line}"));
    }
}

// ---------------------------------------------------------------------------------------
// Copy

pub fn destination(data: &Path) -> PathBuf {
    data.join(IMPORT_DIR).join(COPY_NAME)
}

/// Copies `source` into `<data>\Imported\AIONE-WPF\`. `progress` is called at most about
/// every 100 ms and once at the end. Never writes inside `source`.
pub fn copy(data: &Path, source: &Path, mut progress: impl FnMut(&CopyProgress)) -> CopyResult {
    let final_dir = destination(data);
    let partial = final_dir.with_file_name(format!("{COPY_NAME}.partial"));
    let mut notes = Vec::new();
    let fail = |msg: String, notes: Vec<String>| {
        logging::error("copy", &msg);
        let mut notes = notes;
        notes.push(msg);
        CopyResult {
            ok: false,
            outcome: "failed".into(),
            destination: final_dir.display().to_string(),
            source_unchanged: true,
            notes,
            ..Default::default()
        }
    };

    // Safety: the destination must not be inside the source or the other way round.
    if paths::is_within(&final_dir, source) || paths::is_within(source, &final_dir) {
        return fail(
            format!(
                "refused: the destination {} and the source {} overlap",
                final_dir.display(),
                source.display()
            ),
            notes,
        );
    }

    let started = Instant::now();
    let before = fingerprint(source);
    let files_total = before.files();
    let bytes_total = before.bytes();
    logging::info(
        "copy",
        &format!(
            "copy started: {} -> {} ({} files, {} folders, {} bytes)",
            source.display(),
            final_dir.display(),
            files_total,
            before.entries.len() as u64 - files_total,
            bytes_total
        ),
    );
    for link in &before.skipped_links {
        let n = format!("not copied (link or junction): {}", link.display());
        logging::warn("copy", &n);
        notes.push(n);
    }
    for (p, e) in &before.unreadable {
        logging::warn("copy", &format!("could not read {}: {e}", p.display()));
    }

    // Fresh staging folder (ours; a leftover from an interrupted copy is removed).
    if partial.exists() {
        if let Err(e) = fs::remove_dir_all(&partial) {
            return fail(format!("could not clear {}: {e}", partial.display()), notes);
        }
    }
    if let Err(e) = fs::create_dir_all(&partial) {
        return fail(
            format!("could not create {}: {e}", partial.display()),
            notes,
        );
    }

    let mut p = CopyProgress {
        files_done: 0,
        files_total,
        bytes_done: 0,
        bytes_total,
    };
    progress(&p);
    let mut last_emit = Instant::now();
    let mut failed: Vec<(PathBuf, String)> = Vec::new();

    for e in &before.entries {
        let dst = partial.join(&e.rel);
        if e.is_dir {
            if let Err(err) = fs::create_dir_all(&dst) {
                failed.push((e.rel.clone(), err.to_string()));
            }
            continue;
        }
        if let Some(parent) = dst.parent() {
            let _ = fs::create_dir_all(parent);
        }
        // std::fs::copy opens the source read-only (CopyFileExW on Windows).
        match fs::copy(source.join(&e.rel), &dst) {
            Ok(n) if n == e.size => {}
            Ok(n) => failed.push((
                e.rel.clone(),
                format!("size changed while copying ({} -> {n} bytes)", e.size),
            )),
            Err(err) => failed.push((e.rel.clone(), err.to_string())),
        }
        p.files_done += 1;
        p.bytes_done += e.size;
        if last_emit.elapsed() >= Duration::from_millis(100) {
            progress(&p);
            last_emit = Instant::now();
        }
    }
    progress(&p);

    // Verify: every file present with the source's size.
    for e in before.entries.iter().filter(|e| !e.is_dir) {
        if failed.iter().any(|(r, _)| r == &e.rel) {
            continue;
        }
        match fs::metadata(partial.join(&e.rel)) {
            Ok(m) if m.len() == e.size => {}
            Ok(m) => failed.push((
                e.rel.clone(),
                format!("verify: size {} != {}", m.len(), e.size),
            )),
            Err(err) => failed.push((e.rel.clone(), format!("verify: {err}"))),
        }
    }
    for (rel, why) in &failed {
        logging::warn("copy", &format!("not copied: {} ({why})", rel.display()));
    }

    // Move the finished copy into place (an older copy, if any, is kept aside, never deleted).
    if final_dir.exists() {
        let aside = final_dir.with_file_name(format!(
            "{COPY_NAME}.old-{}",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        ));
        logging::warn(
            "copy",
            &format!(
                "{} already exists; moving it to {}",
                final_dir.display(),
                aside.display()
            ),
        );
        if let Err(e) = fs::rename(&final_dir, &aside) {
            return fail(format!("could not move the older copy aside: {e}"), notes);
        }
    }
    if let Err(e) = fs::rename(&partial, &final_dir) {
        return fail(format!("could not finish the copy (rename): {e}"), notes);
    }

    // The source must be exactly as before.
    let after = fingerprint(source);
    let changes = diff(&before, &after, 20);
    let source_unchanged = changes.is_empty();
    if source_unchanged {
        logging::info(
            "copy",
            "source check: the WPF data folder is unchanged (same files, sizes, times)",
        );
    } else {
        logging::warn(
            "copy",
            &format!(
                "source check: the WPF data folder changed during the copy (another program?): {}",
                changes.join("; ")
            ),
        );
    }

    // Counts from the copy, with the loader's rule (L-17), and from the source for comparison.
    let lib = library::scan_library(&final_dir);
    let lib_src = library::scan_library(source);
    if lib != lib_src {
        let n = format!(
            "library counts differ between source ({} games) and copy ({} games)",
            lib_src.games, lib.games
        );
        logging::warn("copy", &n);
        notes.push(n);
    }
    log_library(&lib);
    if let Some(n) = external_games_folder_note(source) {
        logging::warn("copy", &n);
        notes.push(n);
    }

    let files_failed = failed.len() as u64;
    let files_copied = files_total.saturating_sub(
        failed
            .iter()
            .filter(|(r, _)| !is_dir_in(&before, r))
            .count() as u64,
    );
    let bytes_copied: u64 = before
        .entries
        .iter()
        .filter(|e| !e.is_dir && !failed.iter().any(|(r, _)| r == &e.rel))
        .map(|e| e.size)
        .sum();
    let outcome = if files_failed == 0 {
        "copied"
    } else {
        "partial"
    };
    logging::info(
        "copy",
        &format!(
            "copy {outcome} in {:.1} s: {files_copied} of {files_total} files, {bytes_copied} bytes, {files_failed} failed; {}",
            started.elapsed().as_secs_f64(),
            counts_line(&lib)
        ),
    );

    CopyResult {
        ok: files_failed == 0,
        outcome: outcome.into(),
        destination: final_dir.display().to_string(),
        files_copied,
        files_failed,
        bytes_copied,
        library: lib,
        source_unchanged,
        notes,
    }
}

/// "10 games, 0 hidden, 0 favorites, 0 folders; 1 skipped" — one wording for log and dialog.
pub fn counts_line(c: &LibraryCounts) -> String {
    format!(
        "{} games, {} hidden, {} favorites, {} folders; {} skipped",
        c.games,
        c.hidden,
        c.favorites,
        c.folders,
        c.ignored.len()
    )
}

/// Diagnostics for D-001: the top level of the WPF data folder (names and sizes), so the real
/// format of hidden games, favorites and folders can be confirmed from the log.
fn log_top_level(source: &Path) {
    let mut names: Vec<String> = fs::read_dir(source)
        .map(|r| {
            r.flatten()
                .map(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    match e.metadata() {
                        Ok(m) if m.is_dir() => format!("{n}\\"),
                        Ok(m) => format!("{n} ({} bytes)", m.len()),
                        Err(_) => n,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    logging::info(
        "copy",
        &format!("WPF data folder top level: {}", names.join(", ")),
    );
}

fn is_dir_in(fp: &Fingerprint, rel: &Path) -> bool {
    fp.entries.iter().any(|e| e.is_dir && e.rel == rel)
}

/// WPF had a "Games Folder" setting that can put the library outside the data folder. T0
/// copies only the data folder; this finds such a setting (any top-level .json with a key
/// containing "gamesfolder") so the user is told.
pub fn external_games_folder_note(source: &Path) -> Option<String> {
    let read = fs::read_dir(source).ok()?;
    for entry in read.flatten() {
        let path = entry.path();
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("json"))
            != Some(true)
        {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else { continue };
        let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
        let Ok(serde_json::Value::Object(map)) = serde_json::from_slice::<serde_json::Value>(bytes)
        else {
            continue;
        };
        for (k, v) in map {
            if k.to_lowercase()
                .replace(['_', ' '], "")
                .contains("gamesfolder")
            {
                if let Some(s) = v.as_str().filter(|s| !s.trim().is_empty()) {
                    return Some(format!(
                        "Your previous AIONE points its library to {s} (setting \"{k}\" in {}). That folder was not copied.",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::tests::make_library;

    fn wpf(root: &Path) -> PathBuf {
        let src = root.join("Local").join("AIONE");
        make_library(&src);
        fs::write(
            src.join("settings.json"),
            r#"{"Theme":"Switch","Volume":40}"#,
        )
        .unwrap();
        fs::create_dir_all(src.join("Logs")).unwrap();
        fs::write(src.join("Logs").join("aione.log"), "old log").unwrap();
        fs::create_dir_all(src.join("Empty")).unwrap();
        src
    }

    #[test]
    fn copies_everything_and_leaves_the_source_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let src = wpf(tmp.path());
        let data = tmp.path().join("Local").join("AIONE4");
        fs::create_dir_all(&data).unwrap();
        let before = fingerprint(&src);

        let mut calls = 0;
        let r = copy(&data, &src, |_| calls += 1);
        assert!(r.ok, "{r:?}");
        assert_eq!(r.outcome, "copied");
        assert!(r.source_unchanged);
        assert!(calls >= 2);
        assert_eq!(r.files_copied, before.files());
        assert_eq!(r.bytes_copied, before.bytes());
        // L-17: the reported counts equal what the loader's rule finds in the copy.
        let dest = destination(&data);
        let loaded = library::scan_library(&dest);
        assert_eq!(r.library, loaded);
        assert_eq!(
            (
                r.library.games,
                r.library.hidden,
                r.library.favorites,
                r.library.folders
            ),
            (4, 2, 3, 3)
        );
        // Same tree, same sizes, empty folders included.
        let copied = fingerprint(&dest);
        let shape = |fp: &Fingerprint| {
            fp.entries
                .iter()
                .map(|e| (e.rel.clone(), e.is_dir, e.size))
                .collect::<Vec<_>>()
        };
        assert_eq!(shape(&copied), shape(&before));
        assert!(dest.join("Empty").is_dir());
        // Source byte-for-byte as before.
        assert_eq!(before, fingerprint(&src));
        assert!(!dest.with_file_name("AIONE-WPF.partial").exists());
    }

    #[test]
    fn offer_is_made_once() {
        let tmp = tempfile::tempdir().unwrap();
        let src = wpf(tmp.path());
        let data = tmp.path().join("AIONE4");
        fs::create_dir_all(&data).unwrap();
        match status(&data, &src) {
            MigrationStatus::Offer { library, files, .. } => {
                assert_eq!(library.games, 4);
                assert!(files > 0);
            }
            other => panic!("expected offer, got {other:?}"),
        }
        record_decision(&data, &src, "declined", None).unwrap();
        assert_eq!(
            status(&data, &src),
            MigrationStatus::AlreadyHandled {
                decision: "declined".into(),
                notice: None
            }
        );
    }

    #[test]
    fn not_now_asks_again_and_dont_copy_never_does() {
        let tmp = tempfile::tempdir().unwrap();
        let src = wpf(tmp.path());
        let data = tmp.path().join("AIONE4");
        fs::create_dir_all(&data).unwrap();
        // "Not now" records nothing: the next start (a new status call) offers again.
        assert!(matches!(status(&data, &src), MigrationStatus::Offer { .. }));
        assert!(read_state(&data).is_none());
        assert!(matches!(status(&data, &src), MigrationStatus::Offer { .. }));
        // "Don't copy" is final: never offered again.
        record_decision(&data, &src, "declined", None).unwrap();
        for _ in 0..3 {
            assert!(matches!(
                status(&data, &src),
                MigrationStatus::AlreadyHandled { .. }
            ));
        }
    }

    const ALPHA1: &str = include_str!("../../tests/fixtures/migration/alpha1.migration.json");
    const ALPHA2: &str = include_str!("../../tests/fixtures/migration/alpha2.migration.json");

    fn data_with_record(record: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let src = wpf(tmp.path());
        let data = tmp.path().join("AIONE4");
        fs::create_dir_all(&data).unwrap();
        fs::write(state_path(&data), record).unwrap();
        (tmp, data, src)
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(dir)
            .map(|r| {
                r.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }

    #[test]
    fn b6_every_released_record_reads_as_copied() {
        for (name, record) in [("alpha.1", ALPHA1), ("alpha.2", ALPHA2)] {
            let (_tmp, data, src) = data_with_record(record);
            match read_record(&data) {
                RecordRead::Ok(s) => {
                    assert_eq!(s.decision, "copied", "{name}");
                    let r = s.result.unwrap();
                    assert_eq!((r.files_copied, r.library.games), (2857, 10), "{name}");
                }
                other => panic!("{name} record did not read: {other:?}"),
            }
            assert_eq!(
                status(&data, &src),
                MigrationStatus::AlreadyHandled {
                    decision: "copied".into(),
                    notice: None
                },
                "{name}"
            );
            // Nothing was backed up, rewritten or moved aside.
            assert_eq!(entries(&data), vec!["migration.json".to_string()], "{name}");
            assert_eq!(
                fs::read_to_string(state_path(&data)).unwrap(),
                record,
                "{name}"
            );
        }
    }

    #[test]
    fn alpha1_record_has_no_folders_field() {
        // The fixture really is the alpha.1 shape that alpha.2 could not read.
        assert!(!ALPHA1.contains("folders"));
        assert_eq!(ALPHA1.lines().nth(23), Some("    },"));
    }

    #[test]
    fn unreadable_record_with_a_finished_copy_is_backed_up_reported_and_not_asked() {
        let (_tmp, data, src) = data_with_record("{ not json");
        fs::create_dir_all(destination(&data).join("Games")).unwrap();
        match status(&data, &src) {
            MigrationStatus::AlreadyHandled { decision, notice } => {
                assert_eq!(decision, "copied");
                assert!(notice.unwrap().contains("could not read"));
            }
            other => panic!("expected handled, got {other:?}"),
        }
        let backups: Vec<_> = entries(&data)
            .into_iter()
            .filter(|n| n.starts_with("migration.json.unreadable-"))
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(
            fs::read_to_string(data.join(&backups[0])).unwrap(),
            "{ not json"
        );
        // A new, readable record was written; the next start reads it quietly.
        assert!(matches!(read_record(&data), RecordRead::Ok(_)));
        assert_eq!(
            status(&data, &src),
            MigrationStatus::AlreadyHandled {
                decision: "copied".into(),
                notice: None
            }
        );
    }

    #[test]
    fn unreadable_record_without_a_copy_asks_with_a_notice_and_backs_up_once() {
        let (_tmp, data, src) = data_with_record("{\"schema\":1}");
        for _ in 0..3 {
            match status(&data, &src) {
                MigrationStatus::Offer { notice, .. } => {
                    assert!(notice.unwrap().contains("asks again"))
                }
                other => panic!("expected offer, got {other:?}"),
            }
        }
        let backups = entries(&data)
            .into_iter()
            .filter(|n| n.starts_with("migration.json.unreadable-"))
            .count();
        assert_eq!(backups, 1);
    }

    #[test]
    fn missing_record_with_a_finished_copy_is_not_asked_and_record_is_written() {
        let tmp = tempfile::tempdir().unwrap();
        let src = wpf(tmp.path());
        let data = tmp.path().join("AIONE4");
        fs::create_dir_all(destination(&data).join("Games")).unwrap();
        assert!(matches!(
            status(&data, &src),
            MigrationStatus::AlreadyHandled { .. }
        ));
        assert!(matches!(read_record(&data), RecordRead::Ok(s) if s.decision == "copied"));
    }

    #[test]
    fn an_interrupted_copy_is_not_a_finished_copy() {
        let tmp = tempfile::tempdir().unwrap();
        let src = wpf(tmp.path());
        let data = tmp.path().join("AIONE4");
        fs::create_dir_all(
            data.join(IMPORT_DIR)
                .join("AIONE-WPF.partial")
                .join("Games"),
        )
        .unwrap();
        assert!(!finished_copy_exists(&data));
        assert!(matches!(status(&data, &src), MigrationStatus::Offer { .. }));
    }

    #[test]
    fn copy_writes_every_promised_log_line() {
        let tmp = tempfile::tempdir().unwrap();
        let src = wpf(tmp.path());
        let data = tmp.path().join("AIONE4");
        let r = copy(&data, &src, |_| {});
        assert!(r.ok);
        let log = logging::captured();
        let has = |needle: &str| log.iter().any(|l| l.contains(needle));
        assert!(has("skipped in Games: _Removed (skipped by name"));
        assert!(has("library hidden: 2, read from"));
        assert!(has("library favorites: 3, read from"));
        assert!(has("library folders: 3, read from"));
        assert!(has("not yet read: Config\\"));
        assert!(has("WPF data folder top level") || has("source check:"));
        assert!(has("4 games, 2 hidden, 3 favorites, 3 folders; 5 skipped"));
    }

    #[test]
    fn counts_line_wording() {
        let c = LibraryCounts {
            games: 10,
            ignored: vec![library::IgnoredEntry {
                name: "_Removed".into(),
                reason: "x".into(),
            }],
            ..Default::default()
        };
        assert_eq!(
            counts_line(&c),
            "10 games, 0 hidden, 0 favorites, 0 folders; 1 skipped"
        );
    }

    #[test]
    fn failed_copy_is_offered_again() {
        let tmp = tempfile::tempdir().unwrap();
        let src = wpf(tmp.path());
        let data = tmp.path().join("AIONE4");
        fs::create_dir_all(&data).unwrap();
        record_decision(&data, &src, "failed", None).unwrap();
        assert!(matches!(status(&data, &src), MigrationStatus::Offer { .. }));
    }

    #[test]
    fn no_source_means_no_offer() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("AIONE4");
        assert!(matches!(
            status(&data, &tmp.path().join("missing")),
            MigrationStatus::NoSource { .. }
        ));
        let empty = tmp.path().join("AIONE");
        fs::create_dir_all(&empty).unwrap();
        assert!(matches!(
            status(&data, &empty),
            MigrationStatus::NoSource { .. }
        ));
    }

    #[test]
    fn refuses_overlapping_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let src = wpf(tmp.path());
        // A data folder inside the source would mean writing into it.
        let data = src.join("AIONE4");
        let before = fingerprint(&src);
        let r = copy(&data, &src, |_| {});
        assert!(!r.ok);
        assert_eq!(r.outcome, "failed");
        assert_eq!(before, fingerprint(&src));
    }

    #[test]
    fn an_existing_copy_is_moved_aside_not_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        let src = wpf(tmp.path());
        let data = tmp.path().join("AIONE4");
        fs::create_dir_all(destination(&data)).unwrap();
        fs::write(destination(&data).join("keep.txt"), "mine").unwrap();
        let r = copy(&data, &src, |_| {});
        assert!(r.ok);
        let olds: Vec<_> = fs::read_dir(data.join(IMPORT_DIR))
            .unwrap()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("AIONE-WPF.old-")
            })
            .collect();
        assert_eq!(olds.len(), 1);
        assert!(olds[0].path().join("keep.txt").is_file());
    }

    #[test]
    fn finds_an_external_games_folder_setting() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("settings.json"),
            "\u{feff}{\"GamesFolder\":\"D:\\\\Games\"}",
        )
        .unwrap();
        let n = external_games_folder_note(tmp.path()).unwrap();
        assert!(n.contains("D:\\Games"));
        fs::write(tmp.path().join("settings.json"), r#"{"GamesFolder":""}"#).unwrap();
        assert!(external_games_folder_note(tmp.path()).is_none());
    }

    #[test]
    fn diff_reports_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let src = wpf(tmp.path());
        let a = fingerprint(&src);
        fs::write(src.join("new.txt"), "x").unwrap();
        let d = diff(&a, &fingerprint(&src), 10);
        assert_eq!(d, vec!["added: new.txt".to_string()]);
    }

    #[test]
    fn status_serializes_for_the_ui() {
        let s = MigrationStatus::NoSource {
            source: "C:\\x".into(),
        };
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            r#"{"state":"noSource","source":"C:\\x"}"#
        );
    }
}
