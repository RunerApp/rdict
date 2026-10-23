// Rdict-go is a MyGo native-UI replica of the SwiftUI Rdict desktop
// dictionary app: it opens .rdict packs through the pure-Go reader in
// internal/rdict, searches the headword index, and renders the entry
// AST. Audio/video media is not played.
package main

import (
	"log"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/i18n"
	"github.com/egoist/mygo/ui"
)

func main() {
	s := newStore()
	mygo.App.WhenReady(func() {
		s.start()
		setAppMenu(s)
		s.win = mygo.NewWindow(mygo.WindowOptions{
			Title:     "rdict",
			Width:     1000,
			Height:    640,
			MinWidth:  800,
			MinHeight: 500,
			Content:   ui.View(s.view),
		})
	})
	mygo.App.OnLocaleChanged(func(string) {
		// The menu's labels follow the language; role items take the new
		// language's themselves, the rest say the menu built anew.
		setAppMenu(s)
	})
	if err := mygo.App.Run(); err != nil {
		log.Fatal(err)
	}
}

// setAppMenu sets the menu bar: the default menus plus Open and the
// app's languages (mygo.App.SetLocale keeps the choice).
func setAppMenu(s *store) {
	languages := make([]*mygo.MenuItem, 0, len(mygo.App.Languages()))
	for _, tag := range mygo.App.Languages() {
		tag := tag
		languages = append(languages, &mygo.MenuItem{
			Label:   i18n.Autonym(tag),
			Type:    mygo.MenuItemRadio,
			Checked: tag == mygo.App.Locale(),
			Click:   func(*mygo.MenuItem, *mygo.Window) { mygo.App.SetLocale(tag) },
		})
	}
	mygo.App.SetMenu(mygo.NewMenu([]*mygo.MenuItem{
		{Role: mygo.RoleAppMenu},
		{Label: sOpen(), Accelerator: "CmdOrCtrl+O",
			Click: func(*mygo.MenuItem, *mygo.Window) { s.promptOpen() }},
		{Label: sLanguage(), Submenu: languages},
		{Role: mygo.RoleEditMenu},
	}))
}
