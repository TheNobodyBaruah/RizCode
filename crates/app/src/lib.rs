//! The Tauri application — a thin shell around `sandbox-core`.

pub mod commands;
pub mod state;

use state::AppState;

/// Builds and runs the desktop application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::run_code,
            commands::save_file,
            commands::open_file,
            commands::detect_runtimes,
            commands::app_info,
            commands::clipboard_write_text,
            commands::clipboard_read_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Code Sandbox application");
}
