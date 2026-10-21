//go:build !windows

package main

import (
	"fmt"
	"strconv"
	"strings"
	"syscall"
	"testing"
	"time"
)

func withTimeout(t *testing.T, d time.Duration) {
	t.Helper()
	old := runTimeout
	runTimeout = d
	t.Cleanup(func() { runTimeout = old })
}

func TestRunTimeoutKillsProcessTree(t *testing.T) {
	// Warm the stdlib build cache so the short timeout is spent running,
	// not compiling.
	runGo(modeRun, map[string]string{"main.go": "package main\n\nimport (\"fmt\"; \"os\")\n\nfunc main() { fmt.Println(os.Getpid()) }\n"})
	withTimeout(t, 2*time.Second)

	// A unique comment keeps the result cache from serving an old result.
	src := fmt.Sprintf("package main\n\nimport (\"fmt\"; \"os\")\n\n// %d\nfunc main() { fmt.Println(os.Getpid()); for {} }\n", time.Now().UnixNano())
	start := time.Now()
	res := runGoUncached(modeRun, map[string]string{"main.go": src})
	if took := time.Since(start); took > 4*time.Second {
		t.Fatalf("timeout took %s, want about 2s", took)
	}
	if !res.TimedOut || res.Err == nil || !strings.Contains(res.Err.Error(), "timed out after 2s") {
		t.Fatalf("want TimedOut with a clear error, got %+v", res)
	}
	pid, err := strconv.Atoi(strings.TrimSpace(res.Stdout))
	if err != nil {
		t.Fatalf("program did not print its pid before the timeout (stdout %q, stderr %q)", res.Stdout, res.Stderr)
	}
	deadline := time.Now().Add(3 * time.Second)
	for {
		if err := syscall.Kill(pid, 0); err == syscall.ESRCH {
			return
		}
		if time.Now().After(deadline) {
			syscall.Kill(pid, syscall.SIGKILL)
			t.Fatalf("program %d still alive after the timeout", pid)
		}
		time.Sleep(50 * time.Millisecond)
	}
}

func TestValidatorReportsTimedOutSolution(t *testing.T) {
	root := copyGood(t)
	withTimeout(t, 2*time.Second)
	replace(t, root, printFile, "solution: fmt.Println(42)", "solution: for {}")
	wantProblem(t, root, `chunk "print"`, "drills[1]", "timed out after 2s")
}
