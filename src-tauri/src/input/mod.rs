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
