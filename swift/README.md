# Rdict Swift Binding

Swift library for the Rdict dictionary format. Calls the Rust core via C ABI + static library.

## Usage

```swift
import Rdict

// Open dictionary
let dict = RdictDictionary(path: "/path/to/dict.rdict")
guard dict.isOpen else { return }

// Lookup
do {
    if let entry = try dict.lookup("run") {
        print(entry.headword)           // "run"
        print(entry.etymologies[0].root) // "rinnan"
        print(entry.morphology)          // [Morpheme(kind: "prefix", ...), ...]
    }
} catch {
    print("Lookup error: \(error)")
}

// List all headwords
let headwords = dict.listHeadwords()

// Get manifest
if let manifest = dict.manifest() {
    print(manifest.name)         // "English-Chinese (NGSL)"
    print(manifest.entryCount)   // 50
}

// Close
dict.close()
```

## API

### `RdictDictionary(path: String)`

Open a `.rdict` file. Check `isOpen` to verify success.

### `lookup(_ headword: String) throws -> DictEntry?`

Look up a headword. Returns the entry or `nil` if not found. May throw `LookupError`.

### `listHeadwords() -> [String]`

Returns all headwords (sorted).

### `manifest() -> ManifestResponse?`

Returns manifest info.

### `close()`

Close the dictionary and release resources. Automatically called in `deinit`.

## Build

Requires the Rust static library to be compiled first:

```bash
cd ../core && cargo build --release
cp target/release/librdict.a ../swift/lib/
cd ../swift && swift build --disable-sandbox
```

## Requirements

- Rust ≥ 1.85 (edition 2024)
- Swift 5.9+
- macOS 14.0+

## Architecture

```
Sources/
├── CRdict/              — C header (rdict.h + module.modulemap)
└── Rdict/               — Swift API layer
    ├── Models.swift         — Codable model types (DictEntry, Ety, Sense, ...)
    ├── RdictBridge.swift    — C FFI wrapper + LookupError
    └── RdictDictionary.swift — Public Swift API
lib/
└── librdict.a           — Rust static library (built from ../core)
```

## Types

All model types (`DictEntry`, `Ety`, `Sense`, `Definition`, `Example`, `Translation`, `Pron`, `Form`, `Morpheme`, `DefGroup`, `Note`, `TargetOccurrence`, `TextSpan`) are `public` and their properties can be accessed directly.
