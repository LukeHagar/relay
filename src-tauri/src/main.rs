//! Relay desktop shell.
//!
//! Thin by design (D2/PLAN.md): the Go→Rust engine owns all logic; this shell only
//! hosts the inbox UI in a native webview. The UI talks to the engine over the same
//! wire contract as every other client (docs/api.md).
//!
//! The window loads the control plane URL so the shell works against a local engine
//! (`relay serve`) or a remote one — no engine code is bundled here.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

#[derive(serde::Deserialize)]
struct ShellConfig {
    #[serde(default = "default_engine_url")]
    engine_url: String,
}

fn default_engine_url() -> String {
    "http://127.0.0.1:8000".into()
}

fn main() {
    let config: ShellConfig = std::env::var("RELAY_SHELL_CONFIG")
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(ShellConfig { engine_url: default_engine_url() });

    tauri::Builder::default()
        .setup(move |app| {
            let window = app.get_webview_window("main").expect("main window exists");
            // Navigate the pre-declared window to the engine's embedded UI.
            let url = format!("{}/", config.engine_url.trim_end_matches('/'));
            window.eval(&format!("location.replace({url:?})"))?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running relay desktop");
}
