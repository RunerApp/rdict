# Rdict — Core Library

Rust reference implementation of the Rdict dictionary format. Provides a writer, reader, and C ABI for `.rdict` files.

## Overview

Rdict is a fast, portable dictionary file format. Source files compile to compact binary `.rdict` files using varint encoding, zstd block compression, and a ZIP (store mode) container.

Features:
- **GB-scale support** — block-level zstd compression, lazy decompression, never loads the whole file
- **Fast random access** — front-coding index + block locator, 3M+ QPS in eager mode
- **Case-insensitive prefix search** — case-folded sort order, single binary search
- **Structured content** — native AST (Entry → Ety → Sense → Definition), not HTML strings
- **Audio/video media** — content-addressed dedup, hash-named
- **Morphology & etymology** — `morphology` field for affix decomposition, `Ety.root` for historical roots
- **Forward compatible** — flag bits + opaque record fallback; new readers always read old files

## Usage

### Writing

```rust
use rdict::{Pack, PackMetadata, Entry, RdictWriter};
use std::io::Cursor;

let pack = Pack {
    metadata: PackMetadata {
        name: "Test Dict".into(),
        version: "0.1.0".into(),
        source_lang: "en".into(),
        target_langs: vec!["zh-Hans".into()],
        zstd_level: 19,
        ..Default::default()
    },
    entries: vec![Entry::new("hello")],
    ..Default::default()
};

let mut buf = Vec::new();
RdictWriter::write_pack(Cursor::new(&mut buf), &pack).unwrap();
```

### Reading

```rust
use rdict::{RdictReader, ReadMode, LookupEntry};

// Eager mode: preload all entries for fastest lookups
let mut reader = RdictReader::open_with_mode("dict.rdict", ReadMode::Eager).unwrap();

match reader.lookup("hello").unwrap().unwrap() {
    LookupEntry::Decoded(entry) => println!("{}", entry.headword),
    LookupEntry::Opaque { .. } => println!("unknown format"),
}
```

### Read Modes

| Mode | Open time | Lookup speed | Memory |
|------|-----------|-------------|--------|
| `Lazy` | ~0.2ms | 169K QPS | Low (on-demand decompression + LRU) |
| `Eager` | ~44ms | 3M QPS | High (all entries pre-decoded) |
| `Auto` | Auto-selects | — | — |

## Project Structure

```
src/
├── lib.rs          — Public exports
├── model.rs        — Owned AST types + validate_pack
├── ast.rs          — Binary encode/decode (§6)
├── primitive.rs    — LEB128 varint, fixed-width ints, str/bytes
├── strings.rs      — Six-pool string interning
├── blocks.rs       — zstd block compression
├── index.rs        — headword.idx front-coding index + prefix search
├── postings.rs     — tag.idx / morph.idx inverted indexes
├── media.rs        — Media manifest + SHA-1 dedup
├── container.rs    — ZIP store container
├── writer.rs       — Full write pipeline
├── reader.rs       — Reader (Lazy/Eager/Auto)
├── ffi.rs          — C ABI (for Swift/Python FFI)
└── manifest.rs     — manifest.json
```

## Build

```bash
cargo build --release          # Build library + staticlib
cargo test                     # Run tests
cargo clippy -D warnings       # Lint
cargo fmt --check              # Format check
```

## Requirements

- Rust ≥ 1.85 (edition 2024)

## Dependencies

Only: `serde`, `serde_json`, `thiserror`, `uuid`, `semver`, `sha1`, `zstd`, `zip`

## Specification

See `../rdict-spec.md` for the complete format specification.
