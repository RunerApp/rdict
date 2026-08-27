//! `index/tag.idx` and `index/morph.idx` — inverted indexes with
//! delta-encoded posting lists (§4.6, §4.7).

use crate::error::{Error, Result};
use crate::primitive::{self, BoundedReader, ByteWriter};

/// Magic for `tag.idx`.
pub const TAG_MAGIC: &[u8; 4] = b"RDTI";

/// Magic for `morph.idx`.
pub const MORPH_MAGIC: &[u8; 4] = b"RDMI";

/// 16-byte header.
#[derive(Debug, Clone)]
pub struct InvertedHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub key_count: u32,
}

impl InvertedHeader {
    pub fn write_to(&self, w: &mut ByteWriter) -> Result<()> {
        w.buf.extend_from_slice(&self.magic);
        primitive::write_u16(w, self.version)?;
        primitive::write_u16(w, self.flags)?;
        primitive::write_u32(w, self.key_count)?;
        primitive::write_u32(w, 0)?;
        Ok(())
    }

    pub fn read_from<R: std::io::Read>(r: &mut R) -> Result<Self> {
        let mut magic = [0u8; 4];
        primitive::read_exact(r, &mut magic)?;
        let version = primitive::read_u16(r)?;
        let flags = primitive::read_u16(r)?;
        let key_count = primitive::read_u32(r)?;
        let reserved = primitive::read_u32(r)?;
        if reserved != 0 {
            return Err(Error::Malformed(
                "inverted index: reserved must be 0".into(),
            ));
        }
        Ok(Self {
            magic,
            version,
            flags,
            key_count,
        })
    }
}

/// A posting list directory entry for `tag.idx` (keys are u16 tag ids).
#[derive(Debug, Clone, Copy)]
pub struct TagDirEntry {
    pub tag_id: u16,
    pub posting_offset: u64,
    pub posting_count: u32,
}

/// A posting list directory entry for `morph.idx` (keys are variable-length
/// strings).
#[derive(Debug, Clone)]
pub struct MorphDirEntry {
    pub key: String,
    pub posting_offset: u64,
    pub posting_count: u32,
}

/// Encode a `tag.idx` file. `postings[i]` is the sorted list of entry ids
/// for tag id `i+1` (tag ids start at 1).
pub fn encode_tag_index(entries: &[(u16, Vec<u32>)]) -> Result<Vec<u8>> {
    let mut w = ByteWriter::new();
    let header = InvertedHeader {
        magic: *TAG_MAGIC,
        version: 1,
        flags: 0,
        key_count: entries.len() as u32,
    };
    header.write_to(&mut w)?;

    // Encode posting lists first to compute offsets.
    let mut body = ByteWriter::new();
    let mut offsets: Vec<(u64, u32)> = Vec::with_capacity(entries.len());
    for (_, ids) in entries {
        let offset = body.len() as u64;
        encode_posting_list(&mut body, ids)?;
        offsets.push((offset, ids.len() as u32));
    }

    // Directory: tag_id (u16), pad (u16), posting_offset (u64), posting_count (u32).
    for ((tag_id, _), (offset, count)) in entries.iter().zip(offsets.iter()) {
        primitive::write_u16(&mut w, *tag_id)?;
        primitive::write_u16(&mut w, 0)?;
        primitive::write_u64(&mut w, *offset)?;
        primitive::write_u32(&mut w, *count)?;
    }

    w.buf.extend_from_slice(&body.buf);
    Ok(w.into_bytes())
}

/// Encode a `morph.idx` file. `entries[i]` is the sorted list of entry
/// ids for the i-th key string.
pub fn encode_morph_index(entries: &[(String, Vec<u32>)]) -> Result<Vec<u8>> {
    let mut w = ByteWriter::new();
    let header = InvertedHeader {
        magic: *MORPH_MAGIC,
        version: 1,
        flags: 0,
        key_count: entries.len() as u32,
    };
    header.write_to(&mut w)?;

    let mut body = ByteWriter::new();
    let mut offsets: Vec<(u64, u32)> = Vec::with_capacity(entries.len());
    for (_, ids) in entries {
        let offset = body.len() as u64;
        encode_posting_list(&mut body, ids)?;
        offsets.push((offset, ids.len() as u32));
    }

    // Directory: key_len (varint), key_bytes, posting_offset (u64), posting_count (u32).
    for ((key, _), (offset, count)) in entries.iter().zip(offsets.iter()) {
        primitive::write_varint(&mut w, key.len() as u64)?;
        w.buf.extend_from_slice(key.as_bytes());
        primitive::write_u64(&mut w, *offset)?;
        primitive::write_u32(&mut w, *count)?;
    }

    w.buf.extend_from_slice(&body.buf);
    Ok(w.into_bytes())
}

fn encode_posting_list(w: &mut ByteWriter, ids: &[u32]) -> Result<()> {
    let mut prev: u32 = 0;
    for &id in ids {
        let delta = id - prev;
        primitive::write_varint(w, delta as u64)?;
        prev = id;
    }
    Ok(())
}

/// Decode a `tag.idx` file into (tag_id, entry_ids) pairs.
pub fn decode_tag_index(bytes: &[u8]) -> Result<Vec<(u16, Vec<u32>)>> {
    let mut r = BoundedReader::new(bytes);
    let header = InvertedHeader::read_from(&mut r)?;
    if header.magic != *TAG_MAGIC {
        return Err(Error::Malformed("tag.idx: bad magic".into()));
    }
    if header.version != 1 {
        return Err(Error::UnsupportedVersion(format!(
            "tag.idx version {}",
            header.version
        )));
    }
    let body_start = 16 + (16 * header.key_count as usize);
    let mut out = Vec::with_capacity(header.key_count as usize);
    for _ in 0..header.key_count {
        let tag_id = primitive::read_u16(&mut r)?;
        let pad = primitive::read_u16(&mut r)?;
        if pad != 0 {
            return Err(Error::Malformed("tag.idx: dir pad must be 0".into()));
        }
        let posting_offset = primitive::read_u64(&mut r)?;
        let posting_count = primitive::read_u32(&mut r)?;
        let ids = decode_posting_list(&bytes[body_start..], posting_offset, posting_count)?;
        out.push((tag_id, ids));
    }
    Ok(out)
}

/// Decode a `morph.idx` file into (key, entry_ids) pairs.
pub fn decode_morph_index(bytes: &[u8]) -> Result<Vec<(String, Vec<u32>)>> {
    let mut r = BoundedReader::new(bytes);
    let header = InvertedHeader::read_from(&mut r)?;
    if header.magic != *MORPH_MAGIC {
        return Err(Error::Malformed("morph.idx: bad magic".into()));
    }
    if header.version != 1 {
        return Err(Error::UnsupportedVersion(format!(
            "morph.idx version {}",
            header.version
        )));
    }
    // The body starts after the variable-length directory, but the
    // directory itself is variable-length. We track the body start as
    // the max of all (posting_offset + posting_list_size) — but since
    // posting lists are sequential, we can just track the directory end.
    let mut dir_entries: Vec<(String, u64, u32)> = Vec::with_capacity(header.key_count as usize);
    let mut dir_end = 16usize;
    for _ in 0..header.key_count {
        let key_len = primitive::read_varint(&mut r)? as usize;
        let key_bytes = r.read_bytes(key_len)?;
        let key = String::from_utf8(key_bytes.to_vec())
            .map_err(|e| Error::Malformed(format!("morph.idx: invalid key UTF-8: {e}")))?;
        let posting_offset = primitive::read_u64(&mut r)?;
        let posting_count = primitive::read_u32(&mut r)?;
        dir_end = 16 + r.position();
        dir_entries.push((key, posting_offset, posting_count));
    }
    let _ = dir_end; // body starts immediately after the directory
    let mut out = Vec::with_capacity(dir_entries.len());
    for (key, offset, count) in dir_entries {
        // Posting offset is relative to the body start. The body starts
        // at the current reader position after the directory.
        let body_start = r.position();
        let ids = decode_posting_list(&bytes[body_start..], offset, count)?;
        out.push((key, ids));
    }
    Ok(out)
}

fn decode_posting_list(body: &[u8], offset: u64, count: u32) -> Result<Vec<u32>> {
    let start = offset as usize;
    if start > body.len() {
        return Err(Error::Malformed("posting list offset out of bounds".into()));
    }
    let mut r = BoundedReader::new(&body[start..]);
    let mut ids = Vec::with_capacity(count as usize);
    let mut prev: u32 = 0;
    for _ in 0..count {
        let delta = primitive::read_varint(&mut r)? as u32;
        let id = prev
            .checked_add(delta)
            .ok_or_else(|| Error::Malformed("posting list entry id overflow".into()))?;
        ids.push(id);
        prev = id;
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_index_roundtrip() {
        let entries = vec![(1u16, vec![0u32, 5, 10, 20]), (2u16, vec![1u32, 3, 7])];
        let bytes = encode_tag_index(&entries).unwrap();
        let decoded = decode_tag_index(&bytes).unwrap();
        assert_eq!(decoded, entries);
    }

    #[test]
    fn morph_index_roundtrip() {
        let entries = vec![
            ("ud:ConjugationType=Godan".to_string(), vec![0u32, 5, 10]),
            ("ud:Aspect=Perf".to_string(), vec![1u32, 3, 7, 100]),
        ];
        let bytes = encode_morph_index(&entries).unwrap();
        let decoded = decode_morph_index(&bytes).unwrap();
        assert_eq!(decoded, entries);
    }
}
