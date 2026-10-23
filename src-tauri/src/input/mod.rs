//! Controller input in the core (Master Prompt A4): XInput is read on its own thread, also
//! while AIONE is not focused, and sent to the UI as `aione://pad` events.
//!
//! Polling happens here, not in the UI (XInput has no events): every 8 ms while a pad is
//! connected; empty slots are probed once a second; with no pad connected the thread sleeps
//! 250 ms between checks (idle CPU stays near zero).

pub mod gate;
pub mod mapping;
#[cfg(windows)]
mod xinput;

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PadEvent {
    /// "down" | "up" | "repeat" | "held" | "connected" | "disconnected" | "battery" | "pointer"
    pub kind: &'static str,
    pub control: &'static str,
    /// XInput slot 0–3.
    pub pad: u32,
    pub seq: u64,
    /// D-008: when the core read it (Unix time in ms), so the UI can log the core → UI delay.
    pub at_ms: u64,
    /// B-12 (reference A7): for "pointer" events, the right stick past its dead zone, each axis
    /// -1000..=1000 (up positive); [0, 0] when it's let go. Absent for every other event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pointer: Option<[i16; 2]>,
}

/// T2.3 notifications: a controller's battery as XInput reports it (XINPUT_BATTERY_INFORMATION):
/// "wired", "unknown", or the level of a battery: "empty", "low", "medium", "full".
pub fn battery_word(battery_type: u8, level: u8) -> &'static str {
    match battery_type {
        0x01 => "wired",
        0x00 | 0xFF => "unknown",
        _ => match level {
            0 => "empty",
            1 => "low",
            2 => "medium",
            _ => "full",
        },
    }
}

/// Unix time in milliseconds (the UI's `Date.now()` uses the same clock).
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

static SEQ: AtomicU64 = AtomicU64::new(0);

pub fn next_seq() -> u64 {
    SEQ.fetch_add(1, Ordering::Relaxed) + 1
}

/// The connected slots as the core knows them, with the sequence number of the last
/// connect / disconnect event (D-002: the UI asks for this at start instead of relying only
/// on connect events, which can arrive before the UI listens).
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PadSlots {
    /// Connected XInput slots, 0–3.
    pub slots: Vec<u32>,
    /// Sequence number of the last connection change (0 = none yet).
    pub seq: u64,
}

static SLOTS: Mutex<(u8, u64)> = Mutex::new((0, 0));

/// Records a connection change; called by the backend with the event's sequence number.
pub fn record_connection(slot: u32, connected: bool, seq: u64) {
    if let Ok(mut s) = SLOTS.lock() {
        if connected {
            s.0 |= 1 << slot;
        } else {
            s.0 &= !(1 << slot);
        }
        s.1 = seq;
    }
}

pub fn pad_slots() -> PadSlots {
    let (mask, seq) = SLOTS.lock().map(|s| *s).unwrap_or((0, 0));
    PadSlots {
        slots: (0..4u32).filter(|i| mask & (1 << i) != 0).collect(),
        seq,
    }
}

/// Starts the input thread. `emit` receives every event the UI may act on (called on the input
/// thread). B-7: while AIONE is in the background, only Guide reaches `emit`; a Guide press in
/// the background also calls `guide_in_background` (the core brings AIONE to the front).
pub fn start(
    emit: impl Fn(PadEvent) + Send + 'static,
    guide_in_background: impl Fn() + Send + 'static,
) {
    #[cfg(windows)]
    {
        xinput::start(emit, guide_in_background);
    }
    #[cfg(not(windows))]
    {
        let _ = (emit, guide_in_background);
        crate::logging::warn(
            "input",
            "XInput is only available on Windows; no controller input",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// XInput's battery types: 0 disconnected, 1 wired, 2 alkaline, 3 NiMH, 0xFF unknown; levels
    /// 0 empty … 3 full.
    #[test]
    fn battery_words() {
        assert_eq!(battery_word(1, 0), "wired");
        assert_eq!(battery_word(0, 3), "unknown");
        assert_eq!(battery_word(0xFF, 1), "unknown");
        assert_eq!(battery_word(2, 0), "empty");
        assert_eq!(battery_word(3, 1), "low");
        assert_eq!(battery_word(2, 2), "medium");
        assert_eq!(battery_word(3, 3), "full");
    }

    #[test]
    fn slots_snapshot_follows_connection_changes() {
        record_connection(0, true, 5);
        record_connection(2, true, 6);
        assert_eq!(
            pad_slots(),
            PadSlots {
                slots: vec![0, 2],
                seq: 6
            }
        );
        record_connection(0, false, 9);
        assert_eq!(
            pad_slots(),
            PadSlots {
                slots: vec![2],
                seq: 9
            }
        );
        record_connection(2, false, 10);
        assert_eq!(
            pad_slots(),
            PadSlots {
                slots: vec![],
                seq: 10
            }
        );
    }
}
