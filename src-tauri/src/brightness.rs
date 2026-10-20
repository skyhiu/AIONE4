//! B-26 (rev. ax): the Quick Menu's Brightness row, universal (B-23).
//!
//! 1. A laptop's or tablet's **built-in screen**: Windows' own brightness control (WMI
//!    `WmiMonitorBrightness` / `WmiSetBrightness`), through PowerShell, which every Windows has.
//! 2. An **external monitor**: its DDC/CI control through Windows' monitor API (`Dxva2`), where the
//!    monitor supports it.
//! 3. Neither: the row says "Brightness can't be changed for this display".
//!
//! The way is probed once in the background (logged), so the Quick Menu never waits; changes are
//! applied in the background, the last one winning, and logged.

use crate::logging::{self, Level};
use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Brightness {
    /// "builtin" (Windows' control of a built-in screen), "ddc" (an external monitor), "none".
    pub method: String,
    /// 0–100 % (when supported).
    pub value: Option<u8>,
    /// What the row says when it can't be changed.
    pub reason: Option<String>,
}

impl Brightness {
    fn none(reason: &str) -> Brightness {
        Brightness {
            method: "none".into(),
            value: None,
            reason: Some(reason.into()),
        }
    }
}

static STATE: Mutex<Option<Brightness>> = Mutex::new(None);
static PENDING: Mutex<Option<u8>> = Mutex::new(None);

/// PowerShell's answer for the built-in screen's brightness ("57"), or none.
pub fn parse_percent(out: &str) -> Option<u8> {
    out.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .and_then(|l| l.parse::<u32>().ok())
        .map(|v| v.min(100) as u8)
}

/// A DDC/CI reading (min, current, max) as 0–100 %.
pub fn ddc_percent(min: u32, cur: u32, max: u32) -> u8 {
    if max <= min {
        return 0;
    }
    (((cur.clamp(min, max) - min) as f64 / (max - min) as f64) * 100.0).round() as u8
}

/// 0–100 % as a DDC/CI value.
pub fn ddc_value(min: u32, max: u32, percent: u8) -> u32 {
    min + ((max.saturating_sub(min)) as f64 * percent.min(100) as f64 / 100.0).round() as u32
}

/// The current state (probed at start-up; `probe` refreshes it).
pub fn current() -> Brightness {
    STATE
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_else(|| Brightness::none("Checking the display…"))
}

/// Probes the way (built-in, then DDC/CI) and reads the value. Logged.
pub fn probe() -> Brightness {
    let b = if let Some(v) = builtin_get() {
        Brightness {
            method: "builtin".into(),
            value: Some(v),
            reason: None,
        }
    } else if let Some(v) = ddc_get() {
        Brightness {
            method: "ddc".into(),
            value: Some(v),
            reason: None,
        }
    } else {
        Brightness::none("Brightness can't be changed for this display")
    };
    logging::lifecycle(
        Level::Info,
        &format!(
            "brightness: {}",
            match b.method.as_str() {
                "builtin" => format!(
                    "the built-in screen through Windows, {} %",
                    b.value.unwrap_or(0)
                ),
                "ddc" => format!("the monitor through DDC/CI, {} %", b.value.unwrap_or(0)),
                _ => "no display AIONE can control (neither a built-in screen nor DDC/CI)".into(),
            }
        ),
    );
    if let Ok(mut g) = STATE.lock() {
        *g = Some(b.clone());
    }
    b
}

/// Sets the brightness in the background; the last request wins. Logged.
pub fn set(percent: u8) {
    let percent = percent.min(100);
    let busy = PENDING
        .lock()
        .map(|mut p| p.replace(percent).is_some())
        .unwrap_or(true);
    if let Ok(mut g) = STATE.lock() {
        if let Some(b) = g.as_mut() {
            b.value = Some(percent);
        }
    }
    if busy {
        return; // a worker is running; it picks up the latest value
    }
    std::thread::spawn(|| loop {
        std::thread::sleep(std::time::Duration::from_millis(120));
        let Some(v) = PENDING.lock().ok().and_then(|mut p| p.take()) else {
            return;
        };
        let method = current().method;
        let ok = match method.as_str() {
            "builtin" => builtin_set(v),
            "ddc" => ddc_set(v),
            _ => false,
        };
        logging::lifecycle(
            if ok { Level::Info } else { Level::Warn },
            &format!(
                "brightness: {v} % ({method}){}",
                if ok { "" } else { ": Windows didn't accept it" }
            ),
        );
        if PENDING.lock().map(|p| p.is_none()).unwrap_or(true) {
            return;
        }
    });
}

// ---------- the platform layer ----------

#[cfg(windows)]
fn powershell(cmd: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", cmd])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(windows)]
fn builtin_get() -> Option<u8> {
    powershell("(Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBrightness -ErrorAction Stop | Select-Object -First 1).CurrentBrightness")
        .as_deref()
        .and_then(parse_percent)
}

#[cfg(windows)]
fn builtin_set(v: u8) -> bool {
    powershell(&format!(
        "Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBrightnessMethods -ErrorAction Stop | Invoke-CimMethod -MethodName WmiSetBrightness -Arguments @{{Timeout=0; Brightness={v}}} | Out-Null"
    ))
    .is_some()
}

#[cfg(windows)]
fn with_monitor<T>(f: impl FnOnce(windows::Win32::Foundation::HANDLE) -> Option<T>) -> Option<T> {
    use windows::Win32::Devices::Display::{
        DestroyPhysicalMonitors, GetNumberOfPhysicalMonitorsFromHMONITOR,
        GetPhysicalMonitorsFromHMONITOR, PHYSICAL_MONITOR,
    };
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTOPRIMARY};
    let hmon = unsafe { MonitorFromWindow(HWND::default(), MONITOR_DEFAULTTOPRIMARY) };
    let mut n = 0u32;
    unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(hmon, &mut n) }.ok()?;
    if n == 0 {
        return None;
    }
    let mut mons = vec![PHYSICAL_MONITOR::default(); n as usize];
    unsafe { GetPhysicalMonitorsFromHMONITOR(hmon, &mut mons) }.ok()?;
    let r = f(mons[0].hPhysicalMonitor);
    let _ = unsafe { DestroyPhysicalMonitors(&mons) };
    r
}

#[cfg(windows)]
fn ddc_get() -> Option<u8> {
    use windows::Win32::Devices::Display::GetMonitorBrightness;
    with_monitor(|h| {
        let (mut min, mut cur, mut max) = (0u32, 0u32, 0u32);
        let ok = unsafe { GetMonitorBrightness(h, &mut min, &mut cur, &mut max) } != 0;
        ok.then(|| ddc_percent(min, cur, max))
    })
}

#[cfg(windows)]
fn ddc_set(v: u8) -> bool {
    use windows::Win32::Devices::Display::{GetMonitorBrightness, SetMonitorBrightness};
    with_monitor(|h| {
        let (mut min, mut cur, mut max) = (0u32, 0u32, 0u32);
        if unsafe { GetMonitorBrightness(h, &mut min, &mut cur, &mut max) } == 0 {
            return None;
        }
        (unsafe { SetMonitorBrightness(h, ddc_value(min, max, v)) } != 0).then_some(())
    })
    .is_some()
}

#[cfg(not(windows))]
fn builtin_get() -> Option<u8> {
    None
}
#[cfg(not(windows))]
fn builtin_set(_v: u8) -> bool {
    false
}
#[cfg(not(windows))]
fn ddc_get() -> Option<u8> {
    None
}
#[cfg(not(windows))]
fn ddc_set(_v: u8) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readings_and_values() {
        assert_eq!(parse_percent("57\r\n"), Some(57));
        assert_eq!(parse_percent("\n  100 \n"), Some(100));
        assert_eq!(parse_percent(""), None);
        assert_eq!(parse_percent("not a number"), None);
        assert_eq!(ddc_percent(0, 50, 100), 50);
        assert_eq!(ddc_percent(10, 10, 10), 0); // a broken range
        assert_eq!(ddc_value(0, 100, 30), 30);
        assert_eq!(ddc_value(20, 80, 50), 50);
        assert_eq!(ddc_percent(20, ddc_value(20, 80, 75), 80), 75);
    }
}
