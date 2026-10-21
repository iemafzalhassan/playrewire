package main

import (
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
	"sync"
	"time"
)

// runTimeout is the wall limit for one go run / go build.
var runTimeout = 20 * time.Second

type runMode int

const (
	modeRun   runMode = iota // go run .
	modeBuild                // go build, no execution
)

// Result is the outcome of one toolchain invocation.
type Result struct {
	Stdout   string
	Stderr   string
	Err      error // non-nil on compile error, non-zero exit or timeout
	TimedOut bool
}

var resultCache sync.Map // sha256 of mode+files -> Result

func cacheKey(mode runMode, files map[string]string) string {
	names := make([]string, 0, len(files))
	for n := range files {
		names = append(names, n)
	}
	sort.Strings(names)
	h := sha256.New()
	fmt.Fprintf(h, "mode=%d\n", mode)
	for _, n := range names {
		fmt.Fprintf(h, "%s\x00%d\x00%s\x00", n, len(files[n]), files[n])
	}
	return hex.EncodeToString(h.Sum(nil))
}

// runGo writes files plus a go.mod into a fresh temp dir and runs the real Go
// toolchain on them. Identical inputs are only run once per process.
func runGo(mode runMode, files map[string]string) Result {
	key := cacheKey(mode, files)
	if v, ok := resultCache.Load(key); ok {
		return v.(Result)
	}
	res := runGoUncached(mode, files)
	resultCache.Store(key, res)
	return res
}

func runGoUncached(mode runMode, files map[string]string) Result {
	dir, err := os.MkdirTemp("", "packcheck-*")
	if err != nil {
		return Result{Err: err}
	}
	defer os.RemoveAll(dir)

	if err := os.WriteFile(filepath.Join(dir, "go.mod"), []byte("module lab\n\ngo 1.27\n"), 0o644); err != nil {
		return Result{Err: err}
	}
	for name, src := range files {
		if err := os.WriteFile(filepath.Join(dir, name), []byte(src), 0o644); err != nil {
			return Result{Err: err}
		}
	}

	ctx, cancel := context.WithTimeout(context.Background(), runTimeout)
	defer cancel()
	var cmd *exec.Cmd
	if mode == modeBuild {
		cmd = exec.CommandContext(ctx, "go", "build", "-o", os.DevNull, ".")
	} else {
		cmd = exec.CommandContext(ctx, "go", "run", ".")
	}
	cmd.Dir = dir
	cmd.Env = append(os.Environ(), "GOFLAGS=-mod=mod", "CGO_ENABLED=0", "GOTOOLCHAIN=local", "GOWORK=off")
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	setProcessGroup(cmd)
	cmd.WaitDelay = 2 * time.Second

	runErr := cmd.Run()
	res := Result{Stdout: stdout.String(), Stderr: stderr.String(), Err: runErr}
	if ctx.Err() == context.DeadlineExceeded {
		res.TimedOut = true
		res.Err = fmt.Errorf("timed out after %s", runTimeout)
	}
	return res
}

// job is one toolchain invocation plus the check applied to its result.
type job struct {
	issue Issue // location; Msg is filled in by check
	mode  runMode
	files map[string]string
	check func(Result) string // empty string = pass
}

// runJobs runs every job on a bounded worker pool and returns the issues.
func runJobs(jobs []*job) []Issue {
	n := runtime.NumCPU()
	if n < 1 {
		n = 1
	}
	msgs := make([]string, len(jobs))
	idx := make(chan int)
	var wg sync.WaitGroup
	for w := 0; w < n; w++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for i := range idx {
				j := jobs[i]
				msgs[i] = j.check(runGo(j.mode, j.files))
			}
		}()
	}
	for i := range jobs {
		idx <- i
	}
	close(idx)
	wg.Wait()

	var out []Issue
	for i, m := range msgs {
		if m != "" {
			is := jobs[i].issue
			is.Msg = m
			out = append(out, is)
		}
	}
	return out
}

// normalizeOutput makes stdout comparison tolerant of CRLF, trailing
// whitespace on a line, and trailing blank lines.
func normalizeOutput(s string) string {
	s = strings.ReplaceAll(s, "\r\n", "\n")
	lines := strings.Split(s, "\n")
	for i, l := range lines {
		lines[i] = strings.TrimRight(l, " \t\r")
	}
	return strings.TrimRight(strings.Join(lines, "\n"), "\n")
}

// failureDetail summarizes why a run failed, for error messages.
func failureDetail(r Result) string {
	d := strings.TrimSpace(r.Stderr)
	if d == "" {
		d = r.Err.Error()
	}
	if len(d) > 600 {
		d = d[:600] + "..."
	}
	return d
}
