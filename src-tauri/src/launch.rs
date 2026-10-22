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

/// B-27: does a program's manifest ask for administrator rights? (`requireAdministrator`, or
/// `highestAvailable`, which elevates for an administrator.) Returns the level found.
pub fn manifest_level(bytes: &[u8]) -> Option<&'static str> {
    let find = |n: &[u8]| bytes.windows(n.len()).any(|w| w == n);
    if find(b"requireAdministrator") {
        Some("requireAdministrator")
    } else if find(b"highestAvailable") {
        Some("highestAvailable")
    } else {
        None
    }
}

/// B-27: Windows' compatibility setting (`AppCompatFlags\Layers`) says "Run as administrator".
pub fn layers_run_as_admin(value: &str) -> bool {
    value
        .split_whitespace()
        .any(|f| f.eq_ignore_ascii_case("RUNASADMIN"))
}

/// B-27: what the program asks for (its manifest, read in 1 MB chunks up to 256 MB; Windows'
/// compatibility setting for it, for this user and for all users).
pub fn asks_for_admin(exe: &std::path::Path) -> String {
    use std::io::Read;
    let mut manifest = None;
    if let Ok(mut f) = std::fs::File::open(exe) {
        let mut buf = vec![0u8; 1 << 20];
        let mut carry: Vec<u8> = Vec::new();
        let mut read = 0usize;
        while read < 256 << 20 {
            let n = f.read(&mut buf).unwrap_or(0);
            if n == 0 {
                break;
            }
            read += n;
            let mut chunk = std::mem::take(&mut carry);
            chunk.extend_from_slice(&buf[..n]);
            if let Some(l) = manifest_level(&chunk) {
                manifest = Some(l);
                break;
            }
            carry = chunk[chunk.len().saturating_sub(32)..].to_vec();
        }
    }
    let compat = compat_layers(exe).is_some_and(|v| layers_run_as_admin(&v));
    format!(
        "manifest {}; compatibility setting {}",
        manifest.unwrap_or("asInvoker (none)"),
        if compat {
            "\"Run as administrator\""
        } else {
            "no"
        }
    )
}

/// B-27: measured and logged at every launch, and for the game once it is followed.
pub fn log_admin(name: &str, pid: Option<u32>) {
    let me = elevated(None)
        .map(|b| b.to_string())
        .unwrap_or_else(|| "unknown".into());
    let (exe, it) = match pid {
        Some(p) => (
            full_path_of_pid(p),
            elevated(Some(p))
                .map(|b| b.to_string())
                .unwrap_or_else(|| "unknown (Windows refused the query)".into()),
        ),
        None => (None, "unknown (no process)".into()),
    };
    let asks = exe
        .as_deref()
        .map(asks_for_admin)
        .unwrap_or_else(|| "unknown (no program path)".into());
    let prog = exe
        .as_ref()
        .and_then(|e| e.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    logging::lifecycle(
        Level::Info,
        &format!("admin: \"{name}\": AIONE elevated {me}; the game's process ({prog}) elevated {it}; it asks for administrator rights: {asks}"),
    );
    if asks.contains("requireAdministrator") || asks.contains("Run as administrator") {
        logging::lifecycle(
            Level::Info,
            &format!("guide: if \"{name}\" doesn't need administrator rights: right-click its program ({prog}) → Properties → Compatibility → untick \"Run this program as an administrator\" (also under \"Change settings for all users\"); a manifest that asks can't be changed this way"),
        );
    }
}

/// B-28: the running programs' file names (for the Magnifier check).
pub fn process_names() -> Vec<String> {
    process_list()
        .iter()
        .filter_map(|(_, _, p)| {
            p.as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
        })
        .collect()
}

/// B-27: Windows' permission prompt (UAC, `consent.exe`) is showing.
pub fn consent_showing() -> bool {
    process_list().iter().any(|(_, _, p)| {
        p.as_ref()
            .map(|p| {
                p.to_string_lossy()
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or_default()
                    .eq_ignore_ascii_case("consent.exe")
            })
            .unwrap_or(false)
    })
}

/// D-050: the start whose shell launch call is still blocking (Windows' prompt holds it), and
/// whether Windows' permission prompt was seen meanwhile.
static SHELL_BLOCKING: Mutex<Option<(String, Instant, bool)>> = Mutex::new(None);

/// D-050: is Windows' permission prompt holding the start of `id`?
fn prompt_holding(id: &str) -> bool {
    SHELL_BLOCKING
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .is_some_and(|(i, _, seen)| i == id && seen)
}

/// D-050: the shell's launch call, watched while it blocks (Windows' permission prompt holds it
/// until it is answered); logs when the prompt appeared and how it ended.
fn shell_open_watched(
    id: &str,
    name: &str,
    target: &str,
    params: Option<&str>,
    folder: &std::path::Path,
) -> Result<Option<u32>, String> {
    if let Ok(mut g) = SHELL_BLOCKING.lock() {
        *g = Some((id.to_string(), Instant::now(), false));
    }
    let (id2, name2) = (id.to_string(), name.to_string());
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(150));
        let Some((i, t0, seen)) = SHELL_BLOCKING.lock().ok().and_then(|g| g.clone()) else {
            return;
        };
        if i != id2 {
            return;
        }
        if !seen && t0.elapsed() > Duration::from_millis(300) && consent_showing() {
            if let Ok(mut g) = SHELL_BLOCKING.lock() {
                if let Some(x) = g.as_mut().filter(|x| x.0 == id2) {
                    x.2 = true;
                }
            }
            logging::lifecycle(Level::Info, &format!("start \"{name2}\": Windows is asking for permission (UAC): waiting for the answer"));
        }
    });
    let result = shell_open(target, params, folder);
    let state = SHELL_BLOCKING.lock().ok().and_then(|mut g| g.take());
    let seen = state.as_ref().is_some_and(|s| s.2);
    let ms = state.map(|s| s.1.elapsed().as_millis()).unwrap_or(0);
    let cancelled = matches!(&result, Err(e) if e.to_ascii_lowercase().contains("cancel"));
    if seen || cancelled {
        logging::lifecycle(
            Level::Info,
            &format!(
                "start \"{name}\": Windows' permission prompt: {} after {ms} ms",
                if result.is_ok() {
                    "allowed"
                } else {
                    "cancelled"
                }
            ),
        );
    }
    if cancelled {
        return Err(format!(
            "{name} needs administrator permission; it wasn't given"
        ));
    }
    result
}

/// B-27: whether a process (None: AIONE) runs with administrator rights (token elevation).
pub fn elevated(pid: Option<u32>) -> Option<bool> {
    token_elevated(pid)
}

/// D-048 (B-22 rev. aq): Steam's own window (any user's Steam: its client or its web helper).
pub fn is_steam(exe: &str) -> bool {
    let e = exe.to_ascii_lowercase();
    e == "steamwebhelper.exe" || e == "steam.exe"
}

/// D-048: does the launcher wait give up now? With Steam's window seen during it ("Steam is asking
/// something"), only at the start's 45 s limit; otherwise after the usual wait.
pub fn follow_gives_up(since_follow: Duration, since_launch: Duration, steam_seen: bool) -> bool {
    follow_gives_up_chain(since_follow, since_launch, steam_seen, false)
}

/// D-055: with any process of the launcher's chain alive, the wait lasts to the start's 45 s; the
/// 5 s wait ends a start only when no process of the chain is alive.
pub fn follow_gives_up_chain(
    since_follow: Duration,
    since_launch: Duration,
    steam_seen: bool,
    chain_alive: bool,
) -> bool {
    if steam_seen || chain_alive {
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
    // B-27: Windows' permission prompt holds the start until it is answered.
    if prompt_holding(id) || consent_showing() {
        return Some(
            "Windows is asking for permission: answer it with the mouse or keyboard".into(),
        );
    }
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

/// D-055 (rev. bb): a launcher's whole process chain: the root and every descendant by parent
/// process (children, grandchildren, …), the alive ones, sorted. Windows keeps a dead parent's id on
/// its children, so the chain survives the launcher exiting.
pub fn chain_of(procs: &[(u32, u32, Option<std::path::PathBuf>)], root: u32) -> Vec<u32> {
    let mut known = std::collections::HashSet::from([root]);
    loop {
        let before = known.len();
        for (pid, parent, _) in procs {
            if known.contains(parent) && *pid != 0 && *pid != *parent {
                known.insert(*pid);
            }
        }
        if known.len() == before {
            break;
        }
    }
    let mut alive: Vec<u32> = procs
        .iter()
        .map(|(p, _, _)| *p)
        .filter(|p| known.contains(p))
        .collect();
    alive.sort_unstable();
    alive.dedup();
    alive
}

/// The program names of some processes (for the log).
fn names_of(procs: &[(u32, u32, Option<std::path::PathBuf>)], pids: &[u32]) -> String {
    pids.iter()
        .map(|p| {
            let n = procs
                .iter()
                .find(|(x, _, _)| x == p)
                .and_then(|(_, _, path)| path.as_ref())
                .map(|path| {
                    path.to_string_lossy()
                        .rsplit(['\\', '/'])
                        .next()
                        .unwrap_or_default()
                        .to_string()
                })
                .unwrap_or_else(|| "?".into());
            format!("{n} ({p})")
        })
        .collect::<Vec<_>>()
        .join(", ")
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
    // D-055: any process of the launcher's chain (children, grandchildren, …), not only children.
    let chain = chain_of(procs, launcher);
    if let Some((pid, _, _)) = procs
        .iter()
        .find(|(pid, _, path)| chain.contains(pid) && ok(*pid, path))
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
        Some((exe, params, dir)) => shell_open_watched(id, &f.name, &exe, Some(&params), &dir)?,
        None => shell_open_watched(id, &f.name, &target, None, &f.folder)?,
    };
    // B-27: measured and logged at every launch.
    log_admin(&f.name, pid);
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
    // Rev. ax: a new game session (the "can't pause it" message may show once again).
    crate::pause::new_session();
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
        .filter(|p| tree_alive(*p))
        .ok_or("its process has ended")?;
    let (w, _) = game_window(pid).ok_or("no visible window of the game: no hand-back")?;
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
    if r.pid.is_some() && !alive(r) {
        return Err(format!(
            "{what} \"{}\": its process has ended (no hand-over)",
            r.name
        ));
    }
    let found = r.pid.and_then(game_window);
    let target = found
        .map(|(w, _)| w)
        .or(r.window.filter(|w| window_alive(*w)));
    let Some(main) = target else {
        let m = format!(
            "{what} \"{}\": no window of {} or the programs it started found",
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
            "{what} \"{}\": its window {} ({}){}",
            r.name,
            window_state(main),
            match found {
                Some((_, p)) => format!(
                    "{}, process {p}{}",
                    exe_of(p).unwrap_or_else(|| "unknown".into()),
                    if Some(p) == r.pid {
                        ""
                    } else {
                        ", started by the game's process"
                    }
                ),
                None => "its last known window".into(),
            },
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

/// D-058: one top-level window of the game's process tree, as Windows lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSeen {
    pub window: isize,
    pub pid: u32,
    pub area: i64,
    pub minimized: bool,
    /// Its place in Windows' window order (0 = on top: shown or activated most recently).
    pub order: usize,
}

/// D-058: the game's main window among its tree's windows: the largest visible one, the one
/// higher in Windows' window order (shown or activated more recently) when two are as large;
/// else the highest minimized one. With its process.
pub fn pick_window(seen: &[WindowSeen]) -> Option<(isize, u32)> {
    let best = |minimized: bool| {
        seen.iter()
            .filter(|w| w.minimized == minimized)
            .min_by_key(|w| {
                (
                    std::cmp::Reverse(if minimized { 0 } else { w.area }),
                    w.order,
                )
            })
            .map(|w| (w.window, w.pid))
    };
    best(false).or_else(|| best(true))
}

/// D-058: the game's main window, found again each time **across its process tree** (the
/// counted process and every process it started, as the pause and the close see it), with the
/// process it belongs to. Minecraft: the launcher's `javaw.exe` hides its window while the game
/// it started (a second `javaw.exe`) shows the big one.
fn game_window(pid: u32) -> Option<(isize, u32)> {
    let tree = crate::pause::tree(&process_list(), pid);
    pick_window(&windows_of(&tree))
}

/// D-058: the game (once counted) lives while its process does, or while a process it started
/// shows a window. A leftover helper without a window never keeps it running.
fn tree_alive(pid: u32) -> bool {
    process_alive(pid) || game_window(pid).is_some()
}

fn alive(r: &Running) -> bool {
    match r.pid {
        Some(p) if r.counted_at.is_some() => tree_alive(p),
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
    // D-055: the launcher's process chain being watched (logged when it changes).
    let mut watched: Vec<u32> = Vec::new();
    let mut chain_root: Option<u32> = None;
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
            let w = r.pid.and_then(game_window).map(|(w, _)| w).unwrap_or(0);
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
        // D-051 (rev. ax): a start with no window by about 3 min (watched quietly since the UI's
        // 45 s) ends quietly; nothing was counted.
        if !r.cancelled
            && r.window.is_none()
            && r.counted_at.is_none()
            && following.is_none()
            && since > Duration::from_secs(180)
        {
            if take_if(gen) {
                logging::lifecycle(Level::Info, &format!("start \"{}\": no window after about 3 min: the quiet watch ends (nothing counted)", r.name));
                report(Event::Ended {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    minutes: 0.0,
                    counted: false,
                    behind: None,
                });
            }
            return;
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
                    // B-27: the game itself (after a launcher, a different process).
                    log_admin(&r.name, r.pid);
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
            // D-055 (rev. bb): a window-less start (a launcher like Minecraft's LL.exe) follows the
            // launcher's WHOLE process chain while any of it is alive; the first process of it to show
            // a window is followed as the game. It ends only when no process of the chain is alive.
            let mut chain_alive = false;
            if r.window.is_none() {
                if chain_root.is_none() {
                    chain_root = r.pid; // the process the start began with (the launcher)
                }
                if let Some(root) = chain_root {
                    let procs = process_list();
                    let chain = chain_of(&procs, root);
                    chain_alive = !chain.is_empty();
                    if chain != watched {
                        if !chain.is_empty() {
                            logging::lifecycle(
                                Level::Info,
                                &format!(
                                    "start \"{}\": watching the launcher's processes: {}",
                                    r.name,
                                    names_of(&procs, &chain)
                                ),
                            );
                        }
                        watched = chain.clone();
                    }
                    if let Some(&p) = chain
                        .iter()
                        .find(|&&p| Some(p) != r.pid && main_window_of(p).is_some())
                    {
                        let exe = names_of(&procs, &[p]);
                        logging::lifecycle(Level::Info, &format!("start \"{}\": {exe} (from the launcher's chain) shows a window: following it", r.name));
                        r.pid = Some(p);
                        r.exe = procs
                            .iter()
                            .find(|(x, _, _)| *x == p)
                            .and_then(|(_, _, path)| path.as_ref())
                            .and_then(|path| path.file_name())
                            .map(|n| n.to_string_lossy().into_owned());
                        if let Ok(mut g) = RUNNING.lock() {
                            if let Some(x) = g.as_mut().filter(|x| x.gen == gen) {
                                x.pid = r.pid;
                                x.exe = r.exe.clone();
                            }
                        }
                    }
                }
            }
            // A launcher whose whole chain closed without a game window counts as ended (B-18); so
            // does a start that never shows a window within the quiet watch (D-051, 3 min).
            let launcher_gone = !chain_alive
                && r.pid.is_some_and(|p| !process_alive(p))
                && since > Duration::from_secs(10);
            if r.window.is_none() && (launcher_gone || since > Duration::from_secs(180)) {
                logging::lifecycle(
                    Level::Warn,
                    &format!(
                        "start \"{}\": no game window ({}); counted as ended",
                        r.name,
                        if launcher_gone {
                            "its launcher and every process it started closed"
                        } else {
                            "3 minutes passed"
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
            if !steam_seen
                && (foreground_program().is_some_and(|(_, _, e)| is_steam(&e)) || consent_showing())
            {
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
            // D-055: the launcher's chain still alive (its children, grandchildren, …) keeps the wait.
            let chain_alive = chain_of(&procs, launcher).iter().any(|p| *p != launcher);
            let gives_up =
                follow_gives_up_chain(since_follow.elapsed(), since, steam_seen, chain_alive);
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
    let tree_alive = |root: u32| {
        crate::pause::tree(&process_list(), root)
            .into_iter()
            .filter(|p| process_alive(*p))
            .count()
    };
    // D-058: every window of the game's tree (Minecraft: the game the launcher started, too).
    let tree = crate::pause::tree(&process_list(), pid);
    let n: usize = tree.iter().map(|p| close_windows_of(*p)).sum();
    logging::lifecycle(
        Level::Info,
        &format!(
            "close \"{}\": asked {n} window(s) of {} and the programs it started ({} process(es)) to close (as their ✕ would); up to 3 s",
            r.name,
            r.exe.clone().unwrap_or_default(),
            tree.len()
        ),
    );
    // About 3 s for the game to close itself (most games save their settings in that time).
    while n > 0 && t0.elapsed() < CLOSE_GRACE && tree_alive(pid) > 0 {
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
        if !process_alive(pid) {
            logging::lifecycle(
                Level::Info,
                &format!(
                    "close \"{}\": verified: the process has exited after {} ms",
                    r.name,
                    t1.elapsed().as_millis()
                ),
            );
        }
        if process_alive(pid) {
            // B-27: plainly; never retried in a loop.
            let admin = elevated(Some(pid)) == Some(true) && elevated(None) != Some(true);
            let m = if admin {
                format!("{} runs as administrator: AIONE can't close it", r.name)
            } else {
                format!("{} couldn't be ended (Windows refused)", r.name)
            };
            logging::lifecycle(
                Level::Warn,
                &format!("close \"{}\": Windows refused to end it: {m}", r.name),
            );
            return Err(m);
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

    struct Seen {
        pids: Vec<u32>,
        out: Vec<super::WindowSeen>,
        order: usize,
    }

    unsafe extern "system" fn seen_each(h: HWND, l: LPARAM) -> BOOL {
        let s = unsafe { &mut *(l.0 as *mut Seen) };
        let order = s.order;
        s.order += 1;
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        let owned = unsafe { GetWindow(h, GW_OWNER) }.is_ok_and(|o| !o.is_invalid());
        if !s.pids.contains(&pid) || owned || !unsafe { IsWindowVisible(h) }.as_bool() {
            return BOOL(1);
        }
        let minimized = unsafe { IsIconic(h) }.as_bool();
        let mut rc = RECT::default();
        let area = if unsafe { GetWindowRect(h, &mut rc) }.is_ok() {
            i64::from(rc.right - rc.left) * i64::from(rc.bottom - rc.top)
        } else {
            0
        };
        s.out.push(super::WindowSeen {
            window: h.0 as isize,
            pid,
            area,
            minimized,
            order,
        });
        BOOL(1)
    }

    /// D-058: the visible top-level windows (not owned pop-ups) of these processes, in Windows'
    /// window order (EnumWindows lists the top one first), the same rule as `main_window_of`.
    pub fn windows_of(pids: &[u32]) -> Vec<super::WindowSeen> {
        let mut s = Seen {
            pids: pids.to_vec(),
            out: Vec::new(),
            order: 0,
        };
        let _ = unsafe { EnumWindows(Some(seen_each), LPARAM(&mut s as *mut Seen as isize)) };
        s.out
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
        found: usize,
    }

    unsafe extern "system" fn close_each(h: HWND, l: LPARAM) -> BOOL {
        let c = unsafe { &mut *(l.0 as *mut Closer) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        // D-052 (rev. ax): every top-level window of the game, minimized ones too (a minimized
        // window is still visible to Windows). Windows may refuse the message (UIPI: the game runs
        // as administrator and AIONE doesn't): counted apart, for the log.
        if pid == c.pid && unsafe { IsWindowVisible(h) }.as_bool() {
            c.found += 1;
            if unsafe { PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0)) }.is_ok() {
                c.sent += 1;
            }
        }
        BOOL(1)
    }

    /// WM_CLOSE to every visible top-level window of the process (as its ✕ would).
    pub fn close_windows_of(pid: u32) -> usize {
        let mut c = Closer {
            pid,
            sent: 0,
            found: 0,
        };
        let _ = unsafe { EnumWindows(Some(close_each), LPARAM(&mut c as *mut Closer as isize)) };
        if c.found > c.sent {
            crate::logging::lifecycle(crate::logging::Level::Warn, &format!("close: found {} window(s); Windows refused the close message for {} (UIPI: the game runs as administrator)", c.found, c.found - c.sent));
        }
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
            // D-050: a process AIONE can't open (SYSTEM, e.g. consent.exe) still has the
            // snapshot's own file name, which needs no access.
            let name = {
                let n = e
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(e.szExeFile.len());
                String::from_utf16_lossy(&e.szExeFile[..n])
            };
            let path = full_path_of(e.th32ProcessID)
                .or_else(|| (!name.is_empty()).then(|| std::path::PathBuf::from(&name)));
            out.push((e.th32ProcessID, e.th32ParentProcessID, path));
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

    /// B-27: a process's token elevation (None pid: AIONE itself).
    pub fn token_elevated(pid: Option<u32>) -> Option<bool> {
        use windows::Win32::Security::{
            GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
        };
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
        let (process, owned) = match pid {
            None => (unsafe { GetCurrentProcess() }, false),
            Some(p) => (
                unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, p) }.ok()?,
                true,
            ),
        };
        let mut token = windows::Win32::Foundation::HANDLE::default();
        let opened = unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }.is_ok();
        if owned {
            let _ = unsafe { CloseHandle(process) };
        }
        if !opened {
            return None;
        }
        let mut e = TOKEN_ELEVATION::default();
        let mut len = 0u32;
        let ok = unsafe {
            GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut e as *mut TOKEN_ELEVATION as *mut core::ffi::c_void),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut len,
            )
        }
        .is_ok();
        let _ = unsafe { CloseHandle(token) };
        ok.then_some(e.TokenIsElevated != 0)
    }

    /// B-27: Windows' compatibility setting for a program (`AppCompatFlags\Layers`: this user,
    /// then all users).
    pub fn compat_layers(exe: &std::path::Path) -> Option<String> {
        use windows::Win32::System::Registry::{
            RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ,
        };
        let key = HSTRING::from(
            "Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers",
        );
        let value = HSTRING::from(exe.as_os_str());
        for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            let mut buf = [0u16; 512];
            let mut size = (buf.len() * 2) as u32;
            let r = unsafe {
                RegGetValueW(
                    root,
                    &key,
                    &value,
                    RRF_RT_REG_SZ,
                    None,
                    Some(buf.as_mut_ptr().cast()),
                    Some(&mut size),
                )
            };
            if r.is_ok() {
                let n = buf.iter().position(|&c| c == 0).unwrap_or(0);
                return Some(String::from_utf16_lossy(&buf[..n]));
            }
        }
        None
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
    active_popup, close_windows_of, compat_layers, exe_of, foreground_program, keys_program,
    main_window_of, process_alive, process_list, shell_open, steam_root, step_aside,
    token_elevated, top_window_of, window_alive, window_rect, window_state, windows_of,
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
fn token_elevated(_pid: Option<u32>) -> Option<bool> {
    None
}
#[cfg(not(windows))]
fn compat_layers(_exe: &std::path::Path) -> Option<String> {
    None
}
#[cfg(not(windows))]
fn full_path_of_pid(_pid: u32) -> Option<std::path::PathBuf> {
    None
}
#[cfg(windows)]
fn full_path_of_pid(pid: u32) -> Option<std::path::PathBuf> {
    win::full_path_of(pid)
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
fn windows_of(_pids: &[u32]) -> Vec<WindowSeen> {
    Vec::new()
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

    /// B-27: the manifest and Windows' compatibility setting, read the same for every user.
    #[test]
    fn what_a_game_asks_for() {
        assert_eq!(
            manifest_level(
                b"<requestedExecutionLevel level=\"requireAdministrator\" uiAccess=\"false\"/>"
            ),
            Some("requireAdministrator")
        );
        assert_eq!(
            manifest_level(b"level='highestAvailable'"),
            Some("highestAvailable")
        );
        assert_eq!(manifest_level(b"level=\"asInvoker\""), None);
        assert!(layers_run_as_admin("~ RUNASADMIN"));
        assert!(layers_run_as_admin("~ WIN7RTM RUNASADMIN HIGHDPIAWARE"));
        assert!(!layers_run_as_admin("~ HIGHDPIAWARE"));
    }

    /// D-055: a launcher's whole chain, whatever the launcher (Minecraft: LL.exe → javaw.exe).
    #[test]
    fn the_whole_launcher_chain_is_followed() {
        use std::path::PathBuf;
        let p = |pid: u32, parent: u32, name: &str| {
            (pid, parent, Some(PathBuf::from(format!("C:\\x\\{name}"))))
        };
        // LL.exe (100) has exited; its child (200) and grandchild javaw.exe (300) live on.
        let procs = vec![
            p(4, 0, "System"),
            p(200, 100, "launcherhelper.exe"),
            p(300, 200, "javaw.exe"),
            p(999, 4, "explorer.exe"),
        ];
        assert_eq!(chain_of(&procs, 100), vec![200, 300]);
        // The grandchild with a window is the game (not only direct children).
        let has_window = |pid: u32| pid == 300;
        assert_eq!(
            pick_game(&procs, 100, None, &has_window).map(|g| g.0),
            Some(300)
        );
        // The wait: the chain alive keeps it to the start's 45 s; nothing alive: the usual 5 s.
        let s = Duration::from_secs;
        assert!(!follow_gives_up_chain(s(20), s(25), false, true)); // a window coming at 20 s: still waiting
        assert!(follow_gives_up_chain(s(20), s(46), false, true));
        assert!(follow_gives_up_chain(s(6), s(10), false, false));
        // Nothing of the chain alive: empty.
        assert!(chain_of(&procs, 555).is_empty());
    }

    /// D-058 (A14.2, the reported cause): a launcher (like Minecraft's `javaw.exe`) whose own window
    /// hides while the game it started (a child process) shows the big window: the return must find
    /// the child's window, not "no window". Real processes and windows
    /// (`examples/window_fixture.rs launcher` → `game`).
    #[cfg(windows)]
    #[test]
    fn d058_the_return_finds_the_game_the_launcher_started() {
        let _one = crate::testfix::one_at_a_time();
        let launcher = crate::testfix::Fixture::start(&["launcher"]);
        let line = launcher.line("launcher=");
        let launcher_window = crate::testfix::field(&line, "launcher") as isize;
        let game_pid = crate::testfix::field(&line, "game_pid") as u32;
        // The game's window takes a moment to appear.
        let t0 = Instant::now();
        let mut found = None;
        while t0.elapsed() < Duration::from_secs(10) {
            found = super::game_window(launcher.pid());
            if found.is_some_and(|(_, p)| p == game_pid) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        crate::testfix::notice(
            "D-058 proof",
            &format!(
                "launcher pid {} (window {launcher_window}, hidden), game pid {game_pid}: found {found:?}",
                launcher.pid()
            ),
        );
        let (w, p) = found.expect("no window of the launcher's process tree found");
        assert_eq!(p, game_pid, "the window found isn't the game's");
        assert_ne!(w, launcher_window);
    }

    /// D-058: the largest visible window wins; on a tie the one higher in Windows' order (shown
    /// more recently); a minimized window only when nothing visible is left.
    #[test]
    fn the_games_window_is_the_largest_visible_one_of_its_tree() {
        let w = |window, pid, area, minimized, order| WindowSeen {
            window,
            pid,
            area,
            minimized,
            order,
        };
        // Minecraft: the launcher's small window and the game's big one (the game higher).
        let both = [
            w(1, 14372, 424 * 127, false, 3),
            w(2, 15904, 1920 * 1080, false, 0),
        ];
        assert_eq!(pick_window(&both), Some((2, 15904)));
        // As large: the more recent (higher) one.
        let tie = [w(1, 10, 500, false, 5), w(2, 11, 500, false, 1)];
        assert_eq!(pick_window(&tie), Some((2, 11)));
        // A visible window beats a larger minimized one.
        let min = [w(1, 10, 9_000_000, true, 0), w(2, 11, 50_000, false, 4)];
        assert_eq!(pick_window(&min), Some((2, 11)));
        // Only minimized ones: the highest.
        let only = [w(1, 10, 100, true, 7), w(2, 11, 900, true, 2)];
        assert_eq!(pick_window(&only), Some((2, 11)));
        assert_eq!(pick_window(&[]), None);
    }

    /// D-058, the real mechanism: when the game the launcher started ends, the launcher's window
    /// comes back and is the game's window again (the tree still has a window: not the end yet).
    #[cfg(windows)]
    #[test]
    fn d058_when_the_started_game_ends_the_launchers_window_is_found_again() {
        let _one = crate::testfix::one_at_a_time();
        let launcher = crate::testfix::Fixture::start(&["launcher"]);
        let line = launcher.line("launcher=");
        let launcher_window = crate::testfix::field(&line, "launcher") as isize;
        let game_pid = crate::testfix::field(&line, "game_pid") as u32;
        let t0 = Instant::now();
        while game_window(launcher.pid()).is_none_or(|(_, p)| p != game_pid)
            && t0.elapsed() < Duration::from_secs(10)
        {
            std::thread::sleep(Duration::from_millis(100));
        }
        // The game ends (killed, as a crash or a quit would end it).
        assert!(terminate(game_pid), "couldn't end the fixture's game");
        launcher.line("game_ended");
        let t1 = Instant::now();
        let mut found = None;
        while t1.elapsed() < Duration::from_secs(5) {
            found = game_window(launcher.pid());
            if found.is_some_and(|(w, _)| w == launcher_window) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        crate::testfix::notice(
            "D-058 proof (the game ended)",
            &format!("after the started game ended: found {found:?}; the launcher's window is {launcher_window}; the tree still alive: {}", tree_alive(launcher.pid())),
        );
        assert_eq!(found, Some((launcher_window, launcher.pid())));
        assert!(tree_alive(launcher.pid()));
    }
}
