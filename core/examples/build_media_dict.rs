//! Build a test dictionary with real media (audio + image).
//!
//! Run: cargo run --release --example build_media_dict -- <mp3_path> <png_path> <output.rdict>

use rdict::{
    Def, Definition, Entry, Ety, MediaAsset, MediaKind, MediaRef, Pack, PackMetadata, Sense,
};
use std::io::Cursor;

fn main() {
    // Try both repo root (cargo run --example) and core/ (cargo test) CWDs.
    let candidates = ["tests/fixtures", "../tests/fixtures"];
    let mp3_name = "mp3_15s_sample_file_236KB.mp3";
    let png_name = "png_1000x600_sample_file_21KB.png";

    let mp3_path = candidates
        .iter()
        .map(|d| std::path::Path::new(d).join(mp3_name))
        .find(|p| p.exists())
        .unwrap_or_else(|| panic!("cannot find {}", mp3_name));
    let png_path = candidates
        .iter()
        .map(|d| std::path::Path::new(d).join(png_name))
        .find(|p| p.exists())
        .unwrap_or_else(|| panic!("cannot find {}", png_name));
    let output = "media-test.rdict";

    let mp3_bytes = std::fs::read(mp3_path).expect("read mp3");
    let png_bytes = std::fs::read(png_path).expect("read png");

    let mp3_hash = rdict::sha1_hash(&mp3_bytes);
    let png_hash = rdict::sha1_hash(&png_bytes);

    let entry = Entry {
        headword: "hello".into(),
        see: None,
        tags: Vec::new(),
        media: vec![],
        pron: vec![],
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
            name: "Media Test Dictionary".into(),
            source_lang: "en".into(),
            target_langs: vec![],
            ..Default::default()
        },
        entries: vec![entry],
        media: vec![
            MediaAsset {
                kind: MediaKind::Audio,
                ext: "mp3".into(),
                mime: "audio/mpeg".into(),
                compression: rdict::MediaCompression::None,
                bytes: mp3_bytes.clone(),
                path: None,
            },
            MediaAsset {
                kind: MediaKind::Image,
                ext: "png".into(),
                mime: "image/png".into(),
                compression: rdict::MediaCompression::None,
                bytes: png_bytes.clone(),
                path: None,
            },
        ],
        cover: None,
    };

    // Attach audio to the entry-level media (pronunciation audio).
    let mut pack = pack;
    pack.entries[0].media = vec![MediaRef {
        kind: MediaKind::Audio,
        hash: mp3_hash,
        path: None,
        description: Some("Pronunciation audio".into()),
        alt: None,
    }];

    let mut buf = Cursor::new(Vec::new());
    rdict::RdictWriter::write_pack(&mut buf, &pack).expect("write pack");

    std::fs::write(output, buf.into_inner()).expect("write file");
    println!(
        "Written {} ({} bytes) with 1 audio + 1 image",
        output,
        std::fs::metadata(output).unwrap().len()
    );
    println!("Audio hash: {}", rdict::media::hex::encode(&mp3_hash));
    println!("Image hash: {}", rdict::media::hex::encode(&png_hash));
}
