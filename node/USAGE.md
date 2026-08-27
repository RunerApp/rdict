# rdict — Node.js Binding Usage

## Install

### Local reference (Electron / Node project)

In your project's `package.json`:

```json
{
  "dependencies": {
    "@runer-hq/rdict": "file:../path/to/rdict/node"
  }
}
```

```bash
cd your-project
npm install
```

### From npm (after publish)

```bash
npm install @runer-hq/rdict
```

### Electron ABI rebuild

If Electron reports `Module version mismatch`:

```bash
npx electron-rebuild
```

napi-rs uses N-API, which has a stable ABI — usually no rebuild needed.

## API

### `Dictionary`

```js
const { Dictionary } = require("@runer-hq/rdict");
```

#### `new Dictionary(path)`

Open a `.rdict` file.

```js
const dict = new Dictionary("/path/to/dict.rdict");
```

- **path** `string` — path to `.rdict` file
- **throws** `Error` — file not found or format error

---

#### `dict.lookup(headword)` → `Entry | null`

Look up a headword. Returns the full entry or `null`.

```js
const entry = dict.lookup("hello");
if (entry) {
  console.log(entry.headword);                      // "hello"
  console.log(entry.etymologies[0].senses[0].pos);  // "NOUN"
}
```

- **headword** `string` — exact match (case-sensitive)
- **returns** `Entry | null` — `null` if not found
- **throws** `Error` — decode failure (opaque entry)

---

#### `dict.listHeadwords()` → `string[]`

List all headwords (case-folded sort order).

```js
const headwords = dict.listHeadwords();
console.log(`${headwords.length} headwords`);
```

Warning: avoid for large dictionaries (100K+ entries).

---

#### `dict.prefix(prefix, limit?)` → `string[]`

Case-insensitive prefix search. Returns up to `limit` matches (default 20).

```js
const results = dict.prefix("app", 10);
// ["Apple", "apple", "apply", ...]
```

- **prefix** `string` — search prefix (case-insensitive)
- **limit** `number` — max results, default 20, `<= 0` returns empty array
- **returns** `string[]` — headwords in original case, in sorted order
- Backed by binary search on the sorted index, not a full scan

---

#### `dict.manifest()` → `Manifest`

Get dictionary metadata.

```js
const meta = dict.manifest();
console.log(meta.name);         // "English-Chinese (NGSL)"
console.log(meta.source_lang);  // "en"
console.log(meta.target_langs); // ["zh-Hans"]
console.log(meta.entry_count);  // 2809
```

---

#### `dict.mediaManifest()` → `MediaManifestEntry[] | null`

Get all media manifest entries, or `null` if no media.

```js
const media = dict.mediaManifest();
if (media) {
  for (const m of media) {
    console.log(`${m.kind} ${m.hash}.${m.ext} (${m.size} bytes)`);
  }
}
```

---

#### `dict.mediaInfo(key)` → `MediaInfo | null`

Get media metadata by key (no content read).

```js
const info = dict.mediaInfo({ kind: "audio", hash: "a1b2c3..." });
if (info) {
  console.log(`Size: ${info.uncompressed_size} bytes`);
  console.log(`MIME: ${info.mime}`);
}
```

- **key** `MediaKey` — `{ kind: 'audio'|'image'|'video', hash: string }`
- **returns** `MediaInfo | null` — metadata or `null` if not found

---

#### `dict.readMedia(key)` → `Buffer`

Read media bytes by key. Automatically decompresses zstd-compressed media.

```js
const buf = dict.readMedia({ kind: "audio", hash: "a1b2c3..." });
// buf is a Node.js Buffer, ready to play/display
```

- **key** `MediaKey` — `{ kind, hash }`
- **returns** `Buffer` — raw (uncompressed) media bytes
- **throws** `Error` — not found or read failure

Recommended for small media (< 4 MiB). For large media, use `extractMedia`.

---

#### `dict.extractMedia(key, outputPath)` → `number`

Extract media to disk via streaming. Creates parent directories, writes
to a temp file, then atomically renames. Avoids buffering full media
in memory.

```js
const bytes = dict.extractMedia(
  { kind: "video", hash: "d4e5f6..." },
  "/tmp/rdict-cache/d4e5f6....mp4"
);
console.log(`Extracted ${bytes} bytes`);
```

- **key** `MediaKey` — `{ kind, hash }`
- **outputPath** `string` — destination file path
- **returns** `number` — bytes written
- **throws** `Error` — extraction failure

Recommended for large media (> 4 MiB). Caller manages cache cleanup.

---

#### `dict.readCover()` → `Buffer | null`

Read the dictionary cover image bytes, if present. Returns `null` if no
cover. Format (PNG/JPEG) is determined by the `pack.cover` file extension
in the manifest.

```js
const cover = dict.readCover();
if (cover) {
  // cover is a Buffer containing PNG or JPEG bytes
  fs.writeFileSync("cover.png", cover);
}
```

- **returns** `Buffer | null` — raw image bytes, or `null` if no cover

---

## Data Structures

### Entry

```ts
{
  headword: string;           // headword
  see: string | null;         // redirect target
  tags: string[];             // e.g. ["exam:IELTS", "level:B2"]
  pron: Pron[];               // pronunciations
  etymologies: Ety[];         // etymologies + senses
  morphology: Morpheme[];     // morpheme decomposition
  relations: Relation[];      // synonyms, antonyms, etc.
  media: MediaRef[];          // media references
}
```

### MediaRef

```ts
{
  kind: string;              // 'audio' | 'image' | 'video'
  hash: string;              // SHA-1 hex string
  description: string | null;
  alt: string | null;
}
```

To read media, pass `{ kind, hash }` to `readMedia()` or `extractMedia()`.

### MediaKey

```ts
{
  kind: 'audio' | 'image' | 'video';
  hash: string;  // SHA-1 hex
}
```

### MediaInfo / MediaManifestEntry

```ts
{
  hash: string;
  kind: string;              // 'audio' | 'image' | 'video'
  ext: string;               // file extension, e.g. "mp3"
  mime: string;              // MIME type, e.g. "audio/mpeg"
  compression: string;       // "none" | "zstd"
  size: number;              // stored (compressed) size in bytes
  uncompressed_size: number; // uncompressed size in bytes
}
```

### Relation

```ts
{
  type_: string | null;  // syn / ant / phrase / der / ...
  target: string;        // target headword string
}
```

Call `dict.lookup(target)` to navigate.

### Ety

```ts
{
  id: string | null;
  root: string | null;   // etymological root, e.g. "bancus"
  senses: Sense[];       // sense list
}
```

### Sense

```ts
{
  pos: string | null;          // UD v2 UPOS: NOUN / VERB / ADJ / ADV / ...
  lemma: string | null;
  translations: Translation[];
  forms: Form[];
  tags: string[];
  pron: Pron[];
  definitions: Def[];
}
```

POS values use Universal Dependencies v2 UPOS short codes (uppercase):

| Code | Meaning |
|------|---------|
| `NOUN` | noun |
| `VERB` | verb |
| `ADJ` | adjective |
| `ADV` | adverb |
| `ADP` | adposition |
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

### Def (union type)

```ts
// Plain definition
{ Definition: {
    id: string | null;
    value: string;
    examples: Example[];
    notes: Note[];
    media: MediaRef[];
} }

// Group definition
{ Group: {
    id: string | null;
    description: string | null;
    definitions: Def[];
} }
```

### Translation

```ts
{
  lang: string | null;  // BCP 47, e.g. "zh-Hans"
  value: string;
  pron: Pron[];
}
```

### Example

```ts
{
  value: string;
  translations: Translation[];
  pron: Pron[];
  targets: TargetOccurrence[];  // for highlight
  media: MediaRef[];
}
```

### Pron

```ts
{
  lang: string | null;
  accent: string | null;   // e.g. "General American"
  kind: string | null;     // e.g. "ipa"
  value: string;           // e.g. "/rʌn/"
  media: MediaRef[];
}
```

### Morpheme

```ts
{
  kind: string | null;  // prefix / root / suffix / ...
  term: string;         // e.g. "un-"
}
```

### Form

```ts
{
  kind: string | null;  // infl / pl / sg / comp / sup / ...
  term: string;
  tags: string[];
  feats: string | null; // UD features, e.g. "ud:Tense=Past|ud:VerbForm=Fin"
  pron: Pron[];
}
```

### Manifest

```ts
{
  name: string;
  version: string;
  source_lang: string;
  target_langs: string[];
  entry_count: number;
  block_count: number;
  cover?: string;           // ZIP member path of cover image, e.g. "cover.png"
}
```

Note: the full manifest JSON also contains `pack.cover` (optional, the
ZIP member path of the cover image). Use `readCover()` to get the image
bytes directly.

## Full Example

```js
const { Dictionary } = require("@runer-hq/rdict");

const dict = new Dictionary("./eng-zho-ngsl.rdict");

// Lookup
const entry = dict.lookup("unhappiness");
if (!entry) {
  console.log("Not found");
  process.exit(0);
}

console.log(`Headword: ${entry.headword}`);
console.log(`Tags: ${entry.tags.join(", ") || "none"}`);

// Morphology
if (entry.morphology.length > 0) {
  console.log("Morphology:");
  for (const m of entry.morphology) {
    console.log(`  ${m.kind || "?"}: ${m.term}`);
  }
}

// Etymology
for (const ety of entry.etymologies) {
  if (ety.root) {
    console.log(`Root: ${ety.root}`);
  }
}

// Senses
for (const ety of entry.etymologies) {
  for (const sense of ety.senses) {
    console.log(`\n[${sense.pos || "?"}]`);
    for (const def of sense.definitions) {
      if (def.Definition) {
        console.log(`  ${def.Definition.value}`);
        for (const ex of def.Definition.examples) {
          console.log(`    ex: ${ex.value}`);
          for (const tr of ex.translations) {
            console.log(`    tr: ${tr.value}`);
          }
        }
      }
    }
    for (const tr of sense.translations) {
      console.log(`  tr: ${tr.value}`);
    }
  }
}

// Relations
if (entry.relations.length > 0) {
  console.log("\nRelations:");
  for (const rel of entry.relations) {
    console.log(`  ${rel.type_ || "?"}: ${rel.target}`);
  }
}

// Media
if (entry.media.length > 0) {
  console.log("\nMedia:");
  for (const m of entry.media) {
    const info = dict.mediaInfo({ kind: m.kind, hash: m.hash });
    if (info) {
      console.log(`  ${m.kind} ${m.hash} (${info.uncompressed_size} bytes)`);
    }
  }
}

// Manifest
const meta = dict.manifest();
console.log(`\nDictionary: ${meta.name}`);
console.log(`Entries: ${meta.entry_count}`);
```

## Local Development

### Rebuild native module

```bash
cd /path/to/rdict/node

# Method 1: napi CLI
npm run build

# Method 2: cargo + manual copy
cargo build --release
cp target/release/librdict_js.dylib rdict.darwin-arm64.node
```

### Test

```bash
node test.js       # general tests
node test_media.js # media API tests
```

### Platform file naming

| Platform | Filename |
|----------|----------|
| macOS ARM64 | `rdict.darwin-arm64.node` |
| macOS x64 | `rdict.darwin-x64.node` |
| Linux x64 (glibc) | `rdict.linux-x64-gnu.node` |
| Linux ARM64 (glibc) | `rdict.linux-arm64-gnu.node` |
| Windows x64 | `rdict.win32-x64-msvc.node` |
| Windows ARM64 | `rdict.win32-arm64-msvc.node` |

`index.js` auto-selects the correct `.node` file based on `process.platform` and `process.arch`.

## Performance

- Open dictionary: < 1ms (lazy mode)
- Random lookup: ~170K QPS (lazy) / ~3M QPS (eager)
- Memory: only index is loaded, not full data
