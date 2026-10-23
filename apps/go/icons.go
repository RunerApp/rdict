package main

import (
	"github.com/egoist/mygo/ui"
)

// icon parses the shapes of a 24×24 stroked icon, drawn in currentColor
// as icon sets draw them (the pattern of MyGo's own gallery).
func icon(shapes string) *ui.SVG {
	return ui.MustParseSVG([]byte(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">` + shapes + `</svg>`))
}

// The icons of the app, all stroked and tinted by the text color.
var (
	// iconBook is the empty state and the cover placeholder.
	iconBook = icon(`<path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>`)
	// iconPlus is the add-dictionary card.
	iconPlus = icon(`<path d="M5 12h14M12 5v14"/>`)
	// iconVolume marks a pronunciation.
	iconVolume = icon(`<polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/>`)
	// iconBranch marks an etymology root.
	iconBranch = icon(`<line x1="6" y1="3" x2="6" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/>`)
	// iconNote marks a note under a definition.
	iconNote = icon(`<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/>`)
	// iconRedirect marks a "see also" redirect.
	iconRedirect = icon(`<polyline points="9 10 4 15 9 20"/><path d="M20 4v7a4 4 0 0 1-4 4H4"/>`)
)
