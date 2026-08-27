import SwiftUI
#if os(macOS)
import AppKit
#endif

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
        .alert("Error", isPresented: .init(
            get: { store.errorMessage != nil },
            set: { if !$0 { store.errorMessage = nil } }
        )) {
            Button("OK") { store.errorMessage = nil }
        } message: {
            Text(store.errorMessage ?? "")
        }
    }

    private var emptyState: some View {
        VStack(spacing: 20) {
            Image(systemName: "book.closed")
                .font(.system(size: 64))
                .foregroundStyle(.secondary)
            Text("No dictionary loaded")
                .font(.title2)
                .foregroundStyle(.secondary)
            Button("Open Dictionary…") {
                store.shouldShowOpenPanel = true
            }
            .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
    private var dictionaryView: some View {
        NavigationSplitView {
            // Sidebar: dictionary list + word list
            VStack(spacing: 0) {
                // Dictionary list
                DictionaryListView()

                Divider()

                // Search field
                HStack {
                    Image(systemName: "magnifyingglass")
                        .foregroundStyle(.secondary)
                    TextField("Search words…", text: $store.searchText)
                        .textFieldStyle(.plain)
                    if !store.searchText.isEmpty {
                        Button {
                            store.searchText = ""
                        } label: {
                            Image(systemName: "xmark.circle.fill")
                                .foregroundStyle(.secondary)
                        }
                        .buttonStyle(.borderless)
                    }
                }
                .padding(8)
                #if os(macOS)
                .background(Color(nsColor: .controlBackgroundColor))
                #else
                .background(Color(.systemBackground))
                #endif

                // Search scope picker
                if !store.searchText.isEmpty {
                    Picker("Scope", selection: $store.searchScope) {
                        ForEach(SearchScope.allCases, id: \.self) { scope in
                            Text(scope.rawValue).tag(scope)
                        }
                    }
                    .pickerStyle(.segmented)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                }

                // Word list
                if store.searchScope == .all && !store.searchText.isEmpty {
                    CrossDictSearchView()
                } else {
                    List(store.filteredHeadwords, id: \.self, selection: Binding(
                        get: { store.currentEntry?.headword },
                        set: { newSelection in
                            if let hw = newSelection {
                                store.lookup(hw)
                            }
                        }
                    )) { headword in
                        Text(headword)
                            .tag(headword)
                    }
                    .listStyle(.plain)
                }
            }
            #if os(macOS)
            .navigationSplitViewColumnWidth(min: 220, ideal: 300, max: 500)
            #endif
        } detail: {
            // Detail: entry view
            if let entry = store.currentEntry {
                EntryDetailView(entry: entry)
            } else {
                Text("Select a word")
                    .font(.title3)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .toolbar {
            #if os(macOS)
            ToolbarItem(placement: .navigation) {
                Button {
                    NSApp.keyWindow?.firstResponder?.tryToPerform(
                        #selector(NSSplitViewController.toggleSidebar(_:)),
                        with: nil
                    )
                } label: {
                    Label("Toggle Sidebar", systemImage: "sidebar.left")
                }
            }
            #endif
            ToolbarItem(placement: .primaryAction) {
                Button {
                    store.shouldShowOpenPanel = true
                } label: {
                    Label("Open", systemImage: "plus.folder")
                }
            }
        }
    }
}

// MARK: - Dictionary List

struct DictionaryListView: View {
    @EnvironmentObject var store: DictionaryStore

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Dictionaries")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding(.leading, 12)
                Spacer()
                Button {
                    store.shouldShowOpenPanel = true
                } label: {
                    Image(systemName: "plus")
                }
                .buttonStyle(.borderless)
                .padding(.trailing, 8)
            }
            .padding(.top, 8)
            .padding(.bottom, 4)

            List(store.dictionaries, id: \.id, selection: Binding(
                get: { store.selectedDictId },
                set: { newId in
                    if let id = newId {
                        store.selectDict(id)
                    }
                }
            )) { dict in
                DictRow(dict: dict, coverData: store.coverImages[dict.id])
                    .tag(dict.id)
                    .contextMenu {
                        Button("Close", role: .destructive) {
                            store.closeDict(dict.id)
                        }
                    }
            }
            .listStyle(.plain)
            .frame(maxHeight: 180)
        }
    }
}

struct DictRow: View {
    let dict: OpenDictionary
    let coverData: Data?

    var body: some View {
        HStack(spacing: 10) {
            // Cover thumbnail
            if let data = coverData {
                #if os(macOS)
                if let nsImage = NSImage(data: data) {
                    Image(nsImage: nsImage)
                        .resizable()
                        .scaledToFill()
                        .frame(width: 32, height: 44)
                        .clipShape(RoundedRectangle(cornerRadius: 4))
                } else {
                    coverPlaceholder
                }
                #else
                if let uiImage = UIImage(data: data) {
                    Image(uiImage: uiImage)
                        .resizable()
                        .scaledToFill()
                        .frame(width: 32, height: 44)
                        .clipShape(RoundedRectangle(cornerRadius: 4))
                } else {
                    coverPlaceholder
                }
                #endif
            } else {
                coverPlaceholder
            }

            VStack(alignment: .leading, spacing: 2) {
                Text(dict.name)
                    .font(.system(size: 13, weight: .medium))
                    .lineLimit(1)
                HStack(spacing: 4) {
                    Text("\(dict.entryCount) entries")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                    if !dict.sourceLang.isEmpty {
                        Text(dict.sourceLang)
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    if !dict.targetLangs.isEmpty {
                        Text("→ \(dict.targetLangs.joined(separator: ","))")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                }
            }
        }
        .padding(.vertical, 2)
    }

    private var coverPlaceholder: some View {
        Image(systemName: "book.closed")
            .font(.system(size: 20))
            .foregroundStyle(.secondary)
            .frame(width: 32, height: 44)
            .background(Color.secondary.opacity(0.1))
            .clipShape(RoundedRectangle(cornerRadius: 4))
    }
}

// MARK: - Cross-Dictionary Search

struct CrossDictSearchView: View {
    @EnvironmentObject var store: DictionaryStore

    var body: some View {
        let results = store.searchAll(store.searchText)
        List(results, id: \.0.id) { item in
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
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }
            .buttonStyle(.plain)
        }
        .listStyle(.plain)
    }
}
