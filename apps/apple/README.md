# Rdict App (macOS / iOS)

SwiftUI dictionary viewer for macOS and iOS. Uses the [swift](../../swift/) binding library.

## Features

- **Multiple dictionaries** — Open and manage multiple `.rdict` files in the sidebar
- **Cross-dictionary search** — Search headwords across all open dictionaries
- **Headword filtering** — Real-time filter for the current dictionary's word list
- **Structured display** — POS, definitions, examples, translations, pronunciation
- **Morphology display** — Affix decomposition with prefix/root/suffix tags
- **Etymology root** — `Ety.root` shown with a tree icon
- **Cross-reference navigation** — Clickable `see` field for redirects
- **Example highlighting** — Target span highlights the target word
- **Session restore** — Remembers last opened dictionaries

## Screenshot

```
┌─────────────┬──────────────────────────────────┐
│ Dictionaries│                                  │
│ ┌─────────┐ │  run                              │
│ │ NGSL 50 │ │  [en] (ipa) /rʌn/                │
│ └─────────┘ │                                  │
│             │  🌳 rinnan                        │
│ ─────────── │                                  │
│ run         │  Sense #1: v                     │
│ run         │    [zh-Hans] 跑；运行             │
│ runner      │    • To move swiftly on foot.     │
│ running     │      He runs every morning.       │
│ ...         │        → 他每天早上跑步。          │
└─────────────┴──────────────────────────────────┘
```

## Build

### macOS

```bash
./build.sh
```

Or manually:

```bash
# 1. Build Rust static library
cd ../../core && cargo build --release

# 2. Copy to Swift library
cp target/release/librdict.a ../../swift/lib/

# 3. Build Swift library + app
cd ../../swift && swift build --disable-sandbox -c release
cd ../../apps/apple && swift build --disable-sandbox

# 4. Assemble .app bundle
mkdir -p RdictApp.app/Contents/MacOS
cp .build/debug/RdictApp RdictApp.app/Contents/MacOS/
codesign --force --deep --sign - RdictApp.app
```

### iOS

```bash
./build-ios.sh                # Build for iOS simulator
```

Requires [xcodegen](https://github.com/yonaskolb/XcodeGen) for project generation.

## Run

### macOS

```bash
open RdictApp.app
```

### iOS

```bash
open -a Simulator
xcodebuild -scheme RdictApp -destination 'platform=iOS Simulator,name=iPhone 15' \
  -package-path . -derivedDataPath .build/ios build test
```

## Dependencies

- [swift](../../swift/) — Swift binding library
- Swift 5.9+
- macOS 14.0+ / iOS 17.0+
