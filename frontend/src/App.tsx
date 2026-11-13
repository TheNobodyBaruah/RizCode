import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";

import { Editor } from "./components/Editor";
import { InputPanel } from "./components/InputPanel";
import { OutputPanel } from "./components/OutputPanel";
import { StatusBar } from "./components/StatusBar";
import { Toolbar } from "./components/Toolbar";
import { modifierLabel } from "./lib/clipboard";
import { useSandbox } from "./store/useSandbox";

/** The application shell: editor on the left, input/output on the right. */
export default function App() {
  const sandbox = useSandbox();
  const [vimMode, setVimMode] = useState("normal");
  const [editorRatio, setEditorRatio] = useState(0.62);
  const workspaceRef = useRef<HTMLDivElement | null>(null);
  const draggingRef = useRef(false);

  const { run, toggleVim, zoomIn, zoomOut, resetZoom } = sandbox;

  // Global shortcuts:
  // - Ctrl/Cmd+Enter: runs
  // - Ctrl/Cmd+Shift+V: toggles the VIM mode
  // - Ctrl/Cmd + (+ or =): zoom in
  // - Ctrl/Cmd + (- or _): zoom out
  // - Ctrl/Cmd + 0: reset zoom
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.metaKey && !event.ctrlKey) return;

      if (event.key === "Enter") {
        event.preventDefault();
        void run();
        return;
      }
      if (event.shiftKey && event.key.toLowerCase() === "v") {
        event.preventDefault();
        toggleVim();
        return;
      }
      if (!event.altKey) {
        if (event.key === "=" || event.key === "+") {
          event.preventDefault();
          zoomIn();
          return;
        }
        if (event.key === "-" || event.key === "_") {
          event.preventDefault();
          zoomOut();
          return;
        }
        if (event.key === "0") {
          event.preventDefault();
          resetZoom();
          return;
        }
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [run, toggleVim, zoomIn, zoomOut, resetZoom]);

  const onPointerDown = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    draggingRef.current = true;
    event.currentTarget.setPointerCapture(event.pointerId);
  }, []);

  const onPointerMove = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    if (!draggingRef.current) return;
    const container = workspaceRef.current;
    if (!container) return;
    const rect = container.getBoundingClientRect();
    if (rect.width === 0) return;
    const ratio = (event.clientX - rect.left) / rect.width;
    setEditorRatio(Math.min(0.85, Math.max(0.25, ratio)));
  }, []);

  const onPointerUp = useCallback(() => {
    draggingRef.current = false;
  }, []);

  const style = { "--editor-ratio": editorRatio } as CSSProperties;

  return (
    <div className="app">
      <Toolbar
        language={sandbox.language}
        onLanguageChange={sandbox.setLanguage}
        onRun={() => void sandbox.run()}
        onSave={() => void sandbox.save()}
        onOpen={() => void sandbox.open()}
        onToggleVim={sandbox.toggleVim}
        onToggleAutoRun={sandbox.toggleAutoRun}
        vimEnabled={sandbox.vimEnabled}
        autoRun={sandbox.autoRun}
        running={sandbox.running}
        dirty={sandbox.dirty}
        filePath={sandbox.filePath}
        runtimes={sandbox.runtimes}
        zoom={sandbox.zoom}
        onZoomIn={sandbox.zoomIn}
        onZoomOut={sandbox.zoomOut}
        onResetZoom={sandbox.resetZoom}
      />

      {sandbox.error ? (
        <div className="error-banner" role="alert">
          {sandbox.error}
          <button
            type="button"
            className="error-dismiss"
            onClick={sandbox.dismissError}
            aria-label="Dismiss"
          >
            ×
          </button>
        </div>
      ) : null}

      <div className="workspace" ref={workspaceRef} style={style}>
        <section className="editor-pane" aria-label="Source code">
          <Editor
            value={sandbox.code}
            language={sandbox.language}
            vimEnabled={sandbox.vimEnabled}
            onChange={sandbox.setCode}
            onRun={sandbox.run}
            onModeChange={setVimMode}
          />
        </section>

        <div
          className="splitter"
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize the panels"
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={onPointerUp}
        />

        <section className="side-pane" aria-label="Input and output">
          <InputPanel value={sandbox.stdin} onChange={sandbox.setStdin} />
          <OutputPanel
            stdout={sandbox.stdout}
            stderr={sandbox.stderr}
            truncated={sandbox.truncated}
          />
        </section>
      </div>

      <StatusBar
        status={sandbox.status}
        exitCode={sandbox.exitCode}
        durationMs={sandbox.durationMs}
        running={sandbox.running}
        vimEnabled={sandbox.vimEnabled}
        vimMode={vimMode}
        truncated={sandbox.truncated}
        filePath={sandbox.filePath}
        dirty={sandbox.dirty}
        modifier={modifierLabel()}
        zoom={sandbox.zoom}
        onZoomIn={sandbox.zoomIn}
        onZoomOut={sandbox.zoomOut}
        onResetZoom={sandbox.resetZoom}
      />
    </div>
  );
}
