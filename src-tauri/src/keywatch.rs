//! D-047 (B-22 rev. ap): a read-only diagnostic for Windows' volume / media pop-up.
//!
//! AIONE itself never sends a key (its only synthetic input is one zero-distance mouse move, in
//! `foreground.rs`). To show where the media / volume keys in the logs come from, a low-level
//! keyboard hook observes **only** those keys (virtual-key codes 0xAD–0xB7: mute, volume down / up,
//! next / previous / stop / play-pause, mail, media select) and logs, for each press, whether Windows
//! marks it as synthesized by a program (`LLKHF_INJECTED`) or a physical key, and which program was
//! in front. Every key is passed on unchanged; nothing is blocked.
#![cfg_attr(not(windows), allow(dead_code, unused_imports))]

// ---------- B-28 (rev. ax): Win + Esc, the keyboard's Guide, everywhere ----------
// (The hook that uses these is Windows-only.)

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

type GuideFn = Box<dyn Fn() + Send + Sync>;
type EmitFn = Box<dyn Fn(&'static str) + Send + Sync>;
static HANDLERS: Mutex<Option<(GuideFn, EmitFn)>> = Mutex::new(None);
static HOLD: LazyLock<Mutex<crate::transit::GuideHold>> =
    LazyLock::new(|| Mutex::new(crate::transit::GuideHold::default()));
/// Esc is down as Win + Esc (its repeats are ignored; its release decides).
static ESC_DOWN: AtomicBool = AtomicBool::new(false);
/// Windows' Magnifier uses Win + Esc (to close it): while it runs, the keys are left to Windows.
static MAGNIFIER: AtomicBool = AtomicBool::new(false);

/// lib: what Win + Esc does (the same as the pad's Guide): come forward; send the UI a Guide event.
pub fn set_guide(
    guide: impl Fn() + Send + Sync + 'static,
    emit: impl Fn(&'static str) + Send + Sync + 'static,
) {
    if let Ok(mut h) = HANDLERS.lock() {
        *h = Some((Box::new(guide), Box::new(emit)));
    }
}

fn with_handlers(f: impl FnOnce(&GuideFn, &EmitFn)) {
    if let Ok(h) = HANDLERS.lock() {
        if let Some((g, e)) = h.as_ref() {
            f(g, e);
        }
    }
}

/// The same outcomes as the pad's Guide (B-26): short → today's Guide; held → the Quick Menu.
fn outcome(o: crate::transit::GuideOutcome, ms: u128) {
    use crate::transit::GuideOutcome;
    match o {
        GuideOutcome::Short { background: true } => {
            crate::logging::info("input", &format!("Win + Esc short ({ms} ms) (AIONE in the background: brings AIONE to the front)"));
            with_handlers(|g, _| g());
        }
        GuideOutcome::Short { background: false } => {
            crate::logging::info("input", &format!("Win + Esc short ({ms} ms): Guide"));
            with_handlers(|_, e| {
                e("down");
                e("up");
            });
        }
        GuideOutcome::Held { background } => {
            let (kind, _, _) = crate::transit::current();
            if !crate::transit::guide_allowed(kind) {
                crate::logging::info(
                    "input",
                    &format!("Win + Esc held {ms} ms: ignored ({})", kind.name()),
                );
                return;
            }
            crate::logging::info(
                "input",
                &format!(
                    "Win + Esc held {ms} ms: Quick Menu{}",
                    if background {
                        " (AIONE comes forward)"
                    } else {
                        ""
                    }
                ),
            );
            with_handlers(|g, e| {
                if background {
                    g();
                }
                e("held");
            });
        }
    }
}

#[cfg(windows)]
fn esc_with_win(down: bool) {
    use std::time::Instant;
    if down {
        if ESC_DOWN.swap(true, Ordering::SeqCst) {
            return; // a repeat
        }
        let background = !crate::foreground::aione_is_foreground();
        if let Ok(mut h) = HOLD.lock() {
            h.down(Instant::now(), background);
        }
        cancel_start_menu();
        std::thread::spawn(|| {
            std::thread::sleep(crate::transit::GUIDE_HOLD + std::time::Duration::from_millis(20));
            let o = HOLD.lock().ok().and_then(|mut h| h.tick(Instant::now()));
            if let Some(o) = o {
                outcome(o, crate::transit::GUIDE_HOLD.as_millis());
            }
        });
    } else {
        if !ESC_DOWN.swap(false, Ordering::SeqCst) {
            return;
        }
        let o = HOLD.lock().ok().and_then(|mut h| h.up(Instant::now()));
        if let Some(o) = o {
            std::thread::spawn(move || outcome(o, 0));
        }
    }
}

/// The Windows key's release alone would open the Start menu: one unassigned key (0xE8) between
/// keeps it closed. AIONE's synthetic input (D-047): logged, never counted as a press.
#[cfg(windows)]
fn cancel_start_menu() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    };
    let key = |flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0xE8),
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    let inputs = [key(Default::default()), key(KEYEVENTF_KEYUP)];
    unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    crate::logging::lifecycle(crate::logging::Level::Info, "foreground: synthetic input sent (the unassigned key 0xE8, so the Windows key doesn't open the Start menu)");
}

#[cfg(windows)]
pub fn start() {
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
    };
    unsafe extern "system" fn hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
        if code >= 0 {
            let k = unsafe { &*(l.0 as *const KBDLLHOOKSTRUCT) };
            // B-28: Win + Esc (a real key, not AIONE's own), unless Magnifier runs.
            if k.vkCode == 0x1B
                && k.flags.0 & LLKHF_INJECTED.0 == 0
                && !MAGNIFIER.load(Ordering::Relaxed)
            {
                use windows::Win32::UI::Input::KeyboardAndMouse::{
                    GetAsyncKeyState, VK_LWIN, VK_RWIN,
                };
                let win = unsafe { GetAsyncKeyState(VK_LWIN.0 as i32) } < 0
                    || unsafe { GetAsyncKeyState(VK_RWIN.0 as i32) } < 0;
                let down = w.0 as u32 == WM_KEYDOWN || w.0 as u32 == WM_SYSKEYDOWN;
                if win || (!down && ESC_DOWN.load(Ordering::SeqCst)) {
                    esc_with_win(down);
                    return LRESULT(1); // taken: never passed to the game or Windows
                }
            }
        }
        if code >= 0 && (w.0 as u32 == WM_KEYDOWN || w.0 as u32 == WM_SYSKEYDOWN) {
            let k = unsafe { &*(l.0 as *const KBDLLHOOKSTRUCT) };
            if (0xAD..=0xB7).contains(&k.vkCode) {
                let injected = k.flags.0 & LLKHF_INJECTED.0 != 0;
                let vk = k.vkCode;
                std::thread::spawn(move || {
                    let front =
                        crate::launch::foreground_name().unwrap_or_else(|| "unknown".into());
                    crate::logging::lifecycle(
                        crate::logging::Level::Info,
                        &format!(
                            "media / volume key {} ({:#04x}): {} while {front} was in front (not from AIONE)",
                            name(vk),
                            vk,
                            if injected { "synthesized by a program" } else { "a physical key" }
                        ),
                    );
                });
            }
        }
        unsafe { CallNextHookEx(None, code, w, l) }
    }
    // B-28: whether Windows' Magnifier runs (it owns Win + Esc then), refreshed every 2 s.
    std::thread::spawn(|| loop {
        let running = crate::launch::process_names()
            .iter()
            .any(|n| n.eq_ignore_ascii_case("Magnify.exe"));
        if MAGNIFIER.swap(running, Ordering::Relaxed) != running {
            crate::logging::info(
                "input",
                if running {
                    "Magnifier runs: Win + Esc is left to Windows"
                } else {
                    "Magnifier closed: Win + Esc is AIONE's Guide again"
                },
            );
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    });
    std::thread::spawn(|| {
        // SAFETY: a low-level hook with a static callback for AIONE's lifetime; it only observes,
        // except Win + Esc (B-28), which it takes.
        let h = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), None, 0) };
        if h.is_err() {
            crate::logging::lifecycle(
                crate::logging::Level::Warn,
                "keywatch: the media-key diagnostic could not be installed",
            );
            return;
        }
        let mut msg = MSG::default();
        while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });
}

#[cfg(not(windows))]
pub fn start() {}

/// The key's name for the log.
pub fn name(vk: u32) -> &'static str {
    match vk {
        0xAD => "VolumeMute",
        0xAE => "VolumeDown",
        0xAF => "VolumeUp",
        0xB0 => "MediaNextTrack",
        0xB1 => "MediaPrevTrack",
        0xB2 => "MediaStop",
        0xB3 => "MediaPlayPause",
        0xB4 => "LaunchMail",
        0xB5 => "LaunchMediaSelect",
        0xB6 => "LaunchApp1",
        0xB7 => "LaunchApp2",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn names_the_media_and_volume_keys() {
        assert_eq!(super::name(0xAE), "VolumeDown");
        assert_eq!(super::name(0xB0), "MediaNextTrack");
        assert_eq!(super::name(0x41), "other");
    }
}
