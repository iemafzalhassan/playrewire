package main

import (
	"bytes"
	"encoding/json"
)

// BuildBundle turns validated content into the bundle the UI loads.
func BuildBundle(l *Loaded) Bundle {
	b := Bundle{
		Version:           Version,
		Language:          "go",
		WrapperLineOffset: WrapperLineOffset,
		CityStub:          l.CityStub,
		Path:              l.Path,
		Packs:             []BundlePack{},
	}
	if b.Path == nil {
		b.Path = []PathRef{}
	}
	for _, p := range l.Packs {
		bp := BundlePack{PackMeta: p.Meta, Chunks: p.Chunks, Missions: p.Missions}
		if bp.Chunks == nil {
			bp.Chunks = []*Chunk{}
		}
		if bp.Missions == nil {
			bp.Missions = []*Mission{}
		}
		for _, ch := range bp.Chunks {
			if ch.Drills == nil {
				ch.Drills = []Drill{}
			}
			if ch.Hints == nil {
				ch.Hints = []string{}
			}
			if ch.Type != nil && ch.Type.Tips == nil {
				ch.Type.Tips = []Tip{}
			}
		}
		for _, m := range bp.Missions {
			if m.ExpectEvents == nil {
				m.ExpectEvents = []CityEvent{}
			}
		}
		b.Packs = append(b.Packs, bp)
	}
	return b
}

// MarshalBundle renders the bundle as stable, indented JSON.
func MarshalBundle(b Bundle) ([]byte, error) {
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	enc.SetIndent("", "  ")
	if err := enc.Encode(b); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}
