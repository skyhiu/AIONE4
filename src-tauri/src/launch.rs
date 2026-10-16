//! Starting games (T1.3; reference A9, Part B "Lifecycle"; B-7, B-11, B-17).
//!
//! **What is started (from `game.json`, A9):** the game's `Shortcut` (`game.lnk` or `game.url` in
//! its folder) is opened through the Windows shell, exactly like a double-click, so a `.lnk`'s own
//! target, arguments and working folder apply and a `.url` opens its URL (often `steam://…`).
//! Without a `Shortcut`, a `SteamAppId` is started as `steam://rungameid/<id>`. Nothing else in
//! `game.json` describes launching (the WPF format has no executable or argument fields).
//!
//! **Tracking:** one running game at a time.
//! - **The game's window:** the first visible top-level window of the started process (time
//!   logged). If the process exits without a window (a launcher), or no process is known (Steam
//!   starts the game itself), the game is the first new program that takes the front, never
//!   AIONE, Steam's helper, Explorer or the Game Bar.
//! - **Exit:** when the tracked process ends, AIONE is told (it comes back to the front).
//! - **Already running:** starting the same game again brings its window to the front; it is
//!   never started twice (Part B).

use crate::logging::{self, Level};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How a game was started (for the log and the UI).
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Started {
    pub id: String,
    pub name: String,
    /// "shortcut game.lnk", "shortcut game.url" or "steam://rungameid/123".
    pub how: String,
    /// The process the shell reported, if any.
    pub pid: Option<u32>,
    /// True when the game was already running and was brought to the front instead.
    pub already_running: bool,
    /// The foreground method, when it was brought to the front.
    pub foreground: Option<String>,
}

#[derive(Debug, Clone)]
struct Running {
    /// B-18 / D-025: each launch's watcher has its own generation; a watcher whose game is no
    /// longer the tracked one stops at once (never a second end line).
    gen: u64,
    id: String,
    name: String,
    pid: Option<u32>,
    exe: Option<String>,
    window: Option<isize>,
    started: Instant,
    /// D-032: where the real game runs from (Steam: appmanifest → installdir; a .lnk: its
    /// target's folder), to recognize it after a launcher.
    install_dir: Option<std::path::PathBuf>,
    /// B-20: `"NoPause": true` in game.json.
    no_pause: bool,
    /// B-11 (rev. ag): when the real game appeared (the start counts from then; launcher-only
    /// sessions and the 30 s wait never count).
    counted_at: Option<Instant>,
    /// D-036: AIONE is closing this game (its end is the game's end, never a launcher wait).
    closing: bool,
    /// B-20 (rev. al): the program in front at the last check, if it was neither the game nor AIONE.
    behind: Option<String>,
    /// D-046 (B-22 rev. ao): B cancelled this start; if the game appears anyway, it's closed.
    cancelled: bool,
    /// B-22 rev. ap: while the game is still loading: (when its window appeared, the last change,
    /// the last sample: main window, its rectangle, the game in front).
    loading: Option<Loading>,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);
static GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// What the watcher reports to the app.
pub enum Event {
    /// The game's window appeared (D-026: the UI lifts the black screen only now).
    Window { id: String, name: String, ms: u128 },
    /// B-11 (rev. ag): the real game appeared: its start is counted now.
    Counted { id: String, name: String },
    /// D-032: the launcher closed soon after the launch: AIONE waits for the real game.
    Following { id: String, name: String },
    /// The game ended (once): its session's length in minutes (B-11).
    Ended {
        id: String,
        name: String,
        minutes: f64,
        /// B-11 (rev. ag): false when the real game never appeared (nothing recorded).
        counted: bool,
        /// B-20 (rev. al): the program that was in front when the game ended (AIONE stays behind).
        behind: Option<String>,
    },
}

/// B-20: what the pause needs about the running game.
pub struct GameInfo {
    pub name: String,
    pub pid: Option<u32>,
    pub install_dir: Option<std::path::PathBuf>,
    pub no_pause: bool,
    /// D-044: the real game appeared (never pause a launcher).
    pub counted: bool,
}

pub fn running_game_info() -> Option<GameInfo> {
    let r = RUNNING.lock().ok()?.clone()?;
    Some(GameInfo {
        name: r.name,
        pid: r.pid,
        install_dir: r.install_dir,
        no_pause: r.no_pause,
        counted: r.counted_at.is_some(),
    })
}

/// D-037: when Guide was last pressed (in front or in the background).
static LAST_GUIDE: Mutex<Option<Instant>> = Mutex::new(None);

pub fn note_guide() {
    if let Ok(mut g) = LAST_GUIDE.lock() {
        *g = Some(Instant::now());
    }
}

/// D-037: milliseconds since the last Guide press (u64::MAX if never).
pub fn ms_since_guide() -> u64 {
    LAST_GUIDE
        .lock()
        .ok()
        .and_then(|g| *g)
        .map(|t| t.elapsed().as_millis() as u64)
        .unwrap_or(u64::MAX)
}

/// D-043: the running game's process appeared less than 3 s ago (its input thread is busy then;
/// the foreground step doesn't attach to it).
pub fn game_is_young(pid: u32) -> bool {
    RUNNING
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .filter(|r| r.pid == Some(pid))
        .and_then(|r| r.counted_at)
        .is_some_and(|t| t.elapsed() < Duration::from_secs(3))
}

/// D-047: the program in front now (for the diagnostic's log).
pub fn foreground_name() -> Option<String> {
    foreground_program().map(|(_, _, e)| e)
}

/// A process's program name (for the log).
pub fn exe_name(pid: u32) -> Option<String> {
    exe_of(pid)
}

/// Every running process (pid, parent, path).
pub fn processes() -> Vec<(u32, u32, Option<std::path::PathBuf>)> {
    process_list()
}

/// The id of the game that is running, if any.
pub fn running_id() -> Option<String> {
    RUNNING.lock().ok()?.as_ref().map(|r| r.id.clone())
}

/// B-17 (rev. aa): right after a Guide press, if Steam or the Xbox Game Bar took the front, a
/// plain hint naming the setting to turn off.
pub fn guide_hint() -> Option<String> {
    // Rev. ae: the program holding the KEYS first (the Game Bar's window belongs to a host
    // process; its keys belong to GameBar.exe), then the program in front.
    let keys = keys_program().unwrap_or_default().to_ascii_lowercase();
    let front = foreground_program()
        .map(|(_, _, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    let game_bar = |e: &str| e == "gamebar.exe" || e == "gamebarftserver.exe";
    let steam = |e: &str| e == "steamwebhelper.exe" || e == "steam.exe";
    if game_bar(&keys) || game_bar(&front) {
        Some("hint: Guide also opened the Xbox Game Bar: turn off its controller button in Windows Settings → Gaming → Xbox Game Bar (\"Open Xbox Game Bar using this button on a controller\")".into())
    } else if steam(&keys) || steam(&front) {
        Some("hint: Guide also opened Steam: turn off \"Guide button focuses Steam\" in Steam → Settings → Controller".into())
    } else {
        None
    }
}

/// Programs that are never "the game" when following the foreground (B-17 hints aside).
const NOT_THE_GAME: [&str; 6] = [
    "aione4.exe",
    "steamwebhelper.exe",
    "steam.exe",
    "explorer.exe",
    "gamebar.exe",
    "gamebarftserver.exe",
];

pub fn is_not_the_game(exe: &str) -> bool {
    NOT_THE_GAME.contains(&exe.to_ascii_lowercase().as_str())
}

/// B-22 rev. ap: the loading check's sample: the main window, its rectangle, the game in front.
type Sample = (isize, (i32, i32, i32, i32), bool);
/// B-22 rev. ap: (when the game's window appeared, the last change, the last sample).
type Loading = (Instant, Instant, Sample);

/// B-18 (rev. am): how long a game gets to close itself before AIONE ends it.
pub const CLOSE_GRACE: Duration = Duration::from_secs(3);

/// B-18 (rev. ah): how long AIONE waits for the real game after a launcher closes (the game
/// appeared within 0.60-0.84 s in 4 of 4 Plays on the user's PC).
pub const FOLLOW_WAIT: Duration = Duration::from_secs(5);

/// B-18 (rev. ah): B on "Starting <game>…" cancels the wait: the game counts as ended, nothing is
/// counted (the watcher stops: the game is no longer tracked).
pub fn cancel_start(id: &str) -> Result<(String, Option<ClosedSession>), String> {
    // D-046 (B-22 rev. ao): the start is cancelled for real but stays watched until its limit: a
    // game appearing anyway is closed (never left running untracked).
    let mut g = RUNNING.lock().map_err(|_| "busy")?;
    let r = g
        .as_mut()
        .filter(|r| r.id == id)
        .ok_or("that game isn't starting")?;
    r.cancelled = true;
    let m = format!("start \"{}\": cancelled (B); watching until the start's limit (a game appearing anyway is closed)", r.name);
    logging::lifecycle(Level::Info, &m);
    Ok((m, None))
}

/// D-036: the game is being closed by AIONE (Close Software, Close and start, force close).
fn mark_closing(id: &str) {
    if let Ok(mut g) = RUNNING.lock() {
        if let Some(r) = g.as_mut().filter(|r| r.id == id) {
            r.closing = true;
        }
    }
}

/// B-20 (rev. al): when the game ends, AIONE stays behind if neither the game nor AIONE was in
/// front at the last check: the program then in front, else None.
pub fn behind(
    front_pid: Option<u32>,
    front_exe: &str,
    game_pid: Option<u32>,
    aione_pid: u32,
) -> Option<String> {
    let pid = front_pid?;
    if Some(pid) == game_pid
        || pid == aione_pid
        || front_exe.eq_ignore_ascii_case("aione4.exe")
        || front_exe.is_empty()
    {
        return None;
    }
    Some(front_exe.to_string())
}

/// D-036 (rev. ai): does this process's end start the launcher wait? Never once it IS the game
/// (counted: its window appeared, or it was followed after a launcher) or AIONE is closing it;
/// only a launcher by name, or a process that ended before any game appeared (soon after the
/// launch).
pub fn end_is_launcher(
    counted: bool,
    closing: bool,
    since_launch: Duration,
    exe: Option<&str>,
) -> bool {
    if counted || closing {
        return false;
    }
    exe.is_some_and(is_launcher) || since_launch < Duration::from_secs(30)
}

/// D-048 (B-22 rev. aq): Steam's own window (any user's Steam: its client or its web helper).
pub fn is_steam(exe: &str) -> bool {
    let e = exe.to_ascii_lowercase();
    e == "steamwebhelper.exe" || e == "steam.exe"
}

/// D-048: does the launcher wait give up now? With Steam's window seen during it ("Steam is asking
/// something"), only at the start's 45 s limit; otherwise after the usual wait.
pub fn follow_gives_up(since_follow: Duration, since_launch: Duration, steam_seen: bool) -> bool {
    if steam_seen {
        since_launch > Duration::from_secs(45)
    } else {
        since_follow > FOLLOW_WAIT
    }
}

/// D-046 rev. ar: what is showing during a start (B is offered only when nothing is).
pub fn showing_label(
    steam_in_front: bool,
    started_window: bool,
    started_is_launcher: bool,
) -> Option<&'static str> {
    if steam_in_front {
        Some("Steam is showing a window")
    } else if started_window && started_is_launcher {
        Some("the launcher is showing a window")
    } else if started_window {
        Some("the game is showing a window")
    } else {
        None
    }
}

/// D-046 rev. ar: during the start of `id`, is any window of the game, its launcher or Steam up?
pub fn start_showing(id: &str) -> Option<String> {
    let r = RUNNING
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .filter(|r| r.id == id)?;
    let steam = foreground_program().is_some_and(|(_, _, e)| is_steam(&e));
    let window = r.pid.and_then(main_window_of).is_some();
    let launcher = r.exe.as_deref().is_some_and(is_launcher);
    showing_label(steam, window, launcher).map(str::to_string)
}

/// D-032: programs known to be launchers (they start the real game and exit).
pub fn is_launcher(exe: &str) -> bool {
    let e = exe.to_ascii_lowercase();
    e.contains("launcher") || e == "start.exe" || e == "setup_launcher.exe"
}

/// D-032: the real game after a launcher: among running processes `(pid, parent, full path)`,
/// one with a window that is a child of the launcher, or runs from the game's install folder.
pub fn pick_game(
    procs: &[(u32, u32, Option<std::path::PathBuf>)],
    launcher: u32,
    install_dir: Option<&std::path::Path>,
    has_window: &dyn Fn(u32) -> bool,
) -> Option<(u32, &'static str)> {
    let under = |p: &std::path::Path| {
        install_dir.is_some_and(|d| {
            let d = d.to_string_lossy().to_ascii_lowercase().replace('/', "\\");
            let p = p.to_string_lossy().to_ascii_lowercase().replace('/', "\\");
            !d.is_empty()
                && p.starts_with(d.trim_end_matches('\\'))
                && p.len() > d.trim_end_matches('\\').len()
        })
    };
    let ok = |pid: u32, path: &Option<std::path::PathBuf>| {
        // The program's name after the last \ or / (the same on every system).
        let name = path.as_ref().map(|p| {
            p.to_string_lossy()
                .rsplit(['\\', '/'])
                .next()
                .unwrap_or_default()
                .to_string()
        });
        pid != launcher && has_window(pid) && !name.is_some_and(|n| is_not_the_game(&n))
    };
    if let Some((pid, _, _)) = procs
        .iter()
        .find(|(pid, parent, path)| *parent == launcher && ok(*pid, path))
    {
        return Some((*pid, "started by the launcher"));
    }
    procs
        .iter()
        .find(|(pid, _, path)| path.as_deref().is_some_and(under) && ok(*pid, path))
        .map(|(pid, _, _)| (*pid, "runs from the game's install folder"))
}

/// D-032: a Steam game's install folder: `appmanifest_<id>.acf` → `installdir`, in every library
/// listed in `steamapps\libraryfolders.vdf` (and Steam's own folder).
pub fn steam_install_dir(steam_root: &std::path::Path, app_id: &str) -> Option<std::path::PathBuf> {
    let vdf_value = |text: &str, key: &str| -> Vec<String> {
        text.lines()
            .filter_map(|l| {
                let parts: Vec<&str> = l.split('"').collect();
                (parts.len() >= 4 && parts[1].eq_ignore_ascii_case(key))
                    .then(|| parts[3].replace("\\\\", "\\"))
            })
            .collect()
    };
    let mut libraries = vec![steam_root.to_path_buf()];
    if let Ok(text) =
        std::fs::read_to_string(steam_root.join("steamapps").join("libraryfolders.vdf"))
    {
        libraries.extend(
            vdf_value(&text, "path")
                .into_iter()
                .map(std::path::PathBuf::from),
        );
    }
    for lib in libraries {
        let apps = lib.join("steamapps");
        if let Ok(text) = std::fs::read_to_string(apps.join(format!("appmanifest_{app_id}.acf"))) {
            if let Some(dir) = vdf_value(&text, "installdir").into_iter().next() {
                return Some(apps.join("common").join(dir));
            }
        }
    }
    None
}

/// The Steam app id behind a game: `SteamAppId`, or the `.url`'s `steam://rungameid/<id>`.
pub fn steam_app_id_of(f: &crate::games::LaunchFields) -> Option<String> {
    if let Some(id) = &f.steam_app_id {
        return Some(id.clone());
    }
    let text = std::fs::read_to_string(f.shortcut.as_ref()?).ok()?;
    let i = text.find("steam://rungameid/")? + "steam://rungameid/".len();
    let id: String = text[i..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    (!id.is_empty()).then_some(id)
}

/// What to open for a game: the shell target and a short description for the log.
pub fn target_of(f: &crate::games::LaunchFields) -> Result<(String, String), String> {
    if let Some(p) = &f.shortcut {
        if !p.is_file() {
            return Err(format!(
                "its shortcut {} is missing",
                p.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ));
        }
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        return Ok((p.display().to_string(), format!("shortcut {name}")));
    }
    if let Some(app) = &f.steam_app_id {
        let url = format!("steam://rungameid/{app}");
        return Ok((url.clone(), url));
    }
    Err("its game.json has no Shortcut and no SteamAppId".into())
}

/// Starts the game (or brings it to the front if it is already running). `report` gets the
/// game's window and its end (once), from the watcher thread.
pub fn launch(
    data: &Path,
    id: &str,
    report: impl Fn(Event) + Send + 'static,
) -> Result<Started, String> {
    let f = crate::games::launch_fields(data, id)?;
    // Already running: bring it to the front, never start it twice.
    if let Some(r) = RUNNING.lock().ok().and_then(|g| g.clone()) {
        if r.id == id && alive(&r) {
            let report = r.window.map(crate::foreground::bring_to_front);
            let method = report.as_ref().map(|r| r.summary());
            logging::lifecycle(
                Level::Info,
                &format!(
                    "start \"{}\": already running ({}): brought to the front: {}",
                    r.name,
                    r.exe.clone().unwrap_or_else(|| "its window".into()),
                    method
                        .clone()
                        .unwrap_or_else(|| "its window isn't known yet".into())
                ),
            );
            return Ok(Started {
                id: id.into(),
                name: f.name,
                how: "already running".into(),
                pid: r.pid,
                already_running: true,
                foreground: method,
            });
        }
    }
    let (target, how) = target_of(&f)?;
    logging::lifecycle(Level::Info, &format!("start \"{}\": opening {how}", f.name));
    // B-19 (rev. ad): an emulator's missing options are added at launch; the shortcut is only read.
    let pid = match emulator_launch(&f) {
        Some((exe, params, dir)) => shell_open(&exe, Some(&params), &dir)?,
        None => shell_open(&target, None, &f.folder)?,
    };
    logging::lifecycle(
        Level::Info,
        &format!(
            "start \"{}\": started ({how}); {}",
            f.name,
            pid.map(|p| format!("process {p} ({})", exe_of(p).unwrap_or_else(|| "?".into())))
                .unwrap_or_else(
                    || "no process reported (the shell or Steam starts the game)".into()
                )
        ),
    );
    let gen = GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    // B-22: a new launch clears any input-locking state left over (the UI enters "launching").
    crate::transit::with(|m| {
        if m.kind != crate::transit::Kind::Launching {
            m.clear("a new launch", Instant::now())
        }
    });
    // D-032: where the real game runs from (to recognize it after a launcher).
    let install_dir = steam_app_id_of(&f)
        .and_then(|app| steam_root().and_then(|root| steam_install_dir(&root, &app)))
        .or_else(|| {
            f.shortcut
                .as_ref()
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk")))
                .and_then(|p| crate::emulators::read_shortcut(p).ok())
                .and_then(|sc| sc.target)
                .and_then(|t| t.parent().map(|d| d.to_path_buf()))
        });
    logging::lifecycle(
        Level::Info,
        &format!(
            "start \"{}\": install folder {}",
            f.name,
            install_dir
                .as_ref()
                .map(|d| d.display().to_string())
                .unwrap_or_else(|| "unknown".into())
        ),
    );
    let run = Running {
        gen,
        id: id.into(),
        name: f.name.clone(),
        pid,
        exe: pid.and_then(exe_of),
        window: None,
        started: Instant::now(),
        install_dir,
        no_pause: f.no_pause,
        counted_at: None,
        closing: false,
        behind: None,
        cancelled: false,
        loading: None,
    };
    if let Ok(mut g) = RUNNING.lock() {
        if let Some(old) = g.replace(run) {
            logging::lifecycle(
                Level::Warn,
                &format!(
                    "\"{}\" is no longer tracked (another game was started)",
                    old.name
                ),
            );
        }
    }
    std::thread::spawn(move || watch(gen, report));
    Ok(Started {
        id: id.into(),
        name: f.name,
        how,
        pid,
        already_running: false,
        foreground: None,
    })
}

/// B-19: for a `.lnk` shortcut to a known emulator, the program, the arguments with only the
/// missing options added in front, and the working folder. None: open the shortcut as before.
fn emulator_launch(f: &crate::games::LaunchFields) -> Option<(String, String, std::path::PathBuf)> {
    let lnk = f
        .shortcut
        .as_ref()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk")))?;
    // D-034: Windows' own shortcut reader (read only).
    let sc = match crate::emulators::read_shortcut(lnk) {
        Ok(sc) => sc,
        Err(e) => {
            logging::lifecycle(
                Level::Info,
                &format!(
                    "start \"{}\": its shortcut couldn't be read ({e}); opened as it is",
                    f.name
                ),
            );
            return None;
        }
    };
    let exe = sc.target.clone()?;
    let profiles = crate::emulators::profiles();
    let Some(p) = crate::emulators::recognize(&profiles, &exe) else {
        return None; // not an emulator: unchanged
    };
    let (add, skip) = crate::emulators::missing(p, &sc.arguments);
    let skipped: Vec<String> = skip
        .iter()
        .map(|(o, why)| format!("{} ({why})", o.add.join(" ")))
        .collect();
    if add.is_empty() {
        logging::lifecycle(
            Level::Info,
            &format!(
                "start \"{}\": {}: nothing to add{}",
                f.name,
                p.name,
                if skipped.is_empty() {
                    String::new()
                } else {
                    format!("; not added: {}", skipped.join(", "))
                }
            ),
        );
        return None;
    }
    let added = crate::emulators::joined(&add);
    logging::lifecycle(
        Level::Info,
        &format!(
            "start \"{}\": {}: added {}{}",
            f.name,
            p.name,
            add.iter()
                .map(|o| format!("{} ({})", o.add.join(" "), o.why))
                .collect::<Vec<_>>()
                .join(", "),
            if skipped.is_empty() {
                String::new()
            } else {
                format!("; not added: {}", skipped.join(", "))
            }
        ),
    );
    let params = if sc.arguments.trim().is_empty() {
        added
    } else {
        format!("{added} {}", sc.arguments)
    };
    let dir = sc
        .working_dir
        .clone()
        .filter(|d| !d.as_os_str().is_empty())
        .or_else(|| exe.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| f.folder.clone());
    Some((exe.to_string_lossy().into_owned(), params, dir))
}

/// D-037 (rev. ak): the window-swap hand-back: only for the game (counted: never a launcher), only to
/// its visible main window found again by process, and never with a step-aside.
pub fn hand_back(id: &str) -> Result<String, String> {
    let r = RUNNING
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .filter(|r| r.id == id)
        .ok_or("that game isn't running")?;
    if r.counted_at.is_none() {
        return Err("not the game yet (a launcher): no hand-back".into());
    }
    let pid = r
        .pid
        .filter(|p| process_alive(*p))
        .ok_or("its process has ended")?;
    let w = main_window_of(pid).ok_or("no visible window of the game: no hand-back")?;
    let report = crate::foreground::bring_to_front(w);
    let m = format!("window swap: \"{}\": {}", r.name, report.summary());
    logging::lifecycle(Level::Info, &m);
    if report.ok {
        Ok(m)
    } else {
        Err(m)
    }
}

/// B-18 (rev. ag): force close (only on the user's confirmation): resumes the game if paused, ends
/// its process and those it started (TerminateProcess), and returns its counted session.
pub fn force_close(id: &str) -> Result<Option<ClosedSession>, String> {
    mark_closing(id);
    crate::pause::resume_running("before force closing");
    let r = RUNNING
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .filter(|r| r.id == id)
        .ok_or("that game isn't running")?;
    let pid = r.pid.ok_or("its process isn't known")?;
    let pids = crate::pause::tree(&process_list(), pid);
    let ended: usize = pids.iter().map(|p| usize::from(terminate(*p))).sum();
    logging::lifecycle(
        Level::Warn,
        &format!(
            "force close \"{}\": ended {ended} of {} process(es) {:?} (asked by the user)",
            r.name,
            pids.len(),
            pids
        ),
    );
    let counted = r.counted_at;
    let session = (take_if(r.gen) && counted.is_some()).then(|| ClosedSession {
        id: r.id.clone(),
        name: r.name.clone(),
        minutes: counted
            .map(|t| t.elapsed().as_secs_f64() / 60.0)
            .unwrap_or(0.0),
    });
    Ok(session)
}

#[cfg(windows)]
fn terminate(pid: u32) -> bool {
    use windows::Win32::System::Threading::{TerminateProcess, PROCESS_TERMINATE};
    let Ok(h) =
        (unsafe { windows::Win32::System::Threading::OpenProcess(PROCESS_TERMINATE, false, pid) })
    else {
        return false;
    };
    let ok = unsafe { TerminateProcess(h, 1) }.is_ok();
    let _ = unsafe { windows::Win32::Foundation::CloseHandle(h) };
    ok
}
#[cfg(not(windows))]
fn terminate(_pid: u32) -> bool {
    false
}

/// Brings the running game's window to the front (A or Guide on the running game, B-7).
pub fn return_to_game(id: &str) -> Result<String, String> {
    crate::pause::resume_running("back to the game");
    let r = RUNNING
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .ok_or("no game is running")?;
    if r.id != id || !alive(&r) {
        return Err("that game isn't running".into());
    }
    hand_over(&r, "back to")
}

/// D-027: hands the front to the running game. The game's main window is found again by its
/// process every time (never a stale handle) and its state logged; a minimized window is
/// restored and an active pop-up targeted (by `bring_to_front`, alpha.15's methods). If Windows
/// still refuses, AIONE steps aside: the game's window goes to the top (not activated) and AIONE
/// minimizes itself, so Windows activates the game; the next Guide restores AIONE.
fn hand_over(r: &Running, what: &str) -> Result<String, String> {
    if r.pid.is_some_and(|p| !process_alive(p)) {
        return Err(format!(
            "{what} \"{}\": its process has ended (no hand-over)",
            r.name
        ));
    }
    let target = r
        .pid
        .and_then(main_window_of)
        .or(r.window.filter(|w| window_alive(*w)));
    let Some(main) = target else {
        let m = format!(
            "{what} \"{}\": no window of {} found",
            r.name,
            r.exe.clone().unwrap_or_default()
        );
        logging::lifecycle(Level::Warn, &m);
        return Err(m);
    };
    let popup = active_popup(main);
    let target = popup.unwrap_or(main);
    logging::lifecycle(
        Level::Info,
        &format!(
            "{what} \"{}\": its window {}{}",
            r.name,
            window_state(main),
            popup
                .map(|p| format!("; its active pop-up {}", window_state(p)))
                .unwrap_or_default()
        ),
    );
    if let Ok(mut g) = RUNNING.lock() {
        if let Some(x) = g.as_mut().filter(|x| x.gen == r.gen) {
            x.window = Some(main);
        }
    }
    let report = crate::foreground::bring_to_front(target);
    if report.ok {
        let m = format!("{what} \"{}\": {}", r.name, report.summary());
        logging::lifecycle(Level::Info, &m);
        return Ok(m);
    }
    // Windows refused: AIONE steps aside.
    let aione = crate::foreground::main_window();
    let aside = step_aside(aione, target, r.pid.unwrap_or(0));
    if !aside && aione != 0 {
        // D-038 (rev. ak): a failed step-aside never leaves AIONE minimized.
        let back = crate::foreground::bring_to_front(aione);
        logging::lifecycle(
            Level::Info,
            &format!(
                "{what} \"{}\": the step-aside failed: AIONE restored and brought back ({})",
                r.name,
                back.summary()
            ),
        );
    }
    let m = format!(
        "{what} \"{}\": {}; then {}",
        r.name,
        report.summary(),
        if aside {
            "OK via step-aside (AIONE minimized itself; the game is in front)"
        } else {
            "step-aside also failed (the game isn't in front)"
        }
    );
    logging::lifecycle(if aside { Level::Info } else { Level::Warn }, &m);
    if aside {
        Ok(m)
    } else {
        Err(m)
    }
}

fn alive(r: &Running) -> bool {
    match r.pid {
        Some(p) => process_alive(p),
        None => r.window.is_some_and(window_alive),
    }
}

/// The watcher (one per launch): finds the game's window, follows a launcher to the real game,
/// and reports the game's end once. It stops at once when its game is no longer the tracked one.
fn watch(gen: u64, report: impl Fn(Event)) {
    let mut found_logged = false;
    // D-032: (when the launcher ended, its process id) while waiting for the real game.
    let mut following: Option<(Instant, u32)> = None;
    // D-048: Steam's window was seen in front during the launcher wait.
    let mut steam_seen = false;
    loop {
        std::thread::sleep(Duration::from_millis(250));
        let Some(mut r) = RUNNING
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .filter(|r| r.gen == gen)
        else {
            return;
        };
        let since = r.started.elapsed();
        // B-22 rev. ap: is the game still loading? (the same window, size and position, in front)
        if let Some((t0, changed, last)) = r.loading {
            let now = Instant::now();
            let w = r.pid.and_then(main_window_of).unwrap_or(0);
            let front = foreground_program()
                .map(|(_, p, _)| Some(p) == r.pid)
                .unwrap_or(false);
            let sample = (w, window_rect(w), front);
            let changed = if sample != last { now } else { changed };
            if let Some(how) = crate::transit::settled(now - changed, now - t0) {
                r.loading = None;
                crate::transit::with(|m| m.leave(crate::transit::Kind::Loading, how, now));
            } else {
                r.loading = Some((t0, changed, sample));
            }
        }
        // D-046: a cancelled start that showed nothing by the start's limit ends quietly.
        if r.cancelled && r.window.is_none() && since > Duration::from_secs(46) {
            if take_if(gen) {
                logging::lifecycle(
                    Level::Info,
                    &format!(
                        "start \"{}\": cancelled (B); no game appeared by the start's limit",
                        r.name
                    ),
                );
            }
            return;
        }
        // B-20 (rev. al): who is in front now (the game, AIONE, or another program).
        if r.counted_at.is_some() {
            let (fp, fe) = foreground_program()
                .map(|(_, p, e)| (Some(p), e))
                .unwrap_or((None, String::new()));
            r.behind = behind(fp, &fe, r.pid, std::process::id());
        }
        // 1. The game's window (not while following a launcher: the follow finds the game).
        if r.window.is_none() && following.is_none() {
            if let Some(p) = r.pid {
                r.window = top_window_of(p);
            }
            if r.window.is_none() && (r.pid.is_none() || !r.pid.is_some_and(process_alive)) {
                // A launcher that exited, or Steam: the first new program in front is the game.
                if let Some((w, p, exe)) = foreground_program() {
                    if !is_not_the_game(&exe) {
                        logging::lifecycle(
                            Level::Info,
                            &format!(
                                "start \"{}\": the game continued in {exe} (process {p})",
                                r.name
                            ),
                        );
                        r.window = Some(w);
                        r.pid = Some(p);
                        r.exe = Some(exe);
                    }
                }
            }
            if r.window.is_some() && !found_logged {
                found_logged = true;
                logging::lifecycle(
                    Level::Info,
                    &format!(
                        "start \"{}\": its window appeared after {} ms ({})",
                        r.name,
                        since.as_millis(),
                        r.exe.clone().unwrap_or_default()
                    ),
                );
                if r.cancelled {
                    // D-046: the start was cancelled (B): the game appeared anyway: closed.
                    let id = r.id.clone();
                    let name = r.name.clone();
                    std::thread::spawn(move || {
                        let how = close_game(&id, Duration::from_secs(3), |_| {})
                            .map(|_| "closed")
                            .unwrap_or("could not be closed");
                        logging::lifecycle(
                            Level::Info,
                            &format!(
                                "start \"{name}\": cancelled (B); the game appeared anyway: {how}"
                            ),
                        );
                    });
                    return;
                }
                report(Event::Window {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    ms: since.as_millis(),
                });
                // B-11 (rev. ag): the start counts once the real game appears (not a launcher).
                if r.counted_at.is_none() && !r.exe.as_deref().is_some_and(is_launcher) {
                    r.counted_at = Some(Instant::now());
                    if let Ok(mut g) = RUNNING.lock() {
                        if let Some(x) = g.as_mut().filter(|x| x.gen == gen) {
                            x.counted_at = r.counted_at;
                        }
                    }
                    report(Event::Counted {
                        id: r.id.clone(),
                        name: r.name.clone(),
                    });
                    // B-22 rev. ap: the game is still loading until it has stayed put for 3 s.
                    let now = Instant::now();
                    r.loading = Some((now, now, (0, (0, 0, 0, 0), false)));
                    crate::transit::with(|m| {
                        m.enter(
                            crate::transit::Kind::Loading,
                            Some(r.id.clone()),
                            "the game's window appeared",
                            now,
                        )
                    });
                }
                // D-027: the game's window appeared: AIONE hands it the front (Steam included).
                if let Ok(mut g) = RUNNING.lock() {
                    if g.as_ref().is_some_and(|x| x.gen == gen) {
                        *g = Some(r.clone());
                    }
                }
                let _ = hand_over(&r, "its window appeared:");
            }
            // A launcher that closed without starting a game counts as ended (B-18); so does a
            // start that never shows a window within 2 minutes.
            let launcher_gone =
                r.pid.is_some_and(|p| !process_alive(p)) && since > Duration::from_secs(10);
            if r.window.is_none() && (launcher_gone || since > Duration::from_secs(120)) {
                logging::lifecycle(
                    Level::Warn,
                    &format!(
                        "start \"{}\": no game window ({}); counted as ended",
                        r.name,
                        if launcher_gone {
                            "its launcher closed"
                        } else {
                            "2 minutes passed"
                        }
                    ),
                );
                if take_if(gen) {
                    report(Event::Ended {
                        id: r.id.clone(),
                        name: r.name.clone(),
                        minutes: r
                            .counted_at
                            .map(|t| t.elapsed().as_secs_f64() / 60.0)
                            .unwrap_or(0.0),
                        counted: r.counted_at.is_some(),
                        behind: r.behind.clone(),
                    });
                }
                return;
            }
        }
        // 2. D-032: a launcher's end soon after the launch is not the game's end: follow it.
        if let Some((since_follow, launcher)) = following {
            let procs = process_list();
            // D-048 (B-22 rev. aq): Steam's window in front = Steam is asking something.
            if !steam_seen && foreground_program().is_some_and(|(_, _, e)| is_steam(&e)) {
                steam_seen = true;
                logging::lifecycle(Level::Info, &format!("start \"{}\": Steam is showing a window during the start: waiting (up to the start's 45 s)", r.name));
                crate::transit::with(|m| {
                    m.enter(
                        crate::transit::Kind::Launching,
                        Some(r.id.clone()),
                        "Steam is showing a window",
                        Instant::now(),
                    )
                });
            }
            let gives_up = follow_gives_up(since_follow.elapsed(), since, steam_seen);
            if let Some((pid, why)) = pick_game(&procs, launcher, r.install_dir.as_deref(), &|p| {
                main_window_of(p).is_some()
            }) {
                let exe = exe_of(pid).unwrap_or_default();
                logging::lifecycle(
                    Level::Info,
                    &format!(
                        "start \"{}\": followed {exe} (process {pid}) {} ms after the launcher ended: {why}",
                        r.name,
                        since_follow.elapsed().as_millis()
                    ),
                );
                if r.cancelled {
                    // D-046: the start was cancelled (B): the game appeared anyway (after its launcher): closed.
                    r.pid = Some(pid);
                    if let Ok(mut g) = RUNNING.lock() {
                        if let Some(x) = g.as_mut().filter(|x| x.gen == gen) {
                            x.pid = Some(pid);
                        }
                    }
                    let id = r.id.clone();
                    let name = r.name.clone();
                    std::thread::spawn(move || {
                        let how = close_game(&id, Duration::from_secs(3), |_| {})
                            .map(|_| "closed")
                            .unwrap_or("could not be closed");
                        logging::lifecycle(
                            Level::Info,
                            &format!(
                                "start \"{name}\": cancelled (B); the game appeared anyway: {how}"
                            ),
                        );
                    });
                    return;
                }
                r.pid = Some(pid);
                r.exe = Some(exe);
                r.window = main_window_of(pid);
                following = None;
                if let Ok(mut g) = RUNNING.lock() {
                    if g.as_ref().is_some_and(|x| x.gen == gen) {
                        *g = Some(r.clone());
                    }
                }
                report(Event::Window {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    ms: r.started.elapsed().as_millis(),
                });
                // B-11 (rev. ag): the start counts once the real game appears (not a launcher).
                if r.counted_at.is_none() && !r.exe.as_deref().is_some_and(is_launcher) {
                    r.counted_at = Some(Instant::now());
                    if let Ok(mut g) = RUNNING.lock() {
                        if let Some(x) = g.as_mut().filter(|x| x.gen == gen) {
                            x.counted_at = r.counted_at;
                        }
                    }
                    report(Event::Counted {
                        id: r.id.clone(),
                        name: r.name.clone(),
                    });
                }
                let _ = hand_over(&r, "the game after its launcher:");
            } else if gives_up {
                if foreground_program().is_some_and(|(_, _, e)| is_steam(&e)) {
                    // D-048: never take the front from Steam's window when the start ends.
                    r.behind = Some("steamwebhelper.exe".into());
                }
                logging::lifecycle(Level::Info, &format!("start \"{}\": no game after the launcher in {} s (closed without Play): ended", r.name, FOLLOW_WAIT.as_secs()));
                if take_if(gen) {
                    report(Event::Ended {
                        id: r.id.clone(),
                        name: r.name.clone(),
                        minutes: r
                            .counted_at
                            .map(|t| t.elapsed().as_secs_f64() / 60.0)
                            .unwrap_or(0.0),
                        counted: r.counted_at.is_some(),
                        behind: r.behind.clone(),
                    });
                }
                return;
            }
        } else if r.window.is_some() && !alive(&r) {
            let launcherish =
                end_is_launcher(r.counted_at.is_some(), r.closing, since, r.exe.as_deref());
            if launcherish {
                let launcher = r.pid.unwrap_or(0);
                logging::lifecycle(
                    Level::Info,
                    &format!(
                        "start \"{}\": {} (process {launcher}) ended {} s after the launch: a launcher; waiting up to {} s for the game (install folder {})",
                        r.name,
                        r.exe.clone().unwrap_or_default(),
                        since.as_secs(),
                        FOLLOW_WAIT.as_secs(),
                        r.install_dir.as_ref().map(|d| d.display().to_string()).unwrap_or_else(|| "unknown".into())
                    ),
                );
                following = Some((Instant::now(), launcher));
                r.window = None;
                report(Event::Following {
                    id: r.id.clone(),
                    name: r.name.clone(),
                });
            } else {
                // The game's end: its process ending (an emulator: the emulator ending, B-18).
                let minutes = since.as_secs_f64() / 60.0;
                if take_if(gen) {
                    logging::lifecycle(
                        Level::Info,
                        &format!("\"{}\" ended after {:.1} min", r.name, minutes),
                    );
                    report(Event::Ended {
                        id: r.id.clone(),
                        name: r.name.clone(),
                        minutes: r
                            .counted_at
                            .map(|t| t.elapsed().as_secs_f64() / 60.0)
                            .unwrap_or(0.0),
                        counted: r.counted_at.is_some(),
                        behind: r.behind.clone(),
                    });
                }
                return;
            }
        }
        if let Ok(mut g) = RUNNING.lock() {
            if g.as_ref().is_some_and(|x| x.gen == gen) {
                *g = Some(r);
            }
        }
    }
}

/// Clears the tracked game if it is still this watcher's; true when it was (the end is then
/// reported exactly once).
fn take_if(gen: u64) -> bool {
    RUNNING
        .lock()
        .map(|mut g| {
            if g.as_ref().is_some_and(|x| x.gen == gen) {
                *g = None;
                true
            } else {
                false
            }
        })
        .unwrap_or(false)
}

/// B-18: closes the running game politely (WM_CLOSE to its windows, as its ✕ would; never
/// killed) and waits up to `wait` for its process to end. The watcher reports the end.
/// D-033: the session of a game AIONE closed (recorded when the close is done).
pub struct ClosedSession {
    pub id: String,
    pub name: String,
    pub minutes: f64,
}

pub fn close_game(
    id: &str,
    _wait: Duration,
    _hint: impl Fn(String),
) -> Result<Option<ClosedSession>, String> {
    // B-18 (rev. am): closing from AIONE, for sure, after the user's one question.
    mark_closing(id);
    crate::pause::resume_running("before closing");
    let r = RUNNING
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .filter(|r| r.id == id)
        .ok_or("that game isn't running")?;
    let Some(pid) = r.pid else {
        return Err("its process isn't known, so AIONE can't close it".into());
    };
    let t0 = Instant::now();
    let n = close_windows_of(pid);
    logging::lifecycle(
        Level::Info,
        &format!(
            "close \"{}\": asked {n} window(s) of {} to close (as their ✕ would); up to 3 s",
            r.name,
            r.exe.clone().unwrap_or_default()
        ),
    );
    let tree_alive = |root: u32| {
        crate::pause::tree(&process_list(), root)
            .into_iter()
            .filter(|p| process_alive(*p))
            .count()
    };
    // About 3 s for the game to close itself (most games save their settings in that time).
    while n > 0 && t0.elapsed() < CLOSE_GRACE && process_alive(pid) {
        std::thread::sleep(Duration::from_millis(100));
    }
    if !process_alive(pid) && tree_alive(pid) == 0 {
        logging::lifecycle(
            Level::Info,
            &format!(
                "close \"{}\": asked; ended by itself after {} ms",
                r.name,
                t0.elapsed().as_millis()
            ),
        );
    } else {
        // Still running (or nothing to ask): AIONE ends it and what it started, no second question.
        let pids = crate::pause::tree(&process_list(), pid);
        let ended: usize = pids
            .iter()
            .filter(|p| process_alive(**p))
            .map(|p| usize::from(terminate(*p)))
            .sum();
        logging::lifecycle(
            Level::Info,
            &format!(
                "close \"{}\": {}: ended by AIONE ({ended} process(es))",
                r.name,
                if n == 0 {
                    "no window to ask".to_string()
                } else {
                    "still running after 3 s".to_string()
                }
            ),
        );
        let t1 = Instant::now();
        while t1.elapsed() < Duration::from_secs(3) && process_alive(pid) {
            std::thread::sleep(Duration::from_millis(50));
        }
        if process_alive(pid) {
            return Err(format!("{} couldn't be ended (Windows refused)", r.name));
        }
    }
    // The session, recorded once (the watcher then stops quietly).
    let counted = RUNNING.lock().ok().and_then(|g| {
        g.as_ref()
            .filter(|x| x.gen == r.gen)
            .and_then(|x| x.counted_at)
    });
    let session = (take_if(r.gen) && counted.is_some()).then(|| ClosedSession {
        id: r.id.clone(),
        name: r.name.clone(),
        minutes: counted
            .map(|t| t.elapsed().as_secs_f64() / 60.0)
            .unwrap_or(0.0),
    });
    Ok(session)
}

// ---------- the platform layer ----------

#[cfg(windows)]
mod win {
    use windows::core::{BOOL, HSTRING, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT, WPARAM};
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, GetProcessId, OpenProcess, QueryFullProcessImageNameW,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetAncestor, GetClassNameW, GetForegroundWindow, GetGUIThreadInfo,
        GetLastActivePopup, GetSystemMetrics, GetWindow, GetWindowRect, GetWindowTextLengthW,
        GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, PostMessageW, SetWindowPos,
        ShowWindow, GA_ROOT, GUITHREADINFO, GW_OWNER, HWND_TOP, SM_CXSCREEN, SM_CYSCREEN,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_MINIMIZE, SW_SHOWNORMAL, WM_CLOSE,
    };

    pub fn shell_open(
        target: &str,
        params: Option<&str>,
        folder: &std::path::Path,
    ) -> Result<Option<u32>, String> {
        let file = HSTRING::from(target);
        let dir = HSTRING::from(folder.as_os_str());
        let args = HSTRING::from(params.unwrap_or(""));
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI,
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: if params.is_some() {
                PCWSTR(args.as_ptr())
            } else {
                PCWSTR::null()
            },
            lpDirectory: PCWSTR(dir.as_ptr()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        unsafe { ShellExecuteExW(&mut info) }.map_err(|e| e.message().trim().to_string())?;
        if info.hProcess.is_invalid() {
            return Ok(None);
        }
        let pid = unsafe { GetProcessId(info.hProcess) };
        let _ = unsafe { CloseHandle(info.hProcess) };
        Ok(if pid == 0 { None } else { Some(pid) })
    }

    pub fn exe_of(pid: u32) -> Option<String> {
        let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = unsafe {
            QueryFullProcessImageNameW(
                h,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
        };
        let _ = unsafe { CloseHandle(h) };
        ok.ok()?;
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        full.rsplit(['\\', '/']).next().map(str::to_string)
    }

    pub fn process_alive(pid: u32) -> bool {
        let Ok(h) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
            return false;
        };
        let mut code = 0u32;
        let ok = unsafe { GetExitCodeProcess(h, &mut code) }.is_ok();
        let _ = unsafe { CloseHandle(h) };
        ok && code == 259 // STILL_ACTIVE
    }

    pub fn window_alive(w: isize) -> bool {
        unsafe { IsWindow(Some(HWND(w as *mut _))) }.as_bool()
    }

    struct Search {
        pid: u32,
        found: isize,
    }

    unsafe extern "system" fn each(h: HWND, l: LPARAM) -> BOOL {
        let s = unsafe { &mut *(l.0 as *mut Search) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        let visible = unsafe { IsWindowVisible(h) }.as_bool();
        let owned = unsafe { GetWindow(h, GW_OWNER) }.is_ok_and(|o| !o.is_invalid());
        let titled = unsafe { GetWindowTextLengthW(h) } > 0;
        if pid == s.pid && visible && !owned && titled {
            s.found = h.0 as isize;
            return BOOL(0);
        }
        BOOL(1)
    }

    pub fn top_window_of(pid: u32) -> Option<isize> {
        let mut s = Search { pid, found: 0 };
        let _ = unsafe { EnumWindows(Some(each), LPARAM(&mut s as *mut Search as isize)) };
        (s.found != 0).then_some(s.found)
    }

    struct Mains {
        pid: u32,
        best: isize,
        area: i64,
        iconic: isize,
    }

    unsafe extern "system" fn main_each(h: HWND, l: LPARAM) -> BOOL {
        let m = unsafe { &mut *(l.0 as *mut Mains) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        let owned = unsafe { GetWindow(h, GW_OWNER) }.is_ok_and(|o| !o.is_invalid());
        if pid != m.pid || owned || !unsafe { IsWindowVisible(h) }.as_bool() {
            return BOOL(1);
        }
        if unsafe { IsIconic(h) }.as_bool() {
            if m.iconic == 0 {
                m.iconic = h.0 as isize;
            }
            return BOOL(1);
        }
        let mut rc = RECT::default();
        if unsafe { GetWindowRect(h, &mut rc) }.is_ok() {
            let area = i64::from(rc.right - rc.left) * i64::from(rc.bottom - rc.top);
            if area > m.area {
                m.area = area;
                m.best = h.0 as isize;
            }
        }
        BOOL(1)
    }

    /// D-027: the process's main window, found again each time: its largest visible top-level
    /// window, else a minimized one.
    pub fn main_window_of(pid: u32) -> Option<isize> {
        let mut m = Mains {
            pid,
            best: 0,
            area: 0,
            iconic: 0,
        };
        let _ = unsafe { EnumWindows(Some(main_each), LPARAM(&mut m as *mut Mains as isize)) };
        if m.best != 0 {
            Some(m.best)
        } else if m.iconic != 0 {
            Some(m.iconic)
        } else {
            None
        }
    }

    /// The window's active pop-up (e.g. a "Quit?" question), if it has a visible one.
    pub fn active_popup(w: isize) -> Option<isize> {
        let h = HWND(w as *mut _);
        let p = unsafe { GetLastActivePopup(h) };
        (!p.is_invalid() && p != h && unsafe { IsWindowVisible(p) }.as_bool())
            .then_some(p.0 as isize)
    }

    /// D-027: the window's state for the log.
    pub fn window_state(w: isize) -> String {
        let h = HWND(w as *mut _);
        let mut buf = [0u16; 128];
        let n = unsafe { GetClassNameW(h, &mut buf) }.max(0) as usize;
        let class = String::from_utf16_lossy(&buf[..n]);
        let visible = unsafe { IsWindowVisible(h) }.as_bool();
        let minimized = unsafe { IsIconic(h) }.as_bool();
        let top_level = unsafe { GetAncestor(h, GA_ROOT) } == h;
        let owner = unsafe { GetWindow(h, GW_OWNER) }
            .ok()
            .filter(|o| !o.is_invalid())
            .map(|o| format!("{:#x}", o.0 as usize))
            .unwrap_or_else(|| "none".into());
        let mut rc = RECT::default();
        let _ = unsafe { GetWindowRect(h, &mut rc) };
        let (sw, sh) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
        let fullscreen = rc.left <= 0 && rc.top <= 0 && rc.right >= sw && rc.bottom >= sh;
        format!(
            "{:#x}: visible {visible}, minimized {minimized}, top-level {top_level}, class {class}, owner {owner}, fullscreen {fullscreen} ({}x{})",
            w as usize,
            rc.right - rc.left,
            rc.bottom - rc.top
        )
    }

    /// D-027: AIONE steps aside: the game's window to the top (not activated), AIONE minimized,
    /// so Windows activates the game. True when the game's process holds the front afterwards.
    pub fn step_aside(aione: isize, game: isize, pid: u32) -> bool {
        if aione == 0 {
            return false;
        }
        let g = HWND(game as *mut _);
        let _ = unsafe {
            SetWindowPos(
                g,
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        };
        let _ = unsafe { ShowWindow(HWND(aione as *mut _), SW_MINIMIZE) };
        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let fg = unsafe { GetForegroundWindow() };
            let mut fpid = 0u32;
            unsafe { GetWindowThreadProcessId(fg, Some(&mut fpid)) };
            if fpid == pid && pid != 0 {
                return true;
            }
        }
        false
    }

    struct Closer {
        pid: u32,
        sent: usize,
    }

    unsafe extern "system" fn close_each(h: HWND, l: LPARAM) -> BOOL {
        let c = unsafe { &mut *(l.0 as *mut Closer) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        if pid == c.pid
            && unsafe { IsWindowVisible(h) }.as_bool()
            && unsafe { PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0)) }.is_ok()
        {
            c.sent += 1;
        }
        BOOL(1)
    }

    /// WM_CLOSE to every visible top-level window of the process (as its ✕ would).
    pub fn close_windows_of(pid: u32) -> usize {
        let mut c = Closer { pid, sent: 0 };
        let _ = unsafe { EnumWindows(Some(close_each), LPARAM(&mut c as *mut Closer as isize)) };
        c.sent
    }

    /// D-032: every running process: (pid, parent pid, full program path).
    pub fn process_list() -> Vec<(u32, u32, Option<std::path::PathBuf>)> {
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        };
        let mut out = Vec::new();
        let Ok(snap) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
            return out;
        };
        let mut e = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut ok = unsafe { Process32FirstW(snap, &mut e) }.is_ok();
        while ok {
            out.push((
                e.th32ProcessID,
                e.th32ParentProcessID,
                full_path_of(e.th32ProcessID),
            ));
            ok = unsafe { Process32NextW(snap, &mut e) }.is_ok();
        }
        let _ = unsafe { CloseHandle(snap) };
        out
    }

    pub fn full_path_of(pid: u32) -> Option<std::path::PathBuf> {
        let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = unsafe {
            QueryFullProcessImageNameW(
                h,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
        };
        let _ = unsafe { CloseHandle(h) };
        ok.ok()?;
        Some(std::path::PathBuf::from(String::from_utf16_lossy(
            &buf[..len as usize],
        )))
    }

    /// D-032: Steam's folder (`HKCU\Software\Valve\Steam\SteamPath`).
    pub fn steam_root() -> Option<std::path::PathBuf> {
        use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
        let mut buf = [0u16; 1024];
        let mut size = (buf.len() * 2) as u32;
        let r = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                &HSTRING::from("Software\\Valve\\Steam"),
                &HSTRING::from("SteamPath"),
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        if r.is_err() {
            return None;
        }
        let n = buf.iter().position(|&c| c == 0).unwrap_or(0);
        Some(std::path::PathBuf::from(
            String::from_utf16_lossy(&buf[..n]).replace('/', "\\"),
        ))
    }

    /// B-22 rev. ap: a window's rectangle (a move, resize or fullscreen switch changes it).
    pub fn window_rect(w: isize) -> (i32, i32, i32, i32) {
        let mut rc = RECT::default();
        if w != 0 {
            let _ = unsafe { GetWindowRect(HWND(w as *mut _), &mut rc) };
        }
        (rc.left, rc.top, rc.right, rc.bottom)
    }

    /// B-17 (rev. ae): the program holding the keyboard focus (e.g. GameBar.exe).
    pub fn keys_program() -> Option<String> {
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        unsafe { GetGUIThreadInfo(0, &mut info) }.ok()?;
        let h = if info.hwndFocus.is_invalid() {
            info.hwndActive
        } else {
            info.hwndFocus
        };
        if h.is_invalid() {
            return None;
        }
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        exe_of(pid)
    }

    pub fn foreground_program() -> Option<(isize, u32, String)> {
        let h = unsafe { GetForegroundWindow() };
        if h.is_invalid() {
            return None;
        }
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        let exe = exe_of(pid)?;
        Some((h.0 as isize, pid, exe))
    }
}

#[cfg(windows)]
use win::{
    active_popup, close_windows_of, exe_of, foreground_program, keys_program, main_window_of,
    process_alive, process_list, shell_open, steam_root, step_aside, top_window_of, window_alive,
    window_rect, window_state,
};

#[cfg(not(windows))]
fn shell_open(_target: &str, _params: Option<&str>, _folder: &Path) -> Result<Option<u32>, String> {
    Err("starting games needs Windows".into())
}
#[cfg(not(windows))]
fn exe_of(_pid: u32) -> Option<String> {
    None
}
#[cfg(not(windows))]
fn process_alive(_pid: u32) -> bool {
    false
}
#[cfg(not(windows))]
fn window_alive(_w: isize) -> bool {
    false
}
#[cfg(not(windows))]
fn top_window_of(_pid: u32) -> Option<isize> {
    None
}
#[cfg(not(windows))]
fn foreground_program() -> Option<(isize, u32, String)> {
    None
}
#[cfg(not(windows))]
fn close_windows_of(_pid: u32) -> usize {
    0
}
#[cfg(not(windows))]
fn window_rect(_w: isize) -> (i32, i32, i32, i32) {
    (0, 0, 0, 0)
}
#[cfg(not(windows))]
fn process_list() -> Vec<(u32, u32, Option<std::path::PathBuf>)> {
    Vec::new()
}
#[cfg(not(windows))]
fn steam_root() -> Option<std::path::PathBuf> {
    None
}
#[cfg(not(windows))]
fn keys_program() -> Option<String> {
    None
}
#[cfg(not(windows))]
fn main_window_of(_pid: u32) -> Option<isize> {
    None
}
#[cfg(not(windows))]
fn active_popup(_w: isize) -> Option<isize> {
    None
}
#[cfg(not(windows))]
fn window_state(_w: isize) -> String {
    String::new()
}
#[cfg(not(windows))]
fn step_aside(_aione: isize, _game: isize, _pid: u32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::LaunchFields;
    use std::path::PathBuf;

    #[test]
    fn what_is_started() {
        let dir = tempfile::tempdir().unwrap();
        let lnk = dir.path().join("game.lnk");
        std::fs::write(&lnk, b"x").unwrap();
        let f = |shortcut: Option<PathBuf>, steam: Option<&str>| LaunchFields {
            name: "G".into(),
            folder: dir.path().into(),
            shortcut,
            steam_app_id: steam.map(str::to_string),
            no_pause: false,
        };
        assert_eq!(
            target_of(&f(Some(lnk.clone()), Some("1"))).unwrap().1,
            "shortcut game.lnk"
        );
        assert_eq!(
            target_of(&f(None, Some("620"))).unwrap(),
            (
                "steam://rungameid/620".into(),
                "steam://rungameid/620".into()
            )
        );
        assert!(target_of(&f(Some(dir.path().join("gone.lnk")), None))
            .unwrap_err()
            .contains("gone.lnk is missing"));
        assert!(target_of(&f(None, None))
            .unwrap_err()
            .contains("no Shortcut and no SteamAppId"));
        assert!(
            is_not_the_game("SteamWebHelper.exe")
                && is_not_the_game("GameBar.exe")
                && !is_not_the_game("Balatro.exe")
        );
    }

    /// D-032: the real game after a launcher (invented processes and folders).
    #[test]
    fn follows_the_launcher_to_the_game() {
        use std::path::PathBuf;
        let dir = PathBuf::from("D:\\Steam\\steamapps\\common\\An Invented Game");
        let procs = vec![
            (10, 1, Some(PathBuf::from("C:\\Windows\\explorer.exe"))),
            (20, 5, Some(dir.join("Launcher.exe"))),
            (30, 20, Some(PathBuf::from("C:\\Steam\\steamwebhelper.exe"))),
            (40, 7, Some(dir.join("Game_us.exe"))),
            (50, 20, Some(PathBuf::from("E:\\Other\\game_child.exe"))),
        ];
        let windows = |p: u32| p != 10;
        // A child of the launcher with a window comes first.
        assert_eq!(
            pick_game(&procs, 20, Some(&dir), &windows),
            Some((50, "started by the launcher"))
        );
        // No child: the process running from the install folder.
        let no_child: Vec<_> = procs.iter().filter(|p| p.0 != 50).cloned().collect();
        assert_eq!(
            pick_game(&no_child, 20, Some(&dir), &windows),
            Some((40, "runs from the game's install folder"))
        );
        // Steam's helper (a child of the launcher here) is never the game; nothing without a window.
        assert_eq!(pick_game(&no_child, 20, None, &windows), None);
        assert_eq!(pick_game(&no_child, 20, Some(&dir), &|_| false), None);
        assert!(
            is_launcher("Launcher.exe")
                && is_launcher("GameLauncher.exe")
                && !is_launcher("DR2_us.exe")
        );
    }

    /// D-032: Steam's install folder from libraryfolders.vdf and appmanifest (invented files).
    #[test]
    fn finds_a_steam_games_install_folder() {
        let root = tempfile::tempdir().unwrap();
        let lib2 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("steamapps")).unwrap();
        std::fs::create_dir_all(lib2.path().join("steamapps")).unwrap();
        let lib2_vdf = lib2.path().display().to_string().replace('\\', "\\\\");
        std::fs::write(
            root.path().join("steamapps").join("libraryfolders.vdf"),
            format!(
                "\"libraryfolders\"\n{{\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{lib2_vdf}\"\n\t}}\n}}\n"
            ),
        )
        .unwrap();
        std::fs::write(
            lib2.path().join("steamapps").join("appmanifest_123450.acf"),
            "\"AppState\"\n{\n\t\"appid\"\t\t\"123450\"\n\t\"installdir\"\t\t\"An Invented Game\"\n}\n",
        )
        .unwrap();
        assert_eq!(
            steam_install_dir(root.path(), "123450"),
            Some(
                lib2.path()
                    .join("steamapps")
                    .join("common")
                    .join("An Invented Game")
            )
        );
        assert_eq!(steam_install_dir(root.path(), "999"), None);
        // The Steam id from a .url.
        let g = tempfile::tempdir().unwrap();
        let url = g.path().join("game.url");
        std::fs::write(&url, "[InternetShortcut]\nURL=steam://rungameid/123450\n").unwrap();
        let f = crate::games::LaunchFields {
            name: "G".into(),
            folder: g.path().into(),
            shortcut: Some(url),
            steam_app_id: None,
            no_pause: false,
        };
        assert_eq!(steam_app_id_of(&f).as_deref(), Some("123450"));
    }

    /// D-036 (rev. ai): the game's end is the game's end; the wait only for launchers.
    #[test]
    fn the_game_is_the_game() {
        let at10 = Duration::from_secs(10);
        // A counted game ending at 10 s (its window appeared): no wait.
        assert!(!end_is_launcher(true, false, at10, Some("Balatro.exe")));
        // A followed game ending at 10 s (followed after a launcher = counted): no wait.
        assert!(!end_is_launcher(true, false, at10, Some("DR2_us.exe")));
        // Close Software at 10 s, even before the game was counted: no wait.
        assert!(!end_is_launcher(false, true, at10, Some("ze1.exe")));
        // A launcher closing (nothing counted yet): the wait.
        assert!(end_is_launcher(false, false, at10, Some("Launcher.exe")));
        assert!(end_is_launcher(
            false,
            false,
            Duration::from_secs(60),
            Some("GameLauncher.exe")
        ));
        // A process ending before any game appeared, soon after the launch: the wait.
        assert!(end_is_launcher(false, false, at10, Some("start.exe")));
        // An uncounted process ending late, not a launcher: no wait.
        assert!(!end_is_launcher(
            false,
            false,
            Duration::from_secs(60),
            Some("game.exe")
        ));
    }

    /// B-20 (rev. al): AIONE stays behind only when another program was in front.
    #[test]
    fn stays_behind_only_for_another_program() {
        assert_eq!(behind(Some(10), "Balatro.exe", Some(10), 99), None); // the game in front
        assert_eq!(behind(Some(99), "AIONE4.exe", Some(10), 99), None); // AIONE in front
        assert_eq!(
            behind(Some(42), "chrome.exe", Some(10), 99).as_deref(),
            Some("chrome.exe")
        ); // Chrome
        assert_eq!(behind(None, "", Some(10), 99), None); // unknown: AIONE comes forward as before
    }

    /// D-048 / D-046 rev. ar (B-23: any user's Steam).
    #[test]
    fn steam_asking_extends_the_wait_and_b_only_when_nothing_shows() {
        let s = Duration::from_secs;
        assert!(is_steam("steamwebhelper.exe") && is_steam("Steam.exe") && !is_steam("DR2_us.exe"));
        // Without Steam: the usual 5 s wait.
        assert!(!follow_gives_up(s(4), s(10), false));
        assert!(follow_gives_up(s(6), s(10), false));
        // With Steam's window seen: up to the start's 45 s limit.
        assert!(!follow_gives_up(s(30), s(40), true));
        assert!(follow_gives_up(s(40), s(46), true));
        // B is offered only when nothing is showing.
        assert_eq!(
            showing_label(true, false, false),
            Some("Steam is showing a window")
        );
        assert_eq!(
            showing_label(false, true, true),
            Some("the launcher is showing a window")
        );
        assert_eq!(
            showing_label(false, true, false),
            Some("the game is showing a window")
        );
        assert_eq!(showing_label(false, false, false), None);
    }
}
