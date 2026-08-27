//! Pack reader: opens a `.rdict` file, loads the manifest and headword
//! index, and performs exact headword lookup with on-demand block
//! decompression.

use crate::ast::{self, DecodedEntry};
use crate::blocks;
use crate::container::ZipContainer;
use crate::error::{Error, Result};
use crate::index::IndexReader;
use crate::manifest::Manifest;
use crate::media::{MediaManifest, MediaManifestEntry};
use crate::model::Entry;
use crate::strings::StringPools;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;
use std::sync::Arc;

const BLOCK_CACHE_MAX_BYTES: usize = 16 * 1024 * 1024;
pub const DEFAULT_EAGER_TEXT_LIMIT: u64 = 10 * 1024 * 1024;

struct CachedBlock {
    bytes: Arc<[u8]>,
    last_used: u64,
}

/// The result of a headword lookup.
#[derive(Clone)]
pub enum LookupEntry {
    /// The entry was successfully decoded. The `Arc` makes cloning cheap
    /// (refcount bump) so that eager-mode lookups avoid deep-cloning
    /// every `String` and `Vec` in the entry tree.
    Decoded(Arc<Entry>),
    /// The entry contained an unknown flag bit or Def kind. The raw
    /// bytes are returned for opaque handling per §6.3.
    Opaque { raw: Vec<u8> },
}

/// A decoded tag index entry: (tag_id, entry_ids).
pub type TagPostings = Vec<(u16, Vec<u32>)>;

/// A decoded morph index entry: (key, entry_ids).
pub type MorphPostings = Vec<(String, Vec<u32>)>;

/// Controls whether text entries are decoded on open or on first lookup.
#[derive(Debug, Clone, Copy)]
pub enum ReadMode {
    /// Preload when the manifest's text-size upper bound is under the limit.
    Auto { eager_text_limit: u64 },
    /// Preload all text entries regardless of size.
    Eager,
    /// Decode entries on demand.
    Lazy,
}

impl Default for ReadMode {
    fn default() -> Self {
        Self::Auto {
            eager_text_limit: DEFAULT_EAGER_TEXT_LIMIT,
        }
    }
}

/// A reader over an open `.rdict` file.
pub struct RdictReader<R: Read + Seek> {
    container: ZipContainer<R>,
    manifest: Manifest,
    pools: StringPools,
    index: IndexReader,
    block_cache: HashMap<u32, CachedBlock>,
    block_cache_bytes: usize,
    cache_clock: u64,
    eager_cache: Option<HashMap<String, LookupEntry>>,
    media_manifest: Option<Option<MediaManifest>>,
}

impl RdictReader<File> {
    /// Open a `.rdict` file from disk.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        Self::new(file)
    }

    /// Open a `.rdict` file with an explicit text loading mode.
    pub fn open_with_mode(path: impl AsRef<Path>, mode: ReadMode) -> Result<Self> {
        let file = File::open(path)?;
        Self::new_with_mode(file, mode)
    }
}

impl<R: Read + Seek> RdictReader<R> {
    /// Create a reader from a seekable read stream.
    pub fn new(reader: R) -> Result<Self> {
        Self::new_with_mode(reader, ReadMode::default())
    }

    /// Create a reader with an explicit text loading mode.
    pub fn new_with_mode(reader: R, mode: ReadMode) -> Result<Self> {
        let mut container = ZipContainer::new(reader)?;

        // Verify mimetype.
        let mimetype = container.read_mimetype()?;
        crate::container::check_mimetype(&mimetype)?;

        // Load manifest.
        let manifest_bytes = container.read_member("manifest.json")?;
        let manifest = Manifest::from_json(&manifest_bytes)?;
        manifest.validate()?;

        // Load strings.tbl.
        let strings_bytes = container.read_member("index/strings.tbl")?;
        let pools = StringPools::decode(&strings_bytes)?;

        // Load headword.idx.
        let idx_bytes = container.read_member("index/headword.idx")?;
        let index = IndexReader::from_bytes(idx_bytes)?;

        // Cross-check counts.
        if index.header.entry_count != manifest.index.entry_count {
            return Err(Error::Malformed(format!(
                "entry_count mismatch: manifest={}, index={}",
                manifest.index.entry_count, index.header.entry_count
            )));
        }
        if index.header.block_count != manifest.index.block_count {
            return Err(Error::Malformed(format!(
                "block_count mismatch: manifest={}, index={}",
                manifest.index.block_count, index.header.block_count
            )));
        }
        if index.header.data_block_count != manifest.data.block_count {
            return Err(Error::Malformed(format!(
                "data_block_count mismatch: manifest={}, index={}",
                manifest.data.block_count, index.header.data_block_count
            )));
        }

        let should_eager = match mode {
            ReadMode::Eager => true,
            ReadMode::Lazy => false,
            ReadMode::Auto { eager_text_limit } => {
                (manifest.data.block_max_uncompressed as u64)
                    .saturating_mul(manifest.data.block_count as u64)
                    <= eager_text_limit
            }
        };
        let mut out = Self {
            container,
            manifest,
            pools,
            index,
            block_cache: HashMap::new(),
            block_cache_bytes: 0,
            cache_clock: 0,
            eager_cache: None,
            media_manifest: None,
        };
        if should_eager {
            out.preload()?;
        }
        Ok(out)
    }

    /// Look up a headword. Returns `Ok(None)` if not found, `Ok(Some(_))`
    /// if found (decoded or opaque), or `Err` on read/decode errors.
    pub fn lookup(&mut self, headword: &str) -> Result<Option<LookupEntry>> {
        if let Some(cache) = &self.eager_cache {
            return Ok(cache.get(headword).cloned());
        }
        self.lookup_lazy(headword)
    }

    fn lookup_lazy(&mut self, headword: &str) -> Result<Option<LookupEntry>> {
        let (block_id, offset, size) = match self.index.lookup(headword) {
            Some(loc) => loc,
            None => return Ok(None),
        };

        Ok(Some(self.decode_location(
            headword.to_string(),
            block_id,
            offset,
            size,
        )?))
    }

    fn decode_location(
        &mut self,
        headword: String,
        block_id: u32,
        offset: u64,
        size: u64,
    ) -> Result<LookupEntry> {
        // Decompress the block (cached), then decode directly from the cached
        // bytes. Only opaque/error paths copy the entry slice.
        self.ensure_block(block_id)?;
        let start = usize::try_from(offset)
            .map_err(|_| Error::Malformed("entry offset overflows usize".into()))?;
        let end = start
            .checked_add(
                usize::try_from(size)
                    .map_err(|_| Error::Malformed("entry size overflows usize".into()))?,
            )
            .ok_or_else(|| Error::Malformed("entry slice overflows usize".into()))?;
        let block_bytes = &self
            .block_cache
            .get(&block_id)
            .ok_or_else(|| Error::Malformed("cached block disappeared".into()))?
            .bytes;
        if end > block_bytes.len() {
            return Err(Error::Malformed(format!(
                "entry slice [{}, {}) extends past block (len {})",
                offset,
                end,
                block_bytes.len()
            )));
        }
        let entry_bytes = &block_bytes[start..end];

        // Decode AST.
        match ast::decode_entry(headword, entry_bytes, &self.pools) {
            Ok(DecodedEntry::Decoded(entry)) => Ok(LookupEntry::Decoded(Arc::new(entry))),
            Ok(DecodedEntry::Opaque(raw)) => Ok(LookupEntry::Opaque { raw }),
            Err(_) => {
                // Per §6.3, decode failures make the entry opaque.
                Ok(LookupEntry::Opaque {
                    raw: entry_bytes.to_vec(),
                })
            }
        }
    }

    /// Decode all text entries into memory. Media remains on-demand.
    pub fn preload(&mut self) -> Result<()> {
        if self.eager_cache.is_some() {
            return Ok(());
        }
        let locations = self.index.collect_locations()?;
        let mut cache = HashMap::with_capacity(locations.len());
        for location in locations {
            let key = location.headword.clone();
            let entry = self.decode_location(
                location.headword,
                location.data_block_id,
                location.offset,
                location.size,
            )?;
            cache.insert(key, entry);
        }
        self.block_cache.clear();
        self.block_cache_bytes = 0;
        self.eager_cache = Some(cache);
        Ok(())
    }

    /// Ensure a decompressed block is resident without copying it on lookup.
    fn ensure_block(&mut self, block_id: u32) -> Result<()> {
        self.cache_clock = self.cache_clock.wrapping_add(1);
        let clock = self.cache_clock;
        if let Some(block) = self.block_cache.get_mut(&block_id) {
            block.last_used = clock;
            return Ok(());
        }

        let path = blocks::block_path(block_id);
        let compressed = self.container.read_member(&path)?;
        let decompressed: Arc<[u8]> = blocks::decompress_block(&compressed)?.into();
        let size = decompressed.len();

        while !self.block_cache.is_empty()
            && self.block_cache_bytes.saturating_add(size) > BLOCK_CACHE_MAX_BYTES
        {
            let evict = self
                .block_cache
                .iter()
                .min_by_key(|(_, block)| block.last_used)
                .map(|(id, _)| *id)
                .expect("cache is not empty");
            if let Some(block) = self.block_cache.remove(&evict) {
                self.block_cache_bytes -= block.bytes.len();
            }
        }
        // Keep one oversized block for the current lookup. The next insert
        // evicts it before adding another block.
        self.block_cache.insert(
            block_id,
            CachedBlock {
                bytes: decompressed,
                last_used: clock,
            },
        );
        self.block_cache_bytes += size;
        Ok(())
    }

    /// Access the manifest.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Access the string pools.
    pub fn pools(&self) -> &StringPools {
        &self.pools
    }

    /// Access the headword index header.
    pub fn index_header(&self) -> &crate::index::Header {
        &self.index.header
    }

    /// Load the media manifest (if present), caching it for subsequent calls.
    /// Validates the manifest on first load.
    pub fn media_manifest(&mut self) -> Result<Option<MediaManifest>> {
        if self.media_manifest.is_none() {
            if self.container.has_member("media/manifest.json") {
                let bytes = self.container.read_member("media/manifest.json")?;
                let manifest = MediaManifest::from_json(&bytes)?;
                crate::media::validate(&manifest)?;
                self.media_manifest = Some(Some(manifest));
            } else {
                self.media_manifest = Some(None);
            }
        }
        Ok(self.media_manifest.as_ref().unwrap().clone())
    }

    /// Look up a media entry by (kind, hash_hex) in the manifest.
    pub fn media_info(&mut self, kind: &str, hash_hex: &str) -> Result<Option<MediaManifestEntry>> {
        let manifest = self.media_manifest()?;
        match manifest {
            None => Ok(None),
            Some(m) => Ok(m
                .entries
                .iter()
                .find(|e| e.kind == kind && e.hash == hash_hex)
                .cloned()),
        }
    }

    /// Reconstruct the ZIP member path for a manifest entry.
    fn media_zip_path(entry: &MediaManifestEntry) -> String {
        let hh = if entry.hash.len() >= 2 {
            &entry.hash[..2]
        } else {
            "00"
        };
        format!("media/{}/{}/{}.{}", entry.kind, hh, entry.hash, entry.ext)
    }

    /// Read a media file's bytes by (kind, hash_hex). Automatically
    /// decompresses zstd-compressed media. Returns the raw (uncompressed)
    /// media bytes.
    pub fn read_media(&mut self, kind: &str, hash_hex: &str) -> Result<Vec<u8>> {
        let entry = self
            .media_info(kind, hash_hex)?
            .ok_or_else(|| Error::NotFound(format!("media not found: {kind}/{hash_hex}")))?;
        let path = Self::media_zip_path(&entry);
        let raw = self.container.read_member(&path)?;
        match entry.compression.as_str() {
            "none" => Ok(raw),
            "zstd" => zstd::decode_all(raw.as_slice()).map_err(|e| {
                Error::Malformed(format!("zstd decompress media {kind}/{hash_hex}: {e}"))
            }),
            other => Err(Error::Malformed(format!(
                "unknown compression {other} for media {kind}/{hash_hex}"
            ))),
        }
    }

    /// Extract a media file directly to `output_path` via streaming,
    /// avoiding buffering the full media in memory. Creates parent
    /// directories, writes to a temp file, then atomically renames.
    /// Returns the number of bytes written.
    pub fn extract_media(&mut self, kind: &str, hash_hex: &str, output_path: &Path) -> Result<u64> {
        let entry = self
            .media_info(kind, hash_hex)?
            .ok_or_else(|| Error::NotFound(format!("media not found: {kind}/{hash_hex}")))?;
        let zip_path = Self::media_zip_path(&entry);

        // Create parent directories.
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write to a temp file in the same directory (for same-filesystem rename).
        let tmp_path = output_path.with_extension("rdict-tmp");

        let bytes_written = match entry.compression.as_str() {
            "zstd" => {
                // Stream compressed member through zstd decoder directly to file.
                let mut f = File::create(&tmp_path)?;
                self.container.stream_member_zstd(&zip_path, &mut f)?
            }
            "none" => {
                // For uncompressed media, stream directly from ZIP to file.
                let mut f = File::create(&tmp_path)?;
                self.container.stream_member(&zip_path, &mut f)?
            }
            other => {
                return Err(Error::Malformed(format!(
                    "unknown compression {other} for media {kind}/{hash_hex}"
                )));
            }
        };

        // Atomic rename.
        std::fs::rename(&tmp_path, output_path)?;
        Ok(bytes_written)
    }

    /// Read the cover image bytes, if present. Returns None if no cover.
    pub fn read_cover(&mut self) -> Result<Option<Vec<u8>>> {
        let cover_path = self.manifest.pack.cover.as_deref();
        match cover_path {
            Some(path) if self.container.has_member(path) => {
                Ok(Some(self.container.read_member(path)?))
            }
            _ => Ok(None),
        }
    }

    /// Read the raw bytes of the tag index (`index/tag.idx`), if present.
    pub fn read_tag_index(&mut self) -> Result<Option<Vec<u8>>> {
        if self.container.has_member("index/tag.idx") {
            Ok(Some(self.container.read_member("index/tag.idx")?))
        } else {
            Ok(None)
        }
    }

    /// Read the raw bytes of the morph index (`index/morph.idx`), if present.
    pub fn read_morph_index(&mut self) -> Result<Option<Vec<u8>>> {
        if self.container.has_member("index/morph.idx") {
            Ok(Some(self.container.read_member("index/morph.idx")?))
        } else {
            Ok(None)
        }
    }

    /// Decode the tag index, if present. Returns `(tag_id, entry_ids)`
    /// pairs where `tag_id` is a 1-based strref into the tag pool and
    /// `entry_ids` are 0-based ordinals in headword-sorted order.
    pub fn decode_tag_index(&mut self) -> Result<Option<TagPostings>> {
        if let Some(bytes) = self.read_tag_index()? {
            Ok(Some(crate::postings::decode_tag_index(&bytes)?))
        } else {
            Ok(None)
        }
    }

    /// Decode the morph index, if present. Returns `(key, entry_ids)`
    /// pairs where `key` is a feats `key=value` string and `entry_ids`
    /// are 0-based ordinals in headword-sorted order.
    pub fn decode_morph_index(&mut self) -> Result<Option<MorphPostings>> {
        if let Some(bytes) = self.read_morph_index()? {
            Ok(Some(crate::postings::decode_morph_index(&bytes)?))
        } else {
            Ok(None)
        }
    }

    /// List all headwords by scanning the index. Useful for tests and
    /// tooling; not optimized for large dictionaries.
    pub fn list_headwords(&self) -> Result<Vec<String>> {
        self.index.collect_headwords()
    }

    /// Case-insensitive prefix search. Returns up to `limit` headwords
    /// whose lowercased form starts with `prefix`. Headwords are in
    /// original case, in index (sorted) order. `limit <= 0` returns empty.
    pub fn prefix(&self, prefix: &str, limit: usize) -> Vec<String> {
        self.index.prefix_search(prefix, limit)
    }
}

#[cfg(test)]
mod tests {
    // Integration tests live in tests/roundtrip.rs
}
