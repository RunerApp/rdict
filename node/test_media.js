const { Dictionary } = require("./index.js");
const assert = require("assert");
const fs = require("fs");
const path = require("path");
const os = require("os");

const DICT_PATH = path.join(__dirname, "..", "media-test.rdict");

function test() {
  console.log("=== Rdict Node.js Media API Tests ===\n");

  // Build test dictionary if it doesn't exist
  if (!fs.existsSync(DICT_PATH)) {
    console.log("Building test dictionary...");
    const { execSync } = require("child_process");
    execSync(
      "cargo run --release --manifest-path ../core/Cargo.toml --example build_media_dict -- " +
        "../tests/fixtures/mp3_15s_sample_file_236KB.mp3 " +
        "../tests/fixtures/png_1000x600_sample_file_21KB.png " +
        "../media-test.rdict",
      { stdio: "inherit" }
    );
  }

  // 1. Open
  console.log("1. Open dictionary with media...");
  const dict = new Dictionary(DICT_PATH);
  console.log("   OK\n");

  // 2. Media manifest
  console.log("2. Media manifest...");
  const entries = dict.mediaManifest();
  assert.ok(entries, "mediaManifest should return entries");
  assert.strictEqual(entries.length, 2);

  const audioEntry = entries.find((e) => e.kind === "audio");
  const imageEntry = entries.find((e) => e.kind === "image");
  assert.ok(audioEntry, "should have audio entry");
  assert.ok(imageEntry, "should have image entry");
  assert.strictEqual(audioEntry.ext, "mp3");
  assert.strictEqual(imageEntry.ext, "png");
  assert.strictEqual(audioEntry.mime, "audio/mpeg");
  assert.strictEqual(imageEntry.mime, "image/png");
  assert.ok(audioEntry.size > 0);
  assert.ok(imageEntry.size > 0);
  console.log(
    `   audio: ${audioEntry.size} bytes, image: ${imageEntry.size} bytes`
  );
  console.log("   OK\n");

  // 3. Read media (Buffer) via MediaKey
  console.log("3. Read media as Buffer (via MediaKey)...");
  const audioKey = { kind: "audio", hash: audioEntry.hash };
  const audioBuf = dict.readMedia(audioKey);
  assert.ok(Buffer.isBuffer(audioBuf), "should be a Buffer");
  assert.strictEqual(audioBuf.length, audioEntry.size);
  // Verify MP3 magic bytes (ID3)
  assert.strictEqual(audioBuf[0], 0x49); // 'I'
  assert.strictEqual(audioBuf[1], 0x44); // 'D'
  assert.strictEqual(audioBuf[2], 0x33); // '3'
  console.log(`   audio buffer: ${audioBuf.length} bytes, ID3 header OK`);
  console.log("   OK\n");

  // 4. Read image (Buffer) via MediaKey
  console.log("4. Read image as Buffer (via MediaKey)...");
  const imageKey = { kind: "image", hash: imageEntry.hash };
  const imageBuf = dict.readMedia(imageKey);
  assert.ok(Buffer.isBuffer(imageBuf), "should be a Buffer");
  assert.strictEqual(imageBuf.length, imageEntry.size);
  // Verify PNG magic bytes
  assert.strictEqual(imageBuf[0], 0x89);
  assert.strictEqual(imageBuf[1], 0x50); // 'P'
  assert.strictEqual(imageBuf[2], 0x4e); // 'N'
  assert.strictEqual(imageBuf[3], 0x47); // 'G'
  console.log(`   image buffer: ${imageBuf.length} bytes, PNG header OK`);
  console.log("   OK\n");

  // 5. mediaInfo
  console.log("5. mediaInfo...");
  const info = dict.mediaInfo(audioKey);
  assert.ok(info, "mediaInfo should return info");
  assert.strictEqual(info.hash, audioEntry.hash);
  assert.strictEqual(info.kind, "audio");
  assert.strictEqual(info.size, audioEntry.size);
  console.log(`   mediaInfo: ${info.kind} ${info.ext} ${info.size} bytes`);
  console.log("   OK\n");

  // 6. mediaInfo for non-existent media
  console.log("6. mediaInfo for non-existent media...");
  const noInfo = dict.mediaInfo({ kind: "audio", hash: "nonexistent" });
  assert.strictEqual(noInfo, null);
  console.log("   returns null (OK)\n");

  // 7. Extract media to file via MediaKey
  console.log("7. Extract media to file (via MediaKey)...");
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "rdict-test-"));
  const tmpImage = path.join(tmpDir, "subdir", "test.png");
  const written = dict.extractMedia(imageKey, tmpImage);
  assert.strictEqual(written, imageEntry.size);
  const fileData = fs.readFileSync(tmpImage);
  assert.strictEqual(fileData.length, imageEntry.size);
  assert.deepStrictEqual(fileData, imageBuf);
  console.log(`   extracted ${written} bytes to ${tmpImage} (with subdir creation)`);
  fs.rmSync(tmpDir, { recursive: true });
  console.log("   OK\n");

  // 8. Read non-existent media throws
  console.log("8. Read non-existent media throws...");
  try {
    dict.readMedia({ kind: "audio", hash: "nonexistent" });
    assert.fail("should have thrown");
  } catch (e) {
    assert.ok(e.message.includes("not found"));
  }
  console.log("   throws error (OK)\n");

  // 9. Entry has media refs
  console.log("9. Verify entry media refs...");
  const entry = dict.lookup("hello");
  assert.ok(entry);
  assert.strictEqual(entry.media.length, 1);
  assert.strictEqual(entry.media[0].kind, "audio");
  assert.ok(entry.media[0].hash.length > 0);

  const def = entry.etymologies[0].senses[0].definitions[0].Definition;
  assert.strictEqual(def.media.length, 1);
  assert.strictEqual(def.media[0].kind, "image");
  console.log("   entry.media[0].kind = audio");
  console.log("   definition.media[0].kind = image");
  console.log("   OK\n");

  console.log("=== All media tests passed! ===");
}

try {
  test();
} catch (e) {
  console.error("TEST FAILED:", e.message);
  console.error(e.stack);
  process.exit(1);
}
