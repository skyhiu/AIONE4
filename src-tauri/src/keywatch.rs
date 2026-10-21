//! D-047 (B-22 rev. ap): a read-only diagnostic for Windows' volume / media pop-up.
//!
//! AIONE itself never sends a key (its only synthetic input is one zero-distance mouse move, in
//! `foreground.rs`). To show where the media / volume keys in the logs come from, a low-level
//! keyboard hook observes **only** those keys (virtual-key codes 0xAD–0xB7: mute, volume down / up,
//! next / previous / stop / play-pause, mail, media select) and logs, for each press, whether Windows
//! marks it as synthesized by a program (`LLKHF_INJECTED`) or a physical key, and which program was
//! in front. Every key is passed on unchanged; nothing is blocked.
#![cfg_attr(not(windows), allow(dead_code, unused_imports))]

// ---------- B-28 (rev. ba): Alt + `, the keyboard's Guide, everywhere (short press only) ----------
// The key under Esc, matched by its POSITION (scan code 0x29), so it works on every keyboard layout
// (` on US, ^ on German, ² on French). Supersedes Win + Esc (rev. ax / ay), its hold and its
// Start-menu workaround.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// The key under Esc (its scan code, the same on every layout).
pub const BACKTICK_SCAN: u32 = 0x29;

type GuideFn = Box<dyn Fn() + Send + Sync>;
type EmitFn = Box<dyn Fn(&'static str) + Send + Sync>;
static HANDLERS: Mutex<Option<(GuideFn, EmitFn)>> = Mutex::new(None);
/// The key is down as Alt + ` (its repeats and release are the shortcut's too).
static TAKEN: AtomicBool = AtomicBool::new(false);

/// Is this key event Alt + `'s? The key under Esc with Alt held, or the rest of an Alt + ` already
/// taken (its repeats and release, even if Alt is let go first): never reaching the game or AIONE.
pub fn backtick_taken(scan: u32, alt_held: bool, already_taken: bool) -> bool {
    scan == BACKTICK_SCAN && (alt_held || already_taken)
}

/// lib: what Alt + ` does (a short Guide): come forward; or send the UI a Guide (back to the game).
pub fn set_guide(
    guide: impl Fn() + Send + Sync + 'static,
    emit: impl Fn(&'static str) + Send + Sync + 'static,
) {
    if let Ok(mut h) = HANDLERS.lock() {
        *h = Some((Box::new(guide), Box::new(emit)));
    }
}

/// The press: a short Guide (no hold, rev. ba). From the background AIONE comes forward (the game
/// paused); in front, the UI's Guide (back to the running game).
fn shortcut_pressed() {
    let background = !crate::foreground::aione_is_foreground();
    // A Guide press for every Guide rule (D-037), recorded at the press.
    crate::launch::note_guide();
    crate::logging::info(
        "input",
        &format!(
            "Alt + ` (the key under Esc): Guide{}",
            if background {
                " (AIONE comes forward)"
            } else {
                ""
            }
        ),
    );
    if let Ok(h) = HANDLERS.lock() {
        if let Some((guide, emit)) = h.as_ref() {
            if background {
                guide();
            } else {
                emit("down");
                emit("up");
            }
        }
    }
}

#[cfg(windows)]
pub fn start() {
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN,
        WM_SYSKEYDOWN,
    };
    unsafe extern "system" fn hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
        if code >= 0 {
            let k = unsafe { &*(l.0 as *const KBDLLHOOKSTRUCT) };
            // B-28 (rev. ba): Alt + ` (a real key, not a synthetic one), short press only.
            if k.scanCode == BACKTICK_SCAN && k.flags.0 & LLKHF_INJECTED.0 == 0 {
                let alt = k.flags.0 & LLKHF_ALTDOWN.0 != 0;
                let down = w.0 as u32 == WM_KEYDOWN || w.0 as u32 == WM_SYSKEYDOWN;
                if backtick_taken(k.scanCode, alt, TAKEN.load(Ordering::SeqCst)) {
                    if down {
                        if !TAKEN.swap(true, Ordering::SeqCst) {
                            std::thread::spawn(shortcut_pressed); // the hook must stay fast
                        }
                    } else {
                        TAKEN.store(false, Ordering::SeqCst);
                    }
                    return LRESULT(1); // taken: never passed to the game, Windows or AIONE
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
    crate::logging::lifecycle(
        crate::logging::Level::Info,
        "keyboard shortcut: Alt + ` (the key under Esc, scan code 0x29, on any layout): a short press comes to AIONE (the game paused) or goes back",
    );
    std::thread::spawn(|| {
        // SAFETY: a low-level hook with a static callback for AIONE's lifetime; it only observes,
        // except Alt + ` (B-28), which it takes.
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
    /// B-28 rev. ba: Alt + ` (by position) is taken whole; ` alone and other keys are not.
    #[test]
    fn alt_backtick_is_taken_whole() {
        use super::{backtick_taken, BACKTICK_SCAN};
        assert!(backtick_taken(BACKTICK_SCAN, true, false)); // Alt + `
        assert!(backtick_taken(BACKTICK_SCAN, false, true)); // Alt let go first: the rest still taken
        assert!(!backtick_taken(BACKTICK_SCAN, false, false)); // ` alone: the game's (or AIONE's)
        assert!(!backtick_taken(0x01, true, false)); // Alt + Esc: not ours
    }

    #[test]
    fn names_the_media_and_volume_keys() {
        assert_eq!(super::name(0xAE), "VolumeDown");
        assert_eq!(super::name(0xB0), "MediaNextTrack");
        assert_eq!(super::name(0x41), "other");
    }
}
