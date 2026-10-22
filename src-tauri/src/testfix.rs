//! Windows tests only (A14.3): the real test program `examples/window_fixture.rs` (built by
//! `cargo test`), started as a separate process, and a stand-in for AIONE's own window.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

/// The Windows window tests run one at a time (they share the desktop and the foreground).
pub fn one_at_a_time() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// The fixture program: `target/<profile>/examples/window_fixture.exe`, next to the test's `deps`.
fn fixture_path() -> std::path::PathBuf {
    let me = std::env::current_exe().expect("the test's own path");
    let profile = me
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>");
    profile.join("examples").join("window_fixture.exe")
}

/// One running fixture process; killed when dropped (never left running).
pub struct Fixture {
    child: Child,
    lines: Receiver<String>,
}

impl Fixture {
    pub fn start(args: &[&str]) -> Fixture {
        let path = fixture_path();
        assert!(
            path.exists(),
            "the fixture program is missing: {} (built by `cargo test` with the examples)",
            path.display()
        );
        let mut child = Command::new(&path)
            .args(args)
            .stdout(Stdio::piped())
            .spawn()
            .expect("start the fixture program");
        let out = child.stdout.take().expect("its output");
        let (tx, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Fixture { child, lines }
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// The next output line starting with `prefix` (waits up to 10 s).
    pub fn line(&self, prefix: &str) -> String {
        loop {
            let l = self
                .lines
                .recv_timeout(Duration::from_secs(10))
                .unwrap_or_else(|_| panic!("the fixture never printed \"{prefix}…\""));
            if l.starts_with(prefix) {
                return l;
            }
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `key=value` from a fixture line, as a number.
pub fn field(line: &str, key: &str) -> i64 {
    line.split_whitespace()
        .find_map(|kv| kv.strip_prefix(&format!("{key}=")))
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| panic!("no {key}= in \"{line}\""))
}

/// The stand-in window's procedure: Windows' default for every message.
unsafe extern "system" fn default_proc(
    h: windows::Win32::Foundation::HWND,
    msg: u32,
    wp: windows::Win32::Foundation::WPARAM,
    lp: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    // SAFETY: passes the message on unchanged.
    unsafe { windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(h, msg, wp, lp) }
}

/// A stand-in for AIONE's window: a visible window owned by the calling thread (which then has a
/// message queue, as AIONE's main thread has).
pub fn own_window() -> isize {
    use windows::core::w;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, RegisterClassW, WINDOW_EX_STYLE, WNDCLASSW, WS_OVERLAPPEDWINDOW,
        WS_VISIBLE,
    };
    // SAFETY: a plain window of the test process, with the default window procedure.
    unsafe {
        let instance = GetModuleHandleW(None).expect("module handle");
        let class = w!("AIONE4TestOwnWindow");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(default_proc),
            hInstance: instance.into(),
            lpszClassName: class,
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            w!("AIONE4 test: AIONE's stand-in"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            100,
            100,
            500,
            300,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .expect("window")
        .0 as isize
    }
}

/// Answers the calling thread's messages for a while.
pub fn pump(for_how_long: Duration) {
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
    };
    let until = std::time::Instant::now() + for_how_long;
    let mut msg = MSG::default();
    while std::time::Instant::now() < until {
        // SAFETY: the standard message loop of this thread.
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Destroys a window of the calling thread.
pub fn close(w: isize) {
    use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;
    // SAFETY: a window this thread created.
    let _ = unsafe { DestroyWindow(windows::Win32::Foundation::HWND(w as *mut _)) };
}

/// A GitHub notice line (shown as an annotation of the CI run): the measured evidence.
pub fn notice(title: &str, text: &str) {
    println!("::notice title={title}::{text}");
}
