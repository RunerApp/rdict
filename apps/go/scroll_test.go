package main

import (
	"testing"

	"github.com/egoist/mygo/ui"

	"rdictapp/internal/rdict"
)

// TestBigPackFlingScroll uses a real 2809-word pack (built by the repo
// compiler from headwords/entries) to reproduce the user's fling-scroll
// sequence: many small wheel deltas arrive one per frame, the fling hits
// the bottom of the loaded page mid-sequence, and loadMore grows the
// list while the wheel stops there. The frame that loads more is the
// one that laid the old rows out, so the app must ask for another frame
// — without it the list stays put until some other event draws one, and
// the user has to scroll up and back down to see the new rows. Skipped
// when the big pack is not present.
func TestBigPackFlingScroll(t *testing.T) {
	const bigPack = "/tmp/eng-zho-big.rdict"
	s := newStore()
	d, err := rdict.Open(bigPack)
	if err != nil {
		t.Skipf("big pack missing: %v", err)
	}
	t.Cleanup(d.Close)
	m, _ := d.Manifest()
	s.dicts = append(s.dicts, &openDictionary{
		path: bigPack, name: m.Name, dict: d,
	})
	s.selectedPath = bigPack
	s.applyFilter()
	if len(s.filtered) != 200 {
		t.Fatalf("first page = %d words, want 200", len(s.filtered))
	}

	tt := ui.NewTester(s.view, 1000, 640)
	tt.Frame()
	s.applyFilter()
	tt.Frame()

	// A fling: wheel deltas of 400px, one per frame, as on device, until
	// the bottom of the first page loads the next.
	for i := 0; i < 24 && s.listLimit == 200; i++ {
		tt.Scroll(150, 400, 0, 400)
		tt.Frame()
	}
	if s.listLimit != 400 {
		t.Fatalf("fling did not load page 2: limit=%d", s.listLimit)
	}
	// The frame that loaded the page was laid out with the old rows: the
	// app must ask for another, which lays the new rows and refreshes
	// the list's content height. Without it the list sits at the end of
	// the old page — still "at end" — and every wheel delta clamps there.
	if s.list.AtEnd() {
		t.Fatal("list still at the end of the old page after loading more: no frame laid the new rows out")
	}
	// Wheel deltas after the fling must move the list on their own: the
	// frame that loaded the page was laid out with the old rows, and
	// unless the app asks for another the list's content height stays
	// that of the old page and every delta clamps at its end — the user
	// has to scroll up and back down to see the new rows.
	for range 8 {
		tt.Scroll(150, 400, 0, 120)
	}
	tt.Frame()
	_, l := s.list.Visible()
	if l < 210 {
		t.Fatalf("list stuck at row %d of %d after scrolling on; the user would have to scroll up and back down", l, len(s.filtered))
	}
	if !tt.HasText(s.filtered[l-1]) {
		t.Errorf("row %d (%q) not rendered after scrolling into page 2", l-1, s.filtered[l-1])
	}
}
