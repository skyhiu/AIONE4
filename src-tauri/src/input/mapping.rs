//! Pure controller mapping (no OS calls, fully tested): raw XInput state in, normalized
//! button events out. Sticks and triggers become buttons with hysteresis, so a stick resting
//! near the threshold can never turn one push into two presses.

use std::time::{Duration, Instant};

/// Raw XInput gamepad state (XINPUT_GAMEPAD).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RawPad {
    pub buttons: u16,
    pub lt: u8,
    pub rt: u8,
    pub lx: i16,
    pub ly: i16,
    pub rx: i16,
    pub ry: i16,
}

/// Every control AIONE knows, in a fixed order (index = bit in `PadTracker::held`).
pub const CONTROLS: [&str; 25] = [
    "A",
    "B",
    "X",
    "Y",
    "LB",
    "RB",
    "LT",
    "RT",
    "View",
    "Menu",
    "Guide",
    "LS",
    "RS",
    "DPadUp",
    "DPadDown",
    "DPadLeft",
    "DPadRight",
    "LStickUp",
    "LStickDown",
    "LStickLeft",
    "LStickRight",
    "RStickUp",
    "RStickDown",
    "RStickLeft",
    "RStickRight",
];

/// XInput button masks → control index.
const BUTTON_BITS: [(u16, usize); 15] = [
    (0x1000, 0),  // A
    (0x2000, 1),  // B
    (0x4000, 2),  // X
    (0x8000, 3),  // Y
    (0x0100, 4),  // LB
    (0x0200, 5),  // RB
    (0x0020, 8),  // Back = View
    (0x0010, 9),  // Start = Menu
    (0x0400, 10), // Guide (XInputGetStateEx only)
    (0x0040, 11), // LS (left stick click)
    (0x0080, 12), // RS
    (0x0001, 13), // DPadUp
    (0x0002, 14), // DPadDown
    (0x0004, 15), // DPadLeft
    (0x0008, 16), // DPadRight
];

const LT: usize = 6;
const RT: usize = 7;
const LSTICK: usize = 17; // Up, Down, Left, Right follow
const RSTICK: usize = 21;

/// Tunable thresholds and timing (first values; to be confirmed against the reference feel).
#[derive(Debug, Clone, Copy)]
pub struct Tuning {
    /// Stick component that counts as a press (of 32767).
    pub stick_press: i32,
    /// Below this the held stick direction is released (hysteresis).
    pub stick_release: i32,
    pub trigger_press: u8,
    pub trigger_release: u8,
    pub repeat_delay: Duration,
    pub repeat_interval: Duration,
}

impl Default for Tuning {
    fn default() -> Self {
        Tuning {
            stick_press: 16_000,   // ~49 %
            stick_release: 11_500, // ~35 %
            trigger_press: 64,
            trigger_release: 32,
            repeat_delay: Duration::from_millis(400),
            repeat_interval: Duration::from_millis(90),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Down,
    Up,
    Repeat,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Down => "down",
            Kind::Up => "up",
            Kind::Repeat => "repeat",
        }
    }
}

/// One output event of the tracker. For `Up`, `held` and `repeats` describe the hold that
/// just ended (logged as "held N ms, R repeats").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PadOut {
    pub kind: Kind,
    pub index: usize,
    pub held: Duration,
    pub repeats: u32,
}

/// Only navigation directions repeat while held; buttons never do (one press, one action).
pub fn repeats(index: usize) -> bool {
    (13..=20).contains(&index)
}

#[derive(Debug, Clone)]
pub struct PadTracker {
    pub tuning: Tuning,
    held: u32,
    next_repeat: [Option<Instant>; 25],
    down_at: [Option<Instant>; 25],
    repeat_count: [u32; 25],
}

impl Default for PadTracker {
    fn default() -> Self {
        Self::new(Tuning::default())
    }
}

impl PadTracker {
    pub fn new(tuning: Tuning) -> Self {
        PadTracker {
            tuning,
            held: 0,
            next_repeat: [None; 25],
            down_at: [None; 25],
            repeat_count: [0; 25],
        }
    }

    pub fn is_held(&self, index: usize) -> bool {
        self.held & (1 << index) != 0
    }

    /// Computes the controls held for `raw`, given what was held before (hysteresis).
    fn target(&self, raw: &RawPad) -> u32 {
        let mut t = 0u32;
        for (mask, idx) in BUTTON_BITS {
            if raw.buttons & mask != 0 {
                t |= 1 << idx;
            }
        }
        let tr = |value: u8, idx: usize| -> bool {
            if self.is_held(idx) {
                value >= self.tuning.trigger_release
            } else {
                value >= self.tuning.trigger_press
            }
        };
        if tr(raw.lt, LT) {
            t |= 1 << LT;
        }
        if tr(raw.rt, RT) {
            t |= 1 << RT;
        }
        t |= self.stick(raw.lx, raw.ly, LSTICK);
        t |= self.stick(raw.rx, raw.ry, RSTICK);
        t
    }

    /// One direction per stick: the held one stays while its component is above the release
    /// threshold; otherwise the dominant axis above the press threshold is pressed.
    fn stick(&self, x: i16, y: i16, base: usize) -> u32 {
        let (x, y) = (x as i32, y as i32);
        // Order: Up, Down, Left, Right. XInput Y is positive up.
        let comps = [y, -y, -x, x];
        for (i, c) in comps.iter().enumerate() {
            if self.is_held(base + i) && *c >= self.tuning.stick_release {
                return 1 << (base + i);
            }
        }
        let (ax, ay) = (x.abs(), y.abs());
        if ax.max(ay) < self.tuning.stick_press {
            return 0;
        }
        let dir = if ay >= ax {
            if y > 0 {
                0
            } else {
                1
            }
        } else if x < 0 {
            2
        } else {
            3
        };
        1 << (base + dir)
    }

    /// Applies one poll. Returns events in a stable order (control index order).
    pub fn update(&mut self, raw: &RawPad, now: Instant) -> Vec<PadOut> {
        let target = self.target(raw);
        let mut out = Vec::new();
        for i in 0..CONTROLS.len() {
            let was = self.held & (1 << i) != 0;
            let is = target & (1 << i) != 0;
            match (was, is) {
                (false, true) => {
                    out.push(self.event(Kind::Down, i, now));
                    self.down_at[i] = Some(now);
                    self.repeat_count[i] = 0;
                    if repeats(i) {
                        self.next_repeat[i] = Some(now + self.tuning.repeat_delay);
                        // B-3 (rev. u, D-019): only the newest held direction repeats, so a held
                        // diagonal never repeats two directions at once ("staircase").
                        for j in (0..CONTROLS.len()).filter(|j| *j != i && repeats(*j)) {
                            self.next_repeat[j] = None;
                        }
                    }
                }
                (true, false) => {
                    out.push(self.event(Kind::Up, i, now));
                    self.next_repeat[i] = None;
                    self.down_at[i] = None;
                    // When the repeating direction is released, the most recent one still held
                    // resumes repeating after the usual delay.
                    if repeats(i) {
                        let resume = (0..CONTROLS.len())
                            .filter(|j| {
                                *j != i
                                    && repeats(*j)
                                    && target & (1 << *j) != 0
                                    && self.held & (1 << *j) != 0
                            })
                            .max_by_key(|j| self.down_at[*j]);
                        if let Some(j) = resume {
                            if self
                                .next_repeat
                                .iter()
                                .enumerate()
                                .all(|(k, n)| n.is_none() || !repeats(k))
                            {
                                self.next_repeat[j] = Some(now + self.tuning.repeat_delay);
                            }
                        }
                    }
                }
                (true, true) => {
                    if let Some(at) = self.next_repeat[i] {
                        if now >= at {
                            self.repeat_count[i] += 1;
                            out.push(self.event(Kind::Repeat, i, now));
                            // Never burst to catch up after a stall: one repeat per poll.
                            let next = at + self.tuning.repeat_interval;
                            self.next_repeat[i] = Some(if next <= now {
                                now + self.tuning.repeat_interval
                            } else {
                                next
                            });
                        }
                    }
                }
                (false, false) => {}
            }
        }
        self.held = target;
        out
    }

    fn event(&self, kind: Kind, index: usize, now: Instant) -> PadOut {
        PadOut {
            kind,
            index,
            held: self.down_at[index]
                .map(|t| now.saturating_duration_since(t))
                .unwrap_or_default(),
            repeats: self.repeat_count[index],
        }
    }

    /// Releases everything (pad disconnected): an Up for every held control.
    pub fn release_all(&mut self, now: Instant) -> Vec<PadOut> {
        let out = (0..CONTROLS.len())
            .filter(|i| self.is_held(*i))
            .map(|i| self.event(Kind::Up, i, now))
            .collect();
        self.held = 0;
        self.next_repeat = [None; 25];
        self.down_at = [None; 25];
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(ev: &[PadOut]) -> Vec<String> {
        ev.iter()
            .map(|e| format!("{} {}", e.kind.as_str(), CONTROLS[e.index]))
            .collect()
    }

    /// D-019 (B-3 rev. u): Up + Left held together: only the newest (Left) repeats; when Left is
    /// released, Up resumes repeating after the 400 ms delay.
    #[test]
    fn only_the_newest_held_direction_repeats() {
        let mut t = PadTracker::default();
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let pad = |b: u16| RawPad {
            buttons: b,
            ..RawPad::default()
        };
        const UP: u16 = 0x0001;
        const LEFT: u16 = 0x0004;
        let _ = t.update(&pad(UP), ms(0));
        let _ = t.update(&pad(UP | LEFT), ms(100));
        let mut kinds = Vec::new();
        for n in (110..=1500).step_by(10) {
            for e in t.update(&pad(UP | LEFT), ms(n)) {
                if e.kind == Kind::Repeat {
                    kinds.push(CONTROLS[e.index]);
                }
            }
        }
        assert!(!kinds.is_empty());
        assert!(kinds.iter().all(|c| *c == "DPadLeft"), "{kinds:?}");
        // Left released at 1500 ms: Up resumes 400 ms later, alone.
        let _ = t.update(&pad(UP), ms(1510));
        let mut after = Vec::new();
        for n in (1520..=2400).step_by(10) {
            for e in t.update(&pad(UP), ms(n)) {
                if e.kind == Kind::Repeat {
                    after.push((n, CONTROLS[e.index]));
                }
            }
        }
        assert!(after.iter().all(|(_, c)| *c == "DPadUp"), "{after:?}");
        assert!(after.first().is_some_and(|(n, _)| *n >= 1910), "{after:?}");
    }

    #[test]
    fn one_press_is_one_down_and_one_up() {
        let mut t = PadTracker::default();
        let now = Instant::now();
        let a = RawPad {
            buttons: 0x1000,
            ..Default::default()
        };
        assert_eq!(names(&t.update(&a, now)), ["down A"]);
        // Held across many polls: no extra events (A never repeats).
        for i in 1..100 {
            assert!(t.update(&a, now + Duration::from_millis(8 * i)).is_empty());
        }
        assert_eq!(
            names(&t.update(&RawPad::default(), now + Duration::from_secs(1))),
            ["up A"]
        );
    }

    #[test]
    fn guide_and_view_menu_are_named() {
        let mut t = PadTracker::default();
        let raw = RawPad {
            buttons: 0x0400 | 0x0020 | 0x0010,
            ..Default::default()
        };
        assert_eq!(
            names(&t.update(&raw, Instant::now())),
            ["down View", "down Menu", "down Guide"]
        );
    }

    #[test]
    fn stick_hysteresis_prevents_double_presses() {
        let mut t = PadTracker::default();
        let now = Instant::now();
        let at = |x: i16| RawPad {
            lx: x,
            ..Default::default()
        };
        assert_eq!(names(&t.update(&at(17_000), now)), ["down LStickRight"]);
        // Wobbling between press and release thresholds keeps it held.
        assert!(t.update(&at(15_000), now).is_empty());
        assert!(t.update(&at(12_000), now).is_empty());
        assert!(t.update(&at(16_500), now).is_empty());
        assert_eq!(names(&t.update(&at(5_000), now)), ["up LStickRight"]);
        // Below the press threshold nothing happens.
        assert!(t.update(&at(15_000), now).is_empty());
    }

    #[test]
    fn stick_uses_the_dominant_axis_and_up_is_positive_y() {
        let mut t = PadTracker::default();
        let raw = RawPad {
            lx: 17_000,
            ly: 20_000,
            ..Default::default()
        };
        assert_eq!(names(&t.update(&raw, Instant::now())), ["down LStickUp"]);
        let mut t = PadTracker::default();
        let raw = RawPad {
            rx: -30_000,
            ry: -1_000,
            ..Default::default()
        };
        assert_eq!(names(&t.update(&raw, Instant::now())), ["down RStickLeft"]);
    }

    #[test]
    fn stick_flick_to_the_other_side_releases_then_presses() {
        let mut t = PadTracker::default();
        let now = Instant::now();
        t.update(
            &RawPad {
                lx: -30_000,
                ..Default::default()
            },
            now,
        );
        let ev = t.update(
            &RawPad {
                lx: 30_000,
                ..Default::default()
            },
            now,
        );
        assert_eq!(names(&ev), ["up LStickLeft", "down LStickRight"]);
    }

    #[test]
    fn triggers_have_hysteresis() {
        let mut t = PadTracker::default();
        let now = Instant::now();
        assert!(t
            .update(
                &RawPad {
                    lt: 50,
                    ..Default::default()
                },
                now
            )
            .is_empty());
        assert_eq!(
            names(&t.update(
                &RawPad {
                    lt: 70,
                    ..Default::default()
                },
                now
            )),
            ["down LT"]
        );
        assert!(t
            .update(
                &RawPad {
                    lt: 40,
                    ..Default::default()
                },
                now
            )
            .is_empty());
        assert_eq!(
            names(&t.update(
                &RawPad {
                    lt: 10,
                    ..Default::default()
                },
                now
            )),
            ["up LT"]
        );
    }

    #[test]
    fn directions_repeat_after_the_delay_at_the_interval() {
        let mut t = PadTracker::default();
        let t0 = Instant::now();
        let down = RawPad {
            buttons: 0x0002,
            ..Default::default()
        };
        assert_eq!(names(&t.update(&down, t0)), ["down DPadDown"]);
        assert!(t.update(&down, t0 + Duration::from_millis(399)).is_empty());
        assert_eq!(
            names(&t.update(&down, t0 + Duration::from_millis(400))),
            ["repeat DPadDown"]
        );
        assert!(t.update(&down, t0 + Duration::from_millis(450)).is_empty());
        assert_eq!(
            names(&t.update(&down, t0 + Duration::from_millis(490))),
            ["repeat DPadDown"]
        );
        // A long stall gives one repeat, not a burst.
        assert_eq!(t.update(&down, t0 + Duration::from_millis(2000)).len(), 1);
        assert!(t.update(&down, t0 + Duration::from_millis(2001)).is_empty());
    }

    #[test]
    fn release_reports_hold_length_and_repeats() {
        let mut t = PadTracker::default();
        let t0 = Instant::now();
        let down = RawPad {
            buttons: 0x0008,
            ..Default::default()
        };
        t.update(&down, t0);
        for ms in [400u64, 490, 580] {
            t.update(&down, t0 + Duration::from_millis(ms));
        }
        let up = t.update(&RawPad::default(), t0 + Duration::from_millis(600));
        assert_eq!(up.len(), 1);
        assert_eq!(
            (up[0].kind, up[0].held, up[0].repeats),
            (Kind::Up, Duration::from_millis(600), 3)
        );
        // A new press starts counting from zero; buttons report 0 repeats.
        let a = RawPad {
            buttons: 0x1000,
            ..Default::default()
        };
        t.update(&a, t0 + Duration::from_secs(1));
        let up = t.update(&RawPad::default(), t0 + Duration::from_secs(3));
        assert_eq!((up[0].held, up[0].repeats), (Duration::from_secs(2), 0));
    }

    #[test]
    fn right_stick_never_repeats() {
        let mut t = PadTracker::default();
        let t0 = Instant::now();
        let r = RawPad {
            rx: 30_000,
            ..Default::default()
        };
        assert_eq!(names(&t.update(&r, t0)), ["down RStickRight"]);
        for ms in (8..3000).step_by(8) {
            assert!(t.update(&r, t0 + Duration::from_millis(ms)).is_empty());
        }
    }

    #[test]
    fn release_all_on_disconnect() {
        let mut t = PadTracker::default();
        t.update(
            &RawPad {
                buttons: 0x1000 | 0x0001,
                ..Default::default()
            },
            Instant::now(),
        );
        let now = Instant::now();
        assert_eq!(names(&t.release_all(now)), ["up A", "up DPadUp"]);
        assert!(t.release_all(now).is_empty());
    }
}
