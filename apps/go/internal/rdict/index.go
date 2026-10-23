package rdict

import (
	"bytes"
	"encoding/binary"
)

// headwordIndex is the in-memory reader over index/headword.idx,
// mirroring core/src/index.rs IndexReader::from_bytes (§4).
type headwordIndex struct {
	entryCount uint32
	directory  []dirEntry
	bytes      []byte
	bodyStart  int
}

// dirEntry is the fixed 24-byte block directory entry (§4.2).
type dirEntry struct {
	firstHeadwordLen    uint16
	firstHeadwordOffset uint64 // absolute offset of this block's body
	blockEntryCount     uint32
	poolOffset          uint64 // absolute offset of the first headword in the pool
}

const (
	idxHeaderSize   = 24
	idxDirEntrySize = 24
)

func parseHeadwordIndex(buf []byte) (*headwordIndex, error) {
	if len(buf) < idxHeaderSize {
		return nil, errMalformed("headword.idx too small")
	}
	if string(buf[:4]) != "RDID" {
		return nil, errMalformed("headword.idx: bad magic")
	}
	if v := binary.LittleEndian.Uint16(buf[4:6]); v != 1 {
		return nil, errMalformed("headword.idx: unsupported version")
	}
	entryCount := binary.LittleEndian.Uint32(buf[8:12])
	blockCount := binary.LittleEndian.Uint32(buf[12:16])

	dirEnd := idxHeaderSize + idxDirEntrySize*int(blockCount)
	if dirEnd > len(buf) {
		return nil, errMalformed("headword.idx: directory past end")
	}
	directory := make([]dirEntry, 0, blockCount)
	for i := range int(blockCount) {
		b := buf[idxHeaderSize+i*idxDirEntrySize:][:idxDirEntrySize]
		directory = append(directory, dirEntry{
			firstHeadwordLen:    binary.LittleEndian.Uint16(b[0:2]),
			firstHeadwordOffset: binary.LittleEndian.Uint64(b[4:12]),
			blockEntryCount:     binary.LittleEndian.Uint32(b[12:16]),
			poolOffset:          binary.LittleEndian.Uint64(b[16:24]),
		})
	}

	// Body start: max(pool_offset + first_headword_len) across the
	// directory, or the end of the directory when block_count == 0.
	bodyStart := dirEnd
	if blockCount > 0 {
		maxEnd := 0
		for _, d := range directory {
			end := int(d.poolOffset) + int(d.firstHeadwordLen)
			if end > maxEnd {
				maxEnd = end
			}
		}
		if maxEnd > len(buf) || maxEnd < dirEnd {
			return nil, errMalformed("headword.idx: pool/body range invalid")
		}
		for _, d := range directory {
			if int(d.poolOffset) < dirEnd ||
				int(d.poolOffset)+int(d.firstHeadwordLen) > len(buf) ||
				int(d.firstHeadwordOffset) < maxEnd ||
				int(d.firstHeadwordOffset) > len(buf) {
				return nil, errMalformed("headword.idx: block out of bounds")
			}
		}
		bodyStart = maxEnd
	}

	return &headwordIndex{
		entryCount: entryCount,
		directory:  directory,
		bytes:      buf,
		bodyStart:  bodyStart,
	}, nil
}

// firstHeadword reads a block's first headword from the pool.
func (ix *headwordIndex) firstHeadword(d dirEntry) ([]byte, bool) {
	start := int(d.poolOffset)
	end := start + int(d.firstHeadwordLen)
	if start < 0 || end > len(ix.bytes) || end < start {
		return nil, false
	}
	return ix.bytes[start:end], true
}

// entryLoc is (data_block_id, offset, size) for one entry.
type entryLoc struct {
	blockID uint32
	offset  uint64
	size    uint64
}

// decodeEntry decodes one front-coded entry at the reader, returning the
// reconstructed headword and its location.
func decodeIndexEntry(r *boundedReader, prev []byte) (hw []byte, loc entryLoc, err error) {
	shared, err := r.readVarint()
	if err != nil {
		return nil, loc, err
	}
	suffixLen, err := r.readVarint()
	if err != nil {
		return nil, loc, err
	}
	suffix, err := r.readBytes(int(suffixLen))
	if err != nil {
		return nil, loc, err
	}
	blockID, err := r.readVarint()
	if err != nil {
		return nil, loc, err
	}
	offset, err := r.readVarint()
	if err != nil {
		return nil, loc, err
	}
	size, err := r.readVarint()
	if err != nil {
		return nil, loc, err
	}
	if int(shared) > len(prev) {
		return nil, loc, errMalformed("shared prefix exceeds previous headword")
	}
	hw = make([]byte, 0, int(shared)+len(suffix))
	hw = append(hw, prev[:shared]...)
	hw = append(hw, suffix...)
	return hw, entryLoc{uint32(blockID), offset, size}, nil
}

// blockReader returns a bounded reader over a block's body.
func (ix *headwordIndex) blockReader(d dirEntry) (*boundedReader, bool) {
	start := int(d.firstHeadwordOffset)
	if start < ix.bodyStart || start > len(ix.bytes) {
		return nil, false
	}
	return &boundedReader{buf: ix.bytes[start:]}, true
}

// lookup finds a headword by case-folded block search + case-sensitive
// exact match (§4.4). Returns the location, or found=false.
func (ix *headwordIndex) lookup(headword string) (entryLoc, bool) {
	if len(ix.directory) == 0 {
		return entryLoc{}, false
	}
	needleLower := toLowerBytes(headword)
	lo, hi := 0, len(ix.directory)
	for lo+1 < hi {
		mid := (lo + hi) / 2
		first, ok := ix.firstHeadword(ix.directory[mid])
		if !ok {
			return entryLoc{}, false
		}
		if bytes.Compare(toLowerBytes(string(first)), needleLower) <= 0 {
			lo = mid
		} else {
			hi = mid
		}
	}
	block := ix.directory[lo]
	r, ok := ix.blockReader(block)
	if !ok {
		return entryLoc{}, false
	}
	var prev []byte
	for range block.blockEntryCount {
		hw, loc, err := decodeIndexEntry(r, prev)
		if err != nil {
			return entryLoc{}, false
		}
		currentLower := toLowerBytes(string(hw))
		if bytes.Equal(currentLower, needleLower) && string(hw) == headword {
			return loc, true
		}
		if bytes.Compare(currentLower, needleLower) > 0 {
			return entryLoc{}, false // passed the needle; sorted order
		}
		prev = hw
	}
	return entryLoc{}, false
}

// prefixSearch returns up to limit headwords (original case, sorted)
// whose lowercased form starts with prefix, skipping the first skip
// matches (§4.5). Empty prefix = top of the list.
func (ix *headwordIndex) prefixSearch(prefix string, skip, limit int) []string {
	if limit <= 0 || len(ix.directory) == 0 {
		return nil
	}
	prefixLower := toLowerBytes(prefix)
	if len(prefixLower) == 0 {
		return ix.headwordsFromBlock(0, skip, limit)
	}

	// Binary search: first block whose lowercased first headword >= prefix.
	lo, hi := 0, len(ix.directory)
	for lo < hi {
		mid := (lo + hi) / 2
		first, ok := ix.firstHeadword(ix.directory[mid])
		if !ok {
			return nil
		}
		if bytes.Compare(toLowerBytes(string(first)), prefixLower) >= 0 {
			hi = mid
		} else {
			lo = mid + 1
		}
	}
	// The match may start one block earlier (block boundary straddling).
	startBlock := lo
	if startBlock > 0 {
		startBlock--
	}

	var results []string
	skipped := 0
	for _, block := range ix.directory[startBlock:] {
		r, ok := ix.blockReader(block)
		if !ok {
			break
		}
		var prev []byte
		for range block.blockEntryCount {
			hw, _, err := decodeIndexEntry(r, prev)
			if err != nil {
				return results
			}
			currentLower := toLowerBytes(string(hw))
			if len(currentLower) >= len(prefixLower) &&
				bytes.Equal(currentLower[:len(prefixLower)], prefixLower) {
				if skipped < skip {
					skipped++
				} else {
					results = append(results, string(hw))
					if len(results) >= limit {
						return results
					}
				}
			} else if bytes.Compare(currentLower, prefixLower) > 0 {
				return results // passed all matches
			}
			prev = hw
		}
	}
	return results
}

// headwordsFromBlock collects up to limit headwords in index order
// starting at startBlock, skipping the first skip. Backs the
// empty-prefix case.
func (ix *headwordIndex) headwordsFromBlock(startBlock, skip, limit int) []string {
	var out []string
	skipped := 0
	for _, block := range ix.directory[startBlock:] {
		r, ok := ix.blockReader(block)
		if !ok {
			break
		}
		var prev []byte
		for range block.blockEntryCount {
			hw, _, err := decodeIndexEntry(r, prev)
			if err != nil {
				return out
			}
			if skipped < skip {
				skipped++
			} else {
				out = append(out, string(hw))
			}
			prev = hw
			if len(out) >= limit {
				return out
			}
		}
	}
	return out
}
