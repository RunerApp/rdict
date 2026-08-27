//! Owned Rust model of the Rdict AST and pack metadata. These types are
//! the API surface for writers and readers; the binary codec lives in
//! `ast.rs`.

use crate::Error;
use crate::primitive::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Serde helper: serialize `[u8; 20]` as a lowercase hex string,
/// deserialize from either hex string or number array (backward compat).
mod hash_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(hash: &[u8; 20], s: S) -> Result<S::Ok, S::Error> {
        let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        s.serialize_str(&hex)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 20], D::Error> {
        // Try hex string first, fall back to number array for backward compat.
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum HashRepr {
            Hex(String),
            Numbers(Vec<u8>),
        }

        let repr = HashRepr::deserialize(d)?;
        match repr {
            HashRepr::Hex(s) => {
                if s.len() != 40 {
                    return Err(serde::de::Error::custom(format!(
                        "hash hex string must be 40 chars, got {}",
                        s.len()
                    )));
                }
                let mut arr = [0u8; 20];
                for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
                    let hex_str = std::str::from_utf8(chunk)
                        .map_err(|_| serde::de::Error::custom("invalid utf8 in hash"))?;
                    arr[i] = u8::from_str_radix(hex_str, 16)
                        .map_err(|_| serde::de::Error::custom("invalid hex in hash"))?;
                }
                Ok(arr)
            }
            HashRepr::Numbers(nums) => {
                if nums.len() != 20 {
                    return Err(serde::de::Error::custom(format!(
                        "hash array must have 20 elements, got {}",
                        nums.len()
                    )));
                }
                let mut arr = [0u8; 20];
                arr.copy_from_slice(&nums);
                Ok(arr)
            }
        }
    }
}

/// A complete dictionary pack ready for writing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pack {
    pub metadata: PackMetadata,
    pub entries: Vec<Entry>,
    pub media: Vec<MediaAsset>,
    /// Optional cover image bytes (PNG or JPEG). When present, written to
    /// the ZIP root as `pack.metadata.cover` (or "cover.png" if unset).
    pub cover: Option<Vec<u8>>,
}

/// Pack-level metadata. Mirrors the `manifest.json` structure minus the
/// computed index/stat fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source_lang: String,
    pub target_langs: Vec<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub created_at: Option<String>,
    pub description: Option<String>,
    pub cover: Option<String>,
    pub block_max_uncompressed: u32,
    pub zstd_level: i32,
}

impl Default for PackMetadata {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Untitled".into(),
            version: "0.1.0".into(),
            source_lang: "en".into(),
            target_langs: Vec::new(),
            author: None,
            license: None,
            created_at: None,
            description: None,
            cover: None,
            block_max_uncompressed: 262_144,
            // Level 3 keeps compilation fast; callers can opt into level 19
            // for release builds when the small size reduction is worth it.
            zstd_level: 3,
        }
    }
}

/// An entry. The headword is stored separately (in the index) and is
/// attached here for convenience during writing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Entry {
    pub headword: String,
    pub see: Option<String>,
    pub tags: Vec<String>,
    pub media: Vec<MediaRef>,
    pub pron: Vec<Pron>,
    pub etymologies: Vec<Ety>,
    pub morphology: Vec<Morpheme>,
    pub relations: Vec<Relation>,
}

impl Entry {
    pub fn new(headword: impl Into<String>) -> Self {
        Self {
            headword: headword.into(),
            see: None,
            tags: Vec::new(),
            media: Vec::new(),
            pron: Vec::new(),
            etymologies: Vec::new(),
            morphology: Vec::new(),
            relations: Vec::new(),
        }
    }
}

/// A morpheme in an entry's morphological decomposition. See §6.4.1.
/// `kind` is interned in the `form_kind` pool when encoded. Recommended
/// values: `prefix`, `root`, `suffix`, `combining_form`, `infix`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Morpheme {
    pub kind: Option<String>,
    pub term: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Ety {
    pub id: Option<String>,
    pub root: Option<String>,
    pub senses: Vec<Sense>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Sense {
    pub pos: Option<String>,
    pub lemma: Option<String>,
    pub translations: Vec<Translation>,
    pub forms: Vec<Form>,
    pub tags: Vec<String>,
    pub pron: Vec<Pron>,
    pub definitions: Vec<Def>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Def {
    Definition(Definition),
    Group(Group),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Definition {
    pub id: Option<String>,
    pub value: String,
    pub examples: Vec<Example>,
    pub notes: Vec<Note>,
    pub media: Vec<MediaRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Group {
    pub id: Option<String>,
    pub description: Option<String>,
    pub definitions: Vec<Def>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Example {
    pub value: String,
    pub translations: Vec<Translation>,
    pub pron: Vec<Pron>,
    pub media: Vec<MediaRef>,
    pub targets: Vec<TargetOccurrence>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TargetOccurrence {
    pub spans: Vec<TextSpan>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TextSpan {
    pub offset: u64,
    pub length: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Note {
    pub id: Option<String>,
    pub value: String,
    pub examples: Vec<Example>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Translation {
    pub lang: Option<String>,
    pub value: String,
    pub pron: Vec<Pron>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Pron {
    pub lang: Option<String>,
    pub accent: Option<String>,
    pub kind: Option<String>,
    pub value: String,
    pub media: Vec<MediaRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Form {
    pub kind: Option<String>,
    pub term: String,
    pub tags: Vec<String>,
    pub feats: Option<String>,
    pub pron: Vec<Pron>,
}

/// A reference to a media asset stored in `media/`.
///
/// In the distribution format, `hash` is the raw 20-byte SHA-1. In the
/// YAML/JSON source format, `path` is used instead and the compiler
/// resolves it to `hash` (clearing `path`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MediaRef {
    pub kind: MediaKind,
    /// SHA-1 hash, serialized as lowercase hex string in JSON/FFI.
    #[serde(default, with = "hash_serde")]
    pub hash: [u8; 20],
    /// Source-format file path. Not present in the distribution format.
    #[serde(default, skip_serializing)]
    pub path: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub alt: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    #[default]
    Audio,
    Image,
    Video,
}

impl MediaKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaKind::Audio => "audio",
            MediaKind::Image => "image",
            MediaKind::Video => "video",
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            MediaKind::Audio => 0,
            MediaKind::Image => 1,
            MediaKind::Video => 2,
        }
    }

    pub fn parse_byte(v: u8) -> Option<Self> {
        match v {
            0 => Some(MediaKind::Audio),
            1 => Some(MediaKind::Image),
            2 => Some(MediaKind::Video),
            _ => None,
        }
    }
}

/// A media asset to be stored in the pack. The writer computes the
/// SHA-1 hash and dedups by `(kind, hash)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MediaAsset {
    pub kind: MediaKind,
    #[serde(default)]
    pub ext: String,
    #[serde(default)]
    pub mime: String,
    #[serde(default)]
    pub compression: MediaCompression,
    /// Raw bytes. In source format, `path` may be set instead and the
    /// compiler reads the file.
    #[serde(default)]
    pub bytes: Vec<u8>,
    #[serde(default, skip_serializing)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaCompression {
    #[default]
    None,
    Zstd,
}

impl MediaCompression {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaCompression::None => "none",
            MediaCompression::Zstd => "zstd",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "none" => Some(MediaCompression::None),
            "zstd" => Some(MediaCompression::Zstd),
            _ => None,
        }
    }
}

/// An inline typed link from this entry to another headword (§6.11).
/// `type_` is interned in the `relation_type` pool when encoded.
/// Recommended values: `syn`, `ant`, `phrase`, `der`, `hyp`, `hypo`,
/// `hol`, `mer`, `see`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Relation {
    pub type_: Option<String>,
    pub target: String,
}

/// Validate model invariants before writing. Returns the first error
/// encountered, or `Ok(())` if the pack is well-formed.
pub fn validate_pack(pack: &Pack) -> Result<()> {
    if pack.metadata.id.is_empty() {
        return Err(Error::Validation("pack.id is empty".into()));
    }
    if pack.metadata.name.is_empty() {
        return Err(Error::Validation("pack.name is empty".into()));
    }
    if pack.metadata.source_lang.is_empty() {
        return Err(Error::Validation("pack.source_lang is empty".into()));
    }

    let mut headwords: BTreeSet<&[u8]> = BTreeSet::new();
    for entry in &pack.entries {
        if entry.headword.is_empty() {
            return Err(Error::Validation("empty headword".into()));
        }
        if !headwords.insert(entry.headword.as_bytes()) {
            return Err(Error::Validation(format!(
                "duplicate headword: {}",
                entry.headword
            )));
        }
        validate_entry(entry)?;
    }

    // Validate inline relations: target must be non-empty, no self-relation,
    // no duplicate (type, target) within one entry. Target existence in
    // headword.idx is checked by the writer after the headword set is built.
    let headword_set: BTreeSet<&[u8]> =
        pack.entries.iter().map(|e| e.headword.as_bytes()).collect();
    for entry in &pack.entries {
        let mut seen: BTreeSet<(Option<&str>, &str)> = BTreeSet::new();
        for rel in &entry.relations {
            if rel.target.is_empty() {
                return Err(Error::Validation(format!(
                    "relation on '{}' has empty target",
                    entry.headword
                )));
            }
            if rel.target == entry.headword {
                return Err(Error::Validation(format!(
                    "self-relation on '{}' (target == headword)",
                    entry.headword
                )));
            }
            if !headword_set.contains(rel.target.as_bytes()) {
                return Err(Error::Validation(format!(
                    "relation on '{}' targets non-existent headword '{}'",
                    entry.headword, rel.target
                )));
            }
            if !seen.insert((rel.type_.as_deref(), rel.target.as_str())) {
                return Err(Error::Validation(format!(
                    "duplicate relation ({:?}, '{}') on '{}",
                    rel.type_, rel.target, entry.headword
                )));
            }
        }
    }

    Ok(())
}

fn validate_entry(entry: &Entry) -> Result<()> {
    for pron in &entry.pron {
        validate_pron(pron)?;
    }
    for ety in &entry.etymologies {
        for sense in &ety.senses {
            validate_sense(sense)?;
        }
    }
    Ok(())
}

/// The 17 Universal Dependencies v2 UPOS tags (uppercase). POS values
/// in Sense.pos MUST be one of these when present.
pub const UPOS_TAGS: &[&str] = &[
    "NOUN", "VERB", "ADJ", "ADV", "ADP", "DET", "PRON", "NUM", "CCONJ", "SCONJ", "AUX", "PART",
    "INTJ", "PROPN", "PUNCT", "SYM", "X",
];

fn validate_sense(sense: &Sense) -> Result<()> {
    if let Some(ref pos) = sense.pos {
        if !UPOS_TAGS.contains(&pos.as_str()) {
            return Err(Error::Validation(format!(
                "Sense.pos {pos:?} is not a valid UD v2 UPOS tag (must be one of: {})",
                UPOS_TAGS.join(", ")
            )));
        }
    }
    for pron in &sense.pron {
        validate_pron(pron)?;
    }
    for tr in &sense.translations {
        for pron in &tr.pron {
            validate_pron(pron)?;
        }
    }
    for form in &sense.forms {
        for pron in &form.pron {
            validate_pron(pron)?;
        }
    }
    for def in &sense.definitions {
        validate_def(def)?;
    }
    Ok(())
}

fn validate_def(def: &Def) -> Result<()> {
    match def {
        Def::Definition(d) => {
            for ex in &d.examples {
                validate_example(ex)?;
            }
            for note in &d.notes {
                for ex in &note.examples {
                    validate_example(ex)?;
                }
            }
        }
        Def::Group(g) => {
            for def in &g.definitions {
                validate_def(def)?;
            }
        }
    }
    Ok(())
}

fn validate_example(ex: &Example) -> Result<()> {
    for pron in &ex.pron {
        validate_pron(pron)?;
    }
    validate_targets(&ex.value, &ex.targets)?;
    Ok(())
}

fn validate_pron(pron: &Pron) -> Result<()> {
    if pron.value.is_empty() {
        return Err(Error::Validation("Pron.value is empty".into()));
    }
    Ok(())
}

/// Validate target occurrence spans against an example value.
pub fn validate_targets(value: &str, targets: &[TargetOccurrence]) -> Result<()> {
    let value_bytes = value.as_bytes();
    let value_len = value_bytes.len() as u64;
    let mut prev_end: Option<u64> = None;
    for occ in targets {
        if occ.spans.is_empty() {
            return Err(Error::Validation("target occurrence has no spans".into()));
        }
        let mut occ_prev_end: Option<u64> = None;
        for span in &occ.spans {
            if span.length == 0 {
                return Err(Error::Validation("target span length is zero".into()));
            }
            if span.offset + span.length > value_len {
                return Err(Error::Validation(
                    "target span exceeds example value length".into(),
                ));
            }
            if !is_utf8_boundary(value_bytes, span.offset as usize) {
                return Err(Error::Validation(
                    "target span offset is not a UTF-8 boundary".into(),
                ));
            }
            if !is_utf8_boundary(value_bytes, (span.offset + span.length) as usize) {
                return Err(Error::Validation(
                    "target span end is not a UTF-8 boundary".into(),
                ));
            }
            if let Some(prev) = occ_prev_end {
                if span.offset < prev {
                    return Err(Error::Validation(
                        "target spans in one occurrence must be ordered and non-overlapping".into(),
                    ));
                }
            }
            occ_prev_end = Some(span.offset + span.length);
        }
        let first = occ.spans[0].offset;
        if let Some(prev) = prev_end {
            if first < prev {
                return Err(Error::Validation(
                    "target occurrences must be ordered and non-overlapping".into(),
                ));
            }
        }
        prev_end = occ_prev_end;
    }
    Ok(())
}

fn is_utf8_boundary(bytes: &[u8], idx: usize) -> bool {
    if idx == 0 || idx == bytes.len() {
        return true;
    }
    if idx > bytes.len() {
        return false;
    }
    // UTF-8 continuation bytes have the high bit pattern 10xxxxxx.
    (bytes[idx] & 0xC0) != 0x80
}
