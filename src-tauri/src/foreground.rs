//! Taking the foreground at start (L-20, the KD-120 lesson) and holding it briefly while
//! other start-up apps open windows. Never permanently topmost.
//!
//! Windows only lets a process take the foreground in some cases (the "foreground lock").
//! `bring_to_front` tries, in order, and stops at the first that works:
//! 1. `direct`: SetForegroundWindow.
//! 2. `attach-thread-input`: attach to the foreground window's input queue, BringWindowToTop,
//!    SetForegroundWindow, detach.
//! 3. `input-event`: send one zero-distance mouse move (so this process sent the last input
//!    event, which Windows accepts), then SetForegroundWindow.
//! 4. `topmost-pulse`: make the window topmost, SetForegroundWindow, and **immediately**
//!    remove topmost again (the window is never left topmost).
//!
//! If all fail, the taskbar button flashes and the log says so.
//!
//! **D-057 (alpha.36): never waits on a busy game.** Each step (the restore, and each method above)
//! runs on its own short-lived thread with a time limit (`STEP_LIMIT`, 300 ms); a step that doesn't
//! return in time is abandoned (logged) and the next method is tried. Before `attach-thread-input`,
//! the window in front must answer a harmless message within `RESPONDS_LIMIT` (100 ms); if it
//! doesn't (a busy game), that method is skipped. While an abandoned step is still stuck, no new
//! attach is tried. Callers run `bring_to_front` off the main thread (the thread that delivers input
//! to AIONE's page), so input never waits on it.
//!
//! The start-up guard (`start_guard`) listens for foreground changes (a WinEvent hook, no
//! polling) for a short time after start. If another program takes the foreground and the
//! user is not switching (Alt / Win / a mouse button not held), AIONE takes it back, at most
//! three times. Every event is logged.
//!
//! **Closing (T0 QA-5):** when exit begins (window close requested, or the app is asked to
//! exit), `closing()` stops the guard at once and marks AIONE as closing, so Windows giving the
//! foreground to another window *because AIONE's window closes* is never logged as "the user
//! switched away".
//!
//! **B-7:** `aione_is_foreground()` tells the input thread whether AIONE's main window is the
//! foreground window, so pad input in the background is logged but not acted on (except Guide).

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// D-057: the time limit for one step of bringing a window to the front.
pub const STEP_LIMIT: Duration = Duration::from_millis(300);
/// D-057: how long the window in front gets to answer before `attach-thread-input` is used.
pub const RESPONDS_LIMIT: Duration = Duration::from_millis(100);
/// D-057: steps abandoned after their time limit that haven't returned yet.
static ABANDONED: AtomicUsize = AtomicUsize::new(0);

/// D-057: how many abandoned steps are still stuck (no new attach while one is).
pub fn abandoned_steps() -> usize {
    ABANDONED.load(Ordering::SeqCst)
}

/// D-057: runs `step` on its own thread and waits at most `limit` for it. `Some(result)` when it
/// returned in time; `None` when it didn't (the step is abandoned: it keeps running on its thread,
/// is counted in `abandoned_steps()` until it returns, and its late return is logged) or when its
/// thread couldn't start. The caller never waits longer than `limit`.
pub fn run_limited<T: Send + 'static>(
    what: &'static str,
    limit: Duration,
    step: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    // (the result, abandoned)
    let shared = Arc::new((Mutex::new((None::<T>, false)), Condvar::new()));
    let theirs = Arc::clone(&shared);
    let t0 = Instant::now();
    let spawned = std::thread::Builder::new()
        .name("aione-foreground-step".into())
        .spawn(move || {
            let out = step();
            let (lock, cv) = &*theirs;
            if let Ok(mut g) = lock.lock() {
                if g.1 {
                    // Abandoned earlier: nobody waits for it any more.
                    ABANDONED.fetch_sub(1, Ordering::SeqCst);
                    crate::logging::lifecycle(
                        crate::logging::Level::Info,
                        &format!(
                            "foreground: the abandoned {what} step returned after {} ms",
                            t0.elapsed().as_millis()
                        ),
                    );
                } else {
                    g.0 = Some(out);
                    cv.notify_all();
                }
            }
        });
    if spawned.is_err() {
        crate::logging::lifecycle(
            crate::logging::Level::Warn,
            &format!("foreground: the {what} step couldn't start a thread: skipped"),
        );
        return None;
    }
    let (lock, cv) = &*shared;
    let guard = lock.lock().ok()?;
    let (mut g, _) = cv
        .wait_timeout_while(guard, limit, |g| g.0.is_none())
        .ok()?;
    match g.0.take() {
        Some(out) => Some(out),
        None => {
            // Under the lock: the step can't finish unnoticed in between.
            g.1 = true;
            ABANDONED.fetch_add(1, Ordering::SeqCst);
            None
        }
    }
}

/// AIONE's main window handle (0 until the window exists).
static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);
/// Set when exit begins (QA-5).
static CLOSING: AtomicBool = AtomicBool::new(false);

pub fn set_main_window(raw: isize) {
    MAIN_HWND.store(raw, Ordering::SeqCst);
}

pub fn main_window() -> isize {
    MAIN_HWND.load(Ordering::SeqCst)
}

pub fn is_closing() -> bool {
    CLOSING.load(Ordering::SeqCst)
}

/// Exit begins: stop the guard (with that reason) and ignore later foreground changes.
/// Must be called on the main thread (where the guard's hook lives). Idempotent.
pub fn closing(reason: &str) {
    if !CLOSING.swap(true, Ordering::SeqCst) {
        stop_guard(reason);
    }
}

/// What the guard does with one foreground change to another program (pure, tested).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardDecision {
    /// AIONE is closing: ignore (QA-5); the guard is already off.
    IgnoreClosing,
    /// The guard's time is over: switch it off.
    TimeUp,
    /// Alt / Win / a mouse button is held: the user switched; switch the guard off.
    UserSwitched,
    /// Taken back the maximum number of times: switch the guard off.
    LimitReached,
    /// Take the foreground back.
    TakeBack,
}

pub fn guard_decision(
    closing: bool,
    expired: bool,
    user_keys_held: bool,
    reclaims: u32,
    max_reclaims: u32,
) -> GuardDecision {
    if closing {
        GuardDecision::IgnoreClosing
    } else if expired {
        GuardDecision::TimeUp
    } else if user_keys_held {
        GuardDecision::UserSwitched
    } else if reclaims >= max_reclaims {
        GuardDecision::LimitReached
    } else {
        GuardDecision::TakeBack
    }
}

/// B-7: whether AIONE's main window is the foreground window. True when unknown (no window
/// yet, or not Windows), so nothing is dropped by mistake before the window exists.
pub fn aione_is_foreground() -> bool {
    let own = main_window();
    if own == 0 {
        return true;
    }
    #[cfg(windows)]
    {
        win::is_foreground_raw(own)
    }
    #[cfg(not(windows))]
    {
        true
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ForegroundReport {
    pub ok: bool,
    /// The step that worked ("already", "direct", …) or "denied".
    pub method: String,
    /// Each step tried, with its result.
    pub attempts: Vec<String>,
    /// The program that had the foreground before (exe name and pid), if known.
    pub previous: String,
}

impl ForegroundReport {
    pub fn summary(&self) -> String {
        format!(
            "{} via {} (before: {}; tried: {})",
            if self.ok { "OK" } else { "DENIED" },
            self.method,
            if self.previous.is_empty() {
                "unknown"
            } else {
                &self.previous
            },
            if self.attempts.is_empty() {
                "-".to_string()
            } else {
                self.attempts.join(", ")
            }
        )
    }
}

#[cfg(windows)]
pub use win::{bring_to_front, focus_report, start_guard, stop_guard};

#[cfg(not(windows))]
pub fn focus_report(_own: isize) -> String {
    "not available on this platform".into()
}

/// D-005: the wording of the keyboard-focus line. `inside_aione`: the focus window's top-level
/// ancestor is AIONE's main window (judged by window ownership, not by process: WebView2's
/// window belongs to its own `msedgewebview2.exe` process).
pub fn focus_wording(inside_aione: bool, class: &str, program: &str) -> String {
    let web_view =
        class.starts_with("Chrome_") || program.eq_ignore_ascii_case("msedgewebview2.exe");
    match (inside_aione, web_view) {
        (true, true) => format!("AIONE 4's web view: keys reach AIONE (window class \"{class}\" in {program})"),
        (true, false) => format!("AIONE 4's window: keys reach AIONE (window class \"{class}\" in {program})"),
        (false, _) => format!("another program has the keys: {program}, window class \"{class}\" (keys do NOT reach AIONE)"),
    }
}

#[cfg(not(windows))]
pub fn bring_to_front(_hwnd: isize) -> ForegroundReport {
    ForegroundReport {
        ok: false,
        method: "unsupported".into(),
        attempts: vec!["not Windows".into()],
        previous: String::new(),
    }
}
#[cfg(not(windows))]
pub fn start_guard(_hwnd: isize, _seconds: u64) {}
#[cfg(not(windows))]
pub fn stop_guard(_reason: &str) {}

#[cfg(windows)]
mod win {
    use super::ForegroundReport;
    use crate::logging::{self, Level};
    use std::ffi::c_void;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
    use windows::Win32::System::Threading::{
        AttachThreadInput, GetCurrentProcessId, GetCurrentThreadId, OpenProcess,
        QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT,
        VK_LBUTTON, VK_LWIN, VK_MENU, VK_RBUTTON, VK_RWIN,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, FlashWindowEx, GetAncestor, GetClassNameW, GetForegroundWindow,
        GetGUIThreadInfo, GetWindowThreadProcessId, IsIconic, PeekMessageW, SendMessageTimeoutW,
        SetForegroundWindow, SetWindowPos, ShowWindow, EVENT_SYSTEM_FOREGROUND, FLASHWINFO,
        FLASHW_ALL, FLASHW_TIMERNOFG, GA_ROOT, GUITHREADINFO, HWND_NOTOPMOST, HWND_TOPMOST, MSG,
        PM_NOREMOVE, SMTO_ABORTIFHUNG, SMTO_ERRORONEXIT, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
        SW_RESTORE, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_NULL,
    };

    fn hwnd(raw: isize) -> HWND {
        HWND(raw as *mut c_void)
    }

    fn is_foreground(h: HWND) -> bool {
        // SAFETY: plain query.
        unsafe { GetForegroundWindow() == h }
    }

    /// B-7: the foreground window is AIONE's main window (top-level windows only; the web
    /// view is a child of it).
    pub fn is_foreground_raw(own: isize) -> bool {
        is_foreground(hwnd(own))
    }

    /// "explorer.exe (pid 1234)" for the window's process, or "none".
    pub(super) fn describe_window(h: HWND) -> String {
        if h.0.is_null() {
            return "none".into();
        }
        let mut pid = 0u32;
        // SAFETY: valid out pointer.
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        format!("{} (pid {pid})", process_name(pid))
    }

    fn process_name(pid: u32) -> String {
        // SAFETY: handle is checked and closed; buffer length is passed in.
        unsafe {
            let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return "unknown".into();
            };
            let mut buf = [0u16; 520];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .is_ok();
            let _ = CloseHandle(handle);
            if !ok {
                return "unknown".into();
            }
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            path.rsplit('\\').next().unwrap_or(&path).to_string()
        }
    }

    /// Which window has keyboard focus right now (the foreground thread's focus window), and
    /// whether it is inside AIONE's main window `own` (D-005: by window ownership).
    pub fn focus_report(own: isize) -> String {
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: cbSize is set; 0 = the foreground thread.
        if unsafe { GetGUIThreadInfo(0, &mut info) }.is_err() {
            return "unknown (GetGUIThreadInfo failed)".into();
        }
        let h = info.hwndFocus;
        if h.0.is_null() {
            return format!(
                "no window has keyboard focus (the active window is {})",
                describe_window(info.hwndActive)
            );
        }
        let mut buf = [0u16; 128];
        // SAFETY: buffer slice passed with its length.
        let n = unsafe { GetClassNameW(h, &mut buf) }.max(0) as usize;
        let class = String::from_utf16_lossy(&buf[..n]);
        // SAFETY: plain query; GA_ROOT walks up the parent chain to the top-level window.
        let root = unsafe { GetAncestor(h, GA_ROOT) };
        let inside_aione = !root.0.is_null() && root == hwnd(own);
        let mut pid = 0u32;
        // SAFETY: valid out pointer.
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        let program = format!("{} (pid {pid})", process_name(pid));
        super::focus_wording(inside_aione, &class, &program)
    }

    /// D-057: whether the window answers a harmless message (WM_NULL) within `limit`; a busy or
    /// hung window's thread doesn't, and attaching to it would wait on it.
    pub fn responds(raw: isize, limit: Duration) -> bool {
        let mut result = 0usize;
        // SAFETY: WM_NULL does nothing; SMTO_ABORTIFHUNG returns at once for a hung window and
        // the time limit bounds the wait otherwise.
        let r = unsafe {
            SendMessageTimeoutW(
                hwnd(raw),
                WM_NULL,
                WPARAM(0),
                LPARAM(0),
                SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT,
                limit.as_millis() as u32,
                Some(&mut result as *mut usize),
            )
        };
        r.0 != 0
    }

    /// Makes sure the calling thread has a message queue (AttachThreadInput needs one).
    fn ensure_queue() {
        let mut msg = MSG::default();
        // SAFETY: peeks without removing anything; creates this thread's queue.
        let _ = unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE) };
    }

    /// One step under its time limit; its line in the attempts, and whether AIONE is in front.
    fn limited(
        attempts: &mut Vec<String>,
        name: &'static str,
        target: isize,
        step: impl FnOnce() -> String + Send + 'static,
    ) -> bool {
        let t0 = Instant::now();
        match super::run_limited(name, super::STEP_LIMIT, step) {
            Some(note) => {
                if is_foreground(hwnd(target)) {
                    return true;
                }
                attempts.push(note);
                false
            }
            None => {
                attempts.push(format!(
                    "{name}=abandoned after {} ms (didn't return in time)",
                    t0.elapsed().as_millis()
                ));
                is_foreground(hwnd(target))
            }
        }
    }

    pub fn bring_to_front(raw: isize) -> ForegroundReport {
        bring_to_front_with(raw, false)
    }

    /// `skip_direct`: tests start at the second method, as on a PC where Windows refuses the
    /// direct way (D-057's case).
    pub(super) fn bring_to_front_with(raw: isize, skip_direct: bool) -> ForegroundReport {
        let h = hwnd(raw);
        let mut attempts = Vec::new();
        // SAFETY: plain queries.
        let before = unsafe { GetForegroundWindow() };
        let previous = describe_window(before);
        let done = |method: &str, attempts: Vec<String>| ForegroundReport {
            ok: true,
            method: method.into(),
            attempts,
            previous: previous.clone(),
        };
        // D-038 (rev. ak): a minimized window is restored first, and "already" never counts
        // while it is minimized (Windows can keep a minimized window as the active one).
        // SAFETY: plain query.
        let was_minimized = unsafe { IsIconic(h) }.as_bool();
        if was_minimized {
            // D-057: restoring a window of a busy program could wait on it: time-limited.
            let restored = super::run_limited("restore", super::STEP_LIMIT, move || {
                // SAFETY: the window handle is only passed through.
                unsafe { ShowWindow(hwnd(raw), SW_RESTORE) }.as_bool()
            });
            attempts.push(match restored {
                Some(_) => "minimized: true → restored".to_string(),
                None => "minimized: true → restore abandoned after 300 ms".to_string(),
            });
        } else {
            attempts.push("minimized: false".to_string());
        }
        if before == h && !was_minimized {
            return done("already", attempts);
        }

        // 1. direct
        if !skip_direct
            && limited(&mut attempts, "direct", raw, move || {
                // SAFETY: Win32 call on a window handle; the result is checked by the caller.
                let _ = unsafe { SetForegroundWindow(hwnd(raw)) };
                "direct=denied".into()
            })
        {
            return done("direct", attempts);
        }

        // 2. attach to the foreground thread's input queue
        // SAFETY: plain queries.
        let fg = unsafe { GetForegroundWindow() };
        let mut fg_pid = 0u32;
        let fg_thread = unsafe { GetWindowThreadProcessId(fg, Some(&mut fg_pid)) };
        let fg_raw = fg.0 as isize;
        let own_pid = unsafe { GetCurrentProcessId() };
        // D-039 (rev. al): never attach to (and so wait on) a suspended thread.
        if crate::pause::is_paused(fg_pid) {
            attempts.push("attach-thread-input=skipped (the thread in front is paused)".into());
        } else if crate::launch::game_is_young(fg_pid) {
            // D-043: a game in its first seconds: its input thread is busy; don't wait on it.
            attempts
                .push("attach-thread-input=skipped (the game started less than 3 s ago)".into());
        } else if super::abandoned_steps() > 0 {
            // D-057: an earlier step is still stuck: never stack another wait on top of it.
            attempts.push(
                "attach-thread-input=skipped (an earlier step is still waiting on a busy window)"
                    .into(),
            );
        } else if fg_thread == 0 || fg_pid == own_pid {
            attempts.push("attach-thread-input=skipped".into());
        } else if !responds(fg_raw, super::RESPONDS_LIMIT) {
            // D-057: the window in front is busy (didn't answer in 100 ms): attaching would wait.
            attempts.push(format!(
                "attach-thread-input=skipped (the window in front didn't answer within {} ms)",
                super::RESPONDS_LIMIT.as_millis()
            ));
        } else if limited(&mut attempts, "attach-thread-input", raw, move || {
            ensure_queue();
            // SAFETY: attach / detach this step's own thread; the handle is only passed through.
            unsafe {
                let me = GetCurrentThreadId();
                let attached = AttachThreadInput(me, fg_thread, true).as_bool();
                let _ = BringWindowToTop(hwnd(raw));
                let _ = SetForegroundWindow(hwnd(raw));
                if attached {
                    let _ = AttachThreadInput(me, fg_thread, false);
                }
                format!("attach-thread-input=denied(attached={attached})")
            }
        }) {
            return done("attach-thread-input", attempts);
        }

        // 3. one harmless input event, then try again
        if limited(&mut attempts, "input-event", raw, move || {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: MOUSEEVENTF_MOVE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            // SAFETY: one well-formed INPUT with its size.
            let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
            // D-047 (B-22 rev. ap): AIONE's only synthetic input (never a key; never a press of yours).
            crate::logging::lifecycle(
                crate::logging::Level::Info,
                "foreground: synthetic input sent (a zero-distance mouse move)",
            );
            // SAFETY: the window handle is only passed through.
            let _ = unsafe { SetForegroundWindow(hwnd(raw)) };
            format!("input-event=denied(sent={sent})")
        }) {
            return done("input-event", attempts);
        }

        // 4. topmost pulse: on and off again at once (never left topmost)
        if limited(&mut attempts, "topmost-pulse", raw, move || {
            let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW;
            // SAFETY: the window handle is only passed through; topmost is removed at once.
            unsafe {
                let _ = SetWindowPos(hwnd(raw), Some(HWND_TOPMOST), 0, 0, 0, 0, flags);
                let _ = SetForegroundWindow(hwnd(raw));
                let _ = SetWindowPos(hwnd(raw), Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags);
            }
            "topmost-pulse=denied".into()
        }) {
            return done("topmost-pulse", attempts);
        }

        let info = FLASHWINFO {
            cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
            hwnd: h,
            dwFlags: FLASHW_ALL | FLASHW_TIMERNOFG,
            uCount: 0,
            dwTimeout: 0,
        };
        // SAFETY: a well-formed FLASHWINFO.
        let _ = unsafe { FlashWindowEx(&info) };
        attempts.push("taskbar flashed".into());
        ForegroundReport {
            ok: false,
            method: "denied".into(),
            attempts,
            previous,
        }
    }

    struct Guard {
        own: isize,
        hook: isize,
        until: Instant,
        reclaims: u32,
    }

    // Only touched on the main thread (the hook's thread); the Mutex keeps it sound anyway.
    static GUARD: Mutex<Option<Guard>> = Mutex::new(None);
    const MAX_RECLAIMS: u32 = 3;

    /// Must be called on the main (UI) thread: WinEvent callbacks arrive on the installing
    /// thread's message loop.
    pub fn start_guard(own: isize, seconds: u64) {
        if super::is_closing() {
            return;
        }
        // SAFETY: out-of-context hook with a static callback; unhooked in stop_guard.
        let hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(on_foreground),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        };
        if hook.0.is_null() {
            logging::lifecycle(
                Level::Warn,
                "foreground guard could not be installed (SetWinEventHook failed)",
            );
            return;
        }
        if let Ok(mut g) = GUARD.lock() {
            *g = Some(Guard {
                own,
                hook: hook.0 as isize,
                until: Instant::now() + Duration::from_secs(seconds),
                reclaims: 0,
            });
        }
        logging::lifecycle(
            Level::Info,
            &format!("foreground guard on for {seconds} s (takes the foreground back from start-up apps, at most {MAX_RECLAIMS} times)"),
        );
    }

    pub fn stop_guard(reason: &str) {
        let taken = GUARD.lock().ok().and_then(|mut g| g.take());
        if let Some(g) = taken {
            // SAFETY: the handle came from SetWinEventHook.
            unsafe {
                let _ = UnhookWinEvent(HWINEVENTHOOK(g.hook as *mut c_void));
            }
            logging::lifecycle(
                Level::Info,
                &format!(
                    "foreground guard off ({reason}); taken back {} time(s)",
                    g.reclaims
                ),
            );
        }
    }

    fn key_down(vk: u16) -> bool {
        // SAFETY: plain query.
        unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
    }

    unsafe extern "system" fn on_foreground(
        _hook: HWINEVENTHOOK,
        _event: u32,
        window: HWND,
        _object: i32,
        _child: i32,
        _thread: u32,
        _time: u32,
    ) {
        let (own, reclaim) = {
            let Ok(mut lock) = GUARD.lock() else { return };
            let Some(g) = lock.as_mut() else { return };
            let mut pid = 0u32;
            // SAFETY: valid out pointer.
            unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
            // SAFETY: plain query.
            if pid == unsafe { GetCurrentProcessId() } {
                return;
            }
            let user = [VK_MENU.0, VK_LWIN.0, VK_RWIN.0, VK_LBUTTON.0, VK_RBUTTON.0]
                .into_iter()
                .any(key_down);
            let decision = super::guard_decision(
                super::is_closing(),
                Instant::now() > g.until,
                user,
                g.reclaims,
                MAX_RECLAIMS,
            );
            let who = describe_window(window);
            match decision {
                // QA-5: AIONE's own window is closing; Windows hands the foreground on.
                super::GuardDecision::IgnoreClosing => return,
                super::GuardDecision::TimeUp => {
                    drop(lock);
                    stop_guard("time is up");
                    return;
                }
                super::GuardDecision::UserSwitched => {
                    logging::lifecycle(
                        Level::Info,
                        &format!("foreground moved to {who} by the user (Alt / Win / mouse held)"),
                    );
                    drop(lock);
                    stop_guard("the user switched away");
                    return;
                }
                super::GuardDecision::LimitReached => {
                    logging::lifecycle(
                        Level::Warn,
                        &format!("foreground taken by {who}; limit reached, not taking it back"),
                    );
                    drop(lock);
                    stop_guard("limit reached");
                    return;
                }
                super::GuardDecision::TakeBack => {}
            }
            g.reclaims += 1;
            logging::lifecycle(
                Level::Info,
                &format!(
                    "foreground taken by {who} during start-up; taking it back ({}/{MAX_RECLAIMS})",
                    g.reclaims
                ),
            );
            (g.own, true)
        };
        if reclaim {
            // D-057: this callback runs on the main thread: the attempt runs on its own thread.
            std::thread::spawn(move || {
                let report = bring_to_front(own);
                logging::lifecycle(
                    if report.ok { Level::Info } else { Level::Warn },
                    &format!("foreground (guard): {}", report.summary()),
                );
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{focus_wording, guard_decision, GuardDecision};

    /// T0 QA-5: once exit has begun, a foreground change is never read as the user switching
    /// away (Alt is still held from Alt+F4 when Windows hands the foreground on).
    #[test]
    fn closing_wins_over_every_other_reason() {
        assert_eq!(
            guard_decision(true, false, true, 0, 3),
            GuardDecision::IgnoreClosing
        );
        assert_eq!(
            guard_decision(true, true, false, 3, 3),
            GuardDecision::IgnoreClosing
        );
        assert_eq!(
            guard_decision(false, false, true, 0, 3),
            GuardDecision::UserSwitched
        );
        assert_eq!(
            guard_decision(false, true, true, 0, 3),
            GuardDecision::TimeUp
        );
        assert_eq!(
            guard_decision(false, false, false, 3, 3),
            GuardDecision::LimitReached
        );
        assert_eq!(
            guard_decision(false, false, false, 1, 3),
            GuardDecision::TakeBack
        );
    }

    #[test]
    fn closing_is_idempotent_and_sticky() {
        assert!(!super::is_closing());
        super::closing("AIONE is closing");
        super::closing("AIONE is closing");
        assert!(super::is_closing());
    }

    #[test]
    fn foreground_is_assumed_until_the_window_exists() {
        // No main window yet: nothing is dropped as "background".
        assert_eq!(super::main_window(), 0);
        assert!(super::aione_is_foreground());
    }

    #[test]
    fn web_view_inside_aione_reaches_aione() {
        let l = focus_wording(true, "Chrome_WidgetWin_1", "msedgewebview2.exe (pid 4280)");
        assert!(l.starts_with("AIONE 4's web view: keys reach AIONE"), "{l}");
    }

    #[test]
    fn another_app_in_front_is_named() {
        let l = focus_wording(false, "Notepad", "notepad.exe (pid 99)");
        assert_eq!(l, "another program has the keys: notepad.exe (pid 99), window class \"Notepad\" (keys do NOT reach AIONE)");
        // Another app's web view is not AIONE's either.
        assert!(
            focus_wording(false, "Chrome_WidgetWin_1", "msedge.exe (pid 7)")
                .starts_with("another program")
        );
    }

    /// D-057 (A14.2, the reported cause): a busy game window in front (its thread answers nothing
    /// for 8 s, like Minecraft at work) and Windows refusing the direct way: bringing AIONE to the
    /// front must come back within about 2 s, never wait for the game. Real windows and processes
    /// (`examples/window_fixture.rs busy`); AIONE's stand-in window lives on a thread with a message
    /// queue, as AIONE's main thread.
    #[cfg(windows)]
    #[test]
    fn d057_bringing_aione_forward_never_waits_on_a_busy_game() {
        use std::time::{Duration, Instant};
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};
        let _one = crate::testfix::one_at_a_time();
        let _limit = limit_lock();
        let busy = crate::testfix::Fixture::start(&["busy", "8000"]);
        let line = busy.line("hwnd=");
        let game = crate::testfix::field(&line, "hwnd") as isize;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let own = crate::testfix::own_window();
            crate::testfix::pump(Duration::from_millis(100));
            // The busy game is the window in front (as Minecraft was).
            // SAFETY: plain calls on window handles.
            let in_front = unsafe {
                let _ = SetForegroundWindow(HWND(game as *mut _));
                GetForegroundWindow().0 as isize == game
            };
            let t0 = Instant::now();
            let report = super::win::bring_to_front_with(own, true);
            let took = t0.elapsed();
            let _ = tx.send((took, report.summary(), in_front));
            crate::testfix::pump(Duration::from_millis(100));
            crate::testfix::close(own);
        });
        let (took, summary, in_front) = rx
            .recv_timeout(Duration::from_secs(30))
            .expect("bringing AIONE forward never came back (30 s)");
        crate::testfix::notice(
            "D-057 proof",
            &format!(
                "busy game in front: {in_front}; bringing AIONE forward took {} ms: {summary}",
                took.as_millis()
            ),
        );
        assert!(
            took < Duration::from_millis(2500),
            "bringing AIONE forward waited {} ms on the busy game (in front: {in_front}): {summary}",
            took.as_millis()
        );
    }

    /// The step-limit tests share the abandoned-step counter: one at a time.
    fn limit_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // A step abandoned by an earlier test may still be returning: start from a settled count.
        let t0 = std::time::Instant::now();
        while super::abandoned_steps() > 0 && t0.elapsed() < std::time::Duration::from_secs(10) {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        guard
    }

    /// D-057: a step that returns in time gives its result.
    #[test]
    fn a_quick_step_gives_its_result() {
        let _one = limit_lock();
        assert_eq!(super::run_limited("test", super::STEP_LIMIT, || 7), Some(7));
    }

    /// D-057: a step that doesn't return in time is abandoned at its limit (the caller never
    /// waits longer), counted while it's stuck, and uncounted when it finally returns.
    #[test]
    fn a_slow_step_is_abandoned_at_its_limit() {
        use std::time::{Duration, Instant};
        let _one = limit_lock();
        let before = super::abandoned_steps();
        let t0 = Instant::now();
        let out = super::run_limited("test", Duration::from_millis(200), || {
            std::thread::sleep(Duration::from_millis(1200));
            1
        });
        let waited = t0.elapsed();
        assert_eq!(out, None);
        assert!(waited >= Duration::from_millis(200), "gave up too early");
        assert!(waited < Duration::from_millis(700), "waited {waited:?}");
        assert_eq!(super::abandoned_steps(), before + 1);
        let t1 = Instant::now();
        while super::abandoned_steps() > before && t1.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(
            super::abandoned_steps(),
            before,
            "still counted after it returned"
        );
    }

    /// D-057, the real mechanism: a busy window (another process whose window thread answers
    /// nothing) is seen as not answering within the 100 ms limit; a window whose thread answers
    /// is seen as answering.
    #[cfg(windows)]
    #[test]
    fn d057_a_busy_window_is_seen_as_not_answering() {
        use std::time::{Duration, Instant};
        let _one = crate::testfix::one_at_a_time();
        let busy = crate::testfix::Fixture::start(&["busy", "6000"]);
        let game = crate::testfix::field(&busy.line("hwnd="), "hwnd") as isize;
        let t0 = Instant::now();
        let answered = super::win::responds(game, super::RESPONDS_LIMIT);
        let took = t0.elapsed();
        // A window whose thread keeps answering (AIONE's stand-in on its own thread).
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let own = crate::testfix::own_window();
            let _ = tx.send(own);
            crate::testfix::pump(Duration::from_secs(2));
            crate::testfix::close(own);
        });
        let own = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("stand-in window");
        let own_answered = super::win::responds(own, super::RESPONDS_LIMIT);
        crate::testfix::notice(
            "D-057 proof (the answer check)",
            &format!(
                "busy window answered: {answered} (checked in {} ms); answering window answered: {own_answered}",
                took.as_millis()
            ),
        );
        assert!(!answered, "the busy window was taken as answering");
        assert!(took < Duration::from_millis(500), "the check took {took:?}");
        assert!(own_answered, "a window that answers was taken as busy");
    }

    /// D-057, the real mechanism: a step stuck in a call that waits on a busy window (a sent
    /// message, as the attach waits) is abandoned at its 300 ms limit; it's counted until the
    /// busy program goes away and the call returns.
    #[cfg(windows)]
    #[test]
    fn d057_a_step_stuck_on_a_busy_window_is_abandoned_in_time() {
        use std::time::{Duration, Instant};
        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_NULL};
        let _one = crate::testfix::one_at_a_time();
        let _limit = limit_lock();
        let before = super::abandoned_steps();
        let busy = crate::testfix::Fixture::start(&["busy", "6000"]);
        let game = crate::testfix::field(&busy.line("hwnd="), "hwnd") as isize;
        let t0 = Instant::now();
        let out = super::run_limited("test", super::STEP_LIMIT, move || {
            // SAFETY: WM_NULL does nothing; this waits until the busy window answers.
            unsafe {
                SendMessageW(
                    HWND(game as *mut _),
                    WM_NULL,
                    Some(WPARAM(0)),
                    Some(LPARAM(0)),
                )
            };
        });
        let waited = t0.elapsed();
        crate::testfix::notice(
            "D-057 proof (the step limit)",
            &format!(
                "a step waiting on the busy window was abandoned after {} ms",
                waited.as_millis()
            ),
        );
        assert!(
            out.is_none(),
            "the step returned although the window was busy"
        );
        assert!(waited < Duration::from_millis(600), "waited {waited:?}");
        assert_eq!(super::abandoned_steps(), before + 1);
        drop(busy); // the busy program ends: the stuck call returns
        let t1 = Instant::now();
        while super::abandoned_steps() > before && t1.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(super::abandoned_steps(), before);
    }

    #[test]
    fn aione_own_window_without_web_view() {
        assert!(focus_wording(true, "Tauri Window", "AIONE4.exe (pid 1)")
            .starts_with("AIONE 4's window: keys reach AIONE"));
    }
}
