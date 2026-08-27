# Rdict Performance Report & Optimization Plan

## 1. Test Environment

- **Machine**: MacBook Pro M1 Pro, 16 GB RAM, macOS 14.5
- **Rust**: stable aarch64-apple-darwin, release mode (`--release`)
- **Data**: Synthetic entries, each with headword, IPA pronunciation, POS, Chinese translation, definition, example (with target span)
- **Scripts**: `core/examples/stress_test.rs`, `core/examples/entry_size_test.rs`

---

## 2. Zstd Compression Level Comparison

**Volume**: 100,000 entries, ~200 bytes each

| Level | File size (MB) | Write time | Open (ms) | Seq 1K (ms) | Rand 10K (ms) | QPS |
|-------|---------------|-----------|----------|------------|--------------|------|
| 1 | 1.77 | **163ms** | 0.57 | **26.1** | 1520 | 6,578 |
| 3 | 1.75 | 207ms | **0.55** | 27.4 | 1693 | 5,906 |
| 5 | 1.76 | 223ms | 0.62 | 26.9 | 1972 | 5,071 |
| 9 | 1.75 | 369ms | 0.52 | 27.9 | 1896 | 5,275 |
| 13 | 1.75 | 834ms | 0.63 | 27.4 | 1957 | 5,111 |
| 19 | **1.70** | 23.8s | 0.66 | 28.4 | **1190** | **8,403** |

### Conclusions

- **Write**: Level 1 is fastest, Level 19 slowest; high compression costs far exceed file-size gains.
- **Read**: Blocks are split by uncompressed size before compression, so level doesn't change block count or cache hit rate. Level 3 typically yields the best throughput; Level 19's advantage isn't reliably reproducible.
- **File size**: Only ~4% difference across levels.
- **Recommendation**: Default Level 3; use higher levels explicitly for release builds that need smaller files.

---

## 3. Scale Scalability

**Volume**: 100K vs 1M entries, Level 19

| Metric | 100K | 1M | Multiplier |
|------|--------|---------|------|
| Write time | 24.3s | 255s | 10.5x |
| File size | 1.70 MB | 17.45 MB | 10.3x |
| Open (in-memory) | 0.46ms | 7.27ms | 15.8x |
| Cold start (file) | 1.2ms | 11.5ms | 9.6x |
| Sequential lookup | 0.028ms | 0.029ms | 1.0x |
| Random lookup | 0.122ms | 0.208ms | 1.7x |
| Miss lookup | 0.028ms | 0.010ms | 0.4x |
| List headwords | 23ms | 193ms | 8.4x |
| QPS | 8,205 | 4,798 | — |

### Conclusions

- **Lookup latency** barely scales with size (100K→1M only 0.12ms→0.21ms)
- **Cold start** at 1M entries is still only 11.5ms
- **Compression ratio** stable at ~18 bytes/entry
- **Write** is linear; 1M entries at Level 19 takes 4.3 minutes

---

## 4. Entry Size Impact

**Volume**: 50,000 entries, Level 19

| Type | Definition length | File size (MB) | Write (s) | Open (ms) | Rand 5K (ms) | QPS |
|------|---------|-------------|---------|----------|-------------|------|
| tiny | 1 byte | — | — | — | — | — |
| small | 50 bytes | — | — | — | — | — |
| medium | 200 bytes | — | — | — | — | — |
| large | 1000 bytes | — | — | — | — | — |
| xlarge | 5000 bytes | — | — | — | — | — |

> **Note**: This test was not completed due to OOM. Needs to be re-run with smaller volume (5,000 entries).

---

## 5. Real-World Performance (50K / 100K entries, Level 3)

Release build with `Arc<[u8]>` block cache, 16 MiB LRU, `Arc<Entry>` eager cache, and single-scan preload enabled. Query loop is 10,000 random lookups. Results are machine baselines, not fixed guarantees.

| Metric | rdict Lazy (50K) | rdict Eager (50K) | rdict Lazy (100K) | rdict Eager (100K) |
|------|------------------:|-------------------:|-------------------:|--------------------:|
| File size | **0.67 MB** | — | 1.60 MB | **1.35 MB** |
| Compile/write | **45ms** | — | 1.45s | **126ms** |
| Open | **0.24ms** | **52.6ms** | 68.3ms | **87.7ms** |
| Random lookup 10K | 59.1ms | **5.18ms** | 6.91ms | **4.58ms** |
| QPS | 169K | **1.93M** | 145K | **2.18M** |

### Analysis

1. **Eager achieves million-level QPS**: Lookup only needs a HashMap lookup and one `Arc` refcount bump—no repeated AST decoding or deep `Entry` clone.
2. **Lazy's scale sensitivity is clear**: ~169k QPS at 50K, drops to ~145k at 100K. The fixed 16 MiB block cache evicts on larger text data, causing repeated reads and decompression.
3. **Eager open cost has dropped**: Single index scan replaces per-headword repeated index lookups in preload; 50K open ~52.6ms, 100K ~87.7ms.

`Arc<Entry>` is a Rust API-layer optimization that doesn't change the disk format. FFI JSON returns still need to clone `Arc<Entry>` into an owned `Entry`, because the serialization boundary can't borrow the reader's internal cache.

### Rdict Advantages

- **Compact files**: zstd block compression + six-pool string dedup
- **Fast compile**: Direct binary encoding, no text parsing overhead
- **Fast Lazy open**: Only reads index header + directory, no content loading
- **Memory friendly**: Large dictionaries aren't fully loaded into memory (Lazy mode)

---

## 6. Optimization: Eager Preload Mode

### 6.1 Motivation

Rdict's lazy mode (on-demand block decompression + decoding) is memory-friendly for large dictionaries; eager mode eliminates repeated AST decoding overhead. Default Auto threshold is 10 MiB; clients can explicitly choose mode and threshold. In real dictionary apps:

- Total pack may be large, but media doesn't participate in eager decision; text size should be measured separately
- Users typically perform multiple lookups after opening
- Memory cost of keeping the whole dictionary in memory is acceptable

### 6.2 Design

Add **eager preload mode** to `RdictReader`, selecting strategy based on conservative text data size limit; media always read on-demand:

```
┌─────────────────────────────────────────────────────┐
│ RdictReader::open(path)                             │
│                                                     │
│  1. Read manifest, estimate text size               │
│  2. Text <= EAGER_THRESHOLD (default 10 MiB)?       │
│     ├─ Yes → eager mode:                            │
│     │   · Decompress all zstd blocks                │
│     │   · Decode all entries to HashMap<String, LookupEntry> │
│     │   · Subsequent lookup() hits HashMap (O(1))   │
│     │                                               │
│     └─ No → lazy mode (current behavior):           │
│         · Only load index header + directory         │
│         · lookup() decompresses block + decodes      │
└─────────────────────────────────────────────────────┘
```

### 6.3 API Changes

```rust
/// Read mode. Readers can let the library auto-select based on file size,
/// or force a specific mode.
pub enum ReadMode {
    /// Auto-select based on text size limit.
    Auto { eager_text_limit: u64 },
    /// Preload: decompress + decode everything to HashMap on open.
    Eager,
    /// Lazy: decompress blocks on demand, minimal memory usage
    Lazy,
}

impl<R: Read + Seek> RdictReader<R> {
    /// Open with explicit mode.
    pub fn open_with_mode(path: impl AsRef<Path>, mode: ReadMode) -> Result<Self>;

    /// Switch mode at runtime (e.g. lazy → eager)
    pub fn preload(&mut self) -> Result<()>;
}
```

### 6.4 Internal Implementation

```rust
struct RdictReader<R> {
    // existing fields...
    container: ZipContainer<R>,
    index: IndexReader,
    pools: StringPools,
    block_cache: HashMap<u32, CachedBlock>, // CachedBlock { bytes: Arc<[u8]>, last_used }

    // eager preload cache
    eager_cache: Option<HashMap<String, LookupEntry>>,
}

/// LookupEntry::Decoded uses Arc<Entry>,
/// so cache.get(hw).cloned() only does a refcount bump.
pub enum LookupEntry {
    Decoded(Arc<Entry>),
    Opaque { raw: Vec<u8> },
}

fn lookup(&mut self, headword: &str) -> Result<Option<LookupEntry>> {
    // 1. Check eager cache first (if preloaded)
    if let Some(cache) = &self.eager_cache {
        return Ok(cache.get(headword).cloned()); // Arc clone — no heap allocation
    }

    // 2. Fall back to lazy path (current logic)
    // ...
}
```

### 6.5 Memory Overhead

| Dictionary size | File size | Eager memory (est.) | Acceptable |
|---------|---------|-----------------|----------|
| 50K | 0.67 MB | ~5 MB | ✅ |
| 100K | 1.70 MB | ~13 MB | ✅ |
| 1M | 17.5 MB | ~130 MB | ⚠️ Near threshold |
| 5M | ~87 MB | ~650 MB | ❌ Should use lazy |

Default threshold 10 MiB applies to text data only; media files are always read on-demand.

### 6.6 What's Unaffected

- **Spec unchanged**: eager/lazy are read-layer optimizations, file format is identical
- **Writer unchanged**: write pipeline has no modifications
- **Lazy mode unchanged**: large dictionaries still load on-demand, memory-friendly
- **Format compatible**: eager/lazy and `Arc<Entry>` don't affect the Rdict v0.1.0 disk format. `LookupEntry::Decoded` holds `Arc<Entry>` — a design choice in this release, not a format change

Eager lookup has stably reached million-level QPS; open bears a one-time decode cost (50K ~53ms, 100K ~88ms), whether to enable should be decided by the client scenario.

### 6.7 Risks

- **Memory**: Eager mode uses more memory, but threshold control prevents triggering on GB-scale dictionaries
- **First-open latency**: Eager open is noticeably slower (50K ~53ms, 100K ~88ms; Lazy <1ms), suitable for clients that perform repeated lookups
- **`Arc<Entry>` reduces return cost**: Eager cache returning `LookupEntry` clone only increments refcount; FFI/JSON boundary still clones owned `Entry`

## 7. Current Status & Next Steps

Completed:

1. Lazy block cache: `Arc<[u8]>`, 16 MiB limit, LRU eviction.
2. Index scan reuses buffer, with pool/body range and integer overflow validation.
3. `ReadMode::{Auto, Eager, Lazy}`, default Auto text limit 10 MiB.
4. Eager text cache: `HashMap<String, LookupEntry>`, decoded entries use `Arc<Entry>`.
5. Media always read on-demand, unaffected by eager text threshold.

Next steps in this order:

1. Add focused API/behavior tests for `ReadMode`, `preload()`, Auto threshold, and eager/lazy result equivalence.
2. Measure uncompressed text size, eager memory peak, and open latency on real dictionaries; calibrate the 10 MiB default.
3. Add explicit Lazy/Eager selection in FFI and Swift bridge; keep Lazy as default to avoid UI open blocking.
4. Make Lazy block cache limit configurable; measure memory and hit rate on 50K/100K/larger real dictionaries; don't increase default limit directly.
5. Only evaluate borrow-based lookup when real data confirms AST decoding is still the main bottleneck; don't migrate to rkyv or maintain a second disk format.
