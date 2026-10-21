// Package labcore runs learner Go code with the Yaegi interpreter.
//
// It is plain Go with no js/wasm dependency, so it is tested natively; the
// browser entry point (Task 4) wraps Run.
package labcore

import (
	"bytes"
	"context"
	"fmt"
	"go/parser"
	"go/token"
	"strconv"
	"strings"
	"sync"
	"testing/fstest"

	"github.com/traefik/yaegi/interp"
)

// MaxOutput is the combined stdout+stderr cap in bytes.
const MaxOutput = 65536

// File is one Go source file. Files are evaluated in the order given.
type File struct{ Name, Source string }

// Result is what a run produced. ExitCode is 0 on success, 1 on any error
// (compile error, panic, forbidden import, output cap) and 2 on timeout.
type Result struct {
	Stdout, Stderr      string
	ExitCode            int
	TimedOut, Truncated bool
}

// capped is a writer pair sharing one byte budget across stdout and stderr.
type capped struct {
	mu        sync.Mutex
	out, err  bytes.Buffer
	used      int
	truncated bool
	stop      context.CancelFunc
}

type stream struct {
	c   *capped
	buf *bytes.Buffer
}

func (s stream) Write(p []byte) (int, error) {
	c := s.c
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.truncated {
		return 0, fmt.Errorf("output limit reached")
	}
	room := MaxOutput - c.used
	if len(p) > room {
		s.buf.Write(p[:room])
		c.used = MaxOutput
		c.truncated = true
		c.stop() // end the program instead of discarding bytes forever
		return room, fmt.Errorf("output limit reached")
	}
	s.buf.Write(p)
	c.used += len(p)
	return len(p), nil
}

// Run evaluates files in order in one interpreter. Cancelling ctx (or its
// deadline passing) stops the run and sets TimedOut.
//
// main(): Yaegi executes the main function of a `package main` source as
// part of evaluating that source. Callers therefore pass the file that
// declares main last (city.go, setup.go, main.go), so main runs exactly
// once, after everything it depends on has been evaluated.
//
// Timeouts: EvalWithContext returns as soon as ctx is done, even if the
// interpreter goroutine has not noticed yet; a loop that never calls a
// function may keep spinning in the background until the host (the web
// worker in Task 4) is terminated. Run itself always returns promptly.
func Run(ctx context.Context, files []File) Result {
	runCtx, stop := context.WithCancel(ctx)
	defer stop()
	w := &capped{stop: stop}

	finish := func(code int, errText string) Result {
		w.mu.Lock()
		defer w.mu.Unlock()
		res := Result{Stdout: w.out.String(), Stderr: w.err.String(), ExitCode: code, Truncated: w.truncated}
		if errText != "" {
			if res.Stderr != "" && !strings.HasSuffix(res.Stderr, "\n") {
				res.Stderr += "\n"
			}
			res.Stderr += errText
		}
		if ctx.Err() != nil && !w.truncated {
			res.TimedOut = true
			res.ExitCode = 2
		} else if w.truncated && res.ExitCode == 0 {
			res.ExitCode = 1
		}
		return res
	}

	// Refuse forbidden imports before anything runs, with a clear message.
	for _, f := range files {
		if msg := checkImports(f); msg != "" {
			return finish(1, msg)
		}
	}
	for _, f := range files {
		if msgs := unusedVariables(f); len(msgs) > 0 {
			return finish(1, strings.Join(msgs, "\n")+"\n")
		}
	}

	mem := make(fstest.MapFS, len(files))
	for _, f := range files {
		mem[f.Name] = &fstest.MapFile{Data: []byte(f.Source)}
	}
	i := interp.New(interp.Options{
		SourcecodeFilesystem: mem, // evaluate files by name so each keeps its own scope and positions
		Stdout:               stream{w, &w.out},
		Stderr:               stream{w, &w.err},
		GoPath:               "/nonexistent", // never resolve imports from disk
		Unrestricted:         false,
	})
	if err := i.Use(allowedSymbols()); err != nil {
		return finish(1, err.Error())
	}

	for _, f := range files {
		if _, err := i.EvalPathWithContext(runCtx, f.Name); err != nil {
			if runCtx.Err() != nil {
				return finish(1, "")
			}
			return finish(1, formatError(f.Name, err))
		}
	}
	return finish(0, "")
}

// checkImports parses only the import block and rejects anything outside
// the allow-list. Syntax errors are left for the interpreter to report.
func checkImports(f File) string {
	af, err := parser.ParseFile(token.NewFileSet(), f.Name, f.Source, parser.ImportsOnly)
	if err != nil {
		return ""
	}
	for _, imp := range af.Imports {
		path, err := strconv.Unquote(imp.Path.Value)
		if err == nil && !isAllowed(path) {
			return fmt.Sprintf("import %q is not allowed\n", path)
		}
	}
	return ""
}

// formatError drops the "<file>:" prefix so errors read "LINE:COL: message".
func formatError(name string, err error) string {
	s := strings.TrimSpace(err.Error())
	s = strings.TrimPrefix(s, name+":")
	return s + "\n"
}
