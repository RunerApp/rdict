package rdict

// Tests run with go test ./internal/rdict from the module root.

import (
	"errors"
	"os"
	"testing"
)

// packPath is the fixture pack compiled into the repo, resolved from
// the module root (../../.. = repo root from this package dir).
const packPath = "../../../../compiler/dictionaries/eng-zho-ngsl.rdict"

func openFixture(t *testing.T) *Dictionary {
	t.Helper()
	if _, err := os.Stat(packPath); err != nil {
		t.Skipf("fixture pack missing: %v", err)
	}
	d, err := Open(packPath)
	if err != nil {
		t.Fatalf("open: %v", err)
	}
	t.Cleanup(d.Close)
	return d
}

func TestManifest(t *testing.T) {
	d := openFixture(t)
	m, err := d.Manifest()
	if err != nil {
		t.Fatalf("manifest: %v", err)
	}
	if m.Name == "" || m.EntryCount == 0 {
		t.Fatalf("unexpected manifest: %+v", m)
	}
	t.Logf("manifest: name=%q entries=%d source=%s targets=%v",
		m.Name, m.EntryCount, m.SourceLang, m.TargetLangs)
}

func TestPrefix(t *testing.T) {
	d := openFixture(t)
	hw := d.Prefix("", 0, 5)
	if len(hw) != 5 {
		t.Fatalf("expected 5 headwords, got %d: %v", len(hw), hw)
	}
	t.Logf("first 5: %v", hw)
	hw2 := d.Prefix(hw[0][:1], 0, 200)
	if len(hw2) == 0 {
		t.Fatal("prefix search for first letter returned nothing")
	}
}

func TestLookup(t *testing.T) {
	d := openFixture(t)
	first := d.Prefix("", 0, 1)
	if len(first) == 0 {
		t.Fatal("no headwords")
	}
	e, err := d.Lookup(first[0])
	if err != nil {
		t.Fatalf("lookup %q: %v", first[0], err)
	}
	if e == nil {
		t.Fatalf("lookup %q: not found (unexpected)", first[0])
	}
	if e.Headword != first[0] {
		t.Fatalf("headword mismatch: %q vs %q", e.Headword, first[0])
	}
	if len(e.Etymologies) == 0 {
		t.Logf("warning: entry has no etymologies: %+v", e)
	}
}

func TestLookupMissing(t *testing.T) {
	d := openFixture(t)
	_, err := d.Lookup("zzzznotawordzzzz")
	if !errors.Is(err, ErrNotFound) {
		t.Fatalf("expected ErrNotFound, got err=%v", err)
	}
}

func TestReadCoverAndMedia(t *testing.T) {
	d := openFixture(t)
	cover := d.ReadCover()
	if len(cover) == 0 {
		t.Log("no cover in fixture (ok)")
	} else if cover[0] != 0x89 || cover[1] != 'P' {
		t.Fatalf("cover is not PNG: % x", cover[:8])
	}
}
