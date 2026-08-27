use std::env;
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::path::PathBuf;

use rdict::{Def, Definition, Entry, Pack, PackMetadata, RdictWriter, Relation};
use serde::Deserialize;

#[derive(Deserialize)]
struct YamlDefinition {
    #[serde(default)]
    id: Option<String>,
    value: String,
    #[serde(default)]
    examples: Vec<rdict::Example>,
    #[serde(default)]
    notes: Vec<rdict::Note>,
    #[serde(default)]
    media: Vec<rdict::MediaRef>,
}

#[derive(Deserialize)]
struct YamlSense {
    #[serde(default)]
    pos: Option<String>,
    #[serde(default)]
    lemma: Option<String>,
    #[serde(default)]
    translations: Vec<rdict::Translation>,
    #[serde(default)]
    forms: Vec<rdict::Form>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    pron: Vec<rdict::Pron>,
    #[serde(default)]
    definitions: Vec<YamlDefinition>,
}

#[derive(Deserialize)]
struct YamlEty {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    root: Option<String>,
    #[serde(default)]
    senses: Vec<YamlSense>,
}

#[derive(Deserialize)]
struct YamlRelation {
    #[serde(default)]
    type_: Option<String>,
    target: String,
}

#[derive(Deserialize)]
struct YamlEntry {
    headword: String,
    #[serde(default)]
    see: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    media: Vec<rdict::MediaRef>,
    #[serde(default)]
    pron: Vec<rdict::Pron>,
    #[serde(default)]
    etymologies: Vec<YamlEty>,
    #[serde(default)]
    morphology: Vec<rdict::Morpheme>,
    #[serde(default)]
    relations: Vec<YamlRelation>,
    /// Source-format only: component words that should carry a phrase
    /// back-reference to this entry. Not written to the binary AST.
    #[serde(default)]
    phrase_of: Vec<String>,
}

#[derive(Deserialize)]
struct YamlDoc {
    #[serde(default)]
    entries: Vec<YamlEntry>,
}

fn convert_entry(y: YamlEntry, default_lang: Option<&str>) -> Entry {
    Entry {
        headword: y.headword,
        see: y.see,
        tags: y.tags,
        media: y.media,
        pron: y.pron,
        etymologies: y
            .etymologies
            .into_iter()
            .map(|e| rdict::Ety {
                id: e.id,
                root: e.root,
                senses: e
                    .senses
                    .into_iter()
                    .map(|s| rdict::Sense {
                        pos: s.pos,
                        lemma: s.lemma,
                        translations: fill_translation_langs(s.translations, default_lang),
                        forms: s.forms,
                        tags: s.tags,
                        pron: s.pron,
                        definitions: s
                            .definitions
                            .into_iter()
                            .map(|d| {
                                Def::Definition(Definition {
                                    id: d.id,
                                    value: d.value,
                                    examples: fill_example_langs(d.examples, default_lang),
                                    notes: d.notes,
                                    media: d.media,
                                })
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
        morphology: y.morphology,
        relations: y
            .relations
            .into_iter()
            .map(|r| Relation {
                type_: r.type_,
                target: r.target,
            })
            .collect(),
    }
}

fn fill_translation_langs(
    translations: Vec<rdict::Translation>,
    default_lang: Option<&str>,
) -> Vec<rdict::Translation> {
    translations
        .into_iter()
        .map(|mut t| {
            if t.lang.is_none() {
                t.lang = default_lang.map(|s| s.to_string());
            }
            t
        })
        .collect()
}

fn fill_example_langs(
    examples: Vec<rdict::Example>,
    default_lang: Option<&str>,
) -> Vec<rdict::Example> {
    examples
        .into_iter()
        .map(|ex| rdict::Example {
            value: ex.value,
            translations: fill_translation_langs(ex.translations, default_lang),
            pron: ex.pron,
            media: ex.media,
            targets: ex.targets,
        })
        .collect()
}

/// Symmetric relation types: the compiler auto-fills the reverse record.
const SYMMETRIC_TYPES: &[&str] = &["syn", "ant"];

/// Process inline relations on entries:
/// 1. Distribute `phrase_of` → add `Relation { type: phrase, target: <phrase> }`
///    to each listed component entry.
/// 2. Auto-fill symmetric relations (syn/ant): if A has a relation to B,
///    ensure B has a relation back to A.
/// 3. Validate: target exists, no self-relation, no duplicate (type, target).
fn process_relations(
    entries: &mut [Entry],
    phrase_of_map: &std::collections::HashMap<String, Vec<String>>,
) {
    let headword_set: std::collections::HashSet<String> =
        entries.iter().map(|e| e.headword.clone()).collect();

    // Build a mutable index for quick entry lookup by headword.
    let mut by_headword: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        by_headword.insert(e.headword.clone(), i);
    }

    // 1. Distribute phrase_of references.
    for (phrase_headword, components) in phrase_of_map {
        for component in components {
            // Validate component exists.
            if !headword_set.contains(component) {
                eprintln!(
                    "Error: phrase_of on '{}' references non-existent component '{}'",
                    phrase_headword, component
                );
                std::process::exit(1);
            }
            // Find the component entry and add a phrase relation.
            if let Some(&idx) = by_headword.get(component) {
                let rel = Relation {
                    type_: Some("phrase".into()),
                    target: phrase_headword.clone(),
                };
                // Dedupe: only add if not already present.
                let exists = entries[idx]
                    .relations
                    .iter()
                    .any(|r| r.type_ == rel.type_ && r.target == rel.target);
                if !exists {
                    entries[idx].relations.push(rel);
                }
            }
        }
    }

    // 2. Auto-fill symmetric relations.
    // Collect reverse relations to add, then apply them.
    let mut to_add: Vec<(usize, Relation)> = Vec::new();
    for entry in entries.iter() {
        for rel in &entry.relations {
            let type_str = rel.type_.as_deref().unwrap_or("");
            if SYMMETRIC_TYPES.contains(&type_str) {
                // Find target entry index.
                if let Some(&target_idx) = by_headword.get(&rel.target) {
                    let reverse = Relation {
                        type_: rel.type_.clone(),
                        target: entry.headword.clone(),
                    };
                    // Check if reverse already exists.
                    let exists = entries[target_idx]
                        .relations
                        .iter()
                        .any(|r| r.type_ == reverse.type_ && r.target == reverse.target);
                    if !exists {
                        to_add.push((target_idx, reverse));
                    }
                }
            }
        }
    }
    for (idx, rel) in to_add {
        entries[idx].relations.push(rel);
    }

    // 3. Validate all relations.
    for entry in entries.iter() {
        let mut seen: std::collections::HashSet<(Option<&str>, &str)> =
            std::collections::HashSet::new();
        for rel in &entry.relations {
            if rel.target.is_empty() {
                eprintln!(
                    "Error: entry '{}' has a relation with empty target",
                    entry.headword
                );
                std::process::exit(1);
            }
            if rel.target == entry.headword {
                eprintln!(
                    "Error: self-relation on '{}' (target == headword)",
                    entry.headword
                );
                std::process::exit(1);
            }
            if !headword_set.contains(&rel.target) {
                eprintln!(
                    "Error: entry '{}' has relation to non-existent headword '{}'",
                    entry.headword, rel.target
                );
                std::process::exit(1);
            }
            if !seen.insert((rel.type_.as_deref(), rel.target.as_str())) {
                eprintln!(
                    "Error: duplicate relation ({:?}, '{}') on '{}'",
                    rel.type_, rel.target, entry.headword
                );
                std::process::exit(1);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: rdict-compile <input.yaml|input_dir> [-o output.rdict]");
        eprintln!("  If input is a directory, all *.yaml files are merged into one pack.");
        std::process::exit(1);
    }

    let input = PathBuf::from(&args[1]);
    let output = if let Some(idx) = args.iter().position(|a| a == "-o") {
        PathBuf::from(&args[idx + 1])
    } else {
        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
        PathBuf::from(format!("{}.rdict", stem))
    };

    let yaml_files = if input.is_dir() {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&input)
            .unwrap_or_else(|e| {
                eprintln!("Error reading directory {}: {}", input.display(), e);
                std::process::exit(1);
            })
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|ext| ext == "yaml" || ext == "yml")
            })
            .collect();
        files.sort();
        files
    } else {
        vec![input.clone()]
    };

    if yaml_files.is_empty() {
        eprintln!("No .yaml files found");
        std::process::exit(1);
    }

    let mut all_entries: Vec<YamlEntry> = Vec::new();
    let mut phrase_of_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for yaml_path in &yaml_files {
        eprint!("Reading {}... ", yaml_path.display());
        io::stderr().flush().ok();
        let file = File::open(yaml_path).unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            std::process::exit(1);
        });
        let reader = BufReader::new(file);
        let doc: YamlDoc = serde_yaml::from_reader(reader).unwrap_or_else(|e| {
            eprintln!("YAML parse error: {}", e);
            std::process::exit(1);
        });
        eprintln!("{} entries", doc.entries.len());

        // Collect phrase_of declarations before dedup.
        for entry in &doc.entries {
            if !entry.phrase_of.is_empty() {
                phrase_of_map.insert(entry.headword.clone(), entry.phrase_of.clone());
            }
        }

        all_entries.extend(doc.entries);
    }

    // Deduplicate by headword (keep last)
    all_entries.sort_by(|a, b| a.headword.cmp(&b.headword));
    all_entries.dedup_by(|a, b| a.headword == b.headword);

    eprintln!("Total unique entries: {}", all_entries.len());

    // Pack-level language config
    let target_langs: Vec<String> = vec!["zh-Hans".into()];
    let default_lang = if target_langs.len() == 1 {
        Some(target_langs[0].as_str())
    } else {
        None
    };

    // Validate + fill lang
    let mut entries: Vec<Entry> = all_entries
        .into_iter()
        .map(|y| convert_entry(y, default_lang))
        .collect();

    // Validate: if multiple target_langs, all translations must have lang
    if target_langs.len() > 1 {
        for entry in &entries {
            for ety in &entry.etymologies {
                for sense in &ety.senses {
                    for t in &sense.translations {
                        if t.lang.is_none() {
                            eprintln!(
                                "Error: entry '{}' has a translation without 'lang' \
                                 but pack has multiple target_langs",
                                entry.headword
                            );
                            std::process::exit(1);
                        }
                    }
                    for def in &sense.definitions {
                        if let rdict::Def::Definition(d) = def {
                            for ex in &d.examples {
                                for t in &ex.translations {
                                    if t.lang.is_none() {
                                        eprintln!(
                                            "Error: entry '{}' has an example translation \
                                             without 'lang' but pack has multiple target_langs",
                                            entry.headword
                                        );
                                        std::process::exit(1);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Post-process relations: auto-fill symmetric, distribute phrase_of,
    // validate targets.
    process_relations(&mut entries, &phrase_of_map);

    let pack = Pack {
        metadata: PackMetadata {
            name: "English-Chinese (NGSL)".into(),
            source_lang: "en".into(),
            target_langs,
            ..Default::default()
        },
        entries,
        media: Vec::new(),
    };

    eprint!("Writing {}... ", output.display());
    io::stderr().flush().ok();
    let out_file = File::create(&output).unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        std::process::exit(1);
    });
    match RdictWriter::write_pack(out_file, &pack) {
        Ok(()) => {
            let size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
            eprintln!("done ({:.1} KB)", size as f64 / 1024.0);
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}
