/**
 * The typed bridge to the Rust backend.
 *
 * Every backend call goes through this module so the rest of the frontend never touches
 * `invoke` directly — that keeps the payload shapes in one place and makes the tests able to
 * mock a single module.
 */
import { invoke } from "@tauri-apps/api/core";

/** The languages the sandbox can execute. */
export type Language = "rust" | "python" | "javascript";

/** The outcome of a run, mirroring `sandbox_core::RunStatus`. */
export type RunStatus =
  | "completed"
  | "compile_error"
  | "timed_out"
  | "launch_error"
  | "spawn_error";

/** The result of a run, mirroring `sandbox_core::RunOutcome`. */
export interface RunOutcome {
  stdout: string;
  stderr: string;
  exit_code: number | null;
  duration_ms: number;
  status: RunStatus;
  truncated: boolean;
}

/** Availability information for one toolchain. */
export interface RuntimeInfo {
  language: Language;
  available: boolean;
  program: string | null;
  version: string | null;
  message: string | null;
}

/** The result of saving a file. */
export interface SaveFileResult {
  path: string;
  bytes_written: number;
}

/** The result of opening a file. */
export interface OpenFileResult {
  path: string;
  contents: string;
}

/** Static information about the running application. */
export interface AppInfo {
  version: string;
  platform: string;
  core_version: string;
}

/** The payload of the `run_code` command. */
export interface RunCodePayload {
  language: Language;
  code: string;
  stdin: string;
}

/** Compiles and runs a piece of code. */
export function runCode(payload: RunCodePayload): Promise<RunOutcome> {
  return invoke<RunOutcome>("run_code", { payload });
}

/** Writes a file to disk (atomically). */
export function saveFile(path: string, contents: string): Promise<SaveFileResult> {
  return invoke<SaveFileResult>("save_file", { path, contents });
}

/** Reads a file from disk. */
export function openFile(path: string): Promise<OpenFileResult> {
  return invoke<OpenFileResult>("open_file", { path });
}

/** Reports which toolchains are installed. */
export function detectRuntimes(): Promise<RuntimeInfo[]> {
  return invoke<RuntimeInfo[]>("detect_runtimes");
}

/** Static information about the application. */
export function appInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app_info");
}

/** Writes text to the system clipboard (native fallback route). */
export function clipboardWriteText(text: string): Promise<void> {
  return invoke<void>("clipboard_write_text", { text });
}

/** Reads text from the system clipboard (native fallback route). */
export function clipboardReadText(): Promise<string> {
  return invoke<string>("clipboard_read_text");
}
