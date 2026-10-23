package main

import (
	"testing"

	"github.com/egoist/mygo/ui"

	"rdictapp/internal/rdict"
)

// TestWordListRendersRows verifies the sidebar word list builds rows
// for a loaded pack: the tester renders frames in memory, and the
// first headwords must appear as texts.
func TestWordListRendersRows(t *testing.T) {
	s := newStore()
	d, err := rdict.Open("../../compiler/dictionaries/eng-zho-ngsl.rdict")
	if err != nil {
		t.Skipf("fixture missing: %v", err)
	}
	t.Cleanup(d.Close)
	m, _ := d.Manifest()
	s.dicts = append(s.dicts, &openDictionary{
		path: "fixture.rdict", name: m.Name, dict: d,
	})
	s.selectedPath = "fixture.rdict"
	s.applyFilter()
	s.lookup(s.filtered[0]) // selectDict does this on open
	if len(s.filtered) == 0 {
		t.Fatal("filtered is empty after applyFilter")
	}
	t.Logf("filtered: %d headwords, first=%q", len(s.filtered), s.filtered[0])

	tt := ui.NewTester(s.view, 1000, 640)
	tt.Frame()
	for _, hw := range s.filtered[:5] {
		if !tt.HasText(hw) {
			t.Errorf("headword %q not rendered in the sidebar", hw)
		}
	}
	if s.entry == nil {
		t.Error("no entry loaded after pack open (first headword lookup)")
	} else {
		t.Logf("entry headword: %s, senses: %d", s.entry.Headword, len(s.entry.Etymologies))
	}
}

// TestClickWordShowsDetail clicks a headword in the list and verifies
// the detail pane renders its translations.
func TestClickWordShowsDetail(t *testing.T) {
	s := newStore()
	d, err := rdict.Open("../../compiler/dictionaries/eng-zho-ngsl.rdict")
	if err != nil {
		t.Skipf("fixture missing: %v", err)
	}
	t.Cleanup(d.Close)
	m, _ := d.Manifest()
	s.dicts = append(s.dicts, &openDictionary{
		path: "fixture.rdict", name: m.Name, dict: d,
	})
	s.selectedPath = "fixture.rdict"
	s.applyFilter()

	tt := ui.NewTester(s.view, 1000, 640)
	tt.Frame()
	// Click the second headword in the list.
	target := s.filtered[1]
	if err := tt.Click(target); err != nil {
		t.Fatalf("click %q: %v", target, err)
	}
	tt.Frame()
	if s.selectedHeadword != target {
		t.Fatalf("selected headword %q, want %q", s.selectedHeadword, target)
	}
	if s.entry == nil {
		t.Fatal("no entry after click")
	}
	// A translation of the entry must render in the detail pane.
	for _, ety := range s.entry.Etymologies {
		for _, sense := range ety.Senses {
			for _, tr := range sense.Translations {
				if tt.HasText(tr.Value) {
					t.Logf("rendered translation: %q", tr.Value)
					return
				}
			}
		}
	}
	t.Error("no translation rendered in the detail pane")
}

// TestScrollLoadsMorePages verifies the infinite scroll: when the list
// nears the end of the loaded page, the next page loads.
func TestScrollLoadsMorePages(t *testing.T) {
	s := newStore()
	d, err := rdict.Open("../../compiler/dictionaries/eng-zho-ngsl.rdict")
	if err != nil {
		t.Skipf("fixture missing: %v", err)
	}
	t.Cleanup(d.Close)
	m, _ := d.Manifest()
	s.dicts = append(s.dicts, &openDictionary{
		path: "fixture.rdict", name: m.Name, dict: d,
	})
	s.selectedPath = "fixture.rdict"
	// A page of 200 synthetic headwords, of a list of 600: scrolling to
	// the end of the first page must load the next one.
	words := make([]string, 600)
	for i := range words {
		words[i] = "word-" + string(rune('a'+i%26)) + "-" + string(rune('0'+i%10)) + string(rune('0'+i/10%10))
	}
	s.filtered = words[:200]
	s.listLimit = 200
	s.lastFilter = s.selectedPath + "|"

	tt := ui.NewTester(s.view, 1000, 640)
	tt.Frame()
	// The first frame's SearchField may report a spurious change; reset
	// the synthetic page after it settles.
	s.filtered = words[:200]
	s.listLimit = 200
	tt.Frame()
	if !tt.HasText(words[0]) {
		t.Fatal("first page not rendered")
	}
	// Scroll the list down in steps until past the first page's end.
	for i := range 30 {
		tt.Scroll(150, 400, 0, 400)
		tt.Frame()
		f, l := s.list.Visible()
		t.Logf("step %d: visible %d..%d of %d, limit %d, atEnd %v", i, f, l, len(s.filtered), s.listLimit, s.list.AtEnd())
		if s.listLimit > 200 {
			break
		}
	}
	if s.listLimit != 400 {
		t.Fatalf("listLimit = %d, want 400 after scrolling to the page end", s.listLimit)
	}
	// loadMore re-runs applyFilter against the real pack, which has only
	// 100 headwords: the limit then settles past its end (no third page).
	tt.Frame()
	if s.canLoadMore() {
		t.Error("canLoadMore still true past the pack's end")
	}
}

// TestSidebarLayout checks the layout details by sampling the rendered
// frame: the search field sits inside the sidebar with side margins,
// and the add card's box aligns with the cover's box above it.
func TestSidebarLayout(t *testing.T) {
	s := newStore()
	d, err := rdict.Open("../../compiler/dictionaries/eng-zho-ngsl.rdict")
	if err != nil {
		t.Skipf("fixture missing: %v", err)
	}
	t.Cleanup(d.Close)
	m, _ := d.Manifest()
	s.dicts = append(s.dicts, &openDictionary{
		path: "fixture.rdict", name: m.Name, dict: d,
	})
	s.selectedPath = "fixture.rdict"
	s.applyFilter()

	tt := ui.NewTester(s.view, 1000, 640)
	tt.Frame()
	tt.Frame() // settle: the first frame may report a search change
	img := tt.Image()
	if img == nil || img.Bounds().Dx() < 300 {
		t.Fatal("no rendered frame")
	}
	at := func(x, y int) (uint8, uint8, uint8) {
		x = min(max(x, 0), img.Bounds().Dx()-1)
		y = min(max(y, 0), img.Bounds().Dy()-1)
		r, g, b, _ := img.At(x, y).RGBA()
		return uint8(r >> 8), uint8(g >> 8), uint8(b >> 8)
	}
	isSurface := func(r, g, b uint8) bool { return r > 235 && g > 235 && b > 235 }

	// The search field: the white box in the sidebar, below the shelf
	// (96 DIP) and above the word list. Scan rows to find it, then its
	// left and right edges.
	left, right := -1, -1
	for y := 106; y < 160 && left < 0; y++ {
		for x := 0; x < 300; x++ {
			r, g, b := at(x, y)
			if isSurface(r, g, b) {
				continue
			}
			// a non-surface run wider than half the sidebar: the field
			end := x
			for end < 300 {
				r, g, b := at(end, y)
				if isSurface(r, g, b) {
					break
				}
				end++
			}
			if end-x > 150 {
				left, right = x, end-1
			}
			x = end
		}
	}
	if left < 0 {
		t.Fatal("search field row not found below the shelf")
	}
	if left < 5 || left > 15 {
		t.Errorf("search field left edge at x=%d, want ~10 (side margin)", left)
	}
	if right < 285 || right > 295 {
		t.Errorf("search field right edge at x=%d, want ~290 (side margin)", right)
	}

	// The cover (first shelf card) and the add card (last): both start
	// at the shelf's top padding. Find each card's topmost pixel.
	cardTop := func(x int) int {
		for y := 0; y < 106; y++ {
			r, g, b := at(x, y)
			if !isSurface(r, g, b) {
				return y
			}
		}
		return -1
	}
	// The cover card runs from x≈10 (shelf padding) + 76 card width.
	coverTop := cardTop(15)
	// The add card follows the cover card plus a 10 gap: x ≈ 96-166.
	addTop := cardTop(105)
	if coverTop < 0 || addTop < 0 {
		t.Fatalf("cards not found: cover top %d, add top %d", coverTop, addTop)
	}
	if abs(addTop-coverTop) > 1 {
		t.Errorf("add card top %d, cover top %d: misaligned", addTop, coverTop)
	}
	// The selected cover shows no accent ring: its top edge is the
	// placeholder's gray border, not the theme accent (a saturated blue).
	r, g, b := at(36, coverTop)
	if abs(int(r)-int(g)) > 12 || abs(int(g)-int(b)) > 12 {
		t.Errorf("selected cover top edge is a colored ring (r=%d g=%d b=%d), want the plain border", r, g, b)
	}
}

func abs(v int) int {
	if v < 0 {
		return -v
	}
	return v
}
