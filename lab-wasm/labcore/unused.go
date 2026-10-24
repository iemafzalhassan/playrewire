package labcore

import (
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"go/types"
	"regexp"
	"sort"
	"strings"
)

// stubImporter hands go/types an empty, complete package for every import.
// Yaegi v0.16.1 (the latest release) accepts several programs gc rejects:
// unused variables, unused imports, functions that can fall off the end
// without returning, and expression statements whose value is thrown away.
// The lessons rely on gc's behavior, so we type-check each file ourselves
// and keep only those diagnostics (go/types shares gc's checker, so the
// wording is gc's). Every other error is expected noise (selectors on the
// stub packages, names from other files) and is dropped; Yaegi remains the
// authority on everything else.
type stubImporter struct{}

func (stubImporter) Import(path string) (*types.Package, error) {
	name := path[strings.LastIndex(path, "/")+1:]
	p := types.NewPackage(path, name)
	p.MarkComplete()
	return p, nil
}

// gcOnly matches the go/types messages Yaegi does not produce itself.
var gcOnly = regexp.MustCompile(`^(declared and not used: \w+|missing return|.+ is not used|"[^"]+" imported (as \w+ )?and not used)$`)

// gcChecks returns "LINE:COL: msg" lines, in source order, for the gc
// errors Yaegi misses: "declared and not used: x", "missing return",
// "n * 3 (value of type int) is not used" and `"strings" imported and not used`.
func gcChecks(f File) []string {
	fset := token.NewFileSet()
	af, err := parser.ParseFile(fset, f.Name, f.Source, 0)
	if err != nil {
		return nil // syntax errors are reported by the interpreter
	}
	type diag struct {
		pos token.Position
		msg string
	}
	var found []diag
	conf := types.Config{
		Importer: stubImporter{},
		Error: func(err error) {
			te, ok := err.(types.Error)
			if !ok || !gcOnly.MatchString(te.Msg) {
				return
			}
			found = append(found, diag{fset.Position(te.Pos), te.Msg})
		},
	}
	_, _ = conf.Check("main", fset, []*ast.File{af}, nil)
	sort.Slice(found, func(a, b int) bool {
		if found[a].pos.Line != found[b].pos.Line {
			return found[a].pos.Line < found[b].pos.Line
		}
		return found[a].pos.Column < found[b].pos.Column
	})
	out := make([]string, len(found))
	for i, d := range found {
		out[i] = fmt.Sprintf("%d:%d: %s", d.pos.Line, d.pos.Column, d.msg)
	}
	return out
}
