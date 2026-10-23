package rdict

// Model types for the decoded entry AST (rdict-spec.md §6). Fields are
// populated by the binary decoder in ast.go; JSON tags remain for the
// entry AST shapes documented in the spec's JSON source form.

// Entry is one dictionary entry.
type Entry struct {
	Headword    string     `json:"headword"`
	See         *string    `json:"see"`
	Tags        []string   `json:"tags"`
	Pron        []Pron     `json:"pron"`
	Etymologies []Ety      `json:"etymologies"`
	Morphology  []Morpheme `json:"morphology"`
	Relations   []Relation `json:"relations"`
	Media       []MediaRef `json:"media"`
}

type Relation struct {
	Type   string `json:"type"`
	Target string `json:"target"`
}

type MediaRef struct {
	Kind        string  `json:"kind"`
	Hash        string  `json:"hash"`
	Description *string `json:"description"`
	Alt         *string `json:"alt"`
}

type Morpheme struct {
	Kind *string `json:"kind"`
	Term string  `json:"term"`
}

type Ety struct {
	ID     *string `json:"id"`
	Root   *string `json:"root"`
	Senses []Sense `json:"senses"`
}

type Sense struct {
	Pos          *string       `json:"pos"`
	Lemma        *string       `json:"lemma"`
	Translations []Translation `json:"translations"`
	Forms        []Form        `json:"forms"`
	Tags         []string      `json:"tags"`
	Pron         []Pron        `json:"pron"`
	Definitions  []DefItem     `json:"definitions"`
}

// DefItem is the tagged union Definition | Group (§6.7). Exactly one
// field is non-nil after binary decoding.
type DefItem struct {
	Definition *Definition
	Group      *DefGroup
}

type Definition struct {
	ID       *string    `json:"id"`
	Value    string     `json:"value"`
	Examples []Example  `json:"examples"`
	Notes    []Note     `json:"notes"`
	Media    []MediaRef `json:"media"`
}

type Example struct {
	Value        string             `json:"value"`
	Translations []Translation      `json:"translations"`
	Pron         []Pron             `json:"pron"`
	Targets      []TargetOccurrence `json:"targets"`
	Media        []MediaRef         `json:"media"`
}

// TargetOccurrence spans are UTF-8 byte offsets into Example.Value.
type TargetOccurrence struct {
	Spans []TextSpan `json:"spans"`
}

type TextSpan struct {
	Offset uint64 `json:"offset"`
	Length uint64 `json:"length"`
}

type Note struct {
	ID       *string   `json:"id"`
	Value    string    `json:"value"`
	Examples []Example `json:"examples"`
}

type DefGroup struct {
	ID          *string   `json:"id"`
	Description *string   `json:"description"`
	Definitions []DefItem `json:"definitions"`
}

type Translation struct {
	Lang  *string `json:"lang"`
	Value string  `json:"value"`
	Pron  []Pron  `json:"pron"`
}

type Pron struct {
	Lang   *string    `json:"lang"`
	Accent *string    `json:"accent"`
	Kind   *string    `json:"kind"`
	Value  string     `json:"value"`
	Media  []MediaRef `json:"media"`
}

type Form struct {
	Kind  *string  `json:"kind"`
	Term  string   `json:"term"`
	Tags  []string `json:"tags"`
	Feats *string  `json:"feats"`
	Pron  []Pron   `json:"pron"`
}

// ManifestInfo is the response of rdict_manifest.
type ManifestInfo struct {
	OK          bool     `json:"ok"`
	Error       *string  `json:"error"`
	Name        string   `json:"name"`
	SourceLang  string   `json:"source_lang"`
	TargetLangs []string `json:"target_langs"`
	Version     string   `json:"version"`
	EntryCount  int      `json:"entry_count"`
}
