// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const GAME_URL: &str = "https://krunker.io/";

/// wry passes these to WebView2 by default. Setting our own browser arguments
/// replaces the defaults, so they have to be repeated.
const DEFAULT_BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";

/// Chromium switches that lift the display-refresh frame cap.
const UNCAP_FPS_ARGS: &str = "--disable-frame-rate-limit --disable-gpu-vsync";

/// User settings, stored as `settings.json` in the app config directory.
/// Missing keys fall back to their defaults, so old files keep working.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    uncap_fps: bool,
    fullscreen: bool,
    width: f64,
    height: f64,
    /// Extra Chromium switches appended to the WebView2 command line, for
    /// experimenting with flags without rebuilding.
    extra_browser_args: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            uncap_fps: true,
            fullscreen: false,
            width: 1280.0,
            height: 720.0,
            extra_browser_args: String::new(),
        }
    }
}

/// Reads settings, writing a default file on first run. A file that fails to
/// parse is left untouched and defaults are used for this session.
fn load_settings(config_dir: &Path) -> Settings {
    let path = config_dir.join("settings.json");
    match fs::read_to_string(&path) {
        // Windows editors and PowerShell often prepend a byte-order mark.
        Ok(text) => {
            serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap_or_else(|err| {
                eprintln!("settings.json is invalid ({err}); using defaults");
                Settings::default()
            })
        }
        Err(_) => {
            let settings = Settings::default();
            if let Ok(text) = serde_json::to_string_pretty(&settings) {
                let _ = fs::write(&path, text);
            }
            settings
        }
    }
}

fn browser_args(settings: &Settings) -> String {
    let mut args = DEFAULT_BROWSER_ARGS.to_string();
    if settings.uncap_fps {
        args.push(' ');
        args.push_str(UNCAP_FPS_ARGS);
    }
    let extra = settings.extra_browser_args.trim();
    if !extra.is_empty() {
        args.push(' ');
        args.push_str(extra);
    }
    args
}

/// Returns `(file name, source)` for every `.js` file in `dir`, sorted by
/// name so the load order is predictable.
fn load_userscripts(dir: &Path) -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut scripts: Vec<(String, String)> = entries
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "js"))
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            fs::read_to_string(entry.path()).ok().map(|src| (name, src))
        })
        .collect();
    scripts.sort_by(|a, b| a.0.cmp(&b.0));
    scripts
}

/// Wraps a userscript so it only runs on Krunker pages and so an error in one
/// script cannot stop the others or the game.
fn wrap_userscript(name: &str, source: &str) -> String {
    let label = serde_json::to_string(name).unwrap_or_else(|_| "\"userscript\"".into());
    format!(
        "(function () {{\n\
         if (!/(^|\\.)krunker\\.io$/.test(location.hostname)) return;\n\
         try {{\n{source}\n}} catch (err) {{\n\
         console.error('[krust] userscript ' + {label} + ' failed:', err);\n\
         }}\n\
         }})();"
    )
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let scripts_dir = config_dir.join("scripts");
            fs::create_dir_all(&scripts_dir)?;

            let settings = load_settings(&config_dir);

            // The game is remote content. No capability grants krunker.io
            // access to Tauri's IPC, so the page cannot call into Rust.
            let mut window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::External(GAME_URL.parse()?))
                    .title("Krust")
                    .inner_size(settings.width, settings.height)
                    .fullscreen(settings.fullscreen)
                    .additional_browser_args(&browser_args(&settings));

            for (name, source) in load_userscripts(&scripts_dir) {
                window = window.initialization_script(&wrap_userscript(&name, &source));
            }

            window.build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Krust");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncapped_args_keep_the_wry_defaults() {
        let args = browser_args(&Settings::default());
        assert!(args.starts_with(DEFAULT_BROWSER_ARGS));
        assert!(args.contains("--disable-frame-rate-limit"));
    }

    #[test]
    fn capped_args_are_only_the_defaults() {
        let settings = Settings {
            uncap_fps: false,
            ..Settings::default()
        };
        assert_eq!(browser_args(&settings), DEFAULT_BROWSER_ARGS);
    }

    #[test]
    fn extra_args_are_appended_last() {
        let settings = Settings {
            extra_browser_args: "  --foo=1 ".into(),
            ..Settings::default()
        };
        assert!(browser_args(&settings).ends_with(" --foo=1"));
    }

    #[test]
    fn partial_settings_fall_back_to_defaults() {
        let settings: Settings = serde_json::from_str(r#"{ "fullscreen": true }"#).unwrap();
        assert!(settings.fullscreen);
        assert_eq!(settings.width, Settings::default().width);
    }

    #[test]
    fn userscript_wrapper_escapes_the_file_name() {
        let wrapped = wrap_userscript("it's \"mine\".js", "let a = 1;");
        assert!(wrapped.contains("let a = 1;"));
        assert!(wrapped.contains(r#""it's \"mine\".js""#));
    }
}
