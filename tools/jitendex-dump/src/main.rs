// Dump Jitendex MDX structure:
//   entries.tsv: internal_id, priority(0/1), headword_kanji, headword_kana
//   forms.tsv:   internal_id, kanji_variant, reading, form_class (pri/valid/rare)
//   links.tsv:   user_key -> internal_id
// Usage: jitendex-dump dumpall <mdx> <outdir>
use std::io::Write;

fn strip_ruby(s: &str) -> (String, String) {
    // parse "<ruby>寿<rt>す</rt></ruby><ruby>司<rt>し</rt></ruby>" or plain text.
    // Text outside <ruby> is shared (appears in both kanji and kana);
    // ruby base -> kanji, <rt> -> kana.
    let mut kanji = String::new();
    let mut kana = String::new();
    let mut rest = s;
    while let Some(p) = rest.find("<ruby>") {
        // shared plain segment
        let plain = &rest[..p];
        let plain = plain.replace("<span>", "").replace("</span>", "");
        kanji.push_str(&plain);
        kana.push_str(&plain);
        rest = &rest[p + 6..];
        if let Some(rt) = rest.find("<rt>") {
            kanji.push_str(&rest[..rt]);
            let after = &rest[rt + 4..];
            if let Some(end) = after.find("</rt>") {
                kana.push_str(&after[..end]);
                rest = &after[end + 5..];
                if let Some(re) = rest.find("</ruby>") {
                    rest = &rest[re + 7..];
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }
    let tail = rest.replace("<span>", "").replace("</span>", "");
    if !tail.is_empty() {
        kanji.push_str(&tail);
        kana.push_str(&tail);
    }
    (kanji, kana)
}

fn text_no_tags(s: &str) -> String {
    // strip tags, unescape basic entities
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
        .replace("&quot;", "\"").replace("&#39;", "'")
}

fn parse_entry(html: &str) -> (usize, u8, String, String) {
    let id = 0; // caller fills from key
    let priority = if html.contains("class=\"headline priority\"") { 1 } else { 0 };
    // headline headword span
    let hw = if let Some(p) = html.find("<span class=\"headword\"") {
        let seg = &html[p..];
        let inner_start = seg.find('>').map(|i| i + 1).unwrap_or(0);
        let seg = &seg[inner_start..];
        // end at priority-symbol span, or at the headline close
        let end = seg.find("<span class=\"priority-symbol\"")
            .or_else(|| seg.find("</span></div>"))
            .unwrap_or(seg.len());
        &seg[..end]
    } else { "" };
    let (kanji, kana) = strip_ruby(hw);
    (id, priority, kanji, kana)
}

fn parse_forms(html: &str, id: &str, forms_out: &mut String) {
    // forms table: <li class="forms">...<table><tr class="forms-header-row">...</tr><tr class="forms-body-row">...</tr>...</table>
    let Some(fp) = html.find("<li class=\"forms\">") else { return };
    let table = &html[fp..];
    let Some(tp) = table.find("<table>") else { return };
    let table = &table[tp..];
    let Some(te) = table.find("</table>") else { return };
    let table = &table[7..te];
    // header row: <th/> or <th></th> then variant ths
    let mut rows = table.split("<tr").skip(1);
    let mut variants: Vec<String> = Vec::new();
    if let Some(header) = rows.next() {
        if header.contains("forms-header-row") {
            for cell in header.split("<th") {
                if cell.is_empty() || !cell.contains('>') { continue; }
                let body = &cell[cell.find('>').unwrap() + 1..];
                let body = body.strip_suffix("</th>").unwrap_or(body);
                let t = text_no_tags(body).trim().to_string();
                if !t.is_empty() && !t.starts_with("/>") {
                    variants.push(t);
                }
            }
        }
    }
    if variants.is_empty() { return; }
    for row in rows {
        if !row.contains("forms-body-row") { continue; }
        // first th = reading, then tds (one per variant)
        let mut cells: Vec<(char, String)> = Vec::new();
        for cell in row.split("<t") {
            if !cell.contains('>') { continue; }
            let (kind, body) = if cell.starts_with("h") {
                let body = &cell[cell.find('>').unwrap() + 1..];
                ('h', body.strip_suffix("</th>").unwrap_or(body))
            } else if cell.starts_with("d") {
                let body = &cell[cell.find('>').unwrap() + 1..];
                ('d', body.strip_suffix("</td>").unwrap_or(body))
            } else { continue };
            let class = if kind == 'd' {
                if cell.contains("form-pri") { 'p' }
                else if cell.contains("form-rare") { 'r' }
                else if cell.contains("form-valid") { 'v' }
                else if cell.contains("form-ik") { 'i' } // irregular kana
                else if cell.contains("form-ok") { 'o' } // old kana
                else if cell.contains("form-uk") { 'u' } // usually kana
                else { '-' }
            } else { '-' };
            let t = text_no_tags(body).trim().to_string();
            if !t.is_empty() {
                cells.push((if kind == 'h' { 'h' } else { class }, t));
            }
        }
        // first cell is reading th
        let Some((_, reading)) = cells.first().cloned() else { continue };
        let tds: Vec<(char, String)> = cells[1..].to_vec();
        for (i, (cls, _)) in tds.iter().enumerate() {
            if *cls == '-' { continue; }
            let Some(variant) = variants.get(i) else { continue };
            forms_out.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                id,
                variant,
                reading,
                match cls {
                    'p' => "pri",
                    'r' => "rare",
                    'v' => "valid",
                    'i' => "ik",
                    'o' => "ok",
                    'u' => "uk",
                    _ => "-",
                }
            ));
        }
    }
}

fn parse_glosses(html: &str, out: &mut String) {
    // first 2 glosses: <li class="gloss">text</li>, unit-separated
    let mut n = 0;
    let mut rest = html;
    while let Some(p) = rest.find("<li class=\"gloss\">") {
        let seg = &rest[p + 18..];
        let Some(end) = seg.find("</li>") else { break };
        let g = text_no_tags(&seg[..end]);
        let g = g.trim();
        if !g.is_empty() {
            if n > 0 { out.push('\u{1f}'); }
            out.push_str(&g);
            n += 1;
            if n >= 2 { break; }
        }
        rest = seg;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args[0].as_str();
    let path = &args[1];
    if mode == "fetch" {
        let opts = mdict::MdictOptions::default_mdx();
        let dict = mdict::Mdx::open_with(path, opts).expect("open mdx");
        for word in &args[2..] {
            let r = dict.lookup(word);
            match r.definition {
                Some(html) => println!("=== {word} ===\n{}\n", html),
                None => println!("=== {word} === NOT FOUND"),
            }
        }
        return;
    }
    if mode == "dumpall" {
        let outdir = &args[2];
        let opts = mdict::MdictOptions::default_mdx();
        let dict = mdict::Mdx::open_with(path, opts).expect("open mdx");
        let mut ef = std::io::BufWriter::new(
            std::fs::File::create(format!("{outdir}/entries.tsv")).unwrap(),
        );
        let mut ff = std::io::BufWriter::new(
            std::fs::File::create(format!("{outdir}/forms.tsv")).unwrap(),
        );
        let mut lf = std::io::BufWriter::new(
            std::fs::File::create(format!("{outdir}/links.tsv")).unwrap(),
        );
        let mut gf = std::io::BufWriter::new(
            std::fs::File::create(format!("{outdir}/glosses.tsv")).unwrap(),
        );
        let keys = dict.list_headwords();
        eprintln!("total keys: {}", keys.len());
        let mut n_entry = 0usize;
        let mut n_link = 0usize;
        for k in keys {
            let key = &k.key_text;
            let r = dict.fetch(k);
            let Some(html) = r.definition else { continue };
            if key.starts_with("@jitendex-") {
                let (id, pri, kanji, kana) = parse_entry(&html);
                let id_str = key.trim_start_matches("@jitendex-").to_string();
                writeln!(ef, "{id_str}\t{pri}\t{kanji}\t{kana}").unwrap();
                let mut forms = String::new();
                parse_forms(&html, &id_str, &mut forms);
                write!(ff, "{forms}").unwrap();
                let mut glosses = String::new();
                parse_glosses(&html, &mut glosses);
                writeln!(gf, "{id_str}\t{kanji}\t{glosses}").unwrap();
                n_entry += 1;
            } else if let Some(target) = html.strip_prefix("@@@LINK=") {
                let target = target.trim();
                writeln!(lf, "{key}\t{target}").unwrap();
                n_link += 1;
            }
            if n_entry % 20000 == 0 && n_entry > 0 {
                eprintln!("  {n_entry} entries...");
            }
        }
        eprintln!("done: {n_entry} entries, {n_link} links");
        return;
    }
    panic!("unknown mode {mode} (dumpall|fetch)");
}
