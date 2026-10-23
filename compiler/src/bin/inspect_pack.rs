//! Spot-check a compiled .rdict pack: dump metadata and look up entries.
//!
//! Usage: inspect_pack <pack.rdict> [headwords...]

use rdict::{Entry, LookupEntry, RdictReader};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = &args[1];
    let probes = &args[2..];

    let mut reader = RdictReader::open(path).unwrap();
    let m = reader.manifest();
    println!(
        "pack: id={} name={:?} source={} targets={:?} entries={}",
        m.pack.id, m.pack.name, m.pack.source_lang, m.pack.target_langs, m.index.entry_count
    );
    println!("  cover={:?}", m.pack.cover);
    if let Some(r) = &m.runer {
        println!("  runer={r}");
    }

    for p in probes {
        match reader.lookup(p) {
            Ok(Some(LookupEntry::Decoded(e))) => dump_entry(&e),
            Ok(Some(LookupEntry::Opaque { .. })) => {
                println!("\n=== {p}: OPAQUE (decode failed) ===")
            }
            Ok(None) => println!("\n=== {p}: NOT FOUND ==="),
            Err(err) => println!("\n=== {p}: ERROR {err} ==="),
        }
    }
}

fn dump_entry(e: &Entry) {
    println!("\n=== {} ===", e.headword);
    for pr in &e.pron {
        println!("  pron: {}", pr.value);
    }
    for r in &e.relations {
        println!(
            "  rel: {} -> {}",
            r.type_.clone().unwrap_or_default(),
            r.target
        );
    }
    for g in &e.etymologies {
        for s in &g.senses {
            let tr: Vec<String> = s
                .translations
                .iter()
                .map(|t| format!("{}={}", t.lang.clone().unwrap_or_default(), t.value))
                .collect();
            println!("  [{}] {:?}", s.pos.clone().unwrap_or_default(), tr);
            for d in &s.definitions {
                if let rdict::Def::Definition(def) = d {
                    let id = def.id.clone().unwrap_or_default();
                    println!("    def[{}]: {}", id, def.value);
                    for ex in &def.examples {
                        println!("    ex: {}", ex.value);
                        if !ex.targets.is_empty() {
                            let spans: Vec<String> = ex
                                .targets
                                .iter()
                                .flat_map(|o| {
                                    o.spans.iter().map(|s| format!("{}+{}", s.offset, s.length))
                                })
                                .collect();
                            println!("    targets: {}", spans.join(","));
                        }
                        for t in &ex.translations {
                            println!(
                                "    ex_tr: {}={}",
                                t.lang.clone().unwrap_or_default(),
                                t.value
                            );
                        }
                    }
                }
            }
        }
    }
}
