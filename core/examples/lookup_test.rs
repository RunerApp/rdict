use rdict::{LookupEntry, RdictReader};

fn main() {
    let path = std::env::args().nth(1).expect("path");
    let mut r = RdictReader::open(&path).expect("open");
    for word in ["abandon", "apple", "2nd", "a", "ability", "grandpa"] {
        match r.lookup(word) {
            Ok(Some(LookupEntry::Decoded(e))) => {
                println!("=== {word} ===");
                let e = &*e;
                println!("  see: {:?}", e.see);
                println!("  pron: {:?}", e.pron);
                println!("  morphology: {:?}", e.morphology);
                println!("  relations: {:?}", e.relations);
                for ety in &e.etymologies {
                    println!("  ety root: {:?}", ety.root);
                    for s in &ety.senses {
                        println!("  sense pos={:?} lemma={:?}", s.pos, s.lemma);
                        for t in &s.translations {
                            println!("    tr[{}]: {}", t.lang.as_deref().unwrap_or("?"), t.value);
                        }
                        for d in &s.definitions {
                            if let rdict::Def::Definition(dd) = d {
                                println!("    def: {}", dd.value);
                                for ex in &dd.examples {
                                    println!("      ex: {}", ex.value);
                                    for t in &ex.translations {
                                        println!(
                                            "        tr[{}]: {}",
                                            t.lang.as_deref().unwrap_or("?"),
                                            t.value
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Some(LookupEntry::Opaque { .. })) => println!("--- {word}: OPAQUE"),
            Ok(None) => println!("--- {word}: not found"),
            Err(e) => println!("--- {word}: ERR {e}"),
        }
    }
}
