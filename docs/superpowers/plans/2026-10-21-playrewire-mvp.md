# PlayRewire MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the first playable Rewire: a Linux desktop app and a web app where a beginner works through an interleaved path of Silicon Quarter and Go Academy chunks (See → Type → Drill → Use), runs real Go on their own device, earns XP and streaks, hacks two terminals in a hand-inked 3D street, and can ask an on-device AI mentor for a nudge.

**Architecture:** A monorepo. Pure game rules (progress, streaks, scheduling, friendly errors, city-event parsing, mission checks) live in one Rust crate, `rewire-core`. It's compiled to WASM and used by the single Svelte UI on **both** desktop and web. The two platforms differ only behind a TypeScript `Backend` interface: desktop (Tauri v2) runs code with the native Go toolchain inside bubblewrap and stores progress in SQLite; web runs code in a Yaegi-in-WASM worker and stores progress in `localStorage`. Lessons are YAML authored under `content/`, validated and bundled to JSON by a Go CLI (`packcheck`). The 3D street is a Bevy app compiled to WASM that renders into a `<canvas>` in the UI.

**Tech Stack:** Rust 1.99 (Arch package), Go 1.27.2, Node 26 + pnpm 12, Tauri v2, Bevy (latest stable, pinned in `Cargo.lock`), Svelte 5 + Vite + TypeScript + CodeMirror 6, Vitest, rusqlite, reqwest, keyring, wasm-bindgen, Yaegi (latest release, pinned in `go.mod`), bubblewrap 0.13.

**Spec:** `docs/superpowers/specs/2026-10-21-playrewire-design.md`

**Scope note:** This plan covers design Phase 0 (spikes) and Phase 1 (MVP). Phases 2–6 (Keycloak, sync, squads, exams, mobile, packages beyond Linux) get their own plans. The **mobile performance spike** (Bevy on mid-range Android and iPhone) needs the owner's devices and SDKs, so it's listed under "Deferred" and doesn't block this plan.

## Global Constraints

- **Learning code never runs on our servers.** Desktop runs it with the local Go toolchain in a sandbox; web runs it in the browser. (Spec §3, §7.1)
- Sandbox defaults (spec §7.3, values pinned here): **no network**, compile wall limit **60 s**, run wall limit **10 s**, `RLIMIT_CPU` **10 s**, `RLIMIT_FSIZE` **10 MiB**, combined stdout+stderr cap **65 536 bytes**.
- Pinned Go toolchain version for the lab: **`go1.27.2`**.
- Every chunk has all four steps in order: **See → Type → Drill → Use** (Use is optional for see-only foundation chunks). Each drill set has **at least 5** drills. (Spec §4.4)
- Help fades by mission encounter: **1st = fill one blank, 2nd = finish half the function, 3rd and later = blank editor + hint button.** (Spec §4.5)
- Rewards are cosmetic only. Streak freezes are **earned through play, never bought.** (Spec §5.4)
- Visual tokens (spec §6.1): paper **`#f6f0e4`**, ink **`#2a2620`**, accent **`#e2572b`**, card **`#fffdf7`**. Display font is handwritten (**Caveat**). Body text uses a highly legible font (**Atkinson Hyperlegible**, my call). Code uses **JetBrains Mono**. The dyslexia option uses **OpenDyslexic**.
- Accessibility from day one: `prefers-reduced-motion` respected plus an in-app toggle, every control reachable by keyboard, all icons labeled, pass/fail never shown by color alone (✓ / ✗ glyphs too), text scales with the system setting. (Spec §6.2)
- Copy tone: plain words, short sentences, no shaming. No red stack trace without a friendly explanation above it. (Spec §3 anti-goals)
- No analytics leave the device in the MVP. Progress events are written to a local **outbox** for Phase 2 sync. No keystroke logging. (Spec §7.8)
- AI mentor: never writes the full solution. It runs from the player's device with the player's key. Keys are stored only in the OS keychain. (Spec §5.6)
- Licenses: code Apache-2.0, lesson content CC BY-SA 4.0. Every commit is signed off (`git commit -s`) and ends with the `Co-Authored-By` trailer.
- No foundation or CNCF references anywhere in the repo. (Owner decision)

## Review Focus

1. **A learner writes an infinite loop or prints forever.** Expect the run to stop at 10 s (or 64 KiB of output) and a friendly "Never-ending loop" or "Too much output" card, not a frozen app. Pinned in Task 9 (native) and Task 4 (web).
2. **Go isn't installed and the device is offline on first launch.** See and Type steps still work. Run shows "Rewire needs to download Go once (about 70 MB). Connect to the internet and tap Retry." Pinned in Task 10.
3. **The player's clock jumps backwards, or they cross midnight or a timezone mid-session.** The streak never drops because of a clock change, and a day is the player's *local* calendar date at the time of the step. Pinned in Task 6.
4. **Pasting or wrong keys in the Type step** (Tab vs spaces, CRLF, pasted text). Expect pasted text to be ignored with a gentle "Type it out, it's how your fingers learn" hint, leading indentation auto-filled after Enter, and CRLF normalized. Pinned in Task 14.
5. **The mentor provider is down, the key is wrong, or the reply is slow.** Expect the panel to fall back to the lesson's built-in hint ladder within 20 s with "The mentor couldn't connect, so here's a built-in hint," and no crash. Pinned in Task 11.

---

## File Structure

```
Cargo.toml                         workspace (crates/*, apps/desktop/src-tauri)
justfile                           one entry point: just test | just dev-web | just dev-desktop | just bundle
go.work                            tools/packcheck, lab-wasm
pnpm-workspace.yaml                apps/web
design/tokens.json                 single source of design tokens
content/go/city/city.go            "city" stub: SetGreen, Lamp, Say → @city JSON lines
content/go/path.yaml               the interleaved learning order (chunk refs)
content/go/packs/<pack>/pack.yaml  pack metadata
content/go/packs/<pack>/chunks/*.yaml
content/go/packs/<pack>/missions/*.yaml
tools/packcheck/                   Go CLI: validate packs, run solutions, emit bundle JSON
lab-wasm/labcore/                  Go: Yaegi-based Run(files) (native-testable)
lab-wasm/cmd/lab/                  Go js/wasm entry exposing rewireRun to JS
crates/rewire-core/                pure Rust rules (no I/O)
crates/rewire-core-wasm/           wasm-bindgen wrapper of rewire-core
crates/rewire-store/               SQLite persistence + outbox
crates/rewire-lab/                 native Go runner (bwrap) + toolchain manager
crates/rewire-mentor/              providers, presets, AG-UI events
crates/rewire-world/               Bevy hand-inked street (wasm cdylib)
apps/web/                          Svelte 5 UI (web app + desktop frontend)
apps/desktop/src-tauri/            Tauri v2 shell and commands
docs/spikes/                       spike reports
```

---

## Milestone 0: Foundation and spikes

### Task 1: Monorepo skeleton and CI

**Files:**
- Create: `Cargo.toml`, `justfile`, `go.work`, `pnpm-workspace.yaml`, `rust-toolchain.toml` (channel `1.99`, targets `wasm32-unknown-unknown`), `.github/workflows/ci.yml`, `.editorconfig`
- Create: `crates/rewire-core/{Cargo.toml,src/lib.rs}` (an empty crate with one smoke test)

**Interfaces:** Produces the commands every later task uses: `just test` (runs `cargo test --workspace`, `go test ./...` in each go.work module, `pnpm -r test`), `just wasm` (builds both wasm crates), `just content` (runs packcheck build).

- [ ] **Step 1: Prerequisites.** Ask the owner to run `! sudo pacman -S --needed rust-wasm`, then run `cargo install wasm-bindgen-cli tauri-cli just`. Verify: `rustc --print target-libdir --target wasm32-unknown-unknown` prints an existing path.
- [ ] **Step 2: Write `crates/rewire-core/src/lib.rs` with `#[test] fn smoke() { assert_eq!(2 + 2, 4) }`.** Run `just test`. Expected: 1 Rust test passes, Go and pnpm report "no packages/tests" without failing.
- [ ] **Step 3: CI workflow** on `ubuntu-24.04`: install Rust 1.99 + the wasm target, Go 1.27.2, Node 26 + pnpm, `libwebkit2gtk-4.1-dev librsvg2-dev bubblewrap`, then run `just test`. Jobs: `test`, plus `content` (from Task 5 on).
- [ ] **Step 4: Commit** `chore: monorepo skeleton and CI`.

### Task 2: Spike: Bevy WASM in the desktop webview, with an ink look

Time-boxed to **1 day**. This tests whether the risky rendering path is viable before we build on it. The code lands in `crates/rewire-world` as the starting point for Task 16 **only if it passes**.

**Files:**
- Create: `crates/rewire-world/{Cargo.toml,src/lib.rs,src/ink.rs,src/ink.wgsl}`, `apps/web/spike/world.html`
- Create: `docs/spikes/2026-10-bevy-webview-ink.md`

- [ ] **Step 1:** Build a minimal scene: a ground plane, a street of 6 box buildings, 5 traffic-light boxes, a capsule player and an orbit camera. Shading is flat/toon (2 bands) plus a screen-space **edge-detect outline post-process** (depth + normal) in ink `#2a2620`. Leave the background transparent and put the CSS paper texture behind the canvas. Use Bevy's `webgl2` feature.
- [ ] **Step 2:** `cargo build -p rewire-world --target wasm32-unknown-unknown --release && wasm-bindgen --target web --out-dir apps/web/public/world target/wasm32-unknown-unknown/release/rewire_world.wasm`. Log the average FPS every 5 s to the console with `FrameTimeDiagnosticsPlugin`.
- [ ] **Step 3: Measure** with Playwright: Chromium and WebKit (Playwright's WebKit on Linux is WebKitGTK-based, the same engine family as the Tauri webview) at 1280×720 for 60 s, then once inside `cargo tauri dev` by hand.
- [ ] **Step 4: Pass criteria.** WebKit ≥ **30 fps** sustained, Chromium ≥ **55 fps**, gzipped wasm ≤ **15 MB**. Write the numbers, screenshots and a verdict to the spike doc.
- [ ] **Step 5:** If it **fails**, stop and bring the owner two options: (a) desktop renders Bevy in a native window next to the webview, (b) cut scene complexity. Don't continue to Task 16 without a decision.
- [ ] **Step 6: Commit** `spike(world): bevy webgl2 in webview with ink outline` (include the report).

### Task 3: Yaegi lab core (Go, native-testable)

**Files:**
- Create: `lab-wasm/go.mod` (module `github.com/iemafzalhassan/playrewire/lab-wasm`, Yaegi pinned), `lab-wasm/labcore/run.go`, `lab-wasm/labcore/symbols.go`, `lab-wasm/labcore/run_test.go`
- Create: `content/go/city/city.go`

**Interfaces:**
- Produces (Go): `labcore.Run(ctx context.Context, files []File) Result`, with `type File struct{ Name, Source string }` and `type Result struct{ Stdout, Stderr string; ExitCode int; TimedOut, Truncated bool }`. Files are evaluated in order (city stub, mission setup, learner program).
- Produces (city contract, used by every runtime): `SetGreen(i, delaySec int)`, `Lamp(i int, on bool)` and `Say(msg string)` each print one line `@city {"op":"green","i":0,"delay":0}` / `{"op":"lamp","i":0,"on":true}` / `{"op":"say","text":"…"}`.

- [ ] **Step 1: Write the failing tests** in `run_test.go`:
  - `TestHello`: running `wrap("fmt.Println(\"hi\")")` gives `Stdout == "hi\n"`, `ExitCode == 0`.
  - `TestCityEvents`: city.go plus a loop calling `SetGreen(i, i*2)` for `i<3` gives Stdout containing exactly three `@city` lines with delays 0, 2, 4.
  - `TestCompileErrorHasPosition`: `x := 1` gives Stderr matching `^\d+:\d+: .*declared and not used`.
  - `TestTimeout`: `for {}` under a 200 ms context gives `TimedOut == true`.
  - `TestOutputCap`: printing in an endless loop gives `Truncated == true` and `len(Stdout) <= 65536`.
  - `TestNoForbiddenPackages`: `import "os"` followed by `os.ReadFile("/etc/passwd")` gives a non-empty Stderr and no file contents in Stdout.
- [ ] **Step 2:** `cd lab-wasm && go test ./labcore/...`. Expected: FAIL (undefined: Run).
- [ ] **Step 3: Implement `Run`.** Use `interp.New(interp.Options{Stdout, Stderr})` with a capped writer, and `EvalWithContext` per file. Make `symbols.go` an **allow-list** filtered from `stdlib.Symbols`: `fmt strings strconv math math/rand sort slices maps errors unicode unicode/utf8 time bytes container/list container/heap`. Never `os`, `net`, `syscall`, `unsafe`. Have `ExitCode` be 1 on any error and 2 on timeout.
- [ ] **Step 4:** `go test ./labcore/...`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(lab-wasm): yaegi lab core with allow-listed stdlib`.

### Task 4: Yaegi in the browser (wasm entry and worker contract)

**Files:**
- Create: `lab-wasm/cmd/lab/main.go` (`//go:build js && wasm`), `apps/web/src/lab/worker.ts`, `apps/web/src/lab/webRunner.ts`, `apps/web/src/lab/webRunner.test.ts`
- Create: `docs/spikes/2026-10-yaegi-wasm.md`

**Interfaces:**
- Consumes: `labcore.Run` (Task 3).
- Produces (TS): `webRun(files: SourceFile[], limits?: {wallMs?: number}): Promise<RawRunResult>`, where `type SourceFile = {name: string; source: string}` and `type RawRunResult = {stdout: string; stderr: string; exitCode: number | null; timedOut: boolean; truncated: boolean; durationMs: number}`. This is the **same shape** the desktop backend returns (Task 13).

- [ ] **Step 1:** In `main.go`, register a global `rewireRun(filesJSON) → Promise<resultJSON>` that calls `labcore.Run` with a background context (the worker owns the timeout). Build: `GOOS=js GOARCH=wasm go build -trimpath -ldflags=-s -o apps/web/public/lab/lab.wasm ./cmd/lab`, then copy `$(go env GOROOT)/lib/wasm/wasm_exec.js` next to it.
- [ ] **Step 2: Write the failing tests** (Vitest in a Node environment, loading the built wasm via `wasm_exec.js` inside a `worker_threads` worker):
  - `runs hello`: stdout is `"hi\n"`.
  - `kills an infinite loop at wallMs`: with `{wallMs: 1000}`, `timedOut === true` and the call resolves in under 2000 ms. The worker is terminated and recreated.
  - `next run after a timeout still works`.
- [ ] **Step 3: Implement** `worker.ts` (load the wasm once, answer `{id, files}` messages) and `webRunner.ts`, which keeps one warm worker, applies the wall timer (default **10 000 ms**), and calls `terminate()` and respawns on timeout.
- [ ] **Step 4:** `pnpm --filter web test webRunner`. Expected: PASS.
- [ ] **Step 5: Spike report.** Record the wasm gzipped size and the cold start (first `webRun` hello) in Chromium and WebKit via Playwright. Pass: gzipped ≤ **12 MB**, cold start ≤ **3 s**. If it fails, trim the allow-list and re-measure before escalating.
- [ ] **Step 6: Commit** `feat(lab-wasm): browser runner via worker with wall-clock kill`.

---

## Milestone 1: Content

### Task 5: Lesson-pack schema and the `packcheck` CLI

**Files:**
- Create: `tools/packcheck/{go.mod,main.go,schema.go,validate.go,runner.go,bundle.go}`, `tools/packcheck/*_test.go`, `tools/packcheck/testdata/{good,bad}/…`
- Create: `apps/web/src/content/types.ts` (TypeScript mirror of the bundle, plus a `zod` schema `BundleSchema`)

**Interfaces:**
- Chunk YAML fields: `id, concept, title, see{scene, caption}, type?{ghost, tips[{match,text}]}, drills[]{prompt, starter?, solution, expect_stdout}, hints[3], fix_it?{broken, solution, expect_stdout}, mission?`. `fix_it.broken` must **fail** (compile error or wrong stdout) and `fix_it.solution` must pass.
- Mission YAML fields: `id, concept, terminal, title, setup, levels{fill_blank{template,solution}, half_function{template,solution}, blank_with_hints{template,solution}}, expect_events[]`.
- `path.yaml`: an ordered list of `{pack, chunk}`.
- **Wrapper (exact).** Drill and mission code is the body of `main`, and user line 1 is file line 8 (`lineOffset = 7`):
  ```go
  package main

  import "fmt"

  var _ = fmt.Sprint

  func main() {
  {code}
  }
  ```
- Produces the bundle `apps/web/public/content/go.json`: `{version, language:"go", wrapperLineOffset:7, cityStub, path[], packs[{id,title,district,chunks[],missions[]}]}`, with camelCase keys.

- [ ] **Step 1: Write the failing tests:**
  - `TestValidPackBundles`: `testdata/good` builds, and the bundle has `wrapperLineOffset == 7`.
  - `TestRejectsFewerThanFiveDrills` → error mentions `at least 5 drills`.
  - `TestRejectsWrongSolution`: a solution whose stdout ≠ `expect_stdout` → error names the chunk id and drill index.
  - `TestRejectsGhostThatDoesNotCompile`.
  - `TestMissionEventsMustMatch`: a solution's `@city` events must equal `expect_events` in order.
  - `TestPathReferencesExist`: an unknown `{pack, chunk}` in `path.yaml` is an error.
  - `TestHintsExactlyThree`.
  - `TestFixItBrokenMustFail`: a `fix_it.broken` that already prints `expect_stdout` is an error.
- [ ] **Step 2:** `cd tools/packcheck && go test ./...`. Expected: FAIL.
- [ ] **Step 3: Implement.** `packcheck validate <dir>` and `packcheck build <dir> -o <file>`. Run solutions by writing temp files (`go.mod`, `city.go`, the setup file, the wrapped `main.go`) and calling `go run .` with a 20 s timeout. Packcheck runs in CI and on the author's machine and is never shipped to players.
- [ ] **Step 4:** Tests PASS. Add a Vitest test `bundle schema accepts packcheck output` that parses the `testdata/good` bundle with `BundleSchema`.
- [ ] **Step 5:** Wire up `just content` and add a CI `content` job (`packcheck validate content/go`).
- [ ] **Step 6: Commit** `feat(packcheck): lesson pack schema, validation and bundling`.

---

## Milestone 2: Core rules, content and native services

### Task 6: `rewire-core` progress engine

**Files:**
- Create: `crates/rewire-core/src/{progress.rs,streak.rs,schedule.rs,types.rs}`, tests inline (`#[cfg(test)]`)

**Interfaces (Produces):**
```rust
pub enum Step { See, Type, Drill, Use }                          // serde: "see"|"type"|"drill"|"use"
pub struct StepRecord { pub chunk_id: String, pub concept: String, pub step: Step,
    pub mission_id: Option<String>, pub success: bool, pub first_try: bool,
    pub hints_used: u32, pub local_date: chrono::NaiveDate }
pub struct ProgressState { pub xp: u64, pub streak: StreakState,
    pub concepts: BTreeMap<String, ConceptState>, pub completed: BTreeSet<String>, // "chunk_id:step"
    pub mission_encounters: BTreeMap<String, u32> }
pub struct StreakState { pub current: u32, pub best: u32, pub last_day: Option<NaiveDate>, pub freezes: u8 }
pub struct ConceptState { pub box_level: u8, pub due: NaiveDate, pub successes: u32, pub fails: u32 }
pub enum Reward { Xp(u64), StreakExtended(u32), FreezeUsed, FreezeEarned, LevelUp(u32) }
pub enum HelpLevel { FillBlank, HalfFunction, BlankWithHints }   // serde snake_case
pub fn apply(state: &ProgressState, rec: &StepRecord) -> (ProgressState, Vec<Reward>);
pub fn level(xp: u64) -> u32;                                   // 1 + xp / 250
pub fn due_concepts(state: &ProgressState, today: NaiveDate) -> Vec<String>; // sorted by due, then id
pub fn help_level(state: &ProgressState, mission_id: &str) -> HelpLevel;
```
All types derive `Serialize, Deserialize, Clone, Debug, PartialEq, Default` where possible. `ProgressState` JSON is the persistence format.

**Rules (pinned, my calls within spec §4–5):**
- **XP on the first completion** of a `chunk:step`: See **5**, Type **10**, Drill **20**, Use **30**, plus a **+10** first-try bonus on Drill and Use. Repeats give **2**. Failures give 0.
- **Streak:** a day *qualifies* on a successful Drill or Use. For a qualifying day `d`:
  - `last_day == None` → 1
  - `d == last` → no change
  - `d == last + 1` → +1
  - `d == last + 2` and `freezes > 0` → spend a freeze, then +1
  - an earlier `d` (clock went backwards) → **no change, never decrease**
  - otherwise → 1
  
  Earn **1 freeze** each time `current` reaches a multiple of **7**, holding at most **2**. `best = max(best, current)`.
- **Schedule (Leitner, the MVP stand-in for FSRS):** intervals in days are `[1, 2, 4, 7, 14, 30]` for boxes 0–5. A Drill or Use outcome updates its concept: success with `first_try && hints_used == 0` → box +1 (max 5); success otherwise → same box; fail → box 0. `due = local_date + interval[box]`.
- **Help level:** count `mission_encounters[mission_id]` on each successful Use. A count of 0 gives FillBlank, 1 gives HalfFunction, and ≥ 2 gives BlankWithHints.

- [ ] **Step 1: Write the failing tests:** `xp_first_completion_values`, `xp_repeat_is_two`, `first_try_bonus_only_drill_and_use`, `streak_consecutive_days`, `streak_same_day_no_change`, `streak_gap_uses_freeze`, `streak_gap_without_freeze_resets`, `streak_clock_backwards_never_decreases` (Review Focus 3), `freeze_earned_at_7_and_capped_at_2`, `schedule_box_moves`, `due_concepts_sorted`, `help_level_fades_after_encounters`, `level_formula` (`level(0)==1`, `level(249)==1`, `level(250)==2`), `state_json_roundtrip`.
- [ ] **Step 2:** `cargo test -p rewire-core`. Expected: FAIL.
- [ ] **Step 3:** Implement the rules exactly as listed.
- [ ] **Step 4:** `cargo test -p rewire-core`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(core): xp, streaks with freezes, leitner schedule, help fading`.

> **Learning-mode note:** the streak rules are the owner's product-feel decision. During execution, offer the owner the `apply_streak` function body (5–10 lines) to write themselves.

### Task 7: Friendly errors, city output and mission checks (`rewire-core`)

**Files:**
- Create: `crates/rewire-core/src/{explain.rs,city.rs,mission.rs}`, `crates/rewire-core/tests/fixtures/{gc_*.txt,yaegi_*.txt}`

**Interfaces (Produces):**
```rust
pub struct FriendlyError { pub line: Option<u32>, pub col: Option<u32>, pub title: String, pub plain: String, pub raw: String }
pub fn explain_go_errors(stderr: &str, line_offset: u32, timed_out: bool, truncated: bool) -> Vec<FriendlyError>;
pub enum CityEvent { Green { i: u32, delay: u32 }, Lamp { i: u32, on: bool }, Say { text: String } } // serde tag "op", lowercase
pub fn parse_city_output(stdout: &str) -> (String /* stdout without @city lines */, Vec<CityEvent>);
pub enum MissionVerdict { Pass, WrongEvents { first_mismatch: usize }, NoEvents }
pub fn check_mission(expected: &[CityEvent], actual: &[CityEvent]) -> MissionVerdict;
```

**Exact copy** (titles / plain text, with `{x}` captured):

| Pattern | Title | Plain |
|---|---|---|
| `declared and not used: (\w+)` | Unused box | You made a box called `{x}` but never used it. Use it, or delete it. |
| `undefined: (\w+)` | Unknown name | Go doesn't know what `{x}` is yet. Check the spelling, or create it first with `{x} := ...`. |
| `syntax error: unexpected (.+)` | Something's out of place | Go got confused near `{x}`. Look for a missing `{`, `}` or `)` just before here. |
| `missing return` | Missing answer | This function promises to give back a value, but one path doesn't. Add a `return`. |
| `cannot use (\S+) .* as (\w+) value` | Wrong type | You gave `{1}` where Go expected a `{2}`. |
| `"(\S+)" imported and not used` | Unused import | You brought in `{x}` but didn't use it. Remove that import line. |
| `index out of range \[(\d+)\] with length (\d+)` | Off the end | You asked for item {1}, but there are only {2} items (numbered 0 to {2-1}). |
| `timed_out == true` | Never-ending loop | Your program ran for 10 seconds without stopping. Check that your loop's condition eventually becomes false. |
| `truncated == true` | Too much output | Your program printed a huge amount, so Rewire stopped it. Is something printing inside a loop that never ends? |
| anything else | Go found a problem | Here's what Go said. Tap to see the exact message. |

Parse positions with `(?m)^(?:\./)?(?:[\w./-]+\.go:)?(\d+):(\d+): (.+)$`, which covers both `./main.go:9:2: msg` (gc) and `9:2: msg` (Yaegi). Subtract `line_offset`. Drop positions that land in wrapper lines (≤ 0) and set `line = None`.

- [ ] **Step 1: Write the failing tests**, each from a real captured fixture (record the gc fixtures with `go run`, the Yaegi ones with `labcore.Run`): `explains_unused_var_gc`, `explains_unused_var_yaegi`, `explains_undefined`, `explains_syntax`, `explains_index_out_of_range`, `timeout_card`, `truncated_card`, `unknown_falls_back_with_raw`, `line_offset_applied` (gc `./main.go:9:2` with offset 7 → line 2), `wrapper_line_becomes_none`, `parse_city_strips_lines_and_keeps_order`, `parse_city_ignores_malformed_line` (kept in stdout, no panic), `mission_pass`, `mission_first_mismatch_index`, `mission_no_events`.
- [ ] **Step 2:** `cargo test -p rewire-core`. Expected: FAIL.
- [ ] **Step 3:** Implement with the `regex` crate, compiled once via `std::sync::LazyLock`.
- [ ] **Step 4:** PASS.
- [ ] **Step 5: Commit** `feat(core): friendly go errors, city events, mission checks`.

### Task 8: MVP lesson content (interleaved path)

**Files:**
- Create: `content/go/path.yaml`, `content/go/packs/silicon-quarter/{pack.yaml,chunks/*.yaml,missions/binary-bridge.yaml}`, `content/go/packs/go-academy/{pack.yaml,chunks/*.yaml,missions/green-wave.yaml}`, `content/LICENSE` (CC BY-SA 4.0 text)

**Interfaces:** Consumes the schema and the exact wrapper (Task 5). Scene ids must exist in `apps/web/src/scenes/` (Task 14): `computer-inside`, `bus-road`, `binary-lamps`, `print-say`, `labeled-boxes`, `fork-road`, `lamp-street`, `train-cars`, `machine`.

**Path order (interleaved, owner decision §4.3):**
1. `sq/what-is-a-computer` (see-only; scene `computer-inside`; CPU, RAM, storage, I/O)
2. `ga/hello-print` (`fmt.Println`)
3. `sq/cpu-ram-bus` (see-only; scene `bus-road`)
4. `ga/variables` (`:=`, `int`, `string`)
5. `sq/bits-and-binary` (drills print numbers in binary with `fmt.Printf("%b")`)
6. `ga/if-else`
7. `ga/for-loop` (mission **green-wave**: `SetGreen(i, i*2)` for 5 lights on Dock St, events `green` delays 0,2,4,6,8)
8. `sq/binary-bridge` (mission **binary-bridge**: light lamps 0–7 to show the number 13 → `Lamp(i, on)` for bits `00001101` read left to right as lamp 0 = most significant)
9. `ga/slices`
10. `ga/functions`

- [ ] **Step 1:** Write the 10 chunks and 2 missions. Every coding chunk has a ghost, 3 tips, ≥ 5 drills with `expect_stdout`, exactly 3 hints, and a `fix_it` (a small broken program with one beginner-typical bug). Each mission has all three help levels. Copy follows the tone constraint (plain words, ≤ 2 short sentences per caption).
- [ ] **Step 2:** `just content`. Expected: `packcheck validate` reports 10 chunks, 2 missions, 0 errors, and writes `apps/web/public/content/go.json`.
- [ ] **Step 3: Commit** `content(go): mvp interleaved path, silicon quarter + go academy basics`.

### Task 9: `rewire-lab` native Go runner (bubblewrap sandbox)

**Files:**
- Create: `crates/rewire-lab/{Cargo.toml,src/lib.rs,src/runtime.rs,src/go_native.rs,src/sandbox.rs}`, `crates/rewire-lab/tests/go_native.rs`

**Interfaces (Produces):**
```rust
pub struct SourceFile { pub name: String, pub source: String }
pub struct Limits { pub compile_wall: Duration, pub run_wall: Duration, pub cpu_secs: u64, pub max_file_bytes: u64, pub max_output: usize, pub network: bool }
impl Default for Limits // 60 s, 10 s, 10, 10 MiB, 65_536, false
pub struct RawRunResult { pub stdout: String, pub stderr: String, pub exit_code: Option<i32>,
    pub timed_out: bool, pub truncated: bool, pub duration_ms: u64 }   // serde camelCase == TS RawRunResult
pub enum LabError { SandboxUnavailable(String), ToolchainMissing, Io(std::io::Error) }
#[async_trait::async_trait]
pub trait LanguageRuntime: Send + Sync { fn id(&self) -> &'static str; async fn run(&self, files: Vec<SourceFile>, limits: Limits) -> Result<RawRunResult, LabError>; }
pub struct GoNativeRuntime { pub go_root: PathBuf, pub cache_dir: PathBuf }
pub fn sandbox_probe() -> Result<(), LabError>;
```

**Sandbox (pinned):** write the files plus `go.mod` (`module lab` / `go 1.27`) to a fresh temp workdir. **Phase 1 (compile):** run `go build -o /work/prog .` inside bwrap under the compile limit. **Phase 2 (run):** run `/work/prog` inside bwrap under the run limit. The bwrap arguments, with the Go toolchain and output fully static (`CGO_ENABLED=0`), so there's **no `/usr` bind**:
`--unshare-all --die-with-parent --new-session --clearenv --ro-bind {go_root} /opt/go --bind {workdir} /work --bind {cache_dir} /cache --proc /proc --dev /dev --tmpfs /tmp --chdir /work --setenv GOROOT /opt/go --setenv GOCACHE /cache/build --setenv GOPATH /cache/gopath --setenv GOTOOLCHAIN local --setenv GOFLAGS -mod=mod --setenv CGO_ENABLED 0 --setenv HOME /tmp --setenv PATH /opt/go/bin`
Apply `RLIMIT_CPU` and `RLIMIT_FSIZE` in `pre_exec`. On timeout, kill the process group. Cap the combined output while reading.

- [ ] **Step 1: Write the failing tests** (skip with a printed reason if `sandbox_probe()` errors; CI installs bubblewrap so they run there): `hello_runs`, `compile_error_returns_stderr_with_position`, `infinite_loop_times_out_at_run_wall` (`run_wall = 2s`, returns in < 4 s, `timed_out`) (Review Focus 1), `endless_print_truncated`, `network_is_denied` (`net.Dial("tcp","1.1.1.1:80")` fails), `cannot_read_home` (`os.ReadFile(os.Getenv("REAL_HOME")+"/.bashrc")` fails; set `REAL_HOME` only in the test's expectation, not in the sandbox), `cannot_write_outside_work`, `file_size_limit`, `city_lines_present_in_stdout`, `second_run_uses_cache_and_is_faster`.
- [ ] **Step 2:** `cargo test -p rewire-lab`. Expected: FAIL.
- [ ] **Step 3:** Implement. `sandbox_probe` runs `bwrap --unshare-all --ro-bind / / true`, and a failure maps to `SandboxUnavailable` with the message "Rewire couldn't start its safe practice space on this system. See docs: Linux sandbox setup." (for example Ubuntu's AppArmor userns restriction; Task 18 ships a profile).
- [ ] **Step 4:** PASS.
- [ ] **Step 5: Commit** `feat(lab): sandboxed native go runner with bubblewrap`.

### Task 10: Go toolchain manager

**Files:**
- Create: `crates/rewire-lab/src/toolchain.rs`, `crates/rewire-lab/tests/toolchain.rs`

**Interfaces (Produces):**
```rust
pub const GO_VERSION: &str = "go1.27.2";
pub enum ToolchainStatus { Ready { go_root: PathBuf }, Missing }
pub enum ToolchainError { Offline, ChecksumMismatch, Unsupported(String), Io(std::io::Error) }
pub fn status(data_dir: &Path) -> ToolchainStatus;          // REWIRE_GO_ROOT env wins, else {data_dir}/toolchains/go1.27.2 with marker .rewire-ok
pub async fn install(data_dir: &Path, base_url: &str, on_progress: impl Fn(u64, u64)) -> Result<PathBuf, ToolchainError>;
pub async fn warm_cache(rt: &GoNativeRuntime) -> Result<(), LabError>; // compiles a hello program to prebuild fmt
```
Downloads use `{base_url}/{GO_VERSION}.linux-{amd64|arm64}.tar.gz`, and the checksums come from `{base_url}/?mode=json&include=all` (field `sha256` for the matching `filename`). Production `base_url = "https://go.dev/dl"`. Extract into a temp dir, then `rename` it into place atomically.

- [ ] **Step 1: Write the failing tests**, serving fixtures from a local `httpmock` server: `status_missing_then_ready_after_install`, `env_override_wins`, `checksum_mismatch_rejected_and_nothing_left_behind`, `offline_maps_to_offline_error` (unreachable base_url) (Review Focus 2), `progress_reported`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement with `reqwest` (rustls), `sha2`, `flate2`, `tar`. **Step 4:** PASS.
- [ ] **Step 5: Commit** `feat(lab): pinned go toolchain download with checksum verification`.

### Task 11: `rewire-mentor`: providers, presets, AG-UI events

**Files:**
- Create: `crates/rewire-mentor/{Cargo.toml,src/lib.rs,src/presets.rs,src/openai_compat.rs,src/anthropic.rs,src/agui.rs,src/prompt.rs,src/keys.rs}`, `crates/rewire-mentor/tests/*.rs`

**Interfaces (Produces):**
```rust
pub enum ApiKind { OpenAiCompatible, AnthropicMessages }
pub struct Preset { pub id: &'static str, pub label: &'static str, pub base_url: &'static str, pub api: ApiKind, pub needs_key: bool, pub local: bool }
pub fn presets() -> &'static [Preset];
pub struct ProviderConfig { pub preset_id: String, pub base_url: String, pub model: String } // user-editable
pub struct HintRequest { pub concept: String, pub exercise_prompt: String, pub code: String, pub last_error: Option<String>, pub attempt: u32 }
pub enum AgUiEvent { RunStarted { thread_id: String, run_id: String }, TextMessageStart { message_id: String, role: String },
    TextMessageContent { message_id: String, delta: String }, TextMessageEnd { message_id: String },
    RunFinished { thread_id: String, run_id: String }, RunError { message: String } }
    // serde: tag "type" with values "RUN_STARTED", "TEXT_MESSAGE_START", "TEXT_MESSAGE_CONTENT", "TEXT_MESSAGE_END", "RUN_FINISHED", "RUN_ERROR"; fields camelCase
pub async fn hint(cfg: &ProviderConfig, api_key: Option<String>, req: &HintRequest, sink: &mut (dyn FnMut(AgUiEvent) + Send)) -> Result<(), MentorError>;
pub fn store_key(preset_id: &str, key: &str) -> Result<(), MentorError>;  // keyring service "dev.playrewire.rewire"
pub fn load_key(preset_id: &str) -> Result<Option<String>, MentorError>;
```

**Presets (id → base URL).** Verify each against the provider's current docs during implementation and fix any that changed:
`ollama` http://localhost:11434/v1 · `llamacpp` http://localhost:8080/v1 · `lmstudio` http://localhost:1234/v1 · `nvidia-nim` https://integrate.api.nvidia.com/v1 · `openrouter` https://openrouter.ai/api/v1 · `openai` https://api.openai.com/v1 · `anthropic` https://api.anthropic.com (AnthropicMessages, header `anthropic-version: 2023-06-01`, default model `claude-haiku-5-5`) · `meta-llama` https://api.llama.com/compat/v1 · `deepseek` https://api.deepseek.com/v1 · `minimax` https://api.minimax.io/v1 · `omniroute` http://localhost:20128/v1 (local) · `custom` (user-supplied).

**System prompt (exact):** "You are Rewire's coding mentor for a beginner who may be nervous about code. Never write the full solution. Reply in at most 3 short sentences. Ask one guiding question or point to one specific line. Use plain words and explain any technical term you use." The user message contains only `concept`, `exercise_prompt`, `code`, `last_error` and `attempt`. **No personal data.**

Timeouts: connect **5 s**, whole hint **20 s**.

- [ ] **Step 1: Write the failing tests** against `httpmock`:
  - `openai_compat_streams_sse_into_agui_events`: the event order is RUN_STARTED, TEXT_MESSAGE_START, ≥ 1 CONTENT, TEXT_MESSAGE_END, RUN_FINISHED.
  - `anthropic_streams_into_agui_events`: checks the `x-api-key` and `anthropic-version` headers.
  - `request_contains_no_extra_fields`: the request body's user content equals the serialized HintRequest fields only.
  - `system_prompt_exact`.
  - `unreachable_emits_run_error_within_timeout`, `401_emits_run_error_with_plain_message` ("The mentor couldn't connect. Check your key in Settings."), `slow_stream_times_out_at_20s` (use a paused tokio clock) (Review Focus 5).
  - `presets_have_unique_ids_and_all_spec_providers`: covers ollama, llamacpp, lmstudio, nvidia-nim, openrouter, openai, anthropic, meta-llama, deepseek, minimax, omniroute, custom.
  - `agui_serialization_shape`: `{"type":"TEXT_MESSAGE_CONTENT","messageId":"m1","delta":"hi"}`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. Load the `claude-api` skill before writing `anthropic.rs`. **Step 4:** PASS.
- [ ] **Step 5: Commit** `feat(mentor): byo-model providers with ag-ui event stream`.

### Task 12: `rewire-store` (SQLite) and `rewire-core-wasm`

**Files:**
- Create: `crates/rewire-store/{Cargo.toml,src/lib.rs}`, `crates/rewire-store/tests/store.rs`
- Create: `crates/rewire-core-wasm/{Cargo.toml,src/lib.rs}`, `apps/web/src/core/index.ts`, `apps/web/src/core/core.test.ts`

**Interfaces (Produces):**
```rust
// rewire-store
pub struct Store { /* rusqlite::Connection */ }
impl Store {
  pub fn open(path: &Path) -> Result<Store, StoreError>;          // WAL, PRAGMA user_version migrations
  pub fn load_progress(&self) -> Result<ProgressState, StoreError>;  // default if absent
  pub fn save(&mut self, state: &ProgressState, record: &StepRecord) -> Result<(), StoreError>; // one transaction: kv + outbox
  pub fn outbox_unsynced(&self, limit: usize) -> Result<Vec<OutboxRow>, StoreError>;
  pub fn get_setting(&self, key: &str) -> Result<Option<String>, StoreError>;
  pub fn set_setting(&mut self, key: &str, value: &str) -> Result<(), StoreError>;
}
// tables: kv(key TEXT PRIMARY KEY, value TEXT NOT NULL); outbox(id INTEGER PRIMARY KEY, kind TEXT NOT NULL, payload TEXT NOT NULL, created_at TEXT NOT NULL, synced INTEGER NOT NULL DEFAULT 0); settings(key TEXT PRIMARY KEY, value TEXT NOT NULL)
```
```ts
// apps/web/src/core/index.ts — thin typed wrapper over rewire-core-wasm (JSON in/out)
export function applyStep(state: ProgressState, rec: StepRecord): {state: ProgressState; rewards: Reward[]}
export function dueConcepts(state: ProgressState, today: string /* YYYY-MM-DD */): string[]
export function helpLevel(state: ProgressState, missionId: string): HelpLevel
export function level(xp: number): number
export function explainGoErrors(stderr: string, lineOffset: number, timedOut: boolean, truncated: boolean): FriendlyError[]
export function parseCityOutput(stdout: string): {stdout: string; events: CityEvent[]}
export function checkMission(expected: CityEvent[], actual: CityEvent[]): MissionVerdict
```

- [ ] **Step 1: Write the failing tests.** Rust: `fresh_db_returns_default_progress`, `save_writes_state_and_outbox_atomically`, `reopen_keeps_state`, `migrations_idempotent`, `corrupt_kv_json_falls_back_to_default_and_keeps_backup` (the bad value is copied to `kv['progress.corrupt.<ts>']`). TS: `applyStep via wasm matches native fixture` (a JSON fixture produced by a Rust test), `explainGoErrors returns Unused box`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. Build the wasm with `just wasm` → `apps/web/src/core/pkg/`. **Step 4:** PASS.
- [ ] **Step 5: Commit** `feat(store,core-wasm): sqlite persistence with outbox; core rules for the ui`.

---

## Milestone 3: UI, desktop shell and world

### Task 13: Web UI shell, design tokens, Backend interface

**Files:**
- Create: `design/tokens.json`, `apps/web/scripts/tokens.ts` (writes `src/design/tokens.css`), `apps/web/{package.json,vite.config.ts,index.html,src/main.ts,src/App.svelte}`
- Create: `apps/web/src/backend/{types.ts,web.ts,tauri.ts,index.ts}`, `apps/web/src/routes/{Home.svelte,Map.svelte,Settings.svelte}`, `apps/web/src/lib/{Card.svelte,InkButton.svelte}`, tests `*.test.ts`

**Interfaces (Produces):**
```ts
export interface Backend {
  kind: 'web' | 'desktop';
  run(files: SourceFile[]): Promise<RawRunResult>;
  loadProgress(): Promise<ProgressState>;
  saveProgress(state: ProgressState, rec: StepRecord): Promise<void>;
  toolchain(): Promise<{status: 'ready' | 'missing' | 'n/a'}>;         // web → 'n/a'
  installToolchain(onProgress: (done: number, total: number) => void): Promise<void>;
  mentorAvailable(): boolean;                                           // web → false in MVP
  mentorHint(req: HintRequest, onEvent: (e: AgUiEvent) => void): Promise<void>;
  getSettings(): Promise<Settings>; saveSettings(s: Settings): Promise<void>;
  setApiKey(presetId: string, key: string): Promise<void>;
}
export function getBackend(): Backend;   // window.__TAURI_INTERNALS__ ? tauri : web
export function nextChunk(path: ChunkRef[], state: ProgressState, bundle: Bundle): ChunkRef | null; // first ref with any incomplete required step
```
`Settings = {reducedMotion: boolean | 'system'; dyslexiaFont: boolean; textScale: 1 | 1.25 | 1.5; mentor: ProviderConfig | null}`.
Web persistence: `localStorage['rewire.progress.v1']`, with every access in try/catch. If it's unavailable, keep the state in memory and show a one-line notice "Progress won't be saved in this window."

- [ ] **Step 1: Write the failing tests:** `tokens.css contains the four spec colors` (`--paper:#f6f0e4`, `--ink:#2a2620`, `--accent:#e2572b`, `--card:#fffdf7`), `getBackend picks web without tauri`, `web backend survives localStorage throwing`, `nextChunk returns first incomplete in path order`, `nextChunk skips Use for see-only chunks`, `Home shows streak, xp, level and a single primary action` (Testing Library: exactly one `button[data-primary]`).
- [ ] **Step 2:** FAIL. **Step 3:** Implement. Home is "today's page" (Paper & Ink direction B, from the chosen mockup: three hand-drawn stat cards for streak, new ideas and to-fix count, a dotted path, and "let's go →"). Map shows two inked districts and the other eight as blank paper. Load fonts from `@fontsource/*` packages so the app is fully offline. **Step 4:** PASS.
- [ ] **Step 5: Commit** `feat(web): paper & ink shell, tokens, backend interface, home and map`.

### Task 14: Chunk player: See, Type, Drill, friendly errors

**Files:**
- Create: `apps/web/src/player/{ChunkPlayer.svelte,SeeStep.svelte,TypeStep.svelte,DrillStep.svelte,ErrorCard.svelte,ghost.ts,ghost.test.ts,drill.ts,drill.test.ts}`, `apps/web/src/scenes/*.svelte` (the 9 scene ids from Task 8)

**Interfaces:**
- Consumes: `Backend.run`, `core.explainGoErrors`, `core.parseCityOutput`, `core.applyStep`, and the bundle types.
- Produces (pure TS):
  ```ts
  export type GhostState = {typed: string; correctUpTo: number; errorAt: number | null; done: boolean};
  export function ghostKey(ghost: string, s: GhostState, key: string): GhostState; // key: printable char | 'Enter' | 'Backspace' | 'Tab'
  export function wrapMain(code: string): {source: string; lineOffset: 7};
  export function checkDrill(expected: string, rawStdout: string): {pass: boolean; diffLine: number | null}; // trims trailing whitespace per line, normalizes CRLF
  ```

**Behavior pinned:**
- **See:** an SVG scene with a CSS animation, caption ≤ 2 lines, a "next" button. With reduced motion, the scene shows its final frame.
- **Type:**
  - A wrong key marks `errorAt` and must be fixed with Backspace before going on.
  - `Enter` at the end of a ghost line jumps past the next line's leading whitespace, and `Tab` counts as matching that whitespace.
  - CRLF in the ghost is normalized to LF.
  - **Paste events are cancelled** and show "Type it out, it's how your fingers learn."
  - Tips appear as handwritten labels under their `match` text.
- **Drill:**
  - Drills appear one at a time in a CodeMirror 6 editor (Go mode).
  - A round timer counts up and shows the characters per minute (cpm) of the final answer.
  - Run → `run(wrapMain(code))` → `explainGoErrors` → ErrorCard (friendly title and plain text on top, raw message behind a disclosure) or a ✓.
  - Three passes in a row completes the Drill step.
  - The hint button reveals the chunk's built-in hints one at a time, and each counts toward `hints_used`.
- After each step, call `applyStep`, then `saveProgress`, then show rewards (XP float-up, which is static with reduced motion).

- [ ] **Step 1: Write the failing tests** in `ghost.test.ts`: `correct keys advance`, `wrong key blocks until backspace`, `enter skips leading indentation`, `tab matches indentation`, `crlf ghost normalized`, `done at end`. In `drill.test.ts`: `checkDrill ignores trailing spaces and crlf`, `diffLine points at first differing line`. Component tests: `TypeStep cancels paste and shows the hint` (Review Focus 4), `DrillStep shows friendly card above raw error`, `pass/fail shows ✓/✗ glyphs not just color`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS.
- [ ] **Step 5: Commit** `feat(web): see/type/drill chunk player with friendly errors`.

### Task 15: Desktop shell (Tauri v2)

**Files:**
- Create: `apps/desktop/src-tauri/{Cargo.toml,tauri.conf.json,build.rs,capabilities/default.json,src/main.rs,src/commands.rs,src/state.rs}`, `apps/desktop/src-tauri/tests/commands.rs`

**Interfaces:**
- Consumes: `rewire-lab` (`GoNativeRuntime`, `Limits::default()`, `toolchain::{status, install, warm_cache}`), `rewire-store::Store`, `rewire-mentor::{hint, presets, store_key, load_key}`.
- Produces the Tauri commands that `apps/web/src/backend/tauri.ts` calls, one per `Backend` method: `run_code(files) -> RawRunResult`, `load_progress() -> ProgressState`, `save_progress(state, record)`, `toolchain_status() -> {status}`, `install_toolchain(on_progress: Channel<(u64,u64)>)`, `mentor_hint(req, on_event: Channel<AgUiEvent>)`, `get_settings`, `save_settings`, `set_api_key(preset_id, key)`.
- Data dir: `$XDG_DATA_HOME/dev.playrewire.rewire/` (from Tauri's `app_data_dir`), holding `rewire.db`, `toolchains/` and `gocache/`.
- `tauri.conf.json`: identifier `dev.playrewire.rewire`, product name `Rewire`, `frontendDist: ../../web/dist`, a CSP that allows `'wasm-unsafe-eval'` for the world and core wasm. Mentor HTTP calls happen in Rust, so the webview needs no outbound network permissions.

- [ ] **Step 1: Write the failing tests** (calling the command functions directly with a temp data dir): `run_code_without_toolchain_returns_toolchain_missing`, `save_then_load_progress`, `settings_roundtrip`, `api_key_never_written_to_db` (after `set_api_key`, a scan of the settings and kv tables doesn't contain the key).
- [ ] **Step 2:** FAIL. **Step 3:** Implement, and on first launch, if `toolchain_status` is `missing`, have the UI show the one-time download card (Review Focus 2 copy) while See and Type stay usable. **Step 4:** PASS, then run `just dev-desktop`. Manual check: hello drill runs, and progress survives a restart.
- [ ] **Step 5: Commit** `feat(desktop): tauri v2 shell wiring lab, store and mentor`.

### Task 16: Hand-inked street and the Use step

**Files:**
- Modify/Create: `crates/rewire-world/src/{lib.rs,scene.rs,player.rs,terminals.rs,city_events.rs,bridge.rs,tokens.rs}`, `crates/rewire-world/build.rs` (reads `design/tokens.json` into `tokens.rs`)
- Create: `apps/web/src/player/UseStep.svelte`, `apps/web/src/world/world.ts`, tests

**Interfaces:**
- Consumes: the spike scene and ink pass (Task 2), `CityEvent` (Task 7), `HelpLevel` and `helpLevel` (Tasks 6/12), mission bundle data (Task 8).
- Produces (wasm exports via wasm-bindgen, wrapped by `world.ts`):
  ```ts
  start(canvasId: string): Promise<void>
  loadMission(missionId: 'green-wave' | 'binary-bridge'): void
  applyEvents(eventsJson: string): void           // animates in order; green delays are scaled by 0.25 for pacing
  setReducedMotion(on: boolean): void             // jump to end state, no camera moves
  onTerminal(cb: (missionId: string) => void): void  // player stands within 2 m of a terminal and presses E/Enter/tap
  onEventsDone(cb: () => void): void
  ```
- Scene: **Dock St** with 5 traffic lights (terminal `dock-st-traffic` → green-wave) and the **Binary Bridge** with 8 lamps (terminal `bridge-panel` → binary-bridge). Movement is WASD/arrow keys plus click-to-walk, and the camera follows the player.
- **Use step:**
  1. The editor overlay opens on the mission template for `helpLevel(state, missionId)`.
  2. Run → `run([cityStub, setup, wrapMain(code)])` → `parseCityOutput` → `applyEvents` → `checkMission`.
  3. On pass, call `applyStep` with `step: "use"` and the mission id.
  4. On `WrongEvents`, highlight the first mismatching light or lamp in the world with an ink circle and a caption like "Light 3 turned green at the wrong time."

- [ ] **Step 1: Write the failing tests.** Bevy headless (`App` with `MinimalPlugins`): `apply_green_events_sets_light_states_in_order`, `lamp_events_set_bridge_lamps`, `terminal_trigger_only_within_2m`, `reduced_motion_applies_end_state_immediately`. Vitest: `UseStep picks template by help level` (encounter 0 → template contains `___`, 2 → empty editor plus hint button), `wrong events show mismatch caption`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS. Manual check on desktop and web: walk to Dock St, press E, fill the blank, and watch the green wave.
- [ ] **Step 5: Commit** `feat(world): hand-inked dock st and binary bridge with use-step missions`.

### Task 17: Today loop, skill map, mentor panel, settings

**Files:**
- Create/Modify: `apps/web/src/routes/{Home.svelte,SkillMap.svelte,Settings.svelte}`, `apps/web/src/player/MentorPanel.svelte`, `apps/web/src/today.ts`, tests

**Interfaces:**
- Consumes: `dueConcepts`, `level`, `nextChunk`, `Backend.mentor*`, `presets` (exposed through a `get_presets` command added to Task 15's command list; on web, a static copy generated at build time from `crates/rewire-mentor`).
- Produces: `buildToday(state, bundle, today: string): {warmup: DrillRef[]; next: ChunkRef | null; fixIt: ChunkRef | null; streak; xp; level; toFix: number}`. The warm-up is **one drill per due concept, at most 3, for about 60 seconds** (spec §4.6). `fixIt` is the `fix_it` of the most recently completed chunk that has one and whose fix-it isn't done yet. Completing it records a `drill` step on that chunk's concept with `chunk_id` suffixed `#fix`.

**Behavior pinned:**
- **Home:** "let's go →" runs the day in order: the warm-up (if any is due), the next chunk, then the break-it/fix-it task (if any).
- **Skill map:** each concept is a hand-drawn bar (mastery = `box_level / 5`), with a "keeps breaking" list (concepts with `fails ≥ 3`).
- **Mentor panel** (desktop):
  - It renders the AG-UI events as a streamed handwritten note.
  - On `RUN_ERROR`, it switches to the built-in hint ladder with the copy from Review Focus 5.
  - On web it shows only the built-in hints, with the line "AI mentor is available in the desktop app."
- **Settings:**
  - Mentor: pick a preset, enter the model and key (the key goes to `setApiKey` and is never shown again).
  - The OmniRoute preset shows the notice: "OmniRoute runs on your computer and pools free tiers from many providers. Each provider's free-tier rules differ, and OmniRoute can see your prompts."
  - Accessibility: reduced motion, dyslexia font, text size.

- [ ] **Step 1: Write the failing tests:** `buildToday caps warmup at 3`, `warmup empty when nothing due`, `fixIt picks most recent completed chunk with an undone fix_it`, `skill map lists keeps-breaking at fails>=3`, `mentor panel falls back on RUN_ERROR`, `web shows built-in hints only`, `omniroute notice shown`, `dyslexia toggle switches body font`, `reduced motion setting overrides system`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS.
- [ ] **Step 5: Commit** `feat(web): today loop, skill map, mentor panel and settings`.

### Task 18: Linux bundles, web build and release workflow

**Files:**
- Create: `.github/workflows/release.yml`, `packaging/linux/apparmor/dev.playrewire.rewire` (lets bundled bwrap create user namespaces on Ubuntu 24.04+), `packaging/linux/README.md`, `docs/install.md`
- Modify: `apps/desktop/src-tauri/tauri.conf.json` (bundle targets `deb`, `rpm`, `appimage`, deb/rpm dependency on `bubblewrap`, deb post-install loads the AppArmor profile), `README.md` (replace "Getting started" with install and dev instructions)

- [ ] **Step 1:** `just bundle`. Expected: `.deb`, `.rpm` and `.AppImage` in `target/release/bundle/`, plus `apps/web/dist/` (static web app with `content/go.json`, `lab/lab.wasm`, `world/*.wasm`).
- [ ] **Step 2: Smoke test.** In an `ubuntu:24.04` container and a `fedora` container, install the package, launch it headless with `xvfb-run` and a `--smoke` flag (added to `main.rs`: boot, run the hello drill through the sandbox, print `SMOKE OK`, exit 0). Expected: `SMOKE OK` in both.
- [ ] **Step 3: Release workflow.** On a `v*` tag, build on `ubuntu-24.04`, generate `SHA256SUMS`, sign with cosign keyless, attach an SBOM (`syft`), and upload the bundles plus a `rewire-web-<ver>.tar.gz` to the GitHub Release.
- [ ] **Step 4: Commit** `build: linux bundles, web build and signed release workflow`.
- [ ] **Step 5:** Ask the owner before pushing a tag. A release is public.

---

## Deferred (outside this plan)

- **Mobile performance spike** (Bevy in Android and iOS webviews): needs the owner's devices and Android SDK/NDK and Xcode. It runs before the Phase 5 plan.
- AI mentor on web (browser CORS per provider), Keycloak login, sync, notifications, squads, exams and `exam-checker`, Windows and macOS, Flatpak, AUR and stores: Phases 2–5 plans.
- FSRS scheduler replacing Leitner: Phase 2.
