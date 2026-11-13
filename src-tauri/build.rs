fn main() {
    // Declaring the app's commands makes Tauri deny them by default. Only
    // windows granted them in a capability (the local settings window) can
    // call them; the remote game page cannot.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_settings",
            "save_settings",
            "open_folder",
            "restart",
        ]),
    ))
    .expect("failed to run tauri-build");
}
