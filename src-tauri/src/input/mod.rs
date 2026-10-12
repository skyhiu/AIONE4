//! Controller input in the core (Master Prompt A4): XInput is read on its own thread, also
//! while AIONE is not focused, and sent to the UI as `aione://pad` events.
//!
//! Polling happens here, not in the UI (XInput has no events): every 8 ms while a pad is
//! connected; empty slots are probed once a second; with no pad connected the thread sleeps
//! 250 ms between checks (idle CPU stays near zero).

pub mod mapping;
#[cfg(windows)]
mod xinput;

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PadEvent {
    /// "down" | "up" | "repeat" | "connected" | "disconnected"
    pub kind: &'static str,
    pub control: &'static str,
    /// XInput slot 0–3.
    pub pad: u32,
    pub seq: u64,
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

/// Starts the input thread. `emit` receives every event (called on the input thread).
pub fn start(emit: impl Fn(PadEvent) + Send + 'static) {
    #[cfg(windows)]
    {
        xinput::start(emit);
    }
    #[cfg(not(windows))]
    {
        let _ = emit;
        crate::logging::warn(
            "input",
            "XInput is only available on Windows; no controller input",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
