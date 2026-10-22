// The Type step as a pure state machine: the learner types over a faint
// "ghost" of the code. Nothing here records or sends keys; the component only
// keeps the current state in memory and reports whether the step finished.

export type GhostState = { typed: string; correctUpTo: number; errorAt: number | null; done: boolean };

/** Shown when someone tries to paste or drop code into the Type step. */
export const PASTE_HINT = "Type it out, it's how your fingers learn.";

export const initialGhost = (): GhostState => ({ typed: "", correctUpTo: 0, errorAt: null, done: false });

/**
 * CRLF (and lone CR) become LF, and trailing blank space is dropped: the last
 * line ends where its code ends, so nobody has to type a final Enter.
 */
export function normalizeGhost(ghost: string): string {
  return ghost.replace(/\r\n?/g, "\n").trimEnd();
}

const isIndent = (c: string | undefined) => c === " " || c === "\t";

function at(ghost: string, pos: number): GhostState {
  return { typed: ghost.slice(0, pos), correctUpTo: pos, errorAt: null, done: pos >= ghost.length };
}

function wrong(ghost: string, pos: number, key: string): GhostState {
  const shown = key === "Enter" ? "\n" : key === "Tab" ? "\t" : key;
  return { typed: ghost.slice(0, pos) + shown, correctUpTo: pos, errorAt: pos, done: false };
}

/** Skips spaces and tabs from `pos`. */
function pastIndent(ghost: string, pos: number): number {
  while (isIndent(ghost[pos])) pos++;
  return pos;
}

/**
 * One key press. `key` is a printable character, 'Enter', 'Backspace' or
 * 'Tab'; anything else (Shift, arrows...) changes nothing.
 *
 * - A wrong key sets `errorAt`; until Backspace clears it, other keys are ignored.
 * - Enter at the end of a line also skips the next line's leading indentation.
 * - Tab matches a run of spaces or tabs.
 * - Backspace steps back one character, or over a whole indentation at once.
 */
export function ghostKey(rawGhost: string, s: GhostState, key: string): GhostState {
  const ghost = normalizeGhost(rawGhost);
  if (s.done) return s;
  const pos = s.correctUpTo;

  if (key === "Backspace") {
    if (s.errorAt !== null) return at(ghost, pos);
    if (pos === 0) return s;
    const lineStart = ghost.lastIndexOf("\n", pos - 1) + 1;
    const indentOnly = pos > lineStart && /^[ \t]+$/.test(ghost.slice(lineStart, pos));
    return at(ghost, indentOnly ? lineStart : pos - 1);
  }

  if (s.errorAt !== null) return s; // fix the mistake first

  if (key === "Enter") {
    // Stray spaces before the line break don't need typing either.
    const nl = pastIndent(ghost, pos);
    if (ghost[nl] !== "\n") return wrong(ghost, pos, key);
    return at(ghost, pastIndent(ghost, nl + 1));
  }

  if (key === "Tab") {
    if (!isIndent(ghost[pos])) return wrong(ghost, pos, key);
    return at(ghost, pastIndent(ghost, pos));
  }

  if ([...key].length !== 1) return s; // not a typing key
  if (ghost[pos] === key) return at(ghost, pos + 1);
  return wrong(ghost, pos, key);
}
