//! Primitive encoding: canonical unsigned LEB128 varints, little-endian
//! fixed-width integers, length-prefixed UTF-8 strings, and a bounded
//! reader that cannot read past its enclosing scope.

use crate::error::Error;
use std::io::{self, Read, Write};

pub type Result<T> = crate::Result<T, Error>;
/// Maximum value a varint can represent (u64).
pub const VARINT_MAX: u64 = u64::MAX;

/// Encode an unsigned integer as canonical LEB128.
pub fn write_varint<W: Write>(w: &mut W, mut value: u64) -> Result<()> {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        w.write_all(&[byte])?;
        if value == 0 {
            break;
        }
    }
    Ok(())
}

/// Decode an unsigned LEB128 varint. Rejects non-canonical encodings
/// (trailing zero-extension bytes, overflow beyond u64).
pub fn read_varint<R: Read>(r: &mut R) -> Result<u64> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    let mut byte;
    loop {
        let mut buf = [0u8; 1];
        match r.read(&mut buf) {
            Ok(0) => return Err(Error::Malformed("varint truncated".into())),
            Ok(_) => byte = buf[0],
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
        if shift >= 64 {
            return Err(Error::Malformed("varint too long".into()));
        }
        let chunk = (byte & 0x7F) as u64;
        if shift == 63 && (byte & 0x80 != 0) {
            return Err(Error::Malformed("varint overflow".into()));
        }
        result |= chunk << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            // Canonical form check: the last chunk must not be zero when
            // a previous continuation byte was present, except for value 0.
            if shift > 7 && chunk == 0 {
                return Err(Error::Malformed("non-canonical varint".into()));
            }
            break;
        }
    }
    Ok(result)
}

/// Write a little-endian u16.
pub fn write_u16<W: Write>(w: &mut W, v: u16) -> Result<()> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

/// Write a little-endian u32.
pub fn write_u32<W: Write>(w: &mut W, v: u32) -> Result<()> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

/// Write a little-endian u64.
pub fn write_u64<W: Write>(w: &mut W, v: u64) -> Result<()> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

/// Read a little-endian u16.
pub fn read_u16<R: Read>(r: &mut R) -> Result<u16> {
    let mut buf = [0u8; 2];
    read_exact(r, &mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

/// Read a little-endian u32.
pub fn read_u32<R: Read>(r: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    read_exact(r, &mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

/// Read a little-endian u64.
pub fn read_u64<R: Read>(r: &mut R) -> Result<u64> {
    let mut buf = [0u8; 8];
    read_exact(r, &mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

/// Read a single u8.
pub fn read_u8<R: Read>(r: &mut R) -> Result<u8> {
    let mut buf = [0u8; 1];
    read_exact(r, &mut buf)?;
    Ok(buf[0])
}

/// Write a single u8.
pub fn write_u8<W: Write>(w: &mut W, v: u8) -> Result<()> {
    w.write_all(&[v])?;
    Ok(())
}

/// Read exactly `buf.len()` bytes or return a malformed error.
pub fn read_exact<R: Read>(r: &mut R, buf: &mut [u8]) -> Result<()> {
    r.read_exact(buf).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            Error::Malformed("unexpected end of input".into())
        } else {
            Error::from(e)
        }
    })
}

/// Write a length-prefixed UTF-8 string. The length is a varint.
pub fn write_str<W: Write>(w: &mut W, s: &str) -> Result<()> {
    write_varint(w, s.len() as u64)?;
    w.write_all(s.as_bytes())?;
    Ok(())
}

/// Read a length-prefixed UTF-8 string. Validates UTF-8.
pub fn read_str<R: Read>(r: &mut R) -> Result<String> {
    let len = read_varint(r)? as usize;
    let mut buf = vec![0u8; len];
    read_exact(r, &mut buf)?;
    String::from_utf8(buf).map_err(|e| Error::Malformed(format!("invalid UTF-8: {e}")))
}

/// Read a length-prefixed byte vector.
pub fn read_bytes<R: Read>(r: &mut R) -> Result<Vec<u8>> {
    let len = read_varint(r)? as usize;
    let mut buf = vec![0u8; len];
    read_exact(r, &mut buf)?;
    Ok(buf)
}

/// Write a length-prefixed byte vector.
pub fn write_bytes<W: Write>(w: &mut W, b: &[u8]) -> Result<()> {
    write_varint(w, b.len() as u64)?;
    w.write_all(b)?;
    Ok(())
}

/// A reader bounded to a fixed number of bytes. Any read past the bound
/// returns a malformed error.
pub struct BoundedReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> BoundedReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn skip(&mut self, n: usize) -> Result<()> {
        if self.pos + n > self.buf.len() {
            return Err(Error::Malformed("skip past end of bounded slice".into()));
        }
        self.pos += n;
        Ok(())
    }

    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.pos + n > self.buf.len() {
            return Err(Error::Malformed("read past end of bounded slice".into()));
        }
        let slice = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    pub fn remaining_slice(&self) -> &'a [u8] {
        &self.buf[self.pos..]
    }
}

impl<'a> Read for BoundedReader<'a> {
    fn read(&mut self, dst: &mut [u8]) -> io::Result<usize> {
        let avail = self.buf.len() - self.pos;
        let n = dst.len().min(avail);
        if n == 0 {
            return Ok(0);
        }
        dst[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// A writer that accumulates bytes into a Vec. Used for encoding AST
/// records before they are placed into blocks.
pub struct ByteWriter {
    pub buf: Vec<u8>,
}

impl ByteWriter {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

impl Default for ByteWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl Write for ByteWriter {
    fn write(&mut self, src: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(src);
        Ok(src.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Encode a varint into a fresh Vec.
pub fn varint_bytes(mut value: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(10);
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_roundtrip() {
        for v in [0u64, 1, 127, 128, 255, 16384, u32::MAX as u64, u64::MAX] {
            let mut w = ByteWriter::new();
            write_varint(&mut w, v).unwrap();
            let mut r = BoundedReader::new(&w.buf);
            assert_eq!(read_varint(&mut r).unwrap(), v);
        }
    }

    #[test]
    fn varint_rejects_noncanonical() {
        // 0x80 0x00 is a non-canonical encoding of 0
        let mut r = BoundedReader::new(&[0x80, 0x00]);
        assert!(read_varint(&mut r).is_err());
    }

    #[test]
    fn varint_rejects_truncated() {
        let mut r = BoundedReader::new(&[0x80]);
        assert!(read_varint(&mut r).is_err());
    }

    #[test]
    fn str_roundtrip() {
        let mut w = ByteWriter::new();
        write_str(&mut w, "héllo wörld").unwrap();
        let mut r = BoundedReader::new(&w.buf);
        assert_eq!(read_str(&mut r).unwrap(), "héllo wörld");
    }
}
