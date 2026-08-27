#!/usr/bin/env node
// Quick try: node try.js [path-to-.rdict] [word]

const { Dictionary } = require('./index.js');

const dictPath = process.argv[2] || '/tmp/batch_0004.rdict';
const word = process.argv[3] || 'run';

console.log(`Opening ${dictPath} ...`);
const dict = new Dictionary(dictPath);

const m = dict.manifest();
console.log(`\nManifest:`);
console.log(`  name:         ${m.name}`);
console.log(`  version:      ${m.version}`);
console.log(`  source_lang:  ${m.source_lang}`);
console.log(`  target_langs: ${m.target_langs.join(', ')}`);
console.log(`  entry_count:  ${m.entry_count}`);
console.log(`  block_count:  ${m.block_count}`);

const headwords = dict.listHeadwords();
console.log(`\nHeadwords (${headwords.length}):`);
console.log(`  ${headwords.slice(0, 10).join(', ')}${headwords.length > 10 ? ', ...' : ''}`);

console.log(`\nLooking up "${word}" ...`);
const entry = dict.lookup(word);

if (!entry) {
  console.log('  Not found.');
  process.exit(0);
}

console.log(`\n=== ${entry.headword} ===`);

if (entry.see) {
  console.log(`  → see: ${entry.see}`);
}

if (entry.tags.length > 0) {
  console.log(`  tags: ${entry.tags.join(', ')}`);
}

for (const pron of entry.pron) {
  const lang = pron.lang ? `[${pron.lang}] ` : '';
  const kind = pron.kind ? `(${pron.kind}) ` : '';
  console.log(`  pron: ${lang}${kind}${pron.value}`);
}

if (entry.morphology.length > 0) {
  console.log(`  morphology:`);
  for (const morph of entry.morphology) {
    const kind = morph.kind ? `[${morph.kind}] ` : '';
    console.log(`    ${kind}${morph.term}`);
  }
}

for (let i = 0; i < entry.etymologies.length; i++) {
  const ety = entry.etymologies[i];
  console.log(`\n  Ety #${i + 1}${ety.id ? ` (${ety.id})` : ''}:`);
  if (ety.root) {
    console.log(`    root: ${ety.root}`);
  }
  for (let j = 0; j < ety.senses.length; j++) {
    const sense = ety.senses[j];
    console.log(`\n    Sense #${j + 1}:`);
    if (sense.pos) console.log(`      pos: ${sense.pos}`);
    if (sense.lemma) console.log(`      lemma: ${sense.lemma}`);

    for (const tr of sense.translations) {
      const lang = tr.lang ? `[${tr.lang}] ` : '';
      console.log(`      translation: ${lang}${tr.value}`);
    }

    for (let k = 0; k < sense.definitions.length; k++) {
      const def = sense.definitions[k];
      if (def.Definition) {
        const d = def.Definition;
        console.log(`      def #${k + 1}: ${d.value}`);
        for (const ex of d.examples) {
          console.log(`        example: ${ex.value}`);
          for (const tr of ex.translations) {
            const lang = tr.lang ? `[${tr.lang}] ` : '';
            console.log(`          → ${lang}${tr.value}`);
          }
        }
        for (const note of d.notes) {
          console.log(`        note: ${note.value}`);
        }
      } else if (def.Group) {
        const g = def.Group;
        console.log(`      group: ${g.description || '(no desc)'}`);
      }
    }

    if (sense.forms.length > 0) {
      console.log(`      forms:`);
      for (const form of sense.forms) {
        const kind = form.kind ? `[${form.kind}] ` : '';
        console.log(`        ${kind}${form.term}`);
        if (form.feats) console.log(`          feats: ${form.feats}`);
      }
    }
  }
}

console.log('\nDone.');
