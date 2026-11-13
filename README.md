# Krust

[![CI](https://github.com/avias8/krust/actions/workflows/ci.yml/badge.svg)](https://github.com/avias8/krust/actions/workflows/ci.yml)
[![CodeQL](https://github.com/avias8/krust/actions/workflows/codeql.yml/badge.svg)](https://github.com/avias8/krust/actions/workflows/codeql.yml)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/avias8/krust/badge)](https://scorecard.dev/viewer/?uri=github.com/avias8/krust)

A Krunker.io client written in Rust with [Tauri 2](https://tauri.app). It uses
the system WebView2 runtime instead of bundling Chromium, so the browser engine
stays current through Windows and Edge updates.

Unofficial. Not affiliated with Krunker or FRVR.

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
- Optional uncapped frame rate (WebView2 browser arguments)
- Userscript loader
- Resource swapper, compatible with Crankshaft-style swap folders
- Settings window (<kbd>F1</kbd>) and keyboard shortcuts
- Signed automatic updates
- Log file for bug reports (Settings → Advanced → Logs)

Planned: ad and tracker blocking, Discord presence.

## Status

Pre-release. Tested on Windows 11 with WebView2 157 and a 240 Hz display:

- The installer is under 2 MB and installs per user, without admin rights.
  Uninstalling removes the app and its shortcuts and keeps your settings.
- On a live game page: 240 FPS with `uncap_fps` off, and 900 to 1,200 FPS
  with it on (short samples). Long-session stability is not yet tested.
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

## Settings, userscripts and swaps

All live in the app config directory, `%APPDATA%\dev.krust.client`. The
settings window (<kbd>F1</kbd>) edits them and opens these folders for you.

- `settings.json` is created on first run. Changes to frame rate, the
  swapper and browser arguments apply after a restart; the settings window
  says when one is needed and offers a restart button.

  | Key          | Default | Meaning                              |
  | ------------ | ------- | ------------------------------------ |
  | `uncap_fps`  | `true`  | Lift the display-refresh frame cap   |
  | `fullscreen` | `false` | Start in fullscreen                  |
  | `width`      | `1280`  | Window width                         |
  | `height`     | `720`   | Window height                        |
  | `extra_browser_args` | `""` | Extra Chromium switches for WebView2 |
  | `resource_swapper` | `true` | Serve files from the `swap` folder |
  | `check_for_updates` | `true` | Look for updates at launch |

- `scripts\*.js` are injected into Krunker pages before the game's own
  scripts, in file-name order. Changes apply on the next launch.

- `swap\` replaces Krunker's assets with your own files. Mirror the URL
  path: `swap\textures\weapons\weapon_2.png` replaces
  `https://assets.krunker.io/textures/weapons/weapon_2.png` (and the same
  path on any `krunker.io` subdomain, with or without an `/assets` prefix).
  Query strings are ignored, so swaps survive game updates. Swap packs made
  for Crankshaft work unchanged. The folder is scanned at launch, so restart
  after adding or removing files; edits to an existing file apply on the
  next load of that file.

  Swapped responses are sent with `Cache-Control: no-store`, so a removed
  swap never lingers in the browser cache.

## Security

The game is remote content and gets no access to Tauri's IPC, so the page
cannot call into the Rust side. The settings commands are declared in
`build.rs`, which makes Tauri deny them by default, and
`capabilities/settings.json` grants them only to the bundled settings
window. The folder command takes a name (`swap`, `scripts`, `config`),
never a path. Userscripts run with the page's privileges;
only install ones you trust.

The swapper only serves files found when the `swap` folder was scanned, so
a crafted URL such as `/../../Windows/win.ini` cannot reach anything else on
disk.

## Releasing

See [RELEASING.md](RELEASING.md).

## Credits

Feature ideas come from [Crankshaft](https://github.com/KraXen72/crankshaft)
(GPL-3.0) and the project layout follows
[tauri-krunker](https://github.com/vaqqq/tauri-krunker) (MIT). No code from
either is included.

## License

MIT
