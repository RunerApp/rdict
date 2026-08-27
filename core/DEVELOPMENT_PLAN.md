# Rdict Rust Development Plan

## 1. Development Root

`core/` is an independent Rust project and the only directory used for
this implementation.

Commands run from the repository root must use the explicit manifest:

```bash
cargo check --manifest-path core/Cargo.toml
cargo test --manifest-path core/Cargo.toml
cargo fmt --manifest-path core/Cargo.toml --check
cargo clippy --manifest-path core/Cargo.toml --all-targets -- -D warnings
```

This crate is a standalone project with its own `[workspace]` table.

## 2. Scope

Build a Rust library that reads and writes Rdict v0.1.0 distribution packs.
The first release includes:

- typed Rust AST model;
- manifest and string pools;
- zstd data blocks;
- front-coded exact headword index;
- ZIP/ZIP64 container access;
- Entry/Sense/Form pronunciation hierarchy;
- target-word spans in examples;
- media manifest and media references;
- optional tag, morphology, and relation indexes;
- malformed-input validation and round-trip tests.

The first release does not include the CLI, YAML source compiler, Node/Python
bindings, HTTP server, fuzzy search index, encryption, or incremental update
packs.

## 3. Specification Gate

Before coding, reconcile the implementation with the current
`../rdict-spec.md`:

1. `Entry` field order is `see`, `tag`, `media`, `pron`, `ety`.
2. `Entry.pron` is the entry-level pronunciation.
3. `Ety` contains only `id`, `root`, and `senses`; it has no `pron`.
4. `Sense.pron` is for a sense-specific pronunciation.
5. `Form.pron` is for a concrete form such as `ran` -> `/raen/`.
6. `Pron.value` is non-empty; audio-only pronunciation is unsupported.
7. `Pron.lang` is BCP-47 and `Pron.accent` is an optional label.
8. `media/manifest.json` is emitted only when media exists.

If an implementation detail conflicts with the spec, stop and update the
spec before writing the codec.

## 4. Project Layout

```text
core/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── error.rs
│   ├── model.rs
│   ├── manifest.rs
│   ├── primitive.rs
│   ├── strings.rs
│   ├── ast.rs
│   ├── blocks.rs
│   ├── index.rs
│   ├── postings.rs
│   ├── media.rs
│   ├── relations.rs
│   ├── container.rs
│   ├── writer.rs
│   └── reader.rs
└── tests/
    ├── roundtrip.rs
    ├── malformed.rs
    └── fixtures.rs
```

Use `edition = "2024"`. Keep public exports in `lib.rs`; internal binary
helpers stay private to their modules.

## 5. Dependencies

Add only dependencies required by the distribution format:

```toml
serde
serde_json
thiserror
uuid
semver
sha1
zstd
zip
```

Use `serde_yaml` only in a later source-compiler task. Do not depend on
`rkyv`; the Rdict format is deliberately varint-based and cross-language.

## 6. Public Model

Define owned Rust types in `model.rs`:

```text
Pack
PackMetadata
Entry
Ety
Sense
Definition
Group
Example
TargetOccurrence
TextSpan
Translation
Pron
Form
MediaRef
MediaAsset
Relation
```

Required invariants:

- headwords are non-empty and unique;
- `Pron.value` is non-empty;
- `Pron.kind` identifies notation, not language or accent;
- `Pron.lang` is optional BCP-47;
- absent `Pron.lang` inherits `Translation.lang`, then the nearest language
  context, then `pack.source_lang`;
- `Pron.accent` is never inferred from `lang`;
- `Form.pron` is independent from `Entry.pron` and `Sense.pron`;
- `TargetOccurrence` has one or more spans;
- a continuous phrase uses one span, a discontinuous phrase uses multiple
  spans in one occurrence, and repeated occurrences use separate records.

## 7. Primitive Encoding and Validation

Implement in `primitive.rs`:

- canonical unsigned LEB128 varints;
- overflow and truncation checks;
- little-endian fixed-width integers;
- length-prefixed UTF-8 strings;
- bounded readers that cannot read outside the enclosing file, block, or
  entry slice.

Reject malformed or non-canonical varints, invalid UTF-8, invalid counts,
and out-of-bounds offsets/sizes.

Validate example target spans as follows:

- offsets and lengths refer to UTF-8 payload bytes, excluding the length
  prefix;
- `offset + length` cannot exceed `value.len()`;
- boundaries must be UTF-8 code-point boundaries;
- spans cover literal text only, not markup or escape control bytes;
- spans in one occurrence are ordered and non-overlapping;
- occurrences in one example are ordered by their first span and do not
  overlap;
- `has_target` must be clear when the target list is empty.

## 8. String Pools

Implement the six independent pools in `strings.rs`:

```text
pos, lang, pron_kind, form_kind, tag, relation_type
```

Index 0 is implicit `unspecified`. Pool counts include that implicit value;
binary `strings.tbl` is authoritative and manifest arrays are consistency
copies. Enforce the u16 index limit and reject duplicate or inconsistent
pool entries.

## 9. AST Codec

Implement the complete §6 codec in `ast.rs`:

```text
Entry, Ety, Sense, Def, Definition, Group,
Example, Note, Translation, Pron, Form,
TargetOccurrence, TextSpan, MediaRef
```

Rules:

- optional fields are encoded in ascending flag-bit order;
- `Entry` uses the bit layout `see`, `tag`, `media`, `pron`, `ety`;
- `Ety` uses `id`, `root`, `sense`;
- all media arrays have an explicit count;
- unknown nested flags make the containing Entry opaque;
- unknown `Def.kind` makes the containing Entry opaque;
- opaque records are not errors and must not prevent later lookups.

Expose a lookup result that preserves this behavior. Decoded entries use
`Arc<Entry>` so eager-cache hits do not deep-clone the entry tree:

```rust
pub enum LookupEntry {
    Decoded(std::sync::Arc<Entry>),
    Opaque { raw: Vec<u8> },
}
```

## 10. Data Blocks and Headword Index

Implement in `blocks.rs` and `index.rs`:

- one zstd frame per `data/NNNNN.zst`;
- default target block size 256 KiB;
- entries never cross block boundaries;
- a large entry may occupy one block by itself;
- UTF-8 byte ordering for `sort_order = "codepoint"`;
- front-coding blocks of 256 headwords;
- 24-byte index header and 24-byte directory entries;
- absolute offsets for the first-headword pool and block body;
- complete-directory scan to compute the body start;
- rejection of overlapping, truncated, or out-of-bounds ranges;
- duplicate headword rejection before writing.

The reader keeps the index directory resident, decompresses only the
selected data block in Lazy mode, and uses a bounded 16 MiB LRU block cache.
The cache is not part of the on-disk format.

## 11. ZIP/ZIP64 Container

Implement in `container.rs`:

- ZIP64 read and write support;
- `mimetype` as the first member;
- store method for every member;
- no encryption;
- no data descriptors;
- UTF-8 filenames;
- unique member names;
- rejection of `.` and `..` path components;
- central-directory and local-header consistency checks.

Use `Write + Seek` for the writer so local headers can contain final sizes
and CRCs. Verify the produced bytes in tests rather than trusting library
defaults.

## 12. Manifest, Reader, and Writer API

Implement in `manifest.rs`, `writer.rs`, and `reader.rs`:

```rust
pub struct RdictWriter<W: Write + Seek>;
pub struct RdictReader<R: Read + Seek>;

impl RdictWriter<File> {
    pub fn create(path: impl AsRef<Path>) -> Result<Self>;
}

impl<W: Write + Seek> RdictWriter<W> {
    pub fn write_pack(&mut self, pack: &Pack) -> Result<()>;
}

impl RdictReader<File> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;
}

impl<R: Read + Seek> RdictReader<R> {
    pub fn lookup(&mut self, headword: &str) -> Result<Option<LookupEntry>>;
}
```

Writer pipeline:

```text
validate model
-> collect pools
-> encode AST entries
-> split zstd blocks
-> build headword index
-> build optional indexes
-> build media manifest
-> build manifest.json
-> write ZIP members in required order
```

Reader pipeline:

```text
parse ZIP central directory
-> validate mimetype and manifest
-> load strings.tbl and headword.idx
-> binary-search headword blocks
-> decompress one data block
-> slice entry bytes
-> decode Entry or return Opaque
```

Reader modes add an optional text-only eager path:

```text
Auto (default, 10 MiB conservative text limit)
-> Eager: decode all text entries into HashMap<String, LookupEntry>
-> Lazy: keep text blocks on demand
```

Media remains on demand in every mode. `preload()` performs the eager step
synchronously and requires exclusive `&mut self` access.

## 13. Media

Implement in `media.rs`:

- `(kind, hash)` uniquely identifies a media asset;
- SHA-1 covers logical media bytes;
- writer initially stores media with `compression = "none"`;
- reader supports manifest-declared zstd media;
- path is reconstructed from `(kind, hash, ext)`;
- MIME, size, uncompressed size, and hash are checked;
- zstd media do not advertise byte-range access.

## 14. Optional Indexes

Implement only after core lookup roundtrip works:

1. `index/tag.idx`;
2. `index/morph.idx`.

Each optional file needs independent header/version validation, bounded
offset checks, delta-encoded posting decode, and graceful fallback when
unknown header flags are present.

## 15. Tests and Acceptance

Required tests:

- minimal single-entry write/read roundtrip;
- Unicode and multi-byte headwords;
- duplicate headword rejection;
- redirects and entries with no Ety;
- Entry/Sense/Form pronunciation roundtrip;
- language, accent, and notation kind roundtrip;
- non-empty pronunciation enforcement;
- continuous and discontinuous target spans;
- repeated target occurrences;
- malformed UTF-8 and varints;
- span boundary and overlap rejection;
- unknown AST flag and unknown Def kind returning `Opaque`;
- multiple zstd blocks;
- ZIP64 fixture;
- duplicate ZIP names and unsafe paths;
- media hash/path validation;
- tag, morphology, and relation index roundtrips.

Acceptance commands:

```bash
cargo fmt --manifest-path core/Cargo.toml --check
cargo check --manifest-path core/Cargo.toml
cargo test --manifest-path core/Cargo.toml
cargo clippy --manifest-path core/Cargo.toml --all-targets -- -D warnings
```

The first implementation is complete only when the core reader/writer,
AST roundtrip, malformed-input tests, and ZIP validation pass. CLI and
language bindings are separate follow-up tasks.

## 17. Post-implementation Plan

The current reader already provides Lazy, Eager, and configurable Auto text
loading. The next work is deliberately narrow:

1. Add focused tests for `ReadMode`, `preload()`, the 10 MiB default, and
   eager/lazy result equivalence.
2. Measure real dictionaries, including uncompressed text size, eager memory
   peak, media-heavy pack size, and first-open latency.
3. Expose loading-mode selection through FFI and the Swift bridge; keep Lazy
   as the non-blocking default until the UI chooses otherwise.
4. Optimize the AST decoder only if real data shows it dominates Lazy lookup.
5. Make the Lazy block-cache budget configurable after measuring cache
   churn on larger packs; keep the default conservative until then.
6. Do not introduce rkyv or a second on-disk representation unless a
   representative workload demonstrates a material product benefit.
