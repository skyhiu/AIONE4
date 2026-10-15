//! B-7 (Master Prompt rev. f): while AIONE is in the background, it keeps reading the pad (so
//! Guide can bring it back) but **acts only on Guide**. Every other press, repeat and release is
//! logged and never reaches the UI.
//!
//! A press that **began** in the background stays background-only until it is released, even if
//! AIONE comes to the front while it is held: its repeats never start moving the focus.
//! A press that began in the front and is released in the background still sends its release
//! (the UI never acts on releases; it keeps the UI's view of held buttons consistent).
//!
//! Pure logic (no OS calls); the XInput loop asks `aione_is_foreground()` per event.

use super::mapping::Kind;

/// What happens to one pad event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// AIONE is in front: the UI gets the event.
    Act,
    /// AIONE is in the background and the control is Guide: the UI gets the event, and the
    /// core brings AIONE to the front (on the press).
    ActGuide,
    /// Logged only (background).
    LogOnly,
}

pub const GUIDE: &str = "Guide";

/// Per slot, one bit per control index: "this press began in the background".
#[derive(Debug, Clone, Default)]
pub struct BackgroundGate {
    began_in_background: [u32; 4],
}

impl BackgroundGate {
    pub fn route(
        &mut self,
        slot: u32,
        index: usize,
        control: &str,
        kind: Kind,
        foreground: bool,
    ) -> Route {
        let s = (slot as usize).min(3);
        let bit = 1u32 << index;
        let began_bg = self.began_in_background[s] & bit != 0;
        match kind {
            Kind::Down => {
                if foreground {
                    self.began_in_background[s] &= !bit;
                    Route::Act
                } else if control == GUIDE {
                    // Guide acts in the background (it brings AIONE to the front). D-028: the
                    // press is consumed there: marked, so its repeats and release are only logged
                    // and never reach the UI as an action (only a NEW press in front does).
                    self.began_in_background[s] |= bit;
                    Route::ActGuide
                } else {
                    self.began_in_background[s] |= bit;
                    Route::LogOnly
                }
            }
            Kind::Repeat => {
                if began_bg || !foreground {
                    Route::LogOnly
                } else {
                    Route::Act
                }
            }
            Kind::Up => {
                if began_bg {
                    self.began_in_background[s] &= !bit;
                    Route::LogOnly
                } else {
                    Route::Act
                }
            }
        }
    }
}

/// The note added to a pad log line that was not acted on.
pub const BACKGROUND_NOTE: &str = " (AIONE in the background: logged, not acted on)";

#[cfg(test)]
mod tests {
    use super::*;

    const A: usize = 0;
    const GUIDE_I: usize = 10;
    const DOWN: usize = 14;

    #[test]
    fn in_front_everything_acts() {
        let mut g = BackgroundGate::default();
        assert_eq!(g.route(0, A, "A", Kind::Down, true), Route::Act);
        assert_eq!(g.route(0, A, "A", Kind::Up, true), Route::Act);
        assert_eq!(g.route(0, DOWN, "DPadDown", Kind::Down, true), Route::Act);
        assert_eq!(g.route(0, DOWN, "DPadDown", Kind::Repeat, true), Route::Act);
    }

    #[test]
    fn in_the_background_only_guide_acts() {
        let mut g = BackgroundGate::default();
        for (i, name) in [
            (A, "A"),
            (1, "B"),
            (DOWN, "DPadDown"),
            (17, "LStickUp"),
            (9, "Menu"),
        ] {
            assert_eq!(
                g.route(0, i, name, Kind::Down, false),
                Route::LogOnly,
                "{name}"
            );
            assert_eq!(
                g.route(0, i, name, Kind::Up, false),
                Route::LogOnly,
                "{name}"
            );
        }
        assert_eq!(
            g.route(0, GUIDE_I, GUIDE, Kind::Down, false),
            Route::ActGuide
        );
        // D-028: that press is consumed: its repeats and release are only logged, even once AIONE
        // is in front (it brought AIONE there).
        assert_eq!(
            g.route(0, GUIDE_I, GUIDE, Kind::Repeat, true),
            Route::LogOnly
        );
        assert_eq!(g.route(0, GUIDE_I, GUIDE, Kind::Up, true), Route::LogOnly);
        // A NEW Guide press with AIONE in front acts (it returns to the running game).
        assert_eq!(g.route(0, GUIDE_I, GUIDE, Kind::Down, true), Route::Act);
        assert_eq!(g.route(0, GUIDE_I, GUIDE, Kind::Up, true), Route::Act);
    }

    #[test]
    fn a_hold_that_began_in_the_background_never_acts() {
        let mut g = BackgroundGate::default();
        assert_eq!(
            g.route(0, DOWN, "DPadDown", Kind::Down, false),
            Route::LogOnly
        );
        assert_eq!(
            g.route(0, DOWN, "DPadDown", Kind::Repeat, false),
            Route::LogOnly
        );
        // AIONE comes to the front while the direction is still held: still not acted on.
        assert_eq!(
            g.route(0, DOWN, "DPadDown", Kind::Repeat, true),
            Route::LogOnly
        );
        assert_eq!(g.route(0, DOWN, "DPadDown", Kind::Up, true), Route::LogOnly);
        // The next press in front acts normally.
        assert_eq!(g.route(0, DOWN, "DPadDown", Kind::Down, true), Route::Act);
    }

    #[test]
    fn a_hold_that_began_in_front_stops_repeating_in_the_background() {
        let mut g = BackgroundGate::default();
        assert_eq!(g.route(0, DOWN, "DPadDown", Kind::Down, true), Route::Act);
        assert_eq!(g.route(0, DOWN, "DPadDown", Kind::Repeat, true), Route::Act);
        assert_eq!(
            g.route(0, DOWN, "DPadDown", Kind::Repeat, false),
            Route::LogOnly
        );
        // Its release still reaches the UI (the UI never acts on releases).
        assert_eq!(g.route(0, DOWN, "DPadDown", Kind::Up, false), Route::Act);
    }

    #[test]
    fn slots_are_independent() {
        let mut g = BackgroundGate::default();
        assert_eq!(g.route(0, A, "A", Kind::Down, false), Route::LogOnly);
        assert_eq!(g.route(1, A, "A", Kind::Down, true), Route::Act);
        assert_eq!(g.route(1, A, "A", Kind::Up, true), Route::Act);
        assert_eq!(g.route(0, A, "A", Kind::Up, true), Route::LogOnly);
    }
}
