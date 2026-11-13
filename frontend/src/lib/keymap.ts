/**
 * The clipboard keymap and its DOM level guard.
 *
 * The user requirement is that the *platform native* shortcuts work:
 *
 * | OS | Copy | Paste | Cut |
 * |---|---|---|---|
 * | Windows | `Ctrl+C` | `Ctrl+V` | `Ctrl+X` |
 * | Linux | `Ctrl+C` | `Ctrl+V` | `Ctrl+X` |
 * | macOS | `Cmd+C` | `Cmd+V` | `Cmd+X` |
 *
 * CodeMirror's `Mod` modifier already resolves to `Cmd` on macOS and to `Ctrl` everywhere
 * else, so a single `Mod-c` binding covers all three platforms without sniffing the OS.
 *
 * The bindings are additionally guarded at the DOM level (`Prec.highest`) because the VIM
 * extension installs its own `keydown` handler that would otherwise swallow `Ctrl+V` (it is
 * bound to *block visual* in VIM) before the keymap ever sees it.
 */
import { EditorView, keymap } from "@codemirror/view";
import { Prec } from "@codemirror/state";
import { selectAll } from "@codemirror/commands";
import { getCM, Vim } from "@replit/codemirror-vim";

import { isMac, readClipboard, writeClipboard } from "./clipboard";

/** Whether nothing is selected (the cursor is just sitting there). */
export function selectionIsEmpty(view: EditorView): boolean {
  return view.state.selection.ranges.every((range) => range.empty);
}

/** The text that a copy/cut operation should place on the clipboard. */
export function selectionText(view: EditorView): string {
  const { state } = view;
  const ranges = state.selection.ranges;
  if (ranges.length === 0) return "";

  // Nothing selected: copy the whole current line, like every other editor does.
  if (selectionIsEmpty(view)) {
    const line = state.doc.lineAt(state.selection.main.head);
    return line.text;
  }

  return ranges.map((range) => state.sliceDoc(range.from, range.to)).join("\n");
}

/** Leaves VIM visual mode (if the editor is in it) after a clipboard operation. */
function leaveVisualMode(view: EditorView): void {
  try {
    const cm = getCM(view);
    const vim = cm?.state?.vim;
    if (!cm || !vim || !vim.visualMode) return;
    Vim.exitVisualMode(cm as unknown as Parameters<typeof Vim.exitVisualMode>[0]);
  } catch {
    // The editor is not in VIM mode — nothing to do.
  }
}

/** Copies the selection (or the current line) to the clipboard. */
export function copySelection(view: EditorView): boolean {
  const text = selectionText(view);
  if (!text) return false;
  leaveVisualMode(view);
  void writeClipboard(text);
  return true;
}

/** Copies the selection (or the current line) and removes it from the document. */
export function cutSelection(view: EditorView): boolean {
  const { state } = view;
  const text = selectionText(view);
  if (!text) return false;
  leaveVisualMode(view);
  void writeClipboard(text);

  if (selectionIsEmpty(view)) {
    const line = state.doc.lineAt(state.selection.main.head);
    view.dispatch({
      changes: { from: line.from, to: line.to, insert: "" },
      userEvent: "delete.selection",
    });
  } else {
    view.dispatch({ ...state.replaceSelection(""), userEvent: "delete.selection" });
  }
  return true;
}

/** Inserts the clipboard contents at the cursor. */
export function pasteClipboard(view: EditorView): boolean {
  void (async () => {
    const text = await readClipboard();
    if (!text) return;
    const state = view.state;
    view.dispatch({
      ...state.replaceSelection(text),
      userEvent: "input.paste",
      scrollIntoView: true,
    });
  })();
  return true;
}

/** Recognises the platform's copy/paste/cut combination. */
export function clipboardCombo(
  event: KeyboardEvent,
): "copy" | "cut" | "paste" | null {
  const mod = isMac() ? event.metaKey : event.ctrlKey;
  if (!mod || event.altKey || event.shiftKey) return null;

  switch (event.key.toLowerCase()) {
    case "c":
      return "copy";
    case "x":
      return "cut";
    case "v":
      return "paste";
    default:
      return null;
  }
}

/**
 * The explicit keymap.
 *
 * `Mod` is `Cmd` on macOS and `Ctrl` on Windows and Linux, so this single binding implements
 * the platform native shortcuts everywhere.
 */
export const clipboardKeymap = Prec.highest(
  keymap.of([
    { key: "Mod-c", run: copySelection, preventDefault: true },
    { key: "Mod-v", run: pasteClipboard, preventDefault: true },
    { key: "Mod-x", run: cutSelection, preventDefault: true },
    { key: "Mod-a", run: selectAll, preventDefault: true },
  ]),
);

/**
 * A DOM level guard that runs before the VIM extension.
 *
 * Without it `Ctrl+V` would start a block-wise visual selection instead of pasting, because the
 * VIM extension binds `<C-v>` to `toggleVisualMode`.
 */
export const clipboardDomGuard = Prec.highest(
  EditorView.domEventHandlers({
    keydown(event, view) {
      const action = clipboardCombo(event);
      if (!action) return false;

      event.preventDefault();
      event.stopPropagation();

      if (action === "copy") copySelection(view);
      else if (action === "cut") cutSelection(view);
      else pasteClipboard(view);

      return true;
    },
  }),
);
