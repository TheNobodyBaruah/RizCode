import { useEffect, useRef } from "react";
import { Compartment, EditorState, type Extension } from "@codemirror/state";
import {
  EditorView,
  drawSelection,
  dropCursor,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from "@codemirror/commands";
import { bracketMatching, indentOnInput } from "@codemirror/language";
import { javascript } from "@codemirror/lang-javascript";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { getCM, vim } from "@replit/codemirror-vim";

import type { Language } from "../lib/api";
import { clipboardDomGuard, clipboardKeymap } from "../lib/keymap";
import { sandboxHighlighting, sandboxTheme } from "../lib/theme";
import { setVimRunHandler } from "../lib/vim";

export interface EditorProps {
  /** The current source code. */
  value: string;
  /** The language of the source code. */
  language: Language;
  /** Whether the VIM mode is enabled. */
  vimEnabled: boolean;
  /** Called whenever the source code changes. */
  onChange: (value: string) => void;
  /** Called when the user asks for a run (`:w` in VIM mode). */
  onRun: () => void;
  /** Called whenever the VIM mode changes. */
  onModeChange?: (mode: string) => void;
}

/** The CodeMirror language mode for a language. */
function languageExtension(language: Language): Extension {
  switch (language) {
    case "rust":
      return rust();
    case "python":
      return python();
    case "javascript":
      return javascript();
    default:
      return [];
  }
}

/** The current VIM mode of an editor (`normal`, `insert`, `visual`, …). */
export function currentVimMode(view: EditorView): string {
  try {
    const cm = getCM(view);
    return cm?.state?.vim?.mode ?? "normal";
  } catch {
    return "normal";
  }
}

/** The CodeMirror 6 editor. */
export function Editor({
  value,
  language,
  vimEnabled,
  onChange,
  onRun,
  onModeChange,
}: EditorProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const viewRef = useRef<EditorView | null>(null);
  const languageCompartment = useRef(new Compartment());
  const vimCompartment = useRef(new Compartment());
  const onChangeRef = useRef(onChange);
  const onModeChangeRef = useRef(onModeChange);
  const onRunRef = useRef(onRun);

  onChangeRef.current = onChange;
  onModeChangeRef.current = onModeChange;
  onRunRef.current = onRun;

  // `:w` runs the program (decision D1).
  useEffect(() => {
    setVimRunHandler(() => onRunRef.current());
    return () => setVimRunHandler(null);
  }, []);

  // Create the editor once.
  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    const view = new EditorView({
      state: EditorState.create({
        doc: value,
        extensions: [
          lineNumbers(),
          highlightActiveLine(),
          highlightActiveLineGutter(),
          history(),
          drawSelection(),
          dropCursor(),
          EditorState.allowMultipleSelections.of(true),
          indentOnInput(),
          bracketMatching(),
          keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
          EditorView.lineWrapping,
          sandboxTheme,
          sandboxHighlighting,
          languageCompartment.current.of(languageExtension(language)),
          vimCompartment.current.of(vimEnabled ? vim({ status: false }) : []),
          clipboardKeymap,
          clipboardDomGuard,
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              onChangeRef.current(update.state.doc.toString());
            }
            onModeChangeRef.current?.(currentVimMode(update.view));
          }),
        ],
      }),
      parent: host,
    });

    viewRef.current = view;
    return () => {
      view.destroy();
      viewRef.current = null;
    };
    // The editor is created once; everything else is reconfigured through compartments.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Switch the language mode.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: languageCompartment.current.reconfigure(languageExtension(language)),
    });
  }, [language]);

  // Toggle the VIM mode.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: vimCompartment.current.reconfigure(
        vimEnabled ? vim({ status: false }) : [],
      ),
    });
  }, [vimEnabled]);

  // Adopt a value that was changed outside of the editor (opening a file, for example).
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    const current = view.state.doc.toString();
    if (current === value) return;
    view.dispatch({ changes: { from: 0, to: current.length, insert: value } });
  }, [value]);

  return <div className="editor-host" ref={hostRef} data-testid="editor" />;
}
