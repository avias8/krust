//! Userscript loader: `.js` files from the `scripts` folder, injected into
//! Krunker pages before the game's own scripts.

use std::{fs, path::Path};

/// Returns `(file name, source)` for every `.js` file in `dir`, sorted by
/// name so the load order is predictable.
pub fn load(dir: &Path) -> Vec<(String, String)> {
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
pub fn wrap(name: &str, source: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_escapes_the_file_name() {
        let wrapped = wrap("it's \"mine\".js", "let a = 1;");
        assert!(wrapped.contains("let a = 1;"));
        assert!(wrapped.contains(r#""it's \"mine\".js""#));
    }
}
