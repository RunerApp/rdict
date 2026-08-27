//! ZIP container access (§2.2). All members use `store` method; the
//! `mimetype` member is first and uncompressed.

use crate::error::{Error, Result};
use std::io::{Read, Seek, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// The mimetype member's content.
pub const MIMETYPE: &[u8] = b"application/rdict";

/// A member to be written into the ZIP container.
pub struct Member {
    pub name: String,
    pub data: Vec<u8>,
}

/// Validate a member name: must not contain `.` or `..` path components,
/// must not be empty, must use `/` separators (not `\`).
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::Validation("empty member name".into()));
    }
    if name.contains('\\') {
        return Err(Error::Validation(format!(
            "member name contains backslash: {}",
            name
        )));
    }
    for comp in name.split('/') {
        if comp == "." || comp == ".." {
            return Err(Error::Validation(format!(
                "member name contains unsafe component: {}",
                name
            )));
        }
        if comp.is_empty() && name != "/" {
            // allow trailing slash for directory entries, but we don't use those
        }
    }
    Ok(())
}

/// Write members into a ZIP archive. The first member MUST be `mimetype`
/// with `store` compression and no extra fields, per §2.2 rule 5.
pub fn write_zip<W: Write + Seek>(members: Vec<Member>, w: W) -> Result<()> {
    // Validate all names first.
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for m in &members {
        validate_name(&m.name)?;
        if !seen.insert(m.name.as_str()) {
            return Err(Error::Validation(format!(
                "duplicate member name: {}",
                m.name
            )));
        }
    }
    // Verify mimetype is first if present.
    if !members.is_empty() && members[0].name != "mimetype" {
        return Err(Error::Validation("first member must be 'mimetype'".into()));
    }

    let mut zw = ZipWriter::new(w);
    let stored = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .large_file(true); // enable ZIP64 entries

    for m in members {
        if m.name == "mimetype" {
            // mimetype: stored, first, no extra fields, UTF-8 name.
            let opts = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .large_file(false);
            zw.start_file("mimetype", opts)?;
            zw.write_all(&m.data)?;
        } else {
            zw.start_file(&m.name, stored)?;
            zw.write_all(&m.data)?;
        }
    }
    zw.finish()?;
    Ok(())
}

/// A reader over a ZIP archive that can fetch member bytes on demand.
pub struct ZipContainer<R: Read + Seek> {
    archive: ZipArchive<R>,
}

impl<R: Read + Seek> ZipContainer<R> {
    pub fn new(reader: R) -> Result<Self> {
        let mut archive = ZipArchive::new(reader)?;
        // Validate: all members stored, names safe, mimetype first.
        if archive.is_empty() {
            return Err(Error::Malformed("empty zip archive".into()));
        }
        // Check mimetype first.
        let first_name = archive.by_index_raw(0)?.name().to_string();
        if first_name != "mimetype" {
            return Err(Error::Malformed(format!(
                "first member must be 'mimetype', got '{}'",
                first_name
            )));
        }
        for i in 0..archive.len() {
            let entry = archive.by_index_raw(i)?;
            let name = entry.name().to_string();
            validate_name(&name)?;
            if entry.compression() != CompressionMethod::Stored {
                return Err(Error::Malformed(format!(
                    "member {} is not stored (got {:?})",
                    name,
                    entry.compression()
                )));
            }
        }
        Ok(Self { archive })
    }

    /// Read a member's bytes by name.
    pub fn read_member(&mut self, name: &str) -> Result<Vec<u8>> {
        validate_name(name)?;
        let mut entry = self.archive.by_name(name)?;
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        Ok(buf)
    }

    /// Stream a member's bytes directly to a writer (e.g. a file) without
    /// buffering the entire content in memory. Returns bytes written.
    pub fn stream_member<W: Write>(&mut self, name: &str, w: &mut W) -> Result<u64> {
        validate_name(name)?;
        let mut entry = self.archive.by_name(name)?;
        let n = std::io::copy(&mut entry, w)?;
        Ok(n)
    }

    /// Stream a member through a zstd decoder to a writer. The member's
    /// compressed bytes are decompressed on the fly without buffering the
    /// full decompressed content in memory. Returns bytes written.
    pub fn stream_member_zstd<W: Write>(&mut self, name: &str, w: &mut W) -> Result<u64> {
        validate_name(name)?;
        let entry = self.archive.by_name(name)?;
        let mut decoder = zstd::stream::read::Decoder::new(entry)
            .map_err(|e| Error::Malformed(format!("zstd decoder init: {e}")))?;
        let n = std::io::copy(&mut decoder, w)?;
        Ok(n)
    }

    /// Check if a member exists.
    pub fn has_member(&mut self, name: &str) -> bool {
        self.archive.by_name(name).is_ok()
    }

    /// Read the mimetype member (should be `application/rdict`).
    pub fn read_mimetype(&mut self) -> Result<Vec<u8>> {
        self.read_member("mimetype")
    }

    /// List all member names.
    pub fn member_names(&mut self) -> Vec<String> {
        let len = self.archive.len();
        (0..len)
            .map(|i| {
                self.archive
                    .by_index_raw(i)
                    .map(|e| e.name().to_string())
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Get the underlying archive (for advanced access).
    pub fn archive(&mut self) -> &mut ZipArchive<R> {
        &mut self.archive
    }
}

/// Verify the mimetype member equals `application/rdict`.
pub fn check_mimetype(bytes: &[u8]) -> Result<()> {
    if bytes != MIMETYPE {
        return Err(Error::Malformed(format!(
            "bad mimetype: expected 'application/rdict', got {:?}",
            String::from_utf8_lossy(bytes)
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn write_and_read_basic() {
        let members = vec![
            Member {
                name: "mimetype".into(),
                data: MIMETYPE.to_vec(),
            },
            Member {
                name: "data/hello.txt".into(),
                data: b"hello world".to_vec(),
            },
        ];
        let mut buf = Vec::new();
        write_zip(members, Cursor::new(&mut buf)).unwrap();
        let mut c = ZipContainer::new(Cursor::new(&buf)).unwrap();
        assert_eq!(c.read_mimetype().unwrap(), MIMETYPE);
        assert_eq!(c.read_member("data/hello.txt").unwrap(), b"hello world");
    }

    #[test]
    fn reject_duplicate_names() {
        let members = vec![
            Member {
                name: "mimetype".into(),
                data: MIMETYPE.to_vec(),
            },
            Member {
                name: "dup.txt".into(),
                data: b"a".to_vec(),
            },
            Member {
                name: "dup.txt".into(),
                data: b"b".to_vec(),
            },
        ];
        let mut buf = Vec::new();
        assert!(write_zip(members, Cursor::new(&mut buf)).is_err());
    }

    #[test]
    fn reject_dotdot_path() {
        let members = vec![
            Member {
                name: "mimetype".into(),
                data: MIMETYPE.to_vec(),
            },
            Member {
                name: "../escape.txt".into(),
                data: b"x".to_vec(),
            },
        ];
        let mut buf = Vec::new();
        assert!(write_zip(members, Cursor::new(&mut buf)).is_err());
    }

    #[test]
    fn reject_mimetype_not_first() {
        let members = vec![Member {
            name: "other.txt".into(),
            data: b"x".to_vec(),
        }];
        let mut buf = Vec::new();
        assert!(write_zip(members, Cursor::new(&mut buf)).is_err());
    }
}
