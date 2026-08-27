# Rdict Reader API

## Loading modes

`RdictReader` separates dictionary text from media. Eager loading only
decodes text entries; audio and video remain on-demand.

```rust
use rdict::{ReadMode, RdictReader};

// Default: Auto with a 10 MiB conservative text-size limit.
let mut reader = RdictReader::open("english.rdict")?;

// Open immediately and decode entries only when queried.
let mut reader = RdictReader::open_with_mode("english.rdict", ReadMode::Lazy)?;

// Decode all text before returning. Media is still lazy.
let mut reader = RdictReader::open_with_mode("english.rdict", ReadMode::Eager)?;

// Client-defined Auto threshold.
let mode = ReadMode::Auto {
    eager_text_limit: 32 * 1024 * 1024,
};
let mut reader = RdictReader::open_with_mode("english.rdict", mode)?;
```

`Auto` uses `block_max_uncompressed * block_count` as a conservative upper
bound for text data. This intentionally overestimates slightly so media size
does not affect the decision. The default is exported as
`DEFAULT_EAGER_TEXT_LIMIT` (10 MiB).

## Lookup

```rust
match reader.lookup("run")? {
    Some(rdict::LookupEntry::Decoded(entry)) => {
        println!("{}", entry.headword);
    }
    Some(rdict::LookupEntry::Opaque { raw }) => {
        // Unknown flags/kinds are preserved for forward-compatible handling.
        println!("opaque entry: {} bytes", raw.len());
    }
    None => println!("not found"),
}
```

`LookupEntry::Decoded` contains `Arc<Entry>`. Cloning the lookup result is
cheap and only increments the reference count. This is a public API choice;
callers that require an owned value can use `entry.as_ref().clone()`.

Malformed data returns `Err`. Unknown entry flags or definition kinds return
`Opaque` according to the format's forward-compatibility rule.

## Explicit preload

Start with Lazy and block at a deliberate point if the client wants to switch
to eager lookup:

```rust
let mut reader = RdictReader::open_with_mode(path, ReadMode::Lazy)?;
// The call blocks until all text entries are decoded.
reader.preload()?;
let result = reader.lookup("run")?;
```

`RdictReader` is not a concurrent reader. `lookup()` and `preload()` both need
`&mut self`; do not call them concurrently on the same instance. A client
that needs a non-blocking UI should run a separate Lazy/Eager reader task and
swap at the application layer.

## Media

Use `read_media()` with the path reconstructed from the media manifest. Media
bytes are never placed in the eager text cache.

## FFI note

The C ABI serializes a decoded entry to JSON. That boundary converts the
internal `Arc<Entry>` to an owned `Entry`, so the Arc optimization applies to
Rust lookups and eager cache hits, not to the final JSON allocation.

## Compatibility

The loading modes and `Arc<Entry>` are reader API behavior only. They do not
change the Rdict v0.1.0 on-disk format or its varint AST encoding.
