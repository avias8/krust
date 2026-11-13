//! Self-updates from GitHub Releases.
//!
//! The updater plugin downloads `latest.json` from the release endpoint in
//! tauri.conf.json, then refuses any installer whose minisign signature does
//! not match the public key compiled into this build. Only the holder of the
//! private key (a GitHub Actions secret) can produce an update users accept.

use std::{sync::Mutex, time::Duration};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::{Update, UpdaterExt};

/// How long after launch to check, so the check never slows game start-up.
const STARTUP_DELAY: Duration = Duration::from_secs(8);

/// The update found by the last check, waiting for the user to accept it.
#[derive(Default)]
pub struct Pending(pub Mutex<Option<Update>>);

#[derive(Debug, Serialize)]
pub struct UpdateInfo {
    pub current: String,
    /// The newer version, or `None` when already up to date.
    pub available: Option<String>,
    pub notes: Option<String>,
}

/// Development builds carry no public key, so they cannot verify updates and
/// do not look for them.
pub fn configured(app: &AppHandle) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|updater| updater.get("pubkey"))
        .and_then(|key| key.as_str())
        .is_some_and(|key| !key.trim().is_empty())
}

pub async fn check(app: &AppHandle) -> Result<UpdateInfo, String> {
    if !configured(app) {
        return Err(
            "This build was made without an update key, so it cannot update itself.".into(),
        );
    }
    let update = app
        .updater()
        .map_err(|err| err.to_string())?
        .check()
        .await
        .map_err(|err| err.to_string())?;

    let info = UpdateInfo {
        current: app.package_info().version.to_string(),
        available: update.as_ref().map(|u| u.version.clone()),
        notes: update.as_ref().and_then(|u| u.body.clone()),
    };
    match &info.available {
        Some(version) => log::info!("update available: {} -> {version}", info.current),
        None => log::info!("no update available (running {})", info.current),
    }
    *app.state::<Pending>().0.lock().unwrap() = update;
    Ok(info)
}

/// Downloads, verifies and installs the pending update. On Windows the
/// installer runs in passive mode (progress bar, no questions) and Krust
/// closes while it does; the installer starts the new version afterwards.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = app
        .state::<Pending>()
        .0
        .lock()
        .unwrap()
        .take()
        .ok_or("No update is waiting. Check for updates first.")?;
    log::info!("installing update {}", update.version);
    update
        .download_and_install(
            |_, _| {},
            || log::info!("update downloaded; checking signature"),
        )
        .await
        .map_err(|err| {
            log::error!("update failed: {err}");
            err.to_string()
        })?;
    app.restart();
}

/// Checks once, shortly after launch, and asks before installing anything.
pub fn spawn_startup_check(app: AppHandle) {
    if !configured(&app) {
        log::info!("update checks disabled: no update key in this build");
        return;
    }
    std::thread::spawn(move || {
        std::thread::sleep(STARTUP_DELAY);
        let info = match tauri::async_runtime::block_on(check(&app)) {
            Ok(info) => info,
            Err(err) => {
                // Offline, GitHub unreachable or no release yet: try again next launch.
                log::warn!("update check failed: {err}");
                return;
            }
        };
        let Some(version) = info.available else {
            return;
        };

        let accepted = app
            .dialog()
            .message(format!(
                "Krust {version} is available. You have {}.\n\n\
                 Krust will close, install the update and reopen.",
                info.current
            ))
            .title("Krust update")
            .kind(MessageDialogKind::Info)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Update now".into(),
                "Later".into(),
            ))
            .blocking_show();
        if !accepted {
            log::info!("user postponed update {version}");
            return;
        }
        if let Err(err) = tauri::async_runtime::block_on(install(&app)) {
            app.dialog()
                .message(format!("The update could not be installed.\n\n{err}"))
                .title("Krust update")
                .kind(MessageDialogKind::Error)
                .blocking_show();
        }
    });
}
