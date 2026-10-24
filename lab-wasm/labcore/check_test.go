package labcore

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// prog is a whole package main file with the given top-level code.
func prog(imports, body string) string {
	return "package main\n\nimport (\n" + imports + "\n)\n\n" + body + "\n"
}

func TestMissingReturnIsRejected(t *testing.T) {
	cases := []string{
		prog(`"fmt"`, "func triple(n int) int { fmt.Println(n * 3) }\n\nfunc main() { triple(5) }"),
		prog(`"fmt"`, "func max(a, b int) int {\n\tif a > b {\n\t\treturn a\n\t}\n\tif b >= a {\n\t\treturn b\n\t}\n}\n\nfunc main() { fmt.Println(max(1, 2)) }"),
	}
	for _, src := range cases {
		r := Run(context.Background(), []File{{Name: "main.go", Source: src}})
		if r.ExitCode != 1 || r.Stdout != "" || !regexp.MustCompile(`(?m)^\d+:\d+: missing return$`).MatchString(r.Stderr) {
			t.Errorf("want missing return, got %+v\nfor:\n%s", r, src)
		}
	}
}

func TestUnusedValueIsRejected(t *testing.T) {
	src := prog(`"fmt"`, "func triple(n int) int {\n\tn * 3\n}\n\nfunc main() { fmt.Println(triple(5)) }")
	r := Run(context.Background(), []File{{Name: "main.go", Source: src}})
	if r.ExitCode != 1 || r.Stdout != "" || !strings.Contains(r.Stderr, "8:2: n * 3 (value of type int) is not used") {
		t.Fatalf("got %+v", r)
	}
}

func TestUnusedImportIsRejected(t *testing.T) {
	src := prog("\t\"fmt\"\n\t\"strings\"", "func main() { fmt.Println(1) }")
	r := Run(context.Background(), []File{{Name: "main.go", Source: src}})
	if r.ExitCode != 1 || r.Stdout != "" || !strings.Contains(r.Stderr, `5:2: "strings" imported and not used`) {
		t.Fatalf("got %+v", r)
	}
}

// corpus holds programs gc and labcore must agree on. None of them panics
// at run time, so accept/reject is decided at compile time on both sides.
var corpus = []struct {
	name, src string
	city      bool
}{
	{"hello", prog(`"fmt"`, `func main() { fmt.Println("hi") }`), false},
	{"triple-ok", prog(`"fmt"`, "func triple(n int) int { return n * 3 }\n\nfunc main() { fmt.Println(triple(5)) }"), false},
	{"triple-print-no-return", prog(`"fmt"`, "func triple(n int) int { fmt.Println(n * 3) }\n\nfunc main() { triple(5) }"), false},
	{"triple-bare-expr", prog(`"fmt"`, "func triple(n int) int { n * 3 }\n\nfunc main() { fmt.Println(triple(5)) }"), false},
	{"max-two-ifs", prog(`"fmt"`, "func max(a, b int) int {\n\tif a > b {\n\t\treturn a\n\t}\n\tif b >= a {\n\t\treturn b\n\t}\n}\n\nfunc main() { fmt.Println(max(1, 2)) }"), false},
	{"max-if-else", prog(`"fmt"`, "func max(a, b int) int {\n\tif a > b {\n\t\treturn a\n\t} else {\n\t\treturn b\n\t}\n}\n\nfunc main() { fmt.Println(max(1, 2)) }"), false},
	{"switch-default", prog(`"fmt"`, "func sign(n int) string {\n\tswitch {\n\tcase n < 0:\n\t\treturn \"-\"\n\tdefault:\n\t\treturn \"+\"\n\t}\n}\n\nfunc main() { fmt.Println(sign(1)) }"), false},
	{"switch-no-default", prog(`"fmt"`, "func sign(n int) string {\n\tswitch {\n\tcase n < 0:\n\t\treturn \"-\"\n\tcase n >= 0:\n\t\treturn \"+\"\n\t}\n}\n\nfunc main() { fmt.Println(sign(1)) }"), false},
	{"for-ever", prog(`"fmt"`, "func spin() int {\n\tfor {\n\t\treturn 1\n\t}\n}\n\nfunc main() { fmt.Println(spin()) }"), false},
	{"for-break", prog(`"fmt"`, "func spin() int {\n\tfor {\n\t\tbreak\n\t}\n}\n\nfunc main() { fmt.Println(spin()) }"), false},
	{"panic-ends", prog(`"fmt"`, "func must(n int) int {\n\tif n > 0 {\n\t\treturn n\n\t}\n\tpanic(\"no\")\n}\n\nfunc main() { fmt.Println(must(1)) }"), false},
	{"unused-import", prog("\t\"fmt\"\n\t\"strings\"", "func main() { fmt.Println(1) }"), false},
	{"used-import", prog("\t\"fmt\"\n\t\"strings\"", `func main() { fmt.Println(strings.ToUpper("a")) }`), false},
	{"unused-var", prog(`"fmt"`, "func main() {\n\tx := 1\n\tfmt.Println(2)\n}"), false},
	{"bare-literal", prog(`"fmt"`, "func main() {\n\t1 + 2\n\tfmt.Println(3)\n}"), false},
	{"bare-name", prog(`"fmt"`, "func main() {\n\tx := 1\n\tx\n\tfmt.Println(x)\n}"), false},
	{"bare-compare", prog(`"fmt"`, "func main() {\n\tx := 1\n\tx == 2\n\tfmt.Println(x)\n}"), false},
	{"call-statement", prog(`"fmt"`, "func f() int { return 1 }\n\nfunc main() {\n\tf()\n\tfmt.Println(2)\n}"), false},
	{"closure-return", prog(`"fmt"`, "func main() {\n\tf := func(a int) int { return a + 1 }\n\tfmt.Println(f(1))\n}"), false},
	{"closure-missing-return", prog(`"fmt"`, "func main() {\n\tf := func(a int) int { fmt.Println(a) }\n\tfmt.Println(f(1))\n}"), false},
	{"no-result-ok", prog(`"fmt"`, "func say(s string) { fmt.Println(s) }\n\nfunc main() { say(\"a\") }"), false},
	{"city-ok", prog(`"fmt"`, "func main() {\n\tfor i := 0; i < 3; i++ {\n\t\tSetGreen(i, i*2)\n\t}\n\tfmt.Println(\"done\")\n}"), true},
	{"city-unused-var", prog(`"fmt"`, "func main() {\n\tn := 5\n\tSetGreen(0, 0)\n\tfmt.Println(1)\n}"), true},
	{"city-missing-return", prog(`"fmt"`, "func lights(n int) int {\n\tSetGreen(n, 0)\n}\n\nfunc main() { fmt.Println(lights(1)) }"), true},
}

// TestParityWithGc builds every corpus program with the real Go toolchain
// and checks labcore accepts exactly the programs gc accepts.
func TestParityWithGc(t *testing.T) {
	gobin, err := exec.LookPath("go")
	if err != nil {
		t.Skip("go toolchain not on PATH")
	}
	city := cityFile(t)
	for _, c := range corpus {
		t.Run(c.name, func(t *testing.T) {
			dir := t.TempDir()
			names := []string{"main.go"}
			files := []File{{Name: "main.go", Source: c.src}}
			if c.city {
				names = append([]string{"city.go"}, names...)
				files = append([]File{city}, files...)
				if err := os.WriteFile(filepath.Join(dir, "city.go"), []byte(city.Source), 0o644); err != nil {
					t.Fatal(err)
				}
			}
			if err := os.WriteFile(filepath.Join(dir, "main.go"), []byte(c.src), 0o644); err != nil {
				t.Fatal(err)
			}
			cmd := exec.Command(gobin, append([]string{"build", "-o", os.DevNull}, names...)...)
			cmd.Dir = dir
			cmd.Env = append(os.Environ(), "GOWORK=off", "GO111MODULE=off", "GOFLAGS=")
			out, gcErr := cmd.CombinedOutput()
			gcOK := gcErr == nil

			r := Run(context.Background(), files)
			labOK := r.ExitCode == 0
			if gcOK != labOK {
				t.Fatalf("gc ok=%v, labcore ok=%v\ngc: %s\nlabcore: %+v", gcOK, labOK, out, r)
			}
		})
	}
}
