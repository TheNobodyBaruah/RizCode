import type { Language, RuntimeInfo } from "../lib/api";
import { LANGUAGES } from "../lib/languages";

export interface ToolbarProps {
  language: Language;
  onLanguageChange: (language: Language) => void;
  onRun: () => void;
  onSave: () => void;
  onOpen: () => void;
  onToggleVim: () => void;
  onToggleAutoRun: () => void;
  vimEnabled: boolean;
  autoRun: boolean;
  running: boolean;
  dirty: boolean;
  filePath: string | null;
  runtimes: RuntimeInfo[];
}

/** The toolbar above the editor. */
export function Toolbar({
  language,
  onLanguageChange,
  onRun,
  onSave,
  onOpen,
  onToggleVim,
  onToggleAutoRun,
  vimEnabled,
  autoRun,
  running,
  dirty,
  filePath,
  runtimes,
}: ToolbarProps) {
  return (
    <header className="toolbar">
      <div className="toolbar-group">
        <span className="brand">
          <span className="brand-mark">&gt;_</span>
          Code Sandbox
        </span>

        <label className="field">
          <span className="field-label">Language</span>
          <select
            aria-label="Language"
            value={language}
            onChange={(event) => onLanguageChange(event.target.value as Language)}
          >
            {LANGUAGES.map((entry) => (
              <option key={entry.id} value={entry.id}>
                {entry.label}
              </option>
            ))}
          </select>
        </label>

        <div className="runtime-badges" aria-label="Available runtimes">
          {runtimes.map((runtime) => (
            <span
              key={runtime.language}
              className={`badge ${runtime.available ? "badge-ok" : "badge-missing"}`}
              title={
                runtime.available
                  ? `${runtime.program} ${runtime.version ?? ""}`
                  : (runtime.message ?? "not available")
              }
            >
              {runtime.language}
            </span>
          ))}
        </div>
      </div>

      <div className="toolbar-group">
        <button
          type="button"
          className="button button-primary"
          onClick={onRun}
          disabled={running}
        >
          {running ? "Running…" : "Run"}
        </button>

        <button
          type="button"
          className="button"
          onClick={onOpen}
          title="Open a file"
        >
          Open
        </button>

        <button
          type="button"
          className="button"
          onClick={onSave}
          title="Save to a file"
        >
          Save
        </button>

        <button
          type="button"
          className={`button toggle ${autoRun ? "toggle-on" : ""}`}
          onClick={onToggleAutoRun}
          aria-pressed={autoRun}
          title="Run the code automatically while typing"
        >
          Auto-run
        </button>

        <button
          type="button"
          className={`button toggle ${vimEnabled ? "toggle-on" : ""}`}
          onClick={onToggleVim}
          aria-pressed={vimEnabled}
          title="Toggle the VIM mode (Ctrl+Shift+V)"
        >
          VIM
        </button>

        <span className="file-name" title={filePath ?? "not saved yet"}>
          {filePath ? (
            <>
              {dirty ? <span className="dirty-dot" aria-label="unsaved changes" /> : null}
              {filePath}
            </>
          ) : (
            <>
              {dirty ? <span className="dirty-dot" aria-label="unsaved changes" /> : null}
              unsaved
            </>
          )}
        </span>
      </div>
    </header>
  );
}
