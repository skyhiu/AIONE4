//! B-22 (rev. an): every input-locking state can be left, and one owner decides.
//!
//! One state machine, in the core, owns:
//! - the **current input-locking state** (Boot Up, Boot Down, launching a game, the launcher wait,
//!   going back to the game, closing the game, a dialog), with its game and its start;
//! - the **set of held buttons and keys**: every pad press **and release** (Guide included) is
//!   recorded here, where the core reads the pad; the UI reports keyboard keys (the only place they
//!   arrive). The UI asks; it keeps no copy (D-041: a UI copy missed Guide's release).
//!
//! Each state has a **limit** (`limit`), an **escape** (B, triggered by the UI), is **cleared** by
//! the game's end, a new launch or a new pause, and is **logged** (started, why it waits, how it
//! ended). During a launch, a launcher wait, a return or a close, **Guide does nothing in the core**
//! (`guide_allowed`), so AIONE never takes the front then (D-042).
//!
//! The rules are pure functions of an explicit clock (`Machine`), so tests drive them with seeded,
//! reproducible button-mashing; the app uses one `Machine` behind a lock (`global`).

use std::collections::BTreeSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    None,
    BootUp,
    BootDown,
    Launching,
    LauncherWait,
    Returning,
    Closing,
    Dialog,
    /// B-22 rev. ap: from the game's window appearing until it has stayed put for about 3 s.
    Loading,
}

impl Kind {
    pub fn parse(s: &str) -> Kind {
        match s {
            "boot-up" => Kind::BootUp,
            "boot-down" => Kind::BootDown,
            "launching" => Kind::Launching,
            "launcher-wait" => Kind::LauncherWait,
            "returning" => Kind::Returning,
            "closing" => Kind::Closing,
            "dialog" => Kind::Dialog,
            "loading" => Kind::Loading,
            _ => Kind::None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Kind::None => "none",
            Kind::BootUp => "Boot Up",
            Kind::BootDown => "Boot Down",
            Kind::Launching => "launching a game",
            Kind::LauncherWait => "the launcher wait",
            Kind::Returning => "going back to the game",
            Kind::Closing => "closing the game",
            Kind::Dialog => "a dialog",
            Kind::Loading => "the game is still loading",
        }
    }
}

/// B-22's table: each state's limit (None: answered by the user; Boot Up has its own skip and cap).
pub fn limit(kind: Kind) -> Option<Duration> {
    match kind {
        Kind::None | Kind::Dialog => None,
        Kind::BootUp | Kind::BootDown => Some(Duration::from_secs(10)),
        Kind::Launching => Some(Duration::from_secs(46)), // the UI says "Still starting" at 45 s
        Kind::LauncherWait => Some(Duration::from_secs(6)), // the core's 5 s wait, plus a margin
        Kind::Returning => Some(Duration::from_secs(3)),  // then resume anyway
        Kind::Closing => Some(Duration::from_millis(7000)), // 3 s, AIONE ends it, up to 3 s more
        Kind::Loading => Some(Duration::from_secs(21)),   // stable 3 s, or the 20 s cap
    }
}

/// B-22 rev. ap: the game has settled once it has stayed put for 3 s (the same window, size and
/// position, in front), or after the 20 s cap.
pub fn settled(stable_for: Duration, since_window: Duration) -> Option<&'static str> {
    if stable_for >= Duration::from_secs(3) {
        Some("settled")
    } else if since_window >= Duration::from_secs(20) {
        Some("the 20 s cap")
    } else {
        None
    }
}

/// Guide does nothing in the core during these (B-22; D-042).
pub fn guide_allowed(kind: Kind) -> bool {
    !matches!(
        kind,
        Kind::Launching
            | Kind::LauncherWait
            | Kind::Returning
            | Kind::Closing
            | Kind::BootUp
            | Kind::BootDown
            | Kind::Loading
    )
}

#[derive(Debug)]
pub struct Machine {
    pub kind: Kind,
    pub game: Option<String>,
    pub since: Instant,
    held: BTreeSet<String>,
    /// What happened, in order (the app logs these lines; the tests read them).
    pub log: Vec<String>,
}

impl Machine {
    pub fn new(now: Instant) -> Machine {
        Machine {
            kind: Kind::None,
            game: None,
            since: now,
            held: BTreeSet::new(),
            log: Vec::new(),
        }
    }

    /// A state starts (any earlier one ends: "replaced").
    pub fn enter(&mut self, kind: Kind, game: Option<String>, why: &str, now: Instant) {
        if self.kind != Kind::None && self.kind != kind {
            self.end("replaced", now);
        }
        if kind == Kind::None {
            return;
        }
        if self.kind == kind && self.game == game {
            return; // already in it
        }
        self.kind = kind;
        self.game = game;
        self.since = now;
        self.log
            .push(format!("lock: {} started ({why})", kind.name()));
    }

    /// The state ends (done / escaped / timed out / cleared …), if it is the one named.
    pub fn leave(&mut self, kind: Kind, how: &str, now: Instant) -> bool {
        if self.kind != kind || kind == Kind::None {
            return false;
        }
        self.end(how, now);
        true
    }

    fn end(&mut self, how: &str, now: Instant) {
        let ms = now.saturating_duration_since(self.since).as_millis();
        self.log.push(format!(
            "lock: {} ended: {how} after {ms} ms",
            self.kind.name()
        ));
        self.kind = Kind::None;
        self.game = None;
        self.since = now;
    }

    /// The game ended, a new launch started, or AIONE paused again: any lock is cleared.
    pub fn clear(&mut self, why: &str, now: Instant) {
        if self.kind != Kind::None {
            self.end(&format!("cleared ({why})"), now);
        }
    }

    /// A press or release (pad or key).
    pub fn press(&mut self, key: &str, down: bool) {
        if down {
            self.held.insert(key.to_string());
        } else {
            self.held.remove(key);
        }
    }

    /// D-045 (B-22 rev. ao): AIONE gained or lost the front: a key's release can't be seen from
    /// another window, so the keyboard part of the held set is cleared (the pad's stays).
    pub fn clear_keys(&mut self) {
        self.held.retain(|k| !k.starts_with("key:"));
    }

    pub fn held(&self) -> Vec<String> {
        self.held.iter().cloned().collect()
    }

    /// The limit passed: the state ends ("timed out"); returns which one.
    pub fn tick(&mut self, now: Instant) -> Option<Kind> {
        let lim = limit(self.kind)?;
        if now.saturating_duration_since(self.since) <= lim {
            return None;
        }
        let k = self.kind;
        if k == Kind::Returning && !self.held.is_empty() {
            let still = self.held().join(", ");
            self.log.push(format!(
                "lock: {}: still held: {still}; resuming anyway",
                k.name()
            ));
        }
        self.end("timed out", now);
        Some(k)
    }

    pub fn guide_allowed(&self) -> bool {
        guide_allowed(self.kind)
    }
}

// ---------- the app's one machine ----------

static GLOBAL: Mutex<Option<Machine>> = Mutex::new(None);

pub fn with<T>(f: impl FnOnce(&mut Machine) -> T) -> T {
    let mut g = GLOBAL.lock().unwrap_or_else(|e| e.into_inner());
    let m = g.get_or_insert_with(|| Machine::new(Instant::now()));
    let before = m.log.len();
    let out = f(m);
    for line in m.log.drain(before..).collect::<Vec<_>>() {
        crate::logging::lifecycle(crate::logging::Level::Info, &line);
    }
    m.log.clear();
    out
}

pub fn current() -> (Kind, Option<String>, u128) {
    with(|m| {
        (
            m.kind,
            m.game.clone(),
            Instant::now()
                .saturating_duration_since(m.since)
                .as_millis(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small seeded generator (the same sequence for the same seed; no extra crate).
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0 >> 33
        }
        fn pick<'a>(&mut self, v: &'a [&'a str]) -> &'a str {
            v[(self.next() as usize) % v.len()]
        }
    }

    #[test]
    fn each_state_has_a_limit_or_an_answer_and_guide_waits_during_transitions() {
        for k in [
            Kind::BootUp,
            Kind::BootDown,
            Kind::Launching,
            Kind::LauncherWait,
            Kind::Returning,
            Kind::Closing,
            Kind::Loading,
        ] {
            assert!(limit(k).is_some(), "{k:?} has no limit");
            assert!(!guide_allowed(k), "Guide acts during {k:?}");
        }
        assert!(guide_allowed(Kind::None) && guide_allowed(Kind::Dialog));
    }

    #[test]
    fn a_guide_release_is_seen_so_a_return_never_waits_forever() {
        let t = Instant::now();
        let mut m = Machine::new(t);
        m.press("pad:Guide", true);
        m.enter(Kind::Returning, Some("g".into()), "Guide back", t);
        assert_eq!(m.held(), vec!["pad:Guide".to_string()]);
        m.press("pad:Guide", false); // D-041: Guide's release reaches the one held set
        assert!(m.held().is_empty());
        // And even if a release were missed, the limit ends it.
        m.press("pad:A", true);
        assert_eq!(
            m.tick(t + Duration::from_millis(3100)),
            Some(Kind::Returning)
        );
        assert_eq!(m.kind, Kind::None);
    }

    /// B-22 rev. ap: each change restarts the 3 s; the 20 s cap ends it regardless.
    #[test]
    fn the_game_settles_after_3_s_put_or_the_cap() {
        assert_eq!(
            settled(Duration::from_millis(2900), Duration::from_secs(5)),
            None
        );
        assert_eq!(
            settled(Duration::from_millis(3000), Duration::from_secs(5)),
            Some("settled")
        );
        assert_eq!(
            settled(Duration::from_millis(500), Duration::from_secs(20)),
            Some("the 20 s cap")
        );
    }

    /// D-045: a key whose release AIONE never saw (e.g. Alt from Alt-Tab) is cleared when AIONE
    /// gains or loses the front; the pad's held buttons stay.
    #[test]
    fn keys_are_cleared_on_a_front_change_the_pad_stays() {
        let t = Instant::now();
        let mut m = Machine::new(t);
        m.press("key:Enter", true); // released in another window: never seen
        m.press("pad:A", true);
        m.clear_keys();
        assert_eq!(m.held(), vec!["pad:A".to_string()]);
    }

    #[test]
    fn the_game_ending_or_a_new_launch_clears_any_lock() {
        let t = Instant::now();
        let mut m = Machine::new(t);
        m.enter(Kind::Returning, Some("a".into()), "A", t);
        m.clear("the game ended", t + Duration::from_millis(500));
        assert_eq!(m.kind, Kind::None);
        m.enter(Kind::Returning, Some("a".into()), "A", t);
        m.enter(
            Kind::Launching,
            Some("b".into()),
            "start",
            t + Duration::from_millis(10),
        );
        assert_eq!(m.kind, Kind::Launching);
        assert!(m
            .log
            .iter()
            .any(|l| l.contains("going back to the game ended: replaced")));
    }

    /// B-22: seeded button-mashing during every transition. Whatever is pressed or held, the state
    /// ends within its limit + 1 s, Guide never acts during a transition, and no lock survives the
    /// game's end. Seeds are fixed (reproducible); the results are printed (`--nocapture`).
    #[test]
    fn button_mashing_never_leaves_a_lock_behind() {
        const SEEDS: [u64; 8] = [1, 7, 42, 1234, 2026, 31337, 65535, 987_654_321];
        let keys = [
            "pad:Guide",
            "pad:A",
            "pad:B",
            "pad:DPadUp",
            "pad:DPadDown",
            "pad:DPadLeft",
            "pad:DPadRight",
            "key:Enter",
        ];
        let states = [
            Kind::BootUp,
            Kind::Launching,
            Kind::LauncherWait,
            Kind::Returning,
            Kind::Closing,
            Kind::BootDown,
            Kind::Loading,
        ];
        for seed in SEEDS {
            let mut r = Lcg(seed);
            let t0 = Instant::now();
            let mut m = Machine::new(t0);
            let mut guide_acted = 0;
            let mut events = 0;
            for (i, &k) in states.iter().enumerate() {
                let start = t0 + Duration::from_secs(60 * i as u64);
                m.enter(k, Some("game".into()), "mash", start);
                let lim = limit(k).unwrap();
                let mut now = start;
                // Mash for the whole limit: presses, releases, holds that are never released.
                while now < start + lim + Duration::from_millis(1000) {
                    now += Duration::from_millis(10 + r.next() % 90);
                    let key = r.pick(&keys);
                    let down = !r.next().is_multiple_of(3); // more presses than releases: some stay held
                    m.press(key, down);
                    events += 1;
                    if key == "pad:Guide" && down && m.guide_allowed() && m.kind != Kind::None {
                        guide_acted += 1;
                    }
                    // B escapes the states that have one (launching after 3 s, the launcher wait, a return).
                    if key == "pad:B" && down && r.next().is_multiple_of(4) {
                        let after = now.saturating_duration_since(start);
                        if (k == Kind::Launching && after > Duration::from_secs(3))
                            || k == Kind::LauncherWait
                            || k == Kind::Returning
                        {
                            m.leave(k, "escaped (B)", now);
                        }
                    }
                    m.tick(now);
                }
                assert_eq!(
                    m.kind,
                    Kind::None,
                    "seed {seed}: {k:?} still locked after its limit + 1 s"
                );
                // A lock entered again, then the game ends: nothing survives.
                m.enter(k, Some("game".into()), "again", now);
                m.clear("the game ended", now + Duration::from_millis(5));
                assert_eq!(
                    m.kind,
                    Kind::None,
                    "seed {seed}: {k:?} survived the game's end"
                );
            }
            assert_eq!(
                guide_acted, 0,
                "seed {seed}: Guide acted during a transition"
            );
            println!("button-mashing seed {seed}: {events} presses/releases over 7 transitions: every lock ended in time; Guide never acted; none survived the game's end");
        }
    }
}
