import Foundation
import CRdict

/// Wrapper around the C FFI for Rdict.
final class RdictBridge {
    private var handle: OpaquePointer?

    var isOpen: Bool { handle != nil }

    /// Open a .rdict file. Returns nil on failure.
    func open(path: String) -> Bool {
        close()
        handle = path.withCString { cPath in
            rdict_open(cPath)
        }
        return handle != nil
    }

    /// Look up a headword. Returns the entry if found.
    func lookup(_ headword: String) -> Result<DictEntry?, LookupError> {
        guard let h = handle else { return .failure(.notOpen) }
        guard let cJson = headword.withCString({ rdict_lookup(h, $0) }) else {
            return .failure(.ffiError)
        }
        defer { rdict_free_string(cJson) }

        let json = String(cString: cJson)
        guard let data = json.data(using: .utf8) else {
            return .failure(.ffiError)
        }

        do {
            let resp = try JSONDecoder().decode(LookupResponse.self, from: data)
            if !resp.ok {
                return .failure(.lookupFailed(resp.error ?? "unknown"))
            }
            if resp.opaque == true {
                return .failure(.opaque)
            }
            return .success(resp.entry)
        } catch {
            return .failure(.decodeError(error))
        }
    }

    /// List all headwords in the dictionary.
    func listHeadwords() -> [String] {
        guard let h = handle else { return [] }
        guard let cJson = rdict_list_headwords(h) else { return [] }
        defer { rdict_free_string(cJson) }

        let json = String(cString: cJson)
        guard let data = json.data(using: .utf8),
              let resp = try? JSONDecoder().decode(ListResponse.self, from: data),
              resp.ok
        else { return [] }

        return resp.headwords
    }

    /// Case-insensitive prefix search. Returns up to `limit` headwords.
    func prefix(_ prefix: String, limit: Int = 20) -> [String] {
        guard let h = handle else { return [] }
        guard let cJson = prefix.withCString({ rdict_prefix(h, $0, UInt32(limit)) })
        else { return [] }
        defer { rdict_free_string(cJson) }

        let json = String(cString: cJson)
        guard let data = json.data(using: .utf8),
              let resp = try? JSONDecoder().decode(PrefixResponse.self, from: data),
              resp.ok
        else { return [] }

        return resp.headwords
    }

    /// Read media bytes by (kind, hash). Automatically decompresses zstd.
    /// Returns nil on failure.
    func readMedia(kind: String, hash: String) -> Data? {
        guard let h = handle else { return nil }
        var len: UInt64 = 0
        guard let ptr = kind.withCString({ kc in
            hash.withCString({ hc in
                rdict_read_media(h, kc, hc, &len)
            })
        }), len > 0
        else { return nil }
        defer { rdict_free_media(ptr, len) }
        return Data(bytes: ptr, count: Int(len))
    }

    /// Extract media to a file via streaming. Returns bytes written, or nil on failure.
    func extractMedia(kind: String, hash: String, outputPath: String) -> UInt64? {
        guard let h = handle else { return nil }
        let n = kind.withCString({ kc in
            hash.withCString({ hc in
                outputPath.withCString({ op in
                    rdict_extract_media(h, kc, hc, op)
                })
            })
        })
        return n > 0 ? n : nil
    }

    /// Get media info by (kind, hash). Returns nil if not found.
    func mediaInfo(kind: String, hash: String) -> MediaManifestEntry? {
        guard let h = handle else { return nil }
        guard let cJson = kind.withCString({ kc in
            hash.withCString({ hc in
                rdict_media_info(h, kc, hc)
            })
        }) else { return nil }
        defer { rdict_free_string(cJson) }

        let json = String(cString: cJson)
        guard let data = json.data(using: .utf8) else { return nil }
        return try? JSONDecoder().decode(MediaManifestEntry.self, from: data)
    }

    /// Get the media manifest entries. Returns nil if no media.
    func mediaManifest() -> [MediaManifestEntry]? {
        guard let h = handle else { return nil }
        guard let cJson = rdict_media_manifest(h) else { return nil }
        defer { rdict_free_string(cJson) }

        let json = String(cString: cJson)
        guard let data = json.data(using: .utf8) else { return nil }

        struct ManifestWrapper: Codable {
            let hash_algo: String
            let entries: [MediaManifestEntry]
        }
        guard let wrapper = try? JSONDecoder().decode(ManifestWrapper.self, from: data)
        else { return nil }
        return wrapper.entries
    }

    /// Get manifest info.
    func manifestInfo() -> ManifestResponse? {
        guard let h = handle else { return nil }
        guard let cJson = rdict_manifest(h) else { return nil }
        defer { rdict_free_string(cJson) }

        let json = String(cString: cJson)
        guard let data = json.data(using: .utf8) else { return nil }
        return try? JSONDecoder().decode(ManifestResponse.self, from: data)
    }

    /// Read the cover image bytes. Returns nil if no cover.
    func readCover() -> Data? {
        guard let h = handle else { return nil }
        var len: UInt64 = 0
        guard let ptr = rdict_read_cover(h, &len), len > 0 else { return nil }
        defer { rdict_free_media(ptr, len) }
        return Data(bytes: ptr, count: Int(len))
    }

    /// Close the dictionary.
    func close() {
        if let h = handle {
            rdict_close(h)
            handle = nil
        }
    }

    deinit { close() }
}

public enum LookupError: Error, LocalizedError {
    case notOpen
    case ffiError
    case opaque
    case lookupFailed(String)
    case decodeError(Error)

    public var errorDescription: String? {
        switch self {
        case .notOpen: "Dictionary not open"
        case .ffiError: "FFI error"
        case .opaque: "Entry is opaque (unknown format)"
        case .lookupFailed(let msg): "Lookup failed: \(msg)"
        case .decodeError(let e): "Decode error: \(e.localizedDescription)"
        }
    }
}
