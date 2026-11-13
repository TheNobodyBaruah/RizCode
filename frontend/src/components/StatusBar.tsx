import type { RunStatus } from "../lib/api";

export interface StatusBarProps {
  status: RunStatus | null;
  exitCode: number | null;
  durationMs: number;
  running: boolean;
  vimEnabled: boolean;
  vimMode: string;
  truncated: boolean;
  filePath: string | null;
  dirty: boolean;
  /** The label of the platform's primary modifier (`⌘` or `Ctrl`). */
  modifier: string;
  zoom?: number;
  onZoomIn?: () => void;
  onZoomOut?: () => void;
  onResetZoom?: () => void;
}

/** A human readable label for a run status. */
export function statusLabel(status: RunStatus | null): string {
  switch (status) {
    case "completed":
      return "completed";
    case "compile_error":
      return "compile error";
    case "timed_out":
      return "timed out";
    case "launch_error":
      return "launch error";
    case "spawn_error":
      return "spawn error";
    default:
      return "idle";
  }
}

/** The status bar at the bottom of the window. */
export function StatusBar({
  status,
  exitCode,
  durationMs,
  running,
  vimEnabled,
  vimMode,
  truncated,
  filePath,
  dirty,
  modifier,
  zoom = 100,
  onZoomIn,
  onZoomOut,
  onResetZoom,
}: StatusBarProps) {
  return (
    <footer className="status-bar">
      <span className={`status-pill status-${status ?? "idle"}`}>
        {running ? "running" : statusLabel(status)}
      </span>

      {exitCode !== null ? (
        <span className="status-item">exit {exitCode}</span>
      ) : null}

      {durationMs > 0 ? (
        <span className="status-item">{durationMs} ms</span>
      ) : null}

      {truncated ? <span className="status-item">truncated</span> : null}

      <div className="status-item status-zoom" data-testid="zoom-controls">
        <button
          type="button"
          className="zoom-btn"
          onClick={onZoomOut}
          disabled={zoom <= 70}
          aria-label="Zoom out"
          title={`Zoom out (${modifier}+-)`}
        >
          -
        </button>
        <button
          type="button"
          className="zoom-label-btn"
          onClick={onResetZoom}
          aria-label="Reset zoom"
          title={`Reset zoom to 100% (${modifier}+0)`}
          data-testid="zoom-level"
        >
          {zoom}%
        </button>
        <button
          type="button"
          className="zoom-btn"
          onClick={onZoomIn}
          disabled={zoom >= 200}
          aria-label="Zoom in"
          title={`Zoom in (${modifier}++)`}
        >
          +
        </button>
      </div>

      <span className="spacer" />

      {vimEnabled ? (
        <span className="status-item status-vim" data-testid="vim-mode">
          VIM · {vimMode.toUpperCase()}
        </span>
      ) : (
        <span className="status-item" data-testid="vim-mode">
          NORMAL
        </span>
      )}

      <span className="status-item status-hint">
        {modifier}+Enter run · {modifier}+C copy · {modifier}+V paste
        {vimEnabled ? " · :w runs" : ""}
      </span>

      <span className="status-item" title={filePath ?? "not saved"}>
        {dirty ? "modified" : filePath ? "saved" : "new"}
      </span>
    </footer>
  );
}
