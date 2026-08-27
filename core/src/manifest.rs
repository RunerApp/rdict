//! `manifest.json` serialization and validation.

use crate::error::{Error, Result};
use crate::model::{MediaCompression, Pack, PackMetadata};
use crate::strings::{PoolKind, StringPools};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub format_version: String,
    pub pack: PackJson,
    pub index: IndexJson,
    pub strings_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos_table: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang_table: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pron_kind_table: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_kind_table: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_table: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_type_table: Option<Vec<String>>,
    pub data: DataJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub morph: Option<MorphJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<TagsJson>,
    pub media: MediaJson,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackJson {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source_lang: String,
    pub target_langs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexJson {
    pub headword_file: String,
    pub entry_count: u32,
    pub block_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataJson {
    pub block_max_uncompressed: u32,
    pub compression: String,
    pub zstd_level: i32,
    pub block_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphJson {
    pub index_file: String,
    pub has_index: bool,
    pub key_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsJson {
    pub index_file: String,
    pub has_index: bool,
    pub tag_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaJson {
    pub hash_algo: String,
    pub total_size: u64,
    pub dedup: bool,
}

/// Current spec version this crate implements.
pub const FORMAT_VERSION: &str = "0.1.0";

/// Computed statistics used to build a manifest.
pub struct PackStats {
    pub block_count: u32,
    pub entry_count: u32,
    pub front_coding_block_count: u32,
    pub media_total_size: u64,
    pub has_morph_index: bool,
    pub morph_key_count: u32,
    pub has_tag_index: bool,
    pub tag_count: u32,
}

impl Manifest {
    /// Build a manifest from a pack, pools, and computed stats.
    pub fn from_pack(pack: &Pack, pools: &StringPools, stats: &PackStats) -> Self {
        let s = stats;
        let m = &pack.metadata;
        let pool_table = |kind: PoolKind| -> Option<Vec<String>> {
            let seg = pools.segment(kind);
            if seg.is_empty() {
                None
            } else {
                Some(seg.to_vec())
            }
        };

        Self {
            format: "rdict".into(),
            format_version: FORMAT_VERSION.into(),
            pack: PackJson {
                id: m.id.clone(),
                name: m.name.clone(),
                version: m.version.clone(),
                source_lang: m.source_lang.clone(),
                target_langs: m.target_langs.clone(),
                author: m.author.clone(),
                license: m.license.clone(),
                created_at: m.created_at.clone(),
                description: m.description.clone(),
                cover: m.cover.clone(),
            },
            index: IndexJson {
                headword_file: "index/headword.idx".into(),
                entry_count: s.entry_count,
                block_count: s.front_coding_block_count,
            },
            strings_file: "index/strings.tbl".into(),
            pos_table: pool_table(PoolKind::Pos),
            lang_table: pool_table(PoolKind::Lang),
            pron_kind_table: pool_table(PoolKind::PronKind),
            form_kind_table: pool_table(PoolKind::FormKind),
            tag_table: pool_table(PoolKind::Tag),
            relation_type_table: pool_table(PoolKind::RelationType),
            data: DataJson {
                block_max_uncompressed: m.block_max_uncompressed,
                compression: "zstd".into(),
                zstd_level: m.zstd_level,
                block_count: s.block_count,
            },
            morph: if s.has_morph_index {
                Some(MorphJson {
                    index_file: "index/morph.idx".into(),
                    has_index: true,
                    key_count: s.morph_key_count,
                })
            } else {
                None
            },
            tags: if s.has_tag_index {
                Some(TagsJson {
                    index_file: "index/tag.idx".into(),
                    has_index: true,
                    tag_count: s.tag_count,
                })
            } else {
                None
            },
            media: MediaJson {
                hash_algo: "sha1".into(),
                total_size: s.media_total_size,
                dedup: true,
            },
        }
    }

    pub fn to_json(&self) -> Result<Vec<u8>> {
        serde_json::to_vec_pretty(self).map_err(Error::from)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(Error::from)
    }

    pub fn validate(&self) -> Result<()> {
        if self.format != "rdict" {
            return Err(Error::Validation(format!(
                "manifest.format must be 'rdict', got {}",
                self.format
            )));
        }
        // Per §9.2, before 1.0.0 each minor is a separate compatibility line.
        // This crate advertises support for 0.1.x.
        let v = semver::Version::parse(&self.format_version).map_err(|e| {
            Error::UnsupportedVersion(format!("bad format_version {}: {}", self.format_version, e))
        })?;
        if v.major != 0 || v.minor != 1 {
            return Err(Error::UnsupportedVersion(format!(
                "this reader supports 0.1.x, got {}",
                self.format_version
            )));
        }
        if self.data.compression != "zstd" {
            return Err(Error::Validation(format!(
                "only zstd compression supported, got {}",
                self.data.compression
            )));
        }
        if self.media.hash_algo != "sha1" {
            return Err(Error::Validation(format!(
                "only sha1 hash algo supported, got {}",
                self.media.hash_algo
            )));
        }
        Ok(())
    }
}

/// Extract a `PackMetadata` skeleton from a manifest (for readers that
/// need to expose metadata without a full Pack).
impl From<&Manifest> for PackMetadata {
    fn from(m: &Manifest) -> Self {
        PackMetadata {
            id: m.pack.id.clone(),
            name: m.pack.name.clone(),
            version: m.pack.version.clone(),
            source_lang: m.pack.source_lang.clone(),
            target_langs: m.pack.target_langs.clone(),
            author: m.pack.author.clone(),
            license: m.pack.license.clone(),
            created_at: m.pack.created_at.clone(),
            description: m.pack.description.clone(),
            cover: m.pack.cover.clone(),
            block_max_uncompressed: m.data.block_max_uncompressed,
            zstd_level: m.data.zstd_level,
        }
    }
}

/// Convert a MediaCompression to the manifest string.
pub fn compression_str(c: MediaCompression) -> &'static str {
    c.as_str()
}
