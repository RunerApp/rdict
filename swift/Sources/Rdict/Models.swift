import Foundation

// MARK: - FFI Response Types

struct LookupResponse: Codable {
    let ok: Bool
    let found: Bool
    let opaque: Bool?
    let error: String?
    let entry: DictEntry?
}

struct ListResponse: Codable {
    let ok: Bool
    let count: Int
    let headwords: [String]
    let error: String?
}

struct PrefixResponse: Codable {
    let ok: Bool
    let count: Int
    let headwords: [String]
    let error: String?
}

public struct MediaManifestEntry: Codable {
    public let hash: String
    public let kind: String
    public let ext: String
    public let mime: String
    public let compression: String
    public let size: Int
    public let uncompressedSize: Int

    enum CodingKeys: String, CodingKey {
        case hash, kind, ext, mime, compression, size
        case uncompressedSize = "uncompressed_size"
    }
}

public struct ManifestResponse: Codable {
    public let ok: Bool
    public let error: String?
    public let name: String
    public let sourceLang: String
    public let targetLangs: [String]
    public let version: String
    public let entryCount: Int

    enum CodingKeys: String, CodingKey {
        case ok, error, name, version
        case sourceLang = "source_lang"
        case targetLangs = "target_langs"
        case entryCount = "entry_count"
    }
}

// MARK: - Dictionary Model
// Renamed to avoid conflicts with SwiftUI types (Group, Entry, etc.)

public struct DictEntry: Codable {
    public let headword: String
    public let see: String?
    public let tags: [String]
    public let pron: [Pron]
    public let etymologies: [Ety]
    public let morphology: [Morpheme]
    public let relations: [Relation]
    public let media: [MediaRef]
}

public struct Relation: Codable {
    public let type_: String?
    public let target: String

    enum CodingKeys: String, CodingKey {
        case type_ = "type_"
        case target
    }
}

public struct MediaRef: Codable {
    public let kind: String
    public let hash: String
    public let description: String?
    public let alt: String?
}

public struct Morpheme: Codable {
    public let kind: String?
    public let term: String
}

public struct Ety: Codable {
    public let id: String?
    public let root: String?
    public let senses: [Sense]
}

public struct Sense: Codable {
    public let pos: String?
    public let lemma: String?
    public let translations: [Translation]
    public let forms: [Form]
    public let tags: [String]
    public let pron: [Pron]
    public let definitions: [DefItem]
}

public enum DefItem: Codable {
    case definition(Definition)
    case group(DefGroup)

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if let wrapper = try? container.decode([String: AnyCodable].self) {
            if wrapper["Definition"] != nil {
                let def = try container.decode(DefinitionWrapper.self).definition
                self = .definition(def)
                return
            }
            if wrapper["Group"] != nil {
                let group = try container.decode(DefGroupWrapper.self).group
                self = .group(group)
                return
            }
        }
        throw DecodingError.dataCorruptedError(
            in: container, debugDescription: "Expected {\"Definition\":...} or {\"Group\":...}")
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .definition(let d):
            try container.encode(DefinitionWrapper(definition: d))
        case .group(let g):
            try container.encode(DefGroupWrapper(group: g))
        }
    }
}

private struct DefinitionWrapper: Codable {
    let definition: Definition
    enum CodingKeys: String, CodingKey { case definition = "Definition" }
}

private struct DefGroupWrapper: Codable {
    let group: DefGroup
    enum CodingKeys: String, CodingKey { case group = "Group" }
}

public struct Definition: Codable {
    public let id: String?
    public let value: String
    public let examples: [Example]
    public let notes: [Note]
    public let media: [MediaRef]
}

public struct Example: Codable {
    public let value: String
    public let translations: [Translation]
    public let pron: [Pron]
    public let targets: [TargetOccurrence]
    public let media: [MediaRef]
}

public struct Note: Codable {
    public let id: String?
    public let value: String
    public let examples: [Example]
}

public struct DefGroup: Codable {
    public let id: String?
    public let description: String?
    public let definitions: [DefItem]
}

public struct Translation: Codable {
    public let lang: String?
    public let value: String
    public let pron: [Pron]
}

public struct Pron: Codable {
    public let lang: String?
    public let accent: String?
    public let kind: String?
    public let value: String
    public let media: [MediaRef]
}

public struct Form: Codable {
    public let kind: String?
    public let term: String
    public let tags: [String]
    public let feats: String?
    public let pron: [Pron]
}

public struct TargetOccurrence: Codable {
    public let spans: [TextSpan]
}

public struct TextSpan: Codable {
    public let offset: UInt64
    public let length: UInt64
}

// Helper for dynamic JSON inspection
struct AnyCodable: Codable {
    let value: Any
    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if let v = try? container.decode(String.self) { value = v }
        else if let v = try? container.decode(Int.self) { value = v }
        else if let v = try? container.decode(Bool.self) { value = v }
        else if let v = try? container.decode([AnyCodable].self) { value = v }
        else if let v = try? container.decode([String: AnyCodable].self) { value = v }
        else { value = NSNull() }
    }
    func encode(to encoder: Encoder) throws {}
}
