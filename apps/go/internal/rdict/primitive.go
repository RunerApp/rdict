package rdict

import "encoding/binary"

// boundedReader reads primitives from a byte slice, failing on
// out-of-bounds access. Mirrors core/src/primitive.rs BoundedReader.
type boundedReader struct {
	buf []byte
	pos int
}

func (r *boundedReader) remaining() int { return len(r.buf) - r.pos }

func (r *boundedReader) readBytes(n int) ([]byte, error) {
	if n < 0 || r.remaining() < n {
		return nil, errMalformed("read past end")
	}
	b := r.buf[r.pos : r.pos+n]
	r.pos += n
	return b, nil
}

func (r *boundedReader) readU8() (byte, error) {
	b, err := r.readBytes(1)
	if err != nil {
		return 0, err
	}
	return b[0], nil
}

func (r *boundedReader) readU16() (uint16, error) {
	b, err := r.readBytes(2)
	if err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint16(b), nil
}

func (r *boundedReader) readU32() (uint32, error) {
	b, err := r.readBytes(4)
	if err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint32(b), nil
}

func (r *boundedReader) readU64() (uint64, error) {
	b, err := r.readBytes(8)
	if err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint64(b), nil
}

// readVarint reads an unsigned LEB128 varint. Non-canonical (overlong,
// >10 bytes, or trailing bits set in the final byte) input is rejected,
// per §6.2.
func (r *boundedReader) readVarint() (uint64, error) {
	var x uint64
	for i := range 10 {
		b, err := r.readU8()
		if err != nil {
			return 0, err
		}
		if i == 9 && b&0x7e != 0 { // 64th bit may only be 0 or 1
			return 0, errMalformed("non-canonical varint")
		}
		x |= uint64(b&0x7f) << (7 * i)
		if b&0x80 == 0 {
			return x, nil
		}
	}
	return 0, errMalformed("varint too long")
}

// readStr reads a length-prefixed UTF-8 string (§6.2 `str`).
func (r *boundedReader) readStr() (string, error) {
	n, err := r.readVarint()
	if err != nil {
		return "", err
	}
	if n > uint64(r.remaining()) {
		return "", errMalformed("string past end")
	}
	b, err := r.readBytes(int(n))
	if err != nil {
		return "", err
	}
	if !utf8Valid(b) {
		return "", errMalformed("invalid UTF-8")
	}
	return string(b), nil
}

func utf8Valid(b []byte) bool {
	for i := 0; i < len(b); {
		if b[i] < 0x80 {
			i++
			continue
		}
		// Multi-byte sequence: use utf8.DecodeRune via string conversion
		// is costly; decode manually.
		r, size := decodeRune(b[i:])
		if r == 0xFFFD && size <= 1 {
			return false
		}
		i += size
	}
	return true
}

// decodeRune decodes one UTF-8 rune, returning RuneError,1 for invalid.
func decodeRune(b []byte) (rune, int) {
	if len(b) == 0 {
		return 0xFFFD, 0
	}
	b0 := b[0]
	switch {
	case b0 < 0x80:
		return rune(b0), 1
	case b0 < 0xC2:
		return 0xFFFD, 1 // continuation or overlong
	case b0 < 0xE0:
		if len(b) < 2 || b[1]&0xC0 != 0x80 {
			return 0xFFFD, 1
		}
		return rune(b0&0x1F)<<6 | rune(b[1]&0x3F), 2
	case b0 < 0xF0:
		if len(b) < 3 || b[1]&0xC0 != 0x80 || b[2]&0xC0 != 0x80 {
			return 0xFFFD, 1
		}
		r := rune(b0&0x0F)<<12 | rune(b[1]&0x3F)<<6 | rune(b[2]&0x3F)
		if r < 0x800 {
			return 0xFFFD, 1 // overlong
		}
		return r, 3
	case b0 < 0xF5:
		if len(b) < 4 || b[1]&0xC0 != 0x80 || b[2]&0xC0 != 0x80 || b[3]&0xC0 != 0x80 {
			return 0xFFFD, 1
		}
		r := rune(b0&0x07)<<18 | rune(b[1]&0x3F)<<12 | rune(b[2]&0x3F)<<6 | rune(b[3]&0x3F)
		if r > 0x10FFFF {
			return 0xFFFD, 1
		}
		return r, 4
	default:
		return 0xFFFD, 1
	}
}
