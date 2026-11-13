//! Resource swapper: serves local files in place of Krunker's own assets.
//!
//! Layout matches Crankshaft and other clients, so existing swap packs work:
//! `swap/textures/foo.png` replaces `https://<any>.krunker.io/textures/foo.png`
//! and `https://<any>.krunker.io/assets/textures/foo.png`. Query strings are
//! ignored.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use tauri::Url;

/// Every file under the swap folder, keyed by URL path (`/textures/foo.png`).
///
/// Lookups only ever return paths found while scanning, so a crafted URL
/// such as `/../secret` cannot reach files outside the folder.
#[derive(Debug, Default)]
pub struct SwapIndex {
    files: HashMap<String, PathBuf>,
}

impl SwapIndex {
    pub fn scan(root: &Path) -> Self {
        let mut index = Self::default();
        index.walk(root, "");
        index
    }

    fn walk(&mut self, dir: &Path, prefix: &str) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let key = format!("{prefix}/{name}");
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => self.walk(&entry.path(), &key),
                Ok(kind) if kind.is_file() => {
                    self.files.insert(key, entry.path());
                }
                _ => {}
            }
        }
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Returns the local file that should answer `url`, if any.
    pub fn resolve(&self, url: &str) -> Option<&Path> {
        let url = Url::parse(url).ok()?;
        let host = url.host_str()?;
        if host != "krunker.io" && !host.ends_with(".krunker.io") {
            return None;
        }
        let path = percent_decode(url.path());
        let stripped = path
            .strip_prefix("/assets")
            .filter(|rest| rest.starts_with('/'));
        self.files
            .get(path.as_str())
            .or_else(|| stripped.and_then(|rest| self.files.get(rest)))
            .map(PathBuf::as_path)
    }
}

/// Decodes `%XX` escapes so file names with spaces and similar match.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = |b: u8| (b as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// MIME type for the response. WebGL and audio decoding mostly sniff the
/// bytes, but scripts and stylesheets are rejected without the right type.
pub fn content_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "obj" | "txt" => "text/plain",
        "json" => "application/json",
        "js" => "text/javascript",
        "css" => "text/css",
        "html" => "text/html",
        "glb" => "model/gltf-binary",
        "gltf" => "model/gltf+json",
        _ => "application/octet-stream",
    }
}

#[cfg(windows)]
pub use windows_impl::attach;

#[cfg(windows)]
mod windows_impl {
    use std::fs;

    use webview2_com::{
        take_pwstr, Microsoft::Web::WebView2::Win32::*, WebResourceRequestedEventHandler,
    };
    use windows::{
        core::{Interface, HSTRING, PWSTR},
        Win32::UI::Shell::SHCreateMemStream,
    };

    use super::{content_type, SwapIndex};

    /// Installs the swapper, then navigates to `url`. Navigating only after
    /// the handler is in place means no early request can slip past it.
    pub fn attach(webview: tauri::webview::PlatformWebview, index: SwapIndex, url: &str) {
        let result = unsafe { webview.controller().CoreWebView2() }.and_then(|core| {
            install(&core, webview.environment(), index)?;
            unsafe { core.Navigate(&HSTRING::from(url)) }
        });
        if let Err(err) = result {
            eprintln!("[krust] resource swapper failed to start: {err}");
        }
    }

    /// Registers the swapper on a WebView2 instance. Requests to krunker.io
    /// that have no local file are left alone and go to the network.
    fn install(
        core: &ICoreWebView2,
        env: ICoreWebView2Environment,
        index: SwapIndex,
    ) -> windows::core::Result<()> {
        let filter = HSTRING::from("*krunker.io/*");
        unsafe {
            // The newer API also covers iframes and workers.
            match core.cast::<ICoreWebView2_22>() {
                Ok(core22) => core22.AddWebResourceRequestedFilterWithRequestSourceKinds(
                    &filter,
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                    COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                )?,
                Err(_) => core.AddWebResourceRequestedFilter(
                    &filter,
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                )?,
            }

            let handler = WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
                let Some(args) = args else { return Ok(()) };
                let mut uri = PWSTR::null();
                args.Request()?.Uri(&mut uri)?;
                let uri = take_pwstr(uri);

                let Some(path) = index.resolve(&uri) else {
                    return Ok(());
                };
                let Ok(body) = fs::read(path) else {
                    eprintln!("[krust] swap file unreadable: {}", path.display());
                    return Ok(());
                };

                // no-store keeps swapped files out of the HTTP cache, so
                // removing a swap takes effect on the next launch.
                let headers = HSTRING::from(format!(
                    "Content-Type: {}\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-store",
                    content_type(path)
                ));
                let stream = SHCreateMemStream(Some(&body));
                let response = env.CreateWebResourceResponse(
                    stream.as_ref(),
                    200,
                    &HSTRING::from("OK"),
                    &headers,
                )?;
                args.SetResponse(&response)
            }));

            let mut token = 0i64;
            core.add_WebResourceRequested(&handler, &mut token)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_with(paths: &[&str]) -> SwapIndex {
        SwapIndex {
            files: paths
                .iter()
                .map(|p| (p.to_string(), PathBuf::from(format!("C:/swap{p}"))))
                .collect(),
        }
    }

    #[test]
    fn matches_root_and_assets_prefix_on_any_subdomain() {
        let index = index_with(&["/textures/a.png"]);
        assert!(index.resolve("https://krunker.io/textures/a.png").is_some());
        assert!(index
            .resolve("https://assets.krunker.io/textures/a.png?build=1")
            .is_some());
        assert!(index
            .resolve("https://krunker.io/assets/textures/a.png")
            .is_some());
        assert!(index.resolve("https://krunker.io/textures/b.png").is_none());
    }

    #[test]
    fn ignores_other_hosts() {
        let index = index_with(&["/textures/a.png"]);
        assert!(index
            .resolve("https://evilkrunker.io/textures/a.png")
            .is_none());
        assert!(index
            .resolve("https://krunker.io.example.com/textures/a.png")
            .is_none());
    }

    #[test]
    fn decodes_escaped_names() {
        let index = index_with(&["/sound/my shot.mp3"]);
        assert!(index
            .resolve("https://krunker.io/sound/my%20shot.mp3")
            .is_some());
    }

    #[test]
    fn traversal_cannot_escape_the_index() {
        let index = index_with(&["/textures/a.png"]);
        assert!(index
            .resolve("https://krunker.io/../../Windows/win.ini")
            .is_none());
    }

    #[test]
    fn scan_finds_nested_files() {
        let root = std::env::temp_dir().join(format!("krust-swap-test-{}", std::process::id()));
        fs::create_dir_all(root.join("textures/sub")).unwrap();
        fs::write(root.join("textures/sub/x.png"), b"x").unwrap();
        let index = SwapIndex::scan(&root);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(index.len(), 1);
        assert!(index
            .resolve("https://krunker.io/textures/sub/x.png")
            .is_some());
    }
}
