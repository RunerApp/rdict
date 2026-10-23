package rdict

import (
	"fmt"
	"strings"
)

func errMalformed(msg string) error { return fmt.Errorf("rdict: malformed: %s", msg) }

// stringPools mirrors core/src/strings.rs StringPools: six pools, index
// 0 implicit "unspecified", 1..N stored in strings.tbl segments.
type stringPools struct {
	pos          []string
	lang         []string
	pronKind     []string
	formKind     []string
	tag          []string
	relationType []string
}

// decodeStringPools parses index/strings.tbl (§5).
func decodeStringPools(buf []byte) (*stringPools, error) {
	r := &boundedReader{buf: buf}
	magic, err := r.readBytes(4)
	if err != nil || string(magic) != "RDST" {
		return nil, errMalformed("strings.tbl: bad magic")
	}
	version, err := r.readU16()
	if err != nil {
		return nil, err
	}
	if version != 1 {
		return nil, fmt.Errorf("rdict: strings.tbl version %d unsupported", version)
	}
	var counts [6]uint32
	for i := range counts {
		if counts[i], err = r.readU32(); err != nil {
			return nil, err
		}
	}
	p := &stringPools{}
	segs := [6]*[]string{&p.pos, &p.lang, &p.pronKind, &p.formKind, &p.tag, &p.relationType}
	for i, seg := range segs {
		if counts[i] < 1 || counts[i] > 65536 {
			return nil, errMalformed("strings.tbl pool count out of range")
		}
		for range counts[i] - 1 {
			s, err := r.readStr()
			if err != nil {
				return nil, err
			}
			*seg = append(*seg, s)
		}
	}
	return p, nil
}

// resolve maps a 1-based strref to its string, or "" for 0. Out-of-range
// is an error (mirrors StringPools::resolve returning Option).
func resolveRef(seg []string, idx uint16) (string, error) {
	if idx == 0 {
		return "", nil
	}
	if int(idx) > len(seg) {
		return "", errMalformed("strref out of range")
	}
	return seg[idx-1], nil
}

// toLowerBytes lowercases a UTF-8 string by rune, mirroring the Rust
// reader's to_lowercase_bytes for the case-folded index ordering (§4.2.1).
func toLowerBytes(s string) []byte {
	var b strings.Builder
	b.Grow(len(s))
	for _, r := range s {
		if r < 0x80 {
			if r >= 'A' && r <= 'Z' {
				r += 'a' - 'A'
			}
			b.WriteByte(byte(r))
			continue
		}
		b.WriteRune(toLowerRune(r))
	}
	return []byte(b.String())
}

// toLowerRune is a minimal Unicode simple lowercase for the BMP; it
// covers Latin/Greek/Cyrillic ranges and leaves others unchanged.
// Full case-folding tables live in golang.org/x/text; the sort order
// only needs to match the writer's, which used simple per-rune folding.
func toLowerRune(r rune) rune {
	switch {
	case r >= 'A' && r <= 'Z' || r >= 0xC0 && r <= 0xDE && r != 0xD7:
		return r + 0x20
	case r >= 0x391 && r <= 0x3A9 && r != 0x3A2: // Greek
		return r + 0x20
	case r >= 0x410 && r <= 0x42F: // Cyrillic А-Я
		return r + 0x20
	case r == 0x400: // Cyrillic Ѐ
		return 0x450
	default:
		return r
	}
}
