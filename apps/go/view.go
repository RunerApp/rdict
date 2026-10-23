package main

import (
	"github.com/egoist/mygo/ui"
)

// view is the root: empty state or the split sidebar/detail layout.
// Mirrors ContentView.body.
func (s *store) view(c *ui.Context) {
	if len(s.dicts) == 0 {
		s.emptyState(c)
	} else {
		ui.Split(c, &s.sidebarWidth,
			func() { s.sidebar(c) },
			func() { s.detailPane(c) },
		).Fill()
	}
	if s.errShown {
		if ui.AlertDialog(c, &s.errShown, sError(), s.errMsg, "OK") != ui.AlertDismissed {
			s.errShown = false
		}
	}
}

// emptyState mirrors ContentView.emptyState: island hero card.
func (s *store) emptyState(c *ui.Context) {
	t := c.Theme()
	ui.Column(c).Fill().Center().Background(t.Background).Children(func() {
		ui.Box(c).Padding(40).Radius(16).Background(t.Surface).
			Border(1, t.Border).
			Shadow(0, 1, 3, 0, ui.RGBA(0, 0, 0, 0.06)).
			Children(func() {
				ui.Column(c).Center().Gap(18).Children(func() {
					ui.Box(c).Size(96, 96).Radius(24).Background(t.Accent.Alpha(0.10)).Center().Children(func() {
						ui.Icon(c, iconBook).FontSize(42).TextColor(t.Accent)
					})
					ui.Column(c).Center().Gap(8).Children(func() {
						ui.Text(c, sNoDict()).FontSize(20).Bold()
						ui.Text(c, sNoDictHint()).FontSize(13).TextColor(t.TextMuted)
					})
					if ui.PrimaryButton(c, sOpenDict()).Clicked() {
						s.promptOpen()
					}
				})
			})
	})
}

// sidebar mirrors ContentView.sidebar: shelf, search island, word list.
func (s *store) sidebar(c *ui.Context) {
	t := c.Theme()
	ui.Column(c).Fill().Background(t.Surface).Children(func() {
		if len(s.dicts) > 0 {
			s.shelf(c)
		}
		if ui.SearchField(c.Key("search"), &s.search).
			Placeholder(sSearchPlaceholder()).Margin(8, 10, 0, 10).Changed() {
			s.applyFilter()
		}
		s.wordList(c)
	})
}

// shelf mirrors DictionaryShelfView: horizontal cover cards + add card.
func (s *store) shelf(c *ui.Context) {
	t := c.Theme()
	ui.ScrollHorizontal(c).Padding(10, 10, 0, 10).Children(func() {
		ui.Row(c).AlignItems(ui.Start).Gap(10).Children(func() {
			for _, d := range s.dicts {
				s.dictCard(c, d)
			}
			// "+" card.
			box := ui.Box(c).Width(76).Children(func() {
				ui.Box(c).Size(52, 68).Radius(6).Background(t.Background).
					Border(1, t.Border).Center().Children(func() {
					ui.Icon(c, iconPlus).FontSize(20).TextColor(t.TextMuted)
				})
			})
			box.OnClick(s.promptOpen)
		})
	}).Height(96)
}

// dictCard mirrors DictCard: cover thumbnail, name, context-menu
// Delete. No selection ring: the selected card's name just turns bold
// (TestSidebarLayout locks the cover's top edge as the plain border).
func (s *store) dictCard(c *ui.Context, d *openDictionary) {
	t := c.Theme()
	selected := d.path == s.selectedPath
	box := ui.Box(c).Width(76).Children(func() {
		ui.Column(c).Gap(6).Children(func() {
			cover := ui.Box(c).Size(52, 68).Radius(8).
				Shadow(0, 2, 3, 0, ui.RGBA(0, 0, 0, 0.13))
			if d.cover != nil {
				cover.Children(func() {
					ui.Image(c, d.cover).Size(52, 68).Fit(ui.Cover).Radius(8)
				})
			} else {
				cover.Background(t.Background).Border(1, t.Border).Center().Children(func() {
					ui.Icon(c, iconBook).FontSize(18).TextColor(t.TextMuted)
				})
			}
			name := ui.Text(c, d.name).FontSize(10).MaxLines(1).Width(76)
			if selected {
				name.Bold()
			} else {
				name.TextColor(t.TextMuted)
			}
		})
	})
	path := d.path
	box.OnClick(func() { s.selectDict(path) })
	box.ContextMenu(func(m *ui.Menu) {
		if m.Item(sClose()).Chosen() {
			s.closeDict(path)
		}
	})
}

// wordList mirrors wordList + LazyVStack infinite scroll: ui.List
func (s *store) wordList(c *ui.Context) {
	t := c.Theme()
	s.list.Label = func(i int) string { return s.filtered[i] }
	ui.List(c, &s.list, len(s.filtered), func(i int) {
		hw := s.filtered[i]
		row := ui.Text(c, hw).FontSize(13).Padding(5, 10)
		if hw == s.selectedHeadword {
			row.FontWeight(500).
				Background(t.Accent.Alpha(0.14)).Radius(6)
		}
		row.OnClick(func() { s.lookup(hw) })
	}).Grow(1).Padding(6, 10)
	// Load more as the end nears (the sentinel view in the Swift app).
	if _, last := s.list.Visible(); last >= len(s.filtered)-5 && s.canLoadMore() {
		s.loadMore()
		// The rows above grew the list after it was laid out: ask for
		// another frame to lay the new rows and let scrolling go on.
		c.Invalidate()
	}
}
