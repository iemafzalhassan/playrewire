package labcore

import (
	"reflect"
	"strings"

	"github.com/traefik/yaegi/stdlib"
)

// allowedPackages is the allow-list of import paths learner code may use.
// Anything else (os, net, syscall, unsafe, io/ioutil, os/exec, ...) is
// refused. Never add a package that reaches the filesystem, network or
// processes.
var allowedPackages = []string{
	"fmt", "strings", "strconv", "math", "math/rand", "sort", "slices", "maps",
	"errors", "unicode", "unicode/utf8", "time", "bytes",
	"container/list", "container/heap",
}

func isAllowed(path string) bool {
	for _, p := range allowedPackages {
		if p == path {
			return true
		}
	}
	return false
}

// allowedSymbols filters stdlib.Symbols down to the allow-list. Yaegi keys
// the map as "<import path>/<package name>", e.g. "math/rand/rand".
func allowedSymbols() map[string]map[string]reflect.Value {
	out := make(map[string]map[string]reflect.Value, len(allowedPackages))
	for key, syms := range stdlib.Symbols {
		i := strings.LastIndex(key, "/")
		if i < 0 {
			continue
		}
		if isAllowed(key[:i]) {
			out[key] = syms
		}
	}
	return out
}
