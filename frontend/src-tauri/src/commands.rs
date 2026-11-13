//! The Tauri commands — a thin adapter over `sandbox-core`.
//!
//! Every command does one of three things: validate the payload, call into `sandbox-core`, and
//! turn the result into something that can be serialised to the frontend. No business logic
//! lives here.

use serde::{Deserialize, Serialize};
use tauri::State;
use tauri_plugin_clipboard_manager::ClipboardExt;

use sandbox_core::{atomic_write, Language, RunOutcome, RuntimeInfo};

use crate::state::AppState;

/// Payload of the `run_code` command.
#[derive(Debug, Clone, Deserialize)]
pub struct RunCodePayload {
    /// The language of the source code.
    pub language: Language,
    /// The source code to execute.
    pub code: String,
    /// Data piped to the program's standard input.
    #[serde(default)]
    pub stdin: String,
}

/// Result of the `save_file` command.
#[derive(Debug, Clone, Serialize)]
pub struct SaveFileResult {
    /// The absolute path that was written.
    pub path: String,
    /// How many bytes were written.
    pub bytes_written: usize,
}

/// Result of the `open_file` command.
#[derive(Debug, Clone, Serialize)]
pub struct OpenFileResult {
    /// The absolute path that was read.
    pub path: String,
    /// The contents of the file.
    pub contents: String,
}

/// Static information about the running application.
#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
    /// The version of the application.
    pub version: String,
    /// A human readable platform name.
    pub platform: String,
    /// The version of the execution engine.
    pub core_version: String,
}

/// Compiles and runs a piece of code.
#[tauri::command]
pub fn run_code(
    payload: RunCodePayload,
    state: State<'_, AppState>,
) -> Result<RunOutcome, String> {
    let request = sandbox_core::RunRequest {
        language: payload.language,
        code: payload.code,
        stdin: payload.stdin,
    };
    Ok(state.runner.run(&request))
}

/// Writes a file to disk atomically.
#[tauri::command]
pub fn save_file(path: String, contents: String) -> Result<SaveFileResult, String> {
    let target = std::path::PathBuf::from(&path);
    if target.as_os_str().is_empty() {
        return Err("no path was given".to_string());
    }
    atomic_write(&target, &contents).map_err(|err| format!("could not write {path}: {err}"))?;
    Ok(SaveFileResult {
        path,
        bytes_written: contents.len(),
    })
}

/// Reads a file from disk.
#[tauri::command]
pub fn open_file(path: String) -> Result<OpenFileResult, String> {
    if path.trim().is_empty() {
        return Err("no path was given".to_string());
    }
    let target = std::path::PathBuf::from(&path);
    let contents = std::fs::read_to_string(&target)
        .map_err(|err| format!("could not read {path}: {err}"))?;
    Ok(OpenFileResult { path, contents })
}

/// Reports which toolchains are installed.
#[tauri::command]
pub fn detect_runtimes() -> Result<Vec<RuntimeInfo>, String> {
    Ok(sandbox_core::detect_runtimes())
}

/// Static information about the application.
#[tauri::command]
pub fn app_info(app: tauri::AppHandle) -> Result<AppInfo, String> {
    let version = app.package_info().version.to_string();
    Ok(AppInfo {
        version,
        platform: platform_name(),
        core_version: sandbox_core::VERSION.to_string(),
    })
}

/// Writes text to the system clipboard.
#[tauri::command]
pub fn clipboard_write_text(app: tauri::AppHandle, text: String) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|err| format!("could not write to the clipboard: {err}"))
}

/// Reads text from the system clipboard.
#[tauri::command]
pub fn clipboard_read_text(app: tauri::AppHandle) -> Result<String, String> {
    app.clipboard()
        .read_text()
        .map_err(|err| format!("could not read from the clipboard: {err}"))
}

/// A human readable platform name.
fn platform_name() -> String {
    if cfg!(windows) {
        "Windows".to_string()
    } else if cfg!(target_os = "macos") {
        "macOS".to_string()
    } else if cfg!(target_os = "linux") {
        "Linux".to_string()
    } else {
        std::env::consts::OS.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "app-commands-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn save_file_writes_the_contents() {
        let dir = temp_dir();
        let target = dir.join("hello.txt");
        let result = save_file(
            target.to_string_lossy().to_string(),
            "hello".to_string(),
        )
        .unwrap();
        assert_eq!(result.bytes_written, 5);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "hello");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_file_rejects_an_empty_path() {
        assert!(save_file(String::new(), "x".to_string()).is_err());
    }

    #[test]
    fn open_file_reads_the_contents() {
        let dir = temp_dir();
        let target = dir.join("hello.txt");
        std::fs::write(&target, "hello").unwrap();
        let result = open_file(target.to_string_lossy().to_string()).unwrap();
        assert_eq!(result.contents, "hello");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_file_reports_a_missing_file() {
        let dir = temp_dir();
        let missing = dir.join("nope.txt");
        let error = open_file(missing.to_string_lossy().to_string()).unwrap_err();
        assert!(error.contains("could not read"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn platform_name_is_not_empty() {
        assert!(!platform_name().is_empty());
    }
}
