import SwiftUI
#if os(macOS)
import AppKit
#else
import UIKit
#endif
import AVFoundation
import Rdict

struct EntryDetailView: View {
    let entry: DictEntry
    @EnvironmentObject var store: DictionaryStore
    @State private var audioPlayer: AVAudioPlayer?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                headerSection

                if !entry.media.isEmpty {
                    entryMediaSection
                }

                if !entry.tags.isEmpty {
                    tagSection
                }

                if let see = entry.see {
                    redirectSection(see)
                }

                if !entry.morphology.isEmpty {
                    morphologySection
                }

                if !entry.relations.isEmpty {
                    relationsSection
                }

                ForEach(Array(entry.etymologies.enumerated()), id: \.offset) { _, ety in
                    EtyView(ety: ety)
                }
            }
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var headerSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(entry.headword)
                .font(.system(size: 32, weight: .bold))

            ForEach(Array(entry.pron.enumerated()), id: \.offset) { _, pron in
                HStack(spacing: 8) {
                    if let lang = pron.lang {
                        Text(lang)
                            .font(.caption)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Color.accentColor.opacity(0.15))
                            .clipShape(RoundedRectangle(cornerRadius: 4))
                    }
                    if let accent = pron.accent {
                        Text(accent)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    Text(pron.value)
                        .font(.system(.title3, design: .serif))
                        .foregroundStyle(Color.accentColor)
                }
            }
        }
    }

    private var entryMediaSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(Array(entry.media.enumerated()), id: \.offset) { _, mref in
                MediaView(mediaRef: mref, store: store, audioPlayer: $audioPlayer)
            }
        }
    }

    private var relationsSection: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Related")
                .font(.caption)
                .foregroundStyle(.secondary)
            FlowLayout(spacing: 6) {
                ForEach(Array(entry.relations.enumerated()), id: \.offset) { _, rel in
                    Button {
                        store.lookup(rel.target)
                    } label: {
                        HStack(spacing: 4) {
                            Text(rel.type_ ?? "rel")
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                            Text(rel.target)
                                .font(.system(.body, design: .serif))
                        }
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(Color.accentColor.opacity(0.08))
                        .clipShape(RoundedRectangle(cornerRadius: 4))
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .padding(.vertical, 4)
    }

    private var tagSection: some View {
        FlowLayout(spacing: 6) {
            ForEach(entry.tags, id: \.self) { tag in
                Text(tag)
                    .font(.caption)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 3)
                    .background(Color.gray.opacity(0.15))
                    .clipShape(RoundedRectangle(cornerRadius: 4))
            }
        }
    }

    private func redirectSection(_ see: String) -> some View {
        Button {
            store.lookup(see)
        } label: {
            HStack {
                Image(systemName: "arrow.right.circle")
                Text("See: \(see)")
                    .font(.headline)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding()
            .background(Color.accentColor.opacity(0.08))
            .clipShape(RoundedRectangle(cornerRadius: 8))
        }
        .buttonStyle(.plain)
    }

    private var morphologySection: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Morphology")
                .font(.caption)
                .foregroundStyle(.secondary)
            FlowLayout(spacing: 6) {
                ForEach(Array(entry.morphology.enumerated()), id: \.offset) { _, morph in
                    HStack(spacing: 4) {
                        if let kind = morph.kind {
                            Text(kind)
                                .font(.caption2)
                                .padding(.horizontal, 4)
                                .padding(.vertical, 1)
                                .background(Color.accentColor.opacity(0.12))
                                .clipShape(RoundedRectangle(cornerRadius: 3))
                        }
                        Text(morph.term)
                            .font(.system(.body, design: .serif))
                    }
                }
            }
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Ety

struct EtyView: View {
    let ety: Ety

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let root = ety.root {
                HStack(spacing: 6) {
                    Image(systemName: "tree")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text(root)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .italic()
                }
            }

            ForEach(Array(ety.senses.enumerated()), id: \.offset) { _, sense in
                SenseView(sense: sense)
            }
        }
    }
}

// MARK: - Sense

struct SenseView: View {
    let sense: Sense

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 8) {
                if let pos = sense.pos {
                    Text(pos)
                        .font(.system(.subheadline, design: .serif))
                        .italic()
                        .foregroundStyle(Color.accentColor)
                }
                if let lemma = sense.lemma {
                    Text("lemma: \(lemma)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
            }

            if !sense.translations.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(Array(sense.translations.enumerated()), id: \.offset) { _, tr in
                        HStack(spacing: 6) {
                            if let lang = tr.lang {
                                Text(lang)
                                    .font(.caption2)
                                    .foregroundStyle(.secondary)
                            }
                            Text(tr.value)
                                .font(.system(.body, design: .serif))
                        }
                    }
                }
            }

            ForEach(Array(sense.definitions.enumerated()), id: \.offset) { _, def in
                DefItemView(def: def)
            }

            if !sense.forms.isEmpty {
                formsSection
            }
        }
        .padding(.vertical, 4)
    }

    private var formsSection: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Forms")
                .font(.caption)
                .foregroundStyle(.secondary)
            ForEach(Array(sense.forms.enumerated()), id: \.offset) { _, form in
                HStack(spacing: 8) {
                    if let kind = form.kind {
                        Text(kind)
                            .font(.caption)
                            .padding(.horizontal, 4)
                            .background(Color.gray.opacity(0.1))
                            .clipShape(RoundedRectangle(cornerRadius: 3))
                    }
                    Text(form.term)
                        .font(.system(.body, design: .serif))
                    if let feats = form.feats {
                        Text(feats)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    ForEach(Array(form.pron.enumerated()), id: \.offset) { _, pron in
                        Text(pron.value)
                            .font(.caption)
                            .foregroundStyle(Color.accentColor)
                    }
                }
            }
        }
    }
}

// MARK: - Def

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

struct DefinitionView: View {
    let definition: Definition
    @EnvironmentObject var store: DictionaryStore
    @State private var audioPlayer: AVAudioPlayer?

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .top, spacing: 8) {
                Text("•")
                    .foregroundStyle(.secondary)
                Text(definition.value)
                    .font(.body)
            }

            ForEach(Array(definition.examples.enumerated()), id: \.offset) { _, ex in
                ExampleView(example: ex)
                    .padding(.leading, 20)
            }

            ForEach(Array(definition.notes.enumerated()), id: \.offset) { _, note in
                NoteView(note: note)
                    .padding(.leading, 20)
            }

            if !definition.media.isEmpty {
                FlowLayout(spacing: 8) {
                    ForEach(Array(definition.media.enumerated()), id: \.offset) { _, mref in
                        MediaView(mediaRef: mref, store: store, audioPlayer: $audioPlayer)
                    }
                }
                .padding(.leading, 20)
            }
        }
    }
}

struct ExampleView: View {
    let example: Example

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            if example.targets.isEmpty {
                Text(example.value)
                    .font(.system(.callout, design: .serif))
                    .foregroundStyle(.secondary)
            } else {
                highlightedExampleText
            }

            ForEach(Array(example.translations.enumerated()), id: \.offset) { _, tr in
                HStack(spacing: 4) {
                    if let lang = tr.lang {
                        Text(lang)
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    Text(tr.value)
                        .font(.system(.callout, design: .serif))
                        .foregroundStyle(.secondary)
                }
            }
        }
        .padding(.vertical, 2)
    }

    private var highlightedExampleText: Text {
        let nsAttr = NSMutableAttributedString(string: example.value)
        let nsString = NSString(string: example.value)
        let utf8View = String(example.value).utf8
        for occ in example.targets {
            for span in occ.spans {
                // Rust gives UTF-8 byte offsets; convert to UTF-16 indices
                // for NSString/NSAttributedString.
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

struct NoteView: View {
    let note: Note

    var body: some View {
        HStack(alignment: .top, spacing: 4) {
            Image(systemName: "note.text")
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(note.value)
                .font(.caption)
                .foregroundStyle(.secondary)
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
                    .font(.subheadline)
                    .fontWeight(.medium)
            }
            ForEach(Array(group.definitions.enumerated()), id: \.offset) { _, def in
                DefItemView(def: def)
            }
        }
        .padding(.leading, 12)
        .overlay(
            RoundedRectangle(cornerRadius: 4)
                .stroke(Color.gray.opacity(0.2), lineWidth: 1)
        )
        .padding(.leading, 4)
    }
}

// MARK: - UTF-8 to UTF-16 conversion

/// Convert a UTF-8 byte offset to a UTF-16 (NSString) index.
private func utf8ToUTF16(_ utf8: String.UTF8View, byteOffset: Int) -> Int {
    var utf16Count = 0
    var byteCount = 0
    for byte in utf8 {
        if byteCount >= byteOffset {
            return utf16Count
        }
        if byte < 0x80 {
            // 1-byte char: 1 UTF-16 unit
            utf16Count += 1
            byteCount += 1
        } else if byte < 0xE0 {
            // 2-byte char: 1 UTF-16 unit
            utf16Count += 1
            byteCount += 2
        } else if byte < 0xF0 {
            // 3-byte char: 1 UTF-16 unit
            utf16Count += 1
            byteCount += 3
        } else {
            // 4-byte char: 2 UTF-16 units (surrogate pair)
            utf16Count += 2
            byteCount += 4
        }
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
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                } else {
                    loadFailedText
                }
                #else
                if let uiImage = UIImage(data: data) {
                    Image(uiImage: uiImage)
                        .resizable()
                        .scaledToFit()
                        .frame(maxWidth: 400, maxHeight: 300)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
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
