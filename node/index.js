// Rdict Node.js binding — JS wrapper with JSON parsing
// The native addon exports a raw `Dictionary` class that returns JSON
// strings. This wrapper parses them into JS objects.

const path = require('path');

function loadNative() {
  const platform = `${process.platform}-${process.arch}`;
  const candidates = [
    path.join(__dirname, `rdict.${platform}.node`),
    path.join(__dirname, `rdict.${platform}-gnu.node`),
    path.join(__dirname, `rdict.${platform}-musl.node`),
    path.join(__dirname, `rdict.${platform}-msvc.node`),
    path.join(__dirname, 'rdict.node'),
  ];
  for (const p of candidates) {
    try {
      return require(p);
    } catch {
      // try next
    }
  }
  throw new Error(`Cannot find native addon for platform ${platform}`);
}

const { Dictionary: NativeDictionary } = loadNative();

/**
 * @typedef {Object} Morpheme
 * @property {string|null} kind
 * @property {string} term
 */

/**
 * @typedef {Object} MediaRef
 * @property {string} kind - 'audio' | 'image' | 'video'
 * @property {string} hash - SHA-1 hex string
 * @property {string|null} description
 * @property {string|null} alt
 */

/**
 * @typedef {Object} Pron
 * @property {string|null} lang
 * @property {string|null} accent
 * @property {string|null} kind
 * @property {string} value
 * @property {MediaRef[]} media
 */

/**
 * @typedef {Object} Translation
 * @property {string|null} lang
 * @property {string} value
 * @property {Pron[]} pron
 */

/**
 * @typedef {Object} Form
 * @property {string|null} kind
 * @property {string} term
 * @property {string[]} tags
 * @property {string|null} feats
 * @property {Pron[]} pron
 */

/**
 * @typedef {Object} TextSpan
 * @property {number} offset
 * @property {number} length
 */

/**
 * @typedef {Object} TargetOccurrence
 * @property {TextSpan[]} spans
 */

/**
 * @typedef {Object} Example
 * @property {string} value
 * @property {Translation[]} translations
 * @property {Pron[]} pron
 * @property {TargetOccurrence[]} targets
 * @property {MediaRef[]} media
 */

/**
 * @typedef {Object} Note
 * @property {string|null} id
 * @property {string} value
 * @property {Example[]} examples
 */

/**
 * @typedef {Object} Definition
 * @property {string|null} id
 * @property {string} value
 * @property {Example[]} examples
 * @property {Note[]} notes
 * @property {MediaRef[]} media
 */

/**
 * @typedef {{Definition: Definition} | {Group: {id: string|null, description: string|null, definitions: Definition[]}}} Def
 */

/**
 * @typedef {Object} Sense
 * @property {string|null} pos - UD v2 UPOS (NOUN, VERB, ADJ, ...)
 * @property {string|null} lemma
 * @property {Translation[]} translations
 * @property {Form[]} forms
 * @property {string[]} tags
 * @property {Pron[]} pron
 * @property {Def[]} definitions
 */

/**
 * @typedef {Object} Ety
 * @property {string|null} id
 * @property {string|null} root
 * @property {Sense[]} senses
 */

/**
 * @typedef {Object} Relation
 * @property {string|null} type_
 * @property {string} target
 */

/**
 * @typedef {Object} Entry
 * @property {string} headword
 * @property {string|null} see
 * @property {string[]} tags
 * @property {Pron[]} pron
 * @property {Ety[]} etymologies
 * @property {Morpheme[]} morphology
 * @property {Relation[]} relations
 * @property {MediaRef[]} media
 */

/**
 * @typedef {Object} Manifest
 * @property {string} name
 * @property {string} version
 * @property {string} source_lang
 * @property {string[]} target_langs
 * @property {number} entry_count
 * @property {number} block_count
 */

/**
 * @typedef {Object} MediaManifestEntry
 * @property {string} hash
 * @property {string} kind
 * @property {string} ext
 * @property {string} mime
 * @property {string} compression
 * @property {number} size
 * @property {number} uncompressed_size
 */

/**
 * @typedef {Object} MediaKey
 * @property {'audio'|'image'|'video'} kind
 * @property {string} hash - SHA-1 hex string
 */

/**
 * @typedef {MediaManifestEntry} MediaInfo
 */

class RdictDictionary {
  /**
   * @param {string} path - Path to the .rdict file
   */
  constructor(path) {
    this._dict = new NativeDictionary(path);
  }

  /**
   * Look up a headword.
   * @param {string} headword
   * @returns {Entry|null} The entry, or null if not found
   * @throws {Error} If the entry is opaque or lookup fails
   */
  lookup(headword) {
    const json = this._dict.lookup(headword);
    return json ? JSON.parse(json) : null;
  }

  /**
   * List all headwords in the dictionary (sorted).
   * @returns {string[]}
   */
  listHeadwords() {
    return this._dict.listHeadwords();
  }

  /**
   * Case-insensitive prefix search.
   * @param {string} prefix
   * @param {number} [limit=20] - Max results (default 20, <=0 returns [])
   * @returns {string[]} Headwords in original case, in sorted order
   */
  prefix(prefix, limit) {
    return this._dict.prefix(prefix, limit);
  }

  /**
   * Get manifest info (name, languages, version, entry count).
   * @returns {Manifest}
   */
  manifest() {
    return JSON.parse(this._dict.manifest());
  }

  /**
   * Get the media manifest entries, or null if no media.
   * @returns {MediaManifestEntry[] | null}
   */
  mediaManifest() {
    const json = this._dict.mediaManifest();
    if (json === null) return null;
    const manifest = JSON.parse(json);
    return manifest.entries;
  }

  /**
   * Get media metadata by key (no content read).
   * @param {MediaKey} key - { kind, hash }
   * @returns {MediaInfo | null} Metadata, or null if not found.
   */
  mediaInfo(key) {
    const json = this._dict.mediaInfo(key.kind, key.hash);
    return json ? JSON.parse(json) : null;
  }

  /**
   * Read a media file's bytes by key. Automatically decompresses
   * zstd-compressed media.
   * @param {MediaKey} key - { kind, hash }
   * @returns {Buffer} File bytes.
   * @throws {Error} If the media is not found or read fails.
   */
  readMedia(key) {
    return this._dict.readMedia(key.kind, key.hash);
  }

  /**
   * Read the cover image bytes, if present.
   * @returns {Buffer | null} Cover image bytes, or null if no cover.
   */
  readCover() {
    return this._dict.readCover();
  }

  /**
   * Extract a media file to disk via streaming (for large files like
   * video). Creates parent directories and writes atomically.
   * @param {MediaKey} key - { kind, hash }
   * @param {string} outputPath - destination file path
   * @returns {number} Bytes written.
   * @throws {Error} If extraction fails.
   */
  extractMedia(key, outputPath) {
    return this._dict.extractMedia(key.kind, key.hash, outputPath);
  }
}

module.exports = { Dictionary: RdictDictionary };
