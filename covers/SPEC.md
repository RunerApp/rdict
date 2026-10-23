# rdict Cover Specification (v2)

SVG is the **master format** for covers. A build step renders each SVG to
PNG (`rsvg-convert`, macOS system fonts) and the compiler embeds it into
the `.rdict` pack via `Pack.cover` (PNG/JPEG bytes) + `metadata.cover`
(file name, written to the ZIP root).

## 1. Canvas

Portrait book cover, **1200 × 1600**, viewBox `0 0 1200 1600`.
Two stacked zones:

```
┌───────────────┐
│               │
│  SRC panel    │  y 0..745     language palette color, big white code
│               │
├───────────────┤
│               │
│  TGT panel    │  y 745..1490  second palette color, big white code
│               │
├───────────────┤
│  metadata bar │  y 1490..1600 graphite, one line, same for all packs
└───────────────┘
```

- **Bilingual packs**: two equal panels, no gap, no arrow — the color
  split itself reads as "from → to". Source lang on top, target below.
- **Monolingual packs**: one full panel (0..1490).
- **Metadata bar** (`meta_bar`): 110 px graphite strip, identical across
  ALL packs — the series identity. Single line, auto-shrunk to fit:
  `"{entries:,} ENTRIES · {native pair name} · rdict {YYYY-MM-DD}"`.

## 2. rdict language palette

Colors are drawn from each language's **writing/publishing tradition**,
deliberately NOT from any national flag — most of these languages are
spoken in many countries, and flag colors would be wrong for most users
(and politically loaded for some).

When adding a language, pick a color that:
1. evokes the language's own literary tradition,
2. contrasts clearly with every existing palette entry,
3. stays legible with near-white ink (#fdfbf7) on top.

| key | color | tradition | ghost glyph |
|---|---|---|---|
| en | `#14213d` Oxford navy | academic publishing | Aa |
| zh | `#8f1d21` cinnabar red | seal ink / calligraphy | 汉 |
| es | `#6b4226` ochre brown | Cervantes-era bindings | Ññ |

Metadata bar: `#2b2f33` graphite (constant). Background (unused in v2
except as SVG fallback): `#101418`.

## 3. Typography

- Language codes: **Menlo bold** (monospaced — proportional fonts
  render e.g. ZH visibly larger than ES at the same font-size due to
  per-glyph cap-height differences + font fallback). White ink
  `#fdfbf7`, ≤400 px bilingual / ≤620 px monolingual, vertically
  centered per panel (cap-height centered).
- Ghost glyphs (background motif, per panel): Georgia italic bold,
  440 px, 5 % ink opacity, one signature glyph from the palette table.
- Metadata line: Helvetica Neue / PingFang SC, ~30 px, auto-shrink.

## 4. Per-pack configuration

| Pack | Top panel | Bottom panel |
|---|---|---|
| dictionary_en_en | EN (full) | — |
| dictionary_en_zh | EN navy | ZH cinnabar |
| dictionary_en_es | EN navy | ES ochre |
| dictionary_es_es | ES (full) | — |
| dictionary_es_zh | ES ochre | ZH cinnabar |
| dictionary_es_en | ES ochre | EN navy |

Native pair names (metadata bar): en→zh 英汉词典, en→es Diccionario
inglés–español, es→zh 西汉词典, es→en Diccionario español–inglés,
monolingual packs omit the name. Entry counts are queried live from
`dictionary.db` at generation time.

## 5. Build pipeline

```
covers/make_covers.py                     → covers/svg/*.svg + covers/png/*.png
db_to_rdict ... --cover covers/png/X.png  → pack.cover embedded
```

```
cd covers && python3 make_covers.py
cd compiler && ./target/release/db_to_rdict ../dictionary.db \
  ../dictionary_en_zh.rdict --lang en --targets zh \
  --cover ../covers/png/dictionary_en_zh.png
```

SVG files are hand-editable and diff-friendly; PNGs are build artifacts
and may be regenerated at any time.

## History

- v2 (2026-09-16): split-panel palette design. Replaced v1's
  gradient/title/pull-quote layout — too much small text, illegible at
  thumbnail size; per-glyph size mismatch in proportional fonts made
  EN/ZH/ES codes render unequal. v2 guarantees equal panels (745 px
  each), equal code sizes (monospace), and one line of metadata.
