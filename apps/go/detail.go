package main

import (
	"strings"

	"github.com/egoist/mygo/ui"

	"rdictapp/internal/rdict"
	"rdictapp/locales"
)

// ── Theme helpers (DictTheme in EntryDetailView.swift) ──────────────

func posColor(pos string) ui.Color {
	switch strings.ToUpper(pos) {
	case "NOUN", "PROPN":
		return ui.RGBA(64, 140, 217, 1)
	case "VERB":
		return ui.RGBA(204, 89, 77, 1)
	case "ADJ":
		return ui.RGBA(77, 166, 115, 1)
	case "ADV":
		return ui.RGBA(191, 128, 51, 1)
	case "PRON":
		return ui.RGBA(153, 102, 191, 1)
	case "DET", "ADP":
		return ui.RGBA(140, 140, 140, 1)
	case "NUM":
		return ui.RGBA(166, 115, 77, 1)
	case "CCONJ", "SCONJ":
		return ui.RGBA(128, 128, 153, 1)
	case "INTJ":
		return ui.RGBA(217, 140, 38, 1)
	default:
		return ui.RGBA(128, 128, 128, 1)
	}
}

func posLabel(pos string) string {
	m := map[string]string{
		"NOUN": "n", "VERB": "v", "ADJ": "adj", "ADV": "adv",
		"PRON": "pron", "DET": "det", "ADP": "prep", "NUM": "num",
		"CCONJ": "conj", "SCONJ": "conj", "INTJ": "intj",
		"PROPN": "n", "AUX": "aux", "PART": "part", "SYM": "sym",
		"PUNCT": "punct", "X": "x",
	}
	if l, ok := m[strings.ToUpper(pos)]; ok {
		return l
	}
	return strings.ToLower(pos)
}

func langLabel(lang string) string {
	switch lang {
	case "zh-Hans":
		return "中"
	case "es-ES":
		return "Es"
	case "en":
		return "En"
	case "ja":
		return "日"
	default:
		return lang
	}
}

// entryTag parses a `category:value` tag (rdict-spec §3.5). Only exam
// levels render; others stay hidden.
type entryTag struct{ category, value string }

func parseEntryTag(raw string) (entryTag, bool) {
	i := strings.Index(raw, ":")
	if i < 0 {
		return entryTag{}, false
	}
	return entryTag{raw[:i], raw[i+1:]}, true
}

func (t entryTag) visible() bool { return t.category == "exam" }

func (t entryTag) label() string {
	if i := strings.LastIndex(t.value, "-"); i >= 0 {
		return t.value[i+1:]
	}
	return t.value
}

func levelColor(label string) ui.Color {
	switch label {
	case "N5", "A1":
		return ui.RGBA(77, 166, 102, 1)
	case "N4", "A2":
		return ui.RGBA(115, 166, 77, 1)
	case "N3", "B1":
		return ui.RGBA(191, 153, 51, 1)
	case "N2", "B2":
		return ui.RGBA(217, 115, 51, 1)
	case "N1", "C1", "C2":
		return ui.RGBA(204, 64, 64, 1)
	default:
		return ui.RGBA(128, 128, 128, 1)
	}
}

// island is the card look, as the gallery's card: surface, hairline
// border, a light shadow.
func island(c *ui.Context, pad float32) ui.Element {
	return ui.Column(c).Padding(pad).FillWidth().
		Background(c.Theme().Surface).Radius(10).
		Border(1, c.Theme().Border).
		Shadow(0, 1, 3, 0, ui.RGBA(0, 0, 0, 0.06))
}

func sectionLabel(c *ui.Context, text string) {
	ui.Text(c, strings.ToUpper(text)).FontSize(12).Bold().
		TextColor(c.Theme().TextMuted)
}

// ── Detail pane (EntryDetailView) ───────────────────────────────────

func (s *store) detailPane(c *ui.Context) {
	t := c.Theme()
	if s.entry == nil {
		ui.Column(c).Fill().Center().Background(t.Background).Children(func() {
			ui.Text(c, sSelectWord()).FontSize(15).TextColor(t.TextMuted)
		})
		return
	}
	e := s.entry
	ui.Scroll(c).Fill().Background(t.Background).Children(func() {
		ui.Column(c).FillWidth().Padding(24).Gap(16).Selectable().
			MaxWidth(720).Margin(0, ui.Auto).Children(func() {
			s.headerCard(c, e)
			if e.See != nil && *e.See != "" {
				s.redirectCard(c, *e.See)
			}
			if len(e.Morphology) > 0 {
				s.morphologyCard(c, e)
			}
			labeled := labeledTranslationLangs(e)
			for _, ety := range e.Etymologies {
				if ety.Root != nil && *ety.Root != "" {
					ui.Row(c).Gap(6).Children(func() {
						ui.Icon(c, iconBranch).FontSize(14).TextColor(t.TextMuted)
						ui.Text(c, *ety.Root).FontSize(12).Italic().
							TextColor(t.TextMuted)
					})
				}
				for i, sense := range ety.Senses {
					s.senseCard(c, sense, i+1, labeled)
				}
			}
			if len(e.Relations) > 0 {
				s.relationsCard(c, e)
			}
		})
	})
}

func (s *store) headerCard(c *ui.Context, e *rdict.Entry) {
	island(c, 22).Children(func() {
		ui.Column(c).FillWidth().Gap(12).Children(func() {
			ui.Row(c).AlignItems(ui.End).Gap(14).FillWidth().Children(func() {
				ui.Text(c, e.Headword).FontSize(36).Bold()
				ui.Spacer(c)
				for _, raw := range e.Tags {
					if tag, ok := parseEntryTag(raw); ok && tag.visible() {
						ui.Text(c, tag.label()).FontSize(11).Bold().TextColor(ui.Color{R: 255, G: 255, B: 255, A: 255}).
							Padding(8, 3).Background(levelColor(tag.label())).Radius(999)
					}
				}
			})
			for _, p := range e.Pron {
				ui.Row(c).Gap(6).Children(func() {
					ui.Icon(c, iconVolume).FontSize(14).TextColor(c.Theme().TextMuted)
					ui.Text(c, p.Value).FontSize(16).TextColor(c.Theme().TextMuted)
				})
			}
		})
	})
}

func (s *store) redirectCard(c *ui.Context, see string) {
	t := c.Theme()
	if ui.Row(c).Gap(8).Padding(14).FillWidth().
		Background(t.Accent.Alpha(0.08)).Radius(8).
		Cursor(ui.CursorPointer).Children(func() {
		ui.Icon(c, iconRedirect).FontSize(16).TextColor(t.Accent)
		ui.Text(c, see).FontSize(16).Bold().TextColor(t.Accent)
	}).Clicked() {
		s.lookup(see)
	}
}

func (s *store) morphologyCard(c *ui.Context, e *rdict.Entry) {
	t := c.Theme()
	island(c, 16).Children(func() {
		ui.Column(c).FillWidth().Gap(10).Children(func() {
			sectionLabel(c, sWordStructure())
			ui.Row(c).AlignItems(ui.Center).Gap(4).Children(func() {
				for i, m := range e.Morphology {
					if i > 0 {
						ui.Text(c, "›").FontSize(12).Bold().TextColor(t.TextMuted)
					}
					ui.Column(c).Gap(2).Padding(8, 10).Background(t.Background).
						Border(1, t.Border).Radius(6).Children(func() {
						if m.Kind != nil {
							ui.Text(c, strings.ToUpper(sMorphemeKind(m.Kind))).FontSize(10).Bold().TextColor(t.TextMuted)
						}
						ui.Text(c, m.Term).FontSize(15)
					})
				}
			})
		})
	})
}

func (s *store) relationsCard(c *ui.Context, e *rdict.Entry) {
	t := c.Theme()
	island(c, 16).Children(func() {
		ui.Column(c).FillWidth().Gap(10).Children(func() {
			sectionLabel(c, sRelated())
			// FlowLayout == Row + Wrap.
			ui.Row(c).Wrap().Gap(8).Children(func() {
				for _, rel := range e.Relations {
					target := rel.Target
					chip := ui.Column(c).Gap(2).Padding(8, 12).Background(t.Background).
						Border(1, t.Border).Radius(6).Cursor(ui.CursorPointer).Children(func() {
						ui.Text(c, strings.ToUpper(sRelationType(&rel.Type))).FontSize(10).Bold().TextColor(t.TextMuted)
						ui.Text(c, rel.Target).FontSize(13)
					})
					chip.OnClick(func() { s.lookup(target) })
				}
			})
		})
	})
}

// labeledTranslationLangs mirrors EntryDetailView.labeledTranslationLangs:
// label translations only when the pack is multi-target.
func labeledTranslationLangs(e *rdict.Entry) map[string]bool {
	seen := map[string]bool{}
	for _, ety := range e.Etymologies {
		for _, sense := range ety.Senses {
			for _, tr := range sense.Translations {
				if tr.Lang != nil {
					seen[*tr.Lang] = true
				}
			}
		}
	}
	if len(seen) > 1 {
		return seen
	}
	return nil
}

// senseCard mirrors SenseCard.
func (s *store) senseCard(c *ui.Context, sense rdict.Sense, index int, labeled map[string]bool) {
	t := c.Theme()
	island(c, 18).Children(func() {
		ui.Column(c).FillWidth().Gap(12).Children(func() {
			ui.Column(c).FillWidth().Gap(8).Children(func() {
				// Leader row: circular number badge + POS.
				ui.Row(c).AlignItems(ui.End).Gap(8).Children(func() {
					pos := ""
					if sense.Pos != nil {
						pos = *sense.Pos
					}
					// The row bottom-aligns the line boxes, which puts the
					// badge 5 DIP (the text's descent) below the baseline:
					// lift it onto the baseline, like SwiftUI's
					// .firstTextBaseline alignment in the original app.
					ui.Box(c).Size(20, 20).Margin(0, 0, 5, 0).Radius(999).
						Background(posColor(pos).Alpha(0.15)).
						Center().Children(func() {
						ui.Textf(c, "%d", index).FontSize(10).Bold().TextColor(posColor(pos))
					})
					if sense.Pos != nil {
						// Plain italic text, a touch shorter than the badge
						// (20 DIP): at 25 DIP, words with an ascender or
						// descender (det, pron, prep) render ~18 DIP, which
						// reads as tall as the circle — text spanning the
						// badge's full height looks bigger than the circle.
						ui.Text(c, posLabel(*sense.Pos)).FontSize(25).
							Italic().TextColor(posColor(*sense.Pos))
					}
				})
				// Content column: definitions and translations at the same x.
				ui.Column(c).FillWidth().Gap(6).Children(func() {
					for _, def := range sense.Definitions {
						if defHasContent(def) {
							s.defItem(c, def)
						}
					}
					for _, tr := range sense.Translations {
						ui.Row(c).AlignItems(ui.Start).Gap(6).FillWidth().Children(func() {
							if tr.Lang != nil && labeled[*tr.Lang] {
								ui.Text(c, langLabel(*tr.Lang)).FontSize(10).Bold().
									TextColor(t.TextMuted).Width(24).Padding(0, 4, 0, 0)
							}
							ui.Text(c, tr.Value).FontSize(15)
						})
					}
				})
			})
			// Examples — standalone section.
			examples := collectExamples(sense.Definitions)
			if len(examples) > 0 {
				exLangs := map[string]bool{}
				for _, ex := range examples {
					for _, tr := range ex.Translations {
						if tr.Lang != nil {
							exLangs[*tr.Lang] = true
						}
					}
				}
				if len(exLangs) <= 1 {
					exLangs = nil
				}
				ui.Column(c).FillWidth().Gap(8).Children(func() {
					sectionLabel(c, sExamples())
					for _, ex := range examples {
						exampleView(c, ex, exLangs)
					}
				})
			}
		})
	})
}

// collectExamples mirrors SenseCard.collectExamples.
func collectExamples(defs []rdict.DefItem) []rdict.Example {
	var out []rdict.Example
	for _, item := range defs {
		switch {
		case item.Definition != nil:
			out = append(out, item.Definition.Examples...)
		case item.Group != nil:
			out = append(out, collectExamples(item.Group.Definitions)...)
		}
	}
	return out
}

// defHasContent mirrors SenseCard.defHasContent: example-only shells
// render no line.
func defHasContent(item rdict.DefItem) bool {
	switch {
	case item.Definition != nil:
		d := item.Definition
		return d.Value != "" || len(d.Notes) > 0 || len(d.Media) > 0
	case item.Group != nil:
		for _, sub := range item.Group.Definitions {
			if defHasContent(sub) {
				return true
			}
		}
	}
	return false
}

// defItem mirrors DefItemView.
func (s *store) defItem(c *ui.Context, item rdict.DefItem) {
	switch {
	case item.Definition != nil:
		s.definitionView(c, *item.Definition)
	case item.Group != nil:
		s.defGroupView(c, *item.Group)
	}
}

func (s *store) definitionView(c *ui.Context, d rdict.Definition) {
	t := c.Theme()
	ui.Column(c).FillWidth().Gap(6).Children(func() {
		if d.Value != "" {
			ui.Text(c, d.Value).FontSize(15)
		}
		for _, n := range d.Notes {
			ui.Row(c).Gap(6).Children(func() {
				ui.Icon(c, iconNote).FontSize(13).TextColor(t.TextMuted)
				ui.Text(c, n.Value).FontSize(12).TextColor(t.TextMuted)
			})
		}
		if len(d.Media) > 0 {
			s.mediaFlow(c, d.Media)
		}
	})
}

func (s *store) defGroupView(c *ui.Context, g rdict.DefGroup) {
	t := c.Theme()
	ui.Column(c).FillWidth().Gap(6).Padding(12).Background(t.Background).
		Radius(8).Children(func() {
		if g.Description != nil {
			ui.Text(c, *g.Description).FontSize(13).TextColor(t.TextMuted)
		}
		for _, def := range g.Definitions {
			s.defItem(c, def)
		}
	})
}

// exampleView mirrors ExampleView: target spans highlighted in the
// accent color, bold. Spans are UTF-8 byte offsets — Go strings slice
// directly, so no UTF-16 conversion is needed.
func exampleView(c *ui.Context, ex rdict.Example, labeled map[string]bool) {
	t := c.Theme()
	ui.Column(c).FillWidth().Gap(4).Padding(12).Background(t.Background).
		Radius(8).Children(func() {
		if len(ex.Targets) == 0 {
			ui.Text(c, ex.Value).FontSize(14).TextColor(t.TextMuted)
		} else {
			ui.RichText(c, highlightSpans(c, ex)...)
		}
		for _, tr := range ex.Translations {
			ui.Row(c).AlignItems(ui.Start).Gap(6).FillWidth().Children(func() {
				if tr.Lang != nil && labeled[*tr.Lang] {
					ui.Text(c, langLabel(*tr.Lang)).FontSize(10).Bold().
						TextColor(t.TextMuted).Width(24).Padding(0, 3, 0, 0)
				}
				ui.Text(c, tr.Value).FontSize(13).TextColor(t.TextMuted)
			})
		}
	})
}

// highlightSpans splits the example text into runs at target-span
// boundaries, styling the covered runs in the accent color and bold.
func highlightSpans(c *ui.Context, ex rdict.Example) []ui.Span {
	type bound struct {
		off int
		on  bool
	}
	var bounds []bound
	for _, occ := range ex.Targets {
		for _, sp := range occ.Spans {
			start, end := int(sp.Offset), int(sp.Offset+sp.Length)
			if start < 0 || end > len(ex.Value) || start >= end {
				continue
			}
			bounds = append(bounds, bound{start, true}, bound{end, false})
		}
	}
	// Sort by offset; toggles mark highlighted regions.
	for i := range bounds {
		for j := i + 1; j < len(bounds); j++ {
			if bounds[j].off < bounds[i].off {
				bounds[i], bounds[j] = bounds[j], bounds[i]
			}
		}
	}
	t := c.Theme()
	var spans []ui.Span
	pos := 0
	in := false
	emit := func(end int) {
		if end > pos {
			run := ex.Value[pos:end]
			if in {
				spans = append(spans, ui.Span{Text: run, Color: t.Accent, Weight: 700})
			} else {
				spans = append(spans, ui.Span{Text: run})
			}
		}
		pos = end
	}
	for _, b := range bounds {
		if b.off < pos {
			continue // overlapping span inside a highlighted region
		}
		emit(b.off)
		if b.on {
			in = true
		} else {
			in = false
		}
	}
	emit(len(ex.Value))
	if len(spans) == 0 {
		spans = append(spans, ui.Span{Text: ex.Value})
	}
	return spans
}

// mediaFlow renders definition media: images inline (audio and video
// are not played in this replica).
func (s *store) mediaFlow(c *ui.Context, media []rdict.MediaRef) {
	t := c.Theme()
	ui.Row(c).Wrap().Gap(8).Children(func() {
		for _, m := range media {
			switch strings.ToLower(m.Kind) {
			case "image":
				if bm := s.mediaBitmap(m); bm != nil {
					ui.Column(c).Gap(4).Children(func() {
						ui.Image(c, bm).Size(400, 300).Fit(ui.Contain).Radius(12)
						mediaCaption(c, m)
					})
				} else {
					ui.Text(c, locales.FailedToLoadImage()).FontSize(12).TextColor(t.TextMuted)
				}
			case "audio", "video":
				// Not supported in this replica.
			}
		}
	})
}

func mediaCaption(c *ui.Context, m rdict.MediaRef) {
	t := c.Theme()
	if m.Description != nil {
		ui.Text(c, *m.Description).FontSize(12).TextColor(t.TextMuted)
	} else if m.Alt != nil {
		ui.Text(c, *m.Alt).FontSize(12).TextColor(t.TextMuted)
	}
}
