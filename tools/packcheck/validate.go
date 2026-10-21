package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strings"

	"gopkg.in/yaml.v3"
)

// Issue is one validation error. Index is -1 when the error is not about a
// list item.
type Issue struct {
	File  string // relative to the content root
	Kind  string // "chunk", "mission" or ""
	ID    string
	Index int
	Item  string // "drills", "tips", "hints", "levels", "expect_events", ...
	Msg   string
	seq   int
}

func (i Issue) String() string {
	var b strings.Builder
	b.WriteString(i.File)
	b.WriteString(": ")
	if i.Kind != "" {
		fmt.Fprintf(&b, "%s %q", i.Kind, i.ID)
		if i.Item != "" {
			b.WriteString(" ")
			b.WriteString(i.Item)
			if i.Index >= 0 {
				fmt.Fprintf(&b, "[%d]", i.Index)
			}
		}
		b.WriteString(": ")
	}
	b.WriteString(i.Msg)
	return b.String()
}

// Pack is one loaded pack directory.
type Pack struct {
	Meta     PackMeta
	Chunks   []*Chunk
	Missions []*Mission
}

// Loaded is a parsed content root.
type Loaded struct {
	Root     string
	CityStub string
	Path     []PathRef
	Packs    []*Pack
}

// issues collects errors with a stable order.
type issues struct {
	list []Issue
	seq  int
}

func (c *issues) add(is Issue) {
	c.seq++
	is.seq = c.seq
	c.list = append(c.list, is)
}

func (c *issues) sorted() []Issue {
	out := append([]Issue(nil), c.list...)
	sort.SliceStable(out, func(a, b int) bool {
		x, y := out[a], out[b]
		if x.File != y.File {
			return x.File < y.File
		}
		if x.ID != y.ID {
			return x.ID < y.ID
		}
		if x.Index != y.Index {
			return x.Index < y.Index
		}
		return x.seq < y.seq
	})
	return out
}

// decodeStrict decodes one YAML document and rejects unknown keys.
func decodeStrict(path string, v any) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	dec := yaml.NewDecoder(bytes.NewReader(data))
	dec.KnownFields(true)
	if err := dec.Decode(v); err != nil {
		if errors.Is(err, io.EOF) {
			return errors.New("file is empty")
		}
		return err
	}
	return nil
}

func sortedYAML(dir string) ([]string, error) {
	ents, err := os.ReadDir(dir)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil
		}
		return nil, err
	}
	var out []string
	for _, e := range ents {
		if !e.IsDir() && strings.HasSuffix(e.Name(), ".yaml") {
			out = append(out, e.Name())
		}
	}
	sort.Strings(out)
	return out, nil
}

// load reads the content root. Problems are added to c; load keeps going so
// one run reports everything.
func load(root string, c *issues) *Loaded {
	l := &Loaded{Root: root}
	rel := func(p string) string { r, _ := filepath.Rel(root, p); return filepath.ToSlash(r) }

	pathFile := filepath.Join(root, "path.yaml")
	if _, err := os.Stat(pathFile); err != nil {
		c.add(Issue{File: "path.yaml", Index: -1, Msg: "no path.yaml in " + root})
	} else if err := decodeStrict(pathFile, &l.Path); err != nil {
		c.add(Issue{File: "path.yaml", Index: -1, Msg: err.Error()})
	}

	cityFile := filepath.Join(root, "city", "city.go")
	if b, err := os.ReadFile(cityFile); err != nil {
		c.add(Issue{File: "city/city.go", Index: -1, Msg: "cannot read city stub: " + err.Error()})
	} else {
		l.CityStub = string(b)
	}

	ents, err := os.ReadDir(filepath.Join(root, "packs"))
	if err != nil {
		c.add(Issue{File: "packs", Index: -1, Msg: "cannot read packs directory: " + err.Error()})
		return l
	}
	for _, e := range ents {
		if !e.IsDir() {
			continue
		}
		pdir := filepath.Join(root, "packs", e.Name())
		p := &Pack{}
		metaFile := filepath.Join(pdir, "pack.yaml")
		if err := decodeStrict(metaFile, &p.Meta); err != nil {
			c.add(Issue{File: rel(metaFile), Index: -1, Msg: err.Error()})
			continue
		}
		if p.Meta.ID != e.Name() {
			c.add(Issue{File: rel(metaFile), Index: -1, Msg: fmt.Sprintf("pack id %q must match its directory name %q", p.Meta.ID, e.Name())})
		}
		if p.Meta.Title == "" || p.Meta.District == "" {
			c.add(Issue{File: rel(metaFile), Index: -1, Msg: "pack needs a title and a district"})
		}

		names, _ := sortedYAML(filepath.Join(pdir, "chunks"))
		for _, n := range names {
			f := filepath.Join(pdir, "chunks", n)
			ch := &Chunk{file: rel(f)}
			if err := decodeStrict(f, ch); err != nil {
				c.add(Issue{File: ch.file, Index: -1, Msg: err.Error()})
				continue
			}
			ch.Pack = p.Meta.ID
			p.Chunks = append(p.Chunks, ch)
		}
		names, _ = sortedYAML(filepath.Join(pdir, "missions"))
		for _, n := range names {
			f := filepath.Join(pdir, "missions", n)
			m := &Mission{file: rel(f)}
			if err := decodeStrict(f, m); err != nil {
				c.add(Issue{File: m.file, Index: -1, Msg: err.Error()})
				continue
			}
			m.Pack = p.Meta.ID
			p.Missions = append(p.Missions, m)
		}
		l.Packs = append(l.Packs, p)
	}
	return l
}

// Validate loads root, checks structure, then runs every solution with the
// real Go toolchain. It returns the loaded content (usable only when the
// issue list is empty) and every issue found, sorted.
func Validate(root string) (*Loaded, []Issue) {
	c := &issues{}
	l := load(root, c)
	var jobs []*job

	// Pack ids and per-pack id uniqueness.
	packs := map[string]*Pack{}
	for _, p := range l.Packs {
		if _, dup := packs[p.Meta.ID]; dup {
			c.add(Issue{File: "packs/" + p.Meta.ID, Index: -1, Msg: "duplicate pack id " + p.Meta.ID})
		}
		packs[p.Meta.ID] = p
	}

	for _, p := range l.Packs {
		missions := map[string]*Mission{}
		for _, m := range p.Missions {
			if prev, dup := missions[m.ID]; dup {
				c.add(Issue{File: m.file, Kind: "mission", ID: m.ID, Index: -1, Msg: "duplicate mission id (also in " + prev.file + ")"})
			}
			missions[m.ID] = m
			jobs = append(jobs, validateMission(c, l, m)...)
		}
		seen := map[string]bool{}
		for _, ch := range p.Chunks {
			if seen[ch.ID] {
				c.add(Issue{File: ch.file, Kind: "chunk", ID: ch.ID, Index: -1, Msg: "duplicate chunk id in pack " + p.Meta.ID})
			}
			seen[ch.ID] = true
			jobs = append(jobs, validateChunk(c, ch, missions)...)
			ch.Steps = deriveSteps(ch)
		}
	}

	// path.yaml references.
	inPath := map[PathRef]bool{}
	for i, r := range l.Path {
		is := Issue{File: "path.yaml", Kind: "", Index: i}
		if inPath[r] {
			is.Msg = fmt.Sprintf("path[%d]: duplicate entry %s/%s", i, r.Pack, r.Chunk)
			c.add(is)
		}
		inPath[r] = true
		p, ok := packs[r.Pack]
		if !ok {
			is.Msg = fmt.Sprintf("path[%d]: unknown pack %q", i, r.Pack)
			c.add(is)
			continue
		}
		found := false
		for _, ch := range p.Chunks {
			if ch.ID == r.Chunk {
				found = true
			}
		}
		if !found {
			is.Msg = fmt.Sprintf("path[%d]: pack %q has no chunk %q", i, r.Pack, r.Chunk)
			c.add(is)
		}
	}

	// Toolchain jobs; only worth running when the shape is sound enough to
	// report per-item results.
	for _, is := range runJobs(jobs) {
		c.add(is)
	}
	return l, c.sorted()
}

func deriveSteps(ch *Chunk) []string {
	steps := []string{StepSee}
	if ch.Type != nil {
		steps = append(steps, StepType)
	}
	if len(ch.Drills) > 0 {
		steps = append(steps, StepDrill)
	}
	if ch.Mission != "" {
		steps = append(steps, StepUse)
	}
	return steps
}

func validateChunk(c *issues, ch *Chunk, missions map[string]*Mission) []*job {
	var jobs []*job
	at := func(item string, idx int, msg string) {
		c.add(Issue{File: ch.file, Kind: "chunk", ID: ch.ID, Item: item, Index: idx, Msg: msg})
	}
	if ch.ID == "" || ch.Concept == "" || ch.Title == "" {
		at("", -1, "id, concept and title are required")
	}
	if ch.See.Scene == "" || ch.See.Caption == "" {
		at("see", -1, "see.scene and see.caption are required")
	}

	if ch.Type != nil {
		t := ch.Type
		if strings.TrimSpace(t.Ghost) == "" {
			at("type", -1, "ghost is required")
		} else {
			jobs = append(jobs, &job{
				issue: Issue{File: ch.file, Kind: "chunk", ID: ch.ID, Item: "type", Index: -1},
				mode:  modeBuild,
				files: map[string]string{"main.go": wrapMain(t.Ghost, t.WholeFile)},
				check: func(r Result) string {
					if r.Err != nil {
						return "ghost does not compile: " + failureDetail(r)
					}
					return ""
				},
			})
		}
		for i, tip := range t.Tips {
			if tip.Match == "" || tip.Text == "" {
				at("tips", i, "tip needs match and text")
			}
		}
	}

	if len(ch.Drills) == 0 {
		if len(ch.Hints) > 0 {
			at("hints", -1, "hints are only allowed on a chunk with drills")
		}
		if ch.FixIt != nil {
			at("fix_it", -1, "fix_it is only allowed on a chunk with drills")
		}
	} else {
		if len(ch.Drills) < 5 {
			at("drills", -1, fmt.Sprintf("needs at least 5 drills, has %d", len(ch.Drills)))
		}
		if len(ch.Hints) != 3 {
			at("hints", -1, fmt.Sprintf("needs exactly 3 hints, has %d", len(ch.Hints)))
		}
		for i, h := range ch.Hints {
			if strings.TrimSpace(h) == "" {
				at("hints", i, "hint is empty")
			}
		}
	}

	for i, d := range ch.Drills {
		if d.Prompt == "" || strings.TrimSpace(d.Solution) == "" {
			at("drills", i, "prompt and solution are required")
			continue
		}
		want := d.ExpectStdout
		jobs = append(jobs, &job{
			issue: Issue{File: ch.file, Kind: "chunk", ID: ch.ID, Item: "drills", Index: i},
			mode:  modeRun,
			files: map[string]string{"main.go": wrapMain(d.Solution, d.WholeFile)},
			check: func(r Result) string { return checkPasses("solution", r, want) },
		})
	}

	if f := ch.FixIt; f != nil {
		if strings.TrimSpace(f.Broken) == "" || strings.TrimSpace(f.Solution) == "" {
			at("fix_it", -1, "broken and solution are required")
		} else {
			want := f.ExpectStdout
			jobs = append(jobs,
				&job{
					issue: Issue{File: ch.file, Kind: "chunk", ID: ch.ID, Item: "fix_it.broken", Index: -1},
					mode:  modeRun,
					files: map[string]string{"main.go": wrapMain(f.Broken, f.WholeFile)},
					check: func(r Result) string {
						if r.Err == nil && normalizeOutput(r.Stdout) == normalizeOutput(want) {
							return "broken code already prints expect_stdout; it must fail to compile or print something else"
						}
						return ""
					},
				},
				&job{
					issue: Issue{File: ch.file, Kind: "chunk", ID: ch.ID, Item: "fix_it.solution", Index: -1},
					mode:  modeRun,
					files: map[string]string{"main.go": wrapMain(f.Solution, f.WholeFile)},
					check: func(r Result) string { return checkPasses("solution", r, want) },
				})
		}
	}

	if ch.Mission != "" {
		if _, ok := missions[ch.Mission]; !ok {
			at("mission", -1, fmt.Sprintf("mission %q does not exist in this pack", ch.Mission))
		}
	}
	return jobs
}

func checkPasses(what string, r Result, want string) string {
	if r.Err != nil {
		return what + " does not run: " + failureDetail(r)
	}
	if got := normalizeOutput(r.Stdout); got != normalizeOutput(want) {
		return fmt.Sprintf("%s prints the wrong output\n  want: %q\n  got:  %q", what, normalizeOutput(want), got)
	}
	return ""
}

var eventShapes = map[string][]string{
	"green": {"delay", "i", "op"},
	"lamp":  {"i", "on", "op"},
	"say":   {"op", "text"},
}

func validateMission(c *issues, l *Loaded, m *Mission) []*job {
	var jobs []*job
	at := func(item string, idx int, msg string) {
		c.add(Issue{File: m.file, Kind: "mission", ID: m.ID, Item: item, Index: idx, Msg: msg})
	}
	if m.ID == "" || m.Concept == "" || m.Terminal == "" || m.Title == "" {
		at("", -1, "id, concept, terminal and title are required")
	}
	if len(m.ExpectEvents) == 0 {
		at("expect_events", -1, "needs at least one expected event")
	}
	for i, ev := range m.ExpectEvents {
		op, _ := ev["op"].(string)
		want, ok := eventShapes[op]
		if !ok {
			at("expect_events", i, fmt.Sprintf("unknown op %q (want green, lamp or say)", op))
			continue
		}
		keys := make([]string, 0, len(ev))
		for k := range ev {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		if !reflect.DeepEqual(keys, want) {
			at("expect_events", i, fmt.Sprintf("op %q needs exactly the keys %v, has %v", op, want, keys))
		}
	}

	levels := []struct {
		name  string
		lv    Level
		blank bool
	}{
		{"fill_blank", m.Levels.FillBlank, false},
		{"half_function", m.Levels.HalfFunction, false},
		{"blank_with_hints", m.Levels.BlankWithHints, true},
	}
	setup := m.Setup
	if strings.TrimSpace(setup) == "" {
		setup = "package main\n"
	}
	for _, lv := range levels {
		if strings.TrimSpace(lv.lv.Template) == "" && !lv.blank {
			at("levels."+lv.name, -1, "template must not be empty")
		}
		if strings.TrimSpace(lv.lv.Solution) == "" {
			at("levels."+lv.name, -1, "solution is required")
			continue
		}
		jobs = append(jobs, &job{
			issue: Issue{File: m.file, Kind: "mission", ID: m.ID, Item: "levels." + lv.name, Index: -1},
			mode:  modeRun,
			files: map[string]string{
				"city.go":  l.CityStub,
				"setup.go": setup,
				"main.go":  wrapMain(lv.lv.Solution, lv.lv.WholeFile),
			},
			check: func(r Result) string { return checkEvents(r, m.ExpectEvents) },
		})
	}
	return jobs
}

// parseEvents extracts the @city lines from stdout.
func parseEvents(stdout string) ([]any, error) {
	var out []any
	for _, line := range strings.Split(strings.ReplaceAll(stdout, "\r\n", "\n"), "\n") {
		rest, ok := strings.CutPrefix(line, "@city ")
		if !ok {
			continue
		}
		var v any
		if err := json.Unmarshal([]byte(rest), &v); err != nil {
			return nil, fmt.Errorf("bad @city line %q: %v", line, err)
		}
		out = append(out, v)
	}
	return out, nil
}

func checkEvents(r Result, expect []CityEvent) string {
	if r.Err != nil {
		return "solution does not run: " + failureDetail(r)
	}
	got, err := parseEvents(r.Stdout)
	if err != nil {
		return err.Error()
	}
	// Round-trip the expectation through JSON so numbers compare the same way.
	b, _ := json.Marshal(expect)
	var want []any
	_ = json.Unmarshal(b, &want)
	if len(got) == 0 && len(want) == 0 {
		return ""
	}
	if !reflect.DeepEqual(got, want) {
		gb, _ := json.Marshal(got)
		return fmt.Sprintf("@city events do not match expect_events\n  want: %s\n  got:  %s", b, gb)
	}
	return ""
}
