import SwiftUI

@main
struct RdictApp: App {
    @StateObject private var store = DictionaryStore()
    @State private var showFileImporter = false

    var body: some Scene {
        WindowGroup {
            ContentView()
                #if os(macOS)
                .frame(minWidth: 800, minHeight: 500)
                #endif
                .environmentObject(store)
                .onAppear {
                    if !store.isLoaded {
                        store.restoreLastSession()
                    }
                }
                .onChange(of: store.shouldShowOpenPanel) { _, newValue in
                    if newValue {
                        showFileImporter = true
                        store.shouldShowOpenPanel = false
                    }
                }
                .fileImporter(
                    isPresented: $showFileImporter,
                    allowedContentTypes: [.data],
                    allowsMultipleSelection: true
                ) { result in
                    if case .success(let urls) = result {
                        for url in urls {
                            store.open(url: url)
                        }
                    }
                }
        }
    }
}
