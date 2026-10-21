package main

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

const goodDir = "testdata/good"

// copyGood copies testdata/good into a temp dir so a test can break it.
func copyGood(t *testing.T) string {
	t.Helper()
	dst := t.TempDir()
	err := filepath.Walk(goodDir, func(p string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		rel, _ := filepath.Rel(goodDir, p)
		target := filepath.Join(dst, rel)
		if info.IsDir() {
			return os.MkdirAll(target, 0o755)
		}
		b, err := os.ReadFile(p)
		if err != nil {
			return err
		}
		return os.WriteFile(target, b, 0o644)
	})
	if err != nil {
		t.Fatal(err)
	}
	return dst
}

func write(t *testing.T, root, rel, content string) {
	t.Helper()
	p := filepath.Join(root, rel)
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(p, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
}

// replace edits a fixture file, failing if the old text is not there.
func replace(t *testing.T, root, rel, old, new string) {
	t.Helper()
	b, err := os.ReadFile(filepath.Join(root, rel))
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(b), old) {
		t.Fatalf("%s does not contain %q", rel, old)
	}
	write(t, root, rel, strings.Replace(string(b), old, new, 1))
}

func problems(root string) string {
	_, is := Validate(root)
	var parts []string
	for _, i := range is {
		parts = append(parts, i.String())
	}
	return strings.Join(parts, "\n")
}

func wantProblem(t *testing.T, root string, subs ...string) {
	t.Helper()
	got := problems(root)
	if got == "" {
		t.Fatalf("expected problems, got none")
	}
	for _, s := range subs {
		if !strings.Contains(got, s) {
			t.Errorf("problems do not mention %q:\n%s", s, got)
		}
	}
}

const printFile = "packs/basics/chunks/print.yaml"

func TestValidPackBundles(t *testing.T) {
	l, is := Validate(goodDir)
	if len(is) > 0 {
		t.Fatalf("good fixture has problems:\n%s", problems(goodDir))
	}
	b := BuildBundle(l)
	if b.WrapperLineOffset != 7 || b.Version != 1 || b.Language != "go" {
		t.Fatalf("bad header: %+v", b)
	}
	if !strings.Contains(b.CityStub, "func SetGreen") {
		t.Error("cityStub missing")
	}
	steps := map[string]string{}
	for _, p := range b.Packs {
		for _, c := range p.Chunks {
			steps[c.ID] = strings.Join(c.Steps, ",")
			if c.Pack != p.ID {
				t.Errorf("chunk %s pack = %q", c.ID, c.Pack)
			}
		}
	}
	want := map[string]string{"hello": "see", "print": "see,type,drill,use", "funcs": "see,type,drill"}
	for id, w := range want {
		if steps[id] != w {
			t.Errorf("steps[%s] = %q, want %q", id, steps[id], w)
		}
	}
	// the wrapper puts user line 1 on file line 8
	if got := strings.Split(wrapMain("X", false), "\n")[WrapperLineOffset]; got != "X" {
		t.Errorf("user line 1 is not file line 8: %q", got)
	}
}

func TestGoodBundleIsNotStale(t *testing.T) {
	l, is := Validate(goodDir)
	if len(is) > 0 {
		t.Fatal(problems(goodDir))
	}
	data, err := MarshalBundle(BuildBundle(l))
	if err != nil {
		t.Fatal(err)
	}
	golden := "testdata/good.bundle.json"
	if os.Getenv("UPDATE_BUNDLE") == "1" {
		if err := os.WriteFile(golden, data, 0o644); err != nil {
			t.Fatal(err)
		}
		return
	}
	have, err := os.ReadFile(golden)
	if err != nil {
		t.Fatalf("%v; run `just packcheck-fixture`", err)
	}
	if string(have) != string(data) {
		t.Fatalf("%s is stale; run `just packcheck-fixture` and commit it", golden)
	}
}

func TestFixtureCityMatchesContent(t *testing.T) {
	a, _ := os.ReadFile(goodDir + "/city/city.go")
	b, err := os.ReadFile("../../content/go/city/city.go")
	if err != nil {
		t.Fatal(err)
	}
	if string(a) != string(b) {
		t.Fatal("testdata/good/city/city.go differs from content/go/city/city.go; copy it over")
	}
}

func TestRejectsFewerThanFiveDrills(t *testing.T) {
	root := copyGood(t)
	replace(t, root, printFile, "  - prompt: Print the sum of 2 and 3.\n    solution: fmt.Println(2 + 3)\n    expect_stdout: \"5\\n\"\n", "")
	wantProblem(t, root, "at least 5 drills", `chunk "print"`)
}

func TestHintsExactlyThree(t *testing.T) {
	root := copyGood(t)
	replace(t, root, printFile, "  - Put text inside double quotes.\n", "")
	wantProblem(t, root, "exactly 3 hints", `chunk "print"`)
}

func TestSeeOnlyChunkRejectsHints(t *testing.T) {
	root := copyGood(t)
	write(t, root, "packs/basics/chunks/hello.yaml", `id: hello
concept: program
title: Your first program
see:
  scene: street-intro
  caption: x
hints: [a, b, c]
`)
	wantProblem(t, root, `chunk "hello"`, "only allowed on a chunk with drills")
}

func TestRejectsWrongSolution(t *testing.T) {
	root := copyGood(t)
	replace(t, root, printFile, "solution: fmt.Println(42)", "solution: fmt.Println(43)")
	wantProblem(t, root, `chunk "print"`, "drills[1]", "wrong output")
}

func TestOutputComparisonIgnoresTrailingWhitespace(t *testing.T) {
	if normalizeOutput("a  \r\nb\n\n") != normalizeOutput("a\nb") {
		t.Fatal("normalization too strict")
	}
	if normalizeOutput("a\n b") == normalizeOutput("a\nb") {
		t.Fatal("normalization must keep leading whitespace")
	}
}

func TestRejectsGhostThatDoesNotCompile(t *testing.T) {
	root := copyGood(t)
	replace(t, root, printFile, `fmt.Println("hello")`, `fmt.Printn("hello")`)
	wantProblem(t, root, `chunk "print"`, "type", "ghost does not compile")
}

func TestMissionEventsMustMatch(t *testing.T) {
	root := copyGood(t)
	replace(t, root, "packs/basics/missions/lights1.yaml", "{op: lamp, i: 0, on: true}", "{op: lamp, i: 1, on: true}")
	wantProblem(t, root, `mission "lights1"`, "levels.fill_blank", "do not match")
}

func TestMissionEventOrderMatters(t *testing.T) {
	root := copyGood(t)
	replace(t, root, "packs/basics/missions/lights1.yaml",
		"  - {op: lamp, i: 0, on: true}\n  - {op: say, text: \"on\"}\n",
		"  - {op: say, text: \"on\"}\n  - {op: lamp, i: 0, on: true}\n")
	wantProblem(t, root, `mission "lights1"`, "do not match")
}

func TestMissionTemplateMustBeNonEmptyExceptBlank(t *testing.T) {
	root := copyGood(t)
	replace(t, root, "packs/basics/missions/lights1.yaml", "template: |\n      Lamp(0, true)\n", "template: \"\"\n")
	wantProblem(t, root, `mission "lights1"`, "levels.half_function", "template must not be empty")
}

func TestPathReferencesExist(t *testing.T) {
	root := copyGood(t)
	write(t, root, "path.yaml", "- {pack: basics, chunk: hello}\n- {pack: basics, chunk: nope}\n- {pack: ghost, chunk: x}\n")
	wantProblem(t, root, `no chunk "nope"`, `unknown pack "ghost"`)
}

func TestMissingPathYaml(t *testing.T) {
	root := copyGood(t)
	os.Remove(filepath.Join(root, "path.yaml"))
	wantProblem(t, root, "no path.yaml")
}

func TestChunkMissionMustExist(t *testing.T) {
	root := copyGood(t)
	replace(t, root, printFile, "mission: lights1", "mission: missing")
	wantProblem(t, root, `mission "missing" does not exist`)
}

func TestFixItBrokenMustFail(t *testing.T) {
	root := copyGood(t)
	replace(t, root, printFile, "fmt.Println(\"oops)\n", "fmt.Println(\"oops\")\n")
	wantProblem(t, root, `chunk "print"`, "fix_it.broken", "already prints expect_stdout")
}

func TestFixItSolutionMustPass(t *testing.T) {
	root := copyGood(t)
	replace(t, root, printFile, "  solution: |\n    fmt.Println(\"oops\")", "  solution: |\n    fmt.Println(\"nope\")")
	wantProblem(t, root, "fix_it.solution", "wrong output")
}

func TestUnknownYAMLKeysAreErrors(t *testing.T) {
	root := copyGood(t)
	replace(t, root, printFile, "concept: print\n", "concept: print\nsurprise: yes\n")
	wantProblem(t, root, printFile, "surprise")
}

func TestCollectsAllErrors(t *testing.T) {
	root := copyGood(t)
	replace(t, root, printFile, "solution: fmt.Println(42)", "solution: fmt.Println(43)")
	replace(t, root, printFile, "solution: fmt.Println(2 + 3)", "solution: fmt.Println(6)")
	got := problems(root)
	if !strings.Contains(got, "drills[1]") || !strings.Contains(got, "drills[4]") {
		t.Fatalf("expected both drills reported:\n%s", got)
	}
	if strings.Index(got, "drills[1]") > strings.Index(got, "drills[4]") {
		t.Fatalf("errors not sorted by index:\n%s", got)
	}
}

func TestCLIBuildWritesBundle(t *testing.T) {
	out := filepath.Join(t.TempDir(), "x", "go.json")
	if code := run([]string{"build", goodDir, "-o", out}); code != 0 {
		t.Fatalf("exit %d", code)
	}
	b, err := os.ReadFile(out)
	if err != nil || !strings.Contains(string(b), `"wrapperLineOffset": 7`) {
		t.Fatalf("bundle not written: %v", err)
	}
	if code := run([]string{"validate", "testdata/does-not-exist"}); code != 1 {
		t.Fatalf("validate of empty dir exit %d, want 1", code)
	}
}

func TestToolchainAvailable(t *testing.T) {
	if _, err := exec.LookPath("go"); err != nil {
		t.Fatal("go toolchain required")
	}
}
