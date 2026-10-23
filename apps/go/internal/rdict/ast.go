package rdict

import "encoding/hex"

// Binary AST decoder, mirroring core/src/ast.rs decode_* functions
// (rdict-spec.md §6). Decode failures make the entry opaque (§6.3).

// Flag masks (ast.rs).
const (
	entryHasSee        byte = 1 << 0
	entryHasTag        byte = 1 << 1
	entryHasMedia      byte = 1 << 2
	entryHasPron       byte = 1 << 3
	entryHasEty        byte = 1 << 4
	entryHasMorphology byte = 1 << 5
	entryHasRelations  byte = 1 << 6
	entryKnownMask          = entryHasSee | entryHasTag | entryHasMedia | entryHasPron | entryHasEty | entryHasMorphology | entryHasRelations

	etyHasID    byte = 1 << 0
	etyHasRoot  byte = 1 << 1
	etyHasSense byte = 1 << 2
	etyKnown         = etyHasID | etyHasRoot | etyHasSense

	morphHasKind  byte = 1 << 0
	morphKnown         = morphHasKind
	senseHasLemma      = 1 << 0
	senseHasTr         = 1 << 1
	senseHasForm       = 1 << 2
	senseHasTag        = 1 << 3
	senseHasPron       = 1 << 4
	senseKnown         = senseHasLemma | senseHasTr | senseHasForm | senseHasTag | senseHasPron

	defHasID      byte = 1 << 0
	defHasExample byte = 1 << 1
	defHasNote    byte = 1 << 2
	defHasMedia   byte = 1 << 3
	defKnown           = defHasID | defHasExample | defHasNote | defHasMedia

	groupHasID   byte = 1 << 0
	groupHasDesc byte = 1 << 1
	groupKnown        = groupHasID | groupHasDesc

	exHasTr     byte = 1 << 0
	exHasPron   byte = 1 << 1
	exHasMedia  byte = 1 << 2
	exHasTarget byte = 1 << 3
	exKnown          = exHasTr | exHasPron | exHasMedia | exHasTarget

	noteHasID byte = 1 << 0
	noteHasEx byte = 1 << 1
	noteKnown      = noteHasID | noteHasEx

	trHasPron     byte = 1 << 0
	trKnown            = trHasPron
	pronHasLang   byte = 1 << 0
	pronHasAccent byte = 1 << 1
	pronHasMedia  byte = 1 << 2
	pronKnown          = pronHasLang | pronHasAccent | pronHasMedia

	formHasTag   byte = 1 << 0
	formHasFeats byte = 1 << 1
	formHasPron  byte = 1 << 2
	formKnown         = formHasTag | formHasFeats | formHasPron

	mediaHasDesc byte = 1 << 0
	mediaHasAlt  byte = 1 << 1
	mediaKnown        = mediaHasDesc | mediaHasAlt

	defKindDefinition byte = 0
	defKindGroup      byte = 1
)

// decodeEntry decodes one entry body. Unknown Entry flag bits make the
// entry opaque (returns errOpaque); unknown nested flags or a malformed
// slice are errors the caller may also treat as opaque (§6.3).
func decodeEntry(headword string, buf []byte, p *stringPools) (*Entry, error) {
	r := &boundedReader{buf: buf}
	flags, err := r.readU8()
	if err != nil {
		return nil, err
	}
	if flags&^entryKnownMask != 0 {
		return nil, errOpaque
	}
	e := &Entry{Headword: headword}
	if flags&entryHasSee != 0 {
		if e.See, err = readOptStr(r); err != nil {
			return nil, err
		}
	}
	if flags&entryHasTag != 0 {
		n, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range n {
			idx, err := r.readU16()
			if err != nil {
				return nil, err
			}
			s, err := resolveRef(p.tag, idx)
			if err != nil {
				return nil, err
			}
			e.Tags = append(e.Tags, s)
		}
	}
	if flags&entryHasMedia != 0 {
		if e.Media, err = readMediaRefs(r); err != nil {
			return nil, err
		}
	}
	if flags&entryHasPron != 0 {
		if e.Pron, err = readProns(r, p); err != nil {
			return nil, err
		}
	}
	if flags&entryHasEty != 0 {
		n, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range n {
			ety, err := decodeEty(r, p)
			if err != nil {
				return nil, err
			}
			e.Etymologies = append(e.Etymologies, *ety)
		}
	}
	if flags&entryHasMorphology != 0 {
		n, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range n {
			m, err := decodeMorpheme(r, p)
			if err != nil {
				return nil, err
			}
			e.Morphology = append(e.Morphology, *m)
		}
	}
	if flags&entryHasRelations != 0 {
		n, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range n {
			rel, err := decodeRelation(r, p)
			if err != nil {
				return nil, err
			}
			e.Relations = append(e.Relations, *rel)
		}
	}
	return e, nil
}

func readOptStr(r *boundedReader) (*string, error) {
	s, err := r.readStr()
	if err != nil {
		return nil, err
	}
	return &s, nil
}

func decodeMorpheme(r *boundedReader, p *stringPools) (*Morpheme, error) {
	flags, err := r.readU8()
	if err != nil {
		return nil, err
	}
	if flags&^morphKnown != 0 {
		return nil, errMalformed("morpheme: unknown flag bit")
	}
	idx, err := r.readU16()
	if err != nil {
		return nil, err
	}
	kind, err := resolveRef(p.formKind, idx)
	if err != nil {
		return nil, err
	}
	term, err := r.readStr()
	if err != nil {
		return nil, err
	}
	m := &Morpheme{Term: term}
	if kind != "" {
		m.Kind = &kind
	}
	return m, nil
}

func decodeRelation(r *boundedReader, p *stringPools) (*Relation, error) {
	idx, err := r.readU16()
	if err != nil {
		return nil, err
	}
	typ, err := resolveRef(p.relationType, idx)
	if err != nil {
		return nil, err
	}
	target, err := r.readStr()
	if err != nil {
		return nil, err
	}
	return &Relation{Type: typ, Target: target}, nil
}

func decodeEty(r *boundedReader, p *stringPools) (*Ety, error) {
	flags, err := r.readU8()
	if err != nil {
		return nil, err
	}
	if flags&^etyKnown != 0 {
		return nil, errMalformed("ety: unknown flag bit")
	}
	e := &Ety{}
	if flags&etyHasID != 0 {
		if e.ID, err = readOptStr(r); err != nil {
			return nil, err
		}
	}
	if flags&etyHasRoot != 0 {
		if e.Root, err = readOptStr(r); err != nil {
			return nil, err
		}
	}
	if flags&etyHasSense != 0 {
		n, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range n {
			s, err := decodeSense(r, p)
			if err != nil {
				return nil, err
			}
			e.Senses = append(e.Senses, *s)
		}
	}
	return e, nil
}

func decodeSense(r *boundedReader, p *stringPools) (*Sense, error) {
	flags, err := r.readU8()
	if err != nil {
		return nil, err
	}
	if flags&^senseKnown != 0 {
		return nil, errMalformed("sense: unknown flag bit")
	}
	s := &Sense{}
	posIdx, err := r.readU16()
	if err != nil {
		return nil, err
	}
	if s.Pos, err = optRef(p.pos, posIdx); err != nil {
		return nil, err
	}
	if flags&senseHasLemma != 0 {
		if s.Lemma, err = readOptStr(r); err != nil {
			return nil, err
		}
	}
	if flags&senseHasTr != 0 {
		n, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range n {
			tr, err := decodeTranslation(r, p)
			if err != nil {
				return nil, err
			}
			s.Translations = append(s.Translations, *tr)
		}
	}
	if flags&senseHasForm != 0 {
		n, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range n {
			f, err := decodeForm(r, p)
			if err != nil {
				return nil, err
			}
			s.Forms = append(s.Forms, *f)
		}
	}
	if flags&senseHasTag != 0 {
		n, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range n {
			t, err := r.readStr()
			if err != nil {
				return nil, err
			}
			s.Tags = append(s.Tags, t)
		}
	}
	if flags&senseHasPron != 0 {
		if s.Pron, err = readProns(r, p); err != nil {
			return nil, err
		}
	}
	n, err := r.readVarint()
	if err != nil {
		return nil, err
	}
	for range n {
		d, err := decodeDefItem(r, p)
		if err != nil {
			return nil, err
		}
		s.Definitions = append(s.Definitions, *d)
	}
	return s, nil
}

func decodeDefItem(r *boundedReader, p *stringPools) (*DefItem, error) {
	kind, err := r.readU8()
	if err != nil {
		return nil, err
	}
	switch kind {
	case defKindDefinition:
		d, err := decodeDefinition(r, p)
		if err != nil {
			return nil, err
		}
		return &DefItem{Definition: d}, nil
	case defKindGroup:
		g, err := decodeGroup(r, p)
		if err != nil {
			return nil, err
		}
		return &DefItem{Group: g}, nil
	default:
		return nil, errMalformed("unknown Def kind")
	}
}

func decodeDefinition(r *boundedReader, p *stringPools) (*Definition, error) {
	flags, err := r.readU8()
	if err != nil {
		return nil, err
	}
	if flags&^defKnown != 0 {
		return nil, errMalformed("definition: unknown flag bit")
	}
	d := &Definition{}
	if d.Value, err = r.readStr(); err != nil {
		return nil, err
	}
	if flags&defHasID != 0 {
		if d.ID, err = readOptStr(r); err != nil {
			return nil, err
		}
	}
	if flags&defHasExample != 0 {
		n, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range n {
			ex, err := decodeExample(r, p)
			if err != nil {
				return nil, err
			}
			d.Examples = append(d.Examples, *ex)
		}
	}
	if flags&defHasNote != 0 {
		n, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range n {
			nt, err := decodeNote(r, p)
			if err != nil {
				return nil, err
			}
			d.Notes = append(d.Notes, *nt)
		}
	}
	if flags&defHasMedia != 0 {
		if d.Media, err = readMediaRefs(r); err != nil {
			return nil, err
		}
	}
	return d, nil
}

func decodeGroup(r *boundedReader, p *stringPools) (*DefGroup, error) {
	flags, err := r.readU8()
	if err != nil {
		return nil, err
	}
	if flags&^groupKnown != 0 {
		return nil, errMalformed("group: unknown flag bit")
	}
	g := &DefGroup{}
	if flags&groupHasID != 0 {
		if g.ID, err = readOptStr(r); err != nil {
			return nil, err
		}
	}
	if flags&groupHasDesc != 0 {
		if g.Description, err = readOptStr(r); err != nil {
			return nil, err
		}
	}
	n, err := r.readVarint()
	if err != nil {
		return nil, err
	}
	for range n {
		d, err := decodeDefItem(r, p)
		if err != nil {
			return nil, err
		}
		g.Definitions = append(g.Definitions, *d)
	}
	return g, nil
}

func decodeExample(r *boundedReader, p *stringPools) (*Example, error) {
	flags, err := r.readU8()
	if err != nil {
		return nil, err
	}
	if flags&^exKnown != 0 {
		return nil, errMalformed("example: unknown flag bit")
	}
	e := &Example{}
	if e.Value, err = r.readStr(); err != nil {
		return nil, err
	}
	if flags&exHasTr != 0 {
		n, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range n {
			tr, err := decodeTranslation(r, p)
			if err != nil {
				return nil, err
			}
			e.Translations = append(e.Translations, *tr)
		}
	}
	if flags&exHasPron != 0 {
		if e.Pron, err = readProns(r, p); err != nil {
			return nil, err
		}
	}
	if flags&exHasMedia != 0 {
		if e.Media, err = readMediaRefs(r); err != nil {
			return nil, err
		}
	}
	if flags&exHasTarget != 0 {
		n, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range n {
			spanCount, err := r.readVarint()
			if err != nil {
				return nil, err
			}
			occ := TargetOccurrence{}
			for range spanCount {
				offset, err := r.readVarint()
				if err != nil {
					return nil, err
				}
				length, err := r.readVarint()
				if err != nil {
					return nil, err
				}
				occ.Spans = append(occ.Spans, TextSpan{Offset: offset, Length: length})
			}
			e.Targets = append(e.Targets, occ)
		}
	}
	return e, nil
}

func decodeNote(r *boundedReader, p *stringPools) (*Note, error) {
	flags, err := r.readU8()
	if err != nil {
		return nil, err
	}
	if flags&^noteKnown != 0 {
		return nil, errMalformed("note: unknown flag bit")
	}
	n := &Note{}
	if n.Value, err = r.readStr(); err != nil {
		return nil, err
	}
	if flags&noteHasID != 0 {
		if n.ID, err = readOptStr(r); err != nil {
			return nil, err
		}
	}
	if flags&noteHasEx != 0 {
		count, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range count {
			ex, err := decodeExample(r, p)
			if err != nil {
				return nil, err
			}
			n.Examples = append(n.Examples, *ex)
		}
	}
	return n, nil
}

func decodeTranslation(r *boundedReader, p *stringPools) (*Translation, error) {
	flags, err := r.readU8()
	if err != nil {
		return nil, err
	}
	if flags&^trKnown != 0 {
		return nil, errMalformed("translation: unknown flag bit")
	}
	t := &Translation{}
	langIdx, err := r.readU16()
	if err != nil {
		return nil, err
	}
	if t.Lang, err = optRef(p.lang, langIdx); err != nil {
		return nil, err
	}
	if t.Value, err = r.readStr(); err != nil {
		return nil, err
	}
	if flags&trHasPron != 0 {
		if t.Pron, err = readProns(r, p); err != nil {
			return nil, err
		}
	}
	return t, nil
}

func decodePron(r *boundedReader, p *stringPools) (*Pron, error) {
	flags, err := r.readU8()
	if err != nil {
		return nil, err
	}
	if flags&^pronKnown != 0 {
		return nil, errMalformed("pron: unknown flag bit")
	}
	pr := &Pron{}
	if flags&pronHasLang != 0 {
		idx, err := r.readU16()
		if err != nil {
			return nil, err
		}
		if pr.Lang, err = optRef(p.lang, idx); err != nil {
			return nil, err
		}
	}
	if flags&pronHasAccent != 0 {
		if pr.Accent, err = readOptStr(r); err != nil {
			return nil, err
		}
	}
	kindIdx, err := r.readU16()
	if err != nil {
		return nil, err
	}
	if pr.Kind, err = optRef(p.pronKind, kindIdx); err != nil {
		return nil, err
	}
	if pr.Value, err = r.readStr(); err != nil {
		return nil, err
	}
	if flags&pronHasMedia != 0 {
		if pr.Media, err = readMediaRefs(r); err != nil {
			return nil, err
		}
	}
	return pr, nil
}

func decodeForm(r *boundedReader, p *stringPools) (*Form, error) {
	flags, err := r.readU8()
	if err != nil {
		return nil, err
	}
	if flags&^formKnown != 0 {
		return nil, errMalformed("form: unknown flag bit")
	}
	f := &Form{}
	kindIdx, err := r.readU16()
	if err != nil {
		return nil, err
	}
	if f.Kind, err = optRef(p.formKind, kindIdx); err != nil {
		return nil, err
	}
	if f.Term, err = r.readStr(); err != nil {
		return nil, err
	}
	if flags&formHasTag != 0 {
		n, err := r.readVarint()
		if err != nil {
			return nil, err
		}
		for range n {
			t, err := r.readStr()
			if err != nil {
				return nil, err
			}
			f.Tags = append(f.Tags, t)
		}
	}
	if flags&formHasFeats != 0 {
		if f.Feats, err = readOptStr(r); err != nil {
			return nil, err
		}
	}
	if flags&formHasPron != 0 {
		if f.Pron, err = readProns(r, p); err != nil {
			return nil, err
		}
	}
	return f, nil
}

func readProns(r *boundedReader, p *stringPools) ([]Pron, error) {
	n, err := r.readVarint()
	if err != nil {
		return nil, err
	}
	v := make([]Pron, 0, n)
	for range n {
		pr, err := decodePron(r, p)
		if err != nil {
			return nil, err
		}
		v = append(v, *pr)
	}
	return v, nil
}

func readMediaRefs(r *boundedReader) ([]MediaRef, error) {
	n, err := r.readVarint()
	if err != nil {
		return nil, err
	}
	v := make([]MediaRef, 0, n)
	for range n {
		m, err := decodeMediaRef(r)
		if err != nil {
			return nil, err
		}
		v = append(v, *m)
	}
	return v, nil
}

func decodeMediaRef(r *boundedReader) (*MediaRef, error) {
	flags, err := r.readU8()
	if err != nil {
		return nil, err
	}
	if flags&^mediaKnown != 0 {
		return nil, errMalformed("media ref: unknown flag bit")
	}
	kindByte, err := r.readU8()
	if err != nil {
		return nil, err
	}
	var kind string
	switch kindByte {
	case 0:
		kind = "audio"
	case 1:
		kind = "image"
	case 2:
		kind = "video"
	default:
		return nil, errMalformed("invalid media kind")
	}
	hash, err := r.readBytes(20)
	if err != nil {
		return nil, err
	}
	m := &MediaRef{Kind: kind, Hash: hex.EncodeToString(hash)}
	if flags&mediaHasDesc != 0 {
		if m.Description, err = readOptStr(r); err != nil {
			return nil, err
		}
	}
	if flags&mediaHasAlt != 0 {
		if m.Alt, err = readOptStr(r); err != nil {
			return nil, err
		}
	}
	return m, nil
}

// optRef resolves a strref to *string: nil for 0 (unspecified), a
// non-nil pointer for a real value. Mirrors Rust Option<String>.
func optRef(seg []string, idx uint16) (*string, error) {
	s, err := resolveRef(seg, idx)
	if err != nil || s == "" {
		return nil, err
	}
	return &s, nil
}
