//! Round-trip integration tests: write a pack, read it back, verify
//! entries survive the encode/decode cycle.

mod fixtures;

use fixtures::*;
use rdict::*;
use std::io::{Cursor, Read, Write};

fn write_and_read(pack: &Pack) -> RdictReader<Cursor<Vec<u8>>> {
    let mut buf = Vec::new();
    RdictWriter::write_pack(Cursor::new(&mut buf), pack).expect("write failed");
    RdictReader::new(Cursor::new(buf)).expect("read failed")
}

/// Modify a single byte in the first data block (`data/00000.zst`) of a
/// written pack. This decompresses the block, applies the modification,
/// recompresses, and rewrites the entire ZIP container.
fn modify_first_block_byte<F: FnOnce(&mut [u8])>(buf: &[u8], f: F) -> Vec<u8> {
    use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

    let mut archive = ZipArchive::new(Cursor::new(buf)).expect("open zip");

    // Read all members in order.
    let mut members: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let name = {
            let entry = archive.by_index_raw(i).expect("entry");
            entry.name().to_string()
        };
        let mut data = Vec::new();
        archive
            .by_index(i)
            .expect("open entry")
            .read_to_end(&mut data)
            .expect("read entry");
        members.push((name, data));
    }

    // Find and modify the first data block.
    for (name, data) in &mut members {
        if name == "data/00000.zst" {
            let mut decompressed = zstd::decode_all(data.as_slice()).expect("decompress block");
            f(&mut decompressed);
            *data = zstd::encode_all(decompressed.as_slice(), 19).expect("recompress block");
            break;
        }
    }

    // Rewrite the ZIP.
    let mut output = Vec::new();
    let mut zw = ZipWriter::new(Cursor::new(&mut output));
    let stored = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .large_file(true);
    let mimetype_opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .large_file(false);

    for (name, data) in &members {
        if name == "mimetype" {
            zw.start_file("mimetype", mimetype_opts)
                .expect("start mimetype");
        } else {
            zw.start_file(name, stored).expect("start member");
        }
        zw.write_all(data).expect("write member");
    }
    zw.finish().expect("finish zip");
    output
}

#[test]
fn minimal_roundtrip() {
    let pack = minimal_pack();
    let mut reader = write_and_read(&pack);

    let result = reader.lookup("hello").expect("lookup hello");
    assert!(result.is_some());
    match result.unwrap() {
        LookupEntry::Decoded(entry) => {
            assert_eq!(entry.headword, "hello");
            assert_eq!(entry.etymologies.len(), 1);
            let sense = &entry.etymologies[0].senses[0];
            assert_eq!(sense.pos, Some("NOUN".into()));
            if let Def::Definition(d) = &sense.definitions[0] {
                assert_eq!(d.value, "A greeting");
            } else {
                panic!("expected Definition");
            }
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded entry"),
    }

    let result = reader.lookup("world").expect("lookup world");
    assert!(result.is_some());
}

#[test]
fn missing_headword_returns_none() {
    let pack = minimal_pack();
    let mut reader = write_and_read(&pack);
    assert!(reader.lookup("nonexistent").expect("lookup").is_none());
}

#[test]
fn unicode_headwords_roundtrip() {
    let pack = unicode_pack();
    let mut reader = write_and_read(&pack);

    for hw in &["あ", "い", "愛", "日本"] {
        let result = reader.lookup(hw).expect("lookup");
        assert!(result.is_some(), "should find {}", hw);
    }
    assert!(reader.lookup("か").expect("lookup").is_none());
}

#[test]
fn entry_pron_roundtrip() {
    let mut pack = minimal_pack();
    pack.entries[0] = entry_with_pron();
    let mut reader = write_and_read(&pack);

    match reader.lookup("run").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => {
            assert_eq!(entry.pron.len(), 1);
            let pron = &entry.pron[0];
            assert_eq!(pron.lang, Some("en".into()));
            assert_eq!(pron.accent, Some("General American".into()));
            assert_eq!(pron.kind, Some("ipa".into()));
            assert_eq!(pron.value, "/rʌn/");
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }
}

#[test]
fn tags_roundtrip() {
    let mut pack = minimal_pack();
    pack.entries[0] = entry_with_tags();
    let mut reader = write_and_read(&pack);

    match reader.lookup("test").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => {
            assert_eq!(entry.tags, vec!["exam:IELTS", "level:B2"]);
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }
}

#[test]
fn target_span_roundtrip() {
    let mut pack = minimal_pack();
    pack.entries[0] = entry_with_target_spans();
    let mut reader = write_and_read(&pack);

    match reader.lookup("take").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => {
            let d = match &entry.etymologies[0].senses[0].definitions[0] {
                Def::Definition(d) => d,
                _ => panic!("expected Definition"),
            };
            assert_eq!(d.examples.len(), 1);
            let ex = &d.examples[0];
            assert_eq!(ex.value, "Take care of yourself");
            assert_eq!(ex.targets.len(), 1);
            assert_eq!(ex.targets[0].spans.len(), 1);
            assert_eq!(ex.targets[0].spans[0].offset, 0);
            assert_eq!(ex.targets[0].spans[0].length, 13);
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }
}

#[test]
fn discontinuous_target_roundtrip() {
    let mut pack = minimal_pack();
    pack.entries[0] = entry_with_discontinuous_target();
    let mut reader = write_and_read(&pack);

    match reader.lookup("look").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => {
            let d = match &entry.etymologies[0].senses[0].definitions[0] {
                Def::Definition(d) => d,
                _ => panic!("expected Definition"),
            };
            let ex = &d.examples[0];
            assert_eq!(ex.targets.len(), 1);
            assert_eq!(ex.targets[0].spans.len(), 2);
            assert_eq!(ex.targets[0].spans[0].offset, 0);
            assert_eq!(ex.targets[0].spans[0].length, 4);
            assert_eq!(ex.targets[0].spans[1].offset, 13);
            assert_eq!(ex.targets[0].spans[1].length, 2);
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }
}

#[test]
fn form_pron_roundtrip() {
    let mut pack = minimal_pack();
    pack.entries[0] = entry_with_form_pron();
    let mut reader = write_and_read(&pack);

    match reader.lookup("run").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => {
            let sense = &entry.etymologies[0].senses[0];
            assert_eq!(sense.forms.len(), 1);
            let form = &sense.forms[0];
            assert_eq!(form.term, "ran");
            assert_eq!(form.kind, Some("infl".into()));
            assert_eq!(
                form.feats,
                Some("ud:ConjugationForm=Ta|ud:ConjugationType=Godan".into())
            );
            assert_eq!(form.pron.len(), 1);
            assert_eq!(form.pron[0].value, "/ræn/");
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }
}

#[test]
fn relations_roundtrip() {
    let pack = pack_with_relations();
    let mut reader = write_and_read(&pack);

    // "hello" should have two relations: see → world, syn → world.
    match reader.lookup("hello").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => {
            assert_eq!(entry.relations.len(), 2);
            assert_eq!(entry.relations[0].type_, Some("see".into()));
            assert_eq!(entry.relations[0].target, "world");
            assert_eq!(entry.relations[1].type_, Some("syn".into()));
            assert_eq!(entry.relations[1].target, "world");
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }

    // "world" should have one relation: syn → hello (compiler-synthesized).
    match reader.lookup("world").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => {
            assert_eq!(entry.relations.len(), 1);
            assert_eq!(entry.relations[0].type_, Some("syn".into()));
            assert_eq!(entry.relations[0].target, "hello");
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }
}

#[test]
fn empty_pron_rejected() {
    let mut pack = minimal_pack();
    pack.entries[0].pron = vec![Pron {
        lang: None,
        accent: None,
        kind: Some("ipa".into()),
        value: "".into(),
        media: Vec::new(),
    }];
    let mut buf = Vec::new();
    assert!(RdictWriter::write_pack(Cursor::new(&mut buf), &pack).is_err());
}

#[test]
fn duplicate_headword_rejected() {
    let mut pack = minimal_pack();
    pack.entries.push(simple_entry("hello", "duplicate"));
    let mut buf = Vec::new();
    assert!(RdictWriter::write_pack(Cursor::new(&mut buf), &pack).is_err());
}

#[test]
fn empty_headword_rejected() {
    let mut pack = minimal_pack();
    pack.entries.push(simple_entry("", "empty headword"));
    let mut buf = Vec::new();
    assert!(RdictWriter::write_pack(Cursor::new(&mut buf), &pack).is_err());
}

#[test]
fn multiple_entries_sorted() {
    let mut pack = minimal_pack();
    pack.entries = vec![
        simple_entry("zebra", "an animal"),
        simple_entry("apple", "a fruit"),
        simple_entry("mango", "a fruit"),
    ];
    let mut reader = write_and_read(&pack);
    // All should be findable regardless of input order.
    for hw in &["apple", "mango", "zebra"] {
        assert!(reader.lookup(hw).expect("lookup").is_some());
    }
}

#[test]
fn redirect_entry() {
    let mut pack = minimal_pack();
    pack.entries.push(Entry {
        headword: "colour".into(),
        see: Some("color".into()),
        tags: Vec::new(),
        media: Vec::new(),
        pron: Vec::new(),
        etymologies: Vec::new(),
        morphology: Vec::new(),
        relations: Vec::new(),
    });
    let mut reader = write_and_read(&pack);
    match reader.lookup("colour").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => {
            assert_eq!(entry.see, Some("color".into()));
            assert!(entry.etymologies.is_empty());
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }
}

#[test]
fn repeated_target_occurrences_roundtrip() {
    let mut pack = minimal_pack();
    pack.entries[0] = entry_with_repeated_targets();
    let mut reader = write_and_read(&pack);

    match reader.lookup("ran").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => {
            let d = match &entry.etymologies[0].senses[0].definitions[0] {
                Def::Definition(d) => d,
                _ => panic!("expected Definition"),
            };
            let ex = &d.examples[0];
            assert_eq!(ex.value, "He ran and ran.");
            assert_eq!(ex.targets.len(), 2, "expected two occurrences");
            // First occurrence: "ran" at offset 3.
            assert_eq!(ex.targets[0].spans.len(), 1);
            assert_eq!(ex.targets[0].spans[0].offset, 3);
            assert_eq!(ex.targets[0].spans[0].length, 3);
            // Second occurrence: "ran" at offset 11.
            assert_eq!(ex.targets[1].spans.len(), 1);
            assert_eq!(ex.targets[1].spans[0].offset, 11);
            assert_eq!(ex.targets[1].spans[0].length, 3);
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }
}

#[test]
fn multiple_zstd_blocks_roundtrip() {
    let pack = multi_block_pack();
    let mut reader = write_and_read(&pack);

    // Verify multiple data blocks were created.
    assert!(
        reader.manifest().data.block_count > 1,
        "expected multiple blocks, got {}",
        reader.manifest().data.block_count
    );

    // Verify entries in different blocks can be looked up.
    for i in 0..50 {
        let hw = format!("entry{:03}", i);
        assert!(
            reader.lookup(&hw).expect("lookup").is_some(),
            "should find {}",
            hw
        );
    }
}

#[test]
fn large_entry_roundtrip() {
    // The writer enables ZIP64 extended information via `large_file(true)`
    // for all non-mimetype members. This test verifies that a large entry
    // can be written and read back correctly through that code path.
    let mut pack = minimal_pack();
    let large_def = "x".repeat(100_000);
    pack.entries[0] = simple_entry("bigentry", &large_def);
    let mut reader = write_and_read(&pack);

    match reader.lookup("bigentry").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => {
            let d = match &entry.etymologies[0].senses[0].definitions[0] {
                Def::Definition(d) => d,
                _ => panic!("expected Definition"),
            };
            assert_eq!(d.value.len(), 100_000);
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }
}

#[test]
fn media_hash_path_roundtrip() {
    let pack = pack_with_media();
    let mut reader = write_and_read(&pack);

    // Verify media manifest is present.
    let manifest = reader
        .media_manifest()
        .expect("read media manifest")
        .expect("manifest should exist");
    assert_eq!(manifest.entries.len(), 1);

    let entry = &manifest.entries[0];
    assert_eq!(entry.kind, "audio");
    assert_eq!(entry.ext, "mp3");
    assert_eq!(entry.mime, "audio/mpeg");
    assert_eq!(entry.compression, "none");
    assert_eq!(entry.size, 19); // "FAKE_MP3_DATA_BYTES".len()
    assert_eq!(entry.hash.len(), 40); // SHA-1 hex

    // Read media bytes by (kind, hash).
    let media_bytes = reader
        .read_media(&entry.kind, &entry.hash)
        .expect("read media file");
    assert_eq!(media_bytes, b"FAKE_MP3_DATA_BYTES");

    // Verify the entry has a media ref.
    match reader.lookup("hello").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => {
            assert_eq!(entry.media.len(), 1);
            assert_eq!(entry.media[0].kind, MediaKind::Audio);
            assert_eq!(
                entry.media[0].description,
                Some("pronunciation audio".into())
            );
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }
}

#[test]
fn tag_index_roundtrip() {
    let pack = pack_with_tags();
    let mut reader = write_and_read(&pack);

    // Verify manifest indicates tag index exists.
    let tags_meta = reader.manifest().tags.as_ref().expect("tag index metadata");
    assert!(tags_meta.has_index);
    assert!(tags_meta.tag_count > 0);

    // Verify tag_table in manifest lists the tags in pool order.
    let tag_table = reader
        .manifest()
        .tag_table
        .as_ref()
        .expect("tag_table should exist");
    // Pool order: interned from pack.entries (original order: hello, world, test).
    // hello: "exam:IELTS"(1), "level:B2"(2); world: "exam:IELTS"(dup), "level:C1"(3);
    // test: "exam:TOEFL"(4), "level:B2"(dup)
    assert_eq!(
        tag_table,
        &["exam:IELTS", "level:B2", "level:C1", "exam:TOEFL"]
    );

    // Decode the tag index.
    let tag_index = reader
        .decode_tag_index()
        .expect("read tag index")
        .expect("tag index should exist");

    // Sorted entries: hello(0), test(1), world(2)
    // Tag 1 (exam:IELTS): hello(0), world(2)
    // Tag 2 (level:B2): hello(0), test(1)
    // Tag 3 (level:C1): world(2)
    // Tag 4 (exam:TOEFL): test(1)
    assert_eq!(tag_index.len(), 4);
    assert_eq!(tag_index[0], (1, vec![0, 2]));
    assert_eq!(tag_index[1], (2, vec![0, 1]));
    assert_eq!(tag_index[2], (3, vec![2]));
    assert_eq!(tag_index[3], (4, vec![1]));
}

#[test]
fn morph_index_roundtrip() {
    let mut pack = minimal_pack();
    pack.entries[0] = entry_with_morph_feats();
    let mut reader = write_and_read(&pack);

    // Verify manifest indicates morph index exists.
    let morph_meta = reader
        .manifest()
        .morph
        .as_ref()
        .expect("morph index metadata");
    assert!(morph_meta.has_index);
    assert_eq!(morph_meta.key_count, 2);

    // Decode the morph index.
    let morph_index = reader
        .decode_morph_index()
        .expect("read morph index")
        .expect("morph index should exist");

    // Entries sorted: world(0), 走る(1).
    // feats = "ud:ConjugationType=Godan|ud:ConjugationForm=Ta"
    // Parsed pairs sorted in BTreeMap:
    //   "ud:ConjugationForm=Ta" → [1]
    //   "ud:ConjugationType=Godan" → [1]
    assert_eq!(morph_index.len(), 2);
    assert_eq!(morph_index[0].0, "ud:ConjugationForm=Ta");
    assert_eq!(morph_index[0].1, vec![1]);
    assert_eq!(morph_index[1].0, "ud:ConjugationType=Godan");
    assert_eq!(morph_index[1].1, vec![1]);
}

#[test]
fn unknown_entry_flag_returns_opaque() {
    let pack = minimal_pack();
    let mut buf = Vec::new();
    RdictWriter::write_pack(Cursor::new(&mut buf), &pack).unwrap();

    // Corrupt the first entry's flags byte by setting an unknown bit.
    let corrupted_buf = modify_first_block_byte(&buf, |block| {
        if !block.is_empty() {
            block[0] |= 0x80;
        }
    });

    let mut reader = RdictReader::new(Cursor::new(corrupted_buf)).expect("read failed");

    // The first entry ("hello") should be opaque.
    match reader.lookup("hello").expect("lookup").unwrap() {
        LookupEntry::Opaque { .. } => {}
        LookupEntry::Decoded(_) => panic!("expected opaque for unknown entry flag"),
    }

    // The second entry ("world") should still be decodable.
    match reader.lookup("world").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => assert_eq!(entry.headword, "world"),
        LookupEntry::Opaque { .. } => panic!("expected decoded for world"),
    }
}

#[test]
fn unknown_def_kind_returns_opaque() {
    let pack = minimal_pack();
    let mut buf = Vec::new();
    RdictWriter::write_pack(Cursor::new(&mut buf), &pack).unwrap();

    // Byte 8 of the decompressed block is the Def kind for "hello".
    let corrupted_buf = modify_first_block_byte(&buf, |block| {
        if block.len() > 8 {
            block[8] = 0xFF; // unknown Def kind
        }
    });

    let mut reader = RdictReader::new(Cursor::new(corrupted_buf)).expect("read failed");

    // "hello" should be opaque (decode error → reader converts to Opaque).
    match reader.lookup("hello").expect("lookup").unwrap() {
        LookupEntry::Opaque { .. } => {}
        LookupEntry::Decoded(_) => panic!("expected opaque for unknown Def kind"),
    }

    // "world" should still be decodable.
    match reader.lookup("world").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => assert_eq!(entry.headword, "world"),
        LookupEntry::Opaque { .. } => panic!("expected decoded for world"),
    }
}

#[test]
fn morphology_roundtrip() {
    let mut pack = minimal_pack();
    pack.entries[0] = entry_with_morphology();
    let mut reader = write_and_read(&pack);

    match reader.lookup("unhappiness").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => {
            assert_eq!(entry.morphology.len(), 3);
            assert_eq!(entry.morphology[0].kind, Some("prefix".into()));
            assert_eq!(entry.morphology[0].term, "un-");
            assert_eq!(entry.morphology[1].kind, Some("root".into()));
            assert_eq!(entry.morphology[1].term, "happy");
            assert_eq!(entry.morphology[2].kind, Some("suffix".into()));
            assert_eq!(entry.morphology[2].term, "-ness");
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }
}

#[test]
fn prefix_search_case_insensitive() {
    let mut pack = minimal_pack();
    // Add entries with mixed case: Apple, apple, apply, banana
    pack.entries = vec![
        simple_entry("apple", "a fruit"),
        simple_entry("Apple", "a company"),
        simple_entry("apply", "to put on"),
        simple_entry("banana", "a yellow fruit"),
    ];
    let reader = write_and_read(&pack);

    // Prefix "app" should match apple, Apple, apply (case-insensitive)
    let results = reader.prefix("app", 20);
    assert_eq!(
        results.len(),
        3,
        "expected 3 matches for 'app': {:?}",
        results
    );
    // Case-folded sort: Apple < apple < apply
    assert_eq!(results[0], "Apple");
    assert_eq!(results[1], "apple");
    assert_eq!(results[2], "apply");

    // Prefix "APP" should also match (case-insensitive)
    let results = reader.prefix("APP", 20);
    assert_eq!(results.len(), 3);

    // Prefix "ban" should match banana
    let results = reader.prefix("ban", 20);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], "banana");

    // Prefix "xyz" should match nothing
    let results = reader.prefix("xyz", 20);
    assert!(results.is_empty());

    // Limit works
    let results = reader.prefix("app", 2);
    assert_eq!(results.len(), 2);

    // Limit 0 returns empty
    let results = reader.prefix("app", 0);
    assert!(results.is_empty());
}

#[test]
fn case_sensitive_exact_lookup() {
    let mut pack = minimal_pack();
    pack.entries = vec![
        simple_entry("apple", "a fruit"),
        simple_entry("Apple", "a company"),
    ];
    let mut reader = write_and_read(&pack);

    // Both should be findable with exact case
    match reader.lookup("apple").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => assert_eq!(entry.headword, "apple"),
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }
    match reader.lookup("Apple").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => assert_eq!(entry.headword, "Apple"),
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }

    // Wrong case should not match
    assert!(reader.lookup("APPLE").expect("lookup").is_none());
    assert!(reader.lookup("aPPLE").expect("lookup").is_none());
}

#[test]
fn ety_root_roundtrip() {
    let mut pack = minimal_pack();
    pack.entries[0] = entry_with_ety_root();
    let mut reader = write_and_read(&pack);

    match reader.lookup("bank").expect("lookup").unwrap() {
        LookupEntry::Decoded(entry) => {
            assert_eq!(entry.etymologies.len(), 1);
            assert_eq!(entry.etymologies[0].root, Some("bancus".into()));
        }
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    }
}

#[test]
fn real_media_roundtrip() {
    use rdict::{MediaAsset, MediaCompression, MediaKind, MediaRef, RdictWriter};
    use std::io::Cursor;

    let mp3_path = "../tests/fixtures/mp3_15s_sample_file_236KB.mp3";
    let png_path = "../tests/fixtures/png_1000x600_sample_file_21KB.png";

    // Skip test if fixtures not present (e.g. running from a different CWD).
    let mp3_bytes = match std::fs::read(mp3_path) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("skipping real_media_roundtrip: fixture not found");
            return;
        }
    };
    let png_bytes = match std::fs::read(png_path) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("skipping real_media_roundtrip: fixture not found");
            return;
        }
    };

    let mp3_hash = rdict::sha1_hash(&mp3_bytes);
    let png_hash = rdict::sha1_hash(&png_bytes);

    let entry = Entry {
        headword: "hello".into(),
        see: None,
        tags: Vec::new(),
        media: vec![MediaRef {
            kind: MediaKind::Audio,
            hash: mp3_hash,
            path: None,
            description: Some("Pronunciation audio".into()),
            alt: None,
        }],
        pron: Vec::new(),
        etymologies: vec![Ety {
            id: None,
            root: None,
            senses: vec![Sense {
                pos: Some("INTJ".into()),
                lemma: None,
                translations: Vec::new(),
                forms: Vec::new(),
                tags: Vec::new(),
                pron: Vec::new(),
                definitions: vec![Def::Definition(Definition {
                    id: None,
                    value: "Used as a greeting.".into(),
                    examples: Vec::new(),
                    notes: Vec::new(),
                    media: vec![MediaRef {
                        kind: MediaKind::Image,
                        hash: png_hash,
                        path: None,
                        description: Some("Greeting illustration".into()),
                        alt: Some("A greeting illustration".into()),
                    }],
                })],
            }],
        }],
        morphology: Vec::new(),
        relations: Vec::new(),
    };

    let pack = Pack {
        metadata: PackMetadata {
            name: "Media Test".into(),
            source_lang: "en".into(),
            ..Default::default()
        },
        entries: vec![entry],
        media: vec![
            MediaAsset {
                kind: MediaKind::Audio,
                ext: "mp3".into(),
                mime: "audio/mpeg".into(),
                compression: MediaCompression::None,
                bytes: mp3_bytes.clone(),
                path: None,
            },
            MediaAsset {
                kind: MediaKind::Image,
                ext: "png".into(),
                mime: "image/png".into(),
                compression: MediaCompression::None,
                bytes: png_bytes.clone(),
                path: None,
            },
        ],
        cover: None,
    };

    let mut buf = Cursor::new(Vec::new());
    RdictWriter::write_pack(&mut buf, &pack).expect("write pack");

    let mut reader = RdictReader::new(Cursor::new(buf.into_inner())).expect("open");

    // Verify media manifest
    let manifest = reader
        .media_manifest()
        .expect("read manifest")
        .expect("manifest should exist");
    assert_eq!(manifest.entries.len(), 2);
    assert_eq!(manifest.entries[0].kind, "audio");
    assert_eq!(manifest.entries[0].size, mp3_bytes.len() as u64);
    assert_eq!(manifest.entries[1].kind, "image");
    assert_eq!(manifest.entries[1].size, png_bytes.len() as u64);

    // Verify entry-level audio media
    let entry = match reader.lookup("hello").expect("lookup").unwrap() {
        LookupEntry::Decoded(e) => e,
        LookupEntry::Opaque { .. } => panic!("expected decoded"),
    };
    assert_eq!(entry.media.len(), 1);
    assert_eq!(entry.media[0].kind, MediaKind::Audio);
    assert_eq!(entry.media[0].hash, mp3_hash);

    // Verify definition-level image media
    let def = match &entry.etymologies[0].senses[0].definitions[0] {
        Def::Definition(d) => d,
        _ => panic!("expected Definition"),
    };
    assert_eq!(def.media.len(), 1);
    assert_eq!(def.media[0].kind, MediaKind::Image);
    assert_eq!(def.media[0].hash, png_hash);

    // Read audio bytes back and verify content
    let audio_hash_hex = rdict::media::hex::encode(&mp3_hash);
    let audio_data = reader
        .read_media("audio", &audio_hash_hex)
        .expect("read audio");
    assert_eq!(audio_data, mp3_bytes);

    // Read image bytes back and verify content
    let image_hash_hex = rdict::media::hex::encode(&png_hash);
    let image_data = reader
        .read_media("image", &image_hash_hex)
        .expect("read image");
    assert_eq!(image_data, png_bytes);
}

#[test]
fn zstd_media_roundtrip() {
    // Media with zstd compression: the asset.bytes are the original
    // (uncompressed) data; the writer compresses them for storage.
    let original_data = b"REPEATABLE_TEXT_DATA_FOR_ZSTD_COMPRESSION_TEST_".repeat(20);

    let hash = rdict::media::sha1_hash(&original_data);
    let hash_hex = rdict::media::hex::encode(&hash);

    let pack = Pack {
        metadata: PackMetadata {
            id: "00000000-0000-4000-8000-000000000020".into(),
            name: "Zstd Media Test".into(),
            version: "1.0.0".into(),
            source_lang: "en".into(),
            target_langs: vec!["zh".into()],
            ..Default::default()
        },
        entries: vec![Entry {
            headword: "hello".into(),
            see: None,
            tags: Vec::new(),
            media: vec![MediaRef {
                kind: MediaKind::Image,
                hash,
                path: None,
                description: Some("zstd-compressed text".into()),
                alt: None,
            }],
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
                        value: "A greeting".into(),
                        examples: Vec::new(),
                        notes: Vec::new(),
                        media: Vec::new(),
                    })],
                }],
            }],
            morphology: Vec::new(),
            relations: Vec::new(),
        }],
        media: vec![MediaAsset {
            kind: MediaKind::Image,
            ext: "txt".into(),
            mime: "text/plain".into(),
            compression: MediaCompression::Zstd,
            bytes: original_data.clone(),
            path: None,
        }],
        cover: None,
    };

    let mut buf = Cursor::new(Vec::new());
    RdictWriter::write_pack(&mut buf, &pack).expect("write pack");

    let mut reader = RdictReader::new(Cursor::new(buf.into_inner())).expect("open");

    // Verify manifest: compressed size < uncompressed size
    let manifest = reader
        .media_manifest()
        .expect("read manifest")
        .expect("manifest should exist");
    assert_eq!(manifest.entries.len(), 1);
    let entry = &manifest.entries[0];
    assert_eq!(entry.compression, "zstd");
    assert_eq!(entry.kind, "image");
    assert_eq!(entry.uncompressed_size, original_data.len() as u64);
    assert!(
        entry.size < entry.uncompressed_size,
        "compressed size {} should be < uncompressed {}",
        entry.size,
        entry.uncompressed_size
    );
    assert_eq!(entry.hash, hash_hex);

    // read_media returns decompressed bytes
    let data = reader
        .read_media("image", &hash_hex)
        .expect("read zstd media");
    assert_eq!(data, original_data);

    // extract_media streams through zstd decoder to file
    let tmp = tempfile::NamedTempFile::new().expect("create tmp");
    let path = tmp.path();
    let written = reader
        .extract_media("image", &hash_hex, path)
        .expect("extract zstd media");
    assert_eq!(written, original_data.len() as u64);

    let file_data = std::fs::read(path).expect("read extracted file");
    assert_eq!(file_data, original_data);

    // mediaInfo returns correct metadata
    let info = reader
        .media_info("image", &hash_hex)
        .expect("media_info")
        .expect("should find entry");
    assert_eq!(info.compression, "zstd");
    assert_eq!(info.uncompressed_size, original_data.len() as u64);
}

#[test]
fn cover_image_roundtrip() {
    let cover_bytes = b"FAKE_PNG_DATA".to_vec();
    let mut pack = minimal_pack();
    pack.metadata.cover = Some("cover.png".into());
    pack.cover = Some(cover_bytes.clone());

    let mut reader = write_and_read(&pack);

    // Manifest should have cover field.
    assert_eq!(reader.manifest().pack.cover, Some("cover.png".into()));

    // read_cover returns the bytes.
    let data = reader
        .read_cover()
        .expect("read cover")
        .expect("cover exists");
    assert_eq!(data, cover_bytes);
}

#[test]
fn no_cover_returns_none() {
    let pack = minimal_pack();
    let mut reader = write_and_read(&pack);
    assert_eq!(reader.manifest().pack.cover, None);
    assert!(reader.read_cover().expect("read cover").is_none());
}
