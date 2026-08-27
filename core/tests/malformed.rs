//! Malformed-input tests: verify the reader rejects corrupted packs.

mod fixtures;

use fixtures::*;
use rdict::*;
use std::io::Cursor;

#[test]
fn bad_mimetype_rejected() {
    let pack = minimal_pack();
    let mut buf = Vec::new();
    RdictWriter::write_pack(Cursor::new(&mut buf), &pack).unwrap();
    // Corrupt the mimetype bytes (they're right at the start after the
    // ZIP local file header, which is ~30 bytes).
    // Find "application/rdict" and flip a byte.
    let target = b"application/rdict";
    let pos = buf
        .windows(target.len())
        .position(|w| w == target)
        .expect("mimetype not found");
    buf[pos] = b'X';
    let result = RdictReader::new(Cursor::new(&buf));
    assert!(result.is_err());
}

#[test]
fn truncated_zip_rejected() {
    let pack = minimal_pack();
    let mut buf = Vec::new();
    RdictWriter::write_pack(Cursor::new(&mut buf), &pack).unwrap();
    buf.truncate(buf.len() / 2);
    let result = RdictReader::new(Cursor::new(&buf));
    assert!(result.is_err());
}

#[test]
fn target_span_out_of_bounds_rejected() {
    let mut pack = minimal_pack();
    pack.entries[0] = Entry {
        headword: "bad".into(),
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
                    value: "short".into(),
                    examples: vec![Example {
                        value: "short".into(),
                        translations: Vec::new(),
                        pron: Vec::new(),
                        media: Vec::new(),
                        targets: vec![TargetOccurrence {
                            spans: vec![TextSpan {
                                offset: 0,
                                length: 100, // exceeds value length
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
    };
    let mut buf = Vec::new();
    assert!(RdictWriter::write_pack(Cursor::new(&mut buf), &pack).is_err());
}

#[test]
fn target_span_zero_length_rejected() {
    let mut pack = minimal_pack();
    pack.entries[0] = Entry {
        headword: "bad".into(),
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
                    examples: vec![Example {
                        value: "test".into(),
                        translations: Vec::new(),
                        pron: Vec::new(),
                        media: Vec::new(),
                        targets: vec![TargetOccurrence {
                            spans: vec![TextSpan {
                                offset: 0,
                                length: 0,
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
    };
    let mut buf = Vec::new();
    assert!(RdictWriter::write_pack(Cursor::new(&mut buf), &pack).is_err());
}

#[test]
fn overlapping_target_spans_rejected() {
    let mut pack = minimal_pack();
    pack.entries[0] = Entry {
        headword: "bad".into(),
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
                    value: "abcdef".into(),
                    examples: vec![Example {
                        value: "abcdef".into(),
                        translations: Vec::new(),
                        pron: Vec::new(),
                        media: Vec::new(),
                        targets: vec![TargetOccurrence {
                            spans: vec![
                                TextSpan {
                                    offset: 0,
                                    length: 3,
                                },
                                TextSpan {
                                    offset: 2, // overlaps with previous (ends at 3)
                                    length: 2,
                                },
                            ],
                        }],
                    }],
                    notes: Vec::new(),
                    media: Vec::new(),
                })],
            }],
        }],
        morphology: Vec::new(),
        relations: Vec::new(),
    };
    let mut buf = Vec::new();
    assert!(RdictWriter::write_pack(Cursor::new(&mut buf), &pack).is_err());
}

#[test]
fn non_utf8_boundary_target_rejected() {
    let mut pack = minimal_pack();
    // "世界" is 6 bytes: e4 b8 96 e7 95 8c. Offset 1 is mid-codepoint.
    pack.entries[0] = Entry {
        headword: "bad".into(),
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
                    value: "世界".into(),
                    examples: vec![Example {
                        value: "世界".into(),
                        translations: Vec::new(),
                        pron: Vec::new(),
                        media: Vec::new(),
                        targets: vec![TargetOccurrence {
                            spans: vec![TextSpan {
                                offset: 1, // mid-codepoint
                                length: 2,
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
    };
    let mut buf = Vec::new();
    assert!(RdictWriter::write_pack(Cursor::new(&mut buf), &pack).is_err());
}

/// Corrupt a byte inside the `strings.tbl` member data of a written pack.
/// The `strings.tbl` magic `RDST` is located in the raw ZIP buffer, then
/// `offset` bytes after the magic are replaced with `new_value`.
fn corrupt_strings_tbl(buf: &[u8], offset: usize, new_value: u8) -> Vec<u8> {
    let magic = b"RDST";
    let pos = buf
        .windows(magic.len())
        .position(|w| w == magic)
        .expect("RDST magic not found in buffer");
    let mut out = buf.to_vec();
    out[pos + offset] = new_value;
    out
}

#[test]
fn malformed_utf8_in_strings_tbl_rejected() {
    let pack = minimal_pack();
    let mut buf = Vec::new();
    RdictWriter::write_pack(Cursor::new(&mut buf), &pack).unwrap();

    // strings.tbl layout after "RDST": 2B version + 6×4B counts = 26 bytes,
    // then the first string: varint(1) + 'n' (0x6E).
    // Offset 27 from RDST is the 'n' byte. Replace with 0xFF (invalid UTF-8).
    let corrupted = corrupt_strings_tbl(&buf, 27, 0xFF);
    let result = RdictReader::new(Cursor::new(&corrupted));
    assert!(result.is_err(), "reader should reject invalid UTF-8");
}

#[test]
fn non_canonical_varint_in_strings_tbl_rejected() {
    let pack = minimal_pack();
    let mut buf = Vec::new();
    RdictWriter::write_pack(Cursor::new(&mut buf), &pack).unwrap();

    // The first string in strings.tbl is "n" encoded as [0x01, 0x6E].
    // Replace with [0x81, 0x00] — a non-canonical varint encoding of 1
    // (uses 2 bytes where 1 suffices). We overwrite both bytes.
    let magic = b"RDST";
    let pos = buf
        .windows(magic.len())
        .position(|w| w == magic)
        .expect("RDST magic not found");
    let mut corrupted = buf.clone();
    // Offset 26 from RDST is the varint length byte; 27 is the string byte.
    corrupted[pos + 26] = 0x81;
    corrupted[pos + 27] = 0x00;

    let result = RdictReader::new(Cursor::new(&corrupted));
    assert!(result.is_err(), "reader should reject non-canonical varint");
}
