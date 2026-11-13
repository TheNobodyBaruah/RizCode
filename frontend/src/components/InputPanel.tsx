export interface InputPanelProps {
  value: string;
  onChange: (value: string) => void;
}

/** The panel that holds the data piped into the program's standard input. */
export function InputPanel({ value, onChange }: InputPanelProps) {
  return (
    <section className="panel panel-input" aria-label="Input panel">
      <header className="panel-header">
        <h2>Input</h2>
        <span className="panel-hint">stdin</span>
      </header>
      <textarea
        className="stdin-input"
        aria-label="Program input"
        spellCheck={false}
        placeholder="Data piped to the program's standard input…"
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
    </section>
  );
}
