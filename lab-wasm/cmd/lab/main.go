//go:build js && wasm

// Command lab is the browser entry point. It registers a global
// rewireRun(filesJSON) -> Promise<resultJSON> and then parks forever.
//
// Timeouts are NOT handled here. js/wasm has no goroutine preemption, so a
// tight learner loop may never let Go notice a context deadline. The JS side
// (webRunner.ts) owns the wall timer and kills the worker instead.
package main

import (
	"context"
	"encoding/json"
	"syscall/js"

	"github.com/iemafzalhassan/playrewire/lab-wasm/labcore"
)

type fileIn struct {
	Name   string `json:"name"`
	Source string `json:"source"`
}

type resultOut struct {
	Stdout    string `json:"stdout"`
	Stderr    string `json:"stderr"`
	ExitCode  int    `json:"exitCode"`
	TimedOut  bool   `json:"timedOut"`
	Truncated bool   `json:"truncated"`
}

func run(filesJSON string) string {
	var in []fileIn
	if err := json.Unmarshal([]byte(filesJSON), &in); err != nil {
		b, _ := json.Marshal(resultOut{Stderr: "bad request: " + err.Error() + "\n", ExitCode: 1})
		return string(b)
	}
	files := make([]labcore.File, len(in))
	for i, f := range in {
		files[i] = labcore.File{Name: f.Name, Source: f.Source}
	}
	r := labcore.Run(context.Background(), files)
	b, _ := json.Marshal(resultOut{r.Stdout, r.Stderr, r.ExitCode, r.TimedOut, r.Truncated})
	return string(b)
}

func main() {
	js.Global().Set("rewireRun", js.FuncOf(func(this js.Value, args []js.Value) any {
		filesJSON := args[0].String()
		handler := js.FuncOf(func(_ js.Value, p []js.Value) any {
			resolve := p[0]
			// Run in a goroutine so the JS callback returns immediately.
			go func() { resolve.Invoke(run(filesJSON)) }()
			return nil
		})
		return js.Global().Get("Promise").New(handler)
	}))
	js.Global().Set("rewireReady", true)
	select {}
}
