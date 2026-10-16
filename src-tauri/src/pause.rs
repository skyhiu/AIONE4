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

// ---------- D-039 (rev. al): paused only once AIONE is confirmed in front ----------

/// A Guide press asked for a pause; it happens only after the foreground step succeeds.
static REQUESTED: Mutex<Option<std::time::Instant>> = Mutex::new(None);
/// D-043: when the watchdog last fired (a late confirmation within 5 s still pauses).
static WATCHDOG_FIRED: Mutex<Option<std::time::Instant>> = Mutex::new(None);
/// The watchdog: no pause if AIONE isn't confirmed in front within this time.
pub const WATCHDOG_MS: u64 = 1000;

/// What to do with a pause request.
#[derive(Debug, PartialEq)]
pub enum Gate {
    /// The foreground step succeeded in time: pause now.
    Pause,
    /// Still waiting for the foreground step.
    Wait,
    /// The foreground step failed, or took longer than the watchdog: never pause.
    Skip,
}

/// The decision: pause only after the foreground step succeeded, and within the watchdog.
pub fn gate(foreground_ok: Option<bool>, waited_ms: u64) -> Gate {
    match foreground_ok {
        Some(true) if waited_ms <= WATCHDOG_MS => Gate::Pause,
        Some(_) => Gate::Skip,
        None if waited_ms > WATCHDOG_MS => Gate::Skip,
        None => Gate::Wait,
    }
}

/// Guide brought (or is bringing) AIONE forward: a pause is requested, with its watchdog.
pub fn request_pause() {
    let at = std::time::Instant::now();
    if let Ok(mut r) = REQUESTED.lock() {
        *r = Some(at);
    }
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(WATCHDOG_MS + 50));
        let still = REQUESTED.lock().map(|r| *r == Some(at)).unwrap_or(false);
        if still && gate(None, at.elapsed().as_millis() as u64) == Gate::Skip {
            if let Ok(mut r) = REQUESTED.lock() {
                *r = None;
            }
            logging::lifecycle(Level::Warn, "pause: AIONE wasn't confirmed in front within 1 s (watchdog): the game isn't paused (retried when AIONE is confirmed)");
            if let Ok(mut w) = WATCHDOG_FIRED.lock() {
                *w = Some(std::time::Instant::now());
            }
            resume_running("watchdog: AIONE not in front within 1 s");
        }
    });
}

/// The foreground step finished (AIONE visible and active, or not): pause now if it was requested.
pub fn foreground_done(ok: bool) {
    let Some(at) = REQUESTED.lock().ok().and_then(|mut r| r.take()) else {
        // D-043: the watchdog fired, but AIONE is now confirmed in front: pause after all.
        let late = WATCHDOG_FIRED
            .lock()
            .ok()
            .and_then(|mut w| w.take())
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(5));
        if ok && late {
            pause_running("Guide: AIONE confirmed in front after the watchdog (retried)");
        }
        return;
    };
    match gate(Some(ok), at.elapsed().as_millis() as u64) {
        Gate::Pause => {
            pause_running("Guide: AIONE confirmed in front");
        }
        _ => logging::lifecycle(
            Level::Info,
            "pause: not paused (AIONE wasn't confirmed in front in time)",
        ),
    }
}

/// Whether AIONE has this process paused (the foreground step never attaches to its threads).
pub fn is_paused(pid: u32) -> bool {
    PAUSED.lock().map(|p| p.contains(&pid)).unwrap_or(false)
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
    // D-044: only the game is ever paused, never a launcher.
    if !g.counted {
        let m = format!(
            "pause \"{}\" ({why}): not paused: not the game yet (a launcher)",
            g.name
        );
        logging::lifecycle(Level::Info, &m);
        return m;
    }
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
    // B-22: AIONE paused again: any input-locking state left over is cleared.
    crate::transit::with(|m| m.clear("AIONE paused the game again", std::time::Instant::now()));
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

/// D-035 (rev. aj): what AIONE watches to resume the game as soon as the user starts leaving AIONE
/// (a paused program can't answer Windows, so it can never "get the front" itself).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Leaving {
    /// Another window took the front (any program, the taskbar, Start, the Game Bar, Steam).
    Foreground,
    /// Alt-Tab or the task switcher started (before a window is chosen).
    SwitchStart,
}

/// D-035 (rev. aj): AIONE is being left: resume the paused game at once. `program` names the
/// window that took the front (for the log). Returns whether a game was resumed.
pub fn on_leaving(what: Leaving, program: &str) -> bool {
    let paused = PAUSED.lock().map(|p| !p.is_empty()).unwrap_or(false);
    if paused {
        match what {
            Leaving::Foreground => resume_running(&format!("AIONE lost the front: {program}")),
            Leaving::SwitchStart => resume_running("Alt-Tab started"),
        };
    }
    paused
}

/// D-035 (rev. aj): two WinEvent hooks for AIONE's lifetime, on their own thread:
/// `EVENT_SYSTEM_FOREGROUND` (another window took the front; AIONE's own windows are skipped with
/// `WINEVENT_SKIPOWNPROCESS`) and `EVENT_SYSTEM_SWITCHSTART` (Alt-Tab started).
#[cfg(windows)]
pub fn start_watch() {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, GetWindowThreadProcessId, TranslateMessage,
        EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_SWITCHSTART, MSG, WINEVENT_OUTOFCONTEXT,
        WINEVENT_SKIPOWNPROCESS,
    };
    unsafe extern "system" fn on_event(
        _h: HWINEVENTHOOK,
        event: u32,
        window: HWND,
        _o: i32,
        _c: i32,
        _t: u32,
        _time: u32,
    ) {
        if !PAUSED.lock().map(|p| !p.is_empty()).unwrap_or(false) {
            return;
        }
        if event == EVENT_SYSTEM_SWITCHSTART {
            on_leaving(Leaving::SwitchStart, "");
            return;
        }
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
        let program = crate::launch::exe_name(pid).unwrap_or_else(|| format!("process {pid}"));
        on_leaving(Leaving::Foreground, &program);
    }
    std::thread::spawn(|| {
        // SAFETY: out-of-context hooks with a static callback, kept for AIONE's lifetime; the message
        // loop below delivers their events on this thread.
        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        let fg = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(on_event),
                0,
                0,
                flags,
            )
        };
        let sw = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_SWITCHSTART,
                EVENT_SYSTEM_SWITCHSTART,
                None,
                Some(on_event),
                0,
                0,
                flags,
            )
        };
        if fg.0.is_null() || sw.0.is_null() {
            logging::lifecycle(
                Level::Warn,
                "pause: a leaving watch could not be installed (SetWinEventHook failed)",
            );
        }
        let mut msg = MSG::default();
        while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });
}

#[cfg(not(windows))]
pub fn start_watch() {}

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

    /// The tests share PAUSED: one at a time.
    static LOCK: Mutex<()> = Mutex::new(());
    fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

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

    /// D-039 (rev. al): the pause comes only after the foreground step succeeded, within 1 s.
    #[test]
    fn the_pause_waits_for_the_foreground_step() {
        assert_eq!(gate(None, 100), Gate::Wait); // no foreground result yet: never pause first
        assert_eq!(gate(Some(true), 150), Gate::Pause); // AIONE confirmed in front: pause
        assert_eq!(gate(Some(false), 150), Gate::Skip); // the foreground step failed: no pause
        assert_eq!(gate(None, 1200), Gate::Skip); // a slow foreground step: the watchdog, no pause
        assert_eq!(gate(Some(true), 1500), Gate::Skip); // confirmed too late: no pause
    }

    /// D-039: a paused process is known as such (the foreground step skips its threads).
    #[test]
    fn a_paused_process_is_known() {
        let _one = one_at_a_time();
        if let Ok(mut p) = PAUSED.lock() {
            *p = vec![777];
        }
        assert!(is_paused(777) && !is_paused(778));
        if let Ok(mut p) = PAUSED.lock() {
            p.clear();
        }
    }

    /// D-035 (rev. aj): leaving AIONE by any route resumes the game; nothing when nothing is paused.
    #[test]
    fn leaving_aione_resumes_the_game() {
        let _one = one_at_a_time();
        if let Ok(mut p) = PAUSED.lock() {
            *p = vec![4242];
        }
        assert!(on_leaving(Leaving::Foreground, "explorer.exe")); // the taskbar or Start
        assert!(PAUSED.lock().unwrap().is_empty());
        if let Ok(mut p) = PAUSED.lock() {
            *p = vec![4242];
        }
        assert!(on_leaving(Leaving::SwitchStart, "")); // Alt-Tab started
        assert!(PAUSED.lock().unwrap().is_empty());
        assert!(!on_leaving(Leaving::Foreground, "other.exe")); // nothing paused: nothing to do
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
        let _ = child.wait(); // never left as a zombie (clippy::zombie_processes)
    }

    /// Windows only (D-035, rev. aj): a real suspended process (a mock game) is resumed by a
    /// "leaving AIONE" event, through the same path the hooks use.
    #[cfg(windows)]
    #[test]
    fn a_real_paused_process_resumes_when_aione_is_left() {
        let _one = one_at_a_time();
        let mut child = std::process::Command::new("ping")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let n = suspend_process(child.id());
        assert!(n > 0);
        if let Ok(mut p) = PAUSED.lock() {
            *p = vec![child.id()];
        }
        assert!(on_leaving(Leaving::Foreground, "explorer.exe"));
        assert!(PAUSED.lock().unwrap().is_empty());
        // Resumed: suspending again finds its threads running (suspend count back to 0 first).
        assert_eq!(suspend_process(child.id()), n);
        let _ = resume_process(child.id());
        let _ = child.kill();
        let _ = child.wait(); // never left as a zombie (clippy::zombie_processes)
    }

    /// D-039 (rev. al): a slow foreground step (no result within 1 s): the watchdog resumes the
    /// paused process. On Windows a real suspended process; elsewhere a stand-in id.
    #[test]
    fn the_watchdog_resumes_after_a_slow_foreground_step() {
        let _one = one_at_a_time();
        #[cfg(windows)]
        let mut child = std::process::Command::new("ping")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        #[cfg(windows)]
        let pid = {
            std::thread::sleep(std::time::Duration::from_millis(300));
            assert!(suspend_process(child.id()) > 0);
            child.id()
        };
        #[cfg(not(windows))]
        let pid = 4243;
        if let Ok(mut p) = PAUSED.lock() {
            *p = vec![pid];
        }
        let t0 = std::time::Instant::now();
        request_pause(); // Guide; the foreground step never reports (it's "slow")
                         // Not before the watchdog's second...
        std::thread::sleep(std::time::Duration::from_millis(WATCHDOG_MS - 300));
        assert!(
            !PAUSED.lock().unwrap().is_empty(),
            "resumed before the watchdog's 1 s"
        );
        // ...then resumed; a busy machine may take a little longer to wake the watchdog (CI #40).
        while !PAUSED.lock().unwrap().is_empty() && t0.elapsed() < std::time::Duration::from_secs(5)
        {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(
            PAUSED.lock().unwrap().is_empty(),
            "the watchdog didn't resume within 5 s"
        );
        assert!(
            t0.elapsed() >= std::time::Duration::from_millis(WATCHDOG_MS),
            "resumed too early"
        );
        #[cfg(windows)]
        {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
