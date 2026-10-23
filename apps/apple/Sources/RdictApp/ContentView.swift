import SwiftUI
#if os(macOS)
import AppKit
#endif
import Rdict

struct ContentView: View {
    @EnvironmentObject var store: DictionaryStore

    var body: some View {
        SwiftUI.Group {
            if store.isLoaded {
                dictionaryView
            } else {
                emptyState
            }
        }
        .alert(L10n.error, isPresented: .init(
            get: { store.errorMessage != nil },
            set: { if !$0 { store.errorMessage = nil } }
        )) {
            Button("OK") { store.errorMessage = nil }
        } message: {
            Text(store.errorMessage ?? "")
        }
    }

    // MARK: Empty State — island hero card

    private var emptyState: some View {
        VStack(spacing: 0) {
            VStack(spacing: 18) {
                Image(systemName: "character.book.closed")
                    .font(.system(size: 52, weight: .light))
                    .foregroundStyle(.tertiary)
                    .frame(width: 96, height: 96)
                    .background(DictTheme.chipBackground)
                    .clipShape(RoundedRectangle(cornerRadius: 26, style: .continuous))
                VStack(spacing: 8) {
                    Text(L10n.noDict)
                        .font(.system(size: 20, weight: .semibold))
                        .foregroundStyle(.primary)
                    Text(L10n.noDictHint)
                        .font(.system(size: 13))
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.center)
                }
                Button {
                    store.shouldShowOpenPanel = true
                } label: {
                    Label(L10n.openDict, systemImage: "plus")
                        .padding(.horizontal, 8)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
            }
            .padding(40)
            .background(DictTheme.cardBackground)
            .clipShape(RoundedRectangle(cornerRadius: 24, style: .continuous))
            .shadow(
                color: DictTheme.islandShadow.color,
                radius: DictTheme.islandShadow.radius,
                x: 0,
                y: DictTheme.islandShadow.y
            )
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(DictTheme.pageBackground)
    }

    private var dictionaryView: some View {
        NavigationSplitView {
            sidebar
            #if os(macOS)
            .navigationSplitViewColumnWidth(min: 240, ideal: 300, max: 480)
            #endif
        } detail: {
            if let entry = store.currentEntry {
                TimedEntryDetailView(entry: entry)
            } else {
                Text(L10n.selectWord)
                    .font(.title3)
                    .foregroundStyle(.tertiary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button {
                    store.shouldShowOpenPanel = true
                } label: {
                    Label(L10n.open, systemImage: "plus.folder")
                }
            }
            ToolbarItem(placement: .primaryAction) {
                Picker(L10n.language, selection: Binding(
                    get: { L10n.lang },
                    set: { newLang in
                        L10n.setLang(newLang)
                        store.langChangeTrigger.toggle()
                    }
                )) {
                    ForEach(AppLang.allCases, id: \.self) { l in
                        Text(l.rawValue).tag(l)
                    }
                }
                .pickerStyle(.menu)
                .help(L10n.language)
            }
        }
    }

    // MARK: Sidebar

    private var sidebar: some View {
        VStack(spacing: 10) {
            // Dictionary shelf — cover cards
            if !store.dictionaries.isEmpty {
                DictionaryShelfView()
                    .padding(.horizontal, 10)
                    .padding(.top, 10)
            }

            // Search island
            searchBar
                .padding(.horizontal, 10)

            // Word list
            listArea
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(DictTheme.pageBackground)
    }

    @ViewBuilder
    private var listArea: some View {
        if store.searchScope == .all && !store.searchText.isEmpty {
            CrossDictSearchView()
        } else {
            wordList
        }
    }

    private var searchBar: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 12))
                .foregroundStyle(.tertiary)
            TextField(L10n.search, text: $store.searchText)
                .textFieldStyle(.plain)
                .font(.system(size: 13))
            if !store.searchText.isEmpty {
                Button {
                    store.searchText = ""
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .font(.system(size: 12))
                        .foregroundStyle(.tertiary)
                }
                .buttonStyle(.borderless)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(DictTheme.cardBackground)
        .clipShape(Capsule())
        .shadow(
            color: DictTheme.islandShadow.color,
            radius: 8,
            x: 0,
            y: 3
        )
    }

    private var wordList: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 2) {
                ForEach(Array(store.filteredHeadwords.enumerated()), id: \.element) { idx, headword in
                    WordRow(
                        headword: headword,
                        isSelected: headword == store.selectedHeadword,
                        index: idx
                    ) {
                        store.lookup(headword)
                    }
                }
                // Bottom sentinel: appears on screen when the user nears
                // the end of the loaded page — grow the list.
                if store.canLoadMore {
                    Color.clear
                        .frame(height: 1)
                        .onAppear { store.loadMoreHeadwords() }
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
        }
    }
}

// MARK: - Dictionary Shelf (cover cards)

struct DictionaryShelfView: View {
    @EnvironmentObject var store: DictionaryStore

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 10) {
                ForEach(store.dictionaries) { dict in
                    shelfItem(dict)
                }
                addCard
            }
            .padding(.vertical, 4)
        }
        .frame(height: 92)
    }

    /// "+" card at the end of the shelf — the natural place to add a
    /// dictionary (the toolbar button is easy to miss).
    private var addCard: some View {
        Button {
            store.shouldShowOpenPanel = true
        } label: {
            VStack(spacing: 6) {
                ZStack {
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .fill(DictTheme.chipBackground)
                        .frame(width: 52, height: 68)
                    Image(systemName: "plus")
                        .font(.system(size: 20, weight: .medium))
                        .foregroundStyle(.secondary)
                }
                Text(L10n.openDict)
                    .font(.system(size: 10))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .frame(width: 76)
            }
        }
        .buttonStyle(.plain)
        .help(L10n.openDict)
    }

    private func shelfItem(_ dict: OpenDictionary) -> some View {
        let card = DictCard(
            dict: dict,
            coverData: store.coverImages[dict.id],
            isSelected: dict.id == store.selectedDictId
        )
        return card
            .onTapGesture { store.selectDict(dict.id) }
            .contextMenu {
                Button(L10n.close, role: .destructive) {
                    store.closeDict(dict.id)
                }
            }
    }
}

struct DictCard: View {
    let dict: OpenDictionary
    let coverData: Data?
    var isSelected: Bool = false
    @State private var isHovered = false

    var body: some View {
        VStack(spacing: 6) {
            cover
            name
        }
        .onHover { isHovered = $0 }
    }

    private var name: some View {
        Text(dict.name)
            .font(.system(size: 10, weight: isSelected ? .semibold : .regular))
            .foregroundStyle(isSelected ? Color.primary : Color.secondary)
            .lineLimit(1)
            .frame(width: 76)
    }

    private var cover: some View {
        coverImage
            .frame(width: 52, height: 68)
            .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
            .shadow(
                color: Color.black.opacity(isHovered ? 0.22 : 0.13),
                radius: isHovered ? 6 : 3,
                x: 0,
                y: isHovered ? 4 : 2
            )
            .overlay(selectionRing)
            .scaleEffect(isHovered && !isSelected ? 1.04 : 1.0)
            .animation(.spring(response: 0.25, dampingFraction: 0.8), value: isHovered)
    }

    @ViewBuilder
    private var selectionRing: some View {
        if isSelected {
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .stroke(Color.accentColor, lineWidth: 2)
        }
    }

    @ViewBuilder
    private var coverImage: some View {
        if let data = coverData {
            #if os(macOS)
            if let nsImage = NSImage(data: data) {
                Image(nsImage: nsImage).resizable().scaledToFill()
            } else {
                coverPlaceholder
            }
            #else
            if let uiImage = UIImage(data: data) {
                Image(uiImage: uiImage).resizable().scaledToFill()
            } else {
                coverPlaceholder
            }
            #endif
        } else {
            coverPlaceholder
        }
    }

    private var coverPlaceholder: some View {
        Image(systemName: "book.closed")
            .font(.system(size: 18))
            .foregroundStyle(.tertiary)
            .frame(width: 52, height: 68)
            .background(DictTheme.chipBackground)
    }
}

// MARK: - Cross-Dictionary Search

struct CrossDictSearchView: View {
    @EnvironmentObject var store: DictionaryStore

    var body: some View {
        let results = store.searchAll(store.searchText)
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 2) {
                ForEach(results, id: \.0.id) { item in
                    let (dict, headword) = item
                    Button {
                        store.jumpTo(dictId: dict.id, headword: headword)
                        store.searchScope = .current
                    } label: {
                        HStack {
                            Text(headword)
                                .font(.system(size: 13))
                            Spacer()
                            Text(dict.name)
                                .font(.system(size: 10))
                                .foregroundStyle(.tertiary)
                        }
                        .padding(.horizontal, 10)
                        .padding(.vertical, 5)
                        .background(
                            Capsule().fill(Color.secondary.opacity(0.05))
                        )
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
        }
    }
}

// MARK: - Timed Entry Detail View

struct TimedEntryDetailView: View {
    let entry: DictEntry
    @EnvironmentObject var store: DictionaryStore

    var body: some View {
        VStack(spacing: 0) {
            EntryDetailView(entry: entry)
            HStack(spacing: 6) {
                Text("lookup \(String(format: "%.1f", store.lastLookupMs))ms")
                Text("·")
                Text("paint \(String(format: "%.0f", store.lastPaintMs))ms")
                Spacer()
            }
            .font(.system(size: 9, design: .monospaced))
            .foregroundStyle(.quaternary)
            .padding(.horizontal, 8)
            .padding(.vertical, 1)
        }
    }
}

// MARK: - Word Row

struct WordRow: View {
    let headword: String
    let isSelected: Bool
    let index: Int
    let onTap: () -> Void
    @State private var isHovered = false

    var body: some View {
        Text(headword)
            .font(.system(size: 13, weight: isSelected ? .medium : .regular))
            .foregroundStyle(.primary)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(rowBackground)
            .clipShape(Capsule())
            .contentShape(Rectangle())
            .onHover { isHovered = $0 }
            .onTapGesture(perform: onTap)
    }

    private var rowBackground: Color {
        if isSelected {
            return Color.accentColor.opacity(0.14)
        }
        if isHovered {
            return Color.secondary.opacity(0.08)
        }
        return Color.clear
    }
}
