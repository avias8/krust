# Krust

[![CI](https://github.com/avias8/krust/actions/workflows/ci.yml/badge.svg)](https://github.com/avias8/krust/actions/workflows/ci.yml)
[![CodeQL](https://github.com/avias8/krust/actions/workflows/codeql.yml/badge.svg)](https://github.com/avias8/krust/actions/workflows/codeql.yml)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/avias8/krust/badge)](https://scorecard.dev/viewer/?uri=github.com/avias8/krust)

A Krunker.io client written in Rust with [Tauri 2](https://tauri.app). It uses
the system WebView2 runtime instead of bundling Chromium, so the browser engine
stays current through Windows and Edge updates.

Unofficial. Not affiliated with Krunker or FRVR.

Krust is a plain game window. It loads krunker.io exactly as a browser
would and does not inject scripts, replace game files or change how the
game plays.

## Install

1. Download `Krust_x.y.z_x64-setup.exe` from the
   [latest release](https://github.com/avias8/krust/releases/latest).
2. Run it. It installs for your account only and needs no admin rights.
3. Start Krust from the Start menu or desktop, and press <kbd>F1</kbd> for
   settings.

Krust checks for updates when it starts and asks before installing one.
Every update is signature-checked; see [SECURITY.md](SECURITY.md) for how
to verify a download yourself.

The installer is not code-signed with a Windows certificate yet, so
SmartScreen may show "Windows protected your PC" the first time. Choose
**More info → Run anyway**.

## Features

- Opens krunker.io in a native window
- Optional uncapped frame rate (off by default; see Status)
- Settings window (<kbd>F1</kbd>) and keyboard shortcuts
- Signed automatic updates
- Log file for bug reports (Settings → Advanced → Logs)

Planned: ad and tracker blocking, Discord presence.

## Status

Pre-release. Tested on Windows 11 with WebView2 157 and a 240 Hz display:

- The installer is under 2 MB and installs per user, without admin rights.
  Uninstalling removes the app and its shortcuts and keeps your settings.
- Frame rate: capped (the default) follows the monitor, 240 FPS here, and is
  smooth. Uncapped reaches 400 to 1,200 FPS, but frames reach the screen
  unevenly, because modern WebView2 presents through the Windows compositor
  and cannot tear like the old Chrome 85 clients. The result is visible
  stutter, worst while aiming at large window sizes. Disabling
  DirectComposition restores the old path but cut performance to about
  30 FPS while scoped, so it is not used.
- Updates: an installed build found, verified and installed a newer
  version, and refused a copy with one byte altered.

## Keyboard shortcuts

| Key | Action |
| --- | ------ |
| <kbd>F1</kbd> | Open the settings window |
| <kbd>F5</kbd> | Reload the game |
| <kbd>F11</kbd> | Toggle fullscreen |

Shortcuts are handled natively, before the key reaches the game.

## Building from source

- Windows 10 or 11 with the WebView2 runtime (preinstalled on Windows 11)
- [Rust](https://rustup.rs) with the MSVC toolchain
- Node.js, for the Tauri CLI

## Develop

```bash
npm install
npm run dev
```

If the project lives under a long path, the MSVC linker can fail with
`LNK1104`. Point Cargo at a short build directory first, for example
`$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\krust-target"`.

Build an installer with `npm run build`. It lands in
`<target dir>\release\bundle\nsis\`. Run the unit tests with `cargo test`
inside `src-tauri`.

## Settings

Settings live in `%APPDATA%\dev.krust.client\settings.json`, which the
settings window (<kbd>F1</kbd>) edits for you. Changes to the frame rate and
browser arguments apply after a restart; the settings window says when one
is needed and offers a restart button.

| Key | Default | Meaning |
| --- | ------- | ------- |
| `uncap_fps` | `false` | Lift the display-refresh frame cap (can stutter) |
| `fullscreen` | `false` | Start in fullscreen |
| `width` | `1280` | Window width |
| `height` | `720` | Window height |
| `extra_browser_args` | `""` | Extra Chromium switches for WebView2 |
| `check_for_updates` | `true` | Look for updates at launch |

Logs are in `%LOCALAPPDATA%\dev.krust.client\logs`.

## Security

The game is remote content and gets no access to Tauri's IPC, so the page
cannot call into the Rust side. The settings commands are declared in
`build.rs`, which makes Tauri deny them by default, and
`capabilities/settings.json` grants them only to the bundled settings
window. See [SECURITY.md](SECURITY.md) for how releases are signed and
how to report a vulnerability.

## Releasing

See [RELEASING.md](RELEASING.md).

## Credits

The project layout follows
[tauri-krunker](https://github.com/vaqqq/tauri-krunker) (MIT), and some
ideas came from [Crankshaft](https://github.com/KraXen72/crankshaft)
(GPL-3.0). No code from either is included.

## License

MIT
