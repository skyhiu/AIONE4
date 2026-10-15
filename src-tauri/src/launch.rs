//! Starting games (T1.3; reference A9, Part B "Lifecycle"; B-7, B-11, B-17).
//!
//! **What is started (from `game.json`, A9):** the game's `Shortcut` (`game.lnk` or `game.url` in
//! its folder) is opened through the Windows shell, exactly like a double-click, so a `.lnk`'s own
//! target, arguments and working folder apply and a `.url` opens its URL (often `steam://…`).
//! Without a `Shortcut`, a `SteamAppId` is started as `steam://rungameid/<id>`. Nothing else in
//! `game.json` describes launching (the WPF format has no executable or argument fields).
//!
//! **Tracking:** one running game at a time.
//! - **The game's window:** the first visible top-level window of the started process (time
//!   logged). If the process exits without a window (a launcher), or no process is known (Steam
//!   starts the game itself), the game is the first new program that takes the front, never
//!   AIONE, Steam's helper, Explorer or the Game Bar.
//! - **Exit:** when the tracked process ends, AIONE is told (it comes back to the front).
//! - **Already running:** starting the same game again brings its window to the front; it is
//!   never started twice (Part B).

use crate::logging::{self, Level};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How a game was started (for the log and the UI).
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Started {
    pub id: String,
    pub name: String,
    /// "shortcut game.lnk", "shortcut game.url" or "steam://rungameid/123".
    pub how: String,
    /// The process the shell reported, if any.
    pub pid: Option<u32>,
    /// True when the game was already running and was brought to the front instead.
    pub already_running: bool,
    /// The foreground method, when it was brought to the front.
    pub foreground: Option<String>,
}

#[derive(Debug, Clone)]
struct Running {
    /// B-18 / D-025: each launch's watcher has its own generation; a watcher whose game is no
    /// longer the tracked one stops at once (never a second end line).
    gen: u64,
    id: String,
    name: String,
    pid: Option<u32>,
    exe: Option<String>,
    window: Option<isize>,
    started: Instant,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);
static GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// What the watcher reports to the app.
pub enum Event {
    /// The game's window appeared (D-026: the UI lifts the black screen only now).
    Window { id: String, name: String, ms: u128 },
    /// The game ended (once): its session's length in minutes (B-11).
    Ended {
        id: String,
        name: String,
        minutes: f64,
    },
}

/// The id of the game that is running, if any.
pub fn running_id() -> Option<String> {
    RUNNING.lock().ok()?.as_ref().map(|r| r.id.clone())
}

/// B-17 (rev. aa): right after a Guide press, if Steam or the Xbox Game Bar took the front, a
/// plain hint naming the setting to turn off.
pub fn guide_hint() -> Option<String> {
    let (_, _, exe) = foreground_program()?;
    match exe.to_ascii_lowercase().as_str() {
        "steamwebhelper.exe" | "steam.exe" => Some(
            "hint: Guide also opened Steam: turn off \"Guide button focuses Steam\" in Steam → Settings → Controller".into(),
        ),
        "gamebar.exe" | "gamebarftserver.exe" => Some(
            "hint: Guide also opened the Xbox Game Bar: turn off \"Open Xbox Game Bar using this button on a controller\" in Windows Settings → Gaming → Xbox Game Bar".into(),
        ),
        _ => None,
    }
}

/// Programs that are never "the game" when following the foreground (B-17 hints aside).
const NOT_THE_GAME: [&str; 6] = [
    "aione4.exe",
    "steamwebhelper.exe",
    "steam.exe",
    "explorer.exe",
    "gamebar.exe",
    "gamebarftserver.exe",
];

pub fn is_not_the_game(exe: &str) -> bool {
    NOT_THE_GAME.contains(&exe.to_ascii_lowercase().as_str())
}

/// What to open for a game: the shell target and a short description for the log.
pub fn target_of(f: &crate::games::LaunchFields) -> Result<(String, String), String> {
    if let Some(p) = &f.shortcut {
        if !p.is_file() {
            return Err(format!(
                "its shortcut {} is missing",
                p.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ));
        }
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        return Ok((p.display().to_string(), format!("shortcut {name}")));
    }
    if let Some(app) = &f.steam_app_id {
        let url = format!("steam://rungameid/{app}");
        return Ok((url.clone(), url));
    }
    Err("its game.json has no Shortcut and no SteamAppId".into())
}

/// Starts the game (or brings it to the front if it is already running). `report` gets the
/// game's window and its end (once), from the watcher thread.
pub fn launch(
    data: &Path,
    id: &str,
    report: impl Fn(Event) + Send + 'static,
) -> Result<Started, String> {
    let f = crate::games::launch_fields(data, id)?;
    // Already running: bring it to the front, never start it twice.
    if let Some(r) = RUNNING.lock().ok().and_then(|g| g.clone()) {
        if r.id == id && alive(&r) {
            let report = r.window.map(crate::foreground::bring_to_front);
            let method = report.as_ref().map(|r| r.summary());
            logging::lifecycle(
                Level::Info,
                &format!(
                    "start \"{}\": already running ({}): brought to the front: {}",
                    r.name,
                    r.exe.clone().unwrap_or_else(|| "its window".into()),
                    method
                        .clone()
                        .unwrap_or_else(|| "its window isn't known yet".into())
                ),
            );
            return Ok(Started {
                id: id.into(),
                name: f.name,
                how: "already running".into(),
                pid: r.pid,
                already_running: true,
                foreground: method,
            });
        }
    }
    let (target, how) = target_of(&f)?;
    logging::lifecycle(Level::Info, &format!("start \"{}\": opening {how}", f.name));
    let pid = shell_open(&target, &f.folder)?;
    logging::lifecycle(
        Level::Info,
        &format!(
            "start \"{}\": started ({how}); {}",
            f.name,
            pid.map(|p| format!("process {p} ({})", exe_of(p).unwrap_or_else(|| "?".into())))
                .unwrap_or_else(
                    || "no process reported (the shell or Steam starts the game)".into()
                )
        ),
    );
    let gen = GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    let run = Running {
        gen,
        id: id.into(),
        name: f.name.clone(),
        pid,
        exe: pid.and_then(exe_of),
        window: None,
        started: Instant::now(),
    };
    if let Ok(mut g) = RUNNING.lock() {
        if let Some(old) = g.replace(run) {
            logging::lifecycle(
                Level::Warn,
                &format!(
                    "\"{}\" is no longer tracked (another game was started)",
                    old.name
                ),
            );
        }
    }
    std::thread::spawn(move || watch(gen, report));
    Ok(Started {
        id: id.into(),
        name: f.name,
        how,
        pid,
        already_running: false,
        foreground: None,
    })
}

/// Brings the running game's window to the front (A or Guide on the running game, B-7).
pub fn return_to_game(id: &str) -> Result<String, String> {
    let r = RUNNING
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .ok_or("no game is running")?;
    if r.id != id || !alive(&r) {
        return Err("that game isn't running".into());
    }
    hand_over(&r, "back to")
}

/// D-027: hands the front to the running game. The game's main window is found again by its
/// process every time (never a stale handle) and its state logged; a minimized window is
/// restored and an active pop-up targeted (by `bring_to_front`, alpha.15's methods). If Windows
/// still refuses, AIONE steps aside: the game's window goes to the top (not activated) and AIONE
/// minimizes itself, so Windows activates the game; the next Guide restores AIONE.
fn hand_over(r: &Running, what: &str) -> Result<String, String> {
    let target = r
        .pid
        .and_then(main_window_of)
        .or(r.window.filter(|w| window_alive(*w)));
    let Some(main) = target else {
        let m = format!(
            "{what} \"{}\": no window of {} found",
            r.name,
            r.exe.clone().unwrap_or_default()
        );
        logging::lifecycle(Level::Warn, &m);
        return Err(m);
    };
    let popup = active_popup(main);
    let target = popup.unwrap_or(main);
    logging::lifecycle(
        Level::Info,
        &format!(
            "{what} \"{}\": its window {}{}",
            r.name,
            window_state(main),
            popup
                .map(|p| format!("; its active pop-up {}", window_state(p)))
                .unwrap_or_default()
        ),
    );
    if let Ok(mut g) = RUNNING.lock() {
        if let Some(x) = g.as_mut().filter(|x| x.gen == r.gen) {
            x.window = Some(main);
        }
    }
    let report = crate::foreground::bring_to_front(target);
    if report.ok {
        let m = format!("{what} \"{}\": {}", r.name, report.summary());
        logging::lifecycle(Level::Info, &m);
        return Ok(m);
    }
    // Windows refused: AIONE steps aside.
    let aside = step_aside(crate::foreground::main_window(), target, r.pid.unwrap_or(0));
    let m = format!(
        "{what} \"{}\": {}; then {}",
        r.name,
        report.summary(),
        if aside {
            "OK via step-aside (AIONE minimized itself; the game is in front)"
        } else {
            "step-aside also failed (the game isn't in front)"
        }
    );
    logging::lifecycle(if aside { Level::Info } else { Level::Warn }, &m);
    if aside {
        Ok(m)
    } else {
        Err(m)
    }
}

fn alive(r: &Running) -> bool {
    match r.pid {
        Some(p) => process_alive(p),
        None => r.window.is_some_and(window_alive),
    }
}

/// The watcher (one per launch): finds the game's window, follows a launcher to the real game,
/// and reports the game's end once. It stops at once when its game is no longer the tracked one.
fn watch(gen: u64, report: impl Fn(Event)) {
    let mut found_logged = false;
    loop {
        std::thread::sleep(Duration::from_millis(250));
        let Some(mut r) = RUNNING
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .filter(|r| r.gen == gen)
        else {
            return;
        };
        let since = r.started.elapsed();
        // 1. The game's window.
        if r.window.is_none() {
            if let Some(p) = r.pid {
                r.window = top_window_of(p);
            }
            if r.window.is_none() && (r.pid.is_none() || !r.pid.is_some_and(process_alive)) {
                // A launcher that exited, or Steam: the first new program in front is the game.
                if let Some((w, p, exe)) = foreground_program() {
                    if !is_not_the_game(&exe) {
                        logging::lifecycle(
                            Level::Info,
                            &format!(
                                "start \"{}\": the game continued in {exe} (process {p})",
                                r.name
                            ),
                        );
                        r.window = Some(w);
                        r.pid = Some(p);
                        r.exe = Some(exe);
                    }
                }
            }
            if r.window.is_some() && !found_logged {
                found_logged = true;
                logging::lifecycle(
                    Level::Info,
                    &format!(
                        "start \"{}\": its window appeared after {} ms ({})",
                        r.name,
                        since.as_millis(),
                        r.exe.clone().unwrap_or_default()
                    ),
                );
                report(Event::Window {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    ms: since.as_millis(),
                });
                // D-027: the game's window appeared: AIONE hands it the front (Steam included).
                if let Ok(mut g) = RUNNING.lock() {
                    if g.as_ref().is_some_and(|x| x.gen == gen) {
                        *g = Some(r.clone());
                    }
                }
                let _ = hand_over(&r, "its window appeared:");
            }
            // A launcher that closed without starting a game counts as ended (B-18); so does a
            // start that never shows a window within 2 minutes.
            let launcher_gone =
                r.pid.is_some_and(|p| !process_alive(p)) && since > Duration::from_secs(10);
            if r.window.is_none() && (launcher_gone || since > Duration::from_secs(120)) {
                logging::lifecycle(
                    Level::Warn,
                    &format!(
                        "start \"{}\": no game window ({}); counted as ended",
                        r.name,
                        if launcher_gone {
                            "its launcher closed"
                        } else {
                            "2 minutes passed"
                        }
                    ),
                );
                if take_if(gen) {
                    report(Event::Ended {
                        id: r.id.clone(),
                        name: r.name.clone(),
                        minutes: since.as_secs_f64() / 60.0,
                    });
                }
                return;
            }
        }
        // 2. The game's end: its process ending (an emulator: the emulator ending, B-18).
        if r.window.is_some() && !alive(&r) {
            let minutes = since.as_secs_f64() / 60.0;
            if take_if(gen) {
                logging::lifecycle(
                    Level::Info,
                    &format!("\"{}\" ended after {:.1} min", r.name, minutes),
                );
                report(Event::Ended {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    minutes,
                });
            }
            return;
        }
        if let Ok(mut g) = RUNNING.lock() {
            if g.as_ref().is_some_and(|x| x.gen == gen) {
                *g = Some(r);
            }
        }
    }
}

/// Clears the tracked game if it is still this watcher's; true when it was (the end is then
/// reported exactly once).
fn take_if(gen: u64) -> bool {
    RUNNING
        .lock()
        .map(|mut g| {
            if g.as_ref().is_some_and(|x| x.gen == gen) {
                *g = None;
                true
            } else {
                false
            }
        })
        .unwrap_or(false)
}

/// B-18: closes the running game politely (WM_CLOSE to its windows, as its ✕ would; never
/// killed) and waits up to `wait` for its process to end. The watcher reports the end.
pub fn close_game(id: &str, wait: Duration) -> Result<(), String> {
    let r = RUNNING
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .filter(|r| r.id == id)
        .ok_or("that game isn't running")?;
    let Some(pid) = r.pid else {
        return Err("its process isn't known, so AIONE can't close it".into());
    };
    let n = close_windows_of(pid);
    logging::lifecycle(
        Level::Info,
        &format!(
            "close \"{}\": asked {n} window(s) of {} to close (as their ✕ would)",
            r.name,
            r.exe.clone().unwrap_or_default()
        ),
    );
    // D-029: no window found: say so at once (no 10 s wait).
    if n == 0 {
        return Err(format!(
            "AIONE found no window of {} to close; it was left as it is",
            r.name
        ));
    }
    let t0 = Instant::now();
    let mut shown = false;
    while t0.elapsed() < wait {
        std::thread::sleep(Duration::from_millis(200));
        // D-029: still running after a moment: it may be asking something: bring it to the front.
        if !shown && t0.elapsed() > Duration::from_millis(600) && process_alive(pid) {
            shown = true;
            let _ = hand_over(&r, "close: it may be asking something:");
            logging::lifecycle(
                Level::Info,
                &format!(
                    "hint: \"{}\" may be asking something: answer it there",
                    r.name
                ),
            );
        }
        if !process_alive(pid) {
            logging::lifecycle(
                Level::Info,
                &format!(
                    "close \"{}\": ended after {} ms",
                    r.name,
                    t0.elapsed().as_millis()
                ),
            );
            return Ok(());
        }
    }
    logging::lifecycle(
        Level::Warn,
        &format!(
            "close \"{}\": still running after {} s; left as it is (never forced)",
            r.name,
            wait.as_secs()
        ),
    );
    Err(format!(
        "{} is still running after {} seconds (it may be asking to save); it was left as it is",
        r.name,
        wait.as_secs()
    ))
}

// ---------- the platform layer ----------

#[cfg(windows)]
mod win {
    use windows::core::{BOOL, HSTRING, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT, WPARAM};
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, GetProcessId, OpenProcess, QueryFullProcessImageNameW,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetAncestor, GetClassNameW, GetForegroundWindow, GetLastActivePopup,
        GetSystemMetrics, GetWindow, GetWindowRect, GetWindowTextLengthW, GetWindowThreadProcessId,
        IsIconic, IsWindow, IsWindowVisible, PostMessageW, SetWindowPos, ShowWindow, GA_ROOT,
        GW_OWNER, HWND_TOP, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        SW_MINIMIZE, SW_SHOWNORMAL, WM_CLOSE,
    };

    pub fn shell_open(target: &str, folder: &std::path::Path) -> Result<Option<u32>, String> {
        let file = HSTRING::from(target);
        let dir = HSTRING::from(folder.as_os_str());
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI,
            lpFile: PCWSTR(file.as_ptr()),
            lpDirectory: PCWSTR(dir.as_ptr()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        unsafe { ShellExecuteExW(&mut info) }.map_err(|e| e.message().trim().to_string())?;
        if info.hProcess.is_invalid() {
            return Ok(None);
        }
        let pid = unsafe { GetProcessId(info.hProcess) };
        let _ = unsafe { CloseHandle(info.hProcess) };
        Ok(if pid == 0 { None } else { Some(pid) })
    }

    pub fn exe_of(pid: u32) -> Option<String> {
        let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = unsafe {
            QueryFullProcessImageNameW(
                h,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
        };
        let _ = unsafe { CloseHandle(h) };
        ok.ok()?;
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        full.rsplit(['\\', '/']).next().map(str::to_string)
    }

    pub fn process_alive(pid: u32) -> bool {
        let Ok(h) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
            return false;
        };
        let mut code = 0u32;
        let ok = unsafe { GetExitCodeProcess(h, &mut code) }.is_ok();
        let _ = unsafe { CloseHandle(h) };
        ok && code == 259 // STILL_ACTIVE
    }

    pub fn window_alive(w: isize) -> bool {
        unsafe { IsWindow(Some(HWND(w as *mut _))) }.as_bool()
    }

    struct Search {
        pid: u32,
        found: isize,
    }

    unsafe extern "system" fn each(h: HWND, l: LPARAM) -> BOOL {
        let s = unsafe { &mut *(l.0 as *mut Search) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        let visible = unsafe { IsWindowVisible(h) }.as_bool();
        let owned = unsafe { GetWindow(h, GW_OWNER) }.is_ok_and(|o| !o.is_invalid());
        let titled = unsafe { GetWindowTextLengthW(h) } > 0;
        if pid == s.pid && visible && !owned && titled {
            s.found = h.0 as isize;
            return BOOL(0);
        }
        BOOL(1)
    }

    pub fn top_window_of(pid: u32) -> Option<isize> {
        let mut s = Search { pid, found: 0 };
        let _ = unsafe { EnumWindows(Some(each), LPARAM(&mut s as *mut Search as isize)) };
        (s.found != 0).then_some(s.found)
    }

    struct Mains {
        pid: u32,
        best: isize,
        area: i64,
        iconic: isize,
    }

    unsafe extern "system" fn main_each(h: HWND, l: LPARAM) -> BOOL {
        let m = unsafe { &mut *(l.0 as *mut Mains) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        let owned = unsafe { GetWindow(h, GW_OWNER) }.is_ok_and(|o| !o.is_invalid());
        if pid != m.pid || owned || !unsafe { IsWindowVisible(h) }.as_bool() {
            return BOOL(1);
        }
        if unsafe { IsIconic(h) }.as_bool() {
            if m.iconic == 0 {
                m.iconic = h.0 as isize;
            }
            return BOOL(1);
        }
        let mut rc = RECT::default();
        if unsafe { GetWindowRect(h, &mut rc) }.is_ok() {
            let area = i64::from(rc.right - rc.left) * i64::from(rc.bottom - rc.top);
            if area > m.area {
                m.area = area;
                m.best = h.0 as isize;
            }
        }
        BOOL(1)
    }

    /// D-027: the process's main window, found again each time: its largest visible top-level
    /// window, else a minimized one.
    pub fn main_window_of(pid: u32) -> Option<isize> {
        let mut m = Mains {
            pid,
            best: 0,
            area: 0,
            iconic: 0,
        };
        let _ = unsafe { EnumWindows(Some(main_each), LPARAM(&mut m as *mut Mains as isize)) };
        if m.best != 0 {
            Some(m.best)
        } else if m.iconic != 0 {
            Some(m.iconic)
        } else {
            None
        }
    }

    /// The window's active pop-up (e.g. a "Quit?" question), if it has a visible one.
    pub fn active_popup(w: isize) -> Option<isize> {
        let h = HWND(w as *mut _);
        let p = unsafe { GetLastActivePopup(h) };
        (!p.is_invalid() && p != h && unsafe { IsWindowVisible(p) }.as_bool())
            .then_some(p.0 as isize)
    }

    /// D-027: the window's state for the log.
    pub fn window_state(w: isize) -> String {
        let h = HWND(w as *mut _);
        let mut buf = [0u16; 128];
        let n = unsafe { GetClassNameW(h, &mut buf) }.max(0) as usize;
        let class = String::from_utf16_lossy(&buf[..n]);
        let visible = unsafe { IsWindowVisible(h) }.as_bool();
        let minimized = unsafe { IsIconic(h) }.as_bool();
        let top_level = unsafe { GetAncestor(h, GA_ROOT) } == h;
        let owner = unsafe { GetWindow(h, GW_OWNER) }
            .ok()
            .filter(|o| !o.is_invalid())
            .map(|o| format!("{:#x}", o.0 as usize))
            .unwrap_or_else(|| "none".into());
        let mut rc = RECT::default();
        let _ = unsafe { GetWindowRect(h, &mut rc) };
        let (sw, sh) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
        let fullscreen = rc.left <= 0 && rc.top <= 0 && rc.right >= sw && rc.bottom >= sh;
        format!(
            "{:#x}: visible {visible}, minimized {minimized}, top-level {top_level}, class {class}, owner {owner}, fullscreen {fullscreen} ({}x{})",
            w as usize,
            rc.right - rc.left,
            rc.bottom - rc.top
        )
    }

    /// D-027: AIONE steps aside: the game's window to the top (not activated), AIONE minimized,
    /// so Windows activates the game. True when the game's process holds the front afterwards.
    pub fn step_aside(aione: isize, game: isize, pid: u32) -> bool {
        if aione == 0 {
            return false;
        }
        let g = HWND(game as *mut _);
        let _ = unsafe {
            SetWindowPos(
                g,
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        };
        let _ = unsafe { ShowWindow(HWND(aione as *mut _), SW_MINIMIZE) };
        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let fg = unsafe { GetForegroundWindow() };
            let mut fpid = 0u32;
            unsafe { GetWindowThreadProcessId(fg, Some(&mut fpid)) };
            if fpid == pid && pid != 0 {
                return true;
            }
        }
        false
    }

    struct Closer {
        pid: u32,
        sent: usize,
    }

    unsafe extern "system" fn close_each(h: HWND, l: LPARAM) -> BOOL {
        let c = unsafe { &mut *(l.0 as *mut Closer) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        if pid == c.pid
            && unsafe { IsWindowVisible(h) }.as_bool()
            && unsafe { PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0)) }.is_ok()
        {
            c.sent += 1;
        }
        BOOL(1)
    }

    /// WM_CLOSE to every visible top-level window of the process (as its ✕ would).
    pub fn close_windows_of(pid: u32) -> usize {
        let mut c = Closer { pid, sent: 0 };
        let _ = unsafe { EnumWindows(Some(close_each), LPARAM(&mut c as *mut Closer as isize)) };
        c.sent
    }

    pub fn foreground_program() -> Option<(isize, u32, String)> {
        let h = unsafe { GetForegroundWindow() };
        if h.is_invalid() {
            return None;
        }
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        let exe = exe_of(pid)?;
        Some((h.0 as isize, pid, exe))
    }
}

#[cfg(windows)]
use win::{
    close_windows_of, exe_of, foreground_program, process_alive, shell_open, top_window_of,
    window_alive,
};

#[cfg(not(windows))]
fn shell_open(_target: &str, _folder: &Path) -> Result<Option<u32>, String> {
    Err("starting games needs Windows".into())
}
#[cfg(not(windows))]
fn exe_of(_pid: u32) -> Option<String> {
    None
}
#[cfg(not(windows))]
fn process_alive(_pid: u32) -> bool {
    false
}
#[cfg(not(windows))]
fn window_alive(_w: isize) -> bool {
    false
}
#[cfg(not(windows))]
fn top_window_of(_pid: u32) -> Option<isize> {
    None
}
#[cfg(not(windows))]
fn foreground_program() -> Option<(isize, u32, String)> {
    None
}
#[cfg(not(windows))]
fn close_windows_of(_pid: u32) -> usize {
    0
}
#[cfg(not(windows))]
fn main_window_of(_pid: u32) -> Option<isize> {
    None
}
#[cfg(not(windows))]
fn active_popup(_w: isize) -> Option<isize> {
    None
}
#[cfg(not(windows))]
fn window_state(_w: isize) -> String {
    String::new()
}
#[cfg(not(windows))]
fn step_aside(_aione: isize, _game: isize, _pid: u32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::LaunchFields;
    use std::path::PathBuf;

    #[test]
    fn what_is_started() {
        let dir = tempfile::tempdir().unwrap();
        let lnk = dir.path().join("game.lnk");
        std::fs::write(&lnk, b"x").unwrap();
        let f = |shortcut: Option<PathBuf>, steam: Option<&str>| LaunchFields {
            name: "G".into(),
            folder: dir.path().into(),
            shortcut,
            steam_app_id: steam.map(str::to_string),
        };
        assert_eq!(
            target_of(&f(Some(lnk.clone()), Some("1"))).unwrap().1,
            "shortcut game.lnk"
        );
        assert_eq!(
            target_of(&f(None, Some("620"))).unwrap(),
            (
                "steam://rungameid/620".into(),
                "steam://rungameid/620".into()
            )
        );
        assert!(target_of(&f(Some(dir.path().join("gone.lnk")), None))
            .unwrap_err()
            .contains("gone.lnk is missing"));
        assert!(target_of(&f(None, None))
            .unwrap_err()
            .contains("no Shortcut and no SteamAppId"));
        assert!(
            is_not_the_game("SteamWebHelper.exe")
                && is_not_the_game("GameBar.exe")
                && !is_not_the_game("Balatro.exe")
        );
    }
}
