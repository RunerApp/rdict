//! `data/NNNNN.zst` — zstd-compressed text blocks. Each block is a
//! single zstd frame containing entry bytes packed back-to-back.

use crate::error::{Error, Result};

/// Default target uncompressed block size (256 KiB).
pub const DEFAULT_BLOCK_MAX: u32 = 262_144;

/// A single data block: the decompressed bytes and the list of entries
/// with their (offset, size) so the index can reference them.
#[derive(Debug, Clone)]
pub struct Block {
    pub data: Vec<u8>,
    pub entries: Vec<BlockEntry>,
}

#[derive(Debug, Clone, Copy)]
pub struct BlockEntry {
    pub offset: u64,
    pub size: u64,
    pub data_block_id: u32,
}

impl Block {
    pub fn new(_id: u32) -> Self {
        Self {
            data: Vec::new(),
            entries: Vec::new(),
        }
    }

    /// Append an entry's bytes to this block. Returns the offset it was
    /// placed at. Does NOT check size limits — the caller decides when
    /// to start a new block.
    pub fn push(&mut self, bytes: &[u8]) -> u64 {
        let offset = self.data.len() as u64;
        self.data.extend_from_slice(bytes);
        let size = bytes.len() as u64;
        self.entries.push(BlockEntry {
            offset,
            size,
            data_block_id: 0, // filled by caller
        });
        offset
    }
}

/// Split a list of (headword, encoded_entry_bytes) into blocks of at
/// most `block_max_uncompressed` bytes. An entry larger than the target
/// occupies its own block. Returns blocks and per-entry locations.
pub fn split_into_blocks(
    entries: Vec<(String, Vec<u8>)>,
    block_max: u32,
) -> Result<(Vec<Block>, Vec<EntryLocation>)> {
    let block_max = block_max as usize;
    let mut blocks: Vec<Block> = Vec::new();
    let mut locations: Vec<EntryLocation> = Vec::with_capacity(entries.len());

    for (headword, bytes) in entries {
        // Start a new block if current would overflow and it's non-empty.
        let need_new = match blocks.last() {
            None => true,
            Some(b) => b.data.len() + bytes.len() > block_max && !b.data.is_empty(),
        };
        if need_new {
            let id = blocks.len() as u32;
            blocks.push(Block::new(id));
        }
        let block_idx = blocks.len() - 1;
        let block = &mut blocks[block_idx];
        let offset = block.data.len() as u64;
        block.data.extend_from_slice(&bytes);
        let size = bytes.len() as u64;
        block.entries.push(BlockEntry {
            offset,
            size,
            data_block_id: block_idx as u32,
        });
        locations.push(EntryLocation {
            headword,
            data_block_id: block_idx as u32,
            offset,
            size,
        });
    }

    if blocks.is_empty() {
        // Always emit at least one block so the directory has something.
        blocks.push(Block::new(0));
    }

    Ok((blocks, locations))
}

#[derive(Debug, Clone)]
pub struct EntryLocation {
    pub headword: String,
    pub data_block_id: u32,
    pub offset: u64,
    pub size: u64,
}

/// Compress a block's data into a single zstd frame.
pub fn compress_block(data: &[u8], level: i32) -> Result<Vec<u8>> {
    zstd::encode_all(data, level).map_err(|e| Error::Zstd(e.to_string()))
}

/// Decompress a zstd frame into the original block bytes.
pub fn decompress_block(data: &[u8]) -> Result<Vec<u8>> {
    zstd::decode_all(data).map_err(|e| Error::Zstd(e.to_string()))
}

/// Format a block id as `data/NNNNN.zst`.
pub fn block_path(id: u32) -> String {
    format!("data/{:05}.zst", id)
}

/// Parse a `data/NNNNN.zst` path back to its id.
pub fn parse_block_path(path: &str) -> Option<u32> {
    let name = path.strip_prefix("data/")?;
    let stem = name.strip_suffix(".zst")?;
    let id: u32 = stem.parse().ok()?;
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_split_basic() {
        let entries = vec![
            ("a".into(), vec![0u8; 100]),
            ("b".into(), vec![0u8; 100]),
            ("c".into(), vec![0u8; 300]),
        ];
        let (blocks, locs) = split_into_blocks(entries, 256).unwrap();
        // a + b fit in block 0 (200 bytes), c needs block 1.
        assert_eq!(blocks.len(), 2);
        assert_eq!(locs[0].data_block_id, 0);
        assert_eq!(locs[1].data_block_id, 0);
        assert_eq!(locs[2].data_block_id, 1);
    }

    #[test]
    fn block_split_oversize_entry() {
        let entries = vec![("big".into(), vec![0u8; 1000])];
        let (blocks, locs) = split_into_blocks(entries, 256).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(locs[0].data_block_id, 0);
    }

    #[test]
    fn zstd_roundtrip() {
        let data = b"hello world hello world hello world";
        let compressed = compress_block(data, 19).unwrap();
        let decompressed = decompress_block(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }
}
