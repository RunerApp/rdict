import SwiftUI
#if os(macOS)
import AppKit
#else
import UIKit
#endif
import AVFoundation
import Rdict

// MARK: - Island Design Tokens

enum DictTheme {
    static func posColor(_ pos: String?) -> Color {
        guard let pos = pos?.uppercased() else { return .gray }
        switch pos {
        case "NOUN":  return Color(red: 0.25, green: 0.55, blue: 0.85)
        case "VERB":  return Color(red: 0.80, green: 0.35, blue: 0.30)
        case "ADJ":   return Color(red: 0.30, green: 0.65, blue: 0.45)
        case "ADV":   return Color(red: 0.75, green: 0.50, blue: 0.20)
        case "PRON":  return Color(red: 0.60, green: 0.40, blue: 0.75)
        case "DET":   return Color(red: 0.55, green: 0.55, blue: 0.55)
        case "ADP":   return Color(red: 0.55, green: 0.55, blue: 0.55)
        case "NUM":   return Color(red: 0.65, green: 0.45, blue: 0.30)
        case "CCONJ": return Color(red: 0.50, green: 0.50, blue: 0.60)
        case "SCONJ": return Color(red: 0.50, green: 0.50, blue: 0.60)
        case "INTJ":  return Color(red: 0.85, green: 0.55, blue: 0.15)
        case "PROPN": return Color(red: 0.25, green: 0.55, blue: 0.85)
        default:      return .gray
        }
    }

    static func posLabel(_ pos: String?) -> String {
        guard let pos = pos?.uppercased() else { return "" }
        let map: [String: String] = [
            "NOUN": "n", "VERB": "v", "ADJ": "adj", "ADV": "adv",
            "PRON": "pron", "DET": "det", "ADP": "prep", "NUM": "num",
            "CCONJ": "conj", "SCONJ": "conj", "INTJ": "intj",
            "PROPN": "n", "AUX": "aux", "PART": "part", "SYM": "sym",
            "PUNCT": "punct", "X": "x",
        ]
        return map[pos] ?? pos.lowercased()
    }

    static func langLabel(_ lang: String?) -> String {
        guard let lang = lang else { return "" }
        switch lang {
        case "zh-Hans": return "中"
        case "es-ES":   return "Es"
        case "en":      return "En"
        case "ja":      return "日"
        default:        return lang
        }
    }

    // Island palette: adaptive surfaces, no hard borders.
    #if os(macOS)
    static let pageBackground = Color(nsColor: .windowBackgroundColor)
    #else
    static let pageBackground = Color(.systemGroupedBackground)
    #endif

    /// The elevated "island" card surface.
    static var cardBackground: Color {
        #if os(macOS)
        Color(nsColor: .controlBackgroundColor)
        #else
        Color(.secondarySystemGroupedBackground)
        #endif
    }

    /// Subtle inner fill for chips and nested elements on an island.
    static let chipBackground = Color.secondary.opacity(0.10)

    // Island shadow: soft, layered, low-opacity.
    static let islandShadow: (color: Color, radius: CGFloat, y: CGFloat) = (
        color: Color.black.opacity(0.10), radius: 14, y: 5
    )
}

/// Card modifier: the signature "island" look — big radius, soft shadow,
/// no border stroke.
struct IslandCard: ViewModifier {
    var padding: CGFloat = 18

    func body(content: Content) -> some View {
        content
            .padding(padding)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(DictTheme.cardBackground)
            .clipShape(RoundedRectangle(cornerRadius: 18, style: .continuous))
            .shadow(
                color: DictTheme.islandShadow.color,
                radius: DictTheme.islandShadow.radius,
                x: 0,
                y: DictTheme.islandShadow.y
            )
    }
}

extension View {
    func islandCard(padding: CGFloat = 18) -> some View {
        modifier(IslandCard(padding: padding))
    }
}

// MARK: - Entry Tag Chip

/// A parsed `category:value` tag (rdict-spec §3.5). Only exam levels are
/// user-visible; other categories parse but stay hidden.
struct EntryTag: Equatable {
    let category: String
    let value: String

    /// "exam:JLPT-N3" → (category "exam", value "JLPT-N3"). Returns nil
    /// for tags without the `category:value` shape.
    static func parse(_ raw: String) -> EntryTag? {
        guard let idx = raw.firstIndex(of: ":") else { return nil }
        return EntryTag(
            category: String(raw[raw.startIndex..<idx]),
            value: String(raw[raw.index(after: idx)...])
        )
    }

    /// Whether the tag renders in the header. Exam levels only.
    var isVisible: Bool { category == "exam" }

    /// Short label: "JLPT-N3" → "N3" when the exam has a level suffix,
    /// otherwise the raw value.
    var label: String {
        if let dash = value.lastIndex(of: "-") {
            return String(value[value.index(after: dash)...])
        }
        return value
    }

    /// Which exam this level belongs to ("JLPT-N3" → "JLPT"), for a11y.
    var examName: String {
        if let dash = value.lastIndex(of: "-") {
            return String(value[value.startIndex..<dash])
        }
        return value
    }
}

/// Difficulty color per exam level (green = beginner, red = advanced).
struct EntryTagChip: View {
    let tag: EntryTag

    private var levelColor: Color {
        switch tag.label {
        case "N5", "A1": return Color(red: 0.30, green: 0.65, blue: 0.40)
        case "N4", "A2": return Color(red: 0.45, green: 0.65, blue: 0.30)
        case "N3", "B1": return Color(red: 0.75, green: 0.60, blue: 0.20)
        case "N2", "B2": return Color(red: 0.85, green: 0.45, blue: 0.20)
        case "N1", "C1", "C2": return Color(red: 0.80, green: 0.25, blue: 0.25)
        default: return .gray
        }
    }

    var body: some View {
        Text(tag.label)
            .font(.system(size: 11, weight: .semibold, design: .rounded))
            .foregroundStyle(.white)
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(levelColor)
            .clipShape(Capsule())
            .help("\(tag.examName) level \(tag.label)")
    }
}

// MARK: - Entry Detail View

struct EntryDetailView: View {
    let entry: DictEntry
    @EnvironmentObject var store: DictionaryStore
    @State private var audioPlayer: AVAudioPlayer?

    private var contentMaxWidth: CGFloat { 720 }

    /// Languages that need a visible label on translations. With a single
    /// target language the label is redundant (the whole pack is one pair).
    private var labeledTranslationLangs: Set<String> {
        let langs = entry.etymologies.flatMap { ety in
            ety.senses.flatMap { sense in
                sense.translations.compactMap(\.lang)
            }
        }
        let unique = Set(langs)
        return unique.count > 1 ? unique : []
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                headerCard

                if let see = entry.see {
                    redirectCard(see)
                }

                if !entry.morphology.isEmpty {
                    morphologyCard
                }

                ForEach(Array(entry.etymologies.enumerated()), id: \.offset) { _, ety in
                    if let root = ety.root, !root.isEmpty {
                        etymologyRoot(root)
                    }
                    ForEach(Array(ety.senses.enumerated()), id: \.offset) { idx, sense in
                        SenseCard(
                            sense: sense,
                            index: idx + 1,
                            labeledLangs: labeledTranslationLangs
                        )
                    }
                }

                if !entry.relations.isEmpty {
                    relationsCard
                }
            }
            .padding(24)
            .frame(maxWidth: contentMaxWidth, alignment: .leading)
            .frame(maxWidth: .infinity)
        }
        .background(DictTheme.pageBackground)
        .textSelection(.enabled)
    }

    // MARK: Header Card

    private var headerCard: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline, spacing: 14) {
                Text(entry.headword)
                    .font(.system(size: 36, weight: .bold, design: .serif))
                    .foregroundStyle(.primary)
                Spacer()
                if !displayTags.isEmpty {
                    HStack(spacing: 6) {
                        ForEach(Array(displayTags.enumerated()), id: \.offset) { _, tag in
                            EntryTagChip(tag: tag)
                        }
                    }
                }
            }

            ForEach(Array(entry.pron.enumerated()), id: \.offset) { _, pron in
                HStack(spacing: 6) {
                    Image(systemName: "speaker.wave.2")
                        .font(.system(size: 10))
                        .foregroundStyle(.tertiary)
                    Text(pron.value)
                        .font(.system(size: 16, design: .serif))
                        .foregroundStyle(.secondary)
                }
            }
        }
        .islandCard(padding: 22)
    }

    /// Tags worth showing in the header: exam levels rendered as chips.
    /// Other categories (freq:, etc.) stay hidden — too technical.
    private var displayTags: [EntryTag] {
        entry.tags.compactMap { EntryTag.parse($0) }.filter(\.isVisible)
    }

    // MARK: Etymology Root

    private func etymologyRoot(_ root: String) -> some View {
        HStack(spacing: 6) {
            Image(systemName: "tree")
                .font(.system(size: 10))
                .foregroundStyle(.tertiary)
            Text(root)
                .font(.system(size: 12, design: .serif))
                .italic()
                .foregroundStyle(.tertiary)
        }
        .padding(.horizontal, 6)
    }

    // MARK: Redirect Card

    private func redirectCard(_ see: String) -> some View {
        Button {
            store.lookup(see)
        } label: {
            HStack(spacing: 10) {
                Image(systemName: "arrow.right.circle.fill")
                    .font(.system(size: 18))
                    .foregroundStyle(Color.secondary)
                Text(see)
                    .font(.system(size: 20, weight: .medium, design: .serif))
                    .foregroundStyle(.secondary)
                Spacer()
            }
            .padding(16)
            .background(DictTheme.chipBackground)
            .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
        }
        .buttonStyle(.plain)
    }

    // MARK: Morphology Card

    private var morphologyCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            sectionLabel(L10n.wordStructure)

            HStack(spacing: 4) {
                ForEach(Array(entry.morphology.enumerated()), id: \.offset) { idx, morph in
                    if idx > 0 {
                        Image(systemName: "chevron.right")
                            .font(.system(size: 8, weight: .bold))
                            .foregroundStyle(.tertiary)
                    }
                    VStack(spacing: 2) {
                        if let kind = morph.kind {
                            Text(L10n.morphemeKind(kind))
                                .font(.system(size: 9, weight: .semibold))
                                .textCase(.uppercase)
                                .foregroundStyle(.tertiary)
                        }
                        Text(morph.term)
                            .font(.system(size: 15, weight: .medium, design: .serif))
                    }
                    .padding(.horizontal, 10)
                    .padding(.vertical, 6)
                    .background(DictTheme.chipBackground)
                    .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
                }
            }
        }
        .islandCard(padding: 16)
    }

    // MARK: Relations Card

    private var relationsCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            sectionLabel(L10n.related)

            FlowLayout(spacing: 8) {
                ForEach(Array(entry.relations.enumerated()), id: \.offset) { _, rel in
                    Button {
                        store.lookup(rel.target)
                    } label: {
                        VStack(spacing: 2) {
                            Text(L10n.relationType(rel.type))
                                .font(.system(size: 8, weight: .semibold))
                                .textCase(.uppercase)
                                .foregroundStyle(.tertiary)
                            Text(rel.target)
                                .font(.system(size: 13, design: .serif))
                                .foregroundStyle(.primary)
                        }
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(DictTheme.chipBackground)
                        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .islandCard(padding: 16)
    }

    private func sectionLabel(_ text: String) -> some View {
        Text(text)
            .font(.system(size: 11, weight: .semibold))
            .textCase(.uppercase)
            .foregroundStyle(.tertiary)
    }
}

// MARK: - Sense Card

struct SenseCard: View {
    let sense: Sense
    let index: Int
    var labeledLangs: Set<String> = []

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Sense body. Layout:
            //   ①  n          ← circular badge + POS on their own row
            //   Abbreviation of anatomy.   ← every content line at the
            //   解剖学的缩写                 same x (own-row leader, so no
            //   中 磨蚀性的                  hanging-indent gymnastics)
            VStack(alignment: .leading, spacing: 8) {
                // Leader row: circular number badge + POS.
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    ZStack {
                        Circle()
                            .fill(DictTheme.posColor(sense.pos).opacity(0.15))
                        Text("\(index)")
                            .font(.system(size: 10, weight: .semibold, design: .serif))
                            .foregroundStyle(DictTheme.posColor(sense.pos))
                            .minimumScaleFactor(0.6)
                    }
                    .frame(width: 20, height: 20)
                    .offset(y: -2)

                    if let pos = sense.pos {
                        Text(DictTheme.posLabel(pos))
                            .font(.system(size: 13, weight: .medium, design: .serif))
                            .italic()
                            .foregroundStyle(DictTheme.posColor(pos))
                    }
                }

                // Content column: every line — definitions and
                // translations — starts at the same x.
                VStack(alignment: .leading, spacing: 6) {
                    ForEach(Array(sense.definitions.enumerated()), id: \.offset) { _, def in
                        if defHasContent(def) {
                            DefItemView(def: def)
                        }
                    }

                    ForEach(Array(sense.translations.enumerated()), id: \.offset) { _, tr in
                        HStack(alignment: .top, spacing: 6) {
                            if labeledLangs.contains(tr.lang ?? "") {
                                Text(DictTheme.langLabel(tr.lang))
                                    .font(.system(size: 9, weight: .bold))
                                    .foregroundStyle(.tertiary)
                                    .frame(width: 24, alignment: .leading)
                                    .padding(.top, 4)
                            }
                            Text(tr.value)
                                .font(.system(size: 15, weight: .medium, design: .serif))
                                .foregroundStyle(.primary)
                        }
                    }
                }
            }

            // Examples — standalone section with title
            let allExamples = collectExamples(from: sense.definitions)
            if !allExamples.isEmpty {
                VStack(alignment: .leading, spacing: 8) {
                    Text(L10n.examples)
                        .font(.system(size: 10, weight: .semibold))
                        .textCase(.uppercase)
                        .foregroundStyle(.tertiary)
                    ForEach(Array(allExamples.enumerated()), id: \.offset) { _, ex in
                        ExampleView(
                            example: ex,
                            labeledLangs: exampleLangs(allExamples)
                        )
                    }
                }
            }
        }
        .islandCard(padding: 18)
    }

    private func collectExamples(from defs: [DefItem]) -> [Example] {
        var result: [Example] = []
        for item in defs {
            switch item {
            case .definition(let d):
                result.append(contentsOf: d.examples)
            case .group(let g):
                for sub in g.definitions {
                    result.append(contentsOf: collectExamples(from: [sub]))
                }
            }
        }
        return result
    }

    /// Whether a def item renders anything visible in the def column
    /// itself (text, notes, media). Examples are rendered in a separate
    /// section, so a def shell holding only examples is "empty" here —
    /// drop-mode packs keep such shells and must not claim a line.
    private func defHasContent(_ item: DefItem) -> Bool {
        switch item {
        case .definition(let d):
            return !d.value.isEmpty || !d.notes.isEmpty || !d.media.isEmpty
        case .group(let g):
            return g.definitions.contains { defHasContent($0) }
        }
    }

    /// Languages needing labels across this sense's example translations.
    private func exampleLangs(_ examples: [Example]) -> Set<String> {
        let langs = Set(
            examples.flatMap { ex in ex.translations.compactMap(\.lang) }
        )
        return langs.count > 1 ? langs : []
    }
}

// MARK: - Def Item

struct DefItemView: View {
    let def: DefItem

    var body: some View {
        switch def {
        case .definition(let d):
            DefinitionView(definition: d)
        case .group(let g):
            DefGroupView(group: g)
        }
    }
}

// MARK: - Definition

struct DefinitionView: View {
    let definition: Definition
    @EnvironmentObject var store: DictionaryStore
    @State private var audioPlayer: AVAudioPlayer?

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            // Definition text only — no inline examples. Bilingual packs
            // compile with an empty definition value (source-lang text
            // dropped at compile time); render nothing for empty text.
            if !definition.value.isEmpty {
                // No bullet dot here — the sense number in the leader
                // column is the list marker (dictionary style). Styled
                // identically to translation lines: the source-language
                // definition is just "one more language" of the entry.
                Text(definition.value)
                    .font(.system(size: 15, weight: .medium, design: .serif))
                    .foregroundStyle(.primary)
            }

            ForEach(Array(definition.notes.enumerated()), id: \.offset) { _, note in
                NoteView(note: note)
            }

            if !definition.media.isEmpty {
                FlowLayout(spacing: 8) {
                    ForEach(Array(definition.media.enumerated()), id: \.offset) { _, mref in
                        MediaView(mediaRef: mref, store: store, audioPlayer: $audioPlayer)
                    }
                }
            }
        }
    }
}

// MARK: - Example — standalone, not collapsed

struct ExampleView: View {
    let example: Example
    var labeledLangs: Set<String> = []

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            if example.targets.isEmpty {
                Text(example.value)
                    .font(.system(size: 14, design: .serif))
                    .foregroundStyle(.secondary)
            } else {
                highlightedExampleText
            }

            ForEach(Array(example.translations.enumerated()), id: \.offset) { _, tr in
                HStack(alignment: .top, spacing: 6) {
                    if labeledLangs.contains(tr.lang ?? "") {
                        Text(DictTheme.langLabel(tr.lang))
                            .font(.system(size: 9, weight: .bold))
                            .foregroundStyle(.tertiary)
                            .frame(width: 24, alignment: .leading)
                            .padding(.top, 3)
                    }
                    Text(tr.value)
                        .font(.system(size: 13, design: .serif))
                        .foregroundStyle(.tertiary)
                }
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(DictTheme.pageBackground)
        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
    }

    private var highlightedExampleText: Text {
        let nsAttr = NSMutableAttributedString(string: example.value)
        let nsString = NSString(string: example.value)
        let utf8View = String(example.value).utf8
        for occ in example.targets {
            for span in occ.spans {
                let utf8Start = Int(span.offset)
                let utf8End = Int(span.offset + span.length)
                guard utf8Start <= utf8End,
                      utf8End <= utf8View.count
                else { continue }
                let start16 = utf8ToUTF16(utf8View, byteOffset: utf8Start)
                let end16 = utf8ToUTF16(utf8View, byteOffset: utf8End)
                let range = NSRange(location: start16, length: end16 - start16)
                guard range.location != NSNotFound,
                      nsString.length >= range.location + range.length
                else { continue }
                #if os(macOS)
                nsAttr.addAttribute(
                    .foregroundColor,
                    value: NSColor.controlAccentColor,
                    range: range
                )
                nsAttr.addAttribute(
                    .font,
                    value: NSFont.boldSystemFont(ofSize: NSFont.systemFontSize(for: .small)),
                    range: range
                )
                #else
                nsAttr.addAttribute(
                    .foregroundColor,
                    value: UIColor.tintColor,
                    range: range
                )
                nsAttr.addAttribute(
                    .font,
                    value: UIFont.boldSystemFont(ofSize: UIFont.smallSystemFontSize),
                    range: range
                )
                #endif
            }
        }
        return Text(AttributedString(nsAttr))
            .font(.system(.callout, design: .serif))
            .foregroundStyle(.secondary)
    }
}

// MARK: - Note

struct NoteView: View {
    let note: Note

    var body: some View {
        HStack(alignment: .top, spacing: 4) {
            Image(systemName: "note.text")
                .font(.system(size: 10))
                .foregroundStyle(.tertiary)
            Text(note.value)
                .font(.system(size: 12))
                .foregroundStyle(.tertiary)
        }
    }
}

// MARK: - DefGroup

struct DefGroupView: View {
    let group: DefGroup

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let desc = group.description {
                Text(desc)
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(.secondary)
            }
            ForEach(Array(group.definitions.enumerated()), id: \.offset) { _, def in
                DefItemView(def: def)
            }
        }
        .padding(12)
        .background(DictTheme.pageBackground)
        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
    }
}

// MARK: - UTF-8 to UTF-16 conversion

private func utf8ToUTF16(_ utf8: String.UTF8View, byteOffset: Int) -> Int {
    // Walk scalar boundaries, not raw bytes: UTF8View iterates every
    // byte, so counting each byte as a character start over-counts
    // multi-byte sequences (Japanese text highlighted the wrong chars).
    var utf16Count = 0
    var byteCount = 0
    var iter = utf8.makeIterator()
    while let first = iter.next() {
        if byteCount >= byteOffset {
            return utf16Count
        }
        let len: Int
        let utf16Len: Int
        if first < 0x80 {
            len = 1; utf16Len = 1
        } else if first < 0xE0 {
            len = 2; utf16Len = 1
        } else if first < 0xF0 {
            len = 3; utf16Len = 1
        } else {
            len = 4; utf16Len = 2
        }
        // Skip the continuation bytes of this scalar.
        for _ in 1..<len {
            if iter.next() == nil {
                return utf16Count
            }
        }
        byteCount += len
        utf16Count += utf16Len
    }
    return utf16Count
}

// MARK: - Media Views

@MainActor
struct MediaView: View {
    let mediaRef: MediaRef
    let store: DictionaryStore
    @Binding var audioPlayer: AVAudioPlayer?
    @State private var isPlaying = false

    var body: some View {
        let kindLower = mediaRef.kind.lowercased()
        switch kindLower {
        case "audio":
            audioView
        case "image":
            imageAttachmentView
        case "video":
            Text("Video media not supported")
                .font(.caption)
                .foregroundStyle(.secondary)
        default:
            Text("Unknown media: \(mediaRef.kind)")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    private var loadFailedText: some View {
        Text("Failed to load image")
            .font(.caption)
            .foregroundStyle(.secondary)
    }

    private var audioView: some View {
        HStack(spacing: 8) {
            Button {
                playAudio()
            } label: {
                Image(systemName: isPlaying ? "speaker.wave.2.fill" : "speaker.wave.2")
                    .font(.title3)
            }
            .buttonStyle(.plain)

            if let desc = mediaRef.description {
                Text(desc)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer()
        }
        .padding(.vertical, 4)
    }

    private var imageAttachmentView: some View {
        VStack(alignment: .leading, spacing: 4) {
            if let data = store.currentBridge?.readMedia(kind: mediaRef.kind, hash: hashHex())
            {
                #if os(macOS)
                if let nsImage = NSImage(data: data) {
                    Image(nsImage: nsImage)
                        .resizable()
                        .scaledToFit()
                        .frame(maxWidth: 400, maxHeight: 300)
                        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
                } else {
                    loadFailedText
                }
                #else
                if let uiImage = UIImage(data: data) {
                    Image(uiImage: uiImage)
                        .resizable()
                        .scaledToFit()
                        .frame(maxWidth: 400, maxHeight: 300)
                        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
                } else {
                    loadFailedText
                }
                #endif
            } else {
                loadFailedText
            }

            if let desc = mediaRef.description {
                Text(desc)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else if let alt = mediaRef.alt {
                Text(alt)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private func hashHex() -> String {
        mediaRef.hash
    }

    private func playAudio() {
        guard let data = store.currentBridge?.readMedia(kind: mediaRef.kind, hash: hashHex())
        else { return }

        #if os(iOS)
        try? AVAudioSession.sharedInstance().setCategory(.playback, mode: .default)
        try? AVAudioSession.sharedInstance().setActive(true)
        #endif

        do {
            audioPlayer = try AVAudioPlayer(data: data)
            audioPlayer?.play()
            isPlaying = true
            DispatchQueue.main.asyncAfter(deadline: .now() + (audioPlayer?.duration ?? 1)) {
                isPlaying = false
            }
        } catch {
            print("Audio playback error: \(error)")
        }
    }
}

// MARK: - Flow Layout

struct FlowLayout: Layout {
    var spacing: CGFloat = 8

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let maxWidth = proposal.width ?? .infinity
        var height: CGFloat = 0
        var width: CGFloat = 0
        var lineWidth: CGFloat = 0
        var lineHeight: CGFloat = 0

        for subview in subviews {
            let size = subview.sizeThatFits(.unspecified)
            if lineWidth + size.width > maxWidth {
                width = max(width, lineWidth)
                height += lineHeight + spacing
                lineWidth = size.width + spacing
                lineHeight = size.height
            } else {
                lineWidth += size.width + spacing
                lineHeight = max(lineHeight, size.height)
            }
        }
        width = max(width, lineWidth)
        height += lineHeight
        return CGSize(width: width, height: height)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX
        var y = bounds.minY
        var lineHeight: CGFloat = 0

        for subview in subviews {
            let size = subview.sizeThatFits(.unspecified)
            if x + size.width > bounds.maxX {
                x = bounds.minX
                y += lineHeight + spacing
                lineHeight = 0
            }
            subview.place(at: CGPoint(x: x, y: y), proposal: .init(size))
            x += size.width + spacing
            lineHeight = max(lineHeight, size.height)
        }
    }
}
