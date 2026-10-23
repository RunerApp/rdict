//! Compile the SQLite dictionary database directly into a `.rdict` pack.
//!
//! Bypasses the YAML intermediate format entirely: SELECT → core model →
//! RdictWriter. Reads a live database safely (each entry read in one
//! transaction window; concurrent writers are fine thanks to WAL).
//!
//! Usage:
//!   db_to_rdict <dictionary.db> <output.rdict> [--name NAME] [--level N]
//!               [--max N] [--ipa-only-owned]

use std::collections::HashMap;
use std::io::Write;

use rdict::{
    Def, Definition, Entry, Ety, Example, Pack, PackMetadata, Pron, RdictWriter, Relation, Sense,
    TargetOccurrence, TextSpan, Translation,
};
use rusqlite::{Connection, OpenFlags};

struct RowEntry {
    id: i64,
    headword: String,
    root: Option<String>,
    see: Option<String>,
    morphology: Option<String>,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let db_path = args.next().expect("usage: db_to_rdict <db> <out.rdict>");
    let out_path = args.next().expect("usage: db_to_rdict <db> <out.rdict>");
    let mut name: Option<String> = None;
    let mut level: i32 = 3;
    let mut max: Option<i64> = None;
    let mut lang = "en".to_string();
    let mut targets: Option<Vec<String>> = None;
    let mut cover: Option<String> = None;
    // --defs: `drop` (default) strips source-lang definition text in
    // bilingual packs; `keep` compiles definitions + translations together
    // (the "explanatory bilingual" style).
    let mut keep_defs = false;

    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--lang" => lang = args.next().expect("--lang needs a value"),
            "--cover" => cover = Some(args.next().expect("--cover needs a value")),
            "--defs" => {
                match args
                    .next()
                    .expect("--defs needs a value (keep|drop)")
                    .as_str()
                {
                    "keep" => keep_defs = true,
                    "drop" => keep_defs = false,
                    other => panic!("--defs: unknown value {other} (use keep|drop)"),
                }
            }
            "--targets" => {
                targets = Some(
                    args.next()
                        .expect("--targets needs a value")
                        .split(',')
                        .map(|t| match t.trim() {
                            "zh" => "zh-Hans".to_string(),
                            "es" => "es-ES".to_string(),
                            other => other.to_string(),
                        })
                        .collect(),
                )
            }
            "--name" => name = Some(args.next().expect("--name needs a value")),
            "--level" => {
                level = args
                    .next()
                    .expect("--level needs a value")
                    .parse()
                    .expect("level must be an int")
            }
            "--max" => {
                max = Some(
                    args.next()
                        .expect("--max needs a value")
                        .parse()
                        .expect("max must be an int"),
                )
            }
            other => panic!("unknown flag: {other}"),
        }
    }

    let t0 = std::time::Instant::now();
    let conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open db (read-only)");
    conn.pragma_update(None, "busy_timeout", 60_000).ok();

    // Pack registry: packs.toml (single source of truth, shared with
    // covers/make_covers.py). Located next to the compiler crate root.
    let packs_path = std::env::var("RDICT_PACKS_TOML").unwrap_or_else(|_| {
        // <exe>/../../packs.toml works for target/release layout; fall back to CWD
        let exe = std::env::current_exe().ok();
        let candidates = [
            exe.as_ref()
                .and_then(|p| p.parent().and_then(|p| p.parent().and_then(|p| p.parent())))
                .map(|p| p.join("packs.toml")),
            Some(std::path::PathBuf::from("packs.toml")),
        ];
        candidates
            .iter()
            .flatten()
            .find(|p| p.exists())
            .map(|p| p.display().to_string())
            .expect("packs.toml not found (set RDICT_PACKS_TOML)")
    });
    let registry: PackRegistry = toml::from_str(
        &std::fs::read_to_string(&packs_path).unwrap_or_else(|e| panic!("read {packs_path}: {e}")),
    )
    .unwrap_or_else(|e| panic!("parse {packs_path}: {e}"));

    // Default target langs per source lang (from registry; `none` = monolingual).
    let target_langs: Vec<String> = match targets {
        Some(t) if t.iter().any(|t| t == "none") => Vec::new(),
        Some(t) => t,
        None => registry
            .languages
            .get(&lang)
            .unwrap_or_else(|| panic!("unknown --lang {lang} (not in packs.toml [languages])"))
            .default_targets
            .clone(),
    };
    let target_set: std::collections::HashSet<&str> =
        target_langs.iter().map(|s| s.as_str()).collect();

    // Pack id/name per (source lang, targets) combo.
    let mut sorted_targets = target_langs.clone();
    sorted_targets.sort();
    let combo: String = format!("{}|{}", lang, sorted_targets.join(","));
    let pack_cfg = registry
        .packs
        .iter()
        .find(|p| p.combo == combo)
        .unwrap_or_else(|| {
            panic!(
                "no pack identity for combo {combo:?} — add a [[packs]] entry to packs.toml"
            )
        });
    let (pack_id, default_name, description) =
        (pack_cfg.id.as_str(), pack_cfg.name.as_str(), pack_cfg.description.as_str());
    let name = name.unwrap_or_else(|| default_name.to_string());

    // Runer manifest extension (rdict-official-data-plan §8.1): all
    // Runer-specific fields in one top-level `runer` object.
    let short = short_lang_code;
    let mut sorted_short: Vec<&str> = target_langs.iter().map(|t| short(t)).collect();
    sorted_short.sort();
    let runer_pack_id = if keep_defs {
        // explanatory bilingual: runer.<src>-<src>+<t1>+<t2>
        format!(
            "runer.{}-{}{}",
            lang,
            lang,
            sorted_short
                .iter()
                .map(|t| format!("+{t}"))
                .collect::<String>()
        )
    } else if sorted_short.is_empty() {
        format!("runer.{lang}-{lang}")
    } else {
        format!("runer.{lang}-{}", sorted_short.join("+"))
    };
    let runer_ext = serde_json::json!({
        "schema": 1,
        "official": true,
        "pack_id": runer_pack_id,
        "id_namespace": format!("runer:{lang}"),
        "id_scheme": "pos-seq-v1",
        "data_version": "2026.9.1",
        "update_url": "https://runerapp.com/api/dicts/index",
        "id_redirects": load_id_redirects(&conn),
    });

    // Count first for progress display.
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entries e
             WHERE lang = ?1
               AND (EXISTS (SELECT 1 FROM senses s WHERE s.entry_id = e.id)
                    OR COALESCE(see, '') != '')",
            [&lang],
            |r| r.get(0),
        )
        .unwrap_or(0);
    eprintln!("entries with content (lang={lang}): {total}");

    // Stream base entry rows.
    let limit_clause = match max {
        Some(n) => format!("LIMIT {n}"),
        None => String::new(),
    };
    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, headword, root, see, morphology FROM entries e
             WHERE lang = ?1
               AND (EXISTS (SELECT 1 FROM senses s WHERE s.entry_id = e.id)
                    OR COALESCE(see, '') != '')
             ORDER BY headword {limit_clause}"
        ))
        .expect("prepare entries");

    let mut entries: Vec<Entry> = Vec::new();
    let rows = stmt
        .query_map([&lang], |r| {
            Ok(RowEntry {
                id: r.get(0)?,
                headword: r.get(1)?,
                root: r.get(2)?,
                see: r.get(3)?,
                morphology: r.get(4)?,
            })
        })
        .expect("query entries");

    let mut count = 0i64;
    for row in rows {
        let re = row.expect("entry row");
        let mut entry = build_entry_lang(&conn, &re, &lang, &target_set, keep_defs);
        // Tags (e.g. jlpt:n5..n1, freq:*) — loaded per entry; a separate
        // prepared statement reused across rows.
        let mut tag_stmt = conn
            .prepare("SELECT tag FROM entry_tags WHERE entry_id = ?1 ORDER BY tag")
            .expect("prepare tags");
        if let Ok(tag_rows) = tag_stmt.query_map([re.id], |r| r.get::<_, String>(0)) {
            for t in tag_rows.flatten() {
                entry.tags.push(t);
            }
        }
        entries.push(entry);
        count += 1;
        if count % 5000 == 0 {
            eprintln!("  built {count}/{total} entries ({:?})", t0.elapsed());
        }
    }
    eprintln!("built {count} entries in {:?}", t0.elapsed());

    // Deduplicate headwords defensively (keep first).
    let mut seen = std::collections::HashSet::new();
    entries.retain(|e| seen.insert(e.headword.clone()));
    entries.sort_by(|a, b| a.headword.as_bytes().cmp(b.headword.as_bytes()));

    // Sanitize relations: targets must exist in this pack, no self-relations,
    // no duplicate (type, target) pairs. (With --max the pack is a subset, so
    // relations pointing outside it must be dropped; bad rows in the DB are
    // dropped the same way.)
    let headwords: std::collections::HashSet<String> =
        entries.iter().map(|e| e.headword.clone()).collect();
    let mut dropped = 0usize;
    for e in entries.iter_mut() {
        let mut rel_seen = std::collections::HashSet::new();
        let before = e.relations.len();
        e.relations.retain(|r| {
            r.target != e.headword
                && headwords.contains(r.target.as_str())
                && rel_seen.insert((r.type_.clone(), r.target.clone()))
        });
        dropped += before - e.relations.len();
    }
    if dropped > 0 {
        eprintln!("dropped {dropped} invalid/out-of-pack relations");
    }

    // Cover image: read PNG bytes; metadata.cover = file name.
    let cover_bytes = cover
        .as_ref()
        .map(|path| std::fs::read(path).unwrap_or_else(|e| panic!("read cover {path}: {e}")));
    let cover_name = cover.as_ref().map(|path| {
        std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "cover.png".to_string())
    });

    let pack = Pack {
        metadata: PackMetadata {
            id: pack_id.to_string(),
            name,
            version: "0.1.0".to_string(),
            source_lang: lang.clone(),
            target_langs,
            author: Some("rdict project".to_string()),
            license: Some("GPL-3.0".to_string()),
            created_at: Some(chrono_now()),
            description: Some(description.to_string()),
            cover: cover_name,
            // Consumer-specific extension (plan §8.1): the `runer` object
            // rides along in manifest.json; v0.1 readers ignore it.
            runer: Some(runer_ext),
            block_max_uncompressed: 262_144,
            zstd_level: level,
        },
        entries,
        media: Vec::new(),
        cover: cover_bytes,
    };

    eprintln!("writing pack (zstd level {level})...");
    let file = std::fs::File::create(&out_path).expect("create output");
    let mut buf = std::io::BufWriter::new(file);
    RdictWriter::write_pack(&mut buf, &pack).expect("write pack");
    buf.flush().expect("flush");
    drop(buf);

    let size = std::fs::metadata(&out_path).unwrap().len();
    eprintln!(
        "done: {out_path} ({:.1} MiB) in {:?}",
        size as f64 / 1048576.0,
        t0.elapsed()
    );
}

/// Load id_redirects (plan §8.2) from the DB. Rule 3: a redirect's `from`
/// position must NOT still exist in the current data — if it does, the
/// redirect is wrong, so we panic loudly instead of emitting bad data.
fn load_id_redirects(conn: &Connection) -> serde_json::Value {
    let Ok(mut stmt) = conn.prepare(
        "SELECT from_headword, from_id, to_headword, to_id, reason, since FROM id_redirects",
    ) else {
        return serde_json::json!([]);
    };
    let Ok(rows) = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
        ))
    }) else {
        return serde_json::json!([]);
    };
    let mut out = Vec::new();
    for row in rows.flatten() {
        let (from_hw, from_id, to_hw, to_id, reason, since) = row;
        // rule 3: from must not resolve in the current DB
        let still_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM definitions d
                 JOIN senses s ON d.sense_id = s.id
                 JOIN entries e ON s.entry_id = e.id
                 WHERE e.headword = ?1 AND d.public_id = ?2",
                [&from_hw, &from_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if still_exists > 0 {
            panic!(
                "id_redirects rule 3 violation: from position ({from_hw}, {from_id}) still exists"
            );
        }
        let to = match (to_hw, to_id) {
            (Some(h), Some(i)) => serde_json::json!([h, i]),
            _ => serde_json::Value::Null, // deleted with no successor
        };
        out.push(serde_json::json!({
            "from": [from_hw, from_id],
            "to": to,
            "reason": reason,
            "since": since,
        }));
    }
    serde_json::json!(out)
}

fn short_lang_code(l: &str) -> &str {
    match l {
        "zh-Hans" => "zh",
        "es-ES" => "es",
        other => other,
    }
}

fn chrono_now() -> String {
    // RFC3339-ish without pulling chrono: use the system date command is
    // overkill; just emit a unix timestamp marker.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

fn build_entry_lang(
    conn: &Connection,
    re: &RowEntry,
    lang: &str,
    target_set: &std::collections::HashSet<&str>,
    keep_defs: bool,
) -> Entry {
    let mut entry = Entry::new(re.headword.clone());
    entry.see.clone_from(&re.see);

    // Pronunciations: generic per-entry rows from entry_prons (kind +
    // value), no language-specific logic here. Kinds are spec §6.8
    // notation codes (ipa / hira / roma / pinyin / ...). For English the
    // kinds 'ipa' and 'ipa_uk' are regional variants, so they compile as
    // Pron{lang: en-US / en-GB, kind: ipa} (spec §6.8: Pron.lang is
    // BCP-47 language-region, not just language).
    {
        let mut pron_stmt = conn
            .prepare(
                "SELECT kind, value FROM entry_prons WHERE entry_id = ?1 \
                 ORDER BY sort_order, kind",
            )
            .expect("prepare prons");
        if let Ok(pron_rows) = pron_stmt.query_map([re.id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        }) {
            for (kind, value) in pron_rows.flatten() {
                let (pron_lang, kind) = if lang == "en" {
                    match kind.as_str() {
                        "ipa" => ("en-US".to_string(), kind),
                        "ipa_uk" => ("en-GB".to_string(), "ipa".to_string()),
                        _ => (lang.to_string(), kind),
                    }
                } else {
                    (lang.to_string(), kind)
                };
                entry.pron.push(Pron {
                    lang: Some(pron_lang),
                    accent: None,
                    kind: Some(kind),
                    value,
                    media: Vec::new(),
                });
            }
        }
    }

    if let Some(morph) = re.morphology.as_deref().filter(|s| !s.is_empty()) {
        entry.morphology = parse_morphology(morph);
    }

    entry.relations = load_relations(conn, re.id);

    // Senses → single etymology group. root becomes the ety root.
    let senses = load_senses(conn, re.id, &re.headword, target_set, keep_defs);
    if re.root.as_deref().map_or(false, |r| !r.is_empty()) || !senses.is_empty() {
        entry.etymologies.push(Ety {
            id: None,
            root: re.root.clone().filter(|r| !r.is_empty()),
            senses,
        });
    }

    entry
}

fn parse_morphology(raw: &str) -> Vec<rdict::Morpheme> {
    // Accepts either the canonical JSON [["kind","term"],...] or a plain
    // description string (kept as a single root morpheme).
    if let Ok(v) = serde_json::from_str::<Vec<Vec<String>>>(raw) {
        return v
            .into_iter()
            .filter(|p| p.len() == 2)
            .map(|p| rdict::Morpheme {
                kind: Some(p[0].clone()),
                term: p[1].clone(),
            })
            .collect();
    }
    if let Ok(v) = serde_json::from_str::<Vec<serde_json::Value>>(raw) {
        // [{"kind":..,"term":..}] shape
        return v
            .into_iter()
            .filter_map(|m| {
                let kind = m.get("kind").and_then(|x| x.as_str()).map(String::from);
                let term = m.get("term").and_then(|x| x.as_str())?;
                Some(rdict::Morpheme {
                    kind,
                    term: term.to_string(),
                })
            })
            .collect();
    }
    vec![rdict::Morpheme {
        kind: Some("root".into()),
        term: raw.to_string(),
    }]
}

fn load_relations(conn: &Connection, entry_id: i64) -> Vec<Relation> {
    let Ok(mut stmt) = conn.prepare("SELECT type, target FROM entry_relations WHERE entry_id = ?")
    else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map([entry_id], |r| {
        Ok(Relation {
            type_: Some(r.get::<_, String>(0)?),
            target: r.get(1)?,
        })
    }) else {
        return Vec::new();
    };
    rows.filter_map(|r| r.ok()).collect()
}

fn load_senses(
    conn: &Connection,
    entry_id: i64,
    headword: &str,
    target_set: &std::collections::HashSet<&str>,
    keep_defs: bool,
) -> Vec<Sense> {
    let mut senses = Vec::new();
    let Ok(mut stmt) =
        conn.prepare("SELECT id, pos, lemma FROM senses WHERE entry_id = ? ORDER BY sort_order")
    else {
        return senses;
    };
    let Ok(rows) = stmt.query_map([entry_id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    }) else {
        return senses;
    };

    let sense_ids: Vec<(i64, Option<String>, Option<String>)> =
        rows.filter_map(|r| r.ok()).collect();
    for (sid, pos, lemma) in sense_ids {
        let mut sense = Sense {
            pos: pos.filter(|p| !p.is_empty()),
            lemma,
            ..Default::default()
        };
        sense.translations = load_sense_translations(conn, sid, target_set);
        sense.definitions = load_definitions(conn, sid, headword, target_set, keep_defs);
        senses.push(sense);
    }
    senses
}

fn load_sense_translations(
    conn: &Connection,
    sense_id: i64,
    target_set: &std::collections::HashSet<&str>,
) -> Vec<Translation> {
    let Ok(mut stmt) =
        conn.prepare("SELECT lang, value FROM sense_translations WHERE sense_id = ?")
    else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map([sense_id], |r| {
        Ok(Translation {
            lang: Some(r.get(0)?),
            value: r.get(1)?,
            pron: Vec::new(),
        })
    }) else {
        return Vec::new();
    };
    rows.filter_map(|r| r.ok())
        .filter(|t| t.lang.as_deref().map_or(false, |l| target_set.contains(l)))
        .collect()
}

fn load_definitions(
    conn: &Connection,
    sense_id: i64,
    headword: &str,
    target_set: &std::collections::HashSet<&str>,
    keep_defs: bool,
) -> Vec<Def> {
    let mut defs = Vec::new();
    // public_id: stable definition ids (rdict-official-data-plan §2).
    // Read from the DB (never recomputed — the column is the source of
    // truth once backfilled by headwords/db_public_id.py).
    let Ok(mut stmt) = conn.prepare(
        "SELECT id, value, public_id FROM definitions WHERE sense_id = ? ORDER BY sort_order",
    ) else {
        return defs;
    };
    let Ok(rows) = stmt.query_map([sense_id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    }) else {
        return defs;
    };
    let def_rows: Vec<(i64, String, Option<String>)> = rows.filter_map(|r| r.ok()).collect();
    for (did, value, public_id) in def_rows {
        defs.push(Def::Definition(Definition {
            id: public_id,
            // Bilingual packs (has targets) default to dropping the
            // source-language definition TEXT — the translation is the
            // content. `--defs keep` compiles definition + translation
            // together (explanatory-bilingual style). The def structure
            // (examples) is always kept intact.
            value: if target_set.is_empty() || keep_defs {
                value
            } else {
                String::new()
            },
            examples: load_examples(conn, did, headword, target_set),
            notes: Vec::new(),
            media: Vec::new(),
        }));
    }
    defs
}

fn load_examples(
    conn: &Connection,
    def_id: i64,
    headword: &str,
    target_set: &std::collections::HashSet<&str>,
) -> Vec<Example> {
    let mut examples = Vec::new();
    let Ok(mut stmt) =
        conn.prepare("SELECT id, value FROM examples WHERE definition_id = ? ORDER BY sort_order")
    else {
        return examples;
    };
    let Ok(rows) = stmt.query_map([def_id], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    }) else {
        return examples;
    };
    let ex_rows: Vec<(i64, String)> = rows.filter_map(|r| r.ok()).collect();
    for (exid, value) in ex_rows {
        let mut ex = Example {
            value,
            ..Default::default()
        };
        if let Ok(mut stmt) =
            conn.prepare("SELECT lang, value FROM example_translations WHERE example_id = ?")
        {
            if let Ok(rows) = stmt.query_map([exid], |r| {
                Ok(Translation {
                    lang: Some(r.get(0)?),
                    value: r.get(1)?,
                    pron: Vec::new(),
                })
            }) {
                ex.translations = rows
                    .filter_map(|r| r.ok())
                    .filter(|t| t.lang.as_deref().map_or(false, |l| target_set.contains(l)))
                    .collect();
            }
        }
        // target-word span: highlight the headword (incl. inflected forms)
        // inside the example. Spans are UTF-8 byte offsets. Plan §4.4:
        // spans backfilled into example_targets by
        // headwords/db_backfill_targets.py (rule-based inflection/conjugation
        // per language, incl. ja readings from entry.ipa) take precedence;
        // the inline heuristic below is the fallback.
        if let Some(occ) = load_example_targets(conn, exid) {
            ex.targets = occ;
        } else {
            for form in inflected_forms(headword) {
                if let Some(spans) = find_word_spans(&ex.value, &form) {
                    ex.targets.push(TargetOccurrence { spans });
                    break;
                }
            }
        }
        examples.push(ex);
    }
    examples
}

/// Pre-computed target spans from the example_targets table (plan §4.4).
/// One occurrence per matching form, spans grouped per occurrence ordinal.
fn load_example_targets(conn: &Connection, ex_id: i64) -> Option<Vec<TargetOccurrence>> {
    let mut stmt = conn
        .prepare(
            "SELECT occurrence, offset, length FROM example_targets
             WHERE example_id = ?1 ORDER BY occurrence, span_ordinal",
        )
        .ok()?;
    let rows = stmt
        .query_map([ex_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, u64>(1)?,
                r.get::<_, u64>(2)?,
            ))
        })
        .ok()?;
    let mut occs: Vec<TargetOccurrence> = Vec::new();
    for row in rows.flatten() {
        let (occ, offset, length) = row;
        if occ < 0 {
            continue;
        }
        while occs.len() <= occ as usize {
            occs.push(TargetOccurrence { spans: Vec::new() });
        }
        occs[occ as usize].spans.push(TextSpan { offset, length });
    }
    if occs.is_empty() {
        None
    } else {
        Some(occs)
    }
}

/// Find whole-word (ASCII case-insensitive) occurrences of `needle` in
/// `hay`, returning UTF-8 byte spans.
fn find_word_spans(hay: &str, needle: &str) -> Option<Vec<TextSpan>> {
    if needle.is_empty() {
        return None;
    }
    let hay_l = hay.to_lowercase();
    let needle_l = needle.to_lowercase();
    let mut spans = Vec::new();
    let mut start = 0usize;
    while let Some(pos) = hay_l[start..].find(&needle_l) {
        let b = start + pos;
        let e = b + needle_l.len();
        let before_ok = b == 0 || !hay_l.as_bytes()[b - 1].is_ascii_alphanumeric();
        let after_ok = e >= hay_l.len() || !hay_l.as_bytes()[e].is_ascii_alphanumeric();
        if before_ok && after_ok {
            // byte offsets in the lowercased string equal those in the
            // original (to_lowercase preserves ASCII length; non-ASCII
            // chars keep their byte lengths in both)
            spans.push(TextSpan {
                offset: b as u64,
                length: needle_l.len() as u64,
            });
        }
        start = e;
        if spans.len() >= 8 {
            break;
        }
    }
    if spans.is_empty() {
        None
    } else {
        Some(spans)
    }
}

/// Generate the headword plus common inflected forms for target matching.
fn inflected_forms(headword: &str) -> Vec<String> {
    let mut forms = vec![headword.to_string()];
    // Only inflect simple ASCII letter words.
    if !headword.is_ascii() || headword.is_empty() {
        return forms;
    }
    let lower = headword.to_lowercase();
    let mut push = |f: String| {
        if !forms.contains(&f) {
            forms.push(f);
        }
    };
    push(format!("{lower}s"));
    push(format!("{lower}es"));
    push(format!("{lower}ed"));
    push(format!("{lower}d"));
    push(format!("{lower}ing"));
    push(format!("{lower}ly"));
    if let Some(stem) = lower.strip_suffix('e') {
        push(format!("{stem}ing"));
        push(format!("{stem}ed"));
    }
    if lower.ends_with('y') && lower.len() > 1 {
        let stem = &lower[..lower.len() - 1];
        push(format!("{stem}ies"));
        push(format!("{stem}ied"));
    }
    // Prefer longest match first: caller tries forms in order and takes
    // the first that hits, so order base word first is fine.
    forms
}

#[allow(dead_code)]
fn unused(_: HashMap<String, String>) {}

#[derive(serde::Deserialize)]
struct PackRegistry {
    languages: std::collections::HashMap<String, PackLanguage>,
    packs: Vec<PackConfig>,
}

#[derive(serde::Deserialize)]
struct PackLanguage {
    #[serde(default)]
    #[allow(dead_code)]
    name: String,
    default_targets: Vec<String>,
}

#[derive(serde::Deserialize)]
struct PackConfig {
    combo: String,
    id: String,
    name: String,
    #[serde(default)]
    #[allow(dead_code)]
    description: String,
}
