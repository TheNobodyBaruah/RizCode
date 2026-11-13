import { useState } from "react";

export interface OutputPanelProps {
  stdout: string;
  stderr: string;
  truncated: boolean;
}

/** The panel that shows the captured output of the program. */
export function OutputPanel({ stdout, stderr, truncated }: OutputPanelProps) {
  const [showStdout, setShowStdout] = useState(true);
  const [showStderr, setShowStderr] = useState(true);

  return (
    <section className="panel panel-output" aria-label="Program output">
      <header className="panel-header">
        <h2>Output</h2>
        {truncated ? (
          <span className="panel-hint panel-hint-warn">output truncated</span>
        ) : null}
      </header>

      <div className="output-section">
        <button
          type="button"
          className="output-toggle"
          aria-expanded={showStdout}
          onClick={() => setShowStdout((value) => !value)}
        >
          stdout
        </button>
        {showStdout ? (
          <pre className="output-stream" data-testid="stdout">
            {stdout || ""}
          </pre>
        ) : null}
      </div>

      <div className="output-section">
        <button
          type="button"
          className="output-toggle output-toggle-error"
          aria-expanded={showStderr}
          onClick={() => setShowStderr((value) => !value)}
        >
          stderr
        </button>
        {showStderr ? (
          <pre
            className="output-stream output-stream-error"
            data-testid="stderr"
          >
            {stderr || ""}
          </pre>
        ) : null}
      </div>
    </section>
  );
}
