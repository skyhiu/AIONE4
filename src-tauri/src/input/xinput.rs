//! XInput backend (Windows). Loaded dynamically from xinput1_4.dll (falls back to
//! xinput1_3.dll, then xinput9_1_0.dll). Uses `XInputGetStateEx` (ordinal 100) when present,
//! because only it reports the Guide button; otherwise `XInputGetState`.

use super::gate::{BackgroundGate, Route, BACKGROUND_NOTE};
use super::mapping::{pointer_vector, Kind, PadOut, PadTracker, RawPad, CONTROLS};
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

/// XINPUT_BATTERY_INFORMATION.
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct XInputBattery {
    battery_type: u8,
    battery_level: u8,
}

/// XInputGetBatteryInformation (xinput1_4 / xinput1_3; not in xinput9_1_0).
type BatteryFn = unsafe extern "system" fn(u32, u8, *mut XInputBattery) -> u32;
/// T2.3 notifications: a connected pad's battery is read at its connection, then once a minute.
const BATTERY_EVERY: Duration = Duration::from_secs(60);

const ERROR_SUCCESS: u32 = 0;
const POLL_CONNECTED: Duration = Duration::from_millis(8);
const POLL_IDLE: Duration = Duration::from_millis(250);
const PROBE_EMPTY_SLOT: Duration = Duration::from_secs(1);

/// XInputGetBatteryInformation from the loaded XInput, if it has it.
fn load_battery() -> Option<BatteryFn> {
    for dll in [w!("xinput1_4.dll"), w!("xinput1_3.dll")] {
        // SAFETY: loading a system DLL by name; the handle is kept for the process lifetime.
        let Ok(module) = (unsafe { LoadLibraryW(dll) }) else {
            continue;
        };
        // SAFETY: GetProcAddress by name.
        if let Some(f) = unsafe { GetProcAddress(module, s!("XInputGetBatteryInformation")) } {
            // SAFETY: XInputGetBatteryInformation has the signature of BatteryFn.
            return Some(unsafe {
                std::mem::transmute::<unsafe extern "system" fn() -> isize, BatteryFn>(f)
            });
        }
    }
    None
}

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
            let battery = load_battery();
            if battery.is_none() {
                logging::info(
                    "input",
                    "XInputGetBatteryInformation not available: no controller battery notices",
                );
            }
            run(get_state, battery, emit, guide);
        });
    if let Err(e) = spawned {
        logging::error("input", &format!("could not start the input thread: {e}"));
    }
}

fn run(
    get_state: GetStateFn,
    battery: Option<BatteryFn>,
    emit: impl Fn(PadEvent),
    guide: impl Fn(),
) {
    let mut trackers: [PadTracker; 4] = Default::default();
    let mut connected = [false; 4];
    // T2.3 notifications: each pad's last battery word and when it was read.
    let mut battery_seen: [Option<(&'static str, Instant)>; 4] = [None; 4];
    // B-12: each pad's last pointer vector sent ([0, 0] = at rest).
    let mut pointer_last = [[0i16; 2]; 4];
    let mut last_probe = [None::<Instant>; 4];
    // B-7: in the background only Guide acts; everything else is logged only.
    let gate = RefCell::new(BackgroundGate::default());
    // B-26: Guide waits for its release or 0.5 s (short / held), decided here.
    let hold = RefCell::new(crate::transit::GuideHold::default());
    let send = |kind: &'static str, control: &'static str, pad: u32, detail: &str| {
        let ev = PadEvent {
            kind,
            control,
            pad,
            seq: next_seq(),
            at_ms: super::now_ms(),
            pointer: None,
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
    // B-26: a held Guide: the Quick Menu (ignored during transitions and while a game loads).
    let held_guide = |background: bool, ms: u128, slot: u32| {
        let (kind, _, _) = crate::transit::current();
        if !crate::transit::guide_allowed(kind) {
            logging::info(
                "input",
                &format!("Guide held {ms} ms: ignored ({})", kind.name()),
            );
            return;
        }
        logging::info(
            "input",
            &format!(
                "Guide held {ms} ms: Quick Menu{}",
                if background {
                    " (AIONE comes forward)"
                } else {
                    ""
                }
            ),
        );
        if background {
            guide();
        }
        send("held", "Guide", slot, "");
    };
    let out = |e: &PadOut, slot: u32| {
        let control = CONTROLS[e.index];
        // B-22: the one held set sees every press and release, whatever its route (Guide included).
        crate::transit::with(|m| m.press(&format!("pad:{control}"), e.kind != Kind::Up));
        let foreground = crate::foreground::aione_is_foreground();
        // B-26: Guide is decided here (short at its release; held at 0.5 s in the loop).
        if control == "Guide" {
            match e.kind {
                Kind::Down => {
                    // D-053: a Guide press for every Guide rule, recorded at the press.
                    crate::launch::note_guide();
                    hold.borrow_mut().down(Instant::now(), !foreground);
                    logging::info(
                        "input",
                        &format!(
                            "pad {} down Guide (seq {}): short or held?",
                            slot + 1,
                            next_seq()
                        ),
                    );
                }
                Kind::Repeat => {}
                Kind::Up => match hold.borrow_mut().up(Instant::now()) {
                    Some(crate::transit::GuideOutcome::Short { background: true }) => {
                        logging::info("input", &format!("pad {} Guide short ({} ms) (AIONE in the background: Guide brings AIONE to the front; consumed)", slot + 1, e.held.as_millis()));
                        guide();
                    }
                    Some(crate::transit::GuideOutcome::Short { background: false }) => {
                        send("down", control, slot, " (a short Guide)");
                        send(
                            "up",
                            control,
                            slot,
                            &format!(", held {} ms", e.held.as_millis()),
                        );
                    }
                    Some(crate::transit::GuideOutcome::Held { background }) => {
                        held_guide(background, e.held.as_millis(), slot)
                    }
                    None => logging::info(
                        "input",
                        &format!(
                            "pad {} up Guide: released after the Quick Menu opened ({} ms)",
                            slot + 1,
                            e.held.as_millis()
                        ),
                    ),
                },
            }
            return;
        }
        let route = gate
            .borrow_mut()
            .route(slot, e.index, control, e.kind, foreground);
        let detail = if e.kind == Kind::Up {
            format!(", held {} ms, {} repeats", e.held.as_millis(), e.repeats)
        } else {
            String::new()
        };
        match route {
            Route::Act => send(e.kind.as_str(), control, slot, &detail),
            Route::ActGuide => {
                // D-028: the press that brings AIONE to the front is consumed: logged, never
                // passed on to the UI as an action.
                logging::info(
                    "input",
                    &format!(
                        "pad {} {} {control} (seq {}){detail} (AIONE in the background: Guide brings AIONE to the front; consumed)",
                        slot + 1,
                        e.kind.as_str(),
                        next_seq()
                    ),
                );
                guide();
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
        if let Some(crate::transit::GuideOutcome::Held { background }) =
            hold.borrow_mut().tick(Instant::now())
        {
            held_guide(background, crate::transit::GUIDE_HOLD.as_millis(), 0);
        }
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
                // B-12 (reference A7): the right stick drives AIONE's pointer, only while AIONE
                // is in front (B-7: in the background nothing but Guide acts). Sent only when its
                // step changes, so a stick at rest sends nothing.
                let pointing = pointer_vector(raw.rx, raw.ry);
                let v = if pointing != [0, 0] && !crate::foreground::aione_is_foreground() {
                    [0, 0]
                } else {
                    pointing
                };
                if v != pointer_last[i] {
                    if pointer_last[i] == [0, 0] || v == [0, 0] {
                        logging::info(
                            "input",
                            &format!(
                                "pad {} pointer {}",
                                slot + 1,
                                if v == [0, 0] {
                                    "at rest"
                                } else {
                                    "moving (right stick)"
                                }
                            ),
                        );
                    }
                    pointer_last[i] = v;
                    emit(PadEvent {
                        kind: "pointer",
                        control: "RStick",
                        pad: slot,
                        seq: next_seq(),
                        at_ms: super::now_ms(),
                        pointer: Some(v),
                    });
                }
                // T2.3 notifications: the battery, at the connection and then once a minute.
                if let Some(get_battery) = battery {
                    if battery_seen[i].is_none_or(|(_, t)| now.duration_since(t) >= BATTERY_EVERY) {
                        let mut b = XInputBattery::default();
                        // SAFETY: valid pointer; 0 = BATTERY_DEVTYPE_GAMEPAD.
                        let rc = unsafe { get_battery(slot, 0, &mut b) };
                        let word = if rc == ERROR_SUCCESS {
                            super::battery_word(b.battery_type, b.battery_level)
                        } else {
                            "unknown"
                        };
                        if battery_seen[i].map(|(w, _)| w) != Some(word) {
                            logging::info("input", &format!("pad {} battery: {word}", slot + 1));
                            send("battery", word, slot, "");
                        }
                        battery_seen[i] = Some((word, now));
                    }
                }
            } else {
                last_probe[i] = Some(now);
                if connected[i] {
                    connected[i] = false;
                    battery_seen[i] = None;
                    pointer_last[i] = [0, 0];
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
