# Rdict Format Specification

**Name:** Rdict
**Container extension:** `.rdict`
**Format version:** 0.1.0 (draft)
**Status:** Draft — subject to change.

> This format specification draws on open lexicographic standards
> (LIFT, TEI P5 dictionary module, WordNet relation model). The
> container is ZIP (ISO/IEC 21320-1).

---

## 1. Goals

### Goals

- **Small**: competitive with the best practical dictionary formats.
- **Fast random access**: lookup latency independent of total file size;
  no whole-file decompression; no whole-file memory residency.
- **GB-scale**: multi-gigabyte packs (mostly media) usable on memory-
  constrained clients (mobile, WASM, embedded).
- **Rich media**: audio/video/images as first-class, streamable citizens.
- **Structured content**: entries are a typed AST, not pre-rendered HTML
  strings. Consumers render, not parse display markup.
- **Standardized & open**: any conforming implementation can read a pack
  by following this spec, in any language, with no dependency on the
  reference implementation.
- **Evolvable**: forward/backward compatibility without format forks.

---

## 2. Architecture

Two layers, decoupled:

```
Source format (human/program editable, readability first)
   │  compile
   ▼
Distribution format (`.rdict`, size + speed first)
```

### 2.1 Source format

- **YAML** is the canonical authoring form; **JSON** is the identical
  schema for program generation. Either compiles to the same pack.
- Schema is defined in §3 (manifest) and §6 (entry AST) — the same field
  names and structure, just readable.
- The source format is **not** normative for interchange; the `.rdict`
  is. Source format exists for authoring convenience.
- Media references use `path` instead of `hash` in source; see §7.4.

### 2.2 Distribution format — container

A `.rdict` is a **ZIP archive** (ISO/IEC 21320-1, APPNOTE 6.2.0+).
Writers and readers MUST support ZIP64 (APPNOTE 4.5) when an archive,
member, or offset exceeds classic ZIP limits.

**Mandatory container rules:**

1. **All entries use `method=store` (0).** The ZIP layer performs no
   compression. Compression is the responsibility of individual files
   (zstd blocks for text; media stored raw).
   - Rationale: lets zip act as a random-access index; avoids
     double-compression; keeps media byte-exact and range-able.
2. **No ZIP encryption.** Encryption is out of scope for v0.1.
3. **No data descriptors (bit 3 of general purpose flag = 0).** All
   sizes/CRCs must be present in local file headers so readers can
   pread without scanning.
4. **File names are UTF-8** (general purpose bit 11 set).
5. The first entry **must** be `mimetype` containing the ASCII bytes
   `application/rdict` with no trailing newline, stored, not
   compressed. (Fast type sniffing by reading the first ~30 bytes.)
6. File names MUST be unique. Readers MUST reject an archive containing
   duplicate names or a member path containing `.` or `..` components.

**Mandatory directory layout:**

```
mydict.rdict
├── mimetype                  ← "application/rdict" (store, first)
├── manifest.json             ← pack metadata (see §3)
├── cover.png                 ← optional cover image (see §3.2)
├── index/
│   ├── headword.idx          ← binary headword index (see §4)
│   ├── morph.idx             ← morph feats inverted index (see §4.6); optional
│   ├── tag.idx               ← entry-tag inverted index (see §4.7); optional
│   └── strings.tbl           ← dictionary-level string table (see §5)
├── data/
│   ├── 00000.zst             ← text blocks, zstd-compressed (see §6)
│   ├── 00001.zst
│   └── ...
└── media/
    ├── manifest.json             ← required when any media is present
    ├── audio/<hh>/<hash>.<ext>
    ├── image/<hh>/<hash>.<ext>
    └── video/<hh>/<hash>.<ext>
```

`<hh>` is the first two hex chars of the content hash; `<hash>` is the
full lowercase hex SHA-1. See §7.

**Conforming readers MUST NOT require the pack to be unpacked to disk.**
All access is via the ZIP central directory + offset reads.

---

## 3. `manifest.json`

JSON, UTF-8. Defines pack-level metadata and shared tables.

```json
{
  "format": "rdict",
  "format_version": "0.1.0",
  "pack": {
    "id": "uuid-v4-string",
    "name": "Example Dictionary",
    "version": "1.0.0",
    "source_lang": "en",
    "target_langs": ["zh", "ja"],
    "author": "Author Name",
    "license": "CC-BY-SA-4.0",
    "created_at": "2026-08-20T00:00:00Z",
    "description": "Optional human-readable description.",
    "cover": "cover.png"
  },
  "index": {
    "headword_file": "index/headword.idx",
    "entry_count": 123456,
    "block_count": 512
  },
  "strings_file": "index/strings.tbl",
  "pos_table": ["NOUN", "VERB", "ADJ", "ADV", "ADP", "DET", "PRON", "NUM", "CCONJ", "SCONJ", "AUX", "PART", "INTJ", "PROPN", "PUNCT", "SYM", "X"],
  "lang_table": ["en", "zh", "ja"],
  "pron_kind_table": ["ipa", "pinyin", "hira", "kana", "roma", "yale", "jyut", "bopo", "hepburn"],
  "form_kind_table": ["infl", "pl", "sg", "comp", "sup"],
  "tag_table": ["exam:IELTS", "exam:TOEFL", "level:B2", "freq:coca=3000", "domain:medical"],
  "relation_type_table": ["syn", "ant", "phrase", "der", "hyp", "hypo", "hol", "mer", "see"],
  "data": {
    "block_max_uncompressed": 262144,
    "compression": "zstd",
    "zstd_level": 19,
    "block_count": 512
  },
  "morph": {
    "index_file": "index/morph.idx",
    "has_index": true,
    "key_count": 42
  },
  "tags": {
    "index_file": "index/tag.idx",
    "has_index": true,
    "tag_count": 5
  },
  "media": {
    "hash_algo": "sha1",
    "total_size": 5368709120,
    "dedup": true
  }
}
```

### 3.1 Field semantics

| Field | Type | Required | Notes |
|---|---|---|---|
| `format` | string | yes | Must be `"rdict"`. |
| `format_version` | string | yes | Semver of **this spec** the pack targets. |
| `pack.id` | string | yes | UUID v4. Generated at compile time. |
| `pack.name` | string | yes | Human-readable dictionary name. |
| `pack.source_lang` | string | yes | BCP-47 language tag of headwords. |
| `pack.target_langs` | string[] | yes | BCP-47 tags used in translations. May be empty. If exactly one tag is present, `Translation.lang` may be omitted in source (see §6.7). |
| `pack.version` | string | yes | Semver of **the dictionary content**. |
| `pack.author` / `pack.license` / `pack.created_at` / `pack.description` | string | no | Optional descriptive metadata. `created_at`, when present, is RFC 3339 UTC. |
| `pack.cover` | string | no | ZIP member path of a cover image (e.g. `"cover.png"`). The image is stored uncompressed in the ZIP root. Format is auto-detected from file extension; readers SHOULD support PNG and JPEG. No size or dimension limits in v0.1. See §3.2. |
| `index.headword_file` | string | yes | Path to `headword.idx`; must equal `index/headword.idx` in v0.1. |
| `index.entry_count` / `index.block_count` | int | yes | Must equal `headword.idx` `entry_count` / front-coding `block_count`, respectively. |
| `strings_file` | string | yes | Path to `strings.tbl`; must equal `index/strings.tbl` in v0.1. |
| `pos_table` / `lang_table` / `pron_kind_table` / `form_kind_table` / `tag_table` / `relation_type_table` | string[] | yes* | Advisory copies of the nonzero string-pool values, in `strings.tbl` order. `strref` index 0 is reserved for unspecified (see §5) and is never present in these arrays. `tag_table` is required only if entries use tags; `relation_type_table` is required only if any Entry has `has_relations` set. |
| `data.block_max_uncompressed` | int | yes | Target uncompressed size of a text block (default 262144 = 256 KiB). |
| `data.compression` | string | yes | `"zstd"`. Only zstd is defined in v0.1. |
| `data.block_count` | int | yes | Must equal `headword.idx.data_block_count`; every ID from 0 to `block_count - 1` names one `data/NNNNN.zst` member. |
| `morph.index_file` | string | no | Path to the morph feats inverted index (default `index/morph.idx`). Omit if no feats data. |
| `morph.has_index` | bool | no | `true` if `morph.idx` is present. If `false` or absent, consumers must scan entries to query feats. |
| `morph.key_count` | int | no | Number of distinct `key=value` pairs indexed. |
| `tags.index_file` | string | no | Path to the tag inverted index (default `index/tag.idx`). Omit if no entry tags. |
| `tags.has_index` | bool | no | `true` if `tag.idx` is present. If `false` or absent, consumers must scan entries to query tags. |
| `tags.tag_count` | int | no | Number of distinct tags indexed. |
| `media.hash_algo` | string | yes | `"sha1"`. Only sha1 is defined in v0.1. |
| `media.dedup` | bool | yes | Whether media were content-addressed. If `true`, identical content is stored once. |
| `media.total_size` | int | yes | Sum of stored media payload sizes, or 0 if no media members exist. |

### 3.2 Cover image

The cover image is an optional decorative image shown by dictionary apps
when listing or previewing the dictionary. It is not content metadata.

- Stored as a single ZIP member at the path specified by `pack.cover`
  (conventionally `"cover.png"` or `"cover.jpg"` in the ZIP root).
- Stored uncompressed (`method=store`), like all members.
- No size or dimension limits in v0.1; compilers SHOULD warn if the image
  exceeds 2 MiB.
- Readers auto-detect format from the file extension (`.png`, `.jpg`,
  `.jpeg`). Support for PNG and JPEG is REQUIRED; WebP and HEIC are
  optional.
- If `pack.cover` is absent or the member is missing, the dictionary has
  no cover; apps SHOULD render a placeholder.

### 3.3 Backward compatibility

Readers MUST ignore unknown top-level keys in `manifest.json`. Unknown
keys inside known objects MUST also be ignored. Readers handle an
unsupported `format_version` according to §9.2.

### 3.3 Recommended short codes for pools

The `*_table` arrays are open: a compiler MAY use any strings. To keep
pools compact and stable across dictionaries, v0.1 recommends these
short-code vocabularies. Using them is optional but advised for
interoperability.

**`pos_table` — Universal Dependencies v2 POS tags** (17 universal
categories, language-agnostic). Values MUST be the uppercase UPOS short
codes below; lowercase aliases (`n`, `v`, `adj`, ...) are forbidden:

| Code | Meaning |
|---|---|
| `NOUN` | noun |
| `VERB` | verb |
| `ADJ` | adjective |
| `ADV` | adverb |
| `ADP` | adposition (prep/postposition) |
| `DET` | determiner |
| `PRON` | pronoun |
| `NUM` | numeral |
| `CCONJ` | coordinating conjunction |
| `SCONJ` | subordinating conjunction |
| `AUX` | auxiliary |
| `PART` | particle |
| `INTJ` | interjection |
| `PROPN` | proper noun |
| `PUNCT` | punctuation |
| `SYM` | symbol |
| `X` | other / unknown |

Compilers needing language-specific subtypes (e.g. Japanese Godan
classes) SHOULD encode them as `Form` records with tags, not as new
POS values — keeping the POS pool aligned with UD's universal set.

**`pron_kind_table`:**

`Pron.kind` identifies the notation or transcription system, not the
language or accent. Language comes from `Pron.lang` when present and
accent comes from `Pron.accent` when present (§6.8).

| Code | Meaning |
|---|---|
| `ipa` | International Phonetic Alphabet |
| `pinyin` | Mandarin Pinyin |
| `hira` | Hiragana |
| `kana` | Katakana |
| `roma` | Romaji |
| `yale` | Korean Yale romanization |
| `jyut` | Cantonese Jyutping |
| `bopo` | Bopomofo / Zhuyin |
| `hepburn` | Hepburn romanization |

`yale` means Yale romanization. The recommended use is Korean Yale
with `lang = ko`; the same kind code may be used for another Yale
romanization when its `lang` identifies the language. `yale` is not an
accent.

**`form_kind_table`:**

| Code | Meaning |
|---|---|
| `infl` | inflection |
| `pl` | plural |
| `sg` | singular |
| `comp` | comparative |
| `sup` | superlative |

**`relation_type_table`** — recommended codes: `syn` (synonym), `ant`
(antonym), `phrase` (multiword-phrase reference), `der` (derivation),
`hyp` (hypernym), `hypo` (hyponym), `hol` (holonym), `mer` (meronym),
`see` (see-also). These codes are conventions; a compiler MAY use any
strings. Pool indices are **per-pack** and are **not fixed by the spec**
— a reader MUST resolve `strref` values through the current pack's
`strings.tbl`. See §6.11 for the inline `Relation` record.

### 3.4 Morphological features (`feats`)

`feats` is an optional string field on `Form` (§6.8) carrying
language-specific morphological information: Japanese verb conjugation
class, Slavic verb aspect, Finno-Ugric noun case, etc. It is the
escape hatch for morphology that does not fit the universal POS pool.

**Encoding:**

- Format: `key=value` pairs joined by `|`, no spaces.
- Example: `ud:ConjugationType=Godan|ud:ConjugationForm=Ta`
- Empty feats string = no features. `has_feats` flag SHOULD be 0 in
  that case.
- Key and value are open strings; the format does not define a
  registry. The `|` and `=` characters inside a key or value MUST be
  escaped as `\|` and `\=` respectively; `\\` is a literal backslash.

**Prefix convention:**

To avoid collisions when a dictionary mixes feature vocabularies,
keys SHOULD be namespaced with a lowercase prefix and colon:

| Prefix | Vocabulary | Example |
|---|---|---|
| `ud:` | Universal Dependencies treebank features (language-specific extensions included) | `ud:ConjugationType=Godan` |
| `jm:` | JMDict POS/inflection tags | `jm:pos=v5r\|jm:form=iru` |
| `unidic:` | UniDic lemma/inflection types | `unidic:infType=動詞%五段` |
| `iso:` | ISO 12620 terminology categories | `iso:partOfSpeech=noun` |
| `custom:` | Dictionary-defined private vocabulary | `custom:tone=2` |

Prefixes are **advisory, not enforced**. A reader treats `ud:Foo=Bar`
and `jm:Foo=Bar` as two independent keys. Cross-vocabulary mapping
(e.g. JMDict `v5r` ↔ UD `ConjugationType=Godan`) is out of scope —
it is a consumer-side concern.

**Within one dictionary, a compiler SHOULD use a single vocabulary
consistently.** Mixing `ud:` and `jm:` in the same pack is legal but
breaks cross-entry queries on the same key.

**Recommended vocabularies** (non-normative; consumers decide which
to support):

- Japanese: `ud:` with keys from UD Japanese-GSD treebank
  (`ConjugationType`, `ConjugationForm`), or `jm:` with JMDict
  tag values.
- Slavic: `ud:` with `Aspect` (Imp/Perf), `Case`, `Gender`, etc.
- Arabic: `ud:` with `Form`, `Voice`, `Mood`.
- Other languages: consult UD treebanks for the language.

A consumer that does not understand a vocabulary SHOULD render feats
as a literal string or hide it; it MUST NOT error.

**Querying feats:** see §4.6 for the optional `index/morph.idx`
inverted index. Without it, the only way to query is scanning all
entries — acceptable for small dictionaries, not for large ones.

### 3.5 Entry tags

Entry-level `tags` (§6.4) are short strings classifying the whole
headword by **usage context, difficulty, domain, frequency**, etc.
They are interned in the `tag_pool` (§5) and indexed via the optional
`index/tag.idx` (§4.7), enabling queries like "all IELTS entries" or
"all B2-level entries".

**Encoding:**

- A tag is a single string. v0.1 recommends the `category:value`
  convention below, but the field is open — any string is legal.
- Tags live in the `tag_pool` and are referenced as `strref` (u16) from
  the Entry AST. This keeps the AST compact (2 bytes per tag) and
  enables fast inverted-index lookups.
- Sense and Form tags (§6.6, §6.8) are **separate** fields: they use
  open `str` arrays (not interned) and are not indexed in v0.1. This
  reflects their different semantics (sense/form-specific notes vs.
  whole-entry classifications) and lower query frequency.

**Recommended `category:value` convention:**

Tags SHOULD follow `category:value`, where `category` is a short
lowercase name and `value` is the category-specific code. Common
categories:

| Category | Meaning | Example values |
|---|---|---|
| `exam:` | Standardized exam the word appears in | `IELTS`, `TOEFL`, `CET-4`, `CET-6`, `GRE`, `GMAT`, `JLPT-N1` |
| `level:` | Difficulty level (CEFR or equivalent) | `A1`, `A2`, `B1`, `B2`, `C1`, `C2` |
| `domain:` | Subject domain | `medical`, `legal`, `tech`, `finance`, `literary` |
| `register:` | Register / tone | `formal`, `informal`, `archaic`, `slang`, `poetic` |
| `freq:` | Frequency rank from a named corpus. Format: `freq:<source>=<rank>` | `freq:coca=3000`, `freq:bnc=4500` |

**`freq:` details:**

- Replaces a dedicated `rank` numeric field. Multiple sources can
  coexist on the same entry: `freq:coca=3000`, `freq:bnc=4500`.
- Lower number = higher frequency / more common, by convention.
- The source name (e.g. `coca`, `bnc`) is free-form; the spec does not
  maintain a registry. Common sources: `coca` (Corpus of Contemporary
  American English), `bnc` (British National Corpus), `subtlex`,
  `jawiki` (Japanese Wikipedia).
- Sorting / range queries on `freq:` values are a **consumer-side**
  concern: the consumer parses the numeric value out of the string.
  The inverted index `tag.idx` provides exact-match lookup only
  (e.g. "all entries tagged `freq:coca=3000`"); for "top-N by
  frequency" a consumer scans matching entries and sorts. A
  dedicated numeric frequency index may be added in a future version.

**A consumer that does not recognize a category** SHOULD render the
tag literally or hide it; it MUST NOT error.

---

## 4. `index/headword.idx`

Binary, little-endian, **not compressed** (loaded once, kept resident).
Maps headword → `(data_block_id, offset, size)`.

### 4.1 File header (24 bytes)

| Offset | Size | Field | Value |
|---|---|---|---|
| 0 | 4 | magic | `RDID` (`52 44 49 44`) |
| 4 | 2 | version | `u16` = 1 |
| 6 | 2 | flags | `u16` = 0 (all bits reserved, MUST be 0) |
| 8 | 4 | entry_count | `u32`; must equal the manifest value. |
| 12 | 4 | block_count | `u32` (number of front-coding blocks, not data blocks) |
| 16 | 4 | data_block_count | `u32` (number of `data/*.zst` files; must equal `data.block_count`) |
| 20 | 4 | reserved | `u32` = 0 |

### 4.2 Block directory

Immediately after the header: `block_count` entries, each **24 bytes**,
sorted ascending by the block's first headword (per §4.2.1).
Used for binary search.

| Size | Field |
|---|---|
| 2 | `first_headword_len` (`u16`, bytes) |
| 2 | `pad` (`u16` = 0) |
| 8 | `first_headword_offset` (`u64`) — absolute byte offset from the file start where this block begins |
| 4 | `block_entry_count` (`u32`) |
| 8 | `first_headword_pool_offset` (`u64`) — absolute byte offset from the file start in the first-headword pool |

> **Note:** to keep directory entries fixed-size, first-headword bytes
> for all blocks are stored in one contiguous pool immediately after the
> directory. Each directory entry gives that headword's byte length and
> absolute file offset in the pool. Readers pread the directory (24 B × N), then lazily
> fetch comparison headwords during binary search.

**Practical simplification:** for `entry_count < 1<<16`, a reader may
load the entire `first_headword` pool (a few hundred KB) into memory at
startup; binary search then needs no extra IO.

#### 4.2.1 Sort order

Headwords are sorted by **case-folded codepoint order** (Unicode simple
case-folding, equivalent to Rust `char::to_lowercase`), with
**case-sensitive codepoint order as tie-breaker**.

Concretely, two headwords `a` and `b` are compared as follows:

1. Compare `lowercase(a)` vs `lowercase(b)` byte-by-byte (UTF-8).
2. If equal, compare `a` vs `b` byte-by-byte (original case).

This means `Apple` and `apple` are adjacent, with `Apple` before
`apple` (uppercase bytes < lowercase bytes in UTF-8). CJK, Arabic, and
other scripts without case distinctions sort identically to plain
codepoint order.

**Rationale:** a single binary search on this ordering suffices for
both case-insensitive prefix search and case-sensitive exact lookup
(the latter via a secondary case-sensitive comparison within the
matched block).

### 4.3 Index body

A sequence of front-coding blocks. Each block is `block_entry_count`
entries, back-to-back:

```
entry :=
  shared_len: varint       // bytes shared with previous entry (0 for first in block)
  suffix_len: varint       // bytes of this entry's unique suffix
  suffix: [u8] × suffix_len   // UTF-8
  data_block_id: varint    // which data/NNNNN.zst
  offset: varint           // byte offset of this entry inside the decompressed block
  size: varint             // byte length of this entry's AST inside the block
```

`suffix_len` is a varint (not a fixed u8) so headwords of arbitrary
byte length are supported. The **first entry of each block** has
`shared_len = 0` (it stores the full headword as its suffix). This lets
a block be decoded in isolation without the previous block.

The index body begins immediately after the first-headword pool, with
no padding. Its offset is the maximum of
`first_headword_pool_offset + first_headword_len` across the directory,
or the end of the directory when `block_count` is zero. Because readers
already load the complete fixed-size directory, they MUST compute this
maximum once and reject overlapping or out-of-bounds pool/body ranges.

### 4.4 Lookup algorithm

```
1. Read 24-byte header.
2. Read block directory (24 × block_count bytes).
3. Binary-search directory by first headword (fetching comparison
   headwords from the pool as needed).
4. Seek to the matched block's body offset; scan block entries (block
   is small, ~256 entries) decoding front-coding until match.
5. Return (data_block_id, offset, size).
6. Reader opens data/<id zero-padded to 5>.zst, decompresses (cached),
   slices [offset, offset+size), decodes AST (§6).
```

### 4.5 Prefix search

The case-folded sort order (§4.2.1) enables **case-insensitive prefix
search** via a single binary search:

1. Case-fold the query prefix.
2. Binary-search the block directory for the first block whose
   lowercased first headword is >= the lowercased prefix.
3. Scan forward from that position, decoding front-coding entries,
   collecting headwords whose lowercased form starts with the
   lowercased prefix.
4. Stop when a headword's lowercased form exceeds the prefix, or when
   `limit` results have been collected.

The returned headwords are in original case (not folded). The caller
controls the result limit; a limit <= 0 returns an empty list.

**Exact lookup** (§4.4) also benefits: the binary search uses
case-folded comparison to locate the block, then a case-sensitive
comparison within the block distinguishes `Apple` from `apple`.

### 4.6 `index/morph.idx` (optional)

An **inverted index** over `feats` (§3.4) enabling O(log N + K) lookup
of entries by morphological feature, e.g. "all entries whose any Form
has `ud:ConjugationType=Godan`". Without this file, the only way to
query feats is scanning all entries.

**When to emit:** a compiler SHOULD emit `morph.idx` when the source
dictionary has any `feats` data. A compiler MAY omit it for small
dictionaries where full scan is acceptable. Its presence is declared
in `manifest.json` (§3).

**File header (16 bytes):**

| Offset | Size | Field | Value |
|---|---|---|---|
| 0 | 4 | magic | `RDMI` (`52 44 4D 49`) |
| 4 | 2 | version | `u16` = 1 |
| 6 | 2 | flags | `u16` = 0 (reserved) |
| 8 | 4 | key_count | `u32` — number of distinct `key=value` pairs indexed |
| 12 | 4 | reserved | `u32` = 0 |

**Posting list directory** (immediately after header, `key_count` entries):

| Size | Field |
|---|---|
| varint | `key_len` — bytes of the `key=value` string (including prefix like `ud:ConjugationType=Godan`) |
| `key_len` | `key_bytes` — UTF-8 |
| 8 | `posting_offset: u64` — byte offset of this key's posting list in the body |
| 4 | `posting_count: u32` — number of entries in the list |

The directory is sorted ascending by `key_bytes` for binary search.
`posting_offset` is relative to the first byte of the posting-list body.

**Posting lists** (body, after directory):

Each posting list is a run of varint-encoded **entry ids**. An entry id
is the entry's **0-based ordinal in the headword index** (§4). Posting
lists are delta-encoded for compactness:

```
posting_list := count × ( delta: varint )   // entry_id[i] = entry_id[i-1] + delta
                                          // entry_id[0] = delta_0 (absolute, previous = 0)
```

**Semantics:**

- A single `key=value` appears once in the index if **any** Form of an
  entry has it. The entry id is listed once per distinct key=value, not
  once per occurrence.
- An entry with `feats="ud:A=X|ud:B=Y"` contributes to two posting
  lists: `ud:A=X` and `ud:B=Y`.
- Compound queries (AND / OR / NOT) are not defined by the format. A
  consumer reads the relevant posting lists and performs set operations
  in application code (intersection for AND, union for OR, complement
  for NOT against the full entry-id range).

**Lookup example** — "all Godan verbs":

```
1. Binary-search directory for key "ud:ConjugationType=Godan".
2. Read posting_count, pread posting list at posting_offset.
3. Decode delta-encoded entry ids.
4. For each id, resolve via headword index → entry.
```

**Forward compatibility:** unknown flag bits in the header cause the
reader to ignore the file and fall back to scanning. New header fields
go in reserved space with a version gate.

### 4.7 `index/tag.idx` (optional)

An **inverted index** over entry-level `tags` (§3.5, §6.4) enabling
O(log N + K) lookup of entries by tag, e.g. "all entries tagged
`exam:IELTS`". Structurally symmetric to `morph.idx` (§4.6), but keys
are **strref ids** into the `tag_pool` (§5) rather than raw strings.

**When to emit:** a compiler SHOULD emit `tag.idx` when the source
dictionary has any entry-level tags. A compiler MAY omit it for small
dictionaries where full scan is acceptable. Its presence is declared
in `manifest.json` (§3).

**File header (16 bytes):**

| Offset | Size | Field | Value |
|---|---|---|---|
| 0 | 4 | magic | `RDTI` (`52 44 54 49`) |
| 4 | 2 | version | `u16` = 1 |
| 6 | 2 | flags | `u16` = 0 (reserved) |
| 8 | 4 | tag_count | `u32` — number of distinct tags indexed |
| 12 | 4 | reserved | `u32` = 0 |

**Posting list directory** (immediately after header, `tag_count` entries):

| Size | Field |
|---|---|
| 2 | `tag_id: u16` — strref index into the `tag_pool` |
| 2 | `pad: u16` = 0 (reserved, keeps entries 8-byte aligned) |
| 8 | `posting_offset: u64` — byte offset of this tag's posting list in the body |
| 4 | `posting_count: u32` — number of entries tagged with this tag |

The directory is sorted ascending by `tag_id` for binary search.
`posting_offset` is relative to the first byte of the posting-list body.

**Posting lists** (body, after directory):

Each posting list is a run of varint-encoded **entry ids**. An entry id
is the entry's **0-based ordinal in the headword index** (§4). Posting
lists are delta-encoded for compactness:

```
posting_list := count × ( delta: varint )   // entry_id[i] = entry_id[i-1] + delta
                                          // entry_id[0] = delta_0 (absolute, previous = 0)
```

**Semantics:**

- A tag appears once in the index per entry that has it, regardless of
  how many times the tag string would appear in the source. (Tags on
  an entry are a set, deduplicated by the compiler.)
- Compound queries (AND / OR / NOT) are not defined by the format. A
  consumer reads the relevant posting lists and performs set operations
  in application code.
- The `freq:source=value` tag pattern (§3.5) is treated as an opaque
  string by this index: `freq:coca=3000` and `freq:coca=3001` are two
  distinct posting lists. Range / sort queries on frequency values are
  consumer-side concerns.

**Lookup example** — "all IELTS entries":

```
1. Resolve tag string "exam:IELTS" → tag_id (u16) via tag_pool.
2. Binary-search tag.idx directory for tag_id.
3. Read posting_count, pread posting list at posting_offset.
4. Decode delta-encoded entry ids.
5. For each id, resolve via headword index → entry.
```

**Forward compatibility:** same rule as `morph.idx` — unknown flag
bits cause the reader to ignore the file and fall back to scanning.

---

## 5. `index/strings.tbl`

Dictionary-level **string pools** referenced by u16 index from the AST,
for low-cardinality repeated strings. Definition text is **not** interned
(zstd handles local repetition).

There are **six independent pools**, each addressed by u16 index from
a specific AST field. Index 0 is reserved in every pool = "default" /
"unspecified". A `strref` in the AST is always scoped to one pool by
its field name:

| AST field | Pool | Manifest copy |
|---|---|---|
| `Sense.pos` | pos pool | `pos_table` |
| `Translation.lang` | lang pool | `lang_table` |
| `Pron.kind` | pron-kind pool | `pron_kind_table` |
| `Form.kind` | form-kind pool | `form_kind_table` |
| `Morpheme.kind` (§6.4.1) | form-kind pool (shared) | — |
| `Entry.tags` (§6.4) | tag pool | `tag_table` |
| `Relation.type` (§6.11) | relation-type pool | `relation_type_table` |

> **Note:** `Morpheme.kind` shares the form-kind pool with `Form.kind`.
> Both are morphological "kind" labels; sharing one pool keeps the total
> pool count at six. Consumers distinguish them by context (which AST
> field they appear in), not by pool membership.

Each pool has the implicit index 0 = "unspecified"; indices 1..N come
from the pool's own segment in `strings.tbl`.

**File layout** — six segments, one per pool, concatenated in the
fixed order **pos, lang, pron_kind, form_kind, tag, relation_type**:

```
header: magic "RDST" (4B) + version u16 (1)
        pos_count: u32       lang_count: u32
        pron_count: u32      form_count: u32
        tag_count: u32       rel_count: u32
segment × 6:
  each segment := count × [ len:varint bytes:UTF-8 ]
```

Index 0 is implicit ("unspecified") and **not** stored. Each `*_count`
includes that implicit value, so a segment stores exactly `count - 1`
strings at indices 1..`count - 1`. Every count MUST be at least 1 and
at most 65536.

The `*_table` arrays in the manifest are human-readable copies of these
pools; the binary `strings.tbl` is authoritative for decoding. When an
array is present, it MUST contain exactly the nonzero values from its
corresponding binary pool, in the same order. Readers MUST use the
binary file for decoding.

---

## 6. Data Blocks & AST Encoding

### 6.1 Block file

`data/NNNNN.zst` (5-digit zero-padded). The file is a single **zstd
frame** (magic `28 B5 2F FD`) with no custom dictionary. A v0.1 reader
MUST reject a pack requiring a zstd dictionary; dictionary framing is
reserved for a future format version.

Decompressed, a block is a **contiguous byte stream of entries** packed
back-to-back. Entries do **not** carry their own length prefix at the
block level — the **index** records each entry's `(offset, size)`.
(Within the block, an entry's size from the index lets a reader slice
exactly; an entry's own AST is self-delimiting anyway, so a reader that
has the slice can also decode without the size.)

`block_max_uncompressed` (default 256 KiB) is a target; the final block
may be smaller. Block boundaries are chosen by the compiler at entry
boundaries; an entry MUST NOT span two blocks.

### 6.2 Primitive types

| Type | Encoding |
|---|---|
| `u8` | 1 byte, as-is. |
| `u16` / `u32` / `u64` | little-endian. Used only in fixed-size headers. |
| `varint` | LEB128 unsigned. Used for all variable integers in the AST (counts, offsets, sizes, ids). |
| `zigvarint` | zigzag + LEB128. Used for signed values (signed offsets, future numeric fields). |
| `bool` | 1 bit inside a flags byte. |
| `str` | `len:varint` + UTF-8 bytes. |
| `strref` | `u16` index into one of the six pools in `strings.tbl` (§5). 0 = unspecified. Which pool is determined by the field name (`pos`→pos pool, `lang`→lang pool, etc.). |
| `bytes` | `len:varint` + raw bytes. |

Readers MUST reject malformed, non-canonical, or overflowing varints;
invalid UTF-8; and a count, length, offset, or size that extends beyond
the enclosing file, entry slice, or decompressed block.

### 6.3 Flags-byte convention

Most AST records begin with a `u8 flags`. Each bit gates an optional
field; the field order is **fixed by bit number** (bit 0 first).
Unknown flag bits in any nested AST record MUST cause the reader to
stop decoding the containing **Entry** and treat that entry's indexed
`size` bytes as opaque; it MUST then continue with other entries. This
is the core forward-compatibility rule. Nested records have no length
prefix and therefore cannot be skipped independently.

Reserved (MUST be 0 in v0.1) flag bits are documented per record.

### 6.4 Entry AST

```
Entry :=
  flags: u8
    bit 0  has_see
    bit 1  has_tag        // entry-level tags (exam/level/freq/domain/...), see §3.5
    bit 2  has_media      (entry-level media, e.g. a portrait)
    bit 3  has_pron       (entry-level pronunciation)
    bit 4  has_ety        (if 0, ety_count is omitted; redirects may still use has_see)
    bit 5  has_morphology (morphological decomposition, see §6.4.1)
    bit 6  has_relations  (related headwords: synonyms, antonyms, phrases, etc.)
    bit 7  reserved
  [ if has_see:        see: str ]            // cross-reference headword, resolved via headword index (§4)
  [ if has_tag:        tag_count: varint; tag: strref × tag_count ]   // strref into tag_pool (§5)
  [ if has_media:      media_count: varint; media: MediaRef × media_count ]
  [ if has_pron:       pron_count: varint; pron: Pron × pron_count ]
  [ if has_ety:        ety_count: varint; ety: Ety × ety_count ]
  [ if has_morphology: morph_count: varint; morph: Morpheme × morph_count ]
  [ if has_relations:  rel_count: varint; rel: Relation × rel_count ]  // see §6.11
```

The **headword is not stored here** — it lives in the index. The AST
below is the entry **body**. A headword is a non-empty, valid UTF-8 byte
string and MUST occur exactly once in `headword.idx`; comparison and
identity use those exact UTF-8 bytes. Unicode normalization and
case-folding are consumer-side search policies, not part of exact lookup.
A headword MAY contain spaces (multiword expressions, e.g. `"take care
of"`); such entries are first-class and are looked up identically to
single-word entries.

`Entry.see` (bit 0) is a lightweight **redirect**: the entry has no real
content and the consumer SHOULD navigate to the target headword. It is
distinct from `Relation(type = "see")` (§6.11), which is an ordinary
see-also link that does not replace the current entry.

> **Design note:** entry-level tags are stored as `strref` into the
> `tag_pool` (§5), because they are low-cardinality repeated strings
> (e.g. `"IELTS"` appears on tens of thousands of entries) and benefit
> from interning + an inverted index (§4.7). In contrast, Sense and
> Form tags (§6.6, §6.8) are open `str` arrays — those are
> high-cardinality, low-frequency, and not indexed in v0.1.

#### 6.4.1 Morphology (morphological decomposition)

`Entry.morphology` is a flat, ordered array of morphemes that make up
the headword. It describes how a word is constructed from its
components (prefixes, roots, suffixes, combining forms).

```
Morpheme :=
  flags: u8
    bit 0  has_kind
    bit 1–7 reserved
  [ if has_kind: kind: strref ]   // e.g. "prefix", "root", "suffix", "combining_form"
  term: str                       // the morpheme text, e.g. "un-", "happy", "-ness"
```

- The array is **ordered** to reflect the left-to-right composition of
  the headword (e.g. `un-` + `happy` + `-ness` for `unhappiness`).
- The array is **flat** — it does not express recursive/nested
  decomposition. Languages that require deep morphological analysis
  (e.g. agglutinative languages) should use a dedicated morphological
  analyzer, not the dictionary pack.
- `kind` is a `strref` (interned). Recommended values: `prefix`,
  `root`, `suffix`, `combining_form`, `infix`. The spec does not
  enforce an enum; readers MUST treat unknown kinds as opaque text.
- A morpheme with `kind = 0` (unspecified) is valid; consumers render
  `term` without a label.

### 6.5 Ety (etymology group)

An `Ety` groups information that belongs to one etymological or lexical
source grouping. It may contain an identifier, an etymological root,
and the senses associated with that source. Pronunciation belongs to the
containing `Entry`, not to `Ety`; a compiler MUST NOT use `Ety` as a
synthetic container only to carry pronunciation. An entry with no
etymologies is still valid when it is a redirect or alias (`has_ety = 0`).

```
Ety :=
  flags: u8
    bit 0  has_id
    bit 1  has_root
    bit 2  has_sense
    bit 3–7 reserved
  [ if has_id:    id: str ]
  [ if has_root:  root: str ]
  [ if has_sense: sense_count: varint; sense: Sense × sense_count ]
```

`Ety.root` is the etymological root — a historical source word (e.g.
`bancus` for `bank`). It is a plain string, not a structured language
tag. Multiple etymologies on the same entry indicate homographs with
different origins.

### 6.6 Sense

A sense groups definitions of one POS.

```
Sense :=
  flags: u8
    bit 0  has_lemma
    bit 1  has_translation
    bit 2  has_form
    bit 3  has_tag
    bit 4  has_pron         // sense-level pronunciation (rare)
    bit 5–7 reserved
  pos: strref               // 0 = unspecified
  [ if has_lemma:       lemma: str ]
  [ if has_translation: tr_count: varint; tr: Translation × tr_count ]
  [ if has_form:        form_count: varint; form: Form × form_count ]
  [ if has_tag:         tag_count: varint; tag: str × tag_count ]  // tag is str, not strref (open set)
  [ if has_pron:        pron_count: varint; pron × pron_count ]
  def_count: varint
  def: Def × def_count
```

> **Design note:** senses are a `Vec`, not a set keyed by POS. Multiple
> senses with the same POS are legal and preserve insertion order —
> collapsing them would silently lose data.

### 6.7 Def (definition)

A `Def` is either a `Group` or a `Definition`. Tagged by a leading
`u8 kind`:

```
Def :=
  kind: u8
    0 = Definition
    1 = Group
  payload: Definition | Group
```

An unknown `kind` causes the reader to discard the containing Entry
under §6.3.

```
Definition :=
  flags: u8
    bit 0  has_id
    bit 1  has_example
    bit 2  has_note
    bit 3  has_media       // inline figure/audio for this definition
    bit 4–7 reserved
  value: str               // the definition text (UTF-8). MAY contain inline markup — see §6.10.
  [ if has_id:       id: str ]
  [ if has_example:  ex_count: varint; ex: Example × ex_count ]
  [ if has_note:     note_count: varint; note: Note × note_count ]
  [ if has_media:    media_count: varint; media: MediaRef × media_count ]
```

```
Group :=
  flags: u8
    bit 0  has_id
    bit 1  has_description
    bit 2–7 reserved
  [ if has_id:          id: str ]
  [ if has_description: description: str ]
  def_count: varint
  def: Def × def_count
```

### 6.8 Example, Note, Translation, Pron, Form

```
Example :=
  flags: u8
    bit 0  has_translation
    bit 1  has_pron
    bit 2  has_media
    bit 3  has_target
    bit 4–7 reserved
  value: str
  [ if has_translation: tr_count: varint; tr × tr_count ]
  [ if has_pron:        pron_count: varint; pron × pron_count ]
  [ if has_media:       media_count: varint; media: MediaRef × media_count ]
  [ if has_target:      target_count: varint; target: TargetOccurrence × target_count ]
```

```
TargetOccurrence :=
  span_count: varint
  span: TextSpan × span_count

TextSpan :=
  offset: varint
  length: varint
```

A target occurrence identifies the word or multiword expression that
the example illustrates. A continuous word or phrase uses one span. A
discontinuous expression uses multiple spans in one occurrence; repeated
occurrences use separate `TargetOccurrence` records. The surface text
may be an inflected or derived form of the entry headword.

Offsets and lengths count bytes from the start of the raw UTF-8 `value`
payload, excluding the `str` length prefix and before processing inline
markup (§6.10). Each count and length MUST be greater than zero, and
`offset + length` MUST NOT exceed the `value` payload length. Writers
MUST clear `has_target` when there are no target occurrences. Span
boundaries MUST fall on UTF-8 code-point boundaries, and spans MUST
cover literal text only, not escape or markup control bytes. Spans
within an occurrence are sorted by offset and do not overlap;
occurrences are sorted by their first span, and no spans in the same
example may overlap. A writer SHOULD include complete grapheme clusters
in each span.

For example, `take care of yourself` uses one span for `take care of`;
`look the word up` uses one occurrence with spans for `look` and `up`;
`He ran and ran` uses two occurrences. The format assigns no visual
style to targets. Consumers SHOULD distinguish them when rendering.

```
Note :=
  flags: u8
    bit 0  has_id
    bit 1  has_example
    bit 2–7 reserved
  value: str
  [ if has_id:      id: str ]
  [ if has_example: ex_count: varint; ex: Example × ex_count ]
```

```
Translation :=
  flags: u8
    bit 0  has_pron
    bit 1–7 reserved
  lang: strref            // 0 = unspecified
  value: str
  [ if has_pron: pron_count: varint; pron × pron_count ]
```

**`Translation.lang` omission rules (source format):**

In the YAML/JSON source format, `Translation.lang` may be omitted under
the following rules:

- If `pack.target_langs` contains exactly **one** language tag, every
  `Translation` MAY omit `lang`. The compiler fills in `lang` with the
  sole `target_langs` value.
- If `pack.target_langs` contains **multiple** language tags, every
  `Translation` MUST specify `lang` explicitly. The compiler MUST reject
  a translation with an unspecified `lang` as a validation error.
- If `pack.target_langs` is **empty**, no `Translation` records are
  allowed (entries may still have `Definition`-only senses).

In the distribution format, `lang: strref` is always present (0 means
unspecified); these rules apply only to the source format.

```
Pron :=
  flags: u8
    bit 0  has_lang
    bit 1  has_accent
    bit 2  has_media
    bit 3–7 reserved
  [ if has_lang:   lang: strref ]   // BCP-47; 0 = unspecified
  [ if has_accent: accent: str ]    // dialect/accent label
  kind: strref                      // notation system; 0 = unspecified
  value: str                        // non-empty written pronunciation
  [ if has_media: media_count: varint; media: MediaRef × media_count ]
```

`Pron.value` MUST be non-empty. A pronunciation MAY have media, but the
media are supplemental recordings for the written pronunciation; v0.1
does not support audio-only pronunciations. `lang` identifies the
language or language-region of the pronunciation, such as `en` or
`en-US`. `accent` is an optional free-form label for a finer distinction,
such as `General American` or `Received Pronunciation`. A missing
`accent` means that no finer distinction is asserted.

When `Pron` is nested under a `Translation`, an absent `Pron.lang`
inherits `Translation.lang`. Otherwise, an absent `Pron.lang` inherits
the nearest enclosing language context, ultimately falling back to
`pack.source_lang`. Consumers MUST NOT infer an accent from `lang` alone.

```
Form :=
  flags: u8
    bit 0  has_tag
    bit 1  has_feats         // morphological features (key=value|key=value...), see §3.4
    bit 2  has_pron
    bit 3–7 reserved
  kind: strref
  term: str               // the inflected/variant form
  [ if has_tag:   tag_count: varint; tag: str × tag_count ]
  [ if has_feats: feats: str ]
  [ if has_pron:  pron_count: varint; pron: Pron × pron_count ]
```

`Form.pron` records pronunciation for the specific written form in
`Form.term`, such as `ran` → `/ræn/`; it is independent of the
entry-level `Entry.pron` and any `Sense.pron`.

> **`feats`** stores morphological features as a `key=value` pair list
> separated by `|`, e.g. `"ud:ConjugationType=Godan|ud:ConjugationForm=Ta"`.
> Used for language-specific morphology (Japanese verb classes, Slavic
> aspect, Finno-Ugric cases, etc.). Encoding rules and recommended
> prefixes are defined in §3.4. Querying features requires the optional
> `index/morph.idx` (§4.6); without it, consumers fall back to scanning.

### 6.9 MediaRef

A reference from AST to a media file. **Never inline media bytes in
the AST.**

```
MediaRef :=
  flags: u8
    bit 0  has_description
    bit 1  has_alt
    bit 2–7 reserved
  kind: u8                // 0 = audio, 1 = image, 2 = video; other values are invalid in v0.1
  hash: [u8; 20]          // raw SHA-1. Reader maps this to a media-manifest
                          // entry by (kind, hash), then to its ZIP member.
  [ if has_description: desc: str ]
  [ if has_alt:        alt: str ]
```

`hash` is the raw 20-byte SHA-1. `(kind, hash)` MUST identify exactly
one entry in `media/manifest.json`; its MIME type, extension,
and compression are authoritative there. MIME is intentionally a
file-level property and is not repeated in `MediaRef`; `description`
and `alt` remain reference-level properties. A writer MUST reject a
hash collision with different bytes.

### 6.10 Inline markup in `value` strings

Definition/example/note values MAY contain a minimal, well-defined
inline markup so structure survives (bold, italic, link, reference,
ruby). The markup is **not** HTML. v0.1 defines a tiny tag set encoded
as ASCII control bytes:

| Sequence | Meaning |
|---|---|
| `\x01` ... `\x02` | bold span: bytes between are bold |
| `\x03` ... `\x04` | italic span |
| `\x05` ref `\x06` | link/reference to another headword |
| `\x07` base `\x08` text `\x09` | ruby annotation (base + reading) |

All other bytes are literal UTF-8 text. `\x00` escapes the following
byte as literal, and raw `\x00` at end of a string is invalid. This is
how literal bytes `\x00` through `\x09` are represented. Markup spans
MUST be properly closed and MUST NOT nest; readers MUST reject invalid
or mismatched marker sequences. Readers that choose not to render
markup SHOULD strip the markers and render their text content.

> **Design rationale & references:** control-byte delimiters are chosen
> over visible markers (`<b>`, `**`) for two reasons: (1) they are not
> used by ordinary printable text and the escape rule preserves literal
> control bytes; (2) they are compact
> (2 bytes per span vs 7+ for HTML tags). The approach follows the
> long-established convention of **IRC formatting control characters**
> (RFC 1459 + mIRC extensions: `\x02` bold, `\x1D` italic, `\x1F`
> underline, `\x0F` reset), adapted with smaller byte values to keep
> varint-length strings short.

### 6.11 Relation

An inline typed link from this entry to another headword. Relations are
stored on the Entry that owns them; there is **no external relations
file**. When a consumer displays an entry, its `relations` array is the
full set of related headwords for that entry — no secondary lookup,
graph load, or scan is required.

```
Relation :=
  type: strref      // index into relation_type_pool (§5). 0 = unspecified.
  target: str       // headword string; MUST be non-empty and MUST exist in headword.idx (§4)
```

- `type` is a `strref` (u16) into the `relation_type_pool`. The pool is
  per-pack; its indices are **not stable across packs** and are **not
  fixed by the spec**. A reader resolves `type` through the current
  pack's `strings.tbl` only. Recommended codes (compiler MAY use any
  strings): `syn`, `ant`, `phrase`, `der`, `hyp`, `hypo`, `hol`, `mer`,
  `see`.
- `target` is a plain `str` carrying the related headword's UTF-8 bytes.
  It is **not** an integer index or a byte offset: the spec assigns
  headwords no stable numeric id, and storing the string keeps relations
  robust to recompilation (headword reordering, block repacking). zstd
  compresses repeated headword strings effectively across entries.

**Direction.** Every `Relation` record is a **single directed link**.
The format does not synthesize reverse links and the reader MUST NOT
infer them from `type`. If a relation is conceptually symmetric (e.g.
synonymy), the source compiler MUST materialize both directions by
emitting a `Relation` on each entry (see "Source compiler conventions"
below). `type` is opaque to the reader; the consumer interprets it for
display only.

**Validation rules.**

- `rel_count` MUST be greater than 0 when `has_relations` is set.
- `target` MUST be non-empty and MUST byte-exactly match a headword in
  `headword.idx`. A writer MUST reject a dangling `target`.
- Within one Entry, `(type, target)` pairs MUST be unique; duplicates
  are a writer error. The first occurrence wins for ordering purposes.
- `target` MUST NOT equal the entry's own headword (self-relation).
  A writer MUST reject it.
- Relations preserve source/compiler order within the array; the writer
  SHOULD emit any author-declared relations before compiler-synthesized
  ones (e.g. auto-filled synonyms) so the author's intent stays visible
  first.

**Fields removed from v0.1.** The earlier `relations.bin` edge record
carried `weight` (u8 strength), `meta` (key-value pairs), and an
`is_directional` flag. These are **not present** on the inline
`Relation`: direction is implicit in which Entry the record lives on,
and weight/meta were unused by real dictionary data. A future spec
version MAY extend `Relation` with a flags byte + optional fields
appended after `target`; v0.1 readers treat the record as exactly
`type + target` with no length prefix (it is decoded inline as part of
the Entry, governed by `rel_count`).

---

## 7. Media

### 7.1 Storage rules

- Media files are stored **uncompressed** (zip `store`).
- Already-compressed formats (mp3/aac/opus/mp4/webm/jpg/png/webp/...)
  are stored **as-is**.
- Uncompressed source formats (wav/pcm/bmp/raw text) SHOULD be
  compressed by the compiler (zstd) and flagged in the media manifest
  with `compression: "zstd"`. The reader decompresses on fetch.
- Every media file is named by its **content hash** (SHA-1 hex),
  sharded by the first two hex chars: `media/audio/a3/a3f5c8....mp3`.
- Identical content of the same `kind` → identical path → automatic
  dedup. The compiler MUST NOT write two files with the same `(kind,
  hash)` pair.
- When a pack contains media, `media/manifest.json` is required. Its
  entries are unique by `(kind, hash)` and must name an existing ZIP
  member. The SHA-1 is computed over the logical media bytes: for
  `compression: "none"` those are the stored member bytes; for zstd
  they are the bytes after decompression.

### 7.2 `media/manifest.json` (inside the pack)

A single JSON manifest describing every media file.

```json
{
  "hash_algo": "sha1",
  "entries": [
    {
      "hash": "a3f5c8e1...",
      "kind": "audio",
      "ext": "mp3",
      "mime": "audio/mpeg",
      "compression": "none",
      "size": 12345,
      "uncompressed_size": 12345
    }
  ]
}
```

A reader, given a `MediaRef`, hex-encodes its hash, finds the manifest
entry with the same `(kind, hash)`, reconstructs the ZIP member name as
`media/<kind>/<hh>/<hashhex>.<ext>`, then looks it up in the ZIP central
directory. `<hh>` is the first two lowercase hex characters of the
hash. `size` is the stored payload size; `uncompressed_size` is its size
after the optional media compression. There is no `path` field because
the mandatory v0.1 layout makes it redundant.

### 7.3 Streaming

Stored, uncompressed media are real files at real ZIP offsets. A
ZIP-aware server can:

- Translate an HTTP byte range into the member's stored payload range
  after parsing the ZIP local-file header.
- Pipe stored payload bytes to a media decoder without buffering the
  whole file.

A normal static web server cannot expose a member inside a ZIP archive
as `media/...` without such ZIP-aware handling. Media with
`compression: "zstd"` MUST NOT advertise byte-range support over the
uncompressed media representation; readers decompress it before use.

### 7.4 Media references in the source format

The distribution `MediaRef` (§6.9) uses a raw 20-byte SHA-1 `hash` —
impractical for human authoring. In the YAML/JSON source format,
media are referenced by **file path** instead. The compiler resolves
paths to hashes and emits `MediaRef` records in the pack.

#### 7.4.1 Source `MediaRef`

A source media reference replaces `hash` with `path`:

```yaml
kind: audio            # required: audio | image | video
path: ./audio/run.mp3  # required: relative or absolute file path
description: ...       # optional, same as §6.9
alt: ...               # optional, same as §6.9
```

- `path` is resolved relative to the source file's directory (or the
  compiler's working directory for absolute paths).
- The compiler reads the file, computes SHA-1 over the logical bytes
  (pre-decompression for zstd-compressed sources), and produces the
  distribution `MediaRef` with `hash` and without `path`.
- If the file cannot be read, the compiler MUST error.
- `kind` MUST match the `MediaRef.kind` in the pack.

#### 7.4.2 Source `MediaAsset`

Pack-level media assets (§3 `media` array) use the same path form:

```yaml
media:
  - kind: audio
    path: ./audio/run.mp3
    # ext, mime, compression are auto-detected from the file by the
    # compiler. Authors MAY override:
    # ext: mp3
    # mime: audio/mpeg
    # compression: none
```

- When `ext`/`mime`/`compression` are omitted, the compiler detects
  them from the file content (magic bytes) or extension.
- `compression: "zstd"` may be set for uncompressed source formats
  (wav/pcm/bmp) to let the compiler compress before storing.

#### 7.4.3 Where media refs appear

Every AST node that has `media: Vec<MediaRef>` in the distribution
format uses the same path form in source:

| Location | YAML field |
|---|---|
| Entry | `media:` |
| Definition | `media:` |
| Example | `media:` |
| Pron | `media:` |

Example:

```yaml
headword: run
pron:
  - kind: ipa
    value: /rʌn/
    media:
      - kind: audio
        path: ./audio/run_us.mp3
        description: General American
media:
  - kind: image
    path: ./images/run.png
    alt: A person running
etymologies:
  - senses:
      - pos: VERB
        definitions:
          - value: To move swiftly on foot
            media:
              - kind: video
                path: ./video/run_clip.mp4
                description: Video demonstration
```

After compilation, all `path` fields are replaced by `hash`, and the
files are stored at `media/<kind>/<hh>/<hashhex>.<ext>`.

---

## 8. Source compiler conventions (relations)

This section is **non-normative for readers**: it describes how a source
compiler (YAML/JSON → `.rdict`) is expected to produce the inline
`Relation` records defined in §6.11. A conforming reader ignores this
section entirely and decodes `Relation` records as pure `type + target`
links on whatever Entry they appear on.

The binary format stores only directed `Relation` records. Symmetric
semantics and phrase-of distribution are source-format concerns and are
materialized by the compiler before writing the `.rdict` pack.

### 8.1 Symmetric relations (`syn`, `ant`)

The source author writes one direction:

```yaml
- headword: big
  relations:
    - type: syn
      target: large
```

The compiler MUST emit the reverse record on the target entry as well,
so that looking up `large` shows `big` as a synonym. Emitted reverse
records use the same `type`. The compiler MUST be idempotent: if the
source already declares both directions, the compiler MUST NOT duplicate
either record (deduplication is by `(type, target)` within an Entry).

If the target headword does not exist in the source, the compiler MUST
report an error and MUST NOT emit a dangling `Relation`.

### 8.2 Directional relations (`der`, `hyp`, `hypo`, `hol`, `mer`, `see`)

The compiler emits exactly the records the source declares. It MUST NOT
synthesize a reverse record. If the author wants both directions
visible, they write both explicitly.

### 8.3 Multiword phrases (`phrase`)

A multiword headword (e.g. `"take care of"`) is a first-class entry —
it is stored in `headword.idx` like any other headword and looks up
identically. To make the phrase discoverable from its component words,
the source declares which component entries should carry a back-reference:

```yaml
- headword: take care of
  definitions:
    - value: To look after someone.
  phrase_of: [take, care]    # explicit; "of" excluded by the author
```

`phrase_of` is **source-format only** and is never written to the
binary AST. For each listed component word, the compiler inserts a
`Relation { type: phrase, target: "take care of" }` into that
component's Entry. If a listed component has no entry in the source, the
compiler MUST report an error (the reference cannot be emitted without a
host Entry). The compiler MUST NOT silently drop a `phrase_of` entry.

A component word MAY appear in multiple phrases; each contributes one
`Relation` record to that component's Entry. The compiler MUST dedupe
by `(type, target)` so the same phrase is not listed twice on one Entry.

### 8.4 Relation ordering

Within an Entry, the compiler SHOULD emit author-declared relations
first (in source order), followed by compiler-synthesized relations
(auto-filled symmetric partners, `phrase_of` back-references). This
keeps the author's intent visible first in consumer UIs. After
deduplication, the first occurrence of each `(type, target)` wins its
position.

### 8.5 Validation checklist

Before writing the pack, the compiler MUST verify:

1. Every `Relation.target` byte-exactly matches a headword that will be
   present in `headword.idx`.
2. No Entry contains a self-relation (`target == own headword`).
3. No Entry contains duplicate `(type, target)` pairs.
4. Symmetric types declared on one side have a matching reverse record
   on the target (or the target is missing and an error was reported).

---

## 9. Versioning & Compatibility

### 9.1 Versions in play

- **Spec version** (`format_version`): semver of **this spec**. Bumped
  when the on-disk layout changes incompatibly.
- **Pack content version** (`pack.version`): the dictionary author's
  semver. Independent of spec version.

### 9.2 Compatibility rules

A reader MUST publish the SemVer range or ranges of `format_version`
values it supports as part of its API or documentation. These ranges
belong to the reader; a pack cannot declare or extend reader support.
A reader MUST decode every pack inside an advertised range and MUST
refuse a pack outside all advertised ranges with a clear error by
default. It MAY offer an explicit best-effort mode outside those ranges;
unknown manifest keys and AST flags then follow §3.2 and §6.3.

Before format 1.0.0, each minor version is a separate compatibility
line. A reader targeting 0.1.p MUST at least advertise and support
`>=0.1.0, <=0.1.p`; support for another `0.m` line requires an
additional advertised range. Starting with 1.0.0, SemVer major-version
compatibility applies: a reader targeting `M.m.p` MUST at least support
`>=M.0.0, <=M.m.p`.

### 9.3 Evolution hygiene

- Never widen an existing fixed-size header field. Add new fields at
  the end, or in reserved space, with a version gate.
- Never repurpose a reserved bit.
- New optional record fields: new flag bit → `has_*` → field after all
  existing fields.
- New record kinds (e.g. a new `Def` kind): append to the `kind` u8
  space; old readers discard the containing entry under §6.3.

---

## 10. Conformance

A **conforming reader** MUST:

1. Parse the ZIP central directory and `mimetype`.
2. Parse `manifest.json` and the binary `headword.idx` header +
   directory.
3. Perform exact headword lookup → `(block, offset, size)`.
4. Decompress a zstd data block and slice/decode the entry AST, honoring
   §6.3 opaque-record fallback for unknown flag bits.
5. Resolve a `MediaRef` through `media/manifest.json` to media bytes;
   implement HTTP range only when serving stored media through a
   ZIP-aware server.
6. Implement case-folded sort order (§4.2.1) for headword lookup and
   prefix search.
7. If `index/morph.idx` is present and the reader exposes feats queries,
   parse its header and posting lists per §4.6. (A reader MAY ignore
   morph queries entirely.)
8. If `index/tag.idx` is present and the reader exposes tag queries,
   parse its header and posting lists per §4.7. (A reader MAY ignore
   tag queries entirely.)

A **conforming writer** MUST:

1. Emit zip `store` for all entries, `mimetype` first.
2. Populate all required manifest fields.
3. Sort the headword index by case-folded codepoint order (§4.2.1).
4. Content-address all media by SHA-1; dedup identical content.
5. Not split an entry across data blocks.
6. Set unknown/reserved bits to 0.
7. If any Entry sets `has_relations`, emit the `relation_type_pool`
   segment in `strings.tbl` and the `relation_type_table` array in the
   manifest; every `Relation.target` MUST byte-exactly match a headword
   in `headword.idx`. No external relations file is emitted.
8. If the source has `feats` data, SHOULD emit `index/morph.idx` and
   set `morph.has_index`; MAY omit for small dictionaries. If emitted,
   the `key=value` strings in the index MUST match the feats strings
   in the AST byte-for-byte.
9. If the source has entry-level tags, SHOULD emit `index/tag.idx` and
   set `tags.has_index`; MAY omit for small dictionaries. If emitted,
   the `tag_id` values in the index MUST match the `strref` values in
   the Entry AST, and the tag strings MUST match the `tag_pool` (§5).

---

## 11. Open questions (v0.1 draft)

1. **UCA collation**: the default sort is case-folded codepoint order
   (§4.2.1). Locale-specific collation (e.g. Chinese pinyin order,
   German phonebook order) via UCA is a future extension; format TBD
   if demanded.
2. **zstd dictionary**: reserved field; training/distribution TBD.
3. **Encryption**: out of scope for v0.1.
4. **Incremental update packs**: a delta pack format (subset zip with
   a manifest patch) is natural but not specified yet — likely v0.2.
5. **Relation weight/meta**: v0.1 `Relation` (§6.11) carries only
   `type + target`. The earlier `weight` (u8 strength), `meta`
   (key-value pairs), and `is_directional` flag from the removed
   `relations.bin` design are deferred. A future spec version MAY add a
   flags byte + optional fields appended after `target`, gated by new
   flag bits.

---

## Appendix A — Worked example (single entry, illustrative)

Source (YAML):

```yaml
headword: run
tags:
  - exam:IELTS
  - freq:coca=1000
etymologies:
  - description: Latin root
    senses:
      - pos: VERB
        definitions:
          - value: "To move swiftly"
            examples:
              - value: "The dog runs."
```

Pack layout (abridged):

```
run.rdict
├── mimetype                         → "application/rdict"
├── manifest.json                    → pos_table=["NOUN","VERB",...], tag_table=["exam:IELTS","freq:coca=1000",...], ...
├── index/headword.idx               → entry for "run" → (block_id=0, offset=0, size=N)
├── index/strings.tbl                → pos: "VERB"@1; tag: "exam:IELTS"@1, "freq:coca=1000"@2; ...
├── index/tag.idx                    → tag_id=1 → [entry_id for "run", ...], tag_id=2 → [...]
├── data/00000.zst                   → zstd( Entry{ flags=has_tag|has_ety, tags=[1,2], Ety{ root, Sense{ pos=1, Def{ value } } } } )
└── media/manifest.json              → [] (no media in this example)
```

Decoding "run":

1. Read zip EOCD → central directory.
2. Read `manifest.json` → learn `entry_count`, `block_count`,
   `strings.tbl`, `pos_table`, `tag_table`.
3. Read `index/headword.idx` header + directory (resident).
4. Binary search → "run" is in block 0 at body offset X.
5. Scan block 0 → entry → `(data_block_id=0, offset=0, size=N)`.
6. Decompress `data/00000.zst` (cached LRU) → buffer.
7. Slice `[0, N)` → decode AST: `flags`→`has_tag`+`has_ety`,
   `tags=[strref 1="exam:IELTS", strref 2="freq:coca=1000"]`,
   `Ety`{root="rinnan"}, `Sense`{pos=strref 1="VERB"},
   `Def`{value="To move swiftly"}, `Example`{value="The dog runs."}.

---

*End of Rdict Specification v0.1 (draft).*
