//! Start-up arguments, restart, and the Start with Windows registration.
//!
//! Restart (A5): the running instance starts the new one with `--wait-for-pid <its pid>` and
//! exits. The new instance waits for that process to end **before** it claims the single
//! instance, opens its window or plays anything (at most 15 s, logged), so two instances never
//! run at the same time.
//!
//! Start with Windows: `AIONE4.exe --autostart=on|off` writes / removes the value "AIONE 4" in
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` and exits (T0 has no Settings yet; the
//! setting comes with Settings in T3).

use crate::logging::{self, Level};
use std::path::PathBuf;
use std::time::Duration;

pub const WAIT_TIMEOUT: Duration = Duration::from_secs(15);
pub const RUN_VALUE_NAME: &str = "AIONE 4";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Args {
    pub wait_for_pid: Option<u32>,
    pub autostart: Option<bool>,
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
        } else {
            out.unknown.push(a);
        }
    }
    out
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
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows::Win32::System::Registry::{
        RegDeleteKeyValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };

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
            let exe = super::exe_path();
            let value = format!("\"{}\"", exe.display());
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

    #[cfg(not(windows))]
    #[test]
    fn waiting_for_an_exited_pid_returns_at_once() {
        let started = std::time::Instant::now();
        wait_for_previous(u32::MAX - 1);
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
