//! Pack writer: validates the model, interns pools, encodes AST entries,
//! splits into zstd blocks, builds the headword index and optional
//! indexes, then assembles the ZIP container.

use crate::ast;
use crate::blocks;
use crate::container::{self, Member};
use crate::error::Result;
use crate::index::{self, HeadwordIndex};
use crate::manifest::Manifest;
use crate::media;
use crate::model::{self, Pack, validate_pack};
use crate::postings;
use crate::strings::{PoolKind, StringPools};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Seek, Write};
use std::path::Path;

/// A writer that produces `.rdict` files.
pub struct RdictWriter<W: Write + Seek> {
    _phantom: std::marker::PhantomData<W>,
}

impl RdictWriter<File> {
    /// Create a new `.rdict` file at the given path.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let _file = File::create(path)?;
        Ok(Self {
            _phantom: std::marker::PhantomData,
        })
    }
}

impl<W: Write + Seek> RdictWriter<W> {
    /// Write a complete pack to the given writer.
    pub fn write_pack(writer: W, pack: &Pack) -> Result<()> {
        // 1. Validate the model.
        validate_pack(pack)?;

        // 2. Collect and intern all string pools.
        let mut pools = StringPools::new();
        for entry in &pack.entries {
            for tag in &entry.tags {
                pools.intern(PoolKind::Tag, tag)?;
            }
            for pron in &entry.pron {
                intern_pron(&mut pools, pron)?;
            }
            for ety in &entry.etymologies {
                for sense in &ety.senses {
                    if let Some(pos) = &sense.pos {
                        pools.intern(PoolKind::Pos, pos)?;
                    }
                    for tr in &sense.translations {
                        if let Some(lang) = &tr.lang {
                            pools.intern(PoolKind::Lang, lang)?;
                        }
                        for pron in &tr.pron {
                            intern_pron(&mut pools, pron)?;
                        }
                    }
                    for form in &sense.forms {
                        if let Some(kind) = &form.kind {
                            pools.intern(PoolKind::FormKind, kind)?;
                        }
                        for pron in &form.pron {
                            intern_pron(&mut pools, pron)?;
                        }
                    }
                    for pron in &sense.pron {
                        intern_pron(&mut pools, pron)?;
                    }
                }
            }
            for morph in &entry.morphology {
                if let Some(kind) = &morph.kind {
                    pools.intern(PoolKind::FormKind, kind)?;
                }
            }
            for rel in &entry.relations {
                if let Some(t) = &rel.type_ {
                    pools.intern(PoolKind::RelationType, t)?;
                }
            }
        }

        // 3. Sort entries by case-folded headword order (§4.2.1).
        let mut sorted_entries: Vec<&model::Entry> = pack.entries.iter().collect();
        sorted_entries.sort_by(|a, b| index::compare_headwords(&a.headword, &b.headword));

        // 4. Encode each entry's AST body.
        let mut encoded: Vec<(String, Vec<u8>)> = Vec::with_capacity(sorted_entries.len());
        for entry in &sorted_entries {
            let bytes = ast::encode_entry(entry, &pools)?;
            encoded.push((entry.headword.clone(), bytes));
        }

        // 5. Split into zstd blocks.
        let block_max = pack.metadata.block_max_uncompressed;
        let (blocks, locations) = blocks::split_into_blocks(encoded, block_max)?;
        let data_block_count = blocks.len() as u32;

        // 6. Build headword index.
        let headword_index = index::build_index(&locations, data_block_count)?;

        // 7. Build optional tag index (if entries have tags).
        let tag_index_bytes = build_tag_index(pack, &pools, &sorted_entries)?;
        let has_tag_index = tag_index_bytes.is_some();
        let tag_count = pools.segment(PoolKind::Tag).len() as u32;

        // 8. Build optional morph index (if entries have feats).
        let morph_index_bytes = build_morph_index(pack, &pools, &sorted_entries)?;
        let has_morph_index = morph_index_bytes.is_some();
        let morph_key_count = count_morph_keys(&sorted_entries);

        // 9. Build media manifest and dedup assets.
        let (media_manifest, media_map) = media::build_manifest(&pack.media);
        let media_total_size: u64 = media_manifest.entries.iter().map(|e| e.size).sum();
        let has_media = !pack.media.is_empty();

        // 10. Build manifest.json.
        let stats = crate::manifest::PackStats {
            block_count: data_block_count,
            entry_count: sorted_entries.len() as u32,
            front_coding_block_count: headword_index.header.block_count,
            media_total_size,
            has_morph_index,
            morph_key_count,
            has_tag_index,
            tag_count,
        };
        let manifest = Manifest::from_pack(pack, &pools, &stats);
        manifest.validate()?;
        let manifest_bytes = manifest.to_json()?;

        // 11. Assemble ZIP members in required order.
        let mut members: Vec<Member> = Vec::new();
        members.push(Member {
            name: "mimetype".into(),
            data: container::MIMETYPE.to_vec(),
        });
        members.push(Member {
            name: "manifest.json".into(),
            data: manifest_bytes,
        });
        // Write cover image if present.
        if let Some(ref cover_bytes) = pack.cover {
            let cover_name = pack
                .metadata
                .cover
                .clone()
                .unwrap_or_else(|| "cover.png".into());
            members.push(Member {
                name: cover_name,
                data: cover_bytes.clone(),
            });
        }
        members.push(Member {
            name: "index/headword.idx".into(),
            data: headword_index.bytes.clone(),
        });
        if let Some(ref tag_bytes) = tag_index_bytes {
            members.push(Member {
                name: "index/tag.idx".into(),
                data: tag_bytes.clone(),
            });
        }
        if let Some(ref morph_bytes) = morph_index_bytes {
            members.push(Member {
                name: "index/morph.idx".into(),
                data: morph_bytes.clone(),
            });
        }
        members.push(Member {
            name: "index/strings.tbl".into(),
            data: pools.encode(),
        });
        for (i, block) in blocks.iter().enumerate() {
            let compressed = blocks::compress_block(&block.data, manifest.data.zstd_level)?;
            members.push(Member {
                name: blocks::block_path(i as u32),
                data: compressed,
            });
        }
        if has_media {
            let media_manifest_bytes = media_manifest.to_json()?;
            members.push(Member {
                name: "media/manifest.json".into(),
                data: media_manifest_bytes,
            });
            // Write deduped media files.
            let mut written: std::collections::HashSet<(String, String)> =
                std::collections::HashSet::new();
            for asset in &pack.media {
                let hash = media::sha1_hash(&asset.bytes);
                let hash_hex = media::hex::encode(&hash);
                let key = (asset.kind.as_str().to_string(), hash_hex.clone());
                if !written.insert(key) {
                    continue;
                }
                let path = media::media_path(asset.kind, &hash, &asset.ext);
                let stored_bytes = media::compress_asset_bytes(asset);
                members.push(Member {
                    name: path,
                    data: stored_bytes,
                });
            }
        }

        // 12. Write the ZIP.
        container::write_zip(members, writer)?;

        // Silence unused warning for HeadwordIndex.
        let _ = HeadwordIndex {
            header: headword_index.header,
            directory: headword_index.directory,
            bytes: Vec::new(),
        };
        let _ = media_map;

        Ok(())
    }
}

fn intern_pron(pools: &mut StringPools, pron: &model::Pron) -> Result<()> {
    if let Some(lang) = &pron.lang {
        pools.intern(PoolKind::Lang, lang)?;
    }
    if let Some(kind) = &pron.kind {
        pools.intern(PoolKind::PronKind, kind)?;
    }
    Ok(())
}

/// Build the tag.idx inverted index: for each tag strref, the sorted list
/// of entry ids (0-based ordinals in headword order).
fn build_tag_index(
    _pack: &Pack,
    pools: &StringPools,
    sorted_entries: &[&model::Entry],
) -> Result<Option<Vec<u8>>> {
    // Only emit if there are any entry-level tags.
    let any_tags = sorted_entries.iter().any(|e| !e.tags.is_empty());
    if !any_tags {
        return Ok(None);
    }

    // Map tag strref → list of entry ids.
    let mut postings: BTreeMap<u16, Vec<u32>> = BTreeMap::new();
    for (i, entry) in sorted_entries.iter().enumerate() {
        let mut seen: std::collections::HashSet<u16> = std::collections::HashSet::new();
        for tag in &entry.tags {
            let id = pools.lookup(PoolKind::Tag, tag);
            if id != 0 && seen.insert(id) {
                postings.entry(id).or_default().push(i as u32);
            }
        }
    }

    let entries: Vec<(u16, Vec<u32>)> = postings.into_iter().collect();
    Ok(Some(postings::encode_tag_index(&entries)?))
}

/// Build the morph.idx inverted index: for each feats key=value pair,
/// the sorted list of entry ids.
fn build_morph_index(
    _pack: &Pack,
    _pools: &StringPools,
    sorted_entries: &[&model::Entry],
) -> Result<Option<Vec<u8>>> {
    let any_feats = sorted_entries.iter().any(|e| {
        e.etymologies.iter().any(|ety| {
            ety.senses.iter().any(|s| {
                s.forms
                    .iter()
                    .any(|f| f.feats.as_ref().is_some_and(|x| !x.is_empty()))
            })
        })
    });
    if !any_feats {
        return Ok(None);
    }

    let mut postings: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for (i, entry) in sorted_entries.iter().enumerate() {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for ety in &entry.etymologies {
            for sense in &ety.senses {
                for form in &sense.forms {
                    if let Some(feats) = &form.feats {
                        if feats.is_empty() {
                            continue;
                        }
                        for pair in parse_feats(feats) {
                            if seen.insert(pair.clone()) {
                                postings.entry(pair).or_default().push(i as u32);
                            }
                        }
                    }
                }
            }
        }
    }

    let entries: Vec<(String, Vec<u32>)> = postings.into_iter().collect();
    Ok(Some(postings::encode_morph_index(&entries)?))
}

/// Parse a feats string into individual `key=value` pairs.
fn parse_feats(feats: &str) -> Vec<String> {
    // Per §3.4: pairs joined by `|`, with `\|` and `\=` escapes.
    let mut pairs = Vec::new();
    let mut current = String::new();
    let mut chars = feats.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                current.push('\\');
                current.push(next);
            }
        } else if c == '|' {
            pairs.push(current.clone());
            current.clear();
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        pairs.push(current);
    }
    pairs
}

fn count_morph_keys(sorted_entries: &[&model::Entry]) -> u32 {
    let mut keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in sorted_entries {
        for ety in &entry.etymologies {
            for sense in &ety.senses {
                for form in &sense.forms {
                    if let Some(feats) = &form.feats {
                        if feats.is_empty() {
                            continue;
                        }
                        for pair in parse_feats(feats) {
                            keys.insert(pair);
                        }
                    }
                }
            }
        }
    }
    keys.len() as u32
}
