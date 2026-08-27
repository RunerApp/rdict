//! Test how individual entry size affects performance.
//!
//! Usage: cargo run --manifest-path rdict/Cargo.toml --release --example entry_size_test

#![allow(clippy::vec_init_then_push)]
#![allow(clippy::type_complexity)]

use rdict::*;
use std::io::Cursor;
use std::time::Instant;

fn main() {
    let count = 50_000;

    println!("=== Entry Size Impact Test ===");
    println!("Entries: {count} (each scenario)");
    println!();

    let scenarios: [(&str, usize, fn(usize) -> Entry); 5] = [
        ("tiny", 1, make_tiny_entry),
        ("small", 50, make_small_entry),
        ("medium", 200, make_medium_entry),
        ("large", 1000, make_large_entry),
        ("xlarge", 5000, make_xlarge_entry),
    ];

    println!(
        "{:<8} {:>8} {:>10} {:>10} {:>12} {:>14} {:>14}",
        "Type", "Def Len", "Size (MB)", "Write (s)", "Open (ms)", "Rand 5K (ms)", "QPS"
    );
    println!("{}", "-".repeat(80));

    for (name, def_len, builder) in scenarios {
        let entries: Vec<Entry> = (0..count).map(builder).collect();

        let pack = Pack {
            metadata: PackMetadata {
                name: format!("entry-size-{name}"),
                zstd_level: 19,
                ..Default::default()
            },
            entries,
            media: Vec::new(),
            cover: None,
        };

        // Write
        let mut buf = Vec::new();
        let t = Instant::now();
        RdictWriter::write_pack(Cursor::new(&mut buf), &pack).expect("write");
        let write_s = t.elapsed().as_secs_f64();
        let size_mb = buf.len() as f64 / 1_048_576.0;

        // Open
        let t = Instant::now();
        let mut reader = RdictReader::new(Cursor::new(buf)).expect("open");
        let open_ms = t.elapsed().as_secs_f64() * 1000.0;

        // Random lookup 5000
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let samples = 5_000;
        let t = Instant::now();
        for i in 0..samples {
            let mut hasher = DefaultHasher::new();
            i.hash(&mut hasher);
            let idx = (hasher.finish() as usize) % count;
            let hw = format!("entry{:06}", idx);
            let _ = reader.lookup(&hw).unwrap();
        }
        let rand_dur = t.elapsed();
        let rand_ms = rand_dur.as_secs_f64() * 1000.0;
        let qps = samples as f64 / rand_dur.as_secs_f64();

        println!(
            "{:<8} {:>8} {:>10.2} {:>10.2} {:>12.2} {:>12.2} {:>12.0}",
            name, def_len, size_mb, write_s, open_ms, rand_ms, qps
        );
    }

    println!();
    println!("=== Done ===");
}

fn make_tiny_entry(i: usize) -> Entry {
    Entry {
        headword: format!("entry{:06}", i),
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
                    value: "x".into(),
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

fn make_small_entry(i: usize) -> Entry {
    Entry {
        headword: format!("entry{:06}", i),
        see: None,
        tags: Vec::new(),
        media: Vec::new(),
        pron: vec![Pron {
            lang: Some("en".into()),
            accent: None,
            kind: Some("ipa".into()),
            value: "/test/".into(),
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
                    value: "测试".into(),
                    pron: Vec::new(),
                }],
                forms: Vec::new(),
                tags: Vec::new(),
                pron: Vec::new(),
                definitions: vec![Def::Definition(Definition {
                    id: None,
                    value: "A short definition for testing.".into(),
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

fn make_medium_entry(i: usize) -> Entry {
    Entry {
        headword: format!("entry{:06}", i),
        see: None,
        tags: vec!["level:B1".into()],
        media: Vec::new(),
        pron: vec![Pron {
            lang: Some("en".into()),
            accent: None,
            kind: Some("ipa".into()),
            value: "/ˈtɛstɪŋ/".into(),
            media: Vec::new(),
        }],
        etymologies: vec![Ety {
            id: None,
            root: Some("testum".into()),
            senses: vec![Sense {
                pos: Some("NOUN".into()),
                lemma: None,
                translations: vec![Translation {
                    lang: Some("zh".into()),
                    value: "测试；试验".into(),
                    pron: Vec::new(),
                }],
                forms: vec![Form {
                    kind: Some("infl".into()),
                    term: "tests".into(),
                    tags: Vec::new(),
                    feats: Some("ud:Number=Plur".into()),
                    pron: Vec::new(),
                }],
                tags: Vec::new(),
                pron: Vec::new(),
                definitions: vec![
                    Def::Definition(Definition {
                        id: None,
                        value: "A procedure intended to establish the quality, performance, or reliability of something, especially before it is taken into widespread use.".into(),
                        examples: vec![Example {
                            value: "This is a test of the emergency broadcast system.".into(),
                            translations: vec![Translation {
                                lang: Some("zh".into()),
                                value: "这是紧急广播系统的测试。".into(),
                                pron: Vec::new(),
                            }],
                            pron: Vec::new(),
                            media: Vec::new(),
                            targets: Vec::new(),
                        }],
                        notes: Vec::new(),
                        media: Vec::new(),
                    }),
                    Def::Definition(Definition {
                        id: None,
                        value: "A short examination of proficiency or knowledge.".into(),
                        examples: Vec::new(),
                        notes: Vec::new(),
                        media: Vec::new(),
                    }),
                ],
            }],
        }],
        morphology: Vec::new(),
        relations: Vec::new(),
    }
}

fn make_large_entry(i: usize) -> Entry {
    let long_def = "This is a comprehensive definition that spans multiple sentences to simulate a real dictionary entry with detailed explanations. ".repeat(15);
    Entry {
        headword: format!("entry{:06}", i),
        see: None,
        tags: vec!["level:B2".into(), "exam:IELTS".into(), "domain:academic".into()],
        media: Vec::new(),
        pron: vec![Pron {
            lang: Some("en".into()),
            accent: Some("GA".into()),
            kind: Some("ipa".into()),
            value: "/kəmˈpriːhɛnsɪv/".into(),
            media: Vec::new(),
        }],
        etymologies: vec![Ety {
            id: None,
            root: Some("comprehensivus".into()),
            senses: vec![Sense {
                pos: Some("ADJ".into()),
                lemma: None,
                translations: vec![
                    Translation { lang: Some("zh".into()), value: "全面的；综合的".into(), pron: Vec::new() },
                    Translation { lang: Some("ja".into()), value: "包括的な".into(), pron: Vec::new() },
                ],
                forms: Vec::new(),
                tags: Vec::new(),
                pron: Vec::new(),
                definitions: (0..5).map(|_| Def::Definition(Definition {
                    id: None,
                    value: long_def.clone(),
                    examples: vec![Example {
                        value: "The report provides a comprehensive analysis of the current economic situation and its potential impact on various sectors.".into(),
                        translations: vec![Translation { lang: Some("zh".into()), value: "该报告对当前经济形势及其对各行业的潜在影响提供了全面分析。".into(), pron: Vec::new() }],
                        pron: Vec::new(),
                        media: Vec::new(),
                        targets: Vec::new(),
                    }],
                    notes: vec![rdict::Note { id: None, value: "Commonly used in academic and professional contexts.".into(), examples: Vec::new() }],
                    media: Vec::new(),
                })).collect(),
            }],
        }],
        morphology: Vec::new(),
        relations: Vec::new(),
    }
}

fn make_xlarge_entry(i: usize) -> Entry {
    let huge_def = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ".repeat(60);
    Entry {
        headword: format!("entry{:06}", i),
        see: None,
        tags: vec![
            "level:C1".into(),
            "exam:TOEFL".into(),
            "domain:literary".into(),
            "freq:rare".into(),
        ],
        media: Vec::new(),
        pron: vec![Pron {
            lang: Some("en".into()),
            accent: Some("RP".into()),
            kind: Some("ipa".into()),
            value: "/ˌsʌpəˌkalɪˌfrædʒɪˈlɪstɪkˌɛkspiːˌælɪˈdɒʃəs/".into(),
            media: Vec::new(),
        }],
        etymologies: vec![Ety {
            id: None,
            root: Some("Proto-Indo-European".into()),
            senses: (0..3)
                .map(|s| Sense {
                    pos: Some(
                        match s {
                            0 => "ADJ",
                            1 => "NOUN",
                            _ => "VERB",
                        }
                        .into(),
                    ),
                    lemma: None,
                    translations: (0..4)
                        .map(|l| Translation {
                            lang: Some(
                                match l {
                                    0 => "zh",
                                    1 => "ja",
                                    2 => "ko",
                                    _ => "fr",
                                }
                                .into(),
                            ),
                            value: format!("Translation {} for sense {}", l, s),
                            pron: Vec::new(),
                        })
                        .collect(),
                    forms: (0..3)
                        .map(|f| Form {
                            kind: Some("infl".into()),
                            term: format!("form_{}_{}", s, f),
                            tags: Vec::new(),
                            feats: Some("ud:Number=Plur".into()),
                            pron: Vec::new(),
                        })
                        .collect(),
                    tags: vec!["register:formal".into()],
                    pron: Vec::new(),
                    definitions: (0..10)
                        .map(|d| {
                            Def::Definition(Definition {
                                id: None,
                                value: format!("{}. {}", d, huge_def),
                                examples: (0..5)
                                    .map(|e| Example {
                                        value: format!(
                                            "Example sentence {} for definition {} of sense {}.",
                                            e, d, s
                                        ),
                                        translations: vec![Translation {
                                            lang: Some("zh".into()),
                                            value: format!("例句{}.{}.{}", e, d, s),
                                            pron: Vec::new(),
                                        }],
                                        pron: Vec::new(),
                                        media: Vec::new(),
                                        targets: Vec::new(),
                                    })
                                    .collect(),
                                notes: (0..3)
                                    .map(|n| rdict::Note {
                                        id: None,
                                        value: format!("Note {} for def {}", n, d),
                                        examples: Vec::new(),
                                    })
                                    .collect(),
                                media: Vec::new(),
                            })
                        })
                        .collect(),
                })
                .collect(),
        }],
        morphology: Vec::new(),
        relations: Vec::new(),
    }
}
