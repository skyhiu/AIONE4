//! XInput backend (Windows). Loaded dynamically from xinput1_4.dll (falls back to
//! xinput1_3.dll, then xinput9_1_0.dll). Uses `XInputGetStateEx` (ordinal 100) when present,
//! because only it reports the Guide button; otherwise `XInputGetState`.

use super::gate::{BackgroundGate, Route, BACKGROUND_NOTE};
use super::mapping::{Kind, PadOut, PadTracker, RawPad, CONTROLS};
use super::{next_seq, PadEvent};
use crate::logging;
use std::cell::RefCell;
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

pub fn start(emit: impl Fn(PadEvent) + Send + 'static, guide: impl Fn() + Send + 'static) {
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
            run(get_state, emit, guide);
        });
    if let Err(e) = spawned {
        logging::error("input", &format!("could not start the input thread: {e}"));
    }
}

fn run(get_state: GetStateFn, emit: impl Fn(PadEvent), guide: impl Fn()) {
    let mut trackers: [PadTracker; 4] = Default::default();
    let mut connected = [false; 4];
    let mut last_probe = [None::<Instant>; 4];
    // B-7: in the background only Guide acts; everything else is logged only.
    let gate = RefCell::new(BackgroundGate::default());
    let send = |kind: &'static str, control: &'static str, pad: u32, detail: &str| {
        let ev = PadEvent {
            kind,
            control,
            pad,
            seq: next_seq(),
            at_ms: super::now_ms(),
        };
        if kind == "connected" || kind == "disconnected" {
            super::record_connection(pad, kind == "connected", ev.seq);
        }
        // T0 evidence for "one press shows once": presses and releases are logged; a release
        // also says how long it was held and how many repeats it gave.
        if kind == "down" || kind == "up" {
            logging::info(
                "input",
                &format!(
                    "pad {} {} {} (seq {}){detail}",
                    pad + 1,
                    kind,
                    control,
                    ev.seq
                ),
            );
        }
        emit(ev);
    };
    let out = |e: &PadOut, slot: u32| {
        let control = CONTROLS[e.index];
        let foreground = crate::foreground::aione_is_foreground();
        let route = gate
            .borrow_mut()
            .route(slot, e.index, control, e.kind, foreground);
        let mut detail = if e.kind == Kind::Up {
            format!(", held {} ms, {} repeats", e.held.as_millis(), e.repeats)
        } else {
            String::new()
        };
        match route {
            Route::Act => send(e.kind.as_str(), control, slot, &detail),
            Route::ActGuide => {
                detail.push_str(" (AIONE in the background: Guide acts)");
                send(e.kind.as_str(), control, slot, &detail);
                if e.kind == Kind::Down {
                    guide();
                }
            }
            Route::LogOnly => {
                // Logged with the same wording as acted-on presses, never sent to the UI.
                // Background repeats are not logged one by one; the release says how many.
                if e.kind != Kind::Repeat {
                    logging::info(
                        "input",
                        &format!(
                            "pad {} {} {control} (seq {}){detail}{BACKGROUND_NOTE}",
                            slot + 1,
                            e.kind.as_str(),
                            next_seq()
                        ),
                    );
                }
            }
        }
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
                    send("connected", "", slot, "");
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
                for e in trackers[i].update(&raw, now) {
                    out(&e, slot);
                }
            } else {
                last_probe[i] = Some(now);
                if connected[i] {
                    connected[i] = false;
                    for e in trackers[i].release_all(now) {
                        out(&e, slot);
                    }
                    logging::info(
                        "input",
                        &format!("controller disconnected from slot {} (code {rc})", slot + 1),
                    );
                    send("disconnected", "", slot, "");
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
