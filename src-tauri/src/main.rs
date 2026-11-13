// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings;
#[cfg(windows)]
mod shortcuts;
mod updates;

use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use settings::Settings;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

const GAME_URL: &str = "https://krunker.io/";
const MAIN_TITLE: &str = "Krust  \u{2014}  F1 settings";

/// Fixed for the life of the process.
struct AppState {
    config_dir: PathBuf,
    log_dir: PathBuf,
    /// Settings as they were at launch, to tell the user when a change
    /// needs a restart.
    launched: Settings,
    /// Every webview in the process shares one WebView2 environment, which
    /// fails to start a second window with different browser arguments.
    browser_args: String,
}

/// Keyboard shortcuts, handled natively before the game sees the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    OpenSettings,
    Reload,
    ToggleFullscreen,
}

/// Runs a shortcut action. Called from a spare thread: window calls made on
/// the WebView2 event thread can deadlock waiting for that same thread.
pub fn run_action(app: &AppHandle, action: Action) {
    match action {
        Action::OpenSettings => open_settings(app),
        Action::Reload => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.reload();
            }
        }
        Action::ToggleFullscreen => {
            if let Some(window) = app.get_webview_window("main") {
                let on = window.is_fullscreen().unwrap_or(false);
                let _ = window.set_fullscreen(!on);
            }
        }
    }
}

fn open_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    let state = app.state::<AppState>();
    let result =
        WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
            .title("Krust settings")
            .inner_size(460.0, 640.0)
            .min_inner_size(400.0, 480.0)
            .additional_browser_args(&state.browser_args)
            .build();
    if let Err(err) = result {
        log::error!("could not open settings: {err}");
    }
}

// ---- Commands for the settings page. Only the local settings window may
// ---- call these; see capabilities/settings.json.

#[derive(Serialize)]
struct SettingsView {
    settings: Settings,
    needs_restart: bool,
    version: String,
    updates_configured: bool,
}

fn view(app: &AppHandle, state: &AppState, settings: Settings) -> SettingsView {
    SettingsView {
        needs_restart: state.launched.needs_restart_for(&settings),
        settings,
        version: app.package_info().version.to_string(),
        updates_configured: updates::configured(app),
    }
}

#[tauri::command]
fn get_settings(app: AppHandle, state: State<AppState>) -> SettingsView {
    view(&app, &state, settings::load(&state.config_dir))
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<AppState>,
    settings: Settings,
) -> Result<SettingsView, String> {
    settings::save(&state.config_dir, &settings).map_err(|err| err.to_string())?;
    Ok(view(&app, &state, settings))
}

#[tauri::command]
async fn check_update(app: AppHandle) -> Result<updates::UpdateInfo, String> {
    updates::check(&app).await
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    updates::install(&app).await
}

/// The page picks a folder by name, never by path, so it cannot open
/// arbitrary locations.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Folder {
    Config,
    Logs,
}

#[tauri::command]
fn open_folder(state: State<AppState>, folder: Folder) -> Result<(), String> {
    let path = match folder {
        Folder::Config => state.config_dir.clone(),
        Folder::Logs => state.log_dir.clone(),
    };
    fs::create_dir_all(&path).map_err(|err| err.to_string())?;
    #[cfg(windows)]
    std::process::Command::new("explorer")
        .arg(&path)
        .spawn()
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
fn restart(app: AppHandle) {
    app.restart();
}

fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let mut targets = vec![Target::new(TargetKind::LogDir {
        file_name: Some("krust".into()),
    })];
    if cfg!(debug_assertions) {
        targets.push(Target::new(TargetKind::Stderr));
    }
    tauri_plugin_log::Builder::new()
        .clear_targets()
        .targets(targets)
        .level(log::LevelFilter::Info)
        // Keep the log small: one 1 MB file plus one rotated copy.
        .max_file_size(1_000_000)
        .rotation_strategy(RotationStrategy::KeepOne)
        .build()
}

fn main() {
    tauri::Builder::default()
        .plugin(log_plugin())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updates::Pending::default())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            open_folder,
            restart,
            check_update,
            install_update
        ])
        .on_window_event(|window, event| {
            // Closing the game closes Krust, even with settings still open.
            if window.label() == "main" && matches!(event, WindowEvent::Destroyed) {
                window.app_handle().exit(0);
            }
        })
        .setup(|app| {
            log::info!("Krust {} starting", app.package_info().version);
            let config_dir = app.path().app_config_dir()?;
            let log_dir = app.path().app_log_dir()?;
            fs::create_dir_all(&config_dir)?;

            let settings = settings::load(&config_dir);
            let browser_args = settings.browser_args();

            // The game loads exactly as it does in a browser: Krust injects
            // no scripts and changes no game files. It is remote content, and
            // no capability grants krunker.io access to Tauri's IPC.
            let window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::External(GAME_URL.parse()?))
                    .title(MAIN_TITLE)
                    .inner_size(settings.width, settings.height)
                    .fullscreen(settings.fullscreen)
                    .additional_browser_args(&browser_args);

            let check_for_updates = settings.check_for_updates;
            app.manage(AppState {
                config_dir,
                log_dir,
                launched: settings,
                browser_args,
            });

            let window = window.build()?;

            #[cfg(windows)]
            {
                let handle = app.handle().clone();
                window.with_webview(move |webview| shortcuts::attach(&webview, handle))?;
            }
            #[cfg(not(windows))]
            let _ = window;

            if check_for_updates {
                updates::spawn_startup_check(app.handle().clone());
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Krust");
}
