package main

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"

	"rdictapp/internal/rdict"
	"rdictapp/locales"
)

const pageSize = 200

// openDictionary is one open pack: metadata plus its reader.
// Mirrors OpenDictionary + the bridges map in DictionaryStore.swift.
type openDictionary struct {
	path        string
	name        string
	entryCount  int
	sourceLang  string
	targetLangs []string
	version     string
	cover       *ui.Bitmap
	dict        *rdict.Dictionary
}

// session mirrors the UserDefaults keys rdict.paths / rdict.lastPath,
// persisted as JSON in the user data directory. The language is the
// app's own (mygo.App.Locale), which the app keeps across launches.
type session struct {
	Paths    []string `json:"paths"`
	LastPath string   `json:"lastPath"`
}

// store is the app state. All fields are touched on the main thread
// only (view builds, menu items, WhenReady, Window.Update).
type store struct {
	win *mygo.Window

	dicts        []*openDictionary
	selectedPath string

	search     string
	filtered   []string
	listLimit  int
	lastFilter string
	list       ui.ListState

	entry            *rdict.Entry
	selectedHeadword string
	lastLookupMs     float64

	errShown bool
	errMsg   string

	sidebarWidth float32
	mediaCache   map[string]*ui.Bitmap

	sessionPath string
	sess        session
}

func newStore() *store {
	return &store{
		sidebarWidth: 300,
		mediaCache:   map[string]*ui.Bitmap{},
		listLimit:    pageSize,
	}
}

// current returns the selected dictionary, or nil.
func (s *store) current() *openDictionary {
	for _, d := range s.dicts {
		if d.path == s.selectedPath {
			return d
		}
	}
	return nil
}

// start restores the last session once the app is ready (user data
// directory available). Mirrors DictionaryStore.restoreLastSession.
func (s *store) start() {
	if dir, err := mygo.App.Path(mygo.PathUserData); err == nil {
		s.sessionPath = filepath.Join(dir, "rdict-go-session.json")
		s.loadSession()
	}
	for _, path := range s.sess.Paths {
		if _, err := os.Stat(path); err != nil {
			continue
		}
		if d := s.open(path); d != nil && path == s.sess.LastPath {
			s.selectDict(path)
		}
	}
	if s.current() == nil && len(s.dicts) > 0 {
		s.selectDict(s.dicts[0].path)
	}
	if len(s.dicts) == 0 {
		// Fallback: the bundled sample pack next to the executable or
		// in the repo (dev). Mirrors the Swift app's bundled
		// eng-zho-ngsl.rdict resource.
		for _, p := range bundledPackCandidates() {
			if _, err := os.Stat(p); err == nil {
				s.open(p)
				break
			}
		}
	}
}

func bundledPackCandidates() []string {
	exe, err := os.Executable()
	if err == nil {
		dir := filepath.Dir(exe)
		yield := func(rel string) string {
			// Walk up from the executable dir to find the repo root.
			for d := dir; ; d = filepath.Dir(d) {
				p := filepath.Join(d, rel)
				if _, err := os.Stat(p); err == nil {
					return p
				}
				if d == filepath.Dir(d) {
					break
				}
			}
			return ""
		}
		if p := yield(filepath.Join("compiler", "dictionaries", "eng-zho-ngsl.rdict")); p != "" {
			return []string{p}
		}
	}
	return nil
}

func (s *store) loadSession() {
	s.sess = session{}
	if s.sessionPath == "" {
		return
	}
	if b, err := os.ReadFile(s.sessionPath); err == nil {
		_ = json.Unmarshal(b, &s.sess)
	}
	// Drop empty or whitespace paths (a stray empty pick must not
	// become a phantom shelf entry).
	paths := s.sess.Paths[:0]
	for _, p := range s.sess.Paths {
		if strings.TrimSpace(p) != "" {
			paths = append(paths, p)
		}
	}
	s.sess.Paths = paths
}

func (s *store) persist() {
	if s.sessionPath == "" {
		return
	}
	s.sess.Paths = s.sess.Paths[:0]
	for _, d := range s.dicts {
		if d.path != "" {
			s.sess.Paths = append(s.sess.Paths, d.path)
		}
	}
	if cur := s.current(); cur != nil {
		s.sess.LastPath = cur.path
	} else {
		s.sess.LastPath = ""
	}
	b, _ := json.MarshalIndent(s.sess, "", "  ")
	_ = os.WriteFile(s.sessionPath, b, 0o600)
}

// open opens a pack, selects it, and persists the session. Returns the
// dictionary or nil on failure. Mirrors DictionaryStore.open(url:).
func (s *store) open(path string) *openDictionary {
	for _, d := range s.dicts {
		if d.path == path {
			s.selectDict(path)
			return d
		}
	}
	dict, err := rdict.Open(path)
	if err != nil {
		s.showError(locales.FailedToOpen(filepath.Base(path)))
		return nil
	}
	d := &openDictionary{path: path, dict: dict}
	if m, err := dict.Manifest(); err == nil && m != nil {
		d.name, d.entryCount = m.Name, m.EntryCount
		d.sourceLang, d.targetLangs, d.version = m.SourceLang, m.TargetLangs, m.Version
	}
	if d.name == "" {
		d.name = strings.TrimSuffix(filepath.Base(path), filepath.Ext(path))
	}
	if cover := dict.ReadCover(); len(cover) > 0 {
		if bm, err := ui.DecodeBitmap(cover); err == nil {
			d.cover = bm
		}
	}
	s.dicts = append(s.dicts, d)
	s.persist()
	s.selectDict(path)
	return d
}

// selectDict mirrors DictionaryStore.selectDict + reloadHeadwords.
func (s *store) selectDict(path string) {
	s.selectedPath = path
	s.entry = nil
	s.selectedHeadword = ""
	s.applyFilter()
	if len(s.filtered) > 0 {
		s.lookup(s.filtered[0])
	}
	s.persist()
}

// closeDict mirrors DictionaryStore.closeDict.
func (s *store) closeDict(path string) {
	for i, d := range s.dicts {
		if d.path != path {
			continue
		}
		d.dict.Close()
		s.dicts = append(s.dicts[:i], s.dicts[i+1:]...)
		break
	}
	if s.selectedPath == path {
		s.selectedPath = ""
		s.entry = nil
		s.selectedHeadword = ""
		s.applyFilter()
		if len(s.dicts) > 0 {
			s.selectDict(s.dicts[0].path)
			return
		}
	}
	s.persist()
}

// promptOpen shows the system file dialog on a goroutine (Dialog.Open
// blocks) and applies the picks on the main thread.
func (s *store) promptOpen() {
	go func() {
		paths, err := mygo.Dialog.Open(mygo.OpenDialogOptions{
			Parent:   s.win,
			Title:    sOpenDict(),
			Filters:  []mygo.FileFilter{{Name: "Rdict", Extensions: []string{"rdict"}}},
			Multiple: true,
		})
		if err != nil || len(paths) == 0 {
			return
		}
		s.win.Update(func() {
			for _, p := range paths {
				s.open(p)
			}
		})
	}()
}

// lookup mirrors DictionaryStore.lookup.
func (s *store) lookup(headword string) {
	d := s.current()
	if d == nil {
		return
	}
	s.selectedHeadword = headword
	t0 := time.Now()
	e, err := d.dict.Lookup(headword)
	s.lastLookupMs = float64(time.Since(t0).Microseconds()) / 1000
	switch {
	case err == nil:
		s.entry = e
	case errors.Is(err, rdict.ErrNotFound):
		s.entry = nil
	default:
		s.showError(err.Error())
		s.entry = nil
	}
}

// applyFilter mirrors DictionaryStore.applyFilter: prefix search via
// the index, paginated, never materializing the full word list.
func (s *store) applyFilter() {
	key := s.selectedPath + "|" + s.search
	if key != s.lastFilter {
		s.lastFilter = key
		s.listLimit = pageSize
	}
	d := s.current()
	if d == nil {
		s.filtered = nil
		return
	}
	s.filtered = d.dict.Prefix(s.search, 0, s.listLimit)
}

func (s *store) canLoadMore() bool { return len(s.filtered) >= s.listLimit }

// loadMore grows the sidebar list by one page.
func (s *store) loadMore() {
	if !s.canLoadMore() {
		return
	}
	s.listLimit += pageSize
	s.applyFilter()
}

// searchHit is one cross-dictionary match.
type searchHit struct {
	dict     *openDictionary
	headword string
}

// searchAll mirrors DictionaryStore.searchAll: prefix matches across
// every open pack, capped at 200.
func (s *store) searchAll(needle string) []searchHit {
	var results []searchHit
	for _, d := range s.dicts {
		for _, hw := range d.dict.Prefix(needle, 0, 200) {
			results = append(results, searchHit{d, hw})
			if len(results) >= 200 {
				return results
			}
		}
	}
	return results
}

// mediaBitmap returns the decoded bitmap for a media reference,
// decoding and caching it on first use. Returns nil when unavailable
// (including audio and video, which this replica does not play).
func (s *store) mediaBitmap(m rdict.MediaRef) *ui.Bitmap {
	if strings.ToLower(m.Kind) != "image" {
		return nil
	}
	if bm, ok := s.mediaCache[m.Hash]; ok {
		return bm
	}
	d := s.current()
	if d == nil {
		return nil
	}
	data := d.dict.ReadMedia(m.Kind, m.Hash)
	if len(data) == 0 {
		return nil
	}
	bm, err := ui.DecodeBitmap(data)
	if err != nil {
		s.mediaCache[m.Hash] = nil // negative cache
		return nil
	}
	s.mediaCache[m.Hash] = bm
	return bm
}

func (s *store) showError(msg string) {
	s.errMsg = msg
	s.errShown = true
}
