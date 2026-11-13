import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { waitFor } from "@testing-library/react";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";

const mocks = vi.hoisted(() => ({
  clipboardWriteText: vi.fn(async () => undefined),
  clipboardReadText: vi.fn(async () => "pasted text"),
}));

vi.mock("../lib/api", async () => {
  const actual = await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    clipboardWriteText: mocks.clipboardWriteText,
    clipboardReadText: mocks.clipboardReadText,
  };
});

import {
  clipboardCombo,
  clipboardDomGuard,
  clipboardKeymap,
  copySelection,
  cutSelection,
  pasteClipboard,
  selectionText,
} from "../lib/keymap";
// Pre-load the API module so that the dynamic import inside `lib/clipboard.ts` resolves
// immediately instead of during the assertion.
import "../lib/api";

/** Creates an editor with the clipboard extensions. */
function createEditor(doc = "hello world") {
  const view = new EditorView({
    state: EditorState.create({
      doc,
      extensions: [clipboardKeymap, clipboardDomGuard],
    }),
    parent: document.body,
  });
  return view;
}

/** Sends a keydown event to the editor. */
function press(
  view: EditorView,
  key: string,
  options: { ctrl?: boolean; meta?: boolean; shift?: boolean } = {},
) {
  view.contentDOM.dispatchEvent(
    new KeyboardEvent("keydown", {
      key,
      ctrlKey: options.ctrl ?? false,
      metaKey: options.meta ?? false,
      shiftKey: options.shift ?? false,
      bubbles: true,
      cancelable: true,
    }),
  );
}

describe("clipboard keymap", () => {
  beforeEach(() => {
    mocks.clipboardWriteText.mockClear();
    mocks.clipboardReadText.mockClear();
    mocks.clipboardReadText.mockResolvedValue("pasted text");
    Object.defineProperty(navigator, "clipboard", {
      value: undefined,
      configurable: true,
      writable: true,
    });
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("copies the selection", async () => {
    const view = createEditor();
    view.dispatch({ selection: { anchor: 0, head: 5 } });

    expect(copySelection(view)).toBe(true);

    await waitFor(() =>
      expect(mocks.clipboardWriteText).toHaveBeenCalledWith("hello"),
    );
    view.destroy();
  });

  it("copies the whole line when nothing is selected", async () => {
    const view = createEditor("first\nsecond");
    view.dispatch({ selection: { anchor: 3, head: 3 } });

    expect(selectionText(view)).toBe("first");
    expect(copySelection(view)).toBe(true);

    await waitFor(() =>
      expect(mocks.clipboardWriteText).toHaveBeenCalledWith("first"),
    );
    view.destroy();
  });

  it("cuts the selection and removes it from the document", async () => {
    const view = createEditor("hello world");
    view.dispatch({ selection: { anchor: 0, head: 5 } });

    expect(cutSelection(view)).toBe(true);

    await waitFor(() =>
      expect(mocks.clipboardWriteText).toHaveBeenCalledWith("hello"),
    );
    expect(view.state.doc.toString()).toBe(" world");
    view.destroy();
  });

  it("pastes the clipboard contents at the cursor", async () => {
    const view = createEditor("ab");
    view.dispatch({ selection: { anchor: 1, head: 1 } });

    expect(pasteClipboard(view)).toBe(true);

    await waitFor(() =>
      expect(view.state.doc.toString()).toBe("apasted textb"),
    );
    expect(mocks.clipboardReadText).toHaveBeenCalled();
    view.destroy();
  });

  it("replaces the selection when pasting", async () => {
    const view = createEditor("hello world");
    view.dispatch({ selection: { anchor: 0, head: 5 } });

    pasteClipboard(view);

    await waitFor(() =>
      expect(view.state.doc.toString()).toBe("pasted text world"),
    );
    view.destroy();
  });

  it("copies with Ctrl+C", async () => {
    const view = createEditor();
    view.dispatch({ selection: { anchor: 0, head: 5 } });

    press(view, "c", { ctrl: true });

    await waitFor(() =>
      expect(mocks.clipboardWriteText).toHaveBeenCalledWith("hello"),
    );
    view.destroy();
  });

  it("pastes with Ctrl+V", async () => {
    const view = createEditor("ab");
    view.dispatch({ selection: { anchor: 1, head: 1 } });

    press(view, "v", { ctrl: true });

    await waitFor(() =>
      expect(view.state.doc.toString()).toBe("apasted textb"),
    );
    view.destroy();
  });

  it("cuts with Ctrl+X", async () => {
    const view = createEditor("hello world");
    view.dispatch({ selection: { anchor: 0, head: 5 } });

    press(view, "x", { ctrl: true });

    await waitFor(() =>
      expect(mocks.clipboardWriteText).toHaveBeenCalledWith("hello"),
    );
    expect(view.state.doc.toString()).toBe(" world");
    view.destroy();
  });

  it("does not steal Ctrl+Shift+V (the VIM toggle)", () => {
    const view = createEditor();
    press(view, "v", { ctrl: true, shift: true });
    expect(view.state.doc.toString()).toBe("hello world");
    view.destroy();
  });

  it("recognises the platform combinations", () => {
    const makeEvent = (init: KeyboardEventInit) =>
      new KeyboardEvent("keydown", init);

    expect(clipboardCombo(makeEvent({ key: "c", ctrlKey: true }))).toBe(
      "copy",
    );
    expect(clipboardCombo(makeEvent({ key: "v", ctrlKey: true }))).toBe(
      "paste",
    );
    expect(clipboardCombo(makeEvent({ key: "x", ctrlKey: true }))).toBe(
      "cut",
    );
    expect(clipboardCombo(makeEvent({ key: "c" }))).toBeNull();
    expect(
      clipboardCombo(makeEvent({ key: "v", ctrlKey: true, shiftKey: true })),
    ).toBeNull();
  });

  it("exposes the Mod bindings for every platform", () => {
    const state = EditorState.create({ extensions: [clipboardKeymap] });
    const keys = state
      .facet(keymap)
      .flat()
      .map((binding) => binding.key);

    expect(keys).toContain("Mod-c");
    expect(keys).toContain("Mod-v");
    expect(keys).toContain("Mod-x");
    expect(keys).toContain("Mod-a");
  });
});
