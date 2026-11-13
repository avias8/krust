//! User settings, stored as `settings.json` in the app config directory.

use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

/// wry passes these to WebView2 by default. Setting our own browser arguments
/// replaces the defaults, so they have to be repeated.
pub const DEFAULT_BROWSER_ARGS: &str =
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";

/// Chromium switches that lift the display-refresh frame cap.
const UNCAP_FPS_ARGS: &str = "--disable-frame-rate-limit --disable-gpu-vsync";

/// Missing keys fall back to their defaults, so old files keep working.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub uncap_fps: bool,
    pub fullscreen: bool,
    pub width: f64,
    pub height: f64,
    /// Extra Chromium switches appended to the WebView2 command line, for
    /// experimenting with flags without rebuilding.
    pub extra_browser_args: String,
    /// Serve files from the `swap` folder in place of Krunker's assets.
    pub resource_swapper: bool,
    /// Look for a new version shortly after launch and offer to install it.
    pub check_for_updates: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // Capped by default: uncapped frames on modern WebView2 reach the
            // screen unevenly (no tearing through the Windows compositor), which
            // shows as stutter, worst while aiming in fullscreen.
            uncap_fps: false,
            fullscreen: false,
            width: 1280.0,
            height: 720.0,
            extra_browser_args: String::new(),
            resource_swapper: true,
            check_for_updates: true,
        }
    }
}

impl Settings {
    /// True when switching from `self` to `other` only takes effect after a
    /// restart. Browser arguments are fixed when WebView2 starts, and the
    /// swap folder is scanned once at launch.
    pub fn needs_restart_for(&self, other: &Settings) -> bool {
        self.uncap_fps != other.uncap_fps
            || self.resource_swapper != other.resource_swapper
            || self.extra_browser_args.trim() != other.extra_browser_args.trim()
    }

    pub fn browser_args(&self) -> String {
        let mut args = DEFAULT_BROWSER_ARGS.to_string();
        if self.uncap_fps {
            args.push(' ');
            args.push_str(UNCAP_FPS_ARGS);
        }
        let extra = self.extra_browser_args.trim();
        if !extra.is_empty() {
            args.push(' ');
            args.push_str(extra);
        }
        args
    }
}

/// Reads settings, writing a default file on first run. A file that fails to
/// parse is left untouched and defaults are used for this session.
pub fn load(config_dir: &Path) -> Settings {
    let path = config_dir.join("settings.json");
    match fs::read_to_string(&path) {
        // Windows editors and PowerShell often prepend a byte-order mark.
        Ok(text) => {
            serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap_or_else(|err| {
                log::warn!("settings.json is invalid ({err}); using defaults");
                Settings::default()
            })
        }
        Err(_) => {
            let settings = Settings::default();
            let _ = save(config_dir, &settings);
            settings
        }
    }
}

/// Writes to a temporary file first, so a crash mid-write can never leave a
/// half-written settings.json behind.
pub fn save(config_dir: &Path, settings: &Settings) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(settings)?;
    let tmp = config_dir.join("settings.json.tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, config_dir.join("settings.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncapped_args_keep_the_wry_defaults() {
        let args = Settings {
            uncap_fps: true,
            ..Settings::default()
        }
        .browser_args();
        assert!(args.starts_with(DEFAULT_BROWSER_ARGS));
        assert!(args.contains("--disable-frame-rate-limit"));
    }

    #[test]
    fn capped_by_default() {
        assert_eq!(Settings::default().browser_args(), DEFAULT_BROWSER_ARGS);
    }

    #[test]
    fn extra_args_are_appended_last() {
        let settings = Settings {
            extra_browser_args: "  --foo=1 ".into(),
            ..Settings::default()
        };
        assert!(settings.browser_args().ends_with(" --foo=1"));
    }

    #[test]
    fn partial_settings_fall_back_to_defaults() {
        let settings: Settings = serde_json::from_str(r#"{ "fullscreen": true }"#).unwrap();
        assert!(settings.fullscreen);
        assert_eq!(settings.width, Settings::default().width);
    }

    #[test]
    fn only_launch_time_settings_need_a_restart() {
        let base = Settings::default();
        let fullscreen = Settings {
            fullscreen: true,
            ..base.clone()
        };
        let uncapped = Settings {
            uncap_fps: true,
            ..base.clone()
        };
        assert!(!base.needs_restart_for(&fullscreen));
        assert!(base.needs_restart_for(&uncapped));
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = std::env::temp_dir().join(format!("krust-settings-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let settings = Settings {
            fullscreen: true,
            extra_browser_args: "--x".into(),
            ..Settings::default()
        };
        save(&dir, &settings).unwrap();
        let loaded = load(&dir);
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(loaded, settings);
    }
}
