// Package rdict is a pure Go reader for the .rdict dictionary pack
// format (rdict-spec.md v0.1). It mirrors the Rust core's reader
// (core/src/reader.rs) without cgo: ZIP container via archive/zip,
// zstd data blocks via klauspost/compress, and the binary index,
// string pools, and entry AST decoded per the spec.
package rdict

import (
	"archive/zip"
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"path/filepath"
	"strings"
	"sync"

	"github.com/klauspost/compress/zstd"
)

// Errors mirroring the FFI layer's semantics.
var (
	ErrNotFound = errors.New("rdict: headword not found")
	ErrOpaque   = errors.New("rdict: entry is opaque (unknown format)")

	// errOpaque is the internal sentinel decodeEntry returns for unknown
	// entry flag bits (§6.3).
	errOpaque = errors.New("rdict: opaque entry")
)

// blockCacheMaxBytes caps the decompressed data-block cache (matches
// the Rust core's BLOCK_CACHE_MAX_BYTES).
const blockCacheMaxBytes = 16 << 20

// Dictionary is an open .rdict reader. All methods are safe for
// concurrent use.
type Dictionary struct {
	mu       sync.Mutex
	zr       *zip.ReadCloser
	member   map[string]*zip.File
	manifest packManifest
	pools    *stringPools
	index    *headwordIndex

	// Decompressed data-block cache (LRU by use clock).
	blockCache map[uint32]*cachedBlock
	cacheBytes int
	cacheClock uint64

	mediaManifest *mediaManifest
	decoder       *zstd.Decoder
}

type cachedBlock struct {
	bytes    []byte
	lastUsed uint64
}

// packManifest is the subset of manifest.json the reader needs.
type packManifest struct {
	Pack struct {
		Name        string   `json:"name"`
		Version     string   `json:"version"`
		SourceLang  string   `json:"source_lang"`
		TargetLangs []string `json:"target_langs"`
		Cover       *string  `json:"cover"`
	} `json:"pack"`
	Index struct {
		EntryCount uint32 `json:"entry_count"`
	} `json:"index"`
	Data struct {
		BlockCount           uint32 `json:"block_count"`
		BlockMaxUncompressed uint32 `json:"block_max_uncompressed"`
	} `json:"data"`
}

type mediaManifest struct {
	Entries []struct {
		Hash             string `json:"hash"`
		Kind             string `json:"kind"`
		Ext              string `json:"ext"`
		Mime             string `json:"mime"`
		Compression      string `json:"compression"`
		Size             int64  `json:"size"`
		UncompressedSize int64  `json:"uncompressed_size"`
	} `json:"entries"`
}

// Open opens a .rdict file by path.
func Open(path string) (*Dictionary, error) {
	zr, err := zip.OpenReader(path)
	if err != nil {
		return nil, fmt.Errorf("rdict: open %q: %w", path, err)
	}
	d := &Dictionary{
		zr:         zr,
		member:     make(map[string]*zip.File, len(zr.File)),
		blockCache: map[uint32]*cachedBlock{},
	}
	for _, f := range zr.File {
		d.member[f.Name] = f
	}

	// Verify mimetype (§2.2 rule 5).
	mt, err := d.readMember("mimetype")
	if err != nil {
		d.Close()
		return nil, fmt.Errorf("rdict: %s: %w", path, err)
	}
	if !bytes.Equal(mt, []byte("application/rdict")) {
		d.Close()
		return nil, fmt.Errorf("rdict: %s: bad mimetype", path)
	}

	// manifest.json
	mb, err := d.readMember("manifest.json")
	if err != nil {
		d.Close()
		return nil, fmt.Errorf("rdict: %s: %w", path, err)
	}
	if err := json.Unmarshal(mb, &d.manifest); err != nil {
		d.Close()
		return nil, fmt.Errorf("rdict: %s: manifest: %w", path, err)
	}

	// index/strings.tbl
	sb, err := d.readMember("index/strings.tbl")
	if err != nil {
		d.Close()
		return nil, fmt.Errorf("rdict: %s: %w", path, err)
	}
	if d.pools, err = decodeStringPools(sb); err != nil {
		d.Close()
		return nil, fmt.Errorf("rdict: %s: %w", path, err)
	}

	// index/headword.idx
	ib, err := d.readMember("index/headword.idx")
	if err != nil {
		d.Close()
		return nil, fmt.Errorf("rdict: %s: %w", path, err)
	}
	if d.index, err = parseHeadwordIndex(ib); err != nil {
		d.Close()
		return nil, fmt.Errorf("rdict: %s: %w", path, err)
	}
	if d.index.entryCount != d.manifest.Index.EntryCount {
		d.Close()
		return nil, fmt.Errorf("rdict: %s: entry_count mismatch (manifest=%d index=%d)",
			path, d.manifest.Index.EntryCount, d.index.entryCount)
	}

	d.decoder, err = zstd.NewReader(nil)
	if err != nil {
		d.Close()
		return nil, err
	}
	return d, nil
}

// Close releases the reader. Safe to call twice.
func (d *Dictionary) Close() {
	if d == nil {
		return
	}
	d.mu.Lock()
	defer d.mu.Unlock()
	if d.zr != nil {
		d.zr.Close()
		d.zr = nil
	}
	if d.decoder != nil {
		d.decoder.Close()
		d.decoder = nil
	}
}

func (d *Dictionary) readMember(name string) ([]byte, error) {
	f, ok := d.member[name]
	if !ok {
		return nil, fmt.Errorf("member %q not found", name)
	}
	rc, err := f.Open()
	if err != nil {
		return nil, err
	}
	defer rc.Close()
	return io.ReadAll(rc)
}

// Manifest returns pack metadata.
func (d *Dictionary) Manifest() (*ManifestInfo, error) {
	d.mu.Lock()
	defer d.mu.Unlock()
	return &ManifestInfo{
		OK:          true,
		Name:        d.manifest.Pack.Name,
		SourceLang:  d.manifest.Pack.SourceLang,
		TargetLangs: d.manifest.Pack.TargetLangs,
		Version:     d.manifest.Pack.Version,
		EntryCount:  int(d.manifest.Index.EntryCount),
	}, nil
}

// Lookup finds a headword. Returns ErrNotFound when absent, ErrOpaque
// for entries with unknown flag bits (§6.3).
func (d *Dictionary) Lookup(headword string) (*Entry, error) {
	d.mu.Lock()
	defer d.mu.Unlock()
	loc, ok := d.index.lookup(headword)
	if !ok {
		return nil, ErrNotFound
	}
	block, err := d.ensureBlock(loc.blockID)
	if err != nil {
		return nil, err
	}
	start, end := int(loc.offset), int(loc.offset+loc.size)
	if start < 0 || end > len(block) || end < start {
		return nil, errMalformed("entry slice extends past block")
	}
	e, err := decodeEntry(headword, block[start:end], d.pools)
	if errors.Is(err, errOpaque) {
		return nil, ErrOpaque
	}
	if err != nil {
		// Per §6.3, decode failures make the entry opaque.
		return nil, ErrOpaque
	}
	return e, nil
}

// Prefix does a case-insensitive prefix search with pagination.
// Returns up to limit headwords after skipping the first skip matches.
func (d *Dictionary) Prefix(prefix string, skip, limit int) []string {
	d.mu.Lock()
	defer d.mu.Unlock()
	return d.index.prefixSearch(prefix, skip, limit)
}

// ensureBlock returns the decompressed data block, caching it (LRU).
func (d *Dictionary) ensureBlock(blockID uint32) ([]byte, error) {
	d.cacheClock++
	if b, ok := d.blockCache[blockID]; ok {
		b.lastUsed = d.cacheClock
		return b.bytes, nil
	}
	name := fmt.Sprintf("data/%05d.zst", blockID)
	compressed, err := d.readMember(name)
	if err != nil {
		return nil, fmt.Errorf("rdict: %s: %w", name, err)
	}
	decompressed, err := d.decoder.DecodeAll(compressed, nil)
	if err != nil {
		return nil, fmt.Errorf("rdict: %s: zstd: %w", name, err)
	}
	for len(d.blockCache) > 0 && d.cacheBytes+len(decompressed) > blockCacheMaxBytes {
		var evict uint32
		var oldest uint64
		for id, b := range d.blockCache {
			if oldest == 0 || b.lastUsed < oldest {
				oldest, evict = b.lastUsed, id
			}
		}
		d.cacheBytes -= len(d.blockCache[evict].bytes)
		delete(d.blockCache, evict)
	}
	d.blockCache[blockID] = &cachedBlock{bytes: decompressed, lastUsed: d.cacheClock}
	d.cacheBytes += len(decompressed)
	return decompressed, nil
}

// loadMediaManifest reads media/manifest.json on first use.
func (d *Dictionary) loadMediaManifest() error {
	if d.mediaManifest != nil {
		return nil
	}
	if _, ok := d.member["media/manifest.json"]; !ok {
		d.mediaManifest = &mediaManifest{}
		return nil
	}
	b, err := d.readMember("media/manifest.json")
	if err != nil {
		return err
	}
	var m mediaManifest
	if err := json.Unmarshal(b, &m); err != nil {
		return fmt.Errorf("rdict: media manifest: %w", err)
	}
	d.mediaManifest = &m
	return nil
}

// ReadMedia returns the decompressed bytes of a media file by
// (kind, hash_hex). Returns nil on failure or when absent.
func (d *Dictionary) ReadMedia(kind, hash string) []byte {
	d.mu.Lock()
	defer d.mu.Unlock()
	if err := d.loadMediaManifest(); err != nil {
		return nil
	}
	for _, e := range d.mediaManifest.Entries {
		if e.Kind != kind || !strings.EqualFold(e.Hash, hash) {
			continue
		}
		raw, err := d.readMember(mediaZipPath(kind, e.Hash, e.Ext))
		if err != nil {
			return nil
		}
		switch e.Compression {
		case "none":
			return raw
		case "zstd":
			out, err := d.decoder.DecodeAll(raw, nil)
			if err != nil {
				return nil
			}
			return out
		default:
			return nil
		}
	}
	return nil
}

// mediaZipPath reconstructs the member name per §7.2.
func mediaZipPath(kind, hash, ext string) string {
	hh := "00"
	if len(hash) >= 2 {
		hh = hash[:2]
	}
	return filepath.ToSlash(filepath.Join("media", kind, hh, hash+"."+ext))
}

// ReadCover returns the cover image bytes, or nil.
func (d *Dictionary) ReadCover() []byte {
	d.mu.Lock()
	defer d.mu.Unlock()
	cover := d.manifest.Pack.Cover
	if cover == nil || *cover == "" {
		return nil
	}
	f, ok := d.member[*cover]
	if !ok {
		return nil
	}
	rc, err := f.Open()
	if err != nil {
		return nil
	}
	defer rc.Close()
	b, err := io.ReadAll(rc)
	if err != nil {
		return nil
	}
	return b
}
