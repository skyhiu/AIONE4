//! B-19 part 1 (rev. ad): emulator profiles. AIONE reads the game's `game.lnk` (only reads it),
//! recognizes the emulator by its program, and adds only the options the shortcut lacks when it
//! launches. Profiles are data (`emulators.json`); every option comes from the emulator's own
//! source (docs/EMULATORS.md). AIONE never changes a shortcut or an emulator's files.

use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub programs: Vec<String>,
    pub verified: Option<String>,
    pub options: Vec<Opt>,
    pub guide: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Opt {
    pub add: Vec<String>,
    pub present: Vec<String>,
    #[serde(default)]
    pub needs: Vec<String>,
    pub point: String,
    pub why: String,
}

#[derive(Deserialize)]
struct File {
    profiles: Vec<Profile>,
}

pub fn profiles() -> Vec<Profile> {
    serde_json::from_str::<File>(include_str!("../emulators.json"))
        .map(|f| f.profiles)
        .unwrap_or_default()
}

/// The profile whose program is the shortcut's target (by file name, any case).
pub fn recognize<'a>(profiles: &'a [Profile], target: &Path) -> Option<&'a Profile> {
    // The name after the last \ or / (the same on every system; shortcuts hold Windows paths).
    let full = target.to_string_lossy();
    let exe = full.rsplit(['\\', '/']).next()?.to_ascii_lowercase();
    profiles
        .iter()
        .find(|p| p.programs.iter().any(|x| x.eq_ignore_ascii_case(&exe)))
}

/// Splits a command line as Windows does for programs (quotes group; `\"` is a quote).
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
                any = true;
            }
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(cur);
    }
    out
}

/// What to add: each option whose spellings are all absent (never twice) and whose needs are met.
/// Returns the options added and the ones skipped with the reason.
pub fn missing<'a>(p: &'a Profile, args: &str) -> (Vec<&'a Opt>, Vec<(&'a Opt, String)>) {
    let have = split_args(args);
    let has = |spell: &str| {
        have.iter().any(|a| {
            if spell.ends_with('=') {
                a.to_ascii_lowercase()
                    .starts_with(&spell.to_ascii_lowercase())
            } else {
                a.eq_ignore_ascii_case(spell)
            }
        })
    };
    let mut add = Vec::new();
    let mut skip = Vec::new();
    for o in &p.options {
        if o.present.iter().any(|s| has(s)) {
            skip.push((o, "already in the shortcut".to_string()));
        } else if !o.needs.is_empty() && !o.needs.iter().any(|s| has(s)) {
            skip.push((o, format!("needs {} in the shortcut", o.needs.join(" or "))));
        } else {
            add.push(o);
        }
    }
    (add, skip)
}

/// The added options as one argument string (quoted where needed), placed BEFORE the shortcut's
/// own arguments (the game file stays last for emulators that take it as the last argument).
pub fn joined(opts: &[&Opt]) -> String {
    opts.iter()
        .flat_map(|o| o.add.iter())
        .map(|a| {
            if a.contains(' ') {
                format!("\"{a}\"")
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------- the .lnk reader (MS-SHLLINK; reading only) ----------

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Shortcut {
    pub target: Option<PathBuf>,
    pub arguments: String,
    pub working_dir: Option<PathBuf>,
}

fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}
fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}
fn cstr(b: &[u8], o: usize) -> Option<String> {
    let rest = b.get(o..)?;
    let end = rest.iter().position(|&c| c == 0)?;
    Some(rest[..end].iter().map(|&c| c as char).collect())
}
fn wstr(b: &[u8], o: usize) -> Option<String> {
    let mut v = Vec::new();
    let mut i = o;
    loop {
        let c = u16_at(b, i)?;
        if c == 0 {
            break;
        }
        v.push(c);
        i += 2;
    }
    Some(String::from_utf16_lossy(&v))
}

/// Reads a `.lnk` file's target, arguments and working folder (never writes).
pub fn read_lnk(bytes: &[u8], lnk_folder: &Path) -> Result<Shortcut, String> {
    if u32_at(bytes, 0) != Some(0x4C) {
        return Err("not a Windows shortcut".into());
    }
    let flags = u32_at(bytes, 20).ok_or("short file")?;
    let unicode = flags & 0x80 != 0;
    let mut pos = 0x4C;
    if flags & 0x1 != 0 {
        pos += 2 + u16_at(bytes, pos).ok_or("short file")? as usize;
    }
    let mut target = None;
    if flags & 0x2 != 0 {
        let size = u32_at(bytes, pos).ok_or("short file")? as usize;
        let header = u32_at(bytes, pos + 4).ok_or("short file")?;
        let li_flags = u32_at(bytes, pos + 8).ok_or("short file")?;
        if li_flags & 1 != 0 {
            let base_off = u32_at(bytes, pos + 16).ok_or("short file")? as usize;
            let suffix_off = u32_at(bytes, pos + 24).ok_or("short file")? as usize;
            let mut base = cstr(bytes, pos + base_off).unwrap_or_default();
            let mut suffix = cstr(bytes, pos + suffix_off).unwrap_or_default();
            if header >= 0x24 {
                if let Some(o) = u32_at(bytes, pos + 28).filter(|o| *o != 0) {
                    base = wstr(bytes, pos + o as usize).unwrap_or(base);
                }
                if let Some(o) = u32_at(bytes, pos + 32).filter(|o| *o != 0) {
                    suffix = wstr(bytes, pos + o as usize).unwrap_or(suffix);
                }
            }
            if !base.is_empty() {
                target = Some(PathBuf::from(format!("{base}{suffix}")));
            }
        }
        pos += size;
    }
    let mut strings: [Option<String>; 5] = Default::default();
    for (i, bit) in [0x4u32, 0x8, 0x10, 0x20, 0x40].iter().enumerate() {
        if flags & bit != 0 {
            let n = u16_at(bytes, pos).ok_or("short file")? as usize;
            pos += 2;
            let s = if unicode {
                let raw = bytes.get(pos..pos + n * 2).ok_or("short file")?;
                pos += n * 2;
                String::from_utf16_lossy(
                    &raw.chunks(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect::<Vec<_>>(),
                )
            } else {
                let raw = bytes.get(pos..pos + n).ok_or("short file")?;
                pos += n;
                raw.iter().map(|&c| c as char).collect()
            };
            strings[i] = Some(s);
        }
    }
    let [_, relative, working, args, _] = strings;
    if target.is_none() {
        target = relative.map(|r| lnk_folder.join(r.replace('\\', std::path::MAIN_SEPARATOR_STR)));
    }
    Ok(Shortcut {
        target,
        arguments: args.unwrap_or_default(),
        working_dir: working.map(PathBuf::from),
    })
}

/// D-034 (rev. af): reads a shortcut with Windows' own interface (`IShellLinkW` + `IPersistFile`,
/// read only: the same one Explorer uses), which handles every form a `.lnk` stores its target
/// in (ID list, network or mapped drives, `%…%` variables). Elsewhere (tests), the byte reader.
pub fn read_shortcut(path: &Path) -> Result<Shortcut, String> {
    #[cfg(windows)]
    {
        win::read(path)
    }
    #[cfg(not(windows))]
    {
        let bytes = std::fs::read(path).map_err(|e| format!("couldn't open it ({e})"))?;
        read_lnk(&bytes, path.parent().unwrap_or(Path::new(".")))
    }
}

#[cfg(windows)]
mod win {
    use super::Shortcut;
    use std::path::{Path, PathBuf};
    use windows::core::{Interface, HSTRING};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    fn text(buf: &[u16]) -> String {
        let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..n])
    }

    /// Tests only: a real shortcut written with Windows' own interface (so the reader is tested on
    /// what Windows itself makes).
    #[cfg(test)]
    pub fn write(path: &Path, target: &str, args: &str, dir: &str) {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
            link.SetPath(&HSTRING::from(target)).unwrap();
            link.SetArguments(&HSTRING::from(args)).unwrap();
            link.SetWorkingDirectory(&HSTRING::from(dir)).unwrap();
            let file: IPersistFile = link.cast().unwrap();
            file.Save(&HSTRING::from(path.as_os_str()), true).unwrap();
        }
    }

    pub fn read(path: &Path) -> Result<Shortcut, String> {
        // SAFETY: COM calls on this thread with checked results; the shortcut is only loaded.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| {
                    format!("Windows' shortcut reader isn't available ({})", e.message())
                })?;
            let file: IPersistFile = link.cast().map_err(|e| e.message().to_string())?;
            file.Load(&HSTRING::from(path.as_os_str()), STGM_READ)
                .map_err(|e| format!("Windows couldn't open it ({})", e.message().trim()))?;
            let mut target = [0u16; 1024];
            link.GetPath(&mut target, std::ptr::null_mut(), 0)
                .map_err(|e| {
                    format!("Windows couldn't read its target ({})", e.message().trim())
                })?;
            let mut args = [0u16; 4096];
            let _ = link.GetArguments(&mut args);
            let mut dir = [0u16; 1024];
            let _ = link.GetWorkingDirectory(&mut dir);
            let t = text(&target);
            if t.is_empty() {
                return Err(
                    "its target isn't a file on this PC (e.g. a shell item or a store app)".into(),
                );
            }
            let d = text(&dir);
            Ok(Shortcut {
                target: Some(PathBuf::from(t)),
                arguments: text(&args),
                working_dir: (!d.is_empty()).then(|| PathBuf::from(d)),
            })
        }
    }
}

// ---------- read-only settings checks ----------

/// Dolphin's user folder, found as Dolphin's own UICommon.cpp does (portable.txt next to the
/// program; else Documents\Dolphin Emulator if it exists; else AppData\Roaming\Dolphin Emulator).
/// The registry overrides (LocalUserConfig, UserConfigPath) are not read here.
pub fn dolphin_user_dir(
    exe: &Path,
    documents: Option<&Path>,
    appdata: Option<&Path>,
) -> Option<PathBuf> {
    let dir = exe.parent()?;
    if dir.join("portable.txt").exists() {
        return Some(dir.join("User"));
    }
    if let Some(d) = documents
        .map(|d| d.join("Dolphin Emulator"))
        .filter(|d| d.exists())
    {
        return Some(d);
    }
    appdata.map(|a| a.join("Dolphin Emulator"))
}

fn ini_value(text: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_section = t
                .trim_matches(|c| c == '[' || c == ']')
                .eq_ignore_ascii_case(section);
        } else if in_section {
            if let Some((k, v)) = t.split_once('=') {
                if k.trim().eq_ignore_ascii_case(key) {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

/// Dolphin's settings vs B-19 (read only). Each line: what doesn't match and where to change it.
pub fn check_dolphin(ini: &str) -> Vec<String> {
    let mut out = Vec::new();
    // Defaults from MainSettings.cpp: ConfirmStop true, Fullscreen false.
    let confirm = ini_value(ini, "Interface", "ConfirmStop")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);
    if confirm {
        out.push("Confirm on Stop is on: Dolphin → Options → Configuration → Interface → turn off \"Confirm on Stop\" (AIONE also adds -C Dolphin.Interface.ConfirmStop=False at launch).".into());
    }
    // B-20 backup (rev. ag): MAIN_INPUT_BACKGROUND_INPUT = {Main, "Input", "BackgroundInput"},
    // default false (MainSettings.cpp); its checkbox is "Background Input" in Controllers.
    let bg = ini_value(ini, "Input", "BackgroundInput")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if bg {
        out.push("Background Input is on: Dolphin reacts to your controller while AIONE is open. Dolphin → Controllers → turn off \"Background Input\".".into());
    }
    let full = ini_value(ini, "Display", "Fullscreen")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !full {
        out.push("Fullscreen is off in Dolphin's settings: Graphics → General → \"Start in Fullscreen\" (AIONE also adds -C Dolphin.Display.Fullscreen=True at launch).".into());
    }
    out
}

// ---------- the check report ----------

/// One game's line in the check report.
pub fn report_line(
    game: &str,
    sc: Result<&Shortcut, &str>,
    profiles: &[Profile],
    settings: &dyn Fn(&Profile, &Path) -> Vec<String>,
) -> String {
    let sc = match sc {
        Ok(sc) => sc,
        Err(why) => return format!("## {game}\n- {why}\n"),
    };
    let Some(target) = sc.target.as_deref() else {
        return format!("## {game}\n- Its game.lnk has no target. What you can do: right-click game.lnk → Properties, check that \"Target\" points to the program (e.g. the emulator's .exe), and recreate the shortcut if it doesn't.\n");
    };
    let Some(p) = recognize(profiles, target) else {
        return format!(
            "## {game}\n- Program: {} (not an emulator AIONE knows).\n",
            target.display()
        );
    };
    let (add, skip) = missing(p, &sc.arguments);
    let mut s = format!(
        "## {game}\n- Emulator: **{}** ({})\n",
        p.name,
        target.display()
    );
    if add.is_empty() {
        s.push_str("- Added at launch: nothing\n");
    } else {
        for o in &add {
            s.push_str(&format!(
                "- Added at launch: `{}` ({})\n",
                o.add.join(" "),
                o.why
            ));
        }
    }
    for (o, why) in &skip {
        s.push_str(&format!("- Not added: `{}`: {why}\n", o.add.join(" ")));
    }
    for line in settings(p, target) {
        s.push_str(&format!("- **Settings to fix:** {line}\n"));
    }
    let needs_missing = skip.iter().any(|(_, why)| why.starts_with("needs"));
    for g in &p.guide {
        // QA-5: the "opens the game directly" line only when the shortcut lacks what it needs.
        if g.starts_with("Opens the game directly") && !needs_missing && !p.options.is_empty() {
            continue;
        }
        s.push_str(&format!("- Guide: {g}\n"));
    }
    s
}

/// Writes the one-time check report for the library: each game, its emulator (or not), what AIONE
/// adds at launch, and the settings to fix (read only). Returns the file's path.
pub fn write_report(data: &Path) -> Result<PathBuf, String> {
    let profiles = profiles();
    let documents = std::env::var_os("USERPROFILE").map(|u| PathBuf::from(u).join("Documents"));
    let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
    let settings = |p: &Profile, exe: &Path| -> Vec<String> {
        if p.id != "dolphin" {
            return Vec::new();
        }
        let Some(user) = dolphin_user_dir(exe, documents.as_deref(), appdata.as_deref()) else {
            return vec!["Dolphin's settings folder couldn't be found; nothing checked.".into()];
        };
        let ini = user.join("Config").join("Dolphin.ini");
        match std::fs::read_to_string(&ini) {
            Ok(text) => check_dolphin(&text),
            Err(_) => vec![format!(
                "Dolphin's settings file wasn't found at {}; nothing checked.",
                ini.display()
            )],
        }
    };
    let mut games: Vec<(String, Result<Shortcut, String>)> = Vec::new();
    let dir = crate::games::games_dir(data);
    let mut ids: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|e| format!("couldn't read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    ids.sort_by_key(|a| a.to_lowercase());
    for id in ids {
        let Ok(f) = crate::games::launch_fields(data, &id) else {
            continue;
        };
        let sc = match f.shortcut.as_ref() {
            None => Err("No game.lnk: AIONE starts this game another way (its Steam id): not an emulator.".to_string()),
            Some(p) if !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk")) => {
                Err("Starts through a .url (e.g. Steam): not an emulator.".to_string())
            }
            Some(p) => read_shortcut(p).map_err(|e| {
                format!("Its game.lnk couldn't be read: {e}. What you can do: open game.lnk's Properties to check that it starts the game; recreating it in Explorer (right-click the emulator's .exe → Create shortcut, then add the arguments) usually fixes it.")
            }),
        };
        games.push((f.name, sc));
    }
    let mut out = format!(
        "# AIONE 4 — emulator check report\n\nWritten {} by AIONE {}. Read only: AIONE changed no shortcut and no emulator file. Each game: its emulator, what AIONE adds when it starts the game, and any settings to fix (with where).\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M"),
        env!("CARGO_PKG_VERSION")
    );
    for (name, sc) in &games {
        out.push_str(&report_line(
            name,
            sc.as_ref().map_err(String::as_str),
            &profiles,
            &settings,
        ));
        out.push('\n');
    }
    let path = data.join("emulator-check.md");
    std::fs::write(&path, out).map_err(|e| format!("couldn't write {}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal Unicode .lnk: LinkInfo with a local base path, then working dir and arguments.
    fn lnk(target: &str, workdir: &str, args: &str) -> Vec<u8> {
        let mut b = vec![0u8; 0x4C];
        b[0..4].copy_from_slice(&0x4Cu32.to_le_bytes());
        let flags: u32 = 0x2 | 0x10 | 0x20 | 0x80;
        b[20..24].copy_from_slice(&flags.to_le_bytes());
        // LinkInfo: header size 0x1C, flags 1, base path at 0x1C, empty suffix after it.
        let base = format!("{target}\0");
        let suffix_off = 0x1C + base.len();
        let size = suffix_off + 1;
        let mut li = Vec::new();
        for v in [size as u32, 0x1C, 1, 0, 0x1C, 0, suffix_off as u32] {
            li.extend_from_slice(&v.to_le_bytes());
        }
        li.extend_from_slice(base.as_bytes());
        li.push(0);
        b.extend_from_slice(&li);
        for s in [workdir, args] {
            let w: Vec<u16> = s.encode_utf16().collect();
            b.extend_from_slice(&(w.len() as u16).to_le_bytes());
            for c in w {
                b.extend_from_slice(&c.to_le_bytes());
            }
        }
        b
    }

    #[test]
    fn reads_a_shortcut_and_adds_only_what_is_missing() {
        let bytes = lnk(
            "C:\\Emu\\Dolphin.exe",
            "C:\\Emu",
            "-e \"C:\\Games\\An Invented Game.rvz\" -b",
        );
        let sc = read_lnk(&bytes, Path::new("C:\\Lib")).unwrap();
        assert_eq!(sc.target, Some(PathBuf::from("C:\\Emu\\Dolphin.exe")));
        assert_eq!(sc.arguments, "-e \"C:\\Games\\An Invented Game.rvz\" -b");
        assert_eq!(sc.working_dir, Some(PathBuf::from("C:\\Emu")));
        let ps = profiles();
        assert!(ps.len() >= 14);
        let p = recognize(&ps, Path::new("C:\\Emu\\DOLPHIN.EXE")).unwrap();
        assert_eq!(p.id, "dolphin");
        let (add, skip) = missing(p, &sc.arguments);
        // -b is already there: never twice; fullscreen and no-confirm are added.
        assert_eq!(
            joined(&add),
            "-C Dolphin.Display.Fullscreen=True -C Dolphin.Interface.ConfirmStop=False"
        );
        assert_eq!(skip.len(), 1);
        assert_eq!(skip[0].1, "already in the shortcut");
        // Without -e, -b would do nothing: it is not added.
        let (add2, skip2) = missing(p, "");
        assert!(!joined(&add2).contains("-b"));
        assert!(skip2
            .iter()
            .any(|(o, why)| o.add == ["-b"] && why.contains("needs -e")));
        // An existing -C for the same key counts as present.
        let (add3, _) = missing(p, "-e x.iso -C dolphin.display.fullscreen=False");
        assert!(!joined(&add3).contains("Fullscreen"));
        // Unknown programs are not emulators.
        assert!(recognize(&ps, Path::new("C:\\Games\\Balatro.exe")).is_none());
    }

    #[test]
    fn writes_the_check_report() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let g = crate::games::games_dir(data);
        for (id, json) in [
            (
                "a",
                r#"{"Name":"An Invented GameCube Game","Shortcut":"game.lnk"}"#,
            ),
            (
                "b",
                r#"{"Name":"An Invented PC Game","Shortcut":"game.url"}"#,
            ),
        ] {
            std::fs::create_dir_all(g.join(id)).unwrap();
            std::fs::write(g.join(id).join("game.json"), json).unwrap();
        }
        // Windows: a real shortcut made by Windows itself (its reader refuses a hand-built one);
        // elsewhere the byte-built one for the fallback reader.
        #[cfg(windows)]
        win::write(
            &g.join("a").join("game.lnk"),
            "C:\\Emu\\Dolphin.exe",
            "-e \"C:\\G\\x.rvz\"",
            "C:\\Emu",
        );
        #[cfg(not(windows))]
        std::fs::write(
            g.join("a").join("game.lnk"),
            lnk("C:\\Emu\\Dolphin.exe", "C:\\Emu", "-e \"C:\\G\\x.rvz\""),
        )
        .unwrap();
        std::fs::write(
            g.join("b").join("game.url"),
            b"[InternetShortcut]\nURL=steam://rungameid/1\n",
        )
        .unwrap();
        let path = write_report(data).unwrap();
        let text = std::fs::read_to_string(path).unwrap();
        assert!(
            text.contains("## An Invented GameCube Game\n- Emulator: **Dolphin**"),
            "{text}"
        );
        assert!(text.contains("- Added at launch: `-b` (exit when the game stops)"));
        assert!(
            text.contains(
                "## An Invented PC Game\n- Starts through a .url (e.g. Steam): not an emulator."
            ),
            "{text}"
        );
    }

    #[test]
    fn splits_like_windows() {
        assert_eq!(
            split_args(r#"-e "C:\A B\x.iso" -b"#),
            ["-e", "C:\\A B\\x.iso", "-b"]
        );
        assert_eq!(split_args("  "), Vec::<String>::new());
    }

    #[test]
    fn dolphin_settings_are_read_not_changed() {
        assert_eq!(
            check_dolphin("[Interface]\nConfirmStop = False\n[Display]\nFullscreen = True\n").len(),
            0
        );
        let issues = check_dolphin("[Interface]\nConfirmStop = True\n");
        assert_eq!(issues.len(), 2); // confirm on (set), fullscreen off (default)
        assert!(issues[0].contains("Confirm on Stop is on"));
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Dolphin.exe");
        std::fs::write(dir.path().join("portable.txt"), b"").unwrap();
        assert_eq!(
            dolphin_user_dir(&exe, None, None),
            Some(dir.path().join("User"))
        );
    }
}
