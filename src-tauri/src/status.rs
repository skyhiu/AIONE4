//! The PC's status for the Home header (reference A4): the battery (laptops only) and the
//! network connection (wired, Wi-Fi, or offline). Read in the core and pushed to the UI as
//! `aione://status` events **only when something changes** (no polling in the UI, A7).
//!
//! Timers: one core thread checks every 30 s (owner: status). Reading the local adapters and the
//! power status never touches the network (offline-first, A3).
//!
//! Wi-Fi signal strength is not shown yet (it needs the WLAN API); the Wi-Fi icon is the full one.

use serde::Serialize;
use std::time::Duration;

pub const CHECK_EVERY: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    /// 0–100, or None when Windows does not know.
    pub percent: Option<u8>,
    pub charging: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// "wired" | "wifi" | "offline" | "unknown"
    pub network: &'static str,
    /// None: no battery in this PC (a desktop): the header shows no battery icon.
    pub battery: Option<Battery>,
}

/// One network adapter as the classifier needs it.
#[derive(Debug, Clone, Copy)]
pub struct Adapter {
    pub wifi: bool,
    pub wired: bool,
    pub up: bool,
    pub has_gateway: bool,
}

/// Wired wins over Wi-Fi (as Windows' own tray does); an adapter counts only when it is up
/// and has a default gateway (a real connection, not a virtual switch).
pub fn classify(adapters: &[Adapter]) -> &'static str {
    let live = |a: &&Adapter| a.up && a.has_gateway;
    if adapters.iter().filter(live).any(|a| a.wired) {
        "wired"
    } else if adapters.iter().filter(live).any(|a| a.wifi) {
        "wifi"
    } else {
        "offline"
    }
}

/// SYSTEM_POWER_STATUS → the battery, or None without a system battery.
/// BatteryFlag 128 = no system battery, 255 = unknown; percent 255 = unknown.
pub fn battery_from(ac_line: u8, flag: u8, percent: u8) -> Option<Battery> {
    if flag == 128 || flag == 255 {
        return None;
    }
    Some(Battery {
        percent: (percent <= 100).then_some(percent),
        charging: flag & 8 != 0 || ac_line == 1,
    })
}

pub fn describe(s: &Status) -> String {
    let battery = match &s.battery {
        None => "no battery".to_string(),
        Some(b) => format!(
            "battery {}{}",
            b.percent
                .map(|p| format!("{p} %"))
                .unwrap_or_else(|| "unknown".into()),
            if b.charging { ", charging" } else { "" }
        ),
    };
    format!("network {}, {battery}", s.network)
}

pub fn read() -> Status {
    #[cfg(windows)]
    {
        Status {
            network: win::network(),
            battery: win::battery(),
        }
    }
    #[cfg(not(windows))]
    {
        Status {
            network: "unknown",
            battery: None,
        }
    }
}

/// Starts the status thread; `emit` gets the first status at once and then each change.
pub fn start(emit: impl Fn(&Status) + Send + 'static) {
    let spawned = std::thread::Builder::new()
        .name("aione-status".into())
        .spawn(move || {
            let mut last: Option<Status> = None;
            loop {
                let now = read();
                if last.as_ref() != Some(&now) {
                    crate::logging::info("status", &describe(&now));
                    emit(&now);
                    last = Some(now);
                }
                std::thread::sleep(CHECK_EVERY);
            }
        });
    if let Err(e) = spawned {
        crate::logging::error("status", &format!("could not start the status thread: {e}"));
    }
}

#[cfg(windows)]
mod win {
    use super::{battery_from, classify, Adapter, Battery};
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST,
        GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, IF_TYPE_ETHERNET_CSMACD,
        IF_TYPE_IEEE80211, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows::Win32::Networking::WinSock::AF_UNSPEC;
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

    const ERROR_SUCCESS: u32 = 0;
    const ERROR_BUFFER_OVERFLOW: u32 = 111;

    pub fn battery() -> Option<Battery> {
        let mut s = SYSTEM_POWER_STATUS::default();
        // SAFETY: valid out pointer.
        unsafe { GetSystemPowerStatus(&mut s) }.ok()?;
        battery_from(s.ACLineStatus, s.BatteryFlag, s.BatteryLifePercent)
    }

    pub fn network() -> &'static str {
        let flags = GAA_FLAG_INCLUDE_GATEWAYS
            | GAA_FLAG_SKIP_ANYCAST
            | GAA_FLAG_SKIP_MULTICAST
            | GAA_FLAG_SKIP_DNS_SERVER;
        let mut size: u32 = 16 * 1024;
        for _ in 0..3 {
            // u64 words keep the buffer aligned for IP_ADAPTER_ADDRESSES_LH.
            let mut buf = vec![0u64; (size as usize).div_ceil(8)];
            let head = buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
            // SAFETY: the buffer is `size` bytes and aligned; Windows fills a linked list in it.
            let rc = unsafe {
                GetAdaptersAddresses(AF_UNSPEC.0 as u32, flags, None, Some(head), &mut size)
            };
            if rc == ERROR_BUFFER_OVERFLOW {
                continue;
            }
            if rc != ERROR_SUCCESS {
                return "unknown";
            }
            let mut adapters = Vec::new();
            let mut p = head as *const IP_ADAPTER_ADDRESSES_LH;
            while !p.is_null() {
                // SAFETY: p points into `buf`, filled by GetAdaptersAddresses.
                let a = unsafe { &*p };
                adapters.push(Adapter {
                    wifi: a.IfType == IF_TYPE_IEEE80211,
                    wired: a.IfType == IF_TYPE_ETHERNET_CSMACD,
                    up: a.OperStatus == IfOperStatusUp,
                    has_gateway: !a.FirstGatewayAddress.is_null(),
                });
                p = a.Next;
            }
            return classify(&adapters);
        }
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(wifi: bool, wired: bool, up: bool, gw: bool) -> Adapter {
        Adapter {
            wifi,
            wired,
            up,
            has_gateway: gw,
        }
    }

    #[test]
    fn network_kind() {
        assert_eq!(classify(&[]), "offline");
        assert_eq!(classify(&[a(true, false, true, true)]), "wifi");
        assert_eq!(
            classify(&[a(true, false, true, true), a(false, true, true, true)]),
            "wired"
        );
        // Up without a gateway (e.g. a virtual switch) is not a connection.
        assert_eq!(classify(&[a(false, true, true, false)]), "offline");
        assert_eq!(classify(&[a(false, true, false, true)]), "offline");
    }

    #[test]
    fn battery_reading() {
        assert_eq!(battery_from(1, 128, 255), None, "desktop: no battery");
        assert_eq!(battery_from(255, 255, 255), None);
        assert_eq!(
            battery_from(0, 1, 72),
            Some(Battery {
                percent: Some(72),
                charging: false
            })
        );
        assert_eq!(
            battery_from(1, 8, 40),
            Some(Battery {
                percent: Some(40),
                charging: true
            })
        );
        assert_eq!(
            battery_from(0, 2, 255),
            Some(Battery {
                percent: None,
                charging: false
            })
        );
    }
}
