//! C ABI for FFI consumers (Swift, Python, etc.).
//!
//! All strings returned from these functions are heap-allocated UTF-8 C
//! strings (NUL-terminated). The caller MUST free them with
//! `rdict_free_string`. Passing a returned pointer to any function other
//! than `rdict_free_string` is undefined behavior.

use crate::reader::{LookupEntry, RdictReader};
use serde::Serialize;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::os::raw::c_char;
use std::ptr;

/// Opaque handle to an open `.rdict` reader.
pub struct RdictHandle {
    reader: RdictReader<File>,
}

/// Response envelope for FFI JSON lookup calls.
#[derive(Serialize)]
struct LookupResponse {
    ok: bool,
    found: bool,
    opaque: bool,
    error: Option<String>,
    entry: Option<crate::model::Entry>,
}

#[derive(Serialize)]
struct ListResponse {
    ok: bool,
    count: u32,
    headwords: Vec<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct PrefixResponse {
    ok: bool,
    count: u32,
    headwords: Vec<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct ManifestResponse {
    ok: bool,
    error: Option<String>,
    name: String,
    source_lang: String,
    target_langs: Vec<String>,
    version: String,
    entry_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    cover: Option<String>,
}

fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr).to_str().ok() }
}

fn json_to_cstring<T: Serialize>(value: &T) -> *mut c_char {
    match serde_json::to_string(value) {
        Ok(json) => CString::into_raw(CString::new(json).unwrap_or_default()),
        Err(_) => ptr::null_mut(),
    }
}

/// Open a `.rdict` file. Returns a heap-allocated handle, or NULL on
/// failure. The caller must close it with `rdict_close`.
///
/// # Safety
/// `path` must be a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rdict_open(path: *const c_char) -> *mut RdictHandle {
    let Some(path_str) = cstr_to_str(path) else {
        return ptr::null_mut();
    };
    match RdictReader::open(path_str) {
        Ok(reader) => Box::into_raw(Box::new(RdictHandle { reader })),
        Err(_) => ptr::null_mut(),
    }
}

/// Look up a headword. Returns a JSON string (caller must free with
/// `rdict_free_string`), or NULL on catastrophic failure.
///
/// JSON format:
/// ```json
/// {"ok":true,"found":true,"opaque":false,"entry":{...}}
/// {"ok":true,"found":false}
/// {"ok":false,"error":"message"}
/// ```
///
/// # Safety
/// `handle` must be a valid pointer from `rdict_open`.
/// `headword` must be a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rdict_lookup(
    handle: *mut RdictHandle,
    headword: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    let Some(hw_str) = cstr_to_str(headword) else {
        return ptr::null_mut();
    };
    let handle = unsafe { &mut *handle };

    let response = match handle.reader.lookup(hw_str) {
        Ok(Some(LookupEntry::Decoded(entry))) => LookupResponse {
            ok: true,
            found: true,
            opaque: false,
            error: None,
            entry: Some((*entry).clone()),
        },
        Ok(Some(LookupEntry::Opaque { .. })) => LookupResponse {
            ok: true,
            found: true,
            opaque: true,
            error: None,
            entry: None,
        },
        Ok(None) => LookupResponse {
            ok: true,
            found: false,
            opaque: false,
            error: None,
            entry: None,
        },
        Err(e) => LookupResponse {
            ok: false,
            found: false,
            opaque: false,
            error: Some(e.to_string()),
            entry: None,
        },
    };

    json_to_cstring(&response)
}

/// List all headwords in the dictionary. Returns a JSON string (caller
/// must free with `rdict_free_string`), or NULL on failure.
///
/// # Safety
/// `handle` must be a valid pointer from `rdict_open`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rdict_list_headwords(handle: *mut RdictHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    let handle = unsafe { &*handle };

    let response = match handle.reader.list_headwords() {
        Ok(headwords) => ListResponse {
            ok: true,
            count: headwords.len() as u32,
            headwords,
            error: None,
        },
        Err(e) => ListResponse {
            ok: false,
            count: 0,
            headwords: Vec::new(),
            error: Some(e.to_string()),
        },
    };

    json_to_cstring(&response)
}

/// Case-insensitive prefix search. Returns up to `limit` headwords as a
/// JSON string (caller must free), or NULL on failure.
///
/// JSON format:
/// ```json
/// {"ok":true,"count":3,"headwords":["apple","apply","applet"]}
/// {"ok":false,"count":0,"headwords":[],"error":"message"}
/// ```
///
/// # Safety
/// `handle` must be a valid pointer from `rdict_open`.
/// `prefix` must be a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rdict_prefix(
    handle: *mut RdictHandle,
    prefix: *const c_char,
    limit: u32,
) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    let Some(prefix_str) = cstr_to_str(prefix) else {
        return ptr::null_mut();
    };
    let handle = unsafe { &*handle };
    let headwords = handle.reader.prefix(prefix_str, limit as usize);
    let response = PrefixResponse {
        ok: true,
        count: headwords.len() as u32,
        headwords,
        error: None,
    };
    json_to_cstring(&response)
}

/// Read a media file's bytes by (kind, hash_hex). Automatically
/// decompresses zstd-compressed media. Returns a pointer to the bytes
/// and writes the length to `out_len`. Returns NULL on failure. The
/// caller MUST free the returned pointer with `rdict_free_media`.
///
/// # Safety
/// `handle` must be a valid pointer from `rdict_open`.
/// `kind` and `hash` must be valid NUL-terminated C strings.
/// `out_len` must be a valid pointer to u64.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rdict_read_media(
    handle: *mut RdictHandle,
    kind: *const c_char,
    hash: *const c_char,
    out_len: *mut u64,
) -> *mut u8 {
    if handle.is_null() || kind.is_null() || hash.is_null() || out_len.is_null() {
        return ptr::null_mut();
    }
    let Some(kind_str) = cstr_to_str(kind) else {
        return ptr::null_mut();
    };
    let Some(hash_str) = cstr_to_str(hash) else {
        return ptr::null_mut();
    };
    let handle = unsafe { &mut *handle };
    match handle.reader.read_media(kind_str, hash_str) {
        Ok(bytes) => {
            let len = bytes.len();
            let layout = std::alloc::Layout::from_size_align(len, 1).unwrap();
            let ptr = unsafe { std::alloc::alloc(layout) };
            if ptr.is_null() {
                return ptr::null_mut();
            }
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, len);
            }
            unsafe { *out_len = len as u64 };
            ptr
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Free media bytes returned by `rdict_read_media`.
///
/// # Safety
/// `ptr` must be a pointer previously returned by `rdict_read_media`,
/// and `len` must be the length that was written to `out_len`. Passing
/// NULL is a no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rdict_free_media(ptr: *mut u8, len: u64) {
    if !ptr.is_null() && len > 0 {
        let layout = std::alloc::Layout::from_size_align(len as usize, 1).unwrap();
        unsafe { std::alloc::dealloc(ptr, layout) };
    }
}

/// Extract a media file to a file on disk via streaming. Creates parent
/// directories and writes atomically (temp file + rename). Returns the
/// bytes written, or 0 on failure.
///
/// # Safety
/// `handle` must be a valid pointer from `rdict_open`.
/// `kind`, `hash`, and `output_path` must be valid NUL-terminated C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rdict_extract_media(
    handle: *mut RdictHandle,
    kind: *const c_char,
    hash: *const c_char,
    output_path: *const c_char,
) -> u64 {
    if handle.is_null() || kind.is_null() || hash.is_null() || output_path.is_null() {
        return 0;
    }
    let Some(kind_str) = cstr_to_str(kind) else {
        return 0;
    };
    let Some(hash_str) = cstr_to_str(hash) else {
        return 0;
    };
    let Some(path_str) = cstr_to_str(output_path) else {
        return 0;
    };
    let handle = unsafe { &mut *handle };
    handle
        .reader
        .extract_media(kind_str, hash_str, std::path::Path::new(path_str))
        .unwrap_or(0)
}

/// Get media info (manifest entry) by (kind, hash_hex). Returns a JSON
/// string (caller must free), or NULL on failure. Returns `{"found":false}`
/// if no matching media.
///
/// # Safety
/// `handle` must be a valid pointer from `rdict_open`.
/// `kind` and `hash` must be valid NUL-terminated C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rdict_media_info(
    handle: *mut RdictHandle,
    kind: *const c_char,
    hash: *const c_char,
) -> *mut c_char {
    if handle.is_null() || kind.is_null() || hash.is_null() {
        return ptr::null_mut();
    }
    let Some(kind_str) = cstr_to_str(kind) else {
        return ptr::null_mut();
    };
    let Some(hash_str) = cstr_to_str(hash) else {
        return ptr::null_mut();
    };
    let handle = unsafe { &mut *handle };
    match handle.reader.media_info(kind_str, hash_str) {
        Ok(Some(entry)) => {
            let json = serde_json::to_string(&entry).unwrap_or_default();
            CString::into_raw(CString::new(json).unwrap_or_default())
        }
        Ok(None) => {
            let json = r#"{"found":false}"#;
            CString::into_raw(CString::new(json).unwrap_or_default())
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Get the media manifest as a JSON string (caller must free), or NULL
/// on failure. Returns `{"ok":false}` if no media in the dictionary.
///
/// # Safety
/// `handle` must be a valid pointer from `rdict_open`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rdict_media_manifest(handle: *mut RdictHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    let handle = unsafe { &mut *handle };
    match handle.reader.media_manifest() {
        Ok(Some(manifest)) => {
            let json = serde_json::to_string(&manifest).unwrap_or_default();
            CString::into_raw(CString::new(json).unwrap_or_default())
        }
        Ok(None) => {
            let json = r#"{"ok":false}"#;
            CString::into_raw(CString::new(json).unwrap_or_default())
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Get basic manifest info. Returns a JSON string (caller must free),
/// or NULL on failure.
///
/// # Safety
/// `handle` must be a valid pointer from `rdict_open`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rdict_manifest(handle: *mut RdictHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    let handle = unsafe { &*handle };
    let m = &handle.reader.manifest();

    let response = ManifestResponse {
        ok: true,
        error: None,
        name: m.pack.name.clone(),
        source_lang: m.pack.source_lang.clone(),
        target_langs: m.pack.target_langs.clone(),
        version: m.pack.version.clone(),
        entry_count: m.index.entry_count,
        cover: m.pack.cover.clone(),
    };

    json_to_cstring(&response)
}

/// Free a string returned by `rdict_lookup`, `rdict_list_headwords`, or
/// `rdict_manifest`.
///
/// # Safety
/// `s` must be a pointer previously returned by one of the above
/// functions, and must not have been freed already. Passing NULL is a
/// no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rdict_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Read the cover image bytes, if present. Returns a pointer to the
/// bytes and writes the length to `out_len`. Returns NULL if no cover
/// or on failure. The caller MUST free with `rdict_free_media`.
///
/// # Safety
/// `handle` must be a valid pointer from `rdict_open`.
/// `out_len` must be a valid pointer to u64.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rdict_read_cover(handle: *mut RdictHandle, out_len: *mut u64) -> *mut u8 {
    if handle.is_null() || out_len.is_null() {
        return ptr::null_mut();
    }
    let handle = unsafe { &mut *handle };
    match handle.reader.read_cover() {
        Ok(Some(bytes)) => {
            let len = bytes.len();
            let layout = std::alloc::Layout::from_size_align(len, 1).unwrap();
            let ptr = unsafe { std::alloc::alloc(layout) };
            if ptr.is_null() {
                return ptr::null_mut();
            }
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, len);
            }
            unsafe { *out_len = len as u64 };
            ptr
        }
        _ => ptr::null_mut(),
    }
}

/// Close a reader and free its resources.
///
/// # Safety
/// `handle` must be a pointer from `rdict_open` and must not have been
/// closed already. Passing NULL is a no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rdict_close(handle: *mut RdictHandle) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}
