//! Binary codec for the §6 AST. Encodes/decodes Entry and all nested
//! records using varints, flag bytes, and string-pool references.

use crate::model::*;
use crate::primitive::{self, BoundedReader, ByteWriter, Result};
use crate::strings::{PoolKind, StringPools};

// ===== Flag bit constants =====

// Entry: §6.4
const ENTRY_HAS_SEE: u8 = 1 << 0;
const ENTRY_HAS_TAG: u8 = 1 << 1;
const ENTRY_HAS_MEDIA: u8 = 1 << 2;
const ENTRY_HAS_PRON: u8 = 1 << 3;
const ENTRY_HAS_ETY: u8 = 1 << 4;
const ENTRY_HAS_MORPHOLOGY: u8 = 1 << 5;
const ENTRY_HAS_RELATIONS: u8 = 1 << 6;
const ENTRY_KNOWN_MASK: u8 = ENTRY_HAS_SEE
    | ENTRY_HAS_TAG
    | ENTRY_HAS_MEDIA
    | ENTRY_HAS_PRON
    | ENTRY_HAS_ETY
    | ENTRY_HAS_MORPHOLOGY
    | ENTRY_HAS_RELATIONS;

// Ety: §6.5
const ETY_HAS_ID: u8 = 1 << 0;
const ETY_HAS_ROOT: u8 = 1 << 1;
const ETY_HAS_SENSE: u8 = 1 << 2;
const ETY_KNOWN_MASK: u8 = ETY_HAS_ID | ETY_HAS_ROOT | ETY_HAS_SENSE;

// Morpheme: §6.4.1
const MORPH_HAS_KIND: u8 = 1 << 0;
const MORPH_KNOWN_MASK: u8 = MORPH_HAS_KIND;

// Sense: §6.6
const SENSE_HAS_LEMMA: u8 = 1 << 0;
const SENSE_HAS_TRANSLATION: u8 = 1 << 1;
const SENSE_HAS_FORM: u8 = 1 << 2;
const SENSE_HAS_TAG: u8 = 1 << 3;
const SENSE_HAS_PRON: u8 = 1 << 4;
const SENSE_KNOWN_MASK: u8 =
    SENSE_HAS_LEMMA | SENSE_HAS_TRANSLATION | SENSE_HAS_FORM | SENSE_HAS_TAG | SENSE_HAS_PRON;

// Definition: §6.7
const DEF_HAS_ID: u8 = 1 << 0;
const DEF_HAS_EXAMPLE: u8 = 1 << 1;
const DEF_HAS_NOTE: u8 = 1 << 2;
const DEF_HAS_MEDIA: u8 = 1 << 3;
const DEF_KNOWN_MASK: u8 = DEF_HAS_ID | DEF_HAS_EXAMPLE | DEF_HAS_NOTE | DEF_HAS_MEDIA;

// Group: §6.7
const GROUP_HAS_ID: u8 = 1 << 0;
const GROUP_HAS_DESC: u8 = 1 << 1;
const GROUP_KNOWN_MASK: u8 = GROUP_HAS_ID | GROUP_HAS_DESC;

// Example: §6.8
const EX_HAS_TRANSLATION: u8 = 1 << 0;
const EX_HAS_PRON: u8 = 1 << 1;
const EX_HAS_MEDIA: u8 = 1 << 2;
const EX_HAS_TARGET: u8 = 1 << 3;
const EX_KNOWN_MASK: u8 = EX_HAS_TRANSLATION | EX_HAS_PRON | EX_HAS_MEDIA | EX_HAS_TARGET;

// Note: §6.8
const NOTE_HAS_ID: u8 = 1 << 0;
const NOTE_HAS_EXAMPLE: u8 = 1 << 1;
const NOTE_KNOWN_MASK: u8 = NOTE_HAS_ID | NOTE_HAS_EXAMPLE;

// Translation: §6.8
const TR_HAS_PRON: u8 = 1 << 0;
const TR_KNOWN_MASK: u8 = TR_HAS_PRON;

// Pron: §6.8
const PRON_HAS_LANG: u8 = 1 << 0;
const PRON_HAS_ACCENT: u8 = 1 << 1;
const PRON_HAS_MEDIA: u8 = 1 << 2;
const PRON_KNOWN_MASK: u8 = PRON_HAS_LANG | PRON_HAS_ACCENT | PRON_HAS_MEDIA;

// Form: §6.8
const FORM_HAS_TAG: u8 = 1 << 0;
const FORM_HAS_FEATS: u8 = 1 << 1;
const FORM_HAS_PRON: u8 = 1 << 2;
const FORM_KNOWN_MASK: u8 = FORM_HAS_TAG | FORM_HAS_FEATS | FORM_HAS_PRON;

// MediaRef: §6.9
const MEDIA_HAS_DESC: u8 = 1 << 0;
const MEDIA_HAS_ALT: u8 = 1 << 1;
const MEDIA_KNOWN_MASK: u8 = MEDIA_HAS_DESC | MEDIA_HAS_ALT;

// Def kind
const DEF_KIND_DEFINITION: u8 = 0;
const DEF_KIND_GROUP: u8 = 1;

// ===== Encoder =====

/// Encode an entry body (without the headword) into bytes. The pools are
/// used to resolve strref indices; the writer must have interned all
/// pool strings before calling this.
pub fn encode_entry(entry: &Entry, pools: &StringPools) -> Result<Vec<u8>> {
    let mut w = ByteWriter::with_capacity(256);
    let mut flags = 0u8;
    if entry.see.is_some() {
        flags |= ENTRY_HAS_SEE;
    }
    if !entry.tags.is_empty() {
        flags |= ENTRY_HAS_TAG;
    }
    if !entry.media.is_empty() {
        flags |= ENTRY_HAS_MEDIA;
    }
    if !entry.pron.is_empty() {
        flags |= ENTRY_HAS_PRON;
    }
    if !entry.etymologies.is_empty() {
        flags |= ENTRY_HAS_ETY;
    }
    if !entry.morphology.is_empty() {
        flags |= ENTRY_HAS_MORPHOLOGY;
    }
    if !entry.relations.is_empty() {
        flags |= ENTRY_HAS_RELATIONS;
    }
    primitive::write_u8(&mut w, flags)?;
    if let Some(see) = &entry.see {
        primitive::write_str(&mut w, see)?;
    }
    if !entry.tags.is_empty() {
        primitive::write_varint(&mut w, entry.tags.len() as u64)?;
        for tag in &entry.tags {
            let idx = pools.lookup(PoolKind::Tag, tag);
            primitive::write_u16(&mut w, idx)?;
        }
    }
    if !entry.media.is_empty() {
        write_media_refs(&mut w, &entry.media)?;
    }
    if !entry.pron.is_empty() {
        write_prons(&mut w, &entry.pron, pools)?;
    }
    if !entry.etymologies.is_empty() {
        primitive::write_varint(&mut w, entry.etymologies.len() as u64)?;
        for ety in &entry.etymologies {
            encode_ety(&mut w, ety, pools)?;
        }
    }
    if !entry.morphology.is_empty() {
        primitive::write_varint(&mut w, entry.morphology.len() as u64)?;
        for morph in &entry.morphology {
            encode_morpheme(&mut w, morph, pools)?;
        }
    }
    if !entry.relations.is_empty() {
        primitive::write_varint(&mut w, entry.relations.len() as u64)?;
        for rel in &entry.relations {
            encode_relation(&mut w, rel, pools)?;
        }
    }
    Ok(w.into_bytes())
}

fn encode_morpheme(w: &mut ByteWriter, morph: &Morpheme, pools: &StringPools) -> Result<()> {
    let mut flags = 0u8;
    if morph.kind.is_some() {
        flags |= MORPH_HAS_KIND;
    }
    primitive::write_u8(w, flags)?;
    let kind_idx = morph
        .kind
        .as_ref()
        .map(|k| pools.lookup(PoolKind::FormKind, k))
        .unwrap_or(0);
    primitive::write_u16(w, kind_idx)?;
    primitive::write_str(w, &morph.term)?;
    Ok(())
}

fn encode_relation(w: &mut ByteWriter, rel: &Relation, pools: &StringPools) -> Result<()> {
    let type_idx = rel
        .type_
        .as_ref()
        .map(|t| pools.lookup(PoolKind::RelationType, t))
        .unwrap_or(0);
    primitive::write_u16(w, type_idx)?;
    primitive::write_str(w, &rel.target)?;
    Ok(())
}

fn encode_ety(w: &mut ByteWriter, ety: &Ety, pools: &StringPools) -> Result<()> {
    let mut flags = 0u8;
    if ety.id.is_some() {
        flags |= ETY_HAS_ID;
    }
    if ety.root.is_some() {
        flags |= ETY_HAS_ROOT;
    }
    if !ety.senses.is_empty() {
        flags |= ETY_HAS_SENSE;
    }
    primitive::write_u8(w, flags)?;
    if let Some(id) = &ety.id {
        primitive::write_str(w, id)?;
    }
    if let Some(root) = &ety.root {
        primitive::write_str(w, root)?;
    }
    if !ety.senses.is_empty() {
        primitive::write_varint(w, ety.senses.len() as u64)?;
        for sense in &ety.senses {
            encode_sense(w, sense, pools)?;
        }
    }
    Ok(())
}

fn encode_sense(w: &mut ByteWriter, sense: &Sense, pools: &StringPools) -> Result<()> {
    let mut flags = 0u8;
    if sense.lemma.is_some() {
        flags |= SENSE_HAS_LEMMA;
    }
    if !sense.translations.is_empty() {
        flags |= SENSE_HAS_TRANSLATION;
    }
    if !sense.forms.is_empty() {
        flags |= SENSE_HAS_FORM;
    }
    if !sense.tags.is_empty() {
        flags |= SENSE_HAS_TAG;
    }
    if !sense.pron.is_empty() {
        flags |= SENSE_HAS_PRON;
    }
    primitive::write_u8(w, flags)?;
    let pos_idx = sense
        .pos
        .as_ref()
        .map(|p| pools.lookup(PoolKind::Pos, p))
        .unwrap_or(0);
    primitive::write_u16(w, pos_idx)?;
    if let Some(lemma) = &sense.lemma {
        primitive::write_str(w, lemma)?;
    }
    if !sense.translations.is_empty() {
        primitive::write_varint(w, sense.translations.len() as u64)?;
        for tr in &sense.translations {
            encode_translation(w, tr, pools)?;
        }
    }
    if !sense.forms.is_empty() {
        primitive::write_varint(w, sense.forms.len() as u64)?;
        for form in &sense.forms {
            encode_form(w, form, pools)?;
        }
    }
    if !sense.tags.is_empty() {
        primitive::write_varint(w, sense.tags.len() as u64)?;
        for tag in &sense.tags {
            primitive::write_str(w, tag)?;
        }
    }
    if !sense.pron.is_empty() {
        write_prons(w, &sense.pron, pools)?;
    }
    primitive::write_varint(w, sense.definitions.len() as u64)?;
    for def in &sense.definitions {
        encode_def(w, def, pools)?;
    }
    Ok(())
}

fn encode_def(w: &mut ByteWriter, def: &Def, pools: &StringPools) -> Result<()> {
    match def {
        Def::Definition(d) => {
            primitive::write_u8(w, DEF_KIND_DEFINITION)?;
            encode_definition(w, d, pools)?;
        }
        Def::Group(g) => {
            primitive::write_u8(w, DEF_KIND_GROUP)?;
            encode_group(w, g, pools)?;
        }
    }
    Ok(())
}

fn encode_definition(w: &mut ByteWriter, d: &Definition, pools: &StringPools) -> Result<()> {
    let mut flags = 0u8;
    if d.id.is_some() {
        flags |= DEF_HAS_ID;
    }
    if !d.examples.is_empty() {
        flags |= DEF_HAS_EXAMPLE;
    }
    if !d.notes.is_empty() {
        flags |= DEF_HAS_NOTE;
    }
    if !d.media.is_empty() {
        flags |= DEF_HAS_MEDIA;
    }
    primitive::write_u8(w, flags)?;
    primitive::write_str(w, &d.value)?;
    if let Some(id) = &d.id {
        primitive::write_str(w, id)?;
    }
    if !d.examples.is_empty() {
        primitive::write_varint(w, d.examples.len() as u64)?;
        for ex in &d.examples {
            encode_example(w, ex, pools)?;
        }
    }
    if !d.notes.is_empty() {
        primitive::write_varint(w, d.notes.len() as u64)?;
        for note in &d.notes {
            encode_note(w, note, pools)?;
        }
    }
    if !d.media.is_empty() {
        write_media_refs(w, &d.media)?;
    }
    Ok(())
}

fn encode_group(w: &mut ByteWriter, g: &Group, pools: &StringPools) -> Result<()> {
    let mut flags = 0u8;
    if g.id.is_some() {
        flags |= GROUP_HAS_ID;
    }
    if g.description.is_some() {
        flags |= GROUP_HAS_DESC;
    }
    primitive::write_u8(w, flags)?;
    if let Some(id) = &g.id {
        primitive::write_str(w, id)?;
    }
    if let Some(desc) = &g.description {
        primitive::write_str(w, desc)?;
    }
    primitive::write_varint(w, g.definitions.len() as u64)?;
    for def in &g.definitions {
        encode_def(w, def, pools)?;
    }
    Ok(())
}

fn encode_example(w: &mut ByteWriter, ex: &Example, pools: &StringPools) -> Result<()> {
    let mut flags = 0u8;
    if !ex.translations.is_empty() {
        flags |= EX_HAS_TRANSLATION;
    }
    if !ex.pron.is_empty() {
        flags |= EX_HAS_PRON;
    }
    if !ex.media.is_empty() {
        flags |= EX_HAS_MEDIA;
    }
    if !ex.targets.is_empty() {
        flags |= EX_HAS_TARGET;
    }
    primitive::write_u8(w, flags)?;
    primitive::write_str(w, &ex.value)?;
    if !ex.translations.is_empty() {
        primitive::write_varint(w, ex.translations.len() as u64)?;
        for tr in &ex.translations {
            encode_translation(w, tr, pools)?;
        }
    }
    if !ex.pron.is_empty() {
        write_prons(w, &ex.pron, pools)?;
    }
    if !ex.media.is_empty() {
        write_media_refs(w, &ex.media)?;
    }
    if !ex.targets.is_empty() {
        primitive::write_varint(w, ex.targets.len() as u64)?;
        for occ in &ex.targets {
            primitive::write_varint(w, occ.spans.len() as u64)?;
            for span in &occ.spans {
                primitive::write_varint(w, span.offset)?;
                primitive::write_varint(w, span.length)?;
            }
        }
    }
    Ok(())
}

fn encode_note(w: &mut ByteWriter, note: &Note, pools: &StringPools) -> Result<()> {
    let mut flags = 0u8;
    if note.id.is_some() {
        flags |= NOTE_HAS_ID;
    }
    if !note.examples.is_empty() {
        flags |= NOTE_HAS_EXAMPLE;
    }
    primitive::write_u8(w, flags)?;
    primitive::write_str(w, &note.value)?;
    if let Some(id) = &note.id {
        primitive::write_str(w, id)?;
    }
    if !note.examples.is_empty() {
        primitive::write_varint(w, note.examples.len() as u64)?;
        for ex in &note.examples {
            encode_example(w, ex, pools)?;
        }
    }
    Ok(())
}

fn encode_translation(w: &mut ByteWriter, tr: &Translation, pools: &StringPools) -> Result<()> {
    let mut flags = 0u8;
    if !tr.pron.is_empty() {
        flags |= TR_HAS_PRON;
    }
    primitive::write_u8(w, flags)?;
    let lang_idx = tr
        .lang
        .as_ref()
        .map(|l| pools.lookup(PoolKind::Lang, l))
        .unwrap_or(0);
    primitive::write_u16(w, lang_idx)?;
    primitive::write_str(w, &tr.value)?;
    if !tr.pron.is_empty() {
        write_prons(w, &tr.pron, pools)?;
    }
    Ok(())
}

fn encode_pron(w: &mut ByteWriter, pron: &Pron, pools: &StringPools) -> Result<()> {
    let mut flags = 0u8;
    if pron.lang.is_some() {
        flags |= PRON_HAS_LANG;
    }
    if pron.accent.is_some() {
        flags |= PRON_HAS_ACCENT;
    }
    if !pron.media.is_empty() {
        flags |= PRON_HAS_MEDIA;
    }
    primitive::write_u8(w, flags)?;
    if let Some(lang) = &pron.lang {
        let idx = pools.lookup(PoolKind::Lang, lang);
        primitive::write_u16(w, idx)?;
    }
    if let Some(accent) = &pron.accent {
        primitive::write_str(w, accent)?;
    }
    let kind_idx = pron
        .kind
        .as_ref()
        .map(|k| pools.lookup(PoolKind::PronKind, k))
        .unwrap_or(0);
    primitive::write_u16(w, kind_idx)?;
    primitive::write_str(w, &pron.value)?;
    if !pron.media.is_empty() {
        write_media_refs(w, &pron.media)?;
    }
    Ok(())
}

fn encode_form(w: &mut ByteWriter, form: &Form, pools: &StringPools) -> Result<()> {
    let mut flags = 0u8;
    if !form.tags.is_empty() {
        flags |= FORM_HAS_TAG;
    }
    if form.feats.is_some() {
        flags |= FORM_HAS_FEATS;
    }
    if !form.pron.is_empty() {
        flags |= FORM_HAS_PRON;
    }
    primitive::write_u8(w, flags)?;
    let kind_idx = form
        .kind
        .as_ref()
        .map(|k| pools.lookup(PoolKind::FormKind, k))
        .unwrap_or(0);
    primitive::write_u16(w, kind_idx)?;
    primitive::write_str(w, &form.term)?;
    if !form.tags.is_empty() {
        primitive::write_varint(w, form.tags.len() as u64)?;
        for tag in &form.tags {
            primitive::write_str(w, tag)?;
        }
    }
    if let Some(feats) = &form.feats {
        primitive::write_str(w, feats)?;
    }
    if !form.pron.is_empty() {
        write_prons(w, &form.pron, pools)?;
    }
    Ok(())
}

fn write_prons(w: &mut ByteWriter, prons: &[Pron], pools: &StringPools) -> Result<()> {
    primitive::write_varint(w, prons.len() as u64)?;
    for p in prons {
        encode_pron(w, p, pools)?;
    }
    Ok(())
}

fn write_media_refs(w: &mut ByteWriter, refs: &[MediaRef]) -> Result<()> {
    primitive::write_varint(w, refs.len() as u64)?;
    for m in refs {
        encode_media_ref(w, m)?;
    }
    Ok(())
}

fn encode_media_ref(w: &mut ByteWriter, m: &MediaRef) -> Result<()> {
    let mut flags = 0u8;
    if m.description.is_some() {
        flags |= MEDIA_HAS_DESC;
    }
    if m.alt.is_some() {
        flags |= MEDIA_HAS_ALT;
    }
    primitive::write_u8(w, flags)?;
    primitive::write_u8(w, m.kind.as_u8())?;
    w.buf.extend_from_slice(&m.hash);
    if let Some(desc) = &m.description {
        primitive::write_str(w, desc)?;
    }
    if let Some(alt) = &m.alt {
        primitive::write_str(w, alt)?;
    }
    Ok(())
}

// ===== Decoder =====

/// Outcome of decoding an entry slice.
pub enum DecodedEntry {
    Decoded(Entry),
    /// The entry contained an unknown flag bit or Def kind. The raw
    /// bytes are returned for opaque handling.
    Opaque(Vec<u8>),
}

/// Decode an entry body given its headword and raw bytes.
/// On unknown flags the entry is returned as Opaque rather than erroring.
pub fn decode_entry(headword: String, bytes: &[u8], pools: &StringPools) -> Result<DecodedEntry> {
    let mut r = BoundedReader::new(bytes);
    let flags = primitive::read_u8(&mut r)?;
    if flags & !ENTRY_KNOWN_MASK != 0 {
        return Ok(DecodedEntry::Opaque(bytes.to_vec()));
    }
    let see = if flags & ENTRY_HAS_SEE != 0 {
        Some(primitive::read_str(&mut r)?)
    } else {
        None
    };
    let tags = if flags & ENTRY_HAS_TAG != 0 {
        let n = primitive::read_varint(&mut r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let idx = primitive::read_u16(&mut r)?;
            match pools.resolve(PoolKind::Tag, idx) {
                Some(s) => v.push(s.to_string()),
                None => {
                    return Err(crate::Error::Malformed(format!(
                        "entry tag strref {} out of range",
                        idx
                    )));
                }
            }
        }
        v
    } else {
        Vec::new()
    };
    let media = if flags & ENTRY_HAS_MEDIA != 0 {
        read_media_refs(&mut r)?
    } else {
        Vec::new()
    };
    let pron = if flags & ENTRY_HAS_PRON != 0 {
        read_prons(&mut r, pools)?
    } else {
        Vec::new()
    };
    let etymologies = if flags & ENTRY_HAS_ETY != 0 {
        let n = primitive::read_varint(&mut r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(decode_ety(&mut r, pools)?);
        }
        v
    } else {
        Vec::new()
    };
    let morphology = if flags & ENTRY_HAS_MORPHOLOGY != 0 {
        let n = primitive::read_varint(&mut r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(decode_morpheme(&mut r, pools)?);
        }
        v
    } else {
        Vec::new()
    };
    let relations = if flags & ENTRY_HAS_RELATIONS != 0 {
        let n = primitive::read_varint(&mut r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(decode_relation(&mut r, pools)?);
        }
        v
    } else {
        Vec::new()
    };
    Ok(DecodedEntry::Decoded(Entry {
        headword,
        see,
        tags,
        media,
        pron,
        etymologies,
        morphology,
        relations,
    }))
}

fn decode_morpheme(r: &mut BoundedReader, pools: &StringPools) -> Result<Morpheme> {
    let flags = primitive::read_u8(r)?;
    if flags & !MORPH_KNOWN_MASK != 0 {
        return Err(crate::Error::Malformed("morpheme: unknown flag bit".into()));
    }
    let kind_idx = primitive::read_u16(r)?;
    let kind = pools
        .resolve(PoolKind::FormKind, kind_idx)
        .map(|s| s.to_string());
    let term = primitive::read_str(r)?;
    Ok(Morpheme { kind, term })
}

fn decode_relation(r: &mut BoundedReader, pools: &StringPools) -> Result<Relation> {
    let type_idx = primitive::read_u16(r)?;
    let type_ = pools
        .resolve(PoolKind::RelationType, type_idx)
        .map(|s| s.to_string());
    let target = primitive::read_str(r)?;
    Ok(Relation { type_, target })
}

fn decode_ety(r: &mut BoundedReader, pools: &StringPools) -> Result<Ety> {
    let flags = primitive::read_u8(r)?;
    if flags & !ETY_KNOWN_MASK != 0 {
        return Err(crate::Error::Malformed("ety: unknown flag bit".into()));
    }
    let id = if flags & ETY_HAS_ID != 0 {
        Some(primitive::read_str(r)?)
    } else {
        None
    };
    let root = if flags & ETY_HAS_ROOT != 0 {
        Some(primitive::read_str(r)?)
    } else {
        None
    };
    let senses = if flags & ETY_HAS_SENSE != 0 {
        let n = primitive::read_varint(r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(decode_sense(r, pools)?);
        }
        v
    } else {
        Vec::new()
    };
    Ok(Ety { id, root, senses })
}

fn decode_sense(r: &mut BoundedReader, pools: &StringPools) -> Result<Sense> {
    let flags = primitive::read_u8(r)?;
    if flags & !SENSE_KNOWN_MASK != 0 {
        return Err(crate::Error::Malformed("sense: unknown flag bit".into()));
    }
    let pos_idx = primitive::read_u16(r)?;
    let pos = pools.resolve(PoolKind::Pos, pos_idx).map(|s| s.to_string());
    let lemma = if flags & SENSE_HAS_LEMMA != 0 {
        Some(primitive::read_str(r)?)
    } else {
        None
    };
    let translations = if flags & SENSE_HAS_TRANSLATION != 0 {
        let n = primitive::read_varint(r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(decode_translation(r, pools)?);
        }
        v
    } else {
        Vec::new()
    };
    let forms = if flags & SENSE_HAS_FORM != 0 {
        let n = primitive::read_varint(r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(decode_form(r, pools)?);
        }
        v
    } else {
        Vec::new()
    };
    let tags = if flags & SENSE_HAS_TAG != 0 {
        let n = primitive::read_varint(r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(primitive::read_str(r)?);
        }
        v
    } else {
        Vec::new()
    };
    let pron = if flags & SENSE_HAS_PRON != 0 {
        read_prons(r, pools)?
    } else {
        Vec::new()
    };
    let def_count = primitive::read_varint(r)? as usize;
    let mut definitions = Vec::with_capacity(def_count);
    for _ in 0..def_count {
        definitions.push(decode_def(r, pools)?);
    }
    Ok(Sense {
        pos,
        lemma,
        translations,
        forms,
        tags,
        pron,
        definitions,
    })
}

fn decode_def(r: &mut BoundedReader, pools: &StringPools) -> Result<Def> {
    let kind = primitive::read_u8(r)?;
    match kind {
        DEF_KIND_DEFINITION => Ok(Def::Definition(decode_definition(r, pools)?)),
        DEF_KIND_GROUP => Ok(Def::Group(decode_group(r, pools)?)),
        _ => Err(crate::Error::Malformed(format!(
            "unknown Def kind {}",
            kind
        ))),
    }
}

fn decode_definition(r: &mut BoundedReader, pools: &StringPools) -> Result<Definition> {
    let flags = primitive::read_u8(r)?;
    if flags & !DEF_KNOWN_MASK != 0 {
        return Err(crate::Error::Malformed(
            "definition: unknown flag bit".into(),
        ));
    }
    let value = primitive::read_str(r)?;
    let id = if flags & DEF_HAS_ID != 0 {
        Some(primitive::read_str(r)?)
    } else {
        None
    };
    let examples = if flags & DEF_HAS_EXAMPLE != 0 {
        let n = primitive::read_varint(r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(decode_example(r, pools)?);
        }
        v
    } else {
        Vec::new()
    };
    let notes = if flags & DEF_HAS_NOTE != 0 {
        let n = primitive::read_varint(r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(decode_note(r, pools)?);
        }
        v
    } else {
        Vec::new()
    };
    let media = if flags & DEF_HAS_MEDIA != 0 {
        read_media_refs(r)?
    } else {
        Vec::new()
    };
    Ok(Definition {
        id,
        value,
        examples,
        notes,
        media,
    })
}

fn decode_group(r: &mut BoundedReader, pools: &StringPools) -> Result<Group> {
    let flags = primitive::read_u8(r)?;
    if flags & !GROUP_KNOWN_MASK != 0 {
        return Err(crate::Error::Malformed("group: unknown flag bit".into()));
    }
    let id = if flags & GROUP_HAS_ID != 0 {
        Some(primitive::read_str(r)?)
    } else {
        None
    };
    let description = if flags & GROUP_HAS_DESC != 0 {
        Some(primitive::read_str(r)?)
    } else {
        None
    };
    let def_count = primitive::read_varint(r)? as usize;
    let mut definitions = Vec::with_capacity(def_count);
    for _ in 0..def_count {
        definitions.push(decode_def(r, pools)?);
    }
    Ok(Group {
        id,
        description,
        definitions,
    })
}

fn decode_example(r: &mut BoundedReader, pools: &StringPools) -> Result<Example> {
    let flags = primitive::read_u8(r)?;
    if flags & !EX_KNOWN_MASK != 0 {
        return Err(crate::Error::Malformed("example: unknown flag bit".into()));
    }
    let value = primitive::read_str(r)?;
    let translations = if flags & EX_HAS_TRANSLATION != 0 {
        let n = primitive::read_varint(r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(decode_translation(r, pools)?);
        }
        v
    } else {
        Vec::new()
    };
    let pron = if flags & EX_HAS_PRON != 0 {
        read_prons(r, pools)?
    } else {
        Vec::new()
    };
    let media = if flags & EX_HAS_MEDIA != 0 {
        read_media_refs(r)?
    } else {
        Vec::new()
    };
    let targets = if flags & EX_HAS_TARGET != 0 {
        let n = primitive::read_varint(r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let span_count = primitive::read_varint(r)? as usize;
            let mut spans = Vec::with_capacity(span_count);
            for _ in 0..span_count {
                let offset = primitive::read_varint(r)?;
                let length = primitive::read_varint(r)?;
                spans.push(TextSpan { offset, length });
            }
            v.push(TargetOccurrence { spans });
        }
        v
    } else {
        Vec::new()
    };
    Ok(Example {
        value,
        translations,
        pron,
        media,
        targets,
    })
}

fn decode_note(r: &mut BoundedReader, pools: &StringPools) -> Result<Note> {
    let flags = primitive::read_u8(r)?;
    if flags & !NOTE_KNOWN_MASK != 0 {
        return Err(crate::Error::Malformed("note: unknown flag bit".into()));
    }
    let value = primitive::read_str(r)?;
    let id = if flags & NOTE_HAS_ID != 0 {
        Some(primitive::read_str(r)?)
    } else {
        None
    };
    let examples = if flags & NOTE_HAS_EXAMPLE != 0 {
        let n = primitive::read_varint(r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(decode_example(r, pools)?);
        }
        v
    } else {
        Vec::new()
    };
    Ok(Note {
        id,
        value,
        examples,
    })
}

fn decode_translation(r: &mut BoundedReader, pools: &StringPools) -> Result<Translation> {
    let flags = primitive::read_u8(r)?;
    if flags & !TR_KNOWN_MASK != 0 {
        return Err(crate::Error::Malformed(
            "translation: unknown flag bit".into(),
        ));
    }
    let lang_idx = primitive::read_u16(r)?;
    let lang = pools
        .resolve(PoolKind::Lang, lang_idx)
        .map(|s| s.to_string());
    let value = primitive::read_str(r)?;
    let pron = if flags & TR_HAS_PRON != 0 {
        read_prons(r, pools)?
    } else {
        Vec::new()
    };
    Ok(Translation { lang, value, pron })
}

fn decode_pron(r: &mut BoundedReader, pools: &StringPools) -> Result<Pron> {
    let flags = primitive::read_u8(r)?;
    if flags & !PRON_KNOWN_MASK != 0 {
        return Err(crate::Error::Malformed("pron: unknown flag bit".into()));
    }
    let lang = if flags & PRON_HAS_LANG != 0 {
        let idx = primitive::read_u16(r)?;
        pools.resolve(PoolKind::Lang, idx).map(|s| s.to_string())
    } else {
        None
    };
    let accent = if flags & PRON_HAS_ACCENT != 0 {
        Some(primitive::read_str(r)?)
    } else {
        None
    };
    let kind_idx = primitive::read_u16(r)?;
    let kind = pools
        .resolve(PoolKind::PronKind, kind_idx)
        .map(|s| s.to_string());
    let value = primitive::read_str(r)?;
    let media = if flags & PRON_HAS_MEDIA != 0 {
        read_media_refs(r)?
    } else {
        Vec::new()
    };
    Ok(Pron {
        lang,
        accent,
        kind,
        value,
        media,
    })
}

fn decode_form(r: &mut BoundedReader, pools: &StringPools) -> Result<Form> {
    let flags = primitive::read_u8(r)?;
    if flags & !FORM_KNOWN_MASK != 0 {
        return Err(crate::Error::Malformed("form: unknown flag bit".into()));
    }
    let kind_idx = primitive::read_u16(r)?;
    let kind = pools
        .resolve(PoolKind::FormKind, kind_idx)
        .map(|s| s.to_string());
    let term = primitive::read_str(r)?;
    let tags = if flags & FORM_HAS_TAG != 0 {
        let n = primitive::read_varint(r)? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(primitive::read_str(r)?);
        }
        v
    } else {
        Vec::new()
    };
    let feats = if flags & FORM_HAS_FEATS != 0 {
        Some(primitive::read_str(r)?)
    } else {
        None
    };
    let pron = if flags & FORM_HAS_PRON != 0 {
        read_prons(r, pools)?
    } else {
        Vec::new()
    };
    Ok(Form {
        kind,
        term,
        tags,
        feats,
        pron,
    })
}

fn read_prons(r: &mut BoundedReader, pools: &StringPools) -> Result<Vec<Pron>> {
    let n = primitive::read_varint(r)? as usize;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(decode_pron(r, pools)?);
    }
    Ok(v)
}

fn read_media_refs(r: &mut BoundedReader) -> Result<Vec<MediaRef>> {
    let n = primitive::read_varint(r)? as usize;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(decode_media_ref(r)?);
    }
    Ok(v)
}

fn decode_media_ref(r: &mut BoundedReader) -> Result<MediaRef> {
    let flags = primitive::read_u8(r)?;
    if flags & !MEDIA_KNOWN_MASK != 0 {
        return Err(crate::Error::Malformed(
            "media ref: unknown flag bit".into(),
        ));
    }
    let kind_byte = primitive::read_u8(r)?;
    let kind = MediaKind::parse_byte(kind_byte)
        .ok_or_else(|| crate::Error::Malformed(format!("invalid media kind {}", kind_byte)))?;
    let mut hash = [0u8; 20];
    primitive::read_exact(r, &mut hash)?;
    let description = if flags & MEDIA_HAS_DESC != 0 {
        Some(primitive::read_str(r)?)
    } else {
        None
    };
    let alt = if flags & MEDIA_HAS_ALT != 0 {
        Some(primitive::read_str(r)?)
    } else {
        None
    };
    Ok(MediaRef {
        kind,
        hash,
        path: None,
        description,
        alt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entry() -> Entry {
        Entry {
            headword: "test".into(),
            see: None,
            tags: Vec::new(),
            media: Vec::new(),
            pron: Vec::new(),
            etymologies: vec![Ety {
                id: None,
                root: None,
                senses: vec![Sense {
                    pos: Some("NOUN".into()),
                    lemma: None,
                    translations: Vec::new(),
                    forms: Vec::new(),
                    tags: Vec::new(),
                    pron: Vec::new(),
                    definitions: vec![Def::Definition(Definition {
                        id: None,
                        value: "test".into(),
                        examples: Vec::new(),
                        notes: Vec::new(),
                        media: Vec::new(),
                    })],
                }],
            }],
            morphology: Vec::new(),
            relations: Vec::new(),
        }
    }

    fn test_pools() -> StringPools {
        let mut pools = StringPools::new();
        pools.intern(PoolKind::Pos, "NOUN").unwrap();
        pools
    }

    #[test]
    fn unknown_entry_flag_returns_opaque() {
        let entry = test_entry();
        let pools = test_pools();
        let bytes = encode_entry(&entry, &pools).unwrap();
        assert!(!bytes.is_empty());

        // Set an unknown flag bit (bit 7).
        let mut corrupted = bytes.clone();
        corrupted[0] |= 0x80;

        match decode_entry("test".into(), &corrupted, &pools).unwrap() {
            DecodedEntry::Opaque(raw) => assert_eq!(raw, corrupted),
            DecodedEntry::Decoded(_) => panic!("expected opaque for unknown entry flag"),
        }
    }

    #[test]
    fn unknown_def_kind_returns_error() {
        let entry = test_entry();
        let pools = test_pools();
        let bytes = encode_entry(&entry, &pools).unwrap();

        // Byte layout: entry_flags(0), ety_count(1), ety_flags(2),
        // sense_count(3), sense_flags(4), pos_u16(5-6), def_count(7),
        // def_kind(8), ...
        assert!(bytes.len() > 8);
        assert_eq!(bytes[8], DEF_KIND_DEFINITION);

        let mut corrupted = bytes.clone();
        corrupted[8] = 0xFF; // unknown Def kind

        // decode_entry returns Err; the reader converts this to Opaque.
        assert!(decode_entry("test".into(), &corrupted, &pools).is_err());
    }

    #[test]
    fn entry_roundtrip_basic() {
        let entry = test_entry();
        let pools = test_pools();
        let bytes = encode_entry(&entry, &pools).unwrap();

        match decode_entry("test".into(), &bytes, &pools).unwrap() {
            DecodedEntry::Decoded(e) => {
                assert_eq!(e.headword, "test");
                assert_eq!(e.etymologies.len(), 1);
                let sense = &e.etymologies[0].senses[0];
                assert_eq!(sense.pos, Some("NOUN".into()));
                if let Def::Definition(d) = &sense.definitions[0] {
                    assert_eq!(d.value, "test");
                } else {
                    panic!("expected Definition");
                }
            }
            DecodedEntry::Opaque(_) => panic!("expected decoded"),
        }
    }
}
