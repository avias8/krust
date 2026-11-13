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
- Resource swapper, compatible with Crankshaft-style swap folders
- JSON settings file

Planned: settings UI, ad and tracker blocking, Discord presence.

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

## Settings, userscripts and swaps

All live in the app config directory, `%APPDATA%\dev.krust.client`:

- `settings.json` is created on first run:

  | Key          | Default | Meaning                              |
  | ------------ | ------- | ------------------------------------ |
  | `uncap_fps`  | `true`  | Lift the display-refresh frame cap   |
  | `fullscreen` | `false` | Start in fullscreen                  |
  | `width`      | `1280`  | Window width                         |
  | `height`     | `720`   | Window height                        |
  | `extra_browser_args` | `""` | Extra Chromium switches for WebView2 |
  | `resource_swapper` | `true` | Serve files from the `swap` folder |

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
cannot call into the Rust side. Userscripts run with the page's privileges;
only install ones you trust.

The swapper only serves files found when the `swap` folder was scanned, so
a crafted URL such as `/../../Windows/win.ini` cannot reach anything else on
disk.

## Credits

Feature ideas come from [Crankshaft](https://github.com/KraXen72/crankshaft)
(GPL-3.0) and the project layout follows
[tauri-krunker](https://github.com/vaqqq/tauri-krunker) (MIT). No code from
either is included.

## License

MIT
