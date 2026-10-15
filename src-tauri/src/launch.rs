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
    id: String,
    name: String,
    pid: Option<u32>,
    exe: Option<String>,
    window: Option<isize>,
    started: Instant,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

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

/// Starts the game (or brings it to the front if it is already running). `on_exit` is called
/// once, from the watcher thread, when the game's process ends.
pub fn launch(
    data: &Path,
    id: &str,
    on_exit: impl Fn(String, String, u64) + Send + 'static,
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
    let run = Running {
        id: id.into(),
        name: f.name.clone(),
        pid,
        exe: pid.and_then(exe_of),
        window: None,
        started: Instant::now(),
    };
    if let Ok(mut g) = RUNNING.lock() {
        *g = Some(run);
    }
    std::thread::spawn(move || watch(on_exit));
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
    let w = r.window.ok_or("its window isn't known yet")?;
    let report = crate::foreground::bring_to_front(w);
    logging::lifecycle(
        Level::Info,
        &format!("back to \"{}\": {}", r.name, report.summary()),
    );
    if report.ok {
        Ok(report.summary())
    } else {
        Err(report.summary())
    }
}

fn alive(r: &Running) -> bool {
    match r.pid {
        Some(p) => process_alive(p),
        None => r.window.is_some_and(window_alive),
    }
}

/// The watcher: finds the game's window, follows a launcher to the real game, and reports the
/// game's exit. Ends when the game ends (or after 2 minutes without ever finding a game).
fn watch(on_exit: impl Fn(String, String, u64)) {
    let mut found_logged = false;
    loop {
        std::thread::sleep(Duration::from_millis(250));
        let Some(mut r) = RUNNING.lock().ok().and_then(|g| g.clone()) else {
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
            }
            if r.window.is_none() && since > Duration::from_secs(120) {
                logging::lifecycle(
                    Level::Warn,
                    &format!(
                        "start \"{}\": no game window within 2 minutes; no longer watching",
                        r.name
                    ),
                );
                clear_if(&r.id);
                on_exit(r.id.clone(), r.name.clone(), 0);
                return;
            }
        }
        // 2. The game's exit.
        if r.window.is_some() && !alive(&r) {
            let minutes = since.as_secs() / 60;
            logging::lifecycle(
                Level::Info,
                &format!("\"{}\" exited after {} min", r.name, minutes),
            );
            clear_if(&r.id);
            on_exit(r.id.clone(), r.name.clone(), minutes);
            return;
        }
        if let Ok(mut g) = RUNNING.lock() {
            if g.as_ref().is_some_and(|x| x.id == r.id) {
                *g = Some(r);
            }
        }
    }
}

fn clear_if(id: &str) {
    if let Ok(mut g) = RUNNING.lock() {
        if g.as_ref().is_some_and(|x| x.id == id) {
            *g = None;
        }
    }
}

// ---------- the platform layer ----------

#[cfg(windows)]
mod win {
    use windows::core::{BOOL, HSTRING, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, GetProcessId, OpenProcess, QueryFullProcessImageNameW,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindow, GetWindowTextLengthW,
        GetWindowThreadProcessId, IsWindow, IsWindowVisible, GW_OWNER, SW_SHOWNORMAL,
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
use win::{exe_of, foreground_program, process_alive, shell_open, top_window_of, window_alive};

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
