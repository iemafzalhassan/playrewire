<!-- A CodeMirror 6 editor for Go in the ink theme. Ctrl/Cmd+Enter runs the
     code. Tab indents; Esc then Tab moves focus on (CodeMirror's escape hatch). -->
<script lang="ts">
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { go } from "@codemirror/lang-go";
  import { bracketMatching, indentOnInput, indentUnit } from "@codemirror/language";
  import { EditorState } from "@codemirror/state";
  import { drawSelection, EditorView, highlightActiveLine, highlightActiveLineGutter, keymap, lineNumbers } from "@codemirror/view";
  import { onMount } from "svelte";
  import { errorLineField, inkTheme, setErrorLine } from "./inkTheme";

  let {
    doc,
    label,
    errorLine = null,
    onChange,
    onRun,
  }: {
    doc: string;
    label: string;
    /** 1-based user line to mark, or null. A new object re-marks the same line. */
    errorLine?: { line: number } | null;
    onChange: (value: string) => void;
    onRun: () => void;
  } = $props();

  let host: HTMLDivElement;
  let view: EditorView | null = null;

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc,
        extensions: [
          lineNumbers(),
          highlightActiveLineGutter(),
          history(),
          drawSelection(),
          indentOnInput(),
          bracketMatching(),
          highlightActiveLine(),
          indentUnit.of("    "),
          EditorState.tabSize.of(4),
          keymap.of([
            { key: "Mod-Enter", run: () => (onRun(), true) },
            indentWithTab,
            ...defaultKeymap,
            ...historyKeymap,
          ]),
          go(),
          inkTheme,
          errorLineField,
          EditorView.contentAttributes.of({ "aria-label": label, spellcheck: "false", autocapitalize: "off", autocorrect: "off" }),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) onChange(u.state.doc.toString());
          }),
        ],
      }),
    });
    return () => {
      view?.destroy();
      view = null;
    };
  });

  $effect(() => {
    const mark = errorLine;
    view?.dispatch({ effects: setErrorLine.of(mark?.line ?? null) });
  });

  export function focus() {
    view?.focus();
  }
</script>

<div class="editor" bind:this={host}></div>

<style>
  .editor {
    min-height: 10rem;
  }
  .editor :global(.cm-editor) {
    border-radius: var(--radius-wobble-a);
  }
</style>
