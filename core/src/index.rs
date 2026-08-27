//! `index/headword.idx` — binary headword index with front-coding
//! blocks and a fixed-size directory for binary search.

use crate::blocks::EntryLocation;
use crate::error::{Error, Result};
use crate::primitive::{self, BoundedReader, ByteWriter};
use std::io::{Read, Seek, SeekFrom, Write};

/// Magic for `headword.idx`.
pub const MAGIC: &[u8; 4] = b"RDID";

/// Index header version.
pub const VERSION: u16 = 1;

/// Number of entries per front-coding block.
pub const BLOCK_SIZE: usize = 256;

/// Header size in bytes.
pub const HEADER_SIZE: usize = 24;

/// Directory entry size in bytes.
pub const DIR_ENTRY_SIZE: usize = 24;

/// 24-byte header.
#[derive(Debug, Clone)]
pub struct Header {
    pub version: u16,
    pub flags: u16,
    pub entry_count: u32,
    pub block_count: u32,
    pub data_block_count: u32,
}

impl Header {
    pub fn write_to<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_all(MAGIC)?;
        primitive::write_u16(w, self.version)?;
        primitive::write_u16(w, self.flags)?;
        primitive::write_u32(w, self.entry_count)?;
        primitive::write_u32(w, self.block_count)?;
        primitive::write_u32(w, self.data_block_count)?;
        primitive::write_u32(w, 0)?; // reserved
        Ok(())
    }

    pub fn read_from<R: Read>(r: &mut R) -> Result<Self> {
        let mut magic = [0u8; 4];
        primitive::read_exact(r, &mut magic)?;
        if &magic != MAGIC {
            return Err(Error::Malformed("headword.idx: bad magic".into()));
        }
        let version = primitive::read_u16(r)?;
        if version != VERSION {
            return Err(Error::UnsupportedVersion(format!(
                "headword.idx version {}",
                version
            )));
        }
        let flags = primitive::read_u16(r)?;
        let entry_count = primitive::read_u32(r)?;
        let block_count = primitive::read_u32(r)?;
        let data_block_count = primitive::read_u32(r)?;
        let reserved = primitive::read_u32(r)?;
        if reserved != 0 {
            return Err(Error::Malformed("headword.idx: reserved must be 0".into()));
        }
        if flags != 0 {
            // bit 0 = secondary FST, not supported in v0.1 reader; we ignore
            // it gracefully per §4.5.
        }
        Ok(Self {
            version,
            flags,
            entry_count,
            block_count,
            data_block_count,
        })
    }
}

/// 24-byte directory entry.
#[derive(Debug, Clone, Copy)]
pub struct DirEntry {
    pub first_headword_len: u16,
    pub first_headword_offset: u64,
    pub block_entry_count: u32,
    pub first_headword_pool_offset: u64,
}

impl DirEntry {
    pub fn write_to<W: Write>(&self, w: &mut W) -> Result<()> {
        primitive::write_u16(w, self.first_headword_len)?;
        primitive::write_u16(w, 0)?; // pad
        primitive::write_u64(w, self.first_headword_offset)?;
        primitive::write_u32(w, self.block_entry_count)?;
        primitive::write_u64(w, self.first_headword_pool_offset)?;
        Ok(())
    }

    pub fn read_from<R: Read>(r: &mut R) -> Result<Self> {
        let first_headword_len = primitive::read_u16(r)?;
        let pad = primitive::read_u16(r)?;
        if pad != 0 {
            return Err(Error::Malformed("headword.idx: dir pad must be 0".into()));
        }
        let first_headword_offset = primitive::read_u64(r)?;
        let block_entry_count = primitive::read_u32(r)?;
        let first_headword_pool_offset = primitive::read_u64(r)?;
        Ok(Self {
            first_headword_len,
            first_headword_offset,
            block_entry_count,
            first_headword_pool_offset,
        })
    }
}

/// A fully-built headword index, in-memory.
pub struct HeadwordIndex {
    pub header: Header,
    pub directory: Vec<DirEntry>,
    /// The entire file bytes: header + directory + pool + body. This is
    /// what gets written into the ZIP member.
    pub bytes: Vec<u8>,
}

/// Build `headword.idx` bytes from a sorted list of entry locations.
///
/// The entries MUST be sorted by case-folded codepoint order (§4.2.1).
pub fn build_index(locations: &[EntryLocation], data_block_count: u32) -> Result<HeadwordIndex> {
    if locations.is_empty() {
        return build_empty_index(data_block_count);
    }

    // Verify sorted and unique (case-folded order, case-sensitive tie-break).
    for w in locations.windows(2) {
        if compare_headwords(&w[0].headword, &w[1].headword) != std::cmp::Ordering::Less {
            return Err(Error::Validation(format!(
                "headwords not sorted or duplicate: {} >= {}",
                w[0].headword, w[1].headword
            )));
        }
    }

    // Partition into front-coding blocks of BLOCK_SIZE.
    let block_count = locations.len().div_ceil(BLOCK_SIZE);

    // Layout plan:
    //   [0, 24)                          header
    //   [24, 24 + 24*block_count)        directory
    //   [dir_end, ...)                    first-headword pool (one entry per block)
    //   [pool_end, ...)                   index body (front-coded entries)
    let dir_end = HEADER_SIZE + DIR_ENTRY_SIZE * block_count;

    // Compute pool layout: each block's first headword is stored in the pool.
    // Pool entries are variable-length (the raw headword bytes), placed
    // back-to-back. We record each block's pool offset and length.
    let mut pool_offsets: Vec<(u64, u16)> = Vec::with_capacity(block_count); // (offset, len)
    let mut pool_bytes: Vec<u8> = Vec::new();
    for b in 0..block_count {
        let start = b * BLOCK_SIZE;
        let first_hw = &locations[start].headword;
        let hw_bytes = first_hw.as_bytes();
        let len = u16::try_from(hw_bytes.len())
            .map_err(|_| Error::Validation("headword exceeds u16 length".into()))?;
        pool_offsets.push((dir_end as u64 + pool_bytes.len() as u64, len));
        pool_bytes.extend_from_slice(hw_bytes);
    }
    let pool_end = dir_end + pool_bytes.len();

    // Build the index body (front-coded entries).
    let mut body = ByteWriter::new();
    let mut block_body_offsets: Vec<u64> = Vec::with_capacity(block_count);
    for b in 0..block_count {
        block_body_offsets.push(pool_end as u64 + body.len() as u64);
        let start = b * BLOCK_SIZE;
        let end = (start + BLOCK_SIZE).min(locations.len());
        let mut prev: &[u8] = &[];
        for loc in &locations[start..end] {
            let hw_bytes = loc.headword.as_bytes();
            let shared = common_prefix_len(prev, hw_bytes);
            let suffix = &hw_bytes[shared..];
            primitive::write_varint(&mut body, shared as u64)?;
            primitive::write_varint(&mut body, suffix.len() as u64)?;
            body.buf.extend_from_slice(suffix);
            primitive::write_varint(&mut body, loc.data_block_id as u64)?;
            primitive::write_varint(&mut body, loc.offset)?;
            primitive::write_varint(&mut body, loc.size)?;
            prev = hw_bytes;
        }
    }

    // Assemble directory entries.
    let mut directory = Vec::with_capacity(block_count);
    for b in 0..block_count {
        let start = b * BLOCK_SIZE;
        let end = (start + BLOCK_SIZE).min(locations.len());
        let (pool_off, pool_len) = pool_offsets[b];
        let body_off = block_body_offsets[b];
        directory.push(DirEntry {
            first_headword_len: pool_len,
            first_headword_offset: body_off,
            block_entry_count: (end - start) as u32,
            first_headword_pool_offset: pool_off,
        });
    }

    // Assemble the full file.
    let mut bytes = Vec::with_capacity(pool_end + body.len());
    let header = Header {
        version: VERSION,
        flags: 0,
        entry_count: locations.len() as u32,
        block_count: block_count as u32,
        data_block_count,
    };
    header.write_to(&mut bytes)?;
    for d in &directory {
        d.write_to(&mut bytes)?;
    }
    bytes.extend_from_slice(&pool_bytes);
    bytes.extend_from_slice(&body.buf);

    debug_assert_eq!(bytes.len(), pool_end + body.len());

    Ok(HeadwordIndex {
        header,
        directory,
        bytes,
    })
}

fn build_empty_index(data_block_count: u32) -> Result<HeadwordIndex> {
    let header = Header {
        version: VERSION,
        flags: 0,
        entry_count: 0,
        block_count: 0,
        data_block_count,
    };
    let mut bytes = Vec::with_capacity(HEADER_SIZE);
    header.write_to(&mut bytes)?;
    Ok(HeadwordIndex {
        header,
        directory: Vec::new(),
        bytes,
    })
}

fn common_prefix_len(a: &[u8], b: &[u8]) -> usize {
    let mut i = 0;
    while i < a.len() && i < b.len() && a[i] == b[i] {
        i += 1;
    }
    i
}

/// Compare two headwords by case-folded codepoint order, with
/// case-sensitive order as tie-breaker (§4.2.1).
pub fn compare_headwords(a: &str, b: &str) -> std::cmp::Ordering {
    let a_lower = to_lowercase_bytes(a);
    let b_lower = to_lowercase_bytes(b);
    match a_lower.cmp(&b_lower) {
        std::cmp::Ordering::Equal => a.as_bytes().cmp(b.as_bytes()),
        ord => ord,
    }
}

/// Compare a lowercased prefix against a headword's lowercased form.
/// Returns Ordering of lowercased(headword) vs lowercased(prefix).
pub fn compare_prefix(headword: &str, prefix_lower: &[u8]) -> std::cmp::Ordering {
    let hw_lower = to_lowercase_bytes(headword);
    // Compare only up to prefix length for prefix matching
    if hw_lower.len() < prefix_lower.len() {
        hw_lower.as_slice().cmp(prefix_lower)
    } else {
        hw_lower[..prefix_lower.len()].cmp(prefix_lower)
    }
}

/// Lowercase a string to UTF-8 bytes using Unicode simple case-folding.
fn to_lowercase_bytes(s: &str) -> Vec<u8> {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .collect::<String>()
        .into_bytes()
}

// ===== Reader =====

/// In-memory reader over a `headword.idx` file's bytes.
pub struct IndexReader {
    pub header: Header,
    pub directory: Vec<DirEntry>,
    bytes: Vec<u8>,
    body_start: u64,
}

impl IndexReader {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let mut r = BoundedReader::new(&bytes);
        let header = Header::read_from(&mut r)?;
        let mut directory = Vec::with_capacity(header.block_count as usize);
        for _ in 0..header.block_count {
            directory.push(DirEntry::read_from(&mut r)?);
        }

        // Compute body start: max(first_headword_pool_offset + first_headword_len)
        // across the directory, or end of directory when block_count == 0.
        let dir_end = HEADER_SIZE + DIR_ENTRY_SIZE * header.block_count as usize;
        let body_start = if header.block_count == 0 {
            dir_end as u64
        } else {
            let mut max_end = 0u64;
            for d in &directory {
                let end = d
                    .first_headword_pool_offset
                    .checked_add(d.first_headword_len as u64)
                    .ok_or_else(|| {
                        Error::Malformed("headword.idx: pool offset overflows".into())
                    })?;
                if end > max_end {
                    max_end = end;
                }
            }
            // Validate pool/body ranges.
            if max_end > bytes.len() as u64 {
                return Err(Error::Malformed(
                    "headword.idx: pool extends past file end".into(),
                ));
            }
            if max_end < dir_end as u64 {
                return Err(Error::Malformed(
                    "headword.idx: first-headword pool overlaps directory".into(),
                ));
            }
            for d in &directory {
                let pool_end = d
                    .first_headword_pool_offset
                    .checked_add(d.first_headword_len as u64)
                    .ok_or_else(|| {
                        Error::Malformed("headword.idx: pool offset overflows".into())
                    })?;
                if d.first_headword_pool_offset < dir_end as u64 || pool_end > bytes.len() as u64 {
                    return Err(Error::Malformed(
                        "headword.idx: first-headword pool is out of bounds".into(),
                    ));
                }
                if d.first_headword_offset < max_end || d.first_headword_offset > bytes.len() as u64
                {
                    return Err(Error::Malformed(
                        "headword.idx: block body is out of bounds".into(),
                    ));
                }
            }
            max_end
        };

        Ok(Self {
            header,
            directory,
            bytes,
            body_start,
        })
    }

    /// Look up a headword (case-sensitive exact match). Returns its
    /// (data_block_id, offset, size) if found.
    pub fn lookup(&self, headword: &str) -> Option<(u32, u64, u64)> {
        if self.directory.is_empty() {
            return None;
        }
        // Binary search using case-folded comparison to find the block.
        let needle_lower = to_lowercase_bytes(headword);
        let mut lo = 0usize;
        let mut hi = self.directory.len();
        while lo + 1 < hi {
            let mid = (lo + hi) / 2;
            let first_hw = self.read_first_headword(&self.directory[mid]).ok()?;
            let first_lower = to_lowercase_bytes(std::str::from_utf8(first_hw).ok()?);
            if first_lower.as_slice() <= needle_lower.as_slice() {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let block = &self.directory[lo];
        // Scan this block's entries with case-sensitive comparison.
        self.scan_block(block, headword, &needle_lower)
    }

    fn read_first_headword(&self, d: &DirEntry) -> Result<&[u8]> {
        let start = usize::try_from(d.first_headword_pool_offset)
            .map_err(|_| Error::Malformed("first headword offset overflows usize".into()))?;
        let end = start
            .checked_add(d.first_headword_len as usize)
            .ok_or_else(|| Error::Malformed("first headword range overflows usize".into()))?;
        if end > self.bytes.len() {
            return Err(Error::Malformed("first headword out of bounds".into()));
        }
        Ok(&self.bytes[start..end])
    }

    fn scan_block(
        &self,
        block: &DirEntry,
        needle: &str,
        needle_lower: &[u8],
    ) -> Option<(u32, u64, u64)> {
        let body_start = block.first_headword_offset as usize;
        if body_start < self.body_start as usize {
            return None;
        }
        let mut r = BoundedReader::new(&self.bytes[body_start..]);
        let mut prev = Vec::new();
        let mut current = Vec::new();
        for _ in 0..block.block_entry_count {
            let shared = primitive::read_varint(&mut r).ok()?;
            let suffix_len = primitive::read_varint(&mut r).ok()? as usize;
            let suffix = r.read_bytes(suffix_len).ok()?;
            let data_block_id = primitive::read_varint(&mut r).ok()? as u32;
            let offset = primitive::read_varint(&mut r).ok()?;
            let size = primitive::read_varint(&mut r).ok()?;

            let shared = usize::try_from(shared).ok()?;
            if shared > prev.len() {
                return None;
            }
            current.clear();
            current.extend_from_slice(&prev[..shared]);
            current.extend_from_slice(suffix);

            // Case-folded comparison for ordering, case-sensitive for exact match.
            let current_str = std::str::from_utf8(&current).ok()?;
            let current_lower = to_lowercase_bytes(current_str);
            if current_lower == needle_lower && current.as_slice() == needle.as_bytes() {
                return Some((data_block_id, offset, size));
            }
            if current_lower.as_slice() > needle_lower {
                // Passed the needle; not present (entries are sorted by case-folded order).
                return None;
            }
            std::mem::swap(&mut prev, &mut current);
        }
        None
    }

    /// Collect all headwords from the index, in sorted order.
    pub fn collect_headwords(&self) -> Result<Vec<String>> {
        let mut out = Vec::with_capacity(self.header.entry_count as usize);
        for block in &self.directory {
            let body_start = block.first_headword_offset as usize;
            if body_start < self.body_start as usize {
                return Err(Error::Malformed("block body before body_start".into()));
            }
            let mut r = BoundedReader::new(&self.bytes[body_start..]);
            let mut prev: Vec<u8> = Vec::new();
            for _ in 0..block.block_entry_count {
                let shared = primitive::read_varint(&mut r)? as usize;
                let suffix_len = primitive::read_varint(&mut r)? as usize;
                let suffix = r.read_bytes(suffix_len)?;
                // Skip location data (data_block_id, offset, size).
                primitive::read_varint(&mut r)?;
                primitive::read_varint(&mut r)?;
                primitive::read_varint(&mut r)?;
                let mut hw = prev[..shared.min(prev.len())].to_vec();
                hw.extend_from_slice(suffix);
                let hw_str = String::from_utf8(hw.clone())
                    .map_err(|e| Error::Malformed(format!("headword not UTF-8: {e}")))?;
                out.push(hw_str);
                prev = hw;
            }
        }
        Ok(out)
    }

    /// Case-insensitive prefix search. Returns up to `limit` headwords
    /// whose lowercased form starts with `prefix_lower`. The returned
    /// headwords are in original case, in index order.
    pub fn prefix_search(&self, prefix: &str, limit: usize) -> Vec<String> {
        if limit == 0 || self.directory.is_empty() {
            return Vec::new();
        }
        let prefix_lower = to_lowercase_bytes(prefix);
        if prefix_lower.is_empty() {
            return Vec::new();
        }

        // Binary search for the first block whose lowercased first headword
        // is >= prefix_lower. But the match may start in the previous block
        // (e.g. block starts with "banana" but "apple" is in the previous block
        // and "apple" < "banana" but starts with "app"). So we check from
        // max(0, lo-1).
        let mut lo = 0usize;
        let mut hi = self.directory.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            let first_hw = match self.read_first_headword(&self.directory[mid]) {
                Ok(hw) => hw,
                Err(_) => return Vec::new(),
            };
            let first_str = match std::str::from_utf8(first_hw) {
                Ok(s) => s,
                Err(_) => return Vec::new(),
            };
            let first_lower = to_lowercase_bytes(first_str);
            // If this block's first headword (lowercased) starts with the prefix,
            // or is > prefix, this block is a candidate. Otherwise, search right.
            if first_lower.as_slice() >= prefix_lower.as_slice() {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        // Start from one block before (the match may straddle the boundary).
        let start_block = lo.saturating_sub(1);

        // Scan forward from start_block, collecting matching headwords.
        let mut results = Vec::with_capacity(limit.min(64));
        for block in &self.directory[start_block..] {
            let body_start = block.first_headword_offset as usize;
            if body_start < self.body_start as usize {
                break;
            }
            let mut r = BoundedReader::new(&self.bytes[body_start..]);
            let mut prev: Vec<u8> = Vec::new();
            let mut current = Vec::new();
            for _ in 0..block.block_entry_count {
                let shared = match primitive::read_varint(&mut r) {
                    Ok(v) => v as usize,
                    Err(_) => return results,
                };
                let suffix_len = match primitive::read_varint(&mut r) {
                    Ok(v) => v as usize,
                    Err(_) => return results,
                };
                let suffix = match r.read_bytes(suffix_len) {
                    Ok(s) => s,
                    Err(_) => return results,
                };
                // Skip location data.
                let _ = primitive::read_varint(&mut r);
                let _ = primitive::read_varint(&mut r);
                let _ = primitive::read_varint(&mut r);

                if shared > prev.len() {
                    return results;
                }
                current.clear();
                current.extend_from_slice(&prev[..shared]);
                current.extend_from_slice(suffix);

                let current_str = match std::str::from_utf8(&current) {
                    Ok(s) => s,
                    Err(_) => return results,
                };
                let current_lower = to_lowercase_bytes(current_str);

                if current_lower.len() >= prefix_lower.len()
                    && &current_lower[..prefix_lower.len()] == prefix_lower.as_slice()
                {
                    results.push(current_str.to_string());
                    if results.len() >= limit {
                        return results;
                    }
                } else if current_lower.as_slice() > prefix_lower.as_slice() {
                    // Passed all matches.
                    return results;
                }
                std::mem::swap(&mut prev, &mut current);
            }
        }
        results
    }

    /// Collect all headwords and their data locations in one index scan.
    pub(crate) fn collect_locations(&self) -> Result<Vec<EntryLocation>> {
        let mut out = Vec::with_capacity(self.header.entry_count as usize);
        for block in &self.directory {
            let body_start = block.first_headword_offset as usize;
            if body_start < self.body_start as usize {
                return Err(Error::Malformed("block body before body_start".into()));
            }
            let mut r = BoundedReader::new(&self.bytes[body_start..]);
            let mut prev = Vec::new();
            let mut current = Vec::new();
            for _ in 0..block.block_entry_count {
                let shared = usize::try_from(primitive::read_varint(&mut r)?)
                    .map_err(|_| Error::Malformed("shared prefix overflows usize".into()))?;
                let suffix_len = usize::try_from(primitive::read_varint(&mut r)?)
                    .map_err(|_| Error::Malformed("headword suffix overflows usize".into()))?;
                let suffix = r.read_bytes(suffix_len)?;
                let data_block_id = u32::try_from(primitive::read_varint(&mut r)?)
                    .map_err(|_| Error::Malformed("data block id overflows u32".into()))?;
                let offset = primitive::read_varint(&mut r)?;
                let size = primitive::read_varint(&mut r)?;
                if shared > prev.len() {
                    return Err(Error::Malformed(
                        "shared prefix exceeds previous headword".into(),
                    ));
                }
                current.clear();
                current.extend_from_slice(&prev[..shared]);
                current.extend_from_slice(suffix);
                let headword = String::from_utf8(current.clone())
                    .map_err(|e| Error::Malformed(format!("headword not UTF-8: {e}")))?;
                out.push(EntryLocation {
                    headword,
                    data_block_id,
                    offset,
                    size,
                });
                std::mem::swap(&mut prev, &mut current);
            }
        }
        Ok(out)
    }
}

/// A reader that operates over a file handle, loading only the header
/// and directory at open time and preading block bodies on lookup.
pub struct FileIndexReader<R: Read + Seek> {
    pub header: Header,
    pub directory: Vec<DirEntry>,
    reader: R,
    body_start: u64,
}

impl<R: Read + Seek> FileIndexReader<R> {
    pub fn new(mut reader: R) -> Result<Self> {
        let mut header_buf = [0u8; HEADER_SIZE];
        primitive::read_exact(&mut reader, &mut header_buf)?;
        let header = Header::read_from(&mut BoundedReader::new(&header_buf))?;

        let dir_size = DIR_ENTRY_SIZE * header.block_count as usize;
        let mut dir_buf = vec![0u8; dir_size];
        primitive::read_exact(&mut reader, &mut dir_buf)?;
        let mut dr = BoundedReader::new(&dir_buf);
        let mut directory = Vec::with_capacity(header.block_count as usize);
        for _ in 0..header.block_count {
            directory.push(DirEntry::read_from(&mut dr)?);
        }

        // Compute body start.
        let dir_end = HEADER_SIZE + dir_size;
        let body_start = if header.block_count == 0 {
            dir_end as u64
        } else {
            let mut max_end = 0u64;
            for d in &directory {
                let end = d
                    .first_headword_pool_offset
                    .checked_add(d.first_headword_len as u64)
                    .ok_or_else(|| {
                        Error::Malformed("headword.idx: pool offset overflows".into())
                    })?;
                if end > max_end {
                    max_end = end;
                }
            }
            max_end
        };

        Ok(Self {
            header,
            directory,
            reader,
            body_start,
        })
    }

    /// Look up a headword (case-sensitive exact match) by reading block bodies on demand.
    pub fn lookup(&mut self, headword: &str) -> Result<Option<(u32, u64, u64)>> {
        if self.directory.is_empty() {
            return Ok(None);
        }
        let needle_lower = to_lowercase_bytes(headword);
        // Binary search directory using case-folded comparison.
        let mut lo = 0usize;
        let mut hi = self.directory.len();
        while lo + 1 < hi {
            let mid = (lo + hi) / 2;
            let entry = self.directory[mid];
            let first_hw = self.read_first_headword(&entry)?;
            let first_str = std::str::from_utf8(&first_hw)
                .map_err(|_| Error::Malformed("headword not UTF-8".into()))?;
            let first_lower = to_lowercase_bytes(first_str);
            if first_lower.as_slice() <= needle_lower.as_slice() {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let block = self.directory[lo];
        self.scan_block(&block, headword, &needle_lower)
    }

    fn read_first_headword(&mut self, d: &DirEntry) -> Result<Vec<u8>> {
        self.reader
            .seek(SeekFrom::Start(d.first_headword_pool_offset))?;
        let mut buf = vec![0u8; d.first_headword_len as usize];
        primitive::read_exact(&mut self.reader, &mut buf)?;
        Ok(buf)
    }

    fn scan_block(
        &mut self,
        block: &DirEntry,
        needle: &str,
        needle_lower: &[u8],
    ) -> Result<Option<(u32, u64, u64)>> {
        self.reader
            .seek(SeekFrom::Start(block.first_headword_offset))?;
        let mut block_bytes = vec![0u8; 65536]; // blocks are small; read a chunk
        let read = self.reader.read(&mut block_bytes)?;
        block_bytes.truncate(read);
        let mut r = BoundedReader::new(&block_bytes);
        let mut prev: Vec<u8> = Vec::new();
        for _ in 0..block.block_entry_count {
            let shared = match primitive::read_varint(&mut r) {
                Ok(v) => v as usize,
                Err(_) => return Ok(None),
            };
            let suffix_len = match primitive::read_varint(&mut r) {
                Ok(v) => v as usize,
                Err(_) => return Ok(None),
            };
            let suffix = match r.read_bytes(suffix_len) {
                Ok(s) => s,
                Err(_) => return Ok(None),
            };
            let data_block_id = match primitive::read_varint(&mut r) {
                Ok(v) => v as u32,
                Err(_) => return Ok(None),
            };
            let offset = match primitive::read_varint(&mut r) {
                Ok(v) => v,
                Err(_) => return Ok(None),
            };
            let size = match primitive::read_varint(&mut r) {
                Ok(v) => v,
                Err(_) => return Ok(None),
            };
            let mut hw = prev[..shared.min(prev.len())].to_vec();
            hw.extend_from_slice(suffix);
            // Case-folded comparison for ordering, case-sensitive for exact match.
            let hw_str = match std::str::from_utf8(&hw) {
                Ok(s) => s,
                Err(_) => return Ok(None),
            };
            let hw_lower = to_lowercase_bytes(hw_str);
            if hw_lower == needle_lower && hw.as_slice() == needle.as_bytes() {
                return Ok(Some((data_block_id, offset, size)));
            }
            if hw_lower.as_slice() > needle_lower {
                return Ok(None);
            }
            prev = hw;
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_loc(hw: &str, block: u32, off: u64, sz: u64) -> EntryLocation {
        EntryLocation {
            headword: hw.into(),
            data_block_id: block,
            offset: off,
            size: sz,
        }
    }

    #[test]
    fn index_basic_lookup() {
        let locs: Vec<EntryLocation> = ["apple", "banana", "cherry", "date", "egg"]
            .iter()
            .enumerate()
            .map(|(i, hw)| make_loc(hw, 0, i as u64 * 10, 10))
            .collect();
        let idx = build_index(&locs, 1).unwrap();
        let r = IndexReader::from_bytes(idx.bytes).unwrap();
        assert_eq!(r.lookup("cherry"), Some((0, 20, 10)));
        assert_eq!(r.lookup("apple"), Some((0, 0, 10)));
        assert_eq!(r.lookup("egg"), Some((0, 40, 10)));
        assert_eq!(r.lookup("missing"), None);
    }

    #[test]
    fn index_unicode_lookup() {
        let locs: Vec<EntryLocation> = ["あ", "い", "う", "え", "お"]
            .iter()
            .enumerate()
            .map(|(i, hw)| make_loc(hw, 0, i as u64 * 10, 10))
            .collect();
        let idx = build_index(&locs, 1).unwrap();
        let r = IndexReader::from_bytes(idx.bytes).unwrap();
        assert_eq!(r.lookup("う"), Some((0, 20, 10)));
        assert_eq!(r.lookup("あ"), Some((0, 0, 10)));
        assert_eq!(r.lookup("か"), None);
    }

    #[test]
    fn index_multiple_blocks() {
        // 300 entries → 2 blocks (256 + 44).
        let locs: Vec<EntryLocation> = (0..300)
            .map(|i| make_loc(&format!("hw{:04}", i), 0, i as u64, 1))
            .collect();
        let idx = build_index(&locs, 1).unwrap();
        assert_eq!(idx.header.block_count, 2);
        let r = IndexReader::from_bytes(idx.bytes).unwrap();
        assert_eq!(r.lookup("hw0000"), Some((0, 0, 1)));
        assert_eq!(r.lookup("hw0255"), Some((0, 255, 1)));
        assert_eq!(r.lookup("hw0256"), Some((0, 256, 1)));
        assert_eq!(r.lookup("hw0299"), Some((0, 299, 1)));
        assert_eq!(r.lookup("hw0300"), None);
        let locations = r.collect_locations().unwrap();
        assert_eq!(locations.len(), 300);
        assert_eq!(locations[256].headword, "hw0256");
        assert_eq!(locations[256].offset, 256);
    }

    #[test]
    fn index_empty() {
        let idx = build_index(&[], 0).unwrap();
        let r = IndexReader::from_bytes(idx.bytes).unwrap();
        assert_eq!(r.lookup("anything"), None);
    }

    #[test]
    fn index_rejects_unsorted() {
        let locs = vec![make_loc("banana", 0, 0, 0), make_loc("apple", 0, 1, 0)];
        assert!(build_index(&locs, 1).is_err());
    }

    #[test]
    fn index_rejects_duplicate() {
        let locs = vec![make_loc("apple", 0, 0, 0), make_loc("apple", 0, 1, 0)];
        assert!(build_index(&locs, 1).is_err());
    }
}
