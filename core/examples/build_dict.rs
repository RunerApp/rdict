//! Build a sample English-Chinese dictionary and demonstrate lookups.
//!
//! Run: cargo run --manifest-path rdict/Cargo.toml --example build_dict

#![allow(clippy::vec_init_then_push)]

use rdict::*;
use std::fs::File;
use std::io::Cursor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pack = build_sample_pack();

    // Write to the examples directory
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("sample.rdict");
    {
        let file = File::create(&path)?;
        RdictWriter::write_pack(file, &pack)?;
    }
    println!("Dictionary written: {}", path.display());

    // Can also write to an in-memory buffer
    let mut buf = Vec::new();
    RdictWriter::write_pack(Cursor::new(&mut buf), &pack)?;
    println!("In-memory buffer size: {} bytes", buf.len());

    // Open from file and query
    let mut reader = RdictReader::open(&path)?;

    for hw in &["hello", "run", "colour", "世界"] {
        match reader.lookup(hw)? {
            Some(LookupEntry::Decoded(entry)) => {
                println!("\n=== {} ===", entry.headword);
                if let Some(see) = &entry.see {
                    println!("  → see {}", see);
                }
                for pron in &entry.pron {
                    let lang = pron.lang.as_deref().unwrap_or("en");
                    let accent = pron.accent.as_deref().unwrap_or("");
                    println!("  pron [{} {}]: {}", lang, accent, pron.value);
                }
                for ety in &entry.etymologies {
                    for sense in &ety.senses {
                        if let Some(pos) = &sense.pos {
                            println!("  pos: {}", pos);
                        }
                        for def in &sense.definitions {
                            if let Def::Definition(d) = def {
                                println!("  def: {}", d.value);
                                for ex in &d.examples {
                                    println!("    example: {}", ex.value);
                                    for tr in &ex.translations {
                                        println!("      translation: {}", tr.value);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Some(LookupEntry::Opaque { .. }) => {
                println!("\n=== {} === (opaque, undecodable)", hw);
            }
            None => {
                println!("\n=== {} === not found", hw);
            }
        }
    }

    // Demonstrate looking up a non-existent word
    println!(
        "\nLookup 'nonexistent': {:?}",
        reader.lookup("nonexistent")?.is_none()
    );

    Ok(())
}

fn build_sample_pack() -> Pack {
    let mut entries = Vec::new();

    // === hello: with translations and examples ===
    entries.push(Entry {
        headword: "hello".into(),
        see: None,
        tags: vec!["level:A1".into()],
        media: Vec::new(),
        pron: vec![Pron {
            lang: Some("en".into()),
            accent: Some("General American".into()),
            kind: Some("ipa".into()),
            value: "/həˈloʊ/".into(),
            media: Vec::new(),
        }],
        etymologies: vec![Ety {
            id: None,
            root: None,
            senses: vec![Sense {
                pos: Some("INTJ".into()),
                lemma: None,
                translations: vec![Translation {
                    lang: Some("zh".into()),
                    value: "你好".into(),
                    pron: Vec::new(),
                }],
                forms: Vec::new(),
                tags: Vec::new(),
                pron: Vec::new(),
                definitions: vec![
                    Def::Definition(Definition {
                        id: None,
                        value: "Used as a greeting".into(),
                        examples: vec![Example {
                            value: "Hello, how are you?".into(),
                            translations: vec![Translation {
                                lang: Some("zh".into()),
                                value: "你好，你怎么样？".into(),
                                pron: Vec::new(),
                            }],
                            pron: Vec::new(),
                            media: Vec::new(),
                            targets: vec![TargetOccurrence {
                                spans: vec![TextSpan {
                                    offset: 0,
                                    length: 5,
                                }],
                            }],
                        }],
                        notes: Vec::new(),
                        media: Vec::new(),
                    }),
                    Def::Definition(Definition {
                        id: None,
                        value: "Used to attract attention".into(),
                        examples: Vec::new(),
                        notes: Vec::new(),
                        media: Vec::new(),
                    }),
                ],
            }],
        }],
        morphology: Vec::new(),
        relations: Vec::new(),
    });

    // === run: with inflected forms ===
    entries.push(Entry {
        headword: "run".into(),
        see: None,
        tags: vec!["level:B1".into(), "exam:IELTS".into()],
        media: Vec::new(),
        pron: vec![Pron {
            lang: Some("en".into()),
            accent: None,
            kind: Some("ipa".into()),
            value: "/rʌn/".into(),
            media: Vec::new(),
        }],
        etymologies: vec![Ety {
            id: None,
            root: Some("rinnan".into()),
            senses: vec![Sense {
                pos: Some("VERB".into()),
                lemma: None,
                translations: vec![Translation {
                    lang: Some("zh".into()),
                    value: "跑；运行".into(),
                    pron: Vec::new(),
                }],
                forms: vec![
                    Form {
                        kind: Some("infl".into()),
                        term: "ran".into(),
                        tags: Vec::new(),
                        feats: Some("ud:Tense=Past".into()),
                        pron: vec![Pron {
                            lang: None,
                            accent: None,
                            kind: Some("ipa".into()),
                            value: "/ræn/".into(),
                            media: Vec::new(),
                        }],
                    },
                    Form {
                        kind: Some("infl".into()),
                        term: "running".into(),
                        tags: Vec::new(),
                        feats: Some("ud:VerbForm=Ger".into()),
                        pron: Vec::new(),
                    },
                ],
                tags: Vec::new(),
                pron: Vec::new(),
                definitions: vec![Def::Definition(Definition {
                    id: None,
                    value: "To move swiftly on foot".into(),
                    examples: vec![Example {
                        value: "He ran fast.".into(),
                        translations: Vec::new(),
                        pron: Vec::new(),
                        media: Vec::new(),
                        targets: vec![TargetOccurrence {
                            spans: vec![TextSpan {
                                offset: 3,
                                length: 3,
                            }],
                        }],
                    }],
                    notes: Vec::new(),
                    media: Vec::new(),
                })],
            }],
        }],
        morphology: Vec::new(),
        relations: Vec::new(),
    });

    // === colour: redirect ===
    entries.push(Entry {
        headword: "colour".into(),
        see: Some("color".into()),
        tags: Vec::new(),
        media: Vec::new(),
        pron: Vec::new(),
        etymologies: Vec::new(),
        morphology: Vec::new(),
        relations: Vec::new(),
    });

    // === 世界: Chinese headword ===
    entries.push(Entry {
        headword: "世界".into(),
        see: None,
        tags: vec!["level:HSK3".into()],
        media: Vec::new(),
        pron: vec![Pron {
            lang: Some("zh".into()),
            accent: None,
            kind: Some("pinyin".into()),
            value: "shìjiè".into(),
            media: Vec::new(),
        }],
        etymologies: vec![Ety {
            id: None,
            root: None,
            senses: vec![Sense {
                pos: Some("NOUN".into()),
                lemma: None,
                translations: vec![Translation {
                    lang: Some("en".into()),
                    value: "world".into(),
                    pron: Vec::new(),
                }],
                forms: Vec::new(),
                tags: Vec::new(),
                pron: Vec::new(),
                definitions: vec![Def::Definition(Definition {
                    id: None,
                    value: "地球；所有地方".into(),
                    examples: vec![Example {
                        value: "世界很大".into(),
                        translations: vec![Translation {
                            lang: Some("en".into()),
                            value: "The world is big".into(),
                            pron: Vec::new(),
                        }],
                        pron: Vec::new(),
                        media: Vec::new(),
                        targets: vec![TargetOccurrence {
                            spans: vec![TextSpan {
                                offset: 0,
                                length: 6,
                            }],
                        }],
                    }],
                    notes: Vec::new(),
                    media: Vec::new(),
                })],
            }],
        }],
        morphology: Vec::new(),
        relations: Vec::new(),
    });

    // Add an inline relation on "hello": see-also → "run"
    entries[0].relations = vec![Relation {
        type_: Some("see".into()),
        target: "run".into(),
    }];

    // === color: redirect target for colour ===
    entries.push(Entry {
        headword: "color".into(),
        see: None,
        tags: vec!["level:A1".into()],
        media: Vec::new(),
        pron: vec![Pron {
            lang: Some("en".into()),
            accent: Some("General American".into()),
            kind: Some("ipa".into()),
            value: "/ˈkʌlər/".into(),
            media: Vec::new(),
        }],
        etymologies: vec![Ety {
            id: None,
            root: None,
            senses: vec![Sense {
                pos: Some("NOUN".into()),
                lemma: None,
                translations: vec![Translation {
                    lang: Some("zh".into()),
                    value: "颜色".into(),
                    pron: Vec::new(),
                }],
                forms: Vec::new(),
                tags: Vec::new(),
                pron: Vec::new(),
                definitions: vec![Def::Definition(Definition {
                    id: None,
                    value: "The property possessed by an object of producing different sensations on the eye".into(),
                    examples: vec![Example {
                        value: "What is your favorite color?".into(),
                        translations: vec![Translation {
                            lang: Some("zh".into()),
                            value: "你最喜欢什么颜色？".into(),
                            pron: Vec::new(),
                        }],
                        pron: Vec::new(),
                        media: Vec::new(),
                        targets: vec![TargetOccurrence {
                            spans: vec![TextSpan {
                                offset: 22,
                                length: 5,
                            }],
                        }],
                    }],
                    notes: Vec::new(),
                    media: Vec::new(),
                })],
            }],
        }],
        morphology: Vec::new(),
        relations: Vec::new(),
    });

    Pack {
        metadata: PackMetadata {
            id: "00000000-0000-4000-8000-000000000010".into(),
            name: "Sample English-Chinese Dictionary".into(),
            version: "1.0.0".into(),
            source_lang: "en".into(),
            target_langs: vec!["zh".into()],
            author: Some("Rdict Demo".into()),
            license: Some("MIT".into()),
            created_at: Some("2026-08-20".into()),
            description: Some("A sample dictionary for demonstration".into()),
            ..Default::default()
        },
        entries,
        media: Vec::new(),
        cover: None,
    }
}
