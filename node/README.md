# Rdict Node.js Binding

Native Node.js binding for the Rdict dictionary format. Built with napi-rs for near-zero overhead Rust interop.

## Install

```bash
npm install @runer-hq/rdict
```

## Quick Start

```javascript
const { Dictionary } = require('@runer-hq/rdict');

const dict = new Dictionary('/path/to/dict.rdict');

const entry = dict.lookup('run');
if (entry) {
  console.log(entry.headword);           // "run"
  console.log(entry.etymologies[0].root); // "rinnan"
  console.log(entry.morphology);          // [{kind: "prefix", term: "un-"}, ...]
}

const headwords = dict.listHeadwords();   // ["again", "always", ...]
const manifest = dict.manifest();
console.log(manifest.name);              // "English-Chinese (NGSL)"
```

## Documentation

Full API reference, data structures, and usage examples: [`USAGE.md`](USAGE.md).

## Build

```bash
npx napi build --release    # Generates rdict.<platform>-<arch>.node
node test.js                # Run tests
node test_media.js          # Run media API tests
```

## Requirements

- Rust ≥ 1.85 (edition 2024)
- Node.js ≥ 18 (LTS recommended)
- `@napi-rs/cli` (dev dependency)

## TypeScript Types

Full TypeScript type definitions in `index.d.ts`.

## Architecture

- `src/lib.rs` — napi-rs Rust binding (Dictionary class)
- `index.js` — JS wrapper (JSON → JS objects)
- `index.d.ts` — TypeScript type declarations
