use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use rdict::{LookupEntry, RdictReader};
use serde_json::json;
use std::fs::File;
use std::sync::Mutex;

/// A reader for `.rdict` dictionary files.
#[napi]
pub struct Dictionary {
    reader: Mutex<RdictReader<File>>,
}

#[napi]
impl Dictionary {
    /// Open a `.rdict` file by path.
    #[napi(constructor)]
    pub fn new(path: String) -> napi::Result<Self> {
        let reader = RdictReader::open(&path)
            .map_err(|e| napi::Error::from_reason(format!("Failed to open: {e}")))?;
        Ok(Self {
            reader: Mutex::new(reader),
        })
    }

    /// Look up a headword. Returns the entry as a JSON string, or null
    /// if not found. Parse with `JSON.parse()` on the JS side.
    #[napi]
    pub fn lookup(&self, headword: String) -> napi::Result<Option<String>> {
        let mut reader = self
            .reader
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock: {e}")))?;
        match reader.lookup(&headword) {
            Ok(Some(LookupEntry::Decoded(entry))) => {
                let json = serde_json::to_string(&*entry)
                    .map_err(|e| napi::Error::from_reason(format!("Serialize: {e}")))?;
                Ok(Some(json))
            }
            Ok(Some(LookupEntry::Opaque { .. })) => Err(napi::Error::from_reason(
                "Entry is opaque (unknown format version)",
            )),
            Ok(None) => Ok(None),
            Err(e) => Err(napi::Error::from_reason(format!("Lookup: {e}"))),
        }
    }

    /// List all headwords in the dictionary (sorted).
    #[napi]
    pub fn list_headwords(&self) -> napi::Result<Vec<String>> {
        let reader = self
            .reader
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock: {e}")))?;
        reader
            .list_headwords()
            .map_err(|e| napi::Error::from_reason(format!("List: {e}")))
    }

    /// Case-insensitive prefix search. Returns up to `limit` headwords
    /// whose lowercased form starts with `prefix`. `limit <= 0` returns empty.
    #[napi]
    pub fn prefix(&self, prefix: String, limit: Option<u32>) -> napi::Result<Vec<String>> {
        let reader = self
            .reader
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock: {e}")))?;
        let lim = limit.unwrap_or(20) as usize;
        Ok(reader.prefix(&prefix, lim))
    }

    /// Get manifest info (name, languages, version, entry count) as JSON.
    #[napi]
    pub fn manifest(&self) -> napi::Result<String> {
        let reader = self
            .reader
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock: {e}")))?;
        let m = reader.manifest();
        let json = json!({
            "name": m.pack.name,
            "version": m.pack.version,
            "source_lang": m.pack.source_lang,
            "target_langs": m.pack.target_langs,
            "entry_count": m.index.entry_count,
            "block_count": m.data.block_count,
            "cover": m.pack.cover,
        });
        serde_json::to_string_pretty(&json)
            .map_err(|e| napi::Error::from_reason(format!("Serialize: {e}")))
    }

    /// Get the media manifest as a JSON string, or null if no media.
    #[napi]
    pub fn media_manifest(&self) -> napi::Result<Option<String>> {
        let mut reader = self
            .reader
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock: {e}")))?;
        match reader.media_manifest() {
            Ok(Some(manifest)) => {
                let json = serde_json::to_string_pretty(&manifest)
                    .map_err(|e| napi::Error::from_reason(format!("Serialize: {e}")))?;
                Ok(Some(json))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(napi::Error::from_reason(format!("Media manifest: {e}"))),
        }
    }

    /// Get media info (metadata only, no content read) by kind + hash.
    /// Returns a JSON string, or null if not found.
    #[napi]
    pub fn media_info(&self, kind: String, hash: String) -> napi::Result<Option<String>> {
        let mut reader = self
            .reader
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock: {e}")))?;
        match reader.media_info(&kind, &hash) {
            Ok(Some(entry)) => {
                let json = serde_json::to_string(&entry)
                    .map_err(|e| napi::Error::from_reason(format!("Serialize: {e}")))?;
                Ok(Some(json))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(napi::Error::from_reason(format!("Media info: {e}"))),
        }
    }

    /// Read a media file's bytes by kind + hash. Automatically
    /// decompresses zstd-compressed media. Returns a Buffer.
    #[napi]
    pub fn read_media(&self, kind: String, hash: String) -> napi::Result<Buffer> {
        let mut reader = self
            .reader
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock: {e}")))?;
        match reader.read_media(&kind, &hash) {
            Ok(bytes) => Ok(Buffer::from(bytes)),
            Err(e) => Err(napi::Error::from_reason(format!("Read media: {e}"))),
        }
    }

    /// Extract a media file to a file on disk via streaming. Creates
    /// parent directories and writes atomically (temp + rename). Returns
    /// the number of bytes written.
    #[napi]
    pub fn extract_media(
        &self,
        kind: String,
        hash: String,
        output_path: String,
    ) -> napi::Result<f64> {
        let mut reader = self
            .reader
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock: {e}")))?;
        reader
            .extract_media(&kind, &hash, std::path::Path::new(&output_path))
            .map(|n| n as f64)
            .map_err(|e| napi::Error::from_reason(format!("Extract media: {e}")))
    }

    /// Read the cover image bytes, if present. Returns a Buffer, or null
    /// if no cover.
    #[napi]
    pub fn read_cover(&self) -> napi::Result<Option<Buffer>> {
        let mut reader = self
            .reader
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock: {e}")))?;
        match reader.read_cover() {
            Ok(Some(bytes)) => Ok(Some(Buffer::from(bytes))),
            Ok(None) => Ok(None),
            Err(e) => Err(napi::Error::from_reason(format!("Read cover: {e}"))),
        }
    }
}
