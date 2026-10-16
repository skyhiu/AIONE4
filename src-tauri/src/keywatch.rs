//! D-047 (B-22 rev. ap): a read-only diagnostic for Windows' volume / media pop-up.
//!
//! AIONE itself never sends a key (its only synthetic input is one zero-distance mouse move, in
//! `foreground.rs`). To show where the media / volume keys in the logs come from, a low-level
//! keyboard hook observes **only** those keys (virtual-key codes 0xAD–0xB7: mute, volume down / up,
//! next / previous / stop / play-pause, mail, media select) and logs, for each press, whether Windows
//! marks it as synthesized by a program (`LLKHF_INJECTED`) or a physical key, and which program was
//! in front. Every key is passed on unchanged; nothing is blocked.

#[cfg(windows)]
pub fn start() {
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
    };
    unsafe extern "system" fn hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
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
    std::thread::spawn(|| {
        // SAFETY: a low-level hook with a static callback for AIONE's lifetime; it only reads.
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
