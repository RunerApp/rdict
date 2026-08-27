# Rdict Compiler

YAML → `.rdict` compiler. Compiles human-readable YAML dictionary source files into compact binary `.rdict` format.

## Usage

```bash
# Compile a single file
cargo run -- input.yaml -o output.rdict

# Compile a directory (merges all .yaml files)
cargo run -- entries/ -o combined.rdict
```

## YAML Format

```yaml
pack:
  name: English-Chinese (NGSL)
  version: 0.1.0
  source_lang: en
  target_langs:
    - zh-Hans
  zstd_level: 19

entries:
  - headword: run
    pron:
      - lang: en
        kind: ipa
        value: /rʌn/
    etymologies:
      - root: rinnan
        senses:
          - pos: VERB
            definitions:
              - value: To move swiftly on foot.
                examples:
                  - value: He runs every morning.
                    translations:
                      - value: 他每天早上跑步。
            translations:
              - value: 跑；运行

  - headword: unhappiness
    morphology:
      - kind: prefix
        term: un-
      - kind: root
        term: happy
      - kind: suffix
        term: -ness
    etymologies:
      - root: happy
        senses:
          - pos: NOUN
            definitions:
              - value: The state of being sad.
            translations:
              - value: 不幸；不快乐
```

## Features

- **Auto lang fill** — When `target_langs` has only one language, `translation.lang` and `example.translations[].lang` can be omitted; the compiler fills them automatically
- **Multi-language validation** — When multiple target languages exist, `lang` must be specified explicitly
- **Directory merge** — When input is a directory, all `.yaml` files are merged into one pack with headword deduplication
- **Morphology support** — Affix decomposition field
- **Ety.root support** — Etymological root field

## Build

```bash
cargo build --release
```

## Requirements

- Rust ≥ 1.85 (edition 2024)

## Dependencies

- `rdict` — Core library
- `serde_yaml` — YAML parsing
