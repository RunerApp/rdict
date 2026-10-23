package main

import (
	"rdictapp/locales"
)

// L10n: the messages live in the catalogs (locales/en.json,
// locales/zh-Hans.json), whose typed functions mygo generate writes
// (locales/catalog_gen.go). They say them in the language the app
// shows, which the Language menu sets (mygo.App.SetLocale) and the app
// keeps across launches.

func sNoDict() string            { return locales.NoDict() }
func sNoDictHint() string        { return locales.NoDictHint() }
func sOpenDict() string          { return locales.OpenDict() }
func sOpen() string              { return locales.Open() }
func sSearchPlaceholder() string { return locales.SearchPlaceholder() }
func sSelectWord() string        { return locales.SelectWord() }
func sClose() string             { return locales.Close() }
func sError() string             { return locales.Error() }
func sRelated() string           { return locales.Related() }
func sWordStructure() string     { return locales.WordStructure() }
func sExamples() string          { return locales.Examples() }
func sLanguage() string          { return locales.Language() }

// sMorphemeKind mirrors L10n.morphemeKind.
func sMorphemeKind(kind *string) string {
	if kind == nil {
		return ""
	}
	switch *kind {
	case "prefix":
		return locales.MorphemePrefix()
	case "suffix":
		return locales.MorphemeSuffix()
	case "root":
		return locales.MorphemeRoot()
	case "combining_form", "combining form":
		return locales.MorphemeCombiningForm()
	case "particle":
		return locales.MorphemeParticle()
	case "stem":
		return locales.MorphemeStem()
	default:
		return *kind
	}
}

// sRelationType mirrors L10n.relationType.
func sRelationType(typ *string) string {
	if typ == nil {
		return ""
	}
	switch *typ {
	case "SYN":
		return locales.RelationSynonym()
	case "ANT":
		return locales.RelationAntonym()
	case "DER":
		return locales.RelationDerived()
	default:
		return *typ
	}
}
