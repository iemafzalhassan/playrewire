// CodeMirror 6 in Paper & Ink: card paper, ink text, an accent caret, and
// syntax colours taken from the design tokens (CSS variables), so the editor
// follows tokens.json like everything else. Also the error-line marker.
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { StateEffect, StateField, type Extension } from "@codemirror/state";
import { Decoration, EditorView, WidgetType, type DecorationSet } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";

const theme = EditorView.theme({
  "&": {
    color: "var(--ink)",
    backgroundColor: "transparent",
    fontSize: "1.05rem",
  },
  ".cm-scroller": {
    fontFamily: "var(--font-code)",
    lineHeight: "1.65",
  },
  ".cm-content": {
    caretColor: "var(--accent)",
    padding: "var(--space-3) 0",
    minHeight: "9.5rem",
  },
  "&.cm-focused": { outline: "none" },
  ".cm-cursor, .cm-dropCursor": { borderLeft: "2.5px solid var(--accent)" },
  "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
    { backgroundColor: "color-mix(in srgb, var(--highlight) 60%, transparent)" },
  ".cm-activeLine": { backgroundColor: "color-mix(in srgb, var(--highlight) 16%, transparent)" },
  ".cm-gutters": {
    backgroundColor: "transparent",
    color: "var(--pencil)",
    border: "none",
    borderRight: "2px dotted color-mix(in srgb, var(--pencil) 70%, transparent)",
    fontFamily: "var(--font-code)",
  },
  ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--ink-soft)" },
  ".cm-lineNumbers .cm-gutterElement": { padding: "0 var(--space-2) 0 var(--space-3)" },
  ".cm-matchingBracket, &.cm-focused .cm-matchingBracket": {
    backgroundColor: "transparent",
    outline: "1.5px solid var(--pencil)",
    borderRadius: "3px",
  },
  // The line Go complained about: a wavy accent underline and a pen mark.
  ".cm-ink-error": {
    backgroundColor: "color-mix(in srgb, var(--accent) 9%, transparent)",
    textDecoration: "underline wavy var(--accent)",
    textDecorationSkipInk: "none",
    textUnderlineOffset: "5px",
  },
  ".cm-ink-mark": {
    marginLeft: "var(--space-3)",
    fontFamily: "var(--font-display)",
    fontWeight: "700",
    fontSize: "1.2rem",
    color: "var(--accent-deep)",
    textDecoration: "none",
    display: "inline-block",
    transform: "rotate(-3deg)",
  },
});

const highlight = HighlightStyle.define([
  { tag: [t.keyword, t.controlKeyword, t.definitionKeyword, t.moduleKeyword], color: "var(--accent-deep)", fontWeight: "700" },
  { tag: [t.string, t.special(t.string), t.character], color: "var(--ok)" },
  { tag: [t.number, t.bool, t.null], color: "var(--focus)" },
  { tag: [t.comment, t.lineComment, t.blockComment], color: "var(--pencil)", fontStyle: "italic" },
  { tag: [t.typeName, t.standard(t.typeName)], color: "var(--ink-soft)", fontWeight: "700" },
  { tag: [t.function(t.variableName), t.function(t.propertyName)], color: "var(--ink)" },
  { tag: [t.operator, t.punctuation, t.bracket], color: "var(--ink-soft)" },
]);

export const inkTheme: Extension = [theme, syntaxHighlighting(highlight)];

/** Mark one user line (1-based), or clear the mark with null. */
export const setErrorLine = StateEffect.define<number | null>();

class PenMark extends WidgetType {
  constructor(readonly line: number) {
    super();
  }
  eq(o: PenMark) {
    return o.line === this.line;
  }
  toDOM() {
    const s = document.createElement("span");
    s.className = "cm-ink-mark";
    s.setAttribute("aria-hidden", "true");
    s.textContent = "✗ here";
    return s;
  }
}

/** The marked line; any edit clears it, since the line may have moved. */
export const errorLineField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(deco, tr) {
    for (const e of tr.effects) {
      if (!e.is(setErrorLine)) continue;
      if (e.value === null || e.value < 1 || e.value > tr.state.doc.lines) return Decoration.none;
      const line = tr.state.doc.line(e.value);
      return Decoration.set([
        Decoration.line({ class: "cm-ink-error" }).range(line.from),
        Decoration.widget({ widget: new PenMark(e.value), side: 1 }).range(line.to),
      ]);
    }
    return tr.docChanged ? Decoration.none : deco;
  },
  provide: (f) => EditorView.decorations.from(f),
});
