//! A real Windows test program for the core's Windows tests (A14.3: proven with real windows and
//! processes, never a simulation). Built by `cargo test` (examples are built with the tests);
//! never shipped.
//!
//! Modes (one per argument):
//! - `busy <ms>` (D-057): shows a titled window, tries to take the front, prints
//!   `hwnd=<n> front=<0|1>`, then stops answering messages for `<ms>` (like a game whose window
//!   thread is busy), then exits.
//! - `launcher` (D-058): a small launcher-like window (424×127, like Minecraft's) that starts this
//!   program again as `game`, prints `launcher=<hwnd> game_pid=<pid>`, hides its own window while
//!   the game runs, shows it again when the game ends, and exits about 60 s after starting.
//! - `game`: a large window (800×600) that answers messages and closes on WM_CLOSE (or after 60 s).

#[cfg(windows)]
fn main() {
    win::main();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("window_fixture runs on Windows only");
}

#[cfg(windows)]
mod win {
    use std::io::Write;
    use std::time::{Duration, Instant};
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
        PeekMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow, ShowWindow,
        TranslateMessage, CS_HREDRAW, CS_VREDRAW, MSG, PM_REMOVE, SW_HIDE, SW_SHOW,
        WINDOW_EX_STYLE, WM_CLOSE, WM_DESTROY, WM_QUIT, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    };

    unsafe extern "system" fn proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        match msg {
            WM_CLOSE => {
                let _ = unsafe { DestroyWindow(h) };
                LRESULT(0)
            }
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
        }
    }

    fn window(title: PCWSTR, width: i32, height: i32) -> HWND {
        // SAFETY: a plain window class and window of this test program.
        unsafe {
            let instance = GetModuleHandleW(None).expect("module handle");
            let class = w!("AIONE4WindowFixture");
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(proc),
                hInstance: instance.into(),
                lpszClassName: class,
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                title,
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                40,
                40,
                width,
                height,
                None,
                None,
                Some(instance.into()),
                None,
            )
            .expect("window")
        }
    }

    /// Answers messages for up to `for_how_long`; false once the window was closed.
    fn pump(for_how_long: Duration) -> bool {
        let until = Instant::now() + for_how_long;
        let mut msg = MSG::default();
        while Instant::now() < until {
            // SAFETY: the standard message loop of this thread.
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        return false;
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        true
    }

    fn say(line: &str) {
        let mut out = std::io::stdout();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }

    pub fn main() {
        let args: Vec<String> = std::env::args().skip(1).collect();
        match args.first().map(String::as_str) {
            Some("busy") => {
                let ms: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(10_000);
                let h = window(w!("AIONE4 fixture: busy game"), 640, 480);
                // SAFETY: plain calls on this program's own window.
                let front = unsafe {
                    let _ = SetForegroundWindow(h);
                    GetForegroundWindow() == h
                };
                pump(Duration::from_millis(300));
                say(&format!("hwnd={} front={}", h.0 as isize, u8::from(front)));
                // Busy: no messages answered (like a game's window thread at work).
                std::thread::sleep(Duration::from_millis(ms));
            }
            Some("launcher") => {
                let h = window(w!("AIONE4 fixture: launcher"), 424, 127);
                pump(Duration::from_millis(200));
                let exe = std::env::current_exe().expect("own path");
                let mut game = std::process::Command::new(exe)
                    .arg("game")
                    .spawn()
                    .expect("start the game");
                say(&format!("launcher={} game_pid={}", h.0 as isize, game.id()));
                // Like Minecraft's launcher: hidden while the game runs.
                // SAFETY: this program's own window.
                let _ = unsafe { ShowWindow(h, SW_HIDE) };
                let started = Instant::now();
                while started.elapsed() < Duration::from_secs(60) {
                    if game.try_wait().ok().flatten().is_some() {
                        // The game ended: the launcher's window comes back.
                        // SAFETY: this program's own window.
                        let _ = unsafe { ShowWindow(h, SW_SHOW) };
                        say("game_ended");
                        pump(Duration::from_secs(60).saturating_sub(started.elapsed()));
                        break;
                    }
                    if !pump(Duration::from_millis(50)) {
                        break;
                    }
                }
                let _ = game.kill();
                let _ = game.wait();
            }
            Some("game") => {
                let _h = window(w!("AIONE4 fixture: the game"), 800, 600);
                pump(Duration::from_secs(60));
            }
            _ => eprintln!("usage: window_fixture busy <ms> | launcher | game"),
        }
    }
}
