# Krust

A Krunker.io client written in Rust with [Tauri 2](https://tauri.app). It uses
the system WebView2 runtime instead of bundling Chromium, so the browser engine
stays current through Windows and Edge updates.

Unofficial. Not affiliated with Krunker or FRVR.

## Status

Early, but it builds and runs. Measured on Windows 11 with WebView2 157 and a
240 Hz display, on a live Krunker game page: 240 FPS with `uncap_fps` off and
about 920 FPS with it on. Long-session stability is not yet tested.

- Opens krunker.io in a native window
- Optional uncapped frame rate (WebView2 browser arguments)
- Userscript loader
- JSON settings file

Planned: resource swapper, settings UI, ad and tracker blocking, Discord
presence.

## Requirements

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

Build an installer with `npm run build`. Run the unit tests with
`cargo test` inside `src-tauri`.

## Settings and userscripts

Both live in the app config directory, `%APPDATA%\dev.krust.client`:

- `settings.json` is created on first run:

  | Key          | Default | Meaning                              |
  | ------------ | ------- | ------------------------------------ |
  | `uncap_fps`  | `true`  | Lift the display-refresh frame cap   |
  | `fullscreen` | `false` | Start in fullscreen                  |
  | `width`      | `1280`  | Window width                         |
  | `height`     | `720`   | Window height                        |
  | `extra_browser_args` | `""` | Extra Chromium switches for WebView2 |

- `scripts\*.js` are injected into Krunker pages before the game's own
  scripts, in file-name order. Changes apply on the next launch.

## Security

The game is remote content and gets no access to Tauri's IPC, so the page
cannot call into the Rust side. Userscripts run with the page's privileges;
only install ones you trust.

## Credits

Feature ideas come from [Crankshaft](https://github.com/KraXen72/crankshaft)
(GPL-3.0) and the project layout follows
[tauri-krunker](https://github.com/vaqqq/tauri-krunker) (MIT). No code from
either is included.

## License

MIT
