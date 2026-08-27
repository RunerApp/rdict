const { Dictionary } = require('./index.js');
const assert = require('assert');

const RICT_PATH = '/tmp/batch_0004.rdict';

function test() {
  console.log('=== Rdict Node.js Binding Tests ===\n');

  // 1. Open
  console.log('1. Open dictionary...');
  const dict = new Dictionary(RICT_PATH);
  console.log('   OK\n');

  // 2. Manifest
  console.log('2. Manifest...');
  const m = dict.manifest();
  assert.strictEqual(m.name, 'English-Chinese (NGSL)');
  assert.strictEqual(m.source_lang, 'en');
  assert.deepStrictEqual(m.target_langs, ['zh-Hans']);
  assert.strictEqual(m.entry_count, 50);
  assert.ok(m.block_count >= 1);
  console.log(`   name=${m.name}, entries=${m.entry_count}, blocks=${m.block_count}`);
  console.log('   OK\n');

  // 3. List headwords
  console.log('3. List headwords...');
  const headwords = dict.listHeadwords();
  assert.strictEqual(headwords.length, 50);
  assert.ok(headwords.includes('run'));
  assert.ok(headwords.includes('house'));
  assert.ok(headwords.includes('always'));
  console.log(`   ${headwords.length} headwords, first=${headwords[0]}, last=${headwords[headwords.length - 1]}`);
  console.log('   OK\n');

  // 4. Lookup existing word
  console.log('4. Lookup "run"...');
  const run = dict.lookup('run');
  assert.ok(run);
  assert.strictEqual(run.headword, 'run');
  assert.ok(run.etymologies.length > 0);
  assert.strictEqual(run.etymologies[0].root, 'rinnan');
  assert.ok(run.etymologies[0].senses.length > 0);
  const sense = run.etymologies[0].senses[0];
  assert.ok(sense.pos);
  assert.ok(sense.translations.length > 0);
  assert.strictEqual(sense.translations[0].lang, 'zh-Hans');
  console.log(`   headword=${run.headword}, root=${run.etymologies[0].root}, pos=${sense.pos}`);
  console.log(`   translation=${sense.translations[0].value}`);
  console.log('   OK\n');

  // 5. Lookup with morphology (need to compile test file first)
  console.log('5. Lookup non-existent word...');
  const nope = dict.lookup('nonexistent');
  assert.strictEqual(nope, null);
  console.log('   Returns null (OK)\n');

  // 6. Lookup all words and verify
  console.log('6. Lookup all 50 words...');
  let found = 0;
  let withRoot = 0;
  let withExamples = 0;
  for (const hw of headwords) {
    const entry = dict.lookup(hw);
    if (entry) {
      found++;
      if (entry.etymologies[0]?.root) withRoot++;
      const def = entry.etymologies[0]?.senses[0]?.definitions?.[0];
      if (def?.Definition?.examples?.length > 0) withExamples++;
    }
  }
  assert.strictEqual(found, 50);
  console.log(`   Found: ${found}/50`);
  console.log(`   With root: ${withRoot}/50`);
  console.log(`   With examples: ${withExamples}/50`);
  console.log('   OK\n');

  // 7. Verify translation lang auto-fill
  console.log('7. Verify translation lang auto-fill...');
  const live = dict.lookup('live');
  for (const ety of live.etymologies) {
    for (const s of ety.senses) {
      for (const tr of s.translations) {
        assert.strictEqual(tr.lang, 'zh-Hans', `Translation lang not filled for "${live.headword}"`);
      }
    }
  }
  console.log('   All translations have lang=zh-Hans');
  console.log('   OK\n');

  console.log('=== All tests passed! ===');
}

try {
  test();
} catch (e) {
  console.error('TEST FAILED:', e.message);
  process.exit(1);
}
