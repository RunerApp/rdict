import Foundation
import CRdict

/// A reader for `.rdict` dictionary files.
/// Wraps the C FFI and provides a Swift-friendly API.
public final class RdictDictionary {
    private let bridge = RdictBridge()

    /// Open a `.rdict` file by path.
    /// Returns `nil` if the file cannot be opened.
    public init(path: String) {
        _ = bridge.open(path: path)
    }

    /// Whether a dictionary is currently open.
    public var isOpen: Bool {
        bridge.isOpen
    }

    /// Look up a headword.
    /// - Parameter headword: The word to look up.
    /// - Returns: The entry, or `nil` if not found.
    /// - Throws: `LookupError` on decode failure or opaque entry.
    public func lookup(_ headword: String) throws -> DictEntry? {
        switch bridge.lookup(headword) {
        case .success(let entry): return entry
        case .failure(let error): throw error
        }
    }

    /// List all headwords in the dictionary (sorted).
    public func listHeadwords() -> [String] {
        bridge.listHeadwords()
    }

    /// Case-insensitive prefix search.
    /// - Parameters:
    ///   - prefix: The prefix to search for (case-insensitive).
    ///   - limit: Maximum number of results (default 20, <=0 returns empty).
    /// - Returns: Headwords in original case, in sorted order.
    public func prefix(_ prefix: String, limit: Int = 20) -> [String] {
        bridge.prefix(prefix, limit: limit)
    }

    /// Read media bytes by (kind, hash). Automatically decompresses zstd.
    /// - Parameters:
    ///   - kind: Media kind ("audio", "image", or "video").
    ///   - hash: SHA-1 hex string.
    /// - Returns: The media file bytes, or nil if not found.
    public func readMedia(kind: String, hash: String) -> Data? {
        bridge.readMedia(kind: kind, hash: hash)
    }

    /// Extract media to a file via streaming.
    /// Creates parent directories and writes atomically.
    /// - Parameters:
    ///   - kind: Media kind ("audio", "image", or "video").
    ///   - hash: SHA-1 hex string.
    ///   - outputPath: Destination file path.
    /// - Returns: Bytes written, or nil on failure.
    public func extractMedia(kind: String, hash: String, outputPath: String) -> UInt64? {
        bridge.extractMedia(kind: kind, hash: hash, outputPath: outputPath)
    }

    /// Get media info (metadata only, no content read).
    /// - Parameters:
    ///   - kind: Media kind ("audio", "image", or "video").
    ///   - hash: SHA-1 hex string.
    /// - Returns: Media info, or nil if not found.
    public func mediaInfo(kind: String, hash: String) -> MediaManifestEntry? {
        bridge.mediaInfo(kind: kind, hash: hash)
    }

    /// Get the media manifest entries.
    /// - Returns: Array of manifest entries, or nil if no media.
    public func mediaManifest() -> [MediaManifestEntry]? {
        bridge.mediaManifest()
    }

    /// Get manifest info (name, languages, version, entry count).
    public func manifest() -> ManifestResponse? {
        bridge.manifestInfo()
    }

    /// Read the cover image bytes.
    /// - Returns: The cover image data, or nil if no cover.
    public func readCover() -> Data? {
        bridge.readCover()
    }

    /// Close the dictionary and release resources.
    public func close() {
        bridge.close()
    }

    deinit { close() }
}
