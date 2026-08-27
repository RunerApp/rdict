//! Test fixtures: reusable Pack builders for integration tests.

#![allow(dead_code)]

use rdict::*;

pub fn minimal_pack() -> Pack {
    let mut pack = Pack {
        metadata: PackMetadata {
            id: "00000000-0000-4000-8000-000000000001".into(),
            name: "Test Dictionary".into(),
            version: "1.0.0".into(),
            source_lang: "en".into(),
            target_langs: vec!["zh".into()],
            ..Default::default()
        },
        entries: Vec::new(),
        media: Vec::new(),
        cover: None,
    };
    pack.entries.push(simple_entry("hello", "A greeting"));
    pack.entries.push(simple_entry("world", "The earth"));
    pack
}

pub fn simple_entry(headword: &str, def: &str) -> Entry {
    Entry {
        headword: headword.into(),
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
                    value: def.into(),
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

pub fn entry_with_pron() -> Entry {
    let mut e = simple_entry("run", "To move swiftly");
    e.pron = vec![Pron {
        lang: Some("en".into()),
        accent: Some("General American".into()),
        kind: Some("ipa".into()),
        value: "/rʌn/".into(),
        media: Vec::new(),
    }];
    e
}

pub fn entry_with_tags() -> Entry {
    let mut e = simple_entry("test", "An examination");
    e.tags = vec!["exam:IELTS".into(), "level:B2".into()];
    e
}

pub fn entry_with_target_spans() -> Entry {
    let mut e = simple_entry("take", "To grasp");
    if let Some(Def::Definition(d)) = e.etymologies[0].senses[0].definitions.get_mut(0) {
        d.examples = vec![Example {
            value: "Take care of yourself".into(),
            translations: Vec::new(),
            pron: Vec::new(),
            media: Vec::new(),
            targets: vec![TargetOccurrence {
                spans: vec![TextSpan {
                    offset: 0,
                    length: 13, // "Take care of"
                }],
            }],
        }];
    }
    e
}

pub fn entry_with_discontinuous_target() -> Entry {
    let mut e = simple_entry("look", "To see");
    if let Some(Def::Definition(d)) = e.etymologies[0].senses[0].definitions.get_mut(0) {
        d.examples = vec![Example {
            value: "Look the word up".into(),
            translations: Vec::new(),
            pron: Vec::new(),
            media: Vec::new(),
            targets: vec![TargetOccurrence {
                spans: vec![
                    TextSpan {
                        offset: 0,
                        length: 4, // "Look"
                    },
                    TextSpan {
                        offset: 13,
                        length: 2, // "up"
                    },
                ],
            }],
        }];
    }
    e
}

pub fn entry_with_form_pron() -> Entry {
    let mut e = simple_entry("run", "To move swiftly");
    e.etymologies[0].senses[0].forms = vec![Form {
        kind: Some("infl".into()),
        term: "ran".into(),
        tags: Vec::new(),
        feats: Some("ud:ConjugationForm=Ta|ud:ConjugationType=Godan".into()),
        pron: vec![Pron {
            lang: None,
            accent: None,
            kind: Some("ipa".into()),
            value: "/ræn/".into(),
            media: Vec::new(),
        }],
    }];
    e
}

pub fn unicode_pack() -> Pack {
    Pack {
        metadata: PackMetadata {
            id: "00000000-0000-4000-8000-000000000002".into(),
            name: "Unicode Test".into(),
            source_lang: "ja".into(),
            ..Default::default()
        },
        entries: vec![
            simple_entry("あ", "ひらがな"),
            simple_entry("い", "ひらがな"),
            simple_entry("愛", "愛の意味"),
            simple_entry("日本", "国名"),
        ],
        media: Vec::new(),
        cover: None,
    }
}

/// A pack with inline relations on entries: "hello" has a see-also link
/// to "world" and a synonym link to "world". "world" has a synonym link
/// back to "hello" (compiler-synthesized symmetric relation).
pub fn pack_with_relations() -> Pack {
    let mut pack = minimal_pack();
    pack.entries[0].relations = vec![
        Relation {
            type_: Some("see".into()),
            target: "world".into(),
        },
        Relation {
            type_: Some("syn".into()),
            target: "world".into(),
        },
    ];
    pack.entries[1].relations = vec![Relation {
        type_: Some("syn".into()),
        target: "hello".into(),
    }];
    pack
}

pub fn entry_with_repeated_targets() -> Entry {
    // "He ran and ran." — two occurrences of "ran"
    let mut e = simple_entry("ran", "past tense of run");
    if let Some(Def::Definition(d)) = e.etymologies[0].senses[0].definitions.get_mut(0) {
        d.examples = vec![Example {
            value: "He ran and ran.".into(),
            translations: Vec::new(),
            pron: Vec::new(),
            media: Vec::new(),
            targets: vec![
                TargetOccurrence {
                    spans: vec![TextSpan {
                        offset: 3,
                        length: 3, // "ran"
                    }],
                },
                TargetOccurrence {
                    spans: vec![TextSpan {
                        offset: 11,
                        length: 3, // second "ran"
                    }],
                },
            ],
        }];
    }
    e
}

pub fn entry_with_morph_feats() -> Entry {
    let mut e = simple_entry("走る", "to run");
    e.etymologies[0].senses[0].forms = vec![Form {
        kind: Some("infl".into()),
        term: "走った".into(),
        tags: Vec::new(),
        feats: Some("ud:ConjugationType=Godan|ud:ConjugationForm=Ta".into()),
        pron: Vec::new(),
    }];
    e
}

/// An entry with morphological decomposition (prefix + root + suffix).
pub fn entry_with_morphology() -> Entry {
    let mut e = simple_entry("unhappiness", "state of being unhappy");
    e.morphology = vec![
        Morpheme {
            kind: Some("prefix".into()),
            term: "un-".into(),
        },
        Morpheme {
            kind: Some("root".into()),
            term: "happy".into(),
        },
        Morpheme {
            kind: Some("suffix".into()),
            term: "-ness".into(),
        },
    ];
    e
}

/// An entry with an etymological root.
pub fn entry_with_ety_root() -> Entry {
    let mut e = simple_entry("bank", "financial institution");
    e.etymologies[0].root = Some("bancus".into());
    e
}

/// A pack with many entries whose total uncompressed size exceeds
/// the default 256 KiB block target, forcing multiple zstd blocks.
pub fn multi_block_pack() -> Pack {
    let mut pack = Pack {
        metadata: PackMetadata {
            id: "00000000-0000-4000-8000-000000000003".into(),
            name: "Multi Block Test".into(),
            source_lang: "en".into(),
            block_max_uncompressed: 1024, // tiny blocks to force splitting
            ..Default::default()
        },
        entries: Vec::new(),
        media: Vec::new(),
        cover: None,
    };
    // 50 entries × ~100 bytes each = ~5 KiB → 5 blocks at 1 KiB each.
    for i in 0..50 {
        let hw = format!("entry{:03}", i);
        let def = format!(
            "Definition number {} with some padding text to make it longer",
            i
        );
        pack.entries.push(simple_entry(&hw, &def));
    }
    pack
}

/// A pack with media assets.
pub fn pack_with_media() -> Pack {
    let mut pack = minimal_pack();
    pack.media = vec![MediaAsset {
        kind: MediaKind::Audio,
        ext: "mp3".into(),
        mime: "audio/mpeg".into(),
        compression: MediaCompression::None,
        bytes: b"FAKE_MP3_DATA_BYTES".to_vec(),
        path: None,
    }];
    // Attach a media ref to the first entry.
    let hash = {
        use sha1::{Digest, Sha1};
        let mut h = Sha1::new();
        h.update(b"FAKE_MP3_DATA_BYTES");
        let result = h.finalize();
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&result);
        arr
    };
    pack.entries[0].media = vec![MediaRef {
        kind: MediaKind::Audio,
        hash,
        path: None,
        description: Some("pronunciation audio".into()),
        alt: None,
    }];
    pack
}

/// A pack with entry-level tags on multiple entries for tag index testing.
pub fn pack_with_tags() -> Pack {
    let mut pack = minimal_pack();
    pack.entries[0].tags = vec!["exam:IELTS".into(), "level:B2".into()];
    pack.entries[1].tags = vec!["exam:IELTS".into(), "level:C1".into()];
    pack.entries.push({
        let mut e = simple_entry("test", "an exam");
        e.tags = vec!["exam:TOEFL".into(), "level:B2".into()];
        e
    });
    pack
}
