//! Stress test: generate N entries, measure write/read/lookup performance.
//! Also compares different zstd compression levels.
//!
//! Usage: cargo run --manifest-path rdict/Cargo.toml --release --example stress_test -- [count]
//! Default count: 100000

#![allow(clippy::vec_init_then_push)]

use rdict::*;
use std::env;
use std::io::Cursor;
use std::time::Instant;

fn main() {
    let count: usize = env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000);

    println!("=== Rdict Stress Test ===");
    println!("Entries: {count}");
    println!();

    // === 1. Build pack ===
    let t0 = Instant::now();
    let mut pack = build_pack(count);
    let build_time = t0.elapsed();
    println!("[Build] Generated {count} entries in memory: {build_time:?}");

    // === 2. Compare zstd levels (write + read) ===
    println!();
    println!("=== Zstd Level Comparison ===");
    println!(
        "{:<8} {:>10} {:>12} {:>13} {:>14} {:>14} {:>14}",
        "Level", "Size (MB)", "Write Time", "Open (ms)", "Seq 1K (ms)", "Rand 10K (ms)", "QPS"
    );
    println!("{}", "-".repeat(90));

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let levels = [1, 3, 5, 9, 13, 19];
    let sample_count = 10_000.min(count);
    let mut best_buf: Option<Vec<u8>> = None;

    for level in levels {
        pack.metadata.zstd_level = level;
        let mut buf = Vec::new();
        let t = Instant::now();
        RdictWriter::write_pack(Cursor::new(&mut buf), &pack).expect("write failed");
        let write_time = t.elapsed();
        let size_mb = buf.len() as f64 / 1_048_576.0;

        // Read tests
        let t_open = Instant::now();
        let mut reader = RdictReader::new_with_mode(Cursor::new(buf.clone()), ReadMode::Lazy)
            .expect("open failed");
        let open_ms = t_open.elapsed().as_secs_f64() * 1000.0;

        // Sequential 1000
        let t_seq = Instant::now();
        for i in 0..1000.min(count) {
            let hw = format!("entry{:06}", i);
            let _ = reader.lookup(&hw).unwrap();
        }
        let seq_ms = t_seq.elapsed().as_secs_f64() * 1000.0;

        // Random 10000
        let t_rand = Instant::now();
        for i in 0..sample_count {
            let mut hasher = DefaultHasher::new();
            i.hash(&mut hasher);
            let idx = (hasher.finish() as usize) % count;
            let hw = format!("entry{:06}", idx);
            let _ = reader.lookup(&hw).unwrap();
        }
        let rand_dur = t_rand.elapsed();
        let rand_ms = rand_dur.as_secs_f64() * 1000.0;
        let qps = sample_count as f64 / rand_dur.as_secs_f64();

        println!(
            "{:<8} {:>10.2} {:>10.2?} {:>11.2} {:>12.2} {:>12.2} {:>12.0}",
            level, size_mb, write_time, open_ms, seq_ms, rand_ms, qps
        );

        if level == 19 {
            best_buf = Some(buf);
        }
    }

    let best_buf = best_buf.expect("level 19 not tested");
    pack.metadata.zstd_level = 19;

    let file_size = best_buf.len();
    let path = std::env::temp_dir().join(format!("stress_{count}.rdict"));
    std::fs::write(&path, &best_buf).expect("write file");

    println!();
    println!("=== Detailed Tests (level 19) ===");

    // === 3. Open reader (warm) ===
    let t2 = Instant::now();
    let reader = RdictReader::new_with_mode(Cursor::new(best_buf.clone()), ReadMode::Lazy)
        .expect("open failed");
    let open_time = t2.elapsed();
    println!("[Open] Reader from memory: {open_time:?}");
    println!("       Blocks: {}", reader.manifest().data.block_count);
    println!(
        "       Index blocks: {}",
        reader.manifest().index.block_count
    );

    // === 4. List all headwords ===
    let t6 = Instant::now();
    let headwords = reader.list_headwords().unwrap();
    let list_time = t6.elapsed();
    println!();
    println!("[List] All headwords ({})", headwords.len());
    println!("       Time: {list_time:?}");

    // === 5. Cold start (from file) ===
    drop(reader);
    let t7 = Instant::now();
    let mut cold_reader = RdictReader::open_with_mode(&path, ReadMode::Lazy).expect("open file");
    let cold_open = t7.elapsed();

    let t8 = Instant::now();
    let _ = cold_reader.lookup("entry050000").unwrap();
    let cold_first_lookup = t8.elapsed();

    println!();
    println!("[Cold] Open from file: {cold_open:?}");
    println!("[Cold] First lookup:  {cold_first_lookup:?}");
    println!();

    println!(
        "[Memory] File size: {:.2} MB",
        file_size as f64 / 1_048_576.0
    );

    let _ = std::fs::remove_file(&path);
    println!();
    println!("=== Done ===");
}

fn build_pack(count: usize) -> Pack {
    let entries: Vec<Entry> = (0..count)
        .map(|i| {
            let headword = format!("entry{:06}", i);
            Entry {
                headword,
                see: None,
                tags: if i % 10 == 0 {
                    vec!["level:B1".into(), "exam:IELTS".into()]
                } else {
                    Vec::new()
                },
                media: Vec::new(),
                pron: vec![Pron {
                    lang: Some("en".into()),
                    accent: None,
                    kind: Some("ipa".into()),
                    value: format!("/ˈɛntri{i:06}/"),
                    media: Vec::new(),
                }],
                etymologies: vec![Ety {
                    id: None,
                    root: None,
                    senses: vec![Sense {
                        pos: Some(if i % 3 == 0 { "NOUN" } else { "VERB" }.into()),
                        lemma: None,
                        translations: vec![Translation {
                            lang: Some("zh".into()),
                            value: format!("词条{i}"),
                            pron: Vec::new(),
                        }],
                        forms: if i % 5 == 0 {
                            vec![Form {
                                kind: Some("infl".into()),
                                term: format!("entries{i:06}"),
                                tags: Vec::new(),
                                feats: Some("ud:Number=Plur".into()),
                                pron: Vec::new(),
                            }]
                        } else {
                            Vec::new()
                        },
                        tags: Vec::new(),
                        pron: Vec::new(),
                        definitions: vec![Def::Definition(Definition {
                            id: None,
                            value: format!("Definition of entry number {i}. This is a longer definition to simulate real dictionary content with multiple sentences and enough text to fill a reasonable amount of space per entry."),
                            examples: vec![Example {
                                value: format!("This is example sentence number one for entry {i}, used to demonstrate the target span feature."),
                                translations: vec![Translation {
                                    lang: Some("zh".into()),
                                    value: format!("这是词条{i}的例句翻译。"),
                                    pron: Vec::new(),
                                }],
                                pron: Vec::new(),
                                media: Vec::new(),
                                targets: vec![TargetOccurrence {
                                    spans: vec![TextSpan { offset: 22, length: 7 }],
                                }],
                            }],
                            notes: Vec::new(),
                            media: Vec::new(),
                        })],
                    }],
                }],
                morphology: Vec::new(),
                relations: Vec::new(),
            }
        })
        .collect();

    Pack {
        metadata: PackMetadata {
            id: "00000000-0000-4000-8000-0000000000ff".into(),
            name: "Stress Test Dictionary".into(),
            version: "1.0.0".into(),
            source_lang: "en".into(),
            target_langs: vec!["zh".into()],
            description: Some(format!("{count} entries for stress testing")),
            ..Default::default()
        },
        entries,
        media: Vec::new(),
        cover: None,
    }
}
