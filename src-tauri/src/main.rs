// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings;
#[cfg(windows)]
mod shortcuts;
mod swapper;
mod userscripts;

use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use settings::Settings;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

const GAME_URL: &str = "https://krunker.io/";
const MAIN_TITLE: &str = "Krust  \u{2014}  F1 settings";

/// Fixed for the life of the process.
struct AppState {
    config_dir: PathBuf,
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
        eprintln!("[krust] could not open settings: {err}");
    }
}

// ---- Commands for the settings page. Only the local settings window may
// ---- call these; see capabilities/settings.json.

#[derive(Serialize)]
struct SettingsView {
    settings: Settings,
    needs_restart: bool,
}

fn view(state: &AppState, settings: Settings) -> SettingsView {
    SettingsView {
        needs_restart: state.launched.needs_restart_for(&settings),
        settings,
    }
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> SettingsView {
    view(&state, settings::load(&state.config_dir))
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: Settings) -> Result<SettingsView, String> {
    settings::save(&state.config_dir, &settings).map_err(|err| err.to_string())?;
    Ok(view(&state, settings))
}

/// The page picks a folder by name, never by path, so it cannot open
/// arbitrary locations.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Folder {
    Config,
    Swap,
    Scripts,
}

#[tauri::command]
fn open_folder(state: State<AppState>, folder: Folder) -> Result<(), String> {
    let path = match folder {
        Folder::Config => state.config_dir.clone(),
        Folder::Swap => state.config_dir.join("swap"),
        Folder::Scripts => state.config_dir.join("scripts"),
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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            open_folder,
            restart
        ])
        .on_window_event(|window, event| {
            // Closing the game closes Krust, even with settings still open.
            if window.label() == "main" && matches!(event, WindowEvent::Destroyed) {
                window.app_handle().exit(0);
            }
        })
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let scripts_dir = config_dir.join("scripts");
            let swap_dir = config_dir.join("swap");
            fs::create_dir_all(&scripts_dir)?;
            fs::create_dir_all(&swap_dir)?;

            let settings = settings::load(&config_dir);
            let browser_args = settings.browser_args();

            let swaps = if settings.resource_swapper && cfg!(windows) {
                swapper::SwapIndex::scan(&swap_dir)
            } else {
                swapper::SwapIndex::default()
            };
            if !swaps.is_empty() {
                eprintln!("[krust] swapping {} file(s)", swaps.len());
            }

            // With swaps, start blank and navigate once the swapper is ready.
            let start_url = if swaps.is_empty() {
                GAME_URL
            } else {
                "about:blank"
            };

            // The game is remote content. No capability grants krunker.io
            // access to Tauri's IPC, so the page cannot call into Rust.
            let mut window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::External(start_url.parse()?))
                    .title(MAIN_TITLE)
                    .inner_size(settings.width, settings.height)
                    .fullscreen(settings.fullscreen)
                    .additional_browser_args(&browser_args);

            for (name, source) in userscripts::load(&scripts_dir) {
                window = window.initialization_script(&userscripts::wrap(&name, &source));
            }

            app.manage(AppState {
                config_dir,
                launched: settings,
                browser_args,
            });

            let window = window.build()?;

            #[cfg(windows)]
            {
                let handle = app.handle().clone();
                window.with_webview(move |webview| {
                    shortcuts::attach(&webview, handle);
                    if !swaps.is_empty() {
                        swapper::attach(webview, swaps, GAME_URL);
                    }
                })?;
            }
            #[cfg(not(windows))]
            let _ = (window, swaps);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Krust");
}
