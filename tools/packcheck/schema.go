package main

// YAML schema for lesson packs. The same structs carry the bundle's JSON
// tags, so the bundle is "everything in the YAML plus pack and steps", with
// camelCase keys. Fields tagged yaml:"-" exist only in the bundle, and
// strict decoding (KnownFields) rejects them if an author writes them.

// Version is the bundle format version.
const Version = 1

// WrapperLineOffset is the number of wrapper lines above user line 1: user
// line 1 is file line 8.
const WrapperLineOffset = 7

const wrapperHead = "package main\n\nimport \"fmt\"\n\nvar _ = fmt.Sprint\n\nfunc main() {\n"
const wrapperTail = "\n}\n"

// Step names, in bundle order.
const (
	StepSee   = "see"
	StepType  = "type"
	StepDrill = "drill"
	StepUse   = "use"
)

type PathRef struct {
	Pack  string `yaml:"pack" json:"pack"`
	Chunk string `yaml:"chunk" json:"chunk"`
}

type PackMeta struct {
	ID       string `yaml:"id" json:"id"`
	Title    string `yaml:"title" json:"title"`
	District string `yaml:"district" json:"district"`
}

type See struct {
	Scene   string `yaml:"scene" json:"scene"`
	Caption string `yaml:"caption" json:"caption"`
}

type Tip struct {
	Match string `yaml:"match" json:"match"`
	Text  string `yaml:"text" json:"text"`
}

type TypeStep struct {
	Ghost     string `yaml:"ghost" json:"ghost"`
	Tips      []Tip  `yaml:"tips" json:"tips"`
	WholeFile bool   `yaml:"whole_file" json:"wholeFile"`
}

type Drill struct {
	Prompt       string `yaml:"prompt" json:"prompt"`
	Starter      string `yaml:"starter,omitempty" json:"starter,omitempty"`
	Solution     string `yaml:"solution" json:"solution"`
	ExpectStdout string `yaml:"expect_stdout" json:"expectStdout"`
	WholeFile    bool   `yaml:"whole_file" json:"wholeFile"`
}

type FixIt struct {
	Broken       string `yaml:"broken" json:"broken"`
	Solution     string `yaml:"solution" json:"solution"`
	ExpectStdout string `yaml:"expect_stdout" json:"expectStdout"`
	WholeFile    bool   `yaml:"whole_file" json:"wholeFile"`
}

type Chunk struct {
	ID      string    `yaml:"id" json:"id"`
	Concept string    `yaml:"concept" json:"concept"`
	Title   string    `yaml:"title" json:"title"`
	See     See       `yaml:"see" json:"see"`
	Type    *TypeStep `yaml:"type,omitempty" json:"type,omitempty"`
	Drills  []Drill   `yaml:"drills" json:"drills"`
	Hints   []string  `yaml:"hints" json:"hints"`
	FixIt   *FixIt    `yaml:"fix_it,omitempty" json:"fixIt,omitempty"`
	Mission string    `yaml:"mission,omitempty" json:"mission,omitempty"`

	// Bundle only.
	Pack  string   `yaml:"-" json:"pack"`
	Steps []string `yaml:"-" json:"steps"`
	file  string   // path relative to the content root, for error messages
}

type Level struct {
	Template  string `yaml:"template" json:"template"`
	Solution  string `yaml:"solution" json:"solution"`
	WholeFile bool   `yaml:"whole_file" json:"wholeFile"`
}

type Levels struct {
	FillBlank      Level `yaml:"fill_blank" json:"fillBlank"`
	HalfFunction   Level `yaml:"half_function" json:"halfFunction"`
	BlankWithHints Level `yaml:"blank_with_hints" json:"blankWithHints"`
}

// CityEvent is one @city event, kept as a generic map so it round-trips to
// the exact JSON the city stub prints.
type CityEvent map[string]any

type Mission struct {
	ID           string      `yaml:"id" json:"id"`
	Concept      string      `yaml:"concept" json:"concept"`
	Terminal     string      `yaml:"terminal" json:"terminal"`
	Title        string      `yaml:"title" json:"title"`
	Setup        string      `yaml:"setup" json:"setup"`
	Levels       Levels      `yaml:"levels" json:"levels"`
	ExpectEvents []CityEvent `yaml:"expect_events" json:"expectEvents"`

	Pack string `yaml:"-" json:"pack"`
	file string
}

type BundlePack struct {
	PackMeta
	Chunks   []*Chunk   `json:"chunks"`
	Missions []*Mission `json:"missions"`
}

type Bundle struct {
	Version           int          `json:"version"`
	Language          string       `json:"language"`
	WrapperLineOffset int          `json:"wrapperLineOffset"`
	CityStub          string       `json:"cityStub"`
	Path              []PathRef    `json:"path"`
	Packs             []BundlePack `json:"packs"`
}

// wrapMain wraps body-of-main code in the fixed wrapper, unless the code is a
// whole file.
func wrapMain(code string, wholeFile bool) string {
	if wholeFile {
		return code
	}
	for len(code) > 0 && code[len(code)-1] == '\n' {
		code = code[:len(code)-1]
	}
	return wrapperHead + code + wrapperTail
}
