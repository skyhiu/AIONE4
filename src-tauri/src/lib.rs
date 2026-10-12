//! AIONE 4 core (Tauri 2). T0: the window, single instance, restart, the foreground at start,
//! the data folder and logs, the one-time copy of the WPF data, and controller input.
//! See docs/ARCHITECTURE.md.

pub mod foreground;
pub mod input;
pub mod library;
pub mod lifecycle;
pub mod logging;
pub mod migration;
pub mod paths;
pub mod version;

use logging::Level;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};

pub const EVENT_PAD: &str = "aione://pad";
pub const EVENT_COPY_PROGRESS: &str = "aione://copy-progress";
pub const EVENT_COPY_DONE: &str = "aione://copy-done";
const MAIN_WINDOW: &str = "main";
/// The black wait before the window is shown anyway if the UI never reports ready (A9: 4 s).
const SHOW_FALLBACK: Duration = Duration::from_secs(4);
/// How long the start-up foreground guard runs (T0 has no Boot Up yet; T1 ties it to Boot Up).
#[cfg(windows)]
const GUARD_SECONDS: u64 = 8;

struct AppState {
    data_dir: PathBuf,
    wpf_dir: PathBuf,
    started: Instant,
    shown: AtomicBool,
    copy_running: AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: &'static str,
    data_dir: String,
    logs_dir: String,
    platform: &'static str,
    pid: u32,
}

#[tauri::command]
fn app_info(state: tauri::State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: version::VERSION,
        data_dir: state.data_dir.display().to_string(),
        logs_dir: paths::logs_dir(&state.data_dir).display().to_string(),
        platform: std::env::consts::OS,
        pid: std::process::id(),
    }
}

#[tauri::command]
fn ui_ready(app: AppHandle, state: tauri::State<'_, AppState>) {
    logging::lifecycle(
        Level::Info,
        &format!("UI ready after {} ms", state.started.elapsed().as_millis()),
    );
    show_and_focus(&app, "start");
}

#[tauri::command]
fn log_ui(level: String, message: String) {
    let level = match level.as_str() {
        "warn" => Level::Warn,
        "error" => Level::Error,
        _ => Level::Info,
    };
    if let Some(l) = logging::get() {
        l.write(level, "ui", &message);
    }
}

#[tauri::command]
async fn migration_status(app: AppHandle) -> Result<migration::MigrationStatus, String> {
    let (data, wpf) = {
        let s = app.state::<AppState>();
        (s.data_dir.clone(), s.wpf_dir.clone())
    };
    tauri::async_runtime::spawn_blocking(move || migration::status(&data, &wpf))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn migration_decide(app: AppHandle, accept: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let (data, wpf) = (state.data_dir.clone(), state.wpf_dir.clone());
    if !accept {
        logging::info(
            "copy",
            "the user chose not to copy the WPF data (not asked again)",
        );
        return migration::record_decision(&data, &wpf, "declined", None)
            .map_err(|e| e.to_string());
    }
    if state.copy_running.swap(true, Ordering::SeqCst) {
        logging::warn("copy", "copy already running; second request ignored");
        return Ok(());
    }
    let handle = app.clone();
    std::thread::Builder::new()
        .name("aione-copy".into())
        .spawn(move || {
            let result = migration::copy(&data, &wpf, |p| {
                let _ = handle.emit(EVENT_COPY_PROGRESS, p);
            });
            if let Err(e) =
                migration::record_decision(&data, &wpf, &result.outcome, Some(result.clone()))
            {
                logging::error("copy", &format!("could not write migration.json: {e}"));
            }
            let _ = handle.emit(EVENT_COPY_DONE, &result);
            handle
                .state::<AppState>()
                .copy_running
                .store(false, Ordering::SeqCst);
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// D-002: the connected controller slots right now, for the UI at start.
#[tauri::command]
fn pad_slots() -> input::PadSlots {
    input::pad_slots()
}

#[tauri::command]
fn restart_app(app: AppHandle) -> Result<(), String> {
    match lifecycle::spawn_restart() {
        Ok(child) => {
            logging::lifecycle(
                Level::Info,
                &format!(
                    "restart: started pid {child}; it waits for this instance to exit; exiting now"
                ),
            );
            app.exit(0);
            Ok(())
        }
        Err(e) => {
            logging::lifecycle(Level::Error, &format!("restart failed: {e}"));
            Err(e.to_string())
        }
    }
}

/// Shows the main window (first time) and takes the foreground with focus (L-20). Runs on the
/// main thread.
fn show_and_focus(app: &AppHandle, reason: &'static str) {
    let handle = app.clone();
    let run = move || {
        let Some(window) = handle.get_webview_window(MAIN_WINDOW) else {
            logging::lifecycle(Level::Error, "main window missing");
            return;
        };
        let state = handle.state::<AppState>();
        let first = !state.shown.swap(true, Ordering::SeqCst);
        if reason == "start" && !first {
            return; // already shown (UI ready after the fallback, or a second call)
        }
        let _ = window.unminimize();
        if let Err(e) = window.show() {
            logging::lifecycle(Level::Error, &format!("show failed: {e}"));
        }
        #[cfg(windows)]
        let report = match window.hwnd() {
            Ok(h) => foreground::bring_to_front(h.0 as isize),
            Err(e) => {
                logging::lifecycle(Level::Error, &format!("no window handle: {e}"));
                return;
            }
        };
        #[cfg(not(windows))]
        let report = foreground::bring_to_front(0);
        // Keyboard focus into the page (the window may already be foreground).
        let _ = window.set_focus();
        logging::lifecycle(
            if report.ok { Level::Info } else { Level::Warn },
            &format!(
                "foreground ({reason}, {} ms after start): {}",
                state.started.elapsed().as_millis(),
                report.summary()
            ),
        );
        logging::lifecycle(
            Level::Info,
            &format!(
                "keyboard focus after the foreground step: {}",
                foreground::focus_report()
            ),
        );
        if first {
            // WebView2 moves focus into the page asynchronously; check once more shortly after.
            let later = handle.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(500));
                let _ = later.run_on_main_thread(|| {
                    logging::lifecycle(
                        Level::Info,
                        &format!(
                            "keyboard focus 500 ms later: {}",
                            foreground::focus_report()
                        ),
                    );
                });
            });
        }
        #[cfg(windows)]
        if first {
            if let Ok(h) = window.hwnd() {
                foreground::start_guard(h.0 as isize, GUARD_SECONDS);
                let later = handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(GUARD_SECONDS));
                    let _ = later.run_on_main_thread(|| foreground::stop_guard("time is up"));
                });
            }
        }
    };
    if let Err(e) = app.run_on_main_thread(run) {
        logging::lifecycle(
            Level::Error,
            &format!("could not reach the main thread: {e}"),
        );
    }
}

fn install_panic_log() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        logging::lifecycle(Level::Error, &format!("PANIC: {info}"));
        default(info);
    }));
}

#[cfg(windows)]
fn message_box(text: &str) {
    use windows::core::HSTRING;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};
    // SAFETY: plain modal message box without an owner.
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(text),
            &HSTRING::from("AIONE 4"),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

#[cfg(not(windows))]
fn message_box(text: &str) {
    eprintln!("{text}");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let started = Instant::now();
    let args = lifecycle::parse_args(std::env::args().skip(1));

    // 1. Data folder and logs first, so everything after is logged.
    let data_dir = paths::data_dir();
    let logs_dir = match paths::ensure_data_dir(&data_dir) {
        Ok(l) => l,
        Err(e) => {
            message_box(&format!(
                "AIONE 4 could not create its data folder:\n{}\n\n{e}",
                data_dir.display()
            ));
            return;
        }
    };
    logging::init(&logs_dir);
    install_panic_log();
    lifecycle::log_start(&args);
    logging::info("data", &format!("data folder: {}", data_dir.display()));

    // 2. One-shot command line actions.
    if let Some(enable) = args.autostart {
        let msg = lifecycle::set_autostart(enable).unwrap_or_else(|e| e);
        logging::lifecycle(Level::Info, &format!("--autostart: {msg}"));
        message_box(&msg);
        return;
    }

    // 3. Restart: wait for the previous instance before claiming the single instance.
    if let Some(pid) = args.wait_for_pid {
        logging::lifecycle(
            Level::Info,
            &format!("restart: waiting for pid {pid} to exit"),
        );
        let msg = lifecycle::wait_for_previous(pid);
        logging::lifecycle(Level::Info, &format!("restart: {msg}"));
    }

    if lifecycle::another_instance_running() {
        logging::lifecycle(
            Level::Info,
            "second instance: AIONE 4 is already running; handing over to it (it comes to the front) and exiting",
        );
    }

    let state = AppState {
        data_dir,
        wpf_dir: paths::wpf_data_dir(),
        started,
        shown: AtomicBool::new(false),
        copy_running: AtomicBool::new(false),
    };

    let app = tauri::Builder::default()
        // Must be the first plugin: a second start exits here and focuses this instance.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            logging::lifecycle(
                Level::Info,
                &format!(
                    "second start detected (args {:?}); focusing this instance",
                    argv.iter().skip(1).collect::<Vec<_>>()
                ),
            );
            show_and_focus(app, "second start");
        }))
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            app_info,
            ui_ready,
            log_ui,
            migration_status,
            migration_decide,
            pad_slots,
            restart_app
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            input::start(move |ev| {
                let _ = handle.emit(EVENT_PAD, ev);
            });
            // Bounded black wait: show the window even if the UI never says it is ready.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(SHOW_FALLBACK);
                if !handle.state::<AppState>().shown.load(Ordering::SeqCst) {
                    logging::lifecycle(
                        Level::Warn,
                        "UI not ready after 4 s; showing the window anyway",
                    );
                    show_and_focus(&handle, "start");
                }
            });
            logging::lifecycle(Level::Info, "core ready; waiting for the UI's first frame");
            Ok(())
        })
        .on_window_event(|_window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                logging::lifecycle(Level::Info, "window close requested");
            }
        })
        .build(tauri::generate_context!());

    let app = match app {
        Ok(a) => a,
        Err(e) => {
            logging::lifecycle(Level::Error, &format!("could not start the window: {e}"));
            message_box(&format!(
                "AIONE 4 could not start its window.\n\n{e}\n\nIs the Microsoft Edge WebView2 Runtime installed?"
            ));
            return;
        }
    };

    app.run(|_app, event| match event {
        RunEvent::ExitRequested { code, .. } => {
            logging::lifecycle(Level::Info, &format!("exit requested (code {code:?})"));
        }
        RunEvent::Exit => {
            logging::lifecycle(Level::Info, "exit");
        }
        _ => {}
    });
}
