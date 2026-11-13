/**
 * The dark CodeMirror theme of the application.
 */
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";

/** The editor chrome. */
export const sandboxTheme = EditorView.theme(
  {
    "&": {
      color: "#d7dce5",
      backgroundColor: "#11141b",
      height: "100%",
      fontSize: "13.5px",
    },
    ".cm-content": {
      caretColor: "#4ade80",
      fontFamily:
        '"JetBrains Mono", "Fira Code", "SF Mono", Menlo, Consolas, "Liberation Mono", monospace',
      padding: "10px 0",
    },
    ".cm-cursor, .cm-dropCursor": {
      borderLeftColor: "#4ade80",
      borderLeftWidth: "2px",
    },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
      backgroundColor: "#2b4a6b",
    },
    ".cm-panels": {
      backgroundColor: "#171b24",
      color: "#d7dce5",
    },
    ".cm-gutters": {
      backgroundColor: "#11141b",
      color: "#4a5468",
      border: "none",
    },
    ".cm-activeLine": { backgroundColor: "#161a23" },
    ".cm-activeLineGutter": { backgroundColor: "#161a23", color: "#8b95a8" },
    ".cm-selectionMatch": { backgroundColor: "#22344a" },
    ".cm-tooltip": {
      backgroundColor: "#1b1f2a",
      border: "1px solid #2a3040",
      color: "#d7dce5",
    },
    ".cm-tooltip-autocomplete ul li[aria-selected]": {
      backgroundColor: "#2b4a6b",
      color: "#ffffff",
    },
    ".cm-matchingBracket": {
      backgroundColor: "#2f4f3f",
      outline: "1px solid #4ade80",
    },
  },
  { dark: true },
);

/** The syntax highlighting colours. */
export const sandboxHighlightStyle = HighlightStyle.define([
  { tag: t.keyword, color: "#c792ea" },
  { tag: [t.name, t.deleted, t.character, t.propertyName, t.macroName], color: "#d7dce5" },
  { tag: [t.function(t.variableName), t.labelName], color: "#82aaff" },
  { tag: [t.color, t.constant(t.name), t.standard(t.name)], color: "#f78c6c" },
  { tag: [t.definition(t.name), t.separator], color: "#d7dce5" },
  { tag: [t.className, t.changed, t.annotation, t.modifier, t.self, t.namespace], color: "#ffcb6b" },
  { tag: [t.typeName, t.operatorKeyword, t.operator], color: "#ffcb6b" },
  { tag: [t.string, t.special(t.string), t.inserted], color: "#c3e88d" },
  { tag: [t.number, t.bool, t.null, t.atom], color: "#f78c6c" },
  { tag: [t.regexp, t.escape], color: "#89ddff" },
  { tag: [t.meta, t.comment], color: "#5c6773", fontStyle: "italic" },
  { tag: t.strong, fontWeight: "bold" },
  { tag: t.emphasis, fontStyle: "italic" },
  { tag: t.link, color: "#89ddff", textDecoration: "underline" },
  { tag: t.heading, color: "#82aaff", fontWeight: "bold" },
  { tag: [t.quote, t.list], color: "#c3e88d" },
  { tag: t.invalid, color: "#ff5370" },
]);

/** The syntax highlighting extension. */
export const sandboxHighlighting = syntaxHighlighting(sandboxHighlightStyle);
