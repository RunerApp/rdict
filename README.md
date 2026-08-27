# Rdict

A fast, portable dictionary file format. YAML sources compile to compact binary `.rdict` files using varint encoding, zstd block compression, and a ZIP (store mode) container.

## Why Rdict?

- **GB-scale support** — block-level zstd compression, lazy decompression, never loads the whole file
- **Fast random access** — prefix index + block locator, 3M+ QPS in eager mode
- **Structured content** — native AST (Entry → Ety → Sense → Definition), not HTML strings
- **Audio/video media** — content-addressed dedup, hash-named
- **Morphology & etymology** — `morphology` field for affix decomposition, `Ety.root` for historical roots
- **Forward compatible** — flag bits + opaque record fallback; new readers always read old files
- **Cross-platform** — Rust core, bindings for Node.js, Swift, Python (planned)

## Repository Structure

```
rdict/
├── rdict-spec.md       — Format specification v0.1.0
├── core/               — Rust core library (rlib + staticlib + C ABI)
├── compiler/           — YAML → .rdict compiler
├── node/               — Node.js binding (napi-rs)
├── swift/              — Swift binding library
└── apps/
   └── apple/           — SwiftUI dictionary viewer (macOS + iOS)
```

## Quick Start

### Build the core library

```bash
cd core
cargo build --release
cargo test
```

### Compile a dictionary

```bash
cd compiler
cargo run -- input.yaml -o output.rdict
```

### Node.js

```bash
cd node
npm install -D @napi-rs/cli
npx napi build --release
node try.js /path/to/dict.rdict run
```

### Swift app (macOS / iOS)

```bash
./apps/apple/build.sh                # macOS
open apps/apple/RdictApp.app
./apps/apple/build-ios.sh            # iOS simulator
```

## Performance

**Test conditions:**
- Machine: MacBook Pro M1 Pro, 16 GB RAM, macOS 14.5
- Rust: stable aarch64-apple-darwin, `--release`
- Data: 50,000 synthetic entries, each with headword, POS, definition, example, Chinese translation (~200 bytes/entry)
- zstd level 3
- Query: 10,000 random lookups (DefaultHasher seed → index)
- Block cache: 16 MiB LRU (Lazy mode)

| Metric | Lazy | Eager |
|--------|------|-------|
| File size | 0.67 MB | — |
| Write | 45ms | — |
| Open | 0.24ms | 44ms |
| Random lookup 10K | 59.1ms | 4.93ms |
| QPS | 169K | 3.08M |

- **Lazy** — On-demand block decompression + AST decode, minimal memory. Best for large dictionaries or one-off lookups.
- **Eager** — Preload + decode all entries on open, HashMap lookup O(1). Best for repeated queries.
- **Auto** (default) — Auto-selects based on text size (10 MiB threshold).

Full report: [`core/PERFORMANCE_REPORT.md`](core/PERFORMANCE_REPORT.md)

## Format Overview

```
.rdict (ZIP store)
├── manifest.json           — metadata, languages, version
├── index/
│   ├── headword.idx        — prefix-compressed headword index
│   ├── strings.tbl         — six string pools (pos, lang, pron-kind, form-kind, tag, relation-type)
│   ├── tag.idx             — entry tag inverted index (optional)
│   └── morph.idx           — morphological features inverted index (optional)
├── data/
│   ├── 00000.zst           — zstd-compressed entry blocks
│   ├── 00001.zst
│   └── ...
├── media/
│   └── <sha1>.<ext>        — content-addressed media files
└── (no relations file — relations are inline on Entry AST)
```

See [`rdict-spec.md`](rdict-spec.md) for the complete specification (~1500 lines).

## License

GPL-3.0. See [LICENSE](LICENSE).
