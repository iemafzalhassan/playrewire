// Command packcheck validates PlayRewire lesson packs, runs every solution
// with the real Go toolchain, and bundles the packs into one JSON file for
// the UI. It runs in CI and on authors' machines and is never shipped.
package main

import (
	"fmt"
	"os"
	"path/filepath"
)

const usage = `usage:
  packcheck validate <dir>
  packcheck build <dir> -o <file>`

func main() {
	os.Exit(run(os.Args[1:]))
}

func run(args []string) int {
	if len(args) < 2 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	cmd, dir := args[0], args[1]
	out := ""
	rest := args[2:]
	for len(rest) > 0 {
		if rest[0] == "-o" && len(rest) >= 2 && cmd == "build" {
			out, rest = rest[1], rest[2:]
			continue
		}
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	switch cmd {
	case "validate":
	case "build":
		if out == "" {
			fmt.Fprintln(os.Stderr, usage)
			return 2
		}
	default:
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}

	l, problems := Validate(dir)
	if len(problems) > 0 {
		for _, p := range problems {
			fmt.Fprintln(os.Stderr, p)
		}
		fmt.Fprintf(os.Stderr, "packcheck: %d problem(s) in %s\n", len(problems), dir)
		return 1
	}
	if cmd == "validate" {
		fmt.Printf("packcheck: %s ok\n", dir)
		return 0
	}
	data, err := MarshalBundle(BuildBundle(l))
	if err == nil {
		err = os.MkdirAll(filepath.Dir(out), 0o755)
	}
	if err == nil {
		err = os.WriteFile(out, data, 0o644)
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, "packcheck:", err)
		return 1
	}
	fmt.Printf("packcheck: wrote %s\n", out)
	return 0
}
