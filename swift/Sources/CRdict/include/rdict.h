#ifndef RDICT_H
#define RDICT_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to an open .rdict reader. */
typedef struct RdictHandle RdictHandle;

/*
 * Open a .rdict file by path.
 * Returns a handle, or NULL on failure.
 * Caller must close with rdict_close().
 */
RdictHandle *rdict_open(const char *path);

/*
 * Look up a headword.
 * Returns a JSON string (see below), or NULL on failure.
 * Caller MUST free the returned string with rdict_free_string().
 *
 * JSON formats:
 *   {"ok":true,"found":true,"opaque":false,"entry":{...}}
 *   {"ok":true,"found":true,"opaque":true}
 *   {"ok":true,"found":false}
 *   {"ok":false,"error":"message"}
 */
char *rdict_lookup(RdictHandle *handle, const char *headword);

/*
 * List all headwords in the dictionary (sorted).
 * Returns a JSON string, or NULL on failure.
 * Caller MUST free with rdict_free_string().
 *
 * JSON format:
 *   {"ok":true,"count":42,"headwords":["a","b",...]}
 *   {"ok":false,"count":0,"headwords":[],"error":"msg"}
 */
char *rdict_list_headwords(RdictHandle *handle);

/*
 * Case-insensitive prefix search.
 * Returns up to `limit` headwords whose lowercased form starts with `prefix`.
 * Headwords are in original case, in sorted order.
 * Returns a JSON string, or NULL on failure.
 * Caller MUST free with rdict_free_string().
 *
 * JSON format:
 *   {"ok":true,"count":3,"headwords":["apple","apply","applet"]}
 *   {"ok":false,"count":0,"headwords":[],"error":"msg"}
 */
char *rdict_prefix(RdictHandle *handle, const char *prefix, uint32_t limit);

/*
 * Read a media file's bytes by (kind, hash_hex).
 * Automatically decompresses zstd-compressed media.
 * Returns a pointer to the bytes, writes the length to *out_len.
 * Returns NULL on failure.
 * Caller MUST free with rdict_free_media().
 */
void *rdict_read_media(RdictHandle *handle, const char *kind, const char *hash, uint64_t *out_len);

/*
 * Extract a media file to disk via streaming.
 * Creates parent directories and writes atomically (temp + rename).
 * Returns bytes written, or 0 on failure.
 */
uint64_t rdict_extract_media(RdictHandle *handle, const char *kind, const char *hash, const char *output_path);

/*
 * Get media info (metadata) by (kind, hash_hex).
 * Returns a JSON string, or NULL on failure.
 * Returns {"found":false} if not found.
 * Caller MUST free with rdict_free_string().
 */
char *rdict_media_info(RdictHandle *handle, const char *kind, const char *hash);

/*
 * Free media bytes returned by rdict_read_media.
 * Passing NULL is a no-op.
 */
void rdict_free_media(void *ptr, uint64_t len);

/*
 * Get the media manifest as a JSON string, or NULL on failure.
 * Returns {"ok":false} if no media in the dictionary.
 * Caller MUST free with rdict_free_string().
 */
char *rdict_media_manifest(RdictHandle *handle);

/*
 * Get manifest info (name, languages, entry count).
 * Returns a JSON string, or NULL on failure.
 * Caller MUST free with rdict_free_string().
 */
char *rdict_manifest(RdictHandle *handle);

/*
 * Free a string returned by rdict_lookup, rdict_list_headwords,
 * or rdict_manifest. Passing NULL is a no-op.
 */
void rdict_free_string(char *s);

/*
 * Read the cover image bytes, if present.
 * Returns a pointer to the bytes, writes the length to *out_len.
 * Returns NULL if no cover or on failure.
 * Caller MUST free with rdict_free_media().
 */
void *rdict_read_cover(RdictHandle *handle, uint64_t *out_len);

/*
 * Close a reader and free its resources.
 * Passing NULL is a no-op.
 */
void rdict_close(RdictHandle *handle);

#ifdef __cplusplus
}
#endif

#endif /* RDICT_H */
