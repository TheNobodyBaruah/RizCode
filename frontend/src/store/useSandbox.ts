/**
 * The single store of the application.
 *
 * It owns the language, the buffer, the stdin, the last run, the file on disk and the toggles.
 * Everything is plain React state — no state library is needed for a machine this small, and
 * the components stay trivially testable.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";

import * as api from "../lib/api";
import type { Language, RunOutcome, RunStatus, RuntimeInfo } from "../lib/api";
import { LANGUAGES, describeLanguage } from "../lib/languages";

/** How long the app waits after the last keystroke before it runs the code. */
export const AUTO_RUN_DEBOUNCE_MS = 800;

/** The state of the sandbox. */
export interface SandboxState {
  /** The language of the editor. */
  language: Language;
  /** The source code. */
  code: string;
  /** The data piped into the program. */
  stdin: string;
  /** The captured standard output of the last run. */
  stdout: string;
  /** The captured standard error of the last run. */
  stderr: string;
  /** The exit code of the last run. */
  exitCode: number | null;
  /** How long the last run took. */
  durationMs: number;
  /** The status of the last run. */
  status: RunStatus | null;
  /** Whether the last run produced more output than the limit allows. */
  truncated: boolean;
  /** Whether a run is currently in flight. */
  running: boolean;
  /** Whether the VIM mode is enabled. */
  vimEnabled: boolean;
  /** Whether the code is executed automatically while typing. */
  autoRun: boolean;
  /** The path of the file the buffer was last saved to. */
  filePath: string | null;
  /** Whether the buffer contains changes that were not saved. */
  dirty: boolean;
  /** Availability of the toolchains. */
  runtimes: RuntimeInfo[];
  /** The last error that happened outside of a run. */
  error: string | null;
}

/** The actions exposed by the store. */
export interface SandboxActions {
  setLanguage: (language: Language) => void;
  setCode: (code: string) => void;
  setStdin: (stdin: string) => void;
  run: () => Promise<void>;
  toggleVim: () => void;
  toggleAutoRun: () => void;
  save: () => Promise<string | null>;
  open: () => Promise<string | null>;
  dismissError: () => void;
}

/** The store and its actions. */
export function useSandbox(): SandboxState & SandboxActions {
  const [language, setLanguageState] = useState<Language>(LANGUAGES[0].id);
  const [code, setCode] = useState<string>(LANGUAGES[0].starter);
  const [stdin, setStdin] = useState<string>("");
  const [outcome, setOutcome] = useState<RunOutcome | null>(null);
  const [running, setRunning] = useState(false);
  const [vimEnabled, setVimEnabled] = useState(false);
  const [autoRun, setAutoRun] = useState(true);
  const [filePath, setFilePath] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [runtimes, setRuntimes] = useState<RuntimeInfo[]>([]);
  const [error, setError] = useState<string | null>(null);

  // Runs are sequence numbered: a result that arrives after a newer run started is discarded.
  const runIdRef = useRef(0);
  // The buffer that was last handed to the auto-run debounce.
  const lastAutoRunRef = useRef<string>(code);
  // The values the asynchronous callbacks must read.
  const latest = useRef({ language, code, stdin });

  useEffect(() => {
    latest.current = { language, code, stdin };
  }, [language, code, stdin]);

  const run = useCallback(async () => {
    const id = ++runIdRef.current;
    setRunning(true);
    setError(null);

    try {
      const result = await api.runCode({
        language: latest.current.language,
        code: latest.current.code,
        stdin: latest.current.stdin,
      });
      if (id !== runIdRef.current) return; // superseded by a newer run
      setOutcome(result);
    } catch (err) {
      if (id !== runIdRef.current) return;
      setOutcome(null);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      if (id === runIdRef.current) setRunning(false);
    }
  }, []);

  // Auto-run: 800 ms after the last keystroke (decision D10).
  useEffect(() => {
    if (code === lastAutoRunRef.current) return;
    lastAutoRunRef.current = code;
    if (!autoRun) return;

    const timer = setTimeout(() => {
      void run();
    }, AUTO_RUN_DEBOUNCE_MS);

    return () => clearTimeout(timer);
  }, [code, autoRun, run]);

  // Probe the toolchains once at start-up.
  useEffect(() => {
    let cancelled = false;
    api
      .detectRuntimes()
      .then((result) => {
        if (!cancelled) setRuntimes(result);
      })
      .catch(() => {
        if (!cancelled) setRuntimes([]);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const setLanguage = useCallback((next: Language) => {
    setLanguageState(next);
    // The saved path belongs to the previous language — force a new choice on the next save.
    setFilePath(null);
    setDirty(true);
  }, []);

  const handleCodeChange = useCallback((next: string) => {
    setCode(next);
    setDirty(true);
  }, []);

  const toggleVim = useCallback(() => setVimEnabled((value) => !value), []);
  const toggleAutoRun = useCallback(() => setAutoRun((value) => !value), []);
  const dismissError = useCallback(() => setError(null), []);

  const save = useCallback(async () => {
    const descriptor = describeLanguage(latest.current.language);
    const defaultPath = filePath
      ? `${directoryOf(filePath)}/${descriptor.fileName}`
      : descriptor.fileName;

    const target = await saveDialog({
      defaultPath,
      filters: [{ name: descriptor.label, extensions: [descriptor.extension] }],
    });
    if (!target) return null;

    const result = await api.saveFile(target, latest.current.code);
    setFilePath(result.path);
    setDirty(false);
    setError(null);
    return result.path;
  }, [filePath]);

  const open = useCallback(async () => {
    const target = await openDialog({ multiple: false, directory: false });
    if (!target || Array.isArray(target)) return null;

    const result = await api.openFile(target);
    setCode(result.contents);
    setFilePath(result.path);
    setDirty(false);
    setError(null);
    return result.path;
  }, []);

  const state = useMemo<SandboxState>(
    () => ({
      language,
      code,
      stdin,
      stdout: outcome?.stdout ?? "",
      stderr: outcome?.stderr ?? "",
      exitCode: outcome?.exit_code ?? null,
      durationMs: outcome?.duration_ms ?? 0,
      status: outcome?.status ?? null,
      truncated: outcome?.truncated ?? false,
      running,
      vimEnabled,
      autoRun,
      filePath,
      dirty,
      runtimes,
      error,
    }),
    [
      language,
      code,
      stdin,
      outcome,
      running,
      vimEnabled,
      autoRun,
      filePath,
      dirty,
      runtimes,
      error,
    ],
  );

  return {
    ...state,
    setLanguage,
    setCode: handleCodeChange,
    setStdin,
    run,
    toggleVim,
    toggleAutoRun,
    save,
    open,
    dismissError,
  };
}

/** The directory part of a path (both separators are understood). */
export function directoryOf(path: string): string {
  const index = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  if (index <= 0) return ".";
  return path.slice(0, index);
}
