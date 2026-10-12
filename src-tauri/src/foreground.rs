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
//! The start-up guard (`start_guard`) listens for foreground changes (a WinEvent hook, no
//! polling) for a short time after start. If another program takes the foreground and the
//! user is not switching (Alt / Win / a mouse button not held), AIONE takes it back, at most
//! three times. Every event is logged.

use serde::Serialize;

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
pub use win::{bring_to_front, start_guard, stop_guard};

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
    use windows::Win32::Foundation::{CloseHandle, HWND};
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
        BringWindowToTop, FlashWindowEx, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
        SetForegroundWindow, SetWindowPos, ShowWindow, EVENT_SYSTEM_FOREGROUND, FLASHWINFO,
        FLASHW_ALL, FLASHW_TIMERNOFG, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
        SWP_SHOWWINDOW, SW_RESTORE, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
    };

    fn hwnd(raw: isize) -> HWND {
        HWND(raw as *mut c_void)
    }

    fn is_foreground(h: HWND) -> bool {
        // SAFETY: plain query.
        unsafe { GetForegroundWindow() == h }
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

    pub fn bring_to_front(raw: isize) -> ForegroundReport {
        let h = hwnd(raw);
        let mut attempts = Vec::new();
        // SAFETY: Win32 calls on our own window handle; results are checked.
        unsafe {
            let before = GetForegroundWindow();
            let previous = describe_window(before);
            let done = |method: &str, attempts: Vec<String>| ForegroundReport {
                ok: true,
                method: method.into(),
                attempts,
                previous: previous.clone(),
            };
            if IsIconic(h).as_bool() {
                let _ = ShowWindow(h, SW_RESTORE);
            }
            if before == h {
                return done("already", attempts);
            }

            // 1. direct
            let _ = SetForegroundWindow(h);
            if is_foreground(h) {
                return done("direct", attempts);
            }
            attempts.push("direct=denied".into());

            // 2. attach to the foreground thread's input queue
            let fg_thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
            let me = GetCurrentThreadId();
            if fg_thread != 0 && fg_thread != me {
                let attached = AttachThreadInput(me, fg_thread, true).as_bool();
                let _ = BringWindowToTop(h);
                let _ = SetForegroundWindow(h);
                if attached {
                    let _ = AttachThreadInput(me, fg_thread, false);
                }
                if is_foreground(h) {
                    return done("attach-thread-input", attempts);
                }
                attempts.push(format!("attach-thread-input=denied(attached={attached})"));
            } else {
                attempts.push("attach-thread-input=skipped".into());
            }

            // 3. one harmless input event, then try again
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
            let sent = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            let _ = SetForegroundWindow(h);
            if is_foreground(h) {
                return done("input-event", attempts);
            }
            attempts.push(format!("input-event=denied(sent={sent})"));

            // 4. topmost pulse: on and off again at once (never left topmost)
            let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW;
            let _ = SetWindowPos(h, Some(HWND_TOPMOST), 0, 0, 0, 0, flags);
            let _ = SetForegroundWindow(h);
            let _ = SetWindowPos(h, Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags);
            if is_foreground(h) {
                return done("topmost-pulse", attempts);
            }
            attempts.push("topmost-pulse=denied".into());

            let info = FLASHWINFO {
                cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
                hwnd: h,
                dwFlags: FLASHW_ALL | FLASHW_TIMERNOFG,
                uCount: 0,
                dwTimeout: 0,
            };
            let _ = FlashWindowEx(&info);
            attempts.push("taskbar flashed".into());
            ForegroundReport {
                ok: false,
                method: "denied".into(),
                attempts,
                previous,
            }
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
            if Instant::now() > g.until {
                drop(lock);
                stop_guard("time is up");
                return;
            }
            let mut pid = 0u32;
            // SAFETY: valid out pointer.
            unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
            // SAFETY: plain query.
            if pid == unsafe { GetCurrentProcessId() } {
                return;
            }
            let who = describe_window(window);
            let user = [VK_MENU.0, VK_LWIN.0, VK_RWIN.0, VK_LBUTTON.0, VK_RBUTTON.0]
                .into_iter()
                .any(key_down);
            if user {
                logging::lifecycle(
                    Level::Info,
                    &format!("foreground moved to {who} by the user (Alt / Win / mouse held)"),
                );
                drop(lock);
                stop_guard("the user switched away");
                return;
            }
            if g.reclaims >= MAX_RECLAIMS {
                logging::lifecycle(
                    Level::Warn,
                    &format!("foreground taken by {who}; limit reached, not taking it back"),
                );
                drop(lock);
                stop_guard("limit reached");
                return;
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
            let report = bring_to_front(own);
            logging::lifecycle(
                if report.ok { Level::Info } else { Level::Warn },
                &format!("foreground (guard): {}", report.summary()),
            );
        }
    }
}
