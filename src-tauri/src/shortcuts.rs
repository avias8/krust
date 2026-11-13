//! Keyboard shortcuts for the game window, using WebView2's
//! AcceleratorKeyPressed event. It fires before the page sees the key, so a
//! handled shortcut never reaches Krunker.

use tauri::{webview::PlatformWebview, AppHandle};
use webview2_com::{AcceleratorKeyPressedEventHandler, Microsoft::Web::WebView2::Win32::*};

use crate::Action;

const VK_F1: u32 = 0x70;
const VK_F5: u32 = 0x74;
const VK_F11: u32 = 0x7A;

pub fn action_for_key(virtual_key: u32) -> Option<Action> {
    match virtual_key {
        VK_F1 => Some(Action::OpenSettings),
        VK_F5 => Some(Action::Reload),
        VK_F11 => Some(Action::ToggleFullscreen),
        _ => None,
    }
}

pub fn attach(webview: &PlatformWebview, app: AppHandle) {
    let handler = AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
        let Some(args) = args else { return Ok(()) };
        let mut kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
        let mut key = 0u32;
        unsafe {
            args.KeyEventKind(&mut kind)?;
            args.VirtualKey(&mut key)?;
        }
        if kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN {
            return Ok(());
        }
        let Some(action) = action_for_key(key) else {
            return Ok(());
        };

        let mut status = COREWEBVIEW2_PHYSICAL_KEY_STATUS::default();
        unsafe {
            // Swallow the key, including WebView2's own F5 reload, so the
            // action runs exactly once.
            args.SetHandled(true)?;
            args.PhysicalKeyStatus(&mut status)?;
        }
        // Holding a key down repeats it; act on the first press only.
        if status.WasKeyDown.as_bool() {
            return Ok(());
        }

        let app = app.clone();
        std::thread::spawn(move || crate::run_action(&app, action));
        Ok(())
    }));

    let mut token = 0i64;
    if let Err(err) = unsafe {
        webview
            .controller()
            .add_AcceleratorKeyPressed(&handler, &mut token)
    } {
        log::error!("keyboard shortcuts unavailable: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_function_keys() {
        assert_eq!(action_for_key(VK_F1), Some(Action::OpenSettings));
        assert_eq!(action_for_key(VK_F5), Some(Action::Reload));
        assert_eq!(action_for_key(VK_F11), Some(Action::ToggleFullscreen));
        assert_eq!(action_for_key(0x41), None);
    }
}
