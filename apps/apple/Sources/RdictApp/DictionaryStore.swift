import Foundation
import SwiftUI
import Rdict

/// Represents one open dictionary.
struct OpenDictionary: Identifiable, Hashable {
    let id = UUID()
    let path: String
    let name: String
    let entryCount: Int
    let sourceLang: String
    let targetLangs: [String]
    let version: String

    func hash(into hasher: inout Hasher) {
        hasher.combine(id)
    }

    static func == (lhs: OpenDictionary, rhs: OpenDictionary) -> Bool {
        lhs.id == rhs.id
    }
}

@MainActor
final class DictionaryStore: ObservableObject {
    @Published var dictionaries: [OpenDictionary] = []
    @Published var selectedDictId: UUID?
    @Published var filteredHeadwords: [String] = []
    /// Current page size for the sidebar list; grows by `pageSize`
    /// when the user scrolls to the bottom (infinite scroll).
    @Published private(set) var listLimit = 200
    let pageSize = 200
    var canLoadMore: Bool { filteredHeadwords.count >= listLimit }
    @Published var currentEntry: DictEntry?
    @Published var selectedHeadword: String?
    @Published var lastLookupMs: Double = 0
    @Published var lastPaintMs: Double = 0
    @Published var langChangeTrigger: Bool = false
    @Published var errorMessage: String?
    @Published var shouldShowOpenPanel = false
    @Published var searchText = "" {
        didSet { applyFilter() }
    }
    @Published var searchScope: SearchScope = .current {
        didSet { applyFilter() }
    }

    private var bridges: [UUID: RdictDictionary] = [:]
    var currentBridge: RdictDictionary? {
        guard let id = selectedDictId else { return nil }
        return bridges[id]
    }

    /// Cached cover images keyed by dictionary id.
    @Published var coverImages: [UUID: Data] = [:]

    var isLoaded: Bool { !dictionaries.isEmpty }

    var currentDict: OpenDictionary? {
        guard let id = selectedDictId else { return nil }
        return dictionaries.first { $0.id == id }
    }

    /// Try to reopen dictionaries from the last session.
    func restoreLastSession() {
        let paths = UserDefaults.standard.stringArray(forKey: "rdict.paths") ?? []
        var restored: [OpenDictionary] = []
        var lastSelectedPath: String?
        for path in paths {
            guard FileManager.default.fileExists(atPath: path) else { continue }
            let bridge = RdictDictionary(path: path)
            guard bridge.isOpen else { continue }
            let info = bridge.manifest()
            let url = URL(fileURLWithPath: path)
            let dict = OpenDictionary(
                path: path,
                name: info?.name ?? url.deletingPathExtension().lastPathComponent,
                entryCount: info?.entryCount ?? 0,
                sourceLang: info?.sourceLang ?? "",
                targetLangs: info?.targetLangs ?? [],
                version: info?.version ?? ""
            )
            bridges[dict.id] = bridge
            coverImages[dict.id] = bridge.readCover()
            restored.append(dict)
            if path == UserDefaults.standard.string(forKey: "rdict.lastPath") {
                lastSelectedPath = path
            }
        }
        dictionaries = restored
        if let last = lastSelectedPath,
           let dict = restored.first(where: { $0.path == last })
        {
            selectDict(dict.id)
        } else {
            selectedDictId = restored.first?.id
            reloadHeadwords()
        }
        if dictionaries.isEmpty {
            // Try to open the bundled dictionary.
            if let bundled = bundledDictionaryPath(),
               FileManager.default.fileExists(atPath: bundled)
            {
                open(url: URL(fileURLWithPath: bundled))
            } else {
                shouldShowOpenPanel = true
            }
        }
    }

    /// Path to the dictionary file bundled in the app resources.
    private func bundledDictionaryPath() -> String? {
        Bundle.main.url(forResource: "eng-zho-ngsl", withExtension: "rdict")?.path
    }

    func open(url: URL) {
        let path = url.path
        // Skip if already open
        if dictionaries.contains(where: { $0.path == path }) {
            if let existing = dictionaries.first(where: { $0.path == path }) {
                selectDict(existing.id)
            }
            return
        }
        let bridge = RdictDictionary(path: path)
        guard bridge.isOpen else {
            errorMessage = "Failed to open: \(url.lastPathComponent)"
            return
        }
        let info = bridge.manifest()
        let dict = OpenDictionary(
            path: path,
            name: info?.name ?? url.deletingPathExtension().lastPathComponent,
            entryCount: info?.entryCount ?? 0,
            sourceLang: info?.sourceLang ?? "",
            targetLangs: info?.targetLangs ?? [],
            version: info?.version ?? ""
        )
        bridges[dict.id] = bridge
        coverImages[dict.id] = bridge.readCover()
        dictionaries.append(dict)
        persistPaths()
        selectDict(dict.id)
    }

    func selectDict(_ id: UUID) {
        selectedDictId = id
        UserDefaults.standard.set(
            dictionaries.first { $0.id == id }?.path,
            forKey: "rdict.lastPath"
        )
        reloadHeadwords()
    }

    func closeDict(_ id: UUID) {
        bridges[id]?.close()
        bridges.removeValue(forKey: id)
        coverImages.removeValue(forKey: id)
        dictionaries.removeAll { $0.id == id }
        if selectedDictId == id {
            selectedDictId = dictionaries.first?.id
            reloadHeadwords()
        }
        persistPaths()
    }

    private func persistPaths() {
        UserDefaults.standard.set(dictionaries.map(\.path), forKey: "rdict.paths")
        if let cur = currentDict {
            UserDefaults.standard.set(cur.path, forKey: "rdict.lastPath")
        } else {
            UserDefaults.standard.removeObject(forKey: "rdict.lastPath")
        }
    }

    private func reloadHeadwords() {
        guard let bridge = currentBridge else {
            filteredHeadwords = []
            currentEntry = nil
            selectedHeadword = nil
            return
        }
        // No full listHeadwords() (materializes 100k+ Strings per pack —
        // the sidebar only ever shows a prefix window anyway). Empty
        // search shows the first 200 headwords via the index.
        currentEntry = nil
        selectedHeadword = nil
        applyFilter()
        if let first = filteredHeadwords.first {
            lookup(first)
        }
    }

    func lookup(_ headword: String) {
        guard let bridge = currentBridge else { return }
        let clickTime = Date()
        selectedHeadword = headword
        let t0 = Date()
        do {
            currentEntry = try bridge.lookup(headword)
        } catch {
            errorMessage = error.localizedDescription
            currentEntry = nil
        }
        lastLookupMs = Date().timeIntervalSince(t0) * 1000
        // Measure after SwiftUI finishes the render pass triggered by the
        // currentEntry change above. Two runloop ticks: first rebuilds the
        // view tree, second flushes the display.
        DispatchQueue.main.async { [weak self] in
            DispatchQueue.main.async {
                self?.lastPaintMs = Date().timeIntervalSince(clickTime) * 1000
            }
        }
    }

    /// Cross-dictionary search: returns (dict, headword) pairs.
    func searchAll(_ needle: String) -> [(OpenDictionary, String)] {
        var results: [(OpenDictionary, String)] = []
        for dict in dictionaries {
            guard let bridge = bridges[dict.id] else { continue }
            let prefixMatches = bridge.prefix(needle, limit: 200)
            for hw in prefixMatches {
                results.append((dict, hw))
                if results.count >= 200 { return results }
            }
        }
        return results
    }

    /// Jump to a headword in a specific dictionary.
    func jumpTo(dictId: UUID, headword: String) {
        selectDict(dictId)
        lookup(headword)
    }

    /// Grow the sidebar list by one page (called when the user scrolls
    /// near the bottom). No-op when the last page was short (fewer
    /// results than requested = end of the list reached).
    func loadMoreHeadwords() {
        guard canLoadMore else { return }
        listLimit += pageSize
        applyFilter()
    }

    /// Reset pagination (new search text / dictionary switch).
    private func resetPagination() {
        listLimit = pageSize
    }

    private func applyFilter() {
        resetPaginationIfNeeded()
        if searchText.isEmpty {
            // First `listLimit` headwords from the index (empty search =
            // top of the list; no need to materialize the full word list).
            guard let id = selectedDictId, let bridge = bridges[id] else {
                filteredHeadwords = []
                return
            }
            filteredHeadwords = bridge.prefix("", limit: listLimit)
        } else {
            // Use prefix search via the index (case-insensitive, binary search).
            guard let id = selectedDictId, let bridge = bridges[id] else {
                filteredHeadwords = []
                return
            }
            filteredHeadwords = bridge.prefix(searchText, limit: listLimit)
        }
    }

    /// searchText/searchScope didSet call applyFilter directly; keep the
    /// page window in sync when a fresh filter run happens.
    private var lastFilterKey: String = ""
    private func resetPaginationIfNeeded() {
        let key = "\(selectedDictId?.uuidString ?? "")|\(searchText)|\(searchScope.rawValue)"
        if key != lastFilterKey {
            lastFilterKey = key
            listLimit = pageSize
        }
    }
}

enum SearchScope: String, CaseIterable {
    case current = "Current"
    case all = "All"
}
