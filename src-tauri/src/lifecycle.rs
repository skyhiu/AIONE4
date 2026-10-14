//! Start-up arguments, restart, and the Start with Windows registration.
//!
//! Restart (A5): the running instance starts the new one with `--wait-for-pid <its pid>` and
//! exits. The new instance waits for that process to end **before** it claims the single
//! instance, opens its window or plays anything (at most 15 s, logged), so two instances never
//! run at the same time.
//!
//! Start with Windows: `AIONE4.exe --autostart=on|off` writes / removes the value "AIONE 4" in
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` and exits (T0 has no Settings yet; the
//! setting comes with Settings in T3). The entry runs `"<exe>" --from-autostart`, so a start at
//! Windows sign-in is marked in the log on its own (T0 QA-4; L-20 in T9).
//!
//! `--interface-scale=90..150` (steps of 10) sets Interface Scale for this run only, until the
//! setting exists (T3). It is never stored.

use crate::logging::{self, Level};
use std::path::PathBuf;
use std::time::Duration;

pub const WAIT_TIMEOUT: Duration = Duration::from_secs(15);
pub const RUN_VALUE_NAME: &str = "AIONE 4";
/// Passed by the Start with Windows entry (T0 QA-4).
/// B-15: the proposed extras a run may switch on.
pub const EXTRAS: [&str; 3] = ["press", "titlefade", "labelfade"];

pub const FROM_AUTOSTART: &str = "--from-autostart";
/// Interface Scale steps (reference A8: 90–150 % by 10).
pub const SCALE_STEPS: [u32; 7] = [90, 100, 110, 120, 130, 140, 150];

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Args {
    pub wait_for_pid: Option<u32>,
    pub autostart: Option<bool>,
    /// Started by the Start with Windows entry (`--from-autostart`).
    pub from_autostart: bool,
    /// `--interface-scale=N` for this run (percent, one of `SCALE_STEPS`).
    pub interface_scale: Option<u32>,
    /// `--boot=default`: this run shows the default "Window B" Boot Up / Boot Down even when a
    /// custom video exists (T1.2, for testing; never stored).
    pub boot_default: bool,
    /// B-15: `--extras=press,titlefade,labelfade`: proposed extras switched on for this run
    /// (off by default until the user approves each; never stored). Unknown names are ignored.
    pub extras: Vec<String>,
    pub unknown: Vec<String>,
}

pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Args {
    let mut out = Args::default();
    let mut it = args.into_iter().peekable();
    while let Some(a) = it.next() {
        if a == "--wait-for-pid" {
            match it.next().and_then(|v| v.parse().ok()) {
                Some(pid) => out.wait_for_pid = Some(pid),
                None => out.unknown.push(a),
            }
        } else if let Some(v) = a.strip_prefix("--wait-for-pid=") {
            match v.parse() {
                Ok(pid) => out.wait_for_pid = Some(pid),
                Err(_) => out.unknown.push(a),
            }
        } else if let Some(v) = a.strip_prefix("--autostart=") {
            match v.to_ascii_lowercase().as_str() {
                "on" | "true" | "1" => out.autostart = Some(true),
                "off" | "false" | "0" => out.autostart = Some(false),
                _ => out.unknown.push(a),
            }
        } else if a == "--boot=default" {
            out.boot_default = true;
        } else if let Some(v) = a.strip_prefix("--extras=") {
            out.extras = v
                .split(',')
                .map(|x| x.trim().to_ascii_lowercase())
                .filter(|x| EXTRAS.contains(&x.as_str()))
                .collect();
        } else if a == FROM_AUTOSTART {
            out.from_autostart = true;
        } else if let Some(v) = a.strip_prefix("--interface-scale=") {
            match v.trim_end_matches('%').parse::<u32>() {
                Ok(n) if SCALE_STEPS.contains(&n) => out.interface_scale = Some(n),
                _ => out.unknown.push(a),
            }
        } else {
            out.unknown.push(a);
        }
    }
    out
}

/// The command the Start with Windows entry runs: the exe in quotes, then the marker.
pub fn autostart_command(exe: &std::path::Path) -> String {
    format!("\"{}\" {FROM_AUTOSTART}", exe.display())
}

/// Waits for the previous instance to exit. Returns a log line describing what happened.
pub fn wait_for_previous(pid: u32) -> String {
    #[cfg(windows)]
    {
        win::wait_for_pid(pid, WAIT_TIMEOUT)
    }
    #[cfg(not(windows))]
    {
        let started = std::time::Instant::now();
        let proc_dir = PathBuf::from(format!("/proc/{pid}"));
        while proc_dir.exists() && started.elapsed() < WAIT_TIMEOUT {
            std::thread::sleep(Duration::from_millis(50));
        }
        format!("waited {} ms for pid {pid}", started.elapsed().as_millis())
    }
}

/// Starts the new instance (which waits for this one to exit). The caller then exits.
pub fn spawn_restart() -> std::io::Result<u32> {
    let exe = std::env::current_exe()?;
    let child = std::process::Command::new(exe)
        .arg("--wait-for-pid")
        .arg(std::process::id().to_string())
        .spawn()?;
    Ok(child.id())
}

/// True when another AIONE 4 is already running (a named mutex this process then keeps).
/// Used only to log the hand-over: the single-instance plugin does the hand-over itself and
/// exits this process before the window opens.
pub fn another_instance_running() -> bool {
    #[cfg(windows)]
    {
        win::another_instance_running()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub fn exe_path() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

/// `--autostart=on|off`. Returns the message shown to the user and logged.
pub fn set_autostart(enable: bool) -> Result<String, String> {
    #[cfg(windows)]
    {
        win::set_autostart(enable)
    }
    #[cfg(not(windows))]
    {
        let _ = enable;
        Err("Start with Windows is only available on Windows.".into())
    }
}

/// The OS actions the start steps need; a fake records their order in tests.
pub trait StartOps {
    fn set_autostart(&mut self, enable: bool) -> String;
    fn wait_for_previous(&mut self, pid: u32) -> String;
    fn another_instance_running(&mut self) -> bool;
}

/// The real OS actions.
pub struct RealOps;

impl StartOps for RealOps {
    fn set_autostart(&mut self, enable: bool) -> String {
        set_autostart(enable).unwrap_or_else(|e| e)
    }
    fn wait_for_previous(&mut self, pid: u32) -> String {
        wait_for_previous(pid)
    }
    fn another_instance_running(&mut self) -> bool {
        another_instance_running()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreStart {
    /// A one-shot action ran (`--autostart`); show the message and exit.
    Exit { message: String },
    /// Start the app. `second_instance`: another AIONE 4 still runs; the single-instance
    /// plugin will hand over to it and exit this process.
    Continue { second_instance: bool },
}

/// The start steps before Tauri, in their required order (A5, restart risk of alpha.2):
/// 1. `--autostart=on|off`: do it and exit;
/// 2. `--wait-for-pid N` (restart): wait until the old instance has exited;
/// 3. only then the named-mutex "already running" check (logging the hand-over).
///
/// The caller registers the single-instance plugin after this returns.
pub fn pre_start(args: &Args, ops: &mut dyn StartOps) -> PreStart {
    if let Some(enable) = args.autostart {
        let message = ops.set_autostart(enable);
        logging::lifecycle(Level::Info, &format!("--autostart: {message}"));
        return PreStart::Exit { message };
    }
    if let Some(pid) = args.wait_for_pid {
        logging::lifecycle(
            Level::Info,
            &format!("restart: waiting for pid {pid} to exit"),
        );
        let msg = ops.wait_for_previous(pid);
        logging::lifecycle(Level::Info, &format!("restart: {msg}"));
    }
    let second_instance = ops.another_instance_running();
    if second_instance {
        logging::lifecycle(
            Level::Info,
            "second instance: AIONE 4 is already running; handing over to it (it comes to the front) and exiting",
        );
    }
    PreStart::Continue { second_instance }
}

pub fn log_start(args: &Args) {
    logging::lifecycle(
        Level::Info,
        &format!(
            "start: AIONE 4 {} pid {} exe {} args {:?}",
            crate::version::VERSION,
            std::process::id(),
            exe_path().display(),
            std::env::args().skip(1).collect::<Vec<_>>()
        ),
    );
    if args.from_autostart {
        logging::lifecycle(
            Level::Info,
            "started by Windows at sign-in (the Start with Windows entry passed --from-autostart)",
        );
    }
    if !args.extras.is_empty() {
        logging::lifecycle(
            Level::Info,
            &format!(
                "--extras: {} for this run (not stored)",
                args.extras.join(", ")
            ),
        );
    }
    if args.boot_default {
        logging::lifecycle(
            Level::Info,
            "--boot=default: this run shows the default Window B Boot Up and Boot Down (not stored)",
        );
    }
    if let Some(n) = args.interface_scale {
        logging::lifecycle(
            Level::Info,
            &format!("Interface Scale {n} % for this run (--interface-scale; not stored)"),
        );
    }
    if !args.unknown.is_empty() {
        logging::lifecycle(
            Level::Warn,
            &format!("unknown arguments ignored: {:?}", args.unknown),
        );
    }
}

#[cfg(windows)]
mod win {
    use std::time::{Duration, Instant};
    use windows::core::{w, HSTRING};
    use windows::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, WAIT_OBJECT_0, WAIT_TIMEOUT,
    };
    use windows::Win32::System::Registry::{
        RegDeleteKeyValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ,
    };
    use windows::Win32::System::Threading::{
        CreateMutexW, OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };

    pub fn another_instance_running() -> bool {
        // SAFETY: the handle is intentionally kept open for the life of the process (it marks
        // this instance as running); last-error is read right after the call.
        unsafe {
            match CreateMutexW(None, false, w!("Local\\AIONE4.InstanceProbe")) {
                Ok(_handle) => GetLastError() == ERROR_ALREADY_EXISTS,
                Err(_) => false,
            }
        }
    }

    pub fn wait_for_pid(pid: u32, timeout: Duration) -> String {
        let started = Instant::now();
        // SAFETY: handle checked and closed.
        unsafe {
            let Ok(handle) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) else {
                return format!("previous instance pid {pid} had already exited");
            };
            let rc = WaitForSingleObject(handle, timeout.as_millis() as u32);
            let _ = CloseHandle(handle);
            let ms = started.elapsed().as_millis();
            if rc == WAIT_OBJECT_0 {
                format!("previous instance pid {pid} exited after {ms} ms")
            } else if rc == WAIT_TIMEOUT {
                format!("previous instance pid {pid} still running after {ms} ms; continuing (the single-instance check decides)")
            } else {
                format!("waiting for pid {pid} failed (code {}); continuing", rc.0)
            }
        }
    }

    const RUN_KEY: windows::core::PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

    pub fn set_autostart(enable: bool) -> Result<String, String> {
        let name = HSTRING::from(super::RUN_VALUE_NAME);
        if enable {
            let value = super::autostart_command(&super::exe_path());
            let wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
            // SAFETY: the data pointer and byte length describe `wide`.
            let rc = unsafe {
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    RUN_KEY,
                    &name,
                    REG_SZ.0,
                    Some(wide.as_ptr() as *const _),
                    (wide.len() * 2) as u32,
                )
            };
            if rc.is_ok() {
                Ok(format!("AIONE 4 will start with Windows ({value})."))
            } else {
                Err(format!(
                    "Could not turn on Start with Windows (code {}).",
                    rc.0
                ))
            }
        } else {
            // SAFETY: plain registry call.
            let rc = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, &name) };
            // ERROR_FILE_NOT_FOUND (2): it was not registered; that is fine.
            if rc.is_ok() || rc.0 == 2 {
                Ok("AIONE 4 will no longer start with Windows.".into())
            } else {
                Err(format!(
                    "Could not turn off Start with Windows (code {}).",
                    rc.0
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(v: &[&str]) -> Args {
        parse_args(v.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_restart_and_autostart() {
        assert_eq!(p(&["--wait-for-pid", "1234"]).wait_for_pid, Some(1234));
        assert_eq!(p(&["--wait-for-pid=99"]).wait_for_pid, Some(99));
        assert_eq!(p(&["--autostart=on"]).autostart, Some(true));
        assert_eq!(p(&["--autostart=OFF"]).autostart, Some(false));
        assert_eq!(p(&[]), Args::default());
        assert!(p(&["--from-autostart"]).from_autostart);
        assert!(p(&["--boot=default"]).boot_default);
        assert_eq!(
            p(&["--extras=press,TitleFade,unknown"]).extras,
            vec!["press", "titlefade"]
        );
        assert!(p(&[]).extras.is_empty());
        assert_eq!(p(&["--interface-scale=150"]).interface_scale, Some(150));
        assert_eq!(p(&["--interface-scale=90%"]).interface_scale, Some(90));
    }

    #[test]
    fn interface_scale_accepts_only_the_steps() {
        for bad in [
            "--interface-scale=155",
            "--interface-scale=80",
            "--interface-scale=x",
        ] {
            let a = p(&[bad]);
            assert_eq!(a.interface_scale, None, "{bad}");
            assert_eq!(a.unknown, vec![bad]);
        }
    }

    /// T0 QA-4: the Start with Windows entry carries the marker, after the quoted exe path.
    #[test]
    fn autostart_entry_passes_the_marker() {
        let cmd = autostart_command(std::path::Path::new(r"D:\Games\AIONE 4\AIONE4.exe"));
        assert_eq!(cmd, r#""D:\Games\AIONE 4\AIONE4.exe" --from-autostart"#);
        // What Windows runs at sign-in parses back to a marked start.
        let args: Vec<String> = cmd
            .split('"')
            .nth(2)
            .unwrap()
            .split_whitespace()
            .map(String::from)
            .collect();
        assert!(parse_args(args).from_autostart);
    }

    #[test]
    fn keeps_unknown_arguments_for_the_log() {
        let a = p(&["--wait-for-pid", "x", "--autostart=maybe", "--selftest"]);
        assert_eq!(a.wait_for_pid, None);
        assert_eq!(a.autostart, None);
        assert_eq!(
            a.unknown,
            vec!["--wait-for-pid", "--autostart=maybe", "--selftest"]
        );
    }

    /// Records the order of the OS actions; `running_after_wait` is what the mutex check finds.
    struct Fake {
        calls: Vec<String>,
        running_after_wait: bool,
    }

    impl StartOps for Fake {
        fn set_autostart(&mut self, enable: bool) -> String {
            self.calls.push(format!("autostart {enable}"));
            "ok".into()
        }
        fn wait_for_previous(&mut self, pid: u32) -> String {
            self.calls.push(format!("wait {pid}"));
            "exited".into()
        }
        fn another_instance_running(&mut self) -> bool {
            self.calls.push("mutex check".into());
            self.running_after_wait
        }
    }

    #[test]
    fn restart_waits_for_the_old_instance_before_the_already_running_check() {
        let mut f = Fake {
            calls: vec![],
            running_after_wait: false,
        };
        let r = pre_start(&p(&["--wait-for-pid", "4242"]), &mut f);
        assert_eq!(f.calls, vec!["wait 4242", "mutex check"]);
        // The old instance is gone, so the restarted one is not a second instance.
        assert_eq!(
            r,
            PreStart::Continue {
                second_instance: false
            }
        );
    }

    #[test]
    fn a_plain_second_start_checks_without_waiting() {
        let mut f = Fake {
            calls: vec![],
            running_after_wait: true,
        };
        assert_eq!(
            pre_start(&p(&[]), &mut f),
            PreStart::Continue {
                second_instance: true
            }
        );
        assert_eq!(f.calls, vec!["mutex check"]);
    }

    #[test]
    fn autostart_runs_alone_and_exits() {
        let mut f = Fake {
            calls: vec![],
            running_after_wait: true,
        };
        let r = pre_start(&p(&["--autostart=on", "--wait-for-pid", "1"]), &mut f);
        assert_eq!(f.calls, vec!["autostart true"]);
        assert!(matches!(r, PreStart::Exit { .. }));
    }

    #[cfg(not(windows))]
    #[test]
    fn waiting_for_an_exited_pid_returns_at_once() {
        let started = std::time::Instant::now();
        wait_for_previous(u32::MAX - 1);
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
