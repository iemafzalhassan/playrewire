// Pure helpers for the Drill step: wrapping body-of-main code into a program,
// checking output, the round's typing speed and the learner's local day.

/** Same wrapper as tools/packcheck (schema.go): user line 1 is file line 8. */
const WRAPPER_HEAD = 'package main\n\nimport "fmt"\n\nvar _ = fmt.Sprint\n\nfunc main() {\n';
const WRAPPER_TAIL = "\n}\n";

/**
 * Body-of-main code goes inside the fixed wrapper (lineOffset 7). Whole-file
 * code (ruling R14) is already a complete `package main` program and runs
 * unchanged (lineOffset 0).
 */
export function wrapMain(code: string, wholeFile = false): { source: string; lineOffset: 0 | 7 } {
  if (wholeFile) return { source: code, lineOffset: 0 };
  return { source: WRAPPER_HEAD + code.replace(/\n+$/, "") + WRAPPER_TAIL, lineOffset: 7 };
}

/** CRLF to LF, trailing whitespace off every line, trailing blank lines dropped. */
function outputLines(s: string): string[] {
  const lines = s
    .replace(/\r\n?/g, "\n")
    .split("\n")
    .map((l) => l.replace(/\s+$/, ""));
  while (lines.length && lines[lines.length - 1] === "") lines.pop();
  return lines;
}

/**
 * Compares a run's stdout with the drill's expected output. Spaces at line
 * ends, CRLF and a missing final newline never fail a drill. `diffLine` is the
 * first line (1-based) that differs.
 */
export function checkDrill(expected: string, rawStdout: string): { pass: boolean; diffLine: number | null } {
  const want = outputLines(expected);
  const got = outputLines(rawStdout);
  const n = Math.max(want.length, got.length);
  for (let i = 0; i < n; i++) {
    if (want[i] !== got[i]) return { pass: false, diffLine: i + 1 };
  }
  return { pass: true, diffLine: null };
}

/** Characters per minute for the round: what the learner added beyond the starter. */
export function cpm(code: string, starter: string, ms: number): number {
  if (ms <= 0) return 0;
  const added = Math.max(0, code.trim().length - starter.trim().length);
  return Math.round(added / (ms / 60_000));
}

/** The learner's own calendar day, YYYY-MM-DD, in local time (never UTC). */
export function localDate(d: Date = new Date()): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

/** m:ss for the round timer. */
export function clock(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}
