package labcore

import (
	"context"
	"os"
	"regexp"
	"strings"
	"testing"
	"time"
)

// wrap builds the exact wrapper main.go has: user line 1 is file line 8.
func wrap(code string) string {
	return "package main\n\nimport \"fmt\"\n\nvar _ = fmt.Sprint\n\nfunc main() {\n" + code + "\n}\n"
}

func cityFile(t *testing.T) File {
	t.Helper()
	b, err := os.ReadFile("../../content/go/city/city.go")
	if err != nil {
		t.Fatal(err)
	}
	return File{Name: "city.go", Source: string(b)}
}

func TestHello(t *testing.T) {
	r := Run(context.Background(), []File{{Name: "main.go", Source: wrap(`fmt.Println("hi")`)}})
	if r.Stdout != "hi\n" || r.ExitCode != 0 {
		t.Fatalf("got %+v", r)
	}
}

func TestCityEvents(t *testing.T) {
	code := wrap("for i := 0; i < 3; i++ {\nSetGreen(i, i*2)\n}")
	r := Run(context.Background(), []File{cityFile(t), {Name: "main.go", Source: code}})
	if r.ExitCode != 0 {
		t.Fatalf("got %+v", r)
	}
	lines := strings.Split(strings.TrimSpace(r.Stdout), "\n")
	if len(lines) != 3 {
		t.Fatalf("want 3 lines, got %q", r.Stdout)
	}
	for i, l := range lines {
		want := `@city {"op":"green","i":` + string(rune('0'+i)) + `,"delay":` + string(rune('0'+i*2)) + `}`
		if l != want {
			t.Errorf("line %d: %q want %q", i, l, want)
		}
	}
}

func TestMainRunsOnceAcrossFiles(t *testing.T) {
	setup := "package main\n\nimport \"fmt\"\n\nfunc init() { fmt.Println(\"setup\") }\n\nfunc helper() { fmt.Println(\"helper\") }\n"
	r := Run(context.Background(), []File{
		{Name: "setup.go", Source: setup},
		{Name: "main.go", Source: wrap("helper()\nfmt.Println(\"main\")")},
	})
	if r.Stdout != "setup\nhelper\nmain\n" || r.ExitCode != 0 {
		t.Fatalf("got %+v", r)
	}
}

func TestSayEscapesJSON(t *testing.T) {
	r := Run(context.Background(), []File{cityFile(t), {Name: "main.go", Source: wrap("Say(\"a\\\"b\\\\c\\nd\")")}})
	want := "@city {\"op\":\"say\",\"text\":\"a\\\"b\\\\c\\nd\"}\n"
	if r.Stdout != want {
		t.Fatalf("got %q want %q (%+v)", r.Stdout, want, r)
	}
	r = Run(context.Background(), []File{cityFile(t), {Name: "main.go", Source: wrap("Lamp(1, true)")}})
	if r.Stdout != "@city {\"op\":\"lamp\",\"i\":1,\"on\":true}\n" {
		t.Fatalf("lamp: %q", r.Stdout)
	}
}

func TestCompileErrorHasPosition(t *testing.T) {
	r := Run(context.Background(), []File{{Name: "main.go", Source: wrap("x := 1")}})
	if r.ExitCode != 1 {
		t.Fatalf("exit %d", r.ExitCode)
	}
	if !regexp.MustCompile(`^\d+:\d+: .*declared and not used`).MatchString(r.Stderr) {
		t.Fatalf("stderr %q", r.Stderr)
	}
}

func TestRuntimePanicIsExit1(t *testing.T) {
	r := Run(context.Background(), []File{{Name: "main.go", Source: wrap("var a []int\n_ = a[3]")}})
	if r.ExitCode != 1 || r.Stderr == "" {
		t.Fatalf("got %+v", r)
	}
}

func TestTimeout(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 200*time.Millisecond)
	defer cancel()
	start := time.Now()
	r := Run(ctx, []File{{Name: "main.go", Source: wrap("for {}")}})
	if !r.TimedOut || r.ExitCode != 2 {
		t.Fatalf("got %+v", r)
	}
	if d := time.Since(start); d > 2*time.Second {
		t.Fatalf("took %v", d)
	}
}

func TestOutputCap(t *testing.T) {
	r := Run(context.Background(), []File{{Name: "main.go", Source: wrap(`for { fmt.Println("spam spam spam spam") }`)}})
	if !r.Truncated || len(r.Stdout) > 65536 {
		t.Fatalf("truncated=%v len=%d", r.Truncated, len(r.Stdout))
	}
}

func TestNoForbiddenPackages(t *testing.T) {
	code := "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n)\n\nfunc main() {\n\tb, _ := os.ReadFile(\"/etc/passwd\")\n\tfmt.Println(string(b))\n}\n"
	r := Run(context.Background(), []File{{Name: "main.go", Source: code}})
	if r.Stderr == "" || r.ExitCode == 0 {
		t.Fatalf("got %+v", r)
	}
	if strings.Contains(r.Stdout, "root:") || r.Stdout != "" {
		t.Fatalf("leaked: %q", r.Stdout)
	}
	if !strings.Contains(r.Stderr, `"os"`) {
		t.Fatalf("unclear error: %q", r.Stderr)
	}
}

func TestAllowedPackagesWork(t *testing.T) {
	code := "package main\n\nimport (\n\t\"fmt\"\n\t\"sort\"\n\t\"strings\"\n)\n\nfunc main() {\n\ts := []int{3, 1, 2}\n\tsort.Ints(s)\n\tfmt.Println(s, strings.ToUpper(\"ok\"))\n}\n"
	r := Run(context.Background(), []File{{Name: "main.go", Source: code}})
	if r.Stdout != "[1 2 3] OK\n" || r.ExitCode != 0 {
		t.Fatalf("got %+v", r)
	}
}

func TestNoFalseUnusedReports(t *testing.T) {
	code := wrap("x := 2\ny := []int{x}\nfor _, v := range y {\n\tfmt.Println(v)\n}\nf := func(a int) int { return a + x }\nfmt.Println(f(1))")
	r := Run(context.Background(), []File{{Name: "main.go", Source: code}})
	if r.Stdout != "2\n3\n" || r.ExitCode != 0 {
		t.Fatalf("got %+v", r)
	}
}

func TestUnusedInsideCityContextStillReported(t *testing.T) {
	r := Run(context.Background(), []File{cityFile(t), {Name: "main.go", Source: wrap("n := 5\nSetGreen(0, 0)")}})
	if r.ExitCode != 1 || !strings.Contains(r.Stderr, "declared and not used: n") || r.Stdout != "" {
		t.Fatalf("got %+v", r)
	}
}
