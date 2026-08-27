use rdict::*;
use std::io::Cursor;

fn main() {
    let mut buf = Vec::new();
    let pack = Pack {
        metadata: PackMetadata::default(),
        entries: vec![Entry {
            headword: "test".into(),
            see: None,
            tags: vec![],
            media: vec![],
            pron: vec![],
            etymologies: vec![Ety {
                id: None,
                root: None,
                senses: vec![Sense {
                    pos: Some("NOUN".into()),
                    lemma: None,
                    translations: vec![],
                    forms: vec![],
                    tags: vec![],
                    pron: vec![],
                    definitions: vec![
                        Def::Definition(Definition {
                            id: None,
                            value: "a test".into(),
                            examples: vec![],
                            notes: vec![],
                            media: vec![],
                        }),
                        Def::Group(Group {
                            id: None,
                            description: Some("grouped".into()),
                            definitions: vec![],
                        }),
                    ],
                }],
            }],
            morphology: vec![],
            relations: vec![],
        }],
        media: vec![],
        cover: None,
    };
    RdictWriter::write_pack(Cursor::new(&mut buf), &pack).unwrap();
    let mut reader = RdictReader::new(Cursor::new(buf)).unwrap();
    if let LookupEntry::Decoded(entry) = reader.lookup("test").unwrap().unwrap() {
        let json = serde_json::to_string_pretty(&*entry).unwrap();
        println!("{}", json);
    }
}
