//! `index/strings.tbl` — six independent string pools. Index 0 is the
//! implicit "unspecified" value in every pool.

use crate::Error;
use crate::primitive::{self, BoundedReader, ByteWriter, Result};
use std::collections::HashMap;

/// Magic header for `strings.tbl`.
pub const MAGIC: &[u8; 4] = b"RDST";

/// Pool segment order in the file, per §5.
pub const POOL_ORDER: [PoolKind; 6] = [
    PoolKind::Pos,
    PoolKind::Lang,
    PoolKind::PronKind,
    PoolKind::FormKind,
    PoolKind::Tag,
    PoolKind::RelationType,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoolKind {
    Pos,
    Lang,
    PronKind,
    FormKind,
    Tag,
    RelationType,
}

/// The six string pools. Each maps string → 1-based index (0 is
/// implicit "unspecified"). `strings.tbl` stores only the 1..N values.
#[derive(Debug, Clone, Default)]
pub struct StringPools {
    pub pos: Vec<String>,
    pub lang: Vec<String>,
    pub pron_kind: Vec<String>,
    pub form_kind: Vec<String>,
    pub tag: Vec<String>,
    pub relation_type: Vec<String>,
}

impl StringPools {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn segment(&self, kind: PoolKind) -> &[String] {
        match kind {
            PoolKind::Pos => &self.pos,
            PoolKind::Lang => &self.lang,
            PoolKind::PronKind => &self.pron_kind,
            PoolKind::FormKind => &self.form_kind,
            PoolKind::Tag => &self.tag,
            PoolKind::RelationType => &self.relation_type,
        }
    }

    pub fn segment_mut(&mut self, kind: PoolKind) -> &mut Vec<String> {
        match kind {
            PoolKind::Pos => &mut self.pos,
            PoolKind::Lang => &mut self.lang,
            PoolKind::PronKind => &mut self.pron_kind,
            PoolKind::FormKind => &mut self.form_kind,
            PoolKind::Tag => &mut self.tag,
            PoolKind::RelationType => &mut self.relation_type,
        }
    }

    /// Intern a string into the given pool, returning its 1-based index.
    /// Idempotent: returns the existing index if already present.
    pub fn intern(&mut self, kind: PoolKind, s: &str) -> Result<u16> {
        let seg = self.segment_mut(kind);
        if let Some(idx) = seg.iter().position(|x| x == s) {
            return Ok((idx + 1) as u16);
        }
        if seg.len() + 1 > u16::MAX as usize {
            return Err(Error::Validation(format!(
                "pool {:?} exceeds u16 limit",
                kind
            )));
        }
        seg.push(s.to_string());
        Ok(seg.len() as u16)
    }

    /// Look up a string's 1-based index in a pool, or 0 if absent.
    pub fn lookup(&self, kind: PoolKind, s: &str) -> u16 {
        self.segment(kind)
            .iter()
            .position(|x| x == s)
            .map(|i| (i + 1) as u16)
            .unwrap_or(0)
    }

    /// Resolve a 1-based index back to a string. Returns None for 0
    /// (unspecified) or out-of-range.
    pub fn resolve(&self, kind: PoolKind, idx: u16) -> Option<&str> {
        if idx == 0 {
            return None;
        }
        self.segment(kind)
            .get((idx - 1) as usize)
            .map(|s| s.as_str())
    }

    /// Build a reverse lookup map for one pool.
    pub fn index_map(&self, kind: PoolKind) -> HashMap<String, u16> {
        self.segment(kind)
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), (i + 1) as u16))
            .collect()
    }

    /// Serialize to `strings.tbl` bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ByteWriter::with_capacity(1024);
        w.buf.extend_from_slice(MAGIC);
        primitive::write_u16(&mut w, 1).unwrap();
        // counts include the implicit index 0, so +1 per segment
        for kind in POOL_ORDER {
            let count = self.segment(kind).len() as u32 + 1;
            primitive::write_u32(&mut w, count).unwrap();
        }
        for kind in POOL_ORDER {
            for s in self.segment(kind) {
                primitive::write_str(&mut w, s).unwrap();
            }
        }
        w.into_bytes()
    }

    /// Deserialize from `strings.tbl` bytes.
    pub fn decode(buf: &[u8]) -> Result<Self> {
        let mut r = BoundedReader::new(buf);
        let mut magic = [0u8; 4];
        primitive::read_exact(&mut r, &mut magic)?;
        if &magic != MAGIC {
            return Err(Error::Malformed("strings.tbl: bad magic".into()));
        }
        let version = primitive::read_u16(&mut r)?;
        if version != 1 {
            return Err(Error::UnsupportedVersion(format!(
                "strings.tbl version {}",
                version
            )));
        }
        let mut counts = [0u32; 6];
        for count in counts.iter_mut() {
            *count = primitive::read_u32(&mut r)?;
        }
        let mut pools = StringPools::new();
        for (i, kind) in POOL_ORDER.iter().enumerate() {
            let count = counts[i];
            if !(1..=65536).contains(&count) {
                return Err(Error::Malformed(format!(
                    "strings.tbl: pool {:?} count {} out of range",
                    kind, count
                )));
            }
            let n = (count - 1) as usize;
            let seg = pools.segment_mut(*kind);
            seg.reserve(n);
            for _ in 0..n {
                seg.push(primitive::read_str(&mut r)?);
            }
        }
        Ok(pools)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_roundtrip() {
        let mut p = StringPools::new();
        p.intern(PoolKind::Pos, "n").unwrap();
        p.intern(PoolKind::Pos, "v").unwrap();
        p.intern(PoolKind::Lang, "en").unwrap();
        p.intern(PoolKind::Tag, "exam:IELTS").unwrap();
        p.intern(PoolKind::Pos, "n").unwrap(); // idempotent

        assert_eq!(p.lookup(PoolKind::Pos, "n"), 1);
        assert_eq!(p.lookup(PoolKind::Pos, "v"), 2);
        assert_eq!(p.lookup(PoolKind::Pos, "missing"), 0);
        assert_eq!(p.resolve(PoolKind::Pos, 1), Some("n"));
        assert_eq!(p.resolve(PoolKind::Pos, 0), None);

        let bytes = p.encode();
        let p2 = StringPools::decode(&bytes).unwrap();
        assert_eq!(p2.segment(PoolKind::Pos), p.segment(PoolKind::Pos));
        assert_eq!(p2.segment(PoolKind::Lang), p.segment(PoolKind::Lang));
        assert_eq!(p2.segment(PoolKind::Tag), p.segment(PoolKind::Tag));
    }
}
