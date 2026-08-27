//! Media manifest and path reconstruction (§7).

use crate::error::{Error, Result};
use crate::model::{MediaAsset, MediaCompression, MediaKind, MediaRef};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaManifest {
    pub hash_algo: String,
    pub entries: Vec<MediaManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaManifestEntry {
    pub hash: String,
    pub kind: String,
    pub ext: String,
    pub mime: String,
    pub compression: String,
    pub size: u64,
    pub uncompressed_size: u64,
}

impl MediaManifest {
    pub fn new() -> Self {
        Self {
            hash_algo: "sha1".into(),
            entries: Vec::new(),
        }
    }

    pub fn to_json(&self) -> Result<Vec<u8>> {
        serde_json::to_vec_pretty(self).map_err(Error::from)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(Error::from)
    }
}

impl Default for MediaManifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute the SHA-1 hash of media bytes.
pub fn sha1_hash(bytes: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    let mut out = [0u8; 20];
    out.copy_from_slice(&result);
    out
}

/// Build the media manifest and a dedup map from a list of assets.
/// The caller MUST store the same (possibly compressed) bytes via
/// `compress_asset_bytes()` so that `size` matches what's written.
/// Returns (manifest, map from (kind, hash) → asset index in `assets`).
pub fn build_manifest(
    assets: &[MediaAsset],
) -> (MediaManifest, BTreeMap<(MediaKind, [u8; 20]), usize>) {
    let mut manifest = MediaManifest::new();
    let mut map: BTreeMap<(MediaKind, [u8; 20]), usize> = BTreeMap::new();
    let mut deduped: Vec<&MediaAsset> = Vec::new();
    for asset in assets {
        // Hash is always computed on the original (uncompressed) bytes.
        let hash = sha1_hash(&asset.bytes);
        let key = (asset.kind, hash);
        if let std::collections::btree_map::Entry::Vacant(e) = map.entry(key) {
            e.insert(deduped.len());
            deduped.push(asset);
        }
    }
    for asset in &deduped {
        let hash = sha1_hash(&asset.bytes);
        let hash_hex = hex::encode(&hash);
        let uncompressed_size = asset.bytes.len() as u64;
        let stored_size = match asset.compression {
            MediaCompression::None => uncompressed_size,
            MediaCompression::Zstd => {
                // Compressed size — compute the actual compressed output.
                zstd::encode_all(asset.bytes.as_slice(), 0)
                    .map(|c| c.len() as u64)
                    .unwrap_or(uncompressed_size)
            }
        };
        manifest.entries.push(MediaManifestEntry {
            hash: hash_hex,
            kind: asset.kind.as_str().into(),
            ext: asset.ext.clone(),
            mime: asset.mime.clone(),
            compression: asset.compression.as_str().into(),
            size: stored_size,
            uncompressed_size,
        });
    }
    (manifest, map)
}

/// Return the bytes to store in the ZIP for an asset.
/// For `Zstd` compression, this compresses the original bytes.
/// For `None`, returns the original bytes unchanged.
pub fn compress_asset_bytes(asset: &MediaAsset) -> Vec<u8> {
    match asset.compression {
        MediaCompression::None => asset.bytes.clone(),
        MediaCompression::Zstd => {
            zstd::encode_all(asset.bytes.as_slice(), 0).unwrap_or_else(|_| asset.bytes.clone())
        }
    }
}

/// Reconstruct the ZIP member path for a media asset.
pub fn media_path(kind: MediaKind, hash: &[u8; 20], ext: &str) -> String {
    let hash_hex = hex::encode(hash);
    let hh = &hash_hex[..2];
    format!("media/{}/{}/{}.{}", kind.as_str(), hh, hash_hex, ext)
}

/// Look up a media ref in the manifest, returning its path and metadata.
pub fn resolve_ref<'a>(
    mref: &MediaRef,
    manifest: &'a MediaManifest,
) -> Result<(String, &'a MediaManifestEntry)> {
    let hash_hex = hex::encode(&mref.hash);
    for entry in &manifest.entries {
        if entry.hash == hash_hex && entry.kind == mref.kind.as_str() {
            let path = media_path(mref.kind, &mref.hash, &entry.ext);
            return Ok((path, entry));
        }
    }
    Err(Error::NotFound(format!(
        "media ref {}:{} not in manifest",
        mref.kind.as_str(),
        hash_hex
    )))
}

/// Validate that the manifest has no duplicate (kind, hash) pairs and
/// that all hashes are valid hex.
pub fn validate(manifest: &MediaManifest) -> Result<()> {
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for e in &manifest.entries {
        if e.hash.len() != 40 {
            return Err(Error::Validation(format!(
                "media manifest: hash {} is not 40 hex chars",
                e.hash
            )));
        }
        if hex::decode(&e.hash).is_err() {
            return Err(Error::Validation(format!(
                "media manifest: hash {} is not valid hex",
                e.hash
            )));
        }
        let key = (e.kind.clone(), e.hash.clone());
        if !seen.insert(key) {
            return Err(Error::Validation(format!(
                "media manifest: duplicate (kind, hash) = ({}, {})",
                e.kind, e.hash
            )));
        }
        if MediaKind::parse_str(&e.kind).is_none() {
            return Err(Error::Validation(format!(
                "media manifest: unknown kind {} (expected audio, image, or video)",
                e.kind
            )));
        }
        match e.compression.as_str() {
            "none" | "zstd" => {}
            other => {
                return Err(Error::Validation(format!(
                    "media manifest: unknown compression {} (expected none or zstd)",
                    other
                )));
            }
        }
    }
    Ok(())
}

/// Minimal hex encoder/decoder to avoid an extra dependency.
pub mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    pub fn decode(s: &str) -> std::result::Result<Vec<u8>, String> {
        if s.len() % 2 != 0 {
            return Err("odd length".into());
        }
        let mut out = Vec::with_capacity(s.len() / 2);
        let bytes = s.as_bytes();
        for i in (0..bytes.len()).step_by(2) {
            let hi = hex_digit(bytes[i]).map_err(|_| "bad hex digit".to_string())?;
            let lo = hex_digit(bytes[i + 1]).map_err(|_| "bad hex digit".to_string())?;
            out.push(hi << 4 | lo);
        }
        Ok(out)
    }

    fn hex_digit(b: u8) -> Result<u8, ()> {
        match b {
            b'0'..=b'9' => Ok(b - b'0'),
            b'a'..=b'f' => Ok(b - b'a' + 10),
            b'A'..=b'F' => Ok(b - b'A' + 10),
            _ => Err(()),
        }
    }
}

impl MediaKind {
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "audio" => Some(MediaKind::Audio),
            "image" => Some(MediaKind::Image),
            "video" => Some(MediaKind::Video),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_reconstruction() {
        let hash = [0xa3u8; 20];
        let path = media_path(MediaKind::Audio, &hash, "mp3");
        assert_eq!(
            path,
            "media/audio/a3/a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3.mp3"
        );
    }

    #[test]
    fn manifest_dedup() {
        let asset = MediaAsset {
            kind: MediaKind::Audio,
            ext: "mp3".into(),
            mime: "audio/mpeg".into(),
            compression: MediaCompression::None,
            bytes: vec![1, 2, 3],
            path: None,
        };
        let assets = vec![asset.clone(), asset];
        let (manifest, map) = build_manifest(&assets);
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(map.len(), 1);
    }
}
