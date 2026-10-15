//! B-20 (rev. ag): the game is paused while AIONE is in front, like Switch.
//!
//! **Mechanism (Windows, documented):** every thread of the game's process (and of the processes
//! it started) is suspended with `SuspendThread` and resumed with `ResumeThread`, found with a
//! ToolHelp thread snapshot (`CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD)`). This is what
//! debuggers do; the undocumented `NtSuspendProcess` isn't used.
//!
//! **Known risks:** audio may repeat or click briefly at the pause or the resume; a game that
//! measures real time may jump forward on resume; online games may time out (they get
//! `"NoPause": true`); anti-cheat tools may object, so games with one are never paused.
//!
//! **Never paused:** Steam's client and helper, Explorer, the Game Bar, AIONE; games with an
//! anti-cheat in their folder or running; games with `"NoPause": true` in game.json.
//!
//! **Safety:** the paused processes are recorded in `paused.json` (AIONE's data folder); the next
//! start resumes any left paused (if AIONE closed or crashed meanwhile).

use crate::logging::{self, Level};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static PAUSED: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static DATA: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Where `paused.json` lives (set at start-up).
pub fn init(data: &Path) {
    if let Ok(mut d) = DATA.lock() {
        *d = Some(data.to_path_buf());
    }
}

fn record_file() -> Option<PathBuf> {
    DATA.lock().ok()?.as_ref().map(|d| d.join("paused.json"))
}

fn save(pids: &[u32]) {
    if let Some(f) = record_file() {
        if pids.is_empty() {
            let _ = std::fs::remove_file(f);
        } else {
            let _ = std::fs::write(f, serde_json::to_vec(pids).unwrap_or_default());
        }
    }
}

/// The processes to pause: the game's process and every process it started (by parent),
/// never the programs AIONE must leave alone.
pub fn tree(procs: &[(u32, u32, Option<PathBuf>)], root: u32) -> Vec<u32> {
    let name_ok = |path: &Option<PathBuf>| {
        let n = path
            .as_ref()
            .map(|p| {
                p.to_string_lossy()
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or_default()
                    .to_string()
            })
            .unwrap_or_default();
        !crate::launch::is_not_the_game(&n)
    };
    let mut out = vec![root];
    let mut i = 0;
    while i < out.len() {
        let parent = out[i];
        for (pid, ppid, path) in procs {
            if *ppid == parent && *pid != parent && !out.contains(pid) && name_ok(path) {
                out.push(*pid);
            }
        }
        i += 1;
    }
    out
}

/// An anti-cheat in the game's folder or running: the reason, if so (never paused).
pub fn anti_cheat(
    install_dir: Option<&Path>,
    procs: &[(u32, u32, Option<PathBuf>)],
) -> Option<String> {
    const MARKS: [&str; 4] = ["easyanticheat", "battleye", "beservice", "vgc.exe"];
    if let Some(dir) = install_dir {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                let n = e.file_name().to_string_lossy().to_ascii_lowercase();
                if let Some(m) = MARKS.iter().find(|m| n.contains(*m)) {
                    return Some(format!("an anti-cheat ({m}) is in its folder"));
                }
            }
        }
    }
    for (_, _, path) in procs {
        let n = path
            .as_ref()
            .map(|p| {
                p.to_string_lossy()
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
            })
            .unwrap_or_default();
        if let Some(m) = MARKS.iter().find(|m| n.contains(*m)) {
            return Some(format!("an anti-cheat ({m}) is running"));
        }
    }
    None
}

/// Pauses the running game (AIONE came to the front over it). Returns what happened.
pub fn pause_running(why: &str) -> String {
    let Some(g) = crate::launch::running_game_info() else {
        return "no game is running".into();
    };
    if PAUSED.lock().map(|p| !p.is_empty()).unwrap_or(false) {
        return format!("\"{}\" is already paused", g.name);
    }
    let Some(pid) = g.pid else {
        let m = format!(
            "pause \"{}\" ({why}): its process isn't known; not paused",
            g.name
        );
        logging::lifecycle(Level::Info, &m);
        return m;
    };
    if g.no_pause {
        let m = format!(
            "pause \"{}\" ({why}): not paused: \"NoPause\": true in its game.json",
            g.name
        );
        logging::lifecycle(Level::Info, &m);
        return m;
    }
    let procs = crate::launch::processes();
    if let Some(reason) = anti_cheat(g.install_dir.as_deref(), &procs) {
        let m = format!("pause \"{}\" ({why}): not paused: {reason}", g.name);
        logging::lifecycle(Level::Info, &m);
        return m;
    }
    let pids = tree(&procs, pid);
    let mut done = Vec::new();
    let mut threads = 0usize;
    for p in &pids {
        let n = suspend_process(*p);
        if n > 0 {
            done.push(*p);
            threads += n;
        }
    }
    if let Ok(mut paused) = PAUSED.lock() {
        *paused = done.clone();
    }
    save(&done);
    let m = format!(
        "pause \"{}\" ({why}): paused {} process(es) {:?}, {threads} thread(s) (SuspendThread)",
        g.name,
        done.len(),
        done
    );
    logging::lifecycle(Level::Info, &m);
    m
}

/// Resumes the paused game (A Continue, Guide back, before any close, AIONE closing).
pub fn resume_running(why: &str) -> String {
    let pids = PAUSED
        .lock()
        .map(|mut p| std::mem::take(&mut *p))
        .unwrap_or_default();
    if pids.is_empty() {
        return "nothing paused".into();
    }
    let threads: usize = pids.iter().map(|p| resume_process(*p)).sum();
    save(&[]);
    let m = format!(
        "resume ({why}): {} process(es) {:?}, {threads} thread(s) (ResumeThread)",
        pids.len(),
        pids
    );
    logging::lifecycle(Level::Info, &m);
    m
}

/// At start-up: resumes any process a previous AIONE left paused (it closed or crashed).
pub fn resume_left_over() {
    let Some(f) = record_file() else { return };
    let Ok(bytes) = std::fs::read(&f) else { return };
    let pids: Vec<u32> = serde_json::from_slice(&bytes).unwrap_or_default();
    let threads: usize = pids.iter().map(|p| resume_process(*p)).sum();
    let _ = std::fs::remove_file(&f);
    logging::lifecycle(
        Level::Warn,
        &format!(
            "resume (left paused by a previous AIONE): {} process(es) {:?}, {threads} thread(s)",
            pids.len(),
            pids
        ),
    );
}

// ---------- the platform layer ----------

#[cfg(windows)]
fn each_thread(pid: u32, f: impl Fn(windows::Win32::Foundation::HANDLE) -> bool) -> usize {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };
    use windows::Win32::System::Threading::{OpenThread, THREAD_SUSPEND_RESUME};
    let mut n = 0;
    let Ok(snap) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }) else {
        return 0;
    };
    let mut e = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    let mut ok = unsafe { Thread32First(snap, &mut e) }.is_ok();
    while ok {
        if e.th32OwnerProcessID == pid {
            if let Ok(h) = unsafe { OpenThread(THREAD_SUSPEND_RESUME, false, e.th32ThreadID) } {
                if f(h) {
                    n += 1;
                }
                let _ = unsafe { CloseHandle(h) };
            }
        }
        ok = unsafe { Thread32Next(snap, &mut e) }.is_ok();
    }
    let _ = unsafe { CloseHandle(snap) };
    n
}

#[cfg(windows)]
pub fn suspend_process(pid: u32) -> usize {
    use windows::Win32::System::Threading::SuspendThread;
    each_thread(pid, |h| unsafe { SuspendThread(h) } != u32::MAX)
}

#[cfg(windows)]
pub fn resume_process(pid: u32) -> usize {
    use windows::Win32::System::Threading::ResumeThread;
    each_thread(pid, |h| unsafe { ResumeThread(h) } != u32::MAX)
}

#[cfg(not(windows))]
pub fn suspend_process(_pid: u32) -> usize {
    0
}

#[cfg(not(windows))]
pub fn resume_process(_pid: u32) -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pauses_the_game_and_what_it_started_never_steam() {
        let procs = vec![
            (100, 1, Some(PathBuf::from("D:\\Console\\Cemu\\Cemu.exe"))),
            (
                101,
                100,
                Some(PathBuf::from("D:\\Console\\Cemu\\helper.exe")),
            ),
            (
                102,
                101,
                Some(PathBuf::from("D:\\Console\\Cemu\\grandchild.exe")),
            ),
            (
                103,
                100,
                Some(PathBuf::from("C:\\Steam\\steamwebhelper.exe")),
            ),
            (200, 1, Some(PathBuf::from("C:\\Other\\other.exe"))),
        ];
        assert_eq!(tree(&procs, 100), vec![100, 101, 102]);
    }

    #[test]
    fn games_with_an_anti_cheat_are_never_paused() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(anti_cheat(Some(dir.path()), &[]), None);
        std::fs::create_dir(dir.path().join("EasyAntiCheat")).unwrap();
        assert!(anti_cheat(Some(dir.path()), &[])
            .unwrap()
            .contains("easyanticheat"));
        let running = vec![(
            5,
            1,
            Some(PathBuf::from(
                "C:\\Program Files\\Common Files\\BattlEye\\BEService.exe",
            )),
        )];
        assert!(anti_cheat(None, &running).unwrap().contains("running"));
    }

    /// Windows only: a real process is suspended and resumed (a mock game: `ping`).
    #[cfg(windows)]
    #[test]
    fn suspends_and_resumes_a_real_process() {
        let mut child = std::process::Command::new("ping")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let n = suspend_process(child.id());
        assert!(n > 0, "no thread suspended");
        assert!(child.try_wait().unwrap().is_none());
        assert_eq!(resume_process(child.id()), n);
        let _ = child.kill();
    }
}
