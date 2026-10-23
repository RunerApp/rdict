#!/usr/bin/env python3
"""Generate SVG covers for all rdict packs per covers/SPEC.md.

Design v2: split-panel language blocks. The cover is divided into two
stacked color panels (source lang on top, target lang below, arrow in the
gap; monolingual = one full panel). Each panel carries its language's
palette color with the big language code set flush in white.

Masters: covers/svg/<pack>.svg   Rendered: covers/png/<pack>.png
Stats are pulled live from dictionary.db.

Pack list comes from ../compiler/packs.toml (single source of truth,
shared with db_to_rdict): every [[packs]] combo maps to a cover for the
pack file ../dictionary_<combo-with-dashes>.rdict.
"""

import html
import os
import sqlite3
import subprocess
import time
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
DB = os.path.join(HERE, "..", "dictionary.db")
PACKS_TOML = os.path.join(HERE, "..", "compiler", "packs.toml")
SVG_DIR = os.path.join(HERE, "svg")
PNG_DIR = os.path.join(HERE, "png")

W, H = 1200, 1600
INK = "#fdfbf7"  # near-white ink for codes
META = "#2b2f33"  # graphite metadata bar — same across ALL packs

SANS = "'Helvetica Neue', 'PingFang SC', sans-serif"
# monospaced bold: identical glyph advance for every letter
MONO = "'Menlo', 'Courier New', monospace"

# ── rdict language palette ─────────────────────────────────────────────
# Colors are drawn from each language's WRITING/PUBLISHING tradition,
# deliberately NOT from any national flag (most of these languages are
# spoken in many countries; flag colors would be wrong for most users).
# When adding a language, pick a color that (a) evokes the language's
# own literary tradition, (b) contrasts with all existing palette
# entries, (c) stays legible with white ink.
LANG = {
    # code   color       name                     ghost
    "en": dict(color="#14213d", ghost="Aa"),  # Oxford navy — academic publishing
    "zh": dict(color="#8f1d21", ghost="汉"),  # cinnabar red — seal ink
    "es": dict(color="#6b4226", ghost="Ññ"),  # ochre brown — Cervantes-era bindings
    # indigo (藍): Edo-period aizuri-e woodblock prints and ai-dyed
    # textile; deep blue-violet, strong contrast with navy/ochre/cinnabar
    "ja": dict(color="#2d4a6b", ghost="あ"),  # 藍 indigo — Edo woodblock prints
    # bordeaux (酒红): French fine-binding tradition and wine country;
    # deep purple-red, distinct from the brighter cinnabar #8f1d21
    "fr": dict(color="#4a1d40", ghost="Éé"),  # bordeaux — French fine bindings
    # 오방색 (五方色) traditional Korean palette; deep indigo-blue (창청)
    # echoes dancheong 藍彩 but is bluer than ja's #2d4a6b... keep distinct:
    # deep pine-green (석간주/칫돌 oxhair jade) — Korean celadon tradition
    "ko": dict(color="#1d5c4a", ghost="한"),  # deep celadon green — 고려청자
    # de: Bremer blockdruck blue — German incunabula/Impressio paper
    # wrappers; historic printing blue, distinct from ja indigo (bluer,
    # less violet) and en navy (lighter, less saturated)
    "de": dict(color="#1e3a5f", ghost="ß"),  # Bremer blue — Fraktur tradition
    # pt: azulejo cobalt — Portuguese tilework; brighter cobalt than en
    # navy, distinct from all blues above
    "pt": dict(color="#0f4c81", ghost="Ãã"),  # azulejo cobalt — azulejo tiles
    # it: terracotta — Etruscan pottery / Tuscan cotto, warm earth red
    # distinct from cinnabar #8f1d21 (brighter, yellower) and fr bordeaux
    "it": dict(color="#8c4a2f", ghost="Èè"),  # terracotta — cotto toscano
}
SRC_CODE = {
    "en": "EN",
    "es": "ES",
    "ja": "JA",
    "fr": "FR",
    "ko": "KO",
    "de": "DE",
    "pt": "PT",
    "it": "IT",
}
TGT_CODE = {None: None, "zh-Hans": "ZH", "es-ES": "ES", "en": "EN"}
TGT_NAME = {
    None: None,
    "zh-Hans": "英汉词典",
    "es-ES": "Diccionario inglés–español",
    "en": "Diccionario español–inglés",
}


def load_packs():
    """[[packs]] entries from packs.toml -> cover configs.

    File-name targets use the conventional build order (zh-Hans first,
    then en), regardless of the alphabetical combo sort in packs.toml.
    """
    reg = tomllib.load(open(PACKS_TOML, "rb"))
    order = {"zh-Hans": 0, "en": 1, "es-ES": 2}
    packs = []
    for p in reg["packs"]:
        src_lang, _, tgt_str = p["combo"].partition("|")
        tgts = sorted(
            (t for t in tgt_str.split(",") if t), key=lambda t: order.get(t, 9)
        )
        cfg = dict(
            pack="dictionary_"
            + (src_lang if not tgts else f"{src_lang}_" + "+".join(tgts)),
            src=src_lang,
            tgt=tgts[0] if tgts else None,
            tgt2=tgts[1] if len(tgts) > 1 else None,
            native=p["name"],
        )
        packs.append(cfg)
    return packs


def esc(s):
    return html.escape(s, quote=True)


def lang_key(tgt, src):
    """DB lang tag (en / es / zh-Hans / es-ES) → palette key."""
    if tgt is None:
        return src
    return tgt.split("-")[0]


def stats(db, lang):
    entries = db.execute(
        "SELECT COUNT(*) FROM entries e WHERE e.lang=? AND EXISTS"
        " (SELECT 1 FROM senses s WHERE s.entry_id = e.id)",
        (lang,),
    ).fetchone()[0]
    return entries


def code_panel(x, y, w, h, color, code, code_size):
    """One colored panel with the language code centered, flush to edges."""
    parts = []
    parts.append(
        f'<rect x="{x}" y="{y:.0f}" width="{w}" height="{h:.0f}" fill="{color}"/>'
    )
    if code:
        # vertically center the cap-height in the panel
        cap_h = code_size * 0.72
        base_y = y + h / 2 + cap_h / 2
        parts.append(
            f'<text x="{x + w / 2:.0f}" y="{base_y:.0f}" text-anchor="middle"'
            f' font-family="{MONO}" font-weight="bold" font-size="{code_size:.0f}"'
            f' fill="{INK}">{esc(code)}</text>'
        )
    return "".join(parts)


def meta_bar(entries, native):
    """Bottom metadata bar: one short graphite strip, single line,
    identical across ALL packs (series identity)."""
    bar_h = 110
    bar_y = H - bar_h
    text = f"{entries:,} ENTRIES"
    if native:
        text += f" · {native}"
    text += f" · rdict {time.strftime('%Y-%m-%d')}"
    size = 30
    while size > 16 and len(text) * size * 0.58 > W - 80:
        size -= 1
    parts = []
    parts.append(
        f'<rect x="0" y="{bar_y}" width="{W}" height="{bar_h}" fill="{META}"/>'
    )
    parts.append(
        f'<text x="{W / 2}" y="{bar_y + bar_h / 2 + size * 0.36:.0f}"'
        f' text-anchor="middle" font-family="{SANS}" font-size="{size}"'
        f' letter-spacing="2" fill="{INK}">{esc(text)}</text>'
    )
    return "".join(parts), bar_h


def build_svg(db, cfg, bg="#101418"):
    entries = stats(db, cfg["src"])
    src_key = cfg["src"]
    tgt_key = lang_key(cfg["tgt"], cfg["src"])
    tgt2_key = lang_key(cfg.get("tgt2"), cfg["src"]) if cfg.get("tgt2") else None
    src_c = LANG[src_key]["color"]
    tgt_c = LANG[tgt_key]["color"]
    tgt2_c = LANG[tgt2_key]["color"] if tgt2_key else None

    src_code = SRC_CODE[cfg["src"]]
    tgt_code = TGT_CODE[cfg["tgt"]]
    tgt2_code = TGT_CODE[cfg["tgt2"]] if tgt2_key else None

    p = []
    p.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}"'
        f' viewBox="0 0 {W} {H}">'
    )
    p.append(f'<rect width="{W}" height="{H}" fill="{bg}"/>')

    # metadata bar reserves the bottom strip; language panels fill the rest
    bar, bar_h = meta_bar(entries, cfg.get("native") or TGT_NAME.get(cfg["tgt"]))
    body_h = H - bar_h

    if tgt2_code:
        # three stacked panels for dual-target packs
        top_h = body_h / 3
        code_size = min(top_h * 0.62, 300)
        panels = [
            (src_key, src_c, src_code, 0),
            (tgt_key, tgt_c, tgt_code, top_h),
            (tgt2_key, tgt2_c, tgt2_code, top_h * 2),
        ]
        for key, color, code, y0 in panels:
            p.append(code_panel(0, y0, W, top_h, color, code, code_size))
            g = LANG[key]["ghost"]
            p.append(
                f'<text x="{W / 2}" y="{y0 + top_h / 2 + 130:.0f}" text-anchor="middle"'
                f' font-family="Georgia, serif" font-style="italic"'
                f' font-weight="bold" font-size="330" fill="{INK}"'
                f' opacity="0.05">{esc(g)}</text>'
            )
    elif tgt_code:
        # two stacked panels, no gap — the color split itself reads
        # as "from → to"
        top_h = body_h / 2
        code_size = min(top_h * 0.62, 400)
        p.append(code_panel(0, 0, W, top_h, src_c, src_code, code_size))
        p.append(code_panel(0, top_h, W, top_h, tgt_c, tgt_code, code_size))
        # ghost letters, faint, one per panel
        for key, y0, hh in ((src_key, 0, top_h), (tgt_key, top_h, top_h)):
            g = LANG[key]["ghost"]
            p.append(
                f'<text x="{W / 2}" y="{y0 + hh / 2 + 170:.0f}" text-anchor="middle"'
                f' font-family="Georgia, serif" font-style="italic"'
                f' font-weight="bold" font-size="440" fill="{INK}"'
                f' opacity="0.05">{esc(g)}</text>'
            )
    else:
        # monolingual: one full panel above the metadata bar
        code_size = min(body_h * 0.5, 620)
        p.append(code_panel(0, 0, W, body_h, src_c, src_code, code_size))
        g = LANG[src_key]["ghost"]
        p.append(
            f'<text x="{W / 2}" y="{body_h / 2 + 320:.0f}" text-anchor="middle"'
            f' font-family="Georgia, serif" font-style="italic"'
            f' font-weight="bold" font-size="440" fill="{INK}"'
            f' opacity="0.05">{esc(g)}</text>'
        )

    p.append(bar)
    p.append("</svg>")
    return "\n".join(p)


def main():
    db = sqlite3.connect(f"file:{DB}?mode=ro", uri=True)
    os.makedirs(SVG_DIR, exist_ok=True)
    os.makedirs(PNG_DIR, exist_ok=True)
    for cfg in load_packs():
        svg = build_svg(db, cfg)
        svg_path = os.path.join(SVG_DIR, cfg["pack"] + ".svg")
        with open(svg_path, "w", encoding="utf-8") as f:
            f.write(svg)
        png_path = os.path.join(PNG_DIR, cfg["pack"] + ".png")
        subprocess.run(
            ["rsvg-convert", "-w", str(W), "-h", str(H), "-o", png_path, svg_path],
            check=True,
        )
        print(f"  {cfg['pack']}: svg + png written")
    print("all covers generated")


if __name__ == "__main__":
    main()
