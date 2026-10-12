//! XInput backend (Windows). Loaded dynamically from xinput1_4.dll (falls back to
//! xinput1_3.dll, then xinput9_1_0.dll). Uses `XInputGetStateEx` (ordinal 100) when present,
//! because only it reports the Guide button; otherwise `XInputGetState`.

use super::mapping::{PadTracker, RawPad, CONTROLS};
use super::{next_seq, PadEvent};
use crate::logging;
use std::time::{Duration, Instant};
use windows::core::{s, w, PCSTR, PCWSTR};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct XInputGamepad {
    buttons: u16,
    left_trigger: u8,
    right_trigger: u8,
    thumb_lx: i16,
    thumb_ly: i16,
    thumb_rx: i16,
    thumb_ry: i16,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct XInputState {
    _packet: u32,
    gamepad: XInputGamepad,
}

/// XInputGetStateEx writes one extra DWORD after XINPUT_STATE; give it room.
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct XInputStateEx {
    state: XInputState,
    _reserved: u32,
}

type GetStateFn = unsafe extern "system" fn(u32, *mut XInputStateEx) -> u32;

const ERROR_SUCCESS: u32 = 0;
const POLL_CONNECTED: Duration = Duration::from_millis(8);
const POLL_IDLE: Duration = Duration::from_millis(250);
const PROBE_EMPTY_SLOT: Duration = Duration::from_secs(1);

fn load() -> Option<(GetStateFn, String)> {
    let dlls: [(PCWSTR, &str); 3] = [
        (w!("xinput1_4.dll"), "xinput1_4.dll"),
        (w!("xinput1_3.dll"), "xinput1_3.dll"),
        (w!("xinput9_1_0.dll"), "xinput9_1_0.dll"),
    ];
    for (dll, name) in dlls {
        // SAFETY: loading a system DLL by name; the handle is kept for the process lifetime.
        let Ok(module) = (unsafe { LoadLibraryW(dll) }) else {
            continue;
        };
        // SAFETY: GetProcAddress with an ordinal (100 = XInputGetStateEx) or a name.
        let ex = unsafe { GetProcAddress(module, PCSTR(100usize as *const u8)) };
        if let Some(f) = ex {
            // SAFETY: XInputGetStateEx has the signature of GetStateFn.
            let f: GetStateFn = unsafe {
                std::mem::transmute::<unsafe extern "system" fn() -> isize, GetStateFn>(f)
            };
            return Some((
                f,
                format!("{name} XInputGetStateEx (Guide button available)"),
            ));
        }
        let plain = unsafe { GetProcAddress(module, s!("XInputGetState")) };
        if let Some(f) = plain {
            // SAFETY: XInputGetState writes an XINPUT_STATE, which fits in XInputStateEx.
            let f: GetStateFn = unsafe {
                std::mem::transmute::<unsafe extern "system" fn() -> isize, GetStateFn>(f)
            };
            return Some((f, format!("{name} XInputGetState (no Guide button)")));
        }
    }
    None
}

pub fn start(emit: impl Fn(PadEvent) + Send + 'static) {
    let spawned = std::thread::Builder::new()
        .name("aione-xinput".into())
        .spawn(move || {
            let Some((get_state, how)) = load() else {
                logging::error("input", "XInput could not be loaded; no controller input");
                return;
            };
            logging::info(
                "input",
                &format!("XInput ready: {how}; polling 8 ms while a pad is connected"),
            );
            run(get_state, emit);
        });
    if let Err(e) = spawned {
        logging::error("input", &format!("could not start the input thread: {e}"));
    }
}

fn run(get_state: GetStateFn, emit: impl Fn(PadEvent)) {
    let mut trackers: [PadTracker; 4] = Default::default();
    let mut connected = [false; 4];
    let mut last_probe = [None::<Instant>; 4];
    let send = |kind: &'static str, control: &'static str, pad: u32| {
        let ev = PadEvent {
            kind,
            control,
            pad,
            seq: next_seq(),
        };
        // T0 evidence for "one press shows once": presses and releases are logged.
        if kind == "down" || kind == "up" {
            logging::info(
                "input",
                &format!("pad {} {} {} (seq {})", pad + 1, kind, control, ev.seq),
            );
        }
        emit(ev);
    };

    loop {
        let now = Instant::now();
        for slot in 0..4u32 {
            let i = slot as usize;
            if !connected[i]
                && last_probe[i].is_some_and(|t| now.duration_since(t) < PROBE_EMPTY_SLOT)
            {
                continue;
            }
            let mut st = XInputStateEx::default();
            // SAFETY: valid pointer to a buffer large enough for either function.
            let rc = unsafe { get_state(slot, &mut st) };
            if rc == ERROR_SUCCESS {
                if !connected[i] {
                    connected[i] = true;
                    logging::info(
                        "input",
                        &format!("controller connected in slot {}", slot + 1),
                    );
                    send("connected", "", slot);
                }
                let g = st.state.gamepad;
                let raw = RawPad {
                    buttons: g.buttons,
                    lt: g.left_trigger,
                    rt: g.right_trigger,
                    lx: g.thumb_lx,
                    ly: g.thumb_ly,
                    rx: g.thumb_rx,
                    ry: g.thumb_ry,
                };
                // Every poll runs the tracker, changed or not (repeats need the clock).
                for (kind, idx) in trackers[i].update(&raw, now) {
                    send(kind.as_str(), CONTROLS[idx], slot);
                }
            } else {
                last_probe[i] = Some(now);
                if connected[i] {
                    connected[i] = false;
                    for (kind, idx) in trackers[i].release_all() {
                        send(kind.as_str(), CONTROLS[idx], slot);
                    }
                    logging::info(
                        "input",
                        &format!("controller disconnected from slot {} (code {rc})", slot + 1),
                    );
                    send("disconnected", "", slot);
                }
            }
        }
        std::thread::sleep(if connected.iter().any(|c| *c) {
            POLL_CONNECTED
        } else {
            POLL_IDLE
        });
    }
}
